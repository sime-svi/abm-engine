//! Puntos calientes, mesetas oceánicas y trazas volcánicas (nota de puntos calientes).
//!
//! Los puntos calientes están fijos en el marco del manto: el de la malla del mundo, en el que las
//! velocidades de las placas no tienen rotación neta (`velocidades`). Cada uno vive `T_hs`; al morir
//! nace otro en un sitio al azar y, con probabilidad `f_cabeza`, empieza con una gran erupción.
//! - **Gran erupción** (6.2): durante `Δt_LIP`, en un disco de área `A_LIP`, añade `h_LIP` de corteza
//!   flotante a las columnas oceánicas (una meseta) y `h_CFB` a las continentales (un basalto de
//!   inundación, que reinicia su edad tectonotérmica).
//! - **Traza** (6.3): cada paso, `q·Δt` repartido en las columnas oceánicas a menos de `r_tr`: la
//!   placa que pasa por encima va dibujando la cadena. Es corteza oceánica que se hunde en las fosas
//!   (montes submarinos) salvo donde la columna pasa del umbral de flotación (`h_cont_min`): allí toda
//!   la que añadió el punto caliente pasa a flotar.
//! - **Abombamiento** (6.4): `U(d) = H·cos²(π·d/W)` sobre la litosfera oceánica a menos de `W/2`,
//!   como una masa de menos en el manto que sigue al punto caliente, no a la placa.
//!
//! La corteza flotante de las mesetas lleva fracción de arco 1 (la «corteza flotante no
//! continental» de la nota, §3): en las fosas y los choques se acreciona con `η_arco` (§6.5), como la
//! de los arcos. Las columnas oceánicas no pasan a continentales por ella: con la isostasia de los
//! continentes (R4), una meseta de 22 km quedaría a ~0,6–0,9 km bajo el mar; con la del océano, a
//! ~2,6 km, como las reales (§11.2 de la nota).

use super::*;

/// Una gran erupción en curso: celdas del mundo de su disco, Myr que le quedan, área (km²) y si es
/// sobre todo oceánica.
#[derive(Debug, Clone)]
pub(super) struct Erupcion {
    celdas: Vec<u32>,
    restante_myr: f64,
    area_km2: f64,
}

/// Un punto caliente (nota de puntos calientes §3).
#[derive(Debug, Clone)]
pub struct PuntoCaliente {
    pub id: u64,
    /// Posición en el marco del manto.
    pub dir: V3,
    pub edad_myr: f64,
    pub vida_myr: f64,
    /// Altura y anchura de su abombamiento (km).
    pub h_sw_km: f64,
    pub w_sw_km: f64,
    /// Celdas del mundo de su traza y de su abombamiento (con su `U`, m).
    traza: Vec<u32>,
    abombamiento: Vec<(u32, f64)>,
    erupcion: Option<Erupcion>,
}

/// Lo que añaden los puntos calientes en el paso (km³): trazas (todo y la parte que flota),
/// mesetas y basaltos continentales.
#[derive(Debug, Clone, Copy, Default)]
pub struct IntraplacaPaso {
    pub trazas_km3: f64,
    pub trazas_flotante_km3: f64,
    pub mesetas_km3: f64,
    pub cfb_km3: f64,
}

impl PuntoCaliente {
    /// ¿Tiene una gran erupción en curso?
    pub fn erupcion_en_curso(&self) -> bool {
        self.erupcion.is_some()
    }
}

