//! Métricas agregadas por tick (diseño §8) y esquema serde de la salida. Structs tipados (orden de
//! campos estable) => JSON byte-comparable entre corridas, igual que el motor de forrajeo.

use serde::{Deserialize, Serialize};

use crate::terreno::config::TerrenoConfig;
use crate::terreno::systems::TickAccount;
use crate::terreno::world::{Biome, Surface, World};

/// Nº de franjas de latitud para la temperatura media por banda (cada una de 15°).
pub const N_BANDS: usize = 6;

/// Área por bioma (m²), con nombre estable para claves de salida.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BiomeArea {
    pub biome: String,
    pub area_m2: f64,
}

/// Métricas agregadas de un tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrenoTickMetrics {
    pub tick: u64,
    pub temp_mean: f64,
    pub temp_by_band: Vec<f64>,
    pub precip_total_m3: f64,
    pub theta_mean: f64,
    pub elev_mean_land: f64,
    pub snow_cover_m2: f64,
    pub river_cells: u64,
    pub lake_cells: u64,
    pub river_length_km: f64,
    pub discharge_max_m3: f64,
    pub water_residual_m3: f64,
    pub water_residual_rel: f64,
    pub biome_area: Vec<BiomeArea>,
}

/// Metadatos de la ejecución de terreno.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrenoRunMeta {
    pub name: String,
    pub seed: u64,
    pub engine_version: String,
    pub face_n: u32,
    pub num_cells: u64,
    pub r_km: f64,
    pub total_ticks: u64,
}

/// Documento completo de salida.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrenoDoc {
    pub schema_version: u32,
    pub run: TerrenoRunMeta,
    pub ticks: Vec<TerrenoTickMetrics>,
}

/// Calcula las métricas del tick actual. Reducciones en orden de id (deterministas).
pub fn compute(
    world: &World,
    cfg: &TerrenoConfig,
    acct: &TickAccount,
    t: u64,
) -> TerrenoTickMetrics {
    let num = world.topo.num_cells;

    // Temperatura media global (ponderada por área) y por franja de latitud.
    let mut temp_area = 0.0;
    let mut area_total = 0.0;
    let mut band_temp = [0.0; N_BANDS];
    let mut band_area = [0.0; N_BANDS];
    for id in 0..num {
        let a = world.topo.area[id];
        temp_area += world.temp[id] * a;
        area_total += a;
        let abslat = world.topo.lat[id].to_degrees().abs();
        let b = ((abslat / 15.0).floor() as usize).min(N_BANDS - 1);
        band_temp[b] += world.temp[id] * a;
        band_area[b] += a;
    }
    let temp_mean = if area_total > 0.0 {
        temp_area / area_total
    } else {
        0.0
    };
    let temp_by_band: Vec<f64> = band_temp
        .iter()
        .zip(band_area.iter())
        .map(|(t, a)| if *a > 0.0 { t / a } else { 0.0 })
        .collect();

    // Agregados sobre tierra + conteos de superficie.
    let mut precip_total = 0.0;
    let mut theta_area = 0.0;
    let mut land_area = 0.0;
    let mut elev_area = 0.0;
    let mut snow_cover = 0.0;
    let mut river_cells = 0u64;
    let mut lake_cells = 0u64;
    let mut biome_m2 = [0.0_f64; Biome::ALL.len()];
    for id in 0..num {
        let a = world.topo.area[id];
        precip_total += world.precip[id] / 1000.0 * a;
        if !matches!(world.surface[id], Surface::Oceano) {
            theta_area += world.theta(id) * a;
            elev_area += world.h[id] * a;
            land_area += a;
            if world.swe[id] > 0.0 {
                snow_cover += a;
            }
        }
        match world.surface[id] {
            Surface::Rio => river_cells += 1,
            Surface::Lago => lake_cells += 1,
            _ => {}
        }
        let bi = Biome::ALL
            .iter()
            .position(|b| *b == world.biome[id])
            .unwrap();
        biome_m2[bi] += a;
    }
    let theta_mean = if land_area > 0.0 {
        theta_area / land_area
    } else {
        0.0
    };
    let elev_mean_land = if land_area > 0.0 {
        elev_area / land_area
    } else {
        0.0
    };
    let cell_side_km = (std::f64::consts::FRAC_PI_2 * cfg.planet.r_km) / world.topo.n as f64;

    let biome_area = Biome::ALL
        .iter()
        .enumerate()
        .map(|(i, b)| BiomeArea {
            biome: b.key().to_string(),
            area_m2: biome_m2[i],
        })
        .collect();

    let residual = acct.residual();
    let water_residual_rel = if acct.precip_vol.abs() > 0.0 {
        residual / acct.precip_vol
    } else {
        0.0
    };

    TerrenoTickMetrics {
        tick: t,
        temp_mean,
        temp_by_band,
        precip_total_m3: precip_total,
        theta_mean,
        elev_mean_land,
        snow_cover_m2: snow_cover,
        river_cells,
        lake_cells,
        river_length_km: river_cells as f64 * cell_side_km,
        discharge_max_m3: acct.discharge_max,
        water_residual_m3: residual,
        water_residual_rel,
        biome_area,
    }
}
