//! Tests de la fase «terreno» — incremento 3 (relieve vivo): conservación de material, estabilidad
//! del esquema implícito, relleno de hondonadas, compatibilidad con el LOD, no regresión y
//! determinismo. Diseño: `docs/design/terreno/incremento-3-diseno.md` §11.

use abm_terreno::config::TerrenoConfig;
use abm_terreno::erosion;
use abm_terreno::geometry::{parent_key, split_key};
use abm_terreno::lod::{self, Marks};
use abm_terreno::world::{Surface, World};
use abm_terreno::worldgen;

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
    let (doc, w) = abm_terreno::run(&cfg).unwrap();
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
        let todo = Marks::forced(vec![true; n], vec![false; n]);
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
            let nada = Marks::forced(vec![false; m], hija_nueva);
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
    let (doc, w) = abm_terreno::run(&cfg).unwrap();
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

/// Totales físicos de un mundo: agua almacenada en tierra (m³), material erosionado/depositado
/// acumulado `Σ e·a` (m³) y volumen de suelo en tierra `Σ z·a` (m³).
fn totales(w: &World) -> (f64, f64, f64) {
    let agua = abm_terreno::systems::total_store_vol(w);
    let suelo = (0..w.topo.num_cells)
        .filter(|&s| !w.es_oceano(s))
        .map(|s| w.z_soil[s] * w.topo.area[s])
        .sum();
    (agua, vol_e(w), suelo)
}

/// Avanza `n` ticks desde `t0` con la física y la erosión en ejecución, sin LOD automático.
/// Devuelve la lluvia caída sobre la tierra de la cara 0 (la zona que se afina) en esos ticks (m³).
fn avanzar(
    w: &mut World,
    cfg: &TerrenoConfig,
    sc: &mut abm_terreno::systems::Scratch,
    t0: u64,
    n: u64,
    ero: &mut erosion::ErosionReport,
) -> f64 {
    let every = cfg.erosion.run_every;
    let dt = every as f64 / cfg.climate.t_year * cfg.erosion.run_accel;
    let mut lluvia = 0.0;
    for t in t0..t0 + n {
        let antes = abm_terreno::systems::total_store_vol(w);
        abm_terreno::systems::step(w, cfg, sc, t, antes, &Default::default());
        lluvia += (0..w.topo.num_cells)
            .filter(|&s| w.topo.face(s) == 0 && !w.es_oceano(s))
            .map(|s| w.precip[s] / 1000.0 * w.topo.area[s])
            .sum::<f64>();
        if every > 0 && t > 0 && t % every == 0 {
            let (p, tm) = erosion::running_means(w, cfg);
            ero.add(&erosion::step(w, cfg, dt, &p, &tm));
        }
    }
    lluvia
}

