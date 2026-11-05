//! Observador de la biología de los agentes (diseño de biología §8; incremento 1 §8): recuentos
//! por tick, balance de energía y tablas de vida anuales. Solo lee el modelo; las sumas son exactas
//! o enteras, así que no dependen del orden de las entidades.

use bevy_ecs::prelude::*;

use abm_core::time::Tick;
use abm_persistence::schema_bio::{
    BIO_SCHEMA_VERSION, Balance, BioAnio, BioDoc, BioRunMeta, BioTick, EspecieAnio, EspecieMeta,
    EspecieTick,
};
use abm_ruleset::biologia::BioConfig;
use abm_ruleset::biologia::azar::SumaExacta;
use abm_ruleset::biologia::componentes::{
    Dependiente, Diario, Edad, EspecieRef, EstadoRepro, IdBio, Masa, Repro, Reserva, Sexo,
};
use abm_ruleset::biologia::config::Termico;
use abm_ruleset::biologia::habitat::Habitat;
use abm_ruleset::biologia::recursos::{Catalogo, Libro, N_CAUSAS};

/// Recuentos de una especie acumulados desde el registro anterior.
#[derive(Debug, Clone, Default)]
struct Intervalo {
    nacimientos: u64,
    muertes: [u64; N_CAUSAS],
    concepciones: u64,
    abortos: u64,
    destetes: u64,
    destetes_forzosos: u64,
    crias_destetadas: u64,
    consumo_kcal: f64,
    rebrote_g: f64,
}

/// Acumuladores anuales de una especie (tabla de vida del periodo).
#[derive(Debug, Clone, Default)]
struct Anual {
    poblacion_dias: u64,
    nacimientos: u64,
    muertes: [u64; N_CAUSAS],
    /// Exposición (individuos·día) y muertes por sexo (0 hembra, 1 macho) y clase de edad.
    expo: [Vec<u64>; 2],
    muertes_clase: [Vec<u64>; 2],
    /// Crías nacidas por clase de edad de la madre.
    nac_madre: Vec<u64>,
    intervalos_dias: u64,
    intervalos_n: u64,
    consumo_kcal: f64,
    rebrote_g: f64,
    /// Demografía (revisión 1): crías por grupo de edad de la madre y exposición de las hembras
    /// (hembras·día) por grupo; exposición y muertes por sexo y grupo de mortalidad; suma de las
    /// edades del último parto (días) y cuántas hembras terminan su vida reproductiva con partos.
    nac_grupo: Vec<u64>,
    expo_hembras_grupo: Vec<u64>,
    expo_g: [[u64; 4]; 2],
    muertes_g: [[u64; 4]; 2],
    ultimo_parto_dias: u64,
    ultimo_parto_n: u64,
}

impl Anual {
    fn nuevo(n_clases: usize, n_grupos: usize) -> Anual {
        Anual {
            expo: [vec![0; n_clases], vec![0; n_clases]],
            muertes_clase: [vec![0; n_clases], vec![0; n_clases]],
            nac_madre: vec![0; n_clases],
            nac_grupo: vec![0; n_grupos],
            expo_hembras_grupo: vec![0; n_grupos],
            ..Anual::default()
        }
    }
}

/// Grupo de mortalidad de una edad (días): 0 crías, 1 jóvenes, 2 adultos, 3 viejos.
#[inline]
fn grupo_mortalidad(a: u32, limites: [u32; 3]) -> usize {
    limites.iter().take_while(|&&x| a >= x).count()
}

/// Registro de métricas de la biología (recurso del mundo).
#[derive(Resource, Debug)]
pub struct RegistroBio {
    every: u64,
    total: u64,
    dias_anio: u64,
    area_km2: f64,
    n_max: Vec<f64>,
    intervalo: Vec<Intervalo>,
    balance: Balance,
    anual: Vec<Anual>,
    ids: Vec<u64>,
    /// Anchura de los grupos de fecundidad (días), nº de grupos y límites de los grupos de
    /// mortalidad, por especie.
    ancho_fec: u32,
    n_grupos: Vec<usize>,
    limites: Vec<[u32; 3]>,
    pub meta: BioRunMeta,
    pub ticks: Vec<BioTick>,
    pub anios: Vec<BioAnio>,
}

