//! Lector mínimo de GeoTIFF (modo Tierra): la primera imagen, de una banda, guardada en tiras, sin
//! comprimir o con Deflate, con o sin predictor horizontal, de enteros de 16 bits sin signo. Es lo que
//! usa CHELSA V2.1; cualquier otra cosa da un error claro en vez de leer mal (igual que
//! `herramientas/analisis/tiffmini.py`). El archivo es un dato no fiable: solo se interpretan sus
//! bytes y se comprueba cada tamaño antes de usarlo.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::error::TerrenoError;

fn error(ruta: &Path, m: impl std::fmt::Display) -> TerrenoError {
    TerrenoError::Tierra(format!("{}: {m}", ruta.display()))
}

/// Valor de una etiqueta: números (todos como `f64`, exactos hasta 2^53) o texto.
enum Valor {
    Numeros(Vec<f64>),
    Texto(String),
}

/// Una imagen GeoTIFF abierta para leerla fila a fila.
pub struct GeoTiff {
    f: File,
    ruta: std::path::PathBuf,
    le: bool,
    pub ancho: usize,
    pub alto: usize,
    filas_por_tira: usize,
    tiras: Vec<u64>,
    cuentas: Vec<u64>,
    comprimida: bool,
    predictor: u16,
    /// Borde oeste de la columna 0 y borde norte de la fila 0 (grados), y paso del píxel.
    pub lon0: f64,
    pub lat0: f64,
    pub paso_lon: f64,
    pub paso_lat: f64,
    /// Valor físico = crudo · escala + desplazamiento (metadatos de GDAL).
    pub escala: f64,
    pub desplazamiento: f64,
    bruto: Vec<u8>,
    tira: usize,
    valores: Vec<u16>,
}

