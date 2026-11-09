//! Red de ríos y lagos del modo Tierra (adenda 1 §1): la red es un dato, el caudal sale del modelo.
//!
//! - **Red:** las direcciones de flujo de HydroSHEDS a 15″ se reescalan al nivel de carga por píxel
//!   de salida (Eilander et al. 2021). El píxel de salida de una celda es el de mayor área drenada
//!   dentro de ella; desde él se sigue el camino de alta resolución río abajo hasta el píxel de
//!   salida de otra celda, que es la de aguas abajo. Como el área drenada crece siempre río abajo, la
//!   red no tiene ciclos.
//! - **La celda de aguas abajo puede no ser vecina.** La adenda dice que, si no lo es, se tome la
//!   vecina por la que pasa el camino. Pero el camino cruza esa vecina sin pasar por su salida: el
//!   agua acabaría en el río mayor de la vecina, que puede ser de otra cuenca. El motor no necesita
//!   que la celda de aguas abajo sea vecina (encamina por la lista de receptores), así que se enlaza
//!   con la celda cuya salida se alcanza de verdad y las cuencas son exactas. La regla de la adenda se
//!   calcula también ([`Red::vecina`]), solo para medir la diferencia.
//! - **Cuencas cerradas:** si el camino acaba en el fondo de una cuenca cerrada (sin salida al mar)
//!   antes de llegar a otra salida, la celda es terminal: allí el agua se evapora y nunca rebosa a
//!   otra cuenca. Lo mismo si acaba en un mar interior (el Caspio), que no es mar sino lago.
//! - **Mar:** solo lo que tiene profundidad, algún píxel de mar de HydroSHEDS y está conectado con el
//!   océano mundial. La conexión se busca entre las celdas con algún píxel de mar, así que los
//!   estrechos más finos que una celda (Gibraltar, el Bósforo) siguen uniendo; las depresiones
//!   interiores (El Fayum, Qattara, Turfán) y la tierra bajo el nivel del mar (los pólderes) no son
//!   mar. Donde HydroSHEDS no llega (al sur de 56° S y al norte de 84° N) cuenta solo la altura.
//! - **Lagos:** la fracción de lago de cada celda, de los polígonos de HydroLAKES.

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
    /// Acaba en una cuenca cerrada (su fondo o un mar interior).
    pub const CERRADA: u8 = 2;
}

/// Red del nivel de carga.
#[derive(Debug, Clone, Default)]
pub struct Red {
    /// Celda de aguas abajo de cada celda ([`NINGUNA`] si no tiene): la primera cuya salida alcanza el
    /// camino de 15″ que sale de la salida de la celda. Puede no ser vecina.
    pub abajo: Vec<u32>,
    /// Cómo acaba el agua de las que no tienen celda de aguas abajo ([`fin`]).
    pub fin: Vec<u8>,
    /// Píxeles de 15″ que drenan al píxel de salida de la celda (0 si no tiene salida o es de mar).
    pub acum: Vec<u32>,
    /// Píxeles de mar y de tierra de HydroSHEDS de cada celda (los dos a 0: HydroSHEDS no llega).
    pub pix_mar: Vec<u32>,
    pub pix_tierra: Vec<u32>,
    /// La celda de aguas abajo según la letra de la adenda: la misma de [`Red::abajo`] si es vecina y,
    /// si no, la primera celda por la que pasa el camino. Solo para medir cuánto cambia.
    pub vecina: Vec<u32>,
}

/// Códigos de un píxel en memoria, además de los ocho del D8. HydroSHEDS v1.1 marca con 0 todo píxel
/// final, sea la salida al mar o el fondo de una cuenca cerrada, y da el mar como «sin dato» (255); su
/// máscara (`MSK`: 1 tierra, 2 salida al mar, 3 fondo de cuenca cerrada) los distingue. Aquí cada uno
/// lleva su código.
const P_SALIDA: u8 = 0;
const P_FONDO: u8 = 255;
const P_MAR: u8 = 247;

/// Valor de la máscara de HydroSHEDS en el fondo de una cuenca cerrada.
const MSK_FONDO: i64 = 3;

/// Componente de una celda que no ve el mar.
const SIN_COMPONENTE: u32 = u32::MAX;

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

