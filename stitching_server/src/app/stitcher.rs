use std::{fs::File, io::BufReader};

use axum::extract::ws::Message;
use stitch::{
    buf::FrameSize,
    camera::{live, Camera},
    loader::{self, Loader, OwnedWriteBuffer},
    proj::{
        self, DepthData, GpuDirectBufferWrite, GpuProjector, InverseView, ProjectionStyle,
        TexturedVertex, ViewStyle,
    },
    Result,
};

use crate::{infer_host::InferHost, util::IntervalTimer};

use super::proto::VideoPacket;
pub enum Update {
    ProjStyle(Box<dyn FnOnce(&mut ProjectionStyle) + Send>),
    ViewStyle(Box<dyn FnOnce(&mut ViewStyle) + Send>),
    Bounds(Vec<TexturedVertex>),
}

pub struct Sticher {
    msg_recv: kanal::AsyncReceiver<Message>,
    update_send: kanal::Sender<Update>,
}

impl Sticher {
    pub fn from_cfg_gpu(cfg: proj::Config<live::Config>, proj_w: usize, proj_h: usize) -> Self {
        let subs = 8;

        let cam_resolutions = cfg
            .cameras
            .iter()
            .map(|c| c.meta.resolution)
            .collect::<Vec<_>>();

        let mut proj_builder = GpuProjector::builder()
            .num_subs(subs)
            .input_sizes(cam_resolutions)
            .out_size(proj_w, proj_h)
            .world(&cfg.world)
            .masks_from_cfgs(&cfg.cameras);

        if let Some(model) = &cfg.model {
            proj_builder = proj_builder
                .model(|m, ops| {
                    ops.light_dir
                        .set_global(&model.light_dir.unwrap_or([1., -0.5, 1.]).into());
                    m.obj_file_reader(BufReader::new(
                        File::open(&model.path).expect("unable to read model"),
                    ))
                })
                .model_origin(model.origin)
                .model_scale(model.scale.unwrap_or([1., 1., 1.]))
                .model_rot_deg(model.rot.unwrap_or([0., 0., 0.]))
        }

        let mut proj = proj_builder.build();

        let (msg_send, msg_recv) = kanal::bounded(0);
        let (update_send, update_recv) = kanal::bounded(subs);

        let inferer = InferHost::<InverseView>::spawn(subs).expect("failed to create infer host");

        let req_inferer = inferer.clone();
        let bound_update_send = update_send.clone_async();
        tokio::spawn(async move {
            loop {
                let (mut done_sends, done_recvs): (Vec<_>, Vec<_>) = (0..subs)
                    .map(|_| kanal::oneshot::<Vec<TexturedVertex>>())
                    .map(|(s, r)| (Some(s), r))
                    .unzip();
                req_inferer
                    .req_infer(move |n, view, bbs, depth| {
                        let mut vertices = Vec::new();

                        if let Some(InverseView(inv_mat)) = view {
                            for bb in bbs {
                                let lt_depth = depth.at(bb.xmin() as _, bb.ymin() as _);
                                let rt_depth = depth.at(bb.xmax() as _, bb.ymin() as _);
                                // let rb_depth = depth.at(bb.xmin() as _, bb.ymax() as _);
                                // let lb_depth = depth.at(bb.xmax() as _, bb.ymax() as _);

                                let sbb = bb.rescale(640., 640., 2., 2.);
                                let lt = inv_mat
                                    * glam::vec4(sbb.xmin() - 1., -(sbb.ymin() - 1.), lt_depth, 1.);
                                let lt = TexturedVertex::from_pos(lt / lt.w, -1., -1.);

                                let rt = inv_mat
                                    * glam::vec4(sbb.xmax() - 1., -(sbb.ymin() - 1.), rt_depth, 1.);
                                let rt = TexturedVertex::from_pos(rt / rt.w, 1., -1.);

                                let lb = inv_mat
                                    * glam::vec4(sbb.xmin() - 1., -(sbb.ymax() - 1.), lt_depth, 1.);
                                let lb = TexturedVertex::from_pos(lb / lb.w, -1., 1.);

                                let rb = inv_mat
                                    * glam::vec4(sbb.xmax() - 1., -(sbb.ymax() - 1.), rt_depth, 1.);
                                let rb = TexturedVertex::from_pos(rb / rb.w, 1., 1.);

                                vertices.extend([rt, lt, lb, lb, rb, rt]);
                                // tracing::info!("{n} {rt:?}: {bb}");
                            }
                        }

                        // if this fails, the receiver has already closed for
                        // this loop, so we can ignore the error.
                        _ = done_sends[n].take()
                            .expect(
                                "the infer request done signal was already used, which should be impossible"
                            )
                            .send(vertices);
                    })
                    .await;

                let vertices = futures_util::future::join_all(
                    done_recvs
                        .into_iter()
                        .map(|r| async { r.to_async().recv().await.ok() }),
                )
                .await
                .into_iter()
                .filter_map(|v| v)
                .flatten()
                .collect();

                // if this fails, the stitcher has probably exited and we also need exit
                if let Err(err) = bound_update_send.send(Update::Bounds(vertices)).await {
                    tracing::error!("bound updater exiting because it was unable to message the stitcher: {err:?}");
                    return;
                }
            }
        });

        tokio::spawn(async move {
            let res = SticherInner::from_cfg(&cfg, (proj_w, proj_h), msg_send, update_recv);
            let Ok(inner) =
                res.inspect_err(|err| tracing::error!("stitcher exiting because {err}"))
            else {
                return;
            };

            let res = SticherInner::run(inner, &mut proj, move |n, view, img, depth| {
                inferer.run_input(n, view, img, depth);
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

    pub fn update_proj_style(&self, f: impl FnOnce(&mut ProjectionStyle) + Send + 'static) {
        _ = self.update_send.send(Update::ProjStyle(Box::new(f)));
    }

    pub fn update_view_style(&self, f: impl FnOnce(&mut ViewStyle) + Send + 'static) {
        _ = self.update_send.send(Update::ViewStyle(Box::new(f)));
    }
}

struct SticherInner<B: OwnedWriteBuffer> {
    pub sender: kanal::Sender<Message>,
    pub update_chan: kanal::Receiver<Update>,
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
        update_chan: kanal::Receiver<Update>,
    ) -> Result<Self> {
        let cams = cfg
            .cameras
            .iter()
            .map(|cfg| match cfg.clone().load() {
                Ok(cam) => {
                    let (w, h, c) = cam.data.frame_size();
                    tracing::info!("loaded camera {:?} ({w} * {h} * {c})", cfg.meta.mode);
                    cam
                }
                Err(err) => {
                    tracing::error!("{err}");

                    cfg.load_with(Loader::new_blocking(
                        cfg.meta.resolution[0],
                        cfg.meta.resolution[1],
                        4,
                        move |b| {
                            b.fill(255);
                        },
                    ))
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
        sub_handler: impl FnOnce(usize, InverseView, &[u8], DepthData<'_>) + Send + Clone,
    ) -> stitch::Result<()> {
        let mut timer = IntervalTimer::new();
        while self.avail_updates(proj) {
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
            timer.mark("frame-load");

            proj.update_render();
            proj.copy_render_to(&mut self.proj_buf).await;
            timer.mark("backward");

            self.proj_buf.update_time();
            timer.mark_from_base("generation");

            let msg_handle = self.proj_buf.to_message();

            proj.wait_for_subs(sub_handler.clone()).await;
            timer.mark("sub-wait");

            let Ok(msg) = msg_handle.await else {
                tracing::error!("failed to receive encoded frame, dropping...");
                continue;
            };
            timer.mark("encode-wait");

            if self.sender.send(msg).is_err() {
                break;
            }
            timer.mark("handoff");

            timer.log_iters_per_sec("render");
        }

        tracing::info!("stitching thread exiting because updater has closed");
        Ok(())
    }

    #[inline]
    fn avail_updates(&mut self, proj: &mut GpuProjector) -> bool {
        loop {
            match self.update_chan.try_recv() {
                Ok(Some(msg)) => match msg {
                    Update::ProjStyle(f) => f(&mut self.proj_style),
                    Update::ViewStyle(f) => f(&mut self.view_style),
                    Update::Bounds(tris) => proj.update_bounding_verts(&tris),
                },
                Ok(None) => return true,
                Err(_) => return false,
            }
        }
    }
}
