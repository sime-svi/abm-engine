//! Pruebas del modo Tierra (`docs/design/terreno/modo-tierra-diseno.md`).
//!
//! Las primeras usan unos datos inventados y pequeños (no necesitan las fuentes): el clima del día
//! (interpolación entre meses, lluvia repartida, años reales y anomalías) y la conservación entre
//! niveles. Las marcadas `#[ignore]` necesitan las fuentes de la Tierra en `output/referencias/`
//! (`herramientas/tierra/descargar.sh`): la lluvia del Sahel en 1984 y el diagnóstico de la
//! variabilidad de un año a otro por regiones (§10 de la nota).

use std::path::PathBuf;
use std::sync::Arc;

use abm_terreno::TerrenoConfig;
use abm_terreno::config::Modo;
use abm_terreno::geometry::{center_dir, face_n, lat_lon, split_key};
use abm_terreno::tierra::carga::{areas, desde_nivel_fino};
use abm_terreno::tierra::clima::{Estado, secuencia_anios};
use abm_terreno::tierra::{Cru, DatosTierra, Nivel};

/// Config pequeña del modo Tierra: 4 celdas por arista en el nivel 0 y datos en el nivel 2.
fn cfg_pequena() -> TerrenoConfig {
    let mut cfg = TerrenoConfig::default();
    cfg.mundo.modo = Modo::Tierra;
    cfg.planet.n0 = 4;
    cfg.lod.l_min = 0;
    cfg.lod.l_max = 2;
    cfg.tierra.nivel_carga = 2;
    cfg.run.genesis_cache = String::new();
    cfg.run.total_ticks = 365;
    cfg
}

/// Datos inventados: tierra en la mitad de la cara +X, hielo cerca del polo norte y dos celdas de CRU
/// con los años 1981–2010 a 15 °C y 100 mm cada mes, salvo julio de 2001 en la celda 0: 17 °C y 40 mm.
fn datos_inventados(cfg: &TerrenoConfig) -> Arc<DatosTierra> {
    let l = cfg.tierra.nivel_carga;
    let n = face_n(cfg.planet.n0, l);
    let m = 6 * (n as usize) * (n as usize);
    let (mut z, mut hielo, mut t, mut p, mut cru_de_celda) = (
        vec![0.0; m],
        vec![0.0; m],
        vec![0.0f32; 12 * m],
        vec![0.0f32; 12 * m],
        vec![u32::MAX; m],
    );
    for c in 0..6u8 {
        for j in 0..n {
            for i in 0..n {
                let k = ((c as usize * n as usize + j as usize) * n as usize) + i as usize;
                let d = center_dir(c, i, j, n);
                let (lat, _) = lat_lon(d);
                let tierra = d[0] > 0.2;
                z[k] = if tierra {
                    300.0 + 1500.0 * d[2].abs()
                } else {
                    -3000.0
                };
                hielo[k] = if tierra && d[2] > 0.85 { 1.0 } else { 0.0 };
                for mes in 0..12 {
                    t[k * 12 + mes] = (25.0 - 30.0 * lat.abs() + mes as f64) as f32;
                    p[k * 12 + mes] = (50.0 + 10.0 * mes as f64 + 40.0 * d[1].abs()) as f32;
                }
                if tierra {
                    cru_de_celda[k] = if d[2] < 0.5 { 0 } else { 1 };
                }
            }
        }
    }
    let (n_anios, nc) = (30usize, 2usize);
    let mut tmp = vec![150i16; n_anios * 12 * nc];
    let mut pre = vec![1000u16; n_anios * 12 * nc];
    let g = 20 * 12 + 6; // julio de 2001
    tmp[g * nc] = 170;
    pre[g * nc] = 400;
    let cru = Cru {
        anio0: 1981,
        n_anios,
        lat: vec![10.0, 40.0],
        lon: vec![0.0, 0.0],
        tmp,
        pre,
        pre_recortados: 0,
        sin_estaciones: vec![[0.0, 0.0]; n_anios],
    };
    let fino = Nivel {
        n,
        area: areas(n, cfg.planet.r_km * 1000.0),
        z,
        hielo,
        t,
        p,
        cobertura: vec![1.0; m],
    };
    Arc::new(desde_nivel_fino(cfg, fino, cru, cru_de_celda, 0))
}

