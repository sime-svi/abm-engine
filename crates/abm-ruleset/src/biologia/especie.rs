//! Especie derivada por su masa (leyes de escala, §5.1 y §6.0) y sus funciones vitales: metabolismo
//! de Kleiber, reservas, curva de crecimiento de West, mortalidad de Siler y leche.

use abm_core::ConfigError;

use super::azar::{Proposito, uniforme};
use super::config::{AjustesCfg, BioConfig, EspecieCfg, Termico};

/// Constante de Boltzmann (eV/K), para el metabolismo de los ectotermos.
pub const K_BOLTZMANN_EV: f64 = 8.617e-5;

/// Parámetros de una especie, ya derivados (se calculan una vez al cargar).
#[derive(Debug, Clone)]
pub struct Especie {
    pub nombre: String,
    pub termico: Termico,
    /// Masa adulta `M` (kg) y al nacer `m0 = f_neo·M`.
    pub masa: f64,
    pub m0: f64,
    /// Constante metabólica: `B0` (endotermo) o `B0_ecto` (ectotermo, a `T_ref`).
    pub b0: f64,
    pub pal: f64,
    pub f_grasa_max: f64,
    pub rho_grasa: f64,
    pub k_ap: f64,
    pub eps: f64,
    pub f_com: f64,
    pub camada: f64,
    pub p_conc: f64,
    pub rho_par: f64,
    pub rho_sexo: f64,
    pub p_huerf: f64,
    /// Duraciones en días.
    pub a_max: u32,
    pub a_crec90: u32,
    pub a_rep: u32,
    pub a_fin_rep: u32,
    pub d_gest: u32,
    pub d_destete: u32,
    pub delta_a: u32,
    /// Constante de crecimiento de West (kg^¼/día).
    pub c_w: f64,
    /// Siler en 1/año: `[α1, β1, α2, α3, β3]`.
    pub siler: [f64; 5],
    /// Días del año (para pasar edades y tasas anuales a días).
    pub dias_anio: f64,
    /// Por sexo (`[hembra, macho]`, revisión 1 §3.4): masa adulta, grasa máxima y constante de
    /// crecimiento de West. Sin dimorfismo, las tres son las de la especie.
    pub masa_sexo: [f64; 2],
    pub f_grasa_sexo: [f64; 2],
    pub c_w_sexo: [f64; 2],
    /// `true` si las masas adultas de los sexos difieren.
    pub dimorfico: bool,
    /// Semisaturación de la respuesta funcional relativa a `K` (revisión 1 §2).
    pub h_medio: f64,
    /// Edad de esterilidad (revisión 1 §4): media y desviación típica en días.
    pub mu_est: f64,
    pub sigma_est: f64,
    /// Edad a la que la mortalidad senescente iguala a la de fondo (días): separa "adultos" de
    /// "viejos" en las métricas de mortalidad por edad.
    pub a_vejez: u32,
}

fn elegir(especie: Option<f64>, preset: Option<f64>, derivado: f64) -> f64 {
    especie.or(preset).unwrap_or(derivado)
}

fn dias(nombre: &str, especie: &str, x: f64) -> Result<u32, ConfigError> {
    let d = x.round();
    if !(d >= 1.0 && d < u32::MAX as f64) {
        return Err(ConfigError::Invalid(format!(
            "especie '{especie}': {nombre} = {x} días fuera de rango"
        )));
    }
    Ok(d as u32)
}

