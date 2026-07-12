//! Binario ejecutable `abm` (CLAUDE §5: `anyhow` solo aquí, en el arranque).
//!
//! Uso: `abm [ruta/config.toml]` (por defecto `config/default.toml`).

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config/default.toml".to_string());

    let cfg = abm::Config::load(&config_path)
        .with_context(|| format!("al cargar la configuración '{config_path}'"))?;

    let doc = abm::run_simulation(&cfg).context("al ejecutar la simulación")?;

    let path = abm::persistence::write_json(&cfg.run.output_path, &cfg.run.name, &doc)
        .context("al escribir el JSON de salida")?;

    println!(
        "Simulación '{}' completada: {} ticks -> {}",
        cfg.run.name,
        cfg.run.total_ticks,
        path.display()
    );
    Ok(())
}
