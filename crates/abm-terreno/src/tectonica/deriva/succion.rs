//! Succión de las losas profundas y de los restos de losa (nota de succión de las losas).
//!
//! La losa de una fosa activa que pasa de `L_losa` sigue hundiéndose en el manto inferior: su parte
//! profunda (`L_inf = L_j − L_losa`) arrastra el manto y empuja la base de las placas cercanas hacia
//! ella (Conrad y Lithgow-Bertelloni 2002). Se coloca a lo largo de la fosa, desplazada `d_off` hacia la
//! placa de arriba, y se recalcula cada paso. Cuando una fosa muere o su losa se rompe tras un choque,
//! la losa entera pasa a ser un resto, fijo en el marco del manto, que se apaga en `τ_hund`.
//!
//! La tracción en la base de cada celda a menos de `W_suc` de una losa es
//! `τ = ε_suc·B·(1 − d/W)/W` hacia su punto más cercano, con `B` el peso por unidad de longitud de la
//! losa en las unidades del tirón (`f_sp·min(1, √(edad/t_sat))` por `L/L_losa`). Se suman todas las
//! losas y entra en el equilibrio de fuerzas de cada placa como un par más (§6.3). Se calcula en una
//! malla más gruesa (la del nivel 0, ~190 km) con los vecinos a menos de `W_suc` precalculados: la
//! placa de cada celda gruesa es la de su centro.

use super::*;

/// Un resto de losa (nota §6.2): puntos fijos en el marco del manto con su peso inicial (fuerza, en
/// las unidades del tirón) y la longitud de fosa que representa cada uno (km), y su edad desde que se
/// separó.
#[derive(Debug, Clone)]
pub struct RestoLosa {
    puntos: Vec<(V3, f64, f64)>,
    pub edad_myr: f64,
}

/// La malla gruesa de la succión: la de nivel 0, la celda gruesa de cada celda fina y las vecinas de
/// cada celda gruesa a menos de `W_suc` (con su distancia, km). La tracción se calcula en las celdas
/// gruesas y el par se suma por las celdas finas, cada una con su placa y su área: así una microplaca
/// solo recibe la tracción de su área.
pub(super) struct MallaSuccion {
    malla: Malla,
    gruesa_de: Vec<u32>,
    vecinas: Vec<Vec<(u32, f32)>>,
    w_km: f64,
    marca: Vec<u32>,
    tau: Vec<V3>,
}

impl MallaSuccion {
    fn nueva(fina: &Malla, w_km: f64) -> Self {
        let n0 = fina.n >> fina.nivel;
        let malla = Malla::new(n0, 0, fina.r_km);
        let r = malla.r_km;
        let gruesa_de: Vec<u32> = fina
            .dir
            .iter()
            .map(|&x| malla.localizar(x) as u32)
            .collect();
        let mut vecinas = Vec::with_capacity(malla.num);
        for k0 in 0..malla.num {
            let x0 = malla.dir[k0];
            let mut visto = std::collections::BTreeSet::new();
            let mut cola = std::collections::VecDeque::new();
            let mut lista = Vec::new();
            visto.insert(k0 as u32);
            cola.push_back(k0 as u32);
            while let Some(k) = cola.pop_front() {
                let d = r * dot(malla.dir[k as usize], x0).clamp(-1.0, 1.0).acos();
                if d > w_km {
                    continue;
                }
                lista.push((k, d as f32));
                for &v in &malla.vec[k as usize] {
                    if v != NADA && visto.insert(v) {
                        cola.push_back(v);
                    }
                }
            }
            vecinas.push(lista);
        }
        let num = malla.num;
        MallaSuccion {
            malla,
            gruesa_de,
            vecinas,
            w_km,
            marca: vec![u32::MAX; num],
            tau: vec![[0.0; 3]; num],
        }
    }
}

/// Puntos de una losa (posición, peso por unidad de longitud y longitud de fosa, km), por par de
/// placas.
type LineasPorPar = std::collections::BTreeMap<(u32, u32), Vec<(V3, f64, f64)>>;

/// Desplaza `x` una distancia `d_km` por la superficie en la dirección tangente `n`.
fn desplazar(x: V3, n: V3, d_km: f64, r_km: f64) -> V3 {
    let th = d_km / r_km;
    normalize(add(esc(x, th.cos()), esc(n, th.sin())))
}

impl Deriva {
    /// Peso por unidad de longitud de una losa de `l_km` (en las unidades del tirón): el del tirón de
    /// una losa de `L_losa` con el fondo de `edad`, por `l/L_losa`.
    fn peso_losa(&self, edad: f64, l_km: f64) -> f64 {
        self.cfg.f_sp * (edad.max(0.0) / self.cfg.t_sat_myr).sqrt().min(1.0) * l_km
            / self.cfg.l_losa_km
    }

    /// La edad del fondo que se hunde en el borde `b`, como en el tirón de la losa.
    fn edad_de_la_losa(&self, b: &Borde) -> f64 {
        if b.edad_losa > 0.0 {
            b.edad_losa
        } else {
            self.corteza(b.k as usize).map_or(0.0, |c| c.edad_oc)
        }
    }

