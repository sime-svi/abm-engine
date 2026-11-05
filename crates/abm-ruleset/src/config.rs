//! Configuración de la simulación (CLAUDE §5: todo parámetro vive en `config/*.toml`).
//!
//! Cada sección tiene valores por defecto en código (`Default`) y se valida al cargar. El TOML
//! puede omitir claves (se rellenan con el default) pero no puede traer claves desconocidas.

use serde::{Deserialize, Serialize};

use abm_core::ConfigError;

/// Configuración completa de una ejecución.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub run: RunConfig,
    pub grid: GridConfig,
    pub resource: ResourceConfig,
    pub agents: AgentsConfig,
    pub reproduction: ReproductionConfig,
    pub movement: MovementConfig,
    pub metrics: MetricsConfig,
    pub persistence: PersistenceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RunConfig {
    pub name: String,
    pub seed: u64,
    pub total_ticks: u64,
    pub output_path: String,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            name: "baseline".to_string(),
            seed: 42,
            total_ticks: 200,
            output_path: "output".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GridConfig {
    pub width: u32,
    pub height: u32,
    pub toroidal: bool,
    pub neighborhood: u8,
    pub single_occupancy: bool,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            width: 100,
            height: 100,
            toroidal: true,
            neighborhood: 8,
            single_occupancy: true,
        }
    }
}

/// Cómo se rellena el recurso de las celdas al inicio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InitialFill {
    /// Todas las celdas al máximo.
    Full,
    /// Valor uniforme en `[0, max]` con el RNG sembrado.
    Random,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ResourceConfig {
    pub max: f32,
    pub regrowth: f32,
    pub initial_fill: InitialFill,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            max: 8.0,
            regrowth: 1.0,
            initial_fill: InitialFill::Full,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AgentsConfig {
    pub initial_count: u64,
    pub init_energy_min: f32,
    pub init_energy_max: f32,
    pub appetite: f32,
    pub metabolism: f32,
    pub max_age: u32,
}

impl Default for AgentsConfig {
    fn default() -> Self {
        Self {
            initial_count: 300,
            init_energy_min: 5.0,
            init_energy_max: 25.0,
            appetite: 4.0,
            metabolism: 1.0,
            max_age: 200,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReproductionConfig {
    pub enabled: bool,
    pub energy_threshold: f32,
    pub child_energy_fraction: f32,
}

impl Default for ReproductionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            energy_threshold: 20.0,
            child_energy_fraction: 0.5,
        }
    }
}

/// Regla de desempate al elegir celda de destino en el movimiento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tiebreak {
    /// Orden de offset fijo (celda actual primero); sin sorteo en el bucle.
    FixedOrder,
    /// Sorteo con el RNG sembrado. Reservado para una fase posterior.
    Rng,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MovementConfig {
    pub tiebreak: Tiebreak,
}

impl Default for MovementConfig {
    fn default() -> Self {
        Self {
            tiebreak: Tiebreak::FixedOrder,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MetricsConfig {
    pub metrics_every: u64,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self { metrics_every: 1 }
    }
}

/// Destino de la salida de métricas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SinkKind {
    /// Volcado a JSON en `output/` (por defecto; no necesita BD).
    Json,
    /// Escritura en PostgreSQL/TimescaleDB (conexión tomada del entorno).
    Postgres,
    /// Ambos destinos.
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PersistenceConfig {
    pub sink: SinkKind,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            sink: SinkKind::Json,
        }
    }
}

impl Config {
    /// Carga y valida la configuración desde un archivo TOML.
    pub fn load(path: &str) -> Result<Config, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_string(),
            source,
        })?;
        let cfg: Config = toml::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_string(),
            source,
        })?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Comprueba invariantes que el TOML por sí solo no garantiza.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let invalid = |m: &str| ConfigError::Invalid(m.to_string());

        if self.grid.width == 0 || self.grid.height == 0 {
            return Err(invalid("grid.width y grid.height deben ser > 0"));
        }
        if self.grid.neighborhood != 4 && self.grid.neighborhood != 8 {
            return Err(invalid(
                "grid.neighborhood debe ser 4 (Von Neumann) u 8 (Moore)",
            ));
        }
        let cells = self.grid.width as u64 * self.grid.height as u64;
        if self.agents.initial_count > cells {
            return Err(invalid(
                "agents.initial_count no puede superar el número de celdas (width*height)",
            ));
        }
        if self.resource.max < 0.0 || self.resource.regrowth < 0.0 {
            return Err(invalid("resource.max y resource.regrowth deben ser >= 0"));
        }
        if self.agents.appetite < 0.0 || self.agents.metabolism < 0.0 {
            return Err(invalid(
                "agents.appetite y agents.metabolism deben ser >= 0",
            ));
        }
        if self.agents.init_energy_min < 0.0
            || self.agents.init_energy_max < self.agents.init_energy_min
        {
            return Err(invalid(
                "se requiere 0 <= agents.init_energy_min <= agents.init_energy_max",
            ));
        }
        if !(0.0..=1.0).contains(&self.reproduction.child_energy_fraction) {
            return Err(invalid(
                "reproduction.child_energy_fraction debe estar en [0, 1]",
            ));
        }
        if self.movement.tiebreak == Tiebreak::Rng {
            return Err(ConfigError::Unsupported(
                "movement.tiebreak = \"rng\" (usa \"fixed_order\" en este hito)".to_string(),
            ));
        }
        Ok(())
    }
}
