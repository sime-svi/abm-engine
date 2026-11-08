//! Carga del modo Tierra (`modo-tierra-diseno.md` §2.2): proyecta ETOPO 2022, CHELSA V2.1 y CRU TS
//! 4.09 a la malla del nivel de carga, guarda el resultado en la caché y calcula los niveles gruesos
//! como medias por área de sus hijas (la conservación entre niveles es exacta, salvo el redondeo).
//!
//! Cada punto de las fuentes cae en una celda (la que contiene su centro) y pesa `cos φ` (su área en
//! la esfera): el valor de una celda es la media ponderada de sus puntos. Todo se recorre en un orden
//! fijo, así que el resultado es el mismo bit a bit en cada carga. CHELSA se lee con varios hilos, un
//! archivo por hilo: cada archivo suma en sus propios acumuladores, en el orden de sus filas, así que
//! el reparto de los hilos no cambia nada.

use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::geotiff::GeoTiff;
use super::hidro::{self, Red};
use super::netcdf3::{Gzip, Netcdf3, anio_mes_desde_1900};
use super::{Cru, DatosTierra, Nivel};
use crate::config::TerrenoConfig;
use crate::error::TerrenoError;
use crate::geometry::{angle_between, cell_area, center_dir, face_n, lat_lon, locate};

const MAGIA: &[u8; 8] = b"ABMTIE03";
const HUELLA_CODIGO: &str = env!("ABM_TERRENO_FINGERPRINT");

/// ETOPO 2022 a 2': filas y columnas, primera fila de la franja polar norte de la roca (latitud
/// 59° N en filas de 1', leída cada 2) y filas de las dos franjas (`herramientas/analisis/
/// referencia_tierra.py`).
const ETOPO_FILAS: usize = 5400;
const ETOPO_COLS: usize = 10800;
const ETOPO_FILA_NORTE: usize = 149 * 60 / 2;
const ETOPO_FILAS_SUR: usize = 30 * 60 / 2;
const ETOPO_FILAS_NORTE: usize = ETOPO_FILAS - ETOPO_FILA_NORTE;

fn error(m: impl std::fmt::Display) -> TerrenoError {
    TerrenoError::Tierra(m.to_string())
}

fn fnv1a(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

/// Los 24 GeoTIFF de CHELSA: `tas` (meses 1–12) y después `pr`.
fn rutas_chelsa(cfg: &TerrenoConfig) -> Vec<PathBuf> {
    let mut v = Vec::with_capacity(24);
    for var in ["tas", "pr"] {
        for m in 1..=12 {
            v.push(
                Path::new(&cfg.tierra.chelsa)
                    .join(format!("CHELSA_{var}_{m:02}_1981-2010_V.2.1.tif")),
            );
        }
    }
    v
}

fn fuentes(cfg: &TerrenoConfig) -> Vec<PathBuf> {
    let t = &cfg.tierra;
    let mut v: Vec<PathBuf> = [
        &t.etopo_superficie,
        &t.etopo_roca_sur,
        &t.etopo_roca_norte,
        &t.cru_tmp,
        &t.cru_pre,
        &t.rgi,
        &t.hydrosheds_dir,
        &t.hydrosheds_acc,
        &t.hydrolakes,
    ]
    .iter()
    .map(PathBuf::from)
    .collect();
    v.extend(rutas_chelsa(cfg));
    v.push(Path::new(&t.hydrolakes).with_extension("dbf"));
    v
}

/// Huella de los datos: la del código, la malla, el nivel de carga y la identidad de cada fuente
/// (ruta, tamaño y fecha del archivo). Solo decide si la caché vale; no entra en ningún cálculo.
pub fn huella(cfg: &TerrenoConfig) -> Result<u64, TerrenoError> {
    let mut h = fnv1a(0xcbf2_9ce4_8422_2325, HUELLA_CODIGO.as_bytes());
    h = fnv1a(h, &cfg.planet.n0.to_le_bytes());
    h = fnv1a(h, &cfg.tierra.nivel_carga.to_le_bytes());
    h = fnv1a(h, &cfg.tierra.hielo_min_m.to_le_bytes());
    h = fnv1a(h, &cfg.tierra.lagos_raster_km2.to_le_bytes());
    h = fnv1a(h, &cfg.tierra.lagos_paso_grados.to_le_bytes());
    for r in fuentes(cfg) {
        let m = std::fs::metadata(&r).map_err(|e| {
            error(format!(
                "falta la fuente {} ({e}); se baja con herramientas/tierra/descargar.sh",
                r.display()
            ))
        })?;
        let fecha = m
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs());
        // La ruta absoluta: la misma fuente da la misma huella se llame desde donde se llame.
        let r = std::fs::canonicalize(&r).unwrap_or(r);
        h = fnv1a(h, r.to_string_lossy().as_bytes());
        h = fnv1a(h, &m.len().to_le_bytes());
        h = fnv1a(h, &fecha.to_le_bytes());
    }
    Ok(h)
}

