//! Cuadrícula 2D como `Vec` planos (recurso + ocupación), indexados por `y * width + x`. Contigua en
//! memoria (buena para la caché), sin `HashMap` (orden de iteración estable) y sin asignaciones por
//! tick. La vecindad usa órdenes de offset fijos para que el desempate sea determinista.

use bevy_ecs::prelude::*;

/// Vecindad de Moore (8 celdas), en orden fijo.
static MOORE8: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// Vecindad de Von Neumann (4 celdas), en orden fijo.
static VON_NEUMANN4: [(i32, i32); 4] = [(0, -1), (-1, 0), (1, 0), (0, 1)];

/// Cuadrícula del mundo. `resource` y `occupancy` comparten el mismo índice lineal.
#[derive(Resource)]
pub struct Grid {
    pub width: u32,
    pub height: u32,
    pub toroidal: bool,
    pub neighborhood: u8,
    pub max: f32,
    pub regrowth: f32,
    /// Recurso de cada celda. Índice `idx(x, y) = y * width + x`.
    pub resource: Vec<f32>,
    /// Ocupante de cada celda (`None` = libre). Mismo índice que `resource`.
    pub occupancy: Vec<Option<Entity>>,
}

impl Grid {
    /// Índice lineal de la celda `(x, y)`.
    #[inline]
    pub fn idx(&self, x: u32, y: u32) -> usize {
        (y as usize) * (self.width as usize) + (x as usize)
    }

    /// Desplazamientos de vecindad según la config (orden fijo).
    #[inline]
    pub fn offsets(&self) -> &'static [(i32, i32)] {
        if self.neighborhood == 4 {
            &VON_NEUMANN4
        } else {
            &MOORE8
        }
    }

    /// Coordenada vecina tras aplicar `(dx, dy)`. En cuadrícula toroidal los bordes se envuelven; en
    /// caso contrario devuelve `None` si cae fuera.
    #[inline]
    pub fn neighbor(&self, x: u32, y: u32, dx: i32, dy: i32) -> Option<(u32, u32)> {
        let nx = x as i32 + dx;
        let ny = y as i32 + dy;
        if self.toroidal {
            Some((
                nx.rem_euclid(self.width as i32) as u32,
                ny.rem_euclid(self.height as i32) as u32,
            ))
        } else if nx < 0 || ny < 0 || nx >= self.width as i32 || ny >= self.height as i32 {
            None
        } else {
            Some((nx as u32, ny as u32))
        }
    }
}
