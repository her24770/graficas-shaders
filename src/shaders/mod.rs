//! Fragment shaders: cada cuerpo celeste es solo una función que recibe la posición,
//! la normal y la posición local del píxel y devuelve un color. Sin texturas ni materiales:
//! todo sale de ruido procedural, el tiempo y la luz.
//!
//! Cada shader está hecho por **capas** que se van sumando o mezclando sobre el color;
//! en cada archivo están numeradas (`// Capa N`).
use crate::math::{v3, Vec3};

mod anillos;
mod glaciar;
mod ambar;
mod terranova;
mod ignis;
mod nacar;
mod coral;
mod magma;

pub use anillos::ring_density;

/// Variables globales del cuadro.
#[derive(Clone, Copy, Debug)]
pub struct Uniforms {
    pub time: f32,
    /// Posición de la luz (el sol).
    pub light_pos: Vec3,
    pub eye: Vec3,
}

/// Datos interpolados de un píxel.
#[derive(Clone, Copy, Debug)]
pub struct Fragment {
    pub world: Vec3,
    pub normal: Vec3,
    /// Posición en el espacio del modelo: es la "coordenada de textura" 3D, gira con el planeta.
    pub local: Vec3,
}

/// Anillos de un planeta (radios en unidades del radio del planeta).
#[derive(Clone, Copy, Debug)]
pub struct Ring {
    pub normal: Vec3,
}

/// Datos del objeto que se está sombreando.
#[derive(Clone, Copy, Debug)]
pub struct Body {
    pub center: Vec3,
    pub radius: f32,
    pub ring: Option<Ring>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shader {
    Ignis,
    Terranova,
    Ambar,
    Anillos,
    Nacar,
    Magma,
    Coral,
    Glaciar,
}

/// Todo lo que suelen necesitar los shaders, ya calculado.
pub struct Ctx {
    /// Dirección local unitaria (gira con el planeta).
    pub p: Vec3,
    /// Normal en el mundo.
    pub n: Vec3,
    /// Hacia la luz.
    pub l: Vec3,
    /// Hacia la cámara.
    pub v: Vec3,
    /// n · l sin recortar (negativo en el lado de noche).
    pub ndl: f32,
    pub t: f32,
    pub world: Vec3,
    pub body: Body,
    pub light_pos: Vec3,
}

impl Ctx {
    /// Iluminación difusa con un poco de "envoltura" para que el terminador sea suave.
    pub fn diffuse(&self, wrap: f32) -> f32 {
        ((self.ndl + wrap) / (1.0 + wrap)).max(0.0)
    }

    /// Fresnel: 0 al centro del disco, 1 en el borde.
    pub fn fresnel(&self, power: f32) -> f32 {
        (1.0 - self.n.dot(self.v).max(0.0)).powf(power)
    }

