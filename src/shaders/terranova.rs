//! Terranova — planeta rocoso: océano, tierra, polos, nubes y atmósfera.
use super::{rgb, rot_y, smoothstep, Ctx};
use crate::math::{v3, Vec3};
use crate::noise::fbm;

pub fn shade(c: &Ctx) -> Vec3 {
    let (p, t) = (c.p, c.t);
    let h = fbm(p * 2.2 + v3(3.0, 7.0, 1.0), 5);

    // Capa 1: océano, más claro cerca de la costa.
    let mut col = rgb(0x0B2E7A).lerp(rgb(0x2A7FC4), smoothstep(0.3, 0.5, h));

    // Capa 2: tierra: verde en lo bajo y café en lo alto.
    let land = rgb(0x3C8A3A).lerp(rgb(0x8A6A45), smoothstep(0.55, 0.7, h));
    col = col.lerp(land, smoothstep(0.5, 0.51, h));

    // Capa 3: casquetes polares.
    col = col.lerp(rgb(0xF0F4F8), smoothstep(0.85, 0.9, p.y.abs()));

    // Capa 4: nubes que giran.
    let cloud = smoothstep(0.6, 0.8, fbm(rot_y(p, t * 0.05) * 4.0, 4));
    col = col.lerp(Vec3::ONE, cloud * 0.8);

    // Luz del sol.
    col = col * (0.05 + c.diffuse(0.0));

    // Capa 5: atmósfera azul en el borde.
    col + rgb(0x5A9BFF) * (c.fresnel(3.0) * c.diffuse(0.3))
}
