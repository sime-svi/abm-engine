//! Ciclo de vida de las fosas (`nota_ciclo_fosas.md`, v2): cada tramo convergente de corteza
//! oceánica es difuso (aún sin fosa) o una fosa activa, con memoria entre pasos. Un tramo difuso
//! acumula convergencia y, al llegar a `X_ini`, nace la fosa con su polaridad fija; una fosa alarga
//! su losa, que tira más cuanto más larga es, y muere si deja de converger o si le llega corteza que
//! flota. También nacen fosas espontáneas, raras, en transformantes con mucho contraste de edad y
//! fondo viejo (nota de fosas v6 §6.4).
//!
//! El estado va en un mapa de cada placa (celda local → estado), en las celdas de borde de las dos
//! orillas del tramo: viaja con la placa. Cada paso se rehace con los bordes de ahora, heredando el
//! de las celdas cercanas (el margen se mueve una o dos celdas por paso), y lo que ya no está en un
//! borde con su par se descarta.

use super::*;

/// Estado de un tramo convergente en una celda de margen de una placa (nota de fosas §3–§4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EstadoFosa {
    /// Placa del otro lado del tramo (id; tras partirse o fundirse, puede ser un antepasado suyo).
    pub otra: u64,
    /// Fosa activa; si no, tramo difuso.
    pub activa: bool,
    /// En una fosa activa, esta placa es la de arriba (la otra se hunde).
    pub arriba: bool,
    /// `X_j` en un tramo difuso; en una fosa, `L_j`, la longitud de la losa (km).
    pub x_km: f64,
    /// Edad de la fosa (Myr).
    pub edad_myr: f64,
    /// Tiempo seguido sin convergencia (Myr).
    pub sin_conv_myr: f64,
    /// Tiempo desde que llegó corteza que flota (Myr; negativo, sin choque): la losa sigue colgando
    /// y tirando hasta romperse a los `t_rot` (nota de fosas v4 §6.6).
    pub choque_myr: f64,
    /// Edad media del fondo que se hunde (Myr), la última medida antes del choque: fija `t_rot` y el
    /// tirón de la losa mientras cuelga.
    pub edad_fondo_myr: f64,
}

impl EstadoFosa {
    /// Fosa activa recién puesta (nacimiento, invasión o escenario), sin choque.
    pub(super) fn activa(
        otra: u64,
        arriba: bool,
        x_km: f64,
        edad_myr: f64,
        edad_fondo: f64,
    ) -> Self {
        Self {
            otra,
            activa: true,
            arriba,
            x_km,
            edad_myr,
            sin_conv_myr: 0.0,
            choque_myr: -1.0,
            edad_fondo_myr: edad_fondo,
        }
    }
}

/// Lo que pasó con las fosas en el último paso (nota de fosas §7–§8).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ConteoFosas {
    /// Nacimientos: inducidos, de una placa nueva al separarse el océano de un margen pasivo
    /// invadido (v4 §6.4) y espontáneos en transformante.
    pub nacen: [u32; 3],
    /// Longitud que las fosas han invadido en el paso por transformantes y por márgenes pasivos
    /// (km; en los márgenes, el frente, que aún no mueve nada).
    pub invadido_km: [f64; 2],
    /// Intentos de separar el océano de delante de un margen invadido: los que fallan porque el
    /// océano de delante, o lo que queda de la placa, no llega a `A_min`, y celdas de frente
    /// olvidadas (diagnóstico).
    pub separar: [u32; 3],
    /// Muertes: sin convergencia y por choque (al romperse la losa).
    pub mueren: [u32; 2],
    /// Roturas de losa (v4 §6.6).
    pub roturas: u32,
    /// Fondo retirado en la convergencia difusa (km²).
    pub area_difusa_km2: f64,
}

/// Muestra perdida en un solape de un tramo difuso: se absorbe engrosando las dos placas (6.2).
#[derive(Debug, Clone, Copy)]
pub(super) struct Difusa {
    /// Celda del solape.
    pub k: u32,
    /// Placa que se quedó la celda y placa de la muestra.
    pub ganadora: u32,
    pub perdedora: u32,
    pub c: Corteza,
    /// Área de la muestra (km²).
    pub a: f64,
}

