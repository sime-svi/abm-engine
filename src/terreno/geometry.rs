//! Geometría del cubo esférico (equiangular). Convierte `(cara, i, j)` en dirección 3D, latitud,
//! longitud y área real, y precalcula la tabla de vecinos (intra-cara, inter-cara y esquinas).
//!
//! Diseño: `docs/design/terreno/incremento-1-diseno.md` §2. Se usa la proyección **equiangular**
//! (mejor reparto de área que la gnomónica ingenua) y la vecindad entre caras se resuelve **por
//! geometría, una sola vez** (evita tabular a mano las 12 aristas). Todo es función pura de
//! `(cara, i, j, N)` => coherente a cualquier nivel y reproducible.

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

/// `true` si el vecino `k` de [`MOORE8`] es diagonal (pesa `1/√2` en la difusión).
#[inline]
pub fn es_diagonal(k: usize) -> bool {
    matches!(k, 0 | 2 | 5 | 7)
}

type V3 = [f64; 3];

#[inline]
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[inline]
fn norm(a: V3) -> V3 {
    let n = dot(a, a).sqrt();
    [a[0] / n, a[1] / n, a[2] / n]
}

/// Base local de una cara: normal exterior `n̂` y ejes `â` (i crece), `b̂` (j crece).
/// `(â, b̂, n̂)` es dextrógiro. Ver tabla §2.1 del diseño.
fn face_basis(c: u8) -> (V3, V3, V3) {
    // Devuelve (â, b̂, n̂): eje i, eje j, normal exterior. Ver tabla §2.1 del diseño.
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
/// `fi, fj` pueden salirse de `[0, n]` (se usa para los centros de celdas vecinas fantasma).
fn dir_at(c: u8, fi: f64, fj: f64, n: u32) -> V3 {
    let (a, b, nrm) = face_basis(c);
    let step = FRAC_PI_2 / n as f64;
    let xi = -FRAC_PI_4 + fi * step;
    let eta = -FRAC_PI_4 + fj * step;
    let (tx, ty) = (xi.tan(), eta.tan());
    norm([
        nrm[0] + tx * a[0] + ty * b[0],
        nrm[1] + tx * a[1] + ty * b[1],
        nrm[2] + tx * a[2] + ty * b[2],
    ])
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

/// Cara y celda `(c, i, j)` que contiene una dirección (inversa de la proyección, §2.4).
fn locate(dir: V3, n: u32) -> (u8, u32, u32) {
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

/// Índice lineal (id) de la celda `(c, i, j)`: `c·n² + j·n + i`.
#[inline]
pub fn cell_id(c: u8, i: u32, j: u32, n: u32) -> u32 {
    (c as u32) * n * n + j * n + i
}

/// Descompone un id lineal en `(c, i, j)`.
#[inline]
pub fn split_id(id: u32, n: u32) -> (u8, u32, u32) {
    let per_face = n * n;
    let c = (id / per_face) as u8;
    let rem = id % per_face;
    (c, rem % n, rem / n)
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

/// Los 8 vecinos (ids) de la celda `id` en orden de [`MOORE8`]; `-1` si no existe (esquina del cubo).
pub fn neighbors(id: u32, n: u32) -> [i64; 8] {
    let (c, i, j) = split_id(id, n);
    let mut out = [-1_i64; 8];
    for (k, (di, dj)) in MOORE8.iter().enumerate() {
        let ni = i as i64 + di;
        let nj = j as i64 + dj;
        let in_i = ni >= 0 && ni < n as i64;
        let in_j = nj >= 0 && nj < n as i64;
        out[k] = if in_i && in_j {
            cell_id(c, ni as u32, nj as u32, n) as i64
        } else if !in_i && !in_j {
            // Cruza dos aristas a la vez => esquina del cubo: sin vecino diagonal (§2.4).
            -1
        } else {
            // Cruza una sola arista => se localiza en la cara contigua por geometría.
            let dir = dir_at(c, ni as f64 + 0.5, nj as f64 + 0.5, n);
            let (c2, i2, j2) = locate(dir, n);
            cell_id(c2, i2, j2, n) as i64
        };
    }
    out
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
    fn vecindad_ortogonal_es_reciproca() {
        // Los 4 vecinos ORTOGONALES (índices 1,3,4,6 de Moore) deben ser siempre recíprocos: de
        // ellos dependen el flujo del agua y los gradientes. Los diagonales cerca de las esquinas
        // del cubo pueden no serlo (adyacencia diagonal ambigua en las 8 esquinas); se toleran.
        let n = 16;
        let mut diag_no_recip = 0;
        for id in 0..6 * n * n {
            let nbs = neighbors(id, n);
            for (k, &nb) in nbs.iter().enumerate() {
                if nb < 0 {
                    continue;
                }
                let recip = neighbors(nb as u32, n).contains(&(id as i64));
                if super::es_diagonal(k) {
                    if !recip {
                        diag_no_recip += 1;
                    }
                } else {
                    assert!(recip, "ortogonal no recíproco: {id} -[k={k}]-> {nb}");
                }
            }
        }
        // Solo puede haber no-reciprocidad diagonal en el entorno de las 24 celdas-esquina.
        assert!(
            diag_no_recip <= 6 * 4 * 8,
            "demasiadas diagonales no recíprocas: {diag_no_recip}"
        );
    }
}
