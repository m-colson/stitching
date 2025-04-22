use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use axum::extract::ws::Message;
use cam_loader::{OwnedWriteBuffer, buf::FrameSize};
use zerocopy::{FromBytes, FromZeros, Immutable, IntoBytes, KnownLayout, little_endian};

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
enum PacketKind {
    Nop = 0,
    UpdateFrame = 2,
    Timing = 4,
}

pub enum RecvPacket {
    Nop,
    Timing(TimingPacket),
}

impl RecvPacket {
    pub fn from_raw(data: &[u8]) -> Option<Self> {
        (data[0] == PacketKind::Nop as _)
            .then_some(Self::Nop)
            .or_else(|| TimingPacket::from_raw(data).map(Self::Timing))
    }
}

pub struct VideoPacket(Arc<[u8]>);

impl VideoPacket {
    #[inline]
    pub fn new(width: usize, height: usize, chans: usize) -> stitch::Result<Self> {
        let mut inner = <[u8]>::new_box_zeroed_with_elems(width * height * chans + 16)
            .expect("failed to create buffer for video packet");
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
                .expect("failed to write packet send time");
        }
    }

    #[inline]
    pub fn to_message(&self) -> tokio::task::JoinHandle<Message> {
        let buf = self.0.clone();
        let w = self.width();
        let h = self.height();
        tokio::task::spawn_blocking(move || {
            let Ok(enc) = qoi::Encoder::new(&buf[16..], w as _, h as _)
                .inspect_err(|err| tracing::error!("failed to create encoder: {err}"))
            else {
                return Message::Binary(vec![0].into());
            };

            // NOTE: the actual size of this buffer is unknown so this is arbitrary.
            let mut out = vec![0; 16 + enc.required_buf_len()];
            out[0..16].copy_from_slice(&buf[0..16]);

            let usage = enc
                .encode_to_buf(&mut out[16..])
                .expect("should only happen when buf is too small, but we ensured it was");

            out.truncate(16 + usage);

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
            .expect("failed to cast bytes to a u16")
            .get() as _
    }

    fn height(&self) -> usize {
        little_endian::U16::ref_from_bytes(&self.0[3..5])
            .expect("failed to cast bytes to a u16")
            .get() as _
    }

    fn chans(&self) -> usize {
        self.0[5] as usize
    }
}

impl OwnedWriteBuffer for VideoPacket {
    type View<'a> = &'a mut [u8];

    fn owned_to_view(&mut self) -> Option<Self::View<'_>> {
        Some(self)
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
            Duration::from_secs_f64(client_millis.max(0.) / 1000.),
            Duration::from_secs_f64(round_trip.max(0.) / 1000.),
        )
    }
}
