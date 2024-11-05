use image::ImageDecoder;

use crate::{config::DimErrorKind, ConfigError};

pub trait FrameBuffer {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn chans(&self) -> usize;

    fn as_bytes<'a>(&'a self) -> &'a [u8];
    fn as_bytes_mut<'a>(&'a mut self) -> &'a mut [u8];

    fn mask_bytes<'a>(&'a self) -> Option<&'a [u8]> {
        None
    }

    fn check_decoder(&self, dec: &impl ImageDecoder) -> Result<(), ConfigError> {
        let (img_width, img_height) = dec.dimensions();
        let img_chans = dec.color_type().channel_count();

        DimErrorKind::Width.check(self.width(), img_width as usize)?;
        DimErrorKind::Height.check(self.height(), img_height as usize)?;
        DimErrorKind::Channel.check(self.chans(), img_chans as usize)?;

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StaticFrameBuffer<const W: usize, const H: usize, const C: usize = 3> {
    data: [[[u8; C]; W]; H],
}

impl<const W: usize, const H: usize, const C: usize> FrameBuffer for StaticFrameBuffer<W, H, C> {
    fn width(&self) -> usize {
        W
    }

    fn height(&self) -> usize {
        H
    }

    fn chans(&self) -> usize {
        C
    }

    fn as_bytes<'a>(&'a self) -> &'a [u8] {
        unsafe { std::slice::from_raw_parts(&self.data[0][0][0] as *const u8, H * W * C) }
    }

    fn as_bytes_mut<'a>(&'a mut self) -> &'a mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(&mut self.data[0][0][0] as *mut u8, H * W * C) }
    }
}

impl<const W: usize, const H: usize, const C: usize> Default for StaticFrameBuffer<W, H, C> {
    fn default() -> Self {
        Self {
            data: [[[0; C]; W]; H],
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct SizedFrameBuffer<D = Vec<u8>>
where
    D: std::ops::DerefMut<Target = [u8]>,
{
    pub width: usize,
    pub height: usize,
    pub chans: usize,
    pub data: D,
}

impl SizedFrameBuffer<Vec<u8>> {
    pub fn new(width: usize, height: usize, chans: usize) -> Self {
        Self {
            width,
            height,
            chans,
            data: vec![0; width * height * chans],
        }
    }
}

impl<D: std::ops::DerefMut<Target = [u8]>> FrameBuffer for SizedFrameBuffer<D> {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn chans(&self) -> usize {
        self.chans
    }

    fn as_bytes<'a>(&'a self) -> &'a [u8] {
        &self.data
    }

    fn as_bytes_mut<'a>(&'a mut self) -> &'a mut [u8] {
        &mut self.data
    }
}

impl<T: FrameBuffer> FrameBuffer for Box<T> {
    fn width(&self) -> usize {
        (**self).width()
    }
    fn height(&self) -> usize {
        (**self).height()
    }
    fn chans(&self) -> usize {
        (**self).chans()
    }

    fn as_bytes<'a>(&'a self) -> &'a [u8] {
        (**self).as_bytes()
    }
    fn as_bytes_mut<'a>(&'a mut self) -> &'a mut [u8] {
        (**self).as_bytes_mut()
    }

    fn mask_bytes<'a>(&'a self) -> Option<&'a [u8]> {
        (**self).mask_bytes()
    }

    fn check_decoder(&self, dec: &impl ImageDecoder) -> Result<(), ConfigError> {
        (**self).check_decoder(dec)
    }
}
