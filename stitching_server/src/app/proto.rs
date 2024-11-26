use std::marker::PhantomData;

use axum::extract::ws::Message;
use stitch::{
    frame::{FrameBuffer, FrameBufferMut, FrameSize},
    proj::ProjStyle,
};
use zerocopy::{FromBytes, FromZeros, IntoBytes};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
enum PacketKind {
    Nop = 0,
    SettingsSync = 1,
    UpdateFrame = 2,
    UpdateBounds = 3,
}

#[allow(dead_code)]
pub enum Packet {
    Nop,
    SettingsSync(SettingsPacket),
    UpdateFrame(VideoPacket),
}

impl Packet {
    pub fn from_raw(data: &[u8]) -> Option<Self> {
        (data[0] == PacketKind::Nop as _)
            .then_some(Self::Nop)
            .or_else(|| SettingsPacket::from_raw(data).map(Self::SettingsSync))
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SettingsPacket {
    _kind: PacketKind,
    view_type: u8,
}

impl SettingsPacket {
    #[allow(dead_code)]
    #[inline]
    pub fn new(style: ProjStyle) -> Self {
        Self {
            _kind: PacketKind::SettingsSync,
            view_type: match style {
                ProjStyle::RawCamera(n) => n as _,
                ProjStyle::Hemisphere { .. } => 255,
            },
        }
    }

    #[inline]
    pub fn from_raw(data: &[u8]) -> Option<Self> {
        (data[0] == PacketKind::SettingsSync as _).then_some(Self {
            _kind: PacketKind::SettingsSync,
            view_type: data[1],
        })
    }

    #[inline]
    pub fn view_type(self, radius: f32) -> ProjStyle {
        match self.view_type {
            255 => ProjStyle::Hemisphere { radius },
            n => ProjStyle::RawCamera(n as _),
        }
    }
}

pub struct VideoPacket<O: zerocopy::ByteOrder = zerocopy::LittleEndian>(Box<[u8]>, PhantomData<O>);

impl<O: zerocopy::ByteOrder> VideoPacket<O> {
    #[inline]
    pub fn new(width: usize, height: usize, chans: usize) -> Self {
        let mut inner = <[u8]>::new_box_zeroed_with_elems(width * height * chans + 8).unwrap();
        inner[0] = PacketKind::UpdateFrame as _;
        zerocopy::U16::<O>::new(width as u16)
            .write_to(&mut inner[1..3])
            .unwrap();
        zerocopy::U16::<O>::new(height as u16)
            .write_to(&mut inner[3..5])
            .unwrap();
        inner[5] = chans as u8;

        Self(inner, PhantomData)
    }

    #[inline]
    pub fn take_message(&mut self) -> Message {
        let new_buf = Self::new(self.width(), self.height(), self.chans()).0;
        let old_buf = std::mem::replace(&mut self.0, new_buf);
        Message::Binary(old_buf.into_vec())
    }
}

impl<O: zerocopy::ByteOrder> FrameSize for VideoPacket<O> {
    fn width(&self) -> usize {
        zerocopy::U16::<O>::ref_from_bytes(&self.0[1..3])
            .unwrap()
            .get() as _
    }

    fn height(&self) -> usize {
        zerocopy::U16::<O>::ref_from_bytes(&self.0[3..5])
            .unwrap()
            .get() as _
    }

    fn chans(&self) -> usize {
        self.0[5] as usize
    }
}

impl<O: zerocopy::ByteOrder> FrameBuffer for VideoPacket<O> {
    fn as_bytes(&self) -> &[u8] {
        &self.0[8..]
    }
}

impl<O: zerocopy::ByteOrder> FrameBufferMut for VideoPacket<O> {
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.0[8..]
    }
}
