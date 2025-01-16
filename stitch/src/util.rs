pub(crate) mod conv_deg_rad {
    use serde::{Deserialize, Deserializer, Serializer};

    #[allow(clippy::trivially_copy_pass_by_ref)]
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