/// Las 8 vecinas de la celda `k` del nivel de `n` celdas por arista (menos en las esquinas del cubo).
fn vecinas(k: usize, n: u32) -> impl Iterator<Item = usize> {
    let nn = (n as usize) * (n as usize);
    let (c, r) = (k / nn, k % nn);
    let (j, i) = ((r / n as usize) as u32, (r % n as usize) as u32);
    (-1i64..=1)
        .flat_map(|dj| (-1i64..=1).map(move |di| (di, dj)))
        .filter(|&d| d != (0, 0))
        .filter_map(move |(di, dj)| neighbor_addr(c as u8, i, j, n, di, dj))
        .map(move |(c2, i2, j2)| indice_de(c2, i2, j2, n))
}

/// `true` si `b` es una de las 8 vecinas de `a` en el nivel de `n` celdas por arista.
pub fn son_vecinas(a: usize, b: usize, n: u32) -> bool {
    vecinas(a, n).any(|x| x == b)
}

/// Componentes conexas (por las 8 vecinas) de las celdas que ven el mar: las que tienen algún píxel de
/// mar de HydroSHEDS y, donde HydroSHEDS no llega, las que están bajo el nivel del mar. Devuelve la
/// componente de cada celda ([`SIN_COMPONENTE`] si no ve el mar) y la del océano mundial, la mayor.
fn componentes_de_mar(z: &[f64], pix_mar: &[u32], pix_tierra: &[u32], n: u32) -> (Vec<u32>, u32) {
    let m = z.len();
    let nodo = |k: usize| pix_mar[k] > 0 || (pix_tierra[k] == 0 && z[k] < 0.0);
    let mut comp = vec![SIN_COMPONENTE; m];
    let mut tam: Vec<usize> = Vec::new();
    let mut cola = std::collections::VecDeque::new();
    for k0 in 0..m {
        if !nodo(k0) || comp[k0] != SIN_COMPONENTE {
            continue;
        }
        let id = tam.len() as u32;
        tam.push(0);
        comp[k0] = id;
        cola.push_back(k0);
        while let Some(k) = cola.pop_front() {
            tam[id as usize] += 1;
            for k2 in vecinas(k, n) {
                if nodo(k2) && comp[k2] == SIN_COMPONENTE {
                    comp[k2] = id;
                    cola.push_back(k2);
                }
            }
        }
    }
    let mundial = tam
        .iter()
        .enumerate()
        .max_by_key(|(i, t)| (**t, std::cmp::Reverse(*i)))
        .map_or(SIN_COMPONENTE, |(i, _)| i as u32);
    (comp, mundial)
}

/// Mar conectado con el océano mundial (adenda 1 §1.4), en el nivel de carga: las celdas bajo el nivel
/// del mar que ven el mar (algún píxel de mar de HydroSHEDS; donde no llega, basta la altura) y están
/// en la componente del océano mundial.
pub fn mar_conectado(z: &[f64], pix_mar: &[u32], pix_tierra: &[u32], n: u32) -> Vec<bool> {
    let (comp, mundial) = componentes_de_mar(z, pix_mar, pix_tierra, n);
    (0..z.len())
        .map(|k| z[k] < 0.0 && comp[k] == mundial && mundial != SIN_COMPONENTE)
        .collect()
}