    /// Sombra que proyectan los anillos del planeta sobre su superficie (1 = sin sombra).
    pub fn ring_shadow(&self) -> f32 {
        let Some(r) = self.body.ring else { return 1.0 };
        let d = (self.light_pos - self.world).normalize();
        let den = d.dot(r.normal);
        if den.abs() < 1e-5 {
            return 1.0;
        }
        let t = (self.body.center - self.world).dot(r.normal) / den;
        if t <= 0.0 {
            return 1.0;
        }
        let hit = self.world + d * t;
        let dist = (hit - self.body.center).length() / self.body.radius;
        1.0 - ring_density(dist) * 0.85
    }
}

pub fn shade(s: Shader, f: &Fragment, body: &Body, u: &Uniforms) -> (Vec3, f32) {
    let n = f.normal.normalize();
    let l = (u.light_pos - f.world).normalize();
    let ctx = Ctx {
        p: f.local.normalize(),
        n,
        l,
        v: (u.eye - f.world).normalize(),
        ndl: n.dot(l),
        t: u.time,
        world: f.world,
        body: *body,
        light_pos: u.light_pos,
    };
    match s {
        Shader::Ignis => (ignis::shade(&ctx), 1.0),
        Shader::Terranova => (terranova::shade(&ctx), 1.0),
        Shader::Ambar => (ambar::shade(&ctx), 1.0),
        Shader::Anillos => anillos::shade(&ctx, f.local),
        Shader::Nacar => (nacar::shade(&ctx), 1.0),
        Shader::Magma => (magma::shade(&ctx), 1.0),
        Shader::Coral => (coral::shade(&ctx), 1.0),
        Shader::Glaciar => (glaciar::shade(&ctx), 1.0),
    }
}

// ---------- utilidades de color ----------

pub fn rgb(c: u32) -> Vec3 {
    v3(((c >> 16) & 0xFF) as f32, ((c >> 8) & 0xFF) as f32, (c & 0xFF) as f32) * (1.0 / 255.0)
}

/// Convierte a 0x00RRGGBB con un "tonemap" suave para que los brillos fuertes no se quemen de golpe.
pub fn pack(c: Vec3) -> u32 {
    let ch = |v: f32| {
        let v = v.max(0.0);
        let v = if v > 0.8 { 0.8 + (1.0 - (-(v - 0.8) * 5.0).exp()) * 0.2 } else { v };
        (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u32
    };
    (ch(c.x) << 16) | (ch(c.y) << 8) | ch(c.z)
}

pub fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Rampa de color: lista de (posición, color) ordenada por posición.
pub fn ramp(stops: &[(f32, u32)], x: f32) -> Vec3 {
    if x <= stops[0].0 {
        return rgb(stops[0].1);
    }
    for w in stops.windows(2) {
        if x <= w[1].0 {
            let t = (x - w[0].0) / (w[1].0 - w[0].0);
            return rgb(w[0].1).lerp(rgb(w[1].1), t);
        }
    }
    rgb(stops[stops.len() - 1].1)
}

/// Gira `p` alrededor del eje Y.
pub fn rot_y(p: Vec3, a: f32) -> Vec3 {
    let (s, c) = a.sin_cos();
    v3(c * p.x + s * p.z, p.y, -s * p.x + c * p.z)
}

/// Coordenadas 2D alrededor de un punto `c` de la esfera: (distancia, ángulo).
/// Sirve para dibujar tormentas y espirales centradas en un lugar del planeta.
pub fn polar_around(p: Vec3, c: Vec3) -> (f32, f32) {
    let up = if c.y.abs() < 0.99 { Vec3::Y } else { v3(1.0, 0.0, 0.0) };
    let e1 = c.cross(up).normalize();
    let e2 = c.cross(e1);
    let (x, y) = (p.dot(e1), p.dot(e2));
    let d = if p.dot(c) > 0.0 { (x * x + y * y).sqrt() } else { 2.0 };
    (d, y.atan2(x))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cuerpo() -> Body {
        Body { center: Vec3::ZERO, radius: 1.0, ring: None }
    }

    fn color(s: Shader, local: Vec3, t: f32, luz: Vec3) -> u32 {
        let f = Fragment { world: local, normal: local, local };
        let u = Uniforms { time: t, light_pos: luz * 100.0, eye: v3(0.0, 0.0, 5.0) };
        pack(shade(s, &f, &cuerpo(), &u).0)
    }

    const TODOS: [Shader; 7] = [Shader::Ignis, Shader::Terranova, Shader::Ambar, Shader::Nacar, Shader::Magma, Shader::Coral, Shader::Glaciar];

    #[test]
    fn cada_shader_tiene_variedad_de_colores() {
        for s in TODOS {
            let colores: std::collections::HashSet<u32> = (0..200)
                .map(|i| {
                    let a = i as f32 * 0.37;
                    color(s, v3(a.sin() * 0.8, (a * 0.31).cos() * 0.6, 0.6).normalize(), 0.0, v3(0.0, 0.0, 1.0))
                })
                .collect();
            assert!(colores.len() > 40, "{s:?} tiene pocos colores ({})", colores.len());
        }
    }

    #[test]
    fn el_sol_se_anima_con_el_tiempo() {
        let p = v3(0.3, 0.5, 0.8).normalize();
        assert_ne!(color(Shader::Ignis, p, 0.0, Vec3::Y), color(Shader::Ignis, p, 5.0, Vec3::Y));
    }

    #[test]
    fn el_lado_de_noche_es_mas_oscuro() {
        for s in [Shader::Ambar, Shader::Nacar, Shader::Glaciar] {
            let p = v3(0.2, 0.1, 1.0).normalize();
            let brillo = |c: u32| (c >> 16 & 0xFF) + (c >> 8 & 0xFF) + (c & 0xFF);
            let dia = brillo(color(s, p, 0.0, v3(0.0, 0.0, 1.0)));
            let noche = brillo(color(s, p, 0.0, v3(0.0, 0.0, -1.0)));
            assert!(dia > noche, "{s:?}");
        }
    }

    #[test]
    fn los_anillos_tienen_huecos() {
        assert!(ring_density(1.89) < 0.15, "división de Cassini");
        assert!(ring_density(1.7) > 0.4);
        assert_eq!(ring_density(0.5), 0.0);
    }

    #[test]
    fn pack_redondea_y_satura() {
        assert_eq!(pack(v3(0.0, 0.0, 0.0)), 0);
        assert_eq!(pack(v3(10.0, 10.0, 10.0)), 0xFFFFFF);
    }
}
