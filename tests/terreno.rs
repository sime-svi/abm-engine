//! Tests de integración de la fase «terreno».
//!
//! - Incremento 1 (malla uniforme, `l_max = l_min`): determinismo, continuidad esférica,
//!   conservación de agua y casos límite. Diseño: `incremento-1-diseno.md` §9.
//! - Incremento 2 (LOD dinámico): determinismo, no regresión, equilibrio 2:1, conservación con
//!   pasos de topología, independencia del camino, convergencia sin parpadeo y estabilidad de las
//!   métricas globales. Diseño: `incremento-2-diseno.md` §10.

use abm::terreno::config::TerrenoConfig;
use abm::terreno::lod::{self, Marks};
use abm::terreno::systems::total_store_vol;
use abm::terreno::world::{Surface, World};
use abm::terreno::worldgen;

/// Config pequeña y rápida para tests (pocas celdas, pocos ticks), determinista, malla uniforme.
fn cfg_test() -> TerrenoConfig {
    let mut c = TerrenoConfig::default();
    c.run.seed = 7;
    c.run.total_ticks = 40;
    c.planet.n0 = 16; // 6·16² = 1536 celdas
    c.lod.l_max = c.lod.l_min; // incremento 1: sin LOD
    sin_erosion(&mut c);
    c
}

/// Relieve estático (incrementos 1 y 2): sin génesis geológica ni erosión en ejecución.
fn sin_erosion(c: &mut TerrenoConfig) {
    c.erosion.geo_steps = 0;
    c.erosion.run_every = 0;
}

/// Config pequeña con LOD activo (2 niveles de refinamiento y pasos de topología frecuentes).
fn cfg_lod() -> TerrenoConfig {
    let mut c = TerrenoConfig::default();
    c.run.seed = 7;
    c.run.total_ticks = 60;
    c.planet.n0 = 12; // 864 celdas base
    c.lod.l_max = 2;
    c.lod.n_lod = 10;
    sin_erosion(&mut c);
    c
}

/// Peor residual relativo del balance de agua de una corrida.
fn peor_residual(doc: &abm::terreno::TerrenoDoc) -> f64 {
    doc.ticks
        .iter()
        .map(|t| t.water_residual_rel.abs())
        .fold(0.0_f64, f64::max)
}

// ------------------------------------------------------------------------------------------------
// Incremento 1 (malla uniforme)
// ------------------------------------------------------------------------------------------------

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
    let (mut max_intra, mut max_cross) = (0.0_f64, 0.0_f64);
    for p in &world.topo.pairs {
        let (a, b) = (p.a as usize, p.b as usize);
        let dh = (world.h[a] - world.h[b]).abs();
        if world.topo.face(a) == world.topo.face(b) {
            max_intra = max_intra.max(dh);
        } else {
            max_cross = max_cross.max(dh);
        }
    }
    // Sin costura: el mayor salto entre caras es del mismo orden que dentro de una cara.
    assert!(
        max_cross <= 3.0 * max_intra,
        "posible costura entre caras: max_cross={max_cross:.1} vs max_intra={max_intra:.1}"
    );
}

#[test]
fn vecindad_uniforme_de_moore() {
    // A nivel uniforme, el anillo multinivel reproduce los 8 vecinos de Moore (7 en las 24
    // celdas-esquina del cubo), con relación simétrica.
    let world = worldgen::generate(&cfg_test());
    let topo = &world.topo;
    let mut con_siete = 0;
    for s in 0..topo.num_cells {
        let nbs = topo.neighbors(s);
        match nbs.len() {
            8 => {}
            7 => con_siete += 1,
            k => panic!("celda {s} con {k} vecinos a nivel uniforme"),
        }
        for &b in nbs {
            assert!(
                topo.neighbors(b as usize).contains(&(s as u32)),
                "vecindad no simétrica"
            );
        }
    }
    assert_eq!(con_siete, 24);
    let ortogonales = topo.pairs.iter().filter(|p| p.g == 1.0).count();
    let diagonales = topo.pairs.len() - ortogonales;
    // 4 ortogonales y 4 diagonales por celda (cada par se cuenta una vez), salvo las 24 esquinas.
    assert_eq!(ortogonales, 2 * topo.num_cells);
    assert_eq!(diagonales, 2 * topo.num_cells - 24 / 2);
}

