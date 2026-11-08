//! Modo Tierra real (`docs/design/terreno/diseno_modo_tierra.md`, Fase 1; diseño técnico en
//! `docs/design/terreno/modo-tierra-diseno.md`).
//!
//! El proveedor de mundo de la Tierra: la altura y el hielo de cada celda, a cualquier nivel de la
//! malla (ETOPO 2022), y el clima de cada día (CHELSA V2.1 1981–2010 con la variabilidad de los años
//! reales de CRU TS 4.09). El resto del motor recibe los mismos campos que del planeta inventado.
//!
//! - [`carga`]: proyección de las fuentes a la malla del nivel de carga, con su caché, y los niveles
//!   gruesos como medias por área (conservación exacta entre niveles, salvo el redondeo).
//! - [`clima`]: años reales remuestreados por bloques, mapas mensuales y clima del día.

pub mod carga;
pub mod clima;
mod geotiff;
mod netcdf3;

pub use carga::cargar;

use crate::geometry::{V3, face_n, locate, split_key};

/// Días de cada mes del año de 365 días del modo Tierra.
pub const DIAS_MES: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// Tablas de un nivel de la malla: una entrada por celda, en el orden `(cara·n + j)·n + i`.
#[derive(Debug, Clone)]
pub struct Nivel {
    /// Celdas por arista de cara.
    pub n: u32,
    /// Área real de cada celda (m²): el peso de las medias entre niveles.
    pub area: Vec<f64>,
    /// Altura media (m, positiva sobre el nivel del mar actual).
    pub z: Vec<f64>,
    /// Fracción de la celda con hielo (casquetes de ETOPO: superficie − roca > 0).
    pub hielo: Vec<f64>,
    /// Climatología mensual: temperatura (°C) y precipitación (mm/mes), 12 valores por celda
    /// (`[k·12 + mes]`).
    pub t: Vec<f32>,
    pub p: Vec<f32>,
    /// Fracción de la celda con píxeles de CHELSA (0 al norte de 84° N, donde CHELSA no llega: esas
    /// celdas toman el clima de la celda más cercana con dato, en el mismo meridiano).
    pub cobertura: Vec<f32>,
}

impl Nivel {
    /// Índice de la celda `(c, i, j)` de este nivel.
    #[inline]
    pub fn indice(&self, c: u8, i: u32, j: u32) -> usize {
        ((c as usize * self.n as usize + j as usize) * self.n as usize) + i as usize
    }

    /// Número de celdas.
    pub fn len(&self) -> usize {
        self.z.len()
    }

    /// `true` si no tiene celdas.
    pub fn is_empty(&self) -> bool {
        self.z.is_empty()
    }
}

/// Series mensuales de CRU TS en su rejilla (solo las celdas de tierra), en décimas.
#[derive(Debug, Clone)]
pub struct Cru {
    /// Primer año y número de años completos.
    pub anio0: i32,
    pub n_anios: usize,
    /// Centro de cada celda de tierra (grados).
    pub lat: Vec<f32>,
    pub lon: Vec<f32>,
    /// Temperatura (décimas de °C) y precipitación (décimas de mm/mes): `[mes_global·n + celda]`, con
    /// `mes_global = (año − anio0)·12 + mes`.
    pub tmp: Vec<i16>,
    pub pre: Vec<u16>,
    /// Valores de lluvia mensual por encima de 6 553,5 mm, recortados (se cuentan, deberían ser 0).
    pub pre_recortados: u64,
    /// Por año: fracción del área de tierra de CRU sin ninguna estación en la temperatura y en la
    /// lluvia (media de los 12 meses). Donde no hay estaciones, CRU TS vuelve a su climatología: ese
    /// año no tiene variabilidad propia allí.
    pub sin_estaciones: Vec<[f32; 2]>,
}

impl Cru {
    /// Número de celdas de tierra.
    pub fn n_celdas(&self) -> usize {
        self.lat.len()
    }

    /// Último año.
    pub fn anio_final(&self) -> i32 {
        self.anio0 + self.n_anios as i32 - 1
    }

    /// Temperatura (°C) de la celda `c` en el mes `m` del año `a`.
    #[inline]
    pub fn t(&self, a: i32, m: usize, c: usize) -> f64 {
        let g = (a - self.anio0) as usize * 12 + m;
        self.tmp[g * self.n_celdas() + c] as f64 / 10.0
    }

    /// Precipitación (mm/mes) de la celda `c` en el mes `m` del año `a`.
    #[inline]
    pub fn p(&self, a: i32, m: usize, c: usize) -> f64 {
        let g = (a - self.anio0) as usize * 12 + m;
        self.pre[g * self.n_celdas() + c] as f64 / 10.0
    }
}

/// Datos de la Tierra cargados: tablas por nivel y series de CRU TS.
#[derive(Debug, Clone)]
pub struct DatosTierra {
    pub n0: u32,
    pub nivel_carga: u32,
    /// Niveles 0..=`nivel_carga`.
    pub niveles: Vec<Nivel>,
    pub cru: Cru,
    /// Celda de CRU de cada celda del nivel de carga (`u32::MAX`: ninguna; sin variabilidad).
    pub cru_de_celda: Vec<u32>,
    /// Huella de las fuentes y del código: entra en la clave de la caché del mundo.
    pub huella: u64,
}

impl DatosTierra {
    /// Nivel de las tablas que corresponde a una celda de nivel `l` y el índice de su celda (la
    /// antecesora en el nivel de carga si la celda es más fina).
    #[inline]
    pub fn celda(&self, c: u8, l: u32, i: u32, j: u32) -> (usize, usize) {
        let (lv, i, j) = if l > self.nivel_carga {
            let s = l - self.nivel_carga;
            (self.nivel_carga, i >> s, j >> s)
        } else {
            (l, i, j)
        };
        (lv as usize, self.niveles[lv as usize].indice(c, i, j))
    }

    /// Lo mismo a partir de la clave de la celda.
    #[inline]
    pub fn celda_de_clave(&self, k: u64) -> (usize, usize) {
        let (c, l, i, j) = split_key(k);
        self.celda(c, l, i, j)
    }

    /// Altura media (m) de la celda de nivel `l` que contiene la dirección `d`.
    #[inline]
    pub fn h_en(&self, d: V3, l: u32) -> f64 {
        let lv = l.min(self.nivel_carga);
        let (c, i, j) = locate(d, face_n(self.n0, lv));
        let t = &self.niveles[lv as usize];
        t.z[t.indice(c, i, j)]
    }

    /// Fracción con hielo de la celda `(c, l, i, j)`.
    #[inline]
    pub fn hielo(&self, c: u8, l: u32, i: u32, j: u32) -> f64 {
        let (lv, k) = self.celda(c, l, i, j);
        self.niveles[lv].hielo[k]
    }

    /// Temperatura media anual (°C) y precipitación anual (mm) de la climatología de una celda.
    pub fn clima_anual(&self, clave: u64) -> (f64, f64) {
        let (lv, k) = self.celda_de_clave(clave);
        let t = &self.niveles[lv];
        let (mut tm, mut pa) = (0.0, 0.0);
        for (m, &dias) in DIAS_MES.iter().enumerate() {
            tm += t.t[k * 12 + m] as f64 * dias as f64;
            pa += t.p[k * 12 + m] as f64;
        }
        (tm / 365.0, pa)
    }
}
