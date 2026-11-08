//! Lector mínimo de NetCDF-3 clásico, en flujo desde un gzip (modo Tierra: las series de CRU TS 4).
//!
//! Lee la cabecera (dimensiones, atributos y variables), las variables fijas y después los registros
//! uno a uno: no descomprime el archivo entero ni lo guarda en memoria. Formato: «NetCDF Classic
//! Format Specification» (Unidata), versiones 1 (desplazamientos de 32 bits) y 2 (de 64). Los
//! archivos son datos no fiables: cada tamaño se comprueba antes de reservar nada.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use miniz_oxide::inflate::stream::{InflateState, inflate};
use miniz_oxide::{DataFormat, MZError, MZFlush, MZStatus};

use crate::error::TerrenoError;

fn error(ruta: &Path, m: impl std::fmt::Display) -> TerrenoError {
    TerrenoError::Tierra(format!("{}: {m}", ruta.display()))
}

/// Flujo descomprimido de un archivo gzip (RFC 1952, un solo miembro).
pub struct Gzip {
    f: BufReader<File>,
    entrada: Vec<u8>,
    ini: usize,
    fin: usize,
    estado: Box<InflateState>,
    fin_entrada: bool,
    terminado: bool,
}

impl Gzip {
    /// Abre el archivo y salta su cabecera.
    pub fn abrir(ruta: &Path) -> Result<Gzip, TerrenoError> {
        let mut f =
            BufReader::with_capacity(1 << 20, File::open(ruta).map_err(|e| error(ruta, e))?);
        let mut c = [0u8; 10];
        f.read_exact(&mut c).map_err(|e| error(ruta, e))?;
        if c[0] != 0x1f || c[1] != 0x8b || c[2] != 8 {
            return Err(error(ruta, "no es un gzip con Deflate"));
        }
        let flg = c[3];
        let mut uno = [0u8; 1];
        if flg & 4 != 0 {
            let mut l = [0u8; 2];
            f.read_exact(&mut l).map_err(|e| error(ruta, e))?;
            let mut extra = vec![0u8; u16::from_le_bytes(l) as usize];
            f.read_exact(&mut extra).map_err(|e| error(ruta, e))?;
        }
        for bit in [8u8, 16] {
            if flg & bit != 0 {
                loop {
                    f.read_exact(&mut uno).map_err(|e| error(ruta, e))?;
                    if uno[0] == 0 {
                        break;
                    }
                }
            }
        }
        if flg & 2 != 0 {
            let mut crc = [0u8; 2];
            f.read_exact(&mut crc).map_err(|e| error(ruta, e))?;
        }
        Ok(Gzip {
            f,
            entrada: vec![0; 1 << 20],
            ini: 0,
            fin: 0,
            estado: InflateState::new_boxed(DataFormat::Raw),
            fin_entrada: false,
            terminado: false,
        })
    }
}

impl Read for Gzip {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        use std::io::{Error, ErrorKind};
        if out.is_empty() || self.terminado {
            return Ok(0);
        }
        loop {
            if self.ini == self.fin && !self.fin_entrada {
                self.fin = self.f.read(&mut self.entrada)?;
                self.ini = 0;
                self.fin_entrada = self.fin == 0;
            }
            let r = inflate(
                &mut self.estado,
                &self.entrada[self.ini..self.fin],
                out,
                MZFlush::None,
            );
            self.ini += r.bytes_consumed;
            let agotada = self.fin_entrada && self.ini == self.fin;
            match r.status {
                Ok(MZStatus::StreamEnd) => {
                    self.terminado = true;
                    return Ok(r.bytes_written);
                }
                Ok(_) if r.bytes_written > 0 => return Ok(r.bytes_written),
                Ok(_) | Err(MZError::Buf) => {
                    if agotada {
                        return Err(Error::new(ErrorKind::UnexpectedEof, "gzip cortado"));
                    }
                    if r.bytes_consumed == 0 && self.ini < self.fin {
                        return Err(Error::new(ErrorKind::InvalidData, "Deflate sin avance"));
                    }
                }
                Err(e) => {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!("Deflate: {e:?}"),
                    ));
                }
            }
        }
    }
}

