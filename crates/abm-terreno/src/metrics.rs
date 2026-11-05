//! Métricas agregadas por tick (diseño §8) y esquema serde de la salida. Structs tipados (orden de
//! campos estable) => JSON byte-comparable entre corridas, igual que el motor de forrajeo.
//!
//! Esquema v2 (incremento 2): nº de hojas y hojas por nivel, actividad del LOD (subdivisiones,
//! colapsos, agua enviada al mar por el reparto), área de tierra, y longitud de ríos con el lado
//! real de cada celda.
//!
//! Esquema v3 (incremento 3): profundidad media del suelo, balance de material de cada paso de
//! erosión (erosionado, depositado, al mar, residual) y resumen de la génesis geológica.
//!
//! Esquema v4: acondicionamiento hidrológico del paso de topología (material movido y no compensado).
//!
//! Esquema v5: diagnóstico del planeta al final de la corrida (relieve, lluvia y PPN), para que las
//! cifras que sostienen las decisiones se puedan consultar en cada corrida (CLAUDE §5).

use serde::{Deserialize, Serialize};

use crate::config::TerrenoConfig;
use crate::erosion::GeoSummary;
use crate::systems::{self, TickAccount};
use crate::world::{Biome, Surface, World};

/// Nº de franjas de latitud para la temperatura media por banda (cada una de 15°).
pub const N_BANDS: usize = 6;

/// Versión del esquema del documento de salida.
pub const SCHEMA_VERSION: u32 = 5;

/// Límites inferiores de las franjas de altura de la hipsometría (m sobre el nivel del mar).
pub const HIPSOMETRIA_LIMITES_M: [f64; 7] = [0.0, 200.0, 500.0, 1000.0, 2000.0, 3000.0, 5000.0];

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
    /// Nº de hojas activas (celdas) y su reparto por nivel (índice = nivel).
    pub num_cells: u64,
    pub cells_by_level: Vec<u64>,
    pub temp_mean: f64,
    pub temp_by_band: Vec<f64>,
    pub precip_total_m3: f64,
    pub theta_mean: f64,
    pub elev_mean_land: f64,
    pub soil_depth_mean_m: f64,
    pub land_area_m2: f64,
    pub snow_cover_m2: f64,
    pub river_cells: u64,
    pub lake_cells: u64,
    pub river_length_km: f64,
    pub discharge_max_m3: f64,
    /// Actividad del paso de topología del tick (0 si no lo hubo).
    pub lod_subdivisions: u64,
    pub lod_collapses: u64,
    pub lod_to_ocean_m3: f64,
    pub lod_conditioned_m3: f64,
    pub lod_fill_m3: f64,
    /// Paso de erosión del tick (0 si no lo hubo): material arrancado, depositado, al mar y residual.
    pub eroded_m3: f64,
    pub deposited_m3: f64,
    pub sediment_to_ocean_m3: f64,
    pub sediment_residual_m3: f64,
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
    /// Celdas por arista de cara en el nivel 0, y niveles del árbol LOD.
    pub n0: u32,
    pub l_min: u32,
    pub l_max: u32,
    /// Nº de hojas tras la génesis (incluido el refinamiento inicial).
    pub num_cells: u64,
    pub r_km: f64,
    pub total_ticks: u64,
    /// Génesis geológica (incr. 3 §8): años simulados, balance de material y relieve antes/después.
    pub genesis: GeoSummary,
    /// Cuánto desvían la orografía y el agotamiento de humedad la lluvia total sobre tierra, y el
    /// factor `c_P` que la corrige (nota de sombra pluviométrica).
    pub lluvia: LluviaMeta,
}

/// Lluvia total sobre tierra (nota de sombra pluviométrica §2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LluviaMeta {
    /// Desviación relativa de la lluvia media anual sobre tierra frente a la de referencia (sin
    /// efecto orográfico ni agotamiento), antes de aplicar `c_P`.
    pub desvio: f64,
    /// Factor global aplicado a la lluvia de la tierra (1 = sin corregir).
    pub c_p: f64,
}

