use std::{fs::File, io::BufReader, time::Duration};

use axum::extract::ws::Message;
use cam_loader::{Loader, OwnedWriteBuffer, buf::FrameSize};
use stitch::{
    Result,
    camera::Camera,
    proj::{self, GpuDirectBufferWrite, GpuProjector, ProjectionView, TexturedVertex, ViewStyle},
};

use crate::util::IntervalTimer;

#[cfg(feature = "trt")]
use crate::infer_host::InferHost;

use super::proto::VideoPacket;
pub enum StitchUpdate {
    ViewStyle(Box<dyn FnOnce(&mut ViewStyle) + Send>),
    #[allow(dead_code)] // never constructed when trt feature disabled
    Bounds(Vec<TexturedVertex>),
}

#[cfg(feature = "trt")]
pub enum InferUpdate {
    MinIOU(f32),
    MinScore(f32),
}

pub struct Sticher {
    msg_recv: kanal::AsyncReceiver<Message>,
    stitch_update_send: kanal::Sender<StitchUpdate>,

    #[cfg(feature = "trt")]
    infer_update_send: kanal::Sender<InferUpdate>,
}

impl Sticher {
    pub fn from_cfg_gpu(
        cfg: proj::Config<cam_loader::Config>,
        proj_w: usize,
        proj_h: usize,
    ) -> Self {
        let cam_resolutions = cfg
            .cameras
            .iter()
            .map(|c| c.meta.resolution)
            .collect::<Vec<_>>();

        let mut proj_builder = GpuProjector::builder()
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
        let (update_send, update_recv) = kanal::bounded(4);

        #[cfg(feature = "trt")]
        let infer_update_send = {
            use std::f32::consts::PI;
            use stitch::proj::DepthData;

            const NUM_SUBS: i32 = 8;

            let (infer_update_send, infer_updates) = kanal::unbounded::<InferUpdate>();

            let rm = 2. * PI / NUM_SUBS as f32;
            const SUB_HEIGHT: f32 = 10.;

            let subs = (0..NUM_SUBS)
                .map(|i| {
                    let rot = rm * (i as f32);

                    ViewStyle::Perspective {
                        pos: [0., 0., SUB_HEIGHT],
                        look_at: [rot.sin(), rot.cos(), SUB_HEIGHT],
                        fov_y: 70f32.to_radians(),
                    }
                })
                .map(|vs| {
                    let pv = proj.create_depth_view::<Vec<u8>, Vec<u8>>(640, 640);
                    InferView::new(vs, pv)
                })
                .collect::<Vec<_>>();

            let inferer = InferHost::spawn(subs).expect("failed to create infer host");

            let bound_update_send = update_send.clone_async();
            // infer probing task
            tokio::spawn(async move {
                let mut min_iou = 0.75;
                let mut min_score = 0.30;

                let mut timer = IntervalTimer::new();
                while !infer_updates.is_disconnected() {
                    while let Ok(Some(upd)) = infer_updates.try_recv() {
                        match upd {
                            InferUpdate::MinIOU(v) => min_iou = v,
                            InferUpdate::MinScore(v) => min_score = v,
                        }
                    }

                    timer.start();

                    let vertices = inferer
                        .req_infer(min_iou, min_score)
                        .await
                        .into_iter()
                        .flatten()
                        .collect();

                    // if this fails, the stitcher has probably exited and we also need exit
                    if let Err(err) = bound_update_send.send(StitchUpdate::Bounds(vertices)).await {
                        tracing::error!(
                            "bound updater exiting because it was unable to message the stitcher: {err:?}"
                        );
                        return;
                    }

                    timer.mark_from_base("bound-loop");
                }
            });

            infer_update_send
        };

        // stitcher running task
        tokio::spawn(async move {
            let res = SticherInner::from_cfg(&cfg, (proj_w, proj_h), msg_send, update_recv);
            let Ok(inner) =
                res.inspect_err(|err| tracing::error!("stitcher exiting because {err}"))
            else {
                return;
            };

            let res = SticherInner::run(inner, &mut proj).await;

            if let Err(err) = res {
                tracing::error!("stitcher exiting because {err}");
            } else {
                tracing::warn!("stitcher exiting normally");
            }
        });

        Self {
            msg_recv: msg_recv.to_async(),
            stitch_update_send: update_send,
            #[cfg(feature = "trt")]
            infer_update_send,
        }
    }