impl Especie {
    /// Deriva la especie: ajustes propios > preset > ley de escala `Q_ref·(M/M_ref)^b`.
    pub fn derivar(cfg: &BioConfig, e: &EspecieCfg) -> Result<Especie, ConfigError> {
        let r = &cfg.referencia;
        let en = &cfg.energetica;
        let vacio = AjustesCfg::default();
        let pr = match &e.preset {
            Some(p) => cfg
                .presets
                .get(p)
                .ok_or_else(|| ConfigError::Invalid(format!("preset '{p}' inexistente")))?,
            None => &vacio,
        };
        let aj = &e.ajustes;
        let invalida = |m: &str| ConfigError::Invalid(format!("especie '{}': {m}", e.nombre));
        let masa = aj
            .masa_kg
            .or(pr.masa_kg)
            .ok_or_else(|| invalida("falta masa_kg (ni en la especie ni en su preset)"))?;
        if !(masa > 0.0 && masa.is_finite()) {
            return Err(invalida("masa_kg debe ser > 0"));
        }
        let ratio = masa / r.masa_kg;
        let escala = |q: f64, b: f64| q * ratio.powf(b);
        let da = cfg.tiempo.dias_anio as f64;

        let a_max_anios = elegir(
            aj.a_max_anios,
            pr.a_max_anios,
            escala(r.a_max_anios, r.b_a_max),
        );
        let a_crec90_anios = elegir(
            aj.a_crec90_anios,
            pr.a_crec90_anios,
            escala(r.a_crec90_anios, r.b_a_crec90),
        );
        let a_rep_anios = elegir(
            aj.a_rep_anios,
            pr.a_rep_anios,
            escala(r.a_rep_anios, r.b_a_rep),
        );
        let a_fin_rep_anios = elegir(aj.a_fin_rep_anios, pr.a_fin_rep_anios, a_max_anios);
        let d_gest = elegir(
            aj.d_gest_dias,
            pr.d_gest_dias,
            escala(r.d_gest_dias, r.b_d_gest),
        );
        let d_destete = elegir(
            aj.d_destete_dias,
            pr.d_destete_dias,
            escala(r.d_destete_dias, r.b_d_destete),
        );
        let delta_a = elegir(
            aj.delta_a_dias,
            pr.delta_a_dias,
            escala(r.delta_a_dias, r.b_delta_a),
        );
        let f_neo = elegir(aj.f_neo, pr.f_neo, r.f_neo);
        let m0 = f_neo * masa;

        // Siler: directo (preset/ajuste) o relativo a a_max (D11).
        let beta3 = elegir(
            aj.beta3,
            pr.beta3,
            std::f64::consts::LN_2 / (r.c_dup * a_max_anios),
        );
        let siler = [
            elegir(aj.siler_a1, pr.siler_a1, r.siler_a1_rel / a_max_anios),
            elegir(aj.siler_b1, pr.siler_b1, r.siler_b1_rel / a_max_anios),
            elegir(aj.siler_a2, pr.siler_a2, r.siler_a2_rel / a_max_anios),
            elegir(aj.siler_a3, pr.siler_a3, r.siler_a3_rel / a_max_anios),
            beta3,
        ];

        let a_crec90 = dias("a_crec90", &e.nombre, a_crec90_anios * da)?;
        // West: c_W tal que la cría llega al 90 % de su masa adulta a la edad a_crec90 (§6.0).
        let c_w_de = |m_adulta: f64| {
            let q0 = (m0 / m_adulta).powf(0.25);
            4.0 * m_adulta.powf(0.25) / a_crec90 as f64
                * ((1.0 - q0) / (1.0 - 0.9f64.powf(0.25))).ln()
        };
        let c_w = c_w_de(masa);
        let f_grasa_max = elegir(aj.f_grasa_max, pr.f_grasa_max, r.f_grasa_max);
        // Dimorfismo (revisión 1 §3.4): masa y grasa de cada sexo, si la especie las da.
        let masa_sexo = [
            elegir(aj.masa_kg_hembra, pr.masa_kg_hembra, masa),
            elegir(aj.masa_kg_macho, pr.masa_kg_macho, masa),
        ];
        let f_grasa_sexo = [
            elegir(aj.f_grasa_max_hembra, pr.f_grasa_max_hembra, f_grasa_max),
            elegir(aj.f_grasa_max_macho, pr.f_grasa_max_macho, f_grasa_max),
        ];
        let c_w_sexo = masa_sexo.map(|m| if m == masa { c_w } else { c_w_de(m) });
        let k_ap = elegir(aj.k_ap, pr.k_ap, r.k_ap);
        let a_fin_rep_d = dias("a_fin_rep", &e.nombre, a_fin_rep_anios * da)?;
        let a_rep_d = dias("a_rep", &e.nombre, a_rep_anios * da)?;
        // Esterilidad (revisión 1 §4); el genérico la deriva de su vida reproductiva.
        let vida_rep = (a_fin_rep_d as f64 - a_rep_d as f64) / da;
        let mu_est = elegir(
            aj.mu_est_anios,
            pr.mu_est_anios,
            a_rep_d as f64 / da + 0.7 * vida_rep,
        ) * da;
        let sigma_est = elegir(aj.sigma_est_anios, pr.sigma_est_anios, 0.1 * vida_rep) * da;
        // Vejez: α3·e^(β3·a) = α2.
        let a_max_d = dias("a_max", &e.nombre, a_max_anios * da)?;
        let a_vejez = if siler[3] > 0.0 && siler[2] > siler[3] && siler[4] > 0.0 {
            ((siler[2] / siler[3]).ln() / siler[4] * da).round() as u32
        } else {
            a_max_d
        }
        .min(a_max_d);

        let esp = Especie {
            nombre: e.nombre.clone(),
            termico: e.termico,
            masa,
            m0,
            b0: match e.termico {
                Termico::Endotermo => r.b0,
                Termico::Ectotermo => en.b0_ecto,
            },
            pal: elegir(aj.pal, pr.pal, r.pal),
            f_grasa_max,
            rho_grasa: en.rho_grasa,
            k_ap,
            eps: elegir(aj.eps, pr.eps, r.eps),
            f_com: elegir(aj.f_com, pr.f_com, r.f_com),
            camada: elegir(aj.camada, pr.camada, r.camada),
            p_conc: elegir(aj.p_conc, pr.p_conc, r.p_conc),
            rho_par: elegir(aj.rho_par, pr.rho_par, r.rho_par),
            rho_sexo: elegir(aj.rho_sexo, pr.rho_sexo, r.rho_sexo),
            p_huerf: elegir(aj.p_huerf, pr.p_huerf, r.p_huerf),
            a_max: a_max_d,
            a_crec90,
            a_rep: a_rep_d,
            a_fin_rep: a_fin_rep_d,
            d_gest: dias("d_gest", &e.nombre, d_gest)?,
            d_destete: dias("d_destete", &e.nombre, d_destete)?,
            delta_a: dias("delta_a", &e.nombre, delta_a)?,
            c_w,
            siler,
            dias_anio: da,
            masa_sexo,
            f_grasa_sexo,
            c_w_sexo,
            dimorfico: masa_sexo[0] != masa || masa_sexo[1] != masa,
            h_medio: elegir(aj.h_medio, pr.h_medio, ((k_ap - 1.0) / 2.0).max(0.0)),
            mu_est,
            sigma_est,
            a_vejez,
        };
        esp.validar()?;
        Ok(esp)
    }

