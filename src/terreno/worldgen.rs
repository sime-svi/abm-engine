//! Generación del mundo en t=0 (diseño §4). Todo depende solo de la semilla => mundo reproducible.
//!
//! Cadena: topología → elevación (ruido 3D) → clasificación → pendiente/suelo → rellenado de
//! depresiones (priority-flood) → red de drenaje → ríos/lagos → recurso. El relieve resultante es
//! **estático** en este incremento (sin erosión ni pedogénesis).

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::VecDeque;

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::terreno::config::TerrenoConfig;
use crate::terreno::geometry;
use crate::terreno::noise::Perlin3;
use crate::terreno::world::{Surface, Topology, World};

#[inline]
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
#[inline]
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
#[inline]
fn normalize(a: [f64; 3]) -> [f64; 3] {
    let n = dot(a, a).sqrt();
    if n > 0.0 {
        [a[0] / n, a[1] / n, a[2] / n]
    } else {
        [1.0, 0.0, 0.0]
    }
}

/// Construye la topología del cubo esférico a nivel uniforme `n`.
fn build_topology(n: u32, r_planet: f64) -> Topology {
    let num_cells = 6 * (n as usize) * (n as usize);
    let mut lat = vec![0.0; num_cells];
    let mut lon = vec![0.0; num_cells];
    let mut area = vec![0.0; num_cells];
    let mut east = vec![[0.0; 3]; num_cells];
    let mut north = vec![[0.0; 3]; num_cells];
    let mut dir = vec![[0.0; 3]; num_cells];
    let mut neighbors = vec![[-1_i64; 8]; num_cells];

    for c in 0..6u8 {
        for j in 0..n {
            for i in 0..n {
                let id = geometry::cell_id(c, i, j, n) as usize;
                let d = geometry::center_dir(c, i, j, n);
                let (la, lo) = geometry::lat_lon(d);
                dir[id] = d;
                lat[id] = la;
                lon[id] = lo;
                area[id] = geometry::cell_area(c, i, j, n, r_planet);
                // Base local este/norte (ENU). En los polos, cross(z, up)~0 => este = +X.
                let z = [0.0, 0.0, 1.0];
                let mut e = cross(z, d);
                if dot(e, e) < 1e-12 {
                    e = [1.0, 0.0, 0.0];
                }
                let e = normalize(e);
                east[id] = e;
                north[id] = normalize(cross(d, e));
                neighbors[id] = geometry::neighbors(id as u32, n);
            }
        }
    }

    Topology {
        n,
        num_cells,
        r_planet,
        lat,
        lon,
        area,
        east,
        north,
        dir,
        neighbors,
    }
}