impl Deriva {
    /// Medidas del océano (nota de puntos calientes §8): mesetas oceánicas de más de 0,2 Mkm²
    /// (regiones conexas de columnas oceánicas con al menos 3 km de corteza flotante, hecha sobre todo
    /// por el magmatismo intraplaca: cuántas y su área), profundidad media y mediana del océano bajo
    /// el nivel del mar (m) e islas volcánicas emergidas (columnas oceánicas sobre el mar hechas sobre
    /// todo por el magmatismo intraplaca, km²).
    pub(super) fn medidas_del_oceano(&self) -> (u32, f64, f64, f64, f64) {
        let num = self.malla.num;
        let z = self.alturas();
        let zeta = self.zeta_m;
        let meseta: Vec<bool> = (0..num)
            .map(|k| {
                self.corteza(k).is_some_and(|c| {
                    let tot: f32 = c.proc.vias.iter().sum();
                    !c.cont && c.hc >= 3.0 && tot > 0.0 && 2.0 * c.proc.vias[via::INTRAPLACA] >= tot
                })
            })
            .collect();
        let mut visto = vec![false; num];
        let (mut n, mut area) = (0u32, 0.0);
        for k0 in 0..num {
            if !meseta[k0] || visto[k0] {
                continue;
            }
            visto[k0] = true;
            let mut pila = vec![k0 as u32];
            let mut a = 0.0;
            while let Some(k) = pila.pop() {
                a += self.malla.area[k as usize];
                for &v in &self.malla.vec[k as usize] {
                    if v != NADA && meseta[v as usize] && !visto[v as usize] {
                        visto[v as usize] = true;
                        pila.push(v);
                    }
                }
            }
            if a >= 0.2e6 {
                n += 1;
                area += a;
            }
        }
        let mut prof: Vec<(f64, f64)> = Vec::new();
        let mut islas = 0.0;
        for (k, &zk) in z.iter().enumerate() {
            let Some(c) = self.corteza(k) else { continue };
            if c.cont {
                continue;
            }
            if zk < zeta {
                prof.push((zeta - zk, self.malla.area[k]));
            } else {
                // Islas volcánicas: las columnas oceánicas emergidas que hizo sobre todo el
                // magmatismo intraplaca (no los arcos ni el sedimento).
                let tot: f32 = c.proc.vias.iter().sum();
                if tot > 0.0 && 2.0 * c.proc.vias[via::INTRAPLACA] >= tot {
                    islas += self.malla.area[k];
                }
            }
        }
        prof.sort_by(|a, b| a.0.total_cmp(&b.0));
        let a_tot: f64 = prof.iter().map(|x| x.1).sum();
        let media = if a_tot > 0.0 {
            prof.iter().map(|x| x.0 * x.1).sum::<f64>() / a_tot
        } else {
            0.0
        };
        let mut acum = 0.0;
        let mut mediana = 0.0;
        for &(d, a) in &prof {
            acum += a;
            if acum >= 0.5 * a_tot {
                mediana = d;
                break;
            }
        }
        (n, area, media, mediana, islas)
    }

    /// Celdas del mundo a menos de `r_km` de `centro` (búsqueda en anchura por la malla), con su
    /// distancia (km), en el orden de la búsqueda.
    fn celdas_a_menos_de(&self, centro: V3, r_km: f64) -> Vec<(u32, f64)> {
        let r = self.malla.r_km;
        let k0 = self.malla.localizar(centro) as u32;
        let dist = |k: u32| {
            r * dot(self.malla.dir[k as usize], centro)
                .clamp(-1.0, 1.0)
                .acos()
        };
        let mut visto = std::collections::BTreeSet::new();
        let mut cola = std::collections::VecDeque::new();
        let mut fuera = Vec::new();
        visto.insert(k0);
        cola.push_back(k0);
        while let Some(k) = cola.pop_front() {
            let d = dist(k);
            if d > r_km && k != k0 {
                continue;
            }
            fuera.push((k, d));
            for &v in &self.malla.vec[k as usize] {
                if v != NADA && visto.insert(v) {
                    cola.push_back(v);
                }
            }
        }
        fuera.retain(|&(_, d)| d <= r_km.max(0.5 * self.malla.lado_km));
        fuera
    }

