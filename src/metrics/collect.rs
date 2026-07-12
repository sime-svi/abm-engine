//! Paso 7 del tick: agregación de métricas macroscópicas y registro en el sink (LOD: `metrics_every`).

use bevy_ecs::prelude::*;

use crate::core::time::Tick;
use crate::domain::buffers::{SimBuffers, TickCounts};
use crate::domain::components::{Age, AgentId, Energy};
use crate::domain::grid::Grid;
use crate::domain::params::Params;
use crate::metrics::schema::TickMetrics;
use crate::persistence::Sink;

/// Agrega población, energía, edad y recurso del tick. Las sumas se hacen en `f64` y en orden
/// canónico de `AgentId` (celdas por índice) para que sean reproducibles entre plataformas.
pub fn collect_metrics(
    tick: Res<Tick>,
    params: Res<Params>,
    grid: Res<Grid>,
    counts: Res<TickCounts>,
    mut buf: ResMut<SimBuffers>,
    mut sink: ResMut<Sink>,
    q: Query<(&AgentId, &Energy, &Age)>,
) {
    let t = tick.0;
    if params.metrics_every == 0 || !t.is_multiple_of(params.metrics_every) {
        return;
    }

    // Recolectar y ordenar por AgentId para una suma determinista.
    buf.metrics_scratch.clear();
    for (id, energy, age) in &q {
        buf.metrics_scratch.push((id.0, energy.0, age.0));
    }
    buf.metrics_scratch.sort_unstable_by_key(|&(id, _, _)| id);

    let population = buf.metrics_scratch.len() as u64;
    let mut energy_total = 0.0f64;
    let mut age_total = 0.0f64;
    for &(_, energy, age) in buf.metrics_scratch.iter() {
        energy_total += energy as f64;
        age_total += age as f64;
    }
    let resource_total: f64 = grid.resource.iter().map(|&r| r as f64).sum();

    let (energy_mean, age_mean) = if population > 0 {
        let p = population as f64;
        (energy_total / p, age_total / p)
    } else {
        (0.0, 0.0)
    };

    sink.0.record(TickMetrics {
        tick: t,
        population,
        births: counts.births,
        deaths: counts.deaths,
        energy_total,
        energy_mean,
        age_mean,
        resource_total,
    });
}
