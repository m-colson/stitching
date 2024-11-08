use std::future::Future;

use image::ImageDecoder;

use crate::Result;

pub trait FrameBuffer: FrameBufferable {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn chans(&self) -> usize;

    fn as_bytes(&self) -> &[u8];
    fn as_bytes_mut(&mut self) -> &mut [u8];

    fn pixel_at(&self, x: usize, y: usize) -> &[u8] {
        if x >= self.width() {
            panic!("pixel's x ({x}) out of range 0..{}", self.width());
        }
        if y >= self.height() {
            panic!("pixel's y ({y}) out of range 0..{}", self.height());
        }

        let chans = self.chans();
        &self.as_bytes()[(x + (y * self.height())) * chans..][..chans]
    }

    fn mask_bytes(&self) -> Option<&[u8]> {
        None
    }

    fn check_decoder(&self, dec: &impl ImageDecoder) -> Result<()> {
        let (img_width, img_height) = dec.dimensions();
        let img_chans = dec.color_type().channel_count();

        DimErrorKind::Bytes.check(1, (img_chans / dec.color_type().bytes_per_pixel()) as usize)?;
        DimErrorKind::Width.check(self.width(), img_width as usize)?;
        DimErrorKind::Height.check(self.height(), img_height as usize)?;
        DimErrorKind::Channel.check(self.chans(), img_chans as usize)?;

        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
#[error("{kind} mismatch: {exp} != {got}")]
pub struct DimError {
    pub kind: DimErrorKind,
    pub exp: usize,
    pub got: usize,
}

#[derive(Clone, Copy, Debug)]
pub enum DimErrorKind {
    Width,
    Height,
    Channel,
    Bytes,
}

impl std::fmt::Display for DimErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Width => write!(f, "width"),
            Self::Height => write!(f, "height"),
            Self::Channel => write!(f, "channel"),
            Self::Bytes => write!(f, "bytes"),
        }
    }
}

impl DimErrorKind {
    pub fn err(self, exp: usize, got: usize) -> DimError {
        DimError {
            kind: self,
            exp,
            got,
        }
    }

    pub fn check(self, exp: usize, got: usize) -> std::result::Result<(), DimError> {
        (exp == got).then_some(()).ok_or(self.err(exp, got))
    }
}

pub trait FrameBufferable {}

pub trait ToFrameBuffer: FrameBufferable {
    type Output: FrameBuffer;
    fn to_frame_buf(&self) -> Self::Output;
}

pub trait ToFrameBufferAsync<'a>: FrameBufferable {
    type Output: FrameBuffer + 'a;
    fn to_frame_async(&'a self) -> impl Future<Output = Self::Output>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StaticFrameBuffer<const W: usize, const H: usize, const C: usize = 3> {
    data: [[[u8; C]; W]; H],
}

impl<const W: usize, const H: usize, const C: usize> FrameBufferable
    for StaticFrameBuffer<W, H, C>
{
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

    fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(&self.data[0][0][0] as *const u8, H * W * C) }
    }

    fn as_bytes_mut(&mut self) -> &mut [u8] {
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
pub struct SizedFrameBuffer {
    pub width: usize,
    pub height: usize,
    pub chans: usize,
    pub data: Box<[u8]>,
}

impl SizedFrameBuffer {
    pub fn new(width: usize, height: usize, chans: usize) -> Self {
        Self {
            width,
            height,
            chans,
            data: vec![0; width * height * chans].into(),
        }
    }

    pub fn take_vec(&mut self) -> Vec<u8> {
        std::mem::replace(
            &mut self.data,
            vec![0; self.width * self.height * self.chans].into(),
        )
        .into_vec()
    }
}

impl FrameBufferable for SizedFrameBuffer {}

impl FrameBuffer for SizedFrameBuffer {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn chans(&self) -> usize {
        self.chans
    }

    fn as_bytes(&self) -> &[u8] {
        self.data.as_ref()
    }

    fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.data.as_mut()
    }
}

impl<B: FrameBufferable + ?Sized> FrameBufferable for Box<B> {}

impl<B: FrameBuffer + ?Sized> FrameBuffer for Box<B> {
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

    fn check_decoder(&self, dec: &impl ImageDecoder) -> Result<()> {
        (**self).check_decoder(dec)
    }
}