/// Diagnóstico del planeta al final de la corrida: las cifras con las que se compara con la Tierra
/// (propuesta de relieve realista) y que sostienen las decisiones sobre la lluvia. Medias por área.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticoPlaneta {
    /// Fracción de la superficie del planeta que no es océano (los lagos cuentan como tierra).
    pub tierra_frac: f64,
    /// Altura media de la tierra sobre el nivel del mar (m).
    pub altura_media_tierra_m: f64,
    /// Altura mediana de la tierra (m): la mitad del área está por debajo.
    pub altura_mediana_tierra_m: f64,
    /// Límites inferiores de las franjas de altura (m): la franja `i` va de `limites[i]` a
    /// `limites[i + 1]` y la última no tiene techo. La tierra bajo el nivel del mar va en la primera.
    pub hipsometria_limites_m: Vec<f64>,
    /// Fracción de la tierra en cada franja.
    pub hipsometria_frac: Vec<f64>,
    /// Profundidad media del océano (m, positiva).
    pub profundidad_media_oceano_m: f64,
    /// Lluvia media anual sobre la tierra (mm/año), con el clima del final de la corrida.
    pub lluvia_tierra_mm: f64,
    /// Lluvia media anual sobre el océano (mm/año).
    pub lluvia_oceano_mm: f64,
    /// Fracción de la tierra donde el aire llega con menos de la mitad de su vapor (`w` < 0,5).
    pub tierra_w_bajo_frac: f64,
    /// PPN media de la tierra con el modelo de Miami (g/m²/año de materia seca), con la temperatura y
    /// la lluvia medias anuales de cada celda. Es la base del forraje de los agentes.
    pub ppn_miami_tierra: f64,
}

/// Documento completo de salida.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrenoDoc {
    pub schema_version: u32,
    pub run: TerrenoRunMeta,
    pub diagnostico: DiagnosticoPlaneta,
    pub ticks: Vec<TerrenoTickMetrics>,
}

/// PPN del modelo de Miami (Lieth, 1975), g/m²/año: el mínimo del límite por temperatura (°C) y del
/// límite por lluvia (mm/año). Solo para el diagnóstico; el forraje de los agentes usa la misma
/// fórmula en su hábitat de prueba (`abm-ruleset`), y se unirán cuando pasen al planeta.
fn ppn_miami(t_c: f64, p_mm: f64) -> f64 {
    let f_t = 3000.0 / (1.0 + (1.315 - 0.119 * t_c).exp());
    let f_p = 3000.0 * (1.0 - (-0.000664 * p_mm).exp());
    f_t.min(f_p).max(0.0)
}

