//! `abm-metrics` — observadores y agregación (CLAUDE §3): aplica el LOD de almacenamiento, es decir,
//! qué se registra y cada cuánto (`metrics_every`). Lee los componentes del modelo y escribe en el
//! destino de la persistencia.

pub mod biologia;
pub mod collect;
