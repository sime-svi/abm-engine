//! Relieve tectónico (incremento 4 del terreno): el relieve de nacimiento sale de la deriva de las
//! placas durante cientos de millones de años, de la isostasia y del volumen de agua.
//!
//! Diseño del responsable: `docs/design/terreno/diseno_relieve_tectonico.md`; decisiones técnicas,
//! correcciones de realismo y resultados: `docs/design/terreno/incremento-4-diseno.md`.
//!
//! Cadena (§9): G1 estado inicial → G2 deriva ([`deriva`]) → G3 altura isostática y fosas
//! ([`isostasia`]) → G4 nivel del mar por volumen → G5 detalle fino por contexto tectónico (en el
//! generador de estáticos, [`crate::statics`]) → G6–G7 prerrodaje de erosión con levantamiento y
//! rebote ([`crate::erosion`]). Durante la simulación no corre nada de esto (§2, §6.10).

pub mod deriva;
pub mod historia;
pub mod isostasia;
pub mod malla;

use crate::config::{TectonicaCfg, TerrenoConfig};
use crate::geometry::{V3, dot};
use deriva::{Deriva, TipoBorde};
use historia::{
    BalanceMedio, EdadesOceano, EventoTectonico, MetricaMyr, PlacaFinal, ResumenTectonica,
};
use malla::{Malla, NADA};

/// Distancia a una fosa cuando no hay ninguna cerca (km).
pub const S_LEJOS: f64 = 1.0e4;

/// Estado tectónico final en la malla tectónica: lo que el terreno necesita para su relieve y lo que
/// queda como dato estático (§3, componentes `Tectonica` y `Borde`).
#[derive(Clone)]
pub struct Tectonica {
    pub malla: Malla,
    /// Altura isostática de base sobre el nivel del mar (m), sin fosa ni detalle.
    pub z_base_m: Vec<f64>,
    /// Distancia firmada a la fosa activa más cercana (km): `≥ 0` en la placa que subduce, `< 0` en
    /// la que cabalga, [`S_LEJOS`] si no hay ninguna a menos de 6 anchuras de fosa.
    pub s_fosa_km: Vec<f64>,
    /// Ritmo de engrosamiento del último paso de la deriva (km/Myr): arcos y choques activos.
    pub dh_dt: Vec<f64>,
    /// Desviación típica del ruido de detalle (m) según la altura de base (decisión del responsable:
    /// continúa el espectro del relieve real).
    pub sigma_det_m: Vec<f64>,
    pub continental: Vec<bool>,
    pub h_km: Vec<f64>,
    pub sed_km: Vec<f64>,
    pub edad_oc: Vec<f64>,
    pub edad_orog: Vec<f64>,
    pub placa: Vec<u64>,
    /// Borde de placa más cercano: tipo (1 divergente, 2 subducción, 3 colisión, 4 transformante),
    /// distancia (km) y lado (1 = la placa de la celda subduce allí, 2 = cabalga, 0 = otro).
    pub borde_tipo: Vec<u8>,
    pub borde_dist_km: Vec<f64>,
    pub borde_lado: Vec<u8>,
    /// Nivel del mar respecto al dato (m): `h = z − ζ`.
    pub zeta_m: f64,
    pub d_fosa_m: f64,
    pub w_fosa_km: f64,
    /// `1 − ρ_c/ρ_m`: levantamiento de la superficie por km de engrosamiento (§6.9a).
    pub f_levantamiento: f64,
}

/// Lo que da la tectónica en un punto del planeta.
#[derive(Debug, Clone, Copy)]
pub struct Base {
    /// Altura sobre el nivel del mar (m), con la fosa y sin el detalle.
    pub z_m: f64,
    /// Desviación típica del detalle (m).
    pub sigma_m: f64,
    /// Engrosamiento activo (km/Myr).
    pub dh_dt: f64,
}

