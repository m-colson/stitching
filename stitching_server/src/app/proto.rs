use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::OnceLock,
    time::{Duration, Instant},
};

use axum::extract::ws::Message;
use stitch::{buf::FrameSize, proj::ProjectionStyle};
use zerocopy::{FromBytes, FromZeros, Immutable, IntoBytes, KnownLayout};

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
enum PacketKind {
    Nop = 0,
    SettingsSync = 1,
    UpdateFrame = 2,
    Timing = 4,
}

pub enum RecvPacket {
    Nop,
    SettingsSync(SettingsPacket),
    Timing(TimingPacket),
}

impl RecvPacket {
    pub fn from_raw(data: &[u8]) -> Option<Self> {
        (data[0] == PacketKind::Nop as _)
            .then_some(Self::Nop)
            .or_else(|| SettingsPacket::from_raw(data).map(Self::SettingsSync))
            .or_else(|| TimingPacket::from_raw(data).map(Self::Timing))
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SettingsPacket {
    _kind: PacketKind,
    view_type: u8,
}

impl SettingsPacket {
    // remains for future reference
    // #[inline]
    // pub const fn new(style: ProjectionStyle) -> Self {
    //     Self {
    //         _kind: PacketKind::SettingsSync,
    //         view_type: match style {
    //             ProjectionStyle::RawCamera(n) => n as _,
    //             ProjectionStyle::Hemisphere { .. } => 255,
    //         },
    //     }
    // }

    #[inline]
    pub fn from_raw(data: &[u8]) -> Option<Self> {
        (data[0] == PacketKind::SettingsSync as _).then_some(Self {
            _kind: PacketKind::SettingsSync,
            view_type: data[1],
        })
    }

    #[allow(dead_code)]
    #[inline]
    pub const fn view_type(self) -> ProjectionStyle {
        match self.view_type {
            255 => ProjectionStyle::Flat,
            n => ProjectionStyle::RawCamera(n as _),
        }
    }
}

pub struct VideoPacket<O: zerocopy::ByteOrder = zerocopy::LittleEndian>(Box<[u8]>, PhantomData<O>);

impl<O: zerocopy::ByteOrder> VideoPacket<O> {
    #[inline]
    pub fn new(width: usize, height: usize, chans: usize) -> stitch::Result<Self> {
        let mut inner = <[u8]>::new_box_zeroed_with_elems(width * height * chans + 16).unwrap();
        inner[0] = PacketKind::UpdateFrame as _;
        zerocopy::U16::<O>::new(width.try_into()?)
            .write_to(&mut inner[1..3])
            .unwrap();
        zerocopy::U16::<O>::new(height.try_into()?)
            .write_to(&mut inner[3..5])
            .unwrap();
        inner[5] = chans.try_into()?;

        Ok(Self(inner, PhantomData))
    }

    #[inline]
    pub fn update_time(&mut self) {
        zerocopy::F64::<O>::new(TimingPacket::new_now().server_send)
            .write_to(&mut self.0[8..16])
            .unwrap();
    }

    #[inline]
    pub fn take_message(&mut self) -> Message {
        let new_buf = Self::new(self.width(), self.height(), self.chans())
            .expect("dimension should already be safe if this type exists")
            .0;
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

impl<O: zerocopy::ByteOrder> Deref for VideoPacket<O> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0[16..]
    }
}

impl<O: zerocopy::ByteOrder> DerefMut for VideoPacket<O> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0[16..]
    }
}

#[derive(FromBytes, IntoBytes, Immutable, KnownLayout, Clone, Copy, Debug)]
pub struct TimingPacket {
    _kind: u64,
    pub server_send: f64,
    pub client_recv: f64,
    pub client_send: f64,
}

impl TimingPacket {
    #[inline]
    fn base_instant() -> &'static Instant {
        static START_TIME: OnceLock<Instant> = OnceLock::new();
        START_TIME.get_or_init(Instant::now)
    }

    #[inline]
    pub fn new_now() -> Self {
        let server_send = Self::base_instant().elapsed();
        Self {
            _kind: PacketKind::Timing as _,
            server_send: server_send.as_secs_f64() * 1000.,
            client_recv: f64::NAN,
            client_send: f64::NAN,
        }
    }

    pub fn from_raw(data: &[u8]) -> Option<Self> {
        if data[0] != PacketKind::Timing as _ {
            return None;
        }

        Self::ref_from_bytes(data).ok().copied()
    }

    #[inline]
    pub fn info_now(self) -> (Duration, Duration) {
        let server_recv = Self::base_instant().elapsed().as_secs_f64() * 1000.;

        let client_millis = self.client_send - self.client_recv;
        let round_trip = (server_recv - self.server_send) - client_millis;
        (
            Duration::from_secs_f64(client_millis / 1000.),
            Duration::from_secs_f64(round_trip / 1000.),
        )
    }
}
