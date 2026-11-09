//! Helios — la estrella del sistema. Es emisiva: no depende de la luz, brilla sola.
use super::{ramp, smoothstep, Ctx};
use crate::math::{v3, Vec3};
use crate::noise::{fbm, ridged, voronoi};

pub fn shade(c: &Ctx) -> Vec3 {
    let (p, t) = (c.p, c.t);

    // Capa 1: plasma. Ruido con "domain warping" (el ruido deforma las coordenadas de otro ruido)
    // para que parezcan corrientes de convección que se mueven con el tiempo.
    let q = p * 2.0 + v3(t * 0.05, -t * 0.03, t * 0.02);
    let warp = v3(fbm(q, 3), fbm(q + v3(5.2, 1.3, 2.8), 3), fbm(q + v3(1.7, 9.2, 4.1), 3));
    let plasma = fbm(p * 3.0 + warp * 2.2 + v3(0.0, t * 0.08, 0.0), 5);
    let mut col = ramp(
        &[(0.15, 0x6E0E00), (0.38, 0xD23C00), (0.55, 0xFF8A1A), (0.72, 0xFFD060), (0.9, 0xFFF4D0)],
        plasma,
    );

    // Capa 2: granulación. Celdas de Voronoi: cada celda es una burbuja de gas caliente
    // y los bordes (donde f2 - f1 es casi 0) son el gas más frío que baja.
    let g = voronoi(p * 42.0 + v3(t * 0.06, 0.0, t * 0.05), 11);
    let lanes = smoothstep(0.0, 0.35, g.f2 - g.f1);
    col = col * (0.86 + 0.18 * lanes);

    // Capa 3: manchas solares con umbra (centro oscuro) y penumbra, solo en latitudes medias.
    let band = 1.0 - smoothstep(0.3, 0.6, p.y.abs());
    let s = fbm(p * 5.5 + v3(11.0, 4.0, t * 0.01), 4) * band;
    let pen = smoothstep(0.7, 0.75, s);
    let umb = smoothstep(0.76, 0.8, s);
    col = col.lerp(col * v3(0.5, 0.25, 0.12), pen);
    col = col.lerp(v3(0.12, 0.025, 0.0), umb);

    // Capa 4: fáculas y filamentos magnéticos: crestas brillantes que fluyen.
    let r = ridged(p * 4.0 + warp * 1.5 + v3(0.0, t * 0.15, 0.0), 4);
    let fil = smoothstep(0.55, 0.85, r) * (1.0 - pen);
    col += v3(1.0, 0.75, 0.4) * (fil * 0.55);

    // Capa 5: oscurecimiento de borde (limb darkening): el borde se ve más rojo y tenue,
    // como en el Sol real, porque ahí vemos capas más frías.
    let mu = c.n.dot(c.v).max(0.0);
    col = col * v3(1.0, 0.55, 0.3).lerp(Vec3::ONE, mu.powf(0.5)) * (0.6 + 0.4 * mu.powf(0.35));

    // Capa 6: pulso de brillo lento de toda la estrella.
    col * (1.2 + 0.08 * (t * 1.7).sin())
}