/// Carga los datos de la Tierra: de la caché si vale, si no de las fuentes (y se guardan).
pub fn cargar(cfg: &TerrenoConfig) -> Result<Arc<DatosTierra>, TerrenoError> {
    let h = huella(cfg)?;
    let l = cfg.tierra.nivel_carga;
    let ruta = (!cfg.run.genesis_cache.is_empty())
        .then(|| Path::new(&cfg.run.genesis_cache).join(format!("tierra_n{l}_{h:016x}.bin")));
    if let Some(r) = &ruta
        && let Ok(b) = std::fs::read(r)
        && let Some(d) = decodificar(&b, cfg, h)
    {
        return Ok(Arc::new(d));
    }
    let n = face_n(cfg.planet.n0, l);
    let r_m = cfg.planet.r_km * 1000.0;
    let area = areas(n, r_m);
    let (z, hielo_etopo) = proyectar_etopo(cfg, n)?;
    // El hielo es el mayor de los casquetes de ETOPO y los glaciares del RGI: los glaciares de
    // alrededor de Groenlandia y de la Antártida ya están en ETOPO y no se cuentan dos veces.
    let glaciares = glaciares_rgi(&cfg.tierra.rgi, n, &area)?;
    let hielo: Vec<f64> = hielo_etopo
        .iter()
        .zip(&glaciares)
        .map(|(&e, &g)| e.max(g))
        .collect();
    let (t, p, cobertura) = proyectar_chelsa(cfg, n)?;
    let cru = cargar_cru(cfg)?;
    let cru_de_celda = celdas_de_cru(&cru, n);
    // Ríos y lagos como dato (adenda 1 §1): la red de HydroSHEDS reescalada, el mar conectado y los
    // lagos de HydroLAKES.
    let tc = &cfg.tierra;
    let red = hidro::reescalar(
        Path::new(&tc.hydrosheds_dir),
        Path::new(&tc.hydrosheds_acc),
        n,
    )?;
    let mar = hidro::mar_conectado(&z, &red.pix_mar, n);
    let (lago, lagos_total_km2) = super::lagos::fraccion_de_lagos(
        Path::new(&tc.hydrolakes),
        n,
        &area,
        tc.lagos_raster_km2,
        tc.lagos_paso_grados,
    )?;
    let fino = Nivel {
        n,
        area,
        z,
        hielo,
        lago,
        t,
        p,
        cobertura,
    };
    if let Some(r) = &ruta {
        if let Some(dir) = r.parent() {
            std::fs::create_dir_all(dir)?;
        }
        // Temporal propio de este proceso y este hilo: dos cargas a la vez no se pisan.
        let tmp = r.with_extension(format!(
            "tmp{}_{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let hid = Hidrologia {
            red,
            mar,
            lagos_total_km2,
        };
        std::fs::write(&tmp, codificar(&fino, &cru, &cru_de_celda, &hid, h))?;
        std::fs::rename(&tmp, r)?;
        return Ok(Arc::new(desde_nivel_fino(
            cfg,
            fino,
            cru,
            cru_de_celda,
            hid,
            h,
        )));
    }
    let hid = Hidrologia {
        red,
        mar,
        lagos_total_km2,
    };
    Ok(Arc::new(desde_nivel_fino(
        cfg,
        fino,
        cru,
        cru_de_celda,
        hid,
        h,
    )))
}

/// Áreas reales de las celdas de un nivel, en el orden de las tablas.
pub fn areas(n: u32, r_m: f64) -> Vec<f64> {
    let mut a = Vec::with_capacity(6 * (n as usize) * (n as usize));
    for c in 0..6u8 {
        for j in 0..n {
            for i in 0..n {
                a.push(cell_area(c, i, j, n, r_m));
            }
        }
    }
    a
}

/// La hidrología del nivel de carga: la red, el mar conectado y el área total de HydroLAKES.
pub struct Hidrologia {
    pub red: Red,
    pub mar: Vec<bool>,
    pub lagos_total_km2: f64,
}

impl Hidrologia {
    /// Sin red (cada celda vierte directamente al mar) y con el mar donde hay profundidad: para las
    /// pruebas con datos inventados.
    pub fn sin_red(z: &[f64]) -> Hidrologia {
        let m = z.len();
        Hidrologia {
            red: Red {
                abajo: vec![hidro::NINGUNA; m],
                fin: vec![hidro::fin::MAR; m],
                acum: vec![0; m],
                pix_mar: z.iter().map(|&x| u32::from(x < 0.0)).collect(),
            },
            mar: z.iter().map(|&x| x < 0.0).collect(),
            lagos_total_km2: 0.0,
        }
    }
}

/// Datos completos a partir del nivel de carga: cada celda de un nivel grueso es la media por área de
/// sus cuatro hijas; la fracción de mar, la de sus hijas en el mar, y la mezcla de cuencas, si entre
/// sus descendientes de tierra hay de las que acaban en una cuenca cerrada y de las que no.
pub fn desde_nivel_fino(
    cfg: &TerrenoConfig,
    fino: Nivel,
    cru: Cru,
    cru_de_celda: Vec<u32>,
    hid: Hidrologia,
    huella: u64,
) -> DatosTierra {
    let cerrada = hidro::acaba_en_cerrada(&hid.red);
    // Por nivel, cuántas descendientes de tierra acaban en una cuenca cerrada y cuántas no.
    let mut cuenta: Vec<Vec<[u32; 2]>> = vec![
        (0..fino.len())
            .map(|k| {
                if hid.mar[k] {
                    [0, 0]
                } else if cerrada[k] {
                    [1, 0]
                } else {
                    [0, 1]
                }
            })
            .collect(),
    ];
    let mut mar_frac: Vec<Vec<f32>> = vec![hid.mar.iter().map(|&x| f32::from(x as u8)).collect()];
    let l = cfg.tierra.nivel_carga;
    let r_m = cfg.planet.r_km * 1000.0;
    let mut niveles = vec![fino];
    for lv in (0..l).rev() {
        let hija = &niveles[0];
        let n = face_n(cfg.planet.n0, lv);
        let m = 6 * (n as usize) * (n as usize);
        let mut madre = Nivel {
            n,
            area: areas(n, r_m),
            z: vec![0.0; m],
            hielo: vec![0.0; m],
            lago: vec![0.0; m],
            t: vec![0.0; 12 * m],
            p: vec![0.0; 12 * m],
            cobertura: vec![0.0; m],
        };
        let mut t = [0.0f64; 12];
        let mut p = [0.0f64; 12];
        for c in 0..6u8 {
            for j in 0..n {
                for i in 0..n {
                    let km = madre.indice(c, i, j);
                    let (mut a, mut z, mut hi, mut la, mut cob) = (0.0, 0.0, 0.0, 0.0, 0.0);
                    t.fill(0.0);
                    p.fill(0.0);
                    for (di, dj) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let k = hija.indice(c, 2 * i + di, 2 * j + dj);
                        let ah = hija.area[k];
                        a += ah;
                        z += hija.z[k] * ah;
                        hi += hija.hielo[k] * ah;
                        la += hija.lago[k] * ah;
                        cob += hija.cobertura[k] as f64 * ah;
                        for mes in 0..12 {
                            t[mes] += hija.t[k * 12 + mes] as f64 * ah;
                            p[mes] += hija.p[k * 12 + mes] as f64 * ah;
                        }
                    }
                    madre.z[km] = z / a;
                    madre.hielo[km] = hi / a;
                    madre.lago[km] = la / a;
                    madre.cobertura[km] = (cob / a) as f32;
                    for mes in 0..12 {
                        madre.t[km * 12 + mes] = (t[mes] / a) as f32;
                        madre.p[km * 12 + mes] = (p[mes] / a) as f32;
                    }
                }
            }
        }
        // Fracción de mar y cuencas de las descendientes.
        let (mut mf, mut cu) = (vec![0.0f32; m], vec![[0u32; 2]; m]);
        for c in 0..6u8 {
            for j in 0..n {
                for i in 0..n {
                    let km = madre.indice(c, i, j);
                    let (mut a, mut x) = (0.0f64, 0.0f64);
                    for (di, dj) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let k = hija.indice(c, 2 * i + di, 2 * j + dj);
                        a += hija.area[k];
                        x += mar_frac[0][k] as f64 * hija.area[k];
                        cu[km][0] += cuenta[0][k][0];
                        cu[km][1] += cuenta[0][k][1];
                    }
                    mf[km] = (x / a) as f32;
                }
            }
        }
        mar_frac.insert(0, mf);
        cuenta.insert(0, cu);
        niveles.insert(0, madre);
    }
    let mezcla = cuenta
        .iter()
        .map(|v| v.iter().map(|c| c[0] > 0 && c[1] > 0).collect())
        .collect();
    DatosTierra {
        n0: cfg.planet.n0,
        nivel_carga: l,
        niveles,
        cru,
        cru_de_celda,
        red: hid.red,
        mar: hid.mar,
        cerrada,
        mar_frac,
        mezcla,
        lagos_total_km2: hid.lagos_total_km2,
        huella,
    }
}

