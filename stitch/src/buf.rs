use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
};

pub trait FrameSize {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn chans(&self) -> usize;

    fn frame_size(&self) -> (usize, usize, usize) {
        (self.width(), self.height(), self.chans())
    }

    fn num_bytes(&self) -> usize {
        self.width() * self.height() * self.chans()
    }

    fn as_empty_view(&self) -> FrameBufferView<'static> {
        FrameBufferView::new(self.frame_size(), &[])
    }
}

pub trait PixelBufferExt: Deref<Target = [u8]> + FrameSize {
    fn pixel_at(&self, x: usize, y: usize) -> Option<&[u8]> {
        (x < self.width() && y < self.height()).then(|| {
            let chans = self.chans();
            &self[(x + (y * self.height())) * chans..][..chans]
        })
    }

    fn pixel_iter(&self) -> Box<dyn Iterator<Item = &[u8]> + '_> {
        let chans = self.chans();
        Box::new(self.chunks(chans))
    }
}

impl<T: Deref<Target = [u8]> + FrameSize> PixelBufferExt for T {}

pub trait PixelBufferMutExt: DerefMut<Target = [u8]> + FrameSize {
    fn pixel_iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut [u8]> + '_> {
        let chans = self.chans();
        Box::new(self.chunks_mut(chans))
    }
}

impl<T: DerefMut<Target = [u8]> + FrameSize> PixelBufferMutExt for T {}

pub struct FrameBufferView<'a> {
    data: Cow<'a, [u8]>,
    width: usize,
    height: usize,
    chans: usize,
}

impl<'a> FrameBufferView<'a> {
    #[must_use]
    #[inline]
    pub const fn new(size: (usize, usize, usize), data: &'a [u8]) -> Self {
        Self {
            data: Cow::Borrowed(data),
            width: size.0,
            height: size.1,
            chans: size.2,
        }
    }
}

impl<'a> FrameSize for FrameBufferView<'a> {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn chans(&self) -> usize {
        self.chans
    }
}

impl<'a> Deref for FrameBufferView<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
