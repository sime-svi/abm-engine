//! Binario `terreno` (CLAUDE §5: `anyhow` solo aquí, en el arranque).
//!
//! Uso: `terreno [ruta/config.toml] [--set ruta=valor]...` (por defecto `config/terreno.toml`).
//! Genera un planeta esférico, corre los sistemas rápidos (clima/agua/vegetación) de forma
//! determinista y vuelca métricas a JSON y, aparte, el informe de tiempos por sistema
//! (`<nombre>_tiempos.json`).
//!
//! Ejemplo: `terreno --set climate.k_oro=60 --set run.name=k60`.

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let args = abm_runner::leer_args(std::env::args().skip(1), "config/terreno.toml")?;
    let cfg = abm_terreno::TerrenoConfig::load_con(&args.config, &args.ajustes)
        .with_context(|| format!("al cargar la configuración '{}'", args.config))?;

    // El reloj de pared solo mide el rendimiento para informar (CLAUDE §6: nunca decide nada).
    let inicio = std::time::Instant::now();
    let (doc, world, info) =
        abm_terreno::run_with_info(&cfg).context("al ejecutar la simulación de terreno")?;
    let segundos = inicio.elapsed().as_secs_f64();

    // Volcado de métricas por tick a JSON.
    std::fs::create_dir_all(&cfg.run.output_path).context("al crear la carpeta de salida")?;
    let metrics_path = format!("{}/{}_terreno.json", cfg.run.output_path, cfg.run.name);
    let json = serde_json::to_string_pretty(&doc).context("al serializar las métricas")?;
    std::fs::write(&metrics_path, json).context("al escribir el JSON de métricas")?;
    println!("  Métricas -> {metrics_path}");
    let path_t = format!("{}/{}_tiempos.json", cfg.run.output_path, cfg.run.name);
    let informe =
        abm_runner::informe_tiempos(&cfg.run.name, "terreno", cfg.run.total_ticks, &info.tiempos);
    std::fs::write(&path_t, serde_json::to_string_pretty(&informe)?)
        .context("al escribir el informe de tiempos")?;
    println!(
        "  Tiempos por sistema: {} -> {path_t}",
        abm_runner::resumen_tiempos(&info.tiempos, 4)
    );

    if cfg.run.dump_world {
        let dump = abm_terreno::world_dump(&world, &cfg);
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
    if let Some(t) = &doc.run.tectonica {
        let c = &t.cuentas;
        println!(
            "  Génesis tectónica: {} pasos en la malla de nivel {} ({:.0} km). Placas al final {}; corteza continental {:.3} -> {:.3}; nivel del mar {:.0} m respecto al dato.",
            t.pasos,
            t.nivel_malla,
            t.lado_km,
            t.placas_final.len(),
            t.f_cont_inicial,
            t.f_cont_final,
            t.nivel_mar_m
        );
        println!(
            "    Velocidad media {:.2} cm/año; edad del fondo media/mediana/máxima {:.0}/{:.0}/{:.0} Myr; eventos {}; volumen continental {:.3e} -> {:.3e} km³ (remuestreo {:.1e}).",
            t.v_rms_media_cm_anio,
            t.edades_oceano.media_myr,
            t.edades_oceano.mediana_myr,
            t.edades_oceano.max_myr,
            t.eventos.len(),
            c.volumen_inicial_km3,
            c.volumen_final_km3,
            c.remuestreo_km3
        );
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
    let l = &doc.run.lluvia;
    println!(
        "  Sombra pluviométrica: la orografía y el agotamiento de humedad cambian la lluvia sobre tierra un {:+.1} %; c_P = {:.4}.",
        100.0 * l.desvio,
        l.c_p
    );
    let d = &doc.diagnostico;
    println!(
        "  Diagnóstico: tierra {:.1} % del planeta, altura media {:.0} m (mediana {:.0} m), océano de {:.0} m; lluvia {:.0} mm/año en tierra y {:.0} en el mar; w < 0,5 en el {:.1} % de la tierra; PPN (Miami) {:.0} g/m²/año.",
        100.0 * d.tierra_frac,
        d.altura_media_tierra_m,
        d.altura_mediana_tierra_m,
        d.profundidad_media_oceano_m,
        d.lluvia_tierra_mm,
        d.lluvia_oceano_mm,
        100.0 * d.tierra_w_bajo_frac,
        d.ppn_miami_tierra
    );
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
