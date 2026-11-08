//! `abm-terreno` — fase «terreno»: el sustrato físico esférico (incrementos 1, 2 y 3).
//!
//! Motor independiente del de forrajeo: genera un planeta (cubo esférico), lo "envejece" con una
//! génesis geológica (erosión y suelo, incremento 3) y corre los sistemas rápidos (clima, nieve,
//! agua, vegetación) de forma determinista, sobre una malla que puede **afinarse sola** donde hay
//! detalle (LOD dinámico, incremento 2), con el relieve aún vivo a ritmo real. Diseño:
//! `docs/design/terreno/incremento-{1,2,3}-diseno.md`. Entrada: `(semilla, config)`. Salida:
//! métricas por tick + (opcional) snapshot por celda del mundo.

pub mod cache;
pub mod config;
pub mod erosion;
pub mod error;
pub mod geometry;
pub mod humedad;
pub mod hydro;
pub mod lod;
pub mod metrics;
pub mod noise;
pub mod statics;
pub mod systems;
pub mod tectonica;
pub mod tierra;
pub mod vegetacion;
pub mod world;
pub mod worldgen;

pub use config::TerrenoConfig;
pub use error::TerrenoError;
pub use metrics::{TerrenoDoc, TerrenoRunMeta, TerrenoTickMetrics};
pub use world::World;

/// Ejecuta una simulación de terreno completa. Devuelve el documento de métricas y el mundo final.
///
/// Es la **misma** función que usan el binario y los tests (el camino verificado es el real). El
/// tiempo es el `tick` entero; no interviene el reloj de pared (CLAUDE §6). Orden dentro del tick
/// (incremento 2 §9): si `t > 0` y `t` es múltiplo de `n_lod`, paso de topología; después los
/// sistemas del incremento 1.
pub fn run(cfg: &TerrenoConfig) -> Result<(TerrenoDoc, World), TerrenoError> {
    run_with_info(cfg).map(|(doc, world, _)| (doc, world))
}

/// Datos de la corrida que no forman parte de la salida del modelo: la salida es idéntica se use o
/// no la caché, y los tiempos cambian de una corrida a otra.
#[derive(Debug, Clone)]
pub struct RunInfo {
    /// La génesis se leyó de la caché de disco (`run.genesis_cache`).
    pub from_cache: bool,
    /// Tiempo de cada paso: la génesis (o su lectura de la caché) y cada sistema del tick.
    pub tiempos: abm_core::cronometro::Cronometro,
}