#[test]
fn independencia_del_camino_con_dinamica() {
    // Una misma zona (la cara 0 del cubo) recorrida de tres maneras, con la erosión actuando
    // durante la partida: (a) fina todo el rato; (b) gruesa, luego fina y luego gruesa otra vez;
    // (c) gruesa todo el rato. Dos comprobaciones:
    // - Sin huella: (b), tras volver a gruesa, acaba como (c) salvo por la física de su tramo fino.
    // - (a) frente a (b), la comparación de la revisión.
    // Las celdas de esta prueba son enormes (834 km la gruesa, 417 km la fina): lo que difiere es
    // la física a una u otra escala, no las operaciones de la malla (exactas, ver
    // `lod_ida_y_vuelta_conserva_erosion_y_suelo`). Medido (2026-11-05), (b)/(c) · (a)/(b) · (a)/(c),
    // con la config por defecto (nota de sombra pluviométrica §6.5):
    // - lluvia sobre la tierra de la zona: 2,3 % · 4,5 % · 6,7 %. Sin la nota de sombra: 1,4 % ·
    //   2,3 % · 3,6 %; con k_oro = 40: 1,0 % · 2,0 % · 3,0 %; con la distancia de humedad medida
    //   desde el centro de la celda de mar, como hasta el incremento 3: 6,2 % · 10,5 % · 16 %. Lo sube
    //   k_oro: estas celdas son más gruesas que d_oro y usan su propia pendiente, que cambia con la
    //   resolución (nota de lluvia orográfica §3). A la escala que usará el Foco (de 52 a 26 km) la
    //   lluvia de la zona cambia un 0,00 %;
    // - erosionado: 0,17 % · 2,0 % · 2,1 % (sin la nota de sombra: 1,1 % · 1,9 % · 3,0 %; con
    //   k_oro = 40: 0,7 % · 3,2 % · 3,9 %, lo subía el agotamiento de humedad). La nota de lluvia
    //   orográfica pide vigilarlo;
    // - material Σe·a: 0,01 % · 0,13 %; agua: 0,18 % · 3,0 %; suelo: 2,6 % · 2,7 %.
    // Las tolerancias dejan margen sobre esas cifras.
    let mut cfg = cfg_geo();
    cfg.lod.l_max = cfg.lod.l_min; // génesis en malla uniforme...
    cfg.erosion.run_every = 10;
    cfg.erosion.run_accel = 1e5; // ≈ 2 700 años por paso de erosión
    let mut base = worldgen::generate(&cfg);
    base.topo.l_max = 1; // ...y se permite un nivel más para forzar la zona fina
    let subdividir = |w: &mut World| {
        let n = w.topo.num_cells;
        let cara0: Vec<bool> = (0..n).map(|s| w.topo.face(s) == 0).collect();
        lod::apply(w, &cfg, &Marks::forced(cara0, vec![false; n]))
    };
    let colapsar = |w: &mut World| {
        let n = w.topo.num_cells;
        let finas: Vec<bool> = (0..n)
            .map(|s| w.topo.face(s) == 0 && w.topo.level[s] > 0)
            .collect();
        lod::apply(w, &cfg, &Marks::forced(vec![false; n], finas))
    };
    let tramo = 30;
    let correr = |plan: &str| {
        let mut w = base.clone();
        let mut sc = abm_terreno::systems::Scratch::new(w.topo.num_cells);
        let mut ero = erosion::ErosionReport::default();
        let mut lluvia = 0.0;
        match plan {
            "fina" => {
                subdividir(&mut w);
                sc.resize(w.topo.num_cells);
                lluvia += avanzar(&mut w, &cfg, &mut sc, 0, 3 * tramo, &mut ero);
            }
            "ida_y_vuelta" => {
                lluvia += avanzar(&mut w, &cfg, &mut sc, 0, tramo, &mut ero);
                subdividir(&mut w);
                sc.resize(w.topo.num_cells);
                lluvia += avanzar(&mut w, &cfg, &mut sc, tramo, tramo, &mut ero);
                assert!(colapsar(&mut w).collapses > 0);
                sc.resize(w.topo.num_cells);
                lluvia += avanzar(&mut w, &cfg, &mut sc, 2 * tramo, tramo, &mut ero);
                assert_eq!(w.topo.keys, base.topo.keys, "no volvió a la malla gruesa");
            }
            _ => lluvia += avanzar(&mut w, &cfg, &mut sc, 0, 3 * tramo, &mut ero),
        }
        relieve_sano(&w, &cfg);
        let (agua, mat, suelo) = totales(&w);
        (agua, mat, suelo, ero.eroded_m3, lluvia)
    };
    let (a, b, c) = (correr("fina"), correr("ida_y_vuelta"), correr("gruesa"));
    let rel = |x: f64, y: f64| (x - y).abs() / x.abs().max(y.abs()).max(1e-30);
    type Tot = (f64, f64, f64, f64, f64);
    let informe = |n: &str, x: Tot, y: Tot| {
        let r = (
            rel(x.0, y.0),
            rel(x.1, y.1),
            rel(x.2, y.2),
            rel(x.3, y.3),
            rel(x.4, y.4),
        );
        println!(
            "{n}: agua {:.2e}, Σe·a {:.2e}, suelo {:.2e}, erosionado {:.2e}, lluvia {:.2e}",
            r.0, r.1, r.2, r.3, r.4
        );
        r
    };
    // Sin huella: (b) frente a (c).
    let bc = informe("(b) vs (c)", b, c);
    assert!(bc.0 < 0.01, "huella en el agua: {:.2e}", bc.0);
    assert!(bc.1 < 2e-3, "huella en el material: {:.2e}", bc.1);
    assert!(bc.2 < 0.04, "huella en el suelo: {:.2e}", bc.2);
    assert!(bc.3 < 0.02, "huella en la erosión: {:.2e}", bc.3);
    assert!(bc.4 < 0.03, "huella en la lluvia: {:.2e}", bc.4);
    // (a) frente a (b), la comparación de la revisión, y (a) frente a (c), fina frente a gruesa.
    let ab = informe("(a) vs (b)", a, b);
    assert!(ab.0 < 0.04, "agua: {:.2e}", ab.0);
    assert!(ab.1 < 3e-3, "material: {:.2e}", ab.1);
    assert!(ab.2 < 0.05, "suelo: {:.2e}", ab.2);
    assert!(ab.3 < 0.04, "erosionado: {:.2e}", ab.3);
    assert!(ab.4 < 0.06, "lluvia: {:.2e}", ab.4);
    let ac = informe("(a) vs (c)", a, c);
    assert!(ac.4 < 0.08, "lluvia, fina frente a gruesa: {:.2e}", ac.4);
}

