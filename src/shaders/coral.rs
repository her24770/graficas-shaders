//! Coral — mundo océano con islas, arrecifes y nubes.
use super::{rgb, rot_y, smoothstep, Ctx};
use crate::math::{v3, Vec3};
use crate::noise::fbm;

pub fn shade(c: &Ctx) -> Vec3 {
    let (p, t) = (c.p, c.t);
    let h = fbm(p * 3.2 + v3(12.0, 4.0, 8.0), 5);

    // Capa 1: océano profundo.
    let mut col = rgb(0x05204F).lerp(rgb(0x0B6D80), smoothstep(0.3, 0.6, h));

    // Capa 2: arrecifes turquesa alrededor de las islas.
    col = col.lerp(rgb(0x2FD6C2), smoothstep(0.6, 0.67, h));

    // Capa 3: islas de arena y selva.
    let island = rgb(0xE8D59E).lerp(rgb(0x2E7D32), smoothstep(0.69, 0.73, h));
    col = col.lerp(island, smoothstep(0.675, 0.68, h));

    col = col * (0.05 + c.diffuse(0.0));

    // Capa 4: nubes que giran.
    let cloud = smoothstep(0.62, 0.8, fbm(rot_y(p, t * 0.05) * 4.0, 4));
    col = col.lerp(Vec3::ONE * c.diffuse(0.1), cloud * 0.9);

    // Capa 5: atmósfera cian.
    col + rgb(0x58D7FF) * (c.fresnel(3.0) * c.diffuse(0.3))
}