/// Ítem de la cola de prioridad del priority-flood (min-heap por elevación, desempate por id).
struct FloodItem {
    elev: f64,
    id: u32,
}
impl PartialEq for FloodItem {
    fn eq(&self, o: &Self) -> bool {
        self.elev == o.elev && self.id == o.id
    }
}
impl Eq for FloodItem {}
impl Ord for FloodItem {
    fn cmp(&self, o: &Self) -> Ordering {
        // Invertido: `BinaryHeap` es max-heap, así extrae la menor elevación (y menor id en empate).
        o.elev.total_cmp(&self.elev).then(o.id.cmp(&self.id))
    }
}
impl PartialOrd for FloodItem {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

/// Genera el mundo completo a partir de la configuración.
pub fn generate(cfg: &TerrenoConfig) -> World {
    let n = cfg.face_n();
    let r_planet = cfg.planet.r_km * 1000.0;
    let topo = build_topology(n, r_planet);
    let num = topo.num_cells;

    // --- a) Elevación por ruido 3D fractal, normalizada a [h_min, h_max] ---
    let perlin = Perlin3::new(cfg.run.seed);
    let f = cfg.worldgen.base_freq;
    let raw: Vec<f64> = (0..num)
        .map(|id| {
            let d = topo.dir[id];
            perlin.fbm(
                d[0] * f,
                d[1] * f,
                d[2] * f,
                cfg.worldgen.noise_octaves,
                cfg.worldgen.persistence,
                cfg.worldgen.lacunarity,
            )
        })
        .collect();
    let (mut mn, mut mx) = (f64::INFINITY, f64::NEG_INFINITY);
    for &v in &raw {
        mn = mn.min(v);
        mx = mx.max(v);
    }
    let span = (mx - mn).max(1e-9);
    let sea = cfg.planet.sea_level;
    let sea_frac =
        ((sea - cfg.planet.h_min) / (cfg.planet.h_max - cfg.planet.h_min)).clamp(0.0, 1.0);
    let mut h = vec![0.0; num];
    for id in 0..num {
        let frac = (raw[id] - mn) / span;
        h[id] = if frac < sea_frac {
            // Océano: profundidad lineal.
            cfg.planet.h_min + frac * (cfg.planet.h_max - cfg.planet.h_min)
        } else {
            // Tierra: curva hipsométrica (potencia) => la mayoría cerca del nivel del mar.
            let lf = (frac - sea_frac) / (1.0 - sea_frac).max(1e-9);
            sea + lf.powf(cfg.worldgen.land_exp) * (cfg.planet.h_max - sea)
        };
    }

    // --- b) Clasificación provisional de superficie ---
    let mut surface = vec![Surface::Tierra; num];
    for id in 0..num {
        if h[id] < cfg.planet.sea_level {
            surface[id] = Surface::Oceano;
        }
    }

    // --- c) Pendiente y orientación (gradiente por mínimos cuadrados en el plano tangente) ---
    let mut slope = vec![0.0; num];
    let mut aspect = vec![0.0; num];
    let mut grad_e = vec![0.0; num];
    let mut grad_n = vec![0.0; num];
    for id in 0..num {
        let (e, nn) = (topo.east[id], topo.north[id]);
        let (mut sxx, mut sxy, mut syy, mut bx, mut by) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for &nb in topo.neighbors[id].iter() {
            if nb < 0 {
                continue;
            }
            let nb = nb as usize;
            let disp = [
                topo.dir[nb][0] - topo.dir[id][0],
                topo.dir[nb][1] - topo.dir[id][1],
                topo.dir[nb][2] - topo.dir[id][2],
            ];
            let de = dot(disp, e) * r_planet;
            let dn = dot(disp, nn) * r_planet;
            let dh = h[nb] - h[id];
            sxx += de * de;
            sxy += de * dn;
            syy += dn * dn;
            bx += de * dh;
            by += dn * dh;
        }
        let det = sxx * syy - sxy * sxy;
        let (gx, gy) = if det.abs() > 1e-6 {
            ((syy * bx - sxy * by) / det, (sxx * by - sxy * bx) / det)
        } else {
            (0.0, 0.0)
        };
        grad_e[id] = gx;
        grad_n[id] = gy;
        slope[id] = (gx * gx + gy * gy).sqrt().atan();
        aspect[id] = gy.atan2(gx);
    }

    // --- d) Textura de suelo y derivados (ruido sembrado sesgado por relieve) ---
    let tex_noise = Perlin3::new(cfg.run.seed ^ 0x5015);
    let mut w_cap = vec![0.0; num];
    let mut k_perm = vec![0.0; num];
    let mut fert = vec![0.0; num];
    for id in 0..num {
        let d = topo.dir[id];
        let ns = 0.5 * (tex_noise.get(d[0] * 6.0, d[1] * 6.0, d[2] * 6.0) + 1.0); // [0,1]
        // Laderas empinadas => más arenoso; llanuras => más limo/arcilla.
        let sand = (0.35 + 0.4 * ns + 0.4 * (slope[id].tan()).min(1.0)).clamp(0.1, 0.9);
        let fine = 1.0 - sand;
        let silt = fine * 0.6;
        let clay = fine * 0.4;
        w_cap[id] = cfg.soil.w_cap_min
            + (cfg.soil.w_cap_max - cfg.soil.w_cap_min) * (clay * 0.7 + silt * 0.5);
        k_perm[id] = sand.clamp(0.05, 1.0);
        fert[id] = (silt * 0.5 + clay * 0.6).clamp(0.05, 0.95);
    }

    // --- e) Rellenado de depresiones (priority-flood desde el océano) + direcciones de flujo ---
    let mut filled_h = h.clone();
    let mut receiver = vec![-1_i64; num];
    let mut visited = vec![false; num];
    let mut heap = BinaryHeap::new();
    let mut has_ocean = false;
    for id in 0..num {
        if matches!(surface[id], Surface::Oceano) {
            visited[id] = true;
            filled_h[id] = h[id];
            heap.push(FloodItem {
                elev: h[id],
                id: id as u32,
            });
            has_ocean = true;
        }
    }
    if !has_ocean {
        // Sin océano: se siembra la celda más baja como único sumidero.
        let low = (0..num).min_by(|&a, &b| h[a].total_cmp(&h[b])).unwrap_or(0);
        visited[low] = true;
        heap.push(FloodItem {
            elev: h[low],
            id: low as u32,
        });
    }
    // Orden de extracción (bajo→alto): el receptor de una celda es su "padre" (aguas abajo).
    let mut pop_order = Vec::with_capacity(num);
    while let Some(FloodItem { elev, id }) = heap.pop() {
        pop_order.push(id);
        for &nb in topo.neighbors[id as usize].iter() {
            if nb < 0 {
                continue;
            }
            let nb = nb as usize;
            if !visited[nb] {
                visited[nb] = true;
                filled_h[nb] = h[nb].max(elev);
                receiver[nb] = id as i64;
                heap.push(FloodItem {
                    elev: filled_h[nb],
                    id: nb as u32,
                });
            }
        }
    }

    // --- f) Acumulación de flujo (área de drenaje) en orden inverso de extracción (aguas arriba primero) ---
    let flow_order: Vec<u32> = pop_order.iter().rev().copied().collect();
    let mut drainage_area = topo.area.clone();
    for &id in flow_order.iter() {
        let r = receiver[id as usize];
        if r >= 0 {
            drainage_area[r as usize] += drainage_area[id as usize];
        }
    }

    // --- Distancia a la fuente de humedad (BFS multi-fuente desde el océano) ---
    let cell_side_km = (std::f64::consts::FRAC_PI_2 * cfg.planet.r_km) / n as f64;
    let mut dist_ocean_km = vec![f64::INFINITY; num];
    let mut queue = VecDeque::new();
    for id in 0..num {
        if matches!(surface[id], Surface::Oceano) {
            dist_ocean_km[id] = 0.0;
            queue.push_back(id);
        }
    }
    while let Some(id) = queue.pop_front() {
        let base = dist_ocean_km[id];
        for &nb in topo.neighbors[id].iter() {
            if nb < 0 {
                continue;
            }
            let nb = nb as usize;
            if dist_ocean_km[nb].is_infinite() {
                dist_ocean_km[nb] = base + cell_side_km;
                queue.push_back(nb);
            }
        }
    }
    for d in dist_ocean_km.iter_mut() {
        if d.is_infinite() {
            *d = 0.0; // mundo sin océano: humedad uniforme
        }
    }

    // --- Clasificación final: ríos (por área de drenaje) y lagos (depresiones rellenadas) ---
    let river_threshold_m2 = cfg.worldgen.river_threshold_km2 * 1e6;
    for id in 0..num {
        if matches!(surface[id], Surface::Oceano) {
            continue;
        }
        if drainage_area[id] > river_threshold_m2 {
            surface[id] = Surface::Rio;
        } else if filled_h[id] > h[id] + 1.0 {
            surface[id] = Surface::Lago;
        }
    }

    // --- g) Recurso genérico (proceso sembrado, sesgado a tierra) ---
    let mut rng = ChaCha8Rng::seed_from_u64(cfg.run.seed ^ 0xACE1);
    let mut resource = vec![0.0; num];
    for id in 0..num {
        let p: f64 = rng.random();
        let v: f64 = rng.random();
        if !matches!(surface[id], Surface::Oceano) && p < cfg.worldgen.resource_density {
            resource[id] = v * 100.0;
        }
    }

    // --- Estado dinámico inicial ---
    use crate::terreno::world::Biome;
    let mut w = vec![0.0; num];
    let mut veg = vec![0.0; num];
    let mut biome = vec![Biome::Desierto; num];
    for id in 0..num {
        w[id] = 0.5 * w_cap[id];
        if matches!(surface[id], Surface::Oceano) {
            biome[id] = Biome::Oceano;
        } else {
            veg[id] = 1.0;
        }
    }

    World {
        topo,
        h,
        slope,
        aspect,
        grad_e,
        grad_n,
        w_cap,
        k_perm,
        fert,
        filled_h,
        receiver,
        drainage_area,
        flow_order,
        dist_ocean_km,
        resource,
        surface,
        w,
        temp: vec![0.0; num],
        precip: vec![0.0; num],
        swe: vec![0.0; num],
        veg,
        biome,
        temp_sum: vec![0.0; num],
        precip_sum: vec![0.0; num],
        sample_count: 0,
    }
}
