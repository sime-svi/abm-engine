//! Parámetros del modelo copiados de la config a tipos planos, para que los sistemas no lean el TOML.

use bevy_ecs::prelude::*;

/// Parámetros del modelo (recurso global e inmutable durante la ejecución).
#[derive(Resource, Debug, Clone)]
pub struct Params {
    pub appetite: f32,
    pub metabolism: f32,
    pub max_age: u32,
    pub repro_enabled: bool,
    pub repro_threshold: f32,
    pub child_fraction: f32,
    pub single_occupancy: bool,
    pub metrics_every: u64,
}
