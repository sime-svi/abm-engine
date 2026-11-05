//! Generación del mundo en t=0 (diseño §4 del incremento 1 y §9 del incremento 2). Todo depende
//! solo de la semilla y la config => mundo reproducible.
//!
//! Cadena: malla uniforme del nivel base `l_min` → estáticos (función pura de cada celda, generador
//! de [`StaticGen`]) → vecindad → derivados globales (priority-flood, drenaje, ríos/lagos, distancia
//! al mar) → **refinamiento inicial** del LOD (si está activo) → **génesis geológica** (erosión y
//! suelo durante ~1 millón de años, incr. 3 §8) → estado dinámico inicial sobre la malla final.

use crate::terreno::config::TerrenoConfig;
use crate::terreno::erosion::{self, GeoSummary};
use crate::terreno::geometry::{face_n, key};
use crate::terreno::hydro;
use crate::terreno::lod;
use crate::terreno::statics::StaticGen;
use crate::terreno::world::{Biome, Surface, Topology, World};

/// Genera el mundo completo a partir de la configuración.
pub fn generate(cfg: &TerrenoConfig) -> World {
    generate_with_report(cfg).0
}

/// Genera el mundo y devuelve también el resumen de la génesis geológica.
pub fn generate_with_report(cfg: &TerrenoConfig) -> (World, GeoSummary) {
    let statgen = StaticGen::new(cfg);
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
    for s in 0..num {
        let (c, lv, i, j) = crate::terreno::geometry::split_key(topo.keys[s]);
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
    }
    lod::build_neighbors(&mut topo);

    let mut world = World {
        topo,
        statgen,
        h0: h.clone(),
        grad0_e: grad_e.clone(),
        grad0_n: grad_n.clone(),
        w_cap0: w_cap.clone(),
        k_perm,
        fert,
        resource,
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

    // Refinamiento inicial (solo criterios estáticos: T y θ aún son uniformes), génesis geológica
    // (erosión y suelo, incr. 3 §8) y estado inicial sobre la malla final: la génesis no transfiere
    // agua al mar.
    // El relleno de hondonadas artificiales del refinado inicial (acondicionamiento, incr. 3 §3)
    // también entra en el balance de material de la génesis.
    let pre_fill = if cfg.lod.enabled() {
        lod::refine_initial(&mut world, cfg).fill_m3
    } else {
        0.0
    };
    let mut geo = erosion::genesis(&mut world, cfg);
    geo.lod_fill_m3 += pre_fill;
    if cfg.lod.enabled() || cfg.erosion.geo_steps > 0 {
        set_initial_state(&mut world, cfg);
    }
    (world, geo)
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
    world.veg = ocean
        .iter()
        .map(|&o| if o { 0.0 } else { v_init })
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