/// Lee la cabecera y el primer IFD: el orden de los bytes y las etiquetas.
fn leer_etiquetas(f: &mut File, ruta: &Path) -> Result<(bool, BTreeMap<u16, Valor>), TerrenoError> {
    let mut cab = [0u8; 16];
    f.read_exact(&mut cab).map_err(|e| error(ruta, e))?;
    let le = match &cab[..2] {
        b"II" => true,
        b"MM" => false,
        _ => return Err(error(ruta, "no es un TIFF")),
    };
    let u16_ = |b: &[u8]| {
        let a = [b[0], b[1]];
        if le {
            u16::from_le_bytes(a)
        } else {
            u16::from_be_bytes(a)
        }
    };
    let u32_ = |b: &[u8]| {
        let a = [b[0], b[1], b[2], b[3]];
        if le {
            u32::from_le_bytes(a)
        } else {
            u32::from_be_bytes(a)
        }
    };
    let u64_ = |b: &[u8]| {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[..8]);
        if le {
            u64::from_le_bytes(a)
        } else {
            u64::from_be_bytes(a)
        }
    };
    let (grande, ifd) = match u16_(&cab[2..4]) {
        42 => (false, u32_(&cab[4..8]) as u64),
        43 => (true, u64_(&cab[8..16])),
        v => return Err(error(ruta, format!("versión de TIFF desconocida: {v}"))),
    };
    let leer = |f: &mut File, desde: u64, n: usize| -> Result<Vec<u8>, TerrenoError> {
        if n > 64 << 20 {
            return Err(error(ruta, "etiqueta demasiado grande"));
        }
        let mut b = vec![0u8; n];
        f.seek(SeekFrom::Start(desde)).map_err(|e| error(ruta, e))?;
        f.read_exact(&mut b).map_err(|e| error(ruta, e))?;
        Ok(b)
    };
    let (n_etiquetas, tam, p0) = if grande {
        (u64_(&leer(f, ifd, 8)?), 20usize, ifd + 8)
    } else {
        (u16_(&leer(f, ifd, 2)?) as u64, 12usize, ifd + 2)
    };
    if n_etiquetas > 4096 {
        return Err(error(ruta, "demasiadas etiquetas"));
    }
    let b = leer(f, p0, n_etiquetas as usize * tam)?;
    let mut t: BTreeMap<u16, Valor> = BTreeMap::new();
    for k in 0..n_etiquetas as usize {
        let q = &b[k * tam..(k + 1) * tam];
        let (etiqueta, tipo) = (u16_(&q[0..2]), u16_(&q[2..4]));
        let (cuenta, campo, cabe) = if grande {
            (u64_(&q[4..12]), &q[12..20], 8usize)
        } else {
            (u32_(&q[4..8]) as u64, &q[8..12], 4usize)
        };
        let tam_tipo: usize = match tipo {
            1 | 2 | 6 | 7 => 1,
            3 | 8 => 2,
            4 | 9 | 11 => 4,
            5 | 10 | 12 | 16 | 17 | 18 => 8,
            _ => continue,
        };
        if cuenta > (16 << 20) {
            return Err(error(ruta, "etiqueta con demasiados valores"));
        }
        let nb = tam_tipo * cuenta as usize;
        let datos = if nb <= cabe {
            campo[..nb].to_vec()
        } else {
            let desde = if grande {
                u64_(campo)
            } else {
                u32_(campo) as u64
            };
            leer(f, desde, nb)?
        };
        let v = if tipo == 2 {
            let s: Vec<u8> = datos.iter().copied().take_while(|&c| c != 0).collect();
            Valor::Texto(String::from_utf8_lossy(&s).into_owned())
        } else {
            let mut nums = Vec::with_capacity(cuenta as usize);
            for c in datos.chunks_exact(tam_tipo) {
                let x = match tipo {
                    1 | 7 => c[0] as f64,
                    6 => c[0] as i8 as f64,
                    3 => u16_(c) as f64,
                    8 => u16_(c) as i16 as f64,
                    4 => u32_(c) as f64,
                    9 => u32_(c) as i32 as f64,
                    11 => f32::from_bits(u32_(c)) as f64,
                    12 => f64::from_bits(u64_(c)),
                    16 | 18 => u64_(c) as f64,
                    17 => u64_(c) as i64 as f64,
                    // Racionales: numerador / denominador.
                    5 => u32_(&c[0..4]) as f64 / (u32_(&c[4..8]) as f64).max(1.0),
                    _ => (u32_(&c[0..4]) as i32) as f64 / ((u32_(&c[4..8]) as i32) as f64),
                };
                nums.push(x);
            }
            Valor::Numeros(nums)
        };
        t.insert(etiqueta, v);
    }
    Ok((le, t))
}

/// Georreferencia: tamaño del píxel (33550), punto de amarre (33922) y tipo de ráster de las claves
/// (34735, clave 1025): 1 = el amarre es la esquina del píxel, 2 = su centro. Devuelve el borde oeste
/// de la columna 0, el borde norte de la fila 0 y el paso en longitud y en latitud (grados).
fn georreferencia(
    t: &BTreeMap<u16, Valor>,
    ruta: &Path,
) -> Result<(f64, f64, f64, f64), TerrenoError> {
    let num = |k: u16| -> Option<&Vec<f64>> {
        match t.get(&k) {
            Some(Valor::Numeros(v)) => Some(v),
            _ => None,
        }
    };
    let escala_px = num(33550).ok_or_else(|| error(ruta, "sin ModelPixelScale"))?;
    let amarre = num(33922).ok_or_else(|| error(ruta, "sin ModelTiepoint"))?;
    if escala_px.len() < 2 || amarre.len() < 6 {
        return Err(error(ruta, "georreferencia incompleta"));
    }
    let mut tipo_raster = 1.0;
    if let Some(claves) = num(34735) {
        let mut k = 4;
        while k + 3 < claves.len() {
            if claves[k] == 1025.0 && claves[k + 1] == 0.0 {
                tipo_raster = claves[k + 3];
            }
            k += 4;
        }
    }
    let desfase = if tipo_raster == 2.0 { 0.5 } else { 0.0 };
    let (sx, sy) = (escala_px[0], escala_px[1]);
    Ok((
        amarre[3] - (amarre[0] + desfase) * sx,
        amarre[4] + (amarre[1] + desfase) * sy,
        sx,
        sy,
    ))
}

