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
    pub lod: LodCfg,
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
    pub sea_level: f64,
    pub h_min: f64,
    pub h_max: f64,
}

impl Default for PlanetCfg {
    fn default() -> Self {
        Self {
            r_km: 6371.0,
            n0: 48,
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
    /// Octavas de ruido que se añaden por cada nivel por encima de `lod.l_min` (incremento 2 §3.1):
    /// con lacunaridad 2, una por nivel mantiene la razón tamaño de celda / detalle más fino.
    pub octaves_per_level: u32,
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
            octaves_per_level: 1,
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
    /// Biomasa inicial de la tierra (génesis) y de la tierra nueva que aparece al afinar la costa.
    pub v_init: f64,
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
            v_init: 1.0,
        }
    }
}

/// LOD dinámico (incremento 2, `docs/design/terreno/incremento-2-diseno.md` §8 y §11).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LodCfg {
    /// Nivel base uniforme de arranque (sustituye a `planet.l_unif` del incremento 1).
    pub l_min: u32,
    /// Nivel máximo de subdivisión. `l_max = l_min` => LOD apagado (malla uniforme).
    pub l_max: u32,
    /// Cadencia del paso de topología (ticks).
    pub n_lod: u64,
    /// Variación de altura con algún vecino de tierra que dispara subdividir (m).
    pub dh_sub: f64,
    /// Variación de temperatura con algún vecino de tierra que dispara subdividir (°C).
    pub dt_sub: f64,
    /// Variación de humedad relativa del suelo con algún vecino de tierra que dispara subdividir.
    pub dtheta_sub: f64,
    /// Histéresis: solo se colapsa si las variaciones bajan de `h_hist · umbral` (0 < h_hist ≤ 1).
    pub h_hist: f64,
    /// Refinar los rasgos geográficos: costa, ríos y lagos.
    pub refine_features: bool,
}

impl Default for LodCfg {
    fn default() -> Self {
        Self {
            l_min: 0,
            l_max: 3,
            n_lod: 60,
            dh_sub: 500.0,
            dt_sub: 4.0,
            dtheta_sub: 0.3,
            h_hist: 0.5,
            refine_features: true,
        }
    }
}

impl LodCfg {
    /// `true` si el LOD dinámico está activo (hay margen para subdividir).
    pub fn enabled(&self) -> bool {
        self.l_max > self.l_min
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

    /// Número de celdas por arista de cara al nivel base: `n0 · 2^l_min`.
    pub fn face_n(&self) -> u32 {
        self.planet.n0 << self.lod.l_min
    }

    /// Comprueba invariantes que el TOML por sí solo no garantiza.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let invalid = |m: &str| ConfigError::Invalid(m.to_string());
        if self.planet.n0 == 0 {
            return Err(invalid("planet.n0 debe ser > 0"));
        }
        if self.lod.l_min > self.lod.l_max {
            return Err(invalid("se requiere lod.l_min <= lod.l_max"));
        }
        if self.lod.l_max > 12 {
            return Err(invalid("lod.l_max fuera de rango razonable (0..=12)"));
        }
        // La clave de celda reserva 28 bits por coordenada y la vecindad mira un nivel más (§2).
        if (self.planet.n0 as u64) << (self.lod.l_max + 1) > 1u64 << 28 {
            return Err(invalid("planet.n0 · 2^(lod.l_max+1) no cabe en 28 bits"));
        }
        if self.lod.n_lod == 0 {
            return Err(invalid("lod.n_lod debe ser >= 1"));
        }
        if !(self.lod.h_hist > 0.0 && self.lod.h_hist <= 1.0) {
            return Err(invalid("se requiere 0 < lod.h_hist <= 1"));
        }
        if !(self.lod.dh_sub > 0.0 && self.lod.dt_sub > 0.0 && self.lod.dtheta_sub > 0.0) {
            return Err(invalid(
                "los umbrales lod.dh_sub, lod.dt_sub y lod.dtheta_sub deben ser > 0",
            ));
        }
        if self.vegetation.v_init < 0.0 {
            return Err(invalid("vegetation.v_init debe ser >= 0"));
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
