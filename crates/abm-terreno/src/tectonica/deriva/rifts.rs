//! Adelgazamiento de los márgenes en los rifts (adenda de balance v8 §6.10).
//!
//! Un continente no se parte en seco. Cuando un rift corta corteza continental de espesor `h_0`,
//! la abertura de las primeras etapas no se rellena con fondo oceánico: se rellena estirando la
//! corteza de una franja de anchura `W_0 = W_m·(h_0 + h_rot)/(2·h_0)` a cada lado, conservando su
//! volumen. En cada paso, el volumen de la franja (la de partida más la abertura ya rellenada) se
//! reparte en una rampa lineal desde el borde interior hasta el eje: con la extensión `e` de ese
//! lado, la franja mide `w = W_0 + e` y el espesor junto al eje es `h_0·(2·W_0/w − 1)`. Cuando
//! `e` llega a `W_m − W_0`, la rampa va de `h_0` a `h_rot` a lo largo de `W_m`: la corteza se
//! rompe y lo que se abre después es fondo oceánico, como hasta ahora.
//!
//! El reparto solo mueve corteza entre las muestras de la franja, en un orden fijo: el volumen
//! continental se conserva exactamente.

use super::*;

/// Un lado de un rift en fase de estiramiento.
#[derive(Debug, Clone)]
pub(super) struct LadoRift {
    /// Placa (id) de este lado y la del otro.
    pub(super) placa: u64,
    pub(super) otra: u64,
    /// Celdas locales de la franja: la corteza continental de partida y la que rellena la abertura.
    celdas: Vec<u32>,
    /// Espesor de partida (km), anchura de la franja de partida `W_0` (km) y longitud del rift (km).
    h0_km: f64,
    w0_km: f64,
    largo_km: f64,
    /// Área de abertura rellenada en este lado (km²): su extensión es `área/largo`.
    abierta_km2: f64,
    /// Ya se rompió: lo que se abre en este lado es fondo oceánico.
    roto: bool,
}

impl Deriva {
    /// Distancia (km) de cada celda del mundo de la placa `pi` a la placa `qi`, por la superficie y
    /// por las celdas de `pi`, hasta `max_d` (∞ si no llega). La primera fila está a media celda.
    fn distancia_a_placa(&mut self, pi: u32, qi: u32, max_d: f64) -> Vec<f64> {
        let num = self.malla.num;
        let media = 0.5 * self.malla.lado_km;
        let fuentes: Vec<(u32, f64, u32)> = (0..num)
            .filter(|&k| {
                self.dueno[k] == pi
                    && self.malla.vec[k]
                        .iter()
                        .any(|&v| v != NADA && self.dueno[v as usize] == qi)
            })
            .map(|k| (k as u32, media, 0))
            .collect();
        let dueno = &self.dueno;
        let mut campo = std::mem::take(&mut self.campo);
        campo.calcular(
            &self.malla,
            &fuentes,
            |u, v| dueno[u] == pi && dueno[v] == pi,
            max_d,
        );
        let d = campo.dist.clone();
        self.campo = campo;
        d
    }

    /// Tras un rift que ha partido una placa en `h1` y `h2` (índices): apunta la franja de cada lado
    /// que se va a estirar. Un lado sin corteza continental junto al corte no se estira.
    pub(super) fn iniciar_estiramiento(&mut self, h1: usize, h2: usize) {
        let (w_m, h_rot) = (self.cfg.w_margen_km, self.cfg.h_rot_km);
        if w_m <= 0.0 {
            return;
        }
        let mut lados: Vec<LadoRift> = Vec::new();
        for (p, q) in [(h1, h2), (h2, h1)] {
            let d = self.distancia_a_placa(p as u32, q as u32, w_m + self.malla.lado_km);
            // Espesor de partida: el medio de la corteza continental a menos de W_m del corte.
            let (mut v, mut a) = (0.0, 0.0);
            for (k, &dk) in d.iter().enumerate() {
                if dk < w_m
                    && let Some(c) = self.corteza(k)
                    && c.cont
                {
                    v += c.hc * self.malla.area[self.rep[k] as usize];
                    a += self.malla.area[self.rep[k] as usize];
                }
            }
            if a <= 0.0 {
                continue;
            }
            let h0 = v / a;
            if h0 <= h_rot {
                continue;
            }
            let w0 = w_m * (h0 + h_rot) / (2.0 * h0);
            let mut celdas = Vec::new();
            let mut a_franja = 0.0;
            for (k, &dk) in d.iter().enumerate() {
                if dk < w0 && self.corteza(k).is_some_and(|c| c.cont) {
                    celdas.push(self.rep[k]);
                    a_franja += self.malla.area[self.rep[k] as usize];
                }
            }
            if celdas.is_empty() {
                continue;
            }
            celdas.sort_unstable();
            celdas.dedup();
            lados.push(LadoRift {
                placa: self.placas[p].id,
                otra: self.placas[q].id,
                celdas,
                h0_km: h0,
                w0_km: w0,
                largo_km: a_franja / w0,
                abierta_km2: 0.0,
                roto: false,
            });
        }
        self.rifts_activos.extend(lados);
    }

