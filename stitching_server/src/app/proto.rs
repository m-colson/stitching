use std::{
    io::{Cursor, Read},
    ops::{Deref, DerefMut},
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use axum::extract::ws::Message;
use stitch::{buf::FrameSize, proj::ProjectionStyle};
use zerocopy::{little_endian, FromBytes, FromZeros, Immutable, IntoBytes, KnownLayout};

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

pub struct VideoPacket(Arc<[u8]>);

impl VideoPacket {
    #[inline]
    pub fn new(width: usize, height: usize, chans: usize) -> stitch::Result<Self> {
        let mut inner = <[u8]>::new_box_zeroed_with_elems(width * height * chans + 16).unwrap();
        inner[0] = PacketKind::UpdateFrame as _;
        little_endian::U16::new(width.try_into()?)
            .write_to(&mut inner[1..3])
            .expect("implementation bug: width of video packet is wrong");
        little_endian::U16::new(height.try_into()?)
            .write_to(&mut inner[3..5])
            .expect("implementation bug: height of video packet is wrong");
        inner[5] = chans.try_into()?;

        Ok(Self(inner.into()))
    }

    #[inline]
    pub fn update_time(&mut self) {
        if let Some(inner) = &mut self.mut_inner_data() {
            little_endian::F64::new(TimingPacket::new_now().server_send)
                .write_to(&mut inner[8..16])
                .unwrap();
        }
    }

    #[inline]
    pub fn to_message(&self) -> tokio::task::JoinHandle<Message> {
        let buf = self.0.clone();
        tokio::task::spawn_blocking(move || {
            let mut out = Vec::new();
            flate2::GzBuilder::new()
                .buf_read(Cursor::new(&buf), flate2::Compression::fast())
                .read_to_end(&mut out)
                .unwrap();

            // tracing::debug!(
            //     "compressed {:?} bytes to {:?} bytes",
            //     self.0.len(),
            //     out.len()
            // );

            Message::Binary(out.into())
        })
    }

    fn mut_inner_data(&mut self) -> Option<&mut [u8]> {
        let buf = Arc::get_mut(&mut self.0);
        match buf {
            Some(buf) => Some(buf),
            None => {
                tracing::error!(
                    "failed to get video packet buffer because another reference to it exists"
                );
                None
            }
        }
    }
}

impl FrameSize for VideoPacket {
    fn width(&self) -> usize {
        little_endian::U16::ref_from_bytes(&self.0[1..3])
            .unwrap()
            .get() as _
    }

    fn height(&self) -> usize {
        little_endian::U16::ref_from_bytes(&self.0[3..5])
            .unwrap()
            .get() as _
    }

    fn chans(&self) -> usize {
        self.0[5] as usize
    }
}

impl Deref for VideoPacket {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0[16..]
    }
}

impl DerefMut for VideoPacket {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mut_inner_data()
            .map_or(&mut [], |inner| &mut inner[16..])
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

        #[repr(align(8))]
        #[derive(Default)]
        struct Data([u8; 32]);

        if size_of::<Data>() != size_of_val(data) {
            return None;
        }

        let mut new_data = Data::default();
        new_data.0.copy_from_slice(data);

        Self::ref_from_bytes(&new_data.0).ok().cloned()
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
