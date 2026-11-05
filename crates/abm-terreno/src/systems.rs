//! Sistemas por tick (diseño §5): clima, nieve, balance hídrico, agua superficial y vegetación,
//! más la interfaz de lectura con agentes. Orden de ejecución fijo (§5 del diseño). Todo recorre las
//! hojas en orden de clave => reducciones deterministas. Sin asignaciones por tick (buffers en
//! `Scratch`; solo se redimensionan en los pasos de topología del LOD).
//!
//! Incremento 2 (§7): la física es local por celda salvo la difusión lateral, que ahora recorre los
//! pares de vecinos multinivel con una fórmula **simétrica**.

use std::f64::consts::PI;

use crate::config::TerrenoConfig;
use crate::erosion::ErosionReport;
use crate::lod::LodReport;
use crate::world::{Biome, Surface, World};

/// Buffers de trabajo reutilizados por tick (CLAUDE §5: sin `Vec::new()` en el bucle caliente).
pub struct Scratch {
    p_liq: Vec<f64>,
    r: Vec<f64>,
    d: Vec<f64>,
    gen_vol: Vec<f64>,
    q: Vec<f64>,
}

impl Scratch {
    pub fn new(num: usize) -> Self {
        Self {
            p_liq: vec![0.0; num],
            r: vec![0.0; num],
            d: vec![0.0; num],
            gen_vol: vec![0.0; num],
            q: vec![0.0; num],
        }
    }

    /// Ajusta los buffers al nº de hojas tras un paso de topología (única asignación fuera de la
    /// génesis; nunca dentro de un tick).
    pub fn resize(&mut self, num: usize) {
        for v in [
            &mut self.p_liq,
            &mut self.r,
            &mut self.d,
            &mut self.gen_vol,
            &mut self.q,
        ] {
            v.clear();
            v.resize(num, 0.0);
        }
    }
}

/// Cierre del balance de agua global del tick (m³). Métrica de validación clave (diseño §9).
#[derive(Debug, Clone, Copy, Default)]
pub struct TickAccount {
    pub precip_vol: f64,
    pub et_vol: f64,
    pub to_ocean_vol: f64,
    /// Agua enviada al mar por el reparto del LOD en el paso de topología del tick (m³).
    pub lod_to_ocean_vol: f64,
    pub dstore_vol: f64,
    pub discharge_max: f64,
    /// Subdivisiones y colapsos del paso de topología del tick (0 si no lo hubo).
    pub lod_subdivisions: u64,
    pub lod_collapses: u64,
    /// Acondicionamiento del paso de topología: material movido y lo no compensado (m³).
    pub lod_conditioned_m3: f64,
    pub lod_fill_m3: f64,
    /// Paso de erosión del tick (vacío si no lo hubo; incr. 3 §9). No mueve agua.
    pub erosion: ErosionReport,
}

impl TickAccount {
    /// Residual del balance: `entrada − evaporación − salida al mar − lod_al_mar − Δalmacenamiento`.
    /// Debe ≈ 0. El almacenamiento previo se mide antes del paso de topología (incremento 2 §6.4).
    pub fn residual(&self) -> f64 {
        self.precip_vol - self.et_vol - self.to_ocean_vol - self.lod_to_ocean_vol - self.dstore_vol
    }
}

/// Precipitación base por franja de latitud (ITCZ húmeda, subtrópicos secos, templada húmeda, polos secos).
fn precip_band(abslat_deg: f64) -> f64 {
    let l = abslat_deg;
    let eq = 0.9 * (-(l / 12.0).powi(2)).exp();
    let mid = 0.6 * (-((l - 52.0) / 15.0).powi(2)).exp();
    (eq + mid + 0.08).clamp(0.02, 1.2)
}

/// Componente este del viento dominante por franja (alisios −, oestes +, polar −). Norte = 0.
pub(crate) fn wind_east(abslat_deg: f64) -> f64 {
    if abslat_deg < 30.0 {
        -1.0
    } else if abslat_deg < 60.0 {
        1.0
    } else {
        -1.0
    }
}