    fn validar(&self) -> Result<(), ConfigError> {
        let invalida = |m: &str| ConfigError::Invalid(format!("especie '{}': {m}", self.nombre));
        let positivos = [
            self.pal,
            self.f_grasa_max,
            self.k_ap,
            self.eps,
            self.m0,
            self.c_w,
        ];
        if positivos.iter().any(|&x| !(x > 0.0 && x.is_finite())) {
            return Err(invalida(
                "pal, f_grasa_max, k_ap, eps, f_neo y la curva de crecimiento deben ser > 0",
            ));
        }
        if self.m0 >= 0.9 * self.masa {
            return Err(invalida("f_neo debe ser < 0,9"));
        }
        let fracciones = [
            self.eps,
            self.f_grasa_max,
            self.f_com,
            self.p_conc,
            self.rho_sexo,
            self.p_huerf,
        ];
        if fracciones.iter().any(|x| !(0.0..=1.0).contains(x)) {
            return Err(invalida(
                "eps, f_grasa_max, f_com, p_conc, rho_sexo y p_huerf deben estar en [0, 1]",
            ));
        }
        if !(self.camada >= 0.0 && self.camada.is_finite()) {
            return Err(invalida("camada debe ser >= 0"));
        }
        if self.siler.iter().any(|x| !(*x >= 0.0 && x.is_finite())) {
            return Err(invalida("los parámetros de Siler deben ser >= 0"));
        }
        if self.a_rep > self.a_max {
            return Err(invalida("a_rep no puede superar a_max"));
        }
        let por_sexo = self.masa_sexo.iter().chain(&self.f_grasa_sexo);
        if por_sexo.into_iter().any(|&x| !(x > 0.0 && x.is_finite())) {
            return Err(invalida("las masas y grasas por sexo deben ser > 0"));
        }
        if self.f_grasa_sexo.iter().any(|&x| x > 1.0) {
            return Err(invalida(
                "la grasa máxima de cada sexo debe estar en (0, 1]",
            ));
        }
        if self.masa_sexo.iter().any(|&m| self.m0 >= 0.9 * m) {
            return Err(invalida(
                "la masa al nacer debe ser < 0,9 de la de cada sexo",
            ));
        }
        if !(self.h_medio >= 0.0 && self.h_medio.is_finite()) {
            return Err(invalida("h_medio debe ser >= 0"));
        }
        if !(self.sigma_est >= 0.0 && self.mu_est.is_finite()) {
            return Err(invalida("se requiere sigma_est_anios >= 0"));
        }
        Ok(())
    }