impl GeoTiff {
    /// Abre el archivo y lee su cabecera.
    pub fn abrir(ruta: &Path) -> Result<GeoTiff, TerrenoError> {
        let mut f = File::open(ruta).map_err(|e| error(ruta, e))?;
        let (le, t) = leer_etiquetas(&mut f, ruta)?;
        let num = |k: u16| -> Option<&Vec<f64>> {
            match t.get(&k) {
                Some(Valor::Numeros(v)) => Some(v),
                _ => None,
            }
        };
        let uno = |k: u16, defecto: f64| num(k).and_then(|v| v.first().copied()).unwrap_or(defecto);
        let texto = |k: u16| match t.get(&k) {
            Some(Valor::Texto(s)) => s.as_str(),
            _ => "",
        };
        let ancho = uno(256, 0.0) as usize;
        let alto = uno(257, 0.0) as usize;
        if ancho == 0 || alto == 0 || ancho > 1 << 20 || alto > 1 << 20 {
            return Err(error(ruta, "tamaño de imagen no válido"));
        }
        if uno(277, 1.0) != 1.0 {
            return Err(error(ruta, "solo una banda (SamplesPerPixel = 1)"));
        }
        if t.contains_key(&322) || t.contains_key(&324) {
            return Err(error(ruta, "imagen en teselas: no soportada (solo tiras)"));
        }
        let compresion = uno(259, 1.0) as u32;
        if !matches!(compresion, 1 | 8 | 32946) {
            return Err(error(
                ruta,
                format!("compresión {compresion} no soportada (solo ninguna o Deflate)"),
            ));
        }
        let predictor = uno(317, 1.0) as u16;
        if !matches!(predictor, 1 | 2) {
            return Err(error(ruta, format!("predictor {predictor} no soportado")));
        }
        if uno(258, 0.0) != 16.0 || uno(339, 1.0) != 1.0 {
            return Err(error(
                ruta,
                "solo píxeles de 16 bits sin signo (los de CHELSA)",
            ));
        }
        let filas_por_tira = (uno(278, alto as f64) as usize).clamp(1, alto);
        let tiras: Vec<u64> = num(273)
            .ok_or_else(|| error(ruta, "sin tiras"))?
            .iter()
            .map(|&x| x as u64)
            .collect();
        let cuentas: Vec<u64> = num(279)
            .ok_or_else(|| error(ruta, "sin tamaños de tira"))?
            .iter()
            .map(|&x| x as u64)
            .collect();
        if tiras.len() != alto.div_ceil(filas_por_tira) || cuentas.len() != tiras.len() {
            return Err(error(ruta, "número de tiras incoherente"));
        }
        let tam_archivo = f.metadata().map_err(|e| error(ruta, e))?.len();
        if tiras
            .iter()
            .zip(&cuentas)
            .any(|(&o, &c)| o.saturating_add(c) > tam_archivo || c > 256 << 20)
        {
            return Err(error(
                ruta,
                "una tira se sale del archivo (¿descarga a medias?)",
            ));
        }
        let (lon0, lat0, sx, sy) = georreferencia(&t, ruta)?;
        let meta = texto(42112);
        let leer_meta = |papel: &str, defecto: f64| -> f64 {
            // <Item ... role="scale" ...>0.1</Item>
            let clave = format!("role=\"{papel}\"");
            meta.find(&clave)
                .and_then(|p| meta[p..].find('>').map(|q| p + q + 1))
                .and_then(|p| meta[p..].find('<').map(|q| meta[p..p + q].trim()))
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(defecto)
        };
        Ok(GeoTiff {
            f,
            ruta: ruta.to_path_buf(),
            le,
            ancho,
            alto,
            filas_por_tira,
            tiras,
            cuentas,
            comprimida: compresion != 1,
            predictor,
            lon0,
            lat0,
            paso_lon: sx,
            paso_lat: sy,
            escala: leer_meta("scale", 1.0),
            desplazamiento: leer_meta("offset", 0.0),
            bruto: Vec::new(),
            tira: usize::MAX,
            valores: Vec::new(),
        })
    }