/// Clima esperado de una celda del mundo en un mes: el de su celda del nivel de carga, o la media por
/// área de sus descendientes en ese nivel.
fn esperado(e: &Estado, d: &DatosTierra, clave: u64, anio: i32, mes: usize) -> (f64, f64) {
    let (c, l, i, j) = split_key(clave);
    let lc = d.nivel_carga;
    let fino = &d.niveles[lc as usize];
    if l >= lc {
        let s = l - lc;
        return e.mes_en_celda(anio, mes, fino.indice(c, i >> s, j >> s));
    }
    let s = lc - l;
    let (mut a, mut tt, mut pp) = (0.0, 0.0, 0.0);
    for jj in (j << s)..((j + 1) << s) {
        for ii in (i << s)..((i + 1) << s) {
            let k = fino.indice(c, ii, jj);
            let (t, p) = e.mes_en_celda(anio, mes, k);
            a += fino.area[k];
            tt += t * fino.area[k];
            pp += p * fino.area[k];
        }
    }
    (tt / a, pp / a)
}

#[test]
fn el_clima_del_dia_sale_de_los_meses_y_se_conserva_entre_niveles() {
    let mut cfg = cfg_pequena();
    cfg.tierra.anio_forzado = 2001;
    let datos = datos_inventados(&cfg);
    let (mut world, _) = abm_terreno::worldgen::generate_tierra(&cfg, Arc::clone(&datos));
    let niveles: std::collections::BTreeSet<u8> = world.topo.level.iter().copied().collect();
    assert!(niveles.len() >= 2, "la malla mezcla niveles: {niveles:?}");
    let mut e = Estado::new(Arc::clone(&datos), &cfg, 1).expect("estado del clima");

    // Mitad de julio (día 196, mes de 31 días): exactamente el valor de julio.
    e.clima(&mut world, 196);
    let mut con_anomalia = 0;
    for s in 0..world.topo.num_cells {
        let (t, p) = esperado(&e, &datos, world.topo.keys[s], 2001, 6);
        assert!(
            (world.temp[s] - t).abs() < 1e-4,
            "celda {s}: {} frente a {t}",
            world.temp[s]
        );
        assert!((world.precip[s] - p / 31.0).abs() < 1e-6, "celda {s}");
    }
    // La anomalía de julio de 2001 llega a las celdas de la celda 0 de CRU, respecto a la media de
    // 1981–2010: +1,93 °C y 40/98 de la lluvia.
    let fino = &datos.niveles[2];
    for k in 0..fino.len() {
        let (t, p) = e.mes_en_celda(2001, 6, k);
        match datos.cru_de_celda[k] {
            0 => {
                con_anomalia += 1;
                let dt = 17.0 - (29.0 * 15.0 + 17.0) / 30.0;
                let r = 40.0 / ((29.0 * 100.0 + 40.0) / 30.0);
                assert!((t - (fino.t[k * 12 + 6] as f64 + dt)).abs() < 1e-5);
                assert!((p - fino.p[k * 12 + 6] as f64 * r).abs() < 1e-4);
            }
            _ => {
                assert!((t - fino.t[k * 12 + 6] as f64).abs() < 1e-5);
                assert!((p - fino.p[k * 12 + 6] as f64).abs() < 1e-4);
            }
        }
    }
    assert!(con_anomalia > 0);

    // 20 de julio: entre las mitades de julio (196,5) y agosto (227,5); la lluvia, la de julio.
    e.clima(&mut world, 200);
    let w = 4.0 / 31.0;
    for s in 0..world.topo.num_cells {
        let (t7, p7) = esperado(&e, &datos, world.topo.keys[s], 2001, 6);
        let (t8, _) = esperado(&e, &datos, world.topo.keys[s], 2001, 7);
        let t = (1.0 - w) * t7 + w * t8;
        assert!((world.temp[s] - t).abs() < 1e-4, "celda {s}");
        assert!((world.precip[s] - p7 / 31.0).abs() < 1e-6);
    }
    // 6 de enero: entre la mitad de diciembre del año anterior y la de enero.
    e.clima(&mut world, 5);
    let w = 21.0 / 31.0;
    for s in 0..world.topo.num_cells {
        let (t12, _) = esperado(&e, &datos, world.topo.keys[s], 2001, 11);
        let (t1, p1) = esperado(&e, &datos, world.topo.keys[s], 2001, 0);
        assert!((world.temp[s] - ((1.0 - w) * t12 + w * t1)).abs() < 1e-4);
        assert!((world.precip[s] - p1 / 31.0).abs() < 1e-6);
    }
}

