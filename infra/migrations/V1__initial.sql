-- V1: esquema inicial de persistencia.
-- Crea el registro de ejecuciones y la serie temporal de métricas agregadas.
-- Ver DATABASE_SCHEMA.md. Aplicada por el propio binario (persistence::migrations).

CREATE EXTENSION IF NOT EXISTS timescaledb;

-- Registro de ejecuciones: una fila por corrida del motor. Tabla raíz; todo cuelga de run_id.
CREATE TABLE IF NOT EXISTS simulation_runs (
    run_id          UUID PRIMARY KEY,
    name            TEXT        NOT NULL,
    seed            BIGINT      NOT NULL,
    engine_version  TEXT        NOT NULL,
    ruleset_version TEXT        NOT NULL,
    config          JSONB       NOT NULL,
    status          TEXT        NOT NULL,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at     TIMESTAMPTZ,
    total_ticks     BIGINT,
    notes           TEXT
);

-- Métricas agregadas por tick, en formato largo (long). Hypertable particionada por 'tick'.
CREATE TABLE IF NOT EXISTS metrics (
    run_id      UUID             NOT NULL REFERENCES simulation_runs (run_id),
    tick        BIGINT           NOT NULL,
    metric_name TEXT             NOT NULL,
    value       DOUBLE PRECISION NOT NULL,
    dimension   TEXT
);

-- Convierte 'metrics' en hypertable de TimescaleDB (partición por rango de ticks).
SELECT create_hypertable('metrics', 'tick', chunk_time_interval => 100000, if_not_exists => TRUE);

-- Índice para las consultas típicas de Grafana: (run_id, metric_name, tick).
CREATE INDEX IF NOT EXISTS idx_metrics_run_metric_tick ON metrics (run_id, metric_name, tick);
