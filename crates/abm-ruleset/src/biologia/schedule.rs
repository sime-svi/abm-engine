//! Orden de los sistemas de la biología en el tick (diseño §9; incremento 1 §6). El orden *es* el
//! modelo: cada paso lee lo que dejó el anterior.

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ScheduleConfigs;
use bevy_ecs::system::ScheduleSystem;

use super::sistemas::*;

/// Sistemas de un tick, encadenados (un solo hilo, orden fijo).
pub fn bio_systems() -> ScheduleConfigs<ScheduleSystem> {
    (
        bio_inicio,  // libro a cero, reservas de partida
        bio_forraje, // T4. rebrote del forraje
        // A1 (conversión individuo ↔ población) y A2 (comportamiento): incrementos siguientes.
        bio_demanda,         // A3. gasto del día, demanda y reparto justo
        bio_energetica,      // A4. ingesta, leche, reservas
        bio_crecimiento,     // A5. crecimiento
        bio_reproduccion,    // A6. madurez, aborto, parto, destete, concepción
        bio_nacimientos,     //     las crías del tick (exclusivo)
        bio_mortalidad,      // A7. inanición, edad máxima, Siler, orfandad
        bio_aplicar_muertes, //     retira a los muertos (exclusivo)
        bio_envejecimiento,  // A8. edad + 1, fin de la dependencia
        bio_escrituras,      // A9. el consumo se descuenta del forraje
    )
        .chain()
}
