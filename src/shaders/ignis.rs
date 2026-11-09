//! Ignis — la estrella del sistema. No depende de la luz, brilla sola.
use super::{ramp, smoothstep, Ctx};
use crate::math::{v3, Vec3};
use crate::noise::fbm;

pub fn shade(c: &Ctx) -> Vec3 {
    let (p, t) = (c.p, c.t);

    // Capa 1: plasma que se mueve con el tiempo.
    let plasma = fbm(p * 3.0 + v3(t * 0.1, t * 0.05, 0.0), 4);
    let mut col = ramp(&[(0.2, 0xC83200), (0.5, 0xFF8C1A), (0.8, 0xFFE070)], plasma);

    // Capa 2: manchas solares oscuras.
    let s = fbm(p * 6.0 + v3(10.0, 0.0, 0.0), 3);
    col = col * (1.0 - smoothstep(0.76, 0.82, s) * 0.7);

    // Capa 3: el borde se ve más oscuro y rojo.
    let mu = c.n.dot(c.v).max(0.0);
    col = col * v3(1.0, 0.6, 0.4).lerp(Vec3::ONE, mu);

    // Capa 4: pulso de brillo.
    col * (1.1 + 0.1 * (t * 1.5).sin())
}
