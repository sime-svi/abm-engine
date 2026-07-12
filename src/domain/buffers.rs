//! Buffers reutilizados cada tick para no asignar memoria en el bucle caliente (CLAUDE §5).

use bevy_ecs::prelude::*;

/// Intención de nacimiento acumulada durante un tick (el spawn real ocurre en `apply`).
#[derive(Clone, Copy)]
pub struct BirthIntent {
    pub cell: usize,
    pub energy: f32,
}

/// Intención de muerte acumulada durante un tick (el despawn real ocurre en `apply`).
#[derive(Clone, Copy)]
pub struct DeathIntent {
    pub entity: Entity,
    pub cell: usize,
}

/// Buffers de trabajo del tick. Se reservan una vez y se reutilizan (se vacían, no se reasignan).
#[derive(Resource, Default)]
pub struct SimBuffers {
    /// Agentes del tick en orden canónico `(AgentId, Entity)`.
    pub order: Vec<(u64, Entity)>,
    /// Nacimientos pendientes de aplicar.
    pub births: Vec<BirthIntent>,
    /// Muertes pendientes de aplicar.
    pub deaths: Vec<DeathIntent>,
    /// Celdas reservadas por un nacimiento este tick (longitud = nº de celdas).
    pub reserved: Vec<bool>,
    /// Auxiliar para agregar métricas en orden canónico `(AgentId, energía, edad)`.
    pub metrics_scratch: Vec<(u64, f32, u32)>,
}

/// Recuentos de nacimientos y muertes aplicados en el tick actual.
#[derive(Resource, Default)]
pub struct TickCounts {
    pub births: u64,
    pub deaths: u64,
}
