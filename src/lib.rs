//! Motor de simulación masiva basada en agentes (ABM), headless y determinista.
//!
//! Binario mínimo (metodología "pirámide invertida" de CLAUDE.md §4). Las carpetas de `src/`
//! anticipan el futuro workspace de crates: `core` (motor), `domain` (reglas), `metrics`
//! (observación) y `persistence` (salida). La regla de dependencias apunta a `core`.

pub mod config;
pub mod error;
pub mod persistence;

mod core;
mod domain;
mod metrics;

pub use config::Config;
pub use error::{ConfigError, SimError};
pub use metrics::schema::{MetricsDoc, RunMeta, TickMetrics};

use bevy_ecs::prelude::*;

use crate::core::time::Tick;
use crate::persistence::Sink;

/// Ejecuta una simulación completa y devuelve el documento de métricas por tick.
///
/// Es la **misma** función que usan el binario y los tests, de modo que el camino verificado por los
/// tests es el real. El tiempo es el `tick` (entero); no interviene el reloj de pared (CLAUDE §6).
pub fn run_simulation(cfg: &Config) -> Result<MetricsDoc, SimError> {
    let mut world = core::world_setup::build(cfg);
    let mut schedule = core::schedule::build();

    for t in 0..cfg.run.total_ticks {
        world.resource_mut::<Tick>().0 = t;
        schedule.run(&mut world);
    }

    let ticks = world.resource_mut::<Sink>().0.drain();
    Ok(MetricsDoc {
        schema_version: 1,
        run: RunMeta {
            name: cfg.run.name.clone(),
            seed: cfg.run.seed,
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            grid_width: cfg.grid.width,
            grid_height: cfg.grid.height,
            total_ticks: cfg.run.total_ticks,
        },
        ticks,
    })
}
