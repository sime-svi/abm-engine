//! Construcción del `Schedule`: el corazón del determinismo (CLAUDE §6).
//!
//! Un `Schedule` es una lista ordenada de sistemas ("system" = función que opera sobre el mundo).
//! Dos garantías blindan el orden frente a cualquier paralelismo:
//!   1. Executor de un solo hilo (`SingleThreadedExecutor`).
//!   2. Orden total explícito con `.chain()` — el orden de los sistemas *es* el modelo.
//!
//! Además, la feature `multi_threaded` de `bevy_ecs` no está compilada (ver `Cargo.toml`).

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{ScheduleLabel, SingleThreadedExecutor};

use crate::domain::systems;
use crate::metrics::collect;

/// Etiqueta del schedule de simulación.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimSchedule;

/// Construye el schedule con los sistemas del tick en orden fijo.
pub fn build() -> Schedule {
    let mut schedule = Schedule::new(SimSchedule);
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(
        (
            systems::regrow::regrow,       // 1. regenera el recurso de cada celda
            systems::order::build_order,   // 2. orden canónico por AgentId
            systems::forage::forage,       // 3. mover + consumir + metabolizar/envejecer
            systems::reproduce::reproduce, // 4. intenciones de nacimiento (buffer)
            systems::mortality::mortality, // 5. intenciones de muerte (buffer)
            systems::apply::apply,         // 6. aplica muertes y nacimientos (exclusivo)
            collect::collect_metrics,      // 7. agrega métricas del tick
        )
            .chain(),
    );
    schedule
}
