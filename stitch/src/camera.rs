//! This module contains the types and function used for storing information
//! about the physical properties of a camera.

use std::future::Future;

use cam_loader::{FrameSize, Loader, OwnedWriteBuffer};
use serde::{Deserialize, Serialize};

use crate::util::conv_deg_rad;

/// Stores [`ViewParams`] and (usually image) data together.
#[derive(Clone, Debug)]
pub struct Camera<T> {
    /// view parameters of the camera
    pub view: ViewParams,
    /// some additional (likely image) data that camera has.
    pub data: T,
}

impl<T> Camera<T> {
    /// Creates a new camera with the provided [`ViewParams`] and data.
    #[inline]
    pub const fn new(view: ViewParams, data: T) -> Self {
        Self { view, data }
    }

    /// Takes a reference to the camera and replaces the inner data with the
    /// results of the callback.
    #[inline]
    pub fn with_map<N>(&self, f: impl FnOnce(&T) -> N) -> Camera<N> {
        Camera {
            view: self.view,
            data: f(&self.data),
        }
    }

    /// Takes a reference to the camera and replaces the inner data with the
    /// results of the callback, asynchronously.
    #[allow(clippy::future_not_send)]
    #[inline]
    pub async fn with_map_fut<'a, N, Fut: Future<Output = N>>(
        &'a self,
        f: impl FnOnce(&'a T) -> Fut,
    ) -> Camera<N> {
        Camera {
            view: self.view,
            data: f(&self.data).await,
        }
    }
}

impl<T: FrameSize> Camera<T> {
    /// Returns the width of the camera's data.
    #[inline]
    pub fn width(&self) -> usize {
        self.data.width()
    }
    /// Returns the height of the camera's data.
    #[inline]
    pub fn height(&self) -> usize {
        self.data.height()
    }
    /// Returns the channel count of the camera's data.
    #[inline]
    pub fn chans(&self) -> usize {
        self.data.chans()
    }
}

/// Contains settings for a camera.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Config<K> {
    /// camera's view parameters
    #[serde(flatten)]
    pub view: ViewParams,
    /// addition information for the camera.
    #[serde(flatten)]
    pub meta: K,
}

impl<K> Config<K> {
    /// Updates config's view with [ViewParams::with_dims].
    #[must_use]
    pub fn with_dims(mut self, w: f32, h: f32) -> Self {
        self.view = self.view.with_dims(w, h);
        self
    }

    /// Creates a [`Camera`] with the view parameters of `self` and the provided buffer.
    pub fn with_buffer<B>(self, buf: B) -> Camera<B> {
        Camera::new(self.view, buf)
    }
}

impl<T> Config<T> {
    /// Creates a [`Camera`] with the view parameters of `self` and data from a new [`Loader`] based on the config's metadata.
    pub fn load<B: OwnedWriteBuffer + 'static>(
        self,
    ) -> std::result::Result<Camera<Loader<B>>, T::Error>
    where
        T: TryInto<Loader<B>>,
    {
        let buf: Loader<_> = self.meta.try_into()?;
        let (w, h, _) = buf.frame_size();

        Ok(Camera::new(self.view.with_dims(w as f32, h as f32), buf))
    }

    /// Creates a /// Creates a [`Camera`] with the view parameters of `self` and the provided loader.
    pub fn load_with<B: OwnedWriteBuffer + 'static>(&self, buf: Loader<B>) -> Camera<Loader<B>> {
        let (w, h, _) = buf.frame_size();
        Camera::new(self.view.with_dims(w as f32, h as f32), buf)
    }
}

/// Stores the position, angle, sensor, and lens of a camera.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ViewParams {
    /// XYZ coordinate of the camera
    pub pos: [f32; 3],
    /// Pitch angle of the camera in radians. Automatically (de)serializes from degrees.
    #[serde(with = "conv_deg_rad")]
    pub pitch: f32,
    /// Azimuth angle of the camera in radians. Automatically (de)serializes from degrees.
    #[serde(with = "conv_deg_rad")]
    pub azimuth: f32,
    /// Roll angle of the camera in radians. Automatically (de)serializes from degrees.
    #[serde(default, with = "conv_deg_rad")]
    pub roll: f32,
    /// Sensor parameters of the camera.
    pub sensor: SensorParams,
    /// Kind of lens on the camera.
    #[serde(default)]
    pub lens: LensKind,
}