    /// Valores crudos de la fila `r` (sin escala).
    pub fn fila(&mut self, r: usize) -> Result<&[u16], TerrenoError> {
        if r >= self.alto {
            return Err(error(&self.ruta, format!("fila {r} fuera de la imagen")));
        }
        let s = r / self.filas_por_tira;
        if self.tira != s {
            let (desde, n) = (self.tiras[s], self.cuentas[s] as usize);
            self.bruto.resize(n, 0);
            self.f
                .seek(SeekFrom::Start(desde))
                .map_err(|e| error(&self.ruta, e))?;
            self.f
                .read_exact(&mut self.bruto)
                .map_err(|e| error(&self.ruta, e))?;
            let filas = self.filas_por_tira.min(self.alto - s * self.filas_por_tira);
            let necesarios = filas * self.ancho * 2;
            let datos = if self.comprimida {
                miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(&self.bruto, necesarios)
                    .map_err(|e| error(&self.ruta, format!("tira {s}: Deflate: {e:?}")))?
            } else {
                std::mem::take(&mut self.bruto)
            };
            if datos.len() < necesarios {
                return Err(error(&self.ruta, format!("tira {s} incompleta")));
            }
            self.valores.clear();
            self.valores
                .extend(datos[..necesarios].chunks_exact(2).map(|c| {
                    if self.le {
                        u16::from_le_bytes([c[0], c[1]])
                    } else {
                        u16::from_be_bytes([c[0], c[1]])
                    }
                }));
            if self.predictor == 2 {
                // Diferencias horizontales: cada valor es la suma acumulada de su fila, módulo 2^16.
                for fila in self.valores.chunks_exact_mut(self.ancho) {
                    let mut acumulado = 0u16;
                    for v in fila.iter_mut() {
                        acumulado = acumulado.wrapping_add(*v);
                        *v = acumulado;
                    }
                }
            }
            if !self.comprimida {
                self.bruto = datos;
            }
            self.tira = s;
        }
        let k = (r - s * self.filas_por_tira) * self.ancho;
        Ok(&self.valores[k..k + self.ancho])
    }
}

// --- Lector general (HydroSHEDS): teselas o tiras, LZW o Deflate, 8, 16 o 32 bits ------------------

/// Tipo de las muestras de un [`Raster`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tipo {
    U8,
    U16,
    I16,
    U32,
    I32,
    F32,
}

/// Imagen GeoTIFF de una banda en tiras o en teselas, sin comprimir, con Deflate, LZW o PackBits, con
/// o sin predictor horizontal (la de HydroSHEDS). Se lee por filas: cada fila sale de la banda de
/// bloques que la contiene, que se guarda descomprimida hasta que se pide otra.
pub struct Raster {
    f: File,
    ruta: std::path::PathBuf,
    le: bool,
    pub ancho: usize,
    pub alto: usize,
    pub tipo: Tipo,
    bytes: usize,
    bloque_ancho: usize,
    bloque_alto: usize,
    bloques_por_fila: usize,
    offsets: Vec<u64>,
    cuentas: Vec<u64>,
    compresion: u32,
    predictor: u16,
    /// Borde oeste de la columna 0 y borde norte de la fila 0 (grados), y paso del píxel.
    pub lon0: f64,
    pub lat0: f64,
    pub paso_lon: f64,
    pub paso_lat: f64,
    /// Valor «sin dato» declarado (etiqueta de GDAL), si lo hay.
    pub sin_dato: Option<f64>,
    banda: usize,
    datos: Vec<u8>,
}