impl Tectonica {
    /// Celdas y pesos de interpolación en `d`: el vecindario 3×3 de la celda que lo contiene, con
    /// un núcleo de Wendland de soporte 1,2 lados de celda. Es continuo: toda celda a menos del
    /// soporte está en ese vecindario (`incremento-4-diseno.md`, I15).
    #[inline]
    fn pesos(&self, d: V3) -> ([u32; 9], [f64; 9]) {
        let m = &self.malla;
        let k = m.localizar(d);
        let mut idx = [NADA; 9];
        let mut w = [0.0; 9];
        idx[0] = k as u32;
        idx[1..].copy_from_slice(&m.vec[k]);
        let soporte = 1.2 * m.lado_km / m.r_km;
        let mut suma = 0.0;
        for i in 0..9 {
            if idx[i] == NADA {
                continue;
            }
            let c = m.dir[idx[i] as usize];
            let dx = [d[0] - c[0], d[1] - c[1], d[2] - c[2]];
            let r = dot(dx, dx).sqrt() / soporte;
            if r < 1.0 {
                let u = 1.0 - r;
                w[i] = u * u * u * u * (4.0 * r + 1.0);
                suma += w[i];
            }
        }
        if suma > 0.0 {
            w.iter_mut().for_each(|x| *x /= suma);
        } else {
            w = [0.0; 9];
            w[0] = 1.0;
        }
        (idx, w)
    }

    #[inline]
    fn interp(&self, idx: &[u32; 9], w: &[f64; 9], campo: &[f64]) -> f64 {
        let mut s = 0.0;
        for i in 0..9 {
            if w[i] != 0.0 {
                s += w[i] * campo[idx[i] as usize];
            }
        }
        s
    }

    /// Base tectónica en la dirección `d`: campos interpolados de la malla tectónica, con la fosa
    /// evaluada en la distancia interpolada (así es estrecha también en las celdas finas).
    pub fn base_en(&self, d: V3) -> Base {
        let (idx, w) = self.pesos(d);
        let s = self.interp(&idx, &w, &self.s_fosa_km);
        let fosa = if s >= 0.0 && s < 6.0 * self.w_fosa_km {
            self.d_fosa_m * (-(s / self.w_fosa_km).powi(2)).exp()
        } else {
            0.0
        };
        Base {
            z_m: self.interp(&idx, &w, &self.z_base_m) - fosa,
            sigma_m: self.interp(&idx, &w, &self.sigma_det_m),
            dh_dt: self.interp(&idx, &w, &self.dh_dt),
        }
    }

    /// Levantamiento tectónico de la superficie (m/año) en `d` para el prerrodaje (§6.9a):
    /// `U = (1 − ρ_c/ρ_m)·dH/dt`, con 1 km/Myr = 1 mm/año.
    pub fn levantamiento_m_anio(&self, d: V3) -> f64 {
        let (idx, w) = self.pesos(d);
        self.f_levantamiento * self.interp(&idx, &w, &self.dh_dt).max(0.0) * 1e-3
    }

    /// Celda tectónica que contiene `d` (para los volcados).
    pub fn celda(&self, d: V3) -> usize {
        self.malla.localizar(d)
    }
}

/// Desviación típica del ruido de detalle en una altura de base (interpolación lineal en las cotas
/// de la config, constante fuera).
pub fn sigma_detalle(c: &TectonicaCfg, z_m: f64) -> f64 {
    let (x, y) = (&c.detalle_cotas_m, &c.detalle_sigma_m);
    if z_m <= x[0] {
        return y[0];
    }
    for i in 1..x.len() {
        if z_m <= x[i] {
            let t = (z_m - x[i - 1]) / (x[i] - x[i - 1]);
            return y[i - 1] + t * (y[i] - y[i - 1]);
        }
    }
    y[y.len() - 1]
}

/// Génesis tectónica completa (G1–G4): deriva, isostasia, fosas y nivel del mar.
pub fn genesis(cfg: &TerrenoConfig) -> (Tectonica, ResumenTectonica) {
    let tc = &cfg.tectonica;
    let malla = Malla::new(cfg.planet.n0, tc.nivel_malla, cfg.planet.r_km);
    let mut d = Deriva::nueva(tc, cfg.run.seed, malla);
    d.correr();
    estado_final(&d)
}

