//! Fase «terreno» — sustrato físico esférico (incremento 1).
//!
//! Motor independiente del de forrajeo: genera un planeta (cubo esférico) y corre los sistemas
//! rápidos (clima, nieve, agua, vegetación) de forma determinista. Diseño:
//! `docs/design/terreno/incremento-1-diseno.md`. Entrada: `(semilla, config)`. Salida: métricas
//! por tick + (opcional) snapshot por celda del mundo.

pub mod config;
pub mod geometry;
pub mod metrics;
pub mod noise;
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
/// tiempo es el `tick` entero; no interviene el reloj de pared (CLAUDE §6).
pub fn run(cfg: &TerrenoConfig) -> Result<(TerrenoDoc, World), SimError> {
    let mut world = worldgen::generate(cfg);
    let mut scratch = systems::Scratch::new(world.topo.num_cells);
    let mut ticks = Vec::with_capacity(cfg.run.total_ticks as usize);

    for t in 0..cfg.run.total_ticks {
        let acct = systems::step(&mut world, cfg, &mut scratch, t);
        ticks.push(metrics::compute(&world, cfg, &acct, t));
    }

    let doc = TerrenoDoc {
        schema_version: 1,
        run: TerrenoRunMeta {
            name: cfg.run.name.clone(),
            seed: cfg.run.seed,
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            face_n: world.topo.n,
            num_cells: world.topo.num_cells as u64,
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
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub h: f64,
    pub surface: world::Surface,
    pub biome: world::Biome,
    pub theta: f64,
    pub veg: f64,
    pub temp: f64,
    pub precip: f64,
}

/// Serializa el snapshot por celda del mundo (orden de id).
pub fn world_dump(world: &World) -> Vec<WorldDumpCell> {
    (0..world.topo.num_cells)
        .map(|id| WorldDumpCell {
            lat_deg: world.topo.lat[id].to_degrees(),
            lon_deg: world.topo.lon[id].to_degrees(),
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
