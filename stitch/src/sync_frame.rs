use crate::{frame::ToFrameBufferAsync, FrameBuffer, FrameBufferMut, FrameSize};

impl<'a, T: FrameBuffer + 'a> ToFrameBufferAsync<'a> for tokio::sync::Mutex<T> {
    type Output = tokio::sync::MutexGuard<'a, T>;

    fn to_frame_async(&'a self) -> impl std::future::Future<Output = Self::Output> {
        self.lock()
    }
}

impl<'a, T: FrameSize + 'a> FrameSize for tokio::sync::MutexGuard<'a, T> {
    fn width(&self) -> usize {
        (**self).width()
    }
    fn height(&self) -> usize {
        (**self).height()
    }
    fn chans(&self) -> usize {
        (**self).chans()
    }
}

impl<'a, T: FrameBuffer + 'a> FrameBuffer for tokio::sync::MutexGuard<'a, T> {
    fn as_bytes(&self) -> &[u8] {
        (**self).as_bytes()
    }

    fn mask_bytes(&self) -> Option<&[u8]> {
        (**self).mask_bytes()
    }
}

impl<'a, T: FrameBufferMut + 'a> FrameBufferMut for tokio::sync::MutexGuard<'a, T> {
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        (**self).as_bytes_mut()
    }
}
