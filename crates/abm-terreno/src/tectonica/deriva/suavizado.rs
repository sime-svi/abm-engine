//! Borde suavizado de las placas (`nota_bordes_placa.md` §6.2–§6.5).
//!
//! Un borde dibujado con celdas tiene dientes del tamaño de una celda. La litosfera es rígida a
//! escalas de 100–200 km, así que el tipo de cada tramo se decide con el borde suavizado a `L_borde`:
//! - **pertenencia suavizada** `φ_p,i`: la fracción del área a menos de `L_borde` de la celda `i`
//!   que es de la placa `p` (una celda en solape cuenta la mitad para cada placa; un hueco, nada);
//! - **normal del borde suavizado**: la dirección en que más crece `φ_q − φ_p`, por mínimos
//!   cuadrados sobre las 8 vecinas;
//! - **tipo de tramo**: transformante si la velocidad relativa está a menos de `θ_tr` del borde;
//!   si no, convergente o divergente.
//!
//! Con él se resuelven los solapes y los huecos (§6.4) y se suaviza la pertenencia de los bordes
//! oceánicos (§6.5).

use super::*;

/// Vecindades de radio `L_borde` de cada celda del mundo (formato comprimido por filas).
#[derive(Default)]
pub(super) struct Vecindad {
    off: Vec<u32>,
    celdas: Vec<u32>,
}

impl Vecindad {
    /// Celdas cuyo centro está a menos de `radio_km` (por la superficie) del de cada celda.
    pub(super) fn nueva(malla: &Malla, radio_km: f64) -> Self {
        let cos_max = (radio_km / malla.r_km).cos();
        let mut off = Vec::with_capacity(malla.num + 1);
        let mut celdas = Vec::new();
        let mut visto = vec![u32::MAX; malla.num];
        let mut pila: Vec<u32> = Vec::new();
        off.push(0);
        for k in 0..malla.num {
            let x = malla.dir[k];
            pila.clear();
            pila.push(k as u32);
            visto[k] = k as u32;
            let ini = celdas.len();
            while let Some(c) = pila.pop() {
                celdas.push(c);
                for &v in &malla.vec[c as usize] {
                    if v == NADA || visto[v as usize] == k as u32 {
                        continue;
                    }
                    visto[v as usize] = k as u32;
                    if dot(x, malla.dir[v as usize]) >= cos_max {
                        pila.push(v);
                    }
                }
            }
            celdas[ini..].sort_unstable();
            off.push(celdas.len() as u32);
        }
        Self { off, celdas }
    }

    #[inline]
    pub(super) fn de(&self, k: usize) -> &[u32] {
        &self.celdas[self.off[k] as usize..self.off[k + 1] as usize]
    }
}

/// Tipo local de un tramo de borde (§6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TipoTramo {
    /// Sin borde suavizado (una sola placa alrededor).
    Ninguno,
    Convergente,
    Divergente,
    Transformante,
}

/// Pertenencia suavizada de una celda: las placas presentes a menos de `L_borde` y su fracción.
#[derive(Debug, Clone, Copy)]
pub(super) struct Fi {
    placa: [u32; 6],
    frac: [f64; 6],
    n: usize,
}

impl Fi {
    /// Fracción de la placa `p` (0 si no está).
    #[inline]
    pub(super) fn de(&self, p: u32) -> f64 {
        (0..self.n)
            .find(|&i| self.placa[i] == p)
            .map_or(0.0, |i| self.frac[i])
    }

    /// Placa con más fracción (desempate por índice de placa) y su fracción.
    pub(super) fn mayor(&self) -> Option<(u32, f64)> {
        (0..self.n)
            .map(|i| (self.placa[i], self.frac[i]))
            .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
    }
}

impl Deriva {
    /// Pertenencia suavizada de la celda `k`. Con `por_reclamo`, la de los reclamos tras mover las
    /// placas (una celda reclamada por `n` placas cuenta `1/n` para cada una; un hueco, nada); si
    /// no, la de los dueños ya resueltos.
    pub(super) fn fi(&self, k: usize, por_reclamo: bool) -> Fi {
        let mut f = Fi {
            placa: [NADA; 6],
            frac: [0.0; 6],
            n: 0,
        };
        let mut total = 0.0;
        let sumar = |f: &mut Fi, p: u32, a: f64| match (0..f.n).find(|&i| f.placa[i] == p) {
            Some(i) => f.frac[i] += a,
            None if f.n < 6 => {
                f.placa[f.n] = p;
                f.frac[f.n] = a;
                f.n += 1;
            }
            None => {}
        };
        for &j in self.vecindad.de(k) {
            let ju = j as usize;
            let a = self.malla.area[ju];
            total += a;
            if por_reclamo {
                let n = self.ncov[ju] as usize;
                for i in 0..n {
                    sumar(&mut f, self.cov[ju][i].0, a / n as f64);
                }
            } else {
                let p = self.dueno[ju];
                if p != NADA {
                    sumar(&mut f, p, a);
                }
            }
        }
        if total > 0.0 {
            for i in 0..f.n {
                f.frac[i] /= total;
            }
        }
        f
    }

