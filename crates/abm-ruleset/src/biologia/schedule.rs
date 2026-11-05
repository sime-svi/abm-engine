//! Orden de los sistemas de la biología en el tick (diseño §9; incremento 1 §6). El orden *es* el
//! modelo: cada paso lee lo que dejó el anterior.

use abm_core::time::Programa;

use super::sistemas::*;

/// Sistemas de un tick, en orden (un solo hilo, orden fijo), cada uno con el nombre con el que sale
/// en el informe de tiempos. El que monta la corrida añade detrás sus observadores (métricas).
pub fn bio_programa() -> Programa {
    Programa::nuevo()
        .etapa("inicio", bio_inicio) // libro a cero, reservas de partida
        .etapa("forraje", bio_forraje) // T4. rebrote del forraje
        // A1 (conversión individuo ↔ población) y A2 (comportamiento): incrementos siguientes.
        .etapa("demanda", bio_demanda) // A3. gasto del día, demanda y reparto justo
        .etapa("energetica", bio_energetica) // A4. ingesta, leche, reservas
        .etapa("crecimiento", bio_crecimiento) // A5. crecimiento
        .etapa("reproduccion", bio_reproduccion) // A6. madurez, aborto, parto, destete, concepción
        .etapa("nacimientos", bio_nacimientos) //     las crías del tick (exclusivo)
        .etapa("mortalidad", bio_mortalidad) // A7. inanición, edad máxima, Siler, orfandad
        .etapa("aplicar_muertes", bio_aplicar_muertes) //     retira a los muertos (exclusivo)
        .etapa("envejecimiento", bio_envejecimiento) // A8. edad + 1, fin de la dependencia
        .etapa("escrituras", bio_escrituras) // A9. el consumo se descuenta del forraje
}
