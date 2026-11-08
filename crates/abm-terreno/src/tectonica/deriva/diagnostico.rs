//! Diagnóstico del supercontinente (pedido del responsable el 2026-11-08): una foto cada 25 Myr de
//! cuánto se juntan los continentes, de los océanos que abren los rifts, de las fuerzas que mueven las
//! placas con continente y de si los continentes van hacia las fosas.
//!
//! Solo mide: no cambia ningún estado que lea el modelo (el campo de distancias es un búfer de
//! trabajo que se toma y se devuelve) ni consume números del azar.

use super::*;
use crate::tectonica::historia::{MargenRift, OceanoRift, PlacaConContinente};

/// Myr entre dos fotos.
const CADA_MYR: f64 = 25.0;
/// Masa continental mínima para la distancia entre masas (fracción del continente).
const MASA_MIN_FRAC: f64 = 0.01;
/// Continente mínimo que lleva una placa para entrar en la foto (fracción del continente).
const PLACA_CONT_MIN_FRAC: f64 = 0.05;

impl Deriva {
    /// Si toca, añade la foto del paso a `fotos_supercontinente` (al final de las métricas del paso).
    pub(super) fn foto_supercontinente_si_toca(&mut self) {
        let cada = (CADA_MYR / self.cfg.dt_myr).round().max(1.0) as u64;
        if !self.paso.is_multiple_of(cada) {
            return;
        }
        let foto = self.foto_supercontinente();
        self.fotos_supercontinente.push(foto);
    }

