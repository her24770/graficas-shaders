//! Boreas — luna de hielo con un océano bajo la corteza, grietas rojizas y auroras polares.
use super::{ramp, rgb, smoothstep, Ctx};
use crate::math::{v3, Vec3};
use crate::noise::{fbm, hash, ridged, value_noise};

/// Grietas: líneas finas y largas donde un ruido suave cruza 0.5 (dos familias a distinta escala).
fn lineae(p: Vec3) -> f32 {
    let w = v3(fbm(p * 2.0, 3), fbm(p * 2.0 + v3(5.0, 5.0, 5.0), 3), 0.0) * 0.6;
    let a = 1.0 - smoothstep(0.0, 0.025, (fbm(p * 2.2 + w, 4) - 0.5).abs());
    let b = 1.0 - smoothstep(0.0, 0.018, (fbm(p * 4.5 + w * 1.5 + v3(8.0, 1.0, 3.0), 3) - 0.5).abs());
    a.max(b * 0.8)
}

pub fn shade(c: &Ctx) -> Vec3 {
    let (p, t) = (c.p, c.t);

    // Capa 1: hielo: blanco y celeste pálido.
    let mut col = ramp(&[(0.0, 0x9CC9DB), (0.5, 0xD4ECF4), (1.0, 0xF7FCFF)], fbm(p * 4.0, 5));

    // Capa 2: zonas donde el océano de abajo se ve a través del hielo (azul profundo).
    let deep = smoothstep(0.58, 0.75, fbm(p * 1.5 + v3(3.0, 0.0, 9.0), 4));
    col = col.lerp(rgb(0x2D6F9C).lerp(rgb(0x4FA3C7), value_noise(p * 20.0)), deep * 0.75);

    // Capa 3: "lineae": grietas largas teñidas de sales rojizas (como en Europa).
    let ln = lineae(p);
    let crack = ln;
    let ridges = ridged(p * 6.0, 3);
    col = col.lerp(rgb(0x8E4B32).lerp(rgb(0xC07A55), value_noise(p * 30.0)), crack * 0.85);

    // Capa 4: escarcha: brillo especular fuerte con destellos de cristales.
    let n = c.bump(0.004, 0.006, |q| lineae(q) * -0.5 + fbm(q * 8.0, 2));
    let dif = c.diffuse_n(n, 0.05);
    let q = p * 300.0;
    let glitter = smoothstep(0.97, 1.0, hash(q.x as i32, q.y as i32, q.z as i32, 9)) * (1.0 - crack);
    let spec = c.specular(n, 40.0) * (0.4 + 2.5 * glitter);
    col = col * (0.9 + 0.15 * ridges);
    col = col * dif + v3(0.9, 0.97, 1.0) * spec + rgb(0x0B1A2E) * 0.5;

    // Capa 5: auroras en ambos polos: cortinas verdes y magenta que ondulan con el tiempo.
    let lat = p.y.abs();
    let lon = p.z.atan2(p.x);
    let center = 0.74 + 0.03 * (lon * 5.0 + t * 0.7).sin() + 0.015 * (lon * 13.0 - t * 1.1).sin();
    let band = (-((lat - center) / 0.045).powi(2)).exp();
    let curtain = smoothstep(0.3, 0.9, value_noise(v3(lon * 14.0, t * 0.6, p.y.signum() * 4.0)));
    let aurora = rgb(0x3BFF8A).lerp(rgb(0xD63BFF), smoothstep(center - 0.01, center + 0.04, lat));
    let night = 0.35 + 0.65 * smoothstep(0.2, -0.2, c.ndl);
    col += aurora * (band * curtain * night * 1.6);

    // Capa 6: atmósfera muy tenue, celeste.
    col + rgb(0x9FE3FF) * (c.fresnel(3.0) * (0.08 + 0.4 * c.diffuse(0.3)))
}
