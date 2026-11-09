//! Cronos — gigante gaseoso con bandas, viento, una gran tormenta y un hexágono polar.
use super::{polar_around, ramp, rgb, rot_axis, rot_y, smoothstep, Ctx};
use crate::math::{v3, Vec3};
use crate::noise::{fbm, value_noise};
use std::f32::consts::PI;

pub fn shade(c: &Ctx) -> Vec3 {
    let (p, t) = (c.p, c.t);

    // Rotación diferencial: cada latitud gira a otra velocidad (corrientes en chorro).
    let mut q = rot_y(p, t * 0.05 * (p.y * 14.0).sin());

    // La gran tormenta deforma todo lo que la rodea: las coordenadas giran alrededor de su centro,
    // más cuanto más cerca, así las bandas se enroscan alrededor del ojo.
    let storm = v3(-0.8, -0.18, 0.55).normalize();
    let (sd, _) = polar_around(q, storm);
    let swirl = 4.5 * (-sd * sd / 0.012).exp();
    q = rot_axis(q, storm, swirl + t * 0.3 * (-sd * sd / 0.01).exp());

    // Capa 1: bandas (zonas claras y cinturones oscuros) distorsionadas por ruido.
    let warp = fbm(v3(q.x * 1.5, q.y * 7.0, q.z * 1.5) + v3(0.0, 0.0, t * 0.02), 4);
    let b = q.y * 3.2 + warp * 0.9;
    let band = (b * PI).sin() * 0.5 + 0.5;
    let fine = ((b * 3.7 + 1.3) * PI).sin() * 0.5 + 0.5;
    let mut col = ramp(
        &[(0.0, 0x6B3A2A), (0.3, 0xB86B3C), (0.5, 0xE3B27A), (0.75, 0xF4E1BE), (1.0, 0xFFF6E6)],
        band * 0.75 + fine * 0.25,
    );

    col = col.lerp(rgb(0x8A9AA5) * (0.7 + 0.4 * band), smoothstep(0.55, 0.85, q.y.abs()) * 0.55);

    // Capa 2: turbulencia: vetas finas estiradas por el viento a lo largo de cada banda.
    let streak = fbm(v3(q.x * 5.0, q.y * 55.0, q.z * 5.0) + v3(t * 0.03, 0.0, 0.0), 4);
    col = col * (0.82 + 0.36 * streak);
    // Pequeños óvalos blancos (tormentas menores).
    let ov = smoothstep(0.83, 0.9, value_noise(v3(q.x * 9.0, q.y * 28.0, q.z * 9.0)) * smoothstep(0.1, 0.0, (q.y + 0.42).abs()));
    col = col.lerp(rgb(0xFFF8EE), ov);

    // Capa 3: la gran tormenta: ojo rojizo con un anillo claro y espiral interna.
    let (d, a) = polar_around(p, storm);
    let oval = d / 0.17;
    let eye = smoothstep(1.0, 0.55, oval);
    let spiral = ((a * 2.0 + oval * 9.0 - t * 1.2).sin() * 0.5 + 0.5) * eye;
    let storm_col = rgb(0xB8442A).lerp(rgb(0xE88A4F), spiral * 0.6);
    col = col.lerp(storm_col, eye);
    col = col.lerp(rgb(0xFFE7C4), smoothstep(0.75, 1.0, oval) * smoothstep(1.25, 1.0, oval) * 0.7);

    // Capa 4: hexágono polar (como el de Saturno): región azulada con borde hexagonal.
    let ang = p.z.atan2(p.x) + t * 0.02;
    let r = (p.x * p.x + p.z * p.z).sqrt();
    let seg = (ang.rem_euclid(PI / 3.0)) - PI / 6.0;
    let hex = r * seg.cos() / (PI / 6.0).cos();
    let cap = if p.y > 0.0 { smoothstep(0.31, 0.28, hex) } else { 0.0 };
    col = col.lerp(rgb(0x5F7E93).lerp(rgb(0x8FB3C4), fbm(p * 12.0, 3)), cap);
    col += rgb(0xFFF2DA) * (smoothstep(0.03, 0.0, (hex - 0.295).abs()) * (p.y > 0.0) as u8 as f32 * 0.35);

    // Iluminación + Capa 5: sombra de los anillos sobre el planeta.
    let light = c.diffuse(0.1) * c.ring_shadow();
    let mut out = col * (0.03 + light);

    // Capa 6: atmósfera: neblina cálida en el borde iluminado.
    out += rgb(0xFFC48A) * (c.fresnel(3.0) * c.diffuse(0.4) * 0.45);
    out
}