impl ViewParams {
    /// Sets the width and height of the camera. Specifically, updates the sensor fov using [`Fov::with_dims`].
    #[inline]
    pub fn set_dims(&mut self, w: f32, h: f32) {
        self.sensor.fov = self.sensor.fov.with_dims(self.lens, w, h);
    }

    /// See [`ViewParams::set_dims`].
    #[must_use]
    #[inline]
    pub fn with_dims(mut self, w: f32, h: f32) -> Self {
        self.set_dims(w, h);
        self
    }

    /// Calculates the equivalent focal distance for the camera using [`Fov::focal_dist`].
    #[must_use]
    #[inline]
    pub fn focal_dist(&self, width: f32, height: f32) -> f32 {
        self.sensor.fov.focal_dist(self.lens, width, height)
    }
}

/// Stores information about a camera's sensor and related fov.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SensorParams {
    /// The number of pixels [X, Y] that the sensor's center is offset from the
    /// optical center.
    #[serde(default)]
    pub img_off: [f32; 2],
    /// Fov created by the sensor and lens configuration.
    pub fov: Fov,
}

/// Represents the field of view of a camera based on either
/// the width (horizontal), height (vertical), diagonal FOV, or equivalent focal distance.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Fov {
    /// Horizontal FOV in degrees
    W(f32),
    /// Vertical FOV in degrees
    H(f32),
    /// Diagonal FOV in degrees
    D(f32),
    /// Equivalent focal distance, that is, the focal distance of the camera
    /// in units if the image's diagonal distance was 1 unit.
    FocalDist(f32),
}

impl Fov {
    /// Uses the provided lens kind, width, and height to construct a new FOV
    /// based on the focal dist. See [`Fov::focal_dist`].
    #[must_use]
    #[inline]
    pub fn with_dims(self, lens: LensKind, width: f32, height: f32) -> Self {
        Self::FocalDist(self.focal_dist(lens, width, height))
    }

    /// Calculates the equivalent focal distance based on `self`'s fov, the lens
    /// kind, width, and height. If `self` is a degree measurement, it will
    /// calculate the radius distance based on the width and height ratios.
    /// See [`LensKind::focal_from_rad_ang`].
    #[must_use]
    #[inline]
    pub fn focal_dist(self, lens: LensKind, width: f32, height: f32) -> f32 {
        let (r, ang) = match self {
            Self::W(f) => (width / width.hypot(height), f.to_radians() / 2.),
            Self::H(f) => (height / width.hypot(height), f.to_radians() / 2.),
            Self::D(f) => (1., f.to_radians() / 2.),
            Self::FocalDist(d) => return d,
        };

        lens.focal_from_rad_ang(r, ang)
    }

    /// Returns the focal dist if `self` is [`Fov::FocalDist`], otherwise returns none.
    /// This is used in places where the focal distance is needed and the image
    /// width and height is not known.
    #[must_use]
    #[inline]
    pub const fn assume_focal_dist(self) -> Option<f32> {
        if let Self::FocalDist(d) = self {
            Some(d)
        } else {
            None
        }
    }
}

/// The kind of lens the camera has, taken from [this wikipedia article](https://en.wikipedia.org/wiki/Fisheye_lens#Mapping_function).
///
/// NOTE: this order is expected within the rendering shader.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
#[allow(missing_docs)]
pub enum LensKind {
    #[default]
    Rectilinear = 0,
    Equidistant = 1,
    Equisolid = 2,
}

impl LensKind {
    #[must_use]
    #[inline]
    /// Calculates focal distance based on the lens kind, image radius and the
    /// corresponding optical angle. See [`LensKind`].
    pub fn focal_from_rad_ang(self, r: f32, ang: f32) -> f32 {
        match self {
            Self::Rectilinear => r / ang.tan(),
            Self::Equidistant => r / ang,
            Self::Equisolid => r / (2. * (ang / 2.).sin()),
        }
    }
}
