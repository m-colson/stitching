use std::f32::consts::PI;

use image::ImageBuffer;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

mod edges;
use edges::*;

pub fn gradients(
    img: &ImageBuffer<image::Rgb<u8>, Vec<u8>>,
) -> ImageBuffer<image::Rgb<u8>, Vec<u8>> {
    const BLOCK_SIZE: usize = 4;

    let width = img.width() as usize;
    let height = img.height() as usize;

    let lines = (0..(width / BLOCK_SIZE) * (height / BLOCK_SIZE))
        .into_par_iter()
        .flat_map(|off| {
            let x = (off * BLOCK_SIZE) % width;
            let y = ((off * BLOCK_SIZE) / width) * BLOCK_SIZE;

            let (gx, gy) = (0..BLOCK_SIZE)
                .flat_map(|block_y| {
                    (0..BLOCK_SIZE).map(move |block_x| {
                        let x = x + block_x;
                        let y = y + block_y;

                        let left = x.saturating_sub(1);
                        let right = (x + 1).min(width - 1);

                        let top = y.saturating_sub(1);
                        let bot = (y + 1).min(height - 1);

                        let conv = [
                            (left, top, 1., 1.),
                            (x, top, 0., 2.),
                            (right, top, -1., 1.),
                            (left, y, 2., 0.),
                            // skip x y
                            (right, y, -2., 0.),
                            (left, bot, 1., -1.),
                            (x, bot, 0., -2.),
                            (right, bot, -1., -1.),
                        ];

                        let mut gx = [0., 0., 0.];
                        let mut gy = [0., 0., 0.];
                        for (fx, fy, wgx, wgy) in conv {
                            let v = img.get_pixel(fx as u32, fy as u32);

                            if wgx != 0. {
                                for (i, c) in v.0.iter().enumerate() {
                                    gx[i] += *c as f32 * wgx;
                                }
                            }
                            if wgy != 0. {
                                for (i, c) in v.0.iter().enumerate() {
                                    gy[i] += *c as f32 * wgy;
                                }
                            }
                        }

                        (gx, gy)
                    })
                })
                .max_by_key(|(gx, gy)| {
                    (gx[0].powi(2) + gy[0].powi(2))
                        .max(gx[1].powi(2) + gy[1].powi(2))
                        .max(gx[2].powi(2) + gy[2].powi(2)) as usize
                })
                .unwrap();

            let line = EdgeLine::from_grads(
                (x as f32 - width as f32 / 2.) / BLOCK_SIZE as f32,
                (y as f32 - height as f32 / 2.) / BLOCK_SIZE as f32,
                gx,
                gy,
            );

            line.conf.iter().all(|l| *l > 0.7).then_some(line)
        })
        .collect::<EdgeSet<3>>();

    let width = width / BLOCK_SIZE;
    let height = height / BLOCK_SIZE;

    let mut pixels = vec![0.; width * height * 3];
    for l in lines {
        let (mut start_x, mut start_y) = l.to_cart();
        start_x += width as f32 / 2.;
        start_y += height as f32 / 2.;
        let (mut step_y, mut step_x) = (l.ang + PI / 2.).sin_cos();

        let step_divider = step_y.abs().max(step_x.abs());
        step_x /= step_divider;
        step_y /= step_divider;

        let mut x = start_x;
        let mut y = start_y;
        while x >= 0. && x < width as f32 && y >= 0. && y < height as f32 {
            let off = (x as usize + y as usize * width) * 3;
            for (i, c) in l.conf.iter().enumerate() {
                pixels[off + i] += c;
            }

            x += step_x;
            y += step_y;
        }

        x = start_x - step_x;
        y = start_y - step_y;
        while x >= 0. && x < width as f32 && y >= 0. && y < height as f32 {
            let off = (x as usize + y as usize * width) * 3;
            for (i, c) in l.conf.iter().enumerate() {
                pixels[off + i] += c;
            }

            x -= step_x;
            y -= step_y;
        }
    }

    let max_m = *pixels
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    ImageBuffer::from_vec(
        width as u32,
        height as u32,
        pixels
            .into_iter()
            .map(|m| ((m / max_m).powi(2) * 255.).clamp(0., 255.) as u8)
            .collect(),
    )
    .unwrap()
}

pub fn guass_filter(
    img: &ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    sigma: f32,
) -> ImageBuffer<image::Rgb<u8>, Vec<u8>> {
    let width = img.width();
    let height = img.height();

    let sig_2sq = 2.0 * sigma * sigma;

    let filter = (0..25)
        .map(|off: i32| {
            let dx = off % 5 - 2;
            let dy = off / 5 - 2;

            (
                dx,
                dy,
                std::f32::consts::FRAC_1_PI * ((dx * dx + dy * dy) as f32 / -sig_2sq).exp() / sigma,
            )
        })
        .collect::<Vec<_>>();

    ImageBuffer::<image::Rgb<u8>, _>::from_vec(
        width,
        height,
        (0..width * height)
            .into_par_iter()
            .flat_map(|off| {
                let x = off % width;
                let y = off / width;

                let vs = filter
                    .iter()
                    .map(|(dx, dy, w)| {
                        let fx = (x as i32 + dx).clamp(0, width as i32 - 1) as u32;
                        let fy = (y as i32 + dy).clamp(0, height as i32 - 1) as u32;
                        (img.get_pixel(fx, fy).0, *w)
                    })
                    .map(|(p, w)| p.map(|c| c as f32 * w))
                    .reduce(|mut acc, v| {
                        for (i, c) in v.into_iter().enumerate() {
                            acc[i] += c;
                        }
                        acc
                    })
                    .unwrap();

                vs.map(|c| c.clamp(0.0, 255.0) as u8)
            })
            .collect(),
    )
    .unwrap()
}
