//! `abm-persistence` — la salida (CLAUDE §3 y §7), aislada tras el trait `MetricsSink`: el esquema de
//! las métricas, el volcado a JSON y el sink a PostgreSQL/TimescaleDB, con sus migraciones. Cambiar
//! el destino no toca el motor ni las reglas.

pub mod error;
pub mod json_sink;
pub mod migrations;
pub mod pg_sink;
pub mod schema;
pub mod schema_bio;
pub mod sink;

use std::path::{Path, PathBuf};

use bevy_ecs::prelude::*;

pub use error::PersistenceError;
use schema::MetricsDoc;
use sink::MetricsSink;

/// Recurso que envuelve el destino de las métricas (intercambiable por config).
#[derive(Resource)]
pub struct Sink(pub Box<dyn MetricsSink>);

/// Escribe el documento de métricas como JSON en `<dir>/<name>.json` y devuelve la ruta escrita.
pub fn write_json(dir: &str, name: &str, doc: &MetricsDoc) -> Result<PathBuf, PersistenceError> {
    std::fs::create_dir_all(dir)?;
    let path = Path::new(dir).join(format!("{name}.json"));
    let json = serde_json::to_string_pretty(doc)?;
    std::fs::write(&path, json)?;
    Ok(path)
}