/// Dirección unitaria de una latitud y una longitud (grados).
#[inline]
fn direccion(lat_g: f64, lon_g: f64) -> [f64; 3] {
    let (f, l) = (lat_g.to_radians(), lon_g.to_radians());
    [f.cos() * l.cos(), f.cos() * l.sin(), f.sin()]
}

#[inline]
fn indice_de(c: u8, i: u32, j: u32, n: u32) -> usize {
    ((c as usize * n as usize + j as usize) * n as usize) + i as usize
}

fn leer_f32be(ruta: &str, filas: usize) -> Result<Vec<f32>, TerrenoError> {
    let b = std::fs::read(ruta).map_err(|e| error(format!("{ruta}: {e}")))?;
    if b.len() != filas * ETOPO_COLS * 4 {
        return Err(error(format!(
            "{ruta}: {} bytes en vez de {} (¿descarga a medias?)",
            b.len(),
            filas * ETOPO_COLS * 4
        )));
    }
    Ok(b.chunks_exact(4)
        .map(|c| f32::from_be_bytes([c[0], c[1], c[2], c[3]]))
        .collect())
}

/// ETOPO 2022 (cada 2'): altura media de cada celda y fracción con hielo (superficie − roca > 0;
/// la roca solo difiere de la superficie bajo los casquetes, en las franjas polares).
fn proyectar_etopo(cfg: &TerrenoConfig, n: u32) -> Result<(Vec<f64>, Vec<f64>), TerrenoError> {
    let t = &cfg.tierra;
    let hielo_min = t.hielo_min_m;
    let sup = leer_f32be(&t.etopo_superficie, ETOPO_FILAS)?;
    let sur = leer_f32be(&t.etopo_roca_sur, ETOPO_FILAS_SUR)?;
    let norte = leer_f32be(&t.etopo_roca_norte, ETOPO_FILAS_NORTE)?;
    let m = 6 * (n as usize) * (n as usize);
    let (mut sw, mut szw, mut shw) = (vec![0.0f64; m], vec![0.0f64; m], vec![0.0f64; m]);
    for r in 0..ETOPO_FILAS {
        let lat = -90.0 + (2 * r) as f64 / 60.0 + 0.5 / 60.0;
        let w = lat.to_radians().cos();
        let fila = &sup[r * ETOPO_COLS..(r + 1) * ETOPO_COLS];
        let roca: Option<&[f32]> = if r < ETOPO_FILAS_SUR {
            Some(&sur[r * ETOPO_COLS..(r + 1) * ETOPO_COLS])
        } else if r >= ETOPO_FILA_NORTE {
            let q = r - ETOPO_FILA_NORTE;
            Some(&norte[q * ETOPO_COLS..(q + 1) * ETOPO_COLS])
        } else {
            None
        };
        for (c, &z) in fila.iter().enumerate() {
            let lon = -180.0 + (2 * c) as f64 / 60.0 + 0.5 / 60.0;
            let (f, i, j) = locate(direccion(lat, lon), n);
            let k = indice_de(f, i, j, n);
            let z = z as f64;
            sw[k] += w;
            szw[k] += w * z;
            // Hielo: superficie sobre el nivel del mar y al menos `hielo_min_m` por encima de la roca.
            if let Some(roca) = roca
                && z >= 0.0
                && z - roca[c] as f64 >= hielo_min
            {
                shw[k] += w;
            }
        }
    }
    if sw.iter().any(|&w| w <= 0.0) {
        return Err(error(
            "ETOPO a 2' deja celdas sin ningún punto: nivel_carga demasiado fino (máximo 5)",
        ));
    }
    let z = szw.iter().zip(&sw).map(|(a, b)| a / b).collect();
    let hielo = shw.iter().zip(&sw).map(|(a, b)| a / b).collect();
    Ok((z, hielo))
}

