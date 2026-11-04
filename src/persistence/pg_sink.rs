//! Sink a PostgreSQL/TimescaleDB (CLAUDE §7). Escribe la ejecución en `simulation_runs` y sus
//! métricas por tick en `metrics`, todo en una transacción. La conexión se toma del **entorno**
//! (nunca del código ni de la config; CLAUDE §9): `DATABASE_URL`, o si no las variables `POSTGRES_*`.

use std::io::Write;

use postgres::{Client, NoTls};
use uuid::Uuid;

use crate::config::Config;
use crate::error::SimError;
use crate::metrics::schema::{MetricsDoc, TickMetrics};
use crate::persistence::migrations;

/// Versión del ruleset. Hoy coincide con la del crate (binario único); divergirá al separar crates.
const RULESET_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Construye la cadena de conexión desde el entorno: `DATABASE_URL` o, si no, las `POSTGRES_*`.
fn connection_string() -> Result<String, SimError> {
    if let Ok(url) = std::env::var("DATABASE_URL") {
        return Ok(url);
    }
    let user = std::env::var("POSTGRES_USER").map_err(|_| {
        SimError::MissingEnv("DATABASE_URL (o POSTGRES_USER/POSTGRES_PASSWORD/POSTGRES_DB)".into())
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
pub fn write_run(cfg: &Config, doc: &MetricsDoc) -> Result<Uuid, SimError> {
    let mut client = Client::connect(&connection_string()?, NoTls)?;
    migrations::run(&mut client)?;

    let run_id = Uuid::new_v4();
    let config_json = serde_json::to_value(cfg)?;

    let mut tx = client.transaction()?;
    tx.execute(
        "INSERT INTO simulation_runs
            (run_id, name, seed, engine_version, ruleset_version, config,
             status, finished_at, total_ticks)
         VALUES ($1, $2, $3, $4, $5, $6, 'finished', now(), $7)",
        &[
            &run_id,
            &cfg.run.name,
            &(cfg.run.seed as i64),
            &doc.run.engine_version,
            &RULESET_VERSION,
            &config_json,
            &(cfg.run.total_ticks as i64),
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