impl Raster {
    /// Abre el archivo y lee su cabecera.
    pub fn abrir(ruta: &Path) -> Result<Raster, TerrenoError> {
        let mut f = File::open(ruta).map_err(|e| error(ruta, e))?;
        let (le, t) = leer_etiquetas(&mut f, ruta)?;
        let num = |k: u16| -> Option<&Vec<f64>> {
            match t.get(&k) {
                Some(Valor::Numeros(v)) => Some(v),
                _ => None,
            }
        };
        let uno = |k: u16, defecto: f64| num(k).and_then(|v| v.first().copied()).unwrap_or(defecto);
        let ancho = uno(256, 0.0) as usize;
        let alto = uno(257, 0.0) as usize;
        if ancho == 0 || alto == 0 || ancho > 1 << 22 || alto > 1 << 22 {
            return Err(error(ruta, "tamaño de imagen no válido"));
        }
        if uno(277, 1.0) != 1.0 {
            return Err(error(ruta, "solo una banda (SamplesPerPixel = 1)"));
        }
        let tipo = match (uno(339, 1.0) as u32, uno(258, 0.0) as u32) {
            (1, 8) => Tipo::U8,
            (1, 16) => Tipo::U16,
            (2, 16) => Tipo::I16,
            (1, 32) => Tipo::U32,
            (2, 32) => Tipo::I32,
            (3, 32) => Tipo::F32,
            (f, b) => {
                return Err(error(
                    ruta,
                    format!("píxel de {b} bits y formato {f} no soportado"),
                ));
            }
        };
        let bytes = match tipo {
            Tipo::U8 => 1,
            Tipo::U16 | Tipo::I16 => 2,
            _ => 4,
        };
        let compresion = uno(259, 1.0) as u32;
        if !matches!(compresion, 1 | 5 | 8 | 32946 | 32773) {
            return Err(error(ruta, format!("compresión {compresion} no soportada")));
        }
        let predictor = uno(317, 1.0) as u16;
        if !matches!(predictor, 1 | 2) {
            return Err(error(ruta, format!("predictor {predictor} no soportado")));
        }
        let (bloque_ancho, bloque_alto, offsets, cuentas) = if t.contains_key(&322) {
            let (tw, th) = (uno(322, 0.0) as usize, uno(323, 0.0) as usize);
            let o = num(324).ok_or_else(|| error(ruta, "sin TileOffsets"))?;
            let c = num(325).ok_or_else(|| error(ruta, "sin TileByteCounts"))?;
            (tw, th, o.clone(), c.clone())
        } else {
            let filas = (uno(278, alto as f64) as usize).clamp(1, alto);
            let o = num(273).ok_or_else(|| error(ruta, "sin tiras"))?;
            let c = num(279).ok_or_else(|| error(ruta, "sin tamaños de tira"))?;
            (ancho, filas, o.clone(), c.clone())
        };
        if bloque_ancho == 0 || bloque_alto == 0 || bloque_ancho * bloque_alto > 1 << 26 {
            return Err(error(ruta, "bloques de tamaño no válido"));
        }
        let bloques_por_fila = ancho.div_ceil(bloque_ancho);
        let n_bloques = bloques_por_fila * alto.div_ceil(bloque_alto);
        if offsets.len() != n_bloques || cuentas.len() != n_bloques {
            return Err(error(ruta, "número de bloques incoherente"));
        }
        let tam = f.metadata().map_err(|e| error(ruta, e))?.len();
        let offsets: Vec<u64> = offsets.iter().map(|&x| x as u64).collect();
        let cuentas: Vec<u64> = cuentas.iter().map(|&x| x as u64).collect();
        if offsets
            .iter()
            .zip(&cuentas)
            .any(|(&o, &c)| o.saturating_add(c) > tam || c > 1 << 28)
        {
            return Err(error(
                ruta,
                "un bloque se sale del archivo (¿descarga a medias?)",
            ));
        }
        let (lon0, lat0, paso_lon, paso_lat) = georreferencia(&t, ruta)?;
        let sin_dato = match t.get(&42113) {
            Some(Valor::Texto(s)) => s.trim().parse::<f64>().ok(),
            _ => None,
        };
        Ok(Raster {
            f,
            ruta: ruta.to_path_buf(),
            le,
            ancho,
            alto,
            tipo,
            bytes,
            bloque_ancho,
            bloque_alto,
            bloques_por_fila,
            offsets,
            cuentas,
            compresion,
            predictor,
            lon0,
            lat0,
            paso_lon,
            paso_lat,
            sin_dato,
            banda: usize::MAX,
            datos: Vec::new(),
        })
    }

