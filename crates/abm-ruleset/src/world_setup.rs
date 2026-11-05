//! Génesis determinista del mundo: crea el `World`, inserta los recursos globales del modelo y siembra
//! los agentes iniciales. Todo sorteo usa el RNG sembrado en orden fijo (CLAUDE §6). El destino de
//! las métricas (`Sink`) no es cosa del modelo: lo inserta quien monta la simulación.

use bevy_ecs::prelude::*;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::buffers::{SimBuffers, TickCounts};
use crate::components::{Age, AgentCounter, AgentId, Energy, Position};
use crate::config::{Config, InitialFill};
use crate::grid::Grid;
use crate::params::Params;
use abm_core::rng::RootRng;
use abm_core::time::Tick;

/// Devuelve un entero en `[0, u32::MAX]` normalizado a `[0, 1)` como `f32`.
fn unit_f32(rng: &mut ChaCha8Rng) -> f32 {
    // 24 bits de mantisa bastan para un f32; dividir por 2^32 da un uniforme reproducible.
    (rng.next_u32() as f64 / (u32::MAX as f64 + 1.0)) as f32
}

/// Construye el mundo inicial a partir de la configuración (ya validada).
pub fn build(cfg: &Config) -> World {
    let mut world = World::new();

    let w = cfg.grid.width;
    let h = cfg.grid.height;
    let n_cells = (w as usize) * (h as usize);
    let count = (cfg.agents.initial_count as usize).min(n_cells);

    let mut rng = ChaCha8Rng::seed_from_u64(cfg.run.seed);

    // --- Recurso de las celdas ---
    let mut resource = vec![0.0f32; n_cells];
    match cfg.resource.initial_fill {
        InitialFill::Full => {
            for r in resource.iter_mut() {
                *r = cfg.resource.max;
            }
        }
        InitialFill::Random => {
            for r in resource.iter_mut() {
                *r = unit_f32(&mut rng) * cfg.resource.max;
            }
        }
    }

    // --- Celdas iniciales de los agentes: Fisher-Yates parcial (distintas, deterministas) ---
    let mut cells: Vec<u32> = (0..n_cells as u32).collect();
    for i in 0..count {
        let remaining = (n_cells - i) as u64;
        let j = i + (rng.next_u64() % remaining) as usize;
        cells.swap(i, j);
    }

    let mut grid = Grid {
        width: w,
        height: h,
        toroidal: cfg.grid.toroidal,
        neighborhood: cfg.grid.neighborhood,
        max: cfg.resource.max,
        regrowth: cfg.resource.regrowth,
        resource,
        occupancy: vec![None; n_cells],
    };

    // --- Agentes iniciales (AgentId 0..count, en orden) ---
    let emin = cfg.agents.init_energy_min;
    let espan = cfg.agents.init_energy_max - cfg.agents.init_energy_min;
    for (i, &cell) in cells.iter().take(count).enumerate() {
        let cell = cell as usize;
        let x = (cell as u32) % w;
        let y = (cell as u32) / w;
        let energy = emin + unit_f32(&mut rng) * espan;
        let entity = world
            .spawn((Position { x, y }, Energy(energy), Age(0), AgentId(i as u64)))
            .id();
        grid.occupancy[cell] = Some(entity);
    }

    // --- Recursos globales del mundo ---
    world.insert_resource(Tick(0));
    world.insert_resource(AgentCounter(count as u64));
    world.insert_resource(Params {
        appetite: cfg.agents.appetite,
        metabolism: cfg.agents.metabolism,
        max_age: cfg.agents.max_age,
        repro_enabled: cfg.reproduction.enabled,
        repro_threshold: cfg.reproduction.energy_threshold,
        child_fraction: cfg.reproduction.child_energy_fraction,
        single_occupancy: cfg.grid.single_occupancy,
        metrics_every: cfg.metrics.metrics_every,
    });
    world.insert_resource(SimBuffers {
        reserved: vec![false; n_cells],
        order: Vec::with_capacity(count.max(64)),
        metrics_scratch: Vec::with_capacity(count.max(64)),
        ..Default::default()
    });
    world.insert_resource(TickCounts::default());
    world.insert_resource(RootRng(rng));
    world.insert_resource(grid);

    world
}
