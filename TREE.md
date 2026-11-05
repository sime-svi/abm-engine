# TREE.md — Estructura del repositorio

> Este archivo refleja la estructura **objetivo** y marca qué está implementado.
> Leyenda: `[x]` existe y funciona · `[~]` existe parcialmente · `[ ]` planificado, aún no creado.
> Regenerar tras cualquier cambio estructural (ver CLAUDE.md §8).

Última actualización: 2026-11-05 — nota de lluvia orográfica implementada (relieve de referencia a 25 km, distancia de humedad desde la costa)

---

## Estructura objetivo

```
abm/
├── [x] CLAUDE.md                  # Instrucciones maestras (fuente de verdad)
├── [x] MEMORY.md                  # Histórico append-only del proyecto
├── [x] DATABASE_SCHEMA.md         # Arquitectura de la BD
├── [x] TREE.md                    # Este archivo
├── [x] README.md                  # Puesta en marcha para humanos
├── [x] Cargo.toml                 # Workspace: miembros + versiones de dependencias ([workspace.dependencies])
├── [x] Cargo.lock                 # Versiones exactas (reproducibilidad)
├── [x] rust-toolchain.toml        # Fija la versión de Rust (reproducibilidad)
├── [x] .gitignore                 # Ignora /target, /output, infra/.env
│
├── config/
│   ├── [x] default.toml           # Parámetros base de simulación (forrajeo)
│   ├── [x] terreno.toml           # Parámetros del planeta ([lod], [erosion], [soil]…)
│   └── experiments/               # Configs con nombre por experimento
│       └── [ ] .gitkeep
│
├── crates/                        # Un paquete por responsabilidad; las flechas apuntan a abm-core
│   ├── abm-core/                  # EL MOTOR (agnóstico al dominio; no depende de nadie)
│   │   ├── [x] Cargo.toml
│   │   └── src/
│   │       ├── [x] lib.rs
│   │       ├── [x] time.rs         # Tick + run_ticks (el bucle de tiempo)
│   │       ├── [x] schedule.rs     # Planificador de un solo hilo (deterministic)
│   │       ├── [x] rng.rs          # RNG raíz determinista sembrado (ChaCha8)
│   │       └── [x] error.rs        # ConfigError (común a todos los paquetes)
│   │
│   ├── abm-ruleset/               # LAS REGLAS del modelo de forrajeo (→ abm-core)
│   │   ├── [x] Cargo.toml
│   │   └── src/
│   │       ├── [x] lib.rs           # + VERSION (versión del ruleset)
│   │       ├── [x] config.rs        # Config del modelo (config/default.toml)
│   │       ├── [x] components.rs    # Componentes del dominio (Position, Energy, Age, AgentId…)
│   │       ├── [x] grid.rs · params.rs · buffers.rs
│   │       ├── [x] world_setup.rs   # Génesis del mundo (agentes iniciales, cuadrícula)
│   │       ├── [x] schedule.rs      # Orden de los sistemas del tick (= el modelo)
│   │       └── systems/            # Un sistema por comportamiento
│   │           └── [x] regrow · order · forage · reproduce · mortality · apply
│   │
│   ├── abm-terreno/               # EL PLANETA (→ abm-core)
│   │   ├── [x] Cargo.toml · build.rs   # build.rs: huella del código (clave de la caché de la génesis)
│   │   ├── src/
│   │   │   ├── [x] lib.rs           # run / run_with_info / world_dump
│   │   │   ├── [x] config · error · geometry · noise · statics · world · hydro · lod
│   │   │   ├── [x] erosion · cache · worldgen · systems · metrics
│   │   └── tests/
│   │       ├── [x] terreno.rs       # incr. 1 (uniforme) + incr. 2 (LOD: 2:1, conservación…) + lluvia vs resolución
│   │       └── [x] erosion.rs       # incr. 3 (material, hondonadas, camino, tope, caché…) + relieve orográfico
│   │
│   ├── abm-metrics/               # OBSERVADORES + LOD de almacenamiento (→ core, ruleset, persistence)
│   │   ├── [x] Cargo.toml
│   │   └── src/ [x] lib.rs · collect.rs
│   │
│   ├── abm-persistence/           # SALIDA (JSON + Postgres tras un trait; sin deps del proyecto)
│   │   ├── [x] Cargo.toml
│   │   └── src/
│   │       ├── [x] lib.rs           # Recurso Sink + write_json
│   │       ├── [x] schema.rs        # TickMetrics, RunMeta, MetricsDoc
│   │       ├── [x] sink.rs          # Trait MetricsSink (destino intercambiable)
│   │       ├── [x] json_sink.rs     # Volcado a JSON
│   │       ├── [x] pg_sink.rs       # Sink a PostgreSQL/TimescaleDB (RunRecord + COPY)
│   │       ├── [x] migrations.rs    # Aplicador de infra/migrations/ (embebidas)
│   │       └── [x] error.rs         # PersistenceError
│   │
│   └── abm-runner/                # BINARIOS + CLI (→ todos)
│       ├── [x] Cargo.toml           # default-run = "abm"
│       ├── src/
│       │   ├── [x] lib.rs           # run_simulation (monta las piezas), run_record, SimError
│       │   ├── [x] main.rs          # binario `abm` (forrajeo; JSON y/o BD)
│       │   └── [x] bin/terreno.rs   # binario `terreno` (planeta)
│       └── tests/
│           ├── [x] determinism.rs   # misma semilla → misma salida + guardias
│           └── [x] persistence_pg.rs # escribe run+métricas en BD (#[ignore]; corre con --ignored)
│
├── infra/
│   ├── [x] docker-compose.yml     # Postgres/TimescaleDB (host :25432) + Grafana (host :23000), subred fija
│   ├── [x] .env                   # Credenciales y puertos (IGNORADO por git)
│   ├── [x] .env.example           # Plantilla de infra/.env
│   ├── grafana/                   # Aprovisionamiento de Grafana (se carga solo al arrancar)
│   │   ├── [x] provisioning/datasources/datasource.yml
│   │   ├── [x] provisioning/dashboards/provider.yml
│   │   └── [x] dashboards/abm-overview.json
│   └── migrations/                # Migraciones SQL versionadas del esquema
│       └── [x] V1__initial.sql    # simulation_runs + metrics (hypertable)
│
├── docs/
│   └── design/                    # Modelos matemáticos y ecuaciones (precede al código)
│       ├── inicial/               # Una carpeta por fase (ver CLAUDE §8.1)
│       │   ├── [x] README.md          # Explicación llana de la fase + consideraciones
│       │   └── [x] minimal-model.md   # Diseño técnico: modelo de forrajeo (v1)
│       ├── persistencia/          # Fase 2: sink a PostgreSQL/TimescaleDB + Grafana
│       │   ├── [x] README.md
│       │   └── [x] design.md          # Diseño técnico: sink PG, migraciones, fan-out
│       ├── terreno/               # Fase 3: sustrato físico (esfera, clima, agua, vegetación, relieve)
│       │   ├── [x] README.md                              # Explicación llana de la fase
│       │   ├── [x] diseno_terreno_espacio_geografia.md    # Visión completa (madre)
│       │   ├── [x] incremento-1-diseno.md                 # Diseño del 1er incremento (IMPLEMENTADO)
│       │   ├── [x] incremento-2-diseno.md                 # Diseño LOD dinámico v1.1 (IMPLEMENTADO)
│       │   ├── [x] incremento-3-diseno.md                 # Diseño relieve vivo v1.2 (IMPLEMENTADO)
│       │   └── [x] nota_terreno_lluvia_orografica.md      # Lluvia independiente de la resolución (IMPLEMENTADA, §8)
│       └── paquetes/              # Fase: código dividido en el workspace de paquetes
│           └── [x] README.md          # Qué es cada paquete, nudos deshechos, antes → ahora
│
└── output/                        # Volcados JSON locales y caché de la génesis (IGNORADO por git)
    └── [ ] .gitkeep
```

