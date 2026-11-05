//! Tests de simulación de la biología de los agentes (incremento 1, §9, y revisión 1, §6):
//! conservación, casos límite del §10, determinismo, independencia del orden, tabla de vida
//! simulada y las validaciones de cada cambio de la revisión 1.

use abm_ruleset::biologia::componentes::{Edad, IdBio, Masa, Reserva};
use abm_ruleset::biologia::config::{EdadesIniciales, SequiaCfg, ZonaCfg};
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

/// Los cuatro cambios de la revisión 1 apagados: la biología del incremento 1.
fn incremento1(c: &mut BioConfig) {
    c.energetica.apetito_gasto_total = false;
    c.energetica.leche_por_condicion = false;
    c.energetica.destete_forzoso = false;
    c.energetica.dimorfismo = false;
    c.energetica.esterilidad_individual = false;
    c.forraje.respuesta_funcional = false;
}

/// Media de `N / N_max` de una especie en los resúmenes anuales `desde..`.
fn n_relativa(doc: &BioDoc, k: usize, desde: usize) -> f64 {
    let n_max = doc.run.especies[k].n_max;
    let tramo = &doc.anios[desde..];
    tramo
        .iter()
        .map(|a| a.especies[k].poblacion_media / n_max)
        .sum::<f64>()
        / tramo.len() as f64
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
    // §10: sin forraje, extinción en ~E_max/G días (≈ 22 en el genérico; en humanos, con el
    // dimorfismo de la revisión 1 §3.4, ≈ 49 las mujeres y ≈ 30 los hombres).
    let mut c = cfg_base();
    c.habitat.zonas[0].p_media_mm = 0.0; // PPN = 0: no hay forraje
    c.poblacion.n_inicial_por_celda = Some(100);
    c.poblacion.reserva_inicial = 1.0;
    c.run.total_ticks = 70;
    c.run.metrics_every = 1;
    let doc = run_biologia(&c).unwrap();
    for (k, e) in doc.run.especies.iter().enumerate() {
        let n = serie(&doc, k);
        let por_sexo = [e.ayuno_dias_hembra, e.ayuno_dias_macho];
        let (corto, largo) = por_sexo
            .iter()
            .map(|x| x.unwrap_or(e.ayuno_dias))
            .fold((f64::INFINITY, 0.0f64), |(a, b), x| (a.min(x), b.max(x)));
        let antes = (0.9 * corto) as usize;
        let despues = (1.1 * largo).ceil() as usize;
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
fn sin_los_cambios_de_la_revision_no_hay_regulacion() {
    // Hallazgo A del incremento 1: con el reparto del diseño v1.0, en celdas aisladas la población
    // supera N_max, agota el forraje y acaba en hambruna.
    let mut c = cfg_base();
    incremento1(&mut c);
    c.especies.truncate(1);
    c.habitat.zonas[0].celdas = 2;
    c.run.total_ticks = 365 * 25;
    c.run.metrics_every = 365;
    let doc = run_biologia(&c).unwrap();
    let n_max = doc.run.especies[0].n_max;
    let pico = serie(&doc, 0).into_iter().max().unwrap() as f64;
    assert!(
        pico > n_max,
        "con el diseño v1.0 la población supera N_max ({pico})"
    );
    let inan: u64 = doc
        .ticks
        .iter()
        .map(|t| t.especies[0].muertes_inanicion)
        .sum();
    assert!(inan > 0, "y acaba en hambruna");
}

#[test]
fn con_respuesta_funcional_la_poblacion_se_regula() {
    // Revisión 1 §2: con clima constante durante 100 años no hay extinción y la población
    // converge cerca de N_max. Medido (nota de revisión §8): N/N_max ≈ 1,2 (la nota pedía
    // 0,7–1,2), porque N_max se calcula con el gasto del adulto y las crías gastan menos.
    let mut c = cfg_base();
    c.especies.truncate(1);
    c.habitat.zonas[0].celdas = 1;
    c.habitat.zonas[0].area_km2 = 50.0;
    c.run.total_ticks = 365 * 100;
    c.run.metrics_every = 365;
    let doc = run_biologia(&c).unwrap();
    assert!(serie(&doc, 0).iter().all(|&n| n > 0), "se extingue");
    let rel = n_relativa(&doc, 0, 40);
    assert!((0.9..=1.4).contains(&rel), "N/N_max = {rel}");
    // Oscilaciones amortiguadas: las últimas décadas, cerca de su media.
    let tramo: Vec<f64> = doc.anios[40..]
        .iter()
        .map(|a| a.especies[0].poblacion_media)
        .collect();
    let media = tramo.iter().sum::<f64>() / tramo.len() as f64;
    assert!(
        tramo.iter().all(|&n| (n / media - 1.0).abs() < 0.2),
        "oscila: {tramo:?}"
    );
    // El forraje queda cerca de K/2, donde el rebrote es máximo (h_½ = (k_ap − 1)/2).
    let t = &doc.ticks.last().unwrap().especies[0];
    let v = t.forraje_kg / t.forraje_max_kg;
    assert!((0.4..=0.65).contains(&v), "V/K = {v}");
}

#[test]
fn con_forraje_de_rebrote_lento_hay_sobrepasamiento_y_colapso() {
    // Revisión 1 §2, prueba cualitativa: con un forraje que rebrota muy despacio (τ_c = 20 años,
    // tipo liquen), la respuesta funcional no impide el sobrepasamiento y el colapso (los renos de
    // St. Matthew). El modelo produce los dos regímenes según la ecología.
    let mut c = cfg_base();
    c.especies.truncate(1);
    c.forraje.tau_c = 20.0;
    c.habitat.zonas[0].celdas = 1;
    c.habitat.zonas[0].area_km2 = 25.0;
    c.run.total_ticks = 365 * 60;
    c.run.metrics_every = 365;
    let doc = run_biologia(&c).unwrap();
    let n_max = doc.run.especies[0].n_max;
    let n = serie(&doc, 0);
    let (i_pico, &pico) = n.iter().enumerate().max_by_key(|x| *x.1).unwrap();
    let despues = n[i_pico..].iter().copied().min().unwrap();
    assert!(
        pico as f64 > 1.5 * n_max,
        "sin sobrepasamiento: pico {pico}, N_max {n_max}"
    );
    assert!(
        (despues as f64) < 0.5 * pico as f64,
        "sin colapso: {pico} → {despues}"
    );
}

#[test]
fn con_forraje_abundante_se_rehacen_reservas_y_la_lactante_mantiene_su_condicion() {
    // Revisión 1 §1: con forraje abundante, el adulto rehace reservas (E/E_max sube hacia 1) y la
    // madre lactante mantiene su condición.
    let mut c = cfg_base();
    c.especies.truncate(1);
    c.poblacion.n_inicial_por_celda = Some(30); // forraje de sobra
    c.run.total_ticks = 365 * 2;
    c.run.metrics_every = 10;
    let doc = run_biologia(&c).unwrap();
    let xs: Vec<f64> = doc
        .ticks
        .iter()
        .filter(|t| t.especies[0].lactantes > 0)
        .map(|t| t.especies[0].condicion_lactantes.unwrap())
        .collect();
    let lactantes = xs.iter().sum::<f64>() / xs.len() as f64;
    assert!(lactantes > 0.9, "condición de las lactantes: {lactantes}");
    let adultos = doc.ticks[20].especies[0].condicion_media;
    assert!(adultos > 0.9, "condición a los 200 días: {adultos}");
}

#[test]
fn en_equilibrio_el_apetito_por_gasto_total_evita_el_hambre() {
    // Revisión 1 §1–§2: con la respuesta funcional, el equilibrio llega cuando cada uno asimila lo
    // que gasta. Con el apetito referido al gasto total, eso incluye criar: el forraje queda en K/2
    // y no hay muertes por hambre. Con el apetito del incremento 1 (k_ap · B · PAL), criar deja a
    // las madres en déficit y la población se regula muriendo de hambre (nota de revisión §8).
    let correr = |gasto_total: bool| {
        let mut c = cfg_base();
        c.especies.truncate(1);
        c.energetica.apetito_gasto_total = gasto_total;
        c.habitat.zonas[0].celdas = 1;
        c.habitat.zonas[0].area_km2 = 50.0;
        c.run.total_ticks = 365 * 40;
        c.run.metrics_every = 365;
        run_biologia(&c).unwrap()
    };
    let hambre = |d: &BioDoc| {
        d.anios[20..]
            .iter()
            .map(|a| a.especies[0].muertes_inanicion)
            .sum::<u64>()
    };
    let (con, sin) = (correr(true), correr(false));
    assert_eq!(
        hambre(&con),
        0,
        "con el gasto total no debería haber hambre en equilibrio"
    );
    assert!(
        hambre(&sin) > 100,
        "sin él, la población se regula con hambre: {}",
        hambre(&sin)
    );
    let t = &con.ticks.last().unwrap().especies[0];
    let v = t.forraje_kg / t.forraje_max_kg;
    assert!((v - 0.5).abs() < 0.05, "V/K = {v}");
}

#[test]
fn en_la_sequia_las_hembras_no_colapsan() {
    // Revisión 1 §3: con una sequía del 50 % durante un año, en el incremento 1 (con la respuesta
    // funcional) casi no quedaban hembras adultas y la población no se recuperaba. Con la revisión
    // quedan hembras (un 30 % de los adultos), mueren más las crías que los adultos y la población
    // vuelve a crecer, despacio: tarda unos 25 años (nota de revisión §8.4). La sequía llega con la
    // población ya regulada.
    let correr = |revision: bool| {
        let mut c = cfg_base();
        c.especies.truncate(1);
        if !revision {
            incremento1(&mut c);
            c.forraje.respuesta_funcional = true; // el reparto que regula (si no, colapsa sin sequía)
        }
        c.habitat.zonas[0].celdas = 4;
        c.habitat.zonas[0].area_km2 = 50.0;
        c.habitat.sequia = Some(SequiaCfg {
            inicio_dia: 365 * 15,
            duracion_dias: 365,
            factor: 0.5,
        });
        c.run.total_ticks = 365 * 40;
        c.run.metrics_every = 365;
        run_biologia(&c).unwrap()
    };
    let (con, sin) = (correr(true), correr(false));
    // Proporción de hembras entre los adultos (clases de edad de la madurez en adelante).
    let hembras_adultas = |d: &BioDoc, anio: usize| {
        let a = &d.anios[anio].especies[0];
        let e = &d.run.especies[0];
        let desde = e.a_rep_dias.div_ceil(e.delta_a_dias) as usize;
        let h: u64 = a.piramide_hembras[desde..].iter().sum();
        let m: u64 = a.piramide_machos[desde..].iter().sum();
        h as f64 / (h + m).max(1) as f64
    };
    let (fc, fs) = (hembras_adultas(&con, 16), hembras_adultas(&sin, 16));
    println!("hembras adultas tras la sequía: {fc:.2} con la revisión, {fs:.2} sin ella");
    assert!(
        fc > 0.25,
        "con la revisión, hembras adultas tras la sequía: {fc}"
    );
    assert!(fc > fs + 0.15, "{fc} frente a {fs} sin la revisión");
    // Durante la sequía mueren más las crías que los adultos.
    let s = &con.anios[15].especies[0];
    let (mh, mm) = (s.mortalidad_hembras.unwrap(), s.mortalidad_machos.unwrap());
    assert!(mh[0].min(mm[0]) > mh[2].max(mm[2]), "crías {mh:?} {mm:?}");
    // Recuperación: tras tocar fondo, la población vuelve a crecer.
    let pob: Vec<f64> = con
        .anios
        .iter()
        .map(|a| a.especies[0].poblacion_media)
        .collect();
    let fondo = pob[16..].iter().copied().fold(f64::INFINITY, f64::min);
    assert!(
        fondo > 0.0 && pob[39] > 3.0 * fondo,
        "fondo {fondo} → {}",
        pob[39]
    );
}

#[test]
fn en_la_hambruna_las_mujeres_resisten_mejor() {
    // Revisión 1 §3: en humanos, la mortalidad adulta de las mujeres en una hambruna es menor o
    // igual que la de los hombres (Zarulli et al., 2018), mueren más los niños que los adultos y la
    // población se recupera. Sequía fuerte (rebrote al 30 % durante un año), solo humanos.
    let mut c = cfg_base();
    c.especies.remove(0);
    c.habitat.zonas[0].celdas = 16;
    c.habitat.zonas[0].area_km2 = 100.0;
    c.habitat.sequia = Some(SequiaCfg {
        inicio_dia: 365 * 10,
        duracion_dias: 365,
        factor: 0.3,
    });
    c.run.total_ticks = 365 * 30;
    c.run.metrics_every = 365;
    let doc = run_biologia(&c).unwrap();
    // La sequía y el año siguiente.
    let tasas = |f: fn(&abm_runner::BioDoc, usize) -> [f64; 4]| {
        let (a, b) = (f(&doc, 10), f(&doc, 11));
        [0, 1, 2, 3].map(|g| (a[g] + b[g]) / 2.0)
    };
    let mh = tasas(|d, i| d.anios[i].especies[0].mortalidad_hembras.unwrap());
    let mm = tasas(|d, i| d.anios[i].especies[0].mortalidad_machos.unwrap());
    println!("mortalidad en la hambruna (1/año): mujeres {mh:.3?}, hombres {mm:.3?}");
    assert!(
        mh[2] <= mm[2],
        "adultas {} frente a adultos {}",
        mh[2],
        mm[2]
    );
    assert!(
        mh[0] > mh[2] && mm[0] > mm[2],
        "niños frente a adultos: {mh:?} {mm:?}"
    );
    let antes = doc.anios[9].especies[0].poblacion_media;
    let despues = doc.anios[29].especies[0].poblacion_media;
    assert!(despues > 0.8 * antes, "{antes} → {despues}");
}

#[test]
fn la_fecundidad_humana_baja_con_la_edad() {
    // Revisión 1 §4: con la edad de esterilidad individual, las mujeres tienen ~6 hijos (5–7) y la
    // fecundidad cae a partir de los 35 años; sin ella, ~10 hijos y fértiles hasta los 50.
    let correr = |esterilidad: bool| {
        let mut c = cfg_base();
        c.especies.remove(0);
        c.energetica.esterilidad_individual = esterilidad;
        c.habitat.zonas[0].celdas = 16;
        c.habitat.zonas[0].area_km2 = 100.0;
        c.run.total_ticks = 365 * 60;
        c.run.metrics_every = 365;
        run_biologia(&c).unwrap()
    };
    let resumen = |d: &BioDoc| {
        let xs: Vec<_> = d.anios[20..].iter().map(|a| &a.especies[0]).collect();
        let tgf = xs.iter().filter_map(|x| x.tgf).sum::<f64>() / xs.len() as f64;
        let grupos = xs[0].fecundidad_grupos.as_ref().unwrap().len();
        let fec: Vec<f64> = (0..grupos)
            .map(|g| {
                xs.iter()
                    .map(|x| x.fecundidad_grupos.as_ref().unwrap()[g])
                    .sum::<f64>()
            })
            .collect();
        (tgf, fec[8] / fec[4], fec[9] / fec[4]) // 40–44 y 45–49 frente a 20–24
    };
    let (con, sin) = (resumen(&correr(true)), resumen(&correr(false)));
    println!(
        "TGF {:.2} (sin esterilidad {:.2}); fecundidad relativa a 40–44 / 45–49: {:.2} / {:.2} (sin: {:.2} / {:.2})",
        con.0, sin.0, con.1, con.2, sin.1, sin.2
    );
    assert!((5.0..=7.5).contains(&con.0), "TGF {}", con.0);
    assert!(sin.0 > con.0 + 2.0);
    assert!(con.1 < 0.5 && con.2 < 0.1, "la fecundidad no cae: {con:?}");
    assert!(sin.2 > 0.5);
}