    /// Fase de rift de cada lado aún sin romper: la corteza nueva de este paso que rellena su
    /// abertura pasa a corteza continental de la franja, y el volumen de la franja se reparte en su
    /// rampa. Se llama tras rehacer los mapas, antes de la isostasia del paso.
    pub(super) fn estirar_rifts(&mut self) {
        if self.rifts_activos.is_empty() {
            return;
        }
        let h_rot = self.cfg.h_rot_km;
        let w_m = self.cfg.w_margen_km;
        let mut lados = std::mem::take(&mut self.rifts_activos);
        for lado in lados.iter_mut() {
            let indice = |d: &Deriva, id: u64| d.placas.iter().position(|p| p.viva && p.id == id);
            let (Some(pi), Some(qi)) = (indice(self, lado.placa), indice(self, lado.otra)) else {
                lado.roto = true;
                continue;
            };
            // 1. La abertura de este paso: corteza nueva de la placa junto a la franja.
            let mut miembro: std::collections::HashSet<u32> = lado.celdas.iter().copied().collect();
            let mut nuevas: Vec<u32> = self
                .creacion_paso
                .iter()
                .copied()
                .filter(|&k| self.dueno[k as usize] == pi as u32)
                .filter(|&k| {
                    self.corteza(k as usize)
                        .is_some_and(|c| !c.cont && c.hc <= 0.0 && c.proc.nace == nace::DORSAL)
                })
                .collect();
            nuevas.sort_unstable();
            nuevas.dedup();
            let mut crecio = true;
            while crecio {
                crecio = false;
                for &k in &nuevas {
                    let l = self.rep[k as usize];
                    if miembro.contains(&l) {
                        continue;
                    }
                    let junto = self.malla.vec[k as usize].iter().any(|&v| {
                        v != NADA
                            && self.dueno[v as usize] == pi as u32
                            && miembro.contains(&self.rep[v as usize])
                    });
                    if junto {
                        miembro.insert(l);
                        lado.celdas.push(l);
                        lado.abierta_km2 += self.malla.area[l as usize];
                        if let Some(c) = self.placas[pi].get_mut(l) {
                            *c = Corteza {
                                cont: true,
                                h: 0.0,
                                hc: 0.0,
                                arco: 0.0,
                                edad_orog: 1000.0,
                                ..*c
                            };
                            c.proc.nace = nace::RIFT;
                            c.proc.vias = [0.0; via::N];
                        }
                        crecio = true;
                    }
                }
            }
            // 2. Rampa: espesor relativo por la distancia al eje.
            let e = lado.abierta_km2 / lado.largo_km.max(1e-9);
            let w = lado.w0_km + e;
            let r = (2.0 * lado.w0_km / w - 1.0).max(h_rot / lado.h0_km);
            let d = self.distancia_a_placa(pi as u32, qi as u32, w + 2.0 * self.malla.lado_km);
            let placa = &self.placas[pi];
            let mut filas: Vec<(u32, f64, f64, f64)> = Vec::new(); // (celda local, área, hc, g)
            for &l in &lado.celdas {
                let s = placa.mapa[l as usize];
                if s == NADA {
                    continue;
                }
                let c = placa.cort[s as usize];
                let k = placa.mundo[s as usize] as usize;
                if !c.cont || self.dueno[k] != pi as u32 || self.rep[k] != l || !d[k].is_finite() {
                    continue;
                }
                let g = r + (1.0 - r) * (d[k] / w).min(1.0);
                filas.push((l, self.malla.area[l as usize], c.hc, g));
            }
            let v: f64 = filas.iter().map(|x| x.2 * x.1).sum();
            let sg: f64 = filas.iter().map(|x| x.3 * x.1).sum();
            // El estiramiento mezcla la corteza de la franja: toda queda con su fracción de arco media
            // (adenda v9; se conserva el volumen de arco).
            let v_arco: f64 = {
                let placa = &self.placas[pi];
                filas
                    .iter()
                    .map(|x| placa.get(x.0).map_or(0.0, |c| c.arco) * x.2 * x.1)
                    .sum()
            };
            if v > 0.0 && sg > 0.0 {
                for &(l, _, antes, g) in &filas {
                    let nuevo = v * g / sg;
                    if let Some(c) = self.placas[pi].get_mut(l) {
                        let base = c.h - c.hc;
                        c.hc = nuevo;
                        c.h = nuevo + base;
                        c.arco = (v_arco / v).clamp(0.0, 1.0);
                        if antes > 0.0 {
                            c.proc = c.proc.escalada(nuevo / antes);
                        } else {
                            c.proc.vias[via::RIFT] = nuevo as f32;
                        }
                    }
                }
            }
            // 3. Rotura: la rampa ya mide W_m.
            if e >= w_m - lado.w0_km {
                lado.roto = true;
                // Nace el océano en este lado (nota de rifts §7).
                let ids = vec![lado.placa, lado.otra];
                self.evento_det(
                    "rotura_rift",
                    ids,
                    lado.largo_km,
                    lado.abierta_km2,
                    None,
                    Detalle {
                        extension_km: Some(e),
                        ..Detalle::default()
                    },
                );
            }
        }
        lados.retain(|l| !l.roto);
        self.rifts_activos = lados;
    }
}

