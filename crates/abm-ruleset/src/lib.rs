//! `abm-ruleset` — las reglas (CLAUDE §3): componentes del dominio, parámetros, sistemas, génesis
//! del mundo y el orden de los sistemas en el tick (que *es* el modelo). Depende de `abm-core`,
//! nunca al revés. Contiene dos modelos:
//! - el **forrajeo** original (cuadrícula, `config/default.toml`), en la raíz del paquete;
//! - la **biología** de los agentes por días (`biologia`, `config/agentes.toml`).

pub mod biologia;
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
