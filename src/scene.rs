//! Escena: qué cuerpos se dibujan en cada vista, cómo se mueven (rotación y traslación)
//! y el pipeline de render: rasterizar → sombrear en paralelo → corona del sol, estrellas y órbitas.
use crate::math::{v3, Mat4, Vec3};
use crate::noise::{fbm, hash, value_noise};
use crate::obj::{self, Mesh};
use crate::raster::{self, GBuffer, EMPTY};
use crate::shaders::{self, rgb, smoothstep, Body, Fragment, Ring, Shader, Uniforms};
use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Sistema,
    Helios,
    Gaia,
    Cronos,
    Selene,
    Vulcano,
    Thalassa,
    Boreas,
}

pub const VIEWS: [View; 8] = [View::Sistema, View::Helios, View::Gaia, View::Cronos, View::Selene, View::Vulcano, View::Thalassa, View::Boreas];

impl View {
    pub fn name(self) -> &'static str {
        match self {
            View::Sistema => "Sistema solar",
            View::Helios => "Helios (estrella)",
            View::Gaia => "Gaia (planeta rocoso) + Selene",
            View::Cronos => "Cronos (gigante gaseoso con anillos)",
            View::Selene => "Selene (luna de Gaia)",
            View::Vulcano => "Vulcano (planeta volcánico)",
            View::Thalassa => "Thalassa (mundo océano)",
            View::Boreas => "Boreas (planeta de hielo)",
        }
    }

    /// Nombre de archivo para las capturas.
    pub fn slug(self) -> &'static str {
        match self {
            View::Sistema => "sistema",
            View::Helios => "helios",
            View::Gaia => "gaia",
            View::Cronos => "cronos",
            View::Selene => "selene",
            View::Vulcano => "vulcano",
            View::Thalassa => "thalassa",
            View::Boreas => "boreas",
        }
    }

    /// Distancia inicial de la cámara.
    pub fn distance(self) -> f32 {
        match self {
            View::Sistema => 40.0,
            View::Cronos => 6.2,
            View::Gaia => 4.1,
            _ => 3.2,
        }
    }

    pub fn pitch(self) -> f32 {
        match self {
            View::Sistema => 0.55,
            View::Cronos => 0.32,
            _ => 0.12,
        }
    }
}

/// Un cuerpo del sistema solar.
struct Planet {
    shader: Shader,
    radius: f32,
    orbit: f32,
    /// Velocidad de traslación (rad/s) y fase inicial.
    orbit_speed: f32,
    phase: f32,
    /// Velocidad de rotación sobre su eje (rad/s) e inclinación del eje.
    spin: f32,
    tilt: f32,
}

const SISTEMA: [Planet; 6] = [
    Planet { shader: Shader::Helios, radius: 2.4, orbit: 0.0, orbit_speed: 0.0, phase: 0.0, spin: 0.05, tilt: 0.1 },
    Planet { shader: Shader::Vulcano, radius: 0.5, orbit: 5.0, orbit_speed: 0.45, phase: 0.5, spin: 0.4, tilt: 0.05 },
    Planet { shader: Shader::Gaia, radius: 0.75, orbit: 8.5, orbit_speed: 0.3, phase: 2.2, spin: 0.35, tilt: 0.41 },
    Planet { shader: Shader::Thalassa, radius: 0.7, orbit: 12.0, orbit_speed: 0.22, phase: 4.0, spin: 0.3, tilt: 0.2 },
    Planet { shader: Shader::Cronos, radius: 1.5, orbit: 17.5, orbit_speed: 0.13, phase: 5.3, spin: 0.5, tilt: 0.47 },
    Planet { shader: Shader::Boreas, radius: 0.6, orbit: 23.5, orbit_speed: 0.08, phase: 1.0, spin: 0.2, tilt: 0.3 },
];

/// Algo que se va a dibujar: una malla con su shader y su transformación.
pub struct Object {
    pub shader: Shader,
    pub model: Mat4,
    pub body: Body,
    pub is_ring: bool,
}

pub struct Frame {
    pub objects: Vec<Object>,
    pub light_pos: Vec3,
    /// Órbitas a dibujar (centro, radio).
    pub orbits: Vec<(Vec3, f32)>,
}

const RING_INNER: f32 = 1.35;
const RING_OUTER: f32 = 2.45;