fn sexo_i(s: Sexo) -> usize {
    match s {
        Sexo::Hembra => 0,
        Sexo::Macho => 1,
    }
}

impl RegistroBio {
    /// Prepara el registro de una corrida recién creada (con la población inicial ya en el mundo).
    pub fn nuevo(
        cfg: &BioConfig,
        world: &mut World,
        engine_version: &str,
        ruleset_version: &str,
    ) -> RegistroBio {
        let n_esp = world.resource::<Catalogo>().especies.len();
        let mut iniciales = vec![0u64; n_esp];
        let mut q = world.query::<&EspecieRef>();
        for e in q.iter(world) {
            iniciales[e.0 as usize] += 1;
        }
        let cat = world.resource::<Catalogo>();
        let hab = world.resource::<Habitat>();
        let rho_veg = cat.forraje.rho_veg;
        let ancho_fec =
            (cfg.metricas.ancho_fecundidad_anios * cfg.tiempo.dias_anio as f64).round() as u32;
        let ancho_fec = ancho_fec.max(1);
        let limites: Vec<[u32; 3]> = cat
            .especies
            .iter()
            .map(|e| [e.d_destete, e.a_rep, e.a_vejez])
            .collect();
        let n_grupos: Vec<usize> = cat
            .especies
            .iter()
            .map(|e| (e.a_max / ancho_fec) as usize + 1)
            .collect();
        let area_km2 = hab.area_m2.iter().sum::<f64>() / 1e6;
        let n_max: Vec<f64> = cat
            .especies
            .iter()
            .map(|e| {
                (0..hab.n_celdas)
                    .map(|c| e.n_max(hab.area_m2[c], hab.ppn[c], rho_veg, hab.f_ecto[c]))
                    .sum()
            })
            .collect();
        let especies = cat
            .especies
            .iter()
            .enumerate()
            .map(|(k, e)| EspecieMeta {
                nombre: e.nombre.clone(),
                termico: match e.termico {
                    Termico::Endotermo => "endotermo".into(),
                    Termico::Ectotermo => "ectotermo".into(),
                },
                masa_kg: e.masa,
                m0_kg: e.m0,
                pal: e.pal,
                k_ap: e.k_ap,
                eps: e.eps,
                f_com: e.f_com,
                f_grasa_max: e.f_grasa_max,
                camada: e.camada,
                p_conc: e.p_conc,
                rho_sexo: e.rho_sexo,
                p_huerf: e.p_huerf,
                a_max_dias: e.a_max,
                a_crec90_dias: e.a_crec90,
                a_rep_dias: e.a_rep,
                a_fin_rep_dias: e.a_fin_rep,
                d_gest_dias: e.d_gest,
                d_destete_dias: e.d_destete,
                delta_a_dias: e.delta_a,
                c_w: e.c_w,
                siler: e.siler,
                basal_adulto_kcal: e.basal(e.masa, 1.0),
                gasto_adulto_kcal: e.gasto_adulto(1.0),
                e_max_adulto_kcal: e.e_max(e.masa),
                ayuno_dias: e.e_max(e.masa) / e.gasto_adulto(1.0),
                n_max: n_max[k],
                poblacion_inicial: iniciales[k],
                masa_kg_hembra: e.masa_sexo[0],
                masa_kg_macho: e.masa_sexo[1],
                ayuno_dias_hembra: e.ayuno_s(0),
                ayuno_dias_macho: e.ayuno_s(1),
                h_medio: e.h_medio,
                mu_est_anios: e.mu_est / e.dias_anio,
                sigma_est_anios: e.sigma_est / e.dias_anio,
                grupos_edad_dias: limites[k],
            })
            .collect();
        let meta = BioRunMeta {
            name: cfg.run.name.clone(),
            seed: cfg.run.seed,
            engine_version: engine_version.to_string(),
            ruleset_version: ruleset_version.to_string(),
            total_ticks: cfg.run.total_ticks,
            dias_anio: cfg.tiempo.dias_anio,
            metrics_every: cfg.run.metrics_every,
            celdas: hab.n_celdas as u64,
            area_total_km2: area_km2,
            especies,
        };
        let anual = cat
            .especies
            .iter()
            .zip(&n_grupos)
            .map(|(e, &g)| Anual::nuevo(e.n_clases(), g))
            .collect();
        RegistroBio {
            every: cfg.run.metrics_every,
            total: cfg.run.total_ticks,
            dias_anio: cfg.tiempo.dias_anio as u64,
            area_km2,
            n_max,
            intervalo: vec![Intervalo::default(); n_esp],
            balance: Balance::default(),
            anual,
            ids: Vec::new(),
            ancho_fec,
            n_grupos,
            limites,
            meta,
            ticks: Vec::new(),
            anios: Vec::new(),
        }
    }

