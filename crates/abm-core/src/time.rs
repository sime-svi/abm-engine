//! Tiempo de simulación: el `tick`, un entero. Nunca el reloj de pared (CLAUDE §6).

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::system::ScheduleSystem;

use crate::cronometro::Cronometro;

/// Tick actual de la simulación (recurso global del mundo).
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct Tick(pub u64);

/// El bucle de tiempo: corre `ticks` pasos del `schedule` sobre el `world`, actualizando antes de
/// cada uno el recurso [`Tick`] (0, 1, 2…). Requiere que el mundo tenga el recurso `Tick`.
pub fn run_ticks(world: &mut World, schedule: &mut Schedule, ticks: u64) {
    for t in 0..ticks {
        world.resource_mut::<Tick>().0 = t;
        schedule.run(world);
    }
}

/// Los sistemas de un tick, en orden, cada uno en su propio `Schedule` de un solo hilo, para poder
/// medir cuánto tarda cada uno (informe de tiempos; el reloj no decide nada, CLAUDE §6).
///
/// Correr las etapas una tras otra equivale a encadenarlas con `.chain()` en un único `Schedule`:
/// mismo orden total y, al final de cada etapa, se aplican sus órdenes diferidas (`Commands`), como
/// en los puntos de sincronización que `.chain()` pone entre sistemas.
pub struct Programa {
    etapas: Vec<(&'static str, Schedule)>,
    cronometro: Cronometro,
}

impl Default for Programa {
    fn default() -> Self {
        Self::nuevo()
    }
}

impl Programa {
    /// Programa vacío.
    pub fn nuevo() -> Self {
        Self {
            etapas: Vec::new(),
            cronometro: Cronometro::default(),
        }
    }

    /// Añade al final la etapa `nombre`, con sus sistemas (en el orden que ya traigan).
    pub fn etapa<M>(
        mut self,
        nombre: &'static str,
        sistemas: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> Self {
        let mut schedule = crate::schedule::deterministic();
        schedule.add_systems(sistemas);
        self.etapas.push((nombre, schedule));
        self
    }

    /// El bucle de tiempo: corre `ticks` pasos de todas las etapas, en orden, sobre el `world`,
    /// poniendo antes de cada paso el recurso [`Tick`] (0, 1, 2…).
    pub fn correr(&mut self, world: &mut World, ticks: u64) {
        for t in 0..ticks {
            world.resource_mut::<Tick>().0 = t;
            self.cronometro.marcar();
            for (nombre, schedule) in &mut self.etapas {
                schedule.run(world);
                self.cronometro.anotar(nombre);
            }
        }
    }

    /// Tiempos medidos por etapa.
    pub fn cronometro(&self) -> &Cronometro {
        &self.cronometro
    }
}
