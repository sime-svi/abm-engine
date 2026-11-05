//! El orden de los sistemas del tick: el modelo (CLAUDE §6). El motor (`abm-core`) da el planificador
//! de un solo hilo; aquí se fija el orden total con `.chain()`.

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ScheduleConfigs;
use bevy_ecs::system::ScheduleSystem;

use crate::systems;

/// Sistemas del modelo en orden fijo. Quien monta la simulación puede encadenar detrás sus
/// observadores (p. ej. las métricas).
pub fn model_systems() -> ScheduleConfigs<ScheduleSystem> {
    (
        systems::regrow::regrow,       // 1. regenera el recurso de cada celda
        systems::order::build_order,   // 2. orden canónico por AgentId
        systems::forage::forage,       // 3. mover + consumir + metabolizar/envejecer
        systems::reproduce::reproduce, // 4. intenciones de nacimiento (buffer)
        systems::mortality::mortality, // 5. intenciones de muerte (buffer)
        systems::apply::apply,         // 6. aplica muertes y nacimientos (exclusivo)
    )
        .chain()
}
