//! Tiempo de simulación: el `tick`, un entero. Nunca el reloj de pared (CLAUDE §6).

use bevy_ecs::prelude::*;

/// Tick actual de la simulación (recurso global del mundo).
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct Tick(pub u64);

/// El bucle de tiempo: corre `ticks` pasos del `schedule` sobre el `world`, actualizando antes de
/// cada uno el recurso [`Tick`] (0, 1, 2…). Requiere que el mundo tenga el recurso `Tick`.
pub fn run_ticks(world: &mut World, schedule: &mut Schedule, ticks: u64) {
    for t in 0..ticks {
        world.resource_mut::<Tick>().0 = t;
        schedule.run(world);
    }
}
