//! Tests de simulación de la biología de los agentes (incremento 1, §9): conservación, casos
//! límite del §10, determinismo, independencia del orden y tabla de vida simulada.

use abm_ruleset::biologia::componentes::{Edad, IdBio, Masa, Reserva};
use abm_ruleset::biologia::config::{EdadesIniciales, RespuestaForraje, ZonaCfg};
use abm_ruleset::biologia::especie::Especie;
use abm_runner::{BioConfig, BioDoc, run_biologia, run_biologia_con_orden};

/// Hábitat pequeño (4 celdas de 50 km², templado), las dos especies por defecto y 3 años.
fn cfg_base() -> BioConfig {
    let mut c = BioConfig::default();
    c.run.total_ticks = 365 * 3;
    c.run.metrics_every = 30;
    c.habitat.zonas = vec![ZonaCfg {
        nombre: "templada".into(),
        celdas: 4,
        area_km2: 50.0,
        t_media: 10.0,
        p_media_mm: 700.0,
    }];
    c
}

/// Población de una especie tras cada registro.
fn serie(doc: &BioDoc, k: usize) -> Vec<u64> {
    doc.ticks.iter().map(|t| t.especies[k].n).collect()
}

#[test]
fn el_balance_de_energia_cierra() {
    // §8: asimilada − gasto − pérdidas − reservas de los muertos = Δ reservas, a redondeo.
    let doc = run_biologia(&cfg_base()).unwrap();
    assert!(
        doc.ticks
            .iter()
            .any(|t| t.especies.iter().any(|e| e.nacimientos > 0))
    );
    for t in &doc.ticks {
        let b = &t.balance;
        let escala = b.asimilada.abs() + b.reservas_muertos.abs() + 1.0;
        assert!(
            b.residuo.abs() <= 1e-12 * escala + 1e-3,
            "tick {}: residuo {:.3e} kcal ({b:?})",
            t.tick,
            b.residuo
        );
        assert!(b.perdida_leche >= -1e-6 && b.perdida_almacen >= 0.0 && b.gasto_propio > 0.0);
        assert!(b.excedente.abs() < 1e-3, "excedente {}", b.excedente);
        assert_eq!(t.colisiones_id, 0);
    }
}

#[test]
fn sin_forraje_se_extinguen_en_su_ayuno() {
    // §10: sin forraje, extinción en ~E_max/G días (≈ 40 en humanos, ≈ 22 en el genérico).
    let mut c = cfg_base();
    c.habitat.zonas[0].p_media_mm = 0.0; // PPN = 0: no hay forraje
    c.poblacion.n_inicial_por_celda = Some(100);
    c.poblacion.reserva_inicial = 1.0;
    c.run.total_ticks = 60;
    c.run.metrics_every = 1;
    let doc = run_biologia(&c).unwrap();
    for (k, e) in doc.run.especies.iter().enumerate() {
        let n = serie(&doc, k);
        let ayuno = e.ayuno_dias;
        let antes = (0.9 * ayuno) as usize;
        let despues = (1.1 * ayuno).ceil() as usize;
        assert!(n[antes] > 0, "{}: nadie llega al día {antes}", e.nombre);
        assert_eq!(n[despues], 0, "{}: quedan vivos el día {despues}", e.nombre);
        let inan: u64 = doc
            .ticks
            .iter()
            .map(|t| t.especies[k].muertes_inanicion)
            .sum();
        assert!(inan as f64 >= 0.9 * e.poblacion_inicial as f64);
    }
}

#[test]
fn sin_machos_no_hay_nacimientos() {
    // §10: sin machos maduros no hay concepciones.
    let mut c = cfg_base();
    for e in c.especies.iter_mut() {
        e.ajustes.rho_sexo = Some(0.0);
    }
    let doc = run_biologia(&c).unwrap();
    for t in &doc.ticks {
        for e in &t.especies {
            assert_eq!((e.machos, e.concepciones, e.nacimientos), (0, 0, 0));
        }
    }
    assert!(doc.ticks[0].especies.iter().all(|e| e.n > 0));
}

#[test]
fn con_k_ap_menor_que_uno_no_se_rehacen_reservas() {
    // §10: con k_ap ≤ 1 el apetito no cubre el gasto y las reservas no se rehacen (regla de
    // estabilidad generalizada; con D1, el apetito se mide en energía asimilable).
    let correr = |k_ap: f64| {
        let mut c = cfg_base();
        c.especies.truncate(1);
        c.especies[0].ajustes.k_ap = Some(k_ap);
        c.especies[0].ajustes.rho_sexo = Some(0.0); // sin reproducción: solo energía
        c.poblacion.n_inicial_por_celda = Some(30); // forraje de sobra
        c.run.total_ticks = 200;
        c.run.metrics_every = 1;
        run_biologia(&c).unwrap()
    };
    let (malo, bueno) = (correr(0.9), correr(2.0));
    let cond = |d: &BioDoc, dia: usize| d.ticks[dia].especies[0].condicion_media;
    assert!(
        cond(&malo, 60) < 0.45,
        "k_ap 0,9: condición {}",
        cond(&malo, 60)
    );
    assert!(
        cond(&bueno, 60) > 0.9,
        "k_ap 2: condición {}",
        cond(&bueno, 60)
    );
    let inan = |d: &BioDoc| {
        d.ticks
            .iter()
            .map(|t| t.especies[0].muertes_inanicion)
            .sum::<u64>()
    };
    assert!(inan(&malo) > 0);
    assert_eq!(inan(&bueno), 0);
}

