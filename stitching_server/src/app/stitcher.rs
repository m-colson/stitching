use axum::extract::ws::Message;
use stitch::{
    camera::{Camera, CameraSpec, LiveSpec},
    frame::FrameSize,
    loader::{block_discard_tickets, FrameLoader, LoadingBuffer, OwnedWriteBuffer},
    proj::{GpuDirectBufferWrite, GpuProjector, ProjSpec},
    Result,
};

use crate::util::IntervalTimer;

use super::proto::VideoPacket;
pub enum UpdateFn {
    ProjCameraSpec(Box<dyn FnOnce(&mut CameraSpec) + Send>),
    ProjSpec(Box<dyn FnOnce(&mut ProjSpec) + Send>),
}

pub struct Sticher {
    msg_recv: kanal::AsyncReceiver<Message>,
    update_send: kanal::Sender<UpdateFn>,
}

impl Sticher {
    pub async fn from_cfg_gpu(
        cfg: stitch::Config<LiveSpec>,
        proj_w: usize,
        proj_h: usize,
        cam_w: usize,
        cam_h: usize,
    ) -> Self {
        let proj = GpuProjector::new_auto(proj_w, proj_h, (cam_w, cam_h, cfg.cameras.len()))
            .await
            .unwrap();
        let (msg_send, msg_recv) = kanal::bounded(0);
        let (update_send, update_recv) = kanal::bounded(4);

        tokio::task::spawn_blocking(move || {
            let inner: SticherInner<GpuDirectBufferWrite> = SticherInner::from_cfg(
                cfg,
                (proj_w, proj_h),
                (cam_w, cam_h),
                msg_send,
                update_recv,
                LoadingBuffer::new_none,
            )
            .unwrap();

            SticherInner::block(inner, proj);
        });

        Self {
            msg_recv: msg_recv.to_async(),
            update_send,
        }
    }

    pub async fn next_frame_msg(&self) -> Option<Message> {
        self.msg_recv.recv().await.ok()
    }

    pub fn update_cam_spec<F: FnOnce(&mut CameraSpec) + Send + 'static>(&self, f: F) {
        _ = self.update_send.send(UpdateFn::ProjCameraSpec(Box::new(f)))
    }

    pub fn update_ty<F: FnOnce(&mut ProjSpec) + Send + 'static>(&self, f: F) {
        _ = self.update_send.send(UpdateFn::ProjSpec(Box::new(f)))
    }
}

struct SticherInner<B: OwnedWriteBuffer> {
    pub sender: kanal::Sender<Message>,
    pub update_chan: kanal::Receiver<UpdateFn>,
    pub proj_cam_spec: CameraSpec,
    pub proj_ty: ProjSpec,
    pub proj_buf: VideoPacket,
    pub cams: Vec<Camera<LoadingBuffer<(), B>>>,
}

impl<B: OwnedWriteBuffer + 'static> SticherInner<B>
where
    LoadingBuffer<(), B>: FrameSize,
{
    pub fn from_cfg(
        cfg: stitch::Config<LiveSpec>,
        proj_size: (usize, usize),
        cam_size: (usize, usize),
        sender: kanal::Sender<Message>,
        update_chan: kanal::Receiver<UpdateFn>,
        buf_maker: impl Fn(FrameLoader<B>) -> LoadingBuffer<(), B> + Clone,
    ) -> Result<Self> {
        let cams = cfg
            .cameras
            .iter()
            .map(|cfg| {
                let cam = cfg
                    .clone()
                    .load(cam_size.0 as _, cam_size.1 as _)?
                    .map_with_meta(buf_maker.clone());
                let (w, h, c) = cam.buf.frame_size();
                tracing::info!("loaded camera {:?} ({w} * {h} * {c})", cfg.meta.live_index);
                Ok(cam)
            })
            .collect::<Result<Vec<_>>>()?;

        tracing::info!("finished loading cameras");

        Ok(Self {
            sender,
            update_chan,
            proj_cam_spec: cfg
                .proj
                .spec
                .with_dims(proj_size.0 as f32, proj_size.1 as f32),
            proj_ty: cfg.proj.meta,
            proj_buf: VideoPacket::new(proj_size.0, proj_size.1, 4),
            cams,
        })
    }
}

impl SticherInner<GpuDirectBufferWrite> {
    pub fn block(mut self, proj: GpuProjector) {
        let mut timer = IntervalTimer::new();
        while self.avail_updates() {
            timer.start();
            let buf_tickets = proj.take_input_buffers(&self.cams);

            proj.update_cam_specs(&self.cams);
            proj.update_proj_view(self.proj_cam_spec, self.proj_ty.style);

            timer.mark("setup");

            block_discard_tickets(buf_tickets);

            timer.mark("frame load");

            proj.update_render();
            proj.block_copy_render_to(&mut self.proj_buf);

            timer.mark("backward");

            let msg = self.proj_buf.take_message();
            self.sender.send(msg).unwrap();

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
                    UpdateFn::ProjCameraSpec(f) => f(&mut self.proj_cam_spec),
                    UpdateFn::ProjSpec(f) => f(&mut self.proj_ty),
                },
                Ok(None) => return true,
                Err(_) => return false,
            }
        }
    }
}
