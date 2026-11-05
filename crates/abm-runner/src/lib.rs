//! `abm-runner` — monta y ejecuta las simulaciones (CLAUDE §3): lee la config, junta motor
//! (`abm-core`) + reglas (`abm-ruleset`) + métricas (`abm-metrics`) + persistencia
//! (`abm-persistence`) y corre el bucle de ticks. Sus binarios son `abm` (el motor de forrajeo),
//! `agentes` (la biología de los agentes por días) y `terreno` (el planeta, `abm-terreno`).

use abm_persistence::Sink;
use abm_persistence::json_sink::JsonSink;
use abm_persistence::pg_sink::RunRecord;
use bevy_ecs::prelude::*;
use thiserror::Error;

pub use abm_core::ConfigError;
pub use abm_persistence::PersistenceError;
pub use abm_persistence::schema::{MetricsDoc, RunMeta, TickMetrics};
pub use abm_persistence::schema_bio::BioDoc;
pub use abm_ruleset::biologia::BioConfig;
pub use abm_ruleset::config::{self, Config};

/// Errores al montar o persistir una simulación.
#[derive(Debug, Error)]
pub enum SimError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Persistence(#[from] PersistenceError),
    #[error("error de serialización JSON")]
    Serialize(#[from] serde_json::Error),
}

/// Ejecuta una simulación completa del modelo de forrajeo y devuelve el documento de métricas por
/// tick.
///
/// Es la **misma** función que usan el binario y los tests, de modo que el camino verificado por los
/// tests es el real. El tiempo es el `tick` (entero); no interviene el reloj de pared (CLAUDE §6).
pub fn run_simulation(cfg: &Config) -> Result<MetricsDoc, SimError> {
    let mut world = abm_ruleset::world_setup::build(cfg);
    world.insert_resource(Sink(Box::new(JsonSink::with_capacity(
        cfg.run.total_ticks as usize,
    ))));
    // El orden del modelo (reglas) y, detrás, la agregación de métricas (observador).
    let mut schedule = abm_core::schedule::deterministic();
    schedule.add_systems(
        (
            abm_ruleset::schedule::model_systems(),
            abm_metrics::collect::collect_metrics, // 7. agrega métricas del tick
        )
            .chain(),
    );

    abm_core::time::run_ticks(&mut world, &mut schedule, cfg.run.total_ticks);

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

/// Ejecuta una corrida de la biología de los agentes (`config/agentes.toml`) y devuelve su
/// documento de métricas.
pub fn run_biologia(cfg: &BioConfig) -> Result<BioDoc, SimError> {
    Ok(run_biologia_con_orden(cfg, None)?.0)
}

/// Igual que `run_biologia`, devolviendo también el mundo final, y con la población inicial creada
/// en un orden barajado si se pide (prueba de independencia del orden de proceso).
pub fn run_biologia_con_orden(
    cfg: &BioConfig,
    barajado: Option<u64>,
) -> Result<(BioDoc, World), SimError> {
    let mut world = abm_ruleset::biologia::genesis::build_con_orden(cfg, barajado)?;
    let registro = abm_metrics::biologia::RegistroBio::nuevo(
        cfg,
        &mut world,
        env!("CARGO_PKG_VERSION"),
        abm_ruleset::VERSION,
    );
    world.insert_resource(registro);
    let mut schedule = abm_core::schedule::deterministic();
    schedule.add_systems(
        (
            abm_ruleset::biologia::schedule::bio_systems(),
            abm_metrics::biologia::collect_bio, // M. métricas del tick
        )
            .chain(),
    );
    abm_core::time::run_ticks(&mut world, &mut schedule, cfg.run.total_ticks);
    let doc = world
        .resource::<abm_metrics::biologia::RegistroBio>()
        .documento();
    Ok((doc, world))
}

/// Datos de la ejecución que se registran en la base de datos junto a sus métricas.
pub fn run_record(cfg: &Config) -> Result<RunRecord, SimError> {
    Ok(RunRecord {
        name: cfg.run.name.clone(),
        seed: cfg.run.seed,
        total_ticks: cfg.run.total_ticks,
        ruleset_version: abm_ruleset::VERSION.to_string(),
        config: serde_json::to_value(cfg)?,
    })
}
