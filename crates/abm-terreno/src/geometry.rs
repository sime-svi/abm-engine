//! Geometría del cubo esférico (equiangular) a **cualquier nivel** del árbol LOD: dirección 3D,
//! latitud, longitud y área real de una celda `(cara, i, j)` con `n` celdas por arista; claves
//! jerárquicas `(cara, nivel, i, j)`; y vecino al mismo nivel (intra-cara, cruzando una arista por
//! el tramo compartido, y esquinas del cubo).
//!
//! Diseño: `incremento-1-diseno.md` §2 (proyección equiangular, por su mejor reparto de área) e
//! `incremento-2-diseno.md` §2 y §5 (claves y vecindad multinivel). Todo es función pura de
//! `(cara, i, j, n)` => coherente a cualquier nivel y reproducible.

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

/// Vecindad de Moore (8 celdas) en orden fijo (mismo orden que la rejilla 2D del motor de forrajeo).
/// Índices ortogonales: 1,3,4,6. Índices diagonales: 0,2,5,7.
pub static MOORE8: [(i64, i64); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// `true` si el vecino `k` de [`MOORE8`] es diagonal (contacto solo por una esquina).
#[inline]
pub fn es_diagonal(k: usize) -> bool {
    matches!(k, 0 | 2 | 5 | 7)
}

/// Vector 3D.
pub type V3 = [f64; 3];

#[inline]
pub fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
pub fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Normaliza `a`; si es nulo devuelve `+X` (no hay NaN posible).
#[inline]
pub fn normalize(a: V3) -> V3 {
    let n = dot(a, a).sqrt();
    if n > 0.0 {
        [a[0] / n, a[1] / n, a[2] / n]
    } else {
        [1.0, 0.0, 0.0]
    }
}

/// Ángulo (rad) entre dos direcciones unitarias. `atan2` es estable también para ángulos pequeños.
#[inline]
pub fn angle_between(a: V3, b: V3) -> f64 {
    let c = cross(a, b);
    dot(c, c).sqrt().atan2(dot(a, b))
}

/// Base local de una cara: normal exterior `n̂` y ejes `â` (i crece), `b̂` (j crece).
/// `(â, b̂, n̂)` es dextrógiro. Ver tabla §2.1 del diseño del incremento 1.
fn face_basis(c: u8) -> (V3, V3, V3) {
    // Devuelve (â, b̂, n̂): eje i, eje j, normal exterior.
    match c {
        0 => ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]), // n=+X, a=+Y, b=+Z
        1 => ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]), // n=+Y, a=-X, b=+Z
        2 => ([0.0, -1.0, 0.0], [0.0, 0.0, 1.0], [-1.0, 0.0, 0.0]), // n=-X, a=-Y, b=+Z
        3 => ([1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]), // n=-Y, a=+X, b=+Z
        4 => ([0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]), // n=+Z, a=+Y, b=-X (polo N)
        _ => ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]), // n=-Z, a=+Y, b=+X (polo S)
    }
}

/// Dirección unitaria del punto equiangular `(fi, fj)` de la cara `c` con `n` celdas por arista.
/// `fi, fj` pueden salirse de `[0, n]` (se usa para localizar celdas de la cara contigua).
pub fn dir_at(c: u8, fi: f64, fj: f64, n: u32) -> V3 {
    let (a, b, nrm) = face_basis(c);
    let step = FRAC_PI_2 / n as f64;
    let xi = -FRAC_PI_4 + fi * step;
    let eta = -FRAC_PI_4 + fj * step;
    let (tx, ty) = (xi.tan(), eta.tan());
    let v = [
        nrm[0] + tx * a[0] + ty * b[0],
        nrm[1] + tx * a[1] + ty * b[1],
        nrm[2] + tx * a[2] + ty * b[2],
    ];
    let l = dot(v, v).sqrt();
    [v[0] / l, v[1] / l, v[2] / l]
}

/// Dirección unitaria del **centro** de la celda `(c, i, j)`.
#[inline]
pub fn center_dir(c: u8, i: u32, j: u32, n: u32) -> V3 {
    dir_at(c, i as f64 + 0.5, j as f64 + 0.5, n)
}

/// Latitud y longitud (radianes) de una dirección unitaria.
#[inline]
pub fn lat_lon(dir: V3) -> (f64, f64) {
    (dir[2].clamp(-1.0, 1.0).asin(), dir[1].atan2(dir[0]))
}

/// Base local este/norte (ENU) en una dirección. En los polos, `cross(z, up) ≈ 0` => este = `+X`.
pub fn east_north(d: V3) -> (V3, V3) {
    let z = [0.0, 0.0, 1.0];
    let mut e = cross(z, d);
    if dot(e, e) < 1e-12 {
        e = [1.0, 0.0, 0.0];
    }
    let e = normalize(e);
    (e, normalize(cross(d, e)))
}

