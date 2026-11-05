//! `abm-ruleset` — las reglas del modelo de forrajeo (CLAUDE §3): componentes del dominio,
//! cuadrícula, parámetros, sistemas (un archivo por comportamiento), génesis del mundo y el orden de
//! los sistemas en el tick (que *es* el modelo). Depende de `abm-core`, nunca al revés.

pub mod buffers;
pub mod components;
pub mod config;
pub mod grid;
pub mod params;
pub mod schedule;
pub mod systems;
pub mod world_setup;

pub use config::Config;

/// Versión del ruleset (se registra en cada ejecución: CLAUDE §6).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
