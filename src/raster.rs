//! Rasterizador de triángulos propio con sombreado diferido:
//! 1. Se rasterizan todas las mallas a un G-buffer (posición, normal, posición local y qué objeto es).
//! 2. Después se corre el fragment shader una sola vez por píxel visible, en paralelo.
//!
//! Hay una capa opaca y una capa transparente (los anillos) que se mezcla encima.
use crate::math::{Mat4, Vec3, Vec4};
use crate::obj::Mesh;

pub const EMPTY: u16 = u16::MAX;

#[derive(Clone, Copy, Default)]
pub struct Sample {
    pub world: Vec3,
    pub normal: Vec3,
    pub local: Vec3,
}

/// Una capa del G-buffer.
pub struct Layer {
    /// Profundidad NDC por píxel (-1 cerca, 1 lejos).
    pub depth: Vec<f32>,
    pub id: Vec<u16>,
    pub data: Vec<Sample>,
}

impl Layer {
    fn new(n: usize) -> Self {
        Self { depth: vec![f32::INFINITY; n], id: vec![EMPTY; n], data: vec![Sample::default(); n] }
    }

    fn clear(&mut self) {
        self.depth.fill(f32::INFINITY);
        self.id.fill(EMPTY);
    }
}

pub struct GBuffer {
    pub width: usize,
    pub height: usize,
    pub opaque: Layer,
    pub transparent: Layer,
}

impl GBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        let n = width * height;
        Self { width, height, opaque: Layer::new(n), transparent: Layer::new(n) }
    }

    pub fn clear(&mut self) {
        self.opaque.clear();
        self.transparent.clear();
    }
}

#[derive(Clone, Copy)]
struct ClipVertex {
    pos: Vec4,
    var: Sample,
}

fn lerp_sample(a: Sample, b: Sample, t: f32) -> Sample {
    Sample { world: a.world.lerp(b.world, t), normal: a.normal.lerp(b.normal, t), local: a.local.lerp(b.local, t) }
}

/// Rasteriza `mesh` en el G-buffer con el identificador `id`. Devuelve cuántos triángulos se dibujaron.
pub fn draw_mesh(gb: &mut GBuffer, mesh: &Mesh, model: &Mat4, view_proj: &Mat4, id: u16, transparent: bool) -> usize {
    let world: Vec<Vec3> = mesh.vertices.iter().map(|v| model.transform_point3(*v)).collect();
    let clip: Vec<Vec4> = world.iter().map(|v| view_proj.transform_point(*v)).collect();
    let normals: Vec<Vec3> = mesh.normals.iter().map(|n| model.transform_dir(*n).normalize()).collect();
    let mut drawn = 0;
    for t in &mesh.triangles {
        let idx = [t[0] as usize, t[1] as usize, t[2] as usize];
        let tri = idx.map(|i| ClipVertex { pos: clip[i], var: Sample { world: world[i], normal: normals[i], local: mesh.vertices[i] } });
        let (poly, n) = clip_near(tri);
        for j in 1..n.saturating_sub(1) {
            if fill_triangle(gb, [poly[0], poly[j], poly[j + 1]], id, transparent) {
                drawn += 1;
            }
        }
    }
    drawn
}

/// Proyecta un punto a píxeles. Devuelve (x, y, profundidad NDC); None si está detrás de la cámara.
pub fn project(view_proj: &Mat4, p: Vec3, width: usize, height: usize) -> Option<(f32, f32, f32)> {
    let c = view_proj.transform_point(p);
    if c.w <= 1e-4 {
        return None;
    }
    let (x, y) = (c.x / c.w, c.y / c.w);
    Some(((x * 0.5 + 0.5) * width as f32, (1.0 - (y * 0.5 + 0.5)) * height as f32, c.z / c.w))
}

/// Recorta el triángulo contra el plano cercano (z >= -w). Devuelve hasta 4 vértices.
fn clip_near(tri: [ClipVertex; 3]) -> ([ClipVertex; 4], usize) {
    let mut out = [tri[0]; 4];
    let mut n = 0;
    for i in 0..3 {
        let (a, b) = (tri[i], tri[(i + 1) % 3]);
        let (da, db) = (a.pos.z + a.pos.w, b.pos.z + b.pos.w);
        if da >= 0.0 {
            out[n] = a;
            n += 1;
        }
        if (da >= 0.0) != (db >= 0.0) {
            let t = da / (da - db);
            out[n] = ClipVertex { pos: Vec4::lerp(a.pos, b.pos, t), var: lerp_sample(a.var, b.var, t) };
            n += 1;
        }
    }
    (out, n)
}

