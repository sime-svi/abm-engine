# Fase «persistencia» — diseño técnico

> El **esquema** canónico vive en `DATABASE_SCHEMA.md` (raíz). Aquí se recogen las **decisiones de
> implementación** propias de esta fase. La explicación en lenguaje llano está en `README.md`.

## Objetivo

Conectar el motor a la BD ya existente (PostgreSQL 16 + TimescaleDB) para persistir las métricas
agregadas por tick, consultables por SQL y visualizables en Grafana. Alcance: tablas
`simulation_runs` + `metrics`. `events` y `agent_snapshots` son fases posteriores.

## Decisiones

- **Cliente `postgres` síncrono** (no `sqlx`). La BD es *salida*: no toca el determinismo del cálculo.
  Un cliente síncrono evita arrastrar un runtime asíncrono a un motor determinista y sencillo.
- **Selección por config**: `[persistence] sink = "json" | "postgres" | "both"`. Por defecto `json`,
  para que los tests de determinismo sigan siendo herméticos (sin BD).
- **Fan-out en el runner** (`main.rs`), no en el bucle: `run_simulation` siempre produce el
  `MetricsDoc`; el runner decide destino(s). El motor no sabe de bases de datos.
- **Conexión desde el entorno** (CLAUDE §9): `DATABASE_URL`, o si no las `POSTGRES_*` de `infra/.env`.
  Nunca en el código ni en la config versionada.
- **Migraciones aplicadas por el propio binario** (`persistence::migrations`), embebidas con
  `include_str!` y registradas en `schema_migrations`. No se depende de `psql` (no está instalado).
- **Escritura atómica**: `simulation_runs` (una fila) + `metrics` (carga masiva con `COPY`) en una
  única transacción. `run_id` es un UUID por ejecución; `started_at/finished_at` los pone la BD
  (`now()`), así no entra reloj de pared en el código Rust.

## Flujo

```
run_simulation(cfg) -> MetricsDoc          (determinista, en memoria)
main.rs, según cfg.persistence.sink:
   json          -> output/<name>.json
   postgres/both -> pg_sink::write_run:
       conecta (entorno) -> migrations::run -> BEGIN
           INSERT simulation_runs (status=finished, config JSONB, versiones, seed)
           COPY   metrics (7 filas por tick, formato largo)
       COMMIT -> run_id
```

## Mapeo de métricas (formato largo)

Cada `TickMetrics` se expande a 7 filas `(run_id, tick, metric_name, value)`:
`population`, `births`, `deaths`, `energy_total`, `energy_mean`, `age_mean`, `resource_total`.

## Fuera de alcance (fases siguientes)

`agent_dim`, `events`, `agent_snapshots` (LOD micro), estado `running`→`finished` en streaming,
compresión / *continuous aggregates* de TimescaleDB, y dashboards de Grafana aprovisionados.
