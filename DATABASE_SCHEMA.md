# DATABASE_SCHEMA.md — Arquitectura de la base de datos

> Estado: **DISEÑO** (aún no implementado). Motor: PostgreSQL 16 + extensión TimescaleDB.
> Este documento describe el esquema objetivo. La implementación se hace mediante migraciones versionadas en `infra/migrations/`, nunca a mano. Mantener este archivo sincronizado con el esquema real es obligatorio (ver CLAUDE.md §8).

---

## 1. Principios de diseño

La ingenuidad que mata a una BD de simulación es intentar guardar el estado de cada agente en cada tick. Con millones de agentes y miles de ticks, eso son miles de millones de filas por ejecución: explosión combinatoria en el almacenamiento, exactamente lo que el LOD evita en el cómputo. El diseño se apoya en cuatro ideas:

1. **Separar tres tipos de dato con costes muy distintos:**
   - **Métricas agregadas** (macro): baratas, se registran en *cada* tick. Son la columna vertebral de la visualización.
   - **Eventos discretos**: se registran solo cuando ocurren (nacimiento, muerte, conflicto, infección). Volumen moderado.
   - **Snapshots de agente** (micro, detalle alto): caros. Se registran solo a intervalos, o para una muestra de agentes, o cuando un agente está "bajo observación". Aquí se aplica el LOD de almacenamiento.

2. **Reconstruir en vez de almacenar todo.** El estado en un tick arbitrario se reconstruye desde el último snapshot + la reproducción de los eventos posteriores. No hace falta persistir cada estado intermedio.

3. **Multi-ejecución de primera clase.** Toda fila cuelga de un `run_id`. La BD guarda muchas ejecuciones/experimentos en paralelo, comparables entre sí. Esencial para barridos de parámetros.

4. **TimescaleDB para las tablas de serie temporal.** `metrics`, `events` y `agent_snapshots` son *hypertables* particionadas por tiempo (el tick), lo que da inserción rápida, compresión y agregados continuos.

---

## 2. Tablas

### 2.1. `simulation_runs` — registro de ejecuciones

Una fila por ejecución del motor. Es la tabla raíz; todo lo demás la referencia.

| Columna | Tipo | Descripción |
|---|---|---|
| `run_id` | `UUID` PK | Identificador único de la ejecución |
| `name` | `TEXT` | Nombre legible del experimento |
| `seed` | `BIGINT` | Semilla del RNG raíz. Con esto + versiones, la ejecución es reproducible |
| `engine_version` | `TEXT` | Versión del crate `abm-core` |
| `ruleset_version` | `TEXT` | Versión del crate `abm-ruleset` |
| `config` | `JSONB` | Copia íntegra de la configuración usada (parámetros del `.toml`) |
| `status` | `TEXT` | `running` / `finished` / `failed` / `aborted` |
| `started_at` | `TIMESTAMPTZ` | Momento real de inicio (metadato, no lógica de simulación) |
| `finished_at` | `TIMESTAMPTZ` | Momento real de fin |
| `total_ticks` | `BIGINT` | Número de ticks simulados |
| `notes` | `TEXT` | Anotaciones libres del responsable |

Guardar `config` como JSONB permite reproducir y comparar experimentos sin una tabla de parámetros rígida.

### 2.2. `agent_dim` — dimensión de agentes

Una fila por agente y ejecución. Tabla de dimensión (ligera) para consultas de ciclo de vida. No guarda estado por tick.

| Columna | Tipo | Descripción |
|---|---|---|
| `run_id` | `UUID` FK | Ejecución a la que pertenece |
| `agent_id` | `BIGINT` | Id del agente dentro de la ejecución |
| `agent_type` | `TEXT` | Tipo/especie del agente |
| `birth_tick` | `BIGINT` | Tick de creación |
| `death_tick` | `BIGINT` NULL | Tick de muerte (NULL si sigue vivo al final) |
| `parent_id` | `BIGINT` NULL | Linaje, para análisis genealógicos/genéticos |

PK compuesta `(run_id, agent_id)`.

### 2.3. `metrics` — series temporales agregadas (hypertable)

El corazón de la visualización. Formato largo (long) para máxima flexibilidad: añadir una métrica nueva no requiere alterar el esquema.

| Columna | Tipo | Descripción |
|---|---|---|
| `run_id` | `UUID` FK | Ejecución |
| `tick` | `BIGINT` | Tiempo de simulación (clave de partición) |
| `metric_name` | `TEXT` | Nombre de la métrica (ej. `population_total`, `avg_resource`, `deaths`) |
| `value` | `DOUBLE PRECISION` | Valor |
| `dimension` | `TEXT` NULL | Desglose opcional (ej. por región, por tipo de agente) |

Hypertable particionada por `tick`. Índice sugerido: `(run_id, metric_name, tick)`. Es la tabla que Grafana consulta directamente.

