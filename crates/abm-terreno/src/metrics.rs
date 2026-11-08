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
//!
//! Esquema v6 (relieve tectónico, incremento 4): la historia geológica de la génesis en los
//! metadatos y el diagnóstico ampliado del relieve (océano, pendientes a 25 km, plataforma, ley de
//! Hack y espectro por escala), para compararlo con las tablas de la Tierra
//! (`config/referencias/tierra_relieve.toml`). Con el relieve de ruido y sin
//! `run.diagnostico_relieve`, la salida es la v5 sin cambios.

use serde::{Deserialize, Serialize};

use crate::config::TerrenoConfig;
use crate::erosion::GeoSummary;
use crate::systems::{self, TickAccount};
use crate::world::{Biome, Surface, World};

/// Nº de franjas de latitud para la temperatura media por banda (cada una de 15°).
pub const N_BANDS: usize = 6;

/// Versión del esquema del documento de salida.
pub const SCHEMA_VERSION: u32 = 5;

/// Versión del esquema con el relieve tectónico o el diagnóstico ampliado del relieve.
pub const SCHEMA_VERSION_RELIEVE: u32 = 7;

/// Versión del esquema del modo Tierra (`run.tierra` y `diagnostico.tierra`).
pub const SCHEMA_VERSION_TIERRA: u32 = 8;

/// Franjas de profundidad del océano (m, positivas) del diagnóstico ampliado.
pub const HIPSOMETRIA_OCEANO_LIMITES_M: [f64; 8] =
    [0.0, 200.0, 1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0];

/// Clases de altura (m) de las pendientes a 25 km.
pub const PENDIENTES_SOBRE_M: [f64; 4] = [0.0, 500.0, 1000.0, 2000.0];

/// Clases del espectro por escala: tierra por altura media (m) y mar por profundidad media (m).
pub const ESPECTRO_TIERRA_M: [f64; 6] = [0.0, 200.0, 500.0, 1000.0, 2000.0, 3000.0];
pub const ESPECTRO_MAR_M: [f64; 4] = [0.0, 200.0, 2000.0, 4000.0];

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
    /// Historia geológica de la génesis tectónica (esquema v6; solo con el relieve tectónico).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tectonica: Option<crate::tectonica::historia::ResumenTectonica>,
    /// Modo Tierra (esquema v8): fuentes, parámetros y el año real de cada año simulado.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tierra: Option<TierraMeta>,
}

/// Año real que usa un año simulado (evento del §7 de la nota del modo Tierra).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnioReal {
    pub anio_simulado: u64,
    pub anio_real: i32,
}

/// Metadatos del modo Tierra.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TierraMeta {
    pub fuentes: Vec<String>,
    pub nivel_carga: u32,
    pub l_bloque: u32,
    pub r_max: f64,
    pub periodo_base: [i32; 2],
    pub anio_forzado: i32,
    /// Años y celdas de tierra de CRU TS, y meses de lluvia recortados a 6 553,5 mm (deberían ser 0).
    pub cru_anios: [i32; 2],
    pub cru_celdas: u64,
    pub cru_pre_recortados: u64,
    /// Por década de CRU TS: `[primer año, fracción del área de tierra sin estaciones en la
    /// temperatura, en la lluvia]`. Allí CRU TS vuelve a su climatología: sin variabilidad propia.
    pub cru_sin_estaciones_por_decada: Vec<[f64; 3]>,
    pub anios: Vec<AnioReal>,
}

/// Cuenca que llega al mar: su desembocadura (la celda de tierra que vierte en el mar) y su área.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cuenca {
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub area_km2: f64,
}

/// Diagnóstico del modo Tierra (esquema v8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticoTierra {
    /// Fracción con hielo de toda la superficie y de la tierra.
    pub hielo_frac: f64,
    pub hielo_tierra_frac: f64,
    /// Climatología del periodo base sobre la tierra: temperatura media (°C) y lluvia anual (mm).
    pub temp_tierra_climatologia_c: f64,
    pub lluvia_tierra_climatologia_mm: f64,
    /// Lo mismo con el clima de la corrida (los años reales que tocaron), si dura años enteros.
    pub temp_tierra_corrida_c: f64,
    pub lluvia_tierra_corrida_mm: f64,
    /// Fracción del área de las celdas con dato de CHELSA (el resto, al norte de 84° N, toma el
    /// clima de su meridiano).
    pub cobertura_chelsa_frac: f64,
    /// Área de los lagos del motor (km²).
    pub lagos_km2: f64,
    /// Conservación del relieve: mayor diferencia relativa de `Σ z·A` entre un nivel de las tablas y el
    /// de carga, y entre la malla activa (niveles mezclados) y el de carga (debe ser de redondeo).
    pub conservacion_niveles_rel: f64,
    pub conservacion_malla_rel: f64,
    /// Las 50 mayores cuencas que llegan al mar (o a una depresión bajo el nivel del mar, como el
    /// Caspio), de mayor a menor.
    pub cuencas: Vec<Cuenca>,
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

