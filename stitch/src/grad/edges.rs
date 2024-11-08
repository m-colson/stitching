use std::{collections::HashMap, iter::zip};

use rayon::iter::{FromParallelIterator, ParallelIterator};

#[derive(Clone, Copy, Debug)]
pub struct EdgeLine<const N: usize> {
    pub r: f32,
    pub ang: f32,
    pub conf: [f32; N],
}

impl<const N: usize> EdgeLine<N> {
    const ANG_ROUND_FACTOR: f32 = 2.;
    const MAG_1_LAMBDA: f32 = 200.;

    #[inline]
    pub fn from_grads(x: f32, y: f32, gx: [f32; N], gy: [f32; N]) -> Self {
        let mut conf = [0.; N];
        // let mut sum_gx = 0.0;
        // let mut sum_gy = 0.0;

        let (best_gx, best_gy) = zip(gx, gy)
            .enumerate()
            .map(|(i, (gx, gy))| {
                let mag = (gx * gx + gy * gy).sqrt();
                // let c_ang = gy.atan2(gx);

                conf[i] = 1. - (-mag / Self::MAG_1_LAMBDA).exp();
                // sum_gx += c_ang.cos();
                // sum_gy += c_ang.sin();

                (gx, gy)
            })
            .max_by_key(|(gx, gy)| (gx * gx + gy * gy) as usize)
            .unwrap();

        let ang = ((best_gy).atan2(best_gx).to_degrees() * Self::ANG_ROUND_FACTOR)
            .round()
            .to_radians()
            / Self::ANG_ROUND_FACTOR;

        Self {
            r: x * ang.cos() + y * ang.sin(),
            ang,
            conf,
        }
    }

    pub fn to_cart(self) -> (f32, f32) {
        (self.r * self.ang.cos(), self.r * self.ang.sin())
    }

    pub fn total_conf(self) -> f32 {
        self.conf.iter().sum()
    }

    fn bucket(self) -> EdgeBucket {
        // let r = self.r + self.r.signum() * EdgeBucket::COORD_BUCKET_DIV * 2.;
        // let x = r * self.ang.cos() / EdgeBucket::COORD_BUCKET_DIV;
        // let y = r * self.ang.sin() / EdgeBucket::COORD_BUCKET_DIV;
        EdgeBucket {
            r: ((self.r / EdgeBucket::R_BUCKET_DIV)
                + self.r.signum() * EdgeBucket::R_ROUNDUP_FACTOR) as _,
            ang: (self.ang.to_degrees() * EdgeBucket::ANG_BUCKET_MUL) as _,
            // x: x as i32,
            // y: y as i32,
        }
    }

    pub fn bucketize(self) -> Self {
        let b = self.bucket();
        // let x = b.x as f32 * EdgeBucket::COORD_BUCKET_DIV;
        // let y = b.y as f32 * EdgeBucket::COORD_BUCKET_DIV;
        // let r = (x * x + y * y).sqrt() - (EdgeBucket::COORD_BUCKET_DIV * 2.);
        // let ang = (y as f32).atan2(x as f32);
        let r = b.r as f32;
        Self {
            r: (r * EdgeBucket::R_BUCKET_DIV) as _,
            ang: (b.ang as f32 / EdgeBucket::ANG_BUCKET_MUL).to_radians() as _,
            // r,
            // ang,
            conf: self.conf,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EdgeBucket {
    // pub x: i32,
    // pub y: i32,
    pub r: i32,
    pub ang: i32,
}

impl EdgeBucket {
    // pub const COORD_BUCKET_DIV: f32 = 4.;
    pub const R_BUCKET_DIV: f32 = 2.;
    pub const R_ROUNDUP_FACTOR: f32 = 0.5;
    pub const ANG_BUCKET_MUL: f32 = 2.;
}

pub struct EdgeSet<const N: usize> {
    edges: HashMap<EdgeBucket, EdgeLine<N>>,
}

impl<const N: usize> EdgeSet<N> {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.edges.len()
    }
}

impl<const N: usize> FromParallelIterator<EdgeLine<N>> for EdgeSet<N> {
    fn from_par_iter<I>(par_iter: I) -> Self
    where
        I: rayon::prelude::IntoParallelIterator<Item = EdgeLine<N>>,
    {
        let it = par_iter.into_par_iter();
        let lines = it.collect_vec_list();

        let mut out = EdgeSet::new();
        for new_line in lines.into_iter().flatten() {
            let b = new_line.bucket();

            out.edges
                .entry(b)
                .and_modify(|old_line| {
                    // for (i, c) in &mut old_line.conf.iter_mut().enumerate() {
                    //     *c += new_line.conf[i];
                    // }
                    if new_line.total_conf() > old_line.total_conf() {
                        *old_line = new_line;
                    }
                })
                .or_insert(new_line.bucketize());
        }

        out
    }
}

impl<const N: usize> IntoIterator for EdgeSet<N> {
    type Item = EdgeLine<N>;

    type IntoIter = std::collections::hash_map::IntoValues<EdgeBucket, EdgeLine<N>>;

    fn into_iter(self) -> Self::IntoIter {
        self.edges.into_values()
    }
}