/// Atributo de una variable o global.
#[derive(Debug, Clone)]
pub enum Atributo {
    Texto(String),
    Numeros(Vec<f64>),
}

/// Variable de la cabecera.
#[derive(Debug, Clone)]
pub struct Variable {
    pub nombre: String,
    /// Índices de sus dimensiones (la primera es la de registro si la variable es de registro).
    pub dims: Vec<usize>,
    /// Tipo NetCDF: 1 byte, 2 char, 3 short, 4 int, 5 float, 6 double.
    pub tipo: u32,
    pub vsize: u64,
    pub begin: u64,
    pub atributos: Vec<(String, Atributo)>,
}

impl Variable {
    /// Primer valor numérico del atributo `nombre`.
    pub fn atributo_num(&self, nombre: &str) -> Option<f64> {
        self.atributos.iter().find_map(|(k, a)| match a {
            Atributo::Numeros(v) if k == nombre => v.first().copied(),
            _ => None,
        })
    }

    /// Texto del atributo `nombre`.
    pub fn atributo_texto(&self, nombre: &str) -> Option<&str> {
        self.atributos.iter().find_map(|(k, a)| match a {
            Atributo::Texto(s) if k == nombre => Some(s.as_str()),
            _ => None,
        })
    }
}

/// Archivo NetCDF-3 leído en flujo.
pub struct Netcdf3<R: Read> {
    r: R,
    ruta: std::path::PathBuf,
    pos: u64,
    pub numrecs: u64,
    pub dims: Vec<(String, u64)>,
    pub vars: Vec<Variable>,
    recsize: u64,
    begin_rec: u64,
}

const TAM_TIPO: [usize; 7] = [0, 1, 1, 2, 4, 4, 8];

impl<R: Read> Netcdf3<R> {
    /// Lee la cabecera.
    pub fn leer(r: R, ruta: &Path) -> Result<Netcdf3<R>, TerrenoError> {
        let mut nc = Netcdf3 {
            r,
            ruta: ruta.to_path_buf(),
            pos: 0,
            numrecs: 0,
            dims: Vec::new(),
            vars: Vec::new(),
            recsize: 0,
            begin_rec: 0,
        };
        let magia = nc.bytes(4)?;
        if &magia[..3] != b"CDF" || !matches!(magia[3], 1 | 2) {
            return Err(error(ruta, "no es un NetCDF-3 clásico"));
        }
        let ancho64 = magia[3] == 2;
        nc.numrecs = nc.u32()? as u64;
        // Dimensiones.
        let (etiqueta, n) = (nc.u32()?, nc.u32()?);
        if !(etiqueta == 0x0A || (etiqueta == 0 && n == 0)) || n > 1024 {
            return Err(error(ruta, "lista de dimensiones no válida"));
        }
        for _ in 0..n {
            let nombre = nc.nombre()?;
            let largo = nc.u32()? as u64;
            nc.dims.push((nombre, largo));
        }
        let _globales = nc.atributos()?;
        // Variables.
        let (etiqueta, n) = (nc.u32()?, nc.u32()?);
        if !(etiqueta == 0x0B || (etiqueta == 0 && n == 0)) || n > 4096 {
            return Err(error(ruta, "lista de variables no válida"));
        }
        for _ in 0..n {
            let nombre = nc.nombre()?;
            let nd = nc.u32()?;
            if nd > 64 {
                return Err(error(ruta, "variable con demasiadas dimensiones"));
            }
            let mut dims = Vec::with_capacity(nd as usize);
            for _ in 0..nd {
                let d = nc.u32()? as usize;
                if d >= nc.dims.len() {
                    return Err(error(ruta, "dimensión inexistente"));
                }
                dims.push(d);
            }
            let atributos = nc.atributos()?;
            let tipo = nc.u32()?;
            if !(1..=6).contains(&tipo) {
                return Err(error(ruta, format!("tipo {tipo} desconocido")));
            }
            let vsize = nc.u32()? as u64;
            let begin = if ancho64 {
                (nc.u32()? as u64) << 32 | nc.u32()? as u64
            } else {
                nc.u32()? as u64
            };
            nc.vars.push(Variable {
                nombre,
                dims,
                tipo,
                vsize,
                begin,
                atributos,
            });
        }
        let de_registro: Vec<&Variable> = nc.vars.iter().filter(|v| nc.es_de_registro(v)).collect();
        // Con una sola variable de registro, el registro no se rellena a 4 bytes (especificación).
        nc.recsize = if de_registro.len() == 1 {
            nc.tam_dato(de_registro[0])
        } else {
            de_registro.iter().map(|v| v.vsize).sum()
        };
        nc.begin_rec = de_registro.iter().map(|v| v.begin).min().unwrap_or(0);
        if nc.recsize > 1 << 30 {
            return Err(error(ruta, "registro demasiado grande"));
        }
        Ok(nc)
    }

