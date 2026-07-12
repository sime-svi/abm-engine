//! Componentes del dominio (datos por agente). `bevy_ecs` los almacena por columnas (Struct of
//! Arrays) automáticamente, cumpliendo la orientación a datos de CLAUDE §5.

use bevy_ecs::prelude::*;

/// Posición del agente en la cuadrícula.
#[derive(Component, Debug, Clone, Copy)]
pub struct Position {
    pub x: u32,
    pub y: u32,
}

/// Energía del agente.
#[derive(Component, Debug, Clone, Copy)]
pub struct Energy(pub f32);

/// Edad del agente en ticks vividos.
#[derive(Component, Debug, Clone, Copy)]
pub struct Age(pub u32);

/// Identificador monotónico y único del agente. Fija el orden canónico de actualización y nunca se
/// reutiliza, lo que desacopla el modelo del layout interno del ECS.
#[derive(Component, Debug, Clone, Copy)]
pub struct AgentId(pub u64);

/// Asignador del siguiente `AgentId` (recurso global).
#[derive(Resource, Debug, Default)]
pub struct AgentCounter(pub u64);