#[test]
fn la_altura_y_el_hielo_son_los_de_los_datos_y_el_relieve_no_se_toca() {
    let cfg = cfg_pequena();
    let datos = datos_inventados(&cfg);
    let (world, _) = abm_terreno::worldgen::generate_tierra(&cfg, Arc::clone(&datos));
    for s in 0..world.topo.num_cells {
        let (lv, k) = datos.celda_de_clave(world.topo.keys[s]);
        assert_eq!(world.h[s].to_bits(), datos.niveles[lv].z[k].to_bits());
        assert_eq!(
            world.hielo[s].to_bits(),
            datos.niveles[lv].hielo[k].to_bits()
        );
        assert_eq!(world.e[s], 0.0, "sin erosión");
    }
    // Un año de los sistemas del tick con el clima de los datos (el mismo camino que la corrida, que
    // carga los datos de las fuentes): el agua se conserva y el hielo no tiene vegetación.
    let mut c = cfg.clone();
    c.tierra.anio_forzado = 2001;
    let (mut w2, _) = abm_terreno::worldgen::generate_tierra(&c, Arc::clone(&datos));
    let mut sc = abm_terreno::systems::Scratch::new(w2.topo.num_cells);
    sc.tierra = Some(Estado::new(Arc::clone(&datos), &c, 1).expect("estado"));
    for t in 0..365 {
        let antes = abm_terreno::systems::total_store_vol(&w2);
        let lod = abm_terreno::lod::LodReport::default();
        let acct = abm_terreno::systems::step(&mut w2, &c, &mut sc, t, antes, &lod);
        assert!(acct.residual().abs() <= 1e-9 * acct.precip_vol.max(1.0));
    }
    let mut con_hielo = 0;
    for s in 0..w2.topo.num_cells {
        if w2.hielo[s] >= 1.0 && !w2.es_oceano(s) {
            con_hielo += 1;
            assert_eq!(w2.veg[s], 0.0, "celda {s} con hielo y vegetación");
            assert_eq!(w2.biome[s], abm_terreno::world::Biome::Hielo);
        }
    }
    assert!(con_hielo > 0, "la prueba necesita tierra con hielo");
}

