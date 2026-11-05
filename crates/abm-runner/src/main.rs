//! Binario ejecutable `abm` (CLAUDE §5: `anyhow` solo aquí, en el arranque).
//!
//! Uso: `abm [ruta/config.toml]` (por defecto `config/default.toml`).

use abm_runner::config::SinkKind;
use anyhow::{Context, Result};

fn main() -> Result<()> {
    // Carga infra/.env (si existe) para la conexión a la BD, sin tener que exportarla a mano.
    // No sobreescribe variables ya definidas en el entorno; se ignora si no hay archivo.
    let _ = dotenvy::from_path("infra/.env");

    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config/default.toml".to_string());

    let cfg = abm_runner::Config::load(&config_path)
        .with_context(|| format!("al cargar la configuración '{config_path}'"))?;

    let doc = abm_runner::run_simulation(&cfg).context("al ejecutar la simulación")?;

    let sink = cfg.persistence.sink;
    if matches!(sink, SinkKind::Json | SinkKind::Both) {
        let path = abm_persistence::write_json(&cfg.run.output_path, &cfg.run.name, &doc)
            .context("al escribir el JSON de salida")?;
        println!("  JSON       -> {}", path.display());
    }
    if matches!(sink, SinkKind::Postgres | SinkKind::Both) {
        let record =
            abm_runner::run_record(&cfg).context("al preparar el registro de la corrida")?;
        let run_id = abm_persistence::pg_sink::write_run(&record, &doc)
            .context("al escribir en PostgreSQL")?;
        println!("  PostgreSQL -> run_id {run_id}");
    }

    println!(
        "Simulación '{}' completada: {} ticks.",
        cfg.run.name, cfg.run.total_ticks
    );
    Ok(())
}
