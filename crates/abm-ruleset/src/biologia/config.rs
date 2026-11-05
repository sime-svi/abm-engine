//! Configuración de la biología de los agentes (`config/agentes.toml`). Todos los parámetros del
//! documento de diseño (§5) viven aquí; el código solo tiene las fórmulas (CLAUDE §5).

use std::collections::BTreeMap;

use abm_core::ConfigError;
use serde::{Deserialize, Serialize};

/// Configuración completa de una corrida de biología.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BioConfig {
    pub run: RunCfg,
    pub tiempo: TiempoCfg,
    /// Especie de referencia (70 kg) y exponentes de escala (§5.1).
    pub referencia: ReferenciaCfg,
    /// Energética, crecimiento y reproducción comunes (§5.3).
    pub energetica: EnergeticaCfg,
    /// Forraje (§5.5).
    pub forraje: ForrajeCfg,
    /// Población inicial (§5.6, §6.0).
    pub poblacion: PoblacionCfg,
    /// Presets con valores directos (§5.2), por nombre.
    pub presets: BTreeMap<String, AjustesCfg>,
    /// Especies de la corrida.
    pub especies: Vec<EspecieCfg>,
    /// Hábitat de prueba del incremento 1.
    pub habitat: HabitatCfg,
    /// Métricas opcionales de la salida.
    pub metricas: MetricasCfg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RunCfg {
    pub name: String,
    pub seed: u64,
    pub total_ticks: u64,
    /// Cada cuántos ticks se guarda un registro (los recuentos se acumulan entre registros).
    pub metrics_every: u64,
    pub output_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TiempoCfg {
    /// Días del año (debe coincidir con `climate.t_year` del terreno).
    pub dias_anio: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReferenciaCfg {
    pub masa_kg: f64,
    /// Constante de Kleiber (kcal/(día·kg^0,75)).
    pub b0: f64,
    pub pal: f64,
    pub a_max_anios: f64,
    pub b_a_max: f64,
    pub a_crec90_anios: f64,
    pub b_a_crec90: f64,
    pub a_rep_anios: f64,
    pub b_a_rep: f64,
    pub d_gest_dias: f64,
    pub b_d_gest: f64,
    pub d_destete_dias: f64,
    pub b_d_destete: f64,
    pub delta_a_dias: f64,
    pub b_delta_a: f64,
    pub camada: f64,
    pub f_neo: f64,
    pub f_grasa_max: f64,
    pub k_ap: f64,
    pub eps: f64,
    pub f_com: f64,
    pub p_conc: f64,
    pub rho_par: f64,
    pub c_dup: f64,
    pub rho_sexo: f64,
    pub p_huerf: f64,
    /// Siler relativo a `a_max` (α·a_max, β·a_max), calibrado en la especie de referencia
    /// (incremento 1, §4). `β3` sale de `c_dup`.
    pub siler_a1_rel: f64,
    pub siler_b1_rel: f64,
    pub siler_a2_rel: f64,
    pub siler_a3_rel: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EnergeticaCfg {
    pub rho_grasa: f64,
    pub eta: f64,
    pub c_crec: f64,
    pub r_crec_min: f64,
    pub k_gest: f64,
    pub eps_leche: f64,
    pub r_0: f64,
    pub r_min: f64,
    pub r_opt: f64,
    pub r_abort: f64,
    pub b0_ecto: f64,
    pub e_a: f64,
    pub t_ref: f64,
    /// Revisión 1, cambio 1 (D1 + hiperfagia): el apetito es energía asimilada y se refiere al
    /// gasto total del día, `Ap = k_ap · G_total`. `false` = `Ap = k_ap · B(m) · PAL`.
    pub apetito_gasto_total: bool,
    /// Revisión 1, cambio 3.1: la leche baja con la condición de la madre, entre `r_lact` (leche
    /// completa) y `r_aband` (sin leche). `false` = leche completa siempre.
    pub leche_por_condicion: bool,
    pub r_lact: f64,
    pub r_aband: f64,
    /// Revisión 1, cambio 3.2: destete forzoso. Si la condición de la lactante baja de `r_aband`,
    /// la lactancia termina; las crías de al menos `f_dest_min · d_destete` días sobreviven
    /// destetadas y las más jóvenes mueren con `p_huerf`. `false` = sin destete forzoso.
    pub destete_forzoso: bool,
    pub f_dest_min: f64,
    /// Revisión 1, cambio 3.4: dimorfismo sexual. Cada sexo usa su masa adulta y su grasa máxima
    /// si la especie las da. `false` = la misma para los dos sexos.
    pub dimorfismo: bool,
    /// Revisión 1, cambio 4: edad de esterilidad individual `a_est ~ Normal(μ_est, σ_est)`,
    /// truncada a `[a_rep, a_fin_rep]`. `false` = fértil hasta `a_fin_rep`.
    pub esterilidad_individual: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ForrajeCfg {
    /// Energía bruta de la materia vegetal seca (kcal/g).
    pub rho_veg: f64,
    /// Años de producción que caben en el stock máximo.
    pub tau_c: f64,
    /// Rebrote mínimo (banco de semillas), fracción de la regeneración máxima.
    pub g0_frac: f64,
    /// Revisión 1, cambio 2 (sustituye a la opción `respuesta` del incremento 1, D16): respuesta
    /// funcional de Holling tipo II. Cada uno come `d · V/(V + V_½)`, con `V_½ = h_½ · K` (`h_½`
    /// por especie). `false` = todo el stock accesible, como el diseño v1.0 (§6.4).
    pub respuesta_funcional: bool,
}

/// Cómo se reparten las edades de la población inicial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdadesIniciales {
    /// Distribución estable aproximada, ∝ supervivencia `l(a)` (§6.0).
    Estable,
    /// Todos recién nacidos (cohorte, para tablas de vida).
    Cero,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PoblacionCfg {
    /// Población inicial / capacidad estimada (§6.0).
    pub fraccion_inicial: f64,
    /// Si se da, nº fijo de individuos por celda y especie (sustituye a `fraccion_inicial`).
    pub n_inicial_por_celda: Option<u32>,
    /// Reserva inicial, en fracción de `E_max` (§6.0: 0,5).
    pub reserva_inicial: f64,
    pub edades_iniciales: EdadesIniciales,
}

/// Valores directos que sustituyen a los derivados por alometría (presets y ajustes por especie).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AjustesCfg {
    pub masa_kg: Option<f64>,
    pub pal: Option<f64>,
    pub f_grasa_max: Option<f64>,
    pub a_max_anios: Option<f64>,
    pub a_crec90_anios: Option<f64>,
    pub a_rep_anios: Option<f64>,
    pub a_fin_rep_anios: Option<f64>,
    pub d_gest_dias: Option<f64>,
    pub d_destete_dias: Option<f64>,
    pub delta_a_dias: Option<f64>,
    pub f_neo: Option<f64>,
    pub camada: Option<f64>,
    pub k_ap: Option<f64>,
    pub eps: Option<f64>,
    pub f_com: Option<f64>,
    pub p_conc: Option<f64>,
    pub rho_par: Option<f64>,
    pub rho_sexo: Option<f64>,
    pub p_huerf: Option<f64>,
    /// Siler directo (1/año).
    pub siler_a1: Option<f64>,
    pub siler_b1: Option<f64>,
    pub siler_a2: Option<f64>,
    pub siler_a3: Option<f64>,
    pub beta3: Option<f64>,
    /// Semisaturación de la respuesta funcional, relativa a `K` (revisión 1 §2). Por defecto
    /// `(k_ap − 1)/2`, que pone el equilibrio en `K/2`.
    pub h_medio: Option<f64>,
    /// Dimorfismo sexual (revisión 1 §3.4): masa adulta y grasa máxima de cada sexo. Por
    /// defecto, las de la especie.
    pub masa_kg_hembra: Option<f64>,
    pub masa_kg_macho: Option<f64>,
    pub f_grasa_max_hembra: Option<f64>,
    pub f_grasa_max_macho: Option<f64>,
    /// Edad de esterilidad (revisión 1 §4): media y desviación típica (años). Por defecto
    /// `a_rep + 0,7 · (a_fin_rep − a_rep)` y `0,1 · (a_fin_rep − a_rep)`.
    pub mu_est_anios: Option<f64>,
    pub sigma_est_anios: Option<f64>,
}

/// Tipo térmico de una especie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Termico {
    #[default]
    Endotermo,
    Ectotermo,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EspecieCfg {
    pub nombre: String,
    pub termico: Termico,
    /// Preset de `[presets]` cuyos valores sustituyen a los derivados.
    pub preset: Option<String>,
    /// Ajustes propios de la especie (prioridad sobre el preset). `masa_kg` es obligatoria si
    /// el preset no la da.
    pub ajustes: AjustesCfg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HabitatCfg {
    pub zonas: Vec<ZonaCfg>,
    /// Sequía de prueba: el rebrote se multiplica por `factor` durante la ventana.
    pub sequia: Option<SequiaCfg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ZonaCfg {
    pub nombre: String,
    pub celdas: u32,
    pub area_km2: f64,
    /// Temperatura media anual (°C): productividad (Miami) y metabolismo de los ectotermos.
    pub t_media: f64,
    /// Precipitación media anual (mm).
    pub p_media_mm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SequiaCfg {
    pub inicio_dia: u64,
    pub duracion_dias: u64,
    pub factor: f64,
}

/// Métricas opcionales (no cambian el modelo, solo lo que se escribe).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MetricasCfg {
    /// Demografía por edades y condición (revisión 1): fecundidad por grupos de edad de la madre,
    /// edad del último parto, mortalidad por grupo de edad y sexo, y condición de las lactantes.
    pub demografia: bool,
    /// Anchura de los grupos de edad de la fecundidad (años).
    pub ancho_fecundidad_anios: f64,
}

impl Default for MetricasCfg {
    fn default() -> Self {
        Self {
            demografia: true,
            ancho_fecundidad_anios: 5.0,
        }
    }
}

impl Default for RunCfg {
    fn default() -> Self {
        Self {
            name: "agentes".into(),
            seed: 42,
            total_ticks: 365 * 30,
            metrics_every: 30,
            output_path: "output".into(),
        }
    }
}

impl Default for TiempoCfg {
    fn default() -> Self {
        Self { dias_anio: 365 }
    }
}

impl Default for ReferenciaCfg {
    fn default() -> Self {
        Self {
            masa_kg: 70.0,
            b0: 70.0,
            pal: 2.5,
            a_max_anios: 25.0,
            b_a_max: 0.20,
            a_crec90_anios: 1.5,
            b_a_crec90: 0.25,
            a_rep_anios: 1.5,
            b_a_rep: 0.25,
            d_gest_dias: 200.0,
            b_d_gest: 0.25,
            d_destete_dias: 120.0,
            b_d_destete: 0.25,
            delta_a_dias: 365.0,
            b_delta_a: 0.25,
            camada: 1.0,
            f_neo: 0.06,
            f_grasa_max: 0.15,
            k_ap: 2.0,
            eps: 0.5,
            f_com: 0.004,
            p_conc: 0.03,
            rho_par: 0.1,
            c_dup: 0.08,
            rho_sexo: 0.5,
            p_huerf: 1.0,
            siler_a1_rel: 85.83,
            siler_b1_rel: 215.2,
            siler_a2_rel: 2.634,
            siler_a3_rel: 0.04116,
        }
    }
}

impl Default for EnergeticaCfg {
    fn default() -> Self {
        Self {
            rho_grasa: 9000.0,
            eta: 0.85,
            c_crec: 3000.0,
            r_crec_min: 0.2,
            k_gest: 0.2,
            eps_leche: 0.8,
            r_0: 0.5,
            r_min: 0.3,
            r_opt: 0.6,
            r_abort: 0.1,
            b0_ecto: 7.0,
            e_a: 0.65,
            t_ref: 20.0,
            apetito_gasto_total: true,
            leche_por_condicion: true,
            r_lact: 0.3,
            r_aband: 0.1,
            destete_forzoso: true,
            f_dest_min: 0.5,
            dimorfismo: true,
            esterilidad_individual: true,
        }
    }
}

impl Default for ForrajeCfg {
    fn default() -> Self {
        Self {
            rho_veg: 4.4,
            tau_c: 0.5,
            g0_frac: 0.01,
            respuesta_funcional: true,
        }
    }
}

impl Default for PoblacionCfg {
    fn default() -> Self {
        Self {
            fraccion_inicial: 0.5,
            n_inicial_por_celda: None,
            reserva_inicial: 0.5,
            edades_iniciales: EdadesIniciales::Estable,
        }
    }
}

impl Default for ZonaCfg {
    fn default() -> Self {
        Self {
            nombre: "templada".into(),
            celdas: 16,
            area_km2: 100.0,
            t_media: 10.0,
            p_media_mm: 700.0,
        }
    }
}

impl Default for HabitatCfg {
    fn default() -> Self {
        Self {
            zonas: vec![ZonaCfg::default()],
            sequia: None,
        }
    }
}

/// Preset "humano" del documento de diseño (§5.2) y Siler de Gurven y Kaplan (2007).
pub fn preset_humano() -> AjustesCfg {
    AjustesCfg {
        masa_kg: Some(60.0),
        pal: Some(1.8),
        f_grasa_max: Some(0.20),
        a_max_anios: Some(110.0),
        a_crec90_anios: Some(16.0),
        a_rep_anios: Some(19.0),
        a_fin_rep_anios: Some(50.0),
        d_gest_dias: Some(270.0),
        d_destete_dias: Some(900.0),
        f_neo: Some(0.055),
        camada: Some(1.0),
        eps: Some(0.85),
        f_com: Some(1.1e-4),
        p_conc: Some(0.007),
        rho_par: Some(1.0),
        rho_sexo: Some(0.512),
        p_huerf: Some(0.5),
        siler_a1: Some(0.422),
        siler_b1: Some(1.131),
        siler_a2: Some(0.013),
        siler_a3: Some(1.47e-4),
        beta3: Some(0.085),
        // Revisión 1 §3.4 (dimorfismo) y §4 (esterilidad).
        masa_kg_hembra: Some(56.0),
        masa_kg_macho: Some(64.0),
        f_grasa_max_hembra: Some(0.25),
        f_grasa_max_macho: Some(0.15),
        mu_est_anios: Some(38.0),
        sigma_est_anios: Some(4.0),
        ..AjustesCfg::default()
    }
}

impl Default for BioConfig {
    fn default() -> Self {
        let mut presets = BTreeMap::new();
        presets.insert("humano".to_string(), preset_humano());
        Self {
            run: RunCfg::default(),
            tiempo: TiempoCfg::default(),
            referencia: ReferenciaCfg::default(),
            energetica: EnergeticaCfg::default(),
            forraje: ForrajeCfg::default(),
            poblacion: PoblacionCfg::default(),
            presets,
            especies: vec![
                EspecieCfg {
                    nombre: "herbivoro_70kg".into(),
                    ajustes: AjustesCfg {
                        masa_kg: Some(70.0),
                        ..AjustesCfg::default()
                    },
                    ..EspecieCfg::default()
                },
                EspecieCfg {
                    nombre: "humano".into(),
                    preset: Some("humano".into()),
                    ..EspecieCfg::default()
                },
            ],
            habitat: HabitatCfg::default(),
            metricas: MetricasCfg::default(),
        }
    }
}

fn invalida(msg: impl Into<String>) -> ConfigError {
    ConfigError::Invalid(msg.into())
}

impl BioConfig {
    /// Lee y valida `config/agentes.toml`.
    pub fn load(path: &str) -> Result<BioConfig, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_string(),
            source,
        })?;
        let cfg: BioConfig = toml::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_string(),
            source,
        })?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Comprueba rangos y coherencia. La derivación de cada especie se valida aparte
    /// (`Especie::derivar`).
    pub fn validate(&self) -> Result<(), ConfigError> {
        let en = &self.energetica;
        let fo = &self.forraje;
        let po = &self.poblacion;
        if self.tiempo.dias_anio == 0 {
            return Err(invalida("tiempo.dias_anio debe ser > 0"));
        }
        if self.run.metrics_every == 0 {
            return Err(invalida("run.metrics_every debe ser > 0"));
        }
        let pos = [
            ("energetica.rho_grasa", en.rho_grasa),
            ("energetica.c_crec", en.c_crec),
            ("energetica.b0_ecto", en.b0_ecto),
            ("forraje.rho_veg", fo.rho_veg),
            ("forraje.tau_c", fo.tau_c),
            ("referencia.masa_kg", self.referencia.masa_kg),
            ("referencia.b0", self.referencia.b0),
        ];
        for (n, v) in pos {
            if !(v > 0.0 && v.is_finite()) {
                return Err(invalida(format!("{n} debe ser > 0")));
            }
        }
        let frac = [
            ("energetica.eta", en.eta),
            ("energetica.eps_leche", en.eps_leche),
            ("energetica.r_0", en.r_0),
            ("energetica.r_crec_min", en.r_crec_min),
            ("energetica.r_abort", en.r_abort),
            ("forraje.g0_frac", fo.g0_frac),
            ("poblacion.fraccion_inicial", po.fraccion_inicial),
            ("poblacion.reserva_inicial", po.reserva_inicial),
        ];
        for (n, v) in frac {
            if !(0.0..=1.0).contains(&v) {
                return Err(invalida(format!("{n} debe estar en [0, 1]")));
            }
        }
        if en.eps_leche <= 0.0 {
            return Err(invalida("energetica.eps_leche debe ser > 0"));
        }
        if !(0.0 <= en.r_min && en.r_min < en.r_opt && en.r_opt <= 1.0) {
            return Err(invalida("se requiere 0 <= r_min < r_opt <= 1"));
        }
        if !(0.0 <= en.r_aband && en.r_aband < en.r_lact && en.r_lact <= 1.0) {
            return Err(invalida("se requiere 0 <= r_aband < r_lact <= 1"));
        }
        if !(0.0..=1.0).contains(&en.f_dest_min) {
            return Err(invalida("energetica.f_dest_min debe estar en [0, 1]"));
        }
        let ancho = self.metricas.ancho_fecundidad_anios;
        if !(ancho > 0.0 && ancho.is_finite()) {
            return Err(invalida("metricas.ancho_fecundidad_anios debe ser > 0"));
        }
        if self.especies.is_empty() {
            return Err(invalida("se requiere al menos una especie en [[especies]]"));
        }
        if self.especies.len() > u16::MAX as usize {
            return Err(invalida("demasiadas especies"));
        }
        for (i, e) in self.especies.iter().enumerate() {
            if e.nombre.is_empty() {
                return Err(invalida(format!("la especie {i} no tiene nombre")));
            }
            if let Some(p) = &e.preset
                && !self.presets.contains_key(p)
            {
                return Err(invalida(format!(
                    "la especie '{}' usa el preset '{p}', que no existe",
                    e.nombre
                )));
            }
        }
        if self.habitat.zonas.is_empty() || self.habitat.zonas.iter().all(|z| z.celdas == 0) {
            return Err(invalida("el hábitat necesita al menos una zona con celdas"));
        }
        let celdas: u64 = self.habitat.zonas.iter().map(|z| z.celdas as u64).sum();
        if celdas > u32::MAX as u64 {
            return Err(invalida("demasiadas celdas en el hábitat"));
        }
        for z in &self.habitat.zonas {
            if !(z.area_km2 > 0.0 && z.p_media_mm >= 0.0 && z.t_media.is_finite()) {
                return Err(invalida(format!(
                    "zona '{}': se requiere area_km2 > 0 y p_media_mm >= 0",
                    z.nombre
                )));
            }
        }
        if let Some(s) = &self.habitat.sequia
            && !(s.factor >= 0.0 && s.factor.is_finite())
        {
            return Err(invalida("habitat.sequia.factor debe ser >= 0"));
        }
        Ok(())
    }
}
