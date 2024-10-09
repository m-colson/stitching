use std::time::Instant;

use camera::Camera;

mod camera;
mod config;

#[cfg(feature = "raylib")]
use std::sync::{Arc, Mutex};

const WIDTH: usize = 1280;
const HEIGHT: usize = 720;

fn main() {
    #[cfg(feature = "raylib")]
    {
        let (state, _watcher) = config::Config::open_state_watch("cams.toml").unwrap();
        render_raylib(state, WIDTH, HEIGHT);
    }

    #[cfg(not(feature = "raylib"))]
    {
        let state = config::Config::open_state("cams.toml").unwrap();
        render_gif(state, WIDTH, HEIGHT);
    }
}

#[derive(Clone, Debug)]
pub struct RenderState {
    pub proj: Camera,
    pub cams: Vec<Camera>,
}

impl RenderState {}

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
        state.proj.pitch -= dt;
        changed = true;
    }
    if rl.is_key_down(ffi::KeyboardKey::KEY_DOWN) && state.proj.pitch < PI / 2. - 1e-1 {
        state.proj.pitch += dt;
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
        move_up -= dt;
        changed = true;
    }
    if rl.is_key_down(ffi::KeyboardKey::KEY_LEFT_CONTROL) {
        move_up += dt;
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

#[cfg(not(feature = "raylib"))]
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
    for r in (0..360).step_by(3) {
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
