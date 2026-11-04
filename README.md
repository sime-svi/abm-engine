# abm — motor de simulación basada en agentes (ABM)

Motor de simulación masiva basada en agentes, **headless** (sin interfaz gráfica; su salida son
**datos**), escrito en **Rust** y **determinista** (misma semilla + versión + config → misma salida,
bit a bit). La visión completa y las reglas del proyecto están en [`CLAUDE.md`](CLAUDE.md).

> Estado actual: motor mínimo funcionando (modelo de forrajeo en cuadrícula tipo *Sugarscape*) con
> salida a JSON y a PostgreSQL/TimescaleDB, y visualización en Grafana; y, aparte, el **sustrato
> físico** de un planeta esférico (clima, agua, ríos, vegetación) con **resolución adaptativa** (LOD).
> Ver el histórico en [`MEMORY.md`](MEMORY.md) y la explicación por fases en [`docs/design/`](docs/design/).

## Requisitos

- **Rust** (la versión la fija `rust-toolchain.toml`: 1.97.0). Instálalo con [rustup](https://rustup.rs/).
- *(Opcional)* **Docker** + Docker Compose — solo si quieres persistir en base de datos y visualizar
  en Grafana. Si solo quieres los datos en JSON, no hace falta.

## Puesta en marcha rápida (motor → JSON)

```bash
cargo run --release -- config/default.toml   # genera output/<name>.json
cargo test                                    # tests (incluye determinismo)
```

## Simulación del terreno (planeta)

Binario aparte, `terreno`: genera un planeta esférico a partir de una semilla y simula un año de
clima, nieve, agua del suelo, ríos y vegetación. La malla se **afina sola** donde hay detalle (costas,
ríos, relieve abrupto) y se vuelve gruesa donde no (LOD dinámico).

```bash
cargo run --release --bin terreno    # usa config/terreno.toml → output/terreno_terreno.json
cargo test --test terreno            # tests de la fase
```

Con el LOD activo conviene `--release` (~13 s por año simulado). Para la malla uniforme sin LOD,
pon `l_max = 0` en la sección `[lod]` de `config/terreno.toml`. Explicación llana en
[`docs/design/terreno/README.md`](docs/design/terreno/README.md).

## Configuración

**Todos** los parámetros viven en `config/*.toml` (nada hardcodeado): tamaño de la cuadrícula, nº de
agentes, semilla, metabolismo, reproducción, destino de la salida, etc. Se dirigen experimentos
cambiando la config, no el código. Puntos de partida: [`config/default.toml`](config/default.toml)
(forrajeo) y [`config/terreno.toml`](config/terreno.toml) (planeta).

## Persistencia en base de datos + Grafana

1. Copia las credenciales: `cp infra/.env.example infra/.env` y edítalas (ahí van también los
   puertos: `PGPORT=25432` para la BD y `GRAFANA_PORT=23000` para Grafana).
2. Levanta la infraestructura: `cd infra && docker compose up -d`.
3. En el config, elige el destino: `[persistence] sink = "postgres"` (o `"both"` para JSON + BD).
4. Corre el motor: `cargo run --release -- config/default.toml`.
   La conexión se toma del entorno; el binario **lee `infra/.env` solo** (ejecuta desde la raíz del
   repo), así que no tienes que exportar nada a mano.
5. Abre **Grafana** en http://localhost:23000 → dashboard **«ABM — Visión general»** (ya viene creado).

El esquema de la base de datos está en [`DATABASE_SCHEMA.md`](DATABASE_SCHEMA.md); las tablas se crean
con migraciones (`infra/migrations/`) que aplica el propio binario.

## Estructura del repositorio

```
src/            # el motor: core/ (motor) · domain/ (reglas) · metrics/ · persistence/ · terreno/ (planeta)
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
| TimescaleDB | `timescale/timescaledb:latest-pg16` | `localhost:25432`      |
| Grafana     | `grafana/grafana:latest`            | http://localhost:23000 |

Variables en `infra/.env` (plantilla en `infra/.env.example`): `POSTGRES_USER`, `POSTGRES_PASSWORD`,
`POSTGRES_DB`, y los puertos del host `PGPORT` (BD, 25432) y `GRAFANA_PORT` (23000). Se evitan los
habituales 5432/3000 porque en esta máquina están ocupados por otros proyectos; si chocan, cámbialos
solo en `infra/.env`. Los datos persisten en volúmenes de Docker (`abm-db-data`, `abm-grafana-data`).

> La red de los contenedores tiene una subred fija (`10.88.42.0/24`): todas las instancias de WSL2
> comparten la red, y si otra instancia tiene una red de Docker con la misma subred, los contenedores
> dejan de ser alcanzables (pasó con `172.18.0.0/16`). Si alguna vez choca, cámbiala en
> `infra/docker-compose.yml` y recrea con `docker compose down && docker compose up -d`.
>
> Si olvidaste la contraseña de Grafana: `docker exec -it abm-grafana grafana cli admin reset-admin-password <nueva>`.

> Si `docker` responde `permission denied`, tu usuario no está en el grupo `docker`: usa
> `sudo docker compose …`, o añádete al grupo una sola vez con `sudo usermod -aG docker $USER` y
> reinicia la sesión. (El **motor** no necesita Docker para escribir en la BD: conecta por el puerto.)

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