/// Agrega un planeta en `pos` (y sus anillos si es Cronos).
fn push_planet(out: &mut Vec<Object>, shader: Shader, pos: Vec3, radius: f32, spin_angle: f32, tilt: f32) {
    let tilt_m = Mat4::rotation_z(tilt) * Mat4::rotation_x(tilt * 0.4);
    let rotation = tilt_m * Mat4::rotation_y(spin_angle);
    let ring = (shader == Shader::Cronos).then(|| Ring { normal: tilt_m.transform_dir(Vec3::Y).normalize() });
    let body = Body { center: pos, radius, rotation, ring };
    let s = Mat4::scale(Vec3::splat(radius));
    out.push(Object { shader, model: Mat4::translation(pos) * rotation * s, body, is_ring: false });
    if ring.is_some() {
        out.push(Object { shader: Shader::Anillos, model: Mat4::translation(pos) * tilt_m * s, body, is_ring: true });
    }
}

/// Posición de Selene alrededor de su planeta.
fn moon_offset(t: f32, dist: f32) -> Vec3 {
    let a = t * 0.5 + 1.2;
    v3(a.cos() * dist, (a * 1.0).sin() * dist * 0.18, a.sin() * dist)
}

pub fn build(view: View, t: f32) -> Frame {
    let mut objects = Vec::new();
    let mut orbits = Vec::new();
    // En las vistas individuales la luz viene de lejos, desde la izquierda: se ve el día y una franja de noche.
    let far_light = v3(-1.0, 0.3, 0.35).normalize() * 1000.0;
    let single = |s: Shader| SISTEMA.iter().find(|p| p.shader == s).unwrap();
    match view {
        View::Sistema => {
            for p in &SISTEMA {
                let a = p.phase + t * p.orbit_speed;
                let pos = v3(a.cos() * p.orbit, 0.0, a.sin() * p.orbit);
                push_planet(&mut objects, p.shader, pos, p.radius, t * p.spin, p.tilt);
                if p.orbit > 0.0 {
                    orbits.push((Vec3::ZERO, p.orbit));
                }
                if p.shader == Shader::Gaia {
                    let mpos = pos + moon_offset(t * 2.0, 1.5);
                    push_planet(&mut objects, Shader::Selene, mpos, 0.22, t * 0.5, 0.05);
                    orbits.push((pos, 1.5));
                }
            }
            Frame { objects, light_pos: Vec3::ZERO, orbits }
        }
        View::Gaia => {
            let p = single(Shader::Gaia);
            push_planet(&mut objects, Shader::Gaia, Vec3::ZERO, 1.0, t * p.spin * 0.5, p.tilt);
            push_planet(&mut objects, Shader::Selene, moon_offset(t, 2.6), 0.27, t * 0.25, 0.05);
            Frame { objects, light_pos: far_light, orbits }
        }
        View::Selene => {
            push_planet(&mut objects, Shader::Selene, Vec3::ZERO, 1.0, t * 0.1, 0.05);
            Frame { objects, light_pos: far_light, orbits }
        }
        v => {
            let s = match v {
                View::Helios => Shader::Helios,
                View::Cronos => Shader::Cronos,
                View::Vulcano => Shader::Vulcano,
                View::Thalassa => Shader::Thalassa,
                _ => Shader::Boreas,
            };
            let p = single(s);
            push_planet(&mut objects, s, Vec3::ZERO, 1.0, t * p.spin * 0.5, p.tilt);
            Frame { objects, light_pos: far_light, orbits }
        }
    }
}

/// Cámara que orbita alrededor de un punto.
#[derive(Clone, Copy, Debug)]
pub struct OrbitCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub target: Vec3,
}

impl OrbitCamera {
    pub fn for_view(view: View) -> Self {
        Self { yaw: 0.0, pitch: view.pitch(), distance: view.distance(), target: Vec3::ZERO }
    }

    pub fn eye(&self) -> Vec3 {
        self.target + v3(self.pitch.cos() * self.yaw.sin(), self.pitch.sin(), self.pitch.cos() * self.yaw.cos()) * self.distance
    }
}

const FOV: f32 = 0.75;

pub struct Renderer {
    sphere: Mesh,
    ring: Mesh,
    gb: GBuffer,
    color: Vec<Vec3>,
}