/// Pendiente a 25 km de la tierra por encima de una altura.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendientesClase {
    pub sobre_m: f64,
    pub area_frac: f64,
    pub media: f64,
    /// Percentiles 50, 75, 90, 95 y 99 de `|∇h|` (m/m), por área.
    pub p50_p75_p90_p95_p99: Vec<f64>,
}

/// Espectro del relieve de nacimiento a una escala: desviación típica de las alturas de las 4 hijas
/// de cada celda madre, por clase de la madre (`tierra_L`: altura media ≥ L m; `mar_L`: profundidad
/// media ≥ L m). Es lo que el ruido de detalle debe continuar (decisión del responsable).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EspectroNivel {
    pub nivel_madre: u32,
    pub lado_madre_km: f64,
    pub clases: Vec<String>,
    pub sigma_m: Vec<f64>,
    pub area_frac: Vec<f64>,
}

/// Diagnóstico ampliado del relieve (esquema v6), para compararlo con la Tierra.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticoRelieve {
    pub hipsometria_oceano_limites_m: Vec<f64>,
    /// Fracción del océano en cada franja de profundidad (la última sin fondo).
    pub hipsometria_oceano_frac: Vec<f64>,
    /// Todo el planeta en franjas de 500 m, de −11 000 a +9 000 m.
    pub histograma_planeta_frac: Vec<f64>,
    pub pendientes_25km: Vec<PendientesClase>,
    /// Plataforma continental: mar de menos de 200 m de profundidad entre la longitud de costa.
    pub plataforma_area_km2: f64,
    pub costa_km: f64,
    pub plataforma_anchura_km: f64,
    /// Ley de Hack: longitud del camino de agua más largo aguas arriba ∝ área^h (celdas de tierra
    /// con más de 10⁴ km² de cuenca).
    pub hack_h: f64,
    pub hack_n: u64,
    /// Fracción de la tierra en cuencas cerradas (altura bajo su cota de derrame), como la de la
    /// Tierra a la misma resolución (`depresiones` en la referencia).
    pub tierra_en_depresion_frac: f64,
    pub espectro: Vec<EspectroNivel>,
    /// Nivel del mar que llenaría las cuencas de la malla final con el agua del planeta (m,
    /// respecto al de la génesis; solo con el relieve tectónico, si no 0).
    pub nivel_mar_malla_m: f64,
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
    /// Diagnóstico ampliado del relieve (esquema v6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relieve: Option<DiagnosticoRelieve>,
    /// Diagnóstico del modo Tierra (esquema v8).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tierra: Option<DiagnosticoTierra>,
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
    // En el modo Tierra, el clima medio es la climatología del periodo base de cada celda.
    let tierra = world.statgen.tierra();
    for s in 0..world.topo.num_cells {
        let a = world.topo.area[s];
        let h = world.h[s] - mar;
        let lluvia = match tierra {
            Some(d) => d.clima_anual(world.topo.keys[s]).1,
            None => systems::lluvia_media_anual_mm(world, cfg, s),
        };
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
        let t_media = match tierra {
            Some(d) => d.clima_anual(world.topo.keys[s]).0,
            None => systems::temp_annual_mean(cfg, world.topo.lat[s], world.h[s], false),
        };
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
        relieve: None,
        tierra: None,
    }
}