#[test]
fn misma_semilla_misma_secuencia_de_anios_reales() {
    let mut cfg = cfg_pequena();
    let datos = datos_inventados(&cfg);
    cfg.tierra.l_bloque = 5;
    cfg.run.seed = 7;
    let a = Estado::new(Arc::clone(&datos), &cfg, 50).expect("estado");
    let b = Estado::new(Arc::clone(&datos), &cfg, 50).expect("estado");
    let sa: Vec<i32> = (0..50).map(|y| a.anio_real(y)).collect();
    let sb: Vec<i32> = (0..50).map(|y| b.anio_real(y)).collect();
    assert_eq!(sa, sb);
    assert!(sa.iter().all(|&x| (1981..=2010).contains(&x)));
    cfg.run.seed = 8;
    assert_ne!(secuencia_anios(&cfg, 1981, 2010, 50)[1..51], sa[..]);
    // Otro periodo base que el de CHELSA se corrige con CRU TS: aquí, 1990–1999 (todo a 15 °C y
    // 100 mm, igual que 1981–2010 salvo julio de 2001): la corrección es nula fuera de julio.
    cfg.tierra.periodo_base = [1990, 1999];
    let c = Estado::new(Arc::clone(&datos), &cfg, 5).expect("estado");
    let fino = &datos.niveles[2];
    let k = (0..fino.len())
        .find(|&k| datos.cru_de_celda[k] == 0)
        .expect("una celda de la celda 0 de CRU");
    let (t1, _) = c.mes_en_celda(1995, 0, k);
    assert!((t1 - fino.t[k * 12] as f64).abs() < 1e-5);
    // En julio, la media de 1981–2010 incluye el julio caliente de 2001: la de 1990–1999 es 0,0667 °C
    // más fría.
    let (t7, _) = c.mes_en_celda(1995, 6, k);
    let corr = 15.0 - (29.0 * 15.0 + 17.0) / 30.0;
    assert!((t7 - (fino.t[k * 12 + 6] as f64 + corr)).abs() < 1e-5);
}

// --- Con las fuentes de la Tierra -----------------------------------------------------------------

/// Config del modo Tierra con las rutas de las fuentes y la caché del repositorio.
fn cfg_tierra() -> TerrenoConfig {
    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut cfg = TerrenoConfig::load(&raiz.join("config/terreno.toml").to_string_lossy())
        .expect("config/terreno.toml");
    cfg.mundo.modo = Modo::Tierra;
    let r = |p: &str| raiz.join(p).to_string_lossy().into_owned();
    let t = &mut cfg.tierra;
    t.etopo_superficie = r(&t.etopo_superficie);
    t.etopo_roca_sur = r(&t.etopo_roca_sur);
    t.etopo_roca_norte = r(&t.etopo_roca_norte);
    t.chelsa = r(&t.chelsa);
    t.cru_tmp = r(&t.cru_tmp);
    t.cru_pre = r(&t.cru_pre);
    cfg.run.genesis_cache = r(&cfg.run.genesis_cache);
    cfg
}

/// Regiones del diagnóstico de la variabilidad: `(nombre, lat_min, lat_max, lon_min, lon_max)`.
const REGIONES: [(&str, f64, f64, f64, f64); 13] = [
    ("sahel", 10.0, 20.0, -20.0, 40.0),
    ("africa_oriental", -5.0, 10.0, 30.0, 50.0),
    ("africa_austral", -35.0, -15.0, 10.0, 40.0),
    ("mediterraneo", 30.0, 45.0, -10.0, 40.0),
    ("europa_centro_norte", 45.0, 70.0, -10.0, 40.0),
    ("grandes_llanuras", 30.0, 50.0, -105.0, -90.0),
    ("suroeste_eeuu_mexico", 20.0, 40.0, -120.0, -100.0),
    ("amazonia", -15.0, 5.0, -75.0, -50.0),
    ("nordeste_brasil", -15.0, -2.0, -45.0, -35.0),
    ("india", 8.0, 30.0, 68.0, 90.0),
    ("china_oriental", 20.0, 40.0, 100.0, 122.0),
    ("australia", -40.0, -10.0, 113.0, 154.0),
    ("siberia", 50.0, 70.0, 60.0, 140.0),
];

fn dentro(r: &(&str, f64, f64, f64, f64), lat: f64, lon: f64) -> bool {
    lat >= r.1 && lat < r.2 && lon >= r.3 && lon < r.4
}

/// Celdas de tierra (sin hielo y con celda de CRU) del nivel de carga en una región.
fn celdas_region(d: &DatosTierra, r: &(&str, f64, f64, f64, f64)) -> Vec<usize> {
    let lc = d.nivel_carga as usize;
    let fino = &d.niveles[lc];
    let n = fino.n;
    let mut v = Vec::new();
    for c in 0..6u8 {
        for j in 0..n {
            for i in 0..n {
                let k = fino.indice(c, i, j);
                let (lat, lon) = lat_lon(center_dir(c, i, j, n));
                if fino.z[k] >= 0.0
                    && fino.hielo[k] < 0.5
                    && d.cru_de_celda[k] != u32::MAX
                    && dentro(r, lat.to_degrees(), lon.to_degrees())
                {
                    v.push(k);
                }
            }
        }
    }
    v
}

