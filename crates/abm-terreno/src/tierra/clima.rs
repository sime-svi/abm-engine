//! Clima del modo Tierra (`modo-tierra-diseno.md` §2.4): el año real de cada año simulado, el mapa de
//! cada mes y el clima de cada día.
//!
//! - **Año real:** el año simulado `y` es del bloque `y / L_bloque`; cada bloque sortea con el RNG por
//!   contador su año real de inicio, uniforme entre los que caben, y recorre `L_bloque` años seguidos.
//!   Se usa siempre el mapa completo de ese año real (las sequías conservan su reparto en el espacio).
//! - **Mapa de un mes:** en el nivel de carga, la climatología de CHELSA más la anomalía de
//!   temperatura de CRU TS (`ΔT`) y por el cociente de lluvia (`r`, con tope `r_max`), respecto a la
//!   climatología de CRU TS del periodo base; sube a los niveles gruesos con las medias por área.
//! - **Día:** temperatura interpolada entre las mitades de los dos meses que lo rodean; precipitación,
//!   la de su mes repartida por igual entre sus días. Las dos son lineales en el mapa del mes, así que
//!   la media de las hijas es la de la madre también cada día.

use std::sync::Arc;

use super::{DIAS_MES, DatosTierra, Nivel};
use crate::config::TerrenoConfig;
use crate::error::TerrenoError;
use crate::statics::hash01;
use crate::world::World;

/// Flujo del RNG por contador del sorteo de los años reales.
const FLUJO_ANIOS: u64 = 0x7A_E5_A1;

/// Primer día (0–364) de cada mes.
const INICIO_MES: [u32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];

/// Día (con decimales) de la mitad del mes `m`.
#[inline]
fn mitad(m: usize) -> f64 {
    INICIO_MES[m] as f64 + DIAS_MES[m] as f64 / 2.0
}

/// Mes del día `d` (0–364).
#[inline]
pub fn mes_del_dia(d: u32) -> usize {
    INICIO_MES.iter().rposition(|&i| d >= i).unwrap_or(0)
}

/// Año real de cada año simulado `y` en `[-1, n_sim]` (posición `y + 1`): bloques de `L_bloque` años
/// seguidos con inicio sorteado; antes del año 0, el año real anterior al del año 0 (o el mismo si es
/// el primero de la serie). Con `anio_forzado`, siempre ese año.
pub fn secuencia_anios(cfg: &TerrenoConfig, anio0: i32, anio_final: i32, n_sim: usize) -> Vec<i32> {
    let t = &cfg.tierra;
    if t.anio_forzado != 0 {
        return vec![t.anio_forzado; n_sim + 2];
    }
    let n_anios = (anio_final - anio0 + 1).max(1) as u64;
    let l = (t.l_bloque as u64).clamp(1, n_anios);
    let inicios = n_anios - l + 1;
    let mut v = Vec::with_capacity(n_sim + 2);
    for y in 0..=n_sim as u64 {
        let b = y / l;
        let u = hash01(cfg.run.seed, b, FLUJO_ANIOS);
        let inicio = anio0 + ((u * inicios as f64) as u64).min(inicios - 1) as i32;
        v.push(inicio + (y % l) as i32);
    }
    let previo = if v[0] > anio0 { v[0] - 1 } else { v[0] };
    v.insert(0, previo);
    v
}

/// Mapa de un mes de un año real: temperatura (°C) y precipitación (mm/mes) de cada celda, por nivel.
struct MapaMes {
    anio: i32,
    mes: usize,
    t: Vec<Vec<f32>>,
    p: Vec<Vec<f32>>,
}

/// Estado del clima de la Tierra durante una corrida.
pub struct Estado {
    datos: Arc<DatosTierra>,
    /// Año real de los años simulados −1..=n (posición `y + 1`).
    anios: Vec<i32>,
    /// Climatología de CRU TS del periodo base, por mes y celda de CRU (`[mes·n + celda]`), y la
    /// corrección de la climatología de CHELSA (1981–2010) a ese periodo.
    t_base: Vec<f64>,
    p_base: Vec<f64>,
    corr_t: Vec<f64>,
    corr_p: Vec<f64>,
    r_max: f64,
    mapas: Vec<MapaMes>,
}

