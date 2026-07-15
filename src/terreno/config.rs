//! Configuración de la simulación de terreno (CLAUDE §5: todo parámetro vive en `config/*.toml`).
//!
//! Los valores por defecto son los del documento madre **reescalados al tick diario** (`T_anio=365`):
//! toda tasa por tick se multiplica por 48/365, toda cadencia por 365/48 (diseño §7).

use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

/// Configuración completa de una ejecución de terreno.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TerrenoConfig {
    pub run: RunCfg,
    pub planet: PlanetCfg,
    pub worldgen: WorldgenCfg,
    pub climate: ClimateCfg,
    pub soil: SoilCfg,
    pub vegetation: VegCfg,
    pub persistence: PersistenceCfg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RunCfg {
    pub name: String,
    pub seed: u64,
    pub total_ticks: u64,
    pub output_path: String,
    /// Vuelca además un snapshot por celda del mundo generado (para inspección).
    pub dump_world: bool,
}

impl Default for RunCfg {
    fn default() -> Self {
        Self {
            name: "terreno".to_string(),
            seed: 42,
            total_ticks: 365,
            output_path: "output".to_string(),
            dump_world: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PlanetCfg {
    pub r_km: f64,
    /// Celdas por arista de cara en L=0.
    pub n0: u32,
    /// Nivel uniforme al que se puebla el árbol (LOD-ready pero sin subdividir).
    pub l_unif: u32,
    pub sea_level: f64,
    pub h_min: f64,
    pub h_max: f64,
}

impl Default for PlanetCfg {
    fn default() -> Self {
        Self {
            r_km: 6371.0,
            n0: 48,
            l_unif: 0,
            sea_level: 0.0,
            h_min: -11000.0,
            h_max: 9000.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorldgenCfg {
    pub noise_octaves: u32,
    pub persistence: f64,
    pub lacunarity: f64,
    pub base_freq: f64,
    /// Exponente hipsométrico de la tierra emergida (>1 => domina la tierra baja, tipo Tierra).
    pub land_exp: f64,
    /// Umbral de área de drenaje (km²) para que una celda sea río.
    pub river_threshold_km2: f64,
    /// Densidad del recurso genérico (fracción de celdas de tierra con depósito, 0..1).
    pub resource_density: f64,
}

impl Default for WorldgenCfg {
    fn default() -> Self {
        Self {
            noise_octaves: 6,
            persistence: 0.5,
            lacunarity: 2.0,
            base_freq: 1.6,
            land_exp: 3.0,
            river_threshold_km2: 80_000.0,
            resource_density: 0.05,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ClimateCfg {
    pub t_equator: f64,
    pub k_lat: f64,
    pub lapse: f64,
    pub seas_amp_pole: f64,
    pub t_year: f64,
    pub t_freeze: f64,
    pub k_melt: f64,
    pub p_ref: f64,
    pub k_oro: f64,
    pub m_oro_min: f64,
    pub m_oro_max: f64,
    pub moisture_length_km: f64,
    pub seas_precip_amp: f64,
}

impl Default for ClimateCfg {
    fn default() -> Self {
        Self {
            t_equator: 30.0,
            k_lat: 0.6,
            lapse: 6.5,
            seas_amp_pole: 25.0,
            t_year: 365.0,
            t_freeze: 0.0,
            k_melt: 0.26, // 2 mm/(°C·tick) · 48/365
            // Reescalado mecánico daría 2.6; se calibra a 5.0 (≈1825 mm/año en el ecuador) para que
            // emerja la selva tropical, que necesita lluvia alta. Ver README de la fase.
            p_ref: 5.0,
            k_oro: 0.5,
            m_oro_min: 0.2,
            m_oro_max: 3.0,
            moisture_length_km: 1500.0,
            seas_precip_amp: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoilCfg {
    pub runoff_base: f64,
    pub k_slope: f64,
    pub k_sat: f64,
    pub k_et: f64,
    pub k_lat_flow: f64,
    pub w_cap_min: f64,
    pub w_cap_max: f64,
}

impl Default for SoilCfg {
    fn default() -> Self {
        Self {
            runoff_base: 0.10,
            k_slope: 0.5,
            k_sat: 0.4,
            k_et: 0.13,         // 1.0 mm/(°C·tick) · 48/365
            k_lat_flow: 0.0066, // 0.05 · 48/365
            w_cap_min: 50.0,
            w_cap_max: 300.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VegCfg {
    pub theta_opt: f64,
    pub sigma_theta: f64,
    pub t_opt: f64,
    pub sigma_t: f64,
    pub k_max: f64,
    pub r_veg: f64,
    pub n_veg: u64,
}

impl Default for VegCfg {
    fn default() -> Self {
        Self {
            theta_opt: 0.6,
            sigma_theta: 0.25,
            t_opt: 22.0,
            sigma_t: 12.0,
            k_max: 100.0,
            r_veg: 0.026, // 0.2 /tick · 48/365
            n_veg: 30,    // 4 ticks · 365/48
        }
    }
}

/// Destino de la salida (reutiliza el sink del motor: aquí solo JSON en este incremento).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SinkKind {
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PersistenceCfg {
    pub sink: SinkKind,
}

impl Default for PersistenceCfg {
    fn default() -> Self {
        Self {
            sink: SinkKind::Json,
        }
    }
}

impl TerrenoConfig {
    /// Carga y valida la configuración desde un archivo TOML.
    pub fn load(path: &str) -> Result<TerrenoConfig, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_string(),
            source,
        })?;
        let cfg: TerrenoConfig = toml::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_string(),
            source,
        })?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Número de celdas por arista de cara al nivel uniforme: `n0 · 2^l_unif`.
    pub fn face_n(&self) -> u32 {
        self.planet.n0 * (1u32 << self.planet.l_unif)
    }

    /// Comprueba invariantes que el TOML por sí solo no garantiza.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let invalid = |m: &str| ConfigError::Invalid(m.to_string());
        if self.planet.n0 == 0 {
            return Err(invalid("planet.n0 debe ser > 0"));
        }
        if self.planet.l_unif > 10 {
            return Err(invalid("planet.l_unif fuera de rango razonable (0..=10)"));
        }
        if self.planet.h_max <= self.planet.h_min {
            return Err(invalid("planet.h_max debe ser > planet.h_min"));
        }
        if self.climate.t_year < 24.0 {
            return Err(invalid(
                "climate.t_year debe ser >= 24 (anti-aliasing estacional, diseño §9)",
            ));
        }
        if self.soil.w_cap_max < self.soil.w_cap_min || self.soil.w_cap_min <= 0.0 {
            return Err(invalid("se requiere 0 < soil.w_cap_min <= soil.w_cap_max"));
        }
        if self.vegetation.n_veg == 0 {
            return Err(invalid("vegetation.n_veg debe ser >= 1"));
        }
        Ok(())
    }
}
