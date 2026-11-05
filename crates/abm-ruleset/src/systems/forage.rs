//! Paso 3 del tick: movimiento + consumo + metabolismo, por agente en orden canónico.

use bevy_ecs::prelude::*;

use crate::buffers::SimBuffers;
use crate::components::{Age, Energy, Position};
use crate::grid::Grid;
use crate::params::Params;

/// Para cada agente: se mueve a la celda de mayor recurso entre la actual y las vecinas libres
/// (desempate por orden de offset fijo, la actual primero), consume recurso, metaboliza y envejece.
pub fn forage(
    buf: Res<SimBuffers>,
    mut grid: ResMut<Grid>,
    params: Res<Params>,
    mut q: Query<(&mut Position, &mut Energy, &mut Age)>,
) {
    let appetite = params.appetite;
    let metabolism = params.metabolism;
    let single = params.single_occupancy;
    let offsets = grid.offsets();

    for &(_, entity) in buf.order.iter() {
        let Ok((mut pos, mut energy, mut age)) = q.get_mut(entity) else {
            continue;
        };

        // Paso 1: elegir destino (celda actual o mejor vecina libre).
        let current = grid.idx(pos.x, pos.y);
        let mut best_idx = current;
        let mut best_x = pos.x;
        let mut best_y = pos.y;
        let mut best_r = grid.resource[current];
        for &(dx, dy) in offsets {
            if let Some((nx, ny)) = grid.neighbor(pos.x, pos.y, dx, dy) {
                let ni = grid.idx(nx, ny);
                let free = !single || grid.occupancy[ni].is_none();
                if free && grid.resource[ni] > best_r {
                    best_r = grid.resource[ni];
                    best_idx = ni;
                    best_x = nx;
                    best_y = ny;
                }
            }
        }

        // Mover si cambia de celda (actualiza la ocupación).
        if best_idx != current {
            grid.occupancy[current] = None;
            grid.occupancy[best_idx] = Some(entity);
            pos.x = best_x;
            pos.y = best_y;
        }

        // Paso 2: consumir.
        let consumed = grid.resource[best_idx].min(appetite);
        energy.0 += consumed;
        grid.resource[best_idx] -= consumed;

        // Paso 3: metabolizar y envejecer.
        energy.0 -= metabolism;
        age.0 += 1;
    }
}
