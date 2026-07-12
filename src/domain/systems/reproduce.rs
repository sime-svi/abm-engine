//! Paso 4 del tick: reproducción (solo registra intenciones; el spawn ocurre en `apply`).

use bevy_ecs::prelude::*;

use crate::domain::buffers::{BirthIntent, SimBuffers};
use crate::domain::components::{Energy, Position};
use crate::domain::grid::Grid;
use crate::domain::params::Params;

/// Cada agente con `E >= umbral` engendra un hijo en la primera celda vecina libre y no reservada.
/// La madre conserva `(1 - phi) * E`; el hijo hereda `phi * E`.
pub fn reproduce(
    mut buf: ResMut<SimBuffers>,
    grid: Res<Grid>,
    params: Res<Params>,
    mut q: Query<(&mut Energy, &Position)>,
) {
    if !params.repro_enabled {
        return;
    }
    let threshold = params.repro_threshold;
    let phi = params.child_fraction;
    let single = params.single_occupancy;
    let offsets = grid.offsets();

    // Índice explícito: dentro del bucle tomamos prestados campos disjuntos de `buf`
    // (`order` de lectura, `births`/`reserved` de escritura), que el iterador no permitiría.
    #[allow(clippy::needless_range_loop)]
    for k in 0..buf.order.len() {
        let (_, entity) = buf.order[k];
        let Ok((mut energy, pos)) = q.get_mut(entity) else {
            continue;
        };
        if energy.0 < threshold {
            continue;
        }

        // Primera celda vecina libre y no reservada (orden de offset fijo).
        let mut target: Option<usize> = None;
        for &(dx, dy) in offsets {
            if let Some((nx, ny)) = grid.neighbor(pos.x, pos.y, dx, dy) {
                let ni = grid.idx(nx, ny);
                let free = !single || grid.occupancy[ni].is_none();
                if free && !buf.reserved[ni] {
                    target = Some(ni);
                    break;
                }
            }
        }
        let Some(cell) = target else {
            continue;
        };

        let child_energy = phi * energy.0;
        energy.0 -= child_energy;
        buf.reserved[cell] = true;
        buf.births.push(BirthIntent {
            cell,
            energy: child_energy,
        });
    }
}
