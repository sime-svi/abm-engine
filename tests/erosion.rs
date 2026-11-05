//! Tests de la fase «terreno» — incremento 3 (relieve vivo): conservación de material, estabilidad
//! del esquema implícito, relleno de hondonadas, compatibilidad con el LOD, no regresión y
//! determinismo. Diseño: `docs/design/terreno/incremento-3-diseno.md` §11.

use abm::terreno::config::TerrenoConfig;
use abm::terreno::erosion;
use abm::terreno::geometry::{parent_key, split_key};
use abm::terreno::lod::{self, Marks};
use abm::terreno::world::{Surface, World};
use abm::terreno::worldgen;

/// Mundo pequeño con LOD y una génesis geológica corta (rápido en debug).
fn cfg_geo() -> TerrenoConfig {
    let mut c = TerrenoConfig::default();
    c.run.seed = 7;
    c.run.total_ticks = 30;
    c.planet.n0 = 12;
    c.lod.l_max = 2;
    c.lod.n_lod = 10;
    c.erosion.geo_steps = 12;
    c.erosion.geo_rounds = 2;
    c.erosion.geo_dt_years = 20_000.0; // 240 000 años en total
    c.erosion.run_every = 0;
    c
}

/// La misma config con el relieve estático (incremento 2).
fn sin_erosion(mut c: TerrenoConfig) -> TerrenoConfig {
    c.erosion.geo_steps = 0;
    c.erosion.run_every = 0;
    c
}

/// Volumen de material erosionado/depositado acumulado en el mundo: `Σ e·a` (m³).
fn vol_e(w: &World) -> f64 {
    (0..w.topo.num_cells).map(|s| w.e[s] * w.topo.area[s]).sum()
}

fn lagos(w: &World) -> usize {
    w.surface
        .iter()
        .filter(|s| matches!(s, Surface::Lago))
        .count()
}

/// Toda la tierra queda en o sobre el nivel del mar y no hay NaN en el relieve ni en el suelo.
fn relieve_sano(w: &World, cfg: &TerrenoConfig) {
    for s in 0..w.topo.num_cells {
        assert!(w.h[s].is_finite() && w.e[s].is_finite() && w.z_soil[s].is_finite());
        assert!(w.z_soil[s] >= 0.0, "suelo negativo en {s}");
        if !w.es_oceano(s) {
            assert!(w.h[s] >= cfg.planet.sea_level, "tierra bajo el mar en {s}");
        }
    }
}

#[test]
fn genesis_conserva_el_material() {
    // Erosionado = depositado + al mar en cada paso, y globalmente Σ e·a + al_mar − relleno = 0
    // (incluidos los repartos del LOD entre rondas).
    let cfg = cfg_geo();
    let (w, g) = worldgen::generate_with_report(&cfg);
    let ero = g.erosion.eroded_m3;
    assert!(ero > 0.0, "la génesis no erosionó nada");
    assert!(
        g.erosion.residual_m3.abs() <= 1e-9 * ero,
        "residual de material por paso: {:.3e} sobre {ero:.3e}",
        g.erosion.residual_m3
    );
    let global = vol_e(&w) + g.erosion.to_ocean_m3 - g.lod_fill_m3;
    assert!(
        global.abs() <= 1e-9 * ero,
        "balance global de material: {global:.3e} sobre {ero:.3e}"
    );
    relieve_sano(&w, &cfg);
}

#[test]
fn genesis_vacia_las_hondonadas_y_respeta_el_mar() {
    // Frente al relieve estático del incremento 2, la génesis elimina la mayoría de los lagos
    // falsos sin cambiar apenas la fracción de tierra.
    let cfg = cfg_geo();
    let estatico = worldgen::generate(&sin_erosion(cfg.clone()));
    let (vivo, g) = worldgen::generate_with_report(&cfg);
    let (antes, despues) = (lagos(&estatico), lagos(&vivo));
    assert!(
        (despues as f64) <= 0.5 * antes as f64,
        "la génesis no vació las hondonadas: {antes} -> {despues} lagos"
    );
    let df = (g.after.land_fraction - g.before.land_fraction).abs();
    assert!(df < 0.03, "la fracción de tierra cambió {df:.3}");
    assert!(
        g.after.elev_mean_land < g.before.elev_mean_land,
        "la erosión no rebajó la tierra"
    );
    relieve_sano(&vivo, &cfg);
}

