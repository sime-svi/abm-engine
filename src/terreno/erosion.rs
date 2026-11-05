//! Relieve vivo (incremento 3): incisión fluvial implícita, sedimento que rellena depresiones,
//! difusión de ladera y suelo dinámico. Diseño: `docs/design/terreno/incremento-3-diseno.md`.
//!
//! El mismo paso [`step`] sirve para los dos regímenes de tiempo (§2): la **génesis geológica**
//! (`dt` de miles de años, antes del tick 0) y la **erosión en ejecución** (`dt` de un año o menos).
//! Solo cambian `dt` (en años) y de dónde salen las medias de clima: analíticas en la génesis,
//! acumuladas en ejecución.
//!
//! Garantías (§3–§6): el esquema implícito es incondicionalmente estable y monótono; el sedimento se
//! contabiliza exactamente (erosionado = depositado + al mar); la difusión conserva volumen y respeta
//! el principio del máximo; y la tierra nunca baja del nivel del mar (la erosión no mueve agua).

use serde::{Deserialize, Serialize};

use crate::terreno::config::TerrenoConfig;
use crate::terreno::geometry::{angle_between, dot};
use crate::terreno::hydro;
use crate::terreno::lod;
use crate::terreno::systems::{precip_no_season, temp_annual_mean};
use crate::terreno::world::{Surface, World};

/// Resumen de uno o varios pasos de erosión (volúmenes en m³).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ErosionReport {
    /// Pasos de erosión acumulados en este resumen.
    pub steps: u64,
    /// Material arrancado por la incisión fluvial.
    pub eroded_m3: f64,
    /// Sedimento depositado al rellenar depresiones.
    pub deposited_m3: f64,
    /// Sedimento que llega al mar (sale del sistema).
    pub to_ocean_m3: f64,
    /// Cierre del balance de material: `Δ(Σ e·a) + al_mar`. Debe ≈ 0.
    pub residual_m3: f64,
}

impl ErosionReport {
    /// Acumula otro resumen.
    pub fn add(&mut self, o: &ErosionReport) {
        self.steps += o.steps;
        self.eroded_m3 += o.eroded_m3;
        self.deposited_m3 += o.deposited_m3;
        self.to_ocean_m3 += o.to_ocean_m3;
        self.residual_m3 += o.residual_m3;
    }
}

/// Estado del relieve en un instante (para el resumen de la génesis).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ReliefSnapshot {
    pub cells: u64,
    pub lake_cells: u64,
    pub river_cells: u64,
    pub elev_mean_land: f64,
    pub land_fraction: f64,
}

/// Resumen de la génesis geológica (va a los metadatos de la corrida).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct GeoSummary {
    pub years: f64,
    pub erosion: ErosionReport,
    /// Material añadido por el acondicionamiento de hojas nuevas en los refinados de la génesis.
    pub lod_fill_m3: f64,
    pub before: ReliefSnapshot,
    pub after: ReliefSnapshot,
}

/// Foto del relieve: nº de celdas, lagos y ríos, elevación media de la tierra y fracción de tierra.
pub fn snapshot(world: &World) -> ReliefSnapshot {
    let (mut land_a, mut total_a, mut elev_a) = (0.0, 0.0, 0.0);
    let (mut lakes, mut rivers) = (0u64, 0u64);
    for s in 0..world.topo.num_cells {
        let a = world.topo.area[s];
        total_a += a;
        match world.surface[s] {
            Surface::Oceano => continue,
            Surface::Lago => lakes += 1,
            Surface::Rio => rivers += 1,
            Surface::Tierra => {}
        }
        land_a += a;
        elev_a += world.h[s] * a;
    }
    ReliefSnapshot {
        cells: world.topo.num_cells as u64,
        lake_cells: lakes,
        river_cells: rivers,
        elev_mean_land: if land_a > 0.0 { elev_a / land_a } else { 0.0 },
        land_fraction: if total_a > 0.0 { land_a / total_a } else { 0.0 },
    }
}