    /// Las losas profundas de las fosas activas (nota §6.1), por par de placas: puntos (posición,
    /// peso por unidad de longitud, longitud de fosa).
    fn losas_profundas(&self) -> LineasPorPar {
        let mut lineas: LineasPorPar = Default::default();
        let (l_losa, d_off, r) = (self.cfg.l_losa_km, self.cfg.d_off_km, self.malla.r_km);
        for b in &self.bordes {
            if !(b.p_subduce && b.losa > 0.0) {
                continue;
            }
            let Some(e) = self.estado_fosa(b.p, self.rep[b.k as usize], b.q) else {
                continue;
            };
            if !e.activa {
                continue;
            }
            let l_inf = e.x_km - l_losa;
            if l_inf <= 0.0 {
                continue;
            }
            let x = desplazar(self.malla.dir[b.k as usize], b.normal, d_off, r);
            let peso = self.peso_losa(self.edad_de_la_losa(b), l_inf);
            lineas
                .entry((b.p, b.q))
                .or_default()
                .push((x, peso, b.largo_km));
        }
        lineas
    }

    /// Los restos de losa de las fosas que mueren o cuya losa se rompe en el paso (nota §6.2): la losa
    /// entera, en la posición de su losa profunda, un resto por par de placas.
    pub(super) fn nacen_restos(&mut self, muertas: &[usize]) {
        let (d_off, r) = (self.cfg.d_off_km, self.malla.r_km);
        let mut por_par: LineasPorPar = Default::default();
        for &i in muertas {
            let b = self.bordes[i];
            if !b.p_subduce {
                continue;
            }
            let Some(e) = self.estado_fosa(b.p, self.rep[b.k as usize], b.q) else {
                continue;
            };
            if e.x_km <= 0.0 {
                continue;
            }
            let x = desplazar(self.malla.dir[b.k as usize], b.normal, d_off, r);
            let peso = self.peso_losa(self.edad_de_la_losa(&b), e.x_km);
            por_par
                .entry((b.p, b.q))
                .or_default()
                .push((x, peso, b.largo_km));
        }
        for (_, puntos) in por_par {
            let peso: f64 = puntos.iter().map(|x| x.1 * x.2).sum();
            self.evento(
                "resto_de_losa",
                vec![],
                puntos.iter().map(|x| x.2).sum(),
                peso,
                Some(puntos[0].0),
            );
            self.restos.push(RestoLosa {
                puntos,
                edad_myr: 0.0,
            });
        }
    }

    /// Pone un resto de losa (pruebas): puntos (posición, peso por unidad de longitud y longitud de
    /// fosa, km) recién separados.
    pub fn poner_resto_de_losa(&mut self, puntos: Vec<(V3, f64, f64)>) {
        self.restos.push(RestoLosa {
            puntos,
            edad_myr: 0.0,
        });
    }

    /// Envejece los restos de losa y quita los que ya no tiran (al principio del paso).
    pub fn envejecer_restos(&mut self) {
        let dt = self.cfg.dt_myr;
        let tau = self.tau_hund_myr();
        for r in self.restos.iter_mut() {
            r.edad_myr += dt;
        }
        self.restos.retain(|r| r.edad_myr < tau);
    }

    /// Tiempo hasta que un resto deja de tirar (Myr): la profundidad que le queda hasta el fondo del
    /// manto a la velocidad de hundimiento en el manto inferior (van der Meer et al. 2010).
    fn tau_hund_myr(&self) -> f64 {
        self.cfg.prof_hund_km / (self.cfg.v_hund_cm_anio * CM_ANIO)
    }

