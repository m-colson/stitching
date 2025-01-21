use std::io::Cursor;

use axum::extract::ws::Message;
use stitch::{
    buf::FrameSize,
    camera::{live, Camera},
    loader::{self, Loader, OwnedWriteBuffer},
    proj::{self, GpuDirectBufferWrite, GpuProjector, ProjectionStyle, ViewStyle},
    Result,
};
use tokio::runtime::Handle;

use crate::util::IntervalTimer;

use super::proto::VideoPacket;
pub enum UpdateFn {
    ProjStyle(Box<dyn FnOnce(&mut ProjectionStyle) + Send>),
    ViewStyle(Box<dyn FnOnce(&mut ViewStyle) + Send>),
}

pub struct Sticher {
    msg_recv: kanal::AsyncReceiver<Message>,
    update_send: kanal::Sender<UpdateFn>,
}

impl Sticher {
    pub fn from_cfg_gpu(cfg: proj::Config<live::Config>, proj_w: usize, proj_h: usize) -> Self {
        let subs = 4;

        let cam_res = cfg.cameras[0]
            .meta
            .resolution
            .expect("missing resolution for camera 0");

        let mut proj = GpuProjector::builder()
            .num_subs(subs)
            .input_size(
                cam_res[0],
                cam_res[1],
                cfg.cameras.len().try_into().unwrap(),
            )
            .out_size(proj_w, proj_h)
            .cylinder_bound()
            .masks_from_cfgs(&cfg.cameras)
            .model(|m| m.obj_file_reader(Cursor::new(include_str!("../../assets/whole_plane.obj"))))
            .build();

        let (msg_send, msg_recv) = kanal::bounded(0);
        let (update_send, update_recv) = kanal::bounded(4);

        let inferer = crate::infer_host::InferHost::spawn(4).unwrap();

        tokio::task::spawn(async move {
            let res = SticherInner::from_cfg(&cfg, (proj_w, proj_h), msg_send, update_recv);
            let Ok(inner) =
                res.inspect_err(|err| tracing::error!("stitcher exiting because {err}"))
            else {
                return;
            };

            let res = SticherInner::run(inner, &mut proj, move |n, img| {
                tokio::task::block_in_place(|| {
                    Handle::current().block_on(inferer.run_input(n, img, |bbs| {
                        for b in bbs {
                            tracing::info!("bound {b}");
                        }
                    }));
                });
            })
            .await;

            if let Err(err) = res {
                tracing::error!("stitcher exiting because {err}");
            } else {
                tracing::warn!("stitcher exiting normally");
            }
        });

        Self {
            msg_recv: msg_recv.to_async(),
            update_send,
        }
    }

    pub async fn next_frame_msg(&self) -> Option<Message> {
        self.msg_recv.recv().await.ok()
    }

    pub fn update_proj_style<F: FnOnce(&mut ProjectionStyle) + Send + 'static>(&self, f: F) {
        _ = self.update_send.send(UpdateFn::ProjStyle(Box::new(f)));
    }

    pub fn update_view_style<F: FnOnce(&mut ViewStyle) + Send + 'static>(&self, f: F) {
        _ = self.update_send.send(UpdateFn::ViewStyle(Box::new(f)));
    }
}

struct SticherInner<B: OwnedWriteBuffer> {
    pub sender: kanal::Sender<Message>,
    pub update_chan: kanal::Receiver<UpdateFn>,
    pub proj_style: ProjectionStyle,
    pub view_style: ViewStyle,
    pub proj_buf: VideoPacket,
    pub cams: Vec<Camera<Loader<B>>>,
}

impl<B: OwnedWriteBuffer + 'static> SticherInner<B> {
    pub fn from_cfg(
        cfg: &proj::Config<live::Config>,
        proj_size: (usize, usize),
        sender: kanal::Sender<Message>,
        update_chan: kanal::Receiver<UpdateFn>,
    ) -> Result<Self> {
        let cams = cfg
            .cameras
            .iter()
            .filter_map(|cfg| match cfg.clone().load() {
                Ok(cam) => {
                    let (w, h, c) = cam.data.frame_size();
                    tracing::info!("loaded camera {:?} ({w} * {h} * {c})", cfg.meta.live_index);
                    Some(cam)
                }
                Err(err) => {
                    tracing::error!("{err}");

                    let out = cfg.meta.resolution.map(|res| {
                        cfg.load_with(Loader::new_blocking(res[0], res[1], 4, move |b| {
                            b.fill(255);
                        }))
                    });

                    if out.is_none() {
                        tracing::error!(
                            "missing fallback resolution for camera {}, removing it from rendering",
                            cfg.meta.live_index
                        )
                    }

                    out
                }
            })
            .collect::<Vec<_>>();

        tracing::info!("finished loading cameras");

        Ok(Self {
            sender,
            update_chan,
            proj_style: cfg.style,
            view_style: cfg.view,
            proj_buf: VideoPacket::new(proj_size.0, proj_size.1, 4)?,
            cams,
        })
    }
}

impl SticherInner<GpuDirectBufferWrite> {
    pub async fn run(
        mut self,
        proj: &mut GpuProjector,
        sub_handler: impl FnOnce(usize, &[u8]) + Send + Clone,
    ) -> stitch::Result<()> {
        // first frame load takes much longer, do it before we starting profiling.
        loader::discard_tickets(proj.take_input_buffers(&self.cams)?).await;

        let mut timer = IntervalTimer::new();
        while self.avail_updates() {
            if let ViewStyle::Orbit {
                theta,
                frame_per_rev,
                ..
            } = &mut self.view_style
            {
                *theta += 2. * std::f32::consts::PI / *frame_per_rev;
            }

            timer.start();
            let buf_tickets = proj.take_input_buffers(&self.cams)?;

            proj.update_cam_specs(&self.cams);
            proj.update_proj_view(self.view_style);
            proj.update_sub_views();

            timer.mark("setup");

            loader::discard_tickets(buf_tickets).await;

            timer.mark("frame load");

            proj.update_render();
            proj.copy_render_to(&mut self.proj_buf).await;

            timer.mark("backward");

            self.proj_buf.update_time();
            timer.mark_from_base("generation");

            let msg = self.proj_buf.take_message();
            if self.sender.send(msg).is_err() {
                break;
            }

            timer.mark("handoff");

            proj.wait_for_subs(sub_handler.clone()).await;

            timer.mark("subexec");
            timer.log_iters_per_sec("render");
        }

        tracing::info!("stitching thread exiting because updater has closed");
        Ok(())
    }

    #[inline]
    fn avail_updates(&mut self) -> bool {
        loop {
            match self.update_chan.try_recv() {
                Ok(Some(msg)) => match msg {
                    UpdateFn::ProjStyle(f) => f(&mut self.proj_style),
                    UpdateFn::ViewStyle(f) => f(&mut self.view_style),
                },
                Ok(None) => return true,
                Err(_) => return false,
            }
        }
    }
}