/// Diagnóstico del modo Tierra con el estado final del mundo (sumas en orden de celda).
pub fn diagnostico_tierra(world: &World) -> Option<DiagnosticoTierra> {
    let d = world.statgen.tierra()?;
    let n = world.topo.num_cells;
    let (mut a_tot, mut a_tierra, mut hielo, mut hielo_tierra) = (0.0, 0.0, 0.0, 0.0);
    let (mut t_clim, mut p_clim, mut t_run, mut p_run) = (0.0, 0.0, 0.0, 0.0);
    let (mut cob, mut lagos) = (0.0, 0.0);
    let muestras = world.sample_count.max(1) as f64;
    let mut salidas: Vec<(f64, usize)> = Vec::new();
    for s in 0..n {
        let a = world.topo.area[s];
        a_tot += a;
        hielo += world.hielo[s] * a;
        let (lv, k) = d.celda_de_clave(world.topo.keys[s]);
        cob += d.niveles[lv].cobertura[k] as f64 * a;
        if world.es_oceano(s) {
            continue;
        }
        a_tierra += a;
        hielo_tierra += world.hielo[s] * a;
        let (t, p) = d.clima_anual(world.topo.keys[s]);
        t_clim += t * a;
        p_clim += p * a;
        t_run += world.temp_sum[s] / muestras * a;
        p_run += world.precip_sum[s] / muestras * 365.0 * a;
        if matches!(world.surface[s], Surface::Lago) {
            lagos += a;
        }
        let r = world.receiver[s];
        if r >= 0 && world.es_oceano(r as usize) {
            salidas.push((world.drainage_area[s], s));
        }
    }
    salidas.sort_by(|x, y| y.0.total_cmp(&x.0).then(x.1.cmp(&y.1)));
    let por = |x: f64, a: f64| if a > 0.0 { x / a } else { 0.0 };
    // Σ z·A por nivel (y Σ |z|·A del nivel de carga, como escala) y el de la malla activa.
    let suma =
        |nv: &crate::tierra::Nivel| nv.z.iter().zip(&nv.area).map(|(z, a)| z * a).sum::<f64>();
    let fino = &d.niveles[d.nivel_carga as usize];
    let escala: f64 = fino
        .z
        .iter()
        .zip(&fino.area)
        .map(|(z, a)| z.abs() * a)
        .sum();
    let ref_fino = suma(fino);
    let conservacion_niveles_rel = d
        .niveles
        .iter()
        .map(|nv| (suma(nv) - ref_fino).abs() / escala)
        .fold(0.0, f64::max);
    let malla: f64 = (0..n).map(|s| world.h0[s] * world.topo.area[s]).sum();
    Some(DiagnosticoTierra {
        hielo_frac: por(hielo, a_tot),
        hielo_tierra_frac: por(hielo_tierra, a_tierra),
        temp_tierra_climatologia_c: por(t_clim, a_tierra),
        lluvia_tierra_climatologia_mm: por(p_clim, a_tierra),
        temp_tierra_corrida_c: por(t_run, a_tierra),
        lluvia_tierra_corrida_mm: por(p_run, a_tierra),
        cobertura_chelsa_frac: por(cob, a_tot),
        lagos_km2: lagos / 1e6,
        conservacion_niveles_rel,
        conservacion_malla_rel: (malla - ref_fino).abs() / escala,
        cuencas: salidas
            .iter()
            .take(50)
            .map(|&(area, s)| Cuenca {
                lat_deg: world.topo.lat[s].to_degrees(),
                lon_deg: world.topo.lon[s].to_degrees(),
                area_km2: area / 1e6,
            })
            .collect(),
    })
}

/// Metadatos del modo Tierra de una corrida de `n_sim` años simulados.
pub fn meta_tierra(
    e: &crate::tierra::clima::Estado,
    cfg: &TerrenoConfig,
    n_sim: usize,
) -> TierraMeta {
    let d = e.datos();
    let t = &cfg.tierra;
    let cru = &d.cru;
    let mut decadas: Vec<[f64; 3]> = Vec::new();
    for (q, chunk) in cru.sin_estaciones.chunks(10).enumerate() {
        let n = chunk.len() as f64;
        decadas.push([
            (cru.anio0 + 10 * q as i32) as f64,
            chunk.iter().map(|x| x[0] as f64).sum::<f64>() / n,
            chunk.iter().map(|x| x[1] as f64).sum::<f64>() / n,
        ]);
    }
    TierraMeta {
        fuentes: vec![
            "ETOPO 2022 v1 (NOAA NCEI), superficie y roca, cada 2'".to_string(),
            "CHELSA V2.1, climatologías mensuales 1981–2010 (tas, pr), 30''".to_string(),
            format!(
                "CRU TS 4.09, series mensuales {}–{} (tmp, pre), 0,5°",
                cru.anio0,
                cru.anio_final()
            ),
        ],
        nivel_carga: d.nivel_carga,
        l_bloque: t.l_bloque,
        r_max: t.r_max,
        periodo_base: t.periodo_base,
        anio_forzado: t.anio_forzado,
        cru_anios: [cru.anio0, cru.anio_final()],
        cru_celdas: cru.n_celdas() as u64,
        cru_pre_recortados: cru.pre_recortados,
        cru_sin_estaciones_por_decada: decadas,
        anios: (0..n_sim as u64)
            .map(|y| AnioReal {
                anio_simulado: y,
                anio_real: e.anio_real(y as i64),
            })
            .collect(),
    }
}

