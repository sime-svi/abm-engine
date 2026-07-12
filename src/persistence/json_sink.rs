//! Sink que acumula las métricas en memoria para volcarlas a JSON al final de la ejecución.

use crate::metrics::schema::TickMetrics;
use crate::persistence::sink::MetricsSink;

/// Acumulador en memoria de métricas por tick.
#[derive(Default)]
pub struct JsonSink {
    buffer: Vec<TickMetrics>,
}

impl JsonSink {
    /// Crea un sink vacío con capacidad reservada para `cap` ticks.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(cap),
        }
    }
}

impl MetricsSink for JsonSink {
    fn record(&mut self, metrics: TickMetrics) {
        self.buffer.push(metrics);
    }

    fn drain(&mut self) -> Vec<TickMetrics> {
        std::mem::take(&mut self.buffer)
    }
}
