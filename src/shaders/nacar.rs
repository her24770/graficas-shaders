//! Nácar — la luna de Terranova: polvo gris, mares oscuros y cráteres.
use super::{rgb, smoothstep, Ctx};
use crate::math::{v3, Vec3};
use crate::noise::{fbm, voronoi};

pub fn shade(c: &Ctx) -> Vec3 {
    let p = c.p;

    // Capa 1: polvo gris.
    let mut col = rgb(0x8A8680).lerp(rgb(0xB0ACA5), fbm(p * 5.0, 3));

    // Capa 2: mares oscuros.
    let maria = smoothstep(0.5, 0.58, fbm(p * 1.6 + v3(4.0, 8.0, 1.0), 4));
    col = col.lerp(rgb(0x4A4A50), maria * 0.8);

    // Capa 3: cráteres: centro oscuro y borde claro.
    let cell = voronoi(p * 6.0, 1);
    if cell.id > 0.4 {
        let x = cell.f1 / 0.3;
        col = col * (1.0 - smoothstep(1.0, 0.6, x) * 0.35) + Vec3::splat(smoothstep(0.25, 0.0, (x - 1.0).abs()) * 0.15);
    }

    // Capa 4: luz dura, sin atmósfera.
    col * c.diffuse(0.0)
}