/// Glaciares del RGI 7.0 (tabla global de atributos: centro y área de cada glaciar) en fracción de
/// cada celda del nivel de carga. Casi todos son mucho más pequeños que una celda y caen enteros en la
/// de su centro. Los que no caben (los grandes casquetes de Svalbard, del Ártico canadiense o de
/// Patagonia) reparten lo que sobra por las celdas vecinas, anillo a anillo, sin perder área.
fn glaciares_rgi(ruta: &str, n: u32, area: &[f64]) -> Result<Vec<f64>, TerrenoError> {
    let texto = std::fs::read_to_string(ruta).map_err(|e| error(format!("{ruta}: {e}")))?;
    let mut lineas = texto.lines();
    let cab = campos_csv(lineas.next().unwrap_or(""));
    let col = |nombre: &str| {
        cab.iter()
            .position(|c| c == nombre)
            .ok_or_else(|| error(format!("{ruta}: sin la columna {nombre}")))
    };
    let (c_lon, c_lat, c_area) = (col("cenlon")?, col("cenlat")?, col("area_km2")?);
    let mut km2 = vec![0.0f64; area.len()];
    for (q, linea) in lineas.enumerate() {
        if linea.trim().is_empty() {
            continue;
        }
        let f = campos_csv(linea);
        let num = |i: usize| -> Result<f64, TerrenoError> {
            f.get(i)
                .and_then(|x| x.parse::<f64>().ok())
                .filter(|x| x.is_finite())
                .ok_or_else(|| error(format!("{ruta}: fila {} mal formada", q + 2)))
        };
        let (lon, lat, a) = (num(c_lon)?, num(c_lat)?, num(c_area)?);
        let (c, i, j) = locate(direccion(lat, lon), n);
        km2[indice_de(c, i, j, n)] += a.max(0.0);
    }
    // Lo que no cabe en una celda pasa a sus vecinas (a la misma distancia en la cara o cruzando la
    // arista), por orden de celda: es determinista y conserva el área total.
    let mut frac: Vec<f64> = km2.iter().zip(area).map(|(k, a)| k * 1e6 / a).collect();
    for c in 0..6u8 {
        for j in 0..n {
            for i in 0..n {
                let k = indice_de(c, i, j, n);
                if frac[k] <= 1.0 {
                    continue;
                }
                let mut sobra = (frac[k] - 1.0) * area[k];
                frac[k] = 1.0;
                for radio in 1i64..=8 {
                    if sobra <= 0.0 {
                        break;
                    }
                    for dj in -radio..=radio {
                        for di in -radio..=radio {
                            if di.abs().max(dj.abs()) != radio || sobra <= 0.0 {
                                continue;
                            }
                            let Some((c2, i2, j2)) =
                                crate::geometry::neighbor_addr(c, i, j, n, di, dj)
                            else {
                                continue;
                            };
                            let k2 = indice_de(c2, i2, j2, n);
                            let hueco = (1.0 - frac[k2]).max(0.0) * area[k2];
                            let pon = hueco.min(sobra);
                            frac[k2] += pon / area[k2];
                            sobra -= pon;
                        }
                    }
                }
            }
        }
    }
    Ok(frac.into_iter().map(|x| x.min(1.0)).collect())
}

/// Campos de una línea CSV con comillas dobles (las comas dentro de comillas no separan).
fn campos_csv(linea: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut actual = String::new();
    let mut dentro = false;
    let mut chars = linea.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' if dentro && chars.peek() == Some(&'"') => {
                actual.push('"');
                chars.next();
            }
            '"' => dentro = !dentro,
            ',' if !dentro => out.push(std::mem::take(&mut actual)),
            _ => actual.push(ch),
        }
    }
    out.push(actual);
    out
}

/// Tramos de una fila de la rejilla de CHELSA que caen en una misma celda: `(columna inicial,
/// celda)`; el tramo acaba donde empieza el siguiente.
type Tramos = Vec<(u32, u32)>;

/// Clima de CHELSA en las celdas: temperatura y precipitación (12 por celda) y cobertura.
type ClimaCeldas = (Vec<f32>, Vec<f32>, Vec<f32>);