    /// Nace un punto caliente (nota §6.1): posición al azar en la esfera, abombamiento al azar en
    /// sus rangos, edad `edad0` y, si `con_cabeza`, con probabilidad `f_cabeza` una gran erupción.
    fn nacer_punto_caliente(&mut self, edad0: f64, con_cabeza: bool) -> PuntoCaliente {
        let cfg = &self.cfg;
        let id = self.puntos_calientes_nacidos;
        self.puntos_calientes_nacidos += 1;
        let u = |j: u64| azar(self.seed, self.paso, id, P_CALIENTE, j);
        let dir = dir_uniforme(u(0), u(1));
        let h_sw = cfg.h_sw_min_km + (cfg.h_sw_max_km - cfg.h_sw_min_km) * u(2);
        let w_sw = cfg.w_sw_min_km + (cfg.w_sw_max_km - cfg.w_sw_min_km) * u(3);
        let traza = self
            .celdas_a_menos_de(dir, cfg.r_traza_km)
            .into_iter()
            .map(|x| x.0)
            .collect();
        let abombamiento = self
            .celdas_a_menos_de(dir, 0.5 * w_sw)
            .into_iter()
            .map(|(k, d)| {
                let c = (std::f64::consts::PI * d / w_sw).cos();
                (k, 1000.0 * h_sw * c * c)
            })
            .collect();
        // f_cabeza = λ_LIP·T_hs/N_hs (§5): así salen λ_LIP erupciones por Myr.
        let f_cabeza = if cfg.n_hs > 0 && cfg.lip_cada_myr > 0.0 {
            (cfg.t_hs_myr / cfg.lip_cada_myr / cfg.n_hs as f64).min(1.0)
        } else {
            0.0
        };
        let erupcion = (con_cabeza && u(4) < f_cabeza).then(|| {
            // Área log-uniforme entre A_min y A_max (Mkm²).
            let (a0, a1) = (cfg.a_lip_min_mkm2.ln(), cfg.a_lip_max_mkm2.ln());
            let area = 1e6 * (a0 + (a1 - a0) * u(5)).exp();
            let r = self.malla.r_km;
            // Radio de un casquete de esa área.
            let radio = r * (1.0 - area / (2.0 * std::f64::consts::PI * r * r)).acos();
            let celdas = self
                .celdas_a_menos_de(dir, radio)
                .into_iter()
                .map(|x| x.0)
                .collect();
            Erupcion {
                celdas,
                restante_myr: cfg.dt_lip_myr,
                area_km2: area,
            }
        });
        PuntoCaliente {
            id,
            dir,
            edad_myr: edad0,
            vida_myr: cfg.t_hs_myr,
            h_sw_km: h_sw,
            w_sw_km: w_sw,
            traza,
            abombamiento,
            erupcion,
        }
    }

    /// Los puntos calientes de partida: `N_hs`, con edades al azar en [0, T_hs) y sin gran erupción
    /// en curso.
    pub(super) fn puntos_calientes_iniciales(&mut self) {
        for i in 0..self.cfg.n_hs {
            let edad = self.cfg.t_hs_myr * azar(self.seed, 0, i as u64, P_CALIENTE_EDAD, 0);
            let pc = self.nacer_punto_caliente(edad, false);
            self.puntos_calientes.push(pc);
        }
        self.rehacer_abombamiento();
    }

    /// Pone un punto caliente en `dir` (pruebas): sin gran erupción y con el abombamiento dado.
    pub fn poner_punto_caliente(&mut self, dir: V3, h_sw_km: f64, w_sw_km: f64) {
        let mut pc = self.nacer_punto_caliente(0.0, false);
        pc.dir = normalize(dir);
        pc.h_sw_km = h_sw_km;
        pc.w_sw_km = w_sw_km;
        pc.traza = self
            .celdas_a_menos_de(pc.dir, self.cfg.r_traza_km)
            .into_iter()
            .map(|x| x.0)
            .collect();
        pc.abombamiento = self
            .celdas_a_menos_de(pc.dir, 0.5 * w_sw_km)
            .into_iter()
            .map(|(k, d)| {
                let c = (std::f64::consts::PI * d / w_sw_km).cos();
                (k, 1000.0 * h_sw_km * c * c)
            })
            .collect();
        self.puntos_calientes.push(pc);
        self.rehacer_abombamiento();
    }