    /// Documento final de la corrida.
    pub fn documento(&self) -> BioDoc {
        BioDoc {
            schema_version: BIO_SCHEMA_VERSION,
            run: self.meta.clone(),
            ticks: self.ticks.clone(),
            anios: self.anios.clone(),
        }
    }
}

/// Grupo de fecundidad (de `ancho` días) de una edad, sin pasar del último.
#[inline]
fn grupo_fec(a: u32, ancho: u32, n_grupos: usize) -> usize {
    ((a / ancho) as usize).min(n_grupos - 1)
}

/// Recuentos instantáneos de una especie.
#[derive(Debug, Clone, Copy, Default)]
struct Censo {
    n: u64,
    hembras: u64,
    machos: u64,
    dependientes: u64,
    gestantes: u64,
    lactantes: u64,
    condicion: SumaExacta,
    condicion_lactantes: SumaExacta,
    reservas: SumaExacta,
}

type QCenso<'a> = (
    &'a IdBio,
    &'a EspecieRef,
    &'a Sexo,
    &'a Edad,
    &'a Masa,
    &'a Reserva,
    &'a Repro,
    Option<&'a Dependiente>,
    &'a Diario,
);

/// Esperanza de vida al nacer de una tabla de vida del periodo con clases de `w` años: exposición
/// (individuos·día) y muertes por clase. La última clase con exposición se cierra con `L = l/m`.
fn e0_periodo(expo: &[u64], muertes: &[u64], w: f64, dias_anio: f64) -> Option<f64> {
    let ultima = expo.iter().rposition(|&x| x > 0)?;
    let mut l = 1.0;
    let mut e0 = 0.0;
    for x in 0..=ultima {
        let m = if expo[x] > 0 {
            muertes[x] as f64 / (expo[x] as f64 / dias_anio)
        } else {
            0.0
        };
        if x == ultima {
            e0 += if m > 0.0 { l / m } else { l * w / 2.0 };
            break;
        }
        let l_sig = l * (-m * w).exp();
        e0 += 0.5 * (l + l_sig) * w;
        l = l_sig;
    }
    Some(e0)
}