/// CHELSA V2.1 (30''): temperatura y precipitación mensuales medias de cada celda (todos los píxeles,
/// ponderados por `cos φ`) y la cobertura (fracción de la celda con píxeles).
fn proyectar_chelsa(cfg: &TerrenoConfig, n: u32) -> Result<ClimaCeldas, TerrenoError> {
    let rutas = rutas_chelsa(cfg);
    let primero = GeoTiff::abrir(&rutas[0])?;
    let (ancho, alto) = (primero.ancho, primero.alto);
    let (lon0, lat0, plon, plat) = (
        primero.lon0,
        primero.lat0,
        primero.paso_lon,
        primero.paso_lat,
    );
    for r in &rutas[1..] {
        let g = GeoTiff::abrir(r)?;
        if g.ancho != ancho
            || g.alto != alto
            || (g.lon0 - lon0).abs() > 1e-9
            || (g.lat0 - lat0).abs() > 1e-9
            || (g.paso_lon - plon).abs() > 1e-12
            || (g.paso_lat - plat).abs() > 1e-12
        {
            return Err(error(format!(
                "{}: otra rejilla que la de {}",
                r.display(),
                rutas[0].display()
            )));
        }
    }
    drop(primero);
    let hilos = std::thread::available_parallelism()
        .map_or(1, |x| x.get())
        .clamp(1, 24);

    // Tramos de cada fila (en paralelo por bloques de filas; se juntan en orden).
    let bloque = alto.div_ceil(hilos);
    let mut tramos: Vec<Tramos> = Vec::with_capacity(alto);
    std::thread::scope(|s| {
        let partes: Vec<_> = (0..hilos)
            .map(|h| {
                s.spawn(move || {
                    let (r0, r1) = (h * bloque, ((h + 1) * bloque).min(alto));
                    let mut out = Vec::with_capacity(r1.saturating_sub(r0));
                    for r in r0..r1 {
                        let lat = lat0 - (r as f64 + 0.5) * plat;
                        let mut v: Tramos = Vec::new();
                        for c in 0..ancho {
                            let lon = lon0 + (c as f64 + 0.5) * plon;
                            let (f, i, j) = locate(direccion(lat, lon), n);
                            let k = indice_de(f, i, j, n) as u32;
                            if v.last().is_none_or(|&(_, kk)| kk != k) {
                                v.push((c as u32, k));
                            }
                        }
                        out.push(v);
                    }
                    out
                })
            })
            .collect();
        for p in partes {
            if let Ok(v) = p.join() {
                tramos.extend(v);
            }
        }
    });
    if tramos.len() != alto {
        return Err(error("CHELSA: fallo al preparar los tramos de las filas"));
    }
    let m = 6 * (n as usize) * (n as usize);
    let peso_fila: Vec<f64> = (0..alto)
        .map(|r| (lat0 - (r as f64 + 0.5) * plat).to_radians().cos())
        .collect();
    // Peso total de los píxeles de cada celda (con dato o sin él), para la cobertura.
    let mut total = vec![0.0f64; m];
    for (r, v) in tramos.iter().enumerate() {
        for (q, &(c0, k)) in v.iter().enumerate() {
            let c1 = v.get(q + 1).map_or(ancho as u32, |x| x.0);
            total[k as usize] += peso_fila[r] * (c1 - c0) as f64;
        }
    }

    // Un archivo por trabajo; cada uno con sus acumuladores (Σ w·v, Σ w).
    type Resultado = Result<(Vec<f64>, Vec<f64>, f64, f64), TerrenoError>;
    let siguiente = AtomicUsize::new(0);
    let mut resultados: Vec<Option<Resultado>> = (0..rutas.len()).map(|_| None).collect();
    std::thread::scope(|s| {
        let partes: Vec<_> = (0..hilos.min(rutas.len()))
            .map(|_| {
                let (rutas, tramos, peso_fila, siguiente) =
                    (&rutas, &tramos, &peso_fila, &siguiente);
                s.spawn(move || {
                    let mut hechos: Vec<(usize, Resultado)> = Vec::new();
                    loop {
                        let k = siguiente.fetch_add(1, Ordering::SeqCst);
                        if k >= rutas.len() {
                            break;
                        }
                        let es_t = k < 12;
                        hechos.push((
                            k,
                            acumular_chelsa(&rutas[k], tramos, peso_fila, m, ancho, es_t),
                        ));
                    }
                    hechos
                })
            })
            .collect();
        for p in partes {
            if let Ok(v) = p.join() {
                for (k, r) in v {
                    resultados[k] = Some(r);
                }
            }
        }
    });
    let mut t = vec![0.0f32; 12 * m];
    let mut p = vec![0.0f32; 12 * m];
    let mut con_dato = vec![0.0f64; m];
    for (k, r) in resultados.into_iter().enumerate() {
        let (sv, sw, escala, despl) =
            r.ok_or_else(|| error("CHELSA: un hilo de lectura falló"))??;
        if k == 0 {
            con_dato.copy_from_slice(&sw);
        }
        let (destino, mes) = if k < 12 {
            (&mut t, k)
        } else {
            (&mut p, k - 12)
        };
        for c in 0..m {
            destino[c * 12 + mes] = if sw[c] > 0.0 {
                (escala * sv[c] / sw[c] + despl) as f32
            } else {
                f32::NAN
            };
        }
    }
    let cobertura: Vec<f32> = con_dato
        .iter()
        .zip(&total)
        .map(|(&a, &b)| if b > 0.0 { (a / b) as f32 } else { 0.0 })
        .collect();
    rellenar_sin_dato(&mut t, &mut p, n)?;
    Ok((t, p, cobertura))
}

/// Suma un GeoTIFF de CHELSA en las celdas: `(Σ w·v, Σ w, escala, desplazamiento)`.
fn acumular_chelsa(
    ruta: &Path,
    tramos: &[Tramos],
    peso_fila: &[f64],
    m: usize,
    ancho: usize,
    es_temperatura: bool,
) -> Result<(Vec<f64>, Vec<f64>, f64, f64), TerrenoError> {
    let mut g = GeoTiff::abrir(ruta)?;
    let (mut sv, mut sw) = (vec![0.0f64; m], vec![0.0f64; m]);
    for (r, v) in tramos.iter().enumerate() {
        let fila = g.fila(r)?;
        let w = peso_fila[r];
        for (q, &(c0, k)) in v.iter().enumerate() {
            let c1 = v.get(q + 1).map_or(ancho as u32, |x| x.0) as usize;
            let (mut s, mut nv) = (0u64, 0u64);
            for &x in &fila[c0 as usize..c1] {
                // Sin dato: 65 535 (y 0 en la temperatura, que serían −273,15 °C).
                if x != u16::MAX && !(es_temperatura && x == 0) {
                    s += x as u64;
                    nv += 1;
                }
            }
            sv[k as usize] += w * s as f64;
            sw[k as usize] += w * nv as f64;
        }
    }
    Ok((sv, sw, g.escala, g.desplazamiento))
}

/// Las celdas sin ningún píxel de CHELSA (al norte de 84° N) toman el clima de la celda con dato más
/// cercana en su meridiano, hacia el ecuador.
fn rellenar_sin_dato(t: &mut [f32], p: &mut [f32], n: u32) -> Result<(), TerrenoError> {
    let m = 6 * (n as usize) * (n as usize);
    for c in 0..6u8 {
        for j in 0..n {
            for i in 0..n {
                let k = indice_de(c, i, j, n);
                if !t[k * 12].is_nan() {
                    continue;
                }
                let (lat, lon) = lat_lon(center_dir(c, i, j, n));
                let (mut lat_g, lon_g) = (lat.to_degrees().clamp(-89.99, 83.99), lon.to_degrees());
                let mut origen = None;
                for _ in 0..2000 {
                    let (f, ii, jj) = locate(direccion(lat_g, lon_g), n);
                    let k2 = indice_de(f, ii, jj, n);
                    if k2 < m && !t[k2 * 12].is_nan() {
                        origen = Some(k2);
                        break;
                    }
                    lat_g -= 0.05 * lat_g.signum();
                }
                let k2 = origen.ok_or_else(|| {
                    error(format!(
                        "CHELSA: celda sin dato y sin vecina con dato (lat {:.2})",
                        lat.to_degrees()
                    ))
                })?;
                for mes in 0..12 {
                    t[k * 12 + mes] = t[k2 * 12 + mes];
                    p[k * 12 + mes] = p[k2 * 12 + mes];
                }
            }
        }
    }
    if t.iter().chain(p.iter()).any(|x| !x.is_finite()) {
        return Err(error("CHELSA: quedan celdas sin clima"));
    }
    Ok(())
}