    /// `true` si la variable es de registro (su primera dimensión es la ilimitada).
    pub fn es_de_registro(&self, v: &Variable) -> bool {
        v.dims.first().is_some_and(|&d| self.dims[d].1 == 0)
    }

    /// Bytes de un valor de la variable por registro (o de toda ella si es fija), sin relleno.
    fn tam_dato(&self, v: &Variable) -> u64 {
        let n: u64 = v
            .dims
            .iter()
            .skip(usize::from(self.es_de_registro(v)))
            .map(|&d| self.dims[d].1)
            .product();
        n * TAM_TIPO[v.tipo as usize] as u64
    }

    /// La variable `nombre`.
    pub fn variable(&self, nombre: &str) -> Result<&Variable, TerrenoError> {
        self.vars
            .iter()
            .find(|v| v.nombre == nombre)
            .ok_or_else(|| error(&self.ruta, format!("sin la variable «{nombre}»")))
    }

    /// Datos de una variable fija. Se piden en orden de su posición en el archivo y antes de los
    /// registros (el archivo se lee en flujo, sin volver atrás).
    pub fn fija(&mut self, nombre: &str) -> Result<Vec<u8>, TerrenoError> {
        let v = self.variable(nombre)?.clone();
        if self.es_de_registro(&v) {
            return Err(error(&self.ruta, format!("«{nombre}» es de registro")));
        }
        self.saltar(v.begin)?;
        let n = self.tam_dato(&v);
        self.bytes(n as usize)
    }

    /// Recorre los registros: para cada uno llama a `f(k, datos)`, con los bytes de cada variable de
    /// `nombres` en ese registro (big-endian, sin relleno).
    pub fn registros(
        &mut self,
        nombres: &[&str],
        mut f: impl FnMut(usize, &[&[u8]]) -> Result<(), TerrenoError>,
    ) -> Result<(), TerrenoError> {
        let mut trozos = Vec::with_capacity(nombres.len());
        for &n in nombres {
            let v = self.variable(n)?;
            if !self.es_de_registro(v) {
                return Err(error(&self.ruta, format!("«{n}» no es de registro")));
            }
            let desde = (v.begin - self.begin_rec) as usize;
            let tam = self.tam_dato(v) as usize;
            if desde + tam > self.recsize as usize {
                return Err(error(&self.ruta, "variable fuera del registro"));
            }
            trozos.push((desde, tam));
        }
        self.saltar(self.begin_rec)?;
        let mut rec = vec![0u8; self.recsize as usize];
        for k in 0..self.numrecs as usize {
            self.r
                .read_exact(&mut rec)
                .map_err(|e| error(&self.ruta, format!("registro {k}: {e}")))?;
            self.pos += self.recsize;
            let datos: Vec<&[u8]> = trozos.iter().map(|&(d, t)| &rec[d..d + t]).collect();
            f(k, &datos)?;
        }
        Ok(())
    }