    /// Bytes de la fila `r`, en little-endian, `ancho · bytes` de largo.
    pub fn fila_bytes(&mut self, r: usize) -> Result<&[u8], TerrenoError> {
        if r >= self.alto {
            return Err(error(&self.ruta, format!("fila {r} fuera de la imagen")));
        }
        let b = r / self.bloque_alto;
        if self.banda != b {
            self.leer_banda(b)?;
        }
        let fila_bytes = self.ancho * self.bytes;
        let k = (r - b * self.bloque_alto) * fila_bytes;
        Ok(&self.datos[k..k + fila_bytes])
    }

    /// Valores enteros de la fila `r` (los de 32 bits en coma flotante, truncados).
    pub fn fila_i64(&mut self, r: usize, out: &mut Vec<i64>) -> Result<(), TerrenoError> {
        let tipo = self.tipo;
        let b = self.fila_bytes(r)?;
        out.clear();
        match tipo {
            Tipo::U8 => out.extend(b.iter().map(|&x| x as i64)),
            Tipo::U16 => out.extend(
                b.chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]) as i64),
            ),
            Tipo::I16 => out.extend(
                b.chunks_exact(2)
                    .map(|c| i16::from_le_bytes([c[0], c[1]]) as i64),
            ),
            Tipo::U32 => out.extend(
                b.chunks_exact(4)
                    .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]) as i64),
            ),
            Tipo::I32 => out.extend(
                b.chunks_exact(4)
                    .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]) as i64),
            ),
            Tipo::F32 => out.extend(
                b.chunks_exact(4)
                    .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]) as i64),
            ),
        }
        Ok(())
    }

    fn leer_banda(&mut self, b: usize) -> Result<(), TerrenoError> {
        let (bw, bh, bs) = (self.bloque_ancho, self.bloque_alto, self.bytes);
        let filas = bh.min(self.alto - b * bh);
        self.datos.clear();
        self.datos.resize(filas * self.ancho * bs, 0);
        let mut bruto = Vec::new();
        for bx in 0..self.bloques_por_fila {
            let q = b * self.bloques_por_fila + bx;
            let (desde, n) = (self.offsets[q], self.cuentas[q] as usize);
            // Un bloque es una tira (filas que caben, todo el ancho) o una tesela entera, con relleno.
            let filas_bloque = if self.bloques_por_fila == 1 {
                filas
            } else {
                bh
            };
            let necesarios = filas_bloque * bw * bs;
            let bloque = if n == 0 {
                vec![0u8; necesarios]
            } else {
                bruto.resize(n, 0);
                self.f
                    .seek(SeekFrom::Start(desde))
                    .map_err(|e| error(&self.ruta, e))?;
                self.f
                    .read_exact(&mut bruto)
                    .map_err(|e| error(&self.ruta, e))?;
                descomprimir(self.compresion, &bruto, necesarios)
                    .map_err(|e| error(&self.ruta, format!("bloque {q}: {e}")))?
            };
            if bloque.len() < necesarios {
                return Err(error(&self.ruta, format!("bloque {q} incompleto")));
            }
            let x0 = bx * bw;
            let ancho_util = bw.min(self.ancho - x0);
            for fila in 0..filas {
                let src = &bloque[fila * bw * bs..(fila + 1) * bw * bs];
                let mut v = src.to_vec();
                if !self.le && bs > 1 {
                    v.chunks_exact_mut(bs).for_each(|c| c.reverse());
                }
                if self.predictor == 2 {
                    deshacer_predictor(&mut v, bs);
                }
                let dst = (fila * self.ancho + x0) * bs;
                self.datos[dst..dst + ancho_util * bs].copy_from_slice(&v[..ancho_util * bs]);
            }
        }
        self.banda = b;
        Ok(())
    }
}