/// Un bloque continental (nota de rifts §3): región conexa de corteza de un continente (no de arco)
/// dentro de una placa, por las celdas del mundo.
pub(super) struct Bloque {
    /// Placa (índice).
    placa: usize,
    /// Celdas del mundo, ordenadas.
    celdas: Vec<u32>,
    area_km2: f64,
    /// Centro, ponderado por área.
    centro: V3,
}

impl Bloque {
    /// Placa del bloque (índice).
    pub(super) fn placa_indice(&self) -> usize {
        self.placa
    }

    /// Celdas del mundo del bloque, ordenadas.
    pub(super) fn celdas_del_mundo(&self) -> &[u32] {
        &self.celdas
    }
}

/// Un trazado de ruptura (nota de rifts §6.2).
enum Trazado {
    /// Por la sutura `id`: celdas del mundo de su lado 0 y de su lado 1 dentro del bloque, y su
    /// longitud (km).
    Sutura(u32, Vec<u32>, Vec<u32>, f64),
    /// Círculo máximo por el centro del bloque, con la orientación del sorteo `i`.
    Circulo(u64),
}

impl Deriva {
    /// Celdas del mundo con corteza de un continente (`continente()`).
    pub(super) fn celdas_continente(&self) -> Vec<bool> {
        (0..self.malla.num)
            .map(|k| self.corteza(k).is_some_and(|c| c.continente()))
            .collect()
    }

    /// Bloques continentales de todas las placas, en orden de su primera celda del mundo.
    pub(super) fn bloques_continentales(&self) -> Vec<Bloque> {
        self.bloques_de(&self.celdas_continente())
    }

    /// Los bloques de las celdas `cont` (de [`Deriva::celdas_continente`]).
    pub(super) fn bloques_de(&self, cont: &[bool]) -> Vec<Bloque> {
        let num = self.malla.num;
        let es = |k: usize| cont[k];
        let mut visto = vec![false; num];
        let mut bloques = Vec::new();
        let mut pila: Vec<u32> = Vec::new();
        for k0 in 0..num {
            if visto[k0] || !es(k0) {
                continue;
            }
            let p = self.dueno[k0];
            visto[k0] = true;
            pila.push(k0 as u32);
            let mut celdas = Vec::new();
            while let Some(k) = pila.pop() {
                celdas.push(k);
                for &v in &self.malla.vec[k as usize] {
                    let vu = v as usize;
                    if v != NADA && !visto[vu] && self.dueno[vu] == p && es(vu) {
                        visto[vu] = true;
                        pila.push(v);
                    }
                }
            }
            celdas.sort_unstable();
            let (mut a, mut c) = (0.0, [0.0; 3]);
            for &k in &celdas {
                let ku = k as usize;
                a += self.malla.area[ku];
                c = add(c, esc(self.malla.dir[ku], self.malla.area[ku]));
            }
            bloques.push(Bloque {
                placa: p as usize,
                celdas,
                area_km2: a,
                centro: normalize(c),
            });
        }
        bloques
    }