    /// Metabolismo basal (kcal/día). `f_ecto` es el factor de temperatura de la celda de los
    /// ectotermos (§6.0); los endotermos lo ignoran.
    #[inline]
    pub fn basal(&self, m: f64, f_ecto: f64) -> f64 {
        match self.termico {
            Termico::Endotermo => self.b0 * m.powf(0.75),
            Termico::Ectotermo => self.b0 * m.powf(0.75) * f_ecto,
        }
    }

    /// Gasto diario del adulto en mantenimiento y actividad, `B(M)·PAL`.
    pub fn gasto_adulto(&self, f_ecto: f64) -> f64 {
        self.basal(self.masa, f_ecto) * self.pal
    }

    /// Reserva máxima `E_max(m) = f_grasa_max · m · ρ_grasa` (kcal), con la grasa de la especie.
    #[inline]
    pub fn e_max(&self, m: f64) -> f64 {
        self.f_grasa_max * m * self.rho_grasa
    }

    /// Reserva máxima de un individuo del sexo `s` (0 hembra, 1 macho), con su grasa máxima.
    #[inline]
    pub fn e_max_s(&self, s: usize, m: f64) -> f64 {
        self.f_grasa_sexo[s] * m * self.rho_grasa
    }

    /// Gasto diario del adulto del sexo `s` en mantenimiento y actividad, `B(M_s)·PAL`.
    pub fn gasto_adulto_s(&self, s: usize, f_ecto: f64) -> f64 {
        self.basal(self.masa_sexo[s], f_ecto) * self.pal
    }

    /// Días que aguanta sin comer un adulto del sexo `s`: `E_max(M_s) / (B(M_s)·PAL)`.
    pub fn ayuno_s(&self, s: usize) -> f64 {
        self.e_max_s(s, self.masa_sexo[s]) / self.gasto_adulto_s(s, 1.0)
    }