/// Predictor horizontal: cada muestra es la suma acumulada de su fila, módulo 2^bits (muestras en
/// little-endian).
fn deshacer_predictor(v: &mut [u8], bs: usize) {
    match bs {
        1 => {
            let mut a = 0u8;
            for x in v.iter_mut() {
                a = a.wrapping_add(*x);
                *x = a;
            }
        }
        2 => {
            let mut a = 0u16;
            for c in v.chunks_exact_mut(2) {
                a = a.wrapping_add(u16::from_le_bytes([c[0], c[1]]));
                c.copy_from_slice(&a.to_le_bytes());
            }
        }
        _ => {
            let mut a = 0u32;
            for c in v.chunks_exact_mut(4) {
                a = a.wrapping_add(u32::from_le_bytes([c[0], c[1], c[2], c[3]]));
                c.copy_from_slice(&a.to_le_bytes());
            }
        }
    }
}

fn descomprimir(compresion: u32, bruto: &[u8], necesarios: usize) -> Result<Vec<u8>, String> {
    match compresion {
        1 => Ok(bruto.to_vec()),
        8 | 32946 => miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(bruto, necesarios)
            .map_err(|e| format!("Deflate: {e:?}")),
        5 => lzw(bruto, necesarios),
        _ => packbits(bruto, necesarios),
    }
}

/// LZW de TIFF (códigos de 9 a 12 bits, el primero el más significativo, con el cambio de anchura un
/// código antes; 256 limpia la tabla y 257 acaba).
pub fn lzw(entrada: &[u8], esperado: usize) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(esperado);
    let mut prefijo = vec![0u16; 4096];
    let mut ultimo = vec![0u8; 4096];
    let mut primero = vec![0u8; 4096];
    let mut largo = vec![0u16; 4096];
    for i in 0..256 {
        ultimo[i] = i as u8;
        primero[i] = i as u8;
        largo[i] = 1;
    }
    let (mut siguiente, mut bits) = (258usize, 9u32);
    let (mut acum, mut n_acum, mut p) = (0u32, 0u32, 0usize);
    let mut anterior: Option<usize> = None;
    let mut tmp: Vec<u8> = Vec::with_capacity(4096);
    loop {
        while n_acum < bits {
            if p >= entrada.len() {
                return Ok(out);
            }
            acum = (acum << 8) | entrada[p] as u32;
            p += 1;
            n_acum += 8;
        }
        let codigo = ((acum >> (n_acum - bits)) & ((1 << bits) - 1)) as usize;
        n_acum -= bits;
        if codigo == 256 {
            siguiente = 258;
            bits = 9;
            anterior = None;
            continue;
        }
        if codigo == 257 {
            return Ok(out);
        }
        let Some(prev) = anterior else {
            if codigo > 255 {
                return Err("LZW: primer código fuera de la tabla".into());
            }
            out.push(codigo as u8);
            anterior = Some(codigo);
            continue;
        };
        let cabeza = if codigo < siguiente {
            primero[codigo]
        } else if codigo == siguiente {
            primero[prev]
        } else {
            return Err("LZW: código inválido".into());
        };
        if siguiente < 4096 {
            prefijo[siguiente] = prev as u16;
            ultimo[siguiente] = cabeza;
            primero[siguiente] = primero[prev];
            largo[siguiente] = largo[prev] + 1;
            siguiente += 1;
        }
        // Escribe la cadena del código, de atrás adelante.
        tmp.clear();
        let mut c = codigo;
        loop {
            tmp.push(ultimo[c]);
            if largo[c] == 1 {
                break;
            }
            c = prefijo[c] as usize;
        }
        out.extend(tmp.iter().rev());
        if siguiente + 1 >= (1 << bits) && bits < 12 {
            bits += 1;
        }
        anterior = Some(codigo);
        if out.len() >= esperado {
            return Ok(out);
        }
    }
}

