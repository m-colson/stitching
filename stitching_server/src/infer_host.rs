use std::{io, sync::Arc};

use stitch::proj::DepthData;
use tensorrt::{CudaBuffer, CudaError, CudaStream, RuntimeEngineContext};
use thiserror::Error;
use tokio::sync::Mutex;
use trt_yolo::{BoundingClass, Which};

#[derive(Debug, Error)]
pub enum InferError {
    #[error("io error: {0}")]
    IO(#[from] io::Error),
    #[error("cuda error: {0}")]
    Cuda(#[from] CudaError),
}

pub type InferResult<T> = ::std::result::Result<T, InferError>;

type BoundHandler<T> = Box<dyn FnMut(usize, Option<T>, Vec<BoundingClass>, DepthData) + Send>;

#[derive(Clone)]
pub struct InferHost<T> {
    in_bufs: Arc<[Mutex<(CudaBuffer, Option<T>, DepthData<'static>)>]>,
    ready_send: kanal::AsyncSender<BoundHandler<T>>,
}

impl<T: Send + 'static> InferHost<T> {
    pub fn spawn(num_bufs: usize) -> InferResult<Self> {
        let which = Which::best();

        let in_bufs = (0..num_bufs)
            .map(|_| {
                CudaBuffer::new(which.input_elems() * size_of::<u8>())
                    .map(|b| Mutex::new((b, None, DepthData::new_zeroed(640, 640))))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Arc::<[_]>::from)?;

        let (ready_send, ready_recv) = kanal::bounded_async::<BoundHandler<T>>(num_bufs);

        let host_in_bufs = in_bufs.clone();
        tokio::spawn(async move {
            let mut out_mem = CudaBuffer::new(which.out_elems() * size_of::<half::f16>()).unwrap();
            let mut out_buf = vec![half::f16::from_f32_const(0.); which.out_elems()];

            let runtime = RuntimeEngineContext::new_engine_slice(&which.plan_data().unwrap());
            runtime.as_ctx().set_output_tensor(c"output0", &mut out_mem);

            let stream = CudaStream::new().unwrap();
            loop {
                match ready_recv.recv().await {
                    Ok(mut f) => {
                        for (n, locked_buf) in host_in_bufs.iter().enumerate() {
                            let mut locked = locked_buf.lock().await;

                            let ctx = runtime.as_ctx();
                            ctx.set_input_tensor(c"images", &locked.0);

                            // enqueue running the model
                            ctx.enqueue(&stream);

                            if let Err(err) = out_mem
                                .copy_to_async(bytemuck::cast_slice_mut(&mut out_buf), &stream)
                            {
                                tracing::error!("while loading bound buffer: {}", err);
                                continue;
                            }

                            stream.synchronize().unwrap();

                            f(
                                n,
                                locked.1.take(),
                                trt_yolo::nms_cpu(&out_buf, which.out_shape(), 0.65, 0.5),
                                locked.2.to_ref(),
                            )
                        }
                    }
                    Err(err) => {
                        match err {
                            kanal::ReceiveError::Closed => {
                                tracing::warn!("inferer exiting because it was closed")
                            }
                            kanal::ReceiveError::SendClosed => {
                                tracing::warn!("inferer exiting because all senders were dropped")
                            }
                        }
                        break;
                    }
                };
            }
        });

        Ok(Self {
            in_bufs,
            ready_send,
        })
    }

    pub fn run_input(&self, n: usize, custom: T, data: &[u8], depth: DepthData<'_>) {
        let Ok(mut lock) = self.in_bufs[n].try_lock() else {
            // lock is already held, skip this frame or we will block.
            return;
        };
        if let Err(err) = lock.0.copy_from(data) {
            tracing::error!("error setting infer input {n}: {err}");
        }
        lock.1 = Some(custom);
        lock.2.copy_from(&depth);
    }

    pub async fn req_infer(
        &self,
        handler: impl FnMut(usize, Option<T>, Vec<BoundingClass>, DepthData) + Send + 'static,
    ) {
        if let Err(err) = self.ready_send.send(Box::new(handler)).await {
            tracing::error!("error requesting infer: {err}")
        }
    }
}
