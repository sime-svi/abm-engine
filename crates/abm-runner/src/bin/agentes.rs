//! Binario `agentes` (CLAUDE §5: `anyhow` solo aquí, en el arranque).
//!
//! Uso: `agentes [ruta/config.toml]` (por defecto `config/agentes.toml`). Simula la biología de los
//! agentes por días sobre el hábitat de prueba y vuelca las métricas a JSON.

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config/agentes.toml".to_string());
    let cfg = abm_runner::BioConfig::load(&config_path)
        .with_context(|| format!("al cargar la configuración '{config_path}'"))?;

    // El reloj de pared solo mide el rendimiento para informar (CLAUDE §6: nunca decide nada).
    let inicio = std::time::Instant::now();
    let doc = abm_runner::run_biologia(&cfg).context("al ejecutar la biología")?;
    let segundos = inicio.elapsed().as_secs_f64();

    std::fs::create_dir_all(&cfg.run.output_path).context("al crear la carpeta de salida")?;
    let path = format!("{}/{}_agentes.json", cfg.run.output_path, cfg.run.name);
    let json = serde_json::to_string_pretty(&doc).context("al serializar las métricas")?;
    std::fs::write(&path, json).context("al escribir el JSON de métricas")?;

    let anios = cfg.run.total_ticks as f64 / cfg.tiempo.dias_anio as f64;
    println!(
        "Biología: {} ticks ({anios:.1} años), {} celdas, {:.0} km², {segundos:.1} s",
        cfg.run.total_ticks, doc.run.celdas, doc.run.area_total_km2
    );
    let residuo = doc
        .ticks
        .iter()
        .map(|t| t.balance.residuo.abs())
        .fold(0.0, f64::max);
    for (k, e) in doc.run.especies.iter().enumerate() {
        let fin = doc.ticks.last().map_or(0, |t| t.especies[k].n);
        let ultimos: Vec<_> = doc
            .anios
            .iter()
            .rev()
            .take(5)
            .map(|a| &a.especies[k])
            .collect();
        let dens =
            ultimos.iter().map(|a| a.densidad_km2).sum::<f64>() / ultimos.len().max(1) as f64;
        let e0 = ultimos.first().and_then(|a| a.e0_anios);
        println!(
            "  {:<16} {:>6.1} kg: {:>7} → {:>7} ind. | densidad (5 últimos años) {:.3}/km², N_max {:.3}/km² | e0 último año {}",
            e.nombre,
            e.masa_kg,
            e.poblacion_inicial,
            fin,
            dens,
            e.n_max / doc.run.area_total_km2,
            e0.map_or("—".to_string(), |x| format!("{x:.1} años")),
        );
    }
    println!("  Balance de energía: residuo máximo {residuo:.2e} kcal por registro");
    println!("  Métricas -> {path}");
    Ok(())
}
