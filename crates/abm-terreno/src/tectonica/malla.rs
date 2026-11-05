//! Malla de la tectónica y geometría de las rotaciones.
//!
//! La malla es un nivel del mismo cubo esférico que el terreno (por defecto el nivel 2, ~52 km con
//! `n0 = 48`), con sus celdas en el orden canónico `(cara, j, i)`. Cada placa guarda su corteza en
//! una copia de esta malla en su propio sistema de referencia; para saber qué hay en un punto del
//! mundo se gira el punto al sistema de la placa (§3 del diseño, nota de representación).
//!
//! Las rotaciones se acumulan como cuaterniones unitarios (renormalizados en cada paso): el giro de
//! una placa durante 500 pasos no arrastra error de interpolación.

use crate::geometry::{
    MOORE8, V3, cell_area, center_dir, cross, dot, face_n, locate, neighbor_addr,
};

/// Celda inexistente / sin muestra.
pub const NADA: u32 = u32::MAX;

/// Malla de la tectónica: un nivel uniforme del cubo esférico.
#[derive(Clone)]
pub struct Malla {
    /// Celdas por arista de cara.
    pub n: u32,
    /// Nivel del cubo del terreno al que corresponde (`n = n0 · 2^nivel`).
    pub nivel: u32,
    /// Nº de celdas (`6 n²`).
    pub num: usize,
    /// Radio del planeta (km).
    pub r_km: f64,
    /// Dirección unitaria del centro de cada celda.
    pub dir: Vec<V3>,
    /// Área de cada celda (km²).
    pub area: Vec<f64>,
    /// Vecinas (orden de [`MOORE8`]); [`NADA`] en las 24 esquinas del cubo.
    pub vec: Vec<[u32; 8]>,
    /// Distancia (km) por la superficie a cada vecina.
    pub dvec: Vec<[f64; 8]>,
    /// Lado medio de celda (km): raíz del área media.
    pub lado_km: f64,
}

impl Malla {
    /// Malla del nivel `nivel` del cubo con `n0` celdas por arista en el nivel 0.
    pub fn new(n0: u32, nivel: u32, r_km: f64) -> Self {
        let n = face_n(n0, nivel);
        let num = 6 * (n as usize) * (n as usize);
        let mut dir = Vec::with_capacity(num);
        let mut area = Vec::with_capacity(num);
        let mut vec = Vec::with_capacity(num);
        for c in 0..6u8 {
            for j in 0..n {
                for i in 0..n {
                    dir.push(center_dir(c, i, j, n));
                    area.push(cell_area(c, i, j, n, r_km));
                    let mut v = [NADA; 8];
                    for (k, &(di, dj)) in MOORE8.iter().enumerate() {
                        if let Some((c2, i2, j2)) = neighbor_addr(c, i, j, n, di, dj) {
                            v[k] = Self::idx_n(n, c2, i2, j2) as u32;
                        }
                    }
                    vec.push(v);
                }
            }
        }
        let lado_km = (4.0 * std::f64::consts::PI * r_km * r_km / num as f64).sqrt();
        let dvec = (0..num)
            .map(|a| {
                let mut d = [f64::INFINITY; 8];
                for (j, &b) in vec[a].iter().enumerate() {
                    if b != NADA {
                        d[j] = crate::geometry::angle_between(dir[a], dir[b as usize]) * r_km;
                    }
                }
                d
            })
            .collect();
        Self {
            n,
            nivel,
            num,
            r_km,
            dir,
            area,
            vec,
            dvec,
            lado_km,
        }
    }

    #[inline]
    fn idx_n(n: u32, c: u8, i: u32, j: u32) -> usize {
        (c as usize * n as usize + j as usize) * n as usize + i as usize
    }

    /// Índice de la celda `(cara, i, j)`.
    #[inline]
    pub fn idx(&self, c: u8, i: u32, j: u32) -> usize {
        Self::idx_n(self.n, c, i, j)
    }