    /// Bloques de al menos `2·A_trozo,min` (los que pueden romperse) y fracción del continente total
    /// en el mayor bloque (métricas de la nota de rifts §8).
    pub(super) fn metricas_de_bloques(&self, cont: &[bool]) -> (u32, f64) {
        let bloques = self.bloques_de(cont);
        let total: f64 = bloques.iter().map(|b| b.area_km2).sum();
        let a_min = 2.0 * self.cfg.a_trozo_min_mkm2 * 1e6;
        let n = bloques.iter().filter(|b| b.area_km2 >= a_min).count() as u32;
        let mayor = bloques.iter().map(|b| b.area_km2).fold(0.0, f64::max);
        (n, if total > 0.0 { mayor / total } else { 0.0 })
    }

    /// La sutura más larga del bloque con sus dos lados dentro de él, si mide al menos `L_sut,min`:
    /// su longitud es la del lado más corto (celdas por lado de celda).
    fn sutura_mas_larga(&self, b: &Bloque) -> Option<Trazado> {
        let mut por_id: std::collections::BTreeMap<u32, (Vec<u32>, Vec<u32>)> = Default::default();
        for &k in &b.celdas {
            if let Some(c) = self.corteza(k as usize)
                && c.sutura != 0
            {
                let e = por_id.entry(c.sutura >> 1).or_default();
                if c.sutura & 1 == 0 {
                    e.0.push(k);
                } else {
                    e.1.push(k);
                }
            }
        }
        por_id
            .into_iter()
            .map(|(id, (s0, s1))| {
                let largo = s0.len().min(s1.len()) as f64 * self.malla.lado_km;
                (largo, id, s0, s1)
            })
            .filter(|x| x.0 >= self.cfg.l_sut_min_km)
            .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
            .map(|(largo, id, s0, s1)| Trazado::Sutura(id, s0, s1, largo))
    }

    /// Lado de cada muestra de la placa `pi` al cortarla por una sutura (nota de rifts §6.2): el de la
    /// fuente más cercana entre las celdas de sus dos lados (`true`, el lado 1), por las celdas de la
    /// placa. Más allá de los extremos de la sutura, la frontera entre las dos regiones sigue recta
    /// hasta la costa del bloque y luego por el océano de la placa. Lo que no se alcanza así (muestras
    /// ocultas bajo otra placa) toma el lado más cercano por toda la esfera.
    fn lados_por_sutura(&mut self, pi: usize, s0: &[u32], s1: &[u32]) -> Vec<bool> {
        let num = self.malla.num;
        let p = pi as u32;
        let fuentes: Vec<(u32, f64, u32)> = s0
            .iter()
            .map(|&k| (k, 0.0, 0))
            .chain(s1.iter().map(|&k| (k, 0.0, 1)))
            .collect();
        let dueno = &self.dueno;
        let mut campo = std::mem::take(&mut self.campo);
        campo.calcular(
            &self.malla,
            &fuentes,
            |u, v| dueno[u] == p && dueno[v] == p,
            f64::INFINITY,
        );
        let mut lado: Vec<u32> = (0..num)
            .map(|k| if dueno[k] == p { campo.fuente[k] } else { NADA })
            .collect();
        let madre = &self.placas[pi];
        if madre.mundo.iter().any(|&k| lado[k as usize] == NADA) {
            let fuentes: Vec<(u32, f64, u32)> = (0..num)
                .filter(|&k| lado[k] != NADA)
                .map(|k| (k as u32, 0.0, lado[k]))
                .collect();
            campo.calcular(&self.malla, &fuentes, |_, _| true, f64::INFINITY);
            for (k, l) in lado.iter_mut().enumerate() {
                if *l == NADA {
                    *l = campo.fuente[k];
                }
            }
        }
        self.campo = campo;
        madre.mundo.iter().map(|&k| lado[k as usize] == 1).collect()
    }

