use std::{future::Future, ops::Deref};

use crate::frame::ToFrameBufferAsync;

use super::Camera;

pub trait CameraGroupAsync<'a> {
    type Ty;
    type Buf: ToFrameBufferAsync<'a>;
    fn to_cams_async(
        &'a self,
    ) -> impl Future<Output = Vec<Camera<<Self::Buf as ToFrameBufferAsync<'a>>::Output, Self::Ty>>>;
}

impl<'a, K: Clone + 'a, B: ToFrameBufferAsync<'a> + 'a, T: Deref<Target = [Camera<B, K>]>>
    CameraGroupAsync<'a> for T
{
    type Ty = K;
    type Buf = B;

    async fn to_cams_async(&'a self) -> Vec<Camera<B::Output, K>> {
        futures::future::join_all(self.iter().map(|c| c.to_frame_async())).await
    }
}
