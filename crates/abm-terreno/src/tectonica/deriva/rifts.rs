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
    placa: u64,
    otra: u64,
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
                                arco: false,
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
            if v > 0.0 && sg > 0.0 {
                for &(l, _, antes, g) in &filas {
                    let nuevo = v * g / sg;
                    if let Some(c) = self.placas[pi].get_mut(l) {
                        let base = c.h - c.hc;
                        c.hc = nuevo;
                        c.h = nuevo + base;
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
            }
        }
        lados.retain(|l| !l.roto);
        self.rifts_activos = lados;
    }
}