    pub async fn next_frame_msg(&self) -> Option<Message> {
        self.msg_recv.recv().await.ok()
    }

    pub fn update_view_style(&self, f: impl FnOnce(&mut ViewStyle) + Send + 'static) {
        _ = self
            .stitch_update_send
            .send(StitchUpdate::ViewStyle(Box::new(f)));
    }

    #[cfg(feature = "trt")]
    pub fn set_min_iou(&self, v: f32) {
        _ = self.infer_update_send.send(InferUpdate::MinIOU(v));
    }

    #[cfg(feature = "trt")]
    pub fn set_min_score(&self, v: f32) {
        _ = self.infer_update_send.send(InferUpdate::MinScore(v));
    }
}

#[cfg(feature = "trt")]
pub struct InferView {
    view: ViewStyle,
    proj: ProjectionView<(Vec<u8>, Vec<u8>)>,
    tmp_img: Option<Vec<u8>>,
    tmp_depth: Option<Vec<u8>>,
    cutoff_width: f32,
}

#[cfg(feature = "trt")]
impl InferView {
    pub fn new(view: ViewStyle, proj: ProjectionView<(Vec<u8>, Vec<u8>)>) -> Self {
        Self {
            view,
            proj,
            tmp_img: None,
            tmp_depth: None,
            cutoff_width: 25.5,
        }
    }
}

#[cfg(feature = "trt")]
impl crate::infer_host::InferHandler for InferView {
    type Item = Vec<TexturedVertex>;

    async fn fetch_image(&mut self, img: &mut [u8], depth: &mut DepthData<'_>) {
        if self.tmp_img.is_none() {
            self.tmp_img = Some(vec![0; img.len()]);
        }

        if self.tmp_depth.is_none() {
            // times 4 since tmp_depth is u8 and depth is f32
            self.tmp_depth = Some(vec![0; depth.len() * 4]);
        }

        let new_view = self.view;
        self.proj.update_view(move |v| *v = new_view).unwrap();

        let (r_img, r_depth) = self
            .proj
            .load_image2(self.tmp_img.take().unwrap(), self.tmp_depth.take().unwrap())
            .await
            .unwrap();

        img.copy_from_slice(&r_img);
        depth.copy_from(bytemuck::cast_slice(&r_depth));

        self.tmp_img = Some(r_img);
        self.tmp_depth = Some(r_depth);
    }

    fn handle_bounds(
        &mut self,
        bounds: Vec<crate::infer_host::BoundingClass>,
        depth: &DepthData<'_>,
    ) -> Self::Item {
        const WARN_COLOR: glam::Vec3 = glam::vec3(1.0, 1.0, 0.1);
        const ALERT_COLOR: glam::Vec3 = glam::vec3(1.0, 0.1, 0.1);

        let mut vertices = Vec::new();

        let inv_mat = self.view.transform_matrix(640., 640.).inverse();

        for bb in bounds {
            let lt_depth = depth.at(bb.xmin() as _, bb.ymin() as _);
            let rt_depth = depth.at(bb.xmax() as _, bb.ymin() as _);

            let ((xmin, ymin), (xmax, ymax)) = bb.rescale(640., 640., 2., 2.).corners();

            let corners = [
                (xmin, ymin, lt_depth),
                (xmax, ymin, rt_depth),
                (xmin, ymax, lt_depth),
                (xmax, ymax, rt_depth),
            ]
            .map(|(x, y, z)| {
                let p = inv_mat * glam::vec4(x - 1., -(y - 1.), z, 1.);
                p / p.w
            });

            let coord_unsafe = corners
                .into_iter()
                .any(|c| c.x.abs() < self.cutoff_width && c.y > 0.0);
            let color = if coord_unsafe {
                ALERT_COLOR
            } else {
                WARN_COLOR
            };

            let lt = TexturedVertex::from_pos(corners[0], -1., -1., color);
            let rt = TexturedVertex::from_pos(corners[1], 1., -1., color);
            let lb = TexturedVertex::from_pos(corners[2], -1., 1., color);
            let rb = TexturedVertex::from_pos(corners[3], 1., 1., color);

            vertices.extend([rt, lt, lb, lb, rb, rt]);
        }

        vertices
    }
}

