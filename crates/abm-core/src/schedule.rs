//! Planificador determinista: el corazón del determinismo (CLAUDE §6).
//!
//! Un `Schedule` es una lista ordenada de sistemas ("system" = función que opera sobre el mundo).
//! Dos garantías blindan el orden frente a cualquier paralelismo:
//!   1. Executor de un solo hilo (`SingleThreadedExecutor`), que fija este módulo.
//!   2. Orden total explícito con `.chain()`, que pone quien añade los sistemas: el orden de los
//!      sistemas *es* el modelo, y por eso lo define cada ruleset, no el motor.
//!
//! Además, la feature `multi_threaded` de `bevy_ecs` no está compilada (ver `Cargo.toml`).

use bevy_ecs::schedule::{Schedule, ScheduleLabel, SingleThreadedExecutor};

/// Etiqueta del schedule de simulación.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimSchedule;

/// Schedule vacío de un solo hilo, listo para recibir los sistemas del modelo en orden total.
pub fn deterministic() -> Schedule {
    let mut schedule = Schedule::new(SimSchedule);
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule
}