    /// El campo de abombamiento de todos los puntos calientes vivos (m, por celda del mundo).
    fn rehacer_abombamiento(&mut self) {
        self.abombamiento_m.iter_mut().for_each(|x| *x = 0.0);
        for pc in &self.puntos_calientes {
            for &(k, u) in &pc.abombamiento {
                self.abombamiento_m[k as usize] += u;
            }
        }
    }

    /// Vida de los puntos calientes (nota §6.1), al principio del paso: envejecen; el que llega a su
    /// vida muere y nace otro.
    pub(super) fn vida_de_puntos_calientes(&mut self) {
        let dt = self.cfg.dt_myr;
        let mut cambio = false;
        for i in 0..self.puntos_calientes.len() {
            self.puntos_calientes[i].edad_myr += dt;
            if self.puntos_calientes[i].edad_myr < self.puntos_calientes[i].vida_myr {
                continue;
            }
            let viejo = self.puntos_calientes[i].dir;
            self.evento_det(
                "muerte_punto_caliente",
                vec![],
                0.0,
                0.0,
                Some(viejo),
                Detalle::default(),
            );
            let nuevo = self.nacer_punto_caliente(0.0, true);
            let (dir, erupcion) = (nuevo.dir, nuevo.erupcion.as_ref().map(|e| e.area_km2));
            self.puntos_calientes[i] = nuevo;
            self.evento_det(
                "nacimiento_punto_caliente",
                vec![],
                0.0,
                erupcion.unwrap_or(0.0),
                Some(dir),
                Detalle {
                    clase: Some(if erupcion.is_some() {
                        "con_gran_erupcion".to_string()
                    } else {
                        "sin_gran_erupcion".to_string()
                    }),
                    ..Detalle::default()
                },
            );
            cambio = true;
        }
        if cambio {
            self.rehacer_abombamiento();
        }
    }

    /// Las muestras distintas que ven las celdas del mundo `celdas`: (placa, celda local).
    fn muestras_de(&self, celdas: &[u32]) -> Vec<(u32, u32)> {
        let mut m: Vec<(u32, u32)> = celdas
            .iter()
            .filter_map(|&k| {
                let (p, l) = (self.dueno[k as usize], self.rep[k as usize]);
                (p != NADA && l != NADA && self.placas[p as usize].get(l).is_some())
                    .then_some((p, l))
            })
            .collect();
        m.sort_unstable();
        m.dedup();
        m
    }

    /// Suma `dh` km de corteza flotante no continental (fracción de arco 1) a una columna oceánica,
    /// sin pasarla a continental (nota §6.2 y §6.3).
    fn sumar_flotante_oceanica(c: &mut Corteza, dh: f64) {
        let v_arco = c.arco * c.hc + dh;
        c.h += dh;
        c.hc += dh;
        c.arco = (v_arco / c.hc).clamp(0.0, 1.0);
        c.proc.vias[via::INTRAPLACA] += dh as f32;
    }

