//! Anillos de Cronos. Modelo aparte (un disco plano con agujero) y shader con transparencia.
use super::{ramp, smoothstep, Ctx};
use crate::math::{v3, Vec3};
use crate::noise::{hash, value_noise};

/// Densidad (opacidad) de los anillos a una distancia `r` del centro, en radios del planeta.
/// La usa también el planeta para saber cuánta sombra le cae encima.
pub fn ring_density(r: f32) -> f32 {
    if !(1.35..=2.45).contains(&r) {
        return 0.0;
    }
    // Ondas finas de densidad (miles de "anillitos").
    let fine = value_noise(v3(r * 60.0, 0.5, 0.5)) * 0.6 + value_noise(v3(r * 180.0, 2.5, 0.5)) * 0.4;
    // Anillo C (tenue), B (denso), división de Cassini, anillo A con la división de Encke.
    let c_ring = smoothstep(1.35, 1.42, r) * (1.0 - smoothstep(1.55, 1.6, r)) * 0.3;
    let b_ring = smoothstep(1.58, 1.66, r) * (1.0 - smoothstep(1.92, 1.94, r)) * 0.95;
    let a_ring = smoothstep(2.0, 2.04, r) * (1.0 - smoothstep(2.38, 2.45, r)) * 0.7;
    let encke = 1.0 - smoothstep(0.012, 0.0, (r - 2.28).abs());
    ((c_ring + b_ring + a_ring * encke) * (0.55 + 0.6 * fine)).clamp(0.0, 1.0)
}

pub fn shade(c: &Ctx, local: Vec3) -> (Vec3, f32) {
    let r = (local.x * local.x + local.z * local.z).sqrt();

    // Capa 1: densidad radial con huecos (define la transparencia).
    let dens = ring_density(r);

    // Capa 2: color según la distancia: polvo rojizo adentro, hielo claro afuera.
    let tone = value_noise(v3(r * 25.0, 1.0, 0.0));
    let mut col = ramp(&[(1.35, 0x6E5A4E), (1.6, 0xB89C7E), (1.85, 0xE8D6B8), (2.1, 0xD8C8B0), (2.45, 0xA7B1B8)], r + (tone - 0.5) * 0.08);

    // Capa 3: grumos y destellos de partículas de hielo.
    let ang = local.z.atan2(local.x);
    let clump = value_noise(v3(r * 40.0, ang * 30.0, 0.0));
    let ix = (r * 400.0) as i32;
    let iy = (ang * 600.0) as i32;
    let sparkle = smoothstep(0.985, 1.0, hash(ix, iy, 0, 5));
    col = col * (0.75 + 0.5 * clump) * (0.7 + 0.6 * value_noise(v3(r * 90.0, 3.0, 0.0))) + v3(1.0, 1.0, 1.0) * (sparkle * 0.5);

    // Iluminación: el disco recibe luz por cualquiera de sus caras.
    let n = c.n;
    let lit = 0.45 + 0.75 * n.dot(c.l).abs();
    // Retroiluminación: si miramos hacia el sol a través de los anillos, el polvo brilla.
    let back = c.v.dot(-c.l).max(0.0).powf(6.0) * (1.0 - dens) * 0.8;

    // Capa 4: sombra del planeta sobre los anillos (rayo hacia la luz contra la esfera).
    let to_l = (c.light_pos - c.world).normalize();
    let oc = c.body.center - c.world;
    let tc = oc.dot(to_l);
    let shadow = if tc > 0.0 {
        let perp = (oc - to_l * tc).length() / c.body.radius;
        smoothstep(0.96, 1.04, perp)
    } else {
        1.0
    };

    let out = col * (lit * (0.08 + 0.92 * shadow)) + v3(1.0, 0.85, 0.65) * back;
    (out, (dens * 0.95).clamp(0.0, 1.0))
}
