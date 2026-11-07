//! Ciclo de vida de las fosas (`nota_ciclo_fosas.md`, v2): cada tramo convergente de corteza
//! oceánica es difuso (aún sin fosa) o una fosa activa, con memoria entre pasos. Un tramo difuso
//! acumula convergencia y, al llegar a `X_ini`, nace la fosa con su polaridad fija; una fosa alarga
//! su losa, que tira más cuanto más larga es, y muere si deja de converger o si le llega corteza que
//! flota. También nacen fosas espontáneas en márgenes pasivos viejos y en transformantes con mucho
//! contraste de edad.
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
}

/// Lo que pasó con las fosas en el último paso (nota de fosas §7–§8).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ConteoFosas {
    /// Nacimientos: inducidos, espontáneos en margen pasivo y en transformante.
    pub nacen: [u32; 3],
    /// Muertes: sin convergencia y por choque.
    pub mueren: [u32; 2],
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

/// Fosa que muere en un par: longitud, edad, losa, una celda y si fue por choque.
#[derive(Clone, Copy)]
struct Muerte {
    largo: f64,
    edad: f64,
    losa: f64,
    k: u32,
    choque: bool,
}

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
            match est {
                Some(e) if e.activa && nb.tipo != TipoBorde::Colision => {
                    if nb.tipo == TipoBorde::Difusa {
                        nb.tipo = TipoBorde::Subduccion;
                    }
                    nb.p_subduce = !e.arriba;
                    if !e.arriba {
                        nb.losa = (e.x_km / l_losa).clamp(0.0, 1.0);
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
        let mut muertas: Vec<(usize, bool, f64, f64)> = Vec::new();
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
            for &i in regs {
                let b = &self.bordes[i];
                if let Some(e) = previo[i] {
                    if e.activa {
                        let arriba = if e.arriba { b.p } else { b.q };
                        voto[(arriba == pb) as usize] += b.largo_km;
                        l_sum += e.x_km * b.largo_km;
                        l_w += b.largo_km;
                        edad = edad.max(e.edad_myr);
                    } else {
                        x_sum += e.x_km * b.largo_km;
                        x_w += b.largo_km;
                    }
                }
            }
            let punto = Some(self.malla.dir[b0.k as usize]);
            let ids = |a: u32, b: u32| vec![self.placas[a as usize].id, self.placas[b as usize].id];
            let (activa, arriba, x, edad_f) = if l_w > 0.0 {
                let arriba = if voto[1] > voto[0] { pb } else { pa };
                let hunde = if arriba == pa { pb } else { pa };
                let (flota, _) = self.flota_y_edad(regs, hunde);
                if flota >= 0.5 {
                    // Llega corteza que flota a la fosa (6.6): el choque sigue como estaba modelado
                    // y el tramo vuelve a difuso desde X = 0.
                    for &i in regs {
                        muertas.push((i, true, edad, l_sum / l_w));
                    }
                    (false, NADA, 0.0, 0.0)
                } else {
                    (true, arriba, l_sum / l_w + avance, edad + dt)
                }
            } else {
                let x = if x_w > 0.0 { x_sum / x_w } else { 0.0 } + avance;
                if x >= cfg.x_ini_km {
                    let (arriba, hunde, edad_fondo) = self.polaridad_al_nacer(regs, pa, pb);
                    self.fosas_paso.nacen[0] += 1;
                    eventos.push((
                        "nacimiento_fosa",
                        ids(hunde, arriba),
                        largo,
                        punto,
                        Detalle {
                            clase: Some("inducido".into()),
                            edad_fondo_myr: Some(edad_fondo),
                            ..Detalle::default()
                        },
                    ));
                    (true, arriba, 0.0, 0.0)
                } else {
                    (false, NADA, x, 0.0)
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
            if e.activa {
                let t = e.sin_conv_myr + dt;
                let causa = if b.tipo == TipoBorde::Colision {
                    Some(1)
                } else if t >= cfg.tau_muerte_myr {
                    Some(0)
                } else {
                    None
                };
                if let Some(c) = causa {
                    muertas.push((i, c == 1, e.edad_myr, e.x_km));
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
            let m = js.iter().fold(
                Muerte {
                    largo: 0.0,
                    edad: 0.0,
                    losa: 0.0,
                    k: b0.k,
                    choque: false,
                },
                |mut m, &j| {
                    let (i, choque, edad, losa) = muertas[j];
                    m.largo += 0.5 * self.bordes[i].largo_km;
                    m.edad = m.edad.max(edad);
                    m.losa = m.losa.max(losa);
                    m.choque |= choque;
                    m
                },
            );
            let (a, b) = par(&b0);
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

    /// Nacimiento espontáneo (nota de fosas v3 §6.4), solo donde el fondo que se hundiría tiene más
    /// de `a_min`: en márgenes pasivos (transiciones de océano a continente dentro de una placa, sin
    /// otra placa a menos de `L_borde`) y en transformantes entre fondos oceánicos con un contraste de
    /// edad mayor que la mitad de la del lado viejo. Por segmento de unos `L_borde` (bloques de la
    /// malla), `p = 1 − exp(−λ_esp·((a − a_min)/100 Myr)·L·Δt)`, con `L` la longitud candidata del
    /// segmento: una tirada sembrada por segmento y paso, en orden fijo. La fosa ocupa solo el tramo
    /// contiguo de fondo viejo. Devuelve si partió alguna placa.
    pub(super) fn nacimientos_espontaneos(&mut self) -> bool {
        let lam = self.cfg.lambda_esp;
        if lam <= 0.0 {
            self.aplicar_estado_fosas();
            return false;
        }
        let a_min = self.cfg.a_min_myr;
        let dt = self.cfg.dt_myr;
        let num = self.malla.num;
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
        // A. Márgenes pasivos con fondo de más de a_min al lado: celda continental → edad de su
        // fondo oceánico más viejo de la misma placa.
        let mut viejo: HashMap<u32, f64> = HashMap::new();
        for k in 0..num {
            let p = self.dueno[k];
            if p == NADA || !self.corteza(k).is_some_and(|c| c.cont) {
                continue;
            }
            let edad = self.malla.vec[k]
                .iter()
                .filter(|&&v| v != NADA && self.dueno[v as usize] == p)
                .filter_map(|&v| {
                    self.corteza(v as usize)
                        .filter(|c| !c.cont)
                        .map(|c| c.edad_oc)
                })
                .fold(f64::NEG_INFINITY, f64::max);
            if edad <= a_min {
                continue;
            }
            if self.vecindad.de(k).iter().any(|&j| {
                let d = self.dueno[j as usize];
                d != NADA && d != p
            }) {
                continue;
            }
            viejo.insert(k as u32, edad);
        }
        // (placa, bloque) → (longitud, Σ edad·longitud, celdas).
        let mut margen: std::collections::BTreeMap<(u32, u64), (f64, f64, Vec<u32>)> =
            Default::default();
        let mut celdas_viejas: Vec<u32> = viejo.keys().copied().collect();
        celdas_viejas.sort_unstable();
        for &k in &celdas_viejas {
            let p = self.dueno[k as usize];
            let e = margen
                .entry((p, bloque(k as usize)))
                .or_insert((0.0, 0.0, Vec::new()));
            e.0 += lado;
            e.1 += viejo[&k] * lado;
            e.2.push(k);
        }
        let mut partidas: Vec<(usize, usize, f64, u32)> = Vec::new();
        let mut ya: Vec<u32> = Vec::new();
        for (&(p, b), (largo, se, celdas)) in &margen {
            if ya.contains(&p) {
                continue;
            }
            let a = se / largo;
            let id = self.placas[p as usize].id;
            if azar(self.seed, self.paso, id ^ splitmix64(b), P_ESPONTANEO, 0) >= prob(a, *largo) {
                continue;
            }
            // El tramo contiguo (8 vecinas) de margen con fondo viejo al lado.
            let mut tramo: Vec<u32> = celdas.clone();
            let mut en: std::collections::HashSet<u32> = tramo.iter().copied().collect();
            let mut pila = tramo.clone();
            while let Some(c) = pila.pop() {
                for &v in &self.malla.vec[c as usize] {
                    if v != NADA
                        && self.dueno[v as usize] == p
                        && viejo.contains_key(&v)
                        && en.insert(v)
                    {
                        tramo.push(v);
                        pila.push(v);
                    }
                }
            }
            if let Some((h_cont, h_oc)) = self.partir_por_fosa(p as usize, &en) {
                ya.push(p);
                partidas.push((h_cont, h_oc, a, celdas[0]));
            }
        }
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
            if partidas
                .iter()
                .any(|x| [pa as usize, pb as usize].contains(&x.0))
            {
                continue;
            }
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
        if partidas.is_empty() {
            self.aplicar_estado_fosas();
            return false;
        }
        // Fosas de los márgenes: el borde nuevo entre el continente y la placa oceánica nueva, solo
        // donde hay corteza continental a un lado (los cortes por el océano son transformantes).
        self.clasificar_bordes();
        for (h_cont, h_oc, a, kc) in partidas {
            let regs: Vec<usize> = (0..self.bordes.len())
                .filter(|&i| {
                    let b = &self.bordes[i];
                    let par = (b.p as usize, b.q as usize);
                    (par == (h_cont, h_oc) || par == (h_oc, h_cont))
                        && (self.corteza(b.k as usize).is_some_and(|c| c.cont)
                            || self.corteza(b.k_q as usize).is_some_and(|c| c.cont))
                })
                .collect();
            if regs.is_empty() {
                continue;
            }
            let largo: f64 = regs.iter().map(|&i| 0.5 * self.bordes[i].largo_km).sum();
            self.poner_fosa(&regs, h_cont as u32, h_oc as u32);
            self.fosas_paso.nacen[1] += 1;
            let (id_c, id_o) = (self.placas[h_cont].id, self.placas[h_oc].id);
            let area_oc: f64 = (0..num)
                .filter(|&k| self.dueno[k] as usize == h_oc)
                .map(|k| self.malla.area[k])
                .sum();
            let punto = Some(self.malla.dir[kc as usize]);
            self.evento_det(
                "nacimiento_fosa",
                vec![id_o, id_c],
                largo,
                0.0,
                punto,
                Detalle {
                    clase: Some("espontaneo_margen".into()),
                    edad_fondo_myr: Some(a),
                    ..Detalle::default()
                },
            );
            self.evento("placa_nueva", vec![id_c, id_o], 0.0, area_oc, punto);
        }
        self.aplicar_estado_fosas();
        true
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
        let x = self.cfg.x_ini_km;
        for &i in regs {
            let b = self.bordes[i];
            if b.p != arriba && b.p != hunde {
                continue;
            }
            let l = self.rep[b.k as usize];
            let otra = self.placas[b.q as usize].id;
            self.placas[b.p as usize].fosas.insert(
                l,
                EstadoFosa {
                    otra,
                    activa: true,
                    arriba: b.p == arriba,
                    x_km: x,
                    edad_myr: 0.0,
                    sin_conv_myr: 0.0,
                },
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
                self.placas[b.p as usize].fosas.insert(
                    l,
                    EstadoFosa {
                        otra,
                        activa: true,
                        arriba: b.p == arriba,
                        x_km: l_km,
                        edad_myr: 0.0,
                        sin_conv_myr: 0.0,
                    },
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
    ) -> Option<(usize, usize)> {
        let num = self.malla.num;
        let p = pi as u32;
        let oceano =
            |d: &Deriva, k: usize| d.dueno[k] == p && d.corteza(k).is_some_and(|c| !c.cont);
        // Fuentes: el fondo oceánico junto al margen, con 1 si toca el tramo de fosa.
        let mut fuentes: Vec<(u32, f64, u32)> = Vec::new();
        for k in 0..num {
            if !oceano(self, k) {
                continue;
            }
            let mut junto = None;
            for &v in &self.malla.vec[k] {
                if v != NADA
                    && self.dueno[v as usize] == p
                    && self.corteza(v as usize).is_some_and(|c| c.cont)
                {
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
        let a_min = self.cfg.a_min_frac * self.malla.area_total();
        if a_o <= 0.0 || a_o < a_min || a_tot - a_o < a_min {
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
            let dest = if delante[madre.mundo[s] as usize] && !c.cont {
                &mut h_o
            } else {
                &mut h_c
            };
            dest.insertar(l, c, madre.mundo[s]);
            if let Some(e) = madre.fosas.get(&l) {
                dest.fosas.insert(l, *e);
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
                        c.sumar_hc(v_hc * f, d.c.arco, rs, rc, h_cont_min);
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
