//! Relieve tectónico (incremento 4): pruebas de la deriva de placas y del relieve que deja.
//!
//! Diseño: `docs/design/terreno/diseno_relieve_tectonico.md` §10 y `incremento-4-diseno.md`.
//! Las pruebas corren en mallas gruesas y derivas cortas para que quepan en `cargo test` (debug).

use abm_terreno::config::TectonicaCfg;
use abm_terreno::geometry::lat_lon;
use abm_terreno::tectonica::deriva::Deriva;
use abm_terreno::tectonica::malla::{Malla, NADA};

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
    abm_terreno::tectonica::deriva::Corteza {
        cont,
        h,
        sed: 0.0,
        edad_oc: edad,
        edad_orog: 0.0,
        dh_dt: 0.0,
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
fn india_asia(c_col: f64, nivel: u32, n0: u32, pasos: u64) -> Vec<(f64, f64, f64)> {
    let cfg = TectonicaCfg {
        c_col,
        a_min_frac: 0.0,
        tau_rift_myr: 1e12,
        n_min: 2,
        recalibrar_myr: 1e9, // c_b se calibra una vez, al principio
        ..TectonicaCfg::default()
    };
    let malla = Malla::new(n0, nivel, 6371.0);
    let dirs = malla.dir.clone();
    let geo = |d: [f64; 3]| {
        let (la, lo) = lat_lon(d);
        (la.to_degrees(), lo.to_degrees())
    };
    let india = |la: f64, lo: f64| (0.0..=15.0).contains(&la) && (70.0..=90.0).contains(&lo);
    let mut d = Deriva::desde(&cfg, 7, malla, 0.0, &[1, 2, 3], |k| {
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
fn sin_corteza_continental_todo_es_oceano_con_profundidad_por_edad() {
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
        assert!(
            !c.cont,
            "no puede haber corteza continental en 30 Myr sin cratones"
        );
        if c.h == cfg.h_oc_km {
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
