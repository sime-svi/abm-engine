//! Generador de números pseudoaleatorios determinista y sembrado (CLAUDE §6).
//!
//! `ChaCha8Rng` produce la misma secuencia entre versiones y plataformas dada la misma semilla, a
//! diferencia de `StdRng`. Es la única fuente de aleatoriedad de la simulación.

use bevy_ecs::prelude::*;
use rand_chacha::ChaCha8Rng;

/// RNG raíz de la ejecución. En este hito se usa solo en la génesis del mundo (posiciones y
/// energías iniciales); el bucle de ticks no sortea nada (desempate por orden fijo). Se conserva
/// como recurso del mundo —con su estado ya avanzado— para los sistemas estocásticos de fases
/// posteriores (p. ej. desempate aleatorio). Aún sin lectores en el bucle: de ahí el `allow`.
#[derive(Resource)]
#[allow(dead_code)]
pub struct RootRng(pub ChaCha8Rng);
