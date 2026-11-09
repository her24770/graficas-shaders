//! Thalassa — mundo océano: archipiélagos, arrecifes, huracanes y mares bioluminiscentes de noche.
use super::{polar_around, ramp, rgb, rot_y, smoothstep, Ctx};
use crate::math::{v3, Vec3};
use crate::noise::{fbm, value_noise};

fn islands(p: Vec3) -> f32 {
    fbm(p * 3.2 + v3(12.0, 4.0, 8.0), 6)
}

/// Brazos de un huracán centrado en `center`.
fn hurricane(p: Vec3, center: Vec3, size: f32, t: f32) -> f32 {
    let (d, a) = polar_around(p, center);
    if d > size * 1.6 {
        return 0.0;
    }
    let x = d / size;
    let arms = ((a * 2.0 + (x + 0.02).ln() * 3.2 + t * 0.8).sin() * 0.5 + 0.5).powf(1.5);
    let eye = smoothstep(0.05, 0.14, x);
    (arms * smoothstep(1.4, 0.3, x) + smoothstep(0.6, 0.15, x) * 0.6).min(1.0) * eye
}

pub fn shade(c: &Ctx) -> Vec3 {
    let (p, t) = (c.p, c.t);
    let h = islands(p);

    // Capa 1: océano profundo, de índigo a verde azulado según la profundidad.
    let mut col = ramp(&[(0.3, 0x05153F), (0.5, 0x073E66), (0.6, 0x0B6D80)], h);

    // Capa 2: arrecifes y aguas poco profundas color turquesa alrededor de las islas.
    let reef = smoothstep(0.6, 0.67, h);
    col = col.lerp(rgb(0x2FD6C2).lerp(rgb(0x7BE8D0), value_noise(p * 40.0)), reef);

    // Capa 3: islas: playa de arena y selva tropical en el interior.
    let land = smoothstep(0.675, 0.68, h);
    let ground = rgb(0xE8D59E).lerp(rgb(0x2E7D32), smoothstep(0.69, 0.72, h)).lerp(rgb(0x1B4D20), smoothstep(0.74, 0.8, h));
    col = col.lerp(ground, land);

    // Olas: relieve fino en el agua que hace temblar el reflejo del sol.
    let water = 1.0 - land;
    let n = c.bump(0.003, 0.0004 * water, |q| value_noise(q * 140.0 + v3(t * 0.4, 0.0, 0.0)));
    let dif = c.diffuse_n(n, 0.05);
    let spec = c.specular(n, 90.0) * water;
    col = col * dif + rgb(0xFFF4DD) * spec + col * 0.02;

    // Capa 4: nubes: dos huracanes en espiral y nubes dispersas.
    let pc = rot_y(p, t * 0.05);
    let detail = fbm(pc * 16.0, 3);
    let clouds = smoothstep(0.66, 0.85, fbm(pc * 4.0 + v3(0.0, t * 0.02, 0.0), 5) * 0.85 + detail * 0.2)
        .max(hurricane(pc, v3(-0.5, 0.35, 0.8).normalize(), 0.3, t))
        .max(hurricane(pc, v3(0.2, -0.45, 0.85).normalize(), 0.2, t * 1.3))
        * (0.75 + 0.35 * detail);
    col = col.lerp(v3(0.96, 0.98, 1.0) * c.diffuse(0.1), clouds * 0.95);

    // Capa 5: bioluminiscencia: de noche el mar brilla con corrientes de plancton.
    let night = smoothstep(0.1, -0.25, c.ndl);
    let flow = v3(fbm(p * 2.0 + v3(t * 0.05, 0.0, 0.0), 3), fbm(p * 2.0 + v3(4.0, t * 0.04, 1.0), 3), 0.0) * 2.5;
    let plankton = smoothstep(0.6, 0.95, crate::noise::ridged(p * 5.0 + flow, 4));
    let bio = rgb(0x00E5FF).lerp(rgb(0x39FF9A), value_noise(p * 8.0));
    col += bio * (plankton * night * water * (1.0 - clouds) * 0.9);

    // Capa 6: atmósfera húmeda color cian.
    col + rgb(0x58D7FF) * (c.fresnel(2.2) * (0.12 + c.diffuse(0.3) * 0.9))
}