    /// Celda que contiene la dirección `d`.
    #[inline]
    pub fn localizar(&self, d: V3) -> usize {
        let (c, i, j) = locate(d, self.n);
        self.idx(c, i, j)
    }

    /// Distancia (km) por la superficie entre los centros de dos celdas.
    #[inline]
    pub fn dist_km(&self, a: usize, b: usize) -> f64 {
        crate::geometry::angle_between(self.dir[a], self.dir[b]) * self.r_km
    }

    /// Área total (km²).
    pub fn area_total(&self) -> f64 {
        self.area.iter().sum()
    }
}

// --- Cuaterniones ------------------------------------------------------------------------------

/// Cuaternión unitario `w + x i + y j + z k`: una rotación del espacio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Quat {
    pub const IDENTIDAD: Quat = Quat {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    /// Rotación de vector `omega · dt` (eje = dirección de `omega`, ángulo = su módulo, rad).
    pub fn de_rotacion(omega: V3, dt: f64) -> Quat {
        let ang = dot(omega, omega).sqrt() * dt;
        if ang < 1e-300 {
            return Quat::IDENTIDAD;
        }
        let s = (0.5 * ang).sin() / (ang / dt);
        Quat {
            w: (0.5 * ang).cos(),
            x: omega[0] * s,
            y: omega[1] * s,
            z: omega[2] * s,
        }
    }

    /// Producto `self ⊗ o` (primero `o`, luego `self`).
    pub fn componer(self, o: Quat) -> Quat {
        Quat {
            w: self.w * o.w - self.x * o.x - self.y * o.y - self.z * o.z,
            x: self.w * o.x + self.x * o.w + self.y * o.z - self.z * o.y,
            y: self.w * o.y - self.x * o.z + self.y * o.w + self.z * o.x,
            z: self.w * o.z + self.x * o.y - self.y * o.x + self.z * o.w,
        }
    }

    /// El mismo cuaternión con norma 1.
    pub fn normalizado(self) -> Quat {
        let n = (self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        if n > 0.0 {
            Quat {
                w: self.w / n,
                x: self.x / n,
                y: self.y / n,
                z: self.z / n,
            }
        } else {
            Quat::IDENTIDAD
        }
    }

    /// Matriz de rotación 3×3 (por filas).
    pub fn matriz(self) -> Mat3 {
        let Quat { w, x, y, z } = self;
        [
            [
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y - w * z),
                2.0 * (x * z + w * y),
            ],
            [
                2.0 * (x * y + w * z),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z - w * x),
            ],
            [
                2.0 * (x * z - w * y),
                2.0 * (y * z + w * x),
                1.0 - 2.0 * (x * x + y * y),
            ],
        ]
    }
}

/// Matriz 3×3 por filas.
pub type Mat3 = [[f64; 3]; 3];

/// `m · v`.
#[inline]
pub fn aplicar(m: &Mat3, v: V3) -> V3 {
    [dot(m[0], v), dot(m[1], v), dot(m[2], v)]
}

/// `mᵀ · v` (la rotación inversa).
#[inline]
pub fn aplicar_t(m: &Mat3, v: V3) -> V3 {
    [
        m[0][0] * v[0] + m[1][0] * v[1] + m[2][0] * v[2],
        m[0][1] * v[0] + m[1][1] * v[1] + m[2][1] * v[2],
        m[0][2] * v[0] + m[1][2] * v[1] + m[2][2] * v[2],
    ]
}

/// Velocidad de la superficie (km/Myr) en `x` para una placa que gira con `omega` (rad/Myr).
#[inline]
pub fn velocidad(omega: V3, x: V3, r_km: f64) -> V3 {
    let v = cross(omega, x);
    [v[0] * r_km, v[1] * r_km, v[2] * r_km]
}

