//! Binario `agentes` (CLAUDE §5: `anyhow` solo aquí, en el arranque).
//!
//! Uso: `agentes [ruta/config.toml] [--set ruta=valor]...` (por defecto `config/agentes.toml`).
//! Simula la biología de los agentes por días sobre el hábitat de prueba y vuelca las métricas a JSON
//! y, aparte, el informe de tiempos por sistema (`<nombre>_tiempos.json`).
//!
//! Ejemplo: `agentes --set run.total_ticks=36500 --set habitat.sequia.factor=0.3`.

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let args = abm_runner::leer_args(std::env::args().skip(1), "config/agentes.toml")?;
    let cfg = abm_runner::BioConfig::load_con(&args.config, &args.ajustes)
        .with_context(|| format!("al cargar la configuración '{}'", args.config))?;

    // El reloj de pared solo mide el rendimiento para informar (CLAUDE §6: nunca decide nada).
    let inicio = std::time::Instant::now();
    let (doc, _, tiempos) =
        abm_runner::run_biologia_completa(&cfg, None).context("al ejecutar la biología")?;
    let segundos = inicio.elapsed().as_secs_f64();

    std::fs::create_dir_all(&cfg.run.output_path).context("al crear la carpeta de salida")?;
    let path = format!("{}/{}_agentes.json", cfg.run.output_path, cfg.run.name);
    let json = serde_json::to_string_pretty(&doc).context("al serializar las métricas")?;
    std::fs::write(&path, json).context("al escribir el JSON de métricas")?;
    let path_t = format!("{}/{}_tiempos.json", cfg.run.output_path, cfg.run.name);
    let informe =
        abm_runner::informe_tiempos(&cfg.run.name, "agentes", cfg.run.total_ticks, &tiempos);
    std::fs::write(&path_t, serde_json::to_string_pretty(&informe)?)
        .context("al escribir el informe de tiempos")?;

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
    println!(
        "  Tiempos por sistema: {} -> {path_t}",
        abm_runner::resumen_tiempos(&tiempos, 4)
    );
    println!("  Métricas -> {path}");
    Ok(())
}
