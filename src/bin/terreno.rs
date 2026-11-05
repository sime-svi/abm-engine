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

    // El reloj de pared solo mide el rendimiento para informar (CLAUDE §6: nunca decide nada).
    let inicio = std::time::Instant::now();
    let (doc, world, info) =
        abm::terreno::run_with_info(&cfg).context("al ejecutar la simulación de terreno")?;
    let segundos = inicio.elapsed().as_secs_f64();

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

    if !cfg.run.genesis_cache.is_empty() {
        let origen = if info.from_cache {
            "leída de la caché"
        } else {
            "calculada y guardada en la caché"
        };
        println!(
            "  Génesis {origen} ({}). Tiempo total: {segundos:.1} s.",
            cfg.run.genesis_cache
        );
    } else {
        println!("  Tiempo total: {segundos:.1} s.");
    }
    let g = &doc.run.genesis;
    if g.erosion.steps > 0 {
        println!(
            "  Génesis geológica: {} pasos ({:.0} años). Erosionado {:.3e} m³, depositado {:.3e} m³, al mar {:.3e} m³ (residual {:.1e}).",
            g.erosion.steps,
            g.years,
            g.erosion.eroded_m3,
            g.erosion.deposited_m3,
            g.erosion.to_ocean_m3,
            g.erosion.residual_m3
        );
        println!(
            "    Lagos {} -> {} celdas; ríos {} -> {}; elevación media de la tierra {:.0} -> {:.0} m; tierra {:.1} % -> {:.1} %.",
            g.before.lake_cells,
            g.after.lake_cells,
            g.before.river_cells,
            g.after.river_cells,
            g.before.elev_mean_land,
            g.after.elev_mean_land,
            100.0 * g.before.land_fraction,
            100.0 * g.after.land_fraction
        );
    }
    let last = doc.ticks.last();
    println!(
        "Simulación de terreno '{}' completada: {} celdas tras la génesis (niveles {}..={}), {} ticks.",
        cfg.run.name, doc.run.num_cells, doc.run.l_min, doc.run.l_max, cfg.run.total_ticks
    );
    if let Some(m) = last {
        println!(
            "  Último tick: {} celdas (por nivel {:?}), T media {:.2} °C, θ media {:.3}, ríos {} ({:.0} km), lagos {}",
            m.num_cells,
            m.cells_by_level,
            m.temp_mean,
            m.theta_mean,
            m.river_cells,
            m.river_length_km,
            m.lake_cells
        );
    }
    let peor = doc
        .ticks
        .iter()
        .map(|t| t.water_residual_rel.abs())
        .fold(0.0_f64, f64::max);
    let (subs, cols): (u64, u64) = doc.ticks.iter().fold((0, 0), |(s, c), t| {
        (s + t.lod_subdivisions, c + t.lod_collapses)
    });
    println!(
        "  LOD durante la corrida: {subs} subdivisiones, {cols} colapsos. Peor residual de agua relativo: {peor:.2e}"
    );
    Ok(())
}