/// Medias anuales de clima analíticas (génesis, §4 y §7): `(P̄ en m/año, T̄ en °C)` por celda.
pub fn analytic_means(world: &World, cfg: &TerrenoConfig) -> (Vec<f64>, Vec<f64>) {
    let n = world.topo.num_cells;
    let t_year = cfg.climate.t_year;
    let mut p = Vec::with_capacity(n);
    let mut t = Vec::with_capacity(n);
    for s in 0..n {
        let lat = world.topo.lat[s];
        let mm_tick = precip_no_season(cfg, lat, world.dist_ocean_km[s], world.grad_e[s]);
        p.push(mm_tick.max(0.0) * t_year / 1000.0);
        t.push(temp_annual_mean(cfg, lat, world.h[s], world.es_oceano(s)));
    }
    (p, t)
}

/// Medias anuales de clima acumuladas durante la simulación (ejecución, §9). Antes de la primera
/// muestra se usan las analíticas.
pub fn running_means(world: &World, cfg: &TerrenoConfig) -> (Vec<f64>, Vec<f64>) {
    if world.sample_count == 0 {
        return analytic_means(world, cfg);
    }
    let n = world.sample_count as f64;
    let t_year = cfg.climate.t_year;
    let p = world
        .precip_sum
        .iter()
        .map(|&x| (x / n).max(0.0) * t_year / 1000.0)
        .collect();
    let t = world.temp_sum.iter().map(|&x| x / n).collect();
    (p, t)
}

