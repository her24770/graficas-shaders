//! Lab de shaders: sistema solar hecho solo con fragment shaders, en un renderer de software propio.
#[allow(dead_code)]
mod math;
mod noise;
mod obj;
mod raster;
mod scene;
mod shaders;

use minifb::{Key, KeyRepeat, ScaleMode, Window, WindowOptions};
use scene::{OrbitCamera, Renderer, View, VIEWS};
use std::time::Instant;

const WIDTH: usize = 960;
const HEIGHT: usize = 540;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--bench") {
        let mut r = Renderer::new(WIDTH, HEIGHT);
        let mut out = vec![0u32; WIDTH * HEIGHT];
        for v in VIEWS {
            let start = Instant::now();
            for i in 0..10 {
                r.render(v, i as f32 * 0.1, &OrbitCamera::for_view(v), &mut out);
            }
            println!("{:<40} {:6.1} ms/cuadro", v.name(), start.elapsed().as_secs_f32() * 100.0);
        }
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--capturas") {
        let dir = args.get(i + 1).cloned().unwrap_or_else(|| "screenshots".into());
        capturas(&dir);
        return;
    }

    let options = WindowOptions { resize: true, scale_mode: ScaleMode::AspectRatioStretch, ..WindowOptions::default() };
    let mut window = Window::new("Lab Shaders", WIDTH, HEIGHT, options).expect("no se pudo crear la ventana");
    window.set_target_fps(60);

    // Dos calidades: completa y rápida (2/3 de la resolución, la ventana la estira).
    let mut renderers = [Renderer::new(WIDTH, HEIGHT), Renderer::new(WIDTH * 2 / 3, HEIGHT * 2 / 3)];
    let mut buffers = [vec![0u32; WIDTH * HEIGHT], vec![0u32; WIDTH * 2 / 3 * HEIGHT * 2 / 3]];
    let mut quality = 0;
    let mut view_idx = 0;
    let mut cam = OrbitCamera::for_view(VIEWS[0]);
    let (mut t, mut speed, mut paused) = (0.0f32, 1.0f32, false);
    let mut last = Instant::now();
    let mut frames = 0u32;
    let mut fps_timer = Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = Instant::now();
        let dt = now.duration_since(last).as_secs_f32().min(0.1);
        last = now;

        // Cambiar de cuerpo: 0-7 o Tab.
        let keys = [Key::Key0, Key::Key1, Key::Key2, Key::Key3, Key::Key4, Key::Key5, Key::Key6, Key::Key7];
        let mut new_idx = view_idx;
        for (i, k) in keys.iter().enumerate() {
            if window.is_key_pressed(*k, KeyRepeat::No) {
                new_idx = i;
            }
        }
        if window.is_key_pressed(Key::Tab, KeyRepeat::No) {
            new_idx = (view_idx + 1) % VIEWS.len();
        }
        if new_idx != view_idx || window.is_key_pressed(Key::R, KeyRepeat::No) {
            view_idx = new_idx;
            cam = OrbitCamera::for_view(VIEWS[view_idx]);
        }

        // Cámara: flechas giran, W/S acercan y alejan.
        if window.is_key_down(Key::Left) || window.is_key_down(Key::A) {
            cam.yaw -= 1.2 * dt;
        }
        if window.is_key_down(Key::Right) || window.is_key_down(Key::D) {
            cam.yaw += 1.2 * dt;
        }
        if window.is_key_down(Key::Up) {
            cam.pitch = (cam.pitch + 1.0 * dt).min(1.5);
        }
        if window.is_key_down(Key::Down) {
            cam.pitch = (cam.pitch - 1.0 * dt).max(-1.5);
        }
        if window.is_key_down(Key::W) {
            cam.distance = (cam.distance * (1.0 - 1.2 * dt)).max(1.3);
        }
        if window.is_key_down(Key::S) {
            cam.distance = (cam.distance * (1.0 + 1.2 * dt)).min(120.0);
        }
        if window.is_key_pressed(Key::Space, KeyRepeat::No) {
            paused = !paused;
        }
        if window.is_key_pressed(Key::Equal, KeyRepeat::Yes) {
            speed = (speed * 1.5).min(20.0);
        }
        if window.is_key_pressed(Key::Minus, KeyRepeat::Yes) {
            speed = (speed / 1.5).max(0.05);
        }
        if window.is_key_pressed(Key::Q, KeyRepeat::No) {
            quality = 1 - quality;
        }
        if !paused {
            t += dt * speed;
        }

        let view = VIEWS[view_idx];
        let (w, h) = renderers[quality].size();
        renderers[quality].render(view, t, &cam, &mut buffers[quality]);
        window.update_with_buffer(&buffers[quality], w, h).expect("no se pudo mostrar el framebuffer");

        frames += 1;
        if fps_timer.elapsed().as_secs_f32() >= 0.5 {
            let fps = frames as f32 / fps_timer.elapsed().as_secs_f32();
            window.set_title(&format!(
                "[{view_idx}] {} | {fps:.0} fps | x{speed:.1}{} | 0-7/Tab: cuerpo  flechas: girar  W/S: zoom  Espacio: pausa  +/-: velocidad  Q: calidad",
                view.name(),
                if paused { " (pausa)" } else { "" }
            ));
            frames = 0;
            fps_timer = Instant::now();
        }
    }
}

/// Guarda una captura PNG de cada vista (con supermuestreo 2x2 para que los bordes se vean suaves).
fn capturas(dir: &str) {
    const SS: usize = 2;
    let (w, h) = (1280, 720);
    std::fs::create_dir_all(dir).expect("no se pudo crear la carpeta de capturas");
    let mut renderer = Renderer::new(w * SS, h * SS);
    let mut big = vec![0u32; w * SS * h * SS];
    for view in VIEWS {
        let t = match view {
            View::Sistema => 6.0,
            View::Terranova => 4.8,
            _ => 4.0,
        };
        let mut cam = OrbitCamera::for_view(view);
        if view == View::Ambar {
            cam.yaw = -0.25;
        }
        renderer.render(view, t, &cam, &mut big);
        let mut data = Vec::with_capacity(w * h * 3);
        for y in 0..h {
            for x in 0..w {
                let mut acc = [0u32; 3];
                for sy in 0..SS {
                    for sx in 0..SS {
                        let p = big[(y * SS + sy) * w * SS + x * SS + sx];
                        acc[0] += (p >> 16) & 0xFF;
                        acc[1] += (p >> 8) & 0xFF;
                        acc[2] += p & 0xFF;
                    }
                }
                data.extend(acc.iter().map(|c| (c / (SS * SS) as u32) as u8));
            }
        }
        let path = format!("{dir}/{}.png", view.slug());
        let file = std::io::BufWriter::new(std::fs::File::create(&path).expect("no se pudo crear el PNG"));
        let mut enc = png::Encoder::new(file, w as u32, h as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().and_then(|mut wr| wr.write_image_data(&data)).expect("no se pudo escribir el PNG");
        println!("{path}");
    }
}
