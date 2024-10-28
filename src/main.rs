use std::time::Instant;

use clap::{Parser, Subcommand};
use image::{GenericImageView, ImageBuffer, Luma};

use stitching::{camera::ProjectionStyle, config, grad, RenderState};

#[cfg(feature = "raylib")]
use std::sync::{Arc, Mutex};

const WIDTH: usize = 1920;
const HEIGHT: usize = 1080;

fn main() {
    let args = Args::parse();
    args.run()
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
                let (state, _watcher) = config::Config::open_state_watch("cams.toml").unwrap();
                render_raylib(state, WIDTH, HEIGHT);
            }
            #[cfg(feature = "gif")]
            ArgCommand::Gif => {
                let state = config::Config::open_state("cams.toml").unwrap();
                render_gif(state, 1280, 720);
            }
            ArgCommand::Png => {
                let state = config::Config::open_state("cams.toml").unwrap();
                render_png(state, 1280, 720);
            }
            ArgCommand::Flat => {
                let state = config::Config::open_state("cams.toml").unwrap();
                render_flat_img(state, WIDTH, WIDTH);
            }
            ArgCommand::Masks { y_thresh, c_thresh } => {
                let cfg = config::Config::open("cams.toml").unwrap();
                for c in cfg.cameras {
                    let config::CameraTypeConfig::Image { path: img_path, .. } = c.ty else {
                        panic!("camera is not an image type");
                    };

                    let img = image::open(&img_path).unwrap();
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
                let cfg = config::Config::open("cams.toml").unwrap();
                for c in cfg.cameras {
                    let config::CameraTypeConfig::Image { path: img_path, .. } = c.ty else {
                        panic!("camera is not an image type");
                    };
                    println!("start {:?}", img_path);
                    let img = image::open(&img_path).unwrap().to_rgb8();
                    let img = grad::guass_filter(&img, 2.5);
                    let img = grad::gradients(&img);

                    img.save(img_path.with_extension("grads.png")).unwrap();
                }
            }
        }
    }
}

#[cfg(feature = "raylib")]
fn render_raylib(state: Arc<Mutex<RenderState>>, width: usize, height: usize) {
    use raylib::{
        ffi,
        prelude::RaylibDraw,
        texture::{self, RaylibTexture2D},
    };

    let (mut rl, thread) = raylib::init().resizable().title("project").build();

    rl.set_target_fps(30);

    let mut img = texture::Image::gen_image_color(
        width as i32,
        height as i32,
        ffi::Color {
            r: 0,
            b: 0,
            g: 0,
            a: 255,
        },
    );
    img.set_format(ffi::PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8);

    let mut txt = rl.load_texture_from_image(&thread, &img).unwrap();
    let mut f_buf = vec![0; width * height * 3];

    let mut last_change = Instant::now();

    while !rl.window_should_close() {
        let mut state = state.lock().unwrap();
        let dt = rl.get_frame_time();

        let changed = check_keys(&rl, &mut state, dt);

        if rl.is_key_pressed(ffi::KeyboardKey::KEY_R) {
            let cs = crate::config::Config::open("cams.toml")
                .unwrap()
                .load_state()
                .unwrap();
            *state = cs;
        }

        if changed || last_change.elapsed().as_millis() > 1000 {
            last_change = Instant::now();

            f_buf.fill(0);

            state
                .proj
                .project_into(width, height, &state.cams, &mut f_buf);

            txt.update_texture(&f_buf);
        }

        let debug_text = format!(
            "az = {:.1} p = {:.1} at {:.2}, {:.2}, {:.2} | {:?}",
            state.proj.azimuth.to_degrees(),
            state.proj.pitch.to_degrees(),
            state.proj.x,
            state.proj.y,
            state.proj.z,
            state.proj.ty,
        );

        drop(state);

        let screen_width = rl.get_screen_width() as f32;
        let screen_height = rl.get_screen_height() as f32;

        let scale = (screen_width / width as f32).min(screen_height / height as f32);

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
                x: ((screen_width - width as f32 * scale) / 2.).max(0.),
                y: ((screen_height - height as f32 * scale) / 2.).max(0.),
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

#[cfg(feature = "raylib")]
fn check_keys(rl: &raylib::RaylibHandle, state: &mut RenderState, dt: f32) -> bool {
    use raylib::ffi;
    use std::f32::consts::PI;

    let mut changed = false;

    if rl.is_key_down(ffi::KeyboardKey::KEY_UP) && state.proj.pitch > -PI / 2. + 1e-1 {
        state.proj.pitch += dt;
        changed = true;
    }
    if rl.is_key_down(ffi::KeyboardKey::KEY_DOWN) && state.proj.pitch < PI / 2. - 1e-1 {
        state.proj.pitch -= dt;
        changed = true;
    }
    if rl.is_key_down(ffi::KeyboardKey::KEY_LEFT) {
        state.proj.azimuth -= dt * 1.5;
        changed = true;
    }
    if rl.is_key_down(ffi::KeyboardKey::KEY_RIGHT) {
        state.proj.azimuth += dt * 1.5;
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
        let az = state.proj.azimuth;
        state.proj.x += (move_forw * az.sin() + move_lat * az.cos()) * 2.0;
        state.proj.y += (move_forw * az.cos() + move_lat * az.sin()) * 2.0;
        state.proj.z += move_up * 2.0;
    }

    changed
}

#[cfg(feature = "gif")]
fn render_gif(mut state: RenderState, width: usize, height: usize) {
    use std::fs;

    fn time_op<T>(name: &str, f: impl FnOnce() -> T) -> T {
        let start = Instant::now();
        let out = f();
        println!("{name} took {} us", start.elapsed().as_micros());

        out
    }

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
        state.proj.azimuth = (r as f32).to_radians();
        time_op(&format!("project {r}"), || {
            state
                .proj
                .project_into(width, height, &state.cams, &mut frame_buf)
        });

        let frame = time_op(&format!("convert {r}"), || {
            gif::Frame::from_rgb_speed(width as u16, height as u16, &frame_buf, 10)
        });

        time_op(&format!("write {r}"), || enc.write_frame(&frame).unwrap());

        frame_buf.fill(0);
    }
}

fn render_png(state: RenderState, width: usize, height: usize) {
    let mut frame_buf = vec![0u8; width * height * 3];
    state
        .proj
        .project_into(width, height, &state.cams, &mut frame_buf);

    image::ImageBuffer::<image::Rgb<_>, _>::from_vec(width as u32, height as u32, frame_buf)
        .unwrap()
        .save("out.png")
        .unwrap();
}

fn render_flat_img(state: RenderState, width: usize, height: usize) {
    let cams = state.cams.as_slice();
    let cz = 100.;
    let ax = 0.03;
    let ay = (ax / width as f32) * height as f32;

    let out = image::RgbImage::from_par_fn(width as u32, height as u32, |x, y| {
        let x = (x as f32 - (width as f32) / 2.) * ax;
        let y = -(y as f32 - (height as f32) / 2.) * ay;
        let (xi, yi, zi) = ((x * ax).sin() * cz, (y * ay).sin() * cz, 0.);
        let (_, p) = cams
            .iter()
            .filter_map(|c| {
                let (dx, dy, dz) = (xi - c.x, yi - c.y, zi - c.z);
                Some((
                    dx * dx + dy * dy + dz * dz,
                    ProjectionStyle::Hemisphere { radius: 0. }.proj_back(c, (xi, yi, zi))?,
                ))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .unwrap_or_default();

        image::Rgb(p)
    });

    out.save("flat.png").unwrap();
}