struct SticherInner<B: OwnedWriteBuffer> {
    pub sender: kanal::Sender<Message>,
    pub update_chan: kanal::Receiver<StitchUpdate>,
    pub view_style: ViewStyle,
    pub proj_buf: VideoPacket,
    pub cams: Vec<Camera<Loader<B>>>,
}

impl<B: OwnedWriteBuffer + Send + 'static> SticherInner<B> {
    pub fn from_cfg(
        cfg: &proj::Config<cam_loader::Config>,
        proj_size: (usize, usize),
        sender: kanal::Sender<Message>,
        update_chan: kanal::Receiver<StitchUpdate>,
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
            view_style: cfg.view,
            proj_buf: VideoPacket::new(proj_size.0, proj_size.1, 4)?,
            cams,
        })
    }
}

impl SticherInner<GpuDirectBufferWrite> {
    pub async fn run(mut self, proj: &mut GpuProjector) -> stitch::Result<()> {
        let main_view = proj.create_vis_view(1280, 720, self.view_style);

        let mut timer = IntervalTimer::new();
        while self.avail_updates(proj, &main_view).await {
            timer.start();

            if let ViewStyle::Orbit { .. } = &self.view_style {
                main_view.update_view(|vs| {
                    if let ViewStyle::Orbit {
                        theta,
                        frame_per_rev,
                        ..
                    } = vs
                    {
                        *theta += 2. * std::f32::consts::PI / *frame_per_rev;
                    }
                })?;
            }

            let buf_tickets = proj.take_input_buffers(&self.cams)?;

            proj.update_cam_specs(&self.cams);
            timer.mark("setup");

            cam_loader::discard_tickets(buf_tickets).await;
            timer.mark("frame-load");

            self.proj_buf = main_view.load_image(self.proj_buf).await.unwrap();
            timer.mark("backward");

            self.proj_buf.update_time();
            timer.mark_from_base("generation");

            let msg_handle = self.proj_buf.to_message();

            let Ok(msg) = msg_handle.await else {
                tracing::error!("failed to receive encoded frame, dropping...");
                continue;
            };
            timer.mark("encode-wait");

            if self.sender.send(msg).is_err() {
                break;
            }
            timer.mark("handoff");

            timer
                .log_and_wait_fps("render", Duration::from_millis(1000 / 30 - 2))
                .await;
        }

        tracing::info!("stitching thread exiting because updater has closed");
        Ok(())
    }

    #[inline]
    async fn avail_updates(
        &mut self,
        proj: &mut GpuProjector,
        forwarding: &ProjectionView<VideoPacket>,
    ) -> bool {
        loop {
            match self.update_chan.try_recv() {
                Ok(Some(msg)) => match msg {
                    StitchUpdate::ViewStyle(f) => forwarding
                        .update_view(f)
                        .unwrap_or_else(|err| tracing::error!("failed to update main view: {err}")),
                    StitchUpdate::Bounds(tris) => proj.update_bounding_verts(&tris).await,
                },
                Ok(None) => return true,
                Err(_) => return false,
            }
        }
    }
}