impl Renderer {
    pub fn new(width: usize, height: usize) -> Self {
        let sphere = obj::parse(include_str!("../assets/sphere.obj")).expect("assets/sphere.obj inválido");
        Self { sphere, ring: Mesh::ring(RING_INNER, RING_OUTER, 160), gb: GBuffer::new(width, height), color: vec![Vec3::ZERO; width * height] }
    }

    pub fn size(&self) -> (usize, usize) {
        (self.gb.width, self.gb.height)
    }

    /// Dibuja la vista y deja el resultado en `out` (0x00RRGGBB).
    pub fn render(&mut self, view: View, t: f32, cam: &OrbitCamera, out: &mut [u32]) {
        let (w, h) = self.size();
        let frame = build(view, t);
        let eye = cam.eye();
        let aspect = w as f32 / h as f32;
        let view_m = Mat4::look_at(eye, cam.target, Vec3::Y);
        let view_proj = Mat4::perspective(FOV, aspect, 0.05, 500.0) * view_m;
        let uniforms = Uniforms { time: t, light_pos: frame.light_pos, eye };

        // 1. Rasterizar: primero lo opaco y después los anillos (transparentes).
        self.gb.clear();
        for pass_ring in [false, true] {
            for (i, o) in frame.objects.iter().enumerate().filter(|(_, o)| o.is_ring == pass_ring) {
                let mesh = if o.is_ring { &self.ring } else { &self.sphere };
                raster::draw_mesh(&mut self.gb, mesh, &o.model, &view_proj, i as u16, o.is_ring);
            }
        }

        // Datos de la corona de cada estrella en pantalla: (x, y, radio en píxeles).
        let forward = (cam.target - eye).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward);
        let suns: Vec<(f32, f32, f32)> = frame
            .objects
            .iter()
            .filter(|o| o.shader == Shader::Helios)
            .filter_map(|o| {
                let (x, y, _) = raster::project(&view_proj, o.body.center, w, h)?;
                let (x2, y2, _) = raster::project(&view_proj, o.body.center + up * o.body.radius, w, h)?;
                Some((x, y, ((x2 - x).powi(2) + (y2 - y).powi(2)).sqrt()))
            })
            .collect();

        // 2. Sombrear en paralelo: cada hilo se encarga de un bloque de filas.
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        let rows = h.div_ceil(threads * 4).max(1);
        let gb = &self.gb;
        let objects = &frame.objects;
        let tan = (FOV * 0.5).tan();
        std::thread::scope(|s| {
            let mut chunks = self.color.chunks_mut(rows * w).enumerate().collect::<Vec<_>>().into_iter();
            for _ in 0..threads {
                let mine: Vec<_> = chunks.by_ref().take(h.div_ceil(rows).div_ceil(threads)).collect();
                let suns = &suns;
                s.spawn(move || {
                    for (ci, chunk) in mine {
                        for (k, px) in chunk.iter_mut().enumerate() {
                            let idx = ci * rows * w + k;
                            let (x, y) = (idx % w, idx / w);
                            let id = gb.opaque.id[idx];
                            let mut c = if id == EMPTY {
                                let nx = ((x as f32 + 0.5) / w as f32) * 2.0 - 1.0;
                                let ny = 1.0 - ((y as f32 + 0.5) / h as f32) * 2.0;
                                let dir = (forward + right * (nx * tan * aspect) + up * (ny * tan)).normalize();
                                let mut bg = background(dir);
                                for &(sx, sy, sr) in suns {
                                    bg += corona(x as f32 + 0.5 - sx, y as f32 + 0.5 - sy, sr, t);
                                }
                                bg
                            } else {
                                let o = &objects[id as usize];
                                let d = gb.opaque.data[idx];
                                shaders::shade(o.shader, &Fragment { world: d.world, normal: d.normal, local: d.local }, &o.body, &uniforms).0
                            };
                            let tid = gb.transparent.id[idx];
                            if tid != EMPTY {
                                let o = &objects[tid as usize];
                                let d = gb.transparent.data[idx];
                                let (rc, a) = shaders::shade(o.shader, &Fragment { world: d.world, normal: d.normal, local: d.local }, &o.body, &uniforms);
                                c = c.lerp(rc, a);
                            }
                            *px = c;
                        }
                    }
                });
            }
        });

