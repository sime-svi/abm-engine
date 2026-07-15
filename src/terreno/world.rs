//! Estructuras de datos del terreno: topología (LOD-ready, poblada a nivel uniforme) y campos SoA.
//!
//! Diseño §3. Las celdas NO son entidades ECS sino columnas `Vec` indexadas por id lineal, igual que
//! el recurso `Grid` del motor de forrajeo: contiguo en memoria (caché), orden de iteración estable
//! (determinismo) y sin asignaciones por tick. La API es invariante a la escala, de modo que activar
//! el LOD adaptativo (incremento 2) no tocará la física.

use serde::{Deserialize, Serialize};

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
}

/// Topología del cubo esférico: geometría estática por celda y tabla de vecinos precalculada.
pub struct Topology {
    /// Celdas por arista de cara (`n0 · 2^l_unif`).
    pub n: u32,
    /// Número total de celdas: `6·n²`.
    pub num_cells: usize,
    /// Radio del planeta (m).
    pub r_planet: f64,
    /// Latitud de cada celda (radianes).
    pub lat: Vec<f64>,
    /// Longitud de cada celda (radianes).
    pub lon: Vec<f64>,
    /// Área real de cada celda (m²).
    pub area: Vec<f64>,
    /// Base local este/norte (unitarios 3D) de cada celda, para gradientes y viento.
    pub east: Vec<[f64; 3]>,
    pub north: Vec<[f64; 3]>,
    /// Dirección unitaria del centro de cada celda (3D).
    pub dir: Vec<[f64; 3]>,
    /// Los 8 vecinos (ids) de cada celda en orden de Moore; `-1` = sin vecino (esquina).
    pub neighbors: Vec<[i64; 8]>,
}

/// Estado completo del mundo: topología + campos estáticos + campos dinámicos.
pub struct World {
    pub topo: Topology,

    // --- Estáticos (fijados en world-gen; el relieve no cambia en el incremento 1) ---
    pub h: Vec<f64>,             // elevación (m)
    pub slope: Vec<f64>,         // pendiente S (rad)
    pub aspect: Vec<f64>,        // orientación (rad; reservado, uso futuro)
    pub grad_e: Vec<f64>,        // ∂h/∂este (m/m)
    pub grad_n: Vec<f64>,        // ∂h/∂norte (m/m)
    pub w_cap: Vec<f64>,         // capacidad de campo (mm)
    pub k_perm: Vec<f64>,        // permeabilidad (0..1)
    pub fert: Vec<f64>,          // fertilidad efectiva (0..1)
    pub filled_h: Vec<f64>,      // elevación tras rellenar depresiones (m)
    pub receiver: Vec<i64>,      // vecino de bajada (id); -1 si sumidero/oceano
    pub drainage_area: Vec<f64>, // área de drenaje acumulada (m²)
    pub flow_order: Vec<u32>,    // ids en orden aguas arriba→abajo (para acumular caudal por tick)
    pub dist_ocean_km: Vec<f64>, // distancia a la fuente de humedad más cercana (km)
    pub resource: Vec<f64>,      // recurso genérico extraíble

    // --- Dinámicos (cada uno a su cadencia) ---
    pub surface: Vec<Surface>,
    pub w: Vec<f64>,      // agua del suelo (mm)
    pub temp: Vec<f64>,   // temperatura del tick (°C)
    pub precip: Vec<f64>, // precipitación del tick (mm)
    pub swe: Vec<f64>,    // nieve acumulada, equiv. agua (mm)
    pub veg: Vec<f64>,    // biomasa vegetal
    pub biome: Vec<Biome>,

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

    /// `true` si la celda es agua (oceano, lago o río).
    #[inline]
    pub fn es_agua(&self, i: usize) -> bool {
        !matches!(self.surface[i], Surface::Tierra)
    }
}
