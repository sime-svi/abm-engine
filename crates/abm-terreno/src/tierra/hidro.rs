//! Red de ríos y lagos del modo Tierra (adenda 1 §1): la red es un dato, el caudal sale del modelo.
//!
//! - **Red:** las direcciones de flujo de HydroSHEDS a 15″ se reescalan al nivel de carga por píxel
//!   de salida (Eilander et al. 2021). El píxel de salida de una celda es el de mayor área drenada
//!   dentro de ella; desde él se sigue el camino de alta resolución río abajo hasta el píxel de
//!   salida de otra celda, que es la de aguas abajo. Si no es vecina, se toma la vecina por la que
//!   pasa el camino. Como el área drenada crece siempre río abajo, la red no tiene ciclos.
//! - **Cuencas cerradas:** si el camino acaba en el fondo de una cuenca cerrada (sin salida al mar)
//!   antes de llegar a otra salida, la celda es terminal: allí el agua se evapora y nunca rebosa a
//!   otra cuenca.
//! - **Mar:** solo lo que está bajo el nivel del mar y conectado con el océano mundial. La conexión se
//!   busca entre las celdas con algún píxel de mar de HydroSHEDS (o bajo el nivel del mar donde
//!   HydroSHEDS no llega), así que los estrechos más finos que una celda (Gibraltar, el Bósforo)
//!   siguen uniendo; las depresiones interiores (El Fayum, Qattara, Turfán) no son mar.
//! - **Lagos:** la fracción de lago de cada celda, de los polígonos de HydroLAKES.

use std::collections::HashMap;
use std::path::Path;

use super::geotiff::Raster;
use crate::error::TerrenoError;
use crate::geometry::{locate, neighbor_addr};

fn error(m: impl std::fmt::Display) -> TerrenoError {
    TerrenoError::Tierra(m.to_string())
}

/// Sin celda de aguas abajo.
pub const NINGUNA: u32 = u32::MAX;

/// Cómo acaba el agua de una celda sin celda de aguas abajo.
pub mod fin {
    /// Tiene celda de aguas abajo.
    pub const SIGUE: u8 = 0;
    /// Llega al mar.
    pub const MAR: u8 = 1;
    /// Acaba en una cuenca cerrada.
    pub const CERRADA: u8 = 2;
}

/// Red del nivel de carga.
#[derive(Debug, Clone, Default)]
pub struct Red {
    /// Celda de aguas abajo de cada celda ([`NINGUNA`] si no tiene).
    pub abajo: Vec<u32>,
    /// Cómo acaba el agua de las que no tienen celda de aguas abajo ([`fin`]).
    pub fin: Vec<u8>,
    /// Píxeles de 15″ que drenan al píxel de salida de la celda (0 si la celda no tiene ninguno).
    pub acum: Vec<u32>,
    /// Píxeles de mar de HydroSHEDS de cada celda.
    pub pix_mar: Vec<u32>,
}

/// Códigos de las direcciones de flujo de HydroSHEDS (D8 de ESRI): `(código, dfila, dcolumna)`.
const D8: [(u8, i64, i64); 8] = [
    (1, 0, 1),
    (2, 1, 1),
    (4, 1, 0),
    (8, 1, -1),
    (16, 0, -1),
    (32, -1, -1),
    (64, -1, 0),
    (128, -1, 1),
];

#[inline]
fn paso(d: u8) -> Option<(i64, i64)> {
    D8.iter().find(|x| x.0 == d).map(|x| (x.1, x.2))
}

#[inline]
fn indice_de(c: u8, i: u32, j: u32, n: u32) -> usize {
    ((c as usize * n as usize + j as usize) * n as usize) + i as usize
}

#[inline]
fn direccion(lat_g: f64, lon_g: f64) -> [f64; 3] {
    let (f, l) = (lat_g.to_radians(), lon_g.to_radians());
    [f.cos() * l.cos(), f.cos() * l.sin(), f.sin()]
}

/// Rejilla de HydroSHEDS: geometría de sus píxeles.
#[derive(Clone, Copy)]
struct Rejilla {
    ancho: usize,
    alto: usize,
    lon0: f64,
    lat0: f64,
    paso: f64,
    global: bool,
}