    fn foto_supercontinente(&mut self) -> FotoSupercontinente {
        let num = self.malla.num;
        let r = self.malla.r_km;
        let cont: Vec<bool> = (0..num)
            .map(|k| self.corteza(k).is_some_and(|c| c.cont))
            .collect();
        let total: f64 = (0..num)
            .filter(|&k| cont[k])
            .map(|k| self.malla.area[k])
            .sum();
        let mut foto = FotoSupercontinente {
            myr: self.t_myr,
            masa_mayor_frac: self.indice_supercontinente(),
            bloque_mayor_frac: self.metricas_de_bloques(&self.celdas_continente()).1,
            ..FotoSupercontinente::default()
        };
        if total <= 0.0 {
            return foto;
        }
        // Velocidad de cada celda con su dueño (km/Myr).
        let vel = |d: &Deriva, k: usize| -> V3 {
            let p = d.dueno[k];
            if p == NADA {
                [0.0; 3]
            } else {
                velocidad(d.placas[p as usize].omega, d.malla.dir[k], r)
            }
        };

        // 1. Masas conexas (por encima de los bordes de placa) y su distancia.
        let mut visto = vec![false; num];
        let mut pila: Vec<u32> = Vec::new();
        let mut masas: Vec<(f64, V3, V3)> = Vec::new(); // (área, Σ a·x, Σ a·v)
        for k0 in 0..num {
            if !cont[k0] || visto[k0] {
                continue;
            }
            visto[k0] = true;
            pila.push(k0 as u32);
            let (mut a, mut sx, mut sv) = (0.0, [0.0; 3], [0.0; 3]);
            while let Some(k) = pila.pop() {
                let ku = k as usize;
                let ak = self.malla.area[ku];
                a += ak;
                sx = add(sx, esc(self.malla.dir[ku], ak));
                sv = add(sv, esc(vel(self, ku), ak));
                for &v in &self.malla.vec[ku] {
                    if v != NADA && cont[v as usize] && !visto[v as usize] {
                        visto[v as usize] = true;
                        pila.push(v);
                    }
                }
            }
            if a >= MASA_MIN_FRAC * total {
                masas.push((a, sx, sv));
            }
        }
        foto.masas = masas.len() as u32;
        let centros: Vec<(f64, V3, V3)> = masas
            .iter()
            .map(|&(a, sx, sv)| {
                let c = normalize(sx);
                let v = esc(sv, 1.0 / a);
                // Velocidad del centro, en el plano tangente.
                (a, c, sub(v, esc(c, dot(v, c))))
            })
            .collect();
        let (mut sw, mut sd, mut sr, mut sa) = (0.0, 0.0, 0.0, 0.0);
        for i in 0..centros.len() {
            for j in i + 1..centros.len() {
                let (ai, ci, vi) = centros[i];
                let (aj, cj, vj) = centros[j];
                let cos = dot(ci, cj).clamp(-1.0, 1.0);
                let th = cos.acos();
                let sin = th.sin();
                if sin < 1e-9 {
                    continue;
                }
                let w = ai * aj;
                // d(R·θ)/dt = −(v_i·c_j + v_j·c_i)/sin θ (con v en km/Myr).
                let ritmo = -(dot(vi, cj) + dot(vj, ci)) / sin;
                sw += w;
                sd += w * r * th;
                sr += w * ritmo;
                if ritmo < 0.0 {
                    sa += w;
                }
            }
        }
        if sw > 0.0 {
            foto.dist_media_km = sd / sw;
            foto.dist_ritmo_km_myr = sr / sw;
            foto.pares_acercandose_frac = sa / sw;
        }

        // 2. Océanos de rift: los dos márgenes de cada rift ya roto.
        let mut activo = vec![false; num];
        for b in &self.bordes {
            if matches!(
                b.tipo,
                TipoBorde::Subduccion | TipoBorde::Difusa | TipoBorde::Colision
            ) {
                for &j in self.vecindad.de(b.k as usize) {
                    activo[j as usize] = true;
                }
            }
        }
        // El margen pasivo de la foto anterior, 25 Myr después: sigue pasivo, es activo por una fosa
        // o una convergencia difusa cerca (invasión) o por un choque, ya no es margen sin borde
        // convergente cerca, o su muestra ya no está.
        let mut choque = vec![false; num];
        for b in &self.bordes {
            if b.tipo == TipoBorde::Colision {
                for &j in self.vecindad.de(b.k as usize) {
                    choque[j as usize] = true;
                }
            }
        }
        let previas = std::mem::take(&mut self.margenes_foto);
        let ahora: Vec<(u64, u32)> = self.margenes_previos.iter().map(|x| x.0).collect();
        if !previas.is_empty() {
            let mut cuenta = [0.0; 5];
            for &(id, l) in &previas {
                let i = if ahora.binary_search(&(id, l)).is_ok() {
                    0
                } else {
                    let k = self.placas.iter().filter(|p| p.viva).find_map(|p| {
                        let s = p.mapa[l as usize];
                        (s != NADA && (p.id == id || p.ancestros.contains(&id)))
                            .then(|| p.mundo[s as usize] as usize)
                    });
                    match k {
                        Some(k) if choque[k] => 2,
                        Some(k) if activo[k] => 1,
                        Some(_) => 3,
                        None => 4,
                    }
                };
                cuenta[i] += 1.0;
            }
            let n = previas.len() as f64;
            foto.margen_tras_25myr_frac = cuenta.map(|x| x / n);
        }
        self.margenes_foto = ahora;
        let mut lados: std::collections::BTreeMap<u32, [Vec<u32>; 2]> = Default::default();
        for (k, &es) in cont.iter().enumerate() {
            if !es {
                continue;
            }
            if let Some(c) = self.corteza(k)
                && c.rift_id != 0
            {
                lados.entry(c.rift_id >> 1).or_default()[(c.rift_id & 1) as usize].push(k as u32);
            }
        }
        for (rid, l) in &lados {
            let rotura = self
                .rotura_rift_myr
                .get(*rid as usize)
                .copied()
                .unwrap_or(f64::NAN);
            if rotura.is_nan() || l[0].is_empty() || l[1].is_empty() {
                continue;
            }
            // Distancia de cada celda de un lado a la más cercana del otro (km).
            let cercanas = |de: &[u32], a: &[u32]| -> Vec<f64> {
                de.iter()
                    .map(|&k| {
                        let x = self.malla.dir[k as usize];
                        let c = a
                            .iter()
                            .map(|&j| dot(x, self.malla.dir[j as usize]))
                            .fold(-1.0f64, f64::max);
                        r * c.clamp(-1.0, 1.0).acos()
                    })
                    .collect()
            };
            let mediana = |mut v: Vec<f64>| -> (f64, f64) {
                v.sort_by(f64::total_cmp);
                (v[v.len() / 2], v[0])
            };
            let (m0, n0) = mediana(cercanas(&l[0], &l[1]));
            let (m1, n1) = mediana(cercanas(&l[1], &l[0]));
            let frac_activo = |cel: &[u32]| {
                cel.iter().filter(|&&k| activo[k as usize]).count() as f64 / cel.len() as f64
            };
            foto.oceanos.push(OceanoRift {
                rift: *rid,
                rotura_myr: rotura,
                ancho_km: 0.5 * (m0 + m1),
                ancho_min_km: n0.min(n1),
                celdas: [l[0].len() as u32, l[1].len() as u32],
                activo_frac: [frac_activo(&l[0]), frac_activo(&l[1])],
            });
        }

        // 3. El continente frente a la fosa activa más cercana: las celdas del lado que se hunde de
        // cada fosa, con su par de placas.
        let fosas: Vec<(u32, u32, u32)> = self
            .bordes
            .iter()
            .filter(|b| b.tipo == TipoBorde::Subduccion && b.p_subduce)
            .map(|b| (b.k, b.p, b.q))
            .collect();
        let n_placas = self.placas.len();
        // Por placa: área, continente, Σ a·|v|², y Σ a·(v·û) y Σ a·[v·û > 0] de su continente.
        let mut por_placa = vec![(0.0, 0.0, 0.0, 0.0, 0.0); n_placas];
        let (mut hacia, mut v_hacia, mut hundiendo, mut cabalgando) = (0.0, 0.0, 0.0, 0.0);
        let campo_listo = if fosas.is_empty() {
            None
        } else {
            let fuentes: Vec<(u32, f64, u32)> = fosas
                .iter()
                .enumerate()
                .map(|(i, f)| (f.0, 0.0, i as u32))
                .collect();
            let mut campo = std::mem::take(&mut self.campo);
            campo.calcular(&self.malla, &fuentes, |_, _| true, f64::INFINITY);
            Some(campo)
        };
        for (k, &es_cont) in cont.iter().enumerate() {
            let p = self.dueno[k];
            if p == NADA {
                continue;
            }
            let a = self.malla.area[k];
            let v = vel(self, k);
            let e = &mut por_placa[p as usize];
            e.0 += a;
            e.2 += a * dot(v, v);
            if !es_cont {
                continue;
            }
            e.1 += a;
            let Some(campo) = campo_listo.as_ref() else {
                continue;
            };
            let et = campo.fuente[k];
            if et == NADA || et as usize >= fosas.len() {
                continue;
            }
            let (kf, pf, qf) = fosas[et as usize];
            let x = self.malla.dir[k];
            let xf = self.malla.dir[kf as usize];
            let t = sub(xf, esc(x, dot(xf, x)));
            let nt = dot(t, t).sqrt();
            let vt = if nt > 1e-12 {
                dot(v, esc(t, 1.0 / nt)) / CM_ANIO
            } else {
                0.0
            };
            e.3 += a * vt;
            if vt > 0.0 {
                e.4 += a;
                hacia += a;
            }
            v_hacia += a * vt;
            if pf == p {
                hundiendo += a;
            } else if qf == p {
                cabalgando += a;
            }
        }
        if let Some(campo) = campo_listo {
            self.campo = campo;
        }
        foto.hacia_fosa_frac = hacia / total;
        foto.v_hacia_fosa_cm_anio = v_hacia / total;
        foto.fosa_propia_hundiendose_frac = hundiendo / total;
        foto.fosa_propia_cabalgando_frac = cabalgando / total;

        // 4. Las placas que llevan continente, con sus fuerzas.
        let a_tot = self.malla.area_total();
        for (p, &(a, ac, v2, vt, ah)) in por_placa.iter().enumerate() {
            if !self.placas[p].viva || a <= 0.0 || ac < PLACA_CONT_MIN_FRAC * total {
                continue;
            }
            let potencias = self
                .potencias
                .iter()
                .find(|x| x.0 as usize == p)
                .map_or([0.0; 4], |x| x.2);
            foto.placas.push(PlacaConContinente {
                id: self.placas[p].id,
                area_frac: a / a_tot,
                continente_frac: ac / total,
                v_cm_anio: (v2 / a).sqrt() / CM_ANIO,
                potencias,
                v_hacia_fosa_cm_anio: vt / ac,
                hacia_fosa_frac: ah / ac,
            });
        }
        foto
    }
}