---

## Estado actual — workspace de paquetes; motor con persistencia + planeta con LOD y relieve vivo (2026-11-05)

El código está dividido en el **workspace de seis paquetes** de CLAUDE §3, con `abm-terreno` como
paquete propio, confirmado por el responsable. Dependencias, comprobadas con `cargo tree`:
- `abm-core` no depende de ningún otro paquete del proyecto;
- `abm-ruleset` y `abm-terreno` dependen solo de `abm-core`;
- `abm-persistence` no depende de ninguno del proyecto;
- `abm-metrics` depende de `abm-core`, `abm-ruleset` y `abm-persistence`;
- `abm-runner` depende de todos.

La división no cambia nada por fuera: los dos binarios dan una salida **idéntica byte a byte** a la
de antes, y los comandos son los mismos. Detalle en `docs/design/paquetes/README.md`.

Cerradas hasta hoy:
- **«inicial»**: motor de forrajeo determinista → JSON.
- **«persistencia»**: sink a PostgreSQL/TimescaleDB + Grafana aprovisionado.
- **«terreno»**, incrementos **1** (sustrato físico esférico: clima, agua, vegetación), **2** (LOD
  dinámico) y **3** (relieve vivo: erosión, sedimento y suelo, con una génesis geológica de un millón
  de años en caché).