/// Lluvia anual de CRU TS en una región (media de sus celdas ponderada por `cos φ`).
fn lluvia_cru(cru: &Cru, r: &(&str, f64, f64, f64, f64), a: i32) -> f64 {
    let (mut s, mut w) = (0.0, 0.0);
    for c in 0..cru.n_celdas() {
        let (lat, lon) = (cru.lat[c] as f64, cru.lon[c] as f64);
        if dentro(r, lat, lon) {
            let p: f64 = (0..12).map(|m| cru.p(a, m, c)).sum();
            let wc = lat.to_radians().cos();
            s += p * wc;
            w += wc;
        }
    }
    s / w
}

/// Media, desviación típica, coeficiente de variación, autocorrelación de un año al siguiente y
/// fracción de años secos (por debajo del 75 % de la media) de una serie.
fn estadisticas(x: &[f64]) -> [f64; 5] {
    let n = x.len() as f64;
    let media = x.iter().sum::<f64>() / n;
    let var = x.iter().map(|v| (v - media).powi(2)).sum::<f64>() / (n - 1.0);
    let cov = x
        .windows(2)
        .map(|w| (w[0] - media) * (w[1] - media))
        .sum::<f64>()
        / (n - 1.0);
    let secos = x.iter().filter(|&&v| v < 0.75 * media).count() as f64 / n;
    let sd = var.sqrt();
    [media, sd, sd / media, cov / var, secos]
}

#[test]
#[ignore = "necesita las fuentes de la Tierra (herramientas/tierra/descargar.sh); diagnóstico"]
fn diagnostico_tierra_variabilidad_por_regiones() {
    let mut cfg = cfg_tierra();
    let datos = abm_terreno::tierra::cargar(&cfg).expect("datos de la Tierra");
    let n_sim = 20_000usize;
    cfg.run.seed = 42;
    // `L_BLOQUE` en el entorno prueba otra longitud de bloque (la salida lleva la longitud en el nombre).
    if let Some(l) = std::env::var("L_BLOQUE").ok().and_then(|v| v.parse().ok()) {
        cfg.tierra.l_bloque = l;
    }
    let e = Estado::new(Arc::clone(&datos), &cfg, n_sim).expect("estado");
    let cru = &datos.cru;
    let (a0, a1) = (cru.anio0, cru.anio_final());
    let mut out = serde_json::Map::new();
    println!(
        "{:<22} {:>9} {:>7} {:>7} {:>7} | {:>7} {:>7} {:>7} | {:>7} {:>7} {:>7}",
        "región",
        "P CRU",
        "CV CRU",
        "CV mod",
        "CV ord",
        "ac1 CRU",
        "ac1 mod",
        "ac1 ord",
        "sec CRU",
        "sec mod",
        "sec ord"
    );
    for r in &REGIONES {
        let celdas = celdas_region(&datos, r);
        // Lluvia de la malla de cada año real (una vez), la serie remuestreada y la serie en orden.
        let por_anio: Vec<f64> = (a0..=a1).map(|a| e.lluvia_anual(a, &celdas)).collect();
        let modelo: Vec<f64> = (0..n_sim as i64)
            .map(|y| por_anio[(e.anio_real(y) - a0) as usize])
            .collect();
        let real: Vec<f64> = (a0..=a1).map(|a| lluvia_cru(cru, r, a)).collect();
        let (sc, sm, so) = (
            estadisticas(&real),
            estadisticas(&modelo),
            estadisticas(&por_anio),
        );
        println!(
            "{:<22} {:>9.0} {:>7.3} {:>7.3} {:>7.3} | {:>7.3} {:>7.3} {:>7.3} | {:>7.3} {:>7.3} {:>7.3}",
            r.0, sc[0], sc[2], sm[2], so[2], sc[3], sm[3], so[3], sc[4], sm[4], so[4]
        );
        let reg = serde_json::json!({
            "celdas": celdas.len(),
            "cru": {"media_mm": sc[0], "sd_mm": sc[1], "cv": sc[2], "ac1": sc[3], "secos_frac": sc[4]},
            "modelo_remuestreado": {"media_mm": sm[0], "sd_mm": sm[1], "cv": sm[2], "ac1": sm[3], "secos_frac": sm[4]},
            "modelo_en_orden": {"media_mm": so[0], "sd_mm": so[1], "cv": so[2], "ac1": so[3], "secos_frac": so[4]},
            "serie_cru_mm": real,
            "serie_malla_por_anio_mm": por_anio,
        });
        out.insert(r.0.to_string(), reg);
    }
    let doc = serde_json::json!({
        "anios_simulados": n_sim,
        "l_bloque": cfg.tierra.l_bloque,
        "semilla": cfg.run.seed,
        "cru_anios": [a0, a1],
        "regiones": out,
    });
    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../output/tierra_validacion");
    std::fs::create_dir_all(&raiz).expect("carpeta de salida");
    std::fs::write(
        raiz.join(format!("variabilidad_l{}.json", cfg.tierra.l_bloque)),
        serde_json::to_string_pretty(&doc).expect("json"),
    )
    .expect("escribir");
}