#[test]
fn el_relieve_orografico_no_depende_de_la_malla() {
    // Nota de lluvia orográfica §3: en un punto del planeta, el relieve de referencia de la lluvia
    // (el de nacimiento al nivel L_oro más la erosión vista a ese nivel) es el mismo esté la zona en
    // malla fina o gruesa. Con n0 = 12 los lados son 834 (L0), 417 (L1), 208 (L2) y 104 km (L3), así
    // que d_oro = 400 km da L_oro = 1 y hay celdas más finas que la referencia.
    let mut cfg = cfg_geo();
    cfg.lod.l_max = 3;
    cfg.climate.d_oro_km = 400.0;
    let a = worldgen::generate(&cfg);
    assert_eq!(a.statgen.oro_level(), 1);
    assert!(
        a.e.iter().any(|&x| x != 0.0),
        "la génesis debe dejar erosión"
    );
    // B: la cara 0 un nivel más fina. Sin acondicionar las hondonadas nuevas, para que las hijas
    // copien `e` tal cual (el acondicionamiento cambia el relieve, y eso sí es otro relieve).
    let mut b = a.clone();
    let n = b.topo.num_cells;
    let marcar: Vec<bool> = (0..n)
        .map(|s| b.topo.face(s) == 0 && (1..3).contains(&b.topo.level[s]))
        .collect();
    let rep = lod::apply(
        &mut b,
        &sin_erosion(cfg.clone()),
        &Marks::forced(marcar, vec![false; n]),
    );
    assert!(rep.subdivisions > 0);
    let (mut peor, mut usadas, mut finas) = (0.0f64, 0, 0);
    for s in (0..b.topo.num_cells).filter(|&s| b.topo.face(s) == 0) {
        let t = &b.topo;
        let ga = erosion::orographic_gradient_at(&a, t.dir[s], t.east[s], t.north[s]);
        let gb = erosion::orographic_gradient_at(&b, t.dir[s], t.east[s], t.north[s]);
        let escala = ga.0.abs().max(ga.1.abs()).max(1e-9);
        peor = peor.max((ga.0 - gb.0).abs().max((ga.1 - gb.1).abs()) / escala);
        if b.statgen.uses_oro_reference(u32::from(t.level[s])) {
            usadas += 1;
            finas += usize::from(t.level[s] > 1);
            // Y es justo lo que usa la lluvia de la celda.
            assert!((b.oro_e[s] - gb.0).abs() <= 1e-12 * gb.0.abs().max(1e-9));
            assert!((b.oro_n[s] - gb.1).abs() <= 1e-12 * gb.1.abs().max(1e-9));
        }
    }
    assert!(
        usadas > 0 && finas > 0,
        "{usadas} celdas con referencia, {finas} más finas"
    );
    assert!(
        peor < 1e-9,
        "el relieve de referencia cambia con la malla: {peor:.2e}"
    );
}