- **«paquetes»**: el workspace.

Estado:
- **Determinismo:** misma semilla → mismo JSON byte a byte, verificado también entre procesos y entre
  `debug` y `release`.
- **Comprobaciones:** `cargo fmt` / `clippy --workspace -D warnings` / `cargo test` en verde (12
  unitarios + forrajeo 3 + terreno 15 + erosión 14; el test de BD es `#[ignore]` y pasa con
  `--ignored`).
- **Lluvia independiente de la resolución** (nota de lluvia orográfica): relieve de referencia a
  25 km para el efecto orográfico y distancia de humedad medida desde la costa. Al afinar una zona a
  la resolución real, su lluvia cambia un 0,03 % (antes +3,9 %).
- **Terreno:** balance de agua a ~1e-12 con LOD y erosión; balance de material cerrado (el
  acondicionamiento también, exacto); 9 biomas emergentes, ríos y ~50 lagos; ~195 000–205 000 celdas
  con la config por defecto.
- **Génesis en caché** (`output/cache`): ~42 s la primera vez, ~8 s después.
- **Base de datos:** el motor de forrajeo escribe en la BD además del JSON (`[persistence] sink =
  "json" | "postgres" | "both"`; conexión desde `infra/.env`).

**Modelo de forrajeo calibrado:** con los parámetros por defecto la población hace un *boom* inicial
(~300 → ~5000) y se **estabiliza** en una capacidad de carga sana (~5000 de 10 000 celdas) con
nacimientos y muertes continuos, gracias a la relación `g < m < apetito` (regeneración por celda <
metabolismo < apetito; `m = 2 > g = 1`). Detalle en `docs/design/inicial/`.

**Pendiente (fases siguientes):**
- recalibrar la biología de los agentes **por días** (diseño del responsable: vida, metabolismo,
  apetito, rebrote);
- acoplar los agentes al planeta;
- terreno incremento 4 (lagos dinámicos, flujo multidireccional, fertilidad dinámica, eventos);
- decidir `k_oro`: con 0,5 no hay sombra pluviométrica (propuesta ≈ 40; nota de lluvia §8.4);
- tablas `events` y `agent_snapshots` (LOD micro) con su sink;
- volcar las métricas del terreno a la BD.

El diseño de las tablas pendientes ya está en `DATABASE_SCHEMA.md`.

**Infraestructura (2026-11-04):** BD en `localhost:25432` y Grafana en `http://localhost:23000`
(puertos en `infra/.env`; los habituales están ocupados en esta máquina), y red Docker con subred
fija `10.88.42.0/24` (chocaba con una red de otra instancia WSL). Verificado: motor → BD, Grafana →
BD y la consulta del dashboard devuelven datos.