/// Bioma de Whittaker a partir de la temperatura media (°C) y la precipitación anual (mm/año).
pub(crate) fn whittaker(t: f64, p_annual: f64) -> Biome {
    if t < -10.0 {
        Biome::Hielo
    } else if t < -2.0 {
        Biome::Tundra
    } else if t < 4.0 {
        if p_annual < 200.0 {
            Biome::Tundra
        } else {
            Biome::Taiga
        }
    } else if t < 13.0 {
        if p_annual < 250.0 {
            Biome::Desierto
        } else if p_annual < 600.0 {
            Biome::Pradera
        } else {
            Biome::BosqueTemplado
        }
    } else if p_annual < 250.0 {
        Biome::Desierto
    } else if p_annual < 700.0 {
        Biome::Sabana
    } else if p_annual < 1400.0 {
        Biome::BosqueTemplado
    } else {
        Biome::Selva
    }
}

/// Ejecuta un tick completo y devuelve el balance de agua.
///
/// `store_before` es el almacenamiento (suelo + nieve) medido **antes** del paso de topología del
/// tick, y `lod` el resumen de ese paso (vacío si no lo hubo): así el residual cubre también la
/// conservación del LOD (incremento 2 §6.4).
pub fn step(
    world: &mut World,
    cfg: &TerrenoConfig,
    sc: &mut Scratch,
    t: u64,
    store_before: f64,
    lod: &LodReport,
) -> TickAccount {
    climate(world, cfg, t);
    snow(world, cfg, sc);
    let et_vol = water_balance(world, cfg, sc);
    lateral_diffusion(world, cfg);
    let (to_ocean_vol, discharge_max) = surface_water(world, sc);

    // Acumuladores de medias (para idoneidad y biomas).
    for id in 0..world.topo.num_cells {
        world.temp_sum[id] += world.temp[id];
        world.precip_sum[id] += world.precip[id];
    }
    world.sample_count += 1;

    if t.is_multiple_of(cfg.vegetation.n_veg) {
        vegetation(world, cfg);
    }

    // Entrada = precipitación total sobre tierra (mm → m³).
    let mut precip_vol = 0.0;
    for id in 0..world.topo.num_cells {
        if !matches!(world.surface[id], Surface::Oceano) {
            precip_vol += world.precip[id] / 1000.0 * world.topo.area[id];
        }
    }
    let store_after = total_store_vol(world);

    TickAccount {
        precip_vol,
        et_vol,
        to_ocean_vol,
        lod_to_ocean_vol: lod.to_ocean_vol,
        dstore_vol: store_after - store_before,
        discharge_max,
        lod_subdivisions: lod.subdivisions,
        lod_collapses: lod.collapses,
        lod_conditioned_m3: lod.conditioned_m3,
        lod_fill_m3: lod.fill_m3,
        erosion: ErosionReport::default(),
    }
}

/// Volumen total almacenado en tierra: agua del suelo + nieve (m³).
pub fn total_store_vol(world: &World) -> f64 {
    let mut v = 0.0;
    for id in 0..world.topo.num_cells {
        if !matches!(world.surface[id], Surface::Oceano) {
            v += (world.w[id] + world.swe[id]) / 1000.0 * world.topo.area[id];
        }
    }
    v
}

/// Temperatura media anual (°C) de una celda: latitud + altura (sin el término estacional, que
/// promedia a cero en un año). La usan el clima del tick y la erosión (incr. 3 §7).
pub(crate) fn temp_annual_mean(cfg: &TerrenoConfig, lat_rad: f64, h: f64, ocean: bool) -> f64 {
    let c = &cfg.climate;
    let abslat = lat_rad.to_degrees().abs();
    let land_h = if ocean {
        0.0
    } else {
        ((h - cfg.planet.sea_level) / 1000.0).max(0.0)
    };
    c.t_equator - c.k_lat * abslat - c.lapse * land_h
}

/// Precipitación por tick (mm) sin el factor estacional: franja · humedad · orografía. Como el factor
/// estacional promedia a 1 (si `seas_precip_amp ≤ 1`), es también la media del año (incr. 3 §4).
pub(crate) fn precip_no_season(
    cfg: &TerrenoConfig,
    lat_rad: f64,
    dist_km: f64,
    grad_e: f64,
) -> f64 {
    let c = &cfg.climate;
    let abslat = lat_rad.to_degrees().abs();
    let p_base = c.p_ref * precip_band(abslat);
    let m_water = (-dist_km / c.moisture_length_km).exp();
    let we = wind_east(abslat);
    let grad_dot = grad_e * we; // viento norte = 0
    let m_oro = (1.0 + c.k_oro * grad_dot).clamp(c.m_oro_min, c.m_oro_max);
    p_base * m_water * m_oro
}

