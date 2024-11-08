use std::f32::consts::PI;

#[derive(Clone, Copy, Debug)]
pub(super) struct SphereCoord {
    pub r: f32,
    pub theta: f32,
    pub phi: f32,
}

impl SphereCoord {
    pub fn new(r: f32, theta: f32, phi: f32) -> Self {
        Self { r, theta, phi }
    }

    pub fn from_cart(x: f32, y: f32, z: f32) -> Self {
        let r = (x * x + y * y + z * z).sqrt();
        let theta = x.atan2(y);
        let phi = z.atan2((x * x + y * y).sqrt());
        Self { r, theta, phi }
    }

    pub fn to_cart(self) -> (f32, f32, f32) {
        let (x, y) = self.theta.sin_cos();
        let (z, m) = self.phi.sin_cos();

        (self.r * x * m, self.r * y * m, self.r * z)
    }
}

pub(super) fn clamp_pi(v: f32) -> f32 {
    if v < 0. {
        let rots = (-v / (2. * PI)).round();
        v + rots * 2. * PI
    } else {
        let rots = (v / (2. * PI)).round();
        v - rots * 2. * PI
    }
}

pub(super) mod deg_rad {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(v: &f32, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_f32(v.to_degrees())
    }

    pub fn deserialize<'de, D>(d: D) -> Result<f32, D::Error>
    where
        D: Deserializer<'de>,
    {
        f32::deserialize(d).map(f32::to_radians)
    }
}
