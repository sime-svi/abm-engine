//! Estructuras de datos del terreno: topología multinivel (hojas del bosque de quadtrees) y campos
//! SoA.
//!
//! Diseño: `incremento-1-diseno.md` §3 e `incremento-2-diseno.md` §2. Las celdas NO son entidades
//! ECS sino columnas `Vec` alineadas con `topo.keys` (claves ordenadas): contiguo en memoria (caché),
//! orden de iteración estable (determinismo) y sin asignaciones por tick. La posición de una hoja
//! en `keys` es su índice ("slot") en todas las columnas; solo cambia en los pasos de topología.

use serde::{Deserialize, Serialize};

use crate::terreno::geometry::split_key;
use crate::terreno::statics::StaticGen;

/// Estado discreto de la superficie de una celda.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    Tierra,
    Oceano,
    Lago,
    Rio,
}

/// Bioma etiquetado (emerge de temperatura y precipitación, diagrama de Whittaker).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Biome {
    Oceano,
    Hielo,
    Desierto,
    Tundra,
    Taiga,
    Pradera,
    Sabana,
    BosqueTemplado,
    Selva,
}

impl Biome {
    /// Todas las variantes de bioma, en orden fijo (para métricas por bioma).
    pub const ALL: [Biome; 9] = [
        Biome::Oceano,
        Biome::Hielo,
        Biome::Desierto,
        Biome::Tundra,
        Biome::Taiga,
        Biome::Pradera,
        Biome::Sabana,
        Biome::BosqueTemplado,
        Biome::Selva,
    ];

    /// Nombre estable en snake_case (para claves de métricas).
    pub fn key(self) -> &'static str {
        match self {
            Biome::Oceano => "oceano",
            Biome::Hielo => "hielo",
            Biome::Desierto => "desierto",
            Biome::Tundra => "tundra",
            Biome::Taiga => "taiga",
            Biome::Pradera => "pradera",
            Biome::Sabana => "sabana",
            Biome::BosqueTemplado => "bosque_templado",
            Biome::Selva => "selva",
        }
    }

    /// Posición del bioma en [`Biome::ALL`].
    pub fn index(self) -> usize {
        self as usize
    }
}

/// Par de hojas vecinas (`a < b`, posiciones en `keys`) con su peso geométrico: `1` si comparten
/// arista, `1/√2` si solo se tocan por una esquina (incremento 2 §5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pair {
    pub a: u32,
    pub b: u32,
    pub g: f64,
}

/// Topología del cubo esférico: hojas activas (a niveles mezclados), su geometría y su vecindad.
#[derive(Clone)]
pub struct Topology {
    /// Celdas por arista de cara en el nivel 0.
    pub n0: u32,
    /// Niveles mínimo y máximo del árbol.
    pub l_min: u32,
    pub l_max: u32,
    /// Número de hojas activas.
    pub num_cells: usize,
    /// Radio del planeta (m).
    pub r_planet: f64,
    /// Claves de las hojas, **ordenadas** (orden canónico de iteración).
    pub keys: Vec<u64>,
    /// Nivel de cada hoja.
    pub level: Vec<u8>,
    /// Latitud y longitud de cada hoja (radianes).
    pub lat: Vec<f64>,
    pub lon: Vec<f64>,
    /// Área real de cada hoja (m²).
    pub area: Vec<f64>,
    /// Base local este/norte (unitarios 3D) de cada hoja, para gradientes y viento.
    pub east: Vec<[f64; 3]>,
    pub north: Vec<[f64; 3]>,
    /// Dirección unitaria del centro de cada hoja (3D).
    pub dir: Vec<[f64; 3]>,
    /// Vecindad en formato CSR: los vecinos de la hoja `s` son `nb[nb_off[s]..nb_off[s+1]]`,
    /// ordenados. Simétrica por construcción.
    pub nb_off: Vec<u32>,
    pub nb: Vec<u32>,
    /// Pares de vecinos únicos (`a < b`), ordenados: la difusión los recorre en este orden.
    pub pairs: Vec<Pair>,
}

impl Topology {
    /// Vecinos (posiciones) de la hoja `s`.
    #[inline]
    pub fn neighbors(&self, s: usize) -> &[u32] {
        &self.nb[self.nb_off[s] as usize..self.nb_off[s + 1] as usize]
    }

    /// Posición de la hoja con clave `k`, si existe (búsqueda binaria: sin `HashMap`).
    #[inline]
    pub fn slot_of(&self, k: u64) -> Option<usize> {
        self.keys.binary_search(&k).ok()
    }

    /// Cara del cubo de la hoja `s`.
    #[inline]
    pub fn face(&self, s: usize) -> u8 {
        split_key(self.keys[s]).0
    }