impl Deriva {
    /// Nace el margen pasivo `rift_id` (un lado de un rift que se rompe).
    pub(super) fn margen_nace(&mut self, rift_id: u32) {
        if self.margen_de_rift.contains_key(&rift_id) {
            return;
        }
        self.margen_de_rift
            .insert(rift_id, self.margenes_rift.len());
        self.margenes_rift.push(MargenRift {
            rift_id,
            rotura_myr: self.t_myr,
            celdas_max: 0,
            fin_myr: None,
            causa: None,
        });
    }

    /// Sigue los márgenes pasivos como entidades (cada paso): cuenta las celdas continentales de cada
    /// uno y las que tienen un borde convergente cerca (a menos de `L_borde`: fosa, convergencia
    /// difusa o choque). Un margen termina cuando la mitad de sus celdas tiene uno cerca (por choque
    /// si los choques cubren al menos la mitad de esas celdas; si no, por invasión) o cuando le queda
    /// menos del 10 % de las celdas que llegó a tener.
    pub(super) fn margenes_rift_paso(&mut self) {
        if self.margenes_rift.iter().all(|m| m.fin_myr.is_some()) {
            return;
        }
        let num = self.malla.num;
        // 0: nada cerca; 1: fosa o convergencia difusa; 2: choque.
        let mut cerca = vec![0u8; num];
        for b in &self.bordes {
            let tipo = match b.tipo {
                TipoBorde::Subduccion | TipoBorde::Difusa => 1,
                TipoBorde::Colision => 2,
                _ => continue,
            };
            for &j in self.vecindad.de(b.k as usize) {
                let c = &mut cerca[j as usize];
                *c = (*c).max(tipo);
            }
        }
        // (celdas, con fosa o difusa cerca, con choque cerca) de cada margen vivo.
        let mut cuenta: std::collections::BTreeMap<u32, (u32, u32, u32)> = Default::default();
        for (k, &ck) in cerca.iter().enumerate() {
            let Some(c) = self.corteza(k) else { continue };
            if !c.cont || c.rift_id == 0 {
                continue;
            }
            let Some(&i) = self.margen_de_rift.get(&c.rift_id) else {
                continue;
            };
            if self.margenes_rift[i].fin_myr.is_some() {
                continue;
            }
            let e = cuenta.entry(c.rift_id).or_default();
            e.0 += 1;
            match ck {
                1 => e.1 += 1,
                2 => e.2 += 1,
                _ => {}
            }
        }
        let t = self.t_myr;
        for m in self
            .margenes_rift
            .iter_mut()
            .filter(|m| m.fin_myr.is_none())
        {
            let (n, n_inv, n_cho) = cuenta.get(&m.rift_id).copied().unwrap_or((0, 0, 0));
            m.celdas_max = m.celdas_max.max(n);
            let activas = n_inv + n_cho;
            let causa = if n > 0 && 2 * activas >= n {
                Some(if 2 * n_cho >= activas {
                    "choque"
                } else {
                    "invasion"
                })
            } else if 10 * n < m.celdas_max {
                Some("desaparece")
            } else {
                None
            };
            if let Some(c) = causa {
                m.fin_myr = Some(t);
                m.causa = Some(c.to_string());
            }
        }
    }
}
