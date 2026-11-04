//! Fase «terreno» — sustrato físico esférico (incrementos 1 y 2).
//!
//! Motor independiente del de forrajeo: genera un planeta (cubo esférico) y corre los sistemas
//! rápidos (clima, nieve, agua, vegetación) de forma determinista, sobre una malla que puede
//! **afinarse sola** donde hay detalle (LOD dinámico, incremento 2). Diseño:
//! `docs/design/terreno/incremento-1-diseno.md` e `incremento-2-diseno.md`. Entrada:
//! `(semilla, config)`. Salida: métricas por tick + (opcional) snapshot por celda del mundo.

pub mod config;
pub mod geometry;
pub mod hydro;
pub mod lod;
pub mod metrics;
pub mod noise;
pub mod statics;
pub mod systems;
pub mod world;
pub mod worldgen;

pub use config::TerrenoConfig;
pub use metrics::{TerrenoDoc, TerrenoRunMeta, TerrenoTickMetrics};
pub use world::World;

use crate::error::SimError;

/// Ejecuta una simulación de terreno completa. Devuelve el documento de métricas y el mundo final.
///
/// Es la **misma** función que usan el binario y los tests (el camino verificado es el real). El
/// tiempo es el `tick` entero; no interviene el reloj de pared (CLAUDE §6). Orden dentro del tick
/// (incremento 2 §9): si `t > 0` y `t` es múltiplo de `n_lod`, paso de topología; después los
/// sistemas del incremento 1.
pub fn run(cfg: &TerrenoConfig) -> Result<(TerrenoDoc, World), SimError> {
    cfg.validate()?;
    let mut world = worldgen::generate(cfg);
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
        let acct = systems::step(&mut world, cfg, &mut scratch, t, store_before, &lod_report);
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
        },
        ticks,
    };
    Ok((doc, world))
}

/// Snapshot por celda del mundo generado (para inspección/depuración).
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorldDumpCell {
    pub level: u8,
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub area_km2: f64,
    pub h: f64,
    pub surface: world::Surface,
    pub biome: world::Biome,
    pub theta: f64,
    pub veg: f64,
    pub temp: f64,
    pub precip: f64,
}

/// Serializa el snapshot por celda del mundo (orden de clave).
pub fn world_dump(world: &World) -> Vec<WorldDumpCell> {
    (0..world.topo.num_cells)
        .map(|id| WorldDumpCell {
            level: world.topo.level[id],
            lat_deg: world.topo.lat[id].to_degrees(),
            lon_deg: world.topo.lon[id].to_degrees(),
            area_km2: world.topo.area[id] / 1e6,
            h: world.h[id],
            surface: world.surface[id],
            biome: world.biome[id],
            theta: world.theta(id),
            veg: world.veg[id],
            temp: world.temp[id],
            precip: world.precip[id],
        })
        .collect()
}
