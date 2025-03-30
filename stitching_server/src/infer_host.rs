use std::io;

use stitch::proj::DepthData;
use tensorrt::{CudaBuffer, CudaError, CudaStream, RuntimeEngineContext};
use thiserror::Error;
use trt_yolo::Which;

#[derive(Debug, Error)]
pub enum InferError {
    #[error("io error: {0}")]
    IO(#[from] io::Error),
    #[error("cuda error: {0}")]
    Cuda(#[from] CudaError),
}

pub type InferResult<T> = ::std::result::Result<T, InferError>;

pub struct InferHost<H: InferHandler> {
    ready_send: kanal::AsyncSender<kanal::OneshotAsyncSender<Vec<H::Item>>>,
}

impl<H: InferHandler> Clone for InferHost<H> {
    fn clone(&self) -> Self {
        Self {
            ready_send: self.ready_send.clone(),
        }
    }
}

impl<H: InferHandler + Send + Sync + 'static> InferHost<H>
where
    H::Item: Send + 'static,
{
    pub fn spawn(handlers: impl IntoIterator<Item = H> + Send + 'static) -> InferResult<Self> {
        let which = Which::best();

        let (ready_send, ready_recv) =
            kanal::bounded_async::<kanal::OneshotAsyncSender<Vec<H::Item>>>(4);

        tokio::spawn(async move {
            let mut in_bufs = handlers
                .into_iter()
                .map(|h| {
                    CudaBuffer::new(which.input_elems() * size_of::<u8>()).map(|b| {
                        (
                            b,
                            h,
                            vec![0; 640 * 640 * 4],
                            DepthData::new_zeroed(640, 640),
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()
                .unwrap();

            let mut out_mem = CudaBuffer::new(which.out_elems() * size_of::<half::f16>())
                .expect("failed to create infer output cuda buffer");
            let mut out_buf = vec![half::f16::from_f32_const(0.); which.out_elems()];

            let runtime = RuntimeEngineContext::new_engine_slice(
                &which
                    .plan_data()
                    .expect("failed to load infer plan data for {which:?}"),
            );
            runtime.as_ctx().set_output_tensor(c"output0", &mut out_mem);

            let stream = CudaStream::new().expect("failed to create infer cuda stream");
            loop {
                match ready_recv.recv().await {
                    Ok(done) => {
                        futures_util::future::join_all(
                            in_bufs.iter_mut().map(|(_, h, img, depth)| async {
                                h.fetch_image(img, depth).await
                            }),
                        )
                        .await;

                        let bound_groups = in_bufs
                            .iter_mut()
                            .map(|(cbuf, _, img_data, _)| {
                                cbuf.copy_from_async(img_data, &stream).unwrap();

                                let ctx = runtime.as_ctx();
                                ctx.set_input_tensor(c"images", cbuf);

                                // enqueue one run of the model
                                ctx.enqueue(&stream);

                                if let Err(err) = out_mem
                                    .copy_to_async(bytemuck::cast_slice_mut(&mut out_buf), &stream)
                                {
                                    tracing::error!("while loading bound buffer: {}", err);
                                    return Vec::new();
                                }

                                stream
                                    .synchronize()
                                    .expect("failed to synchronize infer stream");

                                trt_yolo::nms_cpu(&out_buf, which.out_shape(), 0.65, 0.5)
                            })
                            .collect::<Vec<_>>();

                        let out = futures_util::future::join_all(
                            bound_groups.into_iter().zip(&mut in_bufs).map(
                                |(bbs, (_, h, _, depth))| async move {
                                    h.handle_bounds(bbs, depth).await
                                },
                            ),
                        )
                        .await;

                        // if this fails, the receiver must not have needed a response
                        _ = done.send(out).await;
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

        Ok(Self { ready_send })
    }

    // pub fn run_input(&self, n: usize, custom: T, data: &[u8], depth: DepthData<'_>) {
    //     let Ok(mut lock) = self.in_bufs[n].try_lock() else {
    //         // lock is already held, skip this frame or we will block.
    //         return;
    //     };
    //     if let Err(err) = lock.0.copy_from(data) {
    //         tracing::error!("error setting infer input {n}: {err}");
    //     }
    //     lock.1 = Some(custom);
    //     lock.2.copy_from(&depth);
    // }

    pub async fn req_infer(&self) -> Vec<H::Item> {
        let (resp_send, resp) = kanal::oneshot_async();

        if let Err(err) = self.ready_send.send(resp_send).await {
            tracing::error!("error requesting infer: {err}")
        }

        resp.recv().await.unwrap()
    }
}

pub type BoundingClass = trt_yolo::BoundingClass;

pub trait InferHandler {
    type Item;

    fn fetch_image(
        &mut self,
        img: &mut [u8],
        depth: &mut DepthData<'_>,
    ) -> impl Future<Output = ()> + Send;

    fn handle_bounds(
        &mut self,
        bounds: Vec<BoundingClass>,
        depth: &DepthData,
    ) -> impl Future<Output = Self::Item> + Send;
}