/// PackBits de TIFF.
fn packbits(entrada: &[u8], esperado: usize) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(esperado);
    let mut p = 0;
    while p < entrada.len() && out.len() < esperado {
        let n = entrada[p] as i8;
        p += 1;
        if n >= 0 {
            let k = n as usize + 1;
            let fin = (p + k).min(entrada.len());
            out.extend_from_slice(&entrada[p..fin]);
            p = fin;
        } else if n != -128 {
            let k = (1 - n as i32) as usize;
            if p < entrada.len() {
                out.extend(std::iter::repeat_n(entrada[p], k));
                p += 1;
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Codificador LZW de TIFF (para la prueba): códigos de 9 a 12 bits, el primero el más
    /// significativo, la anchura sube un código antes y la tabla se limpia al llenarse.
    fn lzw_codificar(datos: &[u8]) -> Vec<u8> {
        use std::collections::HashMap;
        let mut out = Vec::new();
        let (mut acum, mut n_acum) = (0u64, 0u32);
        let mut emitir = |codigo: u32, bits: u32, out: &mut Vec<u8>| {
            acum = (acum << bits) | codigo as u64;
            n_acum += bits;
            while n_acum >= 8 {
                out.push((acum >> (n_acum - 8)) as u8);
                n_acum -= 8;
            }
        };
        let mut tabla: HashMap<Vec<u8>, u32> = (0..256u32).map(|i| (vec![i as u8], i)).collect();
        let (mut siguiente, mut bits) = (258u32, 9u32);
        emitir(256, bits, &mut out);
        let mut w: Vec<u8> = Vec::new();
        for &b in datos {
            let mut wb = w.clone();
            wb.push(b);
            if tabla.contains_key(&wb) {
                w = wb;
                continue;
            }
            emitir(tabla[&w], bits, &mut out);
            // Como libtiff: con la tabla llena (4 094), código de limpieza en vez de entrada nueva;
            // si no, la entrada, y la anchura sube cuando el siguiente código ya no cabe.
            if siguiente == 4094 {
                emitir(256, bits, &mut out);
                tabla = (0..256u32).map(|i| (vec![i as u8], i)).collect();
                siguiente = 258;
                bits = 9;
            } else {
                tabla.insert(wb, siguiente);
                siguiente += 1;
                if siguiente > (1 << bits) - 1 && bits < 12 {
                    bits += 1;
                }
            }
            w = vec![b];
        }
        if !w.is_empty() {
            emitir(tabla[&w], bits, &mut out);
        }
        emitir(257, bits, &mut out);
        if n_acum > 0 {
            out.push((acum << (8 - n_acum)) as u8);
        }
        out
    }

    #[test]
    fn lzw_recupera_lo_comprimido() {
        // Datos con repeticiones (como una rejilla de direcciones) y con ruido, que llenan la tabla.
        let mut datos = Vec::new();
        let mut x = 12345u32;
        for k in 0..60_000u32 {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            datos.push(if k % 7 < 4 { 16 } else { (x >> 24) as u8 });
        }
        let comprimido = lzw_codificar(&datos);
        assert!(comprimido.len() < datos.len());
        assert_eq!(lzw(&comprimido, datos.len()).expect("decodificar"), datos);
    }

    #[test]
    fn el_predictor_horizontal_se_deshace_por_muestra() {
        let mut v: Vec<u8> = [1u32, 2, 3].iter().flat_map(|x| x.to_le_bytes()).collect();
        deshacer_predictor(&mut v, 4);
        let s: Vec<u32> = v
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        assert_eq!(s, vec![1, 3, 6]);
    }
}
