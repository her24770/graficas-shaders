//! Lector de archivos Wavefront OBJ (solo geometría: `v` y `f`) y modelos generados por código.
use crate::math::Vec3;
use std::f32::consts::TAU;

#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
    /// Una normal por vértice para sombreado suave.
    pub normals: Vec<Vec3>,
}

pub fn parse(text: &str) -> Result<Mesh, String> {
    let mut mesh = Mesh::default();
    for (n, line) in text.lines().enumerate() {
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("v") => {
                let c: Vec<f32> = parts.take(3).map(|s| s.parse().map_err(|_| format!("número inválido en la línea {}", n + 1))).collect::<Result<_, _>>()?;
                if c.len() != 3 {
                    return Err(format!("vértice incompleto en la línea {}", n + 1));
                }
                mesh.vertices.push(Vec3::new(c[0], c[1], c[2]));
            }
            Some("f") => {
                let mut idx = Vec::new();
                for tok in parts {
                    let raw: i64 = tok.split('/').next().and_then(|s| s.parse().ok()).ok_or(format!("índice inválido en la línea {}", n + 1))?;
                    let i = if raw > 0 { raw - 1 } else { mesh.vertices.len() as i64 + raw };
                    if i < 0 || i as usize >= mesh.vertices.len() {
                        return Err(format!("índice fuera de rango en la línea {}", n + 1));
                    }
                    idx.push(i as u32);
                }
                // Los polígonos se parten en abanico: (0,1,2), (0,2,3), ...
                for k in 1..idx.len().saturating_sub(1) {
                    mesh.triangles.push([idx[0], idx[k], idx[k + 1]]);
                }
            }
            _ => {}
        }
    }
    if mesh.triangles.is_empty() {
        return Err("el modelo no tiene caras".into());
    }
    mesh.recompute_normals();
    Ok(mesh)
}

impl Mesh {
    /// Recalcula una normal suave por vértice promediando las caras que lo tocan.
    pub fn recompute_normals(&mut self) {
        let mut n = vec![Vec3::ZERO; self.vertices.len()];
        for t in &self.triangles {
            let [a, b, c] = [t[0] as usize, t[1] as usize, t[2] as usize];
            let face = (self.vertices[b] - self.vertices[a]).cross(self.vertices[c] - self.vertices[a]);
            for i in [a, b, c] {
                n[i] += face;
            }
        }
        self.normals = n.into_iter().map(Vec3::normalize).collect();
    }

    /// Anillo plano en el plano XZ, de radio `inner` a `outer`, con caras por arriba y por abajo.
    /// Es un modelo aparte del planeta: se dibuja con su propio shader y transparencia.
    pub fn ring(inner: f32, outer: f32, segments: u32) -> Mesh {
        let mut m = Mesh::default();
        for (side, ny) in [(0u32, 1.0f32), (1, -1.0)] {
            let base = m.vertices.len() as u32;
            for i in 0..segments {
                let a = i as f32 / segments as f32 * TAU;
                let (s, c) = a.sin_cos();
                m.vertices.push(Vec3::new(c * inner, 0.0, s * inner));
                m.vertices.push(Vec3::new(c * outer, 0.0, s * outer));
                m.normals.push(Vec3::new(0.0, ny, 0.0));
                m.normals.push(Vec3::new(0.0, ny, 0.0));
            }
            for i in 0..segments {
                let j = (i + 1) % segments;
                let (a, b, c, d) = (base + 2 * i, base + 2 * i + 1, base + 2 * j + 1, base + 2 * j);
                if side == 0 {
                    m.triangles.push([a, d, c]);
                    m.triangles.push([a, c, b]);
                } else {
                    m.triangles.push([a, c, d]);
                    m.triangles.push([a, b, c]);
                }
            }
        }
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_esfera_del_proyecto_carga_y_es_unitaria() {
        let m = parse(include_str!("../assets/sphere.obj")).unwrap();
        assert!(m.triangles.len() > 1000);
        assert!(m.vertices.iter().all(|v| (v.length() - 1.0).abs() < 1e-3));
        assert!(m.normals.iter().zip(&m.vertices).all(|(n, v)| n.dot(*v) > 0.9), "normales hacia afuera");
    }

    #[test]
    fn parte_cuadrilateros_en_dos_triangulos() {
        let m = parse("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n").unwrap();
        assert_eq!(m.triangles, vec![[0, 1, 2], [0, 2, 3]]);
    }

    #[test]
    fn el_anillo_tiene_dos_caras() {
        let m = Mesh::ring(1.0, 2.0, 8);
        assert_eq!(m.triangles.len(), 32);
    }
}