/// Estado final de una deriva ya corrida: alturas, fosas, bordes y resumen.
pub fn estado_final(d: &Deriva) -> (Tectonica, ResumenTectonica) {
    let c = &d.cfg;
    let m = &d.malla;
    let num = m.num;
    let z = d.alturas();
    let zeta = d.nivel_del_mar();
    let mut t = Tectonica {
        malla: m.clone(),
        z_base_m: z.iter().map(|&x| x - zeta).collect(),
        s_fosa_km: vec![S_LEJOS; num],
        dh_dt: vec![0.0; num],
        sigma_det_m: vec![0.0; num],
        continental: vec![false; num],
        h_km: vec![0.0; num],
        sed_km: vec![0.0; num],
        edad_oc: vec![0.0; num],
        edad_orog: vec![0.0; num],
        placa: vec![0; num],
        borde_tipo: vec![0; num],
        borde_dist_km: vec![S_LEJOS; num],
        borde_lado: vec![0; num],
        zeta_m: zeta,
        d_fosa_m: c.d_fosa_m,
        w_fosa_km: c.w_fosa_km,
        f_levantamiento: (c.rho_m - c.rho_c) / c.rho_m,
    };
    for (k, &zk) in z.iter().enumerate() {
        if let Some(cr) = d.corteza(k) {
            t.continental[k] = cr.cont;
            t.h_km[k] = cr.h;
            t.sed_km[k] = cr.sed;
            t.edad_oc[k] = cr.edad_oc;
            t.edad_orog[k] = cr.edad_orog;
            t.dh_dt[k] = cr.dh_dt;
        }
        if d.dueno[k] != NADA {
            t.placa[k] = d.placas[d.dueno[k] as usize].id;
        }
        // El contexto del detalle es la altura isostática respecto al dato (función del espesor de
        // la corteza, como la rugosidad de §6.8), no respecto al nivel del mar del planeta.
        t.sigma_det_m[k] = sigma_detalle(c, zk);
    }
    // Distancia firmada a las fosas activas: hacia dentro de la placa que subduce (+) y de la que
    // cabalga (−), cada una sin salir de su placa.
    let dueno = &d.dueno;
    let mut fuentes_s = Vec::new();
    let mut fuentes_o = Vec::new();
    for b in &d.bordes {
        if b.tipo != TipoBorde::Subduccion {
            continue;
        }
        let d0 = 0.5 * m.dist_km(b.k as usize, b.k_q as usize);
        if b.p_subduce {
            fuentes_s.push((b.k, d0, b.k));
        } else {
            fuentes_o.push((b.k, d0, b.k));
        }
    }
    let max_d = 6.0 * c.w_fosa_km;
    let ds = distancias(m, &fuentes_s, dueno, max_d);
    let dob = distancias(m, &fuentes_o, dueno, max_d);
    for k in 0..num {
        if ds[k] <= dob[k] && ds[k].is_finite() {
            t.s_fosa_km[k] = ds[k];
        } else if dob[k].is_finite() {
            t.s_fosa_km[k] = -dob[k];
        }
    }
    // Borde más cercano (tipo, distancia y lado), sin salir de la placa.
    let fuentes_b: Vec<(u32, f64, u32)> = d
        .bordes
        .iter()
        .enumerate()
        .map(|(i, b)| (b.k, 0.5 * m.dist_km(b.k as usize, b.k_q as usize), i as u32))
        .collect();
    let (db, fb) = distancias_con_fuente(m, &fuentes_b, dueno, f64::INFINITY);
    for k in 0..num {
        if fb[k] != NADA {
            let b = &d.bordes[fb[k] as usize];
            t.borde_tipo[k] = b.tipo.codigo();
            t.borde_dist_km[k] = db[k];
            t.borde_lado[k] = match (b.tipo, b.p_subduce) {
                (TipoBorde::Subduccion, true) => 1,
                (TipoBorde::Subduccion, false) => 2,
                _ => 0,
            };
        }
    }
    let resumen = resumen(d, &t);
    (t, resumen)
}

fn distancias(m: &Malla, fuentes: &[(u32, f64, u32)], dueno: &[u32], max_d: f64) -> Vec<f64> {
    distancias_con_fuente(m, fuentes, dueno, max_d).0
}

