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

use crate::config::TerrenoConfig;
use crate::geometry::angle_between;
use crate::world::{Surface, World};

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
    // Modo Tierra: la red es un dato (adenda 1 §1); la hidrología del motor queda para el planeta
    // inventado, sin cambios.
    if let Some(d) = world.statgen.tierra().cloned() {
        red_de_la_tierra(world, cfg, &d);
        recompute_dist_ocean(world);
        return;
    }
    recompute_flow(world, cfg);
    recompute_dist_ocean(world);
    label_rivers_lakes(world, cfg);
}

/// Receptor de una celda terminal de una cuenca cerrada (modo Tierra): el agua se evapora allí.
pub const CERRADA: i64 = -2;

/// Red de ríos del modo Tierra sobre la malla actual (niveles mezclados), derivada de la red del nivel
/// de carga con la misma regla que esta (de salida a salida):
/// - mar, la celda con al menos la mitad de su área en el mar conectado con el océano mundial;
/// - la salida de una celda es su descendiente de tierra del nivel de carga con más área drenada;
///   desde ella se sigue la red del nivel de carga hasta la salida de otra celda de tierra de la malla,
///   que es la de aguas abajo (puede no ser vecina). Si la red acaba antes, el receptor es −1 (sale al
///   mar) o [`CERRADA`] (el fondo de una cuenca cerrada). El agua de una celda acaba así donde acaba
///   la de su salida en el nivel de carga, y el área drenada de la salida crece estrictamente río
///   abajo, así que la red no tiene ciclos;
/// - orden de flujo de aguas arriba abajo, área de drenaje acumulada, y ríos y lagos: lago si la mitad
///   de la celda es lago de HydroLAKES, río si drena más de `river_threshold_km2`.
fn red_de_la_tierra(world: &mut World, cfg: &TerrenoConfig, d: &crate::tierra::DatosTierra) {
    use crate::geometry::{key, split_key};
    use crate::tierra::hidro::{NINGUNA, fin};
    let num = world.topo.num_cells;
    let lc = d.nivel_carga;
    let fino = &d.niveles[lc as usize];
    let n = fino.n;
    world.surface.clear();
    world.surface.extend((0..num).map(|s| {
        let (lv, k) = d.celda_de_clave(world.topo.keys[s]);
        if d.mar_frac[lv][k] >= 0.5 {
            Surface::Oceano
        } else {
            Surface::Tierra
        }
    }));
    // Celda de la malla que contiene una celda del nivel de carga.
    let malla_de = |k: usize| -> Option<usize> {
        let nn = (n as usize) * (n as usize);
        let (c, r) = ((k / nn) as u8, k % nn);
        let (j, i) = ((r / n as usize) as u32, (r % n as usize) as u32);
        for l in (0..=lc).rev() {
            let s = lc - l;
            if let Some(q) = world.topo.slot_of(key(c, l, i >> s, j >> s)) {
                return Some(q);
            }
        }
        (lc + 1..=world.topo.l_max)
            .find_map(|l| world.topo.slot_of(key(c, l, i << (l - lc), j << (l - lc))))
    };
    // Salida de cada celda de tierra de la malla: su descendiente de tierra del nivel de carga con más
    // área drenada (en empate, la primera). Las celdas más finas que el nivel de carga siguen a su
    // antecesora.
    const SIN_SALIDA: usize = usize::MAX;
    let mut salida = vec![SIN_SALIDA; num];
    for (s, sal) in salida.iter_mut().enumerate() {
        if matches!(world.surface[s], Surface::Oceano) {
            continue;
        }
        let (c, l, i, j) = split_key(world.topo.keys[s]);
        let (l, i, j) = if l > lc {
            (lc, i >> (l - lc), j >> (l - lc))
        } else {
            (l, i, j)
        };
        let sh = lc - l;
        let mut o = SIN_SALIDA;
        for jj in (j << sh)..((j + 1) << sh) {
            for ii in (i << sh)..((i + 1) << sh) {
                let k = fino.indice(c, ii, jj);
                if !d.mar[k] && (o == SIN_SALIDA || d.red.acum[k] > d.red.acum[o]) {
                    o = k;
                }
            }
        }
        *sal = o;
    }
    world.receiver.clear();
    world.receiver.resize(num, -1);
    for s in 0..num {
        if salida[s] == SIN_SALIDA {
            continue;
        }
        let mut x = salida[s];
        world.receiver[s] = loop {
            let a = d.red.abajo[x];
            if a == NINGUNA {
                break if d.red.fin[x] == fin::CERRADA {
                    CERRADA
                } else {
                    -1
                };
            }
            let a = a as usize;
            match malla_de(a) {
                Some(q) if q != s && salida[q] == a => break q as i64,
                _ => x = a,
            }
        };
    }
    // Orden de flujo (Kahn, con la cola en orden de posición): de aguas arriba abajo.
    let mut entradas = vec![0u32; num];
    for &r in &world.receiver {
        if r >= 0 {
            entradas[r as usize] += 1;
        }
    }
    let mut cola: std::collections::VecDeque<u32> = (0..num as u32)
        .filter(|&s| entradas[s as usize] == 0)
        .collect();
    world.flow_order.clear();
    while let Some(s) = cola.pop_front() {
        world.flow_order.push(s);
        let r = world.receiver[s as usize];
        if r >= 0 {
            entradas[r as usize] -= 1;
            if entradas[r as usize] == 0 {
                cola.push_back(r as u32);
            }
        }
    }
    world.drainage_area.clear();
    world.drainage_area.extend_from_slice(&world.topo.area);
    for &s in world.flow_order.iter() {
        let r = world.receiver[s as usize];
        if r >= 0 {
            world.drainage_area[r as usize] += world.drainage_area[s as usize];
        }
    }
    world.filled_h.clear();
    world.filled_h.extend_from_slice(&world.h);
    let umbral = cfg.worldgen.river_threshold_km2 * 1e6;
    for s in 0..num {
        if matches!(world.surface[s], Surface::Oceano) {
            continue;
        }
        if world.lago[s] >= 0.5 {
            world.surface[s] = Surface::Lago;
        } else if world.drainage_area[s] > umbral {
            world.surface[s] = Surface::Rio;
        }
    }
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
///
/// Se mide desde la **línea de costa**: una celda de tierra que toca el mar empieza a la distancia
/// de su centro al borde compartido (el centro a centro repartido según el tamaño de cada celda).
/// Medida desde el centro de la celda de mar, como hasta el incremento 3, sobraba media celda de mar
/// en cada distancia, y al afinar la malla la costa "se acercaba" y llovía más (+3 % de 208 a 104 km;
/// nota de lluvia orográfica, §8).
fn recompute_dist_ocean(world: &mut World) {
    let num = world.topo.num_cells;
    let r_km = world.topo.r_planet / 1000.0;
    world.dist_ocean_km.clear();
    world.dist_ocean_km.resize(num, f64::INFINITY);
    let mut heap = BinaryHeap::with_capacity(num);
    let ocean = |w: &World, s: usize| matches!(w.surface[s], Surface::Oceano);
    for s in 0..num {
        if ocean(world, s) {
            world.dist_ocean_km[s] = 0.0;
        }
    }
    // Las celdas de tierra que tocan el mar empiezan a la distancia de su centro a la costa.
    for s in 0..num {
        if ocean(world, s) {
            continue;
        }
        let side_s = world.topo.area[s].sqrt();
        let mut best = f64::INFINITY;
        for &nb in world.topo.neighbors(s) {
            let nb = nb as usize;
            if ocean(world, nb) {
                let cc = angle_between(world.topo.dir[s], world.topo.dir[nb]) * r_km;
                let side_o = world.topo.area[nb].sqrt();
                best = best.min(cc * side_s / (side_s + side_o));
            }
        }
        if best.is_finite() {
            world.dist_ocean_km[s] = best;
            heap.push(MinItem {
                v: best,
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