/// Pendiente este que "ve" la lluvia (nota de lluvia orográfica): la del relieve de referencia a
/// escala `d_oro` en las celdas tan finas como `d_oro` o más; en las más gruesas (o con la
/// corrección apagada, `d_oro = 0`), la propia de la celda, como hasta el incremento 3.
///
/// Con `climate.mar_plano` (nota de sombra pluviométrica §6) el aire ve el mar plano: sobre el mar
/// no hay pendiente, y en la costa se mide desde el nivel del mar y no desde el fondo marino. La
/// parte de la erosión (el mar no se erosiona) es la misma.
#[inline]
pub(crate) fn oro_slope_e(world: &World, cfg: &TerrenoConfig, s: usize) -> f64 {
    let referencia = world
        .statgen
        .uses_oro_reference(u32::from(world.topo.level[s]));
    if !cfg.climate.mar_plano {
        return if referencia {
            world.oro_e[s]
        } else {
            world.grad_e[s]
        };
    }
    if world.es_oceano(s) {
        return 0.0;
    }
    let erosion = if referencia {
        world.oro_e[s] - world.oro0_e[s]
    } else {
        world.grad_e[s] - world.grad0_e[s]
    };
    world.oro_air0_e[s] + erosion
}

/// Precipitación media por tick (mm) de una celda, sin el factor estacional: franja · humedad ·
/// orografía y, en tierra, el agotamiento `w` y el factor global `c_P` (nota de sombra
/// pluviométrica). Sin agotamiento ni corrección, los dos valen 1 y la lluvia es la de antes, bit a
/// bit.
#[inline]
pub(crate) fn precip_mean_cell(world: &World, cfg: &TerrenoConfig, s: usize) -> f64 {
    let p = precip_no_season(
        cfg,
        world.topo.lat[s],
        world.dist_ocean_km[s],
        oro_slope_e(world, cfg, s),
    );
    if world.es_oceano(s) {
        p
    } else {
        p * (world.c_p * world.w_hum[s])
    }
}

/// Factor orográfico `m_oro` de una celda (para medidas y volcados).
pub fn m_oro_celda(world: &World, cfg: &TerrenoConfig, s: usize) -> f64 {
    let c = &cfg.climate;
    let we = wind_east(world.topo.lat[s].to_degrees().abs());
    (1.0 + c.k_oro * oro_slope_e(world, cfg, s) * we).clamp(c.m_oro_min, c.m_oro_max)
}

/// Lluvia media anual (mm/año) de una celda con el estado actual (para medidas y volcados).
pub fn lluvia_media_anual_mm(world: &World, cfg: &TerrenoConfig, s: usize) -> f64 {
    precip_mean_cell(world, cfg, s).max(0.0) * cfg.climate.t_year
}

/// 6.2 Clima: temperatura (latitud + altura + estación) y precipitación (franja · humedad · orografía).
fn climate(world: &mut World, cfg: &TerrenoConfig, t: u64) {
    let c = &cfg.climate;
    let phase = 2.0 * PI * t as f64 / c.t_year;
    let (sin_p, _) = phase.sin_cos();
    for id in 0..world.topo.num_cells {
        let lat = world.topo.lat[id];
        let lat_deg = lat.to_degrees();
        let abslat = lat_deg.abs();
        let ocean = matches!(world.surface[id], Surface::Oceano);
        let t_media = temp_annual_mean(cfg, lat, world.h[id], ocean);
        let a_est = c.seas_amp_pole * (abslat / 90.0);
        let s = if lat_deg >= 0.0 { 1.0 } else { -1.0 };
        world.temp[id] = t_media + s * a_est * sin_p;

        let p = precip_mean_cell(world, cfg, id);
        let m_est = (1.0 + c.seas_precip_amp * s * sin_p).max(0.0);
        world.precip[id] = (p * m_est).max(0.0);
    }
}

