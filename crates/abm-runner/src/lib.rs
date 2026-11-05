//! `abm-runner` — monta y ejecuta las simulaciones (CLAUDE §3): lee la config, junta motor
//! (`abm-core`) + reglas (`abm-ruleset`) + métricas (`abm-metrics`) + persistencia
//! (`abm-persistence`) y corre el bucle de ticks. Sus binarios son `abm` (el motor de forrajeo),
//! `agentes` (la biología de los agentes por días) y `terreno` (el planeta, `abm-terreno`). Los dos
//! últimos aceptan ajustes de la config en la línea de comandos (`--set ruta=valor`) y escriben,
//! aparte de su salida, un informe de tiempos por sistema (`<nombre>_tiempos.json`).

use abm_persistence::Sink;
use abm_persistence::json_sink::JsonSink;
use abm_persistence::pg_sink::RunRecord;
use bevy_ecs::prelude::*;
use thiserror::Error;

pub use abm_core::ConfigError;
pub use abm_core::cronometro::Cronometro;
pub use abm_persistence::PersistenceError;
pub use abm_persistence::schema::{MetricsDoc, RunMeta, TickMetrics};
pub use abm_persistence::schema_bio::BioDoc;
pub use abm_ruleset::biologia::BioConfig;
pub use abm_ruleset::config::{self, Config};

/// Errores al montar o persistir una simulación.
#[derive(Debug, Error)]
pub enum SimError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Persistence(#[from] PersistenceError),
    #[error("error de serialización JSON")]
    Serialize(#[from] serde_json::Error),
}

/// Ejecuta una simulación completa del modelo de forrajeo y devuelve el documento de métricas por
/// tick.
///
/// Es la **misma** función que usan el binario y los tests, de modo que el camino verificado por los
/// tests es el real. El tiempo es el `tick` (entero); no interviene el reloj de pared (CLAUDE §6).
pub fn run_simulation(cfg: &Config) -> Result<MetricsDoc, SimError> {
    let mut world = abm_ruleset::world_setup::build(cfg);
    world.insert_resource(Sink(Box::new(JsonSink::with_capacity(
        cfg.run.total_ticks as usize,
    ))));
    // El orden del modelo (reglas) y, detrás, la agregación de métricas (observador).
    let mut schedule = abm_core::schedule::deterministic();
    schedule.add_systems(
        (
            abm_ruleset::schedule::model_systems(),
            abm_metrics::collect::collect_metrics, // 7. agrega métricas del tick
        )
            .chain(),
    );

    abm_core::time::run_ticks(&mut world, &mut schedule, cfg.run.total_ticks);

    let ticks = world.resource_mut::<Sink>().0.drain();
    Ok(MetricsDoc {
        schema_version: 1,
        run: RunMeta {
            name: cfg.run.name.clone(),
            seed: cfg.run.seed,
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            grid_width: cfg.grid.width,
            grid_height: cfg.grid.height,
            total_ticks: cfg.run.total_ticks,
        },
        ticks,
    })
}

/// Ejecuta una corrida de la biología de los agentes (`config/agentes.toml`) y devuelve su
/// documento de métricas.
pub fn run_biologia(cfg: &BioConfig) -> Result<BioDoc, SimError> {
    Ok(run_biologia_con_orden(cfg, None)?.0)
}

/// Igual que `run_biologia`, devolviendo también el mundo final, y con la población inicial creada
/// en un orden barajado si se pide (prueba de independencia del orden de proceso).
pub fn run_biologia_con_orden(
    cfg: &BioConfig,
    barajado: Option<u64>,
) -> Result<(BioDoc, World), SimError> {
    let (doc, world, _) = run_biologia_completa(cfg, barajado)?;
    Ok((doc, world))
}

/// La corrida de la biología completa: documento de métricas, mundo final y tiempos por sistema
/// (el informe de rendimiento; no forma parte de la salida del modelo).
pub fn run_biologia_completa(
    cfg: &BioConfig,
    barajado: Option<u64>,
) -> Result<(BioDoc, World, Cronometro), SimError> {
    let mut world = abm_ruleset::biologia::genesis::build_con_orden(cfg, barajado)?;
    let registro = abm_metrics::biologia::RegistroBio::nuevo(
        cfg,
        &mut world,
        env!("CARGO_PKG_VERSION"),
        abm_ruleset::VERSION,
    );
    world.insert_resource(registro);
    let mut programa = abm_ruleset::biologia::schedule::bio_programa()
        .etapa("metricas", abm_metrics::biologia::collect_bio); // M. métricas del tick
    programa.correr(&mut world, cfg.run.total_ticks);
    let doc = world
        .resource::<abm_metrics::biologia::RegistroBio>()
        .documento();
    Ok((doc, world, programa.cronometro().clone()))
}

/// Argumentos de los binarios: `[ruta/config.toml] [--set ruta=valor]...`.
#[derive(Debug, Clone)]
pub struct Args {
    /// Ruta de la config (la primera palabra que no es una opción).
    pub config: String,
    /// Ajustes `ruta=valor` que se aplican a la config sin editarla (ver
    /// [`abm_core::config::cargar`]).
    pub ajustes: Vec<String>,
}

/// Lee los argumentos (sin el nombre del programa). Sin config, usa `por_defecto`.
pub fn leer_args(
    args: impl IntoIterator<Item = String>,
    por_defecto: &str,
) -> Result<Args, ConfigError> {
    let mut config = None;
    let mut ajustes = Vec::new();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        if a == "--set" || a == "-s" {
            let ajuste = it
                .next()
                .ok_or_else(|| ConfigError::Invalid(format!("'{a}' sin ruta=valor detrás")))?;
            ajustes.push(ajuste);
        } else if let Some(ajuste) = a.strip_prefix("--set=") {
            ajustes.push(ajuste.to_string());
        } else if a.starts_with('-') {
            return Err(ConfigError::Invalid(format!(
                "opción desconocida '{a}' (uso: [config.toml] [--set ruta=valor]...)"
            )));
        } else if config.is_none() {
            config = Some(a);
        } else {
            return Err(ConfigError::Invalid(format!(
                "sobra el argumento '{a}' (uso: [config.toml] [--set ruta=valor]...)"
            )));
        }
    }
    Ok(Args {
        config: config.unwrap_or_else(|| por_defecto.to_string()),
        ajustes,
    })
}

