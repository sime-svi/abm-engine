//! Errores del terreno (CLAUDE §5: `Result` + `thiserror`; nada de `panic!`).

use abm_core::ConfigError;
use thiserror::Error;

/// Errores al preparar o correr una simulación de terreno.
#[derive(Debug, Error)]
pub enum TerrenoError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("error de E/S (caché de la génesis)")]
    Io(#[from] std::io::Error),
    #[error("error de serialización JSON")]
    Serialize(#[from] serde_json::Error),
    /// Modo Tierra: una fuente que falta o que no tiene el formato esperado.
    #[error("modo Tierra: {0}")]
    Tierra(String),
}
