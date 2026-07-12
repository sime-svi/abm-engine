//! Paso 5 del tick: mortalidad (solo registra intenciones; el despawn ocurre en `apply`).

use bevy_ecs::prelude::*;

use crate::domain::buffers::{DeathIntent, SimBuffers};
use crate::domain::components::{Age, Energy, Position};
use crate::domain::grid::Grid;
use crate::domain::params::Params;

/// Marca para morir a los agentes sin energía (`E <= 0`) o demasiado viejos (`edad > max_age`).
pub fn mortality(
    mut buf: ResMut<SimBuffers>,
    grid: Res<Grid>,
    params: Res<Params>,
    q: Query<(&Energy, &Age, &Position)>,
) {
    let max_age = params.max_age;

    // Índice explícito por la misma razón que en `reproduce` (campos disjuntos de `buf`).
    #[allow(clippy::needless_range_loop)]
    for k in 0..buf.order.len() {
        let (_, entity) = buf.order[k];
        let Ok((energy, age, pos)) = q.get(entity) else {
            continue;
        };
        if energy.0 <= 0.0 || age.0 > max_age {
            let cell = grid.idx(pos.x, pos.y);
            buf.deaths.push(DeathIntent { entity, cell });
        }
    }
}
