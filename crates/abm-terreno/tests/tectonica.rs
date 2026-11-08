//! Relieve tectónico (incremento 4): pruebas de la deriva de placas y del relieve que deja.
//!
//! Diseño: `docs/design/terreno/diseno_relieve_tectonico.md` §10 y `incremento-4-diseno.md`.
//! Las pruebas corren en mallas gruesas y derivas cortas para que quepan en `cargo test` (debug).

use abm_terreno::config::TectonicaCfg;
use abm_terreno::geometry::{dot, lat_lon};
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
        lambda_a_rift: 0.0,
        // Escenario del choque: sin invasión por los bordes artificiales (meridianos) de los
        // extremos del Tetis, que cambiaría cómo se mueve la placa de la India. Por lo mismo, sin la
        // rama D de la nota de demografía de placas v2: en las esquinas de esos meridianos el
        // océano exterior tiene a la vez una dorsal con Asia y una fosa bajo la placa de la India, y
        // la regla se las comería paso a paso.
        v_inv_km_myr: 0.0,
        d_dc_km: 0.0,
        n_min: 2,
        recalibrar_myr: 1e9, // c_b se calibra una vez, al principio
        ..TectonicaCfg::default()
    }
}

/// India (continental, 0–15°N, 70–90°E, con su margen estirado) en una placa oceánica que va al
/// norte a ~10 cm/año contra Asia (todo lo que está al norte de 25°N).
fn choque_india_asia(cfg: &TectonicaCfg, nivel: u32, n0: u32) -> Deriva {
    let malla = Malla::new(n0, nivel, 6371.0);
    let dirs = malla.dir.clone();
    let geo = |d: [f64; 3]| {
        let (la, lo) = lat_lon(d);
        (la.to_degrees(), lo.to_degrees())
    };
    let india = |la: f64, lo: f64| (0.0..=15.0).contains(&la) && (70.0..=90.0).contains(&lo);
    // Su margen estirado (adenda v8 §6.10): rampa de 35 km a h_rot en los W_m más cercanos a su
    // borde (los márgenes pasivos de la Gran India).
    let (w_m, h_rot) = (cfg.w_margen_km, cfg.h_rot_km);
    let espesor_india = |la: f64, lo: f64| {
        let km_lat = 6371.0f64.to_radians();
        let km_lon = km_lat * la.to_radians().cos();
        let borde = ((15.0 - la) * km_lat)
            .min(la * km_lat)
            .min((lo - 70.0) * km_lon)
            .min((90.0 - lo) * km_lon);
        if w_m > 0.0 {
            h_rot + (35.0 - h_rot) * (borde / w_m).clamp(0.0, 1.0)
        } else {
            35.0
        }
    };
    let mut d = Deriva::desde(cfg, 7, malla, 0.0, &[1, 2, 3], |k| {
        let (la, lo) = geo(dirs[k]);
        if la > 25.0 {
            (1, corteza(true, 35.0, 0.0))
        } else if (40.0..=140.0).contains(&lo) && la >= -40.0 {
            if india(la, lo) {
                (0, corteza(true, espesor_india(la, lo), 0.0))
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
    india_asia_con(&cfg, nivel, n0, pasos)
}

fn india_asia_con(cfg: &TectonicaCfg, nivel: u32, n0: u32, pasos: u64) -> Vec<(f64, f64, f64)> {
    let mut d = choque_india_asia(cfg, nivel, n0);
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
            !c.cont || c.arco > 0.0,
            "corteza continental que no viene de un arco en un planeta sin cratones"
        );
        if c.h == cfg.h_oc_km && c.hc == 0.0 && c.sed == 0.0 {
            // La curva de la edad, más el abombamiento de los puntos calientes (nota de puntos
            // calientes §6.4), que sube el fondo bajo el agua.
            let esperada =
                -abm_terreno::tectonica::isostasia::profundidad_por_edad(&cfg, c.edad_oc)
                    + d.abombamiento_m[k];
            assert!(
                (zk - esperada).abs() < 1e-9,
                "profundidad por edad (Parsons y Sclater) y abombamiento"
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
    // Tras romperse la losa, la tolerancia es de 0,55 veces la velocidad de antes (decisión del
    // responsable, 2026-11-07; antes, 0,5). La India del modelo se acelera tras la rotura por el giro
    // que le dan las fosas de los lados (§40), mientras que la real siguió frenando (Molnar y Stock
    // 2009): queda como causa pendiente, no se toca `c_col`.
    assert!(
        durante < 0.5 * antes && despues < 0.55 * antes,
        "el choque frena la convergencia: {antes:.1} → {durante:.1} con la losa colgando y {despues:.1} tras romperse (cm/año)"
    );
    assert!(
        r.last().is_some_and(|x| x.2.is_finite()),
        "la corteza india no desaparece en 50 Myr"
    );
}

#[test]
fn un_continente_se_parte_y_abre_un_oceano_simetrico() {
    // Un continente de 30° en una placa; λ_A enorme fuerza el rift en el primer paso.
    let cfg = TectonicaCfg {
        lambda_a_rift: 1e3,
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
    d.cfg.lambda_a_rift = 0.0;
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
    // Márgenes estirados en rampa a los dos lados (adenda v8 §6.10).
    let adelgazada = (0..d.malla.num)
        .filter(|&k| d.corteza(k).is_some_and(|c| c.cont && c.h < 0.8 * 35.0))
        .count();
    assert!(adelgazada > 0, "quedan márgenes continentales adelgazados");
}

#[test]
fn un_rift_estira_sus_bordes_en_rampa_conservando_el_volumen_y_despues_abre_el_oceano() {
    // Adenda v8 §6.10: un continente de 35 km (casquete de 30°) se parte en el primer paso
    // (λ_A enorme); después las hijas se separan a v_rift sin fuerzas. Mientras la abertura no
    // llega a E_rot = W_m·(h_0 − h_rot)/h_0 (~107 km), se rellena estirando la corteza de los
    // bordes y el volumen continental se conserva exactamente. Al romperse, cada borde es una
    // rampa de h_rot junto al eje a h_0 a W_m, y lo que se abre después es fondo oceánico.
    let cfg = TectonicaCfg {
        lambda_a_rift: 1e3,
        n_min: 2,
        lambda_esp: 0.0,
        ..TectonicaCfg::default()
    };
    let malla = Malla::new(48, 2, 6371.0);
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
    let rift = d
        .eventos
        .iter()
        .find(|e| e.tipo == "rift")
        .cloned()
        .expect("el continente se parte en el primer paso");
    let (h1, h2) = (rift.placas[1], rift.placas[2]);
    d.cfg.lambda_a_rift = 0.0;
    let v0 = d.inventario_continental();
    let joven = |d: &Deriva| {
        (0..d.malla.num)
            .filter(|&k| d.corteza(k).is_some_and(|c| !c.cont && c.edad_oc < 20.0))
            .count()
    };
    // Fase de rift: 20 km/Myr de abertura, unos 5 Myr hasta romperse.
    for _ in 0..3 {
        d.paso_cinematico();
        let dv = (d.inventario_continental() - v0).abs() / v0;
        assert!(dv < 1e-9, "el estiramiento conserva el volumen: {dv:.2e}");
    }
    assert_eq!(joven(&d), 0, "antes de romperse no nace fondo oceánico");
    for _ in 0..9 {
        d.paso_cinematico();
    }
    let dv = (d.inventario_continental() - v0).abs() / v0;
    assert!(dv < 1e-9, "el volumen continental se conserva: {dv:.2e}");
    assert!(joven(&d) > 0, "tras la rotura se abre el océano");
    // Rampa: espesor medio de la corteza continental de las hijas por su distancia al océano nuevo.
    let idx = |id: u64| d.placas.iter().position(|p| p.id == id && p.viva);
    let (Some(i1), Some(i2)) = (idx(h1), idx(h2)) else {
        panic!("las dos hijas siguen vivas");
    };
    let mut franjas = [(0.0f64, 0.0f64); 4]; // <50, 50–100, 100–150, 200–300 km del océano nuevo
    for k in 0..d.malla.num {
        let p = d.dueno[k] as usize;
        let Some(c) = d.corteza(k) else { continue };
        if (p != i1 && p != i2) || !c.cont {
            continue;
        }
        let dist = (0..d.malla.num)
            .filter(|&j| d.corteza(j).is_some_and(|cj| !cj.cont && cj.edad_oc < 20.0))
            .map(|j| dot(d.malla.dir[k], d.malla.dir[j]).clamp(-1.0, 1.0).acos() * 6371.0)
            .fold(f64::INFINITY, f64::min);
        let i = match dist {
            x if x < 50.0 => 0,
            x if x < 100.0 => 1,
            x if x < 150.0 => 2,
            x if (200.0..300.0).contains(&x) => 3,
            _ => continue,
        };
        franjas[i].0 += c.h;
        franjas[i].1 += 1.0;
    }
    let h: Vec<f64> = franjas.iter().map(|x| x.0 / x.1.max(1.0)).collect();
    assert!(
        h[0] < 20.0 && h[0] < h[1] && h[1] < h[2] && h[2] <= h[3] + 1.0 && h[3] > 33.0,
        "rampa de h_rot junto al eje a h_0 lejos: {h:?} km a <50 / 50–100 / 100–150 / 200–300 km"
    );
    // Nota de isostasia con litosfera §6.2 y §10, prueba 2: al romper, la franja estirada de cada
    // lado lleva su margen y su edad tectonotérmica vuelve a 0; desde ahí envejece con la deriva.
    let rotura = d
        .eventos
        .iter()
        .filter(|e| e.tipo == "rotura_rift")
        .map(|e| e.myr)
        .fold(f64::NEG_INFINITY, f64::max);
    let margen: Vec<_> = [i1, i2]
        .iter()
        .flat_map(|&i| d.placas[i].cort.iter())
        .filter(|c| c.cont && c.rift_id != 0)
        .collect();
    assert!(!margen.is_empty(), "las franjas estiradas llevan su margen");
    let lados: std::collections::BTreeSet<u32> = margen.iter().map(|c| c.rift_id & 1).collect();
    assert_eq!(lados.len(), 2, "un margen a cada lado del rift");
    // El evento lleva la hora del principio del paso en que se rompe; la edad cuenta desde el final
    // de ese paso (el envejecimiento va al principio de cada paso).
    let desde_rotura = d.t_myr - (rotura + d.cfg.dt_myr);
    let jovenes = margen
        .iter()
        .filter(|c| (c.edad_tt - desde_rotura).abs() < 1.0)
        .count();
    assert!(
        jovenes * 10 >= margen.len() * 9,
        "la edad de los márgenes cuenta desde la rotura: {jovenes} de {} ({desde_rotura} Myr)",
        margen.len()
    );
}

#[test]
fn las_edades_tectonotermicas_de_partida_son_viejas_por_dentro_y_deterministas() {
    // Nota de isostasia con litosfera §6.5 y §10, prueba 4: misma semilla, mismas edades; las más
    // viejas, en el interior de cada continente; su distribución, la de la config.
    let cfg = cfg_rapida();
    let a = Deriva::nueva(&cfg, 21, Malla::new(24, 1, 6371.0));
    let b = Deriva::nueva(&cfg, 21, Malla::new(24, 1, 6371.0));
    let edades = |d: &Deriva| -> Vec<f64> {
        d.placas
            .iter()
            .flat_map(|p| p.cort.iter().map(|c| c.edad_tt))
            .collect()
    };
    assert_eq!(edades(&a), edades(&b), "misma semilla, mismas edades");
    // Interior (sin vecinas oceánicas a dos anillos) frente a costa.
    let (mut interior, mut costa) = ((0.0, 0.0), (0.0, 0.0));
    let mut a_cont = 0.0;
    let mut a_viejo = 0.0;
    for k in 0..a.malla.num {
        let Some(c) = a.corteza(k) else { continue };
        if !c.cont {
            continue;
        }
        let w = a.malla.area[k];
        a_cont += w;
        if c.edad_tt > 2000.0 {
            a_viejo += w;
        }
        let cerca_del_mar = a.malla.vec[k].iter().any(|&v| {
            v != NADA
                && (a.corteza(v as usize).is_some_and(|cv| !cv.cont)
                    || a.malla.vec[v as usize]
                        .iter()
                        .any(|&u| u != NADA && a.corteza(u as usize).is_some_and(|cu| !cu.cont)))
        });
        let x = if cerca_del_mar {
            &mut costa
        } else {
            &mut interior
        };
        x.0 += c.edad_tt * w;
        x.1 += w;
    }
    let (ti, tc) = (interior.0 / interior.1, costa.0 / costa.1);
    assert!(
        ti > tc + 300.0,
        "interior {ti:.0} Myr frente a costa {tc:.0}"
    );
    // La fracción de más de 2 000 Myr, la de la distribución (las clases de 2 000 en adelante).
    let esperada: f64 = cfg
        .edad_tt_ini_bordes_myr
        .windows(2)
        .zip(&cfg.edad_tt_ini_frac)
        .filter(|(w, _)| w[0] >= 2000.0)
        .map(|(_, f)| f)
        .sum::<f64>()
        / cfg.edad_tt_ini_frac.iter().sum::<f64>();
    assert!(
        (a_viejo / a_cont - esperada).abs() < 0.03,
        "{:.3} del continente de más de 2 000 Myr (distribución: {esperada:.3})",
        a_viejo / a_cont
    );
}

// --- Rifts (nota_rifts.md §10) -------------------------------------------------------------------

/// Configuración de las pruebas de rifts: la regla por bloques con `λ_A` enorme (cada bloque que
/// puede romperse se rompe en el paso) y sin erosión ni nacimientos espontáneos que la enmascaren.
fn cfg_rifts() -> TectonicaCfg {
    TectonicaCfg {
        lambda_a_rift: 1e3,
        n_min: 2,
        lambda_esp: 0.0,
        e1_km_myr: 1e-12,
        ..TectonicaCfg::default()
    }
}

#[test]
fn un_bloque_grande_se_rompe_por_su_sutura_y_conserva_la_corteza() {
    // Nota de rifts §10, prueba 1: un continente de 35 km (casquete de 30°) con una sutura curva y
    // descentrada (de una placa fundida antes), de |z| < 0,3 rad, a ~640 km del centro. Con λ_A alto
    // se rompe en el primer paso por la sutura, no por un círculo por su centro, y la corteza
    // continental se conserva al partirlo y al estirar los bordes.
    let cfg = cfg_rifts();
    let malla = Malla::new(48, 2, 6371.0);
    let dirs = malla.dir.clone();
    let curva = |z: f64| 0.1 + 0.05 * (8.0 * z).sin();
    let mut d = Deriva::desde(&cfg, 3, malla, 0.0, &[10, 20], |k| {
        let x = dirs[k];
        if x[0] > 0.0 {
            let cont = x[0] > 30f64.to_radians().cos();
            let mut c = corteza(cont, if cont { 35.0 } else { 7.0 }, 60.0);
            let dy = x[1] - curva(x[2]);
            if cont && x[2].abs() < 0.3 && dy.abs() < 0.012 {
                c.sutura = 2 + u32::from(dy > 0.0);
            }
            (0, c)
        } else {
            (1, corteza(false, 7.0, 60.0))
        }
    });
    d.arrancar_con(&[[0.0; 3], [0.0; 3]]);
    let v0 = d.inventario_continental();
    d.dar_paso();
    let rift = d
        .eventos
        .iter()
        .find(|e| e.tipo == "rift")
        .cloned()
        .expect("el bloque se rompe en el primer paso");
    assert_eq!(rift.clase.as_deref(), Some("sutura"), "sigue la sutura");
    let largo = rift.sutura_km.expect("longitud de la sutura");
    assert!(
        largo > 1500.0,
        "la sutura mide ~3 800 km dentro del bloque: {largo:.0} km"
    );
    let trozos = rift.trozos_km2.expect("áreas de los trozos");
    assert!(
        trozos[0] < trozos[1],
        "el corte descentrado deja un trozo menor del lado 1: {trozos:?} km²"
    );
    // Cada lado de la sutura, en su hija: la primera hija es el lado 1.
    let idx = |id: u64| d.placas.iter().position(|p| p.id == id && p.viva);
    let (Some(i1), Some(i2)) = (idx(rift.placas[1]), idx(rift.placas[2])) else {
        panic!("las dos hijas siguen vivas");
    };
    let (mut bien, mut mal) = (0, 0);
    for k in 0..d.malla.num {
        let x = d.malla.dir[k];
        if !d.corteza(k).is_some_and(|c| c.cont) || x[2].abs() >= 0.25 {
            continue;
        }
        let dy = x[1] - curva(x[2]);
        let esperada = if dy > 0.03 {
            i1
        } else if dy < -0.03 {
            i2
        } else {
            continue;
        };
        if d.dueno[k] as usize == esperada {
            bien += 1;
        } else {
            mal += 1;
        }
    }
    assert!(
        bien > 0 && mal * 100 < bien,
        "el corte sigue la sutura: {mal} celdas del lado equivocado de {}",
        bien + mal
    );
    assert!(
        d.placas
            .iter()
            .filter(|p| p.viva)
            .all(|p| p.cort.iter().all(|c| c.sutura == 0)),
        "una sutura reabierta deja de serlo"
    );
    let dv = (d.inventario_continental() - v0).abs() / v0;
    assert!(dv < 1e-9, "partir el bloque conserva la corteza: {dv:.2e}");
    d.cfg.lambda_a_rift = 0.0;
    for _ in 0..3 {
        d.paso_cinematico();
    }
    let dv = (d.inventario_continental() - v0).abs() / v0;
    assert!(
        dv < 1e-9,
        "estirar los bordes conserva la corteza: {dv:.2e}"
    );
}

#[test]
fn un_bloque_pequeno_no_se_rompe_nunca() {
    // Nota de rifts §10, prueba 2: con λ_A enorme, un continente de menos de 2·A_trozo,min
    // (casquete de 3,5°, ~0,48 Mkm²) no se rompe en 20 Myr; uno de ~1,4 Mkm² (6°) sí.
    let rompe = |radio_grados: f64| {
        let cfg = cfg_rifts();
        let malla = Malla::new(48, 1, 6371.0);
        let dirs = malla.dir.clone();
        let mut d = Deriva::desde(&cfg, 4, malla, 0.0, &[10, 20], |k| {
            let x = dirs[k];
            if x[0] > 0.0 {
                let cont = x[0] > radio_grados.to_radians().cos();
                (0, corteza(cont, if cont { 35.0 } else { 7.0 }, 60.0))
            } else {
                (1, corteza(false, 7.0, 60.0))
            }
        });
        d.arrancar_con(&[[0.0; 3], [0.0; 3]]);
        for _ in 0..20 {
            d.dar_paso();
        }
        d.eventos.iter().filter(|e| e.tipo == "rift").count()
    };
    assert_eq!(rompe(3.5), 0, "un bloque de menos de 1 Mkm² no se rompe");
    assert!(rompe(6.0) > 0, "uno de 1,4 Mkm² sí se rompe");
}

/// Continente de 35 km en un casquete de 30° (nota de rifts v2): sus muestras llevan el reloj de los
/// rifts reiniciado en `reinicio` (Myr).
fn continente_con_reloj(cfg: &TectonicaCfg, reinicio: f64) -> Deriva {
    let malla = Malla::new(48, 1, 6371.0);
    let dirs = malla.dir.clone();
    let mut d = Deriva::desde(cfg, 3, malla, 0.0, &[10, 20], |k| {
        let x = dirs[k];
        if x[0] > 0.0 {
            let cont = x[0] > 30f64.to_radians().cos();
            let mut c = corteza(cont, if cont { 35.0 } else { 7.0 }, 60.0);
            if cont {
                c.reinicio_myr = reinicio;
            }
            (0, c)
        } else {
            (1, corteza(false, 7.0, 60.0))
        }
    });
    d.arrancar_con(&[[0.0; 3], [0.0; 3]]);
    d
}

#[test]
fn un_bloque_recien_unido_no_se_rompe_y_uno_viejo_si() {
    // Nota de rifts v2 §6.1: la probabilidad crece con el tiempo que el bloque lleva unido,
    // min(1, t_k/τ_ais). Con λ_A alto, un bloque cuyo reloj acaba de reiniciarse y τ_ais enorme no se
    // rompe en 10 Myr; el mismo bloque con un reloj de más de τ_ais se rompe en el primer paso, y sus
    // dos trozos empiezan el reloj en el Myr de la ruptura.
    let cfg = TectonicaCfg {
        lambda_a_rift: 1.0,
        tau_ais_myr: 1e9,
        ..cfg_rifts()
    };
    let mut d = continente_con_reloj(&cfg, 0.0);
    for _ in 0..10 {
        d.dar_paso();
    }
    assert_eq!(
        d.eventos.iter().filter(|e| e.tipo == "rift").count(),
        0,
        "recién unido, no se rompe"
    );
    let cfg = TectonicaCfg {
        lambda_a_rift: 1e3,
        tau_ais_myr: 100.0,
        ..cfg_rifts()
    };
    let mut d = continente_con_reloj(&cfg, -500.0);
    d.dar_paso();
    let rift = d
        .eventos
        .iter()
        .find(|e| e.tipo == "rift")
        .cloned()
        .expect("unido hace 500 Myr, se rompe en el primer paso");
    for id in &rift.placas[1..] {
        let p = d
            .placas
            .iter()
            .find(|p| p.viva && p.id == *id)
            .expect("hija viva");
        let reinicios: Vec<f64> = p
            .cort
            .iter()
            .filter(|c| c.continente())
            .map(|c| c.reinicio_myr)
            .collect();
        assert!(!reinicios.is_empty());
        let al_romperse = reinicios.iter().filter(|&&t| t == rift.myr).count();
        assert!(
            al_romperse * 10 >= reinicios.len() * 9,
            "los trozos empiezan su reloj al romperse: {al_romperse} de {}",
            reinicios.len()
        );
    }
}

#[test]
fn la_misma_semilla_da_las_mismas_rupturas() {
    // Nota de rifts §10, prueba 3: una tirada por bloque y paso, en orden fijo.
    let cfg = TectonicaCfg {
        lambda_a_rift: 2e-3,
        ..cfg_rapida()
    };
    let rupturas = |semilla: u64| {
        let mut d = Deriva::nueva(&cfg, semilla, Malla::new(12, 1, 6371.0));
        for _ in 0..40 {
            d.dar_paso();
        }
        d.eventos
            .iter()
            .filter(|e| e.tipo == "rift" || e.tipo == "rotura_rift")
            .cloned()
            .collect::<Vec<_>>()
    };
    let (a, b) = (rupturas(42), rupturas(42));
    assert!(
        a.iter().filter(|e| e.tipo == "rift").count() > 0,
        "con λ_A alto hay rupturas"
    );
    assert_eq!(a, b, "misma semilla, mismas rupturas");
    assert_ne!(a, rupturas(43), "otra semilla, otras rupturas");
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
        lambda_a_rift: 1e-3, // fuerza rifts, suturas y absorciones en una deriva corta
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
    // adenda v5; recalibrados con la isostasia con litosfera, incremento-4-diseno.md §73).
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
        lambda_a_rift: 0.0,
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
    /// Adenda v7: reparto de la corteza solapada en los choques (sin la de arco) y de la que se
    /// pierde por la muestra que entra (continental < h_1, h_1–h_2, > h_2, sobre fondo oceánico),
    /// en fracción de toda la deriva.
    por_grosor: [[f64; 4]; 2],
    /// Diagnóstico de procedencia: de la corteza perdida en los choques de cada clase, fracción
    /// por la vía que la puso allí (`via::NOMBRES`).
    por_via: [[f64; abm_terreno::tectonica::deriva::via::N]; 4],
    /// Adenda v8: espesor medio de la corteza continental de los márgenes pasivos al final por su
    /// distancia a la costa (0–50, 50–100, 100–150, 150–200, 200–300 km) y anchura de la franja
    /// estirada: la distancia a la que llega al 90 % del espesor de 200–300 km (km).
    margen_perfil: [f64; 5],
    franja_estirada_km: f64,
    /// Lo mismo solo para los márgenes nacidos de un rift (su costa, corteza estirada en la abertura)
    /// y la fracción de la costa de los márgenes pasivos que son de rift.
    margen_perfil_rift: [f64; 5],
    franja_rift_km: f64,
    costa_rift_frac: f64,
    /// Nota de rifts (tras el rodaje): rupturas, las que siguen una sutura, roturas (nace el océano
    /// en un lado) y suturas hechas, por 100 Myr; bloques continentales y fracción del mayor (media);
    /// índice de supercontinente (mínimo, media y máximo) y supercontinentes formados; espesor en la
    /// costa de los márgenes y anchura de su franja estirada al final, del modelo (km).
    rifts_100: f64,
    rifts_sutura_100: f64,
    roturas_rift_100: f64,
    suturas_100: f64,
    bloques: f64,
    bloque_mayor: f64,
    supercontinente: [f64; 3],
    super_formados: f64,
    costa_final: f64,
    franja_final: f64,
    /// Coste del flujo lateral: subpasos por paso, muestras de la red por subpaso y fracción de los
    /// subpasos por clase del espesor máximo (50–60, …, >100 km).
    flujo_subpasos: f64,
    flujo_muestras: f64,
    flujo_por_hmax: [f64; 6],
    /// Veces que se rehace la red por paso, lados por subpaso y segundos en rehacerla y en los
    /// subpasos.
    flujo_rehechos: f64,
    flujo_lados: f64,
    flujo_seg: [f64; 2],
    seg_partes: [f64; 8],
    /// Adenda v9: corteza de arco que los choques dejan arriba sobre un continente y fuera de él
    /// (AU).
    arco_choque: [f64; 2],
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
    // La vida de los márgenes como entidades que nacen en cada rift (por celdas de costa, el vaivén
    // de la malla la falsea).
    let vidas: Vec<f64> = d
        .margenes_rift
        .iter()
        .filter_map(|m| m.fin_myr.map(|f| f - m.rotura_myr))
        .collect();
    inf.vida_margen = if vidas.is_empty() {
        f64::NAN
    } else {
        vidas.iter().sum::<f64>() / vidas.len() as f64
    };
    inf.edad_margen_final = ultima.edad_media_margen_myr;
    inf.edad_max_margen_final = ultima.edad_max_margen_myr;
    inf.invadido = [media(|m| m.invadido_km[0]), media(|m| m.invadido_km[1])];
    inf.roturas_100 = val.iter().map(|m| m.roturas as f64).sum::<f64>() * 100.0 / n;
    // Perfil de los márgenes pasivos: desde las celdas continentales con fondo oceánico de su
    // placa al lado (y sin otra placa a menos de tres anillos), hacia dentro por su continente.
    {
        let num = d.malla.num;
        let otra_cerca = |k: usize| {
            let p = d.dueno[k];
            let mut anillo = vec![k as u32];
            for _ in 0..3 {
                let mut sig = Vec::new();
                for &c in &anillo {
                    for &v in &d.malla.vec[c as usize] {
                        if v == NADA {
                            continue;
                        }
                        if d.dueno[v as usize] != p {
                            return true;
                        }
                        sig.push(v);
                    }
                }
                sig.sort_unstable();
                sig.dedup();
                anillo = sig;
            }
            false
        };
        let cont = |k: usize| d.corteza(k).is_some_and(|c| c.continente());
        // Perfil desde las celdas de costa que cumplen `vale`: espesor medio por distancia a la
        // costa, franja estirada y número de celdas de costa.
        let perfil = |vale: &dyn Fn(usize) -> bool| -> ([f64; 5], f64, usize) {
            let mut dist = vec![f64::INFINITY; num];
            let mut cola = std::collections::BinaryHeap::new();
            let mut n_costa = 0;
            for (k, dk) in dist.iter_mut().enumerate() {
                if !cont(k) {
                    continue;
                }
                let costa = d.malla.vec[k].iter().any(|&v| {
                    v != NADA
                        && d.dueno[v as usize] == d.dueno[k]
                        && d.corteza(v as usize).is_some_and(|c| !c.cont)
                });
                if costa && !otra_cerca(k) && vale(k) {
                    *dk = 0.5 * d.malla.lado_km;
                    cola.push(std::cmp::Reverse(((*dk * 1000.0) as u64, k)));
                    n_costa += 1;
                }
            }
            while let Some(std::cmp::Reverse((_, k))) = cola.pop() {
                for (j, &v) in d.malla.vec[k].iter().enumerate() {
                    if v == NADA || d.dueno[v as usize] != d.dueno[k] || !cont(v as usize) {
                        continue;
                    }
                    let nd = dist[k] + d.malla.dvec[k][j];
                    if nd < dist[v as usize] && nd < 300.0 {
                        dist[v as usize] = nd;
                        cola.push(std::cmp::Reverse(((nd * 1000.0) as u64, v as usize)));
                    }
                }
            }
            let limites = [50.0, 100.0, 150.0, 200.0, 300.0];
            let mut suma = [(0.0f64, 0.0f64); 5];
            for (k, &dk) in dist.iter().enumerate() {
                if !dk.is_finite() {
                    continue;
                }
                let i = limites.iter().position(|&x| dk < x).unwrap_or(4);
                if let Some(c) = d.corteza(k) {
                    suma[i].0 += c.h * d.malla.area[k];
                    suma[i].1 += d.malla.area[k];
                }
            }
            let mut pf = [f64::NAN; 5];
            for (i, x) in suma.iter().enumerate() {
                pf[i] = if x.1 > 0.0 { x.0 / x.1 } else { f64::NAN };
            }
            let centros = [25.0, 75.0, 125.0, 175.0, 250.0];
            let franja = centros
                .iter()
                .zip(pf)
                .find(|&(_, h)| h >= 0.9 * pf[4])
                .map_or(f64::NAN, |(&c, _)| c);
            (pf, franja, n_costa)
        };
        let (pf, franja, n_todas) = perfil(&|_| true);
        inf.margen_perfil = pf;
        inf.franja_estirada_km = franja;
        // Solo los márgenes nacidos de un rift: su costa es corteza estirada en la abertura.
        let (pf, franja, n_rift) = perfil(&|k| {
            d.corteza(k)
                .is_some_and(|c| c.proc.nace == abm_terreno::tectonica::deriva::nace::RIFT)
        });
        inf.margen_perfil_rift = pf;
        inf.franja_rift_km = franja;
        inf.costa_rift_frac = n_rift as f64 / (n_todas as f64).max(1.0);
    }
    for (fila, x) in inf.por_via.iter_mut().zip(d.diag_choque_km3[1]) {
        let t: f64 = x.iter().sum();
        for (f, v) in fila.iter_mut().zip(x) {
            *f = if t > 0.0 { v / t } else { 0.0 };
        }
    }
    for (fila, x) in inf.por_grosor.iter_mut().zip([
        d.cuentas.choque_solapado_por_grosor_km3,
        d.cuentas.choque_perdida_por_grosor_km3,
    ]) {
        let t: f64 = x.iter().sum();
        for (f, v) in fila.iter_mut().zip(x) {
            *f = if t > 0.0 { v / t } else { 0.0 };
        }
    }
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
    // Nota de rifts: los eventos de los pasos tras el rodaje (un evento lleva el Myr del principio
    // de su paso).
    let t_ini = d
        .metricas
        .iter()
        .rev()
        .find(|m| m.rodaje)
        .map_or(f64::NEG_INFINITY, |m| m.myr);
    let cuenta = |f: &dyn Fn(&abm_terreno::tectonica::historia::EventoTectonico) -> bool| {
        d.eventos.iter().filter(|e| e.myr >= t_ini && f(e)).count() as f64 * 100.0 / n
    };
    inf.rifts_100 = cuenta(&|e| e.tipo == "rift");
    inf.rifts_sutura_100 = cuenta(&|e| e.tipo == "rift" && e.clase.as_deref() == Some("sutura"));
    inf.roturas_rift_100 = cuenta(&|e| e.tipo == "rotura_rift");
    inf.suturas_100 = cuenta(&|e| e.tipo == "sutura");
    // Ciclos con histéresis: el índice pasa de menos de 0,5 a 0,7 o más.
    let mut abajo = false;
    for m in &val {
        if m.supercontinente < 0.5 {
            abajo = true;
        } else if abajo && m.supercontinente >= 0.7 {
            abajo = false;
            inf.super_formados += 1.0;
        }
    }
    inf.bloques = media(|m| m.bloques_continentales as f64);
    inf.bloque_mayor = media(|m| m.bloque_mayor_frac);
    inf.supercontinente = [
        val.iter()
            .map(|m| m.supercontinente)
            .fold(f64::INFINITY, f64::min),
        media(|m| m.supercontinente),
        val.iter().map(|m| m.supercontinente).fold(0.0, f64::max),
    ];
    if let Some(u) = val.last() {
        inf.costa_final = u.costa_margen_km;
        inf.franja_final = u.franja_estirada_km;
    }
    inf.flujo_subpasos = d.flujo_subpasos as f64 / pasos as f64;
    inf.flujo_muestras = d.flujo_muestras as f64 / (d.flujo_subpasos as f64).max(1.0);
    for (f, &x) in inf
        .flujo_por_hmax
        .iter_mut()
        .zip(&d.flujo_subpasos_por_hmax)
    {
        *f = x as f64 / (d.flujo_subpasos as f64).max(1.0);
    }
    inf.flujo_rehechos = d.flujo_rehechos as f64 / pasos as f64;
    inf.flujo_lados = d.flujo_lados as f64 / (d.flujo_subpasos as f64).max(1.0);
    inf.flujo_seg = d.flujo_seg;
    inf.seg_partes = d.seg_partes;
    for (f, &x) in inf.arco_choque.iter_mut().zip(&d.diag_arco_choque_km3) {
        *f = x / anios;
    }
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
            "### márgenes pasivos al final: espesor de su corteza a 0–50 / 50–100 / 100–150 / 150–200 / 200–300 km de la costa {} / {} / {} / {} / {} km; franja estirada (hasta el 90 % del espesor de dentro) {} km (Tierra: 100–300 por lado)",
            r(|i| i.margen_perfil[0]),
            r(|i| i.margen_perfil[1]),
            r(|i| i.margen_perfil[2]),
            r(|i| i.margen_perfil[3]),
            r(|i| i.margen_perfil[4]),
            r(|i| i.franja_estirada_km)
        );
        {
            use abm_terreno::tectonica::deriva::via;
            let clases = [
                "continental <h_1",
                "continental h_1–h_2",
                "continental >h_2",
                "sobre fondo oceánico",
            ];
            for (c, nombre) in clases.iter().enumerate() {
                let partes: Vec<String> = (0..via::N)
                    .filter(|&i| g.iter().any(|x| x.por_via[c][i] > 0.005))
                    .map(|i| {
                        let v: Vec<f64> = g.iter().map(|x| x.por_via[c][i]).collect();
                        let m = v.iter().sum::<f64>() / v.len() as f64;
                        let (lo, hi) =
                            v.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, &x| {
                                (a.0.min(x), a.1.max(x))
                            });
                        format!("{} {m:.2} [{lo:.2}–{hi:.2}]", via::NOMBRES[i])
                    })
                    .collect();
                if !partes.is_empty() {
                    println!(
                        "### pérdida en choques, {nombre}, por la vía que la puso allí: {}",
                        partes.join("; ")
                    );
                }
            }
        }
        println!(
            "### sedimento: máximo de una muestra {} km; al mar siguiente {} AU; en cuencas cerradas {} AU",
            r(|i| i.sed_max),
            r(|i| i.desbordado),
            r(|i| i.en_cuencas)
        );
        println!(
            "### rifts (nota de rifts): rupturas {} por 100 Myr, por sutura {}; roturas {}; suturas {}; bloques continentales {} (el mayor, {} del continente); supercontinente mín {} media {} máx {}, ciclos (de <0,5 a ≥0,7) {}; márgenes al final: costa {} km, franja estirada {} km",
            r(|i| i.rifts_100),
            r(|i| i.rifts_sutura_100),
            r(|i| i.roturas_rift_100),
            r(|i| i.suturas_100),
            r(|i| i.bloques),
            r(|i| i.bloque_mayor),
            r(|i| i.supercontinente[0]),
            r(|i| i.supercontinente[1]),
            r(|i| i.supercontinente[2]),
            r(|i| i.super_formados),
            r(|i| i.costa_final),
            r(|i| i.franja_final)
        );
        println!(
            "### márgenes nacidos de un rift ({} de la costa de los márgenes pasivos): espesor a 0–50 / 50–100 / 100–150 / 150–200 / 200–300 km {} / {} / {} / {} / {} km; franja estirada {} km",
            r(|i| i.costa_rift_frac),
            r(|i| i.margen_perfil_rift[0]),
            r(|i| i.margen_perfil_rift[1]),
            r(|i| i.margen_perfil_rift[2]),
            r(|i| i.margen_perfil_rift[3]),
            r(|i| i.margen_perfil_rift[4]),
            r(|i| i.franja_rift_km)
        );
        println!(
            "### adenda v9: arco que dejan los choques sobre un continente {} AU y fuera de él {} AU",
            r(|i| i.arco_choque[0]),
            r(|i| i.arco_choque[1])
        );
        println!(
            "### flujo lateral: {} subpasos por paso con {} muestras y {} lados en la red; la red se rehace {} veces por paso; segundos en rehacerla {} y en los subpasos {}; subpasos por espesor máximo (50–60, 60–70, 70–80, 80–90, 90–100, >100 km): {:?}",
            r(|i| i.flujo_subpasos),
            r(|i| i.flujo_muestras),
            r(|i| i.flujo_lados),
            r(|i| i.flujo_rehechos),
            r(|i| i.flujo_seg[0]),
            r(|i| i.flujo_seg[1]),
            (0..6)
                .map(
                    |j| (1000.0 * g.iter().map(|i| i.flujo_por_hmax[j]).sum::<f64>()
                        / g.len() as f64)
                        .round()
                        / 10.0
                )
                .collect::<Vec<_>>()
        );
        println!(
            "### coste (s): erosión {}, nivel del mar {}, sedimento al mar {}; métricas: márgenes pasivos {}, perfil de márgenes {}, bloques {}, índice de supercontinente {}",
            r(|i| i.seg_partes[0]),
            r(|i| i.seg_partes[1]),
            r(|i| i.seg_partes[2]),
            r(|i| i.seg_partes[3]),
            r(|i| i.seg_partes[4]),
            r(|i| i.seg_partes[5]),
            r(|i| i.seg_partes[6])
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
        lambda_a_rift: 0.0,
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
        lambda_a_rift: 0.0,
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
    // del choque, y después se rompe y deja de tirar. Sin la succión de las losas: aquí se mide la
    // rotura de la losa; con la succión (ε_suc 0,05), la India llega por otro camino y el tramo que
    // se rompe mezcla fondo de otra edad (rompe a los 25 Myr). Que la India no se acelere tras la
    // rotura con la succión se comprueba en `dos_continentes_chocan_frenan_y_levantan_una_cordillera`.
    let cfg = TectonicaCfg {
        eps_suc: 0.0,
        ..cfg_india_asia()
    };
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
    // El frente visible (tres bordes de choque) se adelanta al inicio del choque del modelo (el
    // primer solape continental): hasta 3 Myr con E_1 = 0,0075 y 4 Myr con E_1 = 0,02, la erosión
    // recalibrada con la isostasia con litosfera (incremento-4-diseno.md §73). La rotura respecto
    // al inicio del modelo no cambia (la aserción de arriba).
    assert!(
        ((rotura - inicio) as f64 - t_rot).abs() <= 4.0,
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
                "al final h {:.0} hc {:.0} sed {:.1} arco {:.2} edad_oc {:.0}; placa id {} con {} muestras ({} continentales, fosa de arriba {:.0} km); la ven {ven} celdas (centro en {}); borde más cercano {dmin:.0} km ({tipo}); vecinas [{}]",
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

/// Procedencia del fondo oceánico (nota de fosas v6, diagnóstico sin cambiar nada; SEMILLAS y
/// VARIANTE como en `diagnostico_fondo_viejo`). Cada 100 Myr y al final:
/// - (a) los escalones de edad enormes entre celdas vecinas de lado de una misma placa (fondo de
///   más de `a_min` junto a fondo de menos de la mitad de su edad, los candidatos de la v5),
///   repartidos por su causa: relleno de huecos (creación residual), costura de un lote (sutura,
///   absorción, salto de dorsal o rift: los dos lados no llegaron juntos a la placa), fondo del
///   estado inicial junto a fondo nuevo, o dorsal;
/// - (b) qué fracción del fondo de más de 180 Myr y de los márgenes pasivos viejos (de más de
///   180 Myr) viene del estado inicial del planeta.
#[test]
#[ignore = "diagnóstico: procedencia del fondo oceánico y de sus escalones de edad (~4 min)"]
fn diagnostico_procedencia_del_fondo() {
    use abm_terreno::tectonica::deriva::nace;
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
                        // (a) Escalones enormes por causa: relleno de huecos, el tipo del lote
                        // más reciente si los dos lados no llegaron juntos (sutura, absorción,
                        // salto de dorsal, rift, diente, isla, suavizado, invasión), fondo
                        // inicial junto a nuevo y dorsal.
                        let mut causa = [0.0f64; 11];
                        let mut largo = 0.0;
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
                                if vieja <= tc.a_min_myr || dif <= 0.5 * vieja {
                                    continue;
                                }
                                let l = d.malla.lado_comun[k][i];
                                largo += l;
                                let (o, y) = if ck.edad_oc > cv.edad_oc { (ck, cv) } else { (cv, ck) };
                                let c = if o.proc.nace == nace::HUECO || y.proc.nace == nace::HUECO {
                                    0
                                } else if o.proc.lote != y.proc.lote {
                                    d.lotes[o.proc.lote.max(y.proc.lote) as usize].clamp(1, 8) as usize
                                } else if o.proc.nace == nace::INICIAL {
                                    9
                                } else {
                                    10
                                };
                                causa[c] += l;
                            }
                        }
                        let f = |x: f64| if largo > 0.0 { 100.0 * x / largo } else { 0.0 };
                        // (b) Fondo de más de 180 Myr y márgenes pasivos viejos que vienen del
                        // estado inicial.
                        let (mut a180, mut a180_ini) = (0.0, 0.0);
                        for k in 0..num {
                            if let Some(c) = (d.dueno[k] != NADA).then(|| d.corteza(k)).flatten()
                                && !c.cont
                                && c.edad_oc > 180.0
                            {
                                a180 += d.malla.area[k];
                                if c.proc.nace == nace::INICIAL {
                                    a180_ini += d.malla.area[k];
                                }
                            }
                        }
                        // Margen pasivo: celda continental (no de arco) con fondo oceánico de su
                        // placa al lado y sin otra placa a menos de tres anillos (~150 km).
                        let (mut m_viejo, mut m_viejo_ini, mut m_todo, mut m_todo_ini) =
                            (0usize, 0usize, 0usize, 0usize);
                        for k in 0..num {
                            let p = d.dueno[k];
                            let Some(c) = (if p == NADA { None } else { d.corteza(k) }) else {
                                continue;
                            };
                            if !c.continente() {
                                continue;
                            }
                            let mut viejo: Option<(f64, u8)> = None;
                            for &v in &d.malla.vec[k] {
                                if v == NADA || d.dueno[v as usize] != p {
                                    continue;
                                }
                                if let Some(cv) = d.corteza(v as usize)
                                    && !cv.cont
                                    && viejo.is_none_or(|x| cv.edad_oc > x.0)
                                {
                                    viejo = Some((cv.edad_oc, cv.proc.nace));
                                }
                            }
                            let Some((edad, n)) = viejo else {
                                continue;
                            };
                            let mut anillo = vec![k as u32];
                            let mut otra = false;
                            for _ in 0..3 {
                                let mut sig = Vec::new();
                                for &c0 in &anillo {
                                    for &v in &d.malla.vec[c0 as usize] {
                                        if v == NADA {
                                            continue;
                                        }
                                        if d.dueno[v as usize] != p {
                                            otra = true;
                                        }
                                        sig.push(v);
                                    }
                                }
                                sig.sort_unstable();
                                sig.dedup();
                                anillo = sig;
                            }
                            if otra {
                                continue;
                            }
                            m_todo += 1;
                            m_todo_ini += usize::from(n == nace::INICIAL);
                            if edad > 180.0 {
                                m_viejo += 1;
                                m_viejo_ini += usize::from(n == nace::INICIAL);
                            }
                        }
                        let pc = |a: usize, b: usize| if b > 0 { 100.0 * a as f64 / b as f64 } else { 0.0 };
                        out += &format!(
                            "  {paso:>3} Myr: escalones enormes {largo:>7.0} km: relleno de huecos {:.0} %, sutura {:.0} %, absorción {:.0} %, salto de dorsal {:.0} %, rift {:.0} %, diente recolocado {:.0} %, isla {:.0} %, suavizado {:.0} %, invasión {:.0} %, fondo inicial junto a nuevo {:.0} %, dorsal {:.0} %; fondo de más de 180 Myr {:.1} Mkm², del estado inicial {:.0} %; márgenes pasivos de más de 180 Myr {} celdas, del estado inicial {:.0} % (todos los márgenes: {} celdas, {:.0} %)\n",
                            f(causa[0]), f(causa[1]), f(causa[2]), f(causa[3]), f(causa[4]), f(causa[5]), f(causa[6]), f(causa[7]), f(causa[8]), f(causa[9]), f(causa[10]),
                            a180 / 1e6,
                            if a180 > 0.0 { 100.0 * a180_ini / a180 } else { 0.0 },
                            m_viejo,
                            pc(m_viejo_ini, m_viejo),
                            m_todo,
                            pc(m_todo_ini, m_todo),
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

// --- Demografía de placas (nota_demografia_placas.md §0) -----------------------------------------

/// Foto del diagnóstico de la demografía de placas (§0), cada 50 Myr y al final.
#[derive(Default, Clone)]
struct FotoDemografia {
    myr: f64,
    /// D1: placas vivas con área, placas vivas sin área (consumidas y que siguen «vivas») y áreas
    /// (fracción de la esfera, de mayor a menor).
    placas: f64,
    fantasmas: f64,
    areas: Vec<f64>,
    /// D3: longitud de fosa activa, de convergencia difusa y de choque (km).
    fosas_km: f64,
    difusas_km: f64,
    choques_km: f64,
    /// D4: borde entre pares de placas con la velocidad relativa máxima de todo su borde común por
    /// debajo de 0,2 cm/año (km), por cuánto tiempo llevan así: <10, 10–20, 20–50, 50–100, ≥100 Myr.
    inactivo_km: [f64; 5],
    /// D4 con otros umbrales: borde de pares con la velocidad relativa máxima por debajo de 0,5 y de
    /// 1 cm/año (km).
    inactivo_otros_km: [f64; 2],
    /// Placas sin área: sus muestras (ocultas) y su volumen de tipo continental (km³).
    fantasmas_muestras: f64,
    fantasmas_v_cont_km3: f64,
    /// D2: longitud de fosa activa por su edad (<10, 10–25, 25–50, 50–100, 100–200, ≥200 Myr) y por
    /// su convergencia (<1, 1–2, 2–4, 4–6, 6–10, ≥10 cm/año).
    fosa_por_edad_km: [f64; 6],
    fosa_por_conv_km: [f64; 6],
    /// D5: por el origen de la placa (inicial, rift, invasión, salto de dorsal): placas, área
    /// (fracción de la esfera), fosa activa propia (km) y fondo que hunde (km²/año, media de los
    /// últimos 50 Myr).
    por_origen: [[f64; 4]; 4],
}

#[derive(Default)]
struct InformeDemografia {
    semilla: u64,
    fotos: Vec<FotoDemografia>,
    /// D1, tras el rodaje, por 100 Myr: nacimientos (rift, invasión, salto de dorsal, espontáneo) y
    /// muertes (consumida entera, soldada en un choque, absorbida, retirada al llegar su dorsal a la
    /// fosa (rama D) y microplaca capturada (A2)).
    nacen: [f64; 4],
    mueren: [f64; 5],
    /// D6, tras el rodaje: encuentros entre dorsales y fosas que empiezan por 100 Myr (un par
    /// dorsal–placa que entra en encuentro), su duración media (Myr), y cómo acaban los que acaban:
    /// por la rama D, con la placa de la franja muerta, o con la placa viva (la dorsal se aleja o la
    /// fosa muere), en fracción; subducciones de dorsal por 100 Myr, fosa que apagan (km por
    /// 100 Myr) y longitud media de dorsal en encuentro (km).
    d6_empiezan: f64,
    d6_duracion: f64,
    d6_final: [f64; 3],
    d6_subducciones: f64,
    d6_fosa_apagada: f64,
    d6_dorsal_km: f64,
    /// Nota de rifts v2 §10, tras el rodaje: rupturas por 100 Myr, placas de al menos el 1 % (media de
    /// los pasos), supercontinentes que se forman y se rompen y su duración media (Myr), y ciclos.
    rifts: f64,
    placas_1pc_media: f64,
    super_terminados: f64,
    super_duracion: f64,
    ciclos: f64,
    /// Bloque continental mayor (fracción del continente; media y máximo tras el rodaje): un
    /// supercontinente es un bloque que reúne casi todo el continente (Pangea, ~0,9).
    bloque_mayor: [f64; 2],
    /// §10, medias tras el rodaje: fondo que se hunde (km²/año), velocidad de las placas (cm/año),
    /// edad media del fondo (Myr; media de los pasos y al final) y correlación de Forsyth y Uyeda.
    a_sub: f64,
    v_rms: f64,
    edad_fondo: [f64; 2],
    fu_fosa: f64,
    /// D2, tras el rodaje, por 100 Myr (km): fosa creada por nacimiento inducido, nacimiento
    /// espontáneo, invasión de transformantes y separación de márgenes invadidos; perdida por muerte
    /// sin convergencia y por choque; y el resto del cambio (geometría del borde, divisiones y
    /// fusiones de placas).
    fosa_creada: [f64; 4],
    fosa_perdida: [f64; 2],
    fosa_resto: f64,
    /// D3 y D4, medias tras el rodaje: fosa, difusa y choque (km), y borde inactivo (km).
    conv_media_km: [f64; 3],
    inactivo_medio_km: f64,
    /// Placas: media tras el rodaje, y potencia de las fuerzas al final (tirón, empuje, choques,
    /// arrastre; fracción de la motriz o de la resistente).
    placas_media: f64,
    potencia: [f64; 4],
}

/// Origen de una placa para D5.
const ORIGENES: [&str; 4] = ["inicial", "rift", "invasión", "salto de dorsal"];

fn diagnosticar_demografia(semilla: u64, variante: &str) -> InformeDemografia {
    use std::collections::HashMap;
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
    let pasos = (tc.t_deriva_myr / tc.dt_myr).round() as u64;
    let rodaje = (tc.t_rodaje_myr / tc.dt_myr).round() as u64;
    let a_tot = d.malla.area_total();
    let v_fus = 0.2 * 10.0; // 0,2 cm/año en km/Myr
    let mut inf = InformeDemografia {
        semilla,
        ..Default::default()
    };
    let mut origen: HashMap<u64, usize> = d.placas.iter().map(|p| (p.id, 0)).collect();
    let mut vistos = 0usize;
    let mut con_area: HashMap<u64, bool> = HashMap::new();
    let mut inactivo_desde: HashMap<(u64, u64), f64> = HashMap::new();
    let mut hundido_ventana: HashMap<u64, f64> = HashMap::new();
    let mut l_fosa_ini = None;
    let (mut n_val, mut placas_suma) = (0.0, 0.0);
    // D6: pares (placa de la franja, placa del otro lado de la dorsal) en encuentro y desde cuándo.
    let mut en_encuentro: HashMap<(u64, u64), f64> = HashMap::new();
    let (mut d6_n_fin, mut d6_dur) = (0.0, 0.0);
    for paso in 1..=pasos {
        d.dar_paso();
        let tras_rodaje = paso > rodaje;
        let mut por_regla_d: Vec<(u64, u64)> = Vec::new();
        // Eventos nuevos: nacimientos y muertes de placas, fosa creada y perdida.
        for e in &d.eventos[vistos..] {
            let cuenta = |x: &mut f64| {
                if tras_rodaje {
                    *x += 1.0
                }
            };
            match e.tipo.as_str() {
                "rift" | "salto_dorsal" => {
                    let o = if e.tipo == "rift" { 1 } else { 3 };
                    if o == 1 && tras_rodaje {
                        inf.rifts += 1.0;
                    }
                    for &id in &e.placas[1..] {
                        origen.insert(id, o);
                    }
                    cuenta(&mut inf.nacen[if o == 1 { 0 } else { 2 }]);
                }
                "placa_nueva" => {
                    // Separación del océano de delante de un margen invadido: la del continente
                    // sigue a su madre y la del océano es nueva.
                    let madre = e.placas.first().and_then(|id| origen.get(id)).copied();
                    if let Some(&id_c) = e.placas.first() {
                        origen.entry(id_c).or_insert(madre.unwrap_or(0));
                    }
                    if let Some(&id_o) = e.placas.get(1) {
                        origen.insert(id_o, 2);
                    }
                    cuenta(&mut inf.nacen[1]);
                }
                "sutura" => cuenta(&mut inf.mueren[1]),
                "absorcion" => cuenta(&mut inf.mueren[2]),
                "subduccion_dorsal" => {
                    if e.clase.as_deref() == Some("retirada") {
                        cuenta(&mut inf.mueren[3]);
                    }
                    if tras_rodaje {
                        inf.d6_subducciones += 1.0;
                        inf.d6_fosa_apagada += e.fosa_km.unwrap_or(0.0);
                        if let (Some(&a), Some(&b)) = (e.placas.first(), e.placas.get(1)) {
                            por_regla_d.push((a, b));
                        }
                    }
                }
                // Una placa sin área ya contó como consumida entera al quedarse sin ella.
                "captura_microplaca" if e.area_km2 > 0.0 => cuenta(&mut inf.mueren[4]),
                "nacimiento_fosa" if tras_rodaje => match e.clase.as_deref() {
                    Some("inducido") => inf.fosa_creada[0] += e.longitud_km,
                    Some("espontaneo_transformante") => inf.fosa_creada[1] += e.longitud_km,
                    _ => {}
                },
                "invasion_margen" if tras_rodaje => inf.fosa_creada[3] += e.longitud_km,
                "muerte_fosa" if tras_rodaje => match e.clase.as_deref() {
                    Some("choque") => inf.fosa_perdida[1] += e.longitud_km,
                    _ => inf.fosa_perdida[0] += e.longitud_km,
                },
                _ => {}
            }
        }
        vistos = d.eventos.len();
        // Placas consumidas enteras: vivas que se quedan sin área.
        let mut area = vec![0.0; d.placas.len()];
        for k in 0..d.malla.num {
            if d.dueno[k] != NADA {
                area[d.dueno[k] as usize] += d.malla.area[k];
            }
        }
        // D6: encuentros que empiezan y que acaban en el paso.
        {
            let t = d.metricas.last().map_or(0.0, |m| m.myr);
            let ahora: std::collections::BTreeSet<(u64, u64)> =
                d.dorsal_fosa_paso.iter().map(|e| (e.p, e.r)).collect();
            let viva_con_area = |id: u64| {
                d.placas
                    .iter()
                    .enumerate()
                    .any(|(i, p)| p.viva && p.id == id && area[i] > 0.0)
            };
            let mut acaban: Vec<(u64, u64)> = en_encuentro
                .keys()
                .filter(|k| !ahora.contains(k))
                .copied()
                .collect();
            acaban.sort_unstable();
            for par in acaban {
                let desde = en_encuentro.remove(&par).unwrap_or(t);
                if tras_rodaje {
                    d6_n_fin += 1.0;
                    d6_dur += t - desde;
                    let i = if por_regla_d.contains(&par) {
                        0
                    } else if !viva_con_area(par.0) {
                        1
                    } else {
                        2
                    };
                    inf.d6_final[i] += 1.0;
                }
            }
            for &par in &ahora {
                if let std::collections::hash_map::Entry::Vacant(e) = en_encuentro.entry(par) {
                    e.insert(t);
                    if tras_rodaje {
                        inf.d6_empiezan += 1.0;
                    }
                }
            }
            // Con la rama D, el encuentro acaba en el mismo paso en que se aplica.
            for par in &por_regla_d {
                if let Some(desde) = en_encuentro.remove(par) {
                    d6_n_fin += 1.0;
                    d6_dur += t - desde;
                    inf.d6_final[0] += 1.0;
                }
            }
            if tras_rodaje {
                inf.d6_dorsal_km += d.dorsal_fosa_paso.iter().map(|e| e.dorsal_km).sum::<f64>();
                inf.placas_1pc_media += area.iter().filter(|&&a| a >= 0.01 * a_tot).count() as f64;
            }
        }
        for (pi, p) in d.placas.iter().enumerate().filter(|(_, p)| p.viva) {
            let tiene = area[pi] > 0.0;
            if con_area.insert(p.id, tiene) == Some(true) && !tiene && tras_rodaje {
                inf.mueren[0] += 1.0;
            }
        }
        let m = d.metricas.last().expect("métrica").clone();
        if tras_rodaje {
            inf.bloque_mayor[0] += m.bloque_mayor_frac;
            inf.bloque_mayor[1] = inf.bloque_mayor[1].max(m.bloque_mayor_frac);
            inf.fosa_creada[2] += m.invadido_km[0];
            if l_fosa_ini.is_none() {
                l_fosa_ini = Some(m.fosas_km);
            }
            n_val += 1.0;
            inf.a_sub += m.area_subducida_km2_anio;
            inf.v_rms += m.v_rms_cm_anio;
            inf.edad_fondo[0] += m.edad_media_oceano_myr;
            inf.conv_media_km[0] += m.fosas_km;
            inf.conv_media_km[1] += m.difusas_km;
            inf.conv_media_km[2] += m.colisiones_km;
            placas_suma += area.iter().filter(|&&a| a > 0.0).count() as f64;
        }
        for &(pi, a_sub) in &d.hundido_paso {
            *hundido_ventana
                .entry(d.placas[pi as usize].id)
                .or_insert(0.0) += a_sub;
        }
        // D4: velocidad relativa máxima de cada par a lo largo de su borde común.
        let mut pares: HashMap<(u64, u64), (f64, f64)> = HashMap::new();
        for b in &d.bordes {
            let (ip, iq) = (d.placas[b.p as usize].id, d.placas[b.q as usize].id);
            let clave = if ip < iq { (ip, iq) } else { (iq, ip) };
            let e = pares.entry(clave).or_insert((0.0, 0.0));
            e.0 = e.0.max(b.v_rel);
            e.1 += 0.5 * b.largo_km;
        }
        let t = m.myr;
        let mut inactivo_otros = [0.0; 2];
        for &(v_max, largo) in pares.values() {
            if v_max < 5.0 {
                inactivo_otros[0] += largo;
            }
            if v_max < 10.0 {
                inactivo_otros[1] += largo;
            }
        }
        inactivo_desde.retain(|par, _| pares.get(par).is_some_and(|x| x.0 < v_fus));
        let mut inactivo = [0.0; 5];
        for (par, &(v_max, largo)) in &pares {
            if v_max >= v_fus {
                continue;
            }
            let desde = *inactivo_desde.entry(*par).or_insert(t);
            let dur = t - desde;
            let i = [10.0, 20.0, 50.0, 100.0]
                .iter()
                .position(|&x| dur < x)
                .unwrap_or(4);
            inactivo[i] += largo;
        }
        if tras_rodaje {
            inf.inactivo_medio_km += inactivo.iter().sum::<f64>();
        }
        if paso % 50 == 0 || paso == pasos {
            let mut foto = FotoDemografia {
                myr: t,
                fosas_km: m.fosas_km,
                difusas_km: m.difusas_km,
                choques_km: m.colisiones_km,
                inactivo_km: inactivo,
                inactivo_otros_km: inactivo_otros,
                ..Default::default()
            };
            for (pi, p) in d.placas.iter().enumerate().filter(|(_, p)| p.viva) {
                if area[pi] > 0.0 {
                    foto.placas += 1.0;
                    foto.areas.push(area[pi] / a_tot);
                } else {
                    foto.fantasmas += 1.0;
                    foto.fantasmas_muestras += p.cort.len() as f64;
                    for (s, c) in p.cort.iter().enumerate() {
                        foto.fantasmas_v_cont_km3 +=
                            c.v_cont(tc.rho_s, tc.rho_c) * d.malla.area[p.celda[s] as usize];
                    }
                }
            }
            foto.areas.sort_by(|a, b| b.total_cmp(a));
            let mut fosa_placa: HashMap<u32, f64> = HashMap::new();
            for (pi, largo, edad, v_n) in d.fosas_activas() {
                *fosa_placa.entry(pi).or_insert(0.0) += largo;
                let ie = [10.0, 25.0, 50.0, 100.0, 200.0]
                    .iter()
                    .position(|&x| edad < x)
                    .unwrap_or(5);
                foto.fosa_por_edad_km[ie] += largo;
                let v = v_n / 10.0;
                let iv = [1.0, 2.0, 4.0, 6.0, 10.0]
                    .iter()
                    .position(|&x| v < x)
                    .unwrap_or(5);
                foto.fosa_por_conv_km[iv] += largo;
            }
            let ventana = if paso % 50 == 0 {
                50.0
            } else {
                (paso % 50) as f64
            } * tc.dt_myr
                * 1e6;
            for (pi, p) in d.placas.iter().enumerate().filter(|(_, p)| p.viva) {
                if area[pi] <= 0.0 {
                    continue;
                }
                let o = origen.get(&p.id).copied().unwrap_or(0);
                let fila = &mut foto.por_origen[o];
                fila[0] += 1.0;
                fila[1] += area[pi] / a_tot;
                fila[2] += fosa_placa.get(&(pi as u32)).copied().unwrap_or(0.0);
                fila[3] += hundido_ventana.get(&p.id).copied().unwrap_or(0.0) / ventana;
            }
            hundido_ventana.clear();
            inf.fotos.push(foto);
        }
    }
    let n100 = n_val / 100.0;
    for x in inf.nacen.iter_mut().chain(inf.mueren.iter_mut()) {
        *x /= n100;
    }
    inf.d6_empiezan /= n100;
    inf.d6_subducciones /= n100;
    inf.d6_fosa_apagada /= n100;
    inf.rifts /= n100;
    inf.d6_dorsal_km /= n_val;
    inf.placas_1pc_media /= n_val;
    inf.bloque_mayor[0] /= n_val;
    inf.d6_duracion = if d6_n_fin > 0.0 {
        d6_dur / d6_n_fin
    } else {
        0.0
    };
    let fin_tot: f64 = inf.d6_final.iter().sum();
    if fin_tot > 0.0 {
        for x in &mut inf.d6_final {
            *x /= fin_tot;
        }
    }
    let balance = abm_terreno::tectonica::estado_final(&d).1.balance;
    inf.super_terminados = balance.supercontinentes_terminados as f64;
    inf.super_duracion = balance.duracion_media_supercontinente_myr;
    inf.ciclos = balance.ciclos_supercontinente as f64;
    for x in inf
        .fosa_creada
        .iter_mut()
        .chain(inf.fosa_perdida.iter_mut())
    {
        *x /= n100;
    }
    let l_fin = d.metricas.last().map_or(0.0, |m| m.fosas_km);
    inf.fosa_resto = (l_fin - l_fosa_ini.unwrap_or(l_fin)) / n100
        - (inf.fosa_creada.iter().sum::<f64>() - inf.fosa_perdida.iter().sum::<f64>());
    for x in &mut inf.conv_media_km {
        *x /= n_val;
    }
    inf.inactivo_medio_km /= n_val;
    inf.placas_media = placas_suma / n_val;
    inf.a_sub /= n_val;
    inf.v_rms /= n_val;
    inf.edad_fondo[0] /= n_val;
    inf.edad_fondo[1] = d.metricas.last().map_or(0.0, |m| m.edad_media_oceano_myr);
    inf.fu_fosa = d.fu_fosa.0 / (d.fu_fosa.1.max(1) as f64);
    // Potencia de las fuerzas al final (nota de fosas §10, medida 3).
    let mut pot = [0.0; 4];
    for (_, _, x) in &d.potencias {
        for i in 0..4 {
            pot[i] += x[i];
        }
        // La succión de las losas (nota de succión) cuenta como motor, con el empuje.
        pot[1] += x[4];
    }
    let motriz = pot[0] + pot[1];
    if motriz > 0.0 {
        inf.potencia = [
            pot[0] / motriz,
            pot[1] / motriz,
            -pot[2] / motriz,
            -pot[3] / motriz,
        ];
    }
    inf
}

#[test]
#[ignore = "diagnóstico: demografía de placas y exceso de fosas (nota_demografia_placas.md §0; SEMILLAS, VARIANTES; ~5 min)"]
fn diagnostico_demografia_placas() {
    let semillas: Vec<u64> = std::env::var("SEMILLAS")
        .unwrap_or_else(|_| "42,1,2,3,4".into())
        .split(',')
        .map(|x| x.trim().parse().expect("semilla"))
        .collect();
    let variantes: Vec<String> = std::env::var("VARIANTES")
        .unwrap_or_default()
        .split(';')
        .map(|x| x.trim().to_string())
        .collect();
    let trabajos: Vec<(u64, &str)> = semillas
        .iter()
        .flat_map(|&s| variantes.iter().map(move |v| (s, v.as_str())))
        .collect();
    let informes: Vec<(String, InformeDemografia)> = std::thread::scope(|sc| {
        let hilos: Vec<_> = trabajos
            .iter()
            .map(|&(s, v)| sc.spawn(move || (v.to_string(), diagnosticar_demografia(s, v))))
            .collect();
        hilos.into_iter().map(|h| h.join().expect("hilo")).collect()
    });
    // Bird (2003) proyectado en la malla (config/referencias/tierra_bordes.toml).
    let bird: Vec<f64> = {
        let t: toml::Value = toml::from_str(
            &std::fs::read_to_string("../../config/referencias/tierra_bordes.toml").expect("Bird"),
        )
        .expect("toml");
        t["bordes"]["todas"]["areas_placas_frac"]["mayor_a_menor"]
            .as_array()
            .expect("áreas")
            .iter()
            .map(|x| x.as_float().expect("número"))
            .collect()
    };
    let mayores = |a: &[f64], n: usize| a.iter().take(n).sum::<f64>();
    let cuantas = |a: &[f64], x: f64| a.iter().filter(|&&v| v >= x).count() as f64;
    println!(
        "Bird en la malla: {} placas; las 15 mayores, {:.3} del planeta; de ≥1 %: {}, ≥0,5 %: {}, ≥0,1 %: {}",
        bird.len(),
        mayores(&bird, 15),
        cuantas(&bird, 0.01),
        cuantas(&bird, 0.005),
        cuantas(&bird, 0.001)
    );
    for var in &variantes {
        let g: Vec<&InformeDemografia> = informes
            .iter()
            .filter(|x| &x.0 == var)
            .map(|x| &x.1)
            .collect();
        let ms = |f: &dyn Fn(&InformeDemografia) -> f64| {
            let v: Vec<f64> = g.iter().map(|i| f(i)).collect();
            let n = v.len() as f64;
            let m = v.iter().sum::<f64>() / n;
            let sd = (v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (n - 1.0).max(1.0)).sqrt();
            format!("{m:.2} ± {sd:.2}")
        };
        println!("\n### [{var}], {} semillas", g.len());
        for i in &g {
            println!(
                "  semilla {}: placas con área (media) {:.1}, fosa media {:.0} km",
                i.semilla, i.placas_media, i.conv_media_km[0]
            );
        }
        println!("D1/D3/D4/D5 cada 50 Myr (media de las semillas):");
        println!(
            "  Myr | placas (sin área) | 15 mayores | ≥1 % / ≥0,5 % / ≥0,1 % | fosa / difusa / choque (km) | inactivo <10/10–20/20–50/50–100/≥100 Myr (km)"
        );
        let nf = g[0].fotos.len();
        for j in 0..nf {
            let f = |x: &dyn Fn(&FotoDemografia) -> f64| {
                g.iter().map(|i| x(&i.fotos[j])).sum::<f64>() / g.len() as f64
            };
            println!(
                "  {:4.0} | {:5.1} ({:3.1}) | {:.3} | {:4.1} / {:4.1} / {:4.1} | {:6.0} / {:6.0} / {:6.0} | {:?}",
                g[0].fotos[j].myr,
                f(&|x| x.placas),
                f(&|x| x.fantasmas),
                f(&|x| mayores(&x.areas, 15)),
                f(&|x| cuantas(&x.areas, 0.01)),
                f(&|x| cuantas(&x.areas, 0.005)),
                f(&|x| cuantas(&x.areas, 0.001)),
                f(&|x| x.fosas_km),
                f(&|x| x.difusas_km),
                f(&|x| x.choques_km),
                (0..5)
                    .map(|i| f(&|x| x.inactivo_km[i]).round())
                    .collect::<Vec<_>>()
            );
        }
        println!(
            "D1 y D4: placas sin área (muestras ocultas que guardan, km³ de tipo continental) y borde inactivo con 0,5 y 1 cm/año (km):"
        );
        for j in 0..nf {
            let f = |x: &dyn Fn(&FotoDemografia) -> f64| {
                g.iter().map(|i| x(&i.fotos[j])).sum::<f64>() / g.len() as f64
            };
            println!(
                "  {:4.0} | {:4.1} placas sin área con {:6.0} muestras y {:9.0} km³ | inactivo <0,5: {:6.0}, <1: {:6.0}",
                g[0].fotos[j].myr,
                f(&|x| x.fantasmas),
                f(&|x| x.fantasmas_muestras),
                f(&|x| x.fantasmas_v_cont_km3),
                f(&|x| x.inactivo_otros_km[0]),
                f(&|x| x.inactivo_otros_km[1])
            );
        }
        println!(
            "D2, fosa activa por edad (<10, 10–25, 25–50, 50–100, 100–200, ≥200 Myr) y por convergencia (<1, 1–2, 2–4, 4–6, 6–10, ≥10 cm/año), fracción de la longitud:"
        );
        for j in 0..nf {
            let tot_e: f64 = g
                .iter()
                .map(|i| i.fotos[j].fosa_por_edad_km.iter().sum::<f64>())
                .sum();
            let tot_v: f64 = g
                .iter()
                .map(|i| i.fotos[j].fosa_por_conv_km.iter().sum::<f64>())
                .sum();
            let fe: Vec<f64> = (0..6)
                .map(|i| {
                    (1000.0
                        * g.iter()
                            .map(|x| x.fotos[j].fosa_por_edad_km[i])
                            .sum::<f64>()
                        / tot_e.max(1e-9))
                    .round()
                        / 1000.0
                })
                .collect();
            let fv: Vec<f64> = (0..6)
                .map(|i| {
                    (1000.0
                        * g.iter()
                            .map(|x| x.fotos[j].fosa_por_conv_km[i])
                            .sum::<f64>()
                        / tot_v.max(1e-9))
                    .round()
                        / 1000.0
                })
                .collect();
            println!(
                "  {:4.0} | edad {fe:?} | convergencia {fv:?}",
                g[0].fotos[j].myr
            );
        }
        println!(
            "D5, por el origen de la placa (placas, área, fosa propia km, fondo que hunde km²/año), media de las semillas:"
        );
        for j in 0..nf {
            let celdas: Vec<String> = (0..4)
                .map(|o| {
                    let f = |c: usize| {
                        g.iter().map(|x| x.fotos[j].por_origen[o][c]).sum::<f64>() / g.len() as f64
                    };
                    format!(
                        "{} {:.1} / {:.3} / {:.0} / {:.2}",
                        ORIGENES[o],
                        f(0),
                        f(1),
                        f(2),
                        f(3)
                    )
                })
                .collect();
            println!("  {:4.0} | {}", g[0].fotos[j].myr, celdas.join(" | "));
        }
        println!(
            "D1, por 100 Myr tras el rodaje: nacen por rift {}, invasión {}, salto de dorsal {}, espontáneo {}; mueren consumidas enteras {}, soldadas en un choque {}, absorbidas {}, retiradas al llegar su dorsal a la fosa {}, microplacas capturadas {}; placas con área (media) {}",
            ms(&|i| i.nacen[0]),
            ms(&|i| i.nacen[1]),
            ms(&|i| i.nacen[2]),
            ms(&|i| i.nacen[3]),
            ms(&|i| i.mueren[0]),
            ms(&|i| i.mueren[1]),
            ms(&|i| i.mueren[2]),
            ms(&|i| i.mueren[3]),
            ms(&|i| i.mueren[4]),
            ms(&|i| i.placas_media)
        );
        println!(
            "§10, medias tras el rodaje: fondo que se hunde {} km²/año (2,7–3,4); velocidad de las placas {} cm/año (4,39 ± 25 %); edad media del fondo {} Myr en la deriva y {} al final (no peor que 88); Forsyth–Uyeda fosa {}",
            ms(&|i| i.a_sub),
            ms(&|i| i.v_rms),
            ms(&|i| i.edad_fondo[0]),
            ms(&|i| i.edad_fondo[1]),
            ms(&|i| i.fu_fosa)
        );
        println!(
            "D2, km de fosa por 100 Myr tras el rodaje: creada por nacimiento inducido {}, espontáneo {}, invasión de transformantes {}, separación de márgenes invadidos {}; perdida sin convergencia {}, por choque {}; resto (geometría, divisiones y fusiones) {}",
            ms(&|i| i.fosa_creada[0]),
            ms(&|i| i.fosa_creada[1]),
            ms(&|i| i.fosa_creada[2]),
            ms(&|i| i.fosa_creada[3]),
            ms(&|i| i.fosa_perdida[0]),
            ms(&|i| i.fosa_perdida[1]),
            ms(&|i| i.fosa_resto)
        );
        println!(
            "D3, medias tras el rodaje (km): fosa {} (Bird SUB 51 309), difusa {} (OCB 17 447), choque {} (CCB 23 003); D4, borde inactivo {} km",
            ms(&|i| i.conv_media_km[0]),
            ms(&|i| i.conv_media_km[1]),
            ms(&|i| i.conv_media_km[2]),
            ms(&|i| i.inactivo_medio_km)
        );
        println!(
            "Potencia al final: tirón {} y empuje {} de la motriz; choques {} y arrastre {} de la resistente",
            ms(&|i| i.potencia[0]),
            ms(&|i| i.potencia[1]),
            ms(&|i| i.potencia[2]),
            ms(&|i| i.potencia[3])
        );
        println!(
            "D6, tras el rodaje: encuentros entre dorsales y fosas que empiezan {} por 100 Myr, duran {} Myr; acaban por la rama D {}, con la placa de la franja muerta {}, con ella viva {} (fracción); subducciones de dorsal {} por 100 Myr, que apagan {} km de fosa por 100 Myr; dorsal en encuentro {} km de media",
            ms(&|i| i.d6_empiezan),
            ms(&|i| i.d6_duracion),
            ms(&|i| i.d6_final[0]),
            ms(&|i| i.d6_final[1]),
            ms(&|i| i.d6_final[2]),
            ms(&|i| i.d6_subducciones),
            ms(&|i| i.d6_fosa_apagada),
            ms(&|i| i.d6_dorsal_km)
        );
        println!(
            "Rifts v2, tras el rodaje: rupturas {} por 100 Myr; placas de al menos el 1 % {} de media (Bird 13); supercontinentes que se forman y se rompen {} con duración media {} Myr (~100–300); ciclos {}; bloque continental mayor {} de media y {} de máximo (fracción del continente)",
            ms(&|i| i.rifts),
            ms(&|i| i.placas_1pc_media),
            ms(&|i| i.super_terminados),
            ms(&|i| i.super_duracion),
            ms(&|i| i.ciclos),
            ms(&|i| i.bloque_mayor[0]),
            ms(&|i| i.bloque_mayor[1])
        );
    }
}

// --- Muerte de placas (nota_demografia_placas.md v2: A2 y rama D) --------------------------------

/// Configuración de las pruebas de la muerte de placas: sin rifts ni saltos de dorsal, sin erosión
/// ni nacimientos espontáneos.
fn cfg_muerte() -> TectonicaCfg {
    TectonicaCfg {
        lambda_a_rift: 0.0,
        n_min: 1,
        lambda_esp: 0.0,
        e1_km_myr: 1e-12,
        ..TectonicaCfg::default()
    }
}

#[test]
fn una_microplaca_sin_fosa_la_captura_la_vecina_con_mas_borde() {
    // Nota de demografía §10, prueba 3: una microplaca (casquete de 3°, ~0,35 Mkm²) sin fosa la
    // captura, a los τ_micro, la vecina con la que comparte más borde: la placa del norte, donde cae
    // la mayor parte de su borde.
    let cfg = cfg_muerte();
    let malla = Malla::new(48, 1, 6371.0);
    let dirs = malla.dir.clone();
    let centro = [1.0f64.to_radians().sin(), 0.0, 1.0f64.to_radians().cos()];
    let centro = [centro[2], 0.0, centro[0]];
    let mut d = Deriva::desde(&cfg, 6, malla, 0.0, &[100, 200, 300], |k| {
        let x = dirs[k];
        if dot(x, centro) > 3f64.to_radians().cos() {
            (2, corteza(false, 7.0, 30.0))
        } else if x[2] >= 0.0 {
            (0, corteza(false, 7.0, 60.0))
        } else {
            (1, corteza(false, 7.0, 60.0))
        }
    });
    d.arrancar_con(&[[0.0; 3], [0.0; 3], [0.0; 3]]);
    let t_micro = cfg.tau_micro_myr.round() as usize;
    for _ in 0..t_micro - 2 {
        d.dar_paso();
    }
    assert!(
        d.placas.iter().any(|p| p.viva && p.id == 300),
        "antes de τ_micro sigue viva"
    );
    for _ in 0..4 {
        d.dar_paso();
    }
    let cap = d
        .eventos
        .iter()
        .find(|e| e.tipo == "captura_microplaca")
        .expect("la microplaca es capturada");
    assert_eq!(
        cap.placas,
        vec![300, 100],
        "la captura la vecina con más borde"
    );
    assert!(!d.placas.iter().any(|p| p.viva && p.id == 300));
}

/// Escenario de la rama D (nota de demografía de placas v2): una placa oceánica estrecha `P` (id 20,
/// 2° de anchura junto al ecuador, ~220 km) entre una dorsal con la placa `R` (id 10, al oeste) y una
/// fosa activa donde se hunde bajo el continente de `U` (id 30, al este). Lleva sedimento para que
/// la cuenta de la corteza vea la franja. La velocidad media del planeta a la que se calibra el
/// rozamiento basal es baja (`v_obj`): solo se mueve `P`, que se llevaría toda la de la Tierra.
fn cfg_dorsal() -> TectonicaCfg {
    TectonicaCfg {
        v_obj_cm_anio: 0.3,
        ..cfg_muerte()
    }
}

fn dorsal_junto_a_la_fosa(cfg: &TectonicaCfg) -> Deriva {
    let malla = Malla::new(48, 1, 6371.0);
    let dirs = malla.dir.clone();
    let mut d = Deriva::desde(cfg, 7, malla, 0.0, &[10, 20, 30], |k| {
        let (la, lo) = lat_lon(dirs[k]);
        let (la, lo) = (la.to_degrees(), lo.to_degrees());
        if (-22.0..-20.0).contains(&lo) && la.abs() < 40.0 {
            let mut c = corteza(false, 7.0, 5.0);
            c.sed = 0.2;
            (1, c)
        } else if (-20.0..160.0).contains(&lo) {
            let cont = lo < -5.0 && la.abs() < 45.0;
            (2, corteza(cont, if cont { 35.0 } else { 7.0 }, 60.0))
        } else {
            (0, corteza(false, 7.0, 30.0))
        }
    });
    // P se aleja de R hacia U (giro alrededor del eje de la Tierra: hacia el este en el ecuador).
    // La losa de P es corta: con el tirón pleno, una placa tan estrecha correría a decenas de cm/año
    // y se hundiría entera en un paso.
    let w = 5.0 * 10.0 / 6371.0;
    d.arrancar_con(&[[0.0, 0.0, -w], [0.0, 0.0, w], [0.0; 3]]);
    d.activar_fosas(10.0);
    d
}

#[test]
fn una_dorsal_que_llega_a_la_fosa_muere_y_su_franja_pasa_a_la_placa_del_otro_lado() {
    // Rama D: la franja de P entre la dorsal y la fosa mide menos de d_dc (300 km), así que la
    // dorsal muere, la franja pasa a R con la corteza conservada y P, sin muestras, se retira.
    let cfg = cfg_dorsal();
    let mut d = dorsal_junto_a_la_fosa(&cfg);
    let mut evento = None;
    for _ in 0..5 {
        d.dar_paso();
        let err = d.metricas.last().map_or(0.0, |m| m.error_balance_rel);
        assert!(err < 1e-9, "el balance de la corteza cierra: {err:.2e}");
        if let Some(e) = d.eventos.iter().find(|e| e.tipo == "subduccion_dorsal") {
            evento = Some(e.clone());
            break;
        }
    }
    let e = evento.expect("la dorsal llega a la fosa y muere");
    assert_eq!(&e.placas[..2], &[20, 10], "la franja de P pasa a R");
    assert!(
        e.placas[2..].contains(&30),
        "la fosa que se apaga es la de U"
    );
    assert!(e.area_km2 > 0.0 && e.longitud_km > 0.0);
    assert!(
        e.fosa_km.is_some_and(|x| x > 0.0),
        "se apaga fosa: {:?}",
        e.fosa_km
    );
    assert_eq!(
        e.clase.as_deref(),
        Some("retirada"),
        "P se queda sin muestras"
    );
    assert!(
        !d.placas.iter().any(|p| p.viva && p.id == 20),
        "P se retira como placa"
    );
    // Ya no hay fosa activa donde se hunda P, y el balance sigue cerrando en el paso siguiente.
    assert!(
        d.fosas_activas()
            .iter()
            .all(|f| d.placas[f.0 as usize].id != 20)
    );
    d.dar_paso();
    let err = d.metricas.last().map_or(0.0, |m| m.error_balance_rel);
    assert!(err < 1e-9, "el balance de la corteza cierra: {err:.2e}");
}

// --- Nivel del mar y edad del fondo (nota de isostasia con litosfera; ronda del 2026-11-08) -------

/// Edad (Myr) del cuantil `q` de una distribución por clases de 10 Myr (`frac`), uniforme dentro de
/// cada clase.
fn edad_del_cuantil_10myr(frac: &[f64], q: f64) -> f64 {
    let total: f64 = frac.iter().sum();
    let mut acum = 0.0;
    for (i, &f) in frac.iter().enumerate() {
        let f = f / total;
        if f > 0.0 && acum + f >= q {
            return 10.0 * (i as f64 + (q - acum) / f);
        }
        acum += f;
    }
    10.0 * frac.len() as f64
}

#[test]
#[ignore = "exploratoria: el nivel del mar al final de la deriva con las edades del fondo del modelo y con las de la Tierra en los mismos cuantiles (Seton et al. 2020, config/referencias/tierra_relieve.toml); SEMILLAS=42,1,2,3,4 y VARIANTES como en deriva_en_varias_semillas (~5 min)"]
fn diagnostico_nivel_del_mar_y_edad_del_fondo() {
    use abm_terreno::tectonica::deriva::Corteza;
    use abm_terreno::tectonica::isostasia;
    let semillas: Vec<u64> = std::env::var("SEMILLAS")
        .unwrap_or_else(|_| "42,1,2,3,4".into())
        .split(',')
        .map(|x| x.trim().parse().expect("semilla"))
        .collect();
    let variante = std::env::var("VARIANTES").unwrap_or_default();
    let ajustes: Vec<String> = variante
        .split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect();
    let referencia: toml::Value = toml::from_str(
        &std::fs::read_to_string("../../config/referencias/tierra_relieve.toml")
            .expect("tierra_relieve.toml"),
    )
    .expect("toml");
    let frac_tierra: Vec<f64> = referencia["fondo_oceanico"]["edades"]["area_frac_por_10_myr"]
        .as_array()
        .expect("area_frac_por_10_myr")
        .iter()
        .map(|x| x.as_float().expect("número"))
        .collect();
    let media_tierra: f64 = frac_tierra
        .iter()
        .enumerate()
        .map(|(i, f)| f * (10.0 * i as f64 + 5.0))
        .sum::<f64>()
        / frac_tierra.iter().sum::<f64>();
    let res: Vec<(u64, [f64; 7], [[f64; 4]; 5])> = std::thread::scope(|sc| {
        let hilos: Vec<_> = semillas
            .iter()
            .map(|&s| {
                let (ajustes, frac_tierra) = (&ajustes, &frac_tierra);
                sc.spawn(move || {
                    let cfg = abm_terreno::config::TerrenoConfig::load_con(
                        "../../config/terreno.toml",
                        ajustes,
                    )
                    .expect("config");
                    let tc = cfg.tectonica.clone();
                    let malla = Malla::new(cfg.planet.n0, tc.nivel_malla, cfg.planet.r_km);
                    let mut d = Deriva::nueva(&tc, s, malla);
                    let pasos = (tc.t_deriva_myr / tc.dt_myr).round() as u64;
                    for _ in 0..pasos {
                        d.dar_paso();
                    }
                    let z = d.alturas();
                    let vol = tc.gel_m / 1000.0 * d.malla.area_total();
                    let zeta = isostasia::nivel_del_mar(&z, &d.malla.area, vol);
                    // Cuantil de área de cada celda oceánica por su edad (desempate por índice).
                    let mut oc: Vec<(f64, usize)> = (0..d.malla.num)
                        .filter_map(|k| d.corteza(k).filter(|c| !c.cont).map(|c| (c.edad_oc, k)))
                        .collect();
                    oc.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
                    let a_oc: f64 = oc.iter().map(|&(_, k)| d.malla.area[k]).sum();
                    let edad_media: f64 =
                        oc.iter().map(|&(e, k)| e * d.malla.area[k]).sum::<f64>() / a_oc;
                    let mut z_t = z.clone();
                    let mut acum = 0.0;
                    for &(_, k) in &oc {
                        let a = d.malla.area[k];
                        let q = (acum + 0.5 * a) / a_oc;
                        acum += a;
                        let c = d.corteza(k).expect("corteza");
                        let nueva = Corteza {
                            edad_oc: edad_del_cuantil_10myr(frac_tierra, q),
                            ..*c
                        };
                        z_t[k] = isostasia::altura_mundo(&tc, &nueva, d.abombamiento_m[k]);
                    }
                    let zeta_t = isostasia::nivel_del_mar(&z_t, &d.malla.area, vol);
                    // Profundidad media del océano (sin la corteza continental) con cada nivel.
                    let prof = |z: &[f64], zeta: f64| {
                        let (mut s, mut a) = (0.0, 0.0);
                        for &(_, k) in &oc {
                            if z[k] < zeta {
                                s += (zeta - z[k]) * d.malla.area[k];
                                a += d.malla.area[k];
                            }
                        }
                        s / a.max(1e-9)
                    };
                    let f_cont = (0..d.malla.num)
                        .filter(|&k| d.corteza(k).is_some_and(|c| c.cont))
                        .map(|k| d.malla.area[k])
                        .sum::<f64>()
                        / d.malla.area_total();
                    // §11.3: por clase de edad tectonotérmica, la tierra (área, altura media sobre el
                    // mar, corteza con su sedimento y litosfera medias).
                    let clases = abm_terreno::tectonica::deriva::CLASES_EDAD_TT_MYR;
                    let mut por_clase = [[0.0f64; 4]; 5];
                    for (k, &zk) in z.iter().enumerate() {
                        let Some(c) = d.corteza(k).filter(|c| c.cont) else {
                            continue;
                        };
                        if zk < zeta {
                            continue;
                        }
                        let i = clases.partition_point(|&x| x <= c.edad_tt) - 1;
                        let a = d.malla.area[k];
                        por_clase[i][0] += a;
                        por_clase[i][1] += (zk - zeta) * a;
                        por_clase[i][2] += (c.h + c.sed) * a;
                        por_clase[i][3] += isostasia::litosfera_km(&tc, c.edad_tt) * a;
                    }
                    let tierra: f64 = por_clase.iter().map(|x| x[0]).sum();
                    for x in por_clase.iter_mut() {
                        if x[0] > 0.0 {
                            x[1] /= x[0];
                            x[2] /= x[0];
                            x[3] /= x[0];
                            x[0] /= tierra;
                        }
                    }
                    (
                        s,
                        [
                            zeta,
                            zeta_t,
                            edad_media,
                            prof(&z, zeta),
                            prof(&z_t, zeta_t),
                            f_cont,
                            d.zeta_m,
                        ],
                        por_clase,
                    )
                })
            })
            .collect();
        hilos.into_iter().map(|h| h.join().expect("hilo")).collect()
    });
    println!(
        "Edad media del fondo de la Tierra (Seton et al. 2020): {media_tierra:.1} Myr. Variante: [{variante}]"
    );
    println!(
        "semilla | nivel del mar (m) | con las edades de la Tierra | coste de la edad | edad media | prof. fondo oceánico (m), modelo / Tierra | f_cont | zeta de la deriva"
    );
    for (s, r, _) in &res {
        println!(
            "{s:>7} | {:>8.0} | {:>8.0} | {:>+6.0} | {:>6.1} | {:>6.0} / {:>6.0} | {:.3} | {:.0}",
            r[0],
            r[1],
            r[0] - r[1],
            r[2],
            r[3],
            r[4],
            r[5],
            r[6]
        );
    }
    let n = res.len() as f64;
    let m = |i: usize| res.iter().map(|x| x.1[i]).sum::<f64>() / n;
    println!(
        "  media | {:>8.0} | {:>8.0} | {:>+6.0} | {:>6.1} | {:>6.0} / {:>6.0} | {:.3}",
        m(0),
        m(1),
        m(0) - m(1),
        m(2),
        m(3),
        m(4),
        m(5)
    );
    println!(
        "§11.3, la tierra por clase de edad tectonotérmica (media de las semillas): fracción de la tierra, altura media (m), corteza con su sedimento (km), litosfera (km)"
    );
    let nombres = ["0–100", "100–500", "500–1 600", "1 600–2 500", "> 2 500"];
    for (i, nom) in nombres.iter().enumerate() {
        let mc = |j: usize| res.iter().map(|x| x.2[i][j]).sum::<f64>() / n;
        println!(
            "  {nom:>12}: {:.3} | {:>6.0} m | {:>5.1} km | {:>5.1} km",
            mc(0),
            mc(1),
            mc(2),
            mc(3)
        );
    }
}

#[test]
#[ignore = "exploratoria: huella del estado final de la deriva del planeta (SEMILLA, por defecto 2), para comparar procesos distintos (determinismo, CLAUDE §6)"]
fn diagnostico_huella_de_la_deriva() {
    let semilla: u64 = std::env::var("SEMILLA")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(2);
    let cfg = abm_terreno::config::TerrenoConfig::load_con("../../config/terreno.toml", &[])
        .expect("config");
    let tc = cfg.tectonica.clone();
    let malla = Malla::new(cfg.planet.n0, tc.nivel_malla, cfg.planet.r_km);
    let mut d = Deriva::nueva(&tc, semilla, malla);
    let pasos = (tc.t_deriva_myr / tc.dt_myr).round() as u64;
    // FNV-1a sobre los bits de todo lo que la génesis toma de la deriva y de su estado.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mete = |x: u64| {
        for b in x.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    };
    for paso in 1..=pasos {
        d.dar_paso();
        if paso % 50 == 0 || paso == pasos {
            let mut hp: u64 = 0;
            for p in d.placas.iter().filter(|p| p.viva) {
                for c in &p.cort {
                    for x in [
                        c.h,
                        c.hc,
                        c.sed,
                        c.arco,
                        c.edad_oc,
                        c.edad_orog,
                        c.dh_dt,
                        c.edad_tt,
                    ] {
                        hp = hp.rotate_left(7) ^ x.to_bits();
                    }
                }
            }
            println!("paso {paso}: {hp:016x}");
        }
    }
    for p in d.placas.iter().filter(|p| p.viva) {
        mete(p.id);
        for c in &p.cort {
            for x in [
                c.h,
                c.hc,
                c.sed,
                c.arco,
                c.edad_oc,
                c.edad_orog,
                c.dh_dt,
                c.edad_tt,
            ] {
                mete(x.to_bits());
            }
        }
    }
    for k in 0..d.malla.num {
        mete(d.dueno[k] as u64);
        mete(d.rep[k] as u64);
    }
    println!("huella final de la deriva (semilla {semilla}): {h:016x}");
}

// --- Hundimiento térmico de los márgenes (nota_hundimiento_margenes.md §10) -----------------------

#[test]
fn un_margen_estirado_se_hunde_con_la_curva_de_mckenzie() {
    // §10, prueba 1: el continente de la prueba del rift (35 km) se parte y se rompe; después, el
    // basamento de cada muestra de los márgenes estirados (β ≥ 1,5), bajo el mar y sin sedimento,
    // se hunde lo que da McKenzie (1978) para su β, ± 15 %.
    use abm_terreno::tectonica::isostasia;
    let cfg = TectonicaCfg {
        lambda_a_rift: 1e3,
        n_min: 2,
        lambda_esp: 0.0,
        ..TectonicaCfg::default()
    };
    let malla = Malla::new(48, 2, 6371.0);
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
    d.cfg.lambda_a_rift = 0.0;
    // Hasta la rotura de los dos lados.
    for _ in 0..30 {
        d.paso_cinematico();
        if d.eventos.iter().filter(|e| e.tipo == "rotura_rift").count() >= 2 {
            break;
        }
    }
    assert!(
        d.eventos.iter().filter(|e| e.tipo == "rotura_rift").count() >= 2,
        "los dos lados del rift se rompen"
    );
    let basamento = |d: &Deriva, c: &abm_terreno::tectonica::deriva::Corteza| {
        isostasia::altura(
            &d.cfg,
            &abm_terreno::tectonica::deriva::Corteza { sed: 0.0, ..*c },
        )
    };
    // Las muestras estiradas bajo el mar, con su altura y su tiempo desde la rotura.
    let mut antes: std::collections::BTreeMap<(u64, u32), (f64, f64, f64)> = Default::default();
    for p in d.placas.iter().filter(|p| p.viva) {
        for (s, c) in p.cort.iter().enumerate() {
            if c.cont && c.beta >= 1.5 && basamento(&d, c) < 0.0 {
                antes.insert((p.id, p.celda[s]), (basamento(&d, c), c.beta, c.t_rot_myr));
            }
        }
    }
    assert!(antes.len() >= 10, "hay márgenes estirados: {}", antes.len());
    for _ in 0..80 {
        d.paso_cinematico();
    }
    let (mut n, mut peor, mut suma) = (0, 0.0f64, 0.0);
    for p in d.placas.iter().filter(|p| p.viva) {
        for (s, c) in p.cort.iter().enumerate() {
            let Some(&(z0, beta, t0)) = antes.get(&(p.id, p.celda[s])) else {
                continue;
            };
            let z1 = basamento(&d, c);
            if z1 >= 0.0 {
                continue;
            }
            let esperado = 1000.0
                * (isostasia::hundimiento_mckenzie(&d.cfg, beta, c.t_rot_myr)
                    - isostasia::hundimiento_mckenzie(&d.cfg, beta, t0));
            let r = (z0 - z1) / esperado;
            peor = peor.max((r - 1.0).abs());
            suma += r;
            n += 1;
        }
    }
    assert!(n >= 10, "siguen {n} muestras estiradas");
    let media = suma / n as f64;
    println!(
        "hundimiento frente a McKenzie: {media:.4} de media, peor desvío {peor:.4} ({n} muestras)"
    );
    assert!(
        (media - 1.0).abs() <= 0.15 && peor <= 0.15,
        "el hundimiento sigue a McKenzie: {media:.3} de media, peor desvío {peor:.3} ({n} muestras)"
    );
}

// --- Puntos calientes (nota_puntos_calientes.md §10) -----------------------------------------------

/// Un planeta todo oceánico (60 Myr) que gira hacia el este a 5 cm/año alrededor del eje z, con un
/// punto caliente en el ecuador a 0° de longitud (prueba de la traza y del abombamiento).
fn oceano_sobre_un_punto_caliente() -> Deriva {
    let cfg = TectonicaCfg {
        n_hs: 0,
        lambda_esp: 0.0,
        ..TectonicaCfg::default()
    };
    let malla = Malla::new(48, 2, 6371.0);
    let mut d = Deriva::desde(&cfg, 5, malla, 0.0, &[1], |_| {
        (0, corteza(false, 7.0, 60.0))
    });
    d.arrancar_con(&[[0.0, 0.0, 50.0 / 6371.0]]);
    d.poner_punto_caliente([1.0, 0.0, 0.0], 1.0, 1200.0);
    d
}

#[test]
fn un_punto_caliente_deja_una_traza_recta_mas_vieja_cuanto_mas_lejos() {
    // §10, prueba 1: bajo una placa que se mueve, la traza es una cadena recta detrás del punto
    // caliente, tan larga como lo que avanzó la placa; cada trozo pasó por encima en su momento.
    let mut d = oceano_sobre_un_punto_caliente();
    let pasos = 20;
    for _ in 0..pasos {
        d.paso_cinematico();
    }
    let lado = d.malla.lado_km;
    let (mut n, mut lon_max, mut lat_max) = (0, 0.0f64, 0.0f64);
    for k in 0..d.malla.num {
        let Some(c) = d.corteza(k) else { continue };
        if c.h - 7.0 < 1e-6 && c.hc <= 0.0 {
            continue;
        }
        let (la, lo) = lat_lon(d.malla.dir[k]);
        let (la, lo) = (la.to_degrees() * 111.2, lo.to_degrees() * 111.2);
        n += 1;
        lat_max = lat_max.max(la.abs());
        lon_max = lon_max.max(lo);
        assert!(
            lo > -(d.cfg.r_traza_km + 2.0 * lado),
            "la traza queda detrás (al oeste no): {lo:.0} km"
        );
    }
    // El volumen, por las muestras de la placa (cada una con el área de su celda).
    let volumen: f64 = d
        .placas
        .iter()
        .filter(|p| p.viva)
        .flat_map(|p| p.cort.iter().zip(&p.celda))
        .map(|(c, &l)| (c.h - 7.0) * d.malla.area[l as usize])
        .sum();
    assert!(n > 5, "hay traza: {n} celdas");
    assert!(
        lat_max <= d.cfg.r_traza_km + 1.5 * lado,
        "es recta, a lo largo del ecuador: hasta {lat_max:.0} km de él"
    );
    let avance = 50.0 * pasos as f64;
    assert!(
        (lon_max - avance).abs() <= d.cfg.r_traza_km + 2.0 * lado,
        "tan larga como lo que avanzó la placa ({avance} km): {lon_max:.0} km"
    );
    let esperado = d.cfg.q_traza_km3_myr * pasos as f64;
    assert!(
        (volumen / esperado - 1.0).abs() < 1e-9,
        "conserva el caudal: {volumen:.6e} frente a {esperado:.6e} km³"
    );
}

#[test]
fn el_abombamiento_no_viaja_con_la_placa() {
    // §10, prueba 2: el abombamiento sigue al punto caliente (fijo en el marco del manto).
    let mut d = oceano_sobre_un_punto_caliente();
    let k0 = d.malla.localizar([1.0, 0.0, 0.0]);
    let campo = d.abombamiento_m.clone();
    assert!(
        campo[k0] > 900.0,
        "sube ~1 km sobre el punto caliente: {:.0} m",
        campo[k0]
    );
    let sube = |d: &Deriva, k: usize| {
        let c = d.corteza(k).expect("corteza");
        d.alturas()[k] - abm_terreno::tectonica::isostasia::altura(&d.cfg, c)
    };
    let antes = sube(&d, k0);
    for _ in 0..20 {
        d.paso_cinematico();
    }
    assert_eq!(d.abombamiento_m, campo, "el campo no se mueve con la placa");
    assert!(
        (sube(&d, k0) - antes).abs() < 1.0,
        "sigue subiendo lo mismo el fondo que pasa por encima"
    );
    // Lejos (a más de W/2), nada.
    let lejos = d.malla.localizar([0.0, 1.0, 0.0]);
    assert_eq!(d.abombamiento_m[lejos], 0.0);
}

#[test]
fn una_meseta_no_se_hunde_en_la_fosa_y_se_acreciona_un_90_por_ciento() {
    // §10, prueba 3: el escenario India–Asia con una meseta oceánica (22 km de corteza, 15 de ella
    // flotante) en lugar de la India: llega a la fosa de Asia y se acreciona con η_arco.
    use abm_terreno::tectonica::deriva::{Corteza, via};
    let cfg = TectonicaCfg {
        n_hs: 0,
        ..cfg_india_asia()
    };
    let mut d = choque_india_asia(&cfg, 1, 48);
    let i_india = d.dueno[d.malla.localizar([
        7.5f64.to_radians().cos() * 80f64.to_radians().cos(),
        7.5f64.to_radians().cos() * 80f64.to_radians().sin(),
        7.5f64.to_radians().sin(),
    ])] as usize;
    let meseta = |c: &Corteza| -> Corteza {
        let mut m = Corteza {
            cont: false,
            h: 22.0,
            hc: 15.0,
            arco: 1.0,
            edad_oc: 80.0,
            ..*c
        };
        m.proc.vias = [0.0; via::N];
        m.proc.vias[via::INTRAPLACA] = 15.0;
        m
    };
    let mut v0 = 0.0;
    let p = &mut d.placas[i_india];
    for s in 0..p.cort.len() {
        if p.cort[s].cont {
            p.cort[s] = meseta(&p.cort[s]);
            v0 += 15.0 * d.malla.area[p.celda[s] as usize];
        }
    }
    let c0 = d.cuentas.mesetas_acrecionadas_km3;
    for _ in 0..40 {
        d.dar_paso();
    }
    let p = &d.placas[i_india];
    let resto: f64 = p
        .cort
        .iter()
        .zip(&p.celda)
        .filter(|(c, _)| c.proc.vias[via::INTRAPLACA] > 0.0)
        .map(|(c, &l)| c.hc * d.malla.area[l as usize])
        .sum();
    let consumido = v0 - resto;
    let acrecionado = d.cuentas.mesetas_acrecionadas_km3 - c0;
    assert!(
        consumido > 0.2 * v0,
        "la meseta llega a la fosa: {:.2} consumida",
        consumido / v0
    );
    let f = acrecionado / consumido;
    assert!(
        (f - cfg.eta_arco).abs() < 0.03,
        "se acreciona como un arco: {f:.3} (η_arco {})",
        cfg.eta_arco
    );
}

#[test]
fn el_magmatismo_intraplaca_entra_en_el_balance_y_es_determinista() {
    // §10, pruebas 4 y 5: con los puntos calientes, el balance de la corteza sigue cuadrando paso a
    // paso con F_int como fuente, y la misma semilla da los mismos puntos calientes y erupciones.
    let cfg = TectonicaCfg {
        lip_cada_myr: 5.0,
        t_hs_myr: 20.0,
        ..cfg_rapida()
    };
    let correr = || {
        let mut d = Deriva::nueva(&cfg, 11, Malla::new(24, 1, 6371.0));
        for _ in 0..40 {
            d.dar_paso();
        }
        d
    };
    let (a, b) = (correr(), correr());
    assert!(a.cuentas.intraplaca_km3 > 0.0, "hay magmatismo intraplaca");
    assert!(
        a.eventos.iter().any(|e| e.tipo == "gran_erupcion"),
        "hay grandes erupciones"
    );
    for m in &a.metricas {
        assert!(
            m.error_balance_rel < 1e-9,
            "balance del paso {}: {:.2e}",
            m.myr,
            m.error_balance_rel
        );
    }
    let suma = a.cuentas.f_int_mesetas_km3 + a.cuentas.f_int_cfb_km3;
    assert!(
        a.cuentas.intraplaca_km3 >= suma - 1e-6,
        "las mesetas y los basaltos entran enteros"
    );
    let ev = |d: &Deriva| -> Vec<(String, u64, i64, i64)> {
        d.eventos
            .iter()
            .filter(|e| e.tipo.contains("caliente") || e.tipo == "gran_erupcion")
            .map(|e| {
                (
                    e.tipo.clone(),
                    e.myr as u64,
                    (e.lat_deg * 1e6) as i64,
                    (e.area_km2) as i64,
                )
            })
            .collect()
    };
    assert_eq!(ev(&a), ev(&b), "mismos puntos calientes y erupciones");
    assert_eq!(a.cuentas.intraplaca_km3, b.cuentas.intraplaca_km3);
}

#[test]
#[ignore = "exploratoria: la India de India–Asia (antes del choque, con la losa colgando y tras romperse) con varios ε_suc (EPS=0,0.1,0.3,1)"]
fn diagnostico_succion_india() {
    let eps: Vec<f64> = std::env::var("EPS")
        .unwrap_or_else(|_| "0,0.1,0.3,1".into())
        .split(',')
        .filter_map(|x| x.trim().parse().ok())
        .collect();
    let base = TectonicaCfg::default();
    let t_rot = base.t_rot_joven_myr + (base.t_rot_viejo_myr - base.t_rot_joven_myr) * 0.8;
    std::thread::scope(|sc| {
        let hilos: Vec<_> = eps
            .iter()
            .map(|&e| {
                sc.spawn(move || {
                    let cfg = TectonicaCfg {
                        eps_suc: e,
                        ..cfg_india_asia()
                    };
                    let r = india_asia_con(&cfg, 1, 24, 50);
                    let antes = r
                        .iter()
                        .take(6)
                        .map(|x| x.0)
                        .fold(f64::NEG_INFINITY, f64::max);
                    let i_choque = r.iter().position(|x| x.1 > 0.0).unwrap_or(0);
                    let i_rotura = (i_choque + t_rot.round() as usize).min(r.len() - 5);
                    let media = |v: &[(f64, f64, f64)]| {
                        v.iter().map(|x| x.0).sum::<f64>() / v.len().max(1) as f64
                    };
                    let durante = media(&r[(i_choque + 4).min(i_rotura)..i_rotura]);
                    let despues = media(&r[i_rotura + 4..]);
                    (e, antes, durante, despues)
                })
            })
            .collect();
        for h in hilos {
            let (e, a, d, p) = h.join().expect("hilo");
            println!(
                "ε_suc {e}: antes {a:.1}, con la losa colgando {d:.1}, tras romperse {p:.1} cm/año (cocientes {:.2} y {:.2})",
                d / a,
                p / a
            );
        }
    });
}

// --- Succión de las losas y soldadura (nota_succion_losas.md §10) ----------------------------------

#[test]
fn una_fosa_con_losa_profunda_arrastra_a_la_placa_de_arriba_hacia_ella() {
    // §10, prueba 1: con la losa del Tetis ya en el manto inferior (3·L_losa), la succión empuja a
    // Asia, la placa de arriba, hacia su fosa: la fosa retrocede. Sin succión, mucho menos.
    let retroceso = |eps: f64| {
        let cfg = TectonicaCfg {
            eps_suc: eps,
            ..cfg_india_asia()
        };
        let mut d = choque_india_asia(&cfg, 1, 48);
        let l = 3.0 * d.cfg.l_losa_km;
        d.activar_fosas(l);
        d.dar_paso();
        let m = d.metricas.last().expect("métrica");
        (m.fosa_retroceso_cm_anio, m.fosa_retrocede_frac)
    };
    let (sin, _) = retroceso(0.0);
    let (con, frac) = retroceso(1.0);
    assert!(
        con > sin + 0.3 && frac > 0.5,
        "la placa de arriba va hacia su fosa: retroceso {con:.2} cm/año ({frac:.2} de la fosa) con succión, {sin:.2} sin ella"
    );
}

#[test]
fn un_resto_de_losa_atrae_a_las_placas_cercanas_hasta_que_toca_fondo() {
    // §10, prueba 2: sin fosa, un resto de losa junto al borde entre dos placas tira de la base de
    // la placa de al lado mientras baja (t_s < τ_hund) y después deja de tirar.
    // La succión está apagada por defecto (decisión del responsable): aquí, encendida con su valor
    // calibrado.
    let cfg = TectonicaCfg {
        n_hs: 0,
        eps_suc: 0.05,
        ..TectonicaCfg::default()
    };
    let malla = Malla::new(48, 1, 6371.0);
    let dirs = malla.dir.clone();
    let mut d = Deriva::desde(&cfg, 3, malla, 0.0, &[1, 2], |k| {
        if dirs[k][0] > 0.0 {
            (0, corteza(false, 7.0, 80.0))
        } else {
            (1, corteza(false, 7.0, 80.0))
        }
    });
    // Un resto de ~1 000 km a lo largo del borde, 300 km dentro de la placa 1.
    let th: f64 = 300.0 / 6371.0;
    let puntos: Vec<([f64; 3], f64, f64)> = (-10..=10)
        .map(|i| {
            let z = (i as f64 * 50.0 / 6371.0).sin();
            let x = -th.sin() * (1.0 - z * z).sqrt();
            let y = th.cos() * (1.0 - z * z).sqrt();
            ([x, y, z], 1.0, 50.0)
        })
        .collect();
    d.poner_resto_de_losa(puntos);
    let par = d.pares_de_succion();
    let modulo = |v: [f64; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    assert!(modulo(par[0]) > 0.0, "tira de la placa de al lado");
    for _ in 0..183 {
        d.envejecer_restos();
    }
    assert_eq!(d.restos.len(), 1, "a los 183 Myr aún tira");
    d.envejecer_restos();
    assert!(d.restos.is_empty(), "a los τ_hund (~183 Myr) ya no");
    let par = d.pares_de_succion();
    assert_eq!(modulo(par[0]) + modulo(par[1]), 0.0, "ya no tira");
}

#[test]
fn un_choque_agotado_suelda_sus_placas_y_lo_conserva_todo() {
    // §10, prueba 3: dos continentes en contacto que convergen a 0,2 cm/año (menos que v_sold), sin
    // losa entre ellos: a los τ_sold se sueldan en una placa, sin perder corteza. A 3 cm/año, no.
    let soldar = |v_cm_anio: f64| {
        let cfg = TectonicaCfg {
            n_hs: 0,
            eps_suc: 0.0,
            ..TectonicaCfg::default()
        };
        let malla = Malla::new(48, 1, 6371.0);
        let dirs = malla.dir.clone();
        let mut d = Deriva::desde(&cfg, 3, malla, 0.0, &[1, 2], |k| {
            if dirs[k][1] > 0.0 {
                (0, corteza(true, 35.0, 0.0))
            } else {
                (1, corteza(true, 35.0, 0.0))
            }
        });
        // La placa 0 gira alrededor del eje x: cerca de los polos va hacia la placa 1.
        d.arrancar_con(&[[v_cm_anio * 10.0 / 6371.0, 0.0, 0.0], [0.0; 3]]);
        let mut paso_soldado = None;
        for paso in 1..=40u32 {
            d.paso_cinematico();
            let v0 = d.inventario_continental();
            if d.comprobar_soldaduras() {
                let v1 = d.inventario_continental();
                assert!(
                    (v1 - v0).abs() <= 1e-9 * v0,
                    "la soldadura conserva la corteza"
                );
                paso_soldado = Some(paso);
                break;
            }
        }
        (paso_soldado, d.placas.iter().filter(|p| p.viva).count())
    };
    let (lento, vivas) = soldar(0.2);
    let t_sold = TectonicaCfg::default().t_sutura_myr;
    assert!(
        lento.is_some_and(|p| p as f64 >= t_sold),
        "a 0,2 cm/año se sueldan tras τ_sold ({t_sold} Myr): en el paso {lento:?}"
    );
    assert_eq!(vivas, 1, "queda una sola placa");
    let (rapido, _) = soldar(3.0);
    assert_eq!(rapido, None, "a 3 cm/año no se sueldan");
}
