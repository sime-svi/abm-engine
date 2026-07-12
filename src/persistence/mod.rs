//! La salida, aislada tras el trait `MetricsSink` (CLAUDE §7). Hoy JSON; mañana PostgreSQL sin tocar
//! el motor. Futuro crate `abm-persistence`.

pub mod json_sink;
pub mod sink;

use std::path::{Path, PathBuf};

use bevy_ecs::prelude::*;

use crate::error::SimError;
use crate::metrics::schema::MetricsDoc;
use sink::MetricsSink;

/// Recurso que envuelve el destino de las métricas (intercambiable por config en el futuro).
#[derive(Resource)]
pub struct Sink(pub Box<dyn MetricsSink>);

/// Escribe el documento de métricas como JSON en `<dir>/<name>.json` y devuelve la ruta escrita.
pub fn write_json(dir: &str, name: &str, doc: &MetricsDoc) -> Result<PathBuf, SimError> {
    std::fs::create_dir_all(dir)?;
    let path = Path::new(dir).join(format!("{name}.json"));
    let json = serde_json::to_string_pretty(doc)?;
    std::fs::write(&path, json)?;
    Ok(path)
}