    /// Normal del borde suavizado en la celda `k`, de la placa `p` hacia la `q`: la dirección en que
    /// más crece `φ_q − φ_p`, por mínimos cuadrados sobre las 8 vecinas. `None` si no crece.
    pub(super) fn normal_suave(&self, k: usize, p: u32, q: u32, por_reclamo: bool) -> Option<V3> {
        let r = self.malla.r_km;
        let x = self.malla.dir[k];
        let (e, nn) = east_north(x);
        let f0 = {
            let f = self.fi(k, por_reclamo);
            f.de(q) - f.de(p)
        };
        let (mut sxx, mut sxy, mut syy, mut bx, mut by) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for &v in &self.malla.vec[k] {
            if v == NADA {
                continue;
            }
            let f = self.fi(v as usize, por_reclamo);
            let df = f.de(q) - f.de(p) - f0;
            let dv = sub(self.malla.dir[v as usize], x);
            let (de, dn) = (dot(dv, e) * r, dot(dv, nn) * r);
            sxx += de * de;
            sxy += de * dn;
            syy += dn * dn;
            bx += de * df;
            by += dn * df;
        }
        let det = sxx * syy - sxy * sxy;
        if det.abs() < 1e-12 {
            return None;
        }
        let (ge, gn) = ((syy * bx - sxy * by) / det, (sxx * by - sxy * bx) / det);
        let g = (ge * ge + gn * gn).sqrt();
        (g > 0.0).then(|| normalize(add(esc(e, ge / g), esc(nn, gn / g))))
    }

    /// Tipo del tramo en `k` entre las placas `p` y `q` (§6.3): transformante si la velocidad de `q`
    /// respecto de `p` está a menos de `θ_tr` de la línea del borde (o es casi nula); si no,
    /// convergente si se acercan y divergente si se separan.
    pub(super) fn tipo_tramo(&self, k: usize, p: u32, q: u32, normal: V3) -> TipoTramo {
        let r = self.malla.r_km;
        let x = self.malla.dir[k];
        let pp = &self.placas[p as usize];
        let pq = &self.placas[q as usize];
        let vrel = sub(
            velocidad(add(pq.omega, pq.empuje), x, r),
            velocidad(add(pp.omega, pp.empuje), x, r),
        );
        let vmod = dot(vrel, vrel).sqrt();
        let v_n = dot(vrel, normal);
        if vmod < 0.05 * CM_ANIO || v_n.abs() < self.cfg.theta_tr_grados.to_radians().sin() * vmod {
            TipoTramo::Transformante
        } else if v_n < 0.0 {
            TipoTramo::Convergente
        } else {
            TipoTramo::Divergente
        }
    }

    /// Tipo de tramo de cada celda en solape o en hueco tras mover las placas (§6.3), con el borde
    /// suavizado de los reclamos y las dos placas de más fracción (en un solape, de entre las que la
    /// reclaman).
    pub(super) fn clasificar_reclamos(&mut self) {
        let num = self.malla.num;
        let mut tipo = std::mem::take(&mut self.tipo_k);
        tipo.clear();
        tipo.resize(num, TipoTramo::Ninguno);
        for (k, t) in tipo.iter_mut().enumerate() {
            let n = self.ncov[k] as usize;
            if n == 1 {
                continue;
            }
            let f = self.fi(k, true);
            let mut cand: Vec<(u32, f64)> = if n >= 2 {
                (0..n)
                    .map(|i| self.cov[k][i].0)
                    .map(|p| (p, f.de(p)))
                    .collect()
            } else {
                (0..f.n).map(|i| (f.placa[i], f.frac[i])).collect()
            };
            cand.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
            cand.dedup_by_key(|x| x.0);
            if cand.len() < 2 {
                continue;
            }
            let (p, q) = (cand[0].0, cand[1].0);
            if let Some(nrm) = self.normal_suave(k, p, q, true) {
                *t = self.tipo_tramo(k, p, q, nrm);
            }
        }
        self.tipo_k = tipo;
    }