#[test]
fn paso_enorme_es_estable() {
    // El esquema implícito es incondicionalmente estable: un paso de mil millones de años no da
    // NaN, no hunde tierra bajo el mar y no eleva el relieve por encima de su máximo.
    let mut cfg = sin_erosion(cfg_geo());
    cfg.erosion.run_every = 1; // relieve vivo, sin génesis
    let mut w = worldgen::generate(&cfg);
    let max_antes = w.h.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let (p, t) = erosion::analytic_means(&w, &cfg);
    let rep = erosion::step(&mut w, &cfg, 1e9, &p, &t);
    assert!(rep.eroded_m3 > 0.0);
    assert!(rep.residual_m3.abs() <= 1e-9 * rep.eroded_m3);
    let max_despues = w.h.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    assert!(
        max_despues <= max_antes,
        "el relieve subió: {max_antes} -> {max_despues}"
    );
    relieve_sano(&w, &cfg);
}

#[test]
fn sin_lluvia_no_hay_incision() {
    // Sin precipitación no hay caudal y la potencia de arroyo no excava (la difusión sí actúa).
    let mut cfg = cfg_geo();
    cfg.climate.p_ref = 0.0;
    let (_, g) = worldgen::generate_with_report(&cfg);
    assert_eq!(g.erosion.eroded_m3, 0.0);
    assert_eq!(g.erosion.to_ocean_m3, 0.0);
}

#[test]
fn difusion_sola_conserva_y_no_crea_extremos() {
    // Solo difusión de ladera: el volumen se conserva y la tierra no supera su máximo ni baja de su
    // mínimo (principio del máximo, con los subpasos automáticos).
    let mut cfg = sin_erosion(cfg_geo());
    cfg.erosion.run_every = 1;
    cfg.erosion.k_ero = 0.0;
    cfg.erosion.kappa_dif = 1e4; // fuerte, para que haya subpasos
    let mut w = worldgen::generate(&cfg);
    let tierra: Vec<usize> = (0..w.topo.num_cells).filter(|&s| !w.es_oceano(s)).collect();
    let rango = |w: &World| {
        tierra
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &s| {
                (lo.min(w.h[s]), hi.max(w.h[s]))
            })
    };
    let (lo0, hi0) = rango(&w);
    let v0 = vol_e(&w);
    let (p, t) = erosion::analytic_means(&w, &cfg);
    let rep = erosion::step(&mut w, &cfg, 1e5, &p, &t);
    assert_eq!(rep.eroded_m3, 0.0);
    assert_eq!(rep.to_ocean_m3, 0.0);
    let (lo1, hi1) = rango(&w);
    assert!(
        hi1 <= hi0 && lo1 >= lo0,
        "extremos nuevos: [{lo0},{hi0}] -> [{lo1},{hi1}]"
    );
    let movido: f64 = tierra.iter().map(|&s| w.e[s].abs() * w.topo.area[s]).sum();
    assert!(movido > 0.0, "la difusión no movió nada");
    assert!(
        (vol_e(&w) - v0).abs() <= 1e-9 * movido,
        "la difusión no conserva volumen"
    );
}

#[test]
fn erosion_apagada_no_toca_el_relieve() {
    // No regresión: sin relieve vivo, aunque el LOD cambie la malla, la elevación es la de nacimiento,
    // el suelo sigue a z_init y la capacidad de campo es la de la textura (bit a bit, incr. 2).
    let mut cfg = sin_erosion(cfg_geo());
    cfg.run.total_ticks = 60;
    cfg.lod.n_lod = 5;
    // Génesis uniforme y criterios dinámicos sensibles: la malla se mueve durante la corrida.
    cfg.lod.refine_features = false;
    cfg.lod.dh_sub = 1e30;
    cfg.lod.dt_sub = 1.0;
    cfg.lod.dtheta_sub = 0.1;
    let (doc, w) = abm::terreno::run(&cfg).unwrap();
    assert!(
        doc.ticks
            .iter()
            .any(|t| t.lod_subdivisions + t.lod_collapses > 0)
    );
    for s in 0..w.topo.num_cells {
        assert_eq!(w.e[s].to_bits(), 0.0_f64.to_bits());
        assert_eq!(w.z_soil[s], cfg.soil.z_init);
        assert_eq!(w.w_cap[s].to_bits(), w.w_cap0[s].to_bits());
        assert_eq!(w.h[s].to_bits(), w.h0[s].to_bits());
    }
}