#[test]
#[ignore = "necesita las fuentes de la Tierra (herramientas/tierra/descargar.sh)"]
fn forzando_1984_la_lluvia_del_sahel_es_la_de_cru_ts() {
    let mut cfg = cfg_tierra();
    cfg.tierra.anio_forzado = 1984;
    cfg.run.total_ticks = 365;
    let (_, world) = abm_terreno::run(&cfg).expect("corrida");
    let d = world.statgen.tierra().expect("datos de la Tierra");
    let sahel = &REGIONES[0];
    // Lluvia del año simulado (suma de los 365 días) y de la climatología, sobre la tierra del Sahel.
    let (mut p, mut p_clim, mut a) = (0.0, 0.0, 0.0);
    for s in 0..world.topo.num_cells {
        let (lat, lon) = (
            world.topo.lat[s].to_degrees(),
            world.topo.lon[s].to_degrees(),
        );
        if world.es_oceano(s) || !dentro(sahel, lat, lon) {
            continue;
        }
        let ar = world.topo.area[s];
        p += world.precip_sum[s] * ar;
        p_clim += d.clima_anual(world.topo.keys[s]).1 * ar;
        a += ar;
    }
    let (p, p_clim) = (p / a, p_clim / a);
    let cru_1984 = lluvia_cru(&d.cru, sahel, 1984);
    let cru_base = (1981..=2010)
        .map(|y| lluvia_cru(&d.cru, sahel, y))
        .sum::<f64>()
        / 30.0;
    println!(
        "Sahel 1984: malla {p:.1} mm (climatología {p_clim:.1}, {:.3}); CRU TS {cru_1984:.1} mm (1981–2010 {cru_base:.1}, {:.3})",
        p / p_clim,
        cru_1984 / cru_base
    );
    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../output/tierra_validacion");
    std::fs::create_dir_all(&raiz).expect("carpeta de salida");
    let doc = serde_json::json!({
        "region": {"lat": [sahel.1, sahel.2], "lon": [sahel.3, sahel.4]},
        "malla_1984_mm": p, "malla_climatologia_mm": p_clim,
        "cru_1984_mm": cru_1984, "cru_1981_2010_mm": cru_base,
    });
    std::fs::write(
        raiz.join("sahel_1984.json"),
        serde_json::to_string_pretty(&doc).expect("json"),
    )
    .expect("escribir");
    assert!(
        (p / p_clim - cru_1984 / cru_base).abs() < 0.02,
        "la anomalía del Sahel en 1984 no es la de CRU TS"
    );
}
