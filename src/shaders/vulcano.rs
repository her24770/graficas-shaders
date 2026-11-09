//! Vulcano — planeta volcánico: corteza de basalto partida por ríos de lava que laten.
use super::{ramp, rgb, smoothstep, Ctx};
use crate::math::{v3, Vec3};
use crate::noise::{fbm, value_noise, voronoi};

fn crust(p: Vec3) -> f32 {
    fbm(p * 5.0 + v3(2.0, 1.0, 0.0), 5)
}

pub fn shade(c: &Ctx) -> Vec3 {
    let (p, t) = (c.p, c.t);

    // Capa 1: corteza de basalto, negra y rugosa (con relieve).
    let h = crust(p);
    let rock = ramp(&[(0.0, 0x120D0C), (0.5, 0x2B211E), (1.0, 0x4A3A33)], h);

    // Capa 2: grietas. Bordes de celdas de Voronoi deformadas: placas de roca flotando sobre lava.
    let warp = v3(fbm(p * 3.0, 3), fbm(p * 3.0 + v3(9.0, 3.0, 1.0), 3), 0.0) * 1.2;
    let cell = voronoi(p * 4.5 + warp, 21);
    let e = cell.f2 - cell.f1;
    let crack = 1.0 - smoothstep(0.0, 0.07, e);
    let halo = 1.0 - smoothstep(0.0, 0.3, e);

    // Capa 3: lagos de lava fundida que fluyen lentamente.
    let lake = smoothstep(0.7, 0.75, fbm(p * 1.4 + v3(7.0, 0.0, 3.0), 4));
    let flow = fbm(p * 7.0 + v3(t * 0.12, -t * 0.08, t * 0.05), 4);

    // Capa 4: depósitos de azufre y ceniza junto a las zonas calientes.
    let sulfur = smoothstep(0.6, 0.75, value_noise(p * 14.0)) * halo * (1.0 - crack);
    let ash = smoothstep(0.55, 0.7, fbm(p * 2.2 + v3(0.0, 5.0, 0.0), 3));
    let mut surf = rock.lerp(rgb(0xB5A23A), sulfur * 0.6).lerp(rgb(0x5B5552), ash * 0.35);

    let n = c.bump(0.01, 0.05, crust);
    surf = surf * (0.04 + c.diffuse_n(n, 0.0));

    // Capa 5: emisión: la lava brilla sola (también de noche) y late con el tiempo.
    let pulse = 0.8 + 0.2 * (t * 2.0 + cell.id * std::f32::consts::TAU).sin();
    let heat = (crack * (0.6 + 0.4 * flow) * pulse).max(lake * (0.55 + 0.45 * flow));
    let lava = ramp(&[(0.0, 0x000000), (0.3, 0x8A0E00), (0.6, 0xFF4A00), (0.85, 0xFFA21C), (1.0, 0xFFF0A0)], heat);
    let glow = rgb(0xFF3A00) * (halo * 0.18 * pulse * (1.0 - lake));
    let col = surf.lerp(lava * 1.3, smoothstep(0.05, 0.4, heat)) + glow;

    // Capa 6: atmósfera de ceniza y humo, rojiza por el calor de abajo.
    col + rgb(0xFF5A2A) * (c.fresnel(2.5) * (0.25 + 0.5 * c.diffuse(0.5)))
}