/// Cara y celda `(c, i, j)` que contiene una dirección (inversa de la proyección, §2.4).
pub fn locate(dir: V3, n: u32) -> (u8, u32, u32) {
    let (ax, ay, az) = (dir[0].abs(), dir[1].abs(), dir[2].abs());
    let c = if ax >= ay && ax >= az {
        if dir[0] > 0.0 { 0 } else { 2 }
    } else if ay >= ax && ay >= az {
        if dir[1] > 0.0 { 1 } else { 3 }
    } else if dir[2] > 0.0 {
        4
    } else {
        5
    };
    let (a, b, nrm) = face_basis(c);
    let denom = dot(dir, nrm);
    let xi = (dot(dir, a) / denom).atan();
    let eta = (dot(dir, b) / denom).atan();
    let step = FRAC_PI_2 / n as f64;
    let fi = ((xi + FRAC_PI_4) / step).floor();
    let fj = ((eta + FRAC_PI_4) / step).floor();
    let i = (fi as i64).clamp(0, n as i64 - 1) as u32;
    let j = (fj as i64).clamp(0, n as i64 - 1) as u32;
    (c, i, j)
}

/// Ángulo sólido de un triángulo esférico de vértices unitarios (Van Oosterom–Strackee).
fn tri_solid_angle(a: V3, b: V3, c: V3) -> f64 {
    let num = dot(a, cross(b, c)).abs();
    let den = 1.0 + dot(a, b) + dot(b, c) + dot(c, a);
    2.0 * num.atan2(den)
}

/// Área real (m²) de la celda `(c, i, j)`: ángulo sólido del cuadrilátero esférico · R².
pub fn cell_area(c: u8, i: u32, j: u32, n: u32, r_planet: f64) -> f64 {
    let v0 = dir_at(c, i as f64, j as f64, n);
    let v1 = dir_at(c, (i + 1) as f64, j as f64, n);
    let v2 = dir_at(c, (i + 1) as f64, (j + 1) as f64, n);
    let v3 = dir_at(c, i as f64, (j + 1) as f64, n);
    let omega = tri_solid_angle(v0, v1, v2) + tri_solid_angle(v0, v2, v3);
    r_planet * r_planet * omega
}

// --- Claves jerárquicas (incremento 2 §2) -------------------------------------------------------

const COORD_BITS: u32 = 28;
const COORD_MASK: u64 = (1 << COORD_BITS) - 1;

/// Celdas por arista de cara al nivel `l`: `n0 · 2^l`.
#[inline]
pub fn face_n(n0: u32, l: u32) -> u32 {
    n0 << l
}

/// Clave canónica `u64` de la celda `(c, l, i, j)`: `c` (3 bits) · `l` (5 bits) · `j` (28) · `i` (28).
/// Ordenar por clave = orden `(cara, nivel, j, i)`, el orden canónico de iteración.
#[inline]
pub fn key(c: u8, l: u32, i: u32, j: u32) -> u64 {
    ((c as u64) << 61) | ((l as u64) << 56) | ((j as u64) << COORD_BITS) | (i as u64)
}

/// Descompone una clave en `(c, l, i, j)`.
#[inline]
pub fn split_key(k: u64) -> (u8, u32, u32, u32) {
    (
        (k >> 61) as u8,
        ((k >> 56) & 0x1F) as u32,
        (k & COORD_MASK) as u32,
        ((k >> COORD_BITS) & COORD_MASK) as u32,
    )
}

/// Clave del padre (un nivel menos). Requiere nivel ≥ 1.
#[inline]
pub fn parent_key(k: u64) -> u64 {
    let (c, l, i, j) = split_key(k);
    key(c, l - 1, i / 2, j / 2)
}

/// Clave de la hija `(di, dj) ∈ {0,1}²` (un nivel más).
#[inline]
pub fn child_key(k: u64, di: u32, dj: u32) -> u64 {
    let (c, l, i, j) = split_key(k);
    key(c, l + 1, 2 * i + di, 2 * j + dj)
}

/// Fracción de celda a la que se sitúa, más allá de la arista, el punto que localiza la celda
/// contigua. Pequeña para no caer en la fila vecina por la curvatura de la rejilla cerca de las
/// esquinas de cara, y grande frente a la precisión de `f64`.
const EDGE_EPS: f64 = 1e-3;

