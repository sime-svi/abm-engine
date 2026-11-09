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
use abm_terreno::tierra::carga::{Hidrologia, areas, desde_nivel_fino};
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
        lago: vec![0.0; m],
        t,
        p,
        cobertura: vec![1.0; m],
    };
    let hid = Hidrologia::sin_red(&fino.z);
    Arc::new(desde_nivel_fino(cfg, fino, cru, cru_de_celda, hid, 0))
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
    let todos = vec![true; 30];
    let otra = secuencia_anios(&cfg, 1981, &todos, 50).expect("secuencia");
    assert_ne!(otra[1..51], sa[..]);
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
    t.rgi = r(&t.rgi);
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

/// Rachas de 2 o más años seguidos por debajo del 75 % de la media de la serie (adenda 1 §4.2).
fn rachas_secas(x: &[f64]) -> usize {
    let media = x.iter().sum::<f64>() / x.len() as f64;
    let (mut rachas, mut largo) = (0, 0);
    for &v in x.iter().chain(std::iter::once(&f64::INFINITY)) {
        if v < 0.75 * media {
            largo += 1;
        } else {
            if largo >= 2 {
                rachas += 1;
            }
            largo = 0;
        }
    }
    rachas
}

/// Rachas de 2 o más años seguidos en el quinto más seco de la serie (por debajo de su percentil 20):
/// la misma prueba con más sucesos que la del 75 % de la media, que casi nunca se alcanza.
fn rachas_quinto_seco(x: &[f64]) -> usize {
    let mut orden = x.to_vec();
    orden.sort_by(|a, b| a.total_cmp(b));
    let umbral = orden[(0.2 * (orden.len() - 1) as f64).round() as usize];
    let (mut rachas, mut largo) = (0, 0);
    for &v in x.iter().chain(std::iter::once(&f64::INFINITY)) {
        if v <= umbral {
            largo += 1;
        } else {
            if largo >= 2 {
                rachas += 1;
            }
            largo = 0;
        }
    }
    rachas
}

