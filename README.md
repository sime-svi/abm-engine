# abm — motor de simulación basada en agentes (ABM)

Motor de simulación masiva basada en agentes, **headless** (sin interfaz gráfica; su salida son
**datos**), escrito en **Rust** y **determinista** (misma semilla + versión + config → misma salida,
bit a bit). La visión completa y las reglas del proyecto están en [`CLAUDE.md`](CLAUDE.md).

> Estado actual: motor mínimo funcionando (modelo de forrajeo en cuadrícula tipo *Sugarscape*) con
> salida a JSON y a PostgreSQL/TimescaleDB, y visualización en Grafana. Ver el histórico en
> [`MEMORY.md`](MEMORY.md) y la explicación por fases en [`docs/design/`](docs/design/).

## Requisitos

- **Rust** (la versión la fija `rust-toolchain.toml`: 1.97.0). Instálalo con [rustup](https://rustup.rs/).
- *(Opcional)* **Docker** + Docker Compose — solo si quieres persistir en base de datos y visualizar
  en Grafana. Si solo quieres los datos en JSON, no hace falta.

## Puesta en marcha rápida (motor → JSON)

```bash
cargo run --release -- config/default.toml   # genera output/<name>.json
cargo test                                    # tests (incluye determinismo)
```

## Configuración

**Todos** los parámetros viven en `config/*.toml` (nada hardcodeado): tamaño de la cuadrícula, nº de
agentes, semilla, metabolismo, reproducción, destino de la salida, etc. Se dirigen experimentos
cambiando la config, no el código. Punto de partida: [`config/default.toml`](config/default.toml).

## Persistencia en base de datos + Grafana

1. Copia las credenciales: `cp infra/.env.example infra/.env` y edítalas.
2. Levanta la infraestructura: `cd infra && docker compose up -d`.
3. En el config, elige el destino: `[persistence] sink = "postgres"` (o `"both"` para JSON + BD).
4. Corre el motor: `cargo run --release -- config/default.toml`.
   La conexión se toma del entorno; el binario **lee `infra/.env` solo** (ejecuta desde la raíz del
   repo), así que no tienes que exportar nada a mano.
5. Abre **Grafana** en http://localhost:3000 → dashboard **«ABM — Visión general»** (ya viene creado).

El esquema de la base de datos está en [`DATABASE_SCHEMA.md`](DATABASE_SCHEMA.md); las tablas se crean
con migraciones (`infra/migrations/`) que aplica el propio binario.

## Estructura del repositorio

```
src/            # el motor: core/ (motor) · domain/ (reglas) · metrics/ · persistence/
config/         # parámetros de simulación (.toml)
docs/design/    # diseño + explicación llana, una carpeta por fase
infra/          # Docker (PostgreSQL/TimescaleDB + Grafana), migraciones y provisioning
tests/          # tests de integración (determinismo, y persistencia con #[ignore])
output/         # volcados JSON (ignorado por git)
```

El árbol detallado y qué está implementado está en [`TREE.md`](TREE.md).

## Servicios (Docker)

| Servicio    | Imagen                              | URL / Puerto          |
|-------------|-------------------------------------|-----------------------|
| TimescaleDB | `timescale/timescaledb:latest-pg16` | `localhost:5432`      |
| Grafana     | `grafana/grafana:latest`            | http://localhost:3000 |

Variables en `infra/.env` (plantilla en `infra/.env.example`): `POSTGRES_USER`, `POSTGRES_PASSWORD`,
`POSTGRES_DB`. Los datos persisten en volúmenes de Docker (`abm-db-data`, `abm-grafana-data`).

## Comandos útiles

```bash
cargo run --release -- config/default.toml   # ejecutar una simulación
cargo test                                    # tests
cargo fmt && cargo clippy -- -D warnings      # formato y linter
cd infra && docker compose up -d              # levantar BD + Grafana
cd infra && docker compose down               # parar (conserva los datos)
```

## Documentación viva

- [`CLAUDE.md`](CLAUDE.md) — instrucciones maestras y visión (fuente de verdad).
- [`MEMORY.md`](MEMORY.md) — histórico de decisiones (append-only).
- [`TREE.md`](TREE.md) — estructura y estado de implementación.
- [`DATABASE_SCHEMA.md`](DATABASE_SCHEMA.md) — arquitectura de la base de datos.
- [`docs/design/`](docs/design/) — diseño técnico y explicación llana por fase.
