//! Lagos de HydroLAKES (Messager et al. 2016) en fracción de cada celda del nivel de carga (adenda 1
//! §1.5). Lectores mínimos de shapefile (polígonos) y de su tabla dBASE.
//!
//! Un lago más pequeño que `area_raster_km2` cae entero en la celda de su desagüe (`Pour_long`,
//! `Pour_lat`): a 26 km la diferencia con su contorno es despreciable. Los grandes se rasterizan por
//! barrido de filas cada `paso_grados`: en cada fila, los cruces del contorno (todas sus partes, par
//! e impar) dan los tramos de dentro. Lo rasterizado de cada lago se escala a su área de HydroLAKES,
//! así que el área total de lagos de la malla es la de la base, salvo donde una celda se llena.

use std::path::Path;

use crate::error::TerrenoError;
use crate::geometry::locate;

fn error(ruta: &Path, m: impl std::fmt::Display) -> TerrenoError {
    TerrenoError::Tierra(format!("{}: {m}", ruta.display()))
}

#[inline]
fn direccion(lat_g: f64, lon_g: f64) -> [f64; 3] {
    let (f, l) = (lat_g.to_radians(), lon_g.to_radians());
    [f.cos() * l.cos(), f.cos() * l.sin(), f.sin()]
}

#[inline]
fn indice_de(c: u8, i: u32, j: u32, n: u32) -> usize {
    ((c as usize * n as usize + j as usize) * n as usize) + i as usize
}

/// Una tabla dBASE leída entera: nombres de los campos y, por registro, sus valores en texto.
struct Dbf {
    campos: Vec<(String, usize, usize)>,
    datos: Vec<u8>,
    cabecera: usize,
    tam_registro: usize,
    n: usize,
}

impl Dbf {
    fn abrir(ruta: &Path) -> Result<Dbf, TerrenoError> {
        let datos = std::fs::read(ruta).map_err(|e| error(ruta, e))?;
        if datos.len() < 32 {
            return Err(error(ruta, "dBASE vacío"));
        }
        let n = u32::from_le_bytes([datos[4], datos[5], datos[6], datos[7]]) as usize;
        let cabecera = u16::from_le_bytes([datos[8], datos[9]]) as usize;
        let tam_registro = u16::from_le_bytes([datos[10], datos[11]]) as usize;
        if cabecera > datos.len() || cabecera + n * tam_registro > datos.len() {
            return Err(error(ruta, "dBASE cortado"));
        }
        let mut campos = Vec::new();
        let (mut p, mut desde) = (32usize, 1usize);
        while p + 32 <= cabecera && datos[p] != 0x0D {
            let nombre: String = datos[p..p + 11]
                .iter()
                .take_while(|&&b| b != 0)
                .map(|&b| b as char)
                .collect();
            let largo = datos[p + 16] as usize;
            campos.push((nombre, desde, largo));
            desde += largo;
            p += 32;
        }
        Ok(Dbf {
            campos,
            datos,
            cabecera,
            tam_registro,
            n,
        })
    }

    fn columna(&self, nombre: &str, ruta: &Path) -> Result<(usize, usize), TerrenoError> {
        self.campos
            .iter()
            .find(|c| c.0 == nombre)
            .map(|c| (c.1, c.2))
            .ok_or_else(|| error(ruta, format!("sin el campo {nombre}")))
    }

    fn numero(&self, registro: usize, (desde, largo): (usize, usize)) -> Option<f64> {
        let p = self.cabecera + registro * self.tam_registro + desde;
        std::str::from_utf8(&self.datos[p..p + largo])
            .ok()?
            .trim()
            .parse()
            .ok()
    }
}