impl Rejilla {
    #[inline]
    fn celda(&self, r: usize, c: usize, n: u32) -> usize {
        let lat = self.lat0 - (r as f64 + 0.5) * self.paso;
        let lon = self.lon0 + (c as f64 + 0.5) * self.paso;
        let (f, i, j) = locate(direccion(lat, lon), n);
        indice_de(f, i, j, n)
    }

    /// Píxel vecino en la dirección `d` (la longitud da la vuelta si la rejilla es global).
    #[inline]
    fn vecino(&self, r: usize, c: usize, (dr, dc): (i64, i64)) -> Option<(usize, usize)> {
        let r2 = r as i64 + dr;
        if r2 < 0 || r2 >= self.alto as i64 {
            return None;
        }
        let mut c2 = c as i64 + dc;
        if self.global {
            c2 = c2.rem_euclid(self.ancho as i64);
        } else if c2 < 0 || c2 >= self.ancho as i64 {
            return None;
        }
        Some((r2 as usize, c2 as usize))
    }
}

/// `true` si `b` es una de las 8 vecinas de `a` en el nivel de `n` celdas por arista.
fn son_vecinas(a: usize, b: usize, n: u32) -> bool {
    let nn = (n as usize) * (n as usize);
    let (c, r) = (a / nn, a % nn);
    let (j, i) = ((r / n as usize) as u32, (r % n as usize) as u32);
    for dj in -1i64..=1 {
        for di in -1i64..=1 {
            if (di, dj) == (0, 0) {
                continue;
            }
            if let Some((c2, i2, j2)) = neighbor_addr(c as u8, i, j, n, di, dj)
                && indice_de(c2, i2, j2, n) == b
            {
                return true;
            }
        }
    }
    false
}

