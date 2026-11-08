//! Ruido procedural propio: valor 3D, fBm, ruido "ridged" y celular (Voronoi).
//! Todos los shaders se construyen con estas piezas; no hay texturas.
use crate::math::Vec3;

fn hash_u(x: i32, y: i32, z: i32, seed: u32) -> u32 {
    let mut h = seed.wrapping_mul(0x9E37_79B9)
        ^ (x as u32).wrapping_mul(0x8DA6_B343)
        ^ (y as u32).wrapping_mul(0xD816_3841)
        ^ (z as u32).wrapping_mul(0xCB1A_B31F);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^ (h >> 15)
}

/// Número pseudoaleatorio en [0, 1] para una celda entera.
pub fn hash(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    (hash_u(x, y, z, seed) & 0xFF_FFFF) as f32 / 16_777_215.0
}

/// Ruido de valor 3D en [0, 1], interpolado con suavizado entre las esquinas de la celda.
pub fn value_noise(p: Vec3) -> f32 {
    let (fx, fy, fz) = (p.x.floor(), p.y.floor(), p.z.floor());
    let (ix, iy, iz) = (fx as i32, fy as i32, fz as i32);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (tx, ty, tz) = (s(p.x - fx), s(p.y - fy), s(p.z - fz));
    let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let c = |dx, dy, dz| hash(ix + dx, iy + dy, iz + dz, 7);
    let x00 = l(c(0, 0, 0), c(1, 0, 0), tx);
    let x10 = l(c(0, 1, 0), c(1, 1, 0), tx);
    let x01 = l(c(0, 0, 1), c(1, 0, 1), tx);
    let x11 = l(c(0, 1, 1), c(1, 1, 1), tx);
    l(l(x00, x10, ty), l(x01, x11, ty), tz)
}

/// Suma de octavas de ruido (fractal Brownian motion). Resultado en [0, 1],
/// estirado para que use casi todo el rango (el promedio de octavas se concentra en 0.5).
pub fn fbm(p: Vec3, octaves: u32) -> f32 {
    let (mut sum, mut amp, mut norm, mut q) = (0.0, 0.5, 0.0, p);
    for _ in 0..octaves {
        sum += amp * value_noise(q);
        norm += amp;
        amp *= 0.5;
        q = Vec3::new(q.y * 1.6 + q.z * 1.2, q.z * 1.6 - q.x * 1.2, q.x * 1.6 + q.y * 1.2) + Vec3::new(17.1, 3.7, 9.2);
    }
    ((sum / norm - 0.5) * 1.7 + 0.5).clamp(0.0, 1.0)
}

/// Ruido "ridged": crestas finas donde el ruido cruza 0.5. Sirve para grietas, cordilleras y filamentos.
pub fn ridged(p: Vec3, octaves: u32) -> f32 {
    let (mut sum, mut amp, mut norm, mut q) = (0.0, 0.5, 0.0, p);
    for _ in 0..octaves {
        let r = 1.0 - (value_noise(q) * 2.0 - 1.0).abs();
        sum += amp * r * r;
        norm += amp;
        amp *= 0.5;
        q = q * 2.1 + Vec3::new(5.3, 1.9, 7.7);
    }
    sum / norm
}

#[derive(Clone, Copy, Debug)]
pub struct Cell {
    /// Distancia al punto más cercano.
    pub f1: f32,
    /// Distancia al segundo más cercano. `f2 - f1` vale 0 en los bordes entre celdas.
    pub f2: f32,
    /// Número aleatorio propio de la celda más cercana.
    pub id: f32,
    /// Posición del punto más cercano.
    pub center: Vec3,
}

/// Ruido celular 3D (Voronoi): cada celda tiene un punto al azar.
pub fn voronoi(p: Vec3, seed: u32) -> Cell {
    let (ix, iy, iz) = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
    let mut best = Cell { f1: 9.0, f2: 9.0, id: 0.0, center: p };
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (cx, cy, cz) = (ix + dx, iy + dy, iz + dz);
                let pt = Vec3::new(
                    cx as f32 + hash(cx, cy, cz, seed),
                    cy as f32 + hash(cx, cy, cz, seed + 1),
                    cz as f32 + hash(cx, cy, cz, seed + 2),
                );
                let d = (pt - p).length();
                if d < best.f1 {
                    best.f2 = best.f1;
                    best.f1 = d;
                    best.id = hash(cx, cy, cz, seed + 3);
                    best.center = pt;
                } else if d < best.f2 {
                    best.f2 = d;
                }
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_ruido_es_determinista_y_en_rango() {
        for i in 0..400 {
            let p = Vec3::new(i as f32 * 0.37, i as f32 * 0.11, i as f32 * 0.73);
            assert_eq!(fbm(p, 4), fbm(p, 4));
            assert!((0.0..=1.0).contains(&fbm(p, 5)));
            assert!((0.0..=1.0).contains(&ridged(p, 4)));
        }
    }

    #[test]
    fn el_ruido_es_continuo() {
        let p = Vec3::new(1.3, 2.7, 0.4);
        assert!((value_noise(p) - value_noise(p + Vec3::new(0.001, 0.0, 0.0))).abs() < 0.01);
    }

    #[test]
    fn voronoi_ordena_las_distancias() {
        for i in 0..100 {
            let c = voronoi(Vec3::new(i as f32 * 0.31, 0.5, i as f32 * 0.17), 1);
            assert!(c.f1 <= c.f2);
        }
    }
}
