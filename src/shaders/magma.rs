//! Magma — planeta volcánico: roca negra partida por ríos de lava.
use super::{ramp, rgb, smoothstep, Ctx};
use crate::math::{v3, Vec3};
use crate::noise::{fbm, voronoi};

pub fn shade(c: &Ctx) -> Vec3 {
    let (p, t) = (c.p, c.t);

    // Capa 1: roca oscura.
    let rock = rgb(0x1A1414).lerp(rgb(0x3E302B), fbm(p * 5.0, 4));
    let mut col = rock * (0.05 + c.diffuse(0.0));

    // Capa 2: grietas de lava (bordes de celdas de Voronoi).
    let cell = voronoi(p * 4.0, 21);
    let crack = 1.0 - smoothstep(0.0, 0.08, cell.f2 - cell.f1);

    // Capa 3: lagos de lava.
    let lake = smoothstep(0.7, 0.75, fbm(p * 1.4 + v3(7.0, 0.0, 3.0), 4));

    // Capa 4: la lava brilla sola y late con el tiempo.
    let heat = crack.max(lake) * (0.8 + 0.2 * (t * 2.0).sin());
    let lava = ramp(&[(0.0, 0x000000), (0.4, 0xC21A00), (0.8, 0xFF7A00), (1.0, 0xFFD060)], heat);
    col = col.lerp(lava, smoothstep(0.1, 0.5, heat));
    col
}