    /// Grandes erupciones y trazas del paso (nota §6.2 y §6.3), después de mover las placas y antes
    /// de los procesos de la adenda. Devuelve lo añadido.
    pub(super) fn magmatismo_intraplaca(&mut self) -> IntraplacaPaso {
        let mut paso = IntraplacaPaso::default();
        if self.puntos_calientes.is_empty() {
            return paso;
        }
        let (dt, dt_lip, h_cfb, h_lip) = (
            self.cfg.dt_myr,
            self.cfg.dt_lip_myr,
            self.cfg.h_cfb_km,
            self.cfg.h_lip_km,
        );
        let (rs, rc, h_cont_min) = (self.cfg.rho_s, self.cfg.rho_c, self.cfg.h_cont_min_km);
        let (q, h_oc) = (self.cfg.q_traza_km3_myr, self.cfg.h_oc_km);
        for i in 0..self.puntos_calientes.len() {
            // Gran erupción en curso.
            if let Some(e) = self.puntos_calientes[i].erupcion.clone() {
                let f = (dt / dt_lip).min(e.restante_myr / dt_lip);
                let mut reinicio = 0.0;
                for (p, l) in self.muestras_de(&e.celdas) {
                    let a = self.malla.area[l as usize];
                    let Some(c) = self.placas[p as usize].get_mut(l) else {
                        continue;
                    };
                    if c.cont {
                        let dh = h_cfb * f;
                        c.sumar_hc(dh, 0.0, Origen::NUEVA, rs, rc, h_cont_min, via::INTRAPLACA);
                        c.edad_tt = 0.0;
                        c.beta = 1.0;
                        c.causa_tt = reinicio::ERUPCION;
                        paso.cfb_km3 += dh * a;
                        reinicio += a;
                    } else {
                        let dh = h_lip * f;
                        Self::sumar_flotante_oceanica(c, dh);
                        paso.mesetas_km3 += dh * a;
                    }
                }
                self.reinicio_km2_paso[4] += reinicio;
                let pc = &mut self.puntos_calientes[i];
                if let Some(er) = pc.erupcion.as_mut() {
                    er.restante_myr -= dt;
                    if er.restante_myr <= 1e-9 {
                        pc.erupcion = None;
                    }
                }
                if e.restante_myr >= dt_lip - 1e-9 {
                    // Primer paso de la erupción: el evento, con su área y su clase.
                    let dir = self.puntos_calientes[i].dir;
                    let oc = self
                        .muestras_de(&e.celdas)
                        .iter()
                        .filter(|&&(p, l)| self.placas[p as usize].get(l).is_some_and(|c| !c.cont))
                        .count();
                    let n = e.celdas.len().max(1);
                    self.evento_det(
                        "gran_erupcion",
                        vec![],
                        0.0,
                        e.area_km2,
                        Some(dir),
                        Detalle {
                            clase: Some(if 2 * oc >= n {
                                "oceanica".to_string()
                            } else {
                                "continental".to_string()
                            }),
                            ..Detalle::default()
                        },
                    );
                }
            }
            // Traza: q·Δt repartido en las columnas oceánicas a menos de r_tr.
            let traza = self.puntos_calientes[i].traza.clone();
            let oceanicas: Vec<(u32, u32)> = self
                .muestras_de(&traza)
                .into_iter()
                .filter(|&(p, l)| self.placas[p as usize].get(l).is_some_and(|c| !c.cont))
                .collect();
            let a_tot: f64 = oceanicas
                .iter()
                .map(|&(_, l)| self.malla.area[l as usize])
                .sum();
            if a_tot <= 0.0 || q <= 0.0 {
                continue;
            }
            let dh = q * dt / a_tot;
            for (p, l) in oceanicas {
                let a = self.malla.area[l as usize];
                let Some(c) = self.placas[p as usize].get_mut(l) else {
                    continue;
                };
                paso.trazas_km3 += dh * a;
                if c.h + dh > h_cont_min {
                    // Pasa del umbral de flotación: toda la corteza que añadió el punto caliente
                    // (la de los montes submarinos de antes y la de ahora) flota.
                    let previa = (c.h - h_oc - c.hc).max(0.0);
                    c.h -= previa;
                    Self::sumar_flotante_oceanica(c, previa + dh);
                    paso.trazas_flotante_km3 += (previa + dh) * a;
                } else {
                    c.h += dh;
                }
            }
        }
        paso
    }
}