#[test]
fn acondicionamiento_conserva_el_material() {
    // Al subdividir un relieve ya erosionado, el detalle fino trae hondonadas: se rellenan con
    // material de su borde, así que Σ e·a no cambia (lo no compensado es solo redondeo).
    let cfg = cfg_geo();
    let base = worldgen::generate(&cfg);
    let mut w = base.clone();
    w.topo.l_max += 1;
    let n = w.topo.num_cells;
    let r = lod::apply(&mut w, &cfg, &Marks::forced(vec![true; n], vec![false; n]));
    println!(
        "movido {:.3e} m³, no compensado {:.3e} m³, lagos {} -> {}",
        r.conditioned_m3,
        r.fill_m3,
        lagos(&base),
        lagos(&w)
    );
    assert!(
        r.conditioned_m3 > 0.0,
        "no hubo hondonadas que acondicionar"
    );
    assert!(
        r.fill_m3.abs() <= 1e-9 * r.conditioned_m3,
        "se creó material: {:.3e}",
        r.fill_m3
    );
    let dv = vol_e(&w) - vol_e(&base);
    assert!(
        dv.abs() <= 1e-9 * vol_e(&base).abs(),
        "Σ e·a cambió {dv:.3e} m³ al subdividir"
    );
    relieve_sano(&w, &cfg);
}

#[test]
fn el_tope_deja_pasar_las_mas_urgentes() {
    // Con un criterio de humedad muy sensible casi todo pide subdividirse; el tope por revisión deja
    // pasar solo las peticiones más urgentes, como mucho `max_sub_frac · hojas`.
    let mut cfg = sin_erosion(cfg_geo());
    cfg.lod.l_max = 2;
    cfg.lod.refine_features = false;
    cfg.lod.dh_sub = 1e30;
    cfg.lod.dtheta_sub = 1e-3;
    cfg.lod.max_sub_frac = 0.01;
    let mut w = worldgen::generate(&cfg);
    let mut sc = abm_terreno::systems::Scratch::new(w.topo.num_cells);
    for t in 0..20 {
        let antes = abm_terreno::systems::total_store_vol(&w);
        abm_terreno::systems::step(&mut w, &cfg, &mut sc, t, antes, &Default::default());
    }
    let mut m = lod::marks(&w, &cfg);
    let piden = m.want_sub.iter().filter(|&&x| x).count();
    let urg = m.urgency.clone();
    lod::cap_subdivisions(&mut m, cfg.lod.max_sub_frac);
    let tope = (cfg.lod.max_sub_frac * w.topo.num_cells as f64).ceil() as usize;
    let pasan: Vec<usize> = (0..w.topo.num_cells).filter(|&s| m.want_sub[s]).collect();
    assert!(
        piden > tope,
        "el caso no ejercita el tope ({piden} peticiones, tope {tope})"
    );
    assert_eq!(pasan.len(), tope);
    let min_pasa = pasan.iter().map(|&s| urg[s]).fold(f64::INFINITY, f64::min);
    let max_queda = (0..w.topo.num_cells)
        .filter(|&s| urg[s] > 0.0 && !m.want_sub[s])
        .map(|s| urg[s])
        .fold(0.0_f64, f64::max);
    assert!(
        min_pasa >= max_queda,
        "pasó una petición menos urgente que otra rechazada"
    );
}

