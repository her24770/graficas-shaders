//! Ámbar — gigante gaseoso con bandas, una tormenta y anillos.
use super::{polar_around, ramp, rgb, smoothstep, Ctx};
use crate::math::{v3, Vec3};
use crate::noise::fbm;

pub fn shade(c: &Ctx) -> Vec3 {
    let (p, t) = (c.p, c.t);

    // Capa 1: bandas horizontales deformadas con ruido.
    let warp = fbm(v3(p.x * 2.0, p.y * 6.0, p.z * 2.0) + v3(t * 0.03, 0.0, 0.0), 3);
    let band = (p.y * 10.0 + warp * 3.0).sin() * 0.5 + 0.5;
    let mut col = ramp(&[(0.0, 0x8A4B2C), (0.5, 0xD9A56B), (1.0, 0xF5E6C8)], band);

    // Capa 2: vetas finas a lo largo de las bandas.
    let streak = fbm(v3(p.x * 4.0, p.y * 40.0, p.z * 4.0), 3);
    col = col * (0.85 + 0.3 * streak);

    // Capa 3: gran tormenta rojiza.
    let (d, _) = polar_around(p, v3(-0.8, -0.2, 0.55).normalize());
    col = col.lerp(rgb(0xB8442A), smoothstep(0.18, 0.1, d));

    // Luz + Capa 4: sombra de los anillos sobre el planeta.
    col * (0.04 + c.diffuse(0.1) * c.ring_shadow())
}
