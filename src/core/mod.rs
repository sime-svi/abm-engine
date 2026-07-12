//! El motor: tiempo, RNG, orquestación de sistemas y génesis del mundo. Agnóstico al dominio
//! (CLAUDE §3: nada aquí depende de las reglas). Futuro crate `abm-core`.

pub mod rng;
pub mod schedule;
pub mod time;
pub mod world_setup;