/// Media de CRU TS en un periodo, por mes y celda.
fn climatologia_cru(d: &DatosTierra, desde: i32, hasta: i32) -> (Vec<f64>, Vec<f64>) {
    let nc = d.cru.n_celdas();
    let (mut t, mut p) = (vec![0.0; 12 * nc], vec![0.0; 12 * nc]);
    let n = (hasta - desde + 1) as f64;
    for a in desde..=hasta {
        for m in 0..12 {
            for c in 0..nc {
                t[m * nc + c] += d.cru.t(a, m, c);
                p[m * nc + c] += d.cru.p(a, m, c);
            }
        }
    }
    t.iter_mut().chain(p.iter_mut()).for_each(|x| *x /= n);
    (t, p)
}

impl Estado {
    /// Prepara el clima de una corrida de `n_sim` años simulados.
    pub fn new(
        datos: Arc<DatosTierra>,
        cfg: &TerrenoConfig,
        n_sim: usize,
    ) -> Result<Estado, TerrenoError> {
        let (a0, a1) = (datos.cru.anio0, datos.cru.anio_final());
        let [b0, b1] = cfg.tierra.periodo_base;
        if b0 < a0 || b1 > a1 {
            return Err(TerrenoError::Tierra(format!(
                "periodo_base {b0}–{b1} fuera de CRU TS ({a0}–{a1})"
            )));
        }
        let f = cfg.tierra.anio_forzado;
        if f != 0 && !(a0..=a1).contains(&f) {
            return Err(TerrenoError::Tierra(format!(
                "anio_forzado {f} fuera de CRU TS ({a0}–{a1})"
            )));
        }
        let r_max = cfg.tierra.r_max;
        let (t_base, p_base) = climatologia_cru(&datos, b0, b1);
        let nc = datos.cru.n_celdas();
        let (corr_t, corr_p) = if [b0, b1] == [1981, 2010] {
            (vec![0.0; 12 * nc], vec![1.0; 12 * nc])
        } else if a0 > 1981 || a1 < 2010 {
            return Err(TerrenoError::Tierra(format!(
                "periodo_base {b0}–{b1}: corregir CHELSA (1981–2010) necesita CRU TS de 1981–2010 ({a0}–{a1})"
            )));
        } else {
            let (t81, p81) = climatologia_cru(&datos, 1981, 2010);
            (
                t_base.iter().zip(&t81).map(|(b, r)| b - r).collect(),
                p_base
                    .iter()
                    .zip(&p81)
                    .map(|(b, r)| if *r > 0.0 { (b / r).min(r_max) } else { 1.0 })
                    .collect(),
            )
        };
        Ok(Estado {
            anios: secuencia_anios(cfg, a0, a1, n_sim),
            datos,
            t_base,
            p_base,
            corr_t,
            corr_p,
            r_max,
            mapas: Vec::with_capacity(3),
        })
    }

    /// Año real del año simulado `y` (desde −1).
    pub fn anio_real(&self, y: i64) -> i32 {
        let k = (y + 1).clamp(0, self.anios.len() as i64 - 1) as usize;
        self.anios[k]
    }

    /// Los datos de la Tierra.
    pub fn datos(&self) -> &Arc<DatosTierra> {
        &self.datos
    }

    /// Temperatura (°C) y precipitación (mm/mes) de la celda `k` del nivel de carga en el mes `m` del
    /// año real `a`.
    #[inline]
    pub fn mes_en_celda(&self, a: i32, m: usize, k: usize) -> (f64, f64) {
        let d = &self.datos;
        let fino = &d.niveles[d.nivel_carga as usize];
        let (mut t, mut p) = (fino.t[k * 12 + m] as f64, fino.p[k * 12 + m] as f64);
        let c = d.cru_de_celda[k];
        if c != u32::MAX {
            let (c, nc) = (c as usize, d.cru.n_celdas());
            let q = m * nc + c;
            let r = if self.p_base[q] > 0.0 {
                (d.cru.p(a, m, c) / self.p_base[q]).clamp(0.0, self.r_max)
            } else {
                1.0
            };
            t += self.corr_t[q] + (d.cru.t(a, m, c) - self.t_base[q]);
            p *= self.corr_p[q] * r;
        }
        (t, p)
    }

