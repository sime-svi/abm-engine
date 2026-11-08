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

impl GeoTiff {
    /// Abre el archivo y lee su cabecera.
    pub fn abrir(ruta: &Path) -> Result<GeoTiff, TerrenoError> {
        let mut f = File::open(ruta).map_err(|e| error(ruta, e))?;
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
            (u64_(&leer(&mut f, ifd, 8)?), 20usize, ifd + 8)
        } else {
            (u16_(&leer(&mut f, ifd, 2)?) as u64, 12usize, ifd + 2)
        };
        if n_etiquetas > 4096 {
            return Err(error(ruta, "demasiadas etiquetas"));
        }
        let b = leer(&mut f, p0, n_etiquetas as usize * tam)?;
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
                leer(&mut f, desde, nb)?
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
        // Georreferencia: tamaño del píxel (33550), punto de amarre (33922) y tipo de ráster de las
        // claves (34735, clave 1025): 1 = el amarre es la esquina del píxel, 2 = su centro.
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
            lon0: amarre[3] - (amarre[0] + desfase) * sx,
            lat0: amarre[4] + (amarre[1] + desfase) * sy,
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