    fn saltar(&mut self, hasta: u64) -> Result<(), TerrenoError> {
        if hasta < self.pos {
            return Err(error(&self.ruta, "lectura hacia atrás en un flujo"));
        }
        let mut resto = hasta - self.pos;
        let mut tirar = [0u8; 8192];
        while resto > 0 {
            let n = resto.min(tirar.len() as u64) as usize;
            self.r
                .read_exact(&mut tirar[..n])
                .map_err(|e| error(&self.ruta, e))?;
            resto -= n as u64;
        }
        self.pos = hasta;
        Ok(())
    }

    fn bytes(&mut self, n: usize) -> Result<Vec<u8>, TerrenoError> {
        if n > 256 << 20 {
            return Err(error(&self.ruta, "bloque demasiado grande"));
        }
        let mut b = vec![0u8; n];
        self.r
            .read_exact(&mut b)
            .map_err(|e| error(&self.ruta, e))?;
        self.pos += n as u64;
        Ok(b)
    }

    fn u32(&mut self) -> Result<u32, TerrenoError> {
        let b = self.bytes(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn nombre(&mut self) -> Result<String, TerrenoError> {
        let n = self.u32()? as usize;
        if n > 1 << 16 {
            return Err(error(&self.ruta, "nombre demasiado largo"));
        }
        let b = self.bytes(n.div_ceil(4) * 4)?;
        Ok(String::from_utf8_lossy(&b[..n]).into_owned())
    }

    fn atributos(&mut self) -> Result<Vec<(String, Atributo)>, TerrenoError> {
        let (etiqueta, n) = (self.u32()?, self.u32()?);
        if !(etiqueta == 0x0C || (etiqueta == 0 && n == 0)) || n > 4096 {
            return Err(error(&self.ruta, "lista de atributos no válida"));
        }
        let mut out = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let nombre = self.nombre()?;
            let tipo = self.u32()?;
            if !(1..=6).contains(&tipo) {
                return Err(error(&self.ruta, "atributo de tipo desconocido"));
            }
            let m = self.u32()? as usize;
            let tam = TAM_TIPO[tipo as usize];
            if m > 1 << 24 {
                return Err(error(&self.ruta, "atributo demasiado largo"));
            }
            let b = self.bytes((m * tam).div_ceil(4) * 4)?;
            let b = &b[..m * tam];
            let a = if tipo == 2 {
                Atributo::Texto(String::from_utf8_lossy(b).into_owned())
            } else {
                Atributo::Numeros(
                    b.chunks_exact(tam)
                        .map(|c| match tipo {
                            1 => c[0] as i8 as f64,
                            3 => i16::from_be_bytes([c[0], c[1]]) as f64,
                            4 => i32::from_be_bytes([c[0], c[1], c[2], c[3]]) as f64,
                            5 => f32::from_be_bytes([c[0], c[1], c[2], c[3]]) as f64,
                            _ => {
                                f64::from_be_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]])
                            }
                        })
                        .collect(),
                )
            };
            out.push((nombre, a));
        }
        Ok(out)
    }
}

/// Fecha civil (año, mes) de un número de días desde el 1 de enero de 1900, en el calendario
/// gregoriano (algoritmo de los días civiles de H. Hinnant).
pub fn anio_mes_desde_1900(dias: f64) -> (i32, u32) {
    // Días desde el 1970-01-01 del 1900-01-01: −25 567.
    let z = dias.floor() as i64 - 25_567 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y as i32, m as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn las_fechas_de_cru_salen_bien() {
        // CRU TS: el primer registro es el 16 de enero de 1901 (380 días desde el 1900-01-01).
        assert_eq!(anio_mes_desde_1900(380.0), (1901, 1));
        assert_eq!(anio_mes_desde_1900(0.0), (1900, 1));
        assert_eq!(anio_mes_desde_1900(59.0), (1900, 3)); // 1900 no es bisiesto
        assert_eq!(anio_mes_desde_1900(36_890.0), (2001, 1));
        assert_eq!(anio_mes_desde_1900(45_306.0), (2024, 1));
    }
}