    /// Construye el mapa del mes `m` del año real `a` si no está ya (y retira los que no hacen falta).
    fn asegurar_mapa(&mut self, a: i32, m: usize, necesarios: &[(i32, usize)]) {
        if self.mapas.iter().any(|x| x.anio == a && x.mes == m) {
            return;
        }
        self.mapas.retain(|x| necesarios.contains(&(x.anio, x.mes)));
        let d = Arc::clone(&self.datos);
        let l = d.nivel_carga as usize;
        let fino = &d.niveles[l];
        let (mut t_l, mut p_l) = (vec![0.0f32; fino.len()], vec![0.0f32; fino.len()]);
        for k in 0..fino.len() {
            let (t, p) = self.mes_en_celda(a, m, k);
            t_l[k] = t as f32;
            p_l[k] = p as f32;
        }
        let mut t = vec![t_l];
        let mut p = vec![p_l];
        for lv in (0..l).rev() {
            let (madre, hija) = (&d.niveles[lv], &d.niveles[lv + 1]);
            t.insert(0, subir(&t[0], hija, madre));
            p.insert(0, subir(&p[0], hija, madre));
        }
        self.mapas.push(MapaMes {
            anio: a,
            mes: m,
            t,
            p,
        });
    }

    /// Clima del tick `t` (un día): escribe la temperatura y la precipitación de cada celda.
    pub fn clima(&mut self, world: &mut World, t: u64) {
        let y = (t / 365) as i64;
        let d = (t % 365) as u32;
        let mes = mes_del_dia(d);
        let x = d as f64 + 0.5;
        // Meses de las dos mitades que rodean el día (con su año simulado) y sus posiciones.
        let (y1, m1, x1, y2, m2, x2) = if x >= mitad(mes) {
            if mes == 11 {
                (y, 11, mitad(11), y + 1, 0, 365.0 + mitad(0))
            } else {
                (y, mes, mitad(mes), y, mes + 1, mitad(mes + 1))
            }
        } else if mes == 0 {
            (y - 1, 11, mitad(11) - 365.0, y, 0, mitad(0))
        } else {
            (y, mes - 1, mitad(mes - 1), y, mes, mitad(mes))
        };
        let w = (x - x1) / (x2 - x1);
        let (a1, a2, a) = (self.anio_real(y1), self.anio_real(y2), self.anio_real(y));
        let necesarios = [(a1, m1), (a2, m2), (a, mes)];
        for &(aa, mm) in &necesarios {
            self.asegurar_mapa(aa, mm, &necesarios);
        }
        let pos = |a: i32, m: usize| {
            self.mapas
                .iter()
                .position(|x| x.anio == a && x.mes == m)
                .unwrap_or(0)
        };
        let (q1, q2, qp) = (pos(a1, m1), pos(a2, m2), pos(a, mes));
        let dias = DIAS_MES[mes] as f64;
        let (mt1, mt2, mp) = (&self.mapas[q1], &self.mapas[q2], &self.mapas[qp]);
        for s in 0..world.topo.num_cells {
            let (lv, k) = self.datos.celda_de_clave(world.topo.keys[s]);
            let t1 = mt1.t[lv][k] as f64;
            let t2 = mt2.t[lv][k] as f64;
            world.temp[s] = (1.0 - w) * t1 + w * t2;
            world.precip[s] = (mp.p[lv][k] as f64 / dias).max(0.0);
        }
    }

    /// Lluvia anual (mm) del año real `a`, media por área de las celdas `celdas` del nivel de carga.
    pub fn lluvia_anual(&self, a: i32, celdas: &[usize]) -> f64 {
        let d = &self.datos;
        let fino = &d.niveles[d.nivel_carga as usize];
        let (mut s, mut ar) = (0.0, 0.0);
        for &k in celdas {
            let mut p = 0.0;
            for m in 0..12 {
                p += self.mes_en_celda(a, m, k).1;
            }
            s += p * fino.area[k];
            ar += fino.area[k];
        }
        if ar > 0.0 { s / ar } else { 0.0 }
    }
}

