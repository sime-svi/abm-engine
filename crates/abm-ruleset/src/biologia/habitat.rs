//! Hábitat de prueba del incremento 1 (§5 del diseño del incremento): celdas con clima constante y
//! un stock de forraje logístico por celda y especie (diseño de biología §6.1).

use bevy_ecs::prelude::*;

use super::config::BioConfig;
use super::especie::{Especie, factor_ecto};

/// Productividad primaria neta anual (g de materia seca/m²/año), modelo Miami (Lieth 1975): la
/// limita el calor o el agua, el que sea más escaso. Es la producción del terreno (adenda 1 al modo
/// Tierra §2: el motor tiene una sola producción vegetal), la misma que da la vegetación del modo Tierra.
pub use abm_terreno::vegetacion::ppn_miami;

/// Celdas del hábitat y forraje de cada especie (índice `celda · n_esp + especie`).
#[derive(Resource, Debug, Clone)]
pub struct Habitat {
    pub n_celdas: usize,
    pub n_esp: usize,
    /// Zona de cada celda (índice en `config.habitat.zonas`).
    pub zona: Vec<u16>,
    pub area_m2: Vec<f64>,
    pub temp: Vec<f64>,
    /// PPN anual (g/m²/año).
    pub ppn: Vec<f64>,
    /// Factor de temperatura del metabolismo de los ectotermos, por celda.
    pub f_ecto: Vec<f64>,
    /// Stock máximo `K_c` (g/m²), rebrote mínimo `g_0` (g/m²/día) y stock `V_c` (g/m²).
    pub k: Vec<f64>,
    pub g0: Vec<f64>,
    pub v: Vec<f64>,
    /// Tasa logística `r_c = 4/(τ_c·días)` (1/día), común.
    pub r: f64,
}

impl Habitat {
    /// Construye el hábitat de la configuración, con el stock lleno.
    pub fn nuevo(cfg: &BioConfig, especies: &[Especie]) -> Habitat {
        let n_esp = especies.len();
        let fo = &cfg.forraje;
        let en = &cfg.energetica;
        let da = cfg.tiempo.dias_anio as f64;
        let mut h = Habitat {
            n_celdas: 0,
            n_esp,
            zona: Vec::new(),
            area_m2: Vec::new(),
            temp: Vec::new(),
            ppn: Vec::new(),
            f_ecto: Vec::new(),
            k: Vec::new(),
            g0: Vec::new(),
            v: Vec::new(),
            r: 4.0 / (fo.tau_c * da),
        };
        for (zi, z) in cfg.habitat.zonas.iter().enumerate() {
            let ppn = ppn_miami(z.t_media, z.p_media_mm);
            for _ in 0..z.celdas {
                h.zona.push(zi as u16);
                h.area_m2.push(z.area_km2 * 1e6);
                h.temp.push(z.t_media);
                h.ppn.push(ppn);
                h.f_ecto.push(factor_ecto(z.t_media, en.e_a, en.t_ref));
                for e in especies {
                    let k = fo.tau_c * e.f_com * ppn;
                    h.k.push(k);
                    h.g0.push(fo.g0_frac * e.f_com * ppn / da);
                    h.v.push(k);
                }
            }
        }
        h.n_celdas = h.zona.len();
        h
    }

    /// Índice de (celda, especie).
    #[inline]
    pub fn idx(&self, celda: usize, esp: usize) -> usize {
        celda * self.n_esp + esp
    }
}