/// Vecino al mismo nivel de la celda `(c, i, j)` (`n` celdas por arista) en la dirección `(di, dj)`.
///
/// - Dentro de la cara: trivial.
/// - Cruzando **una** arista: la celda de la cara contigua que **comparte el tramo** de arista. Se
///   localiza el punto a `EDGE_EPS` celdas más allá del borde y en el centro del tramo: las
///   parametrizaciones equiangulares de las dos caras coinciden sobre la arista común (§5).
/// - Cruzando **dos** aristas a la vez: esquina del cubo, sin vecino (`None`).
pub fn neighbor_addr(c: u8, i: u32, j: u32, n: u32, di: i64, dj: i64) -> Option<(u8, u32, u32)> {
    let nn = n as i64;
    let ni = i as i64 + di;
    let nj = j as i64 + dj;
    let in_i = (0..nn).contains(&ni);
    let in_j = (0..nn).contains(&nj);
    match (in_i, in_j) {
        (true, true) => Some((c, ni as u32, nj as u32)),
        (false, false) => None,
        _ => {
            let coord = |v: i64| -> f64 {
                if v < 0 {
                    -EDGE_EPS
                } else if v >= nn {
                    nn as f64 + EDGE_EPS
                } else {
                    v as f64 + 0.5
                }
            };
            Some(locate(dir_at(c, coord(ni), coord(nj), n), n))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_suma_a_esfera() {
        let n = 24;
        let r = 6371000.0;
        let mut total = 0.0;
        for c in 0..6u8 {
            for j in 0..n {
                for i in 0..n {
                    total += cell_area(c, i, j, n, r);
                }
            }
        }
        let esfera = 4.0 * std::f64::consts::PI * r * r;
        assert!((total - esfera).abs() / esfera < 1e-9, "área no cierra");
    }

    #[test]
    fn las_hijas_teselan_a_la_madre() {
        // Las 4 hijas (nivel siguiente) suman el área de la madre: la rejilla equiangular anida.
        let (n, r) = (12, 6371000.0);
        for c in 0..6u8 {
            for (i, j) in [(0, 0), (5, 7), (11, 11), (3, 0)] {
                let madre = cell_area(c, i, j, n, r);
                let hijas: f64 = [(0, 0), (1, 0), (0, 1), (1, 1)]
                    .iter()
                    .map(|&(di, dj)| cell_area(c, 2 * i + di, 2 * j + dj, 2 * n, r))
                    .sum();
                assert!((hijas - madre).abs() / madre < 1e-12, "hijas no teselan");
            }
        }
    }

    #[test]
    fn claves_ida_y_vuelta() {
        for &(c, l, i, j) in &[(0u8, 0u32, 0u32, 0u32), (5, 3, 383, 17), (2, 7, 6143, 6000)] {
            let k = key(c, l, i, j);
            assert_eq!(split_key(k), (c, l, i, j));
            if l > 0 {
                assert_eq!(split_key(parent_key(k)), (c, l - 1, i / 2, j / 2));
            }
            assert_eq!(parent_key(child_key(k, 1, 0)), k);
        }
        // Orden canónico: cara, luego nivel, luego j, luego i.
        assert!(key(0, 1, 0, 0) < key(1, 0, 0, 0));
        assert!(key(0, 0, 5, 5) < key(0, 1, 0, 0));
        assert!(key(0, 0, 9, 0) < key(0, 0, 0, 1));
    }

    #[test]
    fn vecindad_reciproca_y_esquinas() {
        // Con la regla del tramo compartido, la vecindad al mismo nivel es recíproca en las 8
        // direcciones (también las diagonales que cruzan una arista). Solo las 24 celdas-esquina del
        // cubo pierden un vecino (7 en vez de 8).
        let n = 16;
        let mut con_siete = 0;
        for c in 0..6u8 {
            for j in 0..n {
                for i in 0..n {
                    let mut cuenta = 0;
                    for &(di, dj) in MOORE8.iter() {
                        let Some((c2, i2, j2)) = neighbor_addr(c, i, j, n, di, dj) else {
                            continue;
                        };
                        cuenta += 1;
                        let reciproco = MOORE8.iter().any(|&(ei, ej)| {
                            neighbor_addr(c2, i2, j2, n, ei, ej) == Some((c, i, j))
                        });
                        assert!(reciproco, "no recíproco: ({c},{i},{j}) -> ({c2},{i2},{j2})");
                    }
                    match cuenta {
                        8 => {}
                        7 => con_siete += 1,
                        k => panic!("celda ({c},{i},{j}) con {k} vecinos"),
                    }
                }
            }
        }
        assert_eq!(
            con_siete, 24,
            "solo las 24 esquinas del cubo tienen 7 vecinos"
        );
    }
}
