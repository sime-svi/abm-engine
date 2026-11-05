//! `abm-core` — el motor (CLAUDE §3): tiempo (tick), RNG determinista sembrado, planificador de un
//! solo hilo y errores de configuración. Agnóstico al dominio: no depende de ningún otro paquete del
//! proyecto, y todos dependen de él.

pub mod error;
pub mod rng;
pub mod schedule;
pub mod time;

pub use error::ConfigError;