/// Como [`run`], e informa además de si la génesis salió de la caché y de los tiempos por paso.
pub fn run_with_info(cfg: &TerrenoConfig) -> Result<(TerrenoDoc, World, RunInfo), TerrenoError> {
    cfg.validate()?;
    let mut cron = abm_core::cronometro::Cronometro::default();
    // Modo Tierra: los datos de la Tierra (de su caché o de las fuentes) antes que el mundo.
    let datos_tierra = if cfg.es_tierra() {
        let d = tierra::cargar(cfg)?;
        cron.anotar("carga_tierra");
        Some(d)
    } else {
        None
    };
    let ((mut world, genesis), from_cache) = match cache::load(cfg, datos_tierra.as_ref())? {
        Some(cached) => (cached, true),
        None => {
            let generated = match &datos_tierra {
                Some(d) => worldgen::generate_tierra(cfg, std::sync::Arc::clone(d)),
                None => worldgen::generate_with_report(cfg),
            };
            cache::save(cfg, &generated.0, &generated.1)?;
            (generated, false)
        }
    };
    cron.anotar(if from_cache {
        "genesis_desde_cache"
    } else {
        "genesis"
    });
    let mut scratch = systems::Scratch::new(world.topo.num_cells);
    scratch.cron = cron;
    // Años simulados (enteros o empezados) de la corrida: la secuencia de años reales los cubre.
    let n_sim = cfg.run.total_ticks.div_ceil(365) as usize;
    if let Some(d) = &datos_tierra {
        scratch.tierra = Some(tierra::clima::Estado::new(
            std::sync::Arc::clone(d),
            cfg,
            n_sim,
        )?);
    }
    let mut ticks = Vec::with_capacity(cfg.run.total_ticks as usize);
    let initial_cells = world.topo.num_cells as u64;

    for t in 0..cfg.run.total_ticks {
        // El almacenamiento previo se mide ANTES del paso de topología: el residual del tick cubre
        // también lo que el reparto del LOD envía al mar.
        scratch.cron.marcar();
        let store_before = systems::total_store_vol(&world);
        let lod_report = if cfg.lod.enabled() && t > 0 && t.is_multiple_of(cfg.lod.n_lod) {
            let r = lod::step(&mut world, cfg);
            if r.changed() {
                scratch.resize(world.topo.num_cells);
            }
            r
        } else {
            lod::LodReport::default()
        };
        scratch.cron.anotar("lod");
        let mut acct = systems::step(&mut world, cfg, &mut scratch, t, store_before, &lod_report);
        // Erosión en ejecución (incr. 3 §9), al final del tick: no mueve agua (el exceso de agua
        // sobre una capacidad menor se drena en el tick siguiente).
        // En el modo Tierra el relieve no cambia (la erosión de unos miles de años es despreciable).
        let every = cfg.erosion.run_every;
        if every > 0 && t > 0 && t.is_multiple_of(every) && !cfg.es_tierra() {
            let dt_years = every as f64 / cfg.climate.t_year * cfg.erosion.run_accel;
            let (p, tm) = erosion::running_means(&world, cfg);
            // Con el relieve tectónico, la erosión en ejecución lleva el rebote isostático (§6.9b);
            // sin levantamiento: durante la simulación no hay tectónica (§6.10).
            acct.erosion = if world.historia.is_some() {
                let vecindad = erosion::Vecindad::new(&world, cfg.tectonica.r_flex_km);
                let iso = erosion::Isostasia {
                    levantamiento: &[],
                    rebote: Some((cfg.tectonica.rho_c / cfg.tectonica.rho_m, &vecindad)),
                };
                erosion::step_con(&mut world, cfg, dt_years, &p, &tm, &iso)
            } else {
                erosion::step(&mut world, cfg, dt_years, &p, &tm)
            };
            lod::update_hydro_lock(&mut world);
            scratch.cron.anotar("erosion");
        }
        ticks.push(metrics::compute(&world, &acct, t));
        scratch.cron.anotar("metricas");
    }

    let mut diagnostico = metrics::diagnostico(&world, cfg);
    let ampliado = world.historia.is_some() || cfg.run.diagnostico_relieve;
    if ampliado {
        diagnostico.relieve = Some(metrics::diagnostico_relieve(&world, cfg));
        scratch.cron.anotar("diagnostico_relieve");
    }
    diagnostico.tierra = metrics::diagnostico_tierra(&world);
    let doc = TerrenoDoc {
        schema_version: if cfg.es_tierra() {
            metrics::SCHEMA_VERSION_TIERRA
        } else if ampliado {
            metrics::SCHEMA_VERSION_RELIEVE
        } else {
            metrics::SCHEMA_VERSION
        },
        run: TerrenoRunMeta {
            name: cfg.run.name.clone(),
            seed: cfg.run.seed,
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            n0: cfg.planet.n0,
            l_min: cfg.lod.l_min,
            l_max: cfg.lod.l_max,
            num_cells: initial_cells,
            r_km: cfg.planet.r_km,
            total_ticks: cfg.run.total_ticks,
            genesis,
            lluvia: metrics::LluviaMeta {
                desvio: world.desvio_lluvia,
                c_p: world.c_p,
            },
            tectonica: world.historia.as_ref().map(|h| h.as_ref().clone()),
            tierra: scratch
                .tierra
                .as_ref()
                .map(|e| metrics::meta_tierra(e, cfg, n_sim)),
        },
        diagnostico,
        ticks,
    };
    Ok((
        doc,
        world,
        RunInfo {
            from_cache,
            tiempos: scratch.cron,
        },
    ))
}

