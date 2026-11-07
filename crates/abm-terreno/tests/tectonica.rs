//! Relieve tectónico (incremento 4): pruebas de la deriva de placas y del relieve que deja.
//!
//! Diseño: `docs/design/terreno/diseno_relieve_tectonico.md` §10 y `incremento-4-diseno.md`.
//! Las pruebas corren en mallas gruesas y derivas cortas para que quepan en `cargo test` (debug).

use abm_terreno::config::TectonicaCfg;
use abm_terreno::geometry::lat_lon;
use abm_terreno::tectonica::deriva::{Deriva, TipoBorde};
use abm_terreno::tectonica::malla::{Malla, NADA};

/// Adenda de balance v3 §6.6b: `H_max = 80 km` ya no es una regla, solo una alarma. Ninguna celda
/// debe pasarlo; si alguna se acerca a los 150 km vistos antes del tope, algún proceso apila corteza.
/// Se comprueba en mallas de 52 km o más finas (aquí, el choque India–Asia; en el planeta, la batería).
const ALARMA_ESPESOR_KM: f64 = 80.0;

/// Mapa en texto (equirectangular) de las placas: mayúscula = continental, minúscula = oceánica.
fn mapa(d: &Deriva) -> String {
    let (w, h) = (96usize, 36usize);
    let mut out = String::new();
    let letras: Vec<char> = "abcdefghijklmnopqrstuvwxyz".chars().collect();
    for fila in 0..h {
        let lat = 90.0 - (fila as f64 + 0.5) * 180.0 / h as f64;
        for col in 0..w {
            let lon = -180.0 + (col as f64 + 0.5) * 360.0 / w as f64;
            let (la, lo) = (lat.to_radians(), lon.to_radians());
            let dir = [la.cos() * lo.cos(), la.cos() * lo.sin(), la.sin()];
            let k = d.malla.localizar(dir);
            let p = d.dueno[k];
            let ch = if p == NADA {
                ' '
            } else {
                let c = letras[p as usize % letras.len()];
                if d.corteza(k).is_some_and(|c| c.cont) {
                    c.to_ascii_uppercase()
                } else {
                    c
                }
            };
            out.push(ch);
        }
        out.push('\n');
    }
    out
}

/// Mapa en texto de una zona (latitudes y longitudes en grados, paso en grados).
fn mapa_zona(d: &Deriva, lat0: f64, lat1: f64, lon0: f64, lon1: f64, paso: f64) -> String {
    let letras: Vec<char> = "abcdefghijklmnopqrstuvwxyz".chars().collect();
    let mut out = String::new();
    let mut lat = lat1;
    while lat > lat0 {
        let mut lon = lon0;
        while lon < lon1 {
            let (la, lo) = (lat.to_radians(), lon.to_radians());
            let k = d
                .malla
                .localizar([la.cos() * lo.cos(), la.cos() * lo.sin(), la.sin()]);
            let p = d.dueno[k];
            out.push(if p == NADA {
                ' '
            } else if d.corteza(k).is_some_and(|c| c.cont) {
                letras[p as usize % 26].to_ascii_uppercase()
            } else {
                letras[p as usize % 26]
            });
            lon += paso;
        }
        out.push('\n');
        lat -= paso;
    }
    out
}

#[test]
#[ignore = "exploratoria: cargo test --release -p abm-terreno --test tectonica -- --ignored --nocapture"]
fn exploracion_de_la_deriva() {
    let nivel: u32 = std::env::var("NIVEL")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(0);
    let pasos: u64 = std::env::var("PASOS")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(200);
    let cfg = TectonicaCfg::default();
    let malla = Malla::new(48, nivel, 6371.0);
    let t0 = std::time::Instant::now();
    let mut d = Deriva::nueva(&cfg, 42, malla);
    println!("{}", mapa(&d));
    for i in 0..pasos {
        d.dar_paso();
        if (i + 1) % 25 == 0 {
            let m = d.metricas.last().expect("métrica");
            println!(
                "t={:6.0} placas={:2} v={:5.2} dors={:6.0} fosas={:6.0} col={:6.0} edad={:5.1}/{:5.1} cont={:.3} V={:.3e} super={:.2} zeta={:6.0} prod={:.2} km²/a ({:.1} s)",
                m.myr,
                m.n_placas,
                m.v_rms_cm_anio,
                m.dorsales_km,
                m.fosas_km,
                m.colisiones_km,
                m.edad_media_oceano_myr,
                m.edad_max_oceano_myr,
                m.area_continental_frac,
                m.volumen_continental_km3,
                m.supercontinente,
                m.nivel_mar_m,
                d.area_nueva_km2 / 1e6,
                t0.elapsed().as_secs_f64()
            );
        }
        if (i + 1) % 25 == 0 {
            use abm_terreno::tectonica::deriva::TipoBorde;
            let mut cin = 0.0;
            let mut largo = [0.0f64; 4];
            for b in &d.bordes {
                let i = b.tipo.codigo() as usize - 1;
                largo[i] += 0.5 * b.largo_km;
                if b.tipo == TipoBorde::Divergente {
                    cin += 0.5 * (-b.v_n) * b.largo_km;
                }
            }
            println!(
                "      producción cinemática {:.2} km²/a · bordes div/sub/col/tr = {:.0}/{:.0}/{:.0}/{:.0} km",
                cin / 1e6,
                largo[0],
                largo[1],
                largo[2],
                largo[3]
            );
        }
        if (i + 1) % 250 == 0 && std::env::var("MAPA").is_ok() {
            println!("{}", mapa(&d));
        }
        if std::env::var("ZOOM")
            .ok()
            .and_then(|x| x.parse::<u64>().ok())
            == Some(i + 1)
        {
            println!("{}", mapa_zona(&d, -10.0, 30.0, -60.0, 20.0, 0.45));
        }
    }
    let mut tipos: Vec<(String, usize)> = Vec::new();
    for e in &d.eventos {
        match tipos.iter_mut().find(|t| t.0 == e.tipo) {
            Some(t) => t.1 += 1,
            None => tipos.push((e.tipo.clone(), 1)),
        }
    }
    println!("eventos: {tipos:?}");
    println!("cuentas: {:?}", d.cuentas);
    let (t, r) = abm_terreno::tectonica::estado_final(&d);
    let a: f64 = t.malla.area.iter().sum();
    let (mut at, mut ht, mut am, mut pm) = (0.0, 0.0, 0.0, 0.0);
    let mut alt: Vec<(f64, f64)> = Vec::new();
    for k in 0..t.malla.num {
        let z = t.z_base_m[k];
        if z >= 0.0 {
            at += t.malla.area[k];
            ht += z * t.malla.area[k];
            alt.push((z, t.malla.area[k]));
        } else {
            am += t.malla.area[k];
            pm += -z * t.malla.area[k];
        }
    }
    alt.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut acc = 0.0;
    let mut med = 0.0;
    for &(z, w) in &alt {
        acc += w;
        if acc >= 0.5 * at {
            med = z;
            break;
        }
    }
    println!(
        "ζ = {:.0} m · tierra {:.1} % · media {:.0} m · mediana {:.0} m · océano {:.0} m · f_cont {:.3} · edades media/mediana/max {:.0}/{:.0}/{:.0} · v_rms media {:.2} · FU fosa {:.2} cont {:.2}",
        t.zeta_m,
        100.0 * at / a,
        ht / at,
        med,
        pm / am,
        r.f_cont_final,
        r.edades_oceano.media_myr,
        r.edades_oceano.mediana_myr,
        r.edades_oceano.max_myr,
        r.v_rms_media_cm_anio,
        r.forsyth_uyeda_fosa,
        r.forsyth_uyeda_continente
    );
    let _ = lat_lon;
}

fn corteza(cont: bool, h: f64, edad: f64) -> abm_terreno::tectonica::deriva::Corteza {
    use abm_terreno::tectonica::deriva::Corteza;
    if cont {
        Corteza {
            edad_orog: 0.0,
            ..Corteza::continental(h)
        }
    } else {
        Corteza {
            h,
            edad_oc: edad,
            ..Corteza::oceanica(h)
        }
    }
}

#[test]
#[ignore = "exploratoria"]
fn una_placa_que_gira_conserva_su_forma() {
    let cfg = TectonicaCfg::default();
    let malla = Malla::new(12, 2, 6371.0);
    let num = malla.num;
    let dirs = malla.dir.clone();
    // Una placa con un casquete continental de 40° alrededor de +X; el resto, otra placa quieta.
    let centro = [1.0, 0.0, 0.0];
    let en_cap = |d: [f64; 3]| {
        d[0] * centro[0] + d[1] * centro[1] + d[2] * centro[2] > 40f64.to_radians().cos()
    };
    let mut d = Deriva::desde(&cfg, 1, malla, -100.0, &[10, 20], |k| {
        if en_cap(dirs[k]) {
            (0, corteza(true, 35.0, 0.0))
        } else {
            (1, corteza(false, 7.0, 50.0))
        }
    });
    // Gira alrededor de +Z: 5 cm/año en el ecuador = 50 km/Myr.
    let w = 50.0 / 6371.0;
    d.placas[0].omega = [0.0, 0.0, w];
    d.placas[1].omega = [0.0, 0.0, w];
    for paso in 1..=200 {
        d.paso_cinematico();
        if paso % 50 == 0 {
            let ang = w * paso as f64;
            let c2 = [ang.cos(), ang.sin(), 0.0];
            let (mut malos, mut huecos) = (0, 0);
            for k in 0..num {
                let x = d.malla.dir[k];
                let esperado =
                    x[0] * c2[0] + x[1] * c2[1] + x[2] * c2[2] > 40f64.to_radians().cos();
                let es = d.corteza(k).is_some_and(|c| c.cont);
                if esperado != es {
                    malos += 1;
                }
                if d.dueno[k] == NADA {
                    huecos += 1;
                }
            }
            let n0: usize = d.placas.iter().map(|p| p.cort.len()).sum();
            println!(
                "paso {paso}: celdas mal {malos} de {num}, huecos {huecos}, muestras {n0}, nuevas {:.0} km²",
                d.area_nueva_km2
            );
        }
    }
}

#[test]
#[ignore = "exploratoria"]
fn dos_placas_que_se_separan_hacen_una_dorsal_limpia() {
    let cfg = TectonicaCfg::default();
    let malla = Malla::new(12, 2, 6371.0);
    let num = malla.num;
    let dirs = malla.dir.clone();
    let mut d = Deriva::desde(&cfg, 1, malla, -100.0, &[10, 20], |k| {
        if dirs[k][1] >= 0.0 {
            (0, corteza(false, 7.0, 50.0))
        } else {
            (1, corteza(false, 7.0, 50.0))
        }
    });
    // Se separan girando en sentidos opuestos alrededor de +X: 2,5 cm/año cada una.
    let w = 25.0 / 6371.0;
    d.placas[0].omega = [w, 0.0, 0.0];
    d.placas[1].omega = [-w, 0.0, 0.0];
    for paso in 1..=40 {
        d.paso_cinematico();
        if paso % 10 == 0 {
            let mut edades = [0.0; 9];
            let mut cuenta = [0usize; 9];
            let mut mezcla = 0;
            for k in 0..num {
                let x = d.malla.dir[k];
                if x[0].abs() > 0.3 {
                    continue;
                }
                // distancia angular al plano de la dorsal (y = 0 girado)... se mide por la edad.
                if let Some(c) = d.corteza(k) {
                    let b = ((c.edad_oc / 5.0) as usize).min(8);
                    edades[b] += 1.0;
                    cuenta[b] += 1;
                }
                let p = d.dueno[k];
                if d.malla.vec[k]
                    .iter()
                    .filter(|&&v| v != NADA && d.dueno[v as usize] != p)
                    .count()
                    >= 4
                {
                    mezcla += 1;
                }
            }
            let a0: usize = (0..num).filter(|&k| d.dueno[k] == 0).count();
            let a1: usize = (0..num).filter(|&k| d.dueno[k] == 1).count();
            println!(
                "paso {paso}: celdas placa 0/1 = {a0}/{a1}, celdas con >=4 vecinas ajenas {mezcla}, edades (5 Myr) {cuenta:?}"
            );
        }
    }
    println!("{}", mapa_zona(&d, -40.0, 40.0, -60.0, 60.0, 1.5));
}

/// Escenario India–Asia (§5.5, calibración de `c_col`): el océano viejo de la placa india subduce
/// bajo Asia y tira de ella hacia el norte; al llegar la India, choca. Devuelve, en cada paso, la
/// velocidad hacia el norte de la India (cm/año), el frente de colisión India–Asia (km) y la latitud
/// del borde norte de la corteza india.
/// Configuración del escenario India–Asia: tres placas, sin rifts ni absorciones.
fn cfg_india_asia() -> TectonicaCfg {
    TectonicaCfg {
        a_min_frac: 0.0,
        tau_rift_myr: 1e12,
        // Escenario del choque: sin invasión por los bordes artificiales (meridianos) de los
        // extremos del Tetis, que cambiaría cómo se mueve la placa de la India.
        v_inv_km_myr: 0.0,
        n_min: 2,
        recalibrar_myr: 1e9, // c_b se calibra una vez, al principio
        ..TectonicaCfg::default()
    }
}

/// India (continental, 0–15°N, 70–90°E) en una placa oceánica que va al norte a ~10 cm/año contra
/// Asia (todo lo que está al norte de 25°N).
fn choque_india_asia(cfg: &TectonicaCfg, nivel: u32, n0: u32) -> Deriva {
    let malla = Malla::new(n0, nivel, 6371.0);
    let dirs = malla.dir.clone();
    let geo = |d: [f64; 3]| {
        let (la, lo) = lat_lon(d);
        (la.to_degrees(), lo.to_degrees())
    };
    let india = |la: f64, lo: f64| (0.0..=15.0).contains(&la) && (70.0..=90.0).contains(&lo);
    let mut d = Deriva::desde(cfg, 7, malla, 0.0, &[1, 2, 3], |k| {
        let (la, lo) = geo(dirs[k]);
        if la > 25.0 {
            (1, corteza(true, 35.0, 0.0))
        } else if (40.0..=140.0).contains(&lo) && la >= -40.0 {
            if india(la, lo) {
                (0, corteza(true, 35.0, 0.0))
            } else {
                (0, corteza(false, 7.0, 80.0))
            }
        } else {
            (2, corteza(false, 7.0, 150.0))
        }
    });
    // La India hacia el norte a ~10 cm/año: giro alrededor de un eje ecuatorial a 90° de 80°E.
    let lo = (80.0f64 - 90.0).to_radians();
    let w = 100.0 / 6371.0;
    d.arrancar_con(&[[w * lo.cos(), w * lo.sin(), 0.0], [0.0; 3], [0.0; 3]]);
    // El Tetis ya subduce bajo Asia con su losa madura (nota de fosas: el escenario empieza con la
    // fosa en marcha).
    let l = d.cfg.l_losa_km;
    d.activar_fosas(l);
    d
}

/// Relajación (adenda de balance §10, prueba 2): el choque India–Asia durante `t_choque` Myr y,
/// con la convergencia detenida, cuánto tarda el espesor máximo de la corteza en bajar de 38 km.
/// Devuelve (espesor máximo al detenerla, Myr hasta bajar de 38 km).
fn relajacion(cfg: &TectonicaCfg, nivel: u32, n0: u32, t_choque: u64) -> (f64, f64) {
    let mut d = choque_india_asia(cfg, nivel, n0);
    for _ in 0..t_choque {
        d.dar_paso();
    }
    let h0 = d.espesor_max_continental();
    for i in 1..=2000u64 {
        d.paso_sin_movimiento();
        if d.espesor_max_continental() < 38.0 {
            return (h0, i as f64 * cfg.dt_myr);
        }
    }
    (h0, f64::INFINITY)
}

fn india_asia(c_col: f64, nivel: u32, n0: u32, pasos: u64) -> Vec<(f64, f64, f64)> {
    let cfg = TectonicaCfg {
        c_col,
        ..cfg_india_asia()
    };
    let mut d = choque_india_asia(&cfg, nivel, n0);
    let geo = |d: [f64; 3]| {
        let (la, lo) = lat_lon(d);
        (la.to_degrees(), lo.to_degrees())
    };
    let mut out = Vec::new();
    for _ in 0..pasos {
        d.dar_paso();
        let (la, lo) = (7.5f64.to_radians(), 80f64.to_radians());
        let x = [la.cos() * lo.cos(), la.cos() * lo.sin(), la.sin()];
        let p = d.dueno[d.malla.localizar(x)] as usize;
        let om = d.placas[p].omega;
        let v = [
            om[1] * x[2] - om[2] * x[1],
            om[2] * x[0] - om[0] * x[2],
            om[0] * x[1] - om[1] * x[0],
        ];
        let norte = [-la.sin() * lo.cos(), -la.sin() * lo.sin(), la.cos()];
        let vn = (v[0] * norte[0] + v[1] * norte[1] + v[2] * norte[2]) * 6371.0 / 10.0;
        // Borde norte de la India: latitud máxima de corteza continental de su placa entre 70° y 90°E.
        let mut lat_max = f64::NEG_INFINITY;
        for k in 0..d.malla.num {
            if d.dueno[k] as usize == p && d.corteza(k).is_some_and(|c| c.cont) {
                let (la, lo) = geo(d.malla.dir[k]);
                if (60.0..=100.0).contains(&lo) {
                    lat_max = lat_max.max(la);
                }
            }
        }
        // La de Asia es la placa del punto 40°N 80°E.
        let (la2, lo2) = (40f64.to_radians(), 80f64.to_radians());
        let pa =
            d.dueno[d
                .malla
                .localizar([la2.cos() * lo2.cos(), la2.cos() * lo2.sin(), la2.sin()])]
                as usize;
        out.push((vn, d.frente_colision(p, pa), lat_max));
    }
    out
}