    /// Sorteo sembrado para los desempates de esta nota (como el de los huecos).
    #[inline]
    pub(super) fn sorteo(&self, k: u64, x: u64) -> u64 {
        splitmix64(self.seed ^ splitmix64(k ^ splitmix64(x ^ self.paso)))
    }

    /// Suavizado de la pertenencia (§6.5): una celda de fondo oceánico (sin corteza de tipo
    /// continental ni de arco) cerca de un borde pasa a la placa `q` si su placa tiene `φ < 0,5` y
    /// `q` tiene `φ > 0,5`. La corteza no se mueve: solo cambia de placa (si `q` ya tiene una muestra
    /// oculta en ese sitio, se ve la suya; si no, la muestra pasa a `q` con su volumen). Se decide
    /// con la pertenencia de antes de cambiar nada, así que no depende del orden. Devuelve el área
    /// reasignada (km²).
    pub(super) fn suavizar_pertenencia(&mut self) -> f64 {
        let num = self.malla.num;
        // Cuántas celdas del mundo representa cada muestra (una que represente varias no se mueve).
        let mut usos: HashMap<(u32, u32), u32> = HashMap::new();
        for k in 0..num {
            if self.dueno[k] != NADA && self.rep[k] != NADA {
                *usos.entry((self.dueno[k], self.rep[k])).or_insert(0) += 1;
            }
        }
        let mut cambios: Vec<(u32, u32)> = Vec::new();
        for k in 0..num {
            let p = self.dueno[k];
            if p == NADA {
                continue;
            }
            let Some(c) = self.corteza(k) else {
                continue;
            };
            if c.cont || c.hc > 0.0 {
                continue;
            }
            // Solo cerca de un borde: alguna celda a menos de L_borde es de otra placa.
            if !self.vecindad.de(k).iter().any(|&j| {
                let d = self.dueno[j as usize];
                d != NADA && d != p
            }) {
                continue;
            }
            let f = self.fi(k, false);
            if f.de(p) >= 0.5 {
                continue;
            }
            if let Some((q, fq)) = f.mayor()
                && q != p
                && fq > 0.5
            {
                cambios.push((k as u32, q));
            }
        }
        let mut area = 0.0;
        let mut tocadas: Vec<u32> = Vec::new();
        for (k, q) in cambios {
            let ku = k as usize;
            let p = self.dueno[ku];
            let l = self.rep[ku];
            let x = self.malla.dir[ku];
            let lq = self
                .malla
                .localizar(aplicar_t(&self.placas[q as usize].rot, x)) as u32;
            if let Some(cq) = self.placas[q as usize].get(lq) {
                // `q` ya tiene ahí una muestra oculta: se ve la suya, si también es solo oceánica.
                if cq.cont || cq.hc > 0.0 {
                    continue;
                }
            } else {
                // La muestra pasa a `q` con su volumen (solo si no representa otra celda de `p`).
                if usos.get(&(p, l)).copied().unwrap_or(0) != 1 {
                    continue;
                }
                let Some(mut c) = self.placas[p as usize].quitar(l) else {
                    continue;
                };
                let f = self.malla.area[l as usize] / self.malla.area[lq as usize];
                c.sed *= f;
                c.proc.lote = self.lotes.len() as u32;
                self.lotes.push(lote::SUAVIZADO);
                self.placas[q as usize].insertar(lq, c, k);
            }
            self.dueno[ku] = q;
            self.rep[ku] = lq;
            area += self.malla.area[ku];
            tocadas.push(p);
            tocadas.push(q);
        }
        tocadas.sort_unstable();
        tocadas.dedup();
        for p in tocadas {
            self.placas[p as usize].actualizar_cap(&self.malla);
        }
        area
    }