    /// Lado característico de la hoja `s` (km): raíz de su área real.
    #[inline]
    pub fn side_km(&self, s: usize) -> f64 {
        self.area[s].sqrt() / 1000.0
    }

    /// Nº de hojas por nivel (índice = nivel), sin ceros finales.
    pub fn cells_by_level(&self) -> Vec<u64> {
        let mut v = vec![0u64; self.l_max as usize + 1];
        for &l in &self.level {
            v[l as usize] += 1;
        }
        while v.len() > 1 && v.last() == Some(&0) {
            v.pop();
        }
        v
    }

    /// `true` si toda pareja de hojas vecinas difiere como mucho un nivel (invariante 2:1).
    pub fn is_balanced(&self) -> bool {
        self.pairs
            .iter()
            .all(|p| self.level[p.a as usize].abs_diff(self.level[p.b as usize]) <= 1)
    }
}

/// Estado completo del mundo: topología + campos estáticos + derivados + dinámicos.
#[derive(Clone)]
pub struct World {
    pub topo: Topology,
    /// Generador de estáticos (función pura de la dirección de cada celda, incremento 2 §3).
    pub statgen: StaticGen,

    // --- Estáticos puros (función de la dirección de la celda, incr. 2 §3) ---
    pub h0: Vec<f64>,       // elevación de nacimiento (m), del ruido
    pub grad0_e: Vec<f64>,  // ∂h0/∂este (m/m), diferencias centrales de la función pura
    pub grad0_n: Vec<f64>,  // ∂h0/∂norte (m/m)
    pub w_cap0: Vec<f64>,   // capacidad de campo por textura (mm), con el suelo a z_init
    pub k_perm: Vec<f64>,   // permeabilidad (0..1)
    pub fert: Vec<f64>,     // fertilidad efectiva (0..1)
    pub resource: Vec<f64>, // recurso genérico extraíble

    // --- Relieve y suelo vivos (incr. 3): cambian con la erosión, despacio ---
    pub e: Vec<f64>,      // erosión (−) / depósito (+) acumulados (m); h = h0 + e
    pub z_soil: Vec<f64>, // profundidad de suelo (m)
    pub h: Vec<f64>,      // elevación actual (m) = h0 + e
    pub slope: Vec<f64>,  // pendiente S de h (rad)
    pub aspect: Vec<f64>, // orientación de h (rad; reservado, uso futuro)
    pub grad_e: Vec<f64>, // ∂h/∂este (m/m) = ∂h0/∂este + ∂e/∂este
    pub grad_n: Vec<f64>, // ∂h/∂norte (m/m)
    pub w_cap: Vec<f64>,  // capacidad de campo actual (mm) = w_cap0 · z_soil / z_init

    // --- Derivados globales (se recalculan tras cada cambio de topología, incremento 2 §4) ---
    pub filled_h: Vec<f64>,      // elevación tras rellenar depresiones (m)
    pub receiver: Vec<i64>,      // vecino de bajada (posición); -1 si sumidero/océano
    pub drainage_area: Vec<f64>, // área de drenaje acumulada (m²)
    pub flow_order: Vec<u32>,    // posiciones en orden aguas arriba→abajo
    pub dist_ocean_km: Vec<f64>, // distancia a la fuente de humedad más cercana (km)

    // --- Dinámicos (cada uno a su cadencia) ---
    pub surface: Vec<Surface>,
    pub w: Vec<f64>,      // agua del suelo (mm)
    pub temp: Vec<f64>,   // temperatura del tick (°C)
    pub precip: Vec<f64>, // precipitación del tick (mm)
    pub swe: Vec<f64>,    // nieve acumulada, equiv. agua (mm)
    pub veg: Vec<f64>,    // biomasa vegetal (densidad)
    pub biome: Vec<Biome>,
    /// Marca hidrológica pegajosa del LOD: la hoja fue río o lago (incremento 2 §8).
    pub hydro_lock: Vec<bool>,

    // --- Acumuladores para medias (biomas, idoneidad) ---
    pub temp_sum: Vec<f64>,
    pub precip_sum: Vec<f64>,
    pub sample_count: u64,
}

impl World {
    /// Humedad relativa del suelo `θ = W/W_cap` de una celda.
    #[inline]
    pub fn theta(&self, i: usize) -> f64 {
        if self.w_cap[i] > 0.0 {
            (self.w[i] / self.w_cap[i]).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// `true` si la celda es océano.
    #[inline]
    pub fn es_oceano(&self, i: usize) -> bool {
        matches!(self.surface[i], Surface::Oceano)
    }

    /// `true` si la celda es agua (océano, lago o río).
    #[inline]
    pub fn es_agua(&self, i: usize) -> bool {
        !matches!(self.surface[i], Surface::Tierra)
    }
}
