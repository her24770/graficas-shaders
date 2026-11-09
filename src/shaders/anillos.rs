//! Anillos de Ámbar. Modelo aparte (un disco con agujero) con transparencia.
use super::{ramp, smoothstep, Ctx};
use crate::math::{v3, Vec3};
use crate::noise::value_noise;

/// Opacidad de los anillos a una distancia `r` del centro (en radios del planeta).
/// También la usa el planeta para saber cuánta sombra le cae.
pub fn ring_density(r: f32) -> f32 {
    if !(1.4..=2.3).contains(&r) {
        return 0.0;
    }
    let lines = 0.7 + 0.3 * value_noise(v3(r * 60.0, 0.5, 0.5));
    // Hueco grande (división de Cassini) entre 1.85 y 1.93.
    let gap = smoothstep(1.85, 1.87, r) * (1.0 - smoothstep(1.91, 1.93, r));
    (lines * (1.0 - gap) * 0.8).clamp(0.0, 1.0)
}

pub fn shade(c: &Ctx, local: Vec3) -> (Vec3, f32) {
    let r = (local.x * local.x + local.z * local.z).sqrt();

    // Capa 1: densidad con huecos (la transparencia).
    let dens = ring_density(r);

    // Capa 2: color según la distancia al planeta.
    let col = ramp(&[(1.4, 0x7A6455), (1.75, 0xE0CDB0), (2.3, 0xA8A8A8)], r);

    // Capa 3: sombra del planeta sobre los anillos.
    let to_l = (c.light_pos - c.world).normalize();
    let oc = c.body.center - c.world;
    let tc = oc.dot(to_l);
    let shadow = if tc > 0.0 && (oc - to_l * tc).length() < c.body.radius { 0.1 } else { 1.0 };

    let lit = 0.4 + 0.6 * c.n.dot(c.l).abs();
    (col * (lit * shadow), dens)
}