/// Percentiles por área de pares (valor, área).
fn percentiles_area(v: &mut [(f64, f64)], qs: &[f64]) -> Vec<f64> {
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    let tot: f64 = v.iter().map(|x| x.1).sum();
    qs.iter()
        .map(|&q| {
            let mut acc = 0.0;
            for &(x, a) in v.iter() {
                acc += a;
                if acc >= q * tot {
                    return x;
                }
            }
            v.last().map_or(0.0, |x| x.0)
        })
        .collect()
}

/// Diagnóstico ampliado del relieve (esquema v6). Cálculo pesado (evalúa el relieve de nacimiento en
/// las mallas uniformes de los niveles 0 a 3): solo con el relieve tectónico o
/// `run.diagnostico_relieve`.
pub fn diagnostico_relieve(world: &World, cfg: &TerrenoConfig) -> DiagnosticoRelieve {
    let topo = &world.topo;
    let n = topo.num_cells;
    let mar = cfg.planet.sea_level;
    // Hipsometría del océano e histograma del planeta.
    let lim = HIPSOMETRIA_OCEANO_LIMITES_M;
    let mut fr = [0.0; HIPSOMETRIA_OCEANO_LIMITES_M.len()];
    let mut hist = vec![0.0; 40];
    let (mut a_mar, mut a_tot) = (0.0, 0.0);
    for s in 0..n {
        let a = topo.area[s];
        let h = world.h[s] - mar;
        a_tot += a;
        let b = ((h + 11_000.0) / 500.0).floor().clamp(0.0, 39.0) as usize;
        hist[b] += a;
        if world.es_oceano(s) {
            a_mar += a;
            fr[lim.iter().rposition(|&l| -h >= l).unwrap_or(0)] += a;
        }
    }
    let por = |x: f64, a: f64| if a > 0.0 { x / a } else { 0.0 };
    // Pendientes a 25 km de la tierra (la escala que ve la lluvia, como la referencia de ETOPO).
    let tierra: Vec<usize> = (0..n).filter(|&s| !world.es_oceano(s)).collect();
    let grad = crate::erosion::gradientes_referencia(world, &tierra);
    let a_tierra: f64 = tierra.iter().map(|&s| topo.area[s]).sum();
    let pendientes = PENDIENTES_SOBRE_M
        .iter()
        .map(|&sobre| {
            let mut v: Vec<(f64, f64)> = tierra
                .iter()
                .zip(&grad)
                .filter(|(s, _)| world.h[**s] - mar >= sobre)
                .map(|(&s, &(ge, gn))| ((ge * ge + gn * gn).sqrt(), topo.area[s]))
                .collect();
            let a: f64 = v.iter().map(|x| x.1).sum();
            let media = por(v.iter().map(|x| x.0 * x.1).sum(), a);
            PendientesClase {
                sobre_m: sobre,
                area_frac: por(a, a_tierra),
                media,
                p50_p75_p90_p95_p99: percentiles_area(&mut v, &[0.5, 0.75, 0.9, 0.95, 0.99]),
            }
        })
        .collect();
    // Plataforma: mar de menos de 200 m entre la costa (aristas tierra–mar).
    let mut plat = 0.0;
    for s in 0..n {
        if world.es_oceano(s) && world.h[s] - mar > -200.0 {
            plat += topo.area[s];
        }
    }
    let mut costa = 0.0;
    for p in &topo.pairs {
        let (a, b) = (p.a as usize, p.b as usize);
        if p.g == 1.0 && world.es_oceano(a) != world.es_oceano(b) {
            costa += topo.side_km(a).min(topo.side_km(b));
        }
    }
    // Ley de Hack: camino de agua más largo aguas arriba de cada celda.
    let mut largo = vec![0.0f64; n];
    for &id in &world.flow_order {
        let i = id as usize;
        let r = world.receiver[i];
        if r >= 0 {
            let r = r as usize;
            let d =
                crate::geometry::angle_between(topo.dir[i], topo.dir[r]) * topo.r_planet / 1000.0;
            largo[r] = largo[r].max(largo[i] + d);
        }
    }
    let (mut sx, mut sy, mut sxx, mut sxy, mut cuenta) = (0.0, 0.0, 0.0, 0.0, 0u64);
    for (s, &l) in largo.iter().enumerate() {
        let a_km2 = world.drainage_area.get(s).copied().unwrap_or(0.0) / 1e6;
        if world.es_oceano(s) || a_km2 < 1e4 || l <= 0.0 {
            continue;
        }
        let (x, y) = (a_km2.log10(), l.log10());
        sx += x;
        sy += y;
        sxx += x * x;
        sxy += x * y;
        cuenta += 1;
    }
    // Sin datos, 0 (con `hack_n = 0`): el JSON no admite NaN.
    let hack = if cuenta > 2 {
        let c = cuenta as f64;
        (c * sxy - sx * sy) / (c * sxx - sx * sx)
    } else {
        0.0
    };
    DiagnosticoRelieve {
        hipsometria_oceano_limites_m: lim.to_vec(),
        hipsometria_oceano_frac: fr.iter().map(|&x| por(x, a_mar)).collect(),
        histograma_planeta_frac: hist.iter().map(|&x| por(x, a_tot)).collect(),
        pendientes_25km: pendientes,
        plataforma_area_km2: plat / 1e6,
        costa_km: costa,
        plataforma_anchura_km: por(plat / 1e6, costa),
        hack_h: hack,
        hack_n: cuenta,
        tierra_en_depresion_frac: por(
            (0..n)
                .filter(|&s| !world.es_oceano(s) && world.h[s] < world.filled_h[s])
                .map(|s| topo.area[s])
                .sum(),
            a_tierra,
        ),
        espectro: espectro(world, cfg),
        nivel_mar_malla_m: if world.historia.is_some() {
            crate::worldgen::nivel_mar_malla(world, cfg)
        } else {
            0.0
        },
    }
}