#[test]
#[ignore = "exploratoria: calibración de c_col"]
fn calibracion_c_col_india_asia() {
    let nivel: u32 = std::env::var("NIVEL")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(1);
    let pasos: u64 = std::env::var("PASOS")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(40);
    let lista: Vec<f64> = std::env::var("CCOL")
        .ok()
        .map(|x| x.split(',').filter_map(|v| v.parse().ok()).collect())
        .unwrap_or(vec![0.0, 10_000.0, 30_000.0, 60_000.0]);
    for &c in &lista {
        let r = india_asia(c, nivel, 48, pasos);
        let s: Vec<String> = r
            .iter()
            .step_by(2)
            .map(|(v, f, la)| format!("{v:.1}/{:.0}/{la:.0}", f / 100.0))
            .collect();
        println!(
            "c_col = {c:>6}: v_norte/frente(100 km)/lat_norte cada 2 Myr: {}",
            s.join(" ")
        );
    }
}

// --- Pruebas de comportamiento (§10), en mallas gruesas para que quepan en `cargo test` ---------

fn cfg_rapida() -> TectonicaCfg {
    TectonicaCfg {
        t_deriva_myr: 60.0,
        t_rodaje_myr: 10.0,
        ..TectonicaCfg::default()
    }
}

#[test]
fn la_deriva_es_determinista_y_reparte_toda_la_superficie() {
    let cfg = cfg_rapida();
    let correr = |semilla: u64| {
        let mut d = Deriva::nueva(&cfg, semilla, Malla::new(12, 1, 6371.0));
        let mut huecos = 0;
        for _ in 0..40 {
            d.dar_paso();
            huecos += d.dueno.iter().filter(|&&p| p == NADA).count();
        }
        let z = d.alturas();
        (
            z,
            d.eventos.len(),
            d.metricas.last().map(|m| m.v_rms_cm_anio),
            huecos,
        )
    };
    let (a, ea, va, ha) = correr(42);
    let (b, eb, vb, _) = correr(42);
    let (c, _, _, _) = correr(43);
    assert_eq!(
        ha, 0,
        "toda celda del mundo tiene una placa al acabar cada paso"
    );
    assert!(
        a.iter().zip(&b).all(|(x, y)| x.to_bits() == y.to_bits()) && ea == eb && va == vb,
        "misma semilla, misma deriva bit a bit"
    );
    assert!(a != c, "semilla distinta, deriva distinta");
}

#[test]
fn una_placa_rigida_que_gira_no_se_deforma() {
    let cfg = TectonicaCfg::default();
    let malla = Malla::new(12, 1, 6371.0);
    let num = malla.num;
    let dirs = malla.dir.clone();
    let cap = 40f64.to_radians().cos();
    let mut d = Deriva::desde(&cfg, 1, malla, -100.0, &[10, 20], |k| {
        if dirs[k][0] > cap {
            (0, corteza(true, 35.0, 0.0))
        } else {
            (1, corteza(false, 7.0, 50.0))
        }
    });
    let w = 50.0 / 6371.0;
    d.placas[0].omega = [0.0, 0.0, w];
    d.placas[1].omega = [0.0, 0.0, w];
    for _ in 0..100 {
        d.paso_cinematico();
    }
    let ang = 100.0 * w;
    let c2 = [ang.cos(), ang.sin(), 0.0];
    let mal = (0..num)
        .filter(|&k| {
            let x = d.malla.dir[k];
            let dentro = x[0] * c2[0] + x[1] * c2[1] + x[2] * c2[2] > cap;
            dentro != d.corteza(k).is_some_and(|c| c.cont)
        })
        .count();
    assert!(d.dueno.iter().all(|&p| p != NADA), "sin huecos");
    assert!(
        mal * 100 < num,
        "tras 100 Myr de giro rígido, {mal} de {num} celdas fuera de su sitio (> 1 %)"
    );
}

#[test]
fn sin_cratones_solo_los_arcos_hacen_corteza_continental() {
    let cfg = TectonicaCfg {
        f_cont: 0.0,
        ..cfg_rapida()
    };
    let mut d = Deriva::nueva(&cfg, 5, Malla::new(12, 1, 6371.0));
    for _ in 0..30 {
        d.dar_paso();
    }
    let z = d.alturas();
    for (k, &zk) in z.iter().enumerate() {
        let c = d.corteza(k).expect("corteza");
        // Sin cratones, la única corteza continental posible es la de los arcos (adenda de
        // balance: un arco oceánico rápido llega a ~20 km en unas decenas de Myr).
        assert!(
            !c.cont || c.arco,
            "corteza continental que no viene de un arco en un planeta sin cratones"
        );
        if c.h == cfg.h_oc_km && c.hc == 0.0 && c.sed == 0.0 {
            let esperada =
                -abm_terreno::tectonica::isostasia::profundidad_por_edad(&cfg, c.edad_oc);
            assert!(
                (zk - esperada).abs() < 1e-9,
                "profundidad por edad (Parsons y Sclater)"
            );
        }
    }
    // Hay dorsales: corteza nueva y joven.
    let jovenes = (0..d.malla.num)
        .filter(|&k| d.corteza(k).is_some_and(|c| c.edad_oc < 5.0))
        .count();
    assert!(jovenes > 0, "se crea fondo oceánico nuevo en las dorsales");
}