#[test]
fn conservacion_de_agua_global() {
    // El balance de agua (entrada − ET − salida al mar − Δalmacenamiento) debe cerrar a tolerancia
    // de coma flotante en todos los ticks (métrica de validación clave, diseño §9).
    let (doc, _) = abm::terreno::run(&cfg_test()).unwrap();
    let peor = peor_residual(&doc);
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

// ------------------------------------------------------------------------------------------------
// Incremento 2 (LOD dinámico)
// ------------------------------------------------------------------------------------------------

#[test]
fn lod_determinista() {
    let cfg = cfg_lod();
    let (doc_a, world_a) = abm::terreno::run(&cfg).unwrap();
    let (doc_b, world_b) = abm::terreno::run(&cfg).unwrap();
    assert_eq!(
        doc_a, doc_b,
        "con LOD, las métricas difieren con la misma semilla"
    );
    assert_eq!(world_a.topo.keys, world_b.topo.keys, "la malla difiere");
    assert_eq!(world_a.w, world_b.w, "el agua del suelo difiere");
    // Y el LOD actuó de verdad: hay hojas en más de un nivel.
    let niveles = doc_a.ticks.last().unwrap().cells_by_level.clone();
    assert!(
        niveles.iter().filter(|&&n| n > 0).count() >= 2,
        "el LOD no refinó nada: {niveles:?}"
    );
}

#[test]
fn lod_que_no_dispara_equivale_a_lod_apagado() {
    // No regresión (D11): con LOD activo pero criterios que nunca disparan, la salida es idéntica,
    // byte a byte, a la de la malla uniforme sin LOD.
    let mut apagado = cfg_lod();
    apagado.lod.l_max = apagado.lod.l_min;
    let mut mudo = cfg_lod();
    mudo.lod.refine_features = false;
    mudo.lod.dh_sub = 1e30;
    mudo.lod.dt_sub = 1e30;
    mudo.lod.dtheta_sub = 1e30;
    let (doc_a, world_a) = abm::terreno::run(&apagado).unwrap();
    let (doc_b, world_b) = abm::terreno::run(&mudo).unwrap();
    assert_eq!(doc_a.ticks, doc_b.ticks, "las métricas por tick difieren");
    assert_eq!(world_a.topo.keys, world_b.topo.keys);
    assert_eq!(world_a.w, world_b.w);
    assert_eq!(world_a.veg, world_b.veg);
}

#[test]
fn lod_mantiene_el_equilibrio_2a1() {
    // Tras la génesis y tras cada paso de topología, hojas vecinas difieren como mucho un nivel, y
    // la vecindad es simétrica.
    let cfg = cfg_lod();
    let mut world = worldgen::generate(&cfg);
    for paso in 0..6 {
        assert!(world.topo.is_balanced(), "2:1 roto en el paso {paso}");
        for s in 0..world.topo.num_cells {
            for &b in world.topo.neighbors(s) {
                assert!(world.topo.neighbors(b as usize).contains(&(s as u32)));
            }
        }
        lod::step(&mut world, &cfg);
    }
}

#[test]
fn lod_conserva_el_agua_con_pasos_de_topologia() {
    // El balance global (con el término lod_al_mar) cierra en todos los ticks aunque la malla
    // cambie a menudo; y hubo cambios de malla de verdad en la corrida.
    let mut cfg = cfg_lod();
    cfg.lod.n_lod = 5;
    // Sin criterios estáticos la génesis queda uniforme; los criterios dinámicos (sensibles) mueven
    // la malla durante la corrida, con agua real en juego (también en las costas: hijas que
    // resultan ser mar => término lod_al_mar).
    cfg.lod.refine_features = false;
    cfg.lod.dh_sub = 1e30;
    cfg.lod.dt_sub = 1.0;
    cfg.lod.dtheta_sub = 0.1;
    let (doc, _) = abm::terreno::run(&cfg).unwrap();
    assert_eq!(
        doc.run.num_cells,
        6 * 12 * 12,
        "la génesis debía quedar uniforme"
    );
    let peor = peor_residual(&doc);
    assert!(
        peor < 1e-9,
        "con LOD el balance de agua no cierra: |residual_rel| máx = {peor:.2e}"
    );
    let cambios: u64 = doc
        .ticks
        .iter()
        .map(|t| t.lod_subdivisions + t.lod_collapses)
        .sum();
    assert!(cambios > 0, "la malla no cambió durante la corrida");
    let al_mar: f64 = doc.ticks.iter().map(|t| t.lod_to_ocean_m3).sum();
    assert!(
        al_mar > 0.0,
        "el reparto nunca mandó agua al mar: no se ejercitó lod_al_mar"
    );
}

/// Estado repartible de un mundo, por hoja, para comparar.
fn estado(world: &World) -> Vec<(u64, f64, f64, f64)> {
    (0..world.topo.num_cells)
        .map(|s| (world.topo.keys[s], world.w[s], world.swe[s], world.veg[s]))
        .collect()
}

#[test]
fn lod_ida_y_vuelta_devuelve_el_estado() {
    // Independencia del camino (§6.3): subdividir TODAS las hojas y colapsarlas de inmediato
    // devuelve la malla y el estado celda a celda. Lo único que no vuelve es lo que el reparto
    // mandó al mar (madres de tierra sin hijas de tierra o con exceso de capacidad), y eso cuadra
    // exactamente con el término lod_al_mar.
    let mut cfg = cfg_lod();
    cfg.lod.l_max = 1;
    let mut cfg_uniforme = cfg.clone();
    cfg_uniforme.lod.l_max = cfg_uniforme.lod.l_min;
    let mut world = worldgen::generate(&cfg_uniforme);
    world.topo.l_max = 1; // la malla uniforme admite ahora un nivel más
    // Un poco de dinámica para que el estado no sea trivial (nieve, suelo heterogéneo).
    let mut sc = abm::terreno::systems::Scratch::new(world.topo.num_cells);
    for t in 0..30 {
        let antes = total_store_vol(&world);
        abm::terreno::systems::step(&mut world, &cfg, &mut sc, t, antes, &Default::default());
    }
    let original = estado(&world);
    let vol0 = total_store_vol(&world);
    let n = world.topo.num_cells;

    let todo = Marks::forced(vec![true; n], vec![false; n]);
    let ida = lod::apply(&mut world, &cfg, &todo);
    assert_eq!(ida.subdivisions as usize, n);
    assert!(world.topo.is_balanced());
    let m = world.topo.num_cells;
    let nada = Marks::forced(vec![false; m], vec![true; m]);
    let vuelta = lod::apply(&mut world, &cfg, &nada);
    assert_eq!(vuelta.collapses as usize, n);
    let final_ = estado(&world);
    assert_eq!(final_.len(), original.len());

    // Conservación exacta, contando lo enviado al mar.
    let vol1 = total_store_vol(&world);
    let al_mar = ida.to_ocean_vol + vuelta.to_ocean_vol;
    assert!(
        (vol0 - vol1 - al_mar).abs() <= 1e-9 * vol0,
        "ida y vuelta no conserva: {vol0} vs {vol1} + {al_mar}"
    );

    // Identidad celda a celda (salvo las familias que mandaron algo al mar).
    let mut identicas = 0;
    for (a, b) in original.iter().zip(&final_) {
        assert_eq!(a.0, b.0, "la malla no volvió a la original");
        let iguales = [(a.1, b.1), (a.2, b.2), (a.3, b.3)]
            .iter()
            .all(|(x, y)| (x - y).abs() <= 1e-9 * x.abs().max(1.0));
        if iguales {
            identicas += 1;
        }
    }
    assert!(
        identicas as f64 >= 0.9 * n as f64,
        "solo {identicas} de {n} hojas volvieron a su estado"
    );
}

#[test]
fn lod_converge_sin_parpadeo() {
    // Con solo criterios estáticos (relieve y rasgos), la malla deja de cambiar: los pasos de
    // topología finales no subdividen ni colapsan nada.
    let mut cfg = cfg_lod();
    cfg.lod.dt_sub = 1e30;
    cfg.lod.dtheta_sub = 1e30;
    cfg.lod.n_lod = 4;
    cfg.run.total_ticks = 120;
    let (doc, _) = abm::terreno::run(&cfg).unwrap();
    let lod_ticks: Vec<_> = doc
        .ticks
        .iter()
        .filter(|t| t.tick > 0 && t.tick % cfg.lod.n_lod == 0)
        .collect();
    let finales = &lod_ticks[lod_ticks.len() - 10..];
    for t in finales {
        assert_eq!(
            (t.lod_subdivisions, t.lod_collapses),
            (0, 0),
            "la malla sigue cambiando en el tick {}",
            t.tick
        );
    }
}

#[test]
fn lod_refina_costas_y_rios() {
    // Los rasgos geográficos (costa, río, lago) acaban al nivel máximo, y el LOD apenas toca las
    // métricas globales frente a la malla uniforme.
    let cfg = cfg_lod();
    let (doc, world) = abm::terreno::run(&cfg).unwrap();
    let topo = &world.topo;
    let l_max = topo.l_max as u8;
    let (mut rasgos, mut finos) = (0, 0);
    for s in 0..topo.num_cells {
        let oceano = world.es_oceano(s);
        let costa = topo
            .neighbors(s)
            .iter()
            .any(|&b| world.es_oceano(b as usize) != oceano);
        if costa || matches!(world.surface[s], Surface::Rio | Surface::Lago) {
            rasgos += 1;
            if topo.level[s] == l_max {
                finos += 1;
            }
        }
    }
    assert!(
        finos as f64 >= 0.9 * rasgos as f64,
        "solo {finos} de {rasgos} celdas-rasgo están al nivel máximo"
    );

    let mut uniforme = cfg.clone();
    uniforme.lod.l_max = uniforme.lod.l_min;
    let (doc_u, _) = abm::terreno::run(&uniforme).unwrap();
    let (a, b) = (doc.ticks.last().unwrap(), doc_u.ticks.last().unwrap());
    assert!(
        (a.temp_mean - b.temp_mean).abs() < 1.0,
        "T media cambia demasiado: {:.2} vs {:.2}",
        a.temp_mean,
        b.temp_mean
    );
    let area_total = 4.0 * std::f64::consts::PI * (cfg.planet.r_km * 1000.0).powi(2);
    let (fa, fb) = (a.land_area_m2 / area_total, b.land_area_m2 / area_total);
    assert!(
        (fa - fb).abs() < 0.03,
        "la fracción de tierra cambia demasiado: {fa:.3} vs {fb:.3}"
    );
}
