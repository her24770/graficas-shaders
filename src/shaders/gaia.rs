//! Gaia — planeta rocoso tipo Tierra: océanos, biomas, montañas, hielo, nubes, ciudades y atmósfera.
use super::{ramp, rgb, rot_y, smoothstep, Ctx};
use crate::math::{v3, Vec3};
use crate::noise::{fbm, ridged, value_noise};

const SEA: f32 = 0.5;

/// Altura del terreno: continentes (fBm) más cordilleras (ruido ridged) solo tierra adentro.
fn height(p: Vec3) -> f32 {
    let h = fbm(p * 2.3 + v3(3.1, 7.4, 1.2), 6);
    h + ridged(p * 5.0 + v3(9.0, 2.0, 4.0), 4) * 0.16 * smoothstep(0.5, 0.65, h)
}

pub fn shade(c: &Ctx) -> Vec3 {
    let (p, t) = (c.p, c.t);
    let h = height(p);
    let lat = p.y.abs();
    let land = smoothstep(SEA - 0.004, SEA + 0.004, h);

    // Capa 1: océano. Color según la profundidad, con aguas turquesa en las costas.
    let depth = ((SEA - h) / SEA).max(0.0);
    let ocean = ramp(&[(0.0, 0x3FB8C9), (0.04, 0x1C7FB0), (0.18, 0x0E3F86), (0.45, 0x081C52)], depth);

    // Capa 2: biomas. Dependen de temperatura (latitud y altura) y humedad (otro ruido).
    let e = ((h - SEA) / (1.0 - SEA)).clamp(0.0, 1.0);
    let moist = fbm(p * 2.6 + v3(50.0, 10.0, 3.0), 4);
    let temp = 1.05 - lat * 1.1 - e * 0.9 + (fbm(p * 4.0, 2) - 0.5) * 0.3;
    let desert = rgb(0xD9B77A).lerp(rgb(0xC08A4E), fbm(p * 9.0, 2));
    let jungle = rgb(0x1F5A26);
    let grass = rgb(0x6E9A3C);
    let tundra = rgb(0x7D7F6A);
    let warm = desert.lerp(grass, smoothstep(0.35, 0.5, moist)).lerp(jungle, smoothstep(0.55, 0.7, moist));
    let mut ground = tundra.lerp(warm, smoothstep(0.12, 0.38, temp));

    ground = ground * (0.82 + 0.35 * fbm(p * 28.0, 3));

    // Capa 3: montañas: roca en las alturas y nieve en las cumbres.
    ground = ground.lerp(rgb(0x6A5847), smoothstep(0.3, 0.5, e));
    ground = ground.lerp(rgb(0xF2F4F8), smoothstep(0.74, 0.86, e + (1.0 - temp) * 0.15));

    let mut albedo = ocean.lerp(ground, land);

    // Capa 4: casquetes polares (también congelan el mar).
    let polar = smoothstep(0.88, 0.93, lat + (fbm(p * 6.0, 3) - 0.5) * 0.12);
    albedo = albedo.lerp(rgb(0xEEF4FA), polar);

    // Iluminación de la superficie: difusa + reflejo del sol en el agua.
    let n = c.bump(0.01, 0.02 * land, height);
    let dif = c.diffuse_n(n, 0.05);
    let water = (1.0 - land) * (1.0 - polar);
    let waves = 0.85 + 0.3 * value_noise(p * 120.0);
    let spec = c.specular(c.n, 70.0) * water * waves;

    // Capa 5: nubes que giran más rápido que el planeta, con su sombra sobre el suelo.
    let pc = rot_y(p, t * 0.04);
    let cw = v3(fbm(pc * 2.0, 3), fbm(pc * 2.0 + v3(7.0, 1.0, 3.0), 3), 0.0);
    let cloud_at = |q: Vec3| {
        let base = fbm(q * 3.6 + cw * 1.1 + v3(0.0, 0.0, t * 0.02), 5);
        smoothstep(0.58, 0.85, base * 0.8 + fbm(q * 14.0 + cw, 3) * 0.25)
    };
    let cloud = cloud_at(pc);
    let shadow = cloud_at(rot_y(pc, 0.015));
    let surface = albedo * (dif * (1.0 - 0.45 * shadow)) + v3(1.0, 0.95, 0.85) * (spec * 0.9);
    let surface = surface + albedo * 0.015;
    let mut col = surface.lerp(v3(0.97, 0.98, 1.0) * (c.diffuse(0.1) * 1.05), cloud * 0.92);

    // Capa 6: luces de ciudades en el lado de noche (solo en tierra templada, tapadas por nubes).
    let night = smoothstep(0.05, -0.2, c.ndl);
    let cities = smoothstep(0.7, 0.85, value_noise(p * 70.0) * 0.6 + fbm(p * 7.0 + v3(2.0, 2.0, 2.0), 3) * 0.6);
    let habitable = land * (1.0 - polar) * smoothstep(0.2, 0.45, temp) * (1.0 - smoothstep(0.35, 0.5, e));
    col += rgb(0xFFB347) * (cities * habitable * night * (1.0 - cloud) * 1.4);

    // Capa 7: atmósfera. Halo azul en el borde y un tono anaranjado en el atardecer (terminador).
    let sunset = smoothstep(-0.15, 0.05, c.ndl) * smoothstep(0.35, 0.0, c.ndl);
    col += rgb(0xFF7A3D) * (sunset * 0.18);
    col + rgb(0x5A9BFF) * (c.fresnel(2.5) * (0.15 + c.diffuse(0.3) * 1.1))
}