/// 6.3 Nieve: partición lluvia/nieve por `t_freeze` y fusión por grados-día. Produce `p_liq`.
fn snow(world: &mut World, cfg: &TerrenoConfig, sc: &mut Scratch) {
    let c = &cfg.climate;
    for id in 0..world.topo.num_cells {
        if matches!(world.surface[id], Surface::Oceano) {
            sc.p_liq[id] = 0.0;
            continue;
        }
        let temp = world.temp[id];
        if temp < c.t_freeze {
            world.swe[id] += world.precip[id];
            sc.p_liq[id] = 0.0;
        } else {
            let mut p_liq = world.precip[id];
            if world.swe[id] > 0.0 {
                let melt = (c.k_melt * (temp - c.t_freeze)).min(world.swe[id]);
                world.swe[id] -= melt;
                p_liq += melt;
            }
            sc.p_liq[id] = p_liq;
        }
    }
}

/// 6.4 Balance hídrico (modelo de cubo). Devuelve la evapotranspiración total sobre tierra (m³).
fn water_balance(world: &mut World, cfg: &TerrenoConfig, sc: &mut Scratch) -> f64 {
    let s = &cfg.soil;
    let mut et_vol = 0.0;
    for id in 0..world.topo.num_cells {
        if matches!(world.surface[id], Surface::Oceano) {
            sc.r[id] = 0.0;
            sc.d[id] = 0.0;
            sc.gen_vol[id] = 0.0;
            continue;
        }
        let theta = world.theta(id);
        let c_esc =
            (s.runoff_base + s.k_slope * world.slope[id].tan() + s.k_sat * theta).clamp(0.0, 1.0);
        let p_liq = sc.p_liq[id];
        let r = p_liq * c_esc;
        let p_inf = p_liq - r;
        let pet = s.k_et * world.temp[id].max(0.0);
        let et = pet.min(world.w[id] + p_inf);
        let w_temp = world.w[id] + p_inf - et;
        let d = (w_temp - world.w_cap[id]).max(0.0);
        world.w[id] = (w_temp - d).clamp(0.0, world.w_cap[id]);
        sc.r[id] = r;
        sc.d[id] = d;
        sc.gen_vol[id] = (r + d) / 1000.0 * world.topo.area[id]; // m³ de escorrentía+drenaje
        et_vol += et / 1000.0 * world.topo.area[id];
    }
    et_vol
}

/// Flujo lateral de humedad (difusión casi isótropa) sobre los pares de vecinos (incremento 2 §7).
///
/// **Simétrica**: el flujo de volumen entre `i` y `j` usa la media armónica de `W_cap·A` de las dos
/// celdas, así que intercambiar `i` y `j` da el mismo flujo con signo opuesto (también entre celdas
/// de niveles distintos). **Exactamente conservativa**: se mueve volumen, con un limitador que no
/// vacía al donante ni rebasa la capacidad del receptor. Orden fijo de pares => determinista.
fn lateral_diffusion(world: &mut World, cfg: &TerrenoConfig) {
    let k = cfg.soil.k_lat_flow;
    if k <= 0.0 {
        return;
    }
    for p in world.topo.pairs.iter() {
        let (i, j) = (p.a as usize, p.b as usize);
        // Solo entre celdas de tierra (el océano no tiene agua del suelo).
        if matches!(world.surface[i], Surface::Oceano)
            || matches!(world.surface[j], Surface::Oceano)
        {
            continue;
        }
        let (ai, aj) = (world.topo.area[i], world.topo.area[j]);
        let (xi, xj) = (world.w_cap[i] * ai, world.w_cap[j] * aj);
        let hm = if xi + xj > 0.0 {
            2.0 * xi * xj / (xi + xj)
        } else {
            0.0
        };
        // `q` = volumen (mm·m²) que pasa de i a j (con signo).
        let q = k * p.g * (world.theta(i) - world.theta(j)) * hm;
        let q = if q >= 0.0 {
            q.min(world.w[i].max(0.0) * ai)
                .min((world.w_cap[j] - world.w[j]).max(0.0) * aj)
        } else {
            -(-q)
                .min(world.w[j].max(0.0) * aj)
                .min((world.w_cap[i] - world.w[i]).max(0.0) * ai)
        };
        world.w[i] -= q / ai;
        world.w[j] += q / aj;
    }
}

