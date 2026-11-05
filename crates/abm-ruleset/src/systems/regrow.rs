//! Paso 1 del tick: regeneración del recurso de cada celda.

use bevy_ecs::prelude::*;

use crate::grid::Grid;

/// `R = min(R + regrowth, max)` para toda celda, en orden de índice.
pub fn regrow(mut grid: ResMut<Grid>) {
    let max = grid.max;
    let regrowth = grid.regrowth;
    for r in grid.resource.iter_mut() {
        *r = (*r + regrowth).min(max);
    }
}