### 2.4. `events` — eventos discretos (hypertable)

Sucesos puntuales del modelo. Volumen moderado; base para la reproducción de estado.

| Columna | Tipo | Descripción |
|---|---|---|
| `run_id` | `UUID` FK | Ejecución |
| `tick` | `BIGINT` | Tick en que ocurre (clave de partición) |
| `event_type` | `TEXT` | Tipo (`birth`, `death`, `conflict`, `infection`, `trade`, ...) |
| `agent_id` | `BIGINT` NULL | Agente protagonista, si aplica |
| `target_id` | `BIGINT` NULL | Segundo agente implicado (ej. en un conflicto o comercio) |
| `payload` | `JSONB` | Datos específicos del evento (flexibles por tipo) |

Hypertable particionada por `tick`. Índices sugeridos: `(run_id, event_type, tick)` y `(run_id, agent_id)`.

### 2.5. `agent_snapshots` — estado detallado muestreado (hypertable)

El dato caro. Estado completo de un agente en un tick. Se puebla con LOD: solo en ticks de snapshot (ej. cada N ticks), o para una muestra de agentes, o para agentes marcados como "en observación". **Nunca todos los agentes en todos los ticks.**

| Columna | Tipo | Descripción |
|---|---|---|
| `run_id` | `UUID` FK | Ejecución |
| `tick` | `BIGINT` | Tick del snapshot (clave de partición) |
| `agent_id` | `BIGINT` | Agente |
| `pos_x` | `INTEGER` | Posición en la cuadrícula |
| `pos_y` | `INTEGER` | Posición en la cuadrícula |
| `state` | `SMALLINT` | Estado discreto (enum: buscando/consumiendo/reproduciéndose/...) |
| `components` | `JSONB` | Componentes ECS relevantes serializados (salud, energía, genética, ...) |

Hypertable particionada por `tick`. Usar `JSONB` para `components` permite que el ruleset evolucione (nuevos componentes) sin migraciones constantes; los campos que se consultan mucho se promueven a columnas tipadas cuando el rendimiento lo pida.

### 2.6. (Opcional, fase madura) `world_snapshots`

Estado global de la cuadrícula/entorno (mapas de recursos, clima) a intervalos, para reconstruir el mundo y no solo los agentes. Estructura análoga: `(run_id, tick, layer, data JSONB o BYTEA)`.

---

## 3. Estrategia de LOD en almacenamiento

Parámetros que controlan cuánto se guarda (definidos en `config/*.toml`, no en código):

- `metrics_every`: cada cuántos ticks se agregan métricas (por defecto 1 — siempre).
- `snapshot_every`: cada cuántos ticks se toma snapshot de agentes (ej. 100).
- `snapshot_sample`: fracción de agentes muestreados por snapshot (ej. 0.01 = 1 %), o lista de agentes "en observación".
- `events_enabled`: conjunto de tipos de evento que se persisten.

Esto permite pasar de una ejecución exploratoria y barata (solo métricas) a una ejecución forense y cara (snapshots densos) cambiando solo la config, sin tocar el motor.

---

## 4. Índices, compresión y retención

- Hypertables con `chunk_time_interval` ajustado al rango típico de ticks por partición.
- Compresión de TimescaleDB en chunks antiguos de `metrics` y `agent_snapshots` (los datos históricos rara vez se reescriben).
- Agregados continuos (continuous aggregates) para vistas materializadas de métricas por rangos (ej. media por cada 1000 ticks), que aceleran los dashboards.
- Retención opcional: descartar `agent_snapshots` de ejecuciones exploratorias antiguas conservando sus `metrics`.

---

## 5. Versionado del esquema

- Todo cambio de esquema es una **migración** en `infra/migrations/`, numerada y ordenada.
- La tabla `schema_migrations` (gestionada por la herramienta de migración, ej. `sqlx migrate` o `refinery`) registra qué migraciones se han aplicado.
- Este documento se actualiza en el **mismo commit** que la migración que lo altera. Documento y esquema real nunca divergen.

---

## 6. Flujo de datos (resumen)

```
abm-runner (ejecución)
   │  crea fila en simulation_runs (status=running)
   ▼
por cada tick:
   ├─ abm-metrics agrega  ──▶ INSERT en metrics
   ├─ eventos del ruleset ──▶ INSERT en events
   └─ si toca snapshot     ──▶ INSERT en agent_snapshots (con LOD)
   ▼
al terminar: UPDATE simulation_runs (status=finished, total_ticks, finished_at)
   ▼
Grafana / web app  ◀── SELECT (solo lectura) sobre metrics/events/snapshots
```

La capa de visualización nunca escribe. El motor nunca lee de la BD para su lógica. Comunicación en un solo sentido.
