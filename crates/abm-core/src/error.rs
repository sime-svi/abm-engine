//! Errores comunes del motor (CLAUDE §5: `Result` + `thiserror`; nada de `panic!`): los de cargar y
//! validar una configuración, que comparten todos los paquetes.

use thiserror::Error;

/// Errores al cargar o validar la configuración.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("no se pudo leer el archivo de configuración '{path}'")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("error de sintaxis TOML en '{path}'")]
    Parse {
        path: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("configuración inválida: {0}")]
    Invalid(String),
    #[error("opción no soportada en este hito: {0}")]
    Unsupported(String),
}