/// Inversa de una matriz simétrica 3×3; `None` si es casi singular.
pub fn inversa_simetrica(m: &Mat3) -> Option<Mat3> {
    let a = m;
    let c00 = a[1][1] * a[2][2] - a[1][2] * a[2][1];
    let c01 = a[1][2] * a[2][0] - a[1][0] * a[2][2];
    let c02 = a[1][0] * a[2][1] - a[1][1] * a[2][0];
    let det = a[0][0] * c00 + a[0][1] * c01 + a[0][2] * c02;
    let escala = a[0][0].abs() + a[1][1].abs() + a[2][2].abs();
    // `<=` falla también con NaN (det o escala no finitos): se trata como singular.
    if det.abs().partial_cmp(&(1e-12 * escala * escala * escala))
        != Some(std::cmp::Ordering::Greater)
    {
        return None;
    }
    let inv = 1.0 / det;
    Some([
        [
            c00 * inv,
            (a[0][2] * a[2][1] - a[0][1] * a[2][2]) * inv,
            (a[0][1] * a[1][2] - a[0][2] * a[1][1]) * inv,
        ],
        [
            c01 * inv,
            (a[0][0] * a[2][2] - a[0][2] * a[2][0]) * inv,
            (a[0][2] * a[1][0] - a[0][0] * a[1][2]) * inv,
        ],
        [
            c02 * inv,
            (a[0][1] * a[2][0] - a[0][0] * a[2][1]) * inv,
            (a[0][0] * a[1][1] - a[0][1] * a[1][0]) * inv,
        ],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::normalize;

    #[test]
    fn la_malla_cubre_la_esfera_y_sus_vecinas_son_reciprocas() {
        let m = Malla::new(4, 1, 6371.0);
        let esfera = 4.0 * std::f64::consts::PI * 6371.0f64.powi(2);
        assert!((m.area_total() - esfera).abs() / esfera < 1e-9);
        for a in 0..m.num {
            for &b in m.vec[a].iter().filter(|&&b| b != NADA) {
                assert!(
                    m.vec[b as usize].contains(&(a as u32)),
                    "vecindad no recíproca"
                );
            }
            assert_eq!(
                m.localizar(m.dir[a]),
                a,
                "el centro de una celda cae en ella"
            );
        }
    }

    #[test]
    fn un_cuaternion_gira_como_su_eje_y_su_angulo() {
        let eje = normalize([0.3, -0.5, 0.8]);
        let q = Quat::de_rotacion([eje[0] * 0.2, eje[1] * 0.2, eje[2] * 0.2], 1.0);
        let m = q.matriz();
        // El eje no se mueve y un vector perpendicular gira 0,2 rad.
        let e2 = aplicar(&m, eje);
        assert!((0..3).all(|k| (e2[k] - eje[k]).abs() < 1e-12));
        let p = normalize(cross(eje, [1.0, 0.0, 0.0]));
        let p2 = aplicar(&m, p);
        assert!((dot(p, p2) - 0.2f64.cos()).abs() < 1e-12);
        // La traspuesta deshace el giro.
        let p3 = aplicar_t(&m, p2);
        assert!((0..3).all(|k| (p3[k] - p[k]).abs() < 1e-12));
        // Componer dos medios giros = un giro.
        let h = Quat::de_rotacion([eje[0] * 0.1, eje[1] * 0.1, eje[2] * 0.1], 1.0);
        let q2 = h.componer(h).matriz();
        let p4 = aplicar(&q2, p);
        assert!((0..3).all(|k| (p4[k] - p2[k]).abs() < 1e-12));
    }

    #[test]
    fn inversa_de_una_simetrica() {
        let m = [[4.0, 1.0, 0.5], [1.0, 3.0, 0.2], [0.5, 0.2, 2.0]];
        let inv = inversa_simetrica(&m).expect("invertible");
        for (i, fila) in m.iter().enumerate() {
            for j in 0..3 {
                let s: f64 = fila.iter().zip(&inv).map(|(a, f)| a * f[j]).sum();
                assert!((s - if i == j { 1.0 } else { 0.0 }).abs() < 1e-12);
            }
        }
    }
}
