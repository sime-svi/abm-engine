//! Esquema serde de la salida. Structs tipados (orden de campos estable) => JSON byte-comparable.

use serde::{Deserialize, Serialize};

/// Métricas agregadas de un tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TickMetrics {
    pub tick: u64,
    pub population: u64,
    pub births: u64,
    pub deaths: u64,
    pub energy_total: f64,
    pub energy_mean: f64,
    pub age_mean: f64,
    pub resource_total: f64,
}

/// Metadatos de la ejecución (mapea a la futura tabla `simulation_runs`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunMeta {
    pub name: String,
    pub seed: u64,
    pub engine_version: String,
    pub grid_width: u32,
    pub grid_height: u32,
    pub total_ticks: u64,
}

/// Documento completo de salida de una ejecución.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricsDoc {
    pub schema_version: u32,
    pub run: RunMeta,
    pub ticks: Vec<TickMetrics>,
}