/// Calcula el diagnóstico del planeta con el estado final del mundo. Sumas en orden de celda
/// (deterministas); se llama una vez, al final, así que puede reservar memoria.
pub fn diagnostico(world: &World, cfg: &TerrenoConfig) -> DiagnosticoPlaneta {
    let mar = cfg.planet.sea_level;
    let limites = HIPSOMETRIA_LIMITES_M;
    let mut franjas = [0.0; HIPSOMETRIA_LIMITES_M.len()];
    let (mut a_total, mut a_tierra, mut a_mar) = (0.0, 0.0, 0.0);
    let (mut h_tierra, mut prof_mar, mut lluvia_tierra, mut lluvia_mar) = (0.0, 0.0, 0.0, 0.0);
    let (mut w_bajo, mut ppn) = (0.0, 0.0);
    let mut alturas: Vec<(f64, f64)> = Vec::new();
    for s in 0..world.topo.num_cells {
        let a = world.topo.area[s];
        let h = world.h[s] - mar;
        let lluvia = systems::lluvia_media_anual_mm(world, cfg, s);
        a_total += a;
        if world.es_oceano(s) {
            a_mar += a;
            prof_mar += -h * a;
            lluvia_mar += lluvia * a;
            continue;
        }
        a_tierra += a;
        h_tierra += h * a;
        lluvia_tierra += lluvia * a;
        alturas.push((h, a));
        let franja = limites.iter().rposition(|&l| h >= l).unwrap_or(0);
        franjas[franja] += a;
        if world.w_hum[s] < 0.5 {
            w_bajo += a;
        }
        let t_media = systems::temp_annual_mean(cfg, world.topo.lat[s], world.h[s], false);
        ppn += ppn_miami(t_media, lluvia) * a;
    }
    alturas.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut acumulada = 0.0;
    let mediana = alturas
        .iter()
        .find(|&&(_, a)| {
            acumulada += a;
            acumulada >= 0.5 * a_tierra
        })
        .map_or(0.0, |&(h, _)| h);
    let por = |x: f64, a: f64| if a > 0.0 { x / a } else { 0.0 };
    DiagnosticoPlaneta {
        tierra_frac: por(a_tierra, a_total),
        altura_media_tierra_m: por(h_tierra, a_tierra),
        altura_mediana_tierra_m: mediana,
        hipsometria_limites_m: limites.to_vec(),
        hipsometria_frac: franjas.iter().map(|&x| por(x, a_tierra)).collect(),
        profundidad_media_oceano_m: por(prof_mar, a_mar),
        lluvia_tierra_mm: por(lluvia_tierra, a_tierra),
        lluvia_oceano_mm: por(lluvia_mar, a_mar),
        tierra_w_bajo_frac: por(w_bajo, a_tierra),
        ppn_miami_tierra: por(ppn, a_tierra),
    }
}

/// Calcula las métricas del tick actual. Reducciones en orden de clave (deterministas).
pub fn compute(world: &World, acct: &TickAccount, t: u64) -> TerrenoTickMetrics {
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
    let mut soil_area = 0.0;
    let mut snow_cover = 0.0;
    let mut river_cells = 0u64;
    let mut lake_cells = 0u64;
    let mut river_length_km = 0.0;
    let mut biome_m2 = [0.0_f64; Biome::ALL.len()];
    for id in 0..num {
        let a = world.topo.area[id];
        precip_total += world.precip[id] / 1000.0 * a;
        if !matches!(world.surface[id], Surface::Oceano) {
            theta_area += world.theta(id) * a;
            elev_area += world.h[id] * a;
            soil_area += world.z_soil[id] * a;
            land_area += a;
            if world.swe[id] > 0.0 {
                snow_cover += a;
            }
        }
        match world.surface[id] {
            Surface::Rio => {
                river_cells += 1;
                river_length_km += world.topo.side_km(id);
            }
            Surface::Lago => lake_cells += 1,
            _ => {}
        }
        biome_m2[world.biome[id].index()] += a;
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
    let soil_depth_mean_m = if land_area > 0.0 {
        soil_area / land_area
    } else {
        0.0
    };

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
        num_cells: num as u64,
        cells_by_level: world.topo.cells_by_level(),
        temp_mean,
        temp_by_band,
        precip_total_m3: precip_total,
        theta_mean,
        elev_mean_land,
        soil_depth_mean_m,
        land_area_m2: land_area,
        snow_cover_m2: snow_cover,
        river_cells,
        lake_cells,
        river_length_km,
        discharge_max_m3: acct.discharge_max,
        lod_subdivisions: acct.lod_subdivisions,
        lod_collapses: acct.lod_collapses,
        lod_to_ocean_m3: acct.lod_to_ocean_vol,
        lod_conditioned_m3: acct.lod_conditioned_m3,
        lod_fill_m3: acct.lod_fill_m3,
        eroded_m3: acct.erosion.eroded_m3,
        deposited_m3: acct.erosion.deposited_m3,
        sediment_to_ocean_m3: acct.erosion.to_ocean_m3,
        sediment_residual_m3: acct.erosion.residual_m3,
        water_residual_m3: residual,
        water_residual_rel,
        biome_area,
    }
}
