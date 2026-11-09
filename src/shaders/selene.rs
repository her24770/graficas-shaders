//! Selene — la luna de Gaia: regolito, mares de basalto, cráteres con relieve y rayos de eyección.
use super::{polar_around, ramp, rgb, smoothstep, Ctx};
use crate::math::{v3, Vec3};
use crate::noise::{fbm, value_noise, voronoi};

/// Un campo de cráteres: cada celda de Voronoi puede tener un cráter (cuenco + borde levantado).
fn craters(p: Vec3, scale: f32, seed: u32) -> f32 {
    let cell = voronoi(p * scale, seed);
    if cell.id < 0.45 {
        return 0.0;
    }
    let radius = 0.14 + 0.2 * ((cell.id * 13.7).fract());
    let x = cell.f1 / radius;
    let bowl = if x < 1.0 { x * x - 1.0 } else { 0.0 };
    let rim = (-((x - 1.0) / 0.25).powi(2)).exp() * 0.45 * smoothstep(1.6, 1.2, x);
    bowl * 0.7 + rim
}

fn maria(p: Vec3) -> f32 {
    smoothstep(0.5, 0.58, fbm(p * 1.6 + v3(4.0, 8.0, 1.0), 4))
}

fn height(p: Vec3) -> f32 {
    // Los mares son lava solidificada: más lisos, con menos cráteres.
    let smooth = 1.0 - 0.7 * maria(p);
    fbm(p * 6.0, 3) * 0.15 + (craters(p, 4.0, 1) * 0.5 + craters(p, 10.0, 2) * 0.25 + craters(p, 24.0, 3) * 0.05) * smooth
}

pub fn shade(c: &Ctx) -> Vec3 {
    let p = c.p;
    let h = height(p);

    // Capa 1: regolito: polvo gris con variaciones suaves.
    let mut col = ramp(&[(0.0, 0x6F6B66), (0.5, 0x9A9690), (1.0, 0xB9B5AE)], fbm(p * 5.0 + v3(1.0, 2.0, 3.0), 4));

    // Capa 2: mares de basalto oscuro.
    let m = maria(p);
    col = col.lerp(rgb(0x45454B).lerp(rgb(0x5A5650), fbm(p * 9.0, 2)), m * 0.85);

    // Capa 3: cráteres: el fondo es más oscuro y los bordes más claros (además del relieve).
    col = col * (1.0 + h.clamp(-0.6, 0.4) * 0.45);

    // Capa 4: rayos de eyección de un cráter joven (como Tycho).
    let young = v3(-0.35, 0.45, 0.82).normalize();
    let (d, a) = polar_around(p, young);
    let rays = smoothstep(0.55, 0.85, value_noise(v3(a * 7.0 + 20.0, d * 1.5, 0.0))) * (-d * 2.2).exp();
    let crater = smoothstep(0.09, 0.06, d);
    col = col.lerp(rgb(0xE6E2DA), (rays * 0.55 + crater * 0.6).min(1.0));

    // Capa 5: luz dura sin atmósfera, con relieve (bump) y un poco de "luz de tierra" azulada.
    let n = c.bump(0.006, 0.06, height);
    let lit = c.diffuse_n(n, 0.0).powf(0.85);
    col * (lit * 1.1) + rgb(0x1A2440) * 0.06 * col.max_elem()
}
