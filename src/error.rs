//! Tipos de error de la librería (CLAUDE §5: `Result` + `thiserror`; nada de `panic!`).

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

/// Errores de la simulación / persistencia.
#[derive(Debug, Error)]
pub enum SimError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("error de E/S al escribir la salida")]
    Io(#[from] std::io::Error),
    #[error("error de serialización JSON")]
    Serialize(#[from] serde_json::Error),
    #[error("error de base de datos")]
    Db(#[from] postgres::Error),
    #[error("falta la variable de entorno {0} para conectar a la base de datos")]
    MissingEnv(String),
}
