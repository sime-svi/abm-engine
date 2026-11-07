//! Relieve tectónico (incremento 4): pruebas de la deriva de placas y del relieve que deja.
//!
//! Diseño: `docs/design/terreno/diseno_relieve_tectonico.md` §10 y `incremento-4-diseno.md`.
//! Las pruebas corren en mallas gruesas y derivas cortas para que quepan en `cargo test` (debug).

use abm_terreno::config::TectonicaCfg;
use abm_terreno::geometry::lat_lon;
use abm_terreno::tectonica::deriva::Deriva;
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
    // India llega, choca y la convergencia baja a ~1/3 (calibración de c_col).
    let r = india_asia(TectonicaCfg::default().c_col, 1, 24, 30);
    let antes = r
        .iter()
        .take(6)
        .map(|x| x.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let i_choque = r.iter().position(|x| x.1 > 0.0).expect("hay choque");
    let despues: Vec<f64> = r.iter().skip(i_choque + 4).map(|x| x.0).collect();
    let media_despues = despues.iter().sum::<f64>() / despues.len() as f64;
    assert!(
        antes > 8.0,
        "antes del choque la India avanza deprisa ({antes:.1} cm/año)"
    );
    assert!(
        media_despues < 0.5 * antes,
        "el choque frena la convergencia: {antes:.1} → {media_despues:.1} cm/año"
    );
    assert!(
        r.last().is_some_and(|x| x.2.is_finite()),
        "la corteza india no desaparece en 30 Myr"
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
    // espesor máximo vuelve a menos de 38 km en 100–200 Myr (con τ_ero calibrado así: ~148 Myr).
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
#[ignore = "exploratoria: calibración de tau_orog (τ_ero) con la relajación de la adenda de balance"]
fn calibracion_relajacion() {
    let lista: Vec<f64> = std::env::var("TAU_ERO")
        .ok()
        .map(|x| x.split(',').filter_map(|v| v.parse().ok()).collect())
        .unwrap_or(vec![150.0]);
    let t_choque: u64 = std::env::var("T_CHOQUE")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(50);
    for &tau in &lista {
        let cfg = TectonicaCfg {
            tau_orog_myr: tau,
            ..cfg_india_asia()
        };
        let (h0, t) = relajacion(&cfg, 1, 48, t_choque);
        println!(
            "tau_ero = {tau:>5}: espesor máximo al detener {h0:.1} km, baja de 38 km en {t:.0} Myr"
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
#[ignore = "exploratoria: calibración de D_flujo (adenda v3 §6.6b) con el choque India–Asia"]
fn calibracion_flujo_lateral() {
    let lista: Vec<f64> = std::env::var("D_FLUJO")
        .ok()
        .map(|x| x.split(',').filter_map(|v| v.parse().ok()).collect())
        .unwrap_or(vec![0.0, 2000.0]);
    let nivel: u32 = std::env::var("NIVEL")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(2);
    for &dfl in &lista {
        let cfg = TectonicaCfg {
            d_flujo_km2_myr: dfl,
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
        println!("D_flujo = {dfl:>6}: {}", serie.join(" | "));
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
    h_del_arco: f64,
    tau_ero: f64,
    /// Reparto de la erosión potencial `Σ (h_eq − h0)·A` (sin descontar las cuencas cerradas) por
    /// exceso sobre `h0`: 0–2, 2–5, 5–10, 10–20 y más de 20 km, tras el rodaje.
    erosion_por_exceso: [f64; 5],
    /// Consumo (km²) y solape continental (km³) tras el rodaje, por código de borde (0 sin borde,
    /// 1 dorsal, 2 fosa, 3 choque, 4 transformante) y por si la celda está en una unión triple.
    consumo: [[f64; 2]; 5],
    solape: [[f64; 2]; 5],
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
    delam_arcos: f64,
    al_mar: f64,
    v_cont: f64,
    f_cont: f64,
    placas: u32,
    a_sub: f64,
    fosas_km: f64,
    conv_fosas: f64,
}

fn diagnosticar_deriva(semilla: u64, h_del_arco: f64, tau_ero: f64) -> InformeDeriva {
    let cfg =
        abm_terreno::config::TerrenoConfig::load("../../config/terreno.toml").expect("config");
    let mut tc = cfg.tectonica.clone();
    tc.h_del_arco_km = h_del_arco;
    tc.tau_orog_myr = tau_ero;
    let malla = Malla::new(cfg.planet.n0, tc.nivel_malla, cfg.planet.r_km);
    let mut d = Deriva::nueva(&tc, semilla, malla);
    d.traza = Some(Vec::new());
    let pasos = (tc.t_deriva_myr / tc.dt_myr).round() as u64;
    let rodaje = (tc.t_rodaje_myr / tc.dt_myr).round() as u64;
    let mut inf = InformeDeriva {
        semilla,
        h_del_arco,
        tau_ero,
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
            for p in d.placas.iter().filter(|p| p.viva) {
                for (s, c) in p.cort.iter().enumerate() {
                    if !c.cont {
                        continue;
                    }
                    let x = c.h_eq(tc.rho_s, tc.rho_c) - tc.h0_km;
                    if x > 0.0 {
                        let i = [2.0, 5.0, 10.0, 20.0].partition_point(|&l| l <= x);
                        inf.erosion_por_exceso[i] += x * d.malla.area[p.celda[s] as usize];
                    }
                }
            }
        }
        if picos_paso > 0 {
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
    inf.choques = media(|m| m.f_col_au);
    inf.delam = media(|m| m.f_del_au);
    inf.delam_arcos = media(|m| m.f_del_arcos_au);
    inf.al_mar = media(|m| m.f_den_au);
    inf.a_sub = media(|m| m.area_subducida_km2_anio);
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
    inf
}

#[test]
#[ignore = "exploratoria: la deriva del planeta en varias semillas a la vez (SEMILLAS=42,1,2,3,4; H_DEL_ARCO y TAU_ERO, listas): balance, uniones triples, picos de espesor, finales de fosa y coste por etapa (~4 min con 10 derivas en paralelo)"]
fn deriva_en_varias_semillas() {
    let semillas: Vec<u64> = std::env::var("SEMILLAS")
        .unwrap_or_else(|_| "42".into())
        .split(',')
        .map(|x| x.trim().parse().expect("semilla"))
        .collect();
    let umbrales: Vec<f64> = std::env::var("H_DEL_ARCO")
        .unwrap_or_else(|_| "40".into())
        .split(',')
        .map(|x| x.trim().parse().expect("h_del_arco"))
        .collect();
    let taus: Vec<f64> = std::env::var("TAU_ERO")
        .unwrap_or_else(|_| "63".into())
        .split(',')
        .map(|x| x.trim().parse().expect("tau_ero"))
        .collect();
    let trabajos: Vec<(u64, f64, f64)> = semillas
        .iter()
        .flat_map(|&s| {
            let taus = &taus;
            umbrales
                .iter()
                .flat_map(move |&h| taus.iter().map(move |&t| (s, h, t)))
        })
        .collect();
    let informes: Vec<InformeDeriva> = std::thread::scope(|sc| {
        let hilos: Vec<_> = trabajos
            .iter()
            .map(|&(s, h, t)| sc.spawn(move || diagnosticar_deriva(s, h, t)))
            .collect();
        hilos.into_iter().map(|h| h.join().expect("hilo")).collect()
    });
    let nombres = ["sin borde", "dorsal", "fosa", "choque", "transformante"];
    for inf in &informes {
        println!(
            "\n=== semilla {} h_del,arco {} km tau_ero {} Myr ===",
            inf.semilla, inf.h_del_arco, inf.tau_ero
        );
        let te: f64 = inf.erosion_por_exceso.iter().sum();
        println!(
            "erosión potencial por exceso sobre h0 (0–2, 2–5, 5–10, 10–20, >20 km): {:?} %",
            inf.erosion_por_exceso
                .map(|x| (1000.0 * x / te).round() / 10.0)
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
        let nc = |x: &[[f64; 2]; 5], t: usize| x[1][t] + x[4][t];
        let tot = |x: &[[f64; 2]; 5]| x.iter().flatten().sum::<f64>();
        println!(
            "  no convergente: consumo {:.2} % (en uniones triples {:.2}), solape {:.2} % (en uniones triples {:.2}); borde en uniones triples {:.1} % de la longitud",
            100.0 * (nc(&inf.consumo, 0) + nc(&inf.consumo, 1)) / tot(&inf.consumo),
            100.0 * nc(&inf.consumo, 1) / tot(&inf.consumo),
            100.0 * (nc(&inf.solape, 0) + nc(&inf.solape, 1)) / tot(&inf.solape),
            100.0 * nc(&inf.solape, 1) / tot(&inf.solape),
            100.0 * inf.borde_triple_km / inf.borde_total_km
        );
        println!(
            "picos de más de 80 km: {} celdas·paso ({} en uniones triples), en {} pasos tras el rodaje y {} del rodaje; etapa que más sube el máximo en esos pasos: {:?}",
            inf.picos,
            inf.picos_triples,
            inf.pasos_con_picos,
            inf.pasos_con_picos_rodaje,
            inf.etapa_de_los_picos
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
        let to: f64 = inf.oblicuidad_transf.iter().sum();
        println!(
            "consumo en celdas transformantes por oblicuidad (0–5, 5–10, 10–15, 15–20, >20°): {:?} %; a menos de L_borde de una fosa del mismo par: {:.1} %",
            inf.oblicuidad_transf
                .map(|x| (1000.0 * x / to).round() / 10.0),
            100.0 * inf.transf_junto_a_fosa / inf.transf_total.max(1e-12)
        );
        println!(
            "balance (AU): arcos {:.2} choques {:.2} delaminación {:.3} (arcos {:.3}) al mar {:.2}; V_cont {:+.1} %; f_cont {:.3}; placas {}; A_sub {:.2} km²/año; fosas {:.0} km a {:.2} cm/año",
            inf.arcos,
            inf.choques,
            inf.delam,
            inf.delam_arcos,
            inf.al_mar,
            100.0 * inf.v_cont,
            inf.f_cont,
            inf.placas,
            inf.a_sub,
            inf.fosas_km,
            inf.conv_fosas
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
    // Resumen por umbral y τ_ero: media y extremos entre semillas.
    for (h, t) in umbrales
        .iter()
        .flat_map(|&h| taus.iter().map(move |&t| (h, t)))
    {
        let g: Vec<_> = informes
            .iter()
            .filter(|i| i.h_del_arco == h && i.tau_ero == t)
            .collect();
        let r = |f: fn(&InformeDeriva) -> f64| {
            let v: Vec<f64> = g.iter().map(|i| f(i)).collect();
            let m = v.iter().sum::<f64>() / v.len() as f64;
            let (lo, hi) = v
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &x| {
                    (a.min(x), b.max(x))
                });
            format!("{m:.3} [{lo:.3}–{hi:.3}]")
        };
        println!(
            "\n### h_del,arco {h} km, tau_ero {t} Myr, {} semillas: al mar {} AU; delaminación {} (arcos {}); choques {}; arcos {}; V_cont {}; f_cont {}; A_sub {}; fosas {} km; h_max {}",
            g.len(),
            r(|i| i.al_mar),
            r(|i| i.delam),
            r(|i| i.delam_arcos),
            r(|i| i.choques),
            r(|i| i.arcos),
            r(|i| i.v_cont),
            r(|i| i.f_cont),
            r(|i| i.a_sub),
            r(|i| i.fosas_km),
            r(|i| i.h_max)
        );
    }
}
