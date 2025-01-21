use std::{io, sync::Arc};

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

type BoundHandler = Box<dyn Fn(Vec<BoundingClass>) + Send>;

#[derive(Clone)]
pub struct InferHost {
    in_bufs: Arc<[Mutex<CudaBuffer>]>,
    // stream: Arc<CudaStream>,
    ready_send: kanal::AsyncSender<(usize, BoundHandler)>,
}

impl InferHost {
    pub fn spawn(n: usize) -> InferResult<Self> {
        let which = Which::best();

        let in_bufs = (0..n)
            .map(|_| CudaBuffer::new(which.input_elems() * size_of::<u8>()).map(Mutex::new))
            .collect::<Result<Vec<_>, _>>()
            .map(Arc::<[_]>::from)?;

        let mut out_mem = CudaBuffer::new(which.out_elems() * size_of::<half::f16>())?;

        let stream = Arc::new(CudaStream::new()?);
        let runtime = RuntimeEngineContext::new_engine_slice(&which.plan_data()?);
        runtime.as_ctx().set_output_tensor(c"output0", &mut out_mem);

        let (ready_send, ready_recv) = kanal::bounded_async::<(usize, BoundHandler)>(n);

        let host_stream = stream.clone();
        let host_in_bufs = in_bufs.clone();
        tokio::spawn(async move {
            let mut out_buf = vec![half::f16::from_f32_const(0.); which.out_elems()];
            loop {
                match ready_recv.recv().await {
                    Ok((in_index, f)) => {
                        let locked_buf: &Mutex<CudaBuffer> = &host_in_bufs[in_index];
                        let in_buf = locked_buf.lock().await;
                        let ctx = runtime.as_ctx();

                        ctx.set_input_tensor(c"images", &in_buf);
                        ctx.enqueue(&host_stream);
                        out_mem
                            .copy_to_async(bytemuck::cast_slice_mut(&mut out_buf), &host_stream)
                            .unwrap();
                        host_stream.synchronize().unwrap();

                        f(trt_yolo::nms_cpu(&out_buf, which.out_shape(), 0.65, 0.5))
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
            // stream,
            ready_send,
        })
    }

    pub async fn run_input(
        &self,
        n: usize,
        data: &[u8],
        handler: impl Fn(Vec<BoundingClass>) + Send + 'static,
    ) {
        if let Err(err) = self.in_bufs[n].lock().await.copy_from(data) {
            tracing::error!("error setting infer input {n}: {err}");
        } else if let Err(err) = self.ready_send.send((n, Box::new(handler))).await {
            tracing::error!("error setting infer input {n}: {err}")
        }
    }
}