/// Dijkstra sin salir de la placa de cada fuente.
fn distancias_con_fuente(
    m: &Malla,
    fuentes: &[(u32, f64, u32)],
    dueno: &[u32],
    max_d: f64,
) -> (Vec<f64>, Vec<u32>) {
    use std::cmp::Ordering;
    use std::collections::BinaryHeap;
    #[derive(PartialEq)]
    struct It(f64, u32);
    impl Eq for It {}
    impl Ord for It {
        fn cmp(&self, o: &Self) -> Ordering {
            o.0.total_cmp(&self.0).then_with(|| o.1.cmp(&self.1))
        }
    }
    impl PartialOrd for It {
        fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
            Some(self.cmp(o))
        }
    }
    let mut dist = vec![f64::INFINITY; m.num];
    let mut fuente = vec![NADA; m.num];
    let mut heap = BinaryHeap::new();
    for &(k, d0, et) in fuentes {
        if d0 < dist[k as usize] {
            dist[k as usize] = d0;
            fuente[k as usize] = et;
            heap.push(It(d0, k));
        }
    }
    while let Some(It(dd, k)) = heap.pop() {
        let ku = k as usize;
        if dd > dist[ku] {
            continue;
        }
        for (j, &v) in m.vec[ku].iter().enumerate() {
            if v == NADA || dueno[v as usize] != dueno[ku] {
                continue;
            }
            let nd = dd + m.dvec[ku][j];
            if nd <= max_d && nd < dist[v as usize] {
                dist[v as usize] = nd;
                fuente[v as usize] = fuente[ku];
                heap.push(It(nd, v));
            }
        }
    }
    (dist, fuente)
}

/// Resumen de la génesis tectónica para la salida.
fn resumen(d: &Deriva, t: &Tectonica) -> ResumenTectonica {
    let m = &d.malla;
    let a_tot = m.area_total();
    let (mut a_cont, mut a_oc) = (0.0, 0.0);
    let mut edades: Vec<(f64, f64)> = Vec::new();
    let mut bins = [0.0; 21];
    for k in 0..m.num {
        if t.continental[k] {
            a_cont += m.area[k];
        } else {
            a_oc += m.area[k];
            edades.push((t.edad_oc[k], m.area[k]));
            bins[((t.edad_oc[k] / 10.0) as usize).min(20)] += m.area[k];
        }
    }
    edades.sort_by(|a, b| a.0.total_cmp(&b.0));
    let cuantil = |q: f64| -> f64 {
        let mut acc = 0.0;
        for &(e, a) in &edades {
            acc += a;
            if acc >= q * a_oc {
                return e;
            }
        }
        edades.last().map_or(0.0, |x| x.0)
    };
    let media = if a_oc > 0.0 {
        edades.iter().map(|&(e, a)| e * a).sum::<f64>() / a_oc
    } else {
        0.0
    };
    // Placas finales.
    let n = d.placas.len();
    let mut area = vec![0.0; n];
    let mut cont = vec![0.0; n];
    let mut v2 = vec![0.0; n];
    for k in 0..m.num {
        let p = d.dueno[k];
        if p == NADA {
            continue;
        }
        let p = p as usize;
        area[p] += m.area[k];
        if t.continental[k] {
            cont[p] += m.area[k];
        }
        let v = malla::velocidad(d.placas[p].omega, m.dir[k], m.r_km);
        v2[p] += dot(v, v) * m.area[k];
    }
    let mut fosa = vec![0.0; n];
    for b in &d.bordes {
        if b.tipo == TipoBorde::Subduccion && b.p_subduce {
            fosa[b.p as usize] += b.largo_km;
        }
    }
    let mut placas_final: Vec<PlacaFinal> = (0..n)
        .filter(|&i| d.placas[i].viva && area[i] > 0.0)
        .map(|i| PlacaFinal {
            id: d.placas[i].id,
            area_frac: area[i] / a_tot,
            continental_frac: cont[i] / area[i],
            v_rms_cm_anio: (v2[i] / area[i]).sqrt() / deriva::CM_ANIO,
            fosa_km: fosa[i],
        })
        .collect();
    placas_final.sort_by(|a, b| b.area_frac.total_cmp(&a.area_frac).then(a.id.cmp(&b.id)));
    // Sin pasos tras el rodaje, 0 (el JSON no admite NaN).
    let media_de = |(s, n): (f64, u64)| if n > 0 { s / n as f64 } else { 0.0 };
    ResumenTectonica {
        nivel_malla: m.nivel,
        celdas: m.num as u64,
        lado_km: m.lado_km,
        pasos: d.paso,
        nivel_mar_m: t.zeta_m,
        nivel_mar_tras_prerrodaje_m: 0.0,
        f_cont_inicial: d.cfg.f_cont,
        f_cont_final: a_cont / a_tot,
        cuentas: d.cuentas,
        edades_oceano: EdadesOceano {
            media_myr: media,
            mediana_myr: cuantil(0.5),
            p99_myr: cuantil(0.99),
            max_myr: edades.last().map_or(0.0, |x| x.0),
            frac_por_10_myr: bins
                .iter()
                .map(|&x| if a_oc > 0.0 { x / a_oc } else { 0.0 })
                .collect(),
        },
        v_rms_media_cm_anio: media_de(d.v_rms_acum),
        forsyth_uyeda_fosa: media_de(d.fu_fosa),
        forsyth_uyeda_continente: media_de(d.fu_cont),
        balance: balance_medio(&d.metricas, &d.fosas_por_convergencia_km, &d.eventos),
        placas_final,
        eventos: d.eventos.clone(),
        metricas: d.metricas.clone(),
    }
}