/// Probabilidad de Poisson a dos colas de ver `k` sucesos si se esperan `lambda`: `2·min(P(X ≤ k),
/// P(X ≥ k))`, como mucho 1. «Dentro del ruido» si es al menos 0,05.
fn poisson_dos_colas(k: usize, lambda: f64) -> f64 {
    let mut termino = (-lambda).exp();
    let mut acumulada = termino;
    for i in 1..=k {
        termino *= lambda / i as f64;
        acumulada += termino;
    }
    let menor_o_igual = acumulada;
    let mayor_o_igual = 1.0 - (acumulada - termino);
    (2.0 * menor_o_igual.min(mayor_o_igual)).min(1.0)
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
    // Los años del remuestreo (estaciones en al menos `cobertura_min` de la tierra): la referencia de
    // CRU TS se mide en esos mismos años.
    let validos = abm_terreno::tierra::clima::anios_con_estaciones(
        &cru.sin_estaciones,
        cfg.tierra.cobertura_min,
    );
    let periodo: Vec<i32> = (a0..=a1).filter(|a| validos[(a - a0) as usize]).collect();
    let mut out = serde_json::Map::new();
    println!(
        "años del remuestreo: {}–{} ({})",
        periodo[0],
        periodo[periodo.len() - 1],
        periodo.len()
    );
    println!(
        "{:<22} {:>6} {:>6} {:>6} | {:>6} {:>6} {:>6} | {:>6} {:>6} | {:>5} {:>6} {:>5} | {:>5} {:>6} {:>5}",
        "región",
        "CV CRU",
        "CV mod",
        "CV ord",
        "ac1 CRU",
        "ac1 mod",
        "ac1 ord",
        "sec CRU",
        "sec mod",
        "R CRU",
        "R esp",
        "p",
        "Q CRU",
        "Q esp",
        "p"
    );
    for r in &REGIONES {
        let celdas = celdas_region(&datos, r);
        // Lluvia de la malla de cada año real (una vez), la serie remuestreada y la serie en orden.
        let por_anio: Vec<f64> = (a0..=a1).map(|a| e.lluvia_anual(a, &celdas)).collect();
        let modelo: Vec<f64> = (0..n_sim as i64)
            .map(|y| por_anio[(e.anio_real(y) - a0) as usize])
            .collect();
        let real_todo: Vec<f64> = (a0..=a1).map(|a| lluvia_cru(cru, r, a)).collect();
        let real: Vec<f64> = periodo
            .iter()
            .map(|&a| real_todo[(a - a0) as usize])
            .collect();
        let en_orden: Vec<f64> = periodo
            .iter()
            .map(|&a| por_anio[(a - a0) as usize])
            .collect();
        let (sc, sm, so, st) = (
            estadisticas(&real),
            estadisticas(&modelo),
            estadisticas(&en_orden),
            estadisticas(&real_todo),
        );
        // Rachas de años secos: las de CRU en el periodo frente a las esperadas con el ritmo del
        // modelo en tantos años.
        let (r_cru, r_mod) = (rachas_secas(&real), rachas_secas(&modelo));
        let esperadas = r_mod as f64 * real.len() as f64 / n_sim as f64;
        let p = poisson_dos_colas(r_cru, esperadas);
        let (q_cru, q_mod) = (rachas_quinto_seco(&real), rachas_quinto_seco(&modelo));
        let q_esp = q_mod as f64 * real.len() as f64 / n_sim as f64;
        let q_p = poisson_dos_colas(q_cru, q_esp);
        println!(
            "{:<22} {:>6.3} {:>6.3} {:>6.3} | {:>6.3} {:>6.3} {:>6.3} | {:>6.3} {:>6.3} | {:>5} {:>6.2} {:>5.2} | {:>5} {:>6.2} {:>5.2}",
            r.0,
            sc[2],
            sm[2],
            so[2],
            sc[3],
            sm[3],
            so[3],
            sc[4],
            sm[4],
            r_cru,
            esperadas,
            p,
            q_cru,
            q_esp,
            q_p
        );
        let reg = serde_json::json!({
            "celdas": celdas.len(),
            "cru": {"media_mm": sc[0], "sd_mm": sc[1], "cv": sc[2], "ac1": sc[3], "secos_frac": sc[4], "rachas_secas": r_cru},
            "cru_1901_2024": {"media_mm": st[0], "sd_mm": st[1], "cv": st[2], "ac1": st[3], "secos_frac": st[4], "rachas_secas": rachas_secas(&real_todo)},
            "modelo_remuestreado": {"media_mm": sm[0], "sd_mm": sm[1], "cv": sm[2], "ac1": sm[3], "secos_frac": sm[4],
                                    "rachas_secas_por_100_anios": 100.0 * r_mod as f64 / n_sim as f64},
            "modelo_en_orden": {"media_mm": so[0], "sd_mm": so[1], "cv": so[2], "ac1": so[3], "secos_frac": so[4]},
            "rachas_secas_esperadas_en_el_periodo": esperadas,
            "rachas_secas_p_poisson": p,
            "rachas_quinto_seco_cru": q_cru,
            "rachas_quinto_seco_esperadas": q_esp,
            "rachas_quinto_seco_p_poisson": q_p,
            "serie_cru_mm": real_todo,
            "serie_malla_por_anio_mm": por_anio,
        });
        out.insert(r.0.to_string(), reg);
    }
    let doc = serde_json::json!({
        "anios_simulados": n_sim,
        "l_bloque": cfg.tierra.l_bloque,
        "semilla": cfg.run.seed,
        "cru_anios": [a0, a1],
        "cobertura_min": cfg.tierra.cobertura_min,
        "anios_remuestreo": periodo,
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

// --- La red de ríos con datos inventados ----------------------------------------------------------

/// Escribe un GeoTIFF mínimo (little-endian, sin comprimir, una fila por tira) de una rejilla global
/// de `ancho × alto` píxeles desde (−180°, 90°), con su valor «sin dato».
fn escribir_tiff(
    ruta: &std::path::Path,
    ancho: usize,
    alto: usize,
    datos: &[u8],
    bits: u16,
    formato: u16,
    sin_dato: &str,
) {
    let paso = 360.0 / ancho as f64;
    let mut ent: Vec<(u16, u16, u32, Vec<u8>)> = Vec::new();
    let largo_u32 = |v: u32| v.to_le_bytes().to_vec();
    let corto = |v: u16| v.to_le_bytes().to_vec();
    let bytes_fila = ancho * bits as usize / 8;
    ent.push((256, 4, 1, largo_u32(ancho as u32)));
    ent.push((257, 4, 1, largo_u32(alto as u32)));
    ent.push((258, 3, 1, corto(bits)));
    ent.push((259, 3, 1, corto(1)));
    ent.push((262, 3, 1, corto(1)));
    ent.push((273, 4, alto as u32, vec![]));
    ent.push((277, 3, 1, corto(1)));
    ent.push((278, 4, 1, largo_u32(1)));
    ent.push((
        279,
        4,
        alto as u32,
        (0..alto)
            .flat_map(|_| (bytes_fila as u32).to_le_bytes())
            .collect(),
    ));
    ent.push((339, 3, 1, corto(formato)));
    ent.push((
        33550,
        12,
        3,
        [paso, paso, 0.0]
            .iter()
            .flat_map(|x| x.to_le_bytes())
            .collect(),
    ));
    ent.push((
        33922,
        12,
        6,
        [0.0f64, 0.0, 0.0, -180.0, 90.0, 0.0]
            .iter()
            .flat_map(|x| x.to_le_bytes())
            .collect(),
    ));
    let mut nd = sin_dato.as_bytes().to_vec();
    nd.push(0);
    ent.push((42113, 2, nd.len() as u32, nd));
    let ifd = 8usize;
    let mut extra = ifd + 2 + ent.len() * 12 + 4;
    let mut bloques: Vec<u8> = Vec::new();
    let mut ifd_b: Vec<u8> = (ent.len() as u16).to_le_bytes().to_vec();
    let datos0 = {
        // Lo que va fuera del IFD, con las posiciones de las filas al final.
        let mut x = extra;
        for (tag, _, _, v) in &ent {
            if *tag == 273 {
                x += 4 * alto;
            } else if v.len() > 4 {
                x += v.len();
            }
        }
        x
    };
    for (tag, tipo, cuenta, v) in &ent {
        ifd_b.extend_from_slice(&tag.to_le_bytes());
        ifd_b.extend_from_slice(&tipo.to_le_bytes());
        ifd_b.extend_from_slice(&cuenta.to_le_bytes());
        let v = if *tag == 273 {
            (0..alto)
                .flat_map(|r| ((datos0 + r * bytes_fila) as u32).to_le_bytes())
                .collect()
        } else {
            v.clone()
        };
        if v.len() <= 4 {
            let mut c = v.clone();
            c.resize(4, 0);
            ifd_b.extend_from_slice(&c);
        } else {
            ifd_b.extend_from_slice(&(extra as u32).to_le_bytes());
            bloques.extend_from_slice(&v);
            extra += v.len();
        }
    }
    ifd_b.extend_from_slice(&0u32.to_le_bytes());
    let mut out = b"II".to_vec();
    out.extend_from_slice(&42u16.to_le_bytes());
    out.extend_from_slice(&(ifd as u32).to_le_bytes());
    out.extend_from_slice(&ifd_b);
    out.extend_from_slice(&bloques);
    assert_eq!(out.len(), datos0);
    out.extend_from_slice(datos);
    std::fs::write(ruta, out).expect("escribir el TIFF de prueba");
}

#[test]
fn la_red_reescalada_crece_rio_abajo_y_las_cuencas_cerradas_no_rebosan() {
    use abm_terreno::geometry::locate;
    use abm_terreno::tierra::hidro::{self, NINGUNA, fin};
    // Rejilla global de 1°: un continente (lon −60…60, lat −40…40) que desagua al oeste, con una
    // cuenca cerrada dentro (lon 20…40, lat −10…10) que lleva todo a su centro y un mar interior (lon
    // −40…−20, lat −10…10) sin conexión con el océano, adonde va a parar lo que tiene al este.
    let (w, h) = (360usize, 180usize);
    let tierra = |r: usize, c: usize| (120..240).contains(&c) && (50..130).contains(&r);
    let cerrada = |r: usize, c: usize| (200..220).contains(&c) && (80..100).contains(&r);
    let interior = |r: usize, c: usize| (140..160).contains(&c) && (80..100).contains(&r);
    let (cc, rc) = (210usize, 90usize);
    // Los códigos de HydroSHEDS v1.1: el mar va sin dato (255) y todo píxel final vale 0; la máscara
    // dice si es la salida al mar (2) o el fondo de una cuenca cerrada (3).
    const MAR: u8 = 255;
    let mut dir = vec![MAR; w * h];
    let mut msk = vec![MAR; w * h];
    for r in 0..h {
        for c in 0..w {
            if !tierra(r, c) || interior(r, c) {
                continue;
            }
            msk[r * w + c] = 1;
            dir[r * w + c] = if cerrada(r, c) {
                if c < cc {
                    1
                } else if c > cc {
                    16
                } else if r < rc {
                    4
                } else if r > rc {
                    64
                } else {
                    msk[r * w + c] = 3;
                    0
                }
            } else if c == 120 {
                msk[r * w + c] = 2;
                0
            } else {
                16
            };
        }
    }
    // Acumulación: cada píxel suma los que le vierten (de aguas arriba abajo, por orden topológico).
    let destino = |q: usize| -> Option<usize> {
        let (r, c) = (q / w, q % w);
        let (dr, dc): (i64, i64) = match dir[q] {
            1 => (0, 1),
            4 => (1, 0),
            16 => (0, -1),
            64 => (-1, 0),
            _ => return None,
        };
        let d = ((r as i64 + dr) as usize) * w + (c as i64 + dc) as usize;
        (dir[d] != MAR).then_some(d)
    };
    let mut entradas = vec![0u32; w * h];
    for (q, &d0) in dir.iter().enumerate() {
        if d0 != MAR
            && let Some(d) = destino(q)
        {
            entradas[d] += 1;
        }
    }
    let mut acum = vec![0u32; w * h];
    let mut cola: std::collections::VecDeque<usize> = (0..w * h)
        .filter(|&q| dir[q] != MAR && entradas[q] == 0)
        .collect();
    while let Some(q) = cola.pop_front() {
        acum[q] += 1;
        if let Some(d) = destino(q) {
            acum[d] += acum[q];
            entradas[d] -= 1;
            if entradas[d] == 0 {
                cola.push_back(d);
            }
        }
    }
    let dirs_tmp = std::env::temp_dir().join(format!("abm_red_prueba_{}", std::process::id()));
    std::fs::create_dir_all(&dirs_tmp).expect("carpeta temporal");
    let (p_dir, p_acc, p_msk) = (
        dirs_tmp.join("dir.tif"),
        dirs_tmp.join("acc.tif"),
        dirs_tmp.join("msk.tif"),
    );
    escribir_tiff(&p_dir, w, h, &dir, 8, 1, "255");
    escribir_tiff(&p_msk, w, h, &msk, 8, 1, "255");
    let acc_bytes: Vec<u8> = acum
        .iter()
        .zip(&dir)
        .flat_map(|(&a, &d)| (if d == MAR { u32::MAX } else { a }).to_le_bytes())
        .collect();
    escribir_tiff(&p_acc, w, h, &acc_bytes, 32, 1, "4294967295");

    let mut cfg = cfg_pequena();
    let n = face_n(cfg.planet.n0, cfg.tierra.nivel_carga);
    let m = 6 * (n as usize) * (n as usize);
    // La altura de cada celda, por su centro: el océano y el mar interior tienen profundidad.
    let celda_de = |lat: f64, lon: f64| {
        let (f, l) = (lat.to_radians(), lon.to_radians());
        let (c, i, j) = locate([f.cos() * l.cos(), f.cos() * l.sin(), f.sin()], n);
        ((c as usize * n as usize + j as usize) * n as usize) + i as usize
    };
    let mut z = vec![0.0; m];
    for c in 0..6u8 {
        for j in 0..n {
            for i in 0..n {
                let (lat, lon) = lat_lon(center_dir(c, i, j, n));
                let (lat, lon) = (lat.to_degrees(), lon.to_degrees());
                let k = ((c as usize * n as usize + j as usize) * n as usize) + i as usize;
                z[k] = if lat.abs() < 10.0 && (-40.0..-20.0).contains(&lon) {
                    -500.0
                } else if lat.abs() < 40.0 && lon.abs() < 60.0 {
                    400.0
                } else {
                    -3000.0
                };
            }
        }
    }
    let (red, mar) = hidro::reescalar(&p_dir, &p_acc, &p_msk, &z, n).expect("reescalado");
    std::fs::remove_dir_all(&dirs_tmp).ok();
    // El área drenada crece estrictamente río abajo, también con la regla de la vecina.
    let mut con_red = 0;
    for k in 0..m {
        for a in [red.abajo[k], red.vecina[k]] {
            if a != NINGUNA {
                con_red += 1;
                assert!(red.acum[a as usize] > red.acum[k], "celda {k}");
                assert!(!mar[k] && !mar[a as usize], "el mar no entra en la red");
            }
        }
    }
    assert!(con_red > 50, "la red tiene que tener tramos: {con_red}");
    let cerradas = hidro::acaba_en_cerrada(&red);
    let terminales_cerradas = (0..m)
        .filter(|&k| red.abajo[k] == NINGUNA && red.fin[k] == fin::CERRADA)
        .count();
    assert!(terminales_cerradas >= 1, "la cuenca cerrada tiene su fondo");
    // Cada celda acaba donde acaba su píxel de salida: el fondo de la cuenca cerrada, el mar interior o
    // el océano.
    assert!(
        cerradas[celda_de(0.0, 30.0)],
        "el centro de la cuenca cerrada"
    );
    assert!(
        cerradas[celda_de(0.0, 50.0)],
        "lo que vierte a la cuenca cerrada"
    );
    assert!(
        cerradas[celda_de(0.0, 0.0)],
        "lo que vierte al mar interior"
    );
    assert!(!cerradas[celda_de(30.0, 0.0)], "el norte llega al océano");
    assert!(!cerradas[celda_de(0.0, -50.0)], "el oeste llega al océano");
    // El mar interior tiene profundidad, pero no es mar: es una cuenca cerrada que acaba en sí misma.
    let ki = celda_de(0.0, -30.0);
    assert!(!mar[ki], "el mar interior no está conectado con el océano");
    assert!(red.abajo[ki] == NINGUNA && red.fin[ki] == fin::CERRADA);
    assert!(mar[celda_de(0.0, 150.0)], "el océano es mar");
    // La tierra bajo el nivel del mar sin píxeles de mar no es mar aunque toque la costa.
    let mut z_bajo = z.clone();
    let kb = celda_de(0.0, -50.0);
    z_bajo[kb] = -5.0;
    assert_eq!(
        red.pix_mar[kb], 0,
        "la celda de la prueba es toda de tierra"
    );
    assert!(!hidro::mar_conectado(&z_bajo, &red.pix_mar, &red.pix_tierra, n)[kb]);

    // En una malla de niveles mezclados: ninguna cuenca cerrada rebosa ni ninguna abierta se cierra.
    cfg.tierra.anio_forzado = 2001;
    let datos = datos_inventados(&cfg);
    let fino = &datos.niveles[cfg.tierra.nivel_carga as usize];
    let nivel = Nivel {
        n,
        area: fino.area.clone(),
        z: z.clone(),
        hielo: vec![0.0; m],
        lago: vec![0.0; m],
        t: fino.t.clone(),
        p: fino.p.clone(),
        cobertura: vec![1.0; m],
    };
    let hid = Hidrologia {
        red,
        mar,
        lagos_total_km2: 0.0,
    };
    let d = Arc::new(desde_nivel_fino(
        &cfg,
        nivel,
        datos.cru.clone(),
        datos.cru_de_celda.clone(),
        hid,
        0,
    ));
    let (world, _) = abm_terreno::worldgen::generate_tierra(&cfg, Arc::clone(&d));
    assert_eq!(
        world.flow_order.len(),
        world.topo.num_cells,
        "la red de la malla no tiene ciclos"
    );
    let dg = abm_terreno::metrics::diagnostico_tierra(&world).expect("diagnóstico");
    assert_eq!(
        dg.cerradas_que_rebosan_km2, 0.0,
        "una cuenca cerrada rebosa"
    );
    assert_eq!(dg.abiertas_que_acaban_cerradas_km2, 0.0);
    assert!(dg.red.cuencas_cerradas_km2 > 0.0 && dg.red.mares_interiores_km2 > 0.0);
    assert!(
        (0..world.topo.num_cells).any(|s| world.receiver[s] == abm_terreno::hydro::CERRADA),
        "la malla tiene el fondo de la cuenca cerrada"
    );
    // El agua de cada celda de la malla acaba donde acaba la de su salida en el nivel de carga: el
    // receptor de una celda tiene siempre más área drenada.
    for s in 0..world.topo.num_cells {
        let r = world.receiver[s];
        if r >= 0 {
            assert!(world.drainage_area[r as usize] > world.drainage_area[s]);
        }
    }
}
