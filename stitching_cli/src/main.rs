use std::time::Instant;

use clap::{Parser, Subcommand};
use image::{GenericImageView, ImageBuffer, Luma};

use stitch::{
    camera::ImageSpec,
    config::Config,
    frame::{FrameBuffer, FrameBufferMut, StaticFrameBuffer},
    grad,
    proj::{CpuProjector, UnitProjector},
    RenderState,
};

#[cfg(feature = "raylib")]
use std::sync::{Arc, Mutex};

const WIDTH: usize = 1920;
const HEIGHT: usize = 1080;

pub fn main() {
    let args = Args::parse();

    // hack to get around small default stack size
    std::thread::Builder::new()
        .stack_size(16 * (10 << 20))
        .name("stitcher".to_string())
        .spawn(move || args.run())
        .unwrap()
        .join()
        .unwrap();
}

#[derive(Clone, Debug, Parser)]
pub struct Args {
    #[clap(subcommand)]
    pub cmd: ArgCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ArgCommand {
    #[cfg(feature = "raylib")]
    Window,
    #[cfg(feature = "gif")]
    Gif,
    Png,
    Flat,
    Masks {
        #[clap(long = "yt", default_value_t = 115)]
        y_thresh: i32,
        #[clap(long = "ct", default_value_t = 200.)]
        c_thresh: f32,
    },
    Grads,
}

impl Args {
    pub fn run(&self) {
        match self.cmd {
            #[cfg(feature = "raylib")]
            ArgCommand::Window => {
                let (state, _watcher) =
                    Config::open_state_watch("cams.toml", WIDTH, HEIGHT).unwrap();
                render_raylib::<WIDTH, HEIGHT>(state);
            }
            #[cfg(feature = "gif")]
            ArgCommand::Gif => {
                let state = Config::open_state("cams.toml", WIDTH, HEIGHT).unwrap();
                render_gif::<StaticFrameBuffer<1280, 720>>(state);
            }
            ArgCommand::Png => {
                let state = Config::open_state("cams.toml", WIDTH, HEIGHT).unwrap();
                render_png::<StaticFrameBuffer<1280, 720>>(state);
            }
            ArgCommand::Flat => {
                let state = Config::open_state("cams.toml", WIDTH, WIDTH).unwrap();
                render_flat_img::<StaticFrameBuffer<WIDTH, WIDTH>>(state);
            }
            ArgCommand::Masks { y_thresh, c_thresh } => {
                let cfg = Config::open("cams.toml").unwrap();
                for c in cfg.cameras {
                    let ImageSpec { path: img_path, .. } = &c.meta;

                    let img = image::open(img_path).unwrap();
                    let out_img = ImageBuffer::from_par_fn(img.width(), img.height(), |x, y| {
                        let image::Rgba(p) = img.get_pixel(x, y);
                        let p = p.map(|v| v as i32);

                        let dg = [(0, 1), (1, 2), (2, 0)]
                            .into_iter()
                            .map(|(a, b)| (p[a] - p[b]).pow(2))
                            .sum::<i32>() as f32;

                        let y = (p[0] + p[1] + p[2]) / 3;
                        Luma::from([if (y_thresh - y) as f32 * dg < c_thresh {
                            0u8
                        } else {
                            255
                        }])
                    });

                    out_img.save(img_path.with_extension("mask.png")).unwrap();
                }
            }
            ArgCommand::Grads => {
                let cfg = Config::open("cams.toml").unwrap();
                for c in cfg.cameras {
                    let ImageSpec { path: img_path, .. } = &c.meta;

                    let img = image::open(img_path).unwrap().to_rgb8();
                    let img = grad::guass_filter(&img, 2.5);
                    let img = grad::gradients(&img);

                    img.save(img_path.with_extension("grads.png")).unwrap();
                }
            }
        }
    }
}

#[cfg(feature = "raylib")]
fn render_raylib<const W: usize, const H: usize>(
    state: Arc<Mutex<RenderState<StaticFrameBuffer<W, H>>>>,
) {
    use raylib::{
        ffi,
        prelude::RaylibDraw,
        texture::{self, RaylibTexture2D},
    };
    use stitch::proj::CpuProjector;

    let (mut rl, thread) = raylib::init().resizable().title("project").build();

    rl.set_target_fps(30);

    let mut img = texture::Image::gen_image_color(
        W as i32,
        H as i32,
        ffi::Color {
            r: 0,
            b: 0,
            g: 0,
            a: 255,
        },
    );
    img.set_format(ffi::PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8);

    let mut txt = rl.load_texture_from_image(&thread, &img).unwrap();
    let mut last_change = Instant::now();

    while !rl.window_should_close() {
        let mut state = state.lock().unwrap();
        let dt = rl.get_frame_time();

        let changed = check_keys(&rl, &mut state.proj.spec, dt);

        if rl.is_key_pressed(ffi::KeyboardKey::KEY_R) {
            let cs = crate::Config::open("cams.toml")
                .unwrap()
                .load_state(W, H)
                .unwrap();
            *state = cs;
        }

        if changed || last_change.elapsed().as_millis() > 1000 {
            last_change = Instant::now();

            state.update_proj(&CpuProjector::none());

            txt.update_texture(state.proj.buf.as_bytes());
        }

        let debug_text = format!(
            "az = {:.1} p = {:.1} at {:.2}, {:.2}, {:.2} | {:?}",
            state.proj.spec.azimuth.to_degrees(),
            state.proj.spec.pitch.to_degrees(),
            state.proj.spec.x,
            state.proj.spec.y,
            state.proj.spec.z,
            state.proj.meta,
        );

        drop(state);

        let screen_width = rl.get_screen_width() as f32;
        let screen_height = rl.get_screen_height() as f32;

        let scale = (screen_width / W as f32).min(screen_height / H as f32);

        let mut d = rl.begin_drawing(&thread);

        d.clear_background(ffi::Color {
            r: 10,
            b: 10,
            g: 10,
            a: 255,
        });

        d.draw_texture_ex(
            &txt,
            ffi::Vector2 {
                x: ((screen_width - W as f32 * scale) / 2.).max(0.),
                y: ((screen_height - H as f32 * scale) / 2.).max(0.),
            },
            0.,
            scale,
            ffi::Color {
                r: 255,
                b: 255,
                g: 255,
                a: 255,
            },
        );

        d.draw_fps(10, 10);
        d.draw_text(
            &debug_text,
            10,
            screen_height as i32 - 30,
            14,
            ffi::Color {
                r: 255,
                b: 255,
                g: 255,
                a: 255,
            },
        );
    }
}

// #[cfg(feature = "raylib")]
// async fn render_live_raylib<const W: usize, const H: usize>(
//     state: Arc<Mutex<RenderState<StaticFrameBuffer<W, H>, stitch::LiveBuffer, stitch::LiveSpec>>>,
// ) {
//     use raylib::{
//         ffi,
//         prelude::RaylibDraw,
//         texture::{self, RaylibTexture2D},
//     };

//     let (mut rl, thread) = raylib::init().resizable().title("project").build();

//     rl.set_target_fps(30);

//     let mut img = texture::Image::gen_image_color(
//         W as i32,
//         H as i32,
//         ffi::Color {
//             r: 0,
//             b: 0,
//             g: 0,
//             a: 255,
//         },
//     );
//     img.set_format(ffi::PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8);

//     let mut txt = rl.load_texture_from_image(&thread, &img).unwrap();
//     let mut last_change = Instant::now();

//     while !rl.window_should_close() {
//         let mut state = state.lock().unwrap();
//         let dt = rl.get_frame_time();

//         let changed = check_keys(&rl, &mut state.proj.spec, dt);

//         if rl.is_key_pressed(ffi::KeyboardKey::KEY_R) {
//             let cs = stitch::Config::<stitch::LiveSpec>::open_live("cams.toml")
//                 .unwrap()
//                 .load_state(W, H)
//                 .await
//                 .unwrap();
//             *state = cs;
//         }

//         if changed || last_change.elapsed().as_millis() > 1000 {
//             last_change = Instant::now();

//             state.update_proj_async().await;

//             txt.update_texture(state.proj.buf.as_bytes());
//         }

//         let debug_text = format!(
//             "az = {:.1} p = {:.1} at {:.2}, {:.2}, {:.2} | {:?}",
//             state.proj.spec.azimuth.to_degrees(),
//             state.proj.spec.pitch.to_degrees(),
//             state.proj.spec.x,
//             state.proj.spec.y,
//             state.proj.spec.z,
//             state.proj.ty,
//         );

//         drop(state);

//         let screen_width = rl.get_screen_width() as f32;
//         let screen_height = rl.get_screen_height() as f32;

//         let scale = (screen_width / W as f32).min(screen_height / H as f32);

//         let mut d = rl.begin_drawing(&thread);

//         d.clear_background(ffi::Color {
//             r: 10,
//             b: 10,
//             g: 10,
//             a: 255,
//         });

//         d.draw_texture_ex(
//             &txt,
//             ffi::Vector2 {
//                 x: ((screen_width - W as f32 * scale) / 2.).max(0.),
//                 y: ((screen_height - H as f32 * scale) / 2.).max(0.),
//             },
//             0.,
//             scale,
//             ffi::Color {
//                 r: 255,
//                 b: 255,
//                 g: 255,
//                 a: 255,
//             },
//         );

//         d.draw_fps(10, 10);
//         d.draw_text(
//             &debug_text,
//             10,
//             screen_height as i32 - 30,
//             14,
//             ffi::Color {
//                 r: 255,
//                 b: 255,
//                 g: 255,
//                 a: 255,
//             },
//         );
//     }
// }

#[cfg(feature = "raylib")]
fn check_keys(rl: &raylib::RaylibHandle, spec: &mut stitch::camera::CameraSpec, dt: f32) -> bool {
    use raylib::ffi;
    use std::f32::consts::PI;

    let mut changed = false;

    if rl.is_key_down(ffi::KeyboardKey::KEY_UP) && spec.pitch < PI / 2. - 1e-1 {
        spec.pitch += dt;
        changed = true;
    }
    if rl.is_key_down(ffi::KeyboardKey::KEY_DOWN) && spec.pitch > -PI / 2. + 1e-1 {
        spec.pitch -= dt;
        changed = true;
    }
    if rl.is_key_down(ffi::KeyboardKey::KEY_LEFT) {
        spec.azimuth -= dt * 1.5;
        changed = true;
    }
    if rl.is_key_down(ffi::KeyboardKey::KEY_RIGHT) {
        spec.azimuth += dt * 1.5;
        changed = true;
    }

    let mut move_forw = 0.;
    if rl.is_key_down(ffi::KeyboardKey::KEY_W) {
        move_forw += dt;
        changed = true;
    }
    if rl.is_key_down(ffi::KeyboardKey::KEY_S) {
        move_forw -= dt;
        changed = true;
    }

    let mut move_lat = 0.;
    if rl.is_key_down(ffi::KeyboardKey::KEY_A) {
        move_lat -= dt;
        changed = true;
    }
    if rl.is_key_down(ffi::KeyboardKey::KEY_D) {
        move_lat += dt;
        changed = true;
    }

    let mut move_up = 0.;
    if rl.is_key_down(ffi::KeyboardKey::KEY_SPACE) {
        move_up += dt;
        changed = true;
    }
    if rl.is_key_down(ffi::KeyboardKey::KEY_LEFT_CONTROL) {
        move_up -= dt;
        changed = true;
    }

    if move_lat.abs() > 1e-3 || move_forw.abs() > 1e-3 || move_up.abs() > 1e-3 {
        let az = spec.azimuth;
        spec.x += (move_forw * az.sin() + move_lat * az.cos()) * 2.0;
        spec.y += (move_forw * az.cos() + move_lat * az.sin()) * 2.0;
        spec.z += move_up * 2.0;
    }

    changed
}

#[cfg(feature = "gif")]
fn render_gif<P: FrameBufferMut + Sync>(mut state: RenderState<P>) {
    use std::fs;

    fn time_op<T>(name: &str, f: impl FnOnce() -> T) -> T {
        let start = Instant::now();
        let out = f();
        println!("{name} took {} us", start.elapsed().as_micros());

        out
    }

    let width = state.proj.width();
    let height = state.proj.height();

    let out_file = fs::File::create("out.gif").unwrap();

    let mut enc = gif::Encoder::new(out_file, width as u16, height as u16, &[]).unwrap();
    enc.set_repeat(gif::Repeat::Infinite).unwrap();

    enc.write_extension(gif::ExtensionData::new_control_ext(
        1,
        gif::DisposalMethod::Any,
        false,
        None,
    ))
    .unwrap();

    let mut frame_buf = vec![0u8; width * height * 3];
    for r in (0..360).step_by(1) {
        state.proj.spec.azimuth = (r as f32).to_radians();
        time_op(&format!("project {r}"), || {
            state
                .proj
                .load_projection(&CpuProjector::none(), &state.cams)
        });

        let frame = time_op(&format!("convert {r}"), || {
            gif::Frame::from_rgb_speed(width as u16, height as u16, &frame_buf, 10)
        });

        time_op(&format!("write {r}"), || enc.write_frame(&frame).unwrap());

        frame_buf.fill(0);
    }
}

fn render_png<P: FrameBufferMut + Sync>(mut state: RenderState<P>) {
    state
        .proj
        .load_projection(&CpuProjector::none(), &state.cams);

    image::ImageBuffer::<image::Rgb<_>, _>::from_raw(
        state.proj.width() as u32,
        state.proj.height() as u32,
        state.proj.buf.as_bytes(),
    )
    .unwrap()
    .save("out.png")
    .unwrap();
}

fn render_flat_img<P: FrameBuffer + Sync>(state: RenderState<P>) {
    let width = state.proj.width();
    let height = state.proj.height();

    let cams = state.cams.as_slice();
    let cz = 100.;
    let ax = 0.03;
    let ay = (ax / width as f32) * height as f32;

    let out = image::RgbImage::from_par_fn(width as u32, height as u32, |x, y| {
        let x = (x as f32 - (width as f32) / 2.) * ax;
        let y = -(y as f32 - (height as f32) / 2.) * ay;
        let int_p @ (xi, yi, zi) = ((x * ax).sin() * cz, (y * ay).sin() * cz, 0.);
        let (_, p) = cams
            .iter()
            .filter_map(|c| {
                let (dx, dy, dz) = (xi - c.spec.x, yi - c.spec.y, zi - c.spec.z);
                let (cx, cy) = CpuProjector::none().world_to_img_space(c.spec, int_p);
                c.at(cx, cy).map(|p| (dx * dx + dy * dy + dz * dz, p))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .unwrap_or((0., &[0; 3]));

        image::Rgb(p[..3].try_into().unwrap())
    });

    out.save("flat.png").unwrap();
}