/// Medias del balance de la corteza sin el rodaje (adenda de balance §10).
fn balance_medio(
    metricas: &[MetricaMyr],
    fosas_km: &[f64; 10],
    eventos: &[EventoTectonico],
) -> BalanceMedio {
    let val: Vec<&MetricaMyr> = metricas.iter().filter(|m| !m.rodaje).collect();
    let Some(ultima) = val.last() else {
        return BalanceMedio::default();
    };
    let n = val.len() as f64;
    let media = |f: fn(&MetricaMyr) -> f64| val.iter().map(|m| f(m)).sum::<f64>() / n;
    // V_cont al acabar el rodaje: el del último paso de rodaje (o el primero válido si no hay).
    let v_inicio = metricas
        .iter()
        .rev()
        .find(|m| m.rodaje)
        .unwrap_or(val[0])
        .inventario_km3;
    BalanceMedio {
        arcos_au: media(|m| m.f_arc_au),
        erosion_subduccion_au: media(|m| m.f_se_au),
        sedimento_subducido_au: media(|m| m.f_sed_au),
        sedimento_acrecionado_au: media(|m| m.f_acr_au),
        choques_au: media(|m| m.f_col_au),
        delaminacion_au: media(|m| m.f_del_au),
        sedimento_al_mar_au: media(|m| m.f_den_au),
        rift_au: media(|m| m.f_rift_au),
        area_subducida_km2_anio: media(|m| m.area_subducida_km2_anio),
        area_creada_km2_anio: media(|m| m.area_creada_km2_anio),
        fosa_erosiva_frac: media(|m| m.fosa_erosiva_frac),
        dorsales_km: media(|m| m.dorsales_km),
        fosas_km: media(|m| m.fosas_km),
        colisiones_km: media(|m| m.colisiones_km),
        transformantes_km: media(|m| m.transformantes_km),
        espesor_max_medio_km: media(|m| m.espesor_max_km),
        espesor_max_final_km: ultima.espesor_max_km,
        espesor_max_todo_km: metricas
            .iter()
            .map(|m| m.espesor_max_km)
            .fold(0.0, f64::max),
        area_mas_70km_frac: media(|m| m.area_mas_70km_frac),
        area_mas_70km_final_frac: ultima.area_mas_70km_frac,
        v_cont_cambio_frac: if v_inicio > 0.0 {
            ultima.inventario_km3 / v_inicio - 1.0
        } else {
            0.0
        },
        sedimento_medio_oceano_km: ultima.sedimento_medio_oceano_km,
        perimetro_km: media(|m| m.perimetro_km),
        dientes_recolocados_km2_anio: media(|m| m.dientes_recolocados_km2_anio),
        dientes_sin_hueco_km2_anio: media(|m| m.dientes_sin_hueco_km2_anio),
        creacion_residual_km2_anio: media(|m| m.creacion_residual_km2_anio),
        area_reasignada_km2_anio: media(|m| m.area_reasignada_km2_anio),
        dientes_continentales_au: media(|m| m.dientes_continentales_au),
        convergencia_media_fosas_cm_anio: {
            let l: f64 = val.iter().map(|m| m.fosas_km).sum();
            if l > 0.0 {
                val.iter()
                    .map(|m| m.convergencia_media_fosas_cm_anio * m.fosas_km)
                    .sum::<f64>()
                    / l
            } else {
                0.0
            }
        },
        fosa_lenta_frac: {
            let t: f64 = fosas_km.iter().sum();
            if t > 0.0 {
                (fosas_km[0] + fosas_km[1]) / t
            } else {
                0.0
            }
        },
        difusas_km: media(|m| m.difusas_km),
        area_difusa_km2_anio: media(|m| m.area_difusa_km2_anio),
        edad_media_fosas_myr: {
            let l: f64 = val.iter().map(|m| m.fosas_km).sum();
            if l > 0.0 {
                val.iter()
                    .map(|m| m.edad_media_fosas_myr * m.fosas_km)
                    .sum::<f64>()
                    / l
            } else {
                0.0
            }
        },
        losa_media_frac: media(|m| m.losa_media_frac),
        nacimientos_100myr: {
            // Una métrica por paso de 1 Myr (§8).
            let myr = val.len() as f64;
            let mut x = [0.0; 3];
            for m in &val {
                for (i, v) in m.nacimientos.iter().enumerate() {
                    x[i] += *v as f64 * 100.0 / myr;
                }
            }
            x
        },
        muertes_100myr: {
            let myr = val.len() as f64;
            let mut x = [0.0; 2];
            for m in &val {
                for (i, v) in m.muertes.iter().enumerate() {
                    x[i] += *v as f64 * 100.0 / myr;
                }
            }
            x
        },
        invadido_transf_km_myr: media(|m| m.invadido_km[0]),
        invadido_margen_km_myr: media(|m| m.invadido_km[1]),
        roturas_100myr: val.iter().map(|m| m.roturas as f64).sum::<f64>() * 100.0 / n,
        convergencia_media_choques_cm_anio: {
            let l: f64 = val.iter().map(|m| m.colisiones_km).sum();
            if l > 0.0 {
                val.iter()
                    .map(|m| m.convergencia_media_choques_cm_anio * m.colisiones_km)
                    .sum::<f64>()
                    / l
            } else {
                0.0
            }
        },
        choques_con_losa_au: media(|m| m.f_col_con_losa_au),
        choques_sin_losa_au: media(|m| m.f_col_sin_losa_au),
        margen_pasivo_km: media(|m| m.margen_pasivo_km),
        edad_media_margen_myr: {
            let l: f64 = val.iter().map(|m| m.margen_pasivo_km).sum();
            if l > 0.0 {
                val.iter()
                    .map(|m| m.edad_media_margen_myr * m.margen_pasivo_km)
                    .sum::<f64>()
                    / l
            } else {
                0.0
            }
        },
        edad_max_margen_myr: media(|m| m.edad_max_margen_myr),
        edad_media_margen_final_myr: ultima.edad_media_margen_myr,
        edad_max_margen_final_myr: ultima.edad_max_margen_myr,
        vida_media_margen_myr: {
            let l: f64 = val.iter().map(|m| m.margen_fin_km).sum();
            if l > 0.0 {
                val.iter().map(|m| m.margen_fin_vida_km_myr).sum::<f64>() / l
            } else {
                0.0
            }
        },
        margen_terminado_km_myr: media(|m| m.margen_fin_km),
        // Ponderada por la corteza solapada: cuentan los choques grandes, no los roces de un trozo.
        duracion_media_choques_myr: {
            let (mut dv, mut v) = (0.0, 0.0);
            for e in eventos.iter().filter(|e| {
                e.tipo == "fin_choque" && e.clase.as_deref() == Some("continente_continente")
            }) {
                let sol = e.solapado_km3.unwrap_or(0.0);
                dv += e.tiempo_choque_myr.unwrap_or(0.0) * sol;
                v += sol;
            }
            if v > 0.0 { dv / v } else { 0.0 }
        },
        choques_terminados: eventos
            .iter()
            .filter(|e| {
                e.tipo == "fin_choque" && e.clase.as_deref() == Some("continente_continente")
            })
            .count() as u32,
        nacimientos_zf_100myr: val.iter().map(|m| m.nacimientos_zf as f64).sum::<f64>() * 100.0 / n,
        separar_zf_fallidos_frac: {
            let (i, f) = val.iter().fold((0u32, 0u32), |(i, f), m| {
                (i + m.separar_zf[0], f + m.separar_zf[1])
            });
            if i > 0 { f as f64 / i as f64 } else { 0.0 }
        },
        zf_escalon_km: media(|m| m.zf_escalon_km),
        zf_candidata_km: media(|m| m.zf_candidata_km),
        fosas_por_convergencia_frac: {
            let t: f64 = fosas_km.iter().sum();
            fosas_km
                .iter()
                .map(|&x| if t > 0.0 { x / t } else { 0.0 })
                .collect()
        },
    }
}
