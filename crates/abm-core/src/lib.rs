//! `abm-core` — el motor (CLAUDE §3): tiempo (tick), RNG determinista sembrado, planificador de un
//! solo hilo, carga de la configuración con ajustes, cronómetro de rendimiento y errores de
//! configuración. Agnóstico al dominio: no depende de ningún otro paquete del
//! proyecto, y todos dependen de él.

pub mod config;
pub mod cronometro;
pub mod error;
pub mod rng;
pub mod schedule;
pub mod time;

pub use error::ConfigError;