    /// Par de la succión sobre cada placa (por índice), en las unidades de `velocidades` (nota §6.3),
    /// y lo que se mide de ella en el paso (losas profundas y restos y su peso).
    pub fn pares_de_succion(&mut self) -> Vec<V3> {
        let n = self.placas.len();
        let mut par = vec![[0.0; 3]; n];
        if self.cfg.eps_suc <= 0.0 {
            self.succion_paso = (0, 0, 0.0);
            return par;
        }
        let w = self.cfg.w_suc_km;
        if self.suc.as_ref().is_none_or(|m| m.w_km != w) {
            self.suc = Some(MallaSuccion::nueva(&self.malla, w));
        }
        let tau = self.tau_hund_myr();
        let mut lineas: Vec<Vec<(V3, f64, f64)>> = self.losas_profundas().into_values().collect();
        let n_prof = lineas.len();
        let mut peso_restos = 0.0;
        for rl in &self.restos {
            let f = (1.0 - rl.edad_myr / tau).max(0.0);
            if f > 0.0 {
                peso_restos += rl.puntos.iter().map(|x| x.1 * x.2).sum::<f64>() * f;
                lineas.push(rl.puntos.iter().map(|&(x, p, l)| (x, p * f, l)).collect());
            }
        }
        self.succion_paso = (n_prof as u32, self.restos.len() as u32, peso_restos);
        let Some(mut ms) = self.suc.take() else {
            return par;
        };
        let r = self.malla.r_km;
        let eps = self.cfg.eps_suc;
        ms.tau.iter_mut().for_each(|t| *t = [0.0; 3]);
        for (il, linea) in lineas.iter().enumerate() {
            // Puntos de la línea en la malla gruesa: peso por longitud medio y posición media de los
            // puntos finos de cada celda gruesa.
            let mut gruesos: std::collections::BTreeMap<u32, (V3, f64, f64)> = Default::default();
            for &(x, peso, largo) in linea {
                let kg = ms.malla.localizar(x) as u32;
                let e = gruesos.entry(kg).or_insert(([0.0; 3], 0.0, 0.0));
                e.0 = add(e.0, esc(x, largo));
                e.1 += peso * largo;
                e.2 += largo;
            }
            let puntos: Vec<(u32, V3, f64)> = gruesos
                .into_iter()
                .filter(|(_, v)| v.2 > 0.0)
                .map(|(k, v)| (k, normalize(v.0), v.1 / v.2))
                .collect();
            let marca = il as u32;
            for &(kg, ..) in &puntos {
                for &(c, _) in &ms.vecinas[kg as usize] {
                    let cu = c as usize;
                    if ms.marca[cu] == marca {
                        continue;
                    }
                    ms.marca[cu] = marca;
                    let xc = ms.malla.dir[cu];
                    // El punto más cercano de la línea.
                    let (mut mejor, mut cos_max) = (0usize, -2.0f64);
                    for (j, p) in puntos.iter().enumerate() {
                        let cs = dot(xc, p.1);
                        if cs > cos_max {
                            (mejor, cos_max) = (j, cs);
                        }
                    }
                    let d = r * cos_max.clamp(-1.0, 1.0).acos();
                    if d >= w {
                        continue;
                    }
                    let xp = puntos[mejor].1;
                    let t = sub(xp, esc(xc, dot(xp, xc)));
                    let nt = dot(t, t).sqrt();
                    if nt < 1e-12 {
                        continue;
                    }
                    let tau_c = eps * puntos[mejor].2 * (1.0 - d / w) / w;
                    ms.tau[cu] = add(ms.tau[cu], esc(t, tau_c / nt));
                }
            }
        }
        // El par de cada placa: la tracción de la celda gruesa por el área de cada celda fina suya.
        for k in 0..self.malla.num {
            let p = self.dueno[k];
            if p == NADA {
                continue;
            }
            let t = ms.tau[ms.gruesa_de[k] as usize];
            if t == [0.0; 3] {
                continue;
            }
            let x = self.malla.dir[k];
            par[p as usize] = add(par[p as usize], esc(cross(x, t), r * self.malla.area[k]));
        }
        // Las marcas de esta tanda no deben confundirse con las del paso siguiente.
        ms.marca.iter_mut().for_each(|m| *m = u32::MAX);
        self.suc = Some(ms);
        par
    }
}

impl Deriva {
    /// Reparto de la potencia que mueve las placas (nota de succión de las losas §8): la positiva del
    /// tirón de las losas, de la succión y del empuje (resto del par de borde), sumada en todas las
    /// placas.
    pub(super) fn reparto_de_potencia(&self) -> [f64; 3] {
        let mut s = [0.0; 3];
        for x in &self.potencias {
            s[0] += x.2[0].max(0.0);
            s[1] += x.2[4].max(0.0);
            s[2] += x.2[1].max(0.0);
        }
        let tot: f64 = s.iter().sum();
        if tot > 0.0 {
            s.map(|x| x / tot)
        } else {
            [0.0; 3]
        }
    }

    /// Movimiento de las fosas respecto al manto (nota §8; Schellart et al. 2008): la fosa va con el
    /// borde de la placa de arriba, así que retrocede (hacia la placa que se hunde) donde la placa de
    /// arriba se mueve hacia ella. Devuelve la fracción de la longitud de fosa que retrocede y la
    /// velocidad media de retroceso (cm/año; negativa si de media avanza).
    pub(super) fn retroceso_de_fosas(&self) -> (f64, f64) {
        let r = self.malla.r_km;
        let (mut largo, mut atras, mut v) = (0.0, 0.0, 0.0);
        for b in &self.bordes {
            if b.tipo != TipoBorde::Subduccion || !b.p_subduce {
                continue;
            }
            let q = b.q as usize;
            if q >= self.placas.len() {
                continue;
            }
            let vq = velocidad(self.placas[q].omega, self.malla.dir[b.k as usize], r);
            // La normal va de la placa que se hunde a la de arriba: retrocede si v_q·n < 0.
            let vn = dot(vq, b.normal) / CM_ANIO;
            largo += b.largo_km;
            v -= vn * b.largo_km;
            if vn < 0.0 {
                atras += b.largo_km;
            }
        }
        if largo > 0.0 {
            (atras / largo, v / largo)
        } else {
            (0.0, 0.0)
        }
    }
}
