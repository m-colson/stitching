use crate::{
    frame::{FrameBufferable, ToFrameBufferAsync},
    FrameBuffer, Result,
};

impl<T: FrameBufferable> FrameBufferable for tokio::sync::Mutex<T> {}

impl<'a, T: FrameBuffer + 'a> ToFrameBufferAsync<'a> for tokio::sync::Mutex<T> {
    type Output = tokio::sync::MutexGuard<'a, T>;

    fn to_frame_async(&'a self) -> impl std::future::Future<Output = Self::Output> {
        self.lock()
    }
}

impl<'a, T: FrameBuffer + 'a> FrameBufferable for tokio::sync::MutexGuard<'a, T> {}

impl<'a, T: FrameBuffer + 'a> FrameBuffer for tokio::sync::MutexGuard<'a, T> {
    fn width(&self) -> usize {
        (**self).width()
    }
    fn height(&self) -> usize {
        (**self).height()
    }
    fn chans(&self) -> usize {
        (**self).chans()
    }

    fn as_bytes(&self) -> &[u8] {
        (**self).as_bytes()
    }
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        (**self).as_bytes_mut()
    }

    fn mask_bytes(&self) -> Option<&[u8]> {
        (**self).mask_bytes()
    }

    fn check_decoder(&self, dec: &impl image::ImageDecoder) -> Result<()> {
        (**self).check_decoder(dec)
    }
}
