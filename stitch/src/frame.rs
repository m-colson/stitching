use std::future::Future;

use image::ImageDecoder;

use crate::Result;

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
}

pub trait FrameBuffer: FrameSize {
    fn as_bytes(&self) -> &[u8];

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

    fn pixel_iter(&self) -> impl Iterator<Item = &[u8]> {
        let chans = self.chans();
        self.as_bytes().chunks(chans)
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

pub trait FrameBufferMut: FrameBuffer {
    fn as_bytes_mut(&mut self) -> &mut [u8];

    fn pixel_iter_mut(&mut self) -> impl Iterator<Item = &mut [u8]> {
        let chans = self.chans();
        self.as_bytes_mut().chunks_mut(chans)
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

pub trait ToFrameBuffer<'a> {
    type Output: FrameBuffer + 'a;
    fn to_frame_buf(&'a self) -> Self::Output;
}

pub trait ToFrameBufferAsync<'a> {
    type Output: FrameBuffer + 'a;
    fn to_frame_async(&'a self) -> impl Future<Output = Self::Output>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StaticFrameBuffer<const W: usize, const H: usize, const C: usize = 3> {
    data: [[[u8; C]; W]; H],
}

impl<const W: usize, const H: usize, const C: usize> FrameSize for StaticFrameBuffer<W, H, C> {
    fn width(&self) -> usize {
        W
    }

    fn height(&self) -> usize {
        H
    }

    fn chans(&self) -> usize {
        C
    }
}

impl<const W: usize, const H: usize, const C: usize> FrameBuffer for StaticFrameBuffer<W, H, C> {
    fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(&self.data[0][0][0] as *const u8, H * W * C) }
    }
}

impl<const W: usize, const H: usize, const C: usize> FrameBufferMut for StaticFrameBuffer<W, H, C> {
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

impl FrameSize for SizedFrameBuffer {
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

impl FrameBuffer for SizedFrameBuffer {
    fn as_bytes(&self) -> &[u8] {
        self.data.as_ref()
    }
}

impl FrameBufferMut for SizedFrameBuffer {
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.data.as_mut()
    }
}

impl<B: FrameSize + ?Sized> FrameSize for Box<B> {
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

impl<B: FrameBuffer + ?Sized> FrameBuffer for Box<B> {
    fn as_bytes(&self) -> &[u8] {
        (**self).as_bytes()
    }

    fn mask_bytes(&self) -> Option<&[u8]> {
        (**self).mask_bytes()
    }

    fn check_decoder(&self, dec: &impl ImageDecoder) -> Result<()> {
        (**self).check_decoder(dec)
    }
}

impl<B: FrameBufferMut + ?Sized> FrameBufferMut for Box<B> {
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        (**self).as_bytes_mut()
    }
}

pub struct FrameBufferView<'a> {
    data: &'a [u8],
    width: usize,
    height: usize,
    chans: usize,
}

impl<'a> FrameBufferView<'a> {
    pub fn new(size: (usize, usize, usize), data: &'a [u8]) -> Self {
        Self {
            data,
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

impl<'a> FrameBuffer for FrameBufferView<'a> {
    fn as_bytes(&self) -> &[u8] {
        self.data
    }
}
