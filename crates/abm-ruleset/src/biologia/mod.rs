//! Biología de los agentes: energía, crecimiento, reproducción sexual y mortalidad de individuos
//! definidos por su masa (diseño del responsable: `docs/design/agentes/diseno_biologia_agentes.md`).
//!
//! Incremento 1 (`docs/design/agentes/incremento-1-diseno.md`): individuos sobre un hábitat de
//! prueba con clima constante, sin planeta ni modo población.

pub mod azar;
pub mod componentes;
pub mod config;
pub mod especie;
pub mod genesis;
pub mod habitat;
pub mod recursos;
pub mod schedule;
pub mod sistemas;

pub use config::BioConfig;