/// 6.5 Agua superficial: enruta escorrentía+drenaje por la red hasta el mar. Devuelve (vol al mar, caudal máx).
fn surface_water(world: &mut World, sc: &mut Scratch) -> (f64, f64) {
    sc.q.copy_from_slice(&sc.gen_vol);
    // Acumula aguas arriba→abajo; el receptor de una celda es aguas abajo (garantizado por el flood).
    for &id in world.flow_order.iter() {
        let r = world.receiver[id as usize];
        if r >= 0 {
            sc.q[r as usize] += sc.q[id as usize];
        }
    }
    // Todo el volumen generado en tierra sale al mar (sin almacenamiento superficial en este incremento).
    let mut to_ocean = 0.0;
    let mut discharge_max = 0.0;
    for id in 0..world.topo.num_cells {
        if !matches!(world.surface[id], Surface::Oceano) {
            to_ocean += sc.gen_vol[id];
            if sc.q[id] > discharge_max {
                discharge_max = sc.q[id];
            }
        }
    }
    (to_ocean, discharge_max)
}

/// 6.6 Vegetación: idoneidad (campanas) → capacidad de carga → logístico; bioma de Whittaker.
fn vegetation(world: &mut World, cfg: &TerrenoConfig) {
    let v = &cfg.vegetation;
    let n = v.n_veg as f64;
    let count = world.sample_count.max(1) as f64;
    for id in 0..world.topo.num_cells {
        if matches!(world.surface[id], Surface::Oceano) {
            continue;
        }
        let tbar = world.temp_sum[id] / count;
        let theta = world.theta(id);
        let f_hum = (-((theta - v.theta_opt) / v.sigma_theta).powi(2)).exp();
        let f_temp = (-((tbar - v.t_opt) / v.sigma_t).powi(2)).exp();
        let suit = f_hum * f_temp * world.fert[id];
        let k = v.k_max * suit;
        if k > 1e-6 {
            world.veg[id] += v.r_veg * world.veg[id] * (1.0 - world.veg[id] / k) * n;
        } else {
            world.veg[id] -= v.r_veg * world.veg[id] * n;
        }
        world.veg[id] = world.veg[id].max(0.0);

        let p_annual = world.precip_sum[id] / count * cfg.climate.t_year;
        world.biome[id] = whittaker(tbar, p_annual);
    }
}

// --- 6.9 Interfaz de lectura con agentes (calculada bajo demanda; sin escritura en este incremento) ---

/// Coste de movimiento de una celda (pendiente, barro y vegetación la encarecen).
pub fn coste_movimiento(world: &World, id: usize) -> f64 {
    if world.es_agua(id) {
        return f64::INFINITY;
    }
    let s = world.slope[id].tan();
    let theta = world.theta(id);
    let vrel = (world.veg[id] / 100.0).clamp(0.0, 1.0);
    1.0 * (1.0 + 1.0 * s) * (1.0 + 0.5 * theta) * (1.0 + 0.3 * vrel)
}

/// Habitabilidad de una celda en `[0, 1]` (humedad · temperatura · agua disponible).
pub fn habitabilidad(world: &World, cfg: &TerrenoConfig, id: usize) -> f64 {
    if world.es_agua(id) {
        return 0.0;
    }
    let v = &cfg.vegetation;
    let theta = world.theta(id);
    let f_hum = (-((theta - v.theta_opt) / v.sigma_theta).powi(2)).exp();
    let f_temp = (-((world.temp[id] - v.t_opt) / v.sigma_t).powi(2)).exp();
    let g_agua = (agua_disponible(world, id)).clamp(0.0, 1.0);
    f_hum * f_temp * g_agua
}

/// Agua disponible: humedad del suelo + bonus por río/lago adyacente.
pub fn agua_disponible(world: &World, id: usize) -> f64 {
    let mut a = world.theta(id);
    for &nb in world.topo.neighbors(id) {
        match world.surface[nb as usize] {
            Surface::Rio => a += 0.5,
            Surface::Lago => a += 0.5,
            _ => {}
        }
    }
    a
}

/// Capacidad de alimento: biomasa vegetal (o recurso extraíble).
pub fn capacidad_alimento(world: &World, id: usize) -> f64 {
    world.veg[id].max(world.resource[id])
}