/// Un evento de fosa a apuntar al final del paso.
type EventoPendiente = (&'static str, Vec<u64>, f64, Option<V3>, Detalle);

/// Fosa que muere en un par: longitud, edad, losa, una celda, si fue por choque y, entonces, el
/// tiempo desde el inicio del choque (la rotura de la losa).
#[derive(Clone, Copy)]
struct Muerte {
    largo: f64,
    edad: f64,
    losa: f64,
    k: u32,
    choque: bool,
    t_choque: f64,
}

/// Celda de fosa que muere en el paso: borde, por choque, edad de la fosa, losa y tiempo de choque.
type CeldaMuerta = (usize, bool, f64, f64, f64);

/// Candidato espontáneo en una transformante: longitud, Σ edad vieja·longitud, votos de la placa
/// que se hundiría y sus bordes.
#[derive(Default)]
struct CandTransf {
    largo: f64,
    edad_l: f64,
    votos: [f64; 2],
    regs: Vec<usize>,
}

const P_POLARIDAD: u64 = 11;
const P_ESPONTANEO: u64 = 12;

fn raiz(padre: &mut [u32], mut i: u32) -> u32 {
    while padre[i as usize] != i {
        padre[i as usize] = padre[padre[i as usize] as usize];
        i = padre[i as usize];
    }
    i
}

impl Deriva {
    /// ¿Es `otra` la placa `q` o un antepasado suyo?
    pub(super) fn es_placa(&self, otra: u64, q: u32) -> bool {
        let pq = &self.placas[q as usize];
        pq.id == otra || pq.ancestros.contains(&otra)
    }

    /// Fosas activas de los bordes del paso (diagnóstico de la nota de demografía de placas, D2 y
    /// D5): por cada celda de borde de la placa que se hunde, su índice, la longitud (km), la edad de
    /// la fosa (Myr) y la convergencia (km/Myr).
    pub fn fosas_activas(&self) -> Vec<(u32, f64, f64, f64)> {
        self.bordes
            .iter()
            .filter(|b| b.tipo == TipoBorde::Subduccion && b.p_subduce)
            .filter_map(|b| {
                self.estado_fosa(b.p, self.rep[b.k as usize], b.q)
                    .filter(|e| e.activa)
                    .map(|e| (b.p, b.largo_km, e.edad_myr, b.v_n))
            })
            .collect()
    }

    /// Estado del tramo frente a la placa `q` en la celda local `l` de la placa `p`, o en sus
    /// vecinas a uno y dos anillos (orden fijo): el margen se mueve hasta un par de celdas por paso.
    pub(super) fn estado_fosa(&self, p: u32, l: u32, q: u32) -> Option<EstadoFosa> {
        if l == NADA || p == NADA || q == NADA {
            return None;
        }
        let mapa = &self.placas[p as usize].fosas;
        if mapa.is_empty() {
            return None;
        }
        let ver = |c: u32| mapa.get(&c).filter(|e| self.es_placa(e.otra, q)).copied();
        if let Some(e) = ver(l) {
            return Some(e);
        }
        let vec = &self.malla.vec[l as usize];
        for &v in vec {
            if v != NADA
                && let Some(e) = ver(v)
            {
                return Some(e);
            }
        }
        for &v in vec {
            if v == NADA {
                continue;
            }
            for &w in &self.malla.vec[v as usize] {
                if w != NADA
                    && let Some(e) = ver(w)
                {
                    return Some(e);
                }
            }
        }
        None
    }

    /// Placa que se hunde en la fosa activa entre las muestras `a` y `b` (placa, celda local) de un
    /// solape, si la hay.
    pub(super) fn hunde_en_fosa(&self, a: (u32, u32), b: (u32, u32)) -> Option<u32> {
        if let Some(e) = self.estado_fosa(a.0, a.1, b.0).filter(|e| e.activa) {
            return Some(if e.arriba { b.0 } else { a.0 });
        }
        if let Some(e) = self.estado_fosa(b.0, b.1, a.0).filter(|e| e.activa) {
            return Some(if e.arriba { a.0 } else { b.0 });
        }
        None
    }

    /// Pone en los bordes el estado de sus tramos (nota de fosas §9): un tramo convergente de
    /// corteza oceánica solo es fosa si está activa, con su polaridad y el tirón de su losa
    /// (`min(1, L_j/L_losa)`, 6.5); si no, es convergencia difusa. El tirón sigue mientras la fosa
    /// vive, también si en un paso las placas se deslizan.
    pub(super) fn aplicar_estado_fosas(&mut self) {
        let l_losa = self.cfg.l_losa_km.max(1e-9);
        for i in 0..self.bordes.len() {
            let b = self.bordes[i];
            let est = self.estado_fosa(b.p, self.rep[b.k as usize], b.q);
            let nb = &mut self.bordes[i];
            nb.losa = 0.0;
            nb.edad_losa = 0.0;
            match est {
                // Choque con la losa aún colgada (nota de fosas v4 §6.6): el borde ya es continente
                // contra continente, pero la losa sigue tirando de la placa que se hundía hasta
                // romperse, con la edad del fondo que se hundía.
                Some(e) if e.activa && nb.tipo == TipoBorde::Colision => {
                    if !e.arriba && e.choque_myr >= 0.0 {
                        nb.p_subduce = true;
                        nb.losa = (e.x_km / l_losa).clamp(0.0, 1.0);
                        nb.edad_losa = e.edad_fondo_myr;
                    }
                }
                Some(e) if e.activa => {
                    if nb.tipo == TipoBorde::Difusa {
                        nb.tipo = TipoBorde::Subduccion;
                    }
                    nb.p_subduce = !e.arriba;
                    if !e.arriba {
                        nb.losa = (e.x_km / l_losa).clamp(0.0, 1.0);
                        if e.choque_myr >= 0.0 {
                            nb.edad_losa = e.edad_fondo_myr;
                        }
                    }
                }
                _ => {
                    if nb.tipo == TipoBorde::Subduccion {
                        nb.tipo = TipoBorde::Difusa;
                        nb.p_subduce = false;
                    }
                }
            }
        }
    }

    /// Las celdas de cada orilla de un tramo: las de borde de esa placa y las de enfrente de la otra.
    fn orilla(&self, regs: &[usize], s: u32) -> Vec<(u32, f64)> {
        let mut v = Vec::new();
        for &i in regs {
            let b = &self.bordes[i];
            if b.p == s {
                v.push((b.k, b.largo_km));
            } else if b.q == s {
                v.push((b.k_q, b.largo_km));
            }
        }
        v
    }

    /// Fracción de la orilla `s` de un tramo con corteza que flota y edad media de su fondo oceánico.
    /// Flota la corteza continental, también la de un arco que ya pasó de `h_cont_min` (20 km): una
    /// capa fina de corteza de tipo continental sobre el fondo (sedimento pegado, un arco que empieza)
    /// no atasca una fosa.
    fn flota_y_edad(&self, regs: &[usize], s: u32) -> (f64, f64) {
        let (mut fl, mut tot, mut ed, mut ea) = (0.0, 0.0, 0.0, 0.0);
        for (k, w) in self.orilla(regs, s) {
            let Some(c) = self.corteza(k as usize) else {
                continue;
            };
            tot += w;
            if c.cont {
                fl += w;
            } else {
                ed += c.edad_oc * w;
                ea += w;
            }
        }
        (
            if tot > 0.0 { fl / tot } else { 0.0 },
            if ea > 0.0 { ed / ea } else { 0.0 },
        )
    }

    /// Edad de la losa de la orilla `s` de un tramo (Myr): media por longitud del fondo más viejo
    /// junto a cada celda de la orilla (en ella o a un anillo, de la misma placa). Las celdas del
    /// borde mismo suelen ser corteza recién puesta al rellenar huecos; lo que llega a la fosa es el
    /// fondo de detrás.
    fn edad_losa(&self, regs: &[usize], s: u32) -> f64 {
        let (mut ed, mut w) = (0.0, 0.0);
        for (k, l) in self.orilla(regs, s) {
            let a = self.edad_losa_celda(k, s);
            if a.is_finite() {
                ed += a * l;
                w += l;
            }
        }
        if w > 0.0 { ed / w } else { 0.0 }
    }

    /// Fondo oceánico más viejo de la placa `s` en la celda `k` o a un anillo (Myr; `−∞` si no hay).
    pub(super) fn edad_losa_celda(&self, k: u32, s: u32) -> f64 {
        std::iter::once(k)
            .chain(self.malla.vec[k as usize].iter().copied())
            .filter(|&v| v != NADA && self.dueno[v as usize] == s)
            .filter_map(|v| {
                self.corteza(v as usize)
                    .filter(|c| !c.cont)
                    .map(|c| c.edad_oc)
            })
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// Polaridad al nacer (6.3): se hunde el océano frente al continente o al arco (el que ya flota,
    /// de más de `h_cont_min`); entre dos océanos, el más viejo; empate, sorteo sembrado. Devuelve (arriba, se hunde, edad del fondo que
    /// se hunde).
    fn polaridad_al_nacer(&self, regs: &[usize], pa: u32, pb: u32) -> (u32, u32, f64) {
        let (fa, ea) = self.flota_y_edad(regs, pa);
        let (fb, eb) = self.flota_y_edad(regs, pb);
        let (flota_a, flota_b) = (fa >= 0.5, fb >= 0.5);
        let hunde = if flota_a != flota_b {
            if flota_a { pb } else { pa }
        } else if ea != eb {
            if ea > eb { pa } else { pb }
        } else {
            let (ia, ib) = (self.placas[pa as usize].id, self.placas[pb as usize].id);
            if azar(self.seed, self.paso, ia ^ splitmix64(ib), P_POLARIDAD, 0) < 0.5 {
                pa
            } else {
                pb
            }
        };
        let arriba = if hunde == pa { pb } else { pa };
        (arriba, hunde, if hunde == pa { ea } else { eb })
    }

    /// Avanza los tramos convergentes un paso (nota de fosas §6.1–§6.6) con los bordes del final
    /// del paso: tramos difusos nuevos (`X = 0`), convergencia acumulada, nacimientos inducidos,
    /// vida de las fosas y muertes. Rehace el mapa de estados de cada placa con las celdas de borde
    /// de ahora.
    pub(super) fn avanzar_fosas(&mut self) {
        let cfg = self.cfg.clone();
        let dt = cfg.dt_myr;
        let nb = self.bordes.len();
        let num = self.malla.num;
        let previo: Vec<Option<EstadoFosa>> = self
            .bordes
            .iter()
            .map(|b| self.estado_fosa(b.p, self.rep[b.k as usize], b.q))
            .collect();
        let convergente = |b: &Borde| matches!(b.tipo, TipoBorde::Subduccion | TipoBorde::Difusa);
        let par = |b: &Borde| (b.p.min(b.q), b.p.max(b.q));
        // Rotura de la losa tras un choque (v4 §6.6): de t_rot,joven a t_rot,viejo con la edad del
        // fondo que se hundía.
        let t_rot = |a: f64| {
            cfg.t_rot_joven_myr
                + (cfg.t_rot_viejo_myr - cfg.t_rot_joven_myr) * (a / 100.0).clamp(0.0, 1.0)
        };
        // Tramos: componentes conexas (8 vecinas) de los bordes convergentes de cada par, con las
        // dos orillas.
        let mut reg_en = vec![NADA; num];
        for (i, b) in self.bordes.iter().enumerate() {
            reg_en[b.k as usize] = i as u32;
        }
        let mut padre: Vec<u32> = (0..nb as u32).collect();
        for i in 0..nb {
            let b = self.bordes[i];
            if !convergente(&b) {
                continue;
            }
            for &v in &self.malla.vec[b.k as usize] {
                if v == NADA {
                    continue;
                }
                let j = reg_en[v as usize];
                if j == NADA {
                    continue;
                }
                let bj = &self.bordes[j as usize];
                if convergente(bj) && par(bj) == par(&b) {
                    let (ri, rj) = (raiz(&mut padre, i as u32), raiz(&mut padre, j));
                    if ri != rj {
                        padre[ri.max(rj) as usize] = ri.min(rj);
                    }
                }
            }
        }
        let mut tramos: std::collections::BTreeMap<u32, Vec<usize>> = Default::default();
        for i in 0..nb {
            if convergente(&self.bordes[i]) {
                let r = raiz(&mut padre, i as u32);
                tramos.entry(r).or_default().push(i);
            }
        }
        let mut nuevos: Vec<HashMap<u32, EstadoFosa>> = vec![HashMap::new(); self.placas.len()];
        let mut eventos: Vec<EventoPendiente> = Vec::new();
        // Celdas de fosa que siguen activas y las que mueren en el paso (borde, por choque, edad y
        // losa): una fosa muere si el grupo de celdas que muere no toca ninguna celda activa de su
        // par que siga viva; si la toca, solo se acorta.
        let mut vive = vec![false; nb];
        let mut muertas: Vec<CeldaMuerta> = Vec::new();
        for regs in tramos.values() {
            let b0 = self.bordes[regs[0]];
            let (pa, pb) = par(&b0);
            let largo: f64 = regs.iter().map(|&i| self.bordes[i].largo_km).sum();
            let vn = regs
                .iter()
                .map(|&i| self.bordes[i].v_n * self.bordes[i].largo_km)
                .sum::<f64>()
                / largo.max(1e-12);
            let avance = vn.max(0.0) * dt;
            // Estado del tramo en el paso anterior: fosa si alguna celda lo era (con la polaridad de
            // más longitud); si no, difuso con la X media de las celdas que ya lo eran.
            let mut voto = [0.0f64; 2];
            let (mut l_sum, mut l_w, mut edad, mut x_sum, mut x_w) = (0.0, 0.0, 0.0f64, 0.0, 0.0);
            let (mut choque_prev, mut ef_sum) = (-1.0f64, 0.0);
            for &i in regs {
                let b = &self.bordes[i];
                if let Some(e) = previo[i] {
                    if e.activa {
                        let arriba = if e.arriba { b.p } else { b.q };
                        voto[(arriba == pb) as usize] += b.largo_km;
                        l_sum += e.x_km * b.largo_km;
                        l_w += b.largo_km;
                        edad = edad.max(e.edad_myr);
                        choque_prev = choque_prev.max(e.choque_myr);
                        ef_sum += e.edad_fondo_myr * b.largo_km;
                    } else {
                        x_sum += e.x_km * b.largo_km;
                        x_w += b.largo_km;
                    }
                }
            }
            let punto = Some(self.malla.dir[b0.k as usize]);
            let ids = |a: u32, b: u32| vec![self.placas[a as usize].id, self.placas[b as usize].id];
            let (activa, arriba, x, edad_f, choque_f, fondo_f) = if l_w > 0.0 {
                let arriba = if voto[1] > voto[0] { pb } else { pa };
                let hunde = if arriba == pa { pb } else { pa };
                let (flota, _) = self.flota_y_edad(regs, hunde);
                let edad_oc = self.edad_losa(regs, hunde);
                let fondo_prev = ef_sum / l_w;
                // Choca el tramo cuya orilla que se hunde ya flota en más de la mitad: el reloj de un
                // choque vecino que hereda alguna celda no detiene la parte oceánica de la fosa.
                if flota >= 0.5 {
                    // Ha llegado corteza que flota (v4 §6.6): el choque sigue como estaba modelado,
                    // pero la losa sigue colgando y tirando hasta romperse a los t_rot. Entonces
                    // muere la fosa y el tramo vuelve a difuso desde X = 0.
                    let t_c = if choque_prev >= 0.0 {
                        choque_prev + dt
                    } else {
                        0.0
                    };
                    let fondo = if choque_prev >= 0.0 || edad_oc <= 0.0 {
                        fondo_prev
                    } else {
                        edad_oc
                    };
                    if t_c >= t_rot(fondo) {
                        for &i in regs {
                            muertas.push((i, true, edad, l_sum / l_w, t_c));
                        }
                        (false, NADA, 0.0, 0.0, -1.0, 0.0)
                    } else {
                        (true, arriba, l_sum / l_w + avance, edad + dt, t_c, fondo)
                    }
                } else {
                    let fondo = if edad_oc > 0.0 { edad_oc } else { fondo_prev };
                    (true, arriba, l_sum / l_w + avance, edad + dt, -1.0, fondo)
                }
            } else {
                let x = if x_w > 0.0 { x_sum / x_w } else { 0.0 } + avance;
                if x >= cfg.x_ini_km {
                    let (arriba, hunde, edad_fondo) = self.polaridad_al_nacer(regs, pa, pb);
                    self.fosas_paso.nacen[0] += 1;
                    // `largo` suma las celdas de los dos lados del borde: la fosa mide la mitad.
                    eventos.push((
                        "nacimiento_fosa",
                        ids(hunde, arriba),
                        0.5 * largo,
                        punto,
                        Detalle {
                            clase: Some("inducido".into()),
                            edad_fondo_myr: Some(edad_fondo),
                            ..Detalle::default()
                        },
                    ));
                    (true, arriba, 0.0, 0.0, -1.0, self.edad_losa(regs, hunde))
                } else {
                    (false, NADA, x, 0.0, -1.0, 0.0)
                }
            };
            for &i in regs {
                let b = &self.bordes[i];
                vive[i] = activa;
                nuevos[b.p as usize].insert(
                    self.rep[b.k as usize],
                    EstadoFosa {
                        otra: self.placas[b.q as usize].id,
                        activa,
                        arriba: activa && b.p == arriba,
                        x_km: x,
                        edad_myr: edad_f,
                        sin_conv_myr: 0.0,
                        choque_myr: choque_f,
                        edad_fondo_myr: fondo_f,
                    },
                );
            }
        }
        // Bordes que no convergen ahora y tenían estado: una fosa cuenta el tiempo sin converger y
        // muere a los τ_muerte (o si es ya un choque); un tramo difuso conserva su X.
        for (i, prev) in previo.iter().enumerate() {
            let b = self.bordes[i];
            if convergente(&b) {
                continue;
            }
            let Some(e) = *prev else {
                continue;
            };
            let otra = self.placas[b.q as usize].id;
            if e.activa && b.tipo == TipoBorde::Colision {
                // Borde de la fosa que ya es continente contra continente (v4 §6.6): la losa cuelga
                // y tira hasta romperse a los t_rot del inicio del choque. El reloj es el del choque:
                // el más avanzado de los bordes de choque vecinos de esta orilla, para que toda la
                // zona se rompa a la vez.
                let vecinos: Vec<EstadoFosa> = self.malla.vec[b.k as usize]
                    .iter()
                    .filter(|&&v| v != NADA && reg_en[v as usize] != NADA)
                    .map(|&v| reg_en[v as usize] as usize)
                    .filter(|&j| {
                        self.bordes[j].p == b.p && self.bordes[j].tipo == TipoBorde::Colision
                    })
                    .filter_map(|j| previo[j])
                    .collect();
                let reloj = vecinos
                    .iter()
                    .filter(|x| x.activa && x.choque_myr >= 0.0)
                    .map(|x| x.choque_myr)
                    .fold(e.choque_myr, f64::max);
                // La edad de la losa de la zona de choque: la más vieja que cuelga a su lado.
                let fondo = vecinos
                    .iter()
                    .filter(|x| x.activa && x.choque_myr >= 0.0)
                    .map(|x| x.edad_fondo_myr)
                    .fold(e.edad_fondo_myr, f64::max);
                if reloj < 0.0 && vecinos.iter().any(|x| !x.activa && x.choque_myr >= 0.0) {
                    // Un borde de choque nuevo junto a la losa ya rota: aquí tampoco cuelga.
                    nuevos[b.p as usize].insert(
                        self.rep[b.k as usize],
                        EstadoFosa {
                            otra,
                            activa: false,
                            arriba: false,
                            x_km: 0.0,
                            edad_myr: 0.0,
                            sin_conv_myr: 0.0,
                            choque_myr: 0.0,
                            edad_fondo_myr: 0.0,
                        },
                    );
                    continue;
                }
                let t_c = if reloj >= 0.0 { reloj + dt } else { 0.0 };
                if t_c >= t_rot(fondo) {
                    muertas.push((i, true, e.edad_myr, e.x_km, t_c));
                    // La losa rota no vuelve: el borde queda marcado (sin fosa, con el reloj del choque)
                    // y no hereda la fosa de al lado.
                    nuevos[b.p as usize].insert(
                        self.rep[b.k as usize],
                        EstadoFosa {
                            otra,
                            activa: false,
                            arriba: false,
                            x_km: 0.0,
                            edad_myr: 0.0,
                            sin_conv_myr: 0.0,
                            choque_myr: t_c,
                            edad_fondo_myr: 0.0,
                        },
                    );
                    continue;
                }
                vive[i] = true;
                nuevos[b.p as usize].insert(
                    self.rep[b.k as usize],
                    EstadoFosa {
                        otra,
                        sin_conv_myr: 0.0,
                        edad_myr: e.edad_myr + dt,
                        choque_myr: t_c,
                        edad_fondo_myr: fondo,
                        ..e
                    },
                );
                continue;
            }
            if e.activa {
                let t = e.sin_conv_myr + dt;
                if t >= cfg.tau_muerte_myr {
                    muertas.push((i, false, e.edad_myr, e.x_km, -1.0));
                    continue;
                }
                vive[i] = true;
                nuevos[b.p as usize].insert(
                    self.rep[b.k as usize],
                    EstadoFosa {
                        otra,
                        sin_conv_myr: t,
                        edad_myr: e.edad_myr + dt,
                        ..e
                    },
                );
            } else {
                nuevos[b.p as usize].insert(self.rep[b.k as usize], EstadoFosa { otra, ..e });
            }
        }
        // Grupos conexos de celdas que mueren, por par; una muerte por grupo que no toca celdas
        // activas vivas de su par.
        let mut muere_en = vec![NADA; nb];
        for (j, &(i, ..)) in muertas.iter().enumerate() {
            muere_en[i] = j as u32;
        }
        let mut padre_m: Vec<u32> = (0..muertas.len() as u32).collect();
        for (j, &(i, ..)) in muertas.iter().enumerate() {
            let b = self.bordes[i];
            for &v in &self.malla.vec[b.k as usize] {
                if v == NADA || reg_en[v as usize] == NADA {
                    continue;
                }
                let r = reg_en[v as usize] as usize;
                let jm = muere_en[r];
                if jm != NADA && par(&self.bordes[r]) == par(&b) {
                    let (a, c) = (raiz(&mut padre_m, j as u32), raiz(&mut padre_m, jm));
                    if a != c {
                        padre_m[a.max(c) as usize] = a.min(c);
                    }
                }
            }
        }
        let mut grupos: std::collections::BTreeMap<u32, Vec<usize>> = Default::default();
        for j in 0..muertas.len() {
            let r = raiz(&mut padre_m, j as u32);
            grupos.entry(r).or_default().push(j);
        }
        for js in grupos.values() {
            let b0 = self.bordes[muertas[js[0]].0];
            let m = js.iter().fold(
                Muerte {
                    largo: 0.0,
                    edad: 0.0,
                    losa: 0.0,
                    k: b0.k,
                    choque: false,
                    t_choque: -1.0,
                },
                |mut m, &j| {
                    let (i, choque, edad, losa, t_c) = muertas[j];
                    m.largo += 0.5 * self.bordes[i].largo_km;
                    m.edad = m.edad.max(edad);
                    m.losa = m.losa.max(losa);
                    m.choque |= choque;
                    m.t_choque = m.t_choque.max(t_c);
                    m
                },
            );
            let (a, b) = par(&b0);
            // Rotura de la losa (v4 §6.6): también si el resto de la fosa sigue viva, como bajo la
            // India mientras el Tetis sigue hundiéndose a los lados.
            if m.choque && m.t_choque >= 0.0 {
                self.fosas_paso.roturas += 1;
                eventos.push((
                    "rotura_losa",
                    vec![self.placas[a as usize].id, self.placas[b as usize].id],
                    m.largo,
                    Some(self.malla.dir[m.k as usize]),
                    Detalle {
                        tiempo_choque_myr: Some(m.t_choque),
                        losa_km: Some(m.losa),
                        ..Detalle::default()
                    },
                ));
            }
            let toca_viva = js.iter().any(|&j| {
                let b = self.bordes[muertas[j].0];
                self.malla.vec[b.k as usize].iter().any(|&v| {
                    v != NADA && {
                        let r = reg_en[v as usize];
                        r != NADA && vive[r as usize] && par(&self.bordes[r as usize]) == par(&b)
                    }
                })
            });
            if toca_viva {
                continue;
            }
            let causa = m.choque as usize;
            self.fosas_paso.mueren[causa] += 1;
            eventos.push((
                "muerte_fosa",
                vec![self.placas[a as usize].id, self.placas[b as usize].id],
                m.largo,
                Some(self.malla.dir[m.k as usize]),
                Detalle {
                    clase: Some(
                        if causa == 0 {
                            "sin_convergencia"
                        } else {
                            "choque"
                        }
                        .into(),
                    ),
                    edad_fosa_myr: Some(m.edad),
                    losa_km: Some(m.losa),
                    tiempo_choque_myr: (m.t_choque >= 0.0).then_some(m.t_choque),
                    ..Detalle::default()
                },
            ));
        }
        for (p, m) in nuevos.into_iter().enumerate() {
            self.placas[p].fosas = m;
        }
        for (tipo, placas, largo, punto, det) in eventos {
            self.evento_det(tipo, placas, largo, 0.0, punto, det);
        }
    }

    /// Nacimiento espontáneo (nota de fosas v4 §6.4), raro: solo en fallas transformantes entre
    /// fondos oceánicos con un contraste de edad mayor que la mitad de la del lado viejo y fondo de
    /// más de `a_min` en ese lado, como en Izu-Bonin-Marianas. Los márgenes pasivos no son
    /// candidatos: no empiezan a hundirse solos (los cierra la invasión, [`Deriva::invadir`]). Por
    /// segmento de unos `L_borde` (bloques de la malla), `p = 1 − exp(−λ_esp·((a − a_min)/100 Myr)·
    /// L·Δt)`: una tirada sembrada por segmento y paso, en orden fijo. Se hunde el lado viejo, con
    /// `L = X_ini`, y desde sus extremos puede invadir. Deja puesto en los bordes el estado de los
    /// tramos.
    pub(super) fn nacimientos_espontaneos(&mut self) {
        let lam = self.cfg.lambda_esp;
        if lam <= 0.0 {
            self.aplicar_estado_fosas();
            return;
        }
        let a_min = self.cfg.a_min_myr;
        let dt = self.cfg.dt_myr;
        let lado = self.malla.lado_km;
        let n = self.malla.n as usize;
        let s = ((self.cfg.l_borde_km / lado).round() as usize).max(1);
        let ns = n.div_ceil(s);
        let bloque = |k: usize| -> u64 {
            let (c, r) = (k / (n * n), k % (n * n));
            let (j, i) = (r / n, r % n);
            ((c * ns + j / s) * ns + i / s) as u64
        };
        let prob = |a: f64, largo: f64| 1.0 - (-lam * ((a - a_min) / 100.0) * largo * dt).exp();
        // B. Transformantes con contraste de edad y fondo viejo: (par, bloque) → candidato.
        let viejo_transf = |d: &Deriva, b: &Borde| -> Option<(f64, u32)> {
            if b.tipo != TipoBorde::Transformante {
                return None;
            }
            let (cp, cq) = (d.corteza(b.k as usize)?, d.corteza(b.k_q as usize)?);
            if cp.cont || cq.cont {
                return None;
            }
            let vieja = cp.edad_oc.max(cq.edad_oc);
            if vieja <= a_min || (cp.edad_oc - cq.edad_oc).abs() <= 0.5 * vieja {
                return None;
            }
            Some((vieja, if cp.edad_oc > cq.edad_oc { b.p } else { b.q }))
        };
        let mut trans: std::collections::BTreeMap<((u32, u32), u64), CandTransf> =
            Default::default();
        for (i, b) in self.bordes.iter().enumerate() {
            let Some((vieja, hunde)) = viejo_transf(self, b) else {
                continue;
            };
            if self
                .estado_fosa(b.p, self.rep[b.k as usize], b.q)
                .is_some_and(|e| e.activa)
            {
                continue;
            }
            let par = (b.p.min(b.q), b.p.max(b.q));
            let e = trans.entry((par, bloque(b.k as usize))).or_default();
            e.largo += 0.5 * b.largo_km;
            e.edad_l += vieja * 0.5 * b.largo_km;
            e.votos[(hunde == par.1) as usize] += b.largo_km;
            e.regs.push(i);
        }
        let mut transformantes: Vec<(u32, u32, Vec<usize>, f64)> = Vec::new();
        for (&((pa, pb), b), c) in &trans {
            let a = c.edad_l / c.largo;
            let id = self.placas[pa as usize].id ^ splitmix64(self.placas[pb as usize].id);
            if azar(self.seed, self.paso, id ^ splitmix64(b), P_ESPONTANEO, 1) >= prob(a, c.largo) {
                continue;
            }
            let hunde = if c.votos[1] > c.votos[0] { pb } else { pa };
            let arriba = if hunde == pa { pb } else { pa };
            transformantes.push((arriba, hunde, c.regs.clone(), a));
        }
        // Fosas de las transformantes: solo el tramo contiguo con fondo de más de a_min.
        for (arriba, hunde, semilla, a) in transformantes {
            let regs = self.tramo_de(&semilla, |b| viejo_transf(self, b).is_some());
            let largo: f64 = regs.iter().map(|&i| 0.5 * self.bordes[i].largo_km).sum();
            self.poner_fosa(&regs, arriba, hunde);
            self.fosas_paso.nacen[2] += 1;
            let k = self.bordes[regs[0]].k;
            self.evento_det(
                "nacimiento_fosa",
                vec![
                    self.placas[hunde as usize].id,
                    self.placas[arriba as usize].id,
                ],
                largo,
                0.0,
                Some(self.malla.dir[k as usize]),
                Detalle {
                    clase: Some("espontaneo_transformante".into()),
                    edad_fondo_myr: Some(a),
                    ..Detalle::default()
                },
            );
        }
        self.aplicar_estado_fosas();
    }

    /// Invasión (nota de fosas v4 §6.4), la vía por la que se cierran los océanos viejos: cada
    /// extremo de una fosa activa que no choca se alarga `v_inv·Δt` por paso por el borde de más
    /// allá, celda a celda y en orden fijo (por capas desde el extremo), si al lado hay fondo
    /// oceánico de más de `a_inv`:
    /// - **por una falla transformante de la placa que se hunde**, si la polaridad es compatible
    ///   (su lado, oceánico, se hunde bajo un continente, un arco o un fondo más joven): el tramo
    ///   nuevo es parte de la misma fosa y hereda su losa y su edad;
    /// - **por un margen pasivo** (océano y continente de la misma placa): el frente avanza por las
    ///   celdas continentales del margen sin mover nada, porque la placa aún no se ha roto ahí.
    ///   Cada `√A_min` de margen invadido, o cuando el margen viejo se acaba, se intenta separar el
    ///   océano de delante como placa nueva ([`Deriva::partir_por_fosa`]): la fosa ocupa el tramo
    ///   invadido, con el océano hundiéndose bajo el continente y la losa de la fosa que invade.
    ///
    /// La invasión se detiene donde el fondo de al lado tiene menos de `a_inv`, donde llega a otro
    /// borde o cuando la fosa muere (un frente que nadie toca en `τ_muerte` se olvida).
    pub(super) fn invadir(&mut self) {
        let cfg = self.cfg.clone();
        let v = cfg.v_inv_km_myr;
        if v <= 0.0 || self.bordes.is_empty() {
            return;
        }
        let num = self.malla.num;
        let lado = self.malla.lado_km;
        let dt = cfg.dt_myr;
        let a_inv = cfg.a_inv_myr;
        // Celdas que avanza cada frente en este paso: las que pasa `v_inv·t` de un múltiplo de
        // `lado` al siguiente (sin acumular nada por frente, la media es exacta).
        let t1 = (self.paso + 1) as f64 * v * dt / lado;
        let t0 = self.paso as f64 * v * dt / lado;
        let n_paso = (t1.floor() - t0.floor()).max(0.0) as u32;
        let nb = self.bordes.len();
        let mut reg_en = vec![NADA; num];
        for (i, b) in self.bordes.iter().enumerate() {
            reg_en[b.k as usize] = i as u32;
        }
        let est: Vec<Option<EstadoFosa>> = self
            .bordes
            .iter()
            .map(|b| self.estado_fosa(b.p, self.rep[b.k as usize], b.q))
            .collect();
        // Fosa viva: un borde convergente con fosa activa, sin choque y hundiéndose en este paso (una
        // fosa que ya no converge no rasga la placa de al lado). Las transformantes que heredan el
        // estado de la fosa de al lado (a dos anillos) no cuentan como fosa para hallar su extremo.
        let viva = |d: &Deriva, i: usize| {
            d.bordes[i].tipo == TipoBorde::Subduccion
                && est[i].is_some_and(|e| e.activa && e.choque_myr < 0.0 && e.sin_conv_myr <= 0.0)
        };
        // Fuera de la fosa: un borde que no es fosa viva ni choque (el borde de un choque no es el
        // extremo de una fosa: la losa sigue a los lados, como el Tetis junto a la India).
        let fuera = |d: &Deriva, j: usize| !viva(d, j) && d.bordes[j].tipo != TipoBorde::Colision;
        // Extremos: bordes de fosa viva con algún borde vecino de su placa fuera de la fosa.
        let extremo: Vec<bool> = (0..nb)
            .map(|i| {
                viva(self, i)
                    && self.malla.vec[self.bordes[i].k as usize].iter().any(|&w| {
                        w != NADA && {
                            let j = reg_en[w as usize];
                            j != NADA
                                && self.bordes[j as usize].p == self.bordes[i].p
                                && fuera(self, j as usize)
                        }
                    })
            })
            .collect();

        // A. Transformantes de la placa que se hunde.
        let compatible = |d: &Deriva, j: usize| -> bool {
            let b = &d.bordes[j];
            if b.tipo != TipoBorde::Transformante {
                return false;
            }
            let (Some(cp), Some(cq)) = (d.corteza(b.k as usize), d.corteza(b.k_q as usize)) else {
                return false;
            };
            !cp.cont && cp.edad_oc > a_inv && (cq.cont || cq.edad_oc < cp.edad_oc)
        };
        let libre = |j: usize| est[j].is_none_or(|e| !e.activa);
        // Camino de coste 0–1 por las transformantes de la placa que se hunde: atravesar las que ya
        // tienen el estado de la fosa no cuesta; invadir una nueva cuesta 1, hasta `n_paso`.
        let paso_por = |d: &Deriva, j: usize| -> Option<u32> {
            if !compatible(d, j) {
                return None;
            }
            Some(u32::from(libre(j)))
        };
        // Registro → (coste desde el extremo, losa y edad heredadas).
        let mut capa: HashMap<usize, (u32, f64, f64)> = HashMap::new();
        let mut cola: std::collections::VecDeque<usize> = Default::default();
        if n_paso > 0 {
            let mut semillas: Vec<(usize, u32, f64, f64)> = Vec::new();
            for i in 0..nb {
                let Some(e) = est[i] else {
                    continue;
                };
                if !extremo[i] || e.arriba {
                    continue;
                }
                let b = self.bordes[i];
                for &w in &self.malla.vec[b.k as usize] {
                    if w == NADA || reg_en[w as usize] == NADA {
                        continue;
                    }
                    let j = reg_en[w as usize] as usize;
                    if self.bordes[j].p != b.p {
                        continue;
                    }
                    if let Some(c) = paso_por(self, j) {
                        semillas.push((j, c, e.x_km, e.edad_myr));
                    }
                }
            }
            semillas.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(b.2.total_cmp(&a.2)));
            semillas.dedup_by_key(|x| x.0);
            for (j, c, l, ed) in semillas {
                if c <= n_paso {
                    capa.insert(j, (c, l, ed));
                    if c == 0 {
                        cola.push_front(j);
                    } else {
                        cola.push_back(j);
                    }
                }
            }
        }
        let mut invadidos: Vec<usize> = Vec::new();
        let mut hecho_t: std::collections::HashSet<usize> = Default::default();
        while let Some(j) = cola.pop_front() {
            if !hecho_t.insert(j) {
                continue;
            }
            let (c, l, ed) = capa[&j];
            if libre(j) {
                invadidos.push(j);
            }
            let b = self.bordes[j];
            for &w in &self.malla.vec[b.k as usize] {
                if w == NADA || reg_en[w as usize] == NADA {
                    continue;
                }
                let j2 = reg_en[w as usize] as usize;
                if self.bordes[j2].p != b.p || hecho_t.contains(&j2) {
                    continue;
                }
                let Some(cw) = paso_por(self, j2).map(|x| c + x) else {
                    continue;
                };
                if cw > n_paso || capa.get(&j2).is_some_and(|x| x.0 <= cw) {
                    continue;
                }
                capa.insert(j2, (cw, l, ed));
                if cw == c {
                    cola.push_front(j2);
                } else {
                    cola.push_back(j2);
                }
            }
        }
        invadidos.sort_unstable();
        for j in invadidos {
            let b = self.bordes[j];
            let (_, l_km, edad) = capa[&j];
            let fondo = self.edad_losa_celda(b.k, b.p).max(0.0);
            let (id_p, id_q) = (self.placas[b.p as usize].id, self.placas[b.q as usize].id);
            let l = self.rep[b.k as usize];
            self.placas[b.p as usize]
                .fosas
                .insert(l, EstadoFosa::activa(id_q, false, l_km, edad, fondo));
            let lq = self.rep[b.k_q as usize];
            if lq != NADA && self.dueno[b.k_q as usize] == b.q {
                self.placas[b.q as usize]
                    .fosas
                    .insert(lq, EstadoFosa::activa(id_p, true, l_km, edad, fondo));
            }
            self.fosas_paso.invadido_km[0] += b.largo_km;
        }

        // B. Márgenes pasivos: el borde de un continente o un microcontinente (un cuerpo continental
        // de su placa de al menos (3·L_borde)², ~0,2 Mkm²; Madagascar, 0,59) con fondo oceánico suyo
        // de más de a_inv al lado. Ni una celda continental suelta (un diente) ni un arco de islas que
        // aún no se ha unido a un continente son un margen pasivo.
        let continente = |c: &Corteza| c.continente();
        let a_cuerpo = (3.0 * cfg.l_borde_km).powi(2);
        let mut cuerpo_grande = vec![false; num];
        {
            let mut visto = vec![false; num];
            let mut pila: Vec<u32> = Vec::new();
            let mut celdas: Vec<u32> = Vec::new();
            for k0 in 0..num {
                let p = self.dueno[k0];
                if visto[k0] || p == NADA || !self.corteza(k0).is_some_and(continente) {
                    continue;
                }
                celdas.clear();
                let mut area = 0.0;
                visto[k0] = true;
                pila.push(k0 as u32);
                while let Some(k) = pila.pop() {
                    celdas.push(k);
                    area += self.malla.area[k as usize];
                    for &w in &self.malla.vec[k as usize] {
                        let wu = w as usize;
                        if w != NADA
                            && !visto[wu]
                            && self.dueno[wu] == p
                            && self.corteza(wu).is_some_and(continente)
                        {
                            visto[wu] = true;
                            pila.push(w);
                        }
                    }
                }
                if area >= a_cuerpo {
                    for &k in &celdas {
                        cuerpo_grande[k as usize] = true;
                    }
                }
            }
        }
        let margen_viejo = |d: &Deriva, k: usize| -> bool {
            let p = d.dueno[k];
            p != NADA
                && cuerpo_grande[k]
                && d.rep[k] != NADA
                && d.corteza(k).is_some_and(continente)
                && d.malla.vec[k].iter().any(|&w| {
                    w != NADA
                        && d.dueno[w as usize] == p
                        && d.corteza(w as usize)
                            .is_some_and(|c| !c.cont && c.edad_oc > a_inv)
                })
        };
        let invadida = |d: &Deriva, k: usize| -> bool {
            let p = d.dueno[k];
            p != NADA && d.placas[p as usize].invadidas.contains_key(&d.rep[k])
        };
        // Semillas: celdas de margen viejo de una de las dos placas a menos de dos anillos de un
        // extremo de fosa viva (a la escala de la malla, el extremo y la costa pueden quedar a dos
        // celdas).
        let mut sem: Vec<(u32, f64, f64)> = Vec::new();
        for i in 0..nb {
            let Some(e) = est[i] else {
                continue;
            };
            if !extremo[i] {
                continue;
            }
            let b = self.bordes[i];
            // Anillos alrededor de las dos celdas del borde; vale el más cercano con margen viejo.
            let anillo0: Vec<u32> = vec![b.k, b.k_q];
            let mut anillo1: Vec<u32> = Vec::new();
            for &c in &anillo0 {
                anillo1.extend(
                    self.malla.vec[c as usize]
                        .iter()
                        .copied()
                        .filter(|&v| v != NADA),
                );
            }
            let mut anillo2: Vec<u32> = Vec::new();
            for &c in &anillo1 {
                anillo2.extend(
                    self.malla.vec[c as usize]
                        .iter()
                        .copied()
                        .filter(|&v| v != NADA),
                );
            }
            for mut anillo in [anillo0, anillo1, anillo2] {
                anillo.sort_unstable();
                anillo.dedup();
                let antes = sem.len();
                for w in anillo {
                    if (self.dueno[w as usize] == b.p || self.dueno[w as usize] == b.q)
                        && margen_viejo(self, w as usize)
                    {
                        sem.push((w, e.x_km, e.edad_myr));
                    }
                }
                if sem.len() > antes {
                    break;
                }
            }
        }
        sem.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.total_cmp(&a.1)));
        sem.dedup_by_key(|x| x.0);
        // Cada frente (semillas contiguas, de un mismo extremo) avanza como mucho `n_paso` celdas
        // nuevas por paso: su identificador es la menor celda de su grupo de semillas.
        let mut frente_de: HashMap<u32, u32> = HashMap::new();
        {
            let celdas: std::collections::HashSet<u32> = sem.iter().map(|x| x.0).collect();
            for &(w, ..) in &sem {
                if frente_de.contains_key(&w) {
                    continue;
                }
                let mut grupo = vec![w];
                let mut pila = vec![w];
                let mut vistos: std::collections::HashSet<u32> = [w].into_iter().collect();
                while let Some(c) = pila.pop() {
                    for &v in &self.malla.vec[c as usize] {
                        if v != NADA && celdas.contains(&v) && vistos.insert(v) {
                            grupo.push(v);
                            pila.push(v);
                        }
                    }
                }
                let id = *grupo.iter().min().expect("grupo");
                for c in grupo {
                    frente_de.insert(c, id);
                }
            }
        }
        // Camino de coste 0–1 por el margen: atravesar lo ya invadido no cuesta; invadir una celda
        // nueva cuesta 1, hasta `n_paso`.
        let mut coste: HashMap<u32, (u32, f64, f64, u32)> = HashMap::new();
        let mut cola: std::collections::VecDeque<u32> = Default::default();
        for (w, l, ed) in sem {
            let c = u32::from(!invadida(self, w as usize));
            if c <= n_paso {
                coste.insert(w, (c, l, ed, frente_de[&w]));
                if c == 0 {
                    cola.push_front(w);
                } else {
                    cola.push_back(w);
                }
            }
        }
        let mut hecho: std::collections::HashSet<u32> = Default::default();
        let mut nuevas_por_frente: HashMap<u32, u32> = HashMap::new();
        let mut alcanzadas: Vec<(u32, f64, f64)> = Vec::new();
        while let Some(k) = cola.pop_front() {
            if !hecho.insert(k) {
                continue;
            }
            let (c, l, ed, f) = coste[&k];
            if !invadida(self, k as usize) {
                let n = nuevas_por_frente.entry(f).or_insert(0);
                if *n >= n_paso {
                    continue;
                }
                *n += 1;
            }
            alcanzadas.push((k, l, ed));
            for &w in &self.malla.vec[k as usize] {
                if w == NADA
                    || self.dueno[w as usize] != self.dueno[k as usize]
                    || hecho.contains(&w)
                    || !margen_viejo(self, w as usize)
                {
                    continue;
                }
                let cw = c + u32::from(!invadida(self, w as usize));
                if cw > n_paso || coste.get(&w).is_some_and(|x| x.0 <= cw) {
                    continue;
                }
                coste.insert(w, (cw, l, ed, f));
                if cw == c {
                    cola.push_front(w);
                } else {
                    cola.push_back(w);
                }
            }
        }
        alcanzadas.sort_unstable_by_key(|x| x.0);
        let np = self.placas.len();
        let (mut tocada, mut crecio) = (vec![false; np], vec![false; np]);
        let paso = self.paso;
        for (k, l_km, edad) in alcanzadas {
            let p = self.dueno[k as usize] as usize;
            let l = self.rep[k as usize];
            tocada[p] = true;
            match self.placas[p].invadidas.get_mut(&l) {
                Some(inv) => {
                    inv.paso = paso;
                    inv.l_km = inv.l_km.max(l_km);
                }
                None => {
                    self.placas[p].invadidas.insert(
                        l,
                        Invadida {
                            l_km,
                            edad_myr: edad,
                            paso,
                        },
                    );
                    crecio[p] = true;
                    self.fosas_paso.invadido_km[1] += lado;
                }
            }
        }
        // Intentos de separar el océano de delante: cada 3·L_borde de margen invadido.
        let l_trozo = 3.0 * cfg.l_borde_km;
        let vida = (cfg.tau_muerte_myr / dt).ceil() as u64;
        let mut partidas: Vec<(usize, usize, f64, f64, f64, u32)> = Vec::new();
        for p in 0..np {
            if !self.placas[p].viva || self.placas[p].invadidas.is_empty() {
                continue;
            }
            let antes = self.placas[p].invadidas.len();
            self.placas[p]
                .invadidas
                .retain(|_, x| paso.saturating_sub(x.paso) <= vida);
            self.fosas_paso.separar[2] += (antes - self.placas[p].invadidas.len()) as u32;
            if self.placas[p].invadidas.is_empty() {
                self.placas[p].inv_intento_km = 0.0;
                continue;
            }
            let largo = self.placas[p].invadidas.len() as f64 * lado;
            let ultimo = self.placas[p].inv_intento_km;
            let agotado = tocada[p] && !crecio[p] && n_paso > 0;
            if !(largo >= ultimo + l_trozo || (agotado && largo > ultimo)) {
                continue;
            }
            self.placas[p].inv_intento_km = largo;
            // Celdas del mundo del tramo invadido (las que el mundo ve de esta placa).
            let pl = &self.placas[p];
            let mut tramo: std::collections::HashSet<u32> = Default::default();
            let (mut l_max, mut edad) = (0.0f64, 0.0f64);
            for (&l, inv) in &pl.invadidas {
                let s = pl.mapa[l as usize];
                if s == NADA {
                    continue;
                }
                let k = pl.mundo[s as usize];
                if self.dueno[k as usize] == p as u32 && self.rep[k as usize] == l {
                    tramo.insert(k);
                    l_max = l_max.max(inv.l_km);
                    edad = edad.max(inv.edad_myr);
                }
            }
            let Some(&k0) = tramo.iter().min() else {
                continue;
            };
            self.fosas_paso.separar[0] += 1;
            if let Some((h_c, h_o)) = self.partir_por_fosa(p, &tramo, &cuerpo_grande) {
                self.estampar(h_c, lote::INVASION);
                self.estampar(h_o, lote::INVASION);
                self.placas[h_c].invadidas.clear();
                self.placas[h_c].inv_intento_km = 0.0;
                partidas.push((h_c, h_o, l_max, edad, largo, k0));
            } else {
                self.fosas_paso.separar[1] += 1;
            }
        }
        if partidas.is_empty() {
            return;
        }
        // La fosa del océano separado: el borde nuevo entre el continente y la placa oceánica, solo
        // donde hay corteza continental a un lado (los cortes por el océano son transformantes).
        self.clasificar_bordes();
        for (h_c, h_o, l_km, edad, largo, k0) in partidas {
            let regs: Vec<usize> = (0..self.bordes.len())
                .filter(|&i| {
                    let b = &self.bordes[i];
                    let par = (b.p as usize, b.q as usize);
                    (par == (h_c, h_o) || par == (h_o, h_c))
                        && (self.corteza(b.k as usize).is_some_and(|c| c.cont)
                            || self.corteza(b.k_q as usize).is_some_and(|c| c.cont))
                })
                .collect();
            if regs.is_empty() {
                continue;
            }
            self.poner_fosa_con(&regs, h_c as u32, h_o as u32, l_km, edad);
            self.fosas_paso.nacen[1] += 1;
            let (id_c, id_o) = (self.placas[h_c].id, self.placas[h_o].id);
            let area_oc: f64 = (0..num)
                .filter(|&k| self.dueno[k] as usize == h_o)
                .map(|k| self.malla.area[k])
                .sum();
            let punto = Some(self.malla.dir[k0 as usize]);
            self.evento_det(
                "invasion_margen",
                vec![id_o, id_c],
                largo,
                area_oc,
                punto,
                Detalle {
                    losa_km: Some(l_km),
                    edad_fosa_myr: Some(edad),
                    ..Detalle::default()
                },
            );
            self.evento("placa_nueva", vec![id_c, id_o], 0.0, area_oc, punto);
        }
        self.aplicar_estado_fosas();
    }

    /// Bordes conexos (8 vecinas) del mismo par que `semilla` que cumplen `vale`.
    fn tramo_de(&self, semilla: &[usize], vale: impl Fn(&Borde) -> bool) -> Vec<usize> {
        let num = self.malla.num;
        let mut reg_en = vec![NADA; num];
        for (i, b) in self.bordes.iter().enumerate() {
            reg_en[b.k as usize] = i as u32;
        }
        let b0 = &self.bordes[semilla[0]];
        let par = (b0.p.min(b0.q), b0.p.max(b0.q));
        let mut visto = vec![false; self.bordes.len()];
        let mut pila: Vec<usize> = semilla.to_vec();
        for &i in semilla {
            visto[i] = true;
        }
        let mut out = Vec::new();
        while let Some(i) = pila.pop() {
            out.push(i);
            for &v in &self.malla.vec[self.bordes[i].k as usize] {
                if v == NADA {
                    continue;
                }
                let j = reg_en[v as usize];
                if j == NADA || visto[j as usize] {
                    continue;
                }
                let bj = &self.bordes[j as usize];
                if (bj.p.min(bj.q), bj.p.max(bj.q)) == par && vale(bj) {
                    visto[j as usize] = true;
                    pila.push(j as usize);
                }
            }
        }
        out.sort_unstable();
        out
    }

    /// Fosa recién nacida espontáneamente en los bordes `regs`, con la losa que ya cuelga: la
    /// litosfera vieja se hunde por su peso desde el principio (`L = X_ini`, decisión de quien
    /// implementa: con `L = 0` no tiraría nunca y moriría a los τ_muerte).
    fn poner_fosa(&mut self, regs: &[usize], arriba: u32, hunde: u32) {
        self.poner_fosa_con(regs, arriba, hunde, self.cfg.x_ini_km, 0.0);
    }

    /// Fosa activa en los bordes `regs` con losa `x` (km) y edad `edad` (Myr): la que nace o la que
    /// hereda un tramo invadido.
    fn poner_fosa_con(&mut self, regs: &[usize], arriba: u32, hunde: u32, x: f64, edad: f64) {
        for &i in regs {
            let b = self.bordes[i];
            if b.p != arriba && b.p != hunde {
                continue;
            }
            let l = self.rep[b.k as usize];
            let otra = self.placas[b.q as usize].id;
            let edad_fondo = self.edad_losa_celda(b.k, hunde).max(0.0);
            self.placas[b.p as usize].fosas.insert(
                l,
                EstadoFosa::activa(otra, b.p == arriba, x, edad, edad_fondo),
            );
        }
    }

    /// Fosas activas ya maduras (losa de `l_km`) en todos los tramos convergentes de corteza
    /// oceánica, con la polaridad del nacimiento: para escenarios que empiezan con una subducción en
    /// marcha (India–Asia) y para las pruebas de geometría.
    pub fn activar_fosas(&mut self, l_km: f64) {
        self.clasificar_bordes();
        let convergente = |b: &Borde| matches!(b.tipo, TipoBorde::Subduccion | TipoBorde::Difusa);
        let mut hecho = vec![false; self.bordes.len()];
        for i in 0..self.bordes.len() {
            if hecho[i] || !convergente(&self.bordes[i]) {
                continue;
            }
            let regs = self.tramo_de(&[i], convergente);
            for &j in &regs {
                hecho[j] = true;
            }
            let b0 = self.bordes[i];
            let (pa, pb) = (b0.p.min(b0.q), b0.p.max(b0.q));
            let (arriba, hunde, _) = self.polaridad_al_nacer(&regs, pa, pb);
            for &j in &regs {
                let b = self.bordes[j];
                let l = self.rep[b.k as usize];
                let otra = self.placas[b.q as usize].id;
                let edad_fondo = self.corteza(b.k as usize).map_or(0.0, |c| c.edad_oc);
                self.placas[b.p as usize].fosas.insert(
                    l,
                    EstadoFosa::activa(otra, b.p == arriba, l_km, 0.0, edad_fondo),
                );
                let _ = hunde;
            }
        }
        self.aplicar_estado_fosas();
    }

    /// Parte la placa `pi` para una fosa espontánea en su margen pasivo (nota de fosas v3 §6.4): la
    /// placa nueva es el océano de la placa que queda delante del tramo de fosa `tramo` (celdas
    /// continentales del margen): el que tiene ese tramo como margen más cercano por el océano. Así,
    /// desde los extremos de la fosa, el corte atraviesa el océano hasta los bordes existentes, como
    /// los desgarres del extremo de una fosa real. Devuelve los índices de la placa continental y de
    /// la oceánica, o `None` si alguna quedaría por debajo de `A_min`.
    pub(super) fn partir_por_fosa(
        &mut self,
        pi: usize,
        tramo: &std::collections::HashSet<u32>,
        continente: &[bool],
    ) -> Option<(usize, usize)> {
        let num = self.malla.num;
        let p = pi as u32;
        // El océano de la placa y lo que flota en él (trozos sueltos, arcos): solo los continentes
        // (`continente`, cuerpos de al menos (3·L_borde)²) son márgenes que se reparten el océano.
        let oceano = |d: &Deriva, k: usize| d.dueno[k] == p && d.rep[k] != NADA && !continente[k];
        // Fuentes: el fondo oceánico junto al margen, con 1 si toca el tramo de fosa.
        let mut fuentes: Vec<(u32, f64, u32)> = Vec::new();
        for k in 0..num {
            if !oceano(self, k) {
                continue;
            }
            let mut junto = None;
            for &v in &self.malla.vec[k] {
                if v != NADA && self.dueno[v as usize] == p && continente[v as usize] {
                    junto = Some(junto.unwrap_or(0).max(tramo.contains(&v) as u32));
                }
            }
            if let Some(e) = junto {
                fuentes.push((k as u32, 0.0, e));
            }
        }
        let oc: Vec<bool> = (0..num).map(|k| oceano(self, k)).collect();
        let mut campo = std::mem::take(&mut self.campo);
        campo.calcular(&self.malla, &fuentes, |u, v| oc[u] && oc[v], f64::INFINITY);
        let delante: Vec<bool> = (0..num).map(|k| oc[k] && campo.fuente[k] == 1).collect();
        self.campo = campo;
        let (mut a_o, mut a_tot) = (0.0, 0.0);
        for (k, &d) in delante.iter().enumerate() {
            if self.dueno[k] == p {
                a_tot += self.malla.area[k];
                if d {
                    a_o += self.malla.area[k];
                }
            }
        }
        // El océano que se separa puede ser una microplaca que se hunde, desde (3·L_borde)² (como
        // Juan de Fuca, 0,25 Mkm²); lo que queda de la placa, al menos A_min.
        let a_min = self.cfg.a_min_frac * self.malla.area_total();
        let a_micro = (3.0 * self.cfg.l_borde_km).powi(2);
        if a_o <= 0.0 || a_o < a_micro || a_tot - a_o < a_min {
            return None;
        }
        let paso = self.paso;
        let t = self.t_myr;
        let madre = &self.placas[pi];
        let mut h_c = Placa::hermana(madre, id_hija(madre.id, paso, 2), t);
        let mut h_o = Placa::hermana(madre, id_hija(madre.id, paso, 3), t);
        for s in 0..madre.celda.len() {
            let l = madre.celda[s];
            let c = madre.cort[s];
            let k = madre.mundo[s] as usize;
            let dest = if delante[k] && !continente[k] {
                &mut h_o
            } else {
                &mut h_c
            };
            dest.insertar(l, c, madre.mundo[s]);
            if let Some(e) = madre.fosas.get(&l) {
                dest.fosas.insert(l, *e);
            }
            if let Some(e) = madre.invadidas.get(&l) {
                dest.invadidas.insert(l, *e);
            }
        }
        h_c.actualizar_cap(&self.malla);
        h_o.actualizar_cap(&self.malla);
        self.placas[pi].morir();
        let ic = self.placas.len();
        self.placas.push(h_c);
        self.placas.push(h_o);
        for k in 0..num {
            if self.dueno[k] == p {
                let l = self.rep[k];
                self.dueno[k] = if self.placas[ic].mapa[l as usize] != NADA {
                    ic as u32
                } else {
                    ic as u32 + 1
                };
            }
        }
        Some((ic, ic + 1))
    }

    /// Convergencia difusa (nota de fosas v3 §6.2): el área oceánica solapada de un tramo difuso se
    /// retira como en una fosa, pero sin arco ni sumideros, y su basalto no pasa nunca a corteza
    /// continental (a esta escala, el pliegue del fondo apenas cambia el relieve). La corteza de tipo
    /// continental o de arco y el sedimento de la muestra se reparten como espesor a menos de `W_dif`
    /// (= `L_borde`), por área: en las celdas continentales de las dos placas si las hay; si los dos
    /// lados son oceánicos, entre los dos. Se conserva el volumen continental.
    pub(super) fn absorber_difusa(&mut self, lista: &[Difusa]) {
        self.fosas_paso.area_difusa_km2 = 0.0;
        if lista.is_empty() {
            return;
        }
        let inv = self.indice_inverso();
        let cfg = self.cfg.clone();
        let (rs, rc, h_cont_min) = (cfg.rho_s, cfg.rho_c, cfg.h_cont_min_km);
        for d in lista {
            self.fosas_paso.area_difusa_km2 += d.a;
            let v_hc = d.c.hc * d.a;
            let v_sed = d.c.sed * d.a;
            if v_hc + v_sed <= 0.0 {
                continue;
            }
            let celdas: Vec<u32> = self
                .vecindad
                .de(d.k as usize)
                .iter()
                .copied()
                .filter(|&j| {
                    let p = self.dueno[j as usize];
                    (p == d.ganadora || p == d.perdedora) && self.rep[j as usize] != NADA
                })
                .collect();
            let continentales: Vec<u32> = celdas
                .iter()
                .copied()
                .filter(|&j| self.corteza(j as usize).is_some_and(|c| c.cont))
                .collect();
            let mut destino = if continentales.is_empty() {
                celdas
            } else {
                continentales
            };
            if destino.is_empty() {
                destino.push(d.k);
            }
            let a_tot: f64 = destino.iter().map(|&j| self.malla.area[j as usize]).sum();
            let mut puesto = 0.0;
            for j in destino {
                let w = self.malla.area[j as usize] / a_tot;
                puesto += self.en_celda(&inv, j as usize, w, false, |c, _, f| {
                    // `f` = w / Σ área de las muestras de la celda: el espesor por km³ repartido.
                    if v_hc > 0.0 {
                        c.sumar_hc(v_hc * f, d.c.arco, rs, rc, h_cont_min, via::DIFUSA);
                        c.dh_dt += v_hc * f / cfg.dt_myr;
                        c.edad_orog = 0.0;
                    }
                    c.sed += v_sed * f;
                });
            }
            // Lo que no encontró muestra (no pasa en una deriva normal) se cuenta como sin sitio.
            if puesto < 1.0 - 1e-9 {
                self.cuentas.choque_sin_sitio_km3 += (1.0 - puesto) * (v_hc + v_sed * rs / rc);
            }
        }
    }
}
