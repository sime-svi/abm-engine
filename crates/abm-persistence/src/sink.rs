//! Trait del destino de métricas: la costura para cambiar JSON por PostgreSQL sin tocar el motor.

use crate::schema::TickMetrics;

/// Destino de las métricas por tick.
pub trait MetricsSink: Send + Sync {
    /// Registra las métricas de un tick.
    fn record(&mut self, metrics: TickMetrics);
    /// Extrae y vacía todo lo acumulado (para construir el documento final).
    fn drain(&mut self) -> Vec<TickMetrics>;
}
