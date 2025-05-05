//! This module contains the types and functions used to integrate object detection.

use std::io;

use stitch::proj::DepthData;
use thiserror::Error;
use trt_yolo::{CudaBuffer, CudaError, CudaStream, RuntimeEngineContext, Which};

/// Errors that could happen from the inferer.
#[derive(Debug, Error)]
#[allow(missing_docs)]
pub enum InferError {
    #[error("io error: {0}")]
    IO(#[from] io::Error),
    #[error("cuda error: {0}")]
    Cuda(#[from] CudaError),
}

/// [`Result`] alias for functions that could return [`InferError`]s.
pub type InferResult<T> = ::std::result::Result<T, InferError>;

/// Handle to an object detectector.
pub struct InferHost<H: InferHandler> {
    ready_send: kanal::AsyncSender<InferRequest<H::Item>>,
}

impl<H: InferHandler> Clone for InferHost<H> {
    fn clone(&self) -> Self {
        Self {
            ready_send: self.ready_send.clone(),
        }
    }
}

struct InferRequest<T> {
    pub min_iou: f32,
    pub min_score: f32,
    pub resp_send: kanal::AsyncSender<Vec<T>>,
}

impl<H: InferHandler + Send + Sync + 'static> InferHost<H>
where
    H::Item: Send + 'static,
{
    /// Spawns a new object detector running in a new task and returns a handle to it.
    /// Uses [`Which::best`] to determine which model to run.
    /// The `handlers` will be used as specified in [`InferHandler`].
    pub fn spawn(handlers: impl IntoIterator<Item = H> + Send + 'static) -> InferResult<Self> {
        let which = Which::best();

        let (ready_send, ready_recv) = kanal::bounded_async::<InferRequest<H::Item>>(4);

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
                    Ok(InferRequest {
                        min_iou,
                        min_score,
                        resp_send,
                    }) => {
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

                                trt_yolo::nms_to_bounding(
                                    &out_buf,
                                    which.out_shape(),
                                    min_iou,
                                    min_score,
                                )
                            })
                            .collect::<Vec<_>>();

                        let out = bound_groups
                            .into_iter()
                            .zip(&mut in_bufs)
                            .map(|(bbs, (_, h, _, depth))| h.handle_bounds(bbs, depth))
                            .collect();

                        // if this fails, the receiver must not have needed a response
                        _ = resp_send.send(out).await;
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

    /// Sends a request to the object detector with the specified `min_iou` and `min_score`.
    /// Returns a list of the items generated by [`InferHandler::handle_bounds`].
    pub async fn req_infer(&self, min_iou: f32, min_score: f32) -> Vec<H::Item> {
        let (resp_send, resp) = kanal::bounded_async(1);

        if let Err(err) = self
            .ready_send
            .send(InferRequest {
                min_iou,
                min_score,
                resp_send,
            })
            .await
        {
            tracing::error!("error requesting infer: {err}")
        }

        resp.recv().await.unwrap()
    }
}

/// Alias for [`trt_yolo::BoundingClass`].
pub type BoundingClass = trt_yolo::BoundingClass;

/// Can be implemented on a struct so it can be given as a handler in [`InferHost::spawn`].
pub trait InferHandler {
    /// The item that is ultimately returned by this handler.
    type Item;

    /// Called before detector runs. Implementors should write to the image and depth buffers when called.
    fn fetch_image(
        &mut self,
        img: &mut [u8],
        depth: &mut DepthData<'_>,
    ) -> impl Future<Output = ()> + Send;

    /// Called after bounding boxes are found in the image. Can be used to create
    /// [`InferHandler::Item`] with the detected bounds.
    fn handle_bounds(&mut self, bounds: Vec<BoundingClass>, depth: &DepthData) -> Self::Item;
}