#[test]
fn genesis_determinista() {
    let cfg = cfg_geo();
    let (a, ga) = worldgen::generate_with_report(&cfg);
    let (b, gb) = worldgen::generate_with_report(&cfg);
    assert_eq!(ga, gb);
    assert_eq!(a.topo.keys, b.topo.keys);
    assert_eq!(a.h, b.h);
    assert_eq!(a.e, b.e);
    assert_eq!(a.z_soil, b.z_soil);
}

#[test]
fn lod_ida_y_vuelta_conserva_erosion_y_suelo() {
    // Subdividir todas las hojas y colapsarlas de inmediato devuelve e y z celda a celda (los
    // operadores, sin acondicionamiento); con el acondicionamiento activo, lo que cambia es
    // exactamente el relleno apuntado.
    let cfg = cfg_geo();
    let base = worldgen::generate(&cfg);
    let ida_y_vuelta = |cfg: &TerrenoConfig| {
        let mut w = base.clone();
        let n = w.topo.num_cells;
        let todo = Marks {
            want_sub: vec![true; n],
            may_collapse: vec![false; n],
            verify_collapses: false,
        };
        // La malla de base tiene hojas a l_max: se permite un nivel más para poder subdividirlas.
        w.topo.l_max += 1;
        let r1 = lod::apply(&mut w, cfg, &todo);
        // Se colapsan solo las hijas recién creadas (su madre es una hoja original). Puede llevar
        // varias pasadas: el veto 2:1 es conservador con las familias vecinas que colapsan a la vez.
        let mut relleno = r1.fill_m3;
        for _ in 0..8 {
            let m = w.topo.num_cells;
            let hija_nueva: Vec<bool> = w
                .topo
                .keys
                .iter()
                .map(|&k| {
                    split_key(k).1 > 0 && base.topo.keys.binary_search(&parent_key(k)).is_ok()
                })
                .collect();
            let nada = Marks {
                want_sub: vec![false; m],
                may_collapse: hija_nueva,
                verify_collapses: false,
            };
            let r = lod::apply(&mut w, cfg, &nada);
            relleno += r.fill_m3;
            if !r.changed() {
                break;
            }
        }
        (w, relleno)
    };

    // 1) Operadores puros (sin acondicionamiento): identidad.
    let (w, relleno) = ida_y_vuelta(&sin_erosion(cfg.clone()));
    assert_eq!(relleno, 0.0);
    assert_eq!(
        w.topo.keys, base.topo.keys,
        "la malla no volvió a la original"
    );
    for s in 0..w.topo.num_cells {
        let (e0, e1) = (base.e[s], w.e[s]);
        assert!(
            (e0 - e1).abs() <= 1e-9 * e0.abs().max(1.0),
            "e no vuelve: {e0} vs {e1}"
        );
        assert_eq!(base.z_soil[s], w.z_soil[s], "z no vuelve");
    }

    // 2) Con acondicionamiento: Σ e·a cambia exactamente en el relleno apuntado.
    let (w, relleno) = ida_y_vuelta(&cfg);
    let dv = vol_e(&w) - vol_e(&base);
    assert!(
        (dv - relleno).abs() <= 1e-9 * relleno.max(1.0) + 1e-6 * vol_e(&base).abs(),
        "Δ(Σ e·a) = {dv:.3e} pero relleno = {relleno:.3e}"
    );
}

#[test]
fn erosion_en_ejecucion_conserva_agua_y_material() {
    // Con la erosión en ejecución (acelerada para que se note), el balance de agua sigue cerrando en
    // todos los ticks y cada paso de erosión cierra su balance de material.
    let mut cfg = cfg_geo();
    cfg.run.total_ticks = 40;
    cfg.erosion.run_every = 10;
    cfg.erosion.run_accel = 1e5; // cada paso ≈ 2 700 años
    let (doc, w) = abm::terreno::run(&cfg).unwrap();
    let peor = doc
        .ticks
        .iter()
        .map(|t| t.water_residual_rel.abs())
        .fold(0.0_f64, f64::max);
    assert!(
        peor < 1e-9,
        "el balance de agua no cierra con erosión: {peor:.2e}"
    );
    let pasos: Vec<_> = doc.ticks.iter().filter(|t| t.eroded_m3 > 0.0).collect();
    assert_eq!(pasos.len(), 3, "debía erosionar en los ticks 10, 20 y 30");
    for t in pasos {
        assert!(
            t.sediment_residual_m3.abs() <= 1e-9 * t.eroded_m3,
            "tick {}: residual de material {:.3e}",
            t.tick,
            t.sediment_residual_m3
        );
    }
    relieve_sano(&w, &cfg);
}