/// Un nivel más grueso de un campo mensual: media por área de las cuatro hijas.
fn subir(hija_v: &[f32], hija: &Nivel, madre: &Nivel) -> Vec<f32> {
    let n = madre.n;
    let mut out = vec![0.0f32; madre.len()];
    for c in 0..6u8 {
        for j in 0..n {
            for i in 0..n {
                let (mut a, mut v) = (0.0f64, 0.0f64);
                for (di, dj) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let k = hija.indice(c, 2 * i + di, 2 * j + dj);
                    a += hija.area[k];
                    v += hija_v[k] as f64 * hija.area[k];
                }
                out[madre.indice(c, i, j)] = (v / a) as f32;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn los_meses_y_sus_mitades_cuadran() {
        assert_eq!(mes_del_dia(0), 0);
        assert_eq!(mes_del_dia(30), 0);
        assert_eq!(mes_del_dia(31), 1);
        assert_eq!(mes_del_dia(58), 1);
        assert_eq!(mes_del_dia(59), 2);
        assert_eq!(mes_del_dia(364), 11);
        assert_eq!(DIAS_MES.iter().sum::<u32>(), 365);
        assert!((mitad(0) - 15.5).abs() < 1e-12);
        assert!((mitad(11) - 349.5).abs() < 1e-12);
    }

    #[test]
    fn la_secuencia_va_por_bloques_y_es_reproducible() {
        let mut cfg = TerrenoConfig::default();
        cfg.tierra.l_bloque = 10;
        cfg.run.seed = 42;
        let a = secuencia_anios(&cfg, 1901, 2024, 95);
        let b = secuencia_anios(&cfg, 1901, 2024, 95);
        assert_eq!(a, b, "misma semilla, misma secuencia de años reales");
        assert_eq!(a.len(), 97);
        // Dentro de cada bloque, años seguidos; todos dentro de la serie.
        for y in 0..=95usize {
            let v = a[y + 1];
            assert!((1901..=2024).contains(&v));
            if y % 10 != 0 {
                assert_eq!(v, a[y] + 1, "año {y}");
            }
        }
        // El año −1 es el anterior al del año 0.
        assert!(a[0] == a[1] - 1 || a[1] == 1901);
        // Otra semilla, otra secuencia; y los bloques no empiezan todos igual.
        cfg.run.seed = 43;
        assert_ne!(secuencia_anios(&cfg, 1901, 2024, 95), a);
        let inicios: std::collections::BTreeSet<i32> = (0..10).map(|b| a[b * 10 + 1]).collect();
        assert!(inicios.len() > 5, "{inicios:?}");
        // Con el año forzado, siempre ese.
        cfg.tierra.anio_forzado = 1984;
        assert!(
            secuencia_anios(&cfg, 1901, 2024, 5)
                .iter()
                .all(|&v| v == 1984)
        );
    }

    #[test]
    fn los_inicios_de_bloque_son_uniformes() {
        // 20 000 bloques: cada año de inicio posible (1901–2015) sale ~1/115 de las veces.
        let mut cfg = TerrenoConfig::default();
        cfg.tierra.l_bloque = 10;
        let a = secuencia_anios(&cfg, 1901, 2024, 200_000);
        let mut cuenta = std::collections::BTreeMap::new();
        for b in 0..20_000usize {
            *cuenta.entry(a[b * 10 + 1]).or_insert(0u32) += 1;
        }
        assert_eq!(*cuenta.keys().next().unwrap_or(&0), 1901);
        assert_eq!(*cuenta.keys().last().unwrap_or(&0), 2015);
        let esperado = 20_000.0 / 115.0;
        for (&anio, &c) in &cuenta {
            assert!(
                (c as f64 - esperado).abs() < 5.0 * esperado.sqrt(),
                "{anio}: {c} frente a {esperado:.0}"
            );
        }
    }
}