    /// Reloj de un bloque (nota de rifts v2 §6.1, Myr): el tiempo desde el reinicio más reciente de
    /// sus muestras (una ruptura o un choque que le añadió un continente). Sin ningún reinicio, ∞.
    pub(super) fn reloj_bloque(&self, b: &Bloque) -> f64 {
        let ultimo = b
            .celdas
            .iter()
            .filter_map(|&k| self.corteza(k as usize))
            .map(|c| c.reinicio_myr)
            .fold(f64::NEG_INFINITY, f64::max);
        if ultimo.is_finite() {
            (self.t_myr - ultimo).max(0.0)
        } else {
            f64::INFINITY
        }
    }

    /// Reinicia el reloj de las muestras que ven las celdas del mundo `celdas` (nota de rifts v2
    /// §6.1): un trozo recién separado y un bloque recién soldado a otro empiezan de cero.
    pub(super) fn reiniciar_reloj(&mut self, celdas: &[u32]) {
        let t = self.t_myr;
        for &k in celdas {
            let (p, l) = (self.dueno[k as usize], self.rep[k as usize]);
            if p == NADA || l == NADA {
                continue;
            }
            if let Some(c) = self.placas[p as usize].get_mut(l) {
                c.reinicio_myr = t;
            }
        }
    }

