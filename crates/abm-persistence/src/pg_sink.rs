//! Sink a PostgreSQL/TimescaleDB (CLAUDE §7). Escribe la ejecución en `simulation_runs` y sus
//! métricas por tick en `metrics`, todo en una transacción. La conexión se toma del **entorno**
//! (nunca del código ni de la config; CLAUDE §9): `DATABASE_URL`, o si no las variables `POSTGRES_*`.

use std::io::Write;

use postgres::{Client, NoTls};
use uuid::Uuid;

use crate::error::PersistenceError;
use crate::migrations;
use crate::schema::{MetricsDoc, TickMetrics};

/// Lo que se registra de una ejecución en `simulation_runs`, aparte de sus métricas. Lo rellena quien
/// monta la simulación: así la persistencia no depende de la config de ningún modelo.
#[derive(Debug, Clone)]
pub struct RunRecord {
    pub name: String,
    pub seed: u64,
    pub total_ticks: u64,
    /// Versión del ruleset que generó los datos (CLAUDE §6).
    pub ruleset_version: String,
    /// La config completa de la ejecución, como JSON.
    pub config: serde_json::Value,
}

/// Construye la cadena de conexión desde el entorno: `DATABASE_URL` o, si no, las `POSTGRES_*`.
fn connection_string() -> Result<String, PersistenceError> {
    if let Ok(url) = std::env::var("DATABASE_URL") {
        return Ok(url);
    }
    let user = std::env::var("POSTGRES_USER").map_err(|_| {
        PersistenceError::MissingEnv(
            "DATABASE_URL (o POSTGRES_USER/POSTGRES_PASSWORD/POSTGRES_DB)".into(),
        )
    })?;
    let password = std::env::var("POSTGRES_PASSWORD").unwrap_or_default();
    let dbname = std::env::var("POSTGRES_DB").unwrap_or_else(|_| user.clone());
    let host = std::env::var("PGHOST").unwrap_or_else(|_| "localhost".into());
    // 25432 = puerto por defecto con el que `infra/docker-compose.yml` publica la BD del proyecto
    // (no el 5432 estándar: en esta máquina lo ocupa otro Postgres ajeno al proyecto).
    let port = std::env::var("PGPORT").unwrap_or_else(|_| "25432".into());
    Ok(format!(
        "host={host} port={port} user={user} password={password} dbname={dbname}"
    ))
}

/// Las 7 métricas de un tick (formato largo), en orden fijo.
fn tick_rows(m: &TickMetrics) -> [(&'static str, f64); 7] {
    [
        ("population", m.population as f64),
        ("births", m.births as f64),
        ("deaths", m.deaths as f64),
        ("energy_total", m.energy_total),
        ("energy_mean", m.energy_mean),
        ("age_mean", m.age_mean),
        ("resource_total", m.resource_total),
    ]
}

/// Escribe una ejecución completa (registro + métricas) y devuelve su `run_id`.
///
/// El `run_id` es un UUID por ejecución (identificador de la corrida, no parte del estado
/// reproducible de la simulación). Los `started_at/finished_at` los pone la BD con `now()`.
pub fn write_run(run: &RunRecord, doc: &MetricsDoc) -> Result<Uuid, PersistenceError> {
    let mut client = Client::connect(&connection_string()?, NoTls)?;
    migrations::run(&mut client)?;

    let run_id = Uuid::new_v4();

    let mut tx = client.transaction()?;
    tx.execute(
        "INSERT INTO simulation_runs
            (run_id, name, seed, engine_version, ruleset_version, config,
             status, finished_at, total_ticks)
         VALUES ($1, $2, $3, $4, $5, $6, 'finished', now(), $7)",
        &[
            &run_id,
            &run.name,
            &(run.seed as i64),
            &doc.run.engine_version,
            &run.ruleset_version,
            &run.config,
            &(run.total_ticks as i64),
        ],
    )?;

    // Carga masiva de las métricas con COPY (una sola operación en streaming).
    let mut writer = tx.copy_in("COPY metrics (run_id, tick, metric_name, value) FROM STDIN")?;
    let rid = run_id.to_string();
    let mut line = String::new();
    for m in &doc.ticks {
        for (name, value) in tick_rows(m) {
            line.clear();
            line.push_str(&rid);
            line.push('\t');
            line.push_str(&m.tick.to_string());
            line.push('\t');
            line.push_str(name);
            line.push('\t');
            line.push_str(&value.to_string());
            line.push('\n');
            writer.write_all(line.as_bytes())?;
        }
    }
    writer.finish()?;
    tx.commit()?;

    Ok(run_id)
}