        // 3. Órbitas: líneas tenues que respetan la profundidad (se esconden detrás de los planetas).
        for &(center, radius) in &frame.orbits {
            let n = (radius * 220.0) as usize;
            for k in 0..n {
                let a = k as f32 / n as f32 * TAU;
                let p = center + v3(a.cos() * radius, 0.0, a.sin() * radius);
                if let Some((x, y, z)) = raster::project(&view_proj, p, w, h) {
                    if x >= 0.0 && y >= 0.0 && (x as usize) < w && (y as usize) < h {
                        let idx = y as usize * w + x as usize;
                        if z < self.gb.opaque.depth[idx] {
                            self.color[idx] = self.color[idx].lerp(v3(0.45, 0.55, 0.8), 0.35);
                        }
                    }
                }
            }
        }

        for (o, c) in out.iter_mut().zip(&self.color) {
            *o = shaders::pack(*c);
        }
    }
}

/// Fondo: nebulosa tenue y estrellas de distintos tamaños y colores.
fn background(dir: Vec3) -> Vec3 {
    let neb = fbm(dir * 2.2 + v3(3.0, 1.0, 7.0), 4);
    let neb2 = fbm(dir * 4.5 + v3(9.0, 4.0, 1.0), 3);
    let mut c = rgb(0x1A0B35) * (smoothstep(0.45, 0.9, neb) * 0.6) + rgb(0x07324A) * (smoothstep(0.5, 0.95, neb2) * 0.5);
    c += v3(0.004, 0.004, 0.012);
    for (scale, chance, seed) in [(140.0, 0.93, 1u32), (60.0, 0.97, 2)] {
        let q = dir * scale;
        let (ix, iy, iz) = (q.x.floor() as i32, q.y.floor() as i32, q.z.floor() as i32);
        let hsh = hash(ix, iy, iz, seed);
        if hsh > chance {
            let center = v3(ix as f32 + 0.5, iy as f32 + 0.5, iz as f32 + 0.5);
            let d = (q - center).length();
            let b = smoothstep(0.35, 0.0, d) * (hsh - chance) / (1.0 - chance);
            let tint = rgb(0xFFD6A0).lerp(rgb(0xA8C8FF), hash(ix, iy, iz, seed + 9));
            c += tint * (b * 1.6);
        }
    }
    c
}

/// Corona del sol: brillo alrededor del disco con serpentinas que cambian con el tiempo.
fn corona(dx: f32, dy: f32, radius: f32, t: f32) -> Vec3 {
    let d = (dx * dx + dy * dy).sqrt() / radius.max(1e-3);
    if d < 0.98 {
        return Vec3::ZERO;
    }
    let ang = dy.atan2(dx);
    let streamers = 0.55 + 0.9 * fbm(v3(ang.cos() * 2.5, ang.sin() * 2.5, t * 0.15), 4);
    let rays = 0.8 + 0.4 * value_noise(v3(ang * 12.0, t * 0.4, 0.0));
    let x = (d - 1.0).max(0.0);
    let inner = (-x * 6.0 / streamers).exp();
    let outer = (-x * 1.6 / streamers).exp() * 0.35 * rays;
    let halo = 0.12 / (1.0 + x * x * 4.0);
    rgb(0xFFE3A0) * inner + rgb(0xFF8A2A) * outer + rgb(0xFF6A1A) * halo
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cada_vista_dibuja_algo() {
        let mut r = Renderer::new(96, 54);
        let mut out = vec![0u32; 96 * 54];
        for v in VIEWS {
            r.render(v, 1.0, &OrbitCamera::for_view(v), &mut out);
            let centro = out[27 * 96 + 48];
            assert!(centro != 0, "{v:?} no dibujó nada en el centro");
        }
    }

    #[test]
    fn cronos_tiene_anillos_como_modelo_aparte() {
        let f = build(View::Cronos, 0.0);
        assert_eq!(f.objects.len(), 2);
        assert!(f.objects[1].is_ring && f.objects[1].shader == Shader::Anillos);
    }

    #[test]
    fn los_planetas_se_trasladan_y_rotan() {
        let a = build(View::Sistema, 0.0);
        let b = build(View::Sistema, 3.0);
        let gaia = |f: &Frame| f.objects.iter().find(|o| o.shader == Shader::Gaia).unwrap().body;
        assert!(gaia(&a).center.distance(gaia(&b).center) > 0.5, "traslación");
        assert_ne!(gaia(&a).rotation, gaia(&b).rotation, "rotación");
    }
}
