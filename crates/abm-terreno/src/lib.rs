//! `abm-terreno` — fase «terreno»: el sustrato físico esférico (incrementos 1, 2 y 3).
//!
//! Motor independiente del de forrajeo: genera un planeta (cubo esférico), lo "envejece" con una
//! génesis geológica (erosión y suelo, incremento 3) y corre los sistemas rápidos (clima, nieve,
//! agua, vegetación) de forma determinista, sobre una malla que puede **afinarse sola** donde hay
//! detalle (LOD dinámico, incremento 2), con el relieve aún vivo a ritmo real. Diseño:
//! `docs/design/terreno/incremento-{1,2,3}-diseno.md`. Entrada: `(semilla, config)`. Salida:
//! métricas por tick + (opcional) snapshot por celda del mundo.

pub mod cache;
pub mod config;
pub mod erosion;
pub mod error;
pub mod geometry;
pub mod humedad;
pub mod hydro;
pub mod lod;
pub mod metrics;
pub mod noise;
pub mod statics;
pub mod systems;
pub mod world;
pub mod worldgen;

pub use config::TerrenoConfig;
pub use error::TerrenoError;
pub use metrics::{TerrenoDoc, TerrenoRunMeta, TerrenoTickMetrics};
pub use world::World;

/// Ejecuta una simulación de terreno completa. Devuelve el documento de métricas y el mundo final.
///
/// Es la **misma** función que usan el binario y los tests (el camino verificado es el real). El
/// tiempo es el `tick` entero; no interviene el reloj de pared (CLAUDE §6). Orden dentro del tick
/// (incremento 2 §9): si `t > 0` y `t` es múltiplo de `n_lod`, paso de topología; después los
/// sistemas del incremento 1.
pub fn run(cfg: &TerrenoConfig) -> Result<(TerrenoDoc, World), TerrenoError> {
    run_with_info(cfg).map(|(doc, world, _)| (doc, world))
}

/// Cómo se obtuvo la génesis. No forma parte de la salida: la salida es idéntica se use o no la
/// caché.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenesisInfo {
    /// La génesis se leyó de la caché de disco (`run.genesis_cache`).
    pub from_cache: bool,
}

/// Como [`run`], e informa además de si la génesis salió de la caché.
pub fn run_with_info(
    cfg: &TerrenoConfig,
) -> Result<(TerrenoDoc, World, GenesisInfo), TerrenoError> {
    cfg.validate()?;
    let ((mut world, genesis), from_cache) = match cache::load(cfg)? {
        Some(cached) => (cached, true),
        None => {
            let generated = worldgen::generate_with_report(cfg);
            cache::save(cfg, &generated.0, &generated.1)?;
            (generated, false)
        }
    };
    let mut scratch = systems::Scratch::new(world.topo.num_cells);
    let mut ticks = Vec::with_capacity(cfg.run.total_ticks as usize);
    let initial_cells = world.topo.num_cells as u64;

    for t in 0..cfg.run.total_ticks {
        // El almacenamiento previo se mide ANTES del paso de topología: el residual del tick cubre
        // también lo que el reparto del LOD envía al mar.
        let store_before = systems::total_store_vol(&world);
        let lod_report = if cfg.lod.enabled() && t > 0 && t.is_multiple_of(cfg.lod.n_lod) {
            let r = lod::step(&mut world, cfg);
            if r.changed() {
                scratch.resize(world.topo.num_cells);
            }
            r
        } else {
            lod::LodReport::default()
        };
        let mut acct = systems::step(&mut world, cfg, &mut scratch, t, store_before, &lod_report);
        // Erosión en ejecución (incr. 3 §9), al final del tick: no mueve agua (el exceso de agua
        // sobre una capacidad menor se drena en el tick siguiente).
        let every = cfg.erosion.run_every;
        if every > 0 && t > 0 && t.is_multiple_of(every) {
            let dt_years = every as f64 / cfg.climate.t_year * cfg.erosion.run_accel;
            let (p, tm) = erosion::running_means(&world, cfg);
            acct.erosion = erosion::step(&mut world, cfg, dt_years, &p, &tm);
            lod::update_hydro_lock(&mut world);
        }
        ticks.push(metrics::compute(&world, &acct, t));
    }

    let doc = TerrenoDoc {
        schema_version: metrics::SCHEMA_VERSION,
        run: TerrenoRunMeta {
            name: cfg.run.name.clone(),
            seed: cfg.run.seed,
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            n0: cfg.planet.n0,
            l_min: cfg.lod.l_min,
            l_max: cfg.lod.l_max,
            num_cells: initial_cells,
            r_km: cfg.planet.r_km,
            total_ticks: cfg.run.total_ticks,
            genesis,
            lluvia: metrics::LluviaMeta {
                desvio: world.desvio_lluvia,
                c_p: world.c_p,
            },
        },
        ticks,
    };
    Ok((doc, world, GenesisInfo { from_cache }))
}

/// Snapshot por celda del mundo generado (para inspección/depuración).
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorldDumpCell {
    pub level: u8,
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub area_km2: f64,
    pub h: f64,
    pub erosion_m: f64,
    pub soil_m: f64,
    pub surface: world::Surface,
    pub biome: world::Biome,
    pub theta: f64,
    pub veg: f64,
    pub temp: f64,
    pub precip: f64,
    /// Con `run.dump_climate`: factor orográfico, agotamiento de humedad `w` y lluvia media anual
    /// (mm/año) de la celda (nota de sombra pluviométrica).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub m_oro: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub w_hum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lluvia_anual_mm: Option<f64>,
}

/// Serializa el snapshot por celda del mundo (orden de clave).
pub fn world_dump(world: &World, cfg: &TerrenoConfig) -> Vec<WorldDumpCell> {
    let clima = cfg.run.dump_climate;
    (0..world.topo.num_cells)
        .map(|id| WorldDumpCell {
            level: world.topo.level[id],
            lat_deg: world.topo.lat[id].to_degrees(),
            lon_deg: world.topo.lon[id].to_degrees(),
            area_km2: world.topo.area[id] / 1e6,
            h: world.h[id],
            erosion_m: world.e[id],
            soil_m: world.z_soil[id],
            surface: world.surface[id],
            biome: world.biome[id],
            theta: world.theta(id),
            veg: world.veg[id],
            temp: world.temp[id],
            precip: world.precip[id],
            m_oro: clima.then(|| systems::m_oro_celda(world, cfg, id)),
            w_hum: clima.then(|| world.w_hum[id]),
            lluvia_anual_mm: clima.then(|| systems::lluvia_media_anual_mm(world, cfg, id)),
        })
        .collect()
}
