//! Generación del mundo en t=0 (diseño §4 del incremento 1 y §9 del incremento 2). Todo depende
//! solo de la semilla y la config => mundo reproducible.
//!
//! Cadena: malla uniforme del nivel base `l_min` → estáticos (función pura de cada celda, generador
//! de [`StaticGen`]) → vecindad → derivados globales (priority-flood, drenaje, ríos/lagos, distancia
//! al mar) → **refinamiento inicial** del LOD (si está activo) → **génesis geológica** (erosión y
//! suelo durante ~1 millón de años, incr. 3 §8) → estado dinámico inicial sobre la malla final.

use std::sync::Arc;

use crate::config::{Relieve, TerrenoConfig};
use crate::erosion::{self, GeoSummary};
use crate::geometry::{face_n, key};
use crate::humedad;
use crate::hydro;
use crate::lod;
use crate::statics::StaticGen;
use crate::world::{Biome, Surface, Topology, World};

/// Genera el mundo completo a partir de la configuración.
pub fn generate(cfg: &TerrenoConfig) -> World {
    generate_with_report(cfg).0
}

/// Genera el mundo y devuelve también el resumen de la génesis geológica.
pub fn generate_with_report(cfg: &TerrenoConfig) -> (World, GeoSummary) {
    // Relieve de nacimiento: el ruido de los incrementos 1–3 o la deriva de placas (incremento 4,
    // G1–G4; el detalle fino G5 lo pone el generador de estáticos).
    let (statgen, historia) = match cfg.worldgen.relieve {
        Relieve::Ruido => (StaticGen::new(cfg), None),
        Relieve::Tectonico => {
            let (t, r) = crate::tectonica::genesis(cfg);
            (StaticGen::con_tectonica(cfg, Arc::new(t)), Some(r))
        }
    };
    let mut world = malla_inicial(cfg, statgen);

    // Refinamiento inicial (solo criterios estáticos: T y θ aún son uniformes), génesis geológica
    // (erosión y suelo, incr. 3 §8) y estado inicial sobre la malla final: la génesis no transfiere
    // agua al mar.
    // El relleno de hondonadas artificiales del refinado inicial (acondicionamiento, incr. 3 §3)
    // también entra en el balance de material de la génesis.
    let mut pre = if cfg.lod.enabled() {
        lod::refine_initial(&mut world, cfg)
    } else {
        lod::LodReport::default()
    };
    // G6 (relieve tectónico): acondicionamiento hidrológico de los hoyos del ruido de detalle.
    if world.statgen.tectonica().is_some() && cfg.erosion.enabled() {
        pre.absorb(&lod::acondicionar_detalle(&mut world, cfg));
    }
    let mut geo = erosion::genesis(&mut world, cfg);
    geo.lod_fill_m3 += pre.fill_m3;
    geo.lod_conditioned_m3 += pre.conditioned_m3;
    // Agotamiento de humedad y factor c_P (nota de sombra pluviométrica): se calculan sobre el
    // relieve ya envejecido y se mantienen al día durante la simulación.
    humedad::activar(&mut world, cfg);
    if cfg.lod.enabled() || cfg.erosion.geo_steps > 0 {
        set_initial_state(&mut world, cfg);
    }
    if let Some(mut r) = historia {
        r.nivel_mar_tras_prerrodaje_m = nivel_mar_malla(&world, cfg);
        world.historia = Some(Arc::new(r));
    }
    (world, geo)
}

/// Mundo del modo Tierra (`modo-tierra-diseno.md` §2.3): la misma malla, la hidrología del motor y
/// el refinado inicial del LOD sobre la altura de ETOPO 2022. Sin deriva, génesis geológica ni
/// agotamiento de humedad: el relieve queda tal cual (ni se erosiona ni se rellenan sus hondonadas).
pub fn generate_tierra(
    cfg: &TerrenoConfig,
    datos: Arc<crate::tierra::DatosTierra>,
) -> (World, GeoSummary) {
    let mut world = malla_inicial(cfg, StaticGen::con_tierra(cfg, datos));
    let pre = if cfg.lod.enabled() {
        lod::refine_initial(&mut world, cfg)
    } else {
        lod::LodReport::default()
    };
    let foto = erosion::snapshot(&world);
    let geo = GeoSummary {
        lod_fill_m3: pre.fill_m3,
        lod_conditioned_m3: pre.conditioned_m3,
        before: foto,
        after: foto,
        ..GeoSummary::default()
    };
    set_initial_state(&mut world, cfg);
    (world, geo)
}