/// Informe de tiempos por sistema en JSON (`<nombre>_tiempos.json`), aparte de la salida del modelo:
/// lo que mide el reloj cambia de una corrida a otra (CLAUDE §6).
pub fn informe_tiempos(
    nombre: &str,
    binario: &str,
    ticks: u64,
    cron: &Cronometro,
) -> serde_json::Value {
    let total = cron.total().as_secs_f64();
    let etapas: Vec<_> = cron
        .etapas()
        .iter()
        .map(|e| {
            let s = e.total.as_secs_f64();
            serde_json::json!({
                "nombre": e.nombre,
                "segundos": s,
                "fraccion": if total > 0.0 { s / total } else { 0.0 },
                "veces": e.veces,
                "us_por_vez": 1e6 * s / e.veces.max(1) as f64,
            })
        })
        .collect();
    serde_json::json!({
        "nombre": nombre,
        "binario": binario,
        "ticks": ticks,
        "total_s": total,
        "etapas": etapas,
    })
}

/// Resumen de una línea del informe de tiempos: las etapas que más tardan, con su parte del total.
pub fn resumen_tiempos(cron: &Cronometro, cuantas: usize) -> String {
    let total = cron.total().as_secs_f64().max(1e-12);
    let mut etapas: Vec<_> = cron.etapas().iter().collect();
    etapas.sort_by_key(|e| std::cmp::Reverse(e.total));
    etapas
        .iter()
        .take(cuantas)
        .map(|e| {
            format!(
                "{} {:.0} %",
                e.nombre,
                100.0 * e.total.as_secs_f64() / total
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Datos de la ejecución que se registran en la base de datos junto a sus métricas.
pub fn run_record(cfg: &Config) -> Result<RunRecord, SimError> {
    Ok(RunRecord {
        name: cfg.run.name.clone(),
        seed: cfg.run.seed,
        total_ticks: cfg.run.total_ticks,
        ruleset_version: abm_ruleset::VERSION.to_string(),
        config: serde_json::to_value(cfg)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Result<Args, ConfigError> {
        leer_args(v.iter().map(|s| s.to_string()), "config/x.toml")
    }

    #[test]
    fn los_argumentos_dan_la_config_y_los_ajustes() {
        let a = args(&[]).unwrap();
        assert_eq!((a.config.as_str(), a.ajustes.len()), ("config/x.toml", 0));
        let a = args(&[
            "c.toml",
            "--set",
            "run.seed=3",
            "-s",
            "a.b=1",
            "--set=x=\"y\"",
        ])
        .unwrap();
        assert_eq!(a.config, "c.toml");
        assert_eq!(a.ajustes, vec!["run.seed=3", "a.b=1", "x=\"y\""]);
        assert!(args(&["--set"]).is_err());
        assert!(args(&["--otra"]).is_err());
        assert!(args(&["a.toml", "b.toml"]).is_err());
    }
}
