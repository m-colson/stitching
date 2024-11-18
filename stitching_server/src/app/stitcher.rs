use std::time::Instant;

use axum::extract::ws::Message;
use stitch::{
    camera::{Camera, CameraSpec, LiveSpec},
    frame::FrameSize,
    loader::{FrameLoader, LoadingBuffer, OwnedWriteBuffer},
    proj::{CpuProjector, FetchProjector, GpuDirectBufferWrite, GpuProjector, ProjSpec},
    Result,
};

use crate::util::time_op;

use super::video::VideoPacket;

pub type UpdateFn = Box<dyn FnOnce(&mut CameraSpec) + Send>;

pub struct Sticher {
    msg_recv: kanal::AsyncReceiver<Message>,
    update_send: kanal::Sender<UpdateFn>,
}

impl Sticher {
    pub fn from_cfg(cfg: stitch::Config<LiveSpec>, proj_w: usize, proj_h: usize) -> Self {
        let (msg_send, msg_recv) = kanal::bounded(1);
        let (update_send, update_recv) = kanal::bounded(4);

        tokio::task::spawn_blocking(move || {
            let inner = SticherInner::<Box<[u8]>>::from_cfg(
                cfg,
                proj_w,
                proj_h,
                msg_send,
                update_recv,
                LoadingBuffer::from,
            )
            .unwrap();
            inner.block(CpuProjector::sized(proj_w, proj_h));
        });

        Self {
            msg_recv: msg_recv.to_async(),
            update_send,
        }
    }

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
            let inner: SticherInner<(), GpuDirectBufferWrite> = SticherInner::from_cfg(
                cfg,
                proj_w,
                proj_h,
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

    pub fn update_spec<F: FnOnce(&mut CameraSpec) + Send + 'static>(&self, f: F) {
        _ = self.update_send.send(Box::new(f))
    }
}

struct SticherInner<T, B: OwnedWriteBuffer = T> {
    pub sender: kanal::Sender<Message>,
    pub update_chan: kanal::Receiver<UpdateFn>,
    pub proj_spec: CameraSpec,
    pub proj_ty: ProjSpec,
    pub proj_buf: VideoPacket,
    pub cams: Vec<Camera<LoadingBuffer<T, B>, LiveSpec>>,
}

impl<T, B: OwnedWriteBuffer + 'static> SticherInner<T, B>
where
    LoadingBuffer<T, B>: FrameSize,
{
    pub fn from_cfg(
        cfg: stitch::Config<LiveSpec>,
        proj_w: usize,
        proj_h: usize,
        sender: kanal::Sender<Message>,
        update_chan: kanal::Receiver<UpdateFn>,
        buf_maker: impl Fn(FrameLoader<B>) -> LoadingBuffer<T, B> + Clone,
    ) -> Result<Self> {
        let cams = cfg
            .cameras
            .iter()
            .map(|cfg| {
                let cam = cfg.load()?.map_with_meta(buf_maker.clone());
                let (w, h, c) = cam.buf.frame_size();
                tracing::info!("loaded camera {:?} ({w} * {h} * {c})", cfg.meta.live_index);
                Ok(cam)
            })
            .collect::<Result<Vec<_>>>()?;

        tracing::info!("finished loading cameras");

        Ok(Self {
            sender,
            update_chan,
            proj_spec: cfg.proj.spec.with_dims(proj_w as f32, proj_h as f32),
            proj_ty: cfg.proj.meta,
            proj_buf: VideoPacket::new(proj_w, proj_h, 4),
            cams,
        })
    }
}

impl<T, B: OwnedWriteBuffer> SticherInner<T, B> {
    pub fn block(mut self, proj: impl FetchProjector<T, B>) {
        let mut forws = proj.new_forw();

        while self.avail_updates() {
            let frame_start_time = Instant::now();

            let buf_chans = proj.begin_fetch(&mut self.cams);

            time_op("forward", || {
                proj.load_forw(self.proj_ty.style, self.proj_spec, &mut forws);
            });

            let fetched_cams = time_op("frame load", || {
                proj.block_finish_fetch(&mut self.cams, buf_chans)
            });

            time_op("backward", || {
                proj.load_back(&forws, &fetched_cams, &mut self.proj_buf)
            });

            time_op("handoff", || {
                let msg = self.proj_buf.take_message();
                self.sender.send(msg).unwrap();
            });

            let frame_dur = frame_start_time.elapsed();
            let fps = 1. / frame_dur.as_secs_f32();
            tracing::info!(fps = fps, "render");
        }

        tracing::info!("stitching thread exiting");
    }

    #[inline]
    fn avail_updates(&mut self) -> bool {
        loop {
            match self.update_chan.try_recv() {
                Ok(Some(f)) => f(&mut self.proj_spec),
                Ok(None) => return true,
                Err(_) => return false,
            }
        }
    }
}
