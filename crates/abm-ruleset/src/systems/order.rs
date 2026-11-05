//! Paso 2 del tick: orden canónico y limpieza de buffers.

use bevy_ecs::prelude::*;

use crate::buffers::SimBuffers;
use crate::components::AgentId;

/// Recolecta los agentes vivos y los ordena por `AgentId` (clave única => orden único). Deja listos
/// los buffers del tick (nacimientos/muertes vacíos, reservas a `false`).
pub fn build_order(mut buf: ResMut<SimBuffers>, q: Query<(&AgentId, Entity)>) {
    buf.order.clear();
    for (id, entity) in &q {
        buf.order.push((id.0, entity));
    }
    buf.order.sort_unstable_by_key(|&(id, _)| id);

    buf.births.clear();
    buf.deaths.clear();
    for reserved in buf.reserved.iter_mut() {
        *reserved = false;
    }
}