#[test]
fn dos_continentes_chocan_frenan_y_levantan_una_cordillera() {
    // India–Asia en una malla de ~208 km (n0 = 24, nivel 1): el océano viejo subduce bajo Asia, la
    // India llega, choca y la convergencia baja a menos de la mitad (calibración de c_col). Con la
    // losa que cuelga t_rot tras el choque (nota de fosas v4 §6.6), la velocidad norte de la India
    // en un punto depende también del giro que le dan las fosas de los lados: el tirón de la losa
    // se comprueba en `la_losa_sigue_tirando_tras_el_choque_y_se_rompe_a_los_t_rot`.
    let cfg = TectonicaCfg::default();
    let r = india_asia(cfg.c_col, 1, 24, 50);
    let antes = r
        .iter()
        .take(6)
        .map(|x| x.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let i_choque = r.iter().position(|x| x.1 > 0.0).expect("hay choque");
    let t_rot = cfg.t_rot_joven_myr + (cfg.t_rot_viejo_myr - cfg.t_rot_joven_myr) * 0.8;
    let i_rotura = i_choque + t_rot.round() as usize;
    let media = |v: &[(f64, f64, f64)]| v.iter().map(|x| x.0).sum::<f64>() / v.len() as f64;
    let durante = media(&r[i_choque + 4..i_rotura]);
    let despues = media(&r[i_rotura + 4..]);
    assert!(
        antes > 8.0,
        "antes del choque la India avanza deprisa ({antes:.1} cm/año)"
    );
    assert!(
        durante < 0.5 * antes && despues < 0.5 * antes,
        "el choque frena la convergencia: {antes:.1} → {durante:.1} con la losa colgando y {despues:.1} tras romperse (cm/año)"
    );
    assert!(
        r.last().is_some_and(|x| x.2.is_finite()),
        "la corteza india no desaparece en 50 Myr"
    );
}

#[test]
fn un_continente_se_parte_y_abre_un_oceano_simetrico() {
    // Un continente de 30° en una placa; τ_rift = Δt fuerza el rift en el primer paso.
    let cfg = TectonicaCfg {
        tau_rift_myr: 1.0,
        n_min: 2,
        ..TectonicaCfg::default()
    };
    let malla = Malla::new(12, 2, 6371.0);
    let dirs = malla.dir.clone();
    let mut d = Deriva::desde(&cfg, 3, malla, 0.0, &[10, 20], |k| {
        let x = dirs[k];
        if x[0] > 0.0 {
            let cont = x[0] > 30f64.to_radians().cos();
            (0, corteza(cont, if cont { 35.0 } else { 7.0 }, 60.0))
        } else {
            (1, corteza(false, 7.0, 60.0))
        }
    });
    d.arrancar_con(&[[0.0; 3], [0.0; 3]]);
    d.dar_paso();
    let rift = d.eventos.iter().find(|e| e.tipo == "rift").cloned();
    let rift = rift.expect("el continente se parte en el primer paso");
    let (h1, h2) = (rift.placas[1], rift.placas[2]);
    d.cfg.tau_rift_myr = 1e12;
    for _ in 0..30 {
        d.dar_paso();
    }
    // Fondo joven entre las dos mitades, repartido entre las dos hijas (expansión simétrica).
    let idx = |id: u64| d.placas.iter().position(|p| p.id == id && p.viva);
    let (Some(i1), Some(i2)) = (idx(h1), idx(h2)) else {
        panic!("las dos hijas siguen vivas");
    };
    let joven = |i: usize| {
        (0..d.malla.num)
            .filter(|&k| {
                d.dueno[k] as usize == i
                    && d.corteza(k).is_some_and(|c| !c.cont && c.edad_oc < 30.0)
            })
            .count() as f64
    };
    let (j1, j2) = (joven(i1), joven(i2));
    assert!(j1 + j2 > 0.0, "se abre un océano entre las dos mitades");
    assert!(
        j1.min(j2) / j1.max(j2) > 0.3,
        "la corteza nueva se reparte entre las dos placas: {j1} / {j2}"
    );
    // Márgenes adelgazados (H ← f_rift·H) a los dos lados.
    let adelgazada = (0..d.malla.num)
        .filter(|&k| d.corteza(k).is_some_and(|c| c.cont && c.h < 0.8 * 35.0))
        .count();
    assert!(adelgazada > 0, "quedan márgenes continentales adelgazados");
}

#[test]
fn el_planeta_tectonico_sale_de_la_cache_bit_a_bit() {
    use abm_terreno::config::{Relieve, TerrenoConfig};
    let mut cfg = TerrenoConfig::default();
    cfg.run.seed = 11;
    cfg.run.total_ticks = 3;
    cfg.planet.n0 = 12;
    cfg.lod.l_max = 2;
    cfg.erosion.geo_steps = 4;
    cfg.erosion.geo_rounds = 1;
    cfg.erosion.geo_dt_years = 20_000.0;
    cfg.erosion.run_every = 0;
    cfg.worldgen.relieve = Relieve::Tectonico;
    cfg.tectonica.nivel_malla = 1;
    cfg.tectonica.t_deriva_myr = 30.0;
    cfg.tectonica.t_rodaje_myr = 5.0;
    let dir = std::env::temp_dir().join(format!("abm_cache_tect_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    cfg.run.genesis_cache = dir.to_string_lossy().into_owned();
    let (d1, w1, i1) = abm_terreno::run_with_info(&cfg).unwrap();
    let (d2, w2, i2) = abm_terreno::run_with_info(&cfg).unwrap();
    assert!(
        !i1.from_cache && i2.from_cache,
        "la segunda corrida lee la caché"
    );
    assert_eq!(d1, d2, "la corrida con la génesis de la caché es idéntica");
    assert!(
        w1.h.iter()
            .zip(&w2.h)
            .all(|(a, b)| a.to_bits() == b.to_bits())
    );
    let t = d1
        .run
        .tectonica
        .as_ref()
        .expect("historia tectónica en la salida");
    assert_eq!(t.pasos, 30);
    assert!(
        d1.diagnostico.relieve.is_some(),
        "diagnóstico ampliado del relieve"
    );
    assert_eq!(
        d1.schema_version,
        abm_terreno::metrics::SCHEMA_VERSION_RELIEVE
    );
    // El relieve es bimodal: tierra alta y fondo oceánico hondo, con poco en medio.
    let (mut alto, mut medio, mut hondo) = (0.0, 0.0, 0.0);
    for s in 0..w1.topo.num_cells {
        let (h, a) = (w1.h[s], w1.topo.area[s]);
        if h > -500.0 {
            alto += a;
        } else if h > -2000.0 {
            medio += a;
        } else {
            hondo += a;
        }
    }
    assert!(
        medio < 0.25 * (alto + hondo),
        "curva de alturas bimodal: {alto} {medio} {hondo}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mover_las_placas_conserva_exactamente_la_corteza() {
    // Adenda de balance §0: la corteza continental de las placas solo cambia por procesos con
    // nombre (arcos, choques, rifts, erosión, sedimento); el remuestreo no crea ni destruye nada.
    let cfg = TectonicaCfg {
        t_deriva_myr: 80.0,
        t_rodaje_myr: 10.0,
        tau_rift_myr: 60.0, // fuerza rifts, suturas y absorciones en una deriva corta
        ..TectonicaCfg::default()
    };
    let mut d = Deriva::nueva(&cfg, 9, Malla::new(12, 2, 6371.0));
    d.correr();
    let c = d.cuentas;
    let rel = c.remuestreo_km3.abs() / c.inventario_inicial_km3;
    assert!(
        rel < 1e-9,
        "fuga de conservación: {:.3e} km³ ({rel:.2e} del volumen); cuentas {c:?}",
        c.remuestreo_km3
    );
    let tipos: Vec<&str> = d.eventos.iter().map(|e| e.tipo.as_str()).collect();
    assert!(
        tipos.contains(&"rift"),
        "la prueba debe pasar por rifts: {tipos:?}"
    );
    // Adenda de balance §6.9: en cada paso, el cambio de V_cont son fuentes menos sumideros.
    assert!(
        c.error_balance_max_rel < 1e-9,
        "balance de un paso: error {:.2e}",
        c.error_balance_max_rel
    );
    // Los traslados internos no pierden nada: la erosión acaba en el mar o en cuencas cerradas, y
    // el sedimento de fosa se pega al margen o se hunde.
    let erosion = c.sedimento_al_mar_km3 + c.sedimento_en_cuencas_km3;
    assert!(
        (erosion - c.erosion_km3).abs() <= 1e-9 * c.erosion_km3.max(1.0),
        "erosión {:.6e} frente a sedimento {erosion:.6e}",
        c.erosion_km3
    );
    let fosa = c.sedimento_acrecionado_km3 + c.sedimento_subducido_km3;
    assert!(
        (fosa - c.sedimento_a_fosa_km3).abs() <= 1e-9 * c.sedimento_a_fosa_km3.max(1.0),
        "sedimento de fosa {:.6e} frente a {fosa:.6e}",
        c.sedimento_a_fosa_km3
    );
    assert_eq!(c.sedimento_sin_destino_km3, 0.0);
    assert!(c.arcos_km3 > 0.0 && c.erosion_subduccion_km3 > 0.0 && c.delaminacion_km3 >= 0.0);
    // La alarma de espesor (ALARMA_ESPESOR_KM) se comprueba en el planeta, con la malla de 52 km
    // (batería de escenarios): en esta malla de ~200 km la franja de un choque es una sola celda y
    // el flujo lateral no llega a repartir.
    for m in &d.metricas {
        assert!(
            m.f_arc_au.is_finite() && m.error_balance_rel < 1e-9,
            "métrica del paso {m:?}"
        );
    }
}

#[test]
fn la_relajacion_de_un_orogeno_tarda_entre_100_y_200_myr() {
    // Adenda de balance §10, prueba 2: tras el choque India–Asia se detiene la convergencia y el
    // espesor máximo vuelve a menos de 38 km en 100–200 Myr (con E_1 y n_ero calibrados así,
    // adenda v5).
    let (h0, t) = relajacion(&cfg_india_asia(), 1, 48, 50);
    assert!(h0 > 60.0, "el choque debe engrosar la corteza: {h0:.1} km");
    assert!((100.0..=200.0).contains(&t), "relajación en {t} Myr");
}

#[test]
fn un_planeta_sin_fosas_solo_delamina_y_erosiona() {
    // Adenda de balance §10, caso límite: una sola placa no tiene bordes; no hay arcos ni
    // sumideros de fosa, solo la delaminación y la erosión (con su sedimento, que no se pierde).
    let cfg = TectonicaCfg {
        n_min: 1,
        tau_rift_myr: 1e12,
        a_min_frac: 0.0,
        ..TectonicaCfg::default()
    };
    let malla = Malla::new(12, 1, 6371.0);
    let dirs = malla.dir.clone();
    let mut d = Deriva::desde(&cfg, 3, malla, 0.0, &[1], |k| {
        let (la, _) = lat_lon(dirs[k]);
        if la.to_degrees() > 40.0 {
            (0, corteza(true, 70.0, 0.0))
        } else {
            (0, corteza(false, 7.0, 60.0))
        }
    });
    let v0 = d.inventario_continental();
    for _ in 0..20 {
        d.dar_paso();
    }
    let c = d.cuentas;
    assert_eq!(
        (
            c.arcos_km3,
            c.erosion_subduccion_km3,
            c.sedimento_a_fosa_km3,
            c.choque_solapado_km3
        ),
        (0.0, 0.0, 0.0, 0.0),
        "sin fosas no hay procesos de fosa"
    );
    assert!(c.delaminacion_km3 > 0.0 && c.erosion_km3 > 0.0);
    let dv = d.inventario_continental() - v0;
    assert!(
        (dv + c.delaminacion_km3).abs() <= 1e-9 * v0,
        "V_cont cambia {dv:.4e} km³ y la delaminación es {:.4e}",
        c.delaminacion_km3
    );
}

#[test]
fn las_reglas_del_margen_y_de_la_delaminacion() {
    use abm_terreno::tectonica::deriva::{margen_erosivo, tras_delaminar};
    let cfg = TectonicaCfg::default();
    // Un margen con mucho sedimento es siempre acrecionario, por rápido que converja.
    assert!(!margen_erosivo(&cfg, 20.0, 5.0));
    assert!(margen_erosivo(&cfg, 10.0, 0.5));
    assert!(!margen_erosivo(&cfg, 3.0, 0.0));
    // Sin exceso sobre h_del no delamina.
    assert_eq!(tras_delaminar(45.0, 50.0, 1.0, 50.0), 45.0);
    assert_eq!(tras_delaminar(50.0, 50.0, 1e6, 50.0), 50.0);
    // Con un paso enorme la forma exponencial sigue siendo estable: no baja de h_del.
    let h = tras_delaminar(80.0, 50.0, 1e6, 50.0);
    assert!((50.0..50.0 + 1e-9).contains(&h), "{h}");
    let h = tras_delaminar(80.0, 50.0, 50.0, 50.0);
    assert!((h - (50.0 + 30.0 / std::f64::consts::E)).abs() < 1e-12);
}

/// Dos placas oceánicas: la del oeste (más vieja) va al este a `v` cm/año contra la del este, quieta,
/// a lo largo del borde `lon = borde(lat)` (grados). Área consumida en `pasos` Myr (km²), con solo la
/// cinemática (sin fuerzas: compara la geometría del remuestreo).
fn area_consumida(borde: impl Fn(f64) -> f64, v: f64, pasos: u64) -> f64 {
    let cfg = TectonicaCfg::default();
    let malla = Malla::new(48, 1, 6371.0);
    let dirs = malla.dir.clone();
    let mut d = Deriva::desde(&cfg, 5, malla, 0.0, &[1, 2], |k| {
        let (la, lo) = lat_lon(dirs[k]);
        let (la, lo) = (la.to_degrees(), lo.to_degrees());
        if lo < borde(la) && lo > -150.0 {
            (0, corteza(false, 7.0, 120.0))
        } else {
            (1, corteza(false, 7.0, 40.0))
        }
    });
    d.placas[0].omega = [0.0, 0.0, v * 10.0 / 6371.0];
    // Una fosa ya en marcha: se compara la geometría del consumo, no su nacimiento.
    let l = d.cfg.l_losa_km;
    d.activar_fosas(l);
    let mut total = 0.0;
    for _ in 0..pasos {
        d.paso_cinematico();
        total += d.area_subducida_km2;
    }
    total
}

#[test]
fn un_borde_dentado_consume_la_misma_area_que_uno_recto() {
    // Adenda de balance §10, caso límite: para placas rígidas, el área que cruza un borde es la
    // misma sea recto o en zigzag (si todos sus dientes convergen).
    let recto = area_consumida(|_| 0.0, 5.0, 20);
    let zigzag = area_consumida(
        |la: f64| {
            let f = (la / 20.0).rem_euclid(1.0);
            5.0 * (1.0 - 4.0 * (f - 0.5).abs())
        },
        5.0,
        20,
    );
    // Sin dientes, lo que entra por el ecuador en 20 Myr a 5 cm/año a lo largo de 180° de meridiano.
    let teorica = 5.0 * 10.0 * 20.0 * 2.0 * 6371.0;
    println!(
        "área consumida: recto {:.4} y zigzag {:.4} de la teórica",
        recto / teorica,
        zigzag / teorica
    );
    assert!(
        (recto / teorica - 1.0).abs() < 0.05,
        "recto {recto:.4e} km², teórica {teorica:.4e}"
    );
    assert!(
        (zigzag / recto - 1.0).abs() < 0.05,
        "zigzag {zigzag:.4e} km² frente a recto {recto:.4e}"
    );
}

#[test]
#[ignore = "exploratoria: calibración de la erosión (E_1, n_ero; listas E1, N_ERO y D0) con la relajación de la adenda de balance"]
fn calibracion_relajacion() {
    let lista = |var: &str, def: f64| -> Vec<f64> {
        std::env::var(var)
            .ok()
            .map(|x| x.split(',').filter_map(|v| v.trim().parse().ok()).collect())
            .unwrap_or(vec![def])
    };
    let (e1s, ns) = (lista("E1", 0.02), lista("N_ERO", 2.0));
    let d0s = lista("D0", TectonicaCfg::default().d0_flujo_km2_myr);
    let t_choque: u64 = std::env::var("T_CHOQUE")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(50);
    let casos: Vec<(f64, f64, f64)> = d0s
        .iter()
        .flat_map(|&d0| {
            let e1s = &e1s;
            ns.iter()
                .flat_map(move |&n| e1s.iter().map(move |&e| (d0, n, e)))
        })
        .collect();
    let res: Vec<(f64, f64, f64, f64, f64)> = std::thread::scope(|sc| {
        let hilos: Vec<_> = casos
            .iter()
            .map(|&(d0, n, e1)| {
                sc.spawn(move || {
                    let cfg = TectonicaCfg {
                        e1_km_myr: e1,
                        n_ero: n,
                        d0_flujo_km2_myr: d0,
                        ..cfg_india_asia()
                    };
                    let (h0, t) = relajacion(&cfg, 1, 48, t_choque);
                    (d0, n, e1, h0, t)
                })
            })
            .collect();
        hilos.into_iter().map(|h| h.join().expect("hilo")).collect()
    });
    for (d0, n, e1, h0, t) in res {
        println!(
            "D_0 = {d0}, n_ero = {n}, E_1 = {e1} km/Myr: espesor máximo al detener {h0:.1} km, baja de 38 km en {t:.0} Myr"
        );
    }
}

// --- Bordes de placa (nota_bordes_placa.md §10) ------------------------------------------------

/// Celdas del mundo con alguna vecina de otra placa (la longitud del borde en celdas).
fn celdas_de_borde(d: &Deriva) -> usize {
    (0..d.malla.num)
        .filter(|&k| {
            d.malla.vec[k]
                .iter()
                .any(|&v| v != NADA && d.dueno[v as usize] != d.dueno[k])
        })
        .count()
}

#[test]
fn dos_placas_que_solo_se_deslizan_no_crean_ni_consumen() {
    // Nota de bordes §10, prueba 1: dos placas oceánicas que se deslizan a lo largo del ecuador
    // (la del norte gira alrededor del eje de los polos), con el borde en dientes de una celda.
    // Ni se consume ni nace placa, y los dientes no crecen.
    let cfg = TectonicaCfg::default();
    let malla = Malla::new(48, 1, 6371.0);
    let dirs = malla.dir.clone();
    let mut d = Deriva::desde(&cfg, 11, malla, 0.0, &[1, 2], |k| {
        let (la, lo) = lat_lon(dirs[k]);
        let (la, lo) = (la.to_degrees(), lo.to_degrees());
        // Dientes de ~1° (una celda) cada 3° de longitud.
        let f = (lo / 3.0).rem_euclid(1.0);
        let borde = if f < 0.5 { 1.0 } else { -1.0 };
        if la > borde {
            (0, corteza(false, 7.0, 60.0))
        } else {
            (1, corteza(false, 7.0, 100.0))
        }
    });
    d.placas[0].omega = [0.0, 0.0, 50.0 / 6371.0];
    let borde0 = celdas_de_borde(&d);
    let (mut consumo, mut creacion) = (0.0, 0.0);
    for _ in 0..100 {
        d.paso_cinematico();
        consumo += d.consumo_paso.iter().map(|x| x.1).sum::<f64>();
        creacion += d
            .creacion_paso
            .iter()
            .map(|&k| d.malla.area[k as usize])
            .sum::<f64>();
    }
    // Lo que crearía una dorsal del mismo largo a la misma velocidad: 5 cm/año · 40 000 km · 100 Myr.
    let dorsal = 50.0 * 40_030.0 * 100.0;
    println!(
        "deslizamiento puro: consumo {:.2e} km² y creación {:.2e} km² ({:.3} % y {:.3} % de una dorsal); celdas de borde {borde0} → {}",
        consumo,
        creacion,
        100.0 * consumo / dorsal,
        100.0 * creacion / dorsal,
        celdas_de_borde(&d)
    );
    assert!(consumo < 1e-3 * dorsal, "consumo {consumo:.3e} km²");
    assert!(creacion < 1e-2 * dorsal, "creación {creacion:.3e} km²");
    assert!(celdas_de_borde(&d) <= borde0, "aparecen dientes nuevos");
}

#[test]
fn una_separacion_oblicua_solo_crea_placa() {
    // Nota de bordes §10, prueba 2: la placa del este se separa de la del oeste en diagonal (45° en
    // el ecuador) a lo largo del meridiano 0. Allí solo nace placa; no se consume nada.
    let cfg = TectonicaCfg::default();
    let malla = Malla::new(48, 1, 6371.0);
    let dirs = malla.dir.clone();
    let mut d = Deriva::desde(&cfg, 13, malla, 0.0, &[1, 2], |k| {
        let (_, lo) = lat_lon(dirs[k]);
        if lo.to_degrees() >= 0.0 {
            (1, corteza(false, 7.0, 60.0))
        } else {
            (0, corteza(false, 7.0, 60.0))
        }
    });
    let w = 50.0 / 6371.0;
    d.placas[1].omega = [0.0, -w, w];
    let cerca = |d: &Deriva, k: u32| {
        let (la, lo) = lat_lon(d.malla.dir[k as usize]);
        la.to_degrees().abs() < 55.0 && lo.to_degrees().abs() < 15.0
    };
    let (mut consumo, mut creacion) = (0.0, 0.0);
    for _ in 0..30 {
        d.paso_cinematico();
        consumo += d
            .consumo_paso
            .iter()
            .filter(|x| cerca(&d, x.0))
            .map(|x| x.1)
            .sum::<f64>();
        creacion += d
            .creacion_paso
            .iter()
            .filter(|&&k| cerca(&d, k))
            .map(|&k| d.malla.area[k as usize])
            .sum::<f64>();
    }
    println!("separación oblicua: consumo {consumo:.3e} km², creación {creacion:.3e} km²");
    assert!(creacion > 0.0);
    assert!(
        consumo <= 1e-3 * creacion,
        "consumo {consumo:.3e} km² en una dorsal"
    );
}

#[test]
#[ignore = "exploratoria: calibración de D_0 (adenda v6 §6.6b, lista D0; TAU_DEL opcional) con el choque India–Asia"]
fn calibracion_flujo_lateral() {
    let lista: Vec<f64> = std::env::var("D0")
        .ok()
        .map(|x| x.split(',').filter_map(|v| v.parse().ok()).collect())
        .unwrap_or(vec![0.0, 300.0]);
    let nivel: u32 = std::env::var("NIVEL")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(2);
    let tau_del: f64 = std::env::var("TAU_DEL")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(cfg_india_asia().tau_del_myr);
    println!("τ_del = {tau_del} Myr");
    for &dfl in &lista {
        let cfg = TectonicaCfg {
            d0_flujo_km2_myr: dfl,
            tau_del_myr: tau_del,
            ..cfg_india_asia()
        };
        let mut d = choque_india_asia(&cfg, nivel, 48);
        let mut serie = Vec::new();
        for paso in 1..=60u64 {
            d.dar_paso();
            if paso % 10 == 0 {
                // Meseta: corteza de más de 55 km entre 70° y 100°E; anchura norte–sur y área.
                let (mut la_min, mut la_max, mut area) = (90.0f64, -90.0f64, 0.0);
                for k in 0..d.malla.num {
                    let Some(c) = d.corteza(k) else { continue };
                    if !c.cont || c.h <= 55.0 {
                        continue;
                    }
                    let (la, lo) = lat_lon(d.malla.dir[k]);
                    let (la, lo) = (la.to_degrees(), lo.to_degrees());
                    if (70.0..=100.0).contains(&lo) {
                        la_min = la_min.min(la);
                        la_max = la_max.max(la);
                        area += d.malla.area[k];
                    }
                }
                let ancho = if la_max > la_min {
                    (la_max - la_min) * 111.2
                } else {
                    0.0
                };
                serie.push(format!(
                    "{paso} Myr: máx {:.0} km, meseta {:.0} km × {:.2} Mkm²",
                    d.espesor_max_continental(),
                    ancho,
                    area / 1e6
                ));
            }
        }
        println!("D_0 = {dfl:>6}: {}", serie.join(" | "));
    }
}

#[test]
fn un_gran_choque_no_apila_corteza_por_encima_de_80_km() {
    // Adenda de balance v3 §6.6b: sin tope, el flujo lateral reparte la corteza del choque
    // India–Asia (malla de 104 km) y ninguna celda pasa de la alarma de 80 km... salvo por la
    // resolución: en 104 km la franja del choque son dos celdas; se admite un 10 %.
    let cfg = cfg_india_asia();
    let mut d = choque_india_asia(&cfg, 1, 48);
    let mut maximo = 0.0f64;
    for _ in 0..50 {
        d.dar_paso();
        maximo = maximo.max(d.espesor_max_continental());
    }
    assert!(maximo > 60.0, "el choque debe engrosar: {maximo:.1} km");
    assert!(
        maximo <= 1.1 * ALARMA_ESPESOR_KM,
        "corteza de {maximo:.1} km"
    );
}

#[test]
#[ignore = "exploratoria: evolución de la relajación"]
fn exploracion_relajacion() {
    let cfg = cfg_india_asia();
    let mut d = choque_india_asia(&cfg, 1, 48);
    for _ in 0..50 {
        d.dar_paso();
    }
    let mut serie = Vec::new();
    for i in 1..=600u64 {
        d.paso_sin_movimiento();
        if i % 50 == 0 {
            // La celda más gruesa: espesor, sedimento y si es continental.
            let k = (0..d.malla.num)
                .filter(|&k| d.corteza(k).is_some_and(|c| c.cont))
                .max_by(|&a, &b| d.corteza(a).unwrap().h.total_cmp(&d.corteza(b).unwrap().h))
                .unwrap();
            let c = d.corteza(k).unwrap();
            let z = d.alturas()[k];
            serie.push(format!(
                "{i}: {:.1} (hc {:.1} sed {:.2} z {:.0} zeta {:.0} edad_orog {:.0})",
                c.h, c.hc, c.sed, z, d.zeta_m, c.edad_orog
            ));
        }
    }
    println!("{}", serie.join(" | "));
}

/// Placas distintas a menos de `radio_km` de la celda `k` (por la superficie).
fn placas_cerca(d: &Deriva, k: usize, radio_km: f64) -> usize {
    let cos_max = (radio_km / d.malla.r_km).cos();
    let x = d.malla.dir[k];
    let mut visto = std::collections::HashSet::new();
    let mut placas = std::collections::HashSet::new();
    let mut pila = vec![k];
    visto.insert(k);
    while let Some(c) = pila.pop() {
        if d.dueno[c] != NADA {
            placas.insert(d.dueno[c]);
        }
        for &v in &d.malla.vec[c] {
            let vu = v as usize;
            if v == NADA || !visto.insert(vu) {
                continue;
            }
            let dv = d.malla.dir[vu];
            if x[0] * dv[0] + x[1] * dv[1] + x[2] * dv[2] >= cos_max {
                pila.push(vu);
            }
        }
    }
    placas.len()
}

/// Lo que deja una deriva del planeta para el diagnóstico de las uniones triples, los picos de
/// espesor y el coste de cada etapa.
#[derive(Default)]
struct InformeDeriva {
    semilla: u64,
    /// Ajustes de la config de esta deriva (`clave=valor` separados por comas; vacío: la config).
    variante: String,
    /// Reparto de la erosión de la ley con la altura `Σ E_1·(z/1 km)^n·A` (sin descontar las cuencas
    /// cerradas) y de la tierra emergida por altura sobre el mar: 0–0,5, 0,5–1, 1–2, 2–3 y más de
    /// 3 km, tras el rodaje.
    erosion_por_exceso: [f64; 5],
    tierra_por_altura: [f64; 5],
    /// Consumo (km²) y solape continental (km³) tras el rodaje, por código de borde (0 sin borde,
    /// 1 dorsal, 2 fosa, 3 choque, 4 transformante) y por si la celda está en una unión triple.
    consumo: [[f64; 2]; 6],
    solape: [[f64; 2]; 6],
    /// Longitud de borde (km·paso) en uniones triples y en total, tras el rodaje.
    borde_triple_km: f64,
    borde_total_km: f64,
    /// Celdas·paso de más de 80 km (todas y en uniones triples), pasos con alguna, y por etapa
    /// que más subió el máximo en esos pasos.
    picos: u64,
    picos_triples: u64,
    pasos_con_picos: u64,
    pasos_con_picos_rodaje: u64,
    etapa_de_los_picos: std::collections::BTreeMap<&'static str, u64>,
    h_max: f64,
    /// Pasos en los que la muestra más gruesa está oculta (no la ve el mundo).
    pasos_maximo_oculto: u64,
    /// Celdas·paso de más de 80 km según su muestra en el paso anterior: ya visible, oculta y de
    /// más de 60 km (reaparece), u otra (nueva, o fina el paso anterior).
    picos_seguian: u64,
    picos_reaparecen: u64,
    picos_otros: u64,
    /// Material continental de los dientes por destino (AU, tras el rodaje): recolocados a menos de
    /// L_borde, recolocados en la segunda pasada, sumados a su placa, sumados a otra; y la creación
    /// residual (km²/año).
    dientes_via_au: [f64; 4],
    creacion_residual: f64,
    /// Pasos con picos en los que una muestra que recibió dientes sin hueco (0) o muestras sueltas
    /// de otra placa (1) pasó de 80 km.
    picos_por_dientes: [u64; 2],
    /// Espesor máximo de una muestra continental oculta y de una visible (km), tras el rodaje.
    max_oculta: f64,
    max_visible: f64,
    /// Consumo asignado a celdas transformantes por oblicuidad del movimiento relativo (km²; clases
    /// 0–5, 5–10, 10–15, 15–20 y más de 20°).
    oblicuidad_transf: [f64; 5],
    /// Consumo asignado a celdas transformantes a menos de `L_borde` de una fosa del mismo par de
    /// placas (el final de una fosa), y en total (km²).
    transf_junto_a_fosa: f64,
    transf_total: f64,
    h_max_paso: u64,
    h_max_etapa: &'static str,
    /// Tiempo de pared por etapa (s) y total.
    segundos: std::collections::BTreeMap<&'static str, f64>,
    /// Balance tras el rodaje (medias, AU) y cambio de V_cont desde el final del rodaje.
    arcos: f64,
    choques: f64,
    delam: f64,
    /// Erosión por subducción, sedimento subducido y acrecionado (AU).
    se: f64,
    sed_sub: f64,
    sed_acr: f64,
    al_mar: f64,
    v_cont: f64,
    f_cont: f64,
    placas: u32,
    a_sub: f64,
    fosas_km: f64,
    conv_fosas: f64,
    /// Nota de fosas: convergencia difusa (km, cm/año y km²/año absorbidos), edad media de las fosas,
    /// fracción del tirón, nacimientos y muertes por 100 Myr, edad del fondo (media de los pasos y,
    /// al final, mediana y fracciones por encima de 180 y de 200 Myr), v_rms y Forsyth–Uyeda.
    difusas_km: f64,
    conv_difusas: f64,
    area_difusa: f64,
    edad_fosas: f64,
    losa: f64,
    nacen_100: [f64; 3],
    mueren_100: [f64; 2],
    edad_oc_media: f64,
    edad_oc_mediana: f64,
    edad_oc_180: f64,
    edad_oc_200: f64,
    v_rms: f64,
    fu_fosa: f64,
    /// Medidas de fuerzas de la nota de fosas §10: longitud de borde por oblicuidad (10° por clase;
    /// km·paso, convergente y divergente), áreas de las placas al final (fracción, de mayor a
    /// menor), potencia de cada fuerza sumada en todas las placas (tirón, empuje, choques, arrastre)
    /// y área creada y consumida (km²/año).
    oblicuidad: [[f64; 9]; 2],
    tamanos: Vec<f64>,
    potencia: [f64; 4],
    creada: f64,
    /// Sedimento: máximo de una muestra al final de un paso (km, tras el rodaje), parte del que va
    /// al mar que no cabía en el de su desembocadura y el que queda en cuencas cerradas (AU, toda
    /// la deriva).
    sed_max: f64,
    desbordado: f64,
    en_cuencas: f64,
    /// Nota de fosas v4: márgenes pasivos (vida media de los que terminan, edad media y máxima de
    /// los que hay al final, Myr), invasión por transformantes y por márgenes (km/Myr), roturas de
    /// losa por 100 Myr, choques (convergencia media, cm/año; pérdida con la losa colgando y sin
    /// ella, AU; duración media de los continente–continente que acaban, Myr, y cuántos).
    vida_margen: f64,
    edad_margen_final: f64,
    edad_max_margen_final: f64,
    invadido: [f64; 2],
    separar_100: [f64; 3],
    roturas_100: f64,
    conv_choques: f64,
    choque_con_losa: f64,
    choque_sin_losa: f64,
    duracion_choques: f64,
    choques_acabados: f64,
    /// Nota de fosas v5: nacimientos en zonas de fractura por 100 Myr, intentos de partir la placa
    /// por ellas que fallan por 100 Myr, y longitud media de los escalones de edad de más de
    /// `Δa_fz` dentro de las placas y de los candidatos (km).
    zf: [f64; 4],
    /// Adenda v7: reparto de la corteza solapada en los choques (sin la de arco) y de la que se
    /// pierde por la muestra que entra (continental < h_1, h_1–h_2, > h_2, sobre fondo oceánico),
    /// en fracción de toda la deriva.
    por_grosor: [[f64; 4]; 2],
}

fn diagnosticar_deriva(semilla: u64, variante: &str) -> InformeDeriva {
    let ajustes: Vec<String> = variante
        .split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect();
    let cfg = abm_terreno::config::TerrenoConfig::load_con("../../config/terreno.toml", &ajustes)
        .expect("config");
    let tc = cfg.tectonica.clone();
    let malla = Malla::new(cfg.planet.n0, tc.nivel_malla, cfg.planet.r_km);
    let mut d = Deriva::nueva(&tc, semilla, malla);
    d.traza = Some(Vec::new());
    let pasos = (tc.t_deriva_myr / tc.dt_myr).round() as u64;
    let rodaje = (tc.t_rodaje_myr / tc.dt_myr).round() as u64;
    let mut inf = InformeDeriva {
        semilla,
        variante: variante.to_string(),
        ..Default::default()
    };
    let mut h_prev = 0.0;
    // Muestras visibles (por placa y celda local) y ocultas gruesas del paso anterior.
    let mut vis_ant: Vec<Vec<bool>> = Vec::new();
    let mut ocultas_ant: std::collections::HashMap<(u32, u32), f64> = Default::default();
    for paso in 1..=pasos {
        d.dar_paso();
        let en_rodaje_v = paso <= rodaje;
        let mut vis: Vec<Vec<bool>> = d.placas.iter().map(|p| vec![false; p.mapa.len()]).collect();
        for k in 0..d.malla.num {
            if d.dueno[k] != NADA && d.rep[k] != NADA {
                vis[d.dueno[k] as usize][d.rep[k] as usize] = true;
            }
        }
        let mut ocultas = std::collections::HashMap::new();
        for (pi, p) in d.placas.iter().enumerate().filter(|(_, p)| p.viva) {
            for (s, c) in p.cort.iter().enumerate() {
                let l = p.celda[s];
                if c.cont && !vis[pi][l as usize] {
                    if !en_rodaje_v {
                        inf.max_oculta = inf.max_oculta.max(c.h);
                    }
                    if c.h > 60.0 {
                        ocultas.insert((pi as u32, l), c.h);
                    }
                }
            }
        }
        let traza = d.traza.replace(Vec::new()).unwrap_or_default();
        // La etapa que más sube el máximo de espesor en este paso.
        let (mut sube, mut etapa) = (f64::NEG_INFINITY, "");
        let mut ant = h_prev;
        for e in &traza {
            *inf.segundos.entry(e.etapa).or_insert(0.0) += e.segundos;
            if e.h_max_km - ant > sube {
                sube = e.h_max_km - ant;
                etapa = e.etapa;
            }
            ant = e.h_max_km;
        }
        let h_fin = traza.last().map_or(0.0, |e| e.h_max_km);
        // El máximo de las muestras (con las ocultas) frente al que ve el mundo.
        let h_visible = d.espesor_max_continental();
        if h_fin > h_visible + 1.0 {
            inf.pasos_maximo_oculto += 1;
        }
        let h_paso = traza.iter().map(|e| e.h_max_km).fold(0.0, f64::max);
        if paso > rodaje {
            for c in d
                .placas
                .iter()
                .filter(|p| p.viva)
                .flat_map(|p| p.cort.iter())
            {
                inf.sed_max = inf.sed_max.max(c.sed);
            }
        }
        if h_paso > inf.h_max {
            inf.h_max = h_paso;
            inf.h_max_paso = paso;
            inf.h_max_etapa = etapa;
        }
        h_prev = h_fin;
        let en_rodaje = paso <= rodaje;
        let mut picos_paso = 0;
        for k in 0..d.malla.num {
            if !en_rodaje && let Some(c) = d.corteza(k).filter(|c| c.cont) {
                inf.max_visible = inf.max_visible.max(c.h);
            }
            if d.corteza(k)
                .is_some_and(|c| c.cont && c.h > ALARMA_ESPESOR_KM)
            {
                picos_paso += 1;
                inf.picos += 1;
                if placas_cerca(&d, k, tc.l_borde_km) >= 3 {
                    inf.picos_triples += 1;
                }
                let clave = (d.dueno[k], d.rep[k]);
                let antes = vis_ant
                    .get(clave.0 as usize)
                    .is_some_and(|v| v[clave.1 as usize]);
                if antes {
                    inf.picos_seguian += 1;
                } else if ocultas_ant.contains_key(&clave) {
                    inf.picos_reaparecen += 1;
                } else {
                    inf.picos_otros += 1;
                }
            }
        }
        vis_ant = vis;
        ocultas_ant = ocultas;
        if !en_rodaje {
            for k in 0..d.malla.num {
                let Some(c) = d.corteza(k) else {
                    continue;
                };
                let z = (abm_terreno::tectonica::isostasia::altura(&d.cfg, c) - d.zeta_m) / 1000.0;
                if z > 0.0 {
                    let i = [0.5, 1.0, 2.0, 3.0].partition_point(|&l| l <= z);
                    let a = d.malla.area[k];
                    inf.erosion_por_exceso[i] += tc.e1_km_myr * z.powf(tc.n_ero) * a;
                    inf.tierra_por_altura[i] += a;
                }
            }
        }
        if picos_paso > 0 {
            for (i, &h) in d.max_tras_dientes_km.iter().enumerate() {
                if h > ALARMA_ESPESOR_KM {
                    inf.picos_por_dientes[i] += 1;
                }
            }
            if en_rodaje {
                inf.pasos_con_picos_rodaje += 1;
            } else {
                inf.pasos_con_picos += 1;
            }
            *inf.etapa_de_los_picos.entry(etapa).or_insert(0) += 1;
        }
        if en_rodaje {
            continue;
        }
        for (i, v) in d.dientes_cont_por_via_km3.iter().enumerate() {
            inf.dientes_via_au[i] += v;
        }
        let (mut cd, mut ld) = (0.0, 0.0);
        for b in d.bordes.iter().filter(|b| b.tipo == TipoBorde::Difusa) {
            cd += b.v_n.max(0.0) * b.largo_km;
            ld += b.largo_km;
        }
        for b in &d.bordes {
            let x = d.malla.dir[b.k as usize];
            let (wp, wq) = (d.placas[b.p as usize].omega, d.placas[b.q as usize].omega);
            let w = [wp[0] - wq[0], wp[1] - wq[1], wp[2] - wq[2]];
            let v = [
                w[1] * x[2] - w[2] * x[1],
                w[2] * x[0] - w[0] * x[2],
                w[0] * x[1] - w[1] * x[0],
            ]
            .map(|c| c * d.malla.r_km);
            let vm = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            if vm < 0.05 * 10.0 {
                continue;
            }
            let ang = (b.v_n.abs() / vm).min(1.0).asin().to_degrees();
            let lado = if b.v_n > 0.0 { 0 } else { 1 };
            inf.oblicuidad[lado][((ang / 10.0) as usize).min(8)] += 0.5 * b.largo_km;
        }
        for &(_, _, pw) in &d.potencias {
            for (a, b) in inf.potencia.iter_mut().zip(pw) {
                *a += b;
            }
        }
        inf.conv_difusas += cd;
        inf.difusas_km += ld;
        let mut borde_en = std::collections::HashMap::new();
        let mut fosas_par: std::collections::HashMap<(u32, u32), Vec<usize>> = Default::default();
        for b in &d.bordes {
            borde_en.insert(b.k, *b);
            if b.tipo == abm_terreno::tectonica::deriva::TipoBorde::Subduccion {
                fosas_par
                    .entry((b.p.min(b.q), b.p.max(b.q)))
                    .or_default()
                    .push(b.k as usize);
            }
        }
        let cos_borde = (tc.l_borde_km / d.malla.r_km).cos();
        for &(k, tipo, a) in &d.consumo_en_borde_paso {
            let t = (placas_cerca(&d, k as usize, tc.l_borde_km) >= 3) as usize;
            inf.consumo[tipo as usize][t] += a;
            if tipo == 4
                && let Some(b) = borde_en.get(&k)
            {
                // Oblicuidad: ángulo entre el movimiento relativo y el borde.
                let x = d.malla.dir[k as usize];
                let (wp, wq) = (d.placas[b.p as usize].omega, d.placas[b.q as usize].omega);
                let w = [wp[0] - wq[0], wp[1] - wq[1], wp[2] - wq[2]];
                let v = [
                    w[1] * x[2] - w[2] * x[1],
                    w[2] * x[0] - w[0] * x[2],
                    w[0] * x[1] - w[1] * x[0],
                ]
                .map(|c| c * d.malla.r_km);
                let vm = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                let ang = if vm > 0.0 {
                    (b.v_n.abs() / vm).min(1.0).asin().to_degrees()
                } else {
                    0.0
                };
                inf.oblicuidad_transf[((ang / 5.0) as usize).min(4)] += a;
                inf.transf_total += a;
                let cerca = fosas_par
                    .get(&(b.p.min(b.q), b.p.max(b.q)))
                    .is_some_and(|ks| {
                        ks.iter().any(|&j| {
                            let y = d.malla.dir[j];
                            x[0] * y[0] + x[1] * y[1] + x[2] * y[2] >= cos_borde
                        })
                    });
                if cerca {
                    inf.transf_junto_a_fosa += a;
                }
            }
        }
        for &(k, tipo, v) in &d.solape_en_borde_paso {
            let t = (placas_cerca(&d, k as usize, tc.l_borde_km) >= 3) as usize;
            inf.solape[tipo as usize][t] += v;
        }
        if paso % 10 == 0 {
            for b in &d.bordes {
                inf.borde_total_km += b.largo_km;
                if placas_cerca(&d, b.k as usize, tc.l_borde_km) >= 3 {
                    inf.borde_triple_km += b.largo_km;
                }
            }
        }
    }
    let val: Vec<_> = d.metricas.iter().filter(|m| !m.rodaje).collect();
    let n = val.len() as f64;
    let media = |f: fn(&abm_terreno::tectonica::historia::MetricaMyr) -> f64| {
        val.iter().map(|m| f(m)).sum::<f64>() / n
    };
    inf.arcos = media(|m| m.f_arc_au);
    inf.se = media(|m| m.f_se_au);
    inf.sed_sub = media(|m| m.f_sed_au);
    inf.sed_acr = media(|m| m.f_acr_au);
    inf.choques = media(|m| m.f_col_au);
    inf.delam = media(|m| m.f_del_au);
    inf.al_mar = media(|m| m.f_den_au);
    inf.a_sub = media(|m| m.area_subducida_km2_anio);
    inf.creacion_residual = media(|m| m.creacion_residual_km2_anio);
    inf.conv_difusas = if inf.difusas_km > 0.0 {
        inf.conv_difusas / inf.difusas_km / 10.0
    } else {
        0.0
    };
    inf.difusas_km = media(|m| m.difusas_km);
    inf.area_difusa = media(|m| m.area_difusa_km2_anio);
    inf.edad_fosas = val
        .iter()
        .map(|m| m.edad_media_fosas_myr * m.fosas_km)
        .sum::<f64>()
        / val.iter().map(|m| m.fosas_km).sum::<f64>().max(1e-12);
    inf.losa = media(|m| m.losa_media_frac);
    for m in &val {
        for (a, &v) in inf.nacen_100.iter_mut().zip(&m.nacimientos) {
            *a += v as f64 * 100.0 / n;
        }
        for (a, &v) in inf.mueren_100.iter_mut().zip(&m.muertes) {
            *a += v as f64 * 100.0 / n;
        }
    }
    inf.edad_oc_media = media(|m| m.edad_media_oceano_myr);
    let mut edades: Vec<(f64, f64)> = (0..d.malla.num)
        .filter_map(|k| {
            d.corteza(k)
                .filter(|c| !c.cont)
                .map(|c| (c.edad_oc, d.malla.area[k]))
        })
        .collect();
    edades.sort_by(|a, b| a.0.total_cmp(&b.0));
    let a_oc: f64 = edades.iter().map(|x| x.1).sum();
    let mut acum = 0.0;
    for &(e, a) in &edades {
        acum += a;
        if acum >= 0.5 * a_oc {
            inf.edad_oc_mediana = e;
            break;
        }
    }
    inf.edad_oc_180 = edades
        .iter()
        .filter(|x| x.0 > 180.0)
        .map(|x| x.1)
        .sum::<f64>()
        / a_oc;
    inf.edad_oc_200 = edades
        .iter()
        .filter(|x| x.0 > 200.0)
        .map(|x| x.1)
        .sum::<f64>()
        / a_oc;
    inf.v_rms = d.v_rms_acum.0 / (d.v_rms_acum.1.max(1) as f64);
    inf.creada = media(|m| m.area_creada_km2_anio);
    let mut areas = vec![0.0; d.placas.len()];
    for k in 0..d.malla.num {
        if d.dueno[k] != NADA {
            areas[d.dueno[k] as usize] += d.malla.area[k];
        }
    }
    let total: f64 = areas.iter().sum();
    inf.tamanos = areas
        .into_iter()
        .filter(|&a| a > 0.0)
        .map(|a| a / total)
        .collect();
    inf.tamanos.sort_by(|a, b| b.total_cmp(a));
    inf.fu_fosa = d.fu_fosa.0 / (d.fu_fosa.1.max(1) as f64);
    let anios = n * tc.dt_myr * 1e6;
    for v in inf.dientes_via_au.iter_mut() {
        *v /= anios;
    }
    inf.fosas_km = media(|m| m.fosas_km);
    inf.conv_fosas = val
        .iter()
        .map(|m| m.convergencia_media_fosas_cm_anio * m.fosas_km)
        .sum::<f64>()
        / val.iter().map(|m| m.fosas_km).sum::<f64>();
    let v_inicio = d
        .metricas
        .iter()
        .rev()
        .find(|m| m.rodaje)
        .map_or(val[0].inventario_km3, |m| m.inventario_km3);
    let ultima = val[val.len() - 1];
    inf.v_cont = ultima.inventario_km3 / v_inicio - 1.0;
    inf.f_cont = ultima.area_continental_frac;
    inf.placas = ultima.n_placas;
    let anios = pasos as f64 * tc.dt_myr * 1e6;
    inf.desbordado = d.cuentas.sedimento_desbordado_km3 / anios;
    let l_fin: f64 = val.iter().map(|m| m.margen_fin_km).sum();
    inf.vida_margen = if l_fin > 0.0 {
        val.iter().map(|m| m.margen_fin_vida_km_myr).sum::<f64>() / l_fin
    } else {
        f64::NAN
    };
    inf.edad_margen_final = ultima.edad_media_margen_myr;
    inf.edad_max_margen_final = ultima.edad_max_margen_myr;
    inf.invadido = [media(|m| m.invadido_km[0]), media(|m| m.invadido_km[1])];
    inf.roturas_100 = val.iter().map(|m| m.roturas as f64).sum::<f64>() * 100.0 / n;
    for (fila, x) in inf.por_grosor.iter_mut().zip([
        d.cuentas.choque_solapado_por_grosor_km3,
        d.cuentas.choque_perdida_por_grosor_km3,
    ]) {
        let t: f64 = x.iter().sum();
        for (f, v) in fila.iter_mut().zip(x) {
            *f = if t > 0.0 { v / t } else { 0.0 };
        }
    }
    inf.zf = [
        val.iter().map(|m| m.nacimientos_zf as f64).sum::<f64>() * 100.0 / n,
        val.iter().map(|m| m.separar_zf[1] as f64).sum::<f64>() * 100.0 / n,
        media(|m| m.zf_escalon_km),
        media(|m| m.zf_candidata_km),
    ];
    for i in 0..3 {
        inf.separar_100[i] = val.iter().map(|m| m.separar[i] as f64).sum::<f64>() * 100.0 / n;
    }
    let l_col: f64 = val.iter().map(|m| m.colisiones_km).sum();
    inf.conv_choques = if l_col > 0.0 {
        val.iter()
            .map(|m| m.convergencia_media_choques_cm_anio * m.colisiones_km)
            .sum::<f64>()
            / l_col
    } else {
        0.0
    };
    inf.choque_con_losa = media(|m| m.f_col_con_losa_au);
    inf.choque_sin_losa = media(|m| m.f_col_sin_losa_au);
    // Duración ponderada por la corteza solapada: cuentan los choques grandes.
    let (mut dv, mut vs, mut nc) = (0.0, 0.0, 0.0);
    for e in d
        .eventos
        .iter()
        .filter(|e| e.tipo == "fin_choque" && e.clase.as_deref() == Some("continente_continente"))
    {
        let sol = e.solapado_km3.unwrap_or(0.0);
        dv += e.tiempo_choque_myr.unwrap_or(0.0) * sol;
        vs += sol;
        nc += 1.0;
    }
    inf.choques_acabados = nc;
    inf.duracion_choques = if vs > 0.0 { dv / vs } else { f64::NAN };
    inf.en_cuencas = d.cuentas.sedimento_en_cuencas_km3 / anios;
    inf
}

#[test]
#[ignore = "exploratoria: la deriva del planeta en varias semillas a la vez (SEMILLAS=42,1,2,3,4; VARIANTES=\"clave=valor,…;…\", como --set): balance, uniones triples, picos de espesor, finales de fosa y coste por etapa (~4 min con 10 derivas en paralelo)"]
fn deriva_en_varias_semillas() {
    let semillas: Vec<u64> = std::env::var("SEMILLAS")
        .unwrap_or_else(|_| "42".into())
        .split(',')
        .map(|x| x.trim().parse().expect("semilla"))
        .collect();
    // Variantes de la config separadas por «;» (cada una, ajustes «clave=valor» separados por comas).
    let variantes: Vec<String> = std::env::var("VARIANTES")
        .unwrap_or_default()
        .split(';')
        .map(|x| x.trim().to_string())
        .collect();
    let trabajos: Vec<(u64, &str)> = semillas
        .iter()
        .flat_map(|&s| variantes.iter().map(move |v| (s, v.as_str())))
        .collect();
    let informes: Vec<InformeDeriva> = std::thread::scope(|sc| {
        let hilos: Vec<_> = trabajos
            .iter()
            .map(|&(s, v)| sc.spawn(move || diagnosticar_deriva(s, v)))
            .collect();
        hilos.into_iter().map(|h| h.join().expect("hilo")).collect()
    });
    let nombres = [
        "sin borde",
        "dorsal",
        "fosa",
        "choque",
        "transformante",
        "difusa",
    ];
    for inf in &informes {
        println!("\n=== semilla {} [{}] ===", inf.semilla, inf.variante);
        let te: f64 = inf.erosion_por_exceso.iter().sum();
        let tt: f64 = inf.tierra_por_altura.iter().sum();
        println!(
            "erosión por altura sobre el mar (0–0,5, 0,5–1, 1–2, 2–3, >3 km): {:?} % de la erosión, {:?} % de la tierra",
            inf.erosion_por_exceso
                .map(|x| (1000.0 * x / te).round() / 10.0),
            inf.tierra_por_altura
                .map(|x| (1000.0 * x / tt).round() / 10.0)
        );
        let tc: f64 = inf.consumo.iter().flatten().sum();
        let ts: f64 = inf.solape.iter().flatten().sum();
        println!("consumo y solape continental por borde (% del total; normal | en unión triple):");
        for (i, nom) in nombres.iter().enumerate() {
            println!(
                "  {nom:<14} consumo {:6.2} | {:5.2}   solape {:6.2} | {:5.2}",
                100.0 * inf.consumo[i][0] / tc,
                100.0 * inf.consumo[i][1] / tc,
                100.0 * inf.solape[i][0] / ts,
                100.0 * inf.solape[i][1] / ts
            );
        }
        let nc = |x: &[[f64; 2]; 6], t: usize| x[1][t] + x[4][t];
        let tot = |x: &[[f64; 2]; 6]| x.iter().flatten().sum::<f64>();
        println!(
            "  no convergente: consumo {:.2} % (en uniones triples {:.2}), solape {:.2} % (en uniones triples {:.2}); borde en uniones triples {:.1} % de la longitud",
            100.0 * (nc(&inf.consumo, 0) + nc(&inf.consumo, 1)) / tot(&inf.consumo),
            100.0 * nc(&inf.consumo, 1) / tot(&inf.consumo),
            100.0 * (nc(&inf.solape, 0) + nc(&inf.solape, 1)) / tot(&inf.solape),
            100.0 * nc(&inf.solape, 1) / tot(&inf.solape),
            100.0 * inf.borde_triple_km / inf.borde_total_km
        );
        println!(
            "picos de más de 80 km: {} celdas·paso ({} en uniones triples), en {} pasos tras el rodaje y {} del rodaje; etapa que más sube el máximo en esos pasos: {:?}; pasos en que pasa de 80 una muestra que recibió dientes sin hueco {} o sueltas de otra placa {}",
            inf.picos,
            inf.picos_triples,
            inf.pasos_con_picos,
            inf.pasos_con_picos_rodaje,
            inf.etapa_de_los_picos,
            inf.picos_por_dientes[0],
            inf.picos_por_dientes[1]
        );
        println!(
            "espesor máximo de las muestras {:.1} km en el paso {} (sube sobre todo en {}); pasos con la más gruesa oculta: {}",
            inf.h_max, inf.h_max_paso, inf.h_max_etapa, inf.pasos_maximo_oculto
        );
        println!(
            "tras el rodaje: máximo de una muestra oculta {:.1} km, de una visible {:.1} km; picos según su muestra el paso anterior: visible {}, oculta de más de 60 km {}, otra {}",
            inf.max_oculta,
            inf.max_visible,
            inf.picos_seguian,
            inf.picos_reaparecen,
            inf.picos_otros
        );
        println!(
            "dientes continentales (AU): recolocados {:.2}, en la segunda pasada {:.2}, sumados a su placa {:.2}, a otra placa {:.2}; creación residual {:.3} km²/año",
            inf.dientes_via_au[0],
            inf.dientes_via_au[1],
            inf.dientes_via_au[2],
            inf.dientes_via_au[3],
            inf.creacion_residual
        );
        let to: f64 = inf.oblicuidad_transf.iter().sum();
        println!(
            "consumo en celdas transformantes por oblicuidad (0–5, 5–10, 10–15, 15–20, >20°): {:?} %; a menos de L_borde de una fosa del mismo par: {:.1} %",
            inf.oblicuidad_transf
                .map(|x| (1000.0 * x / to).round() / 10.0),
            100.0 * inf.transf_junto_a_fosa / inf.transf_total.max(1e-12)
        );
        println!(
            "balance (AU): arcos {:.2} choques {:.2} delaminación {:.3} al mar {:.2}; V_cont {:+.1} %; f_cont {:.3}; placas {}; A_sub {:.2} km²/año; fosas {:.0} km a {:.2} cm/año",
            inf.arcos,
            inf.choques,
            inf.delam,
            inf.al_mar,
            100.0 * inf.v_cont,
            inf.f_cont,
            inf.placas,
            inf.a_sub,
            inf.fosas_km,
            inf.conv_fosas
        );
        println!(
            "fosas (nota): activas {:.0} km a {:.2} cm/año, edad media {:.1} Myr, tirón {:.2}; difusas {:.0} km a {:.2} cm/año, absorben {:.3} km²/año; nacen por 100 Myr {:?} (inducidas, margen, transformante); mueren {:?} (sin convergencia, choque); fondo: media de los pasos {:.1} Myr, final mediana {:.1}, >180 {:.3}, >200 {:.3}; v_rms {:.2} cm/año; Forsyth–Uyeda fosa {:.2}",
            inf.fosas_km,
            inf.conv_fosas,
            inf.edad_fosas,
            inf.losa,
            inf.difusas_km,
            inf.conv_difusas,
            inf.area_difusa,
            inf.nacen_100.map(|x| (10.0 * x).round() / 10.0),
            inf.mueren_100.map(|x| (10.0 * x).round() / 10.0),
            inf.edad_oc_media,
            inf.edad_oc_mediana,
            inf.edad_oc_180,
            inf.edad_oc_200,
            inf.v_rms,
            inf.fu_fosa
        );
        let (oc, os) = (
            inf.oblicuidad[0].iter().sum::<f64>(),
            inf.oblicuidad[1].iter().sum::<f64>(),
        );
        let pot_motor = inf.potencia[0] + inf.potencia[1];
        println!(
            "fuerzas: oblicuidad (10° por clase, % que converge / que se separa) {:?} / {:?}; placas al final (% de la esfera) {:?}; potencia: tirón {:.0} % y empuje {:.0} % de la motriz, choques {:.0} % y arrastre {:.0} % de la resistente; creada {:.2} km²/año, consumida {:.2} + difusa {:.2}",
            inf.oblicuidad[0].map(|x| (1000.0 * x / oc).round() / 10.0),
            inf.oblicuidad[1].map(|x| (1000.0 * x / os).round() / 10.0),
            inf.tamanos
                .iter()
                .map(|x| (1000.0 * x).round() / 10.0)
                .collect::<Vec<_>>(),
            100.0 * inf.potencia[0] / pot_motor,
            100.0 * inf.potencia[1] / pot_motor,
            -100.0 * inf.potencia[2] / pot_motor,
            -100.0 * inf.potencia[3] / pot_motor,
            inf.creada,
            inf.a_sub,
            inf.area_difusa
        );
        let total: f64 = inf.segundos.values().sum();
        let mut etapas: Vec<_> = inf.segundos.iter().collect();
        etapas.sort_by(|a, b| b.1.total_cmp(a.1));
        println!(
            "tiempo {total:.0} s: {}",
            etapas
                .iter()
                .map(|(e, s)| format!("{e} {:.1} %", 100.0 * *s / total))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    // Resumen por variante: media ± desviación típica [extremos] entre semillas.
    for var in &variantes {
        let g: Vec<_> = informes.iter().filter(|i| &i.variante == var).collect();
        let r = |f: fn(&InformeDeriva) -> f64| {
            let v: Vec<f64> = g.iter().map(|i| f(i)).collect();
            let n = v.len() as f64;
            let m = v.iter().sum::<f64>() / n;
            let sd = (v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (n - 1.0).max(1.0)).sqrt();
            let (lo, hi) = v
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &x| {
                    (a.min(x), b.max(x))
                });
            format!("{m:.3} ± {sd:.3} [{lo:.3}–{hi:.3}]")
        };
        println!(
            "\n### [{var}], {} semillas: al mar {} AU; delaminación {}; choques {}; arcos {}; erosión por subducción {}; sedimento subducido {} y acrecionado {}; V_cont {}; f_cont {}; A_sub {}; fosas {} km a {} cm/año; h_max {}",
            g.len(),
            r(|i| i.al_mar),
            r(|i| i.delam),
            r(|i| i.choques),
            r(|i| i.arcos),
            r(|i| i.se),
            r(|i| i.sed_sub),
            r(|i| i.sed_acr),
            r(|i| i.v_cont),
            r(|i| i.f_cont),
            r(|i| i.a_sub),
            r(|i| i.fosas_km),
            r(|i| i.conv_fosas),
            r(|i| i.h_max)
        );
        println!(
            "### fosas: edad {} Myr; tirón {}; difusas {} km a {} cm/año, {} km²/año; nacen/100 Myr inducidas {} por separación de un margen invadido {} espontáneas en transformante {}; mueren sin convergencia {} choque {}; fondo media {} mediana {} >180 {} >200 {}; v_rms {}; F–U fosa {}; picos (celdas·paso) {}; máx. visible {}",
            r(|i| i.edad_fosas),
            r(|i| i.losa),
            r(|i| i.difusas_km),
            r(|i| i.conv_difusas),
            r(|i| i.area_difusa),
            r(|i| i.nacen_100[0]),
            r(|i| i.nacen_100[1]),
            r(|i| i.nacen_100[2]),
            r(|i| i.mueren_100[0]),
            r(|i| i.mueren_100[1]),
            r(|i| i.edad_oc_media),
            r(|i| i.edad_oc_mediana),
            r(|i| i.edad_oc_180),
            r(|i| i.edad_oc_200),
            r(|i| i.v_rms),
            r(|i| i.fu_fosa),
            r(|i| i.picos as f64),
            r(|i| i.max_visible)
        );
        let tiempo: Vec<f64> = g.iter().map(|i| i.segundos.values().sum::<f64>()).collect();
        println!(
            "### márgenes pasivos: vida media de los que terminan {} Myr; al final, edad media {} y máxima {} Myr; invasión por transformantes {} y por márgenes {} km/Myr (intentos de separar {}, fallidos {}, celdas de frente olvidadas {} por 100 Myr); roturas de losa {} por 100 Myr; choques: convergencia {} cm/año, pérdida con la losa colgando {} y sin ella {} AU, duración media de los que acaban {} Myr ({} choques)",
            r(|i| i.vida_margen),
            r(|i| i.edad_margen_final),
            r(|i| i.edad_max_margen_final),
            r(|i| i.invadido[0]),
            r(|i| i.invadido[1]),
            r(|i| i.separar_100[0]),
            r(|i| i.separar_100[1]),
            r(|i| i.separar_100[2]),
            r(|i| i.roturas_100),
            r(|i| i.conv_choques),
            r(|i| i.choque_con_losa),
            r(|i| i.choque_sin_losa),
            r(|i| i.duracion_choques),
            r(|i| i.choques_acabados)
        );
        println!(
            "### choques por la corteza que entra (continental <h_1 / h_1–h_2 / >h_2 / sobre fondo oceánico): solapado {} / {} / {} / {}; pérdida {} / {} / {} / {} (fracción de toda la deriva)",
            r(|i| i.por_grosor[0][0]),
            r(|i| i.por_grosor[0][1]),
            r(|i| i.por_grosor[0][2]),
            r(|i| i.por_grosor[0][3]),
            r(|i| i.por_grosor[1][0]),
            r(|i| i.por_grosor[1][1]),
            r(|i| i.por_grosor[1][2]),
            r(|i| i.por_grosor[1][3])
        );
        println!(
            "### zonas de fractura: nacen {} por 100 Myr (fallan al partir la placa {}); escalones de más de Δa_fz {} km, candidatos {} km",
            r(|i| i.zf[0]),
            r(|i| i.zf[1]),
            r(|i| i.zf[2]),
            r(|i| i.zf[3])
        );
        println!(
            "### sedimento: máximo de una muestra {} km; al mar siguiente {} AU; en cuencas cerradas {} AU",
            r(|i| i.sed_max),
            r(|i| i.desbordado),
            r(|i| i.en_cuencas)
        );
        println!(
            "### tiempo de la deriva {:.0} s de media",
            tiempo.iter().sum::<f64>() / tiempo.len() as f64
        );
    }
}

// --- Ciclo de vida de las fosas (nota_ciclo_fosas.md §10) ---------------------------------------

/// Dos placas oceánicas: la del oeste (más vieja, 120 Myr), una franja ecuatorial (|lat| < 10°,
/// −60° < lon < 0°), va al este a `v` cm/año en el ecuador contra la del resto (40 Myr), quieta. El
/// borde convergente es el meridiano 0 entre ±10°, con convergencia casi uniforme: la perpendicular
/// media, con la normal del borde suavizado, es ~0,93·v. Solo cinemática.
fn dos_oceanos_que_convergen(v: f64) -> Deriva {
    let cfg = TectonicaCfg::default();
    let malla = Malla::new(48, 1, 6371.0);
    let dirs = malla.dir.clone();
    let mut d = Deriva::desde(&cfg, 5, malla, 0.0, &[1, 2], |k| {
        let (la, lo) = lat_lon(dirs[k]);
        let (la, lo) = (la.to_degrees(), lo.to_degrees());
        if la.abs() < 10.0 && lo > -60.0 && lo < 0.0 {
            (0, corteza(false, 7.0, 120.0))
        } else {
            (1, corteza(false, 7.0, 40.0))
        }
    });
    d.placas[0].omega = [0.0, 0.0, v * 10.0 / 6371.0];
    d
}

fn nacimientos(d: &Deriva) -> Vec<&abm_terreno::tectonica::historia::EventoTectonico> {
    d.eventos
        .iter()
        .filter(|e| e.tipo == "nacimiento_fosa")
        .collect()
}

#[test]
fn una_convergencia_lenta_y_corta_no_hace_fosa_y_engruesa_conservando() {
    // Nota de fosas §10, prueba 1, con la regla de la v3 (§6.2): a ~1 cm/año durante 10 Myr
    // (~95 km < X_ini) no nace fosa. El fondo que converge se retira sin arco ni sumideros y su
    // basalto no pasa a corteza continental; lo que se reparte por la franja es el sedimento (aquí,
    // 0,5 km sobre los dos fondos), así que «la corteza se engruesa y el volumen se conserva» es el
    // sedimento y la corteza de tipo continental, exactos a redondeo.
    let mut d = dos_oceanos_que_convergen(1.0);
    for p in d.placas.iter_mut() {
        for c in p.cort.iter_mut() {
            c.sed = 0.5;
        }
    }
    let mut retirado = 0.0;
    for _ in 0..10 {
        let v0 = d.inventario_continental();
        d.paso_cinematico();
        let dv = d.inventario_continental() - v0;
        assert!(
            dv.abs() <= 1e-9 * v0,
            "la convergencia difusa conserva el sedimento: ΔV {dv:.6e} km³ de {v0:.6e}"
        );
        assert_eq!(d.area_subducida_km2, 0.0, "sin fosa no se hunde nada");
        retirado += d.fosas_paso.area_difusa_km2;
    }
    assert!(nacimientos(&d).is_empty(), "no nace ninguna fosa");
    assert!(retirado > 0.0, "se retira el fondo que converge");
    assert!(
        d.bordes.iter().any(|b| b.tipo == TipoBorde::Difusa),
        "el borde convergente es difuso"
    );
    let sed_max = (0..d.malla.num)
        .filter_map(|k| d.corteza(k))
        .map(|c| c.sed)
        .fold(0.0, f64::max);
    assert!(
        sed_max > 0.55,
        "el sedimento se acumula en la franja: {sed_max:.3} km"
    );
}

#[test]
fn una_convergencia_suficiente_hace_nacer_la_fosa_con_el_lado_viejo_abajo() {
    // Prueba 2: a ~5 cm/año de convergencia perpendicular (5,5 en el ecuador) la fosa nace a los
    // 3 Myr (150 km) y se hunde el lado más viejo.
    let mut d = dos_oceanos_que_convergen(5.5);
    for paso in 1..=3 {
        d.paso_cinematico();
        let n = nacimientos(&d);
        if paso < 3 {
            assert!(n.is_empty(), "aún no nace a los {paso} Myr");
        } else {
            assert_eq!(n.len(), 1, "nace a los 3 Myr: {n:?}");
            assert_eq!(n[0].clase.as_deref(), Some("inducido"));
            assert_eq!(
                n[0].placas[0], 1,
                "se hunde la placa vieja (id 1): {:?}",
                n[0]
            );
        }
    }
    d.paso_cinematico();
    d.paso_cinematico();
    assert!(d.area_subducida_km2 > 0.0, "la fosa activa consume placa");
    assert!(
        d.bordes
            .iter()
            .any(|b| b.tipo == TipoBorde::Subduccion && b.p_subduce && b.p == 0),
        "la placa vieja es la que se hunde"
    );
}

#[test]
fn la_polaridad_de_una_fosa_no_cambia_con_las_edades() {
    // Prueba 3: tras nacer, la polaridad se mantiene aunque el otro lado pase a ser más viejo.
    let mut d = dos_oceanos_que_convergen(5.5);
    for _ in 0..4 {
        d.paso_cinematico();
    }
    for c in d.placas[1].cort.iter_mut() {
        c.edad_oc = 300.0;
    }
    for _ in 0..3 {
        d.paso_cinematico();
    }
    let fosa: Vec<_> = d
        .bordes
        .iter()
        .filter(|b| b.tipo == TipoBorde::Subduccion && b.p_subduce)
        .collect();
    assert!(!fosa.is_empty(), "sigue habiendo fosa");
    assert!(
        fosa.iter().all(|b| b.p == 0),
        "se sigue hundiendo la placa del oeste"
    );
}

#[test]
fn una_fosa_cuyas_placas_se_separan_muere_tras_tau_muerte() {
    // Prueba 4: si las placas pasan a separarse, la fosa muere a los τ_muerte (10 Myr) y el tramo
    // pasa a divergente.
    let mut d = dos_oceanos_que_convergen(5.5);
    for _ in 0..4 {
        d.paso_cinematico();
    }
    assert_eq!(nacimientos(&d).len(), 1);
    d.placas[0].omega = [0.0, 0.0, -5.5 * 10.0 / 6371.0];
    let muertes = |d: &Deriva| {
        d.eventos
            .iter()
            .filter(|e| e.tipo == "muerte_fosa" && e.clase.as_deref() == Some("sin_convergencia"))
            .count()
    };
    let tau = d.cfg.tau_muerte_myr as usize;
    for paso in 1..=tau + 1 {
        d.paso_cinematico();
        if paso < tau {
            assert_eq!(muertes(&d), 0, "aún viva tras {paso} Myr sin converger");
        }
    }
    assert!(
        muertes(&d) >= 1,
        "la fosa muere tras {tau} Myr sin converger"
    );
    // En el meridiano 0 (al invertir el giro, el borde del oeste pasa a converger y allí puede nacer
    // otra fosa).
    let en_el_meridiano: Vec<_> = d
        .bordes
        .iter()
        .filter(|b| lat_lon(d.malla.dir[b.k as usize]).1.to_degrees().abs() < 8.0)
        .collect();
    assert!(
        !en_el_meridiano
            .iter()
            .any(|b| b.tipo == TipoBorde::Subduccion),
        "no queda fosa en el meridiano 0"
    );
    assert!(
        en_el_meridiano
            .iter()
            .any(|b| b.tipo == TipoBorde::Divergente),
        "el tramo es divergente"
    );
}

/// Un continente (casquete de 30° en 0°N 0°E) con fondo viejo alrededor en una placa, y otra placa
/// al otro lado del planeta.
fn margen_pasivo_aislado(cfg: &TectonicaCfg) -> Deriva {
    let malla = Malla::new(12, 2, 6371.0);
    let dirs = malla.dir.clone();
    Deriva::desde(cfg, 3, malla, 0.0, &[10, 20], |k| {
        let x = dirs[k];
        if x[0] > 0.0 {
            let cont = x[0] > 30f64.to_radians().cos();
            (0, corteza(cont, if cont { 35.0 } else { 7.0 }, 150.0))
        } else {
            (1, corteza(false, 7.0, 150.0))
        }
    })
}

#[test]
fn una_fosa_invade_un_margen_pasivo_y_separa_su_oceano_conservando_la_corteza() {
    // Nota de fosas v4 §10, prueba 5: una fosa cuyo extremo toca un margen pasivo con fondo viejo
    // se alarga por él a v_inv (aquí rápida, para la prueba) sin mover nada; cuando lo invadido ya
    // separa un océano del tamaño de una placa, ese océano pasa a ser una placa nueva con la fosa en
    // el tramo invadido, y la corteza se conserva.
    let cfg = TectonicaCfg {
        v_inv_km_myr: 300.0,
        tau_rift_myr: 1e12,
        lambda_esp: 0.0,
        ..TectonicaCfg::default()
    };
    let malla = Malla::new(12, 3, 6371.0);
    let dirs = malla.dir.clone();
    // Placa 0: una lengua oceánica de 150 Myr al sur del continente (20–60°S, 23,5°O–23,5°E). Placa
    // 1: el resto, con el continente (casquete de 30° en 0°N 0°E, al norte de 20°S) y su fondo, de
    // 170 Myr (más viejo que la lengua: no se invaden las transformantes de sus lados).
    let mut d = Deriva::desde(&cfg, 3, malla, 0.0, &[10, 20], |k| {
        let (la, lo) = lat_lon(dirs[k]);
        let (la, lo) = (la.to_degrees(), lo.to_degrees());
        if (-60.0..-20.0).contains(&la) && lo.abs() < 23.5 {
            (0, corteza(false, 7.0, 150.0))
        } else {
            let cont = dirs[k][0] > 30f64.to_radians().cos() && la >= -20.0;
            (1, corteza(cont, if cont { 35.0 } else { 7.0 }, 170.0))
        }
    });
    // La lengua avanza hacia el norte a ~5 cm/año y se hunde bajo el continente.
    let w = 50.0 / 6371.0;
    d.arrancar_con(&[[0.0, -w, 0.0], [0.0; 3]]);
    d.activar_fosas(d.cfg.l_losa_km);
    let v0 = d.inventario_continental();
    let (mut invadido, mut separa, mut tira) = (0.0, None, false);
    for paso in 1..=15u32 {
        d.paso_cinematico();
        if separa.is_none() {
            invadido += d.fosas_paso.invadido_km[1];
            if let Some(ev) = d.eventos.iter().find(|e| e.tipo == "invasion_margen") {
                separa = Some(paso);
                // La fosa del tramo invadido tira de la placa nueva hacia el continente.
                let nueva = d
                    .placas
                    .iter()
                    .position(|p| p.viva && p.id == ev.placas[0])
                    .expect("placa nueva") as u32;
                tira = d
                    .bordes
                    .iter()
                    .any(|b| b.p == nueva && b.p_subduce && b.losa > 0.0);
            }
        }
    }
    let paso = separa.expect("el océano de delante del margen invadido se separa");
    // Dos frentes (uno por esquina) a v_inv, durante los pasos hasta separar.
    let maximo = 2.0 * cfg.v_inv_km_myr * paso as f64 * 1.5;
    assert!(
        invadido > 0.0 && invadido <= maximo,
        "el frente avanza a v_inv: {invadido:.0} km en {paso} Myr (como mucho {maximo:.0})"
    );
    assert!(
        d.eventos.iter().any(|e| e.tipo == "placa_nueva"),
        "la parte oceánica es una placa nueva"
    );
    assert!(
        tira,
        "la fosa del tramo invadido tira de la placa nueva hacia el continente"
    );
    let dv = (d.inventario_continental() - v0).abs() / v0;
    assert!(
        dv < 1e-9,
        "la corteza se conserva: cambio relativo {dv:.2e}"
    );
}

#[test]
fn un_margen_pasivo_sin_fosa_cerca_no_se_hunde_nunca() {
    // Nota de fosas v4 §10, prueba 5 (segunda parte): un continente rodeado de fondo viejo y sin
    // ninguna fosa cerca no empieza a hundirse, ni con el nacimiento espontáneo muy alto (que ya
    // solo es de transformantes con contraste de edad).
    let cfg = TectonicaCfg {
        lambda_esp: 1.0,
        tau_rift_myr: 1e12,
        ..TectonicaCfg::default()
    };
    let mut d = margen_pasivo_aislado(&cfg);
    d.arrancar_con(&[[0.0; 3], [0.0; 3]]);
    for _ in 0..20 {
        d.paso_cinematico();
    }
    assert!(
        !d.eventos.iter().any(|e| matches!(
            e.tipo.as_str(),
            "placa_nueva" | "invasion_margen" | "nacimiento_fosa"
        )),
        "ningún margen pasivo se hunde solo: {:?}",
        d.eventos
    );
    assert!(
        !d.bordes.iter().any(|b| b.p_subduce && b.losa > 0.0),
        "ninguna fosa tira"
    );
}

/// Una placa oceánica (el hemisferio x > 0) con una zona de fractura en el ecuador: fondo de
/// 200 Myr al norte y de `joven` Myr al sur, con un continente al sur (45°S 0°E, 12° × 15°); la otra
/// placa (x < 0), de 100 Myr. Las dos, quietas.
fn zona_de_fractura(cfg: &TectonicaCfg, joven: f64) -> Deriva {
    let malla = Malla::new(12, 2, 6371.0);
    let dirs = malla.dir.clone();
    let mut d = Deriva::desde(cfg, 3, malla, 0.0, &[10, 20], |k| {
        let (la, lo) = lat_lon(dirs[k]);
        let (la, lo) = (la.to_degrees(), lo.to_degrees());
        if dirs[k][0] > 0.0 {
            let cont = (la + 45.0).abs() < 12.0 && lo.abs() < 15.0;
            let edad = if la > 0.0 { 200.0 } else { joven };
            (0, corteza(cont, if cont { 35.0 } else { 7.0 }, edad))
        } else {
            (1, corteza(false, 7.0, 100.0))
        }
    });
    d.arrancar_con(&[[0.0; 3], [0.0; 3]]);
    d
}

#[test]
fn una_zona_de_fractura_con_fondo_viejo_hace_nacer_una_fosa_y_parte_la_placa() {
    // Nota de fosas v5 §6.4: un escalón de edad dentro de una placa, de más de Δa_fz y de más de la
    // mitad de la edad vieja, con fondo de más de a_min, es candidato al nacimiento espontáneo (aquí
    // casi seguro). La fosa sigue el escalón, la placa se parte por él con el continente entero en
    // su lado, el lado viejo se hunde bajo el joven y tira, y la corteza se conserva.
    let cfg = TectonicaCfg {
        lambda_esp: 1.0,
        tau_rift_myr: 1e12,
        v_inv_km_myr: 0.0,
        ..TectonicaCfg::default()
    };
    let mut d = zona_de_fractura(&cfg, 50.0);
    let v0 = d.inventario_continental();
    d.paso_cinematico();
    assert!(
        d.fosas_paso.zf_candidata_km > 0.0,
        "el escalón del ecuador es candidato: {:?}",
        d.fosas_paso
    );
    let ev = d
        .eventos
        .iter()
        .find(|e| {
            e.tipo == "nacimiento_fosa" && e.clase.as_deref() == Some("espontaneo_zona_fractura")
        })
        .expect("nace una fosa en la zona de fractura");
    assert!(
        d.eventos.iter().any(|e| e.tipo == "placa_nueva"),
        "la placa se parte"
    );
    let hunde = d
        .placas
        .iter()
        .position(|p| p.viva && p.id == ev.placas[0])
        .expect("placa que se hunde") as u32;
    let arriba = d
        .placas
        .iter()
        .position(|p| p.viva && p.id == ev.placas[1])
        .expect("placa de arriba") as u32;
    let malla = d.malla.clone();
    for k in 0..malla.num {
        let (la, _) = lat_lon(malla.dir[k]);
        if d.dueno[k] == hunde {
            assert!(
                la > 0.0,
                "la placa que se hunde es el fondo viejo del norte"
            );
        }
        if d.corteza(k).is_some_and(|c| c.cont) {
            assert_eq!(
                d.dueno[k], arriba,
                "el continente va entero con el lado joven"
            );
        }
    }
    assert!(
        d.bordes
            .iter()
            .any(|b| b.p == hunde && b.q == arriba && b.p_subduce && b.losa > 0.0),
        "el lado viejo se hunde bajo el joven y tira"
    );
    assert!(
        d.dueno.iter().all(|&p| p != u32::MAX),
        "todo el planeta tiene dueño"
    );
    let dv = (d.inventario_continental() - v0).abs() / v0;
    assert!(
        dv < 1e-9,
        "la corteza se conserva: cambio relativo {dv:.2e}"
    );
}

#[test]
fn un_escalon_pequeno_en_fondo_viejo_no_hace_nacer_ninguna_fosa() {
    // Nota de fosas v5 §6.4: 200 frente a 160 Myr es zona de fractura (escalón de más de Δa_fz)
    // pero sin el contraste de la regla (menos de la mitad de 200): no es candidata.
    let cfg = TectonicaCfg {
        lambda_esp: 1.0,
        tau_rift_myr: 1e12,
        v_inv_km_myr: 0.0,
        ..TectonicaCfg::default()
    };
    let mut d = zona_de_fractura(&cfg, 160.0);
    for _ in 0..5 {
        d.paso_cinematico();
        assert!(
            d.fosas_paso.zf_escalon_km > 0.0 && d.fosas_paso.zf_candidata_km == 0.0,
            "zona de fractura sin candidatos: {:?}",
            d.fosas_paso
        );
    }
    assert!(
        !d.eventos.iter().any(|e| e.tipo == "nacimiento_fosa"),
        "no nace ninguna fosa"
    );
}

#[test]
fn en_un_choque_se_pierde_la_corteza_fina_y_se_queda_la_gruesa() {
    // Adenda v7 §6.5: η(h) = η_min por debajo de h_1 = 20 km, 1 por encima de h_2 = 35 km y lineal
    // entre los dos.
    let cfg = TectonicaCfg {
        eta_min: 0.2,
        ..TectonicaCfg::default()
    };
    assert_eq!(cfg.eta_choque(10.0), 0.2);
    assert_eq!(cfg.eta_choque(20.0), 0.2);
    assert!((cfg.eta_choque(27.5) - 0.6).abs() < 1e-12);
    assert_eq!(cfg.eta_choque(35.0), 1.0);
    assert_eq!(cfg.eta_choque(45.0), 1.0);
}

#[test]
fn la_losa_sigue_tirando_tras_el_choque_y_se_rompe_a_los_t_rot() {
    // Nota de fosas v4 §10, prueba 6: en el choque India–Asia, la losa del Tetis (fondo de 80 Myr)
    // sigue colgando y tirando bajo la India durante t_rot = 10 + 15·0,8 = 22 Myr desde el inicio
    // del choque, y después se rompe y deja de tirar.
    let cfg = cfg_india_asia();
    let t_rot = cfg.t_rot_joven_myr + (cfg.t_rot_viejo_myr - cfg.t_rot_joven_myr) * 0.8;
    let mut d = choque_india_asia(&cfg, 1, 48);
    // La India y Asia, por su posición (sus índices pueden cambiar si se parte otra placa).
    let placa_en = |d: &Deriva, la: f64, lo: f64| {
        let (la, lo) = (la.to_radians(), lo.to_radians());
        d.dueno[d
            .malla
            .localizar([la.cos() * lo.cos(), la.cos() * lo.sin(), la.sin()])]
    };
    // Bordes de la India frente a Asia en el frente del choque.
    let centro = |d: &Deriva, b: &abm_terreno::tectonica::deriva::Borde| {
        b.p == placa_en(d, 7.5, 80.0) && b.q == placa_en(d, 40.0, 80.0)
    };
    let (mut inicio, mut rotura, mut colgando, mut pasos_choque) = (None, None, 0u32, 0u32);
    let mut t_evento = None;
    for paso in 1..=60u32 {
        d.dar_paso();
        // El choque empieza cuando choca el frente (al menos tres bordes), no con un contacto suelto.
        let choca = d
            .bordes
            .iter()
            .filter(|b| centro(&d, b) && b.tipo == TipoBorde::Colision)
            .count()
            >= 3;
        if inicio.is_none() && choca {
            inicio = Some(paso);
        }
        let frente = d
            .bordes
            .iter()
            .filter(|b| centro(&d, b) && b.tipo == TipoBorde::Colision)
            .count();
        let tiran = d
            .bordes
            .iter()
            .filter(|b| {
                centro(&d, b) && b.tipo == TipoBorde::Colision && b.p_subduce && b.losa > 0.0
            })
            .count();
        if inicio.is_some() && rotura.is_none() {
            pasos_choque += 1;
            colgando += u32::from(tiran > 0);
            let (ii, ia) = (
                d.placas[placa_en(&d, 7.5, 80.0) as usize].id,
                d.placas[placa_en(&d, 40.0, 80.0) as usize].id,
            );
            if let Some(e) = d.eventos.iter().find(|e| {
                e.tipo == "rotura_losa" && e.placas.contains(&ii) && e.placas.contains(&ia)
            }) {
                rotura = Some(paso);
                t_evento = e.tiempo_choque_myr;
            }
        } else if rotura.is_some_and(|r| (r + 3..=r + 10).contains(&paso)) {
            // Rota la losa del choque, solo puede colgar aún en los costados del frente, donde los
            // contactos nuevos siguen unidos a la losa del Tetis (el desgarro avanza a lo largo).
            assert!(
                2 * tiran < frente,
                "tras la rotura, la losa ya no tira en la mayor parte del frente (paso {paso}: {tiran} de {frente})"
            );
        }
    }
    let inicio = inicio.expect("la India choca con Asia");
    let rotura = rotura.expect("la losa se rompe");
    let t = t_evento.expect("tiempo de la rotura");
    assert!(
        (t - t_rot).abs() <= 2.0,
        "la losa se rompe a los t_rot = {t_rot} Myr del choque: {t}"
    );
    assert!(
        ((rotura - inicio) as f64 - t_rot).abs() <= 3.0,
        "rotura {rotura} Myr tras el inicio del choque {inicio}"
    );
    assert!(
        colgando * 10 >= pasos_choque * 8,
        "mientras cuelga, la losa tira bajo la India: {colgando} de {pasos_choque} pasos"
    );
}

#[test]
#[ignore = "exploratoria: ¿son los picos de más de 100 km columnas reales o artefactos de muestras? (SEMILLAS; ~4 min)"]
fn diagnostico_picos() {
    let semillas: Vec<u64> = std::env::var("SEMILLAS")
        .unwrap_or_else(|_| "42".into())
        .split(',')
        .map(|x| x.trim().parse().expect("semilla"))
        .collect();
    let informes: Vec<String> = std::thread::scope(|sc| {
        let hilos: Vec<_> = semillas
            .iter()
            .map(|&semilla| {
                sc.spawn(move || {
                    let cfg = abm_terreno::config::TerrenoConfig::load("../../config/terreno.toml")
                        .expect("config");
                    let tc = cfg.tectonica.clone();
                    let malla = Malla::new(cfg.planet.n0, tc.nivel_malla, cfg.planet.r_km);
                    let mut d = Deriva::nueva(&tc, semilla, malla);
                    let pasos = (tc.t_deriva_myr / tc.dt_myr).round() as u64;
                    let rodaje = (tc.t_rodaje_myr / tc.dt_myr).round() as u64;
                    let umbral = 100.0;
                    // Por celda-paso de más de 100 km: vecinas gruesas, muestra compartida, centro fuera,
                    // persistencia de su muestra y distancia a un choque.
                    let (mut n, mut meseta, mut aguja, mut compartida, mut centro_fuera) =
                        (0u64, 0u64, 0u64, 0u64, 0u64);
                    let (mut en_choque, mut persiste, mut salto) = (0u64, 0u64, 0u64);
                    let mut area_80: Vec<f64> = Vec::new();
                    let mut ant: std::collections::HashMap<(u32, u32), (f64, u32)> =
                        Default::default();
                    let cos_choque = (300.0 / d.malla.r_km).cos();
                    for paso in 1..=pasos {
                        d.dar_paso();
                        if paso <= rodaje {
                            continue;
                        }
                        let mut usos: std::collections::HashMap<(u32, u32), u32> = Default::default();
                        for k in 0..d.malla.num {
                            if d.dueno[k] != NADA && d.rep[k] != NADA {
                                *usos.entry((d.dueno[k], d.rep[k])).or_insert(0) += 1;
                            }
                        }
                        let choques: Vec<usize> = d
                            .bordes
                            .iter()
                            .filter(|b| b.tipo == TipoBorde::Colision)
                            .map(|b| b.k as usize)
                            .collect();
                        let mut a80 = 0.0;
                        let mut nuevo: std::collections::HashMap<(u32, u32), (f64, u32)> =
                            Default::default();
                        for k in 0..d.malla.num {
                            let Some(c) = d.corteza(k).filter(|c| c.cont) else {
                                continue;
                            };
                            if c.h > 80.0 {
                                a80 += d.malla.area[k];
                            }
                            if c.h <= umbral {
                                continue;
                            }
                            n += 1;
                            let (p, l) = (d.dueno[k], d.rep[k]);
                            let vec: Vec<f64> = d.malla.vec[k]
                                .iter()
                                .filter(|&&v| v != NADA)
                                .filter_map(|&v| d.corteza(v as usize).map(|x| x.h))
                                .collect();
                            let gruesas = vec.iter().filter(|&&x| x > 80.0).count();
                            if gruesas >= 3 {
                                meseta += 1;
                            }
                            if vec.iter().all(|&x| x < 0.7 * c.h) {
                                aguja += 1;
                            }
                            if usos.get(&(p, l)).copied().unwrap_or(0) > 1 {
                                compartida += 1;
                            }
                            let s = d.placas[p as usize].mapa[l as usize] as usize;
                            if d.placas[p as usize].mundo[s] as usize != k {
                                centro_fuera += 1;
                            }
                            let x = d.malla.dir[k];
                            if choques.iter().any(|&j| {
                                let y = d.malla.dir[j];
                                x[0] * y[0] + x[1] * y[1] + x[2] * y[2] >= cos_choque
                            }) {
                                en_choque += 1;
                            }
                            let pasos_seguidos = match ant.get(&(p, l)) {
                                Some(&(h0, m)) => {
                                    if c.h - h0 > 20.0 {
                                        salto += 1;
                                    }
                                    m + 1
                                }
                                None => 1,
                            };
                            if pasos_seguidos >= 5 {
                                persiste += 1;
                            }
                            nuevo.insert((p, l), (c.h, pasos_seguidos));
                        }
                        // Las muestras de más de 80 km que siguen, para la persistencia.
                        for k in 0..d.malla.num {
                            if let Some(c) = d.corteza(k).filter(|c| c.cont && c.h > 80.0 && c.h <= umbral) {
                                nuevo.entry((d.dueno[k], d.rep[k])).or_insert((c.h, 1));
                            }
                        }
                        ant = nuevo;
                        area_80.push(a80);
                    }
                    let f = |x: u64| 100.0 * x as f64 / n.max(1) as f64;
                    let a80_media = area_80.iter().sum::<f64>() / area_80.len() as f64;
                    format!(
                        "semilla {semilla}: {n} celdas·paso de más de {umbral} km; con 3 o más vecinas de más de 80 km {:.0} %; aguja (todas las vecinas < 70 %) {:.0} %; muestra que ven varias celdas {:.0} %; centro de la muestra en otra celda {:.0} %; a menos de 300 km de un choque {:.0} %; la misma muestra 5 pasos o más {:.0} %; subida de más de 20 km en un paso {:.0} %; área de más de 80 km {:.0} km² de media",
                        f(meseta),
                        f(aguja),
                        f(compartida),
                        f(centro_fuera),
                        f(en_choque),
                        f(persiste),
                        f(salto),
                        a80_media
                    )
                })
            })
            .collect();
        hilos.into_iter().map(|h| h.join().expect("hilo")).collect()
    });
    for i in informes {
        println!("{i}");
    }
}

#[test]
#[ignore = "exploratoria: de dónde salen las muestras de cientos de km (SEMILLA, VARIANTE, UMBRAL, PICOS)"]
fn diagnostico_pilas() {
    // Dos pasadas de la misma deriva (es determinista): la primera halla los pasos en los que alguna
    // etapa deja una muestra continental por encima de UMBRAL (aunque otra etapa del mismo paso la
    // baje) y la segunda sigue esa muestra etapa a etapa en ese paso.
    let semilla: u64 = std::env::var("SEMILLA").map_or(3, |x| x.parse().expect("semilla"));
    let variante = std::env::var("VARIANTE").unwrap_or_default();
    let umbral: f64 = std::env::var("UMBRAL").map_or(150.0, |x| x.parse().expect("umbral"));
    let n_picos: usize = std::env::var("PICOS").map_or(12, |x| x.parse().expect("picos"));
    let ajustes: Vec<String> = variante
        .split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect();
    let cfg = abm_terreno::config::TerrenoConfig::load_con("../../config/terreno.toml", &ajustes)
        .expect("config");
    let tc = cfg.tectonica.clone();
    let pasos = (tc.t_deriva_myr / tc.dt_myr).round() as u64;
    let nueva = || {
        let malla = Malla::new(cfg.planet.n0, tc.nivel_malla, cfg.planet.r_km);
        let mut d = Deriva::nueva(&tc, semilla, malla);
        d.traza = Some(Vec::new());
        d
    };
    // Pasada 1.
    let mut d = nueva();
    let mut picos: Vec<(u64, &'static str, (u32, u32), f64)> = Vec::new();
    let mut n_pasos_pico = 0;
    let (mut sed_max, mut sed_max_paso) = (0.0f64, 0u64);
    for paso in 1..=pasos {
        d.dar_paso();
        for c in d
            .placas
            .iter()
            .filter(|p| p.viva)
            .flat_map(|p| p.cort.iter())
        {
            if c.sed > sed_max {
                (sed_max, sed_max_paso) = (c.sed, paso);
            }
        }
        let traza = d.traza.replace(Vec::new()).unwrap_or_default();
        let Some(e) = traza
            .iter()
            .max_by(|a, b| a.h_max_km.total_cmp(&b.h_max_km))
        else {
            continue;
        };
        if e.h_max_km > umbral {
            n_pasos_pico += 1;
            if picos.len() < n_picos {
                picos.push((paso, e.etapa, e.max_en, e.h_max_km));
            }
        }
    }
    println!(
        "semilla {semilla}: {n_pasos_pico} pasos con alguna etapa por encima de {umbral} km; sedimento máximo de una muestra al final de un paso {sed_max:.1} km (paso {sed_max_paso}), desbordado al mar siguiente {:.3} AU; los primeros:",
        d.cuentas.sedimento_desbordado_km3 / (pasos as f64 * tc.dt_myr * 1e6)
    );
    // Pasada 2.
    let mut d = nueva();
    let r = d.malla.r_km;
    let mut cola = picos.iter().peekable();
    for paso in 1..=pasos {
        let Some(&&(pp, etapa, (pi, l), h)) = cola.peek() else {
            break;
        };
        if paso != pp {
            d.dar_paso();
            let _ = d.traza.replace(Vec::new());
            continue;
        }
        cola.next();
        let antes = d.placas.get(pi as usize).and_then(|p| p.get(l)).copied();
        d.vigilada = Some((pi, l));
        d.dar_paso();
        d.vigilada = None;
        let traza = d.traza.replace(Vec::new()).unwrap_or_default();
        let mut cambios = String::new();
        let mut ant = antes;
        for e in &traza {
            match (ant, e.vigilada) {
                (Some(a), Some(b)) => {
                    let (dh, ds) = (b.h - a.h, b.sed - a.sed);
                    if dh.abs() > 0.5 || ds.abs() > 0.5 {
                        cambios += &format!(" {}:{dh:+.0}/{ds:+.1}", e.etapa);
                    }
                }
                (None, Some(b)) => cambios += &format!(" {}:aparece con {:.0}", e.etapa, b.h),
                (Some(_), None) => cambios += &format!(" {}:desaparece", e.etapa),
                (None, None) => {}
            }
            ant = e.vigilada;
        }
        let mut ctx = String::from("(la muestra no sigue al final)");
        if let Some(pl) = d.placas.get(pi as usize).filter(|p| p.viva)
            && let Some(c) = pl.get(l).copied()
        {
            let s = pl.mapa[l as usize] as usize;
            let k0 = pl.mundo[s] as usize;
            let ven = (0..d.malla.num)
                .filter(|&k| d.dueno[k] == pi && d.rep[k] == l)
                .count();
            let n_cont = pl.cort.iter().filter(|c| c.cont).count();
            let (mut dmin, mut tipo) = (f64::INFINITY, String::new());
            let mut fosa_arriba = 0.0;
            for b in d.bordes.iter().filter(|b| b.p == pi) {
                let (a, b2) = (d.malla.dir[k0], d.malla.dir[b.k as usize]);
                let ang = (a[0] * b2[0] + a[1] * b2[1] + a[2] * b2[2])
                    .clamp(-1.0, 1.0)
                    .acos();
                if ang * r < dmin {
                    dmin = ang * r;
                    tipo = format!("{:?}{}", b.tipo, if b.p_subduce { "↓" } else { "" });
                }
                if b.tipo == TipoBorde::Subduccion && !b.p_subduce {
                    fosa_arriba += b.largo_km;
                }
            }
            let vecinas: Vec<String> = d.malla.vec[k0]
                .iter()
                .filter(|&&v| v != NADA)
                .map(|&v| match d.corteza(v as usize) {
                    Some(cv) if d.dueno[v as usize] == pi => format!("{:.0}", cv.hc),
                    Some(cv) => format!("o{:.0}", cv.hc),
                    None => "-".into(),
                })
                .collect();
            ctx = format!(
                "al final h {:.0} hc {:.0} sed {:.1} arco {} edad_oc {:.0}; placa id {} con {} muestras ({} continentales, fosa de arriba {:.0} km); la ven {ven} celdas (centro en {}); borde más cercano {dmin:.0} km ({tipo}); vecinas [{}]",
                c.h,
                c.hc,
                c.sed,
                c.arco,
                c.edad_oc,
                pl.id,
                pl.cort.len(),
                n_cont,
                fosa_arriba,
                if d.dueno[k0] == pi {
                    "su placa"
                } else {
                    "otra placa"
                },
                vecinas.join(" "),
            );
        }
        let a = antes.map_or("no existía".to_string(), |c| {
            format!("h {:.0} hc {:.0} sed {:.1}", c.h, c.hc, c.sed)
        });
        println!(
            "paso {pp}: pico {h:.0} km en {etapa}, placa {pi} celda {l}; antes del paso {a};{cambios}; {ctx}"
        );
    }
}

#[test]
#[ignore = "exploratoria: dónde queda el fondo de más de 180 Myr al final de la deriva (SEMILLAS, VARIANTE)"]
fn diagnostico_fondo_viejo() {
    let semillas: Vec<u64> = std::env::var("SEMILLAS")
        .unwrap_or_else(|_| "42".into())
        .split(',')
        .map(|x| x.trim().parse().expect("semilla"))
        .collect();
    let variante = std::env::var("VARIANTE").unwrap_or_default();
    let informes: Vec<String> = std::thread::scope(|sc| {
        let hilos: Vec<_> = semillas
            .iter()
            .map(|&semilla| {
                let variante = variante.clone();
                sc.spawn(move || {
                    let ajustes: Vec<String> = variante
                        .split(',')
                        .map(|x| x.trim().to_string())
                        .filter(|x| !x.is_empty())
                        .collect();
                    let cfg = abm_terreno::config::TerrenoConfig::load_con(
                        "../../config/terreno.toml",
                        &ajustes,
                    )
                    .expect("config");
                    let tc = cfg.tectonica.clone();
                    let malla = Malla::new(cfg.planet.n0, tc.nivel_malla, cfg.planet.r_km);
                    let mut d = Deriva::nueva(&tc, semilla, malla);
                    let pasos = (tc.t_deriva_myr / tc.dt_myr).round() as u64;
                    for _ in 0..pasos {
                        d.dar_paso();
                    }
                    let num = d.malla.num;
                    let r = d.malla.r_km;
                    let dot3 = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
                    // Celdas por placa, fosas donde se hunde cada placa y su orilla continental.
                    let np = d.placas.len();
                    let mut celdas = vec![0usize; np];
                    for k in 0..num {
                        if d.dueno[k] != NADA {
                            celdas[d.dueno[k] as usize] += 1;
                        }
                    }
                    let mut rango: Vec<usize> = (0..np).collect();
                    rango.sort_by(|&a, &b| celdas[b].cmp(&celdas[a]));
                    let mut puesto = vec![usize::MAX; np];
                    for (i, &p) in rango.iter().enumerate() {
                        puesto[p] = i;
                    }
                    let mut fosa: Vec<Vec<usize>> = vec![Vec::new(); np];
                    for b in &d.bordes {
                        if b.tipo == TipoBorde::Subduccion && b.p_subduce {
                            fosa[b.p as usize].push(b.k as usize);
                        }
                    }
                    let mut orilla: Vec<Vec<usize>> = vec![Vec::new(); np];
                    for k in 0..num {
                        let p = d.dueno[k];
                        if p == NADA || !d.corteza(k).is_some_and(|c| c.cont) {
                            continue;
                        }
                        let junto_al_mar = d.malla.vec[k].iter().any(|&v| {
                            v != NADA
                                && d.dueno[v as usize] == p
                                && d.corteza(v as usize).is_some_and(|c| !c.cont)
                        });
                        if junto_al_mar {
                            orilla[p as usize].push(k);
                        }
                    }
                    let dist_a = |k: usize, lista: &[usize]| -> f64 {
                        lista
                            .iter()
                            .map(|&j| dot3(d.malla.dir[k], d.malla.dir[j]))
                            .fold(-1.0, f64::max)
                            .clamp(-1.0, 1.0)
                            .acos()
                            * r
                    };
                    let (mut n_oc, mut n_viejo) = (0usize, 0usize);
                    let mut por_puesto = [0usize; 4];
                    let (mut sin_fosa, mut fosa_cerca, mut fosa_media, mut fosa_lejos) =
                        (0usize, 0usize, 0usize, 0usize);
                    let (mut sin_cont, mut cont_cerca, mut cont_media, mut cont_lejos) =
                        (0usize, 0usize, 0usize, 0usize);
                    for k in 0..num {
                        let p = d.dueno[k];
                        let Some(c) = d.corteza(k) else {
                            continue;
                        };
                        if p == NADA || c.cont {
                            continue;
                        }
                        n_oc += 1;
                        if c.edad_oc <= 180.0 {
                            continue;
                        }
                        n_viejo += 1;
                        por_puesto[puesto[p as usize].min(3)] += 1;
                        let pf = &fosa[p as usize];
                        if pf.is_empty() {
                            sin_fosa += 1;
                        } else {
                            match dist_a(k, pf) {
                                x if x < 1000.0 => fosa_cerca += 1,
                                x if x < 3000.0 => fosa_media += 1,
                                _ => fosa_lejos += 1,
                            }
                        }
                        let po = &orilla[p as usize];
                        if po.is_empty() {
                            sin_cont += 1;
                        } else {
                            match dist_a(k, po) {
                                x if x < 300.0 => cont_cerca += 1,
                                x if x < 1000.0 => cont_media += 1,
                                _ => cont_lejos += 1,
                            }
                        }
                    }
                    let pc = |x: usize| 100.0 * x as f64 / n_viejo.max(1) as f64;
                    let fr: Vec<String> = rango
                        .iter()
                        .take(4)
                        .map(|&p| format!("{:.0}", 100.0 * celdas[p] as f64 / num as f64))
                        .collect();
                    format!(
                        "semilla {semilla}: fondo de más de 180 Myr {:.1} % del fondo; en la placa mayor {:.0} %, 2.ª {:.0} %, 3.ª {:.0} %, resto {:.0} % (placas mayores {} % del planeta); en placas sin fosa donde se hunda {:.0} %; a la fosa propia <1000 km {:.0} %, 1000–3000 {:.0} %, >3000 {:.0} %; a la orilla continental propia <300 km {:.0} %, 300–1000 {:.0} %, >1000 {:.0} %, placa sin continente {:.0} %",
                        100.0 * n_viejo as f64 / n_oc.max(1) as f64,
                        pc(por_puesto[0]),
                        pc(por_puesto[1]),
                        pc(por_puesto[2]),
                        pc(por_puesto[3]),
                        fr.join("/"),
                        pc(sin_fosa),
                        pc(fosa_cerca),
                        pc(fosa_media),
                        pc(fosa_lejos),
                        pc(cont_cerca),
                        pc(cont_media),
                        pc(cont_lejos),
                        pc(sin_cont),
                    )
                })
            })
            .collect();
        hilos.into_iter().map(|h| h.join().expect("hilo")).collect()
    });
    for i in informes {
        println!("{i}");
    }
}

/// Qué son los candidatos a zona de fractura de la nota v5 en el planeta (SEMILLAS, VARIANTE como
/// en `diagnostico_fondo_viejo`): cada 100 Myr, longitud de los pares candidatos, edades de sus dos
/// lados, cuántas celdas jóvenes son «agujeros» (todas sus vecinas oceánicas de la placa son más
/// viejas que ella en más de `Δa_fz`), cuántas están junto a otra placa y el tamaño de los grupos
/// conexos de pares.
#[test]
#[ignore = "diagnóstico: los candidatos a zona de fractura de la nota v5 en el planeta (~3 min)"]
fn diagnostico_zonas_fractura() {
    use abm_terreno::tectonica::malla::LADOS;
    let semillas: Vec<u64> = std::env::var("SEMILLAS")
        .unwrap_or_else(|_| "42".into())
        .split(',')
        .map(|x| x.trim().parse().expect("semilla"))
        .collect();
    let variante = std::env::var("VARIANTE").unwrap_or_default();
    let informes: Vec<String> = std::thread::scope(|sc| {
        let hilos: Vec<_> = semillas
            .iter()
            .map(|&semilla| {
                let variante = variante.clone();
                sc.spawn(move || {
                    let ajustes: Vec<String> = variante
                        .split(',')
                        .map(|x| x.trim().to_string())
                        .filter(|x| !x.is_empty())
                        .collect();
                    let cfg = abm_terreno::config::TerrenoConfig::load_con(
                        "../../config/terreno.toml",
                        &ajustes,
                    )
                    .expect("config");
                    let tc = cfg.tectonica.clone();
                    let malla = Malla::new(cfg.planet.n0, tc.nivel_malla, cfg.planet.r_km);
                    let mut d = Deriva::nueva(&tc, semilla, malla);
                    let pasos = (tc.t_deriva_myr / tc.dt_myr).round() as u64;
                    let mut out = format!("semilla {semilla} [{variante}]\n");
                    for paso in 1..=pasos {
                        d.dar_paso();
                        if paso % 100 != 0 && paso != pasos {
                            continue;
                        }
                        let num = d.malla.num;
                        let (da, a_min) = (tc.da_fz_myr, tc.a_min_myr);
                        let mut pares: Vec<(usize, usize, f64)> = Vec::new();
                        let (mut largo, mut joven, mut viejo) = (0.0, [0.0f64; 5], [0.0f64; 4]);
                        for k in 0..num {
                            let p = d.dueno[k];
                            let Some(ck) = (if p == NADA { None } else { d.corteza(k) }) else {
                                continue;
                            };
                            if ck.cont {
                                continue;
                            }
                            for (i, &j) in LADOS.iter().enumerate() {
                                let v = d.malla.vec[k][j];
                                if v == NADA || (v as usize) <= k || d.dueno[v as usize] != p {
                                    continue;
                                }
                                let Some(cv) = d.corteza(v as usize) else {
                                    continue;
                                };
                                if cv.cont {
                                    continue;
                                }
                                let dif = (ck.edad_oc - cv.edad_oc).abs();
                                let vieja = ck.edad_oc.max(cv.edad_oc);
                                if dif <= da || vieja <= a_min || dif <= 0.5 * vieja {
                                    continue;
                                }
                                let l = d.malla.lado_comun[k][i];
                                largo += l;
                                let jv = ck.edad_oc.min(cv.edad_oc);
                                let bj = [5.0, 20.0, 50.0, 100.0].iter().filter(|&&x| jv >= x).count();
                                joven[bj] += l;
                                let bv = [200.0, 300.0, 500.0].iter().filter(|&&x| vieja >= x).count();
                                viejo[bv] += l;
                                let (o, y) = if ck.edad_oc > cv.edad_oc { (k, v as usize) } else { (v as usize, k) };
                                pares.push((o, y, l));
                            }
                        }
                        // Celdas jóvenes: agujeros y junto a otra placa.
                        let mut jovenes: Vec<usize> = pares.iter().map(|x| x.1).collect();
                        jovenes.sort_unstable();
                        jovenes.dedup();
                        let (mut agujero, mut junto_borde) = (0usize, 0usize);
                        for &y in &jovenes {
                            let ey = d.corteza(y).map_or(0.0, |c| c.edad_oc);
                            let p = d.dueno[y];
                            let (mut todas_viejas, mut otra) = (true, false);
                            for &v in &d.malla.vec[y] {
                                if v == NADA {
                                    continue;
                                }
                                if d.dueno[v as usize] != p {
                                    otra = true;
                                    continue;
                                }
                                if let Some(cv) = d.corteza(v as usize)
                                    && !cv.cont
                                    && cv.edad_oc <= ey + da
                                {
                                    todas_viejas = false;
                                }
                            }
                            agujero += usize::from(todas_viejas);
                            junto_borde += usize::from(otra);
                        }
                        // Grupos conexos (8 vecinas) de las celdas de los pares, y su longitud.
                        let mut en: std::collections::HashMap<usize, usize> = Default::default();
                        for (i, &(o, y, _)) in pares.iter().enumerate() {
                            en.insert(o, i);
                            en.insert(y, i);
                        }
                        let mut grupo = vec![usize::MAX; pares.len()];
                        let mut largos: Vec<f64> = Vec::new();
                        for s in 0..pares.len() {
                            if grupo[s] != usize::MAX {
                                continue;
                            }
                            let g = largos.len();
                            largos.push(0.0);
                            grupo[s] = g;
                            let mut pila = vec![s];
                            while let Some(i) = pila.pop() {
                                largos[g] += pares[i].2;
                                for c in [pares[i].0, pares[i].1] {
                                    for &w in d.malla.vec[c].iter() {
                                        if w == NADA {
                                            continue;
                                        }
                                        if let Some(&j) = en.get(&(w as usize))
                                            && grupo[j] == usize::MAX
                                        {
                                            grupo[j] = g;
                                            pila.push(j);
                                        }
                                    }
                                }
                            }
                        }
                        let mut por_tam = [0.0f64; 4];
                        for &l in &largos {
                            por_tam[[200.0, 1000.0, 3000.0].iter().filter(|&&x| l >= x).count()] += l;
                        }
                        let f = |x: f64| if largo > 0.0 { 100.0 * x / largo } else { 0.0 };
                        out += &format!(
                            "  {paso:>3} Myr: candidatos {largo:>7.0} km en {} grupos (de <200 / 200–1000 / 1000–3000 / >3000 km: {:.0} / {:.0} / {:.0} / {:.0} % de la longitud); lado joven <5 / 5–20 / 20–50 / 50–100 / >100 Myr: {:.0} / {:.0} / {:.0} / {:.0} / {:.0} %; lado viejo 140–200 / 200–300 / 300–500 / >500: {:.0} / {:.0} / {:.0} / {:.0} %; celdas jóvenes {}: agujeros {:.0} %, junto a otra placa {:.0} %\n",
                            largos.len(),
                            f(por_tam[0]), f(por_tam[1]), f(por_tam[2]), f(por_tam[3]),
                            f(joven[0]), f(joven[1]), f(joven[2]), f(joven[3]), f(joven[4]),
                            f(viejo[0]), f(viejo[1]), f(viejo[2]), f(viejo[3]),
                            jovenes.len(),
                            100.0 * agujero as f64 / jovenes.len().max(1) as f64,
                            100.0 * junto_borde as f64 / jovenes.len().max(1) as f64,
                        );
                    }
                    out
                })
            })
            .collect();
        hilos.into_iter().map(|h| h.join().expect("hilo")).collect()
    });
    for i in informes {
        println!("{i}");
    }
}
