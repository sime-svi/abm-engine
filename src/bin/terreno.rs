//! Binario `terreno` (CLAUDE §5: `anyhow` solo aquí, en el arranque).
//!
//! Uso: `terreno [ruta/config.toml]` (por defecto `config/terreno.toml`). Genera un planeta esférico,
//! corre los sistemas rápidos (clima/agua/vegetación) de forma determinista y vuelca métricas a JSON.

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config/terreno.toml".to_string());

    let cfg = abm::terreno::TerrenoConfig::load(&config_path)
        .with_context(|| format!("al cargar la configuración '{config_path}'"))?;

    let (doc, world) = abm::terreno::run(&cfg).context("al ejecutar la simulación de terreno")?;

    // Volcado de métricas por tick a JSON.
    std::fs::create_dir_all(&cfg.run.output_path).context("al crear la carpeta de salida")?;
    let metrics_path = format!("{}/{}_terreno.json", cfg.run.output_path, cfg.run.name);
    let json = serde_json::to_string_pretty(&doc).context("al serializar las métricas")?;
    std::fs::write(&metrics_path, json).context("al escribir el JSON de métricas")?;
    println!("  Métricas -> {metrics_path}");

    if cfg.run.dump_world {
        let dump = abm::terreno::world_dump(&world);
        let dump_path = format!("{}/{}_world.json", cfg.run.output_path, cfg.run.name);
        let json = serde_json::to_string(&dump).context("al serializar el mundo")?;
        std::fs::write(&dump_path, json).context("al escribir el snapshot del mundo")?;
        println!("  Mundo    -> {dump_path}");
    }

    let last = doc.ticks.last();
    println!(
        "Simulación de terreno '{}' completada: {} celdas, {} ticks.",
        cfg.run.name, doc.run.num_cells, cfg.run.total_ticks
    );
    if let Some(m) = last {
        println!(
            "  Último tick: T media {:.2} °C, θ media {:.3}, ríos {}, lagos {}, residual agua rel {:.2e}",
            m.temp_mean, m.theta_mean, m.river_cells, m.lake_cells, m.water_residual_rel
        );
    }
    Ok(())
}
