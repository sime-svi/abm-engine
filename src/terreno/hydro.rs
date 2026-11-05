//! Derivados globales del relieve (incremento 2 §4). Dependen de la malla entera, así que se
//! recalculan sobre las hojas actuales en la génesis y tras cada cambio de topología:
//!
//! 1. clasificación base tierra/océano (`h < nivel_mar`);
//! 2. priority-flood desde el océano (`filled_h`, receptores, orden de flujo);
//! 3. área de drenaje acumulada (m², invariante a la escala);
//! 4. distancia al mar por Dijkstra con distancias reales entre centros;
//! 5. etiquetas río (área de drenaje sobre umbral) y lago (depresión rellenada).
//!
//! Desempates por posición en `keys` (orden canónico) => determinista. Coste `O(N log N)`.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::terreno::config::TerrenoConfig;
use crate::terreno::geometry::angle_between;
use crate::terreno::world::{Surface, World};

/// Ítem de una cola de prioridad mínima por valor, con desempate por posición.
struct MinItem {
    v: f64,
    id: u32,
}
impl PartialEq for MinItem {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for MinItem {}
impl Ord for MinItem {
    fn cmp(&self, o: &Self) -> Ordering {
        // Invertido: `BinaryHeap` es max-heap, así extrae el menor valor (y menor id en empate).
        o.v.total_cmp(&self.v).then(o.id.cmp(&self.id))
    }
}
impl PartialOrd for MinItem {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

/// Recalcula todos los derivados globales del mundo sobre su malla actual.
pub fn recompute(world: &mut World, cfg: &TerrenoConfig) {
    recompute_flow(world, cfg);
    recompute_dist_ocean(world);
    label_rivers_lakes(world, cfg);
}

/// Recalcula los derivados tras un paso de erosión (incr. 3): la red de flujo y las etiquetas. La
/// distancia al mar no cambia, porque la erosión nunca convierte tierra en mar (incr. 3 §3).
pub fn recompute_after_erosion(world: &mut World, cfg: &TerrenoConfig) {
    recompute_flow(world, cfg);
    label_rivers_lakes(world, cfg);
}

/// Pasos 1–3: clasificación base, priority-flood (receptores, `filled_h`, orden de flujo) y área de
/// drenaje.
fn recompute_flow(world: &mut World, _cfg: &TerrenoConfig) {
    let num = world.topo.num_cells;

    // --- 1) Clasificación base ---
    world.surface.clear();
    world.surface.extend(world.h.iter().map(|&h| {
        if world.statgen.is_ocean_h(h) {
            Surface::Oceano
        } else {
            Surface::Tierra
        }
    }));

    // --- 2) Priority-flood desde el océano + direcciones de flujo ---
    world.filled_h.clear();
    world.filled_h.extend_from_slice(&world.h);
    world.receiver.clear();
    world.receiver.resize(num, -1);
    let mut visited: Vec<bool> = world
        .surface
        .iter()
        .map(|s| matches!(s, Surface::Oceano))
        .collect();
    let mut heap = BinaryHeap::with_capacity(num);
    for (s, _) in visited.iter().enumerate().filter(|(_, v)| **v) {
        heap.push(MinItem {
            v: world.h[s],
            id: s as u32,
        });
    }
    let has_ocean = !heap.is_empty();
    if !has_ocean && num > 0 {
        // Sin océano: se siembra la celda más baja (la primera en empate) como único sumidero.
        let low = (0..num)
            .min_by(|&a, &b| world.h[a].total_cmp(&world.h[b]))
            .unwrap_or(0);
        visited[low] = true;
        heap.push(MinItem {
            v: world.h[low],
            id: low as u32,
        });
    }
    // Orden de extracción (bajo→alto): el receptor de una celda es su "padre" (aguas abajo).
    let mut pop_order = Vec::with_capacity(num);
    while let Some(MinItem { v: elev, id }) = heap.pop() {
        pop_order.push(id);
        for &nb in world.topo.neighbors(id as usize) {
            let nb = nb as usize;
            if !visited[nb] {
                visited[nb] = true;
                world.filled_h[nb] = world.h[nb].max(elev);
                world.receiver[nb] = id as i64;
                heap.push(MinItem {
                    v: world.filled_h[nb],
                    id: nb as u32,
                });
            }
        }
    }

    // --- 3) Acumulación de flujo en orden inverso de extracción (aguas arriba primero) ---
    world.flow_order.clear();
    world.flow_order.extend(pop_order.iter().rev().copied());
    world.drainage_area.clear();
    world.drainage_area.extend_from_slice(&world.topo.area);
    for &id in world.flow_order.iter() {
        let r = world.receiver[id as usize];
        if r >= 0 {
            world.drainage_area[r as usize] += world.drainage_area[id as usize];
        }
    }
}

/// Paso 4: distancia a la fuente de humedad, Dijkstra multi-fuente con distancias reales.
fn recompute_dist_ocean(world: &mut World) {
    let num = world.topo.num_cells;
    let r_km = world.topo.r_planet / 1000.0;
    world.dist_ocean_km.clear();
    world.dist_ocean_km.resize(num, f64::INFINITY);
    let mut heap = BinaryHeap::with_capacity(num);
    for s in 0..num {
        if matches!(world.surface[s], Surface::Oceano) {
            world.dist_ocean_km[s] = 0.0;
            heap.push(MinItem {
                v: 0.0,
                id: s as u32,
            });
        }
    }
    while let Some(MinItem { v: d, id }) = heap.pop() {
        let id = id as usize;
        if d > world.dist_ocean_km[id] {
            continue; // entrada obsoleta
        }
        for &nb in world.topo.neighbors(id) {
            let nb = nb as usize;
            let nd = d + angle_between(world.topo.dir[id], world.topo.dir[nb]) * r_km;
            if nd < world.dist_ocean_km[nb] {
                world.dist_ocean_km[nb] = nd;
                heap.push(MinItem {
                    v: nd,
                    id: nb as u32,
                });
            }
        }
    }
    for d in world.dist_ocean_km.iter_mut() {
        if d.is_infinite() {
            *d = 0.0; // mundo sin océano: humedad uniforme
        }
    }
}

/// Paso 5: clasificación final: ríos (por área de drenaje) y lagos (depresiones rellenadas).
fn label_rivers_lakes(world: &mut World, cfg: &TerrenoConfig) {
    let num = world.topo.num_cells;
    let river_threshold_m2 = cfg.worldgen.river_threshold_km2 * 1e6;
    for s in 0..num {
        if matches!(world.surface[s], Surface::Oceano) {
            continue;
        }
        if world.drainage_area[s] > river_threshold_m2 {
            world.surface[s] = Surface::Rio;
        } else if world.filled_h[s] > world.h[s] + 1.0 {
            world.surface[s] = Surface::Lago;
        }
    }
}