/// Observador del tick (va detrás de los sistemas del modelo).
pub fn collect_bio(
    tick: Res<Tick>,
    cat: Res<Catalogo>,
    hab: Res<Habitat>,
    libro: Res<Libro>,
    mut reg: ResMut<RegistroBio>,
    q: Query<QCenso>,
) {
    let t = tick.0;
    let reg = &mut *reg;
    let n_esp = cat.especies.len();
    let da = reg.dias_anio as f64;

    // Censo y exposición del día.
    let mut censo = vec![Censo::default(); n_esp];
    let mut reservas = SumaExacta::default();
    for (_, esp, sexo, edad, masa, res, repro, dep, _) in q.iter() {
        let k = esp.0 as usize;
        let e = &cat.especies[k];
        let c = &mut censo[k];
        c.n += 1;
        match sexo {
            Sexo::Hembra => c.hembras += 1,
            Sexo::Macho => c.machos += 1,
        }
        c.dependientes += u64::from(dep.is_some());
        c.gestantes += u64::from(repro.estado == EstadoRepro::Gestante);
        let r = res.0 / e.e_max_s(sexo.i(), masa.0);
        if repro.estado == EstadoRepro::Lactante {
            c.lactantes += 1;
            c.condicion_lactantes.sumar(r);
        }
        c.condicion.sumar(r);
        c.reservas.sumar(res.0);
        reservas.sumar(res.0);
        let an = &mut reg.anual[k];
        an.expo[sexo_i(*sexo)][e.clase(edad.0)] += 1;
        an.expo_g[sexo.i()][grupo_mortalidad(edad.0, reg.limites[k])] += 1;
        if *sexo == Sexo::Hembra {
            an.expo_hembras_grupo[grupo_fec(edad.0, reg.ancho_fec, reg.n_grupos[k])] += 1;
        }
    }

    // Recuentos del libro.
    let filas = libro
        .esp
        .iter()
        .zip(reg.intervalo.iter_mut())
        .zip(reg.anual.iter_mut())
        .zip(censo.iter());
    for (((l, iv), an), c) in filas {
        iv.nacimientos += l.nacimientos;
        iv.concepciones += l.concepciones;
        iv.abortos += l.abortos;
        iv.destetes += l.destetes;
        iv.destetes_forzosos += l.destetes_forzosos;
        iv.crias_destetadas += l.crias_destetadas;
        iv.consumo_kcal += l.consumo_kcal.valor();
        iv.rebrote_g += l.rebrote_g.valor();
        an.poblacion_dias += c.n;
        an.nacimientos += l.nacimientos;
        an.consumo_kcal += l.consumo_kcal.valor();
        an.rebrote_g += l.rebrote_g.valor();
        for (causa, &m) in l.muertes.iter().enumerate() {
            iv.muertes[causa] += m;
            an.muertes[causa] += m;
        }
    }
    for m in &libro.muertos {
        let k = m.esp as usize;
        let e = &cat.especies[k];
        let an = &mut reg.anual[k];
        an.muertes_clase[sexo_i(m.sexo)][e.clase(m.edad)] += 1;
        an.muertes_g[m.sexo.i()][grupo_mortalidad(m.edad, reg.limites[k])] += 1;
    }
    for p in &libro.partos {
        let k = p.esp as usize;
        let e = &cat.especies[k];
        let an = &mut reg.anual[k];
        an.nac_madre[e.clase(p.edad_madre)] += p.crias as u64;
        if let Some(i) = p.intervalo {
            an.intervalos_dias += i;
            an.intervalos_n += 1;
        }
        an.nac_grupo[grupo_fec(p.edad_madre, reg.ancho_fec, reg.n_grupos[k])] += p.crias as u64;
    }
    for f in &libro.fin_repro {
        if let Some(a) = f.edad_ultimo_parto {
            let an = &mut reg.anual[f.esp as usize];
            an.ultimo_parto_dias += u64::from(a);
            an.ultimo_parto_n += 1;
        }
    }

    // Balance del tick, acumulado en el intervalo.
    let b = &mut reg.balance;
    let delta = reservas.valor() - libro.reservas_inicio.valor();
    let (asim, rec, prop, pag, alm, exc, muer) = (
        libro.asimilada.valor(),
        libro.leche_recibida.valor(),
        libro.gasto_propio.valor(),
        libro.leche_pagada.valor(),
        libro.perdida_almacen.valor(),
        libro.excedente.valor(),
        libro.reservas_muertos.valor(),
    );
    b.asimilada += asim;
    b.leche_recibida += rec;
    b.gasto_propio += prop;
    b.leche_pagada += pag;
    b.perdida_almacen += alm;
    b.perdida_leche += pag - rec;
    b.excedente += exc;
    b.reservas_muertos += muer;
    b.delta_reservas += delta;
    b.residuo += delta - (asim + rec - prop - pag - alm - exc - muer);

    // Registro del intervalo.
    if (t + 1).is_multiple_of(reg.every) || t + 1 == reg.total {
        reg.ids.clear();
        reg.ids.extend(q.iter().map(|x| x.0.0));
        reg.ids.sort_unstable();
        let colisiones = reg.ids.windows(2).filter(|w| w[0] == w[1]).count() as u64;
        let rho = cat.forraje.rho_veg;
        let especies = (0..n_esp)
            .map(|k| {
                let c = &censo[k];
                let iv = &reg.intervalo[k];
                let (mut stock, mut tope) = (0.0, 0.0);
                for cel in 0..hab.n_celdas {
                    let i = hab.idx(cel, k);
                    stock += hab.v[i] * hab.area_m2[cel];
                    tope += hab.k[i] * hab.area_m2[cel];
                }
                let n = c.n.max(1) as f64;
                EspecieTick {
                    n: c.n,
                    hembras: c.hembras,
                    machos: c.machos,
                    dependientes: c.dependientes,
                    gestantes: c.gestantes,
                    lactantes: c.lactantes,
                    nacimientos: iv.nacimientos,
                    muertes_inanicion: iv.muertes[0],
                    muertes_edad_max: iv.muertes[1],
                    muertes_intrinseca: iv.muertes[2],
                    muertes_orfandad: iv.muertes[3],
                    concepciones: iv.concepciones,
                    abortos: iv.abortos,
                    destetes: iv.destetes,
                    condicion_media: if c.n > 0 {
                        c.condicion.valor() / n
                    } else {
                        0.0
                    },
                    frac_deficit: libro.esp[k].en_deficit as f64 / n,
                    reservas_kcal: c.reservas.valor(),
                    forraje_kg: stock / 1000.0,
                    forraje_max_kg: tope / 1000.0,
                    consumo_kg: iv.consumo_kcal / rho / 1000.0,
                    rebrote_kg: iv.rebrote_g / 1000.0,
                    destetes_forzosos: iv.destetes_forzosos,
                    crias_destetadas_forzoso: iv.crias_destetadas,
                    muertes_destete: iv.muertes[4],
                    condicion_lactantes: if c.lactantes > 0 {
                        c.condicion_lactantes.valor() / c.lactantes as f64
                    } else {
                        0.0
                    },
                }
            })
            .collect();
        let balance = std::mem::take(&mut reg.balance);
        reg.ticks.push(BioTick {
            tick: t,
            especies,
            balance,
            colisiones_id: colisiones,
        });
        reg.intervalo
            .iter_mut()
            .for_each(|x| *x = Intervalo::default());
    }

    // Resumen anual.
    if (t + 1).is_multiple_of(reg.dias_anio) {
        let mut piramides: Vec<[Vec<u64>; 2]> = cat
            .especies
            .iter()
            .map(|e| [vec![0; e.n_clases()], vec![0; e.n_clases()]])
            .collect();
        for (_, esp, sexo, edad, _, _, _, _, _) in q.iter() {
            let e = &cat.especies[esp.0 as usize];
            piramides[esp.0 as usize][sexo_i(*sexo)][e.clase(edad.0)] += 1;
        }
        let rho = cat.forraje.rho_veg;
        let especies = (0..n_esp)
            .map(|k| {
                let e = &cat.especies[k];
                let an = &reg.anual[k];
                let w = e.delta_a as f64 / da;
                let media = an.poblacion_dias as f64 / da;
                let expo: Vec<u64> = (0..e.n_clases())
                    .map(|x| an.expo[0][x] + an.expo[1][x])
                    .collect();
                let muertes: Vec<u64> = (0..e.n_clases())
                    .map(|x| an.muertes_clase[0][x] + an.muertes_clase[1][x])
                    .collect();
                let tgf = if an.expo[0].iter().any(|&x| x > 0) {
                    Some(
                        (0..e.n_clases())
                            .filter(|&x| an.expo[0][x] > 0)
                            .map(|x| an.nac_madre[x] as f64 / (an.expo[0][x] as f64 / da) * w)
                            .sum(),
                    )
                } else {
                    None
                };
                let muertes_total: u64 = an.muertes.iter().sum();
                let [pir_h, pir_m] = std::mem::take(&mut piramides[k]);
                let tasa = |muertes: u64, expo: u64| {
                    if expo > 0 {
                        muertes as f64 / (expo as f64 / da)
                    } else {
                        0.0
                    }
                };
                let mortalidad = |s: usize| -> [f64; 4] {
                    std::array::from_fn(|g| tasa(an.muertes_g[s][g], an.expo_g[s][g]))
                };
                EspecieAnio {
                    poblacion_media: media,
                    densidad_km2: media / reg.area_km2,
                    n_max_km2: reg.n_max[k] / reg.area_km2,
                    nacimientos: an.nacimientos,
                    muertes_inanicion: an.muertes[0],
                    muertes_edad_max: an.muertes[1],
                    muertes_intrinseca: an.muertes[2],
                    muertes_orfandad: an.muertes[3],
                    tasa_natalidad_mil: if media > 0.0 {
                        1000.0 * an.nacimientos as f64 / media
                    } else {
                        0.0
                    },
                    tasa_mortalidad_mil: if media > 0.0 {
                        1000.0 * muertes_total as f64 / media
                    } else {
                        0.0
                    },
                    e0_anios: e0_periodo(&expo, &muertes, w, da),
                    tgf,
                    intervalo_partos_dias: (an.intervalos_n > 0)
                        .then(|| an.intervalos_dias as f64 / an.intervalos_n as f64),
                    presion_herbivoria: (an.rebrote_g > 0.0)
                        .then(|| an.consumo_kcal / rho / an.rebrote_g),
                    piramide_hembras: pir_h,
                    piramide_machos: pir_m,
                    muertes_destete: an.muertes[4],
                    fecundidad_grupos: an
                        .nac_grupo
                        .iter()
                        .zip(&an.expo_hembras_grupo)
                        .map(|(&n, &x)| tasa(n, x))
                        .collect(),
                    n_ultimo_parto: an.ultimo_parto_n,
                    edad_ultimo_parto_anios: (an.ultimo_parto_n > 0)
                        .then(|| an.ultimo_parto_dias as f64 / an.ultimo_parto_n as f64 / da),
                    mortalidad_hembras: mortalidad(0),
                    mortalidad_machos: mortalidad(1),
                }
            })
            .collect();
        reg.anios.push(BioAnio {
            anio: (t + 1) / reg.dias_anio,
            especies,
        });
        let nuevos: Vec<Anual> = cat
            .especies
            .iter()
            .zip(&reg.n_grupos)
            .map(|(e, &g)| Anual::nuevo(e.n_clases(), g))
            .collect();
        reg.anual = nuevos;
    }
}

#[cfg(test)]
mod tests {
    use super::e0_periodo;

    #[test]
    fn tabla_de_vida_del_periodo_con_mortalidad_constante() {
        // Con m constante, e0 = 1/m.
        let m = 0.1; // 1/año
        let n = 400;
        let expo: Vec<u64> = (0..n)
            .map(|x| (1e6 * (-m * x as f64).exp()) as u64 * 365)
            .collect();
        let muertes: Vec<u64> = expo
            .iter()
            .map(|&e| (m * e as f64 / 365.0).round() as u64)
            .collect();
        let e0 = e0_periodo(&expo, &muertes, 1.0, 365.0).unwrap();
        assert!((e0 - 10.0).abs() < 0.05, "e0 {e0}");
        assert_eq!(e0_periodo(&[0, 0], &[0, 0], 1.0, 365.0), None);
    }
}
