//! Paso 6 del tick: sistema **exclusivo** (acceso total al mundo) que materializa las intenciones.
//!
//! Aplica primero las muertes (liberan celda + despawn) y luego los nacimientos (spawn con el
//! siguiente `AgentId` + ocupa la celda reservada), cada grupo en orden determinista. Hacerlo así, y
//! no con `Commands`, mantiene el orden explícito y auditable (clave para un motor científico).

use bevy_ecs::prelude::*;

use crate::domain::buffers::{SimBuffers, TickCounts};
use crate::domain::components::{Age, AgentCounter, AgentId, Energy, Position};
use crate::domain::grid::Grid;

/// Materializa muertes y nacimientos del tick.
pub fn apply(world: &mut World) {
    // Extraer las intenciones reutilizando la capacidad de los Vec.
    let (mut births, mut deaths) = {
        let mut buf = world.resource_mut::<SimBuffers>();
        (
            std::mem::take(&mut buf.births),
            std::mem::take(&mut buf.deaths),
        )
    };

    let n_births = births.len() as u64;
    let n_deaths = deaths.len() as u64;

    // Muertes primero: liberan su celda y se eliminan.
    for death in &deaths {
        world.resource_mut::<Grid>().occupancy[death.cell] = None;
        let _ = world.despawn(death.entity);
    }

    // Nacimientos después: hijo con el siguiente AgentId en su celda reservada.
    for birth in &births {
        let id = {
            let mut counter = world.resource_mut::<AgentCounter>();
            let value = counter.0;
            counter.0 += 1;
            value
        };
        let (x, y) = {
            let grid = world.resource::<Grid>();
            (
                birth.cell as u32 % grid.width,
                birth.cell as u32 / grid.width,
            )
        };
        let child = world
            .spawn((Position { x, y }, Energy(birth.energy), Age(0), AgentId(id)))
            .id();
        world.resource_mut::<Grid>().occupancy[birth.cell] = Some(child);
    }

    // Recuentos del tick.
    {
        let mut counts = world.resource_mut::<TickCounts>();
        counts.births = n_births;
        counts.deaths = n_deaths;
    }

    // Devolver los buffers vacíos (con su capacidad) a `SimBuffers`.
    births.clear();
    deaths.clear();
    let mut buf = world.resource_mut::<SimBuffers>();
    buf.births = births;
    buf.deaths = deaths;
}