    /// Islas (nota de bordes §6.5, extendida a todo tipo de corteza): un trozo suelto de una placa
    /// (componente conexa que no es la mayor) de menos de `L_borde²` dentro de otras placas pasa a la
    /// placa que más lo rodea, como la absorción de placas pequeñas del documento principal. A 52 km,
    /// un trozo de 100 km de una placa dentro de otra no es un borde: es corteza que se mueve con la
    /// que lo rodea. Si se queda en su placa, gana solapes que no le tocan y apila la corteza que le
    /// pasa por debajo. La corteza no se mueve: solo cambia de placa (si la otra ya tiene una muestra
    /// en ese sitio, se suman con su volumen). Devuelve el área reasignada (km²).
    pub(super) fn absorber_islas(&mut self) -> f64 {
        let num = self.malla.num;
        let (rs, rc) = (self.cfg.rho_s, self.cfg.rho_c);
        let h_cont_min = self.cfg.h_cont_min_km;
        let a_isla = self.cfg.l_borde_km * self.cfg.l_borde_km;
        // Componentes conexas (8 vecinas) de las celdas de cada placa.
        let mut comp = vec![NADA; num];
        let mut comps: Vec<(u32, f64, Vec<u32>)> = Vec::new(); // (placa, área, celdas)
        let mut pila: Vec<u32> = Vec::new();
        for k0 in 0..num {
            let p = self.dueno[k0];
            if p == NADA || comp[k0] != NADA {
                continue;
            }
            let id = comps.len() as u32;
            let mut celdas = Vec::new();
            let mut area = 0.0;
            comp[k0] = id;
            pila.push(k0 as u32);
            while let Some(c) = pila.pop() {
                celdas.push(c);
                area += self.malla.area[c as usize];
                for &v in &self.malla.vec[c as usize] {
                    if v != NADA && comp[v as usize] == NADA && self.dueno[v as usize] == p {
                        comp[v as usize] = id;
                        pila.push(v);
                    }
                }
            }
            comps.push((p, area, celdas));
        }
        // La mayor componente de cada placa no es isla.
        let mut mayor: HashMap<u32, (f64, usize)> = HashMap::new();
        for (i, (p, a, _)) in comps.iter().enumerate() {
            let e = mayor.entry(*p).or_insert((-1.0, i));
            if *a > e.0 {
                *e = (*a, i);
            }
        }
        let mut usos: HashMap<(u32, u32), u32> = HashMap::new();
        for k in 0..num {
            if self.dueno[k] != NADA && self.rep[k] != NADA {
                *usos.entry((self.dueno[k], self.rep[k])).or_insert(0) += 1;
            }
        }
        let mut area_total = 0.0;
        let mut tocadas: Vec<u32> = Vec::new();
        for (i, (p, a, celdas)) in comps.iter().enumerate() {
            if *a >= a_isla || mayor.get(p).map(|x| x.1) == Some(i) {
                continue;
            }
            // Placa que más la rodea (por celdas vecinas; desempate por índice).
            let mut cuenta: HashMap<u32, u32> = HashMap::new();
            for &c in celdas {
                for &v in &self.malla.vec[c as usize] {
                    if v == NADA {
                        continue;
                    }
                    let q = self.dueno[v as usize];
                    if q != NADA && q != *p {
                        *cuenta.entry(q).or_insert(0) += 1;
                    }
                }
            }
            let Some((&q, _)) = cuenta.iter().max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0))) else {
                continue;
            };
            for &k in celdas {
                let ku = k as usize;
                let l = self.rep[ku];
                if l == NADA || usos.get(&(*p, l)).copied().unwrap_or(0) != 1 {
                    continue;
                }
                let Some(c) = self.placas[*p as usize].quitar(l) else {
                    continue;
                };
                let x = self.malla.dir[ku];
                let lq = self
                    .malla
                    .localizar(aplicar_t(&self.placas[q as usize].rot, x))
                    as u32;
                let f = self.malla.area[l as usize] / self.malla.area[lq as usize];
                match self.placas[q as usize].get_mut(lq) {
                    Some(d) => {
                        // Ya hay una muestra de q ahí: se suman con su volumen.
                        d.sumar_hc(
                            c.hc * f,
                            c.arco,
                            Origen::de(&c),
                            rs,
                            rc,
                            h_cont_min,
                            via::ISLAS,
                        );
                        d.sed += c.sed * f;
                    }
                    None => {
                        let mut c2 = Corteza {
                            h: if c.cont {
                                c.h * f
                            } else {
                                c.h - c.hc + c.hc * f
                            },
                            hc: c.hc * f,
                            sed: c.sed * f,
                            proc: c.proc.escalada(f),
                            ..c
                        };
                        c2.proc.lote = self.lotes.len() as u32;
                        self.lotes.push(lote::ISLA);
                        self.placas[q as usize].insertar(lq, c2, k);
                    }
                }
                self.dueno[ku] = q;
                self.rep[ku] = lq;
                area_total += self.malla.area[ku];
            }
            tocadas.push(*p);
            tocadas.push(q);
        }
        tocadas.sort_unstable();
        tocadas.dedup();
        for p in tocadas {
            self.placas[p as usize].actualizar_cap(&self.malla);
        }
        area_total
    }
}