/// Una variable de CRU TS: rejilla, celdas de tierra, años y valores en décimas por mes y celda.
struct VariableCru {
    lat: Vec<f32>,
    lon: Vec<f32>,
    tierra: Vec<u32>,
    anio0: i32,
    n_meses: usize,
    valores: Vec<i32>,
    sin_estaciones: Vec<f32>,
}

fn f32s(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_be_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn leer_variable_cru(ruta: &str, nombre: &str) -> Result<VariableCru, TerrenoError> {
    let p = Path::new(ruta);
    let gz = BufReader::with_capacity(1 << 22, Gzip::abrir(p)?);
    let mut nc = Netcdf3::leer(gz, p)?;
    let lon = f32s(&nc.fija("lon")?);
    let lat = f32s(&nc.fija("lat")?);
    let (nlon, nlat) = (lon.len(), lat.len());
    if nlon < 2 || nlat < 2 {
        return Err(error(format!("{ruta}: rejilla vacía")));
    }
    // Sin dato: el `_FillValue` de CRU TS (9,97·10³⁶), sobre el mar y la Antártida.
    let relleno = nc
        .variable(nombre)?
        .atributo_num("_FillValue")
        .unwrap_or(9.969_209_968_386_869e36) as f32;
    let es_dato = |x: f32| x != relleno && x.abs() < 1e30;
    let unidades = nc.variable("time")?.atributo_texto("units").unwrap_or("");
    if !unidades.starts_with("days since 1900-1-1") {
        return Err(error(format!("{ruta}: tiempo en «{unidades}»")));
    }
    let numrecs = nc.numrecs as usize;
    if numrecs == 0 || !numrecs.is_multiple_of(12) {
        return Err(error(format!(
            "{ruta}: {numrecs} meses (no son años completos)"
        )));
    }
    let pesos: Vec<f64> = lat.iter().map(|&f| (f as f64).to_radians().cos()).collect();
    let mut tierra: Vec<u32> = Vec::new();
    let mut valores: Vec<i32> = Vec::new();
    let mut sin_est: Vec<f32> = Vec::with_capacity(numrecs);
    let mut anio0 = 0;
    let mut area_tierra = 0.0;
    nc.registros(&[nombre, "stn", "time"], |k, datos| {
        let v = f32s(datos[0]);
        if v.len() != nlon * nlat || datos[1].len() != 4 * nlon * nlat {
            return Err(error(format!("{ruta}: registro {k} de otro tamaño")));
        }
        if k == 0 {
            let dias = f32s(datos[2]).first().copied().unwrap_or(0.0) as f64;
            let (a, mes) = anio_mes_desde_1900(dias);
            if mes != 1 {
                return Err(error(format!("{ruta}: la serie no empieza en enero")));
            }
            anio0 = a;
            for (g, &x) in v.iter().enumerate() {
                if es_dato(x) {
                    tierra.push(g as u32);
                    area_tierra += pesos[g / nlon];
                }
            }
            valores.reserve(numrecs * tierra.len());
        }
        let mut sin = 0.0;
        for &g in &tierra {
            let x = v[g as usize];
            if !es_dato(x) {
                return Err(error(format!(
                    "{ruta}: celda de tierra sin dato en el mes {k}"
                )));
            }
            valores.push((x as f64 * 10.0).round() as i32);
            let q = 4 * g as usize;
            let s = &datos[1][q..q + 4];
            if i32::from_be_bytes([s[0], s[1], s[2], s[3]]) == 0 {
                sin += pesos[g as usize / nlon];
            }
        }
        sin_est.push((sin / area_tierra.max(1e-300)) as f32);
        Ok(())
    })?;
    Ok(VariableCru {
        lat,
        lon,
        tierra,
        anio0,
        n_meses: numrecs,
        valores,
        sin_estaciones: sin_est,
    })
}

/// CRU TS 4.09: series mensuales de temperatura y lluvia de las celdas de tierra.
fn cargar_cru(cfg: &TerrenoConfig) -> Result<Cru, TerrenoError> {
    // Las dos variables a la vez, cada una en su hilo (cada una suma solo lo suyo).
    let (tmp, pre) = std::thread::scope(|s| {
        let h = s.spawn(|| leer_variable_cru(&cfg.tierra.cru_tmp, "tmp"));
        let pre = leer_variable_cru(&cfg.tierra.cru_pre, "pre");
        (h.join(), pre)
    });
    let tmp = tmp.map_err(|_| error("CRU TS: el hilo de la temperatura falló"))??;
    let pre = pre?;
    if tmp.tierra != pre.tierra
        || tmp.anio0 != pre.anio0
        || tmp.n_meses != pre.n_meses
        || tmp.lat != pre.lat
        || tmp.lon != pre.lon
    {
        return Err(error(
            "CRU TS: la temperatura y la lluvia no tienen la misma rejilla, tierra o años",
        ));
    }
    let nlon = tmp.lon.len();
    let lat: Vec<f32> = tmp
        .tierra
        .iter()
        .map(|&g| tmp.lat[g as usize / nlon])
        .collect();
    let lon: Vec<f32> = tmp
        .tierra
        .iter()
        .map(|&g| tmp.lon[g as usize % nlon])
        .collect();
    let mut t16 = Vec::with_capacity(tmp.valores.len());
    for &x in &tmp.valores {
        t16.push(i16::try_from(x).map_err(|_| error("CRU TS: temperatura fuera de rango"))?);
    }
    let mut recortados = 0u64;
    let p16: Vec<u16> = pre
        .valores
        .iter()
        .map(|&x| {
            if x > u16::MAX as i32 {
                recortados += 1;
            }
            x.clamp(0, u16::MAX as i32) as u16
        })
        .collect();
    let n_anios = tmp.n_meses / 12;
    let sin_estaciones = (0..n_anios)
        .map(|a| {
            let media = |v: &[f32]| v[a * 12..a * 12 + 12].iter().sum::<f32>() / 12.0;
            [media(&tmp.sin_estaciones), media(&pre.sin_estaciones)]
        })
        .collect();
    Ok(Cru {
        anio0: tmp.anio0,
        n_anios,
        lat,
        lon,
        tmp: t16,
        pre: p16,
        pre_recortados: recortados,
        sin_estaciones,
    })
}

/// Celda de CRU de cada celda del nivel de carga: la que contiene su centro si es de tierra; si no,
/// la de tierra más cercana a menos de dos celdas de CRU; si tampoco, ninguna (`u32::MAX`).
fn celdas_de_cru(cru: &Cru, n: u32) -> Vec<u32> {
    // Rejilla de CRU (0,5°): índice de cada celda de tierra en su rejilla completa.
    let (paso, nlat, nlon) = (0.5f64, 360usize, 720usize);
    let mut rejilla = vec![u32::MAX; nlat * nlon];
    let fila = |lat: f64| (((lat + 90.0) / paso).floor() as i64).clamp(0, nlat as i64 - 1);
    let col = |lon: f64| (((lon + 180.0) / paso).floor() as i64).rem_euclid(nlon as i64);
    for (q, (&la, &lo)) in cru.lat.iter().zip(&cru.lon).enumerate() {
        let g = fila(la as f64) as usize * nlon + col(lo as f64) as usize;
        rejilla[g] = q as u32;
    }
    let mut out = Vec::with_capacity(6 * (n as usize) * (n as usize));
    for c in 0..6u8 {
        for j in 0..n {
            for i in 0..n {
                let d = center_dir(c, i, j, n);
                let (lat, lon) = lat_lon(d);
                let (lat, lon) = (lat.to_degrees(), lon.to_degrees());
                let (f0, c0) = (fila(lat), col(lon));
                let directa = rejilla[f0 as usize * nlon + c0 as usize];
                if directa != u32::MAX {
                    out.push(directa);
                    continue;
                }
                let mut mejor = (f64::INFINITY, u32::MAX);
                for df in -2i64..=2 {
                    let f = f0 + df;
                    if !(0..nlat as i64).contains(&f) {
                        continue;
                    }
                    for dc in -2i64..=2 {
                        let cc = (c0 + dc).rem_euclid(nlon as i64);
                        let q = rejilla[f as usize * nlon + cc as usize];
                        if q == u32::MAX {
                            continue;
                        }
                        let e = direccion(cru.lat[q as usize] as f64, cru.lon[q as usize] as f64);
                        let ang = angle_between(d, e);
                        if ang < mejor.0 || (ang == mejor.0 && q < mejor.1) {
                            mejor = (ang, q);
                        }
                    }
                }
                out.push(mejor.1);
            }
        }
    }
    out
}

// --- Caché -----------------------------------------------------------------------------------------

fn codificar(f: &Nivel, cru: &Cru, cru_de_celda: &[u32], hid: &Hidrologia, huella: u64) -> Vec<u8> {
    let mut w: Vec<u8> = Vec::with_capacity(
        64 + f.len() * 120 + cru.tmp.len() * 4 + cru.n_celdas() * 8 + cru.n_anios * 8,
    );
    w.extend_from_slice(MAGIA);
    w.extend_from_slice(&huella.to_le_bytes());
    w.extend_from_slice(&f.n.to_le_bytes());
    w.extend_from_slice(&(f.len() as u64).to_le_bytes());
    for x in f.z.iter().chain(&f.hielo).chain(&f.lago) {
        w.extend_from_slice(&x.to_le_bytes());
    }
    for x in f.t.iter().chain(&f.p).chain(&f.cobertura) {
        w.extend_from_slice(&x.to_le_bytes());
    }
    for x in cru_de_celda {
        w.extend_from_slice(&x.to_le_bytes());
    }
    w.extend_from_slice(&cru.anio0.to_le_bytes());
    w.extend_from_slice(&(cru.n_anios as u64).to_le_bytes());
    w.extend_from_slice(&(cru.n_celdas() as u64).to_le_bytes());
    for x in cru.lat.iter().chain(&cru.lon) {
        w.extend_from_slice(&x.to_le_bytes());
    }
    for x in &cru.tmp {
        w.extend_from_slice(&x.to_le_bytes());
    }
    for x in &cru.pre {
        w.extend_from_slice(&x.to_le_bytes());
    }
    w.extend_from_slice(&cru.pre_recortados.to_le_bytes());
    for x in cru.sin_estaciones.iter().flatten() {
        w.extend_from_slice(&x.to_le_bytes());
    }
    let red = &hid.red;
    for x in red.abajo.iter().chain(&red.acum).chain(&red.pix_mar) {
        w.extend_from_slice(&x.to_le_bytes());
    }
    w.extend_from_slice(&red.fin);
    w.extend(hid.mar.iter().map(|&x| x as u8));
    w.extend_from_slice(&hid.lagos_total_km2.to_le_bytes());
    w
}

struct Lector<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Lector<'a> {
    fn tomar(&mut self, n: usize) -> Option<&'a [u8]> {
        let s = self.b.get(self.p..self.p.checked_add(n)?)?;
        self.p += n;
        Some(s)
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.tomar(8)?.try_into().ok()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.tomar(4)?.try_into().ok()?))
    }
    fn vf64(&mut self, n: usize) -> Option<Vec<f64>> {
        let b = self.tomar(n.checked_mul(8)?)?;
        Some(
            b.chunks_exact(8)
                .map(|c| f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
                .collect(),
        )
    }
    fn vf32(&mut self, n: usize) -> Option<Vec<f32>> {
        let b = self.tomar(n.checked_mul(4)?)?;
        Some(
            b.chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect(),
        )
    }
}