/// Reescala la red de HydroSHEDS al nivel de `n` celdas por arista. `dir`, `acum` y `mascara` son los
/// GeoTIFF de direcciones, de acumulación y de la máscara (`MSK`) a 15″, y `z` la altura media de cada
/// celda (m). Devuelve la red y el mar conectado con el océano mundial.
pub fn reescalar(
    dir: &Path,
    acum: &Path,
    mascara: &Path,
    z: &[f64],
    n: u32,
) -> Result<(Red, Vec<bool>), TerrenoError> {
    let m = 6 * (n as usize) * (n as usize);
    if z.len() != m {
        return Err(error("HydroSHEDS: la altura no es la del nivel de carga"));
    }
    let ra = Raster::abrir(acum)?;
    let rd = Raster::abrir(dir)?;
    let rm = Raster::abrir(mascara)?;
    let otra_rejilla = |x: &Raster| {
        (x.ancho, x.alto) != (rd.ancho, rd.alto)
            || (x.lon0 - rd.lon0).abs() > 1e-9
            || (x.lat0 - rd.lat0).abs() > 1e-9
            || (x.paso_lon - rd.paso_lon).abs() > 1e-12
    };
    if otra_rejilla(&ra) || otra_rejilla(&rm) || (rd.paso_lon - rd.paso_lat).abs() > 1e-12 {
        return Err(error(
            "HydroSHEDS: las direcciones, la acumulación y la máscara no tienen la misma rejilla de píxeles cuadrados",
        ));
    }
    drop(rm);
    let rejilla = Rejilla {
        ancho: rd.ancho,
        alto: rd.alto,
        lon0: rd.lon0,
        lat0: rd.lat0,
        paso: rd.paso_lon,
        global: (rd.ancho as f64 * rd.paso_lon - 360.0).abs() < 1e-6,
    };
    // Direcciones en memoria (un byte por píxel, con los códigos `P_*`): el camino río abajo salta por
    // toda la rejilla. A la vez, el píxel de salida de cada celda (el de mayor acumulación; en empate,
    // el primero) y los píxeles de mar y de tierra de cada celda. Por bandas de filas, un hilo y un
    // lector por banda: el máximo con desempate y las sumas no dependen del reparto, así que el
    // resultado es el mismo con cualquier número de hilos.
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
    type Banda = Result<(Vec<(i64, usize)>, Vec<u32>, Vec<u32>), TerrenoError>;
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
                    let mut rm = Raster::abrir(mascara)?;
                    let (mut fd, mut fa, mut fm) = (Vec::new(), Vec::new(), Vec::new());
                    let mut mejor = vec![(0i64, usize::MAX); m];
                    let mut pix_mar = vec![0u32; m];
                    let mut pix_tierra = vec![0u32; m];
                    let r0 = b * filas_por_hilo;
                    for (dr, fila) in trozo.chunks_mut(rejilla.ancho).enumerate() {
                        let r = r0 + dr;
                        rd.fila_i64(r, &mut fd)?;
                        ra.fila_i64(r, &mut fa)?;
                        rm.fila_i64(r, &mut fm)?;
                        let lat = (rejilla.lat0 - (r as f64 + 0.5) * rejilla.paso).to_radians();
                        let (cp, sp) = (lat.cos(), lat.sin());
                        for c in 0..rejilla.ancho {
                            let v = fd[c];
                            let d = if Some(v) == sin_dato_dir {
                                P_MAR
                            } else if v == 0 {
                                if fm[c] == MSK_FONDO {
                                    P_FONDO
                                } else {
                                    P_SALIDA
                                }
                            } else if (1..=128).contains(&v) && paso(v as u8).is_some() {
                                v as u8
                            } else {
                                P_MAR
                            };
                            fila[c] = d;
                            let (f, i, j) = locate([cp * cl[c], cp * sl[c], sp], n);
                            let k = indice_de(f, i, j, n);
                            if d == P_MAR {
                                pix_mar[k] += 1;
                                continue;
                            }
                            pix_tierra[k] += 1;
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
                    Ok((mejor, pix_mar, pix_tierra))
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
    let mut pix_tierra = vec![0u32; m];
    for r in resultados {
        let (mb, pm, pt) = r?;
        for k in 0..m {
            let (a, q) = mb[k];
            if q != usize::MAX && (a > mejor[k].0 || (a == mejor[k].0 && q < mejor[k].1)) {
                mejor[k] = (a, q);
            }
            pix_mar[k] += pm[k];
            pix_tierra[k] += pt[k];
        }
    }
    // El mar: las celdas de mar no entran en la red (el camino pasa por ellas sin pararse).
    let (comp, mundial) = componentes_de_mar(z, &pix_mar, &pix_tierra, n);
    let mar: Vec<bool> = (0..m)
        .map(|k| z[k] < 0.0 && comp[k] == mundial && mundial != SIN_COMPONENTE)
        .collect();
    // A qué da un mar: al océano mundial o a un mar interior (el Caspio), que es una cuenca cerrada.
    let fin_del_mar = |k: usize| {
        if comp[k] == SIN_COMPONENTE || comp[k] == mundial {
            fin::MAR
        } else {
            fin::CERRADA
        }
    };
    let salida_de = |k: usize| (!mar[k] && mejor[k].1 != usize::MAX).then_some(mejor[k].1);
    // Desde cada salida, río abajo hasta la salida de otra celda, el mar o el fondo de una cuenca.
    let mut red = Red {
        abajo: vec![NINGUNA; m],
        fin: vec![fin::MAR; m],
        acum: (0..m)
            .map(|k| salida_de(k).map_or(0, |_| mejor[k].0.clamp(0, u32::MAX as i64) as u32))
            .collect(),
        pix_mar,
        pix_tierra,
        vecina: vec![NINGUNA; m],
    };
    for (k, &es_mar) in mar.iter().enumerate() {
        if es_mar {
            continue;
        }
        let Some(q0) = salida_de(k) else {
            // Tierra sin píxeles de tierra de HydroSHEDS: un mar interior (acaba en sí mismo) o la
            // tierra adonde HydroSHEDS no llega (la Antártida), que vierte al mar.
            red.fin[k] = fin_del_mar(k);
            continue;
        };
        let (mut r, mut c) = (q0 / rejilla.ancho, q0 % rejilla.ancho);
        let mut primera: Option<usize> = None;
        let mut pasos = 0usize;
        loop {
            pasos += 1;
            let d = dirs[r * rejilla.ancho + c];
            if d == P_FONDO {
                red.fin[k] = fin::CERRADA;
                break;
            }
            if d == P_SALIDA {
                // La salida al mar: el mar es el de un píxel de mar de al lado.
                let al_lado = D8
                    .iter()
                    .filter_map(|x| rejilla.vecino(r, c, (x.1, x.2)))
                    .find(|&(r2, c2)| dirs[r2 * rejilla.ancho + c2] == P_MAR);
                red.fin[k] = match al_lado {
                    Some((r2, c2)) => fin_del_mar(rejilla.celda(r2, c2, n)),
                    None => fin_del_mar(rejilla.celda(r, c, n)),
                };
                break;
            }
            let Some((r2, c2)) = paso(d).and_then(|s| rejilla.vecino(r, c, s)) else {
                // El borde de la rejilla (o un código que no es del D8): sale al mar.
                red.fin[k] = fin::MAR;
                break;
            };
            let q2 = r2 * rejilla.ancho + c2;
            let k2 = rejilla.celda(r2, c2, n);
            if dirs[q2] == P_MAR {
                red.fin[k] = fin_del_mar(k2);
                break;
            }
            if k2 != k && salida_de(k2).is_some() {
                if primera.is_none() {
                    primera = Some(k2);
                }
                if mejor[k2].1 == q2 {
                    red.abajo[k] = k2 as u32;
                    red.fin[k] = fin::SIGUE;
                    red.vecina[k] = if son_vecinas(k, k2, n) {
                        k2 as u32
                    } else {
                        primera.unwrap_or(k2) as u32
                    };
                    break;
                }
            }
            (r, c) = (r2, c2);
            if pasos > 50_000_000 {
                return Err(error(
                    "HydroSHEDS: un camino río abajo no acaba (¿un ciclo?)",
                ));
            }
        }
    }
    Ok((red, mar))
}

/// Celda terminal de cada celda siguiendo los enlaces `abajo` (ella misma si no tiene).
pub fn terminales(abajo: &[u32]) -> Vec<u32> {
    let m = abajo.len();
    let mut out = vec![NINGUNA; m];
    let mut camino = Vec::new();
    for k0 in 0..m {
        if out[k0] != NINGUNA {
            continue;
        }
        camino.clear();
        let mut k = k0;
        let res = loop {
            if out[k] != NINGUNA {
                break out[k];
            }
            camino.push(k);
            let a = abajo[k];
            if a == NINGUNA || camino.len() > m {
                break k as u32;
            }
            k = a as usize;
        };
        for &c in &camino {
            out[c] = res;
        }
    }
    out
}

/// `true` si el agua de cada celda acaba, siguiendo la red, en una cuenca cerrada.
pub fn acaba_en_cerrada(red: &Red) -> Vec<bool> {
    terminales(&red.abajo)
        .iter()
        .map(|&t| red.fin[t as usize] == fin::CERRADA)
        .collect()
}