    /// Rifts (nota de rifts v2 §6): cada bloque continental que no esté ya en fase de rift se rompe
    /// en el paso con probabilidad `1 − exp(−λ_A·A_k·min(1, t_k/τ_ais)·Δt)`, con `t_k` el reloj del
    /// bloque ([`Deriva::reloj_bloque`]), una tirada por bloque en orden fijo. El
    /// trazado sigue la sutura más larga del bloque (de al menos `L_sut,min`) o, si no la tiene, un
    /// círculo máximo por su centro con orientación sorteada. Si algún trozo queda con menos de
    /// `A_trozo,min` de continente (o una mitad de la placa por debajo de `A_min`, que la absorbería
    /// su vecina), se prueba el trazado alternativo, un círculo con otra orientación; si tampoco
    /// vale, no hay ruptura en este paso. Sustituye a `τ_rift`.
    pub(super) fn rifts(&mut self) -> bool {
        let bloques = self.bloques_continentales();
        // Celdas de las franjas que aún se estiran: su bloque ya está en fase de rift.
        let en_rift: std::collections::HashSet<(u64, u32)> = self
            .rifts_activos
            .iter()
            .flat_map(|l| l.celdas.iter().map(move |&c| (l.placa, c)))
            .collect();
        let dt = self.cfg.dt_myr;
        let lambda = self.cfg.lambda_a_rift;
        let a_trozo = self.cfg.a_trozo_min_mkm2 * 1e6;
        let mut cambio = false;
        for b in &bloques {
            let pi = b.placa;
            // Un bloque pequeño no se rompe nunca; uno de una placa ya partida en este paso, al
            // siguiente.
            if !self.placas[pi].viva || b.area_km2 < 2.0 * a_trozo {
                continue;
            }
            let id = self.placas[pi].id;
            if b.celdas
                .iter()
                .any(|&k| en_rift.contains(&(id, self.rep[k as usize])))
            {
                continue;
            }
            // El calor se acumula bajo el bloque con el tiempo que lleva unido (nota de rifts v2).
            let calor = if self.cfg.tau_ais_myr > 0.0 {
                (self.reloj_bloque(b) / self.cfg.tau_ais_myr).min(1.0)
            } else {
                1.0
            };
            let prob = 1.0 - (-lambda * b.area_km2 / 1e6 * calor * dt).exp();
            let k0 = b.celdas[0] as u64;
            if azar(self.seed, self.paso, id, P_RIFT, k0) >= prob {
                continue;
            }
            let mut intentos = Vec::with_capacity(2);
            match self.sutura_mas_larga(b) {
                Some(s) => {
                    intentos.push(s);
                    intentos.push(Trazado::Circulo(0));
                }
                None => {
                    intentos.push(Trazado::Circulo(0));
                    intentos.push(Trazado::Circulo(1));
                }
            }
            for t in intentos {
                let (lados, polo, centro, sutura) = match t {
                    Trazado::Sutura(sid, s0, s1, largo) => {
                        let media = |s: &[u32]| {
                            s.iter()
                                .fold([0.0; 3], |m, &k| add(m, self.malla.dir[k as usize]))
                        };
                        let (m0, m1) = (media(&s0), media(&s1));
                        let centro = normalize(add(m0, m1));
                        let dif = sub(
                            esc(m1, 1.0 / s1.len() as f64),
                            esc(m0, 1.0 / s0.len() as f64),
                        );
                        let polo = sub(dif, esc(centro, dot(dif, centro)));
                        if dot(polo, polo) < 1e-18 {
                            continue;
                        }
                        let lados = self.lados_por_sutura(pi, &s0, &s1);
                        (lados, normalize(polo), centro, Some((sid, largo)))
                    }
                    Trazado::Circulo(i) => {
                        let c = b.centro;
                        let (e, nn) = east_north(c);
                        let th = std::f64::consts::PI
                            * azar(self.seed, self.paso, id, P_ORIENT, 2 * k0 + i);
                        let polo = normalize(add(esc(e, th.cos()), esc(nn, th.sin())));
                        (self.lados_por_circulo(pi, polo), polo, c, None)
                    }
                };
                // Continente de cada trozo (por las muestras que ve el mundo en el bloque).
                let mut trozos = [0.0f64; 2];
                {
                    let madre = &self.placas[pi];
                    for &k in &b.celdas {
                        let s = madre.mapa[self.rep[k as usize] as usize];
                        if s != NADA {
                            trozos[usize::from(!lados[s as usize])] += self.malla.area[k as usize];
                        }
                    }
                }
                if trozos[0] < a_trozo || trozos[1] < a_trozo {
                    continue;
                }
                let Some((h1, h2)) = self.partir_por_lados(pi, &lados, polo, centro) else {
                    continue;
                };
                // Los dos trozos empiezan su reloj (nota de rifts v2 §6.1).
                self.reiniciar_reloj(&b.celdas);
                self.estampar(h1, lote::RIFT);
                self.estampar(h2, lote::RIFT);
                // Una sutura que se reabre deja de serlo.
                if let Some((sid, _)) = sutura {
                    for h in [h1, h2] {
                        for c in &mut self.placas[h].cort {
                            if c.sutura >> 1 == sid {
                                c.sutura = 0;
                            }
                        }
                    }
                }
                // Los bordes se estiran antes de que nazca el océano (adenda v8 §6.10).
                self.iniciar_estiramiento(h1, h2);
                let ids = vec![id, self.placas[h1].id, self.placas[h2].id];
                let largo = self.largo_rift(h1, h2);
                self.evento_det(
                    "rift",
                    ids,
                    largo,
                    b.area_km2,
                    Some(centro),
                    Detalle {
                        clase: Some(
                            if sutura.is_some() {
                                "sutura"
                            } else {
                                "circulo"
                            }
                            .to_string(),
                        ),
                        trozos_km2: Some(trozos),
                        sutura_km: sutura.map(|x| x.1),
                        ..Detalle::default()
                    },
                );
                cambio = true;
                break;
            }
        }
        cambio
    }

    /// Marca la sutura entre las placas `a` (la que absorbe) y `b` antes de fundirlas (nota de rifts
    /// §3): las muestras de los dos lados del frente de choque, cada una con su lado.
    pub(super) fn marcar_sutura(&mut self, a: usize, b: usize) {
        self.suturas_hechas.push(self.t_myr);
        let id = self.suturas_hechas.len() as u32;
        let mut marcas: Vec<(u32, u32, u32)> = Vec::new();
        for bo in &self.bordes {
            let (p, q) = (bo.p as usize, bo.q as usize);
            if bo.tipo != TipoBorde::Colision || !((p == a && q == b) || (p == b && q == a)) {
                continue;
            }
            let l = self.rep[bo.k as usize];
            if l == NADA || self.dueno[bo.k as usize] != bo.p {
                continue;
            }
            marcas.push((bo.p, l, 2 * id + u32::from(p == b)));
        }
        for (p, l, m) in marcas {
            if let Some(c) = self.placas[p as usize].get_mut(l) {
                c.sutura = m;
            }
        }
    }
}
