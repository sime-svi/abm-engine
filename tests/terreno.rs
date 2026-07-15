//! Tests de integración de la fase «terreno» (incremento 1): determinismo, continuidad esférica,
//! conservación de agua y casos límite. Diseño: `docs/design/terreno/incremento-1-diseno.md` §9.

use abm::terreno::config::TerrenoConfig;
use abm::terreno::geometry::split_id;
use abm::terreno::world::Surface;

/// Config pequeña y rápida para tests (pocas celdas, pocos ticks), determinista.
fn cfg_test() -> TerrenoConfig {
    let mut c = TerrenoConfig::default();
    c.run.seed = 7;
    c.run.total_ticks = 40;
    c.planet.n0 = 16; // 6·16² = 1536 celdas
    c
}

#[test]
fn determinismo_misma_semilla_misma_salida() {
    let cfg = cfg_test();
    let (doc_a, world_a) = abm::terreno::run(&cfg).unwrap();
    let (doc_b, world_b) = abm::terreno::run(&cfg).unwrap();
    // Métricas idénticas y mundo idéntico (elevación bit a bit).
    assert_eq!(doc_a, doc_b, "las métricas difieren con la misma semilla");
    assert_eq!(
        world_a.h, world_b.h,
        "la elevación difiere con la misma semilla"
    );
    assert_eq!(
        world_a.w, world_b.w,
        "el agua del suelo difiere con la misma semilla"
    );
}

#[test]
fn semillas_distintas_mundos_distintos() {
    let cfg_a = cfg_test();
    let mut cfg_b = cfg_test();
    cfg_b.run.seed = 99;
    let (_, world_a) = abm::terreno::run(&cfg_a).unwrap();
    let (_, world_b) = abm::terreno::run(&cfg_b).unwrap();
    assert_ne!(
        world_a.h, world_b.h,
        "semillas distintas dieron el mismo relieve"
    );
}

#[test]
fn continuidad_esferica_de_la_elevacion() {
    // La elevación (ruido 3D) debe ser continua al cruzar las aristas del cubo: el salto entre
    // caras no debe ser mayor que el salto típico dentro de una cara (no hay costuras).
    let (_, world) = abm::terreno::run(&cfg_test()).unwrap();
    let n = world.topo.n;
    let (mut max_intra, mut max_cross) = (0.0_f64, 0.0_f64);
    for id in 0..world.topo.num_cells {
        let (cara_i, _, _) = split_id(id as u32, n);
        for &nb in world.topo.neighbors[id].iter() {
            if nb < 0 {
                continue;
            }
            let (cara_j, _, _) = split_id(nb as u32, n);
            let dh = (world.h[id] - world.h[nb as usize]).abs();
            if cara_i == cara_j {
                max_intra = max_intra.max(dh);
            } else {
                max_cross = max_cross.max(dh);
            }
        }
    }
    // Sin costura: el mayor salto entre caras es del mismo orden que dentro de una cara.
    assert!(
        max_cross <= 3.0 * max_intra,
        "posible costura entre caras: max_cross={max_cross:.1} vs max_intra={max_intra:.1}"
    );
}

#[test]
fn conservacion_de_agua_global() {
    // El balance de agua (entrada − ET − salida al mar − Δalmacenamiento) debe cerrar a tolerancia
    // de coma flotante en todos los ticks (métrica de validación clave, diseño §9).
    let (doc, _) = abm::terreno::run(&cfg_test()).unwrap();
    let peor = doc
        .ticks
        .iter()
        .map(|t| t.water_residual_rel.abs())
        .fold(0.0_f64, f64::max);
    assert!(
        peor < 1e-9,
        "el balance de agua no cierra: |residual_rel| máx = {peor:.2e}"
    );
}

#[test]
fn sin_lluvia_seca_el_suelo_y_mata_la_vegetacion() {
    // Caso límite: sin precipitación, la humedad tiende a 0 y la vegetación decae hacia 0.
    let mut cfg = cfg_test();
    cfg.run.total_ticks = 120;
    cfg.climate.p_ref = 0.0;
    let (doc, world) = abm::terreno::run(&cfg).unwrap();

    // θ media parte de 0.5; sin lluvia cae con fuerza (las zonas frías con PET=0 no llegan a 0,
    // por eso la media global se estabiliza por encima de 0, no en 0).
    let theta_final = doc.ticks.last().unwrap().theta_mean;
    assert!(
        theta_final < 0.2,
        "el suelo no se secó: θ media final = {theta_final:.3}"
    );

    let veg_tierra: f64 = (0..world.topo.num_cells)
        .filter(|&i| !matches!(world.surface[i], Surface::Oceano))
        .map(|i| world.veg[i])
        .sum();
    let n_tierra = (0..world.topo.num_cells)
        .filter(|&i| !matches!(world.surface[i], Surface::Oceano))
        .count() as f64;
    let veg_media = veg_tierra / n_tierra.max(1.0);
    assert!(
        veg_media < 0.5,
        "la vegetación no murió sin lluvia: biomasa media = {veg_media:.3}"
    );
}

#[test]
fn con_lluvia_hay_rios() {
    // Con el clima por defecto deben emerger ríos (celdas con área de drenaje sobre el umbral).
    let (doc, _) = abm::terreno::run(&cfg_test()).unwrap();
    let rios = doc.ticks.last().unwrap().river_cells;
    assert!(rios > 0, "no emergió ningún río");
}