fn fill_triangle(gb: &mut GBuffer, tri: [ClipVertex; 3], id: u16, transparent: bool) -> bool {
    let (w, h) = (gb.width as f32, gb.height as f32);
    let mut p = [(0.0f32, 0.0f32, 0.0f32); 3];
    let mut inv_w = [0.0f32; 3];
    for (i, v) in tri.iter().enumerate() {
        inv_w[i] = 1.0 / v.pos.w;
        let (nx, ny, nz) = (v.pos.x * inv_w[i], v.pos.y * inv_w[i], v.pos.z * inv_w[i]);
        p[i] = ((nx * 0.5 + 0.5) * w, (1.0 - (ny * 0.5 + 0.5)) * h, nz);
    }
    let edge = |a: (f32, f32, f32), b: (f32, f32, f32), x: f32, y: f32| (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0);
    // En pantalla (Y hacia abajo) las caras frontales quedan con área negativa.
    let area = edge(p[0], p[1], p[2].0, p[2].1);
    if area >= -1e-9 {
        return false;
    }
    let min_x = p.iter().map(|v| v.0).fold(f32::INFINITY, f32::min).floor().max(0.0) as usize;
    let max_x = p.iter().map(|v| v.0).fold(f32::NEG_INFINITY, f32::max).ceil().min(w - 1.0).max(0.0) as usize;
    let min_y = p.iter().map(|v| v.1).fold(f32::INFINITY, f32::min).floor().max(0.0) as usize;
    let max_y = p.iter().map(|v| v.1).fold(f32::NEG_INFINITY, f32::max).ceil().min(h - 1.0).max(0.0) as usize;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let b0 = edge(p[1], p[2], px, py) / area;
            let b1 = edge(p[2], p[0], px, py) / area;
            let b2 = edge(p[0], p[1], px, py) / area;
            if b0 < 0.0 || b1 < 0.0 || b2 < 0.0 {
                continue;
            }
            let z = b0 * p[0].2 + b1 * p[1].2 + b2 * p[2].2;
            let idx = y * gb.width + x;
            if z > 1.0 || z >= gb.opaque.depth[idx] {
                continue;
            }
            let layer = if transparent { &mut gb.transparent } else { &mut gb.opaque };
            if transparent && z >= layer.depth[idx] {
                continue;
            }
            // Corrección de perspectiva: se interpola attr/w y se divide por 1/w.
            let (q0, q1, q2) = (b0 * inv_w[0], b1 * inv_w[1], b2 * inv_w[2]);
            let k = 1.0 / (q0 + q1 + q2);
            let mix = |f: fn(&Sample) -> Vec3| (f(&tri[0].var) * q0 + f(&tri[1].var) * q1 + f(&tri[2].var) * q2) * k;
            layer.depth[idx] = z;
            layer.id[idx] = id;
            layer.data[idx] = Sample { world: mix(|v| v.world), normal: mix(|v| v.normal), local: mix(|v| v.local) };
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camara() -> Mat4 {
        Mat4::perspective(1.0, 1.0, 0.1, 100.0) * Mat4::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y)
    }

    #[test]
    fn la_esfera_cubre_el_centro_y_no_las_esquinas() {
        let esfera = crate::obj::parse(include_str!("../assets/sphere.obj")).unwrap();
        let mut gb = GBuffer::new(64, 64);
        draw_mesh(&mut gb, &esfera, &Mat4::IDENTITY, &camara(), 3, false);
        assert_eq!(gb.opaque.id[32 * 64 + 32], 3);
        assert_eq!(gb.opaque.id[0], EMPTY);
        // Lo que se ve es la cara de enfrente (z > 0), no la de atrás.
        assert!(gb.opaque.data[32 * 64 + 32].world.z > 0.9);
    }

    #[test]
    fn lo_transparente_detras_de_lo_opaco_no_se_guarda() {
        let esfera = crate::obj::parse(include_str!("../assets/sphere.obj")).unwrap();
        let mut gb = GBuffer::new(64, 64);
        draw_mesh(&mut gb, &esfera, &Mat4::IDENTITY, &camara(), 0, false);
        let lejos = Mat4::translation(Vec3::new(0.0, 0.0, -3.0));
        draw_mesh(&mut gb, &esfera, &lejos, &camara(), 1, true);
        assert_eq!(gb.transparent.id[32 * 64 + 32], EMPTY);
    }
}
