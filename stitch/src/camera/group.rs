use std::{future::Future, ops::Deref};

use crate::frame::{ToFrameBuffer, ToFrameBufferAsync};

use super::Camera;

pub trait CameraGroup<'a> {
    type Buf: ToFrameBuffer<'a>;
    fn to_cams_buf(&'a self) -> Vec<Camera<<Self::Buf as ToFrameBuffer<'a>>::Output>>;
}

impl<'a, K: 'a, B: ToFrameBuffer<'a> + 'a, T: Deref<Target = [Camera<B, K>]>> CameraGroup<'a>
    for T
{
    type Buf = B;

    fn to_cams_buf(&'a self) -> Vec<Camera<<Self::Buf as ToFrameBuffer<'a>>::Output>> {
        self.iter().map(|c| c.to_frame_buf()).collect()
    }
}

pub trait CameraGroupAsync<'a> {
    type Buf: ToFrameBufferAsync<'a>;
    fn to_cams_async(
        &'a self,
    ) -> impl Future<Output = Vec<Camera<<Self::Buf as ToFrameBufferAsync<'a>>::Output>>>;
}

impl<'a, K: 'a, B: ToFrameBufferAsync<'a> + 'a, T: Deref<Target = [Camera<B, K>]>>
    CameraGroupAsync<'a> for T
{
    type Buf = B;

    async fn to_cams_async(&'a self) -> Vec<Camera<B::Output>> {
        futures::future::join_all(self.iter().map(|c| c.to_frame_async())).await
    }
}