/// Snapshot por celda del mundo generado (para inspección/depuración).
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorldDumpCell {
    pub level: u8,
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub area_km2: f64,
    pub h: f64,
    pub erosion_m: f64,
    pub soil_m: f64,
    pub surface: world::Surface,
    pub biome: world::Biome,
    pub theta: f64,
    pub veg: f64,
    pub temp: f64,
    pub precip: f64,
    /// Con `run.dump_climate`: factor orográfico, agotamiento de humedad `w` y lluvia media anual
    /// (mm/año) de la celda (nota de sombra pluviométrica).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub m_oro: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub w_hum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lluvia_anual_mm: Option<f64>,
    /// Con el relieve tectónico: la celda de la malla tectónica que contiene el centro (corteza,
    /// placa, edad del fondo oceánico y borde de placa más cercano).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tectonica: Option<DumpTectonica>,
    /// Modo Tierra: fracción con hielo, área de drenaje de la celda (km²) y, con `run.dump_climate`,
    /// la temperatura media anual de su climatología (°C).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hielo: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temp_anual_c: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drenaje_km2: Option<f64>,
    /// Modo Tierra: fracción de lago, caudal medio que pasa por la celda (km³/año) y su receptor
    /// (−1 al mar, −2 fondo de una cuenca cerrada).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lago: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caudal_km3_anio: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receptor: Option<i64>,
}

/// Datos tectónicos de una celda del volcado.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DumpTectonica {
    pub continental: bool,
    pub placa: u64,
    pub espesor_km: f64,
    pub sedimento_km: f64,
    pub edad_oc_myr: f64,
    pub borde_tipo: u8,
    pub borde_dist_km: f64,
}

/// Serializa el snapshot por celda del mundo (orden de clave).
pub fn world_dump(world: &World, cfg: &TerrenoConfig) -> Vec<WorldDumpCell> {
    let clima = cfg.run.dump_climate;
    let tierra = world.statgen.tierra();
    let anios = (world.sample_count as f64 / 365.0).max(1.0 / 365.0);
    // El factor orográfico y el agotamiento son del clima del planeta inventado.
    let clima_modelo = clima && tierra.is_none();
    (0..world.topo.num_cells)
        .map(|id| WorldDumpCell {
            level: world.topo.level[id],
            lat_deg: world.topo.lat[id].to_degrees(),
            lon_deg: world.topo.lon[id].to_degrees(),
            area_km2: world.topo.area[id] / 1e6,
            h: world.h[id],
            erosion_m: world.e[id],
            soil_m: world.z_soil[id],
            surface: world.surface[id],
            biome: world.biome[id],
            theta: world.theta(id),
            veg: world.veg[id],
            temp: world.temp[id],
            precip: world.precip[id],
            m_oro: clima_modelo.then(|| systems::m_oro_celda(world, cfg, id)),
            w_hum: clima_modelo.then(|| world.w_hum[id]),
            lluvia_anual_mm: clima.then(|| match tierra {
                Some(d) => d.clima_anual(world.topo.keys[id]).1,
                None => systems::lluvia_media_anual_mm(world, cfg, id),
            }),
            tectonica: world.statgen.tectonica().map(|t| {
                let k = t.celda(world.topo.dir[id]);
                DumpTectonica {
                    continental: t.continental[k],
                    placa: t.placa[k],
                    espesor_km: t.h_km[k],
                    sedimento_km: t.sed_km[k],
                    edad_oc_myr: t.edad_oc[k],
                    borde_tipo: t.borde_tipo[k],
                    borde_dist_km: t.borde_dist_km[k],
                }
            }),
            hielo: tierra.map(|_| world.hielo[id]),
            temp_anual_c: tierra
                .filter(|_| clima)
                .map(|d| d.clima_anual(world.topo.keys[id]).0),
            drenaje_km2: tierra.map(|_| world.drainage_area[id] / 1e6),
            lago: tierra.map(|_| world.lago[id]),
            caudal_km3_anio: tierra.map(|_| world.caudal_m3[id] / 1e9 / anios),
            receptor: tierra.map(|_| world.receiver[id]),
        })
        .collect()
}