#[test]
fn la_cache_reproduce_la_genesis_bit_a_bit() {
    // La génesis guardada en disco y vuelta a leer da exactamente la misma corrida que calcularla;
    // la clave cambia con los parámetros de la génesis pero no con los de la corrida.
    use abm_terreno::cache;
    let mut cfg = cfg_geo();
    // 12 ticks: cruza un paso de LOD (n_lod = 10), así que el mundo leído de la caché tiene que
    // reconstruir los anillos del agotamiento de humedad (no se guardan) y dar lo mismo.
    cfg.run.total_ticks = 12;
    let dir = std::env::temp_dir().join(format!("abm_cache_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    cfg.run.genesis_cache = dir.to_string_lossy().into_owned();

    let (d1, w1, i1) = abm_terreno::run_with_info(&cfg).unwrap();
    let (d2, w2, i2) = abm_terreno::run_with_info(&cfg).unwrap();
    let mut sin = cfg.clone();
    sin.run.genesis_cache = String::new();
    let (d0, w0) = abm_terreno::run(&sin).unwrap();
    assert!(
        !i1.from_cache && i2.from_cache,
        "la segunda corrida debía leer la caché"
    );
    assert_eq!(d1, d0);
    assert_eq!(
        d2, d0,
        "con la génesis de la caché la corrida no es idéntica"
    );
    for (a, b) in [(&w0, &w1), (&w0, &w2)] {
        assert_eq!(a.topo.keys, b.topo.keys);
        assert_eq!(a.h, b.h);
        assert_eq!(a.e, b.e);
        assert_eq!(a.z_soil, b.z_soil);
        assert_eq!(a.w, b.w);
        assert_eq!(a.w_hum, b.w_hum);
        assert_eq!(a.c_p.to_bits(), b.c_p.to_bits());
    }
    assert!(w0.humedad_activa && w0.w_hum.iter().any(|&x| x < 1.0));

    let mut otro = cfg.clone();
    otro.erosion.k_ero *= 2.0;
    assert_ne!(cache::key(&cfg).unwrap(), cache::key(&otro).unwrap());
    let mut mas_ticks = cfg.clone();
    mas_ticks.run.total_ticks = 99;
    assert_eq!(cache::key(&cfg).unwrap(), cache::key(&mas_ticks).unwrap());

    // Un fichero dañado se ignora y se recalcula.
    let p = cache::path(&cfg).unwrap().unwrap();
    std::fs::write(&p, b"basura").unwrap();
    let (d3, _, i3) = abm_terreno::run_with_info(&cfg).unwrap();
    assert!(!i3.from_cache);
    assert_eq!(d3, d0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn en_un_mundo_plano_la_sombra_pluviometrica_no_cambia_nada() {
    // Nota de sombra pluviométrica §4: con la tierra llana, a nivel del mar, w = 1 y m_oro = 1 en
    // todas partes, así que la corrida es idéntica a la de sin efecto orográfico (k_oro = 0). El
    // mar plano va en las dos: si el fondo marino contara como relieve, movería la lluvia de la
    // costa (es lo único que distingue esta comparación de una con el código de antes de la nota).
    let mut cfg = cfg_geo();
    cfg.planet.h_max = 0.0; // toda la tierra a 0 m
    cfg.climate.d_oro_km = 200.0; // anillos pequeños (la propiedad no depende de d_oro)
    cfg.run.total_ticks = 25;
    let (da, wa) = abm_terreno::run(&cfg).unwrap();
    assert!(wa.humedad_activa);
    assert!(
        wa.w_hum.iter().all(|&x| x == 1.0),
        "w < 1 en un mundo plano"
    );
    assert_eq!(wa.c_p, 1.0);
    assert!(
        wa.e.iter().all(|&x| x == 0.0),
        "la erosión no debía tocar un mundo plano"
    );
    let mut sin = cfg.clone();
    sin.climate.k_oro = 0.0;
    sin.climate.agotamiento_humedad = false;
    sin.climate.conservar_lluvia = false;
    let (db, wb) = abm_terreno::run(&sin).unwrap();
    assert_eq!(da.ticks, db.ticks);
    assert_eq!(wa.topo.keys, wb.topo.keys);
    assert_eq!(wa.precip, wb.precip);
}
