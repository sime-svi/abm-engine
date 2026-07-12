//! Tiempo de simulación: el `tick`, un entero. Nunca el reloj de pared (CLAUDE §6).

use bevy_ecs::prelude::*;

/// Tick actual de la simulación (recurso global del mundo).
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct Tick(pub u64);