/// Un paso de erosión de `dt_years` años (§4–§7) con las medias de clima dadas (`P̄` en m/año,
/// `T̄` en °C). Requiere los derivados globales al día y los deja al día al terminar.
pub fn step(
    world: &mut World,
    cfg: &TerrenoConfig,
    dt_years: f64,
    p_bar: &[f64],
    t_bar: &[f64],
) -> ErosionReport {
    let n = world.topo.num_cells;
    let ec = &cfg.erosion;
    let sea = cfg.planet.sea_level;
    let r_m = world.topo.r_planet;
    let mut rep = ErosionReport {
        steps: 1,
        ..ErosionReport::default()
    };
    let mut dvol = 0.0; // Δ(Σ e·a), acumulado cambio a cambio (más preciso que restar dos sumas)

    let ocean: Vec<bool> = (0..n).map(|s| world.es_oceano(s)).collect();
    // Celdas en depresión al inicio del paso: no se excavan, reciben sedimento (§4, §5).
    let in_dep: Vec<bool> = (0..n)
        .map(|s| !ocean[s] && world.h[s] < world.filled_h[s])
        .collect();

    // --- Caudal efectivo ponderado por lluvia, aguas arriba → abajo (§4) ---
    let mut q: Vec<f64> = (0..n)
        .map(|s| {
            if ocean[s] {
                0.0
            } else {
                p_bar[s].max(0.0) * world.topo.area[s]
            }
        })
        .collect();
    for &id in world.flow_order.iter() {
        let r = world.receiver[id as usize];
        if r >= 0 {
            q[r as usize] += q[id as usize];
        }
    }

    // --- Incisión implícita, del nivel de base hacia arriba (receptor antes que donante) ---
    let mut eps = vec![0.0; n];
    if ec.k_ero > 0.0 {
        for &id in world.flow_order.iter().rev() {
            let i = id as usize;
            if ocean[i] || in_dep[i] {
                continue;
            }
            let r = world.receiver[i];
            if r < 0 {
                continue; // sumidero sin océano
            }
            let r = r as usize;
            // Nivel de base: el mar en la desembocadura, la superficie del lago si el receptor está
            // en una depresión, y si no la elevación (ya actualizada) del receptor.
            let b = if ocean[r] {
                sea
            } else if in_dep[r] {
                world.filled_h[r]
            } else {
                world.h[r]
            };
            let l = angle_between(world.topo.dir[i], world.topo.dir[r]) * r_m;
            if l <= 0.0 {
                continue;
            }
            let f = ec.k_ero * q[i].powf(ec.m_ero) * dt_years / l;
            let h_old = world.h[i];
            let h_new = ((h_old + f * b) / (1.0 + f)).min(h_old).max(sea);
            let d = h_new - h_old; // ≤ 0
            if d < 0.0 {
                let a = world.topo.area[i];
                world.h[i] = h_new;
                world.e[i] += d;
                eps[i] = -d;
                rep.eroded_m3 += -d * a;
                dvol += d * a;
            }
        }
    }

    // --- Sedimento: viaja aguas abajo y rellena depresiones hasta su cota de derrame (§5) ---
    let mut qs = vec![0.0; n];
    let mut dep = vec![0.0; n];
    for &id in world.flow_order.iter() {
        let i = id as usize;
        if ocean[i] {
            continue;
        }
        let a = world.topo.area[i];
        qs[i] += eps[i] * a;
        if in_dep[i] && qs[i] > 0.0 {
            let room = ((world.filled_h[i] - world.h[i]) * a).max(0.0);
            let d = qs[i].min(room);
            if d > 0.0 {
                let dh = d / a;
                world.h[i] += dh;
                world.e[i] += dh;
                dep[i] = dh;
                rep.deposited_m3 += d;
                dvol += dh * a;
                qs[i] -= d;
            }
        }
        let r = world.receiver[i];
        if r < 0 || ocean[r as usize] {
            rep.to_ocean_m3 += qs[i];
        } else {
            qs[r as usize] += qs[i];
        }
    }

    // --- Difusión de ladera entre tierras que comparten arista, explícita con subpasos (§6) ---
    let mut diff = vec![0.0; n];
    if ec.kappa_dif > 0.0 {
        let edges: Vec<(usize, usize, f64)> = world
            .topo
            .pairs
            .iter()
            .filter(|p| p.g == 1.0 && !ocean[p.a as usize] && !ocean[p.b as usize])
            .map(|p| {
                let (i, j) = (p.a as usize, p.b as usize);
                let ell = world.topo.area[i].min(world.topo.area[j]).sqrt();
                let dist = angle_between(world.topo.dir[i], world.topo.dir[j]) * r_m;
                (i, j, if dist > 0.0 { ell / dist } else { 0.0 })
            })
            .collect();
        let mut coef = vec![0.0; n];
        for &(i, j, c) in &edges {
            coef[i] += c;
            coef[j] += c;
        }
        let worst = (0..n)
            .map(|s| ec.kappa_dif * dt_years * coef[s] / world.topo.area[s])
            .fold(0.0_f64, f64::max);
        let n_sub = (2.0 * worst).ceil().max(1.0) as u64;
        let dts = dt_years / n_sub as f64;
        let mut delta = vec![0.0; n];
        for _ in 0..n_sub {
            delta.iter_mut().for_each(|x| *x = 0.0);
            for &(i, j, c) in &edges {
                let flux = ec.kappa_dif * dts * (world.h[i] - world.h[j]) * c; // m³
                delta[i] -= flux / world.topo.area[i];
                delta[j] += flux / world.topo.area[j];
            }
            for s in 0..n {
                if delta[s] != 0.0 {
                    let h_new = (world.h[s] + delta[s]).max(sea);
                    let d = h_new - world.h[s];
                    world.h[s] = h_new;
                    world.e[s] += d;
                    diff[s] += d;
                    dvol += d * world.topo.area[s];
                }
            }
        }
    }

    // --- Suelo: la incisión se lleva el suelo primero, el depósito y la difusión lo mueven, y la
    //     meteorización convierte roca en suelo (§7) ---
    let sc = &cfg.soil;
    for s in 0..n {
        if ocean[s] {
            continue;
        }
        let mut z = world.z_soil[s];
        z = (z - eps[s]).max(0.0);
        z += dep[s];
        z = (z + diff[s]).max(0.0);
        let f_clima = (t_bar[s].max(0.0) / sc.t_ref) * (p_bar[s] * 1000.0 / sc.p_ref_soil).min(1.0);
        z += sc.k_weath * f_clima * (-z / sc.z_ref).exp() * dt_years;
        world.z_soil[s] = z;
    }

    rep.residual_m3 = dvol + rep.to_ocean_m3;
    refresh_w_cap(world, cfg);
    hydro::recompute_after_erosion(world, cfg);
    refresh_gradients(world);
    rep
}

