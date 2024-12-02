use axum::extract::ws::Message;
use stitch::{
    buf::FrameSize,
    camera::{live, Camera},
    loader::{self, Loader, OwnedWriteBuffer},
    proj::{self, GpuDirectBufferWrite, GpuProjector, ProjectionStyle},
    Result,
};

use crate::util::IntervalTimer;

use super::proto::VideoPacket;
pub enum UpdateFn {
    ProjSpec(Box<dyn FnOnce(&mut ProjectionStyle) + Send>),
}

pub struct Sticher {
    msg_recv: kanal::AsyncReceiver<Message>,
    update_send: kanal::Sender<UpdateFn>,
}

impl Sticher {
    pub async fn from_cfg_gpu(
        cfg: proj::Config<live::Config>,
        proj_w: usize,
        proj_h: usize,
    ) -> Self {
        let cam_res = cfg.cameras[0]
            .meta
            .resolution
            .expect("missing resolution for camera 0");

        let proj = GpuProjector::builder_auto()
            .await
            .unwrap()
            .input_size(
                cam_res[0],
                cam_res[1],
                cfg.cameras.len().try_into().unwrap(),
            )
            .out_size(proj_w, proj_h)
            .flat_bound()
            .masks_from_cfgs(&cfg.cameras)
            .build();

        let (msg_send, msg_recv) = kanal::bounded(0);
        let (update_send, update_recv) = kanal::bounded(4);

        tokio::task::spawn_blocking(move || {
            let inner =
                SticherInner::from_cfg(&cfg, (proj_w, proj_h), msg_send, update_recv).unwrap();

            SticherInner::block(inner, &proj);
        });

        Self {
            msg_recv: msg_recv.to_async(),
            update_send,
        }
    }

    pub async fn next_frame_msg(&self) -> Option<Message> {
        self.msg_recv.recv().await.ok()
    }

    pub fn update_style<F: FnOnce(&mut ProjectionStyle) + Send + 'static>(&self, f: F) {
        _ = self.update_send.send(UpdateFn::ProjSpec(Box::new(f)));
    }
}

struct SticherInner<B: OwnedWriteBuffer> {
    pub sender: kanal::Sender<Message>,
    pub update_chan: kanal::Receiver<UpdateFn>,
    pub proj_style: ProjectionStyle,
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
            .map(|cfg| {
                let cam = cfg.clone().load()?;
                let (w, h, c) = cam.data.frame_size();
                tracing::info!("loaded camera {:?} ({w} * {h} * {c})", cfg.meta.live_index);
                Ok(cam)
            })
            .collect::<Result<Vec<_>>>()?;

        tracing::info!("finished loading cameras");

        Ok(Self {
            sender,
            update_chan,
            proj_style: cfg.style,
            proj_buf: VideoPacket::new(proj_size.0, proj_size.1, 4)?,
            cams,
        })
    }
}

impl SticherInner<GpuDirectBufferWrite> {
    pub fn block(mut self, proj: &GpuProjector) {
        // first frame load takes much longer, do it before we starting profiling.
        loader::block_discard_tickets(proj.take_input_buffers(&self.cams).unwrap());

        let mut timer = IntervalTimer::new();
        while self.avail_updates() {
            timer.start();
            let buf_tickets = proj.take_input_buffers(&self.cams).unwrap();

            proj.update_cam_specs(&self.cams);
            proj.update_proj_view(self.proj_style);

            timer.mark("setup");

            loader::block_discard_tickets(buf_tickets);

            timer.mark("frame load");

            proj.update_render();
            proj.block_copy_render_to(&mut self.proj_buf);

            timer.mark("backward");

            self.proj_buf.update_time();
            timer.mark_from_base("generation");

            let msg = self.proj_buf.take_message();
            if self.sender.send(msg).is_err() {
                break;
            }

            timer.mark("handoff");
            timer.log_iters_per_sec("render");
        }

        tracing::info!("stitching thread exiting");
    }

    #[inline]
    fn avail_updates(&mut self) -> bool {
        loop {
            match self.update_chan.try_recv() {
                Ok(Some(msg)) => match msg {
                    UpdateFn::ProjSpec(f) => f(&mut self.proj_style),
                },
                Ok(None) => return true,
                Err(_) => return false,
            }
        }
    }
}
