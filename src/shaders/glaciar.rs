//! Glaciar — planeta de hielo con grietas rojizas y auroras en los polos.
use super::{rgb, smoothstep, Ctx};
use crate::math::{v3, Vec3};
use crate::noise::{fbm, value_noise};

pub fn shade(c: &Ctx) -> Vec3 {
    let (p, t) = (c.p, c.t);

    // Capa 1: hielo blanco y celeste.
    let mut col = rgb(0xA8D4E6).lerp(rgb(0xF5FBFF), fbm(p * 4.0, 4));

    // Capa 2: zonas azules donde se ve el agua bajo el hielo.
    col = col.lerp(rgb(0x2D6F9C), smoothstep(0.6, 0.75, fbm(p * 1.5 + v3(3.0, 0.0, 9.0), 3)) * 0.7);

    // Capa 3: grietas finas color óxido.
    let crack = 1.0 - smoothstep(0.0, 0.025, (fbm(p * 2.5, 4) - 0.5).abs());
    col = col.lerp(rgb(0x9C5A3C), crack * 0.8);

    col = col * (0.05 + c.diffuse(0.0));

    // Capa 4: auroras verdes cerca de los polos.
    let lon = p.z.atan2(p.x);
    let band = smoothstep(0.06, 0.0, (p.y.abs() - 0.75).abs());
    let wave = value_noise(v3(lon * 10.0, t * 0.5, 0.0));
    col + rgb(0x3BFF8A) * (band * wave * 1.2)
}