/// Fracción de lago de cada celda del nivel de `n` celdas por arista (`area` en m²), y el área total
/// de HydroLAKES (km²).
pub fn fraccion_de_lagos(
    shp: &Path,
    n: u32,
    area: &[f64],
    area_raster_km2: f64,
    paso_grados: f64,
) -> Result<(Vec<f64>, f64), TerrenoError> {
    let dbf_ruta = shp.with_extension("dbf");
    let dbf = Dbf::abrir(&dbf_ruta)?;
    let (c_area, c_lon, c_lat) = (
        dbf.columna("Lake_area", &dbf_ruta)?,
        dbf.columna("Pour_long", &dbf_ruta)?,
        dbf.columna("Pour_lat", &dbf_ruta)?,
    );
    let b = std::fs::read(shp).map_err(|e| error(shp, e))?;
    if b.len() < 100 || i32::from_be_bytes([b[0], b[1], b[2], b[3]]) != 9994 {
        return Err(error(shp, "no es un shapefile"));
    }
    let tipo = i32::from_le_bytes([b[32], b[33], b[34], b[35]]);
    if !matches!(tipo, 5 | 15 | 25) {
        return Err(error(
            shp,
            format!("tipo de forma {tipo}: se esperaban polígonos"),
        ));
    }
    let mut km2 = vec![0.0f64; area.len()];
    let mut total = 0.0;
    let (mut p, mut registro) = (100usize, 0usize);
    let le_i32 = |q: usize| i32::from_le_bytes([b[q], b[q + 1], b[q + 2], b[q + 3]]);
    let le_f64 = |q: usize| {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[q..q + 8]);
        f64::from_le_bytes(a)
    };
    let mut cruces: Vec<f64> = Vec::new();
    let mut muestras: Vec<(usize, f64)> = Vec::new();
    while p + 8 <= b.len() && registro < dbf.n {
        let largo = 2 * u32::from_be_bytes([b[p + 4], b[p + 5], b[p + 6], b[p + 7]]) as usize;
        let q = p + 8;
        if q + largo > b.len() {
            return Err(error(shp, "registro cortado"));
        }
        let a_lago = dbf.numero(registro, c_area).unwrap_or(0.0).max(0.0);
        total += a_lago;
        if a_lago < area_raster_km2 || largo < 44 || le_i32(q) == 0 {
            let lon = dbf.numero(registro, c_lon).unwrap_or(0.0);
            let lat = dbf.numero(registro, c_lat).unwrap_or(0.0);
            let (c, i, j) = locate(direccion(lat, lon), n);
            km2[indice_de(c, i, j, n)] += a_lago;
        } else {
            let (xmin, ymin, xmax, ymax) = (
                le_f64(q + 4),
                le_f64(q + 12),
                le_f64(q + 20),
                le_f64(q + 28),
            );
            let n_partes = le_i32(q + 36) as usize;
            let n_puntos = le_i32(q + 40) as usize;
            let p_puntos = q + 44 + 4 * n_partes;
            if p_puntos + 16 * n_puntos > q + largo {
                return Err(error(shp, "polígono cortado"));
            }
            let partes: Vec<usize> = (0..n_partes)
                .map(|k| le_i32(q + 44 + 4 * k).max(0) as usize)
                .chain(std::iter::once(n_puntos))
                .collect();
            let punto = |k: usize| (le_f64(p_puntos + 16 * k), le_f64(p_puntos + 16 * k + 8));
            // Barrido de filas: cruces de cada arista con la latitud de la fila.
            muestras.clear();
            let filas = ((ymax - ymin) / paso_grados).ceil().max(1.0) as usize;
            for f in 0..filas {
                let y = ymin + (f as f64 + 0.5) * paso_grados;
                if y > ymax {
                    break;
                }
                cruces.clear();
                for w in partes.windows(2) {
                    let (a0, a1) = (w[0], w[1].min(n_puntos));
                    for k in a0..a1.saturating_sub(1) {
                        let ((x1, y1), (x2, y2)) = (punto(k), punto(k + 1));
                        if (y1 <= y) != (y2 <= y) {
                            cruces.push(x1 + (y - y1) * (x2 - x1) / (y2 - y1));
                        }
                    }
                }
                cruces.sort_by(|a, b| a.total_cmp(b));
                let peso =
                    paso_grados.to_radians().powi(2) * y.to_radians().cos() * 6371.0f64.powi(2);
                for par in cruces.chunks_exact(2) {
                    let c0 = ((par[0] - xmin) / paso_grados - 0.5).ceil().max(0.0) as usize;
                    let mut c = c0;
                    loop {
                        let x = xmin + (c as f64 + 0.5) * paso_grados;
                        if x >= par[1] || x > xmax {
                            break;
                        }
                        let (cara, i, j) = locate(direccion(y, x), n);
                        muestras.push((indice_de(cara, i, j, n), peso));
                        c += 1;
                    }
                }
            }
            let s: f64 = muestras.iter().map(|x| x.1).sum();
            if s > 0.0 {
                for &(k, w) in &muestras {
                    km2[k] += a_lago * w / s;
                }
            } else {
                let lon = dbf.numero(registro, c_lon).unwrap_or(0.0);
                let lat = dbf.numero(registro, c_lat).unwrap_or(0.0);
                let (c, i, j) = locate(direccion(lat, lon), n);
                km2[indice_de(c, i, j, n)] += a_lago;
            }
        }
        p = q + largo;
        registro += 1;
    }
    let frac = km2
        .iter()
        .zip(area)
        .map(|(k, a)| (k * 1e6 / a).min(1.0))
        .collect();
    Ok((frac, total))
}