/// Factor de capacidad de campo de un suelo de profundidad `z`: `clamp(z/z_init, f_rock,
/// z_root/z_init)` (§7). Solo retiene agua útil la zona de raíces, y la roca desnuda conserva una
/// fracción a escala de celda. Con `z = z_init` vale exactamente 1.
#[inline]
pub fn w_cap_factor(cfg: &TerrenoConfig, z: f64) -> f64 {
    let s = &cfg.soil;
    (z / s.z_init).clamp(s.f_rock, s.z_root / s.z_init)
}

/// Capacidad de campo actual: `W_cap = W_cap0 · factor(z)` (§7). Con `z = z_init` es exactamente
/// la de la textura (incr. 2).
pub fn refresh_w_cap(world: &mut World, cfg: &TerrenoConfig) {
    for ((wc, &wc0), &z) in world.w_cap.iter_mut().zip(&world.w_cap0).zip(&world.z_soil) {
        *wc = wc0 * w_cap_factor(cfg, z);
    }
}

/// Gradiente, pendiente y orientación de `h = h0 + e` (§3): el de `h0` (puro, guardado) más el de
/// `e` por mínimos cuadrados sobre los vecinos. Con `e ≡ 0` reproduce exactamente el incremento 2.
pub fn refresh_gradients(world: &mut World) {
    let n = world.topo.num_cells;
    let r_m = world.topo.r_planet;
    for s in 0..n {
        let (e_dir, n_dir) = (world.topo.east[s], world.topo.north[s]);
        let (mut sxx, mut sxy, mut syy, mut bx, mut by) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for &b in world.topo.neighbors(s) {
            let b = b as usize;
            let disp = [
                world.topo.dir[b][0] - world.topo.dir[s][0],
                world.topo.dir[b][1] - world.topo.dir[s][1],
                world.topo.dir[b][2] - world.topo.dir[s][2],
            ];
            let de = dot(disp, e_dir) * r_m;
            let dn = dot(disp, n_dir) * r_m;
            let dz = world.e[b] - world.e[s];
            sxx += de * de;
            sxy += de * dn;
            syy += dn * dn;
            bx += de * dz;
            by += dn * dz;
        }
        let det = sxx * syy - sxy * sxy;
        let (gx_e, gy_e) = if det.abs() > 1e-6 {
            ((syy * bx - sxy * by) / det, (sxx * by - sxy * bx) / det)
        } else {
            (0.0, 0.0)
        };
        let gx = world.grad0_e[s] + gx_e;
        let gy = world.grad0_n[s] + gy_e;
        world.grad_e[s] = gx;
        world.grad_n[s] = gy;
        world.slope[s] = (gx * gx + gy * gy).sqrt().atan();
        world.aspect[s] = gy.atan2(gx);
    }
}

/// Génesis geológica (§8): `geo_steps` pasos de `geo_dt_years` años repartidos en `geo_rounds`
/// rondas; tras cada ronda se reinicia la marca hidrológica y se vuelve a refinar el LOD.
pub fn genesis(world: &mut World, cfg: &TerrenoConfig) -> GeoSummary {
    let ec = &cfg.erosion;
    let mut summary = GeoSummary {
        before: snapshot(world),
        ..GeoSummary::default()
    };
    if ec.geo_steps == 0 {
        summary.after = summary.before;
        return summary;
    }
    let rounds = ec.geo_rounds.clamp(1, ec.geo_steps);
    for r in 0..rounds {
        let steps = ec.geo_steps / rounds + u32::from(r < ec.geo_steps % rounds);
        for _ in 0..steps {
            let (p, t) = analytic_means(world, cfg);
            let rep = step(world, cfg, ec.geo_dt_years, &p, &t);
            summary.erosion.add(&rep);
        }
        // Las marcas hidrológicas de antes de la erosión clavaban refinamiento en lagos falsos y
        // en posiciones viejas de los ríos: se reinician con la red actual y se refina de nuevo.
        world.hydro_lock.iter_mut().for_each(|x| *x = false);
        lod::update_hydro_lock(world);
        if cfg.lod.enabled() {
            summary.lod_fill_m3 += lod::refine_initial(world, cfg).fill_m3;
        }
    }
    summary.years = ec.geo_steps as f64 * ec.geo_dt_years;
    summary.after = snapshot(world);
    summary
}