fn decodificar(b: &[u8], cfg: &TerrenoConfig, huella: u64) -> Option<DatosTierra> {
    let mut r = Lector { b, p: 0 };
    if r.tomar(8)? != MAGIA || r.u64()? != huella {
        return None;
    }
    let n = r.u32()?;
    let m = r.u64()? as usize;
    if n != face_n(cfg.planet.n0, cfg.tierra.nivel_carga) || m != 6 * (n as usize) * (n as usize) {
        return None;
    }
    let z = r.vf64(m)?;
    let hielo = r.vf64(m)?;
    let lago = r.vf64(m)?;
    let t = r.vf32(12 * m)?;
    let p = r.vf32(12 * m)?;
    let cobertura = r.vf32(m)?;
    let cru_de_celda: Vec<u32> = r
        .tomar(4 * m)?
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    let anio0 = r.u32()? as i32;
    let n_anios = r.u64()? as usize;
    let nc = r.u64()? as usize;
    let lat = r.vf32(nc)?;
    let lon = r.vf32(nc)?;
    let nv = n_anios.checked_mul(12)?.checked_mul(nc)?;
    let tmp: Vec<i16> = r
        .tomar(2 * nv)?
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect();
    let pre: Vec<u16> = r
        .tomar(2 * nv)?
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let pre_recortados = r.u64()?;
    let se = r.vf32(2 * n_anios)?;
    let u32s = |r: &mut Lector| -> Option<Vec<u32>> {
        Some(
            r.tomar(4 * m)?
                .chunks_exact(4)
                .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect(),
        )
    };
    let abajo = u32s(&mut r)?;
    let acum = u32s(&mut r)?;
    let pix_mar = u32s(&mut r)?;
    let fin = r.tomar(m)?.to_vec();
    let mar: Vec<bool> = r.tomar(m)?.iter().map(|&x| x != 0).collect();
    let lagos_total_km2 = f64::from_le_bytes(r.tomar(8)?.try_into().ok()?);
    if abajo
        .iter()
        .any(|&a| a != hidro::NINGUNA && a as usize >= m)
    {
        return None;
    }
    let hid = Hidrologia {
        red: Red {
            abajo,
            fin,
            acum,
            pix_mar,
        },
        mar,
        lagos_total_km2,
    };
    if r.p != b.len()
        || cru_de_celda
            .iter()
            .any(|&q| q != u32::MAX && q as usize >= nc)
    {
        return None;
    }
    let cru = Cru {
        anio0,
        n_anios,
        lat,
        lon,
        tmp,
        pre,
        pre_recortados,
        sin_estaciones: se.chunks_exact(2).map(|c| [c[0], c[1]]).collect(),
    };
    let fino = Nivel {
        n,
        area: areas(n, cfg.planet.r_km * 1000.0),
        z,
        hielo,
        lago,
        t,
        p,
        cobertura,
    };
    Some(desde_nivel_fino(cfg, fino, cru, cru_de_celda, hid, huella))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn las_franjas_de_etopo_cuadran_con_los_archivos() {
        // 5 400 filas desde el sur cada 2': −60° es la fila 900 y +59° la 4 470.
        assert_eq!(ETOPO_FILAS_SUR, 900);
        assert_eq!(ETOPO_FILA_NORTE, 4470);
        assert_eq!(ETOPO_FILAS_NORTE, 930);
        let lat_norte = -90.0 + (2 * ETOPO_FILA_NORTE) as f64 / 60.0;
        assert!((lat_norte - 59.0).abs() < 1e-12);
    }

    #[test]
    fn los_niveles_gruesos_conservan_la_media_por_area() {
        // Un nivel fino inventado: la madre es la media por área exacta de sus hijas.
        let mut cfg = TerrenoConfig::default();
        cfg.planet.n0 = 2;
        cfg.tierra.nivel_carga = 2;
        let n = face_n(2, 2);
        let m = 6 * (n as usize) * (n as usize);
        let r_m = cfg.planet.r_km * 1000.0;
        let z: Vec<f64> = (0..m).map(|k| (k as f64 * 37.0) % 1000.0 - 400.0).collect();
        let fino = Nivel {
            n,
            area: areas(n, r_m),
            z: z.clone(),
            hielo: (0..m).map(|k| (k % 3) as f64 / 2.0).collect(),
            lago: vec![0.0; m],
            t: (0..12 * m).map(|k| (k % 50) as f32 - 10.0).collect(),
            p: (0..12 * m).map(|k| (k % 70) as f32).collect(),
            cobertura: vec![1.0; m],
        };
        let cru = Cru {
            anio0: 1901,
            n_anios: 1,
            lat: vec![],
            lon: vec![],
            tmp: vec![],
            pre: vec![],
            pre_recortados: 0,
            sin_estaciones: vec![[0.0, 0.0]],
        };
        let hid = Hidrologia::sin_red(&z);
        let d = desde_nivel_fino(&cfg, fino, cru, vec![u32::MAX; m], hid, 0);
        assert_eq!(d.niveles.len(), 3);
        // Σ z·A igual en todos los niveles (salvo el redondeo).
        let total = |nv: &Nivel| nv.z.iter().zip(&nv.area).map(|(z, a)| z * a).sum::<f64>();
        let (t0, t2) = (total(&d.niveles[0]), total(&d.niveles[2]));
        assert!(
            ((t0 - t2) / t2.abs().max(1.0)).abs() < 1e-12,
            "{t0} frente a {t2}"
        );
        // Y una madre concreta es la media de sus cuatro hijas.
        let (madre, hija) = (&d.niveles[1], &d.niveles[2]);
        let km = madre.indice(3, 1, 0);
        let hijas = [(2, 0), (3, 0), (2, 1), (3, 1)].map(|(i, j)| hija.indice(3, i, j));
        let a: f64 = hijas.iter().map(|&k| hija.area[k]).sum();
        let media = hijas.iter().map(|&k| hija.z[k] * hija.area[k]).sum::<f64>() / a;
        assert!((madre.z[km] - media).abs() < 1e-9);
    }
}