/// Espectro del relieve de nacimiento por escala y clase (sin erosión): en las mallas uniformes de
/// los niveles `l_min .. l_max`, la desviación típica de las alturas de las 4 hijas de cada madre.
fn espectro(world: &World, cfg: &TerrenoConfig) -> Vec<EspectroNivel> {
    use crate::geometry::{center_dir, face_n};
    let g = &world.statgen;
    let (n0, r) = (cfg.planet.n0, cfg.planet.r_km);
    let mut out = Vec::new();
    for l in cfg.lod.l_min..cfg.lod.l_max {
        let (n, n2) = (face_n(n0, l), face_n(n0, l + 1));
        let nt = ESPECTRO_TIERRA_M.len();
        let mut var = vec![0.0; nt + ESPECTRO_MAR_M.len()];
        let mut area = vec![0.0; var.len()];
        let mut a_tot = 0.0;
        for c in 0..6u8 {
            for j in 0..n {
                for i in 0..n {
                    let hm = g.h_at(center_dir(c, i, j, n), l);
                    let hs: Vec<f64> = [(0, 0), (1, 0), (0, 1), (1, 1)]
                        .iter()
                        .map(|&(di, dj)| g.h_at(center_dir(c, 2 * i + di, 2 * j + dj, n2), l + 1))
                        .collect();
                    let m = hs.iter().sum::<f64>() / 4.0;
                    let v = hs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / 4.0;
                    let a = crate::geometry::cell_area(c, i, j, n, r);
                    a_tot += a;
                    let k = if hm >= 0.0 {
                        ESPECTRO_TIERRA_M
                            .iter()
                            .rposition(|&x| hm >= x)
                            .unwrap_or(0)
                    } else {
                        nt + ESPECTRO_MAR_M.iter().rposition(|&x| -hm >= x).unwrap_or(0)
                    };
                    var[k] += v * a;
                    area[k] += a;
                }
            }
        }
        let mut clases: Vec<String> = ESPECTRO_TIERRA_M
            .iter()
            .map(|x| format!("tierra_{x}"))
            .collect();
        clases.extend(ESPECTRO_MAR_M.iter().map(|x| format!("mar_{x}")));
        out.push(EspectroNivel {
            nivel_madre: l,
            lado_madre_km: std::f64::consts::FRAC_PI_2 * r / n as f64,
            clases,
            sigma_m: var
                .iter()
                .zip(&area)
                .map(|(v, a)| if *a > 0.0 { (v / a).sqrt() } else { 0.0 })
                .collect(),
            area_frac: area.iter().map(|a| a / a_tot).collect(),
        });
    }
    out
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