/// Malla uniforme del nivel base con sus estáticos, su vecindad, sus derivados globales y el estado
/// dinámico inicial.
fn malla_inicial(cfg: &TerrenoConfig, statgen: StaticGen) -> World {
    let l = cfg.lod.l_min;
    let n = face_n(cfg.planet.n0, l);
    let num = 6 * (n as usize) * (n as usize);

    // Malla uniforme del nivel base, en orden de clave (cara, j, i).
    let mut keys = Vec::with_capacity(num);
    for c in 0..6u8 {
        for j in 0..n {
            for i in 0..n {
                keys.push(key(c, l, i, j));
            }
        }
    }

    let mut topo = Topology {
        n0: cfg.planet.n0,
        l_min: cfg.lod.l_min,
        l_max: cfg.lod.l_max,
        num_cells: num,
        r_planet: cfg.planet.r_km * 1000.0,
        keys,
        level: vec![l as u8; num],
        lat: Vec::with_capacity(num),
        lon: Vec::with_capacity(num),
        area: Vec::with_capacity(num),
        east: Vec::with_capacity(num),
        north: Vec::with_capacity(num),
        dir: Vec::with_capacity(num),
        nb_off: Vec::new(),
        nb: Vec::new(),
        pairs: Vec::new(),
    };

    // Geometría y estáticos (función pura de cada celda).
    let mut h = Vec::with_capacity(num);
    let mut slope = Vec::with_capacity(num);
    let mut aspect = Vec::with_capacity(num);
    let mut grad_e = Vec::with_capacity(num);
    let mut grad_n = Vec::with_capacity(num);
    let mut w_cap = Vec::with_capacity(num);
    let mut k_perm = Vec::with_capacity(num);
    let mut fert = Vec::with_capacity(num);
    let mut resource = Vec::with_capacity(num);
    let mut hielo = Vec::with_capacity(num);
    let mut oro0_e = Vec::with_capacity(num);
    let mut oro0_n = Vec::with_capacity(num);
    let mut oro_air0_e = Vec::with_capacity(num);
    for s in 0..num {
        let (c, lv, i, j) = crate::geometry::split_key(topo.keys[s]);
        let g = statgen.geom(c, lv, i, j);
        let st = statgen.statics(c, lv, i, j, &g);
        topo.lat.push(g.lat);
        topo.lon.push(g.lon);
        topo.area.push(g.area);
        topo.east.push(g.east);
        topo.north.push(g.north);
        topo.dir.push(g.dir);
        h.push(st.h);
        slope.push(st.slope);
        aspect.push(st.aspect);
        grad_e.push(st.grad_e);
        grad_n.push(st.grad_n);
        w_cap.push(st.w_cap);
        k_perm.push(st.k_perm);
        fert.push(st.fert);
        resource.push(st.resource);
        hielo.push(statgen.hielo(c, lv, i, j));
        let (oe, on, oa) = statgen.oro_gradient0_air(&g, lv);
        oro0_e.push(oe);
        oro0_n.push(on);
        // Relieve que ve el aire (mar plano): a la escala de referencia o a la de la celda.
        oro_air0_e.push(if statgen.uses_oro_reference(lv) {
            oa
        } else {
            st.grad_air_e
        });
    }
    lod::build_neighbors(&mut topo);

    let mut world = World {
        topo,
        statgen,
        h0: h.clone(),
        grad0_e: grad_e.clone(),
        grad0_n: grad_n.clone(),
        w_cap0: w_cap.clone(),
        oro_e: oro0_e.clone(),
        oro_n: oro0_n.clone(),
        oro0_e,
        oro0_n,
        oro_air0_e,
        k_perm,
        fert,
        resource,
        hielo,
        e: vec![0.0; num],
        z_soil: vec![cfg.soil.z_init; num],
        h,
        slope,
        aspect,
        grad_e,
        grad_n,
        w_cap,
        filled_h: Vec::new(),
        receiver: Vec::new(),
        drainage_area: Vec::new(),
        flow_order: Vec::new(),
        dist_ocean_km: Vec::new(),
        w_hum: vec![1.0; num],
        c_p: 1.0,
        desvio_lluvia: 0.0,
        humedad_activa: false,
        anillos: None,
        historia: None,
        surface: Vec::new(),
        w: Vec::new(),
        temp: Vec::new(),
        precip: Vec::new(),
        swe: Vec::new(),
        veg: Vec::new(),
        biome: Vec::new(),
        hydro_lock: Vec::new(),
        temp_sum: Vec::new(),
        precip_sum: Vec::new(),
        sample_count: 0,
    };
    hydro::recompute(&mut world, cfg);
    world.hydro_lock = vec![false; world.topo.num_cells];
    lod::update_hydro_lock(&mut world);
    set_initial_state(&mut world, cfg);
    world
}

/// Nivel del mar (m, respecto al de la génesis) que llena las cuencas de la malla del planeta con el
/// agua de `tectonica.gel_m` (G7, §6.9c): `Σ max(0, ζ − h)·A = GEL·A_planeta`.
pub fn nivel_mar_malla(world: &World, cfg: &TerrenoConfig) -> f64 {
    let area_km2: Vec<f64> = world.topo.area.iter().map(|a| a / 1e6).collect();
    let total: f64 = area_km2.iter().sum();
    let h: Vec<f64> = world.h.iter().map(|h| h - cfg.planet.sea_level).collect();
    crate::tectonica::isostasia::nivel_del_mar(&h, &area_km2, cfg.tectonica.gel_m / 1000.0 * total)
}

/// Estado dinámico inicial (incremento 1): suelo a media capacidad y biomasa semilla en tierra;
/// el océano sin agua del suelo, nieve ni biomasa (invariante del LOD, incremento 2 §6).
fn set_initial_state(world: &mut World, cfg: &TerrenoConfig) {
    let num = world.topo.num_cells;
    let v_init = cfg.vegetation.v_init;
    let ocean: Vec<bool> = world
        .surface
        .iter()
        .map(|s| matches!(s, Surface::Oceano))
        .collect();
    world.w = (0..num)
        .map(|s| if ocean[s] { 0.0 } else { 0.5 * world.w_cap[s] })
        .collect();
    // El hielo nace sin vegetación (en el planeta inventado `hielo` = 0 y el factor es exactamente 1).
    world.veg = ocean
        .iter()
        .zip(&world.hielo)
        .map(|(&o, &h)| if o { 0.0 } else { v_init * (1.0 - h) })
        .collect();
    world.biome = ocean
        .iter()
        .map(|&o| if o { Biome::Oceano } else { Biome::Desierto })
        .collect();
    world.swe = vec![0.0; num];
    world.temp = vec![0.0; num];
    world.precip = vec![0.0; num];
    world.temp_sum = vec![0.0; num];
    world.precip_sum = vec![0.0; num];
    world.sample_count = 0;
}