/// Reescala la red de HydroSHEDS al nivel de `n` celdas por arista. `dir` y `acum` son los GeoTIFF de
/// direcciones y de acumulación a 15″.
pub fn reescalar(dir: &Path, acum: &Path, n: u32) -> Result<Red, TerrenoError> {
    let m = 6 * (n as usize) * (n as usize);
    let ra = Raster::abrir(acum)?;
    let rd = Raster::abrir(dir)?;
    if (ra.ancho, ra.alto) != (rd.ancho, rd.alto)
        || (ra.lon0 - rd.lon0).abs() > 1e-9
        || (ra.lat0 - rd.lat0).abs() > 1e-9
        || (ra.paso_lon - rd.paso_lon).abs() > 1e-12
        || (rd.paso_lon - rd.paso_lat).abs() > 1e-12
    {
        return Err(error(
            "HydroSHEDS: las direcciones y la acumulación no tienen la misma rejilla de píxeles cuadrados",
        ));
    }
    let rejilla = Rejilla {
        ancho: rd.ancho,
        alto: rd.alto,
        lon0: rd.lon0,
        lat0: rd.lat0,
        paso: rd.paso_lon,
        global: (rd.ancho as f64 * rd.paso_lon - 360.0).abs() < 1e-6,
    };
    // Direcciones en memoria (un byte por píxel; el mar va como 247, que ningún código de HydroSHEDS
    // usa): el camino río abajo salta por toda la rejilla. A la vez, el píxel de salida de cada celda
    // (el de mayor acumulación; en empate, el primero) y los píxeles de mar de cada celda. Por bandas
    // de filas, un hilo y un lector por banda: el máximo con desempate y las sumas no dependen del
    // reparto, así que el resultado es el mismo con cualquier número de hilos.
    let mut dirs = vec![0u8; rejilla.ancho * rejilla.alto];
    let (sin_dato_dir, sin_dato_acum) =
        (rd.sin_dato.map(|x| x as i64), ra.sin_dato.map(|x| x as i64));
    drop((ra, rd));
    let hilos = std::thread::available_parallelism()
        .map_or(1, |x| x.get())
        .clamp(1, 16);
    let filas_por_hilo = rejilla.alto.div_ceil(hilos);
    // Trigonometría de cada columna (una vez) para localizar los píxeles deprisa.
    let (cl, sl): (Vec<f64>, Vec<f64>) = (0..rejilla.ancho)
        .map(|c| {
            let lon = (rejilla.lon0 + (c as f64 + 0.5) * rejilla.paso).to_radians();
            (lon.cos(), lon.sin())
        })
        .unzip();
    type Banda = Result<(Vec<(i64, usize)>, Vec<u32>), TerrenoError>;
    let mut resultados: Vec<Banda> = Vec::new();
    std::thread::scope(|sc| {
        let partes: Vec<_> = dirs
            .chunks_mut(filas_por_hilo * rejilla.ancho)
            .enumerate()
            .map(|(b, trozo)| {
                let (cl, sl) = (&cl, &sl);
                sc.spawn(move || -> Banda {
                    let mut rd = Raster::abrir(dir)?;
                    let mut ra = Raster::abrir(acum)?;
                    let (mut fd, mut fa) = (Vec::new(), Vec::new());
                    let mut mejor = vec![(0i64, usize::MAX); m];
                    let mut pix_mar = vec![0u32; m];
                    let r0 = b * filas_por_hilo;
                    for (dr, fila) in trozo.chunks_mut(rejilla.ancho).enumerate() {
                        let r = r0 + dr;
                        rd.fila_i64(r, &mut fd)?;
                        ra.fila_i64(r, &mut fa)?;
                        let lat = (rejilla.lat0 - (r as f64 + 0.5) * rejilla.paso).to_radians();
                        let (cp, sp) = (lat.cos(), lat.sin());
                        for c in 0..rejilla.ancho {
                            let v = fd[c];
                            let d = if Some(v) == sin_dato_dir || !(0..=255).contains(&v) {
                                247
                            } else {
                                let d = v as u8;
                                if d == 0 || d == 255 || paso(d).is_some() {
                                    d
                                } else {
                                    247
                                }
                            };
                            fila[c] = d;
                            let (f, i, j) = locate([cp * cl[c], cp * sl[c], sp], n);
                            let k = indice_de(f, i, j, n);
                            if d == 247 {
                                pix_mar[k] += 1;
                                continue;
                            }
                            let a = fa[c];
                            if Some(a) == sin_dato_acum || a < 0 {
                                continue;
                            }
                            let q = r * rejilla.ancho + c;
                            if a > mejor[k].0 || (a == mejor[k].0 && q < mejor[k].1) {
                                mejor[k] = (a, q);
                            }
                        }
                    }
                    Ok((mejor, pix_mar))
                })
            })
            .collect();
        for p in partes {
            resultados.push(
                p.join()
                    .unwrap_or_else(|_| Err(error("HydroSHEDS: un hilo de lectura falló"))),
            );
        }
    });
    let mut mejor = vec![(0i64, usize::MAX); m];
    let mut pix_mar = vec![0u32; m];
    for r in resultados {
        let (mb, pb) = r?;
        for k in 0..m {
            let (a, q) = mb[k];
            if q != usize::MAX && (a > mejor[k].0 || (a == mejor[k].0 && q < mejor[k].1)) {
                mejor[k] = (a, q);
            }
            pix_mar[k] += pb[k];
        }
    }
    let salida: HashMap<usize, usize> = mejor
        .iter()
        .enumerate()
        .filter(|(_, x)| x.1 != usize::MAX)
        .map(|(k, x)| (x.1, k))
        .collect();
    // Desde cada salida, río abajo hasta la salida de otra celda, el mar o el fondo de una cuenca.
    let mut red = Red {
        abajo: vec![NINGUNA; m],
        fin: vec![fin::MAR; m],
        acum: mejor.iter().map(|x| x.0.max(0) as u32).collect(),
        pix_mar,
    };
    for (k, &(_, q0)) in mejor.iter().enumerate() {
        if q0 == usize::MAX {
            continue;
        }
        let (mut r, mut c) = (q0 / rejilla.ancho, q0 % rejilla.ancho);
        let mut primera_vecina: Option<usize> = None;
        let mut pasos = 0usize;
        loop {
            pasos += 1;
            let d = dirs[r * rejilla.ancho + c];
            let siguiente = paso(d).and_then(|s| rejilla.vecino(r, c, s));
            let Some((r2, c2)) = siguiente else {
                // 0: sale al mar; 255: fondo de una cuenca cerrada. Si acaba en otra celda sin pasar
                // por su salida, el agua no puede seguir esa otra red (rebosaría): termina aquí.
                red.fin[k] = if d == 255 { fin::CERRADA } else { fin::MAR };
                break;
            };
            let q2 = r2 * rejilla.ancho + c2;
            if dirs[q2] == 247 {
                red.fin[k] = fin::MAR;
                break;
            }
            let k2 = rejilla.celda(r2, c2, n);
            if k2 != k && primera_vecina.is_none() {
                primera_vecina = Some(k2);
            }
            if k2 != k
                && let Some(&kd) = salida.get(&q2)
            {
                red.abajo[k] = if son_vecinas(k, kd, n) {
                    kd as u32
                } else {
                    primera_vecina.unwrap_or(kd) as u32
                };
                red.fin[k] = fin::SIGUE;
                break;
            }
            (r, c) = (r2, c2);
            if pasos > 50_000_000 {
                return Err(error(
                    "HydroSHEDS: un camino río abajo no acaba (¿un ciclo?)",
                ));
            }
        }
    }
    Ok(red)
}

