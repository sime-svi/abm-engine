//! Errores de la persistencia (CLAUDE §5: `Result` + `thiserror`; nada de `panic!`).

use thiserror::Error;

/// Errores al escribir la salida (JSON o base de datos).
#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("error de E/S al escribir la salida")]
    Io(#[from] std::io::Error),
    #[error("error de serialización JSON")]
    Serialize(#[from] serde_json::Error),
    #[error("error de base de datos")]
    Db(#[from] postgres::Error),
    #[error("falta la variable de entorno {0} para conectar a la base de datos")]
    MissingEnv(String),
}