    /// Edad de esterilidad (días) de una hembra (revisión 1 §4): `Normal(μ_est, σ_est)` truncada a
    /// `[a_rep, a_fin_rep]`, sorteada con el azar por contador `H(semilla, id, "esterilidad")`.
    /// Es función pura del individuo: no hace falta guardarla.
    pub fn a_est(&self, semilla: u64, id: u64) -> u32 {
        let (lo, hi) = (self.a_rep as f64, self.a_fin_rep as f64);
        // Box–Muller con rechazo de lo que cae fuera (con los valores de la nota, ~0,1 %).
        for k in 0..32u64 {
            let u1 = uniforme(semilla, id, 0, Proposito::Esterilidad, 2 * k);
            let u2 = uniforme(semilla, id, 0, Proposito::Esterilidad, 2 * k + 1);
            let z = (-2.0 * (1.0 - u1).ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
            let x = self.mu_est + self.sigma_est * z;
            if (lo..=hi).contains(&x) {
                return x.round() as u32;
            }
        }
        self.mu_est.clamp(lo, hi).round() as u32
    }

    /// Masa por edad sin retraso (solución exacta de West, §6.6), con la masa de la especie.
    pub fn masa_por_edad(&self, a_dias: u32) -> f64 {
        masa_west(self.m0, self.masa, self.c_w, a_dias)
    }

    /// Masa por edad sin retraso de un individuo del sexo `s`.
    pub fn masa_por_edad_s(&self, s: usize, a_dias: u32) -> f64 {
        masa_west(self.m0, self.masa_sexo[s], self.c_w_sexo[s], a_dias)
    }

    /// Crecimiento del día con comida suficiente (West): `c_W·m^¾·(1 − (m/M)^¼)`, sin pasar de `M`.
    #[inline]
    pub fn dm_west(&self, m: f64) -> f64 {
        dm_west(self.masa, self.c_w, m)
    }

    /// Crecimiento del día de un individuo del sexo `s`, hacia su masa adulta.
    #[inline]
    pub fn dm_west_s(&self, s: usize, m: f64) -> f64 {
        dm_west(self.masa_sexo[s], self.c_w_sexo[s], m)
    }

    /// Riesgo anual de Siler `μ(a)` (1/año), con `a` en años.
    #[inline]
    pub fn mu_siler(&self, a_anios: f64) -> f64 {
        let [a1, b1, a2, a3, b3] = self.siler;
        a1 * (-b1 * a_anios).exp() + a2 + a3 * (b3 * a_anios).exp()
    }

    /// Probabilidad de morir hoy por causas intrínsecas, con la edad al empezar el día.
    #[inline]
    pub fn p_muerte_dia(&self, a_dias: u32) -> f64 {
        let mu = self.mu_siler(a_dias as f64 / self.dias_anio);
        1.0 - (-mu / self.dias_anio).exp()
    }

    /// Fracción de su gasto que la cría recibe como leche: `max(0, 1 − a/d_destete)`.
    #[inline]
    pub fn phi_leche(&self, a_dias: u32) -> f64 {
        (1.0 - a_dias as f64 / self.d_destete as f64).max(0.0)
    }

    /// Fertilidad por condición corporal `f_E(r)` (§6.7).
    #[inline]
    pub fn f_e(r: f64, r_min: f64, r_opt: f64) -> f64 {
        ((r - r_min) / (r_opt - r_min)).clamp(0.0, 1.0)
    }

    /// Población máxima sostenible de una celda (§1): `f_com·PPN·A·ρ_veg·ε / (días·G_adulto)` (D6).
    /// Con dimorfismo, `G_adulto` es la media de los dos sexos pesada por la proporción de sexos.
    pub fn n_max(&self, area_m2: f64, ppn: f64, rho_veg: f64, f_ecto: f64) -> f64 {
        let g = if self.dimorfico {
            (1.0 - self.rho_sexo) * self.gasto_adulto_s(0, f_ecto)
                + self.rho_sexo * self.gasto_adulto_s(1, f_ecto)
        } else {
            self.gasto_adulto(f_ecto)
        };
        self.f_com * ppn * area_m2 * rho_veg * self.eps / (self.dias_anio * g)
    }

    /// Clases de edad de anchura `Δa` hasta `a_max` (pirámide y tablas de vida).
    pub fn n_clases(&self) -> usize {
        (self.a_max as usize).div_ceil(self.delta_a as usize) + 1
    }

    /// Clase de edad de una edad en días.
    #[inline]
    pub fn clase(&self, a_dias: u32) -> usize {
        ((a_dias / self.delta_a) as usize).min(self.n_clases() - 1)
    }

    /// Supervivencia de Siler día a día (sin inanición) hasta `a_max`: `l[k]` al empezar el día `k`.
    pub fn supervivencia(&self) -> Vec<f64> {
        let mut l = Vec::with_capacity(self.a_max as usize + 1);
        let mut x = 1.0;
        for a in 0..=self.a_max {
            l.push(x);
            x *= 1.0 - self.p_muerte_dia(a);
        }
        l
    }
}

/// Masa por edad sin retraso (solución exacta de West) hacia la masa adulta `m_adulta`.
fn masa_west(m0: f64, m_adulta: f64, c_w: f64, a_dias: u32) -> f64 {
    let q0 = (m0 / m_adulta).powf(0.25);
    let q = 1.0 - (1.0 - q0) * (-c_w * a_dias as f64 / (4.0 * m_adulta.powf(0.25))).exp();
    (m_adulta * q.powi(4)).min(m_adulta)
}

/// Crecimiento de un día (West) de masa `m` hacia la masa adulta `m_adulta`.
#[inline]
fn dm_west(m_adulta: f64, c_w: f64, m: f64) -> f64 {
    if m >= m_adulta {
        return 0.0;
    }
    let dm = c_w * m.powf(0.75) * (1.0 - (m / m_adulta).powf(0.25));
    dm.min(m_adulta - m).max(0.0)
}

/// Factor de temperatura del metabolismo de los ectotermos (§6.0, teoría metabólica).
pub fn factor_ecto(t: f64, e_a: f64, t_ref: f64) -> f64 {
    (-(e_a / K_BOLTZMANN_EV) * (1.0 / (t + 273.15) - 1.0 / (t_ref + 273.15))).exp()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biologia::config::{BioConfig, EspecieCfg};
    use crate::biologia::habitat::ppn_miami;

    fn especies() -> (Especie, Especie) {
        let cfg = BioConfig::default();
        let g70 = Especie::derivar(&cfg, &cfg.especies[0]).unwrap();
        let hum = Especie::derivar(&cfg, &cfg.especies[1]).unwrap();
        (g70, hum)
    }

    fn cerca(x: f64, objetivo: f64, tol_rel: f64) -> bool {
        (x / objetivo - 1.0).abs() <= tol_rel
    }

    #[test]
    fn tabla_de_calibracion_del_diseno() {
        // §10, "comprobaciones de calibración (ya verificadas numéricamente)".
        let (g70, hum) = especies();
        let cfg = BioConfig::default();
        let en = &cfg.energetica;
        assert!(cerca(hum.basal(60.0, 1.0), 1509.0, 0.002), "Kleiber humano");
        assert!(cerca(hum.gasto_adulto(1.0), 2716.0, 0.002), "gasto humano");
        let ayuno_h = hum.e_max(hum.masa) / hum.gasto_adulto(1.0);
        let ayuno_g = g70.e_max(g70.masa) / g70.gasto_adulto(1.0);
        assert!((ayuno_h - 40.0).abs() < 1.0, "ayuno humano {ayuno_h}");
        assert!((ayuno_g - 22.0).abs() < 1.0, "ayuno genérico {ayuno_g}");
        let gest = en.k_gest * hum.basal(hum.masa, 1.0) * hum.d_gest as f64;
        assert!(cerca(gest, 81_500.0, 0.01), "coste de gestación {gest}");
        // Densidad máxima con PPN 700: 3,9/km² (Damuth: 96·M^-¾) y 0,3/km² humanos.
        let km2 = 1e6;
        let rho_veg = cfg.forraje.rho_veg;
        let d_gen = g70.n_max(km2, 700.0, rho_veg, 1.0);
        let damuth = 96.0 * 70f64.powf(-0.75);
        assert!(
            cerca(d_gen, 3.9, 0.03) && cerca(d_gen, damuth, 0.03),
            "{d_gen} vs {damuth}"
        );
        assert!(cerca(hum.n_max(km2, 700.0, rho_veg, 1.0), 0.29, 0.05));
        // Modelo Miami (g/m²/año).
        for (t, p, ppn) in [
            (25.0, 2500.0, 2430.0),
            (25.0, 100.0, 193.0),
            (-10.0, 300.0, 227.0),
            (10.0, 700.0, 1115.0),
        ] {
            assert!(
                cerca(ppn_miami(t, p), ppn, 0.01),
                "Miami({t}, {p}) = {}",
                ppn_miami(t, p)
            );
        }
    }

    /// Tabla de vida con la regla diaria del modelo: (e0 en años, l en cada edad en años).
    fn vida(e: &Especie) -> (f64, impl Fn(f64) -> f64) {
        let l = e.supervivencia();
        let e0 = l.iter().take(e.a_max as usize).sum::<f64>() / e.dias_anio;
        let da = e.dias_anio;
        (e0, move |anios: f64| l[(anios * da).round() as usize])
    }

    #[test]
    fn tabla_de_vida_de_siler() {
        // Incremento 1 §4: objetivos del §10 con comida suficiente.
        let (g70, hum) = especies();
        let (e0, l) = vida(&hum);
        assert!((30.0..=35.0).contains(&e0), "e0 humano {e0}");
        assert!((0.55..=0.60).contains(&l(15.0)), "l(15) {}", l(15.0));
        // Moda de las muertes adultas (por años, desde los 15).
        let mut mejor = (0, 0.0);
        for a in 15..109 {
            let d = l(a as f64) - l(a as f64 + 1.0);
            if d > mejor.1 {
                mejor = (a, d);
            }
        }
        assert!((65..=75).contains(&mejor.0), "moda adulta {}", mejor.0);
        let (_, lg) = vida(&g70);
        assert!(
            (0.58..=0.62).contains(&lg(1.0)),
            "l(1) genérico {}",
            lg(1.0)
        );
        for a in 2..8 {
            let q = 1.0 - lg(a as f64 + 1.0) / lg(a as f64);
            assert!(
                (0.09..=0.125).contains(&q),
                "mortalidad adulta a {a} años: {q}"
            );
        }
    }

    #[test]
    fn la_curva_de_crecimiento_cumple_west() {
        let (g70, hum) = especies();
        for e in [&g70, &hum] {
            assert!((e.masa_por_edad(0) / e.m0 - 1.0).abs() < 1e-12);
            let m90 = e.masa_por_edad(e.a_crec90);
            assert!(
                (m90 / (0.9 * e.masa) - 1.0).abs() < 1e-3,
                "{}: {m90}",
                e.nombre
            );
            // Integrar el crecimiento diario debe seguir a la solución exacta (±2 %).
            let mut m = e.m0;
            for _ in 0..e.a_crec90 {
                m += e.dm_west(m);
            }
            assert!(
                (m / m90 - 1.0).abs() < 0.02,
                "{}: integrada {m} vs {m90}",
                e.nombre
            );
        }
        // El humano nace creciendo ~7 g/día (pregunta abierta P1).
        assert!((hum.dm_west(hum.m0) * 1000.0 - 7.2).abs() < 0.3);
    }

    #[test]
    fn la_especie_se_escala_con_la_masa() {
        let mut cfg = BioConfig::default();
        cfg.especies.push(EspecieCfg {
            nombre: "grande".into(),
            ajustes: AjustesCfg {
                masa_kg: Some(700.0),
                ..AjustesCfg::default()
            },
            ..EspecieCfg::default()
        });
        let g70 = Especie::derivar(&cfg, &cfg.especies[0]).unwrap();
        let grande = Especie::derivar(&cfg, &cfg.especies[2]).unwrap();
        let r = 10f64;
        assert!(cerca(
            grande.a_max as f64 / g70.a_max as f64,
            r.powf(0.2),
            0.001
        ));
        assert!(cerca(
            grande.d_gest as f64 / g70.d_gest as f64,
            r.powf(0.25),
            0.01
        ));
        // Equivalencia energética: densidad · gasto no depende de la masa.
        let dg = g70.n_max(1e6, 700.0, 4.4, 1.0) * g70.gasto_adulto(1.0);
        let dgr = grande.n_max(1e6, 700.0, 4.4, 1.0) * grande.gasto_adulto(1.0);
        assert!(cerca(dg, dgr, 1e-12));
        // Siler relativo: la supervivencia a la misma fracción de a_max coincide.
        let l1 = g70.supervivencia();
        let l2 = grande.supervivencia();
        let x = 0.3;
        let a1 = (x * g70.a_max as f64) as usize;
        let a2 = (x * grande.a_max as f64) as usize;
        assert!((l1[a1] - l2[a2]).abs() < 0.01, "{} vs {}", l1[a1], l2[a2]);
    }

    #[test]
    fn el_dimorfismo_da_las_cifras_de_la_nota() {
        // Revisión 1 §3.4, con PAL 1,8: mujer 1 433 / 2 579 / 126 000 kcal y ~49 días de ayuno;
        // hombre 1 584 / 2 851 / 86 400 kcal y ~30 días.
        let (_, hum) = especies();
        assert!(hum.dimorfico);
        let tabla = [
            (0, 1433.0, 2579.0, 126_000.0, 49.0),
            (1, 1584.0, 2851.0, 86_400.0, 30.0),
        ];
        for (s, basal, gasto, e_max, ayuno) in tabla {
            let m = hum.masa_sexo[s];
            assert!(cerca(hum.basal(m, 1.0), basal, 0.002), "basal {s}");
            assert!(cerca(hum.gasto_adulto_s(s, 1.0), gasto, 0.002), "gasto {s}");
            assert!(cerca(hum.e_max_s(s, m), e_max, 1e-9), "E_max {s}");
            assert!(
                (hum.ayuno_s(s) - ayuno).abs() < 1.0,
                "ayuno {s}: {}",
                hum.ayuno_s(s)
            );
            // Cada sexo llega al 90 % de su masa adulta a la edad a_crec90, desde los 3,3 kg.
            let m90 = hum.masa_por_edad_s(s, hum.a_crec90);
            assert!((m90 / (0.9 * m) - 1.0).abs() < 1e-3, "West {s}: {m90}");
            assert!((hum.masa_por_edad_s(s, 0) - 3.3).abs() < 1e-9);
        }
        // Sin valores por sexo, las tablas por sexo son las de la especie (bit a bit).
        let mut cfg = BioConfig::default();
        let preset = cfg.presets.get_mut("humano").unwrap();
        preset.masa_kg_hembra = None;
        preset.masa_kg_macho = None;
        preset.f_grasa_max_hembra = None;
        preset.f_grasa_max_macho = None;
        let plano = Especie::derivar(&cfg, &cfg.especies[1]).unwrap();
        assert!(!plano.dimorfico);
        for s in 0..2 {
            assert_eq!(plano.masa_sexo[s].to_bits(), plano.masa.to_bits());
            assert_eq!(plano.f_grasa_sexo[s].to_bits(), plano.f_grasa_max.to_bits());
            assert_eq!(plano.c_w_sexo[s].to_bits(), plano.c_w.to_bits());
        }
        // El genérico no tiene dimorfismo por defecto.
        let (g70, _) = especies();
        assert!(!g70.dimorfico && g70.masa_sexo == [g70.masa; 2]);
    }

    #[test]
    fn la_edad_de_esterilidad_sigue_una_normal_truncada() {
        // Revisión 1 §4: humano Normal(38, 4) años; genérico derivado de su vida reproductiva.
        let (g70, hum) = especies();
        for (e, mu, sigma) in [(&hum, 38.0, 4.0), (&g70, 17.95, 2.35)] {
            let n = 20_000;
            let xs: Vec<f64> = (0..n)
                .map(|id| f64::from(e.a_est(42, id)) / e.dias_anio)
                .collect();
            let media = xs.iter().sum::<f64>() / n as f64;
            let sd = (xs.iter().map(|x| (x - media).powi(2)).sum::<f64>() / n as f64).sqrt();
            assert!((media - mu).abs() < 0.1, "{}: media {media}", e.nombre);
            assert!((sd / sigma - 1.0).abs() < 0.03, "{}: sd {sd}", e.nombre);
            let (lo, hi) = (e.a_rep, e.a_fin_rep);
            assert!((0..n).all(|id| (lo..=hi).contains(&e.a_est(42, id))));
            // Función pura del individuo.
            assert_eq!(e.a_est(42, 7), e.a_est(42, 7));
        }
    }
}