/// Mar conectado con el océano mundial (adenda 1 §1.4), en el nivel de carga: bajo el nivel del mar
/// y en la mayor de las componentes que forman las celdas bajo el nivel del mar o con algún píxel de
/// mar de HydroSHEDS. Donde HydroSHEDS no llega (la Antártida), cuenta solo la altura.
pub fn mar_conectado(z: &[f64], pix_mar: &[u32], n: u32) -> Vec<bool> {
    let m = z.len();
    let nodo: Vec<bool> = (0..m).map(|k| z[k] < 0.0 || pix_mar[k] > 0).collect();
    let mut comp = vec![u32::MAX; m];
    let mut tam: Vec<usize> = Vec::new();
    let mut cola = std::collections::VecDeque::new();
    let nn = (n as usize) * (n as usize);
    for k0 in 0..m {
        if !nodo[k0] || comp[k0] != u32::MAX {
            continue;
        }
        let id = tam.len() as u32;
        tam.push(0);
        comp[k0] = id;
        cola.push_back(k0);
        while let Some(k) = cola.pop_front() {
            tam[id as usize] += 1;
            let (c, r) = (k / nn, k % nn);
            let (j, i) = ((r / n as usize) as u32, (r % n as usize) as u32);
            for dj in -1i64..=1 {
                for di in -1i64..=1 {
                    if (di, dj) == (0, 0) {
                        continue;
                    }
                    let Some((c2, i2, j2)) = neighbor_addr(c as u8, i, j, n, di, dj) else {
                        continue;
                    };
                    let k2 = indice_de(c2, i2, j2, n);
                    if nodo[k2] && comp[k2] == u32::MAX {
                        comp[k2] = id;
                        cola.push_back(k2);
                    }
                }
            }
        }
    }
    let mundial = tam
        .iter()
        .enumerate()
        .max_by_key(|(i, t)| (**t, std::cmp::Reverse(*i)))
        .map_or(u32::MAX, |(i, _)| i as u32);
    (0..m).map(|k| z[k] < 0.0 && comp[k] == mundial).collect()
}

/// `true` si el agua de cada celda acaba, siguiendo la red, en una cuenca cerrada.
pub fn acaba_en_cerrada(red: &Red) -> Vec<bool> {
    let m = red.abajo.len();
    let mut out: Vec<Option<bool>> = vec![None; m];
    let mut camino = Vec::new();
    for k0 in 0..m {
        if out[k0].is_some() {
            continue;
        }
        camino.clear();
        let mut k = k0;
        let res = loop {
            if let Some(v) = out[k] {
                break v;
            }
            camino.push(k);
            let a = red.abajo[k];
            if a == NINGUNA || camino.len() > m {
                break red.fin[k] == fin::CERRADA;
            }
            k = a as usize;
        };
        for &c in &camino {
            out[c] = Some(res);
        }
    }
    out.into_iter().map(|x| x.unwrap_or(false)).collect()
}