#[test]
fn misma_semilla_misma_corrida() {
    let c = cfg_base();
    let a = run_biologia(&c).unwrap();
    let b = run_biologia(&c).unwrap();
    assert_eq!(a, b);
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&b).unwrap()
    );
    let mut otra = c.clone();
    otra.run.seed += 1;
    assert_ne!(a.ticks, run_biologia(&otra).unwrap().ticks);
}

#[test]
fn el_resultado_no_depende_del_orden_de_proceso() {
    // §10 "prueba de orden": la población inicial se crea en otro orden (la ECS la recorre en otro
    // orden) y todo debe salir idéntico bit a bit: azar por contador y sumas exactas (D12–D13).
    let c = cfg_base();
    let (a, mut wa) = run_biologia_con_orden(&c, None).unwrap();
    let (b, mut wb) = run_biologia_con_orden(&c, Some(12345)).unwrap();
    assert_eq!(a, b);
    let estado = |w: &mut bevy_ecs::world::World| {
        let mut v: Vec<(u64, u32, u64, u64)> = w
            .query::<(&IdBio, &Edad, &Masa, &Reserva)>()
            .iter(w)
            .map(|(i, e, m, r)| (i.0, e.0, m.0.to_bits(), r.0.to_bits()))
            .collect();
        v.sort_unstable();
        v
    };
    let (ea, eb) = (estado(&mut wa), estado(&mut wb));
    assert!(!ea.is_empty());
    assert_eq!(ea, eb);
}

#[test]
fn tabla_de_vida_simulada_de_una_cohorte() {
    // Una cohorte de recién nacidos del genérico, con comida de sobra y sin reproducción: su
    // supervivencia debe seguir a la de Siler (incremento 1 §4), dentro del error de muestreo.
    let mut c = cfg_base();
    c.especies.truncate(1);
    c.especies[0].ajustes.rho_sexo = Some(0.0);
    c.habitat.zonas[0].celdas = 4;
    c.habitat.zonas[0].area_km2 = 100.0;
    c.habitat.zonas[0].t_media = 25.0;
    c.habitat.zonas[0].p_media_mm = 2500.0;
    c.poblacion.n_inicial_por_celda = Some(500);
    c.poblacion.edades_iniciales = EdadesIniciales::Cero;
    c.run.total_ticks = 365 * 12;
    c.run.metrics_every = 1;
    let doc = run_biologia(&c).unwrap();
    let n0 = doc.run.especies[0].poblacion_inicial as f64;
    let l_sim: Vec<f64> = serie(&doc, 0).iter().map(|&n| n as f64 / n0).collect();
    let e = Especie::derivar(&c, &c.especies[0]).unwrap();
    // l_teo[t] = supervivencia al final del día t (la edad al empezar el día t es t).
    let lt = e.supervivencia();
    let l_teo = |t: usize| lt[t + 1];
    let anio = 365;
    assert!(
        (l_sim[anio - 1] - l_teo(anio - 1)).abs() < 0.035,
        "l(1): {} vs {}",
        l_sim[anio - 1],
        l_teo(anio - 1)
    );
    let e0_sim: f64 = l_sim.iter().sum::<f64>() / 365.0;
    let e0_teo: f64 = (0..l_sim.len()).map(l_teo).sum::<f64>() / 365.0;
    assert!(
        (e0_sim - e0_teo).abs() < 0.35,
        "e0 (12 años): {e0_sim} vs {e0_teo}"
    );
    let otras: u64 = doc
        .ticks
        .iter()
        .map(|t| t.especies[0].muertes_inanicion + t.especies[0].muertes_orfandad)
        .sum();
    assert_eq!(otras, 0, "con comida de sobra solo hay muertes intrínsecas");
}

#[test]
fn con_respuesta_proporcional_la_poblacion_se_regula() {
    // Propuesta D16: con el consumo proporcional al stock, la población del genérico se estabiliza
    // cerca de N_max; con el diseño tal cual, en celdas aisladas, agota el forraje y colapsa.
    let correr = |r: RespuestaForraje| {
        let mut c = cfg_base();
        c.especies.truncate(1);
        c.forraje.respuesta = r;
        c.habitat.zonas[0].celdas = 2;
        c.run.total_ticks = 365 * 25;
        c.run.metrics_every = 365;
        run_biologia(&c).unwrap()
    };
    let prop = correr(RespuestaForraje::Proporcional);
    let n_max = prop.run.especies[0].n_max;
    let tramo: Vec<f64> = serie(&prop, 0)[15..].iter().map(|&n| n as f64).collect();
    let media = tramo.iter().sum::<f64>() / tramo.len() as f64;
    assert!(
        (0.8 * n_max..=1.6 * n_max).contains(&media),
        "media {media} frente a N_max {n_max}"
    );
    assert!(
        tramo.iter().all(|&n| (n / media - 1.0).abs() < 0.15),
        "oscila demasiado: {tramo:?}"
    );
    let total = correr(RespuestaForraje::Total);
    let pico = serie(&total, 0).into_iter().max().unwrap() as f64;
    assert!(
        pico > n_max,
        "con el diseño tal cual la población supera N_max ({pico})"
    );
    let inan: u64 = total
        .ticks
        .iter()
        .map(|t| t.especies[0].muertes_inanicion)
        .sum();
    assert!(inan > 0, "y acaba en hambruna");
}
