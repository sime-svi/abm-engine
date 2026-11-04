# TREE.md — Estructura del repositorio

> Este archivo refleja la estructura **objetivo** y marca qué está implementado.
> Leyenda: `[x]` existe y funciona · `[~]` existe parcialmente · `[ ]` planificado, aún no creado.
> Regenerar tras cualquier cambio estructural (ver CLAUDE.md §8).

Última actualización: 2026-11-04 — terreno incremento 2 (LOD dinámico) + infra en puertos 25432/23000

---

## Estructura objetivo

```
abm/
├── [x] CLAUDE.md                  # Instrucciones maestras (fuente de verdad)
├── [x] MEMORY.md                  # Histórico append-only del proyecto
├── [x] DATABASE_SCHEMA.md         # Arquitectura de la BD
├── [x] TREE.md                    # Este archivo
├── [x] README.md                  # Puesta en marcha para humanos
├── [~] Cargo.toml                 # Hoy paquete único `abm`; será el workspace de crates
├── [x] rust-toolchain.toml        # Fija la versión de Rust (reproducibilidad)
├── [x] .gitignore                 # Ignora /target, /output, infra/.env
│
├── config/
│   ├── [x] default.toml           # Parámetros base de simulación (forrajeo)
│   ├── [x] terreno.toml           # Parámetros del planeta (fase «terreno», incl. [lod])
│   └── experiments/               # Configs con nombre por experimento
│       └── [ ] .gitkeep
│
├── crates/
│   ├── abm-core/                  # EL MOTOR (agnóstico al dominio)
│   │   ├── [ ] Cargo.toml
│   │   └── src/
│   │       ├── [ ] lib.rs
│   │       ├── [ ] world.rs        # Mundo/entidades (ECS)
│   │       ├── [ ] scheduler.rs    # Orquestación de sistemas por tick
│   │       ├── [ ] time.rs         # Bucle de tiempo, contador de ticks
│   │       └── [ ] rng.rs          # RNG determinista sembrado
│   │
│   ├── abm-ruleset/               # LAS REGLAS (dominio)
│   │   ├── [ ] Cargo.toml
│   │   └── src/
│   │       ├── [ ] lib.rs
│   │       ├── [ ] components.rs    # Componentes del dominio
│   │       └── systems/            # Un sistema por comportamiento
│   │           ├── [ ] resource.rs  # Buscar/consumir recursos
│   │           ├── [ ] reproduce.rs # Reproducción
│   │           └── [ ] mortality.rs # Muerte
│   │
│   ├── abm-metrics/               # OBSERVADORES + LOD de almacenamiento
│   │   ├── [ ] Cargo.toml
│   │   └── src/ [ ] lib.rs
│   │
│   ├── abm-persistence/           # SALIDA (JSON + Postgres tras un trait)
│   │   ├── [ ] Cargo.toml
│   │   └── src/
│   │       ├── [ ] lib.rs
│   │       ├── [ ] sink.rs          # Trait Sink (destino intercambiable)
│   │       ├── [ ] json_sink.rs     # Volcado a JSON (fase inicial)
│   │       └── [ ] pg_sink.rs       # Sink a PostgreSQL (fase madura)
│   │
│   └── abm-runner/                # BINARIO ejecutable + CLI
│       ├── [ ] Cargo.toml
│       └── src/ [ ] main.rs
│
├── infra/
│   ├── [x] docker-compose.yml     # Postgres/TimescaleDB (host :25432) + Grafana (host :23000), subred fija
│   ├── [x] .env                   # Credenciales (IGNORADO por git)
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
│       │   ├── [x] README.md          # Explicación llana de la fase + consideraciones
│       │   └── [x] design.md          # Diseño técnico: sink PG, migraciones, fan-out
│       └── terreno/               # Fase 3: sustrato físico (esfera, clima, agua, vegetación)
│           ├── [x] README.md                              # Explicación llana de la fase
│           ├── [x] diseno_terreno_espacio_geografia.md    # Visión completa (madre)
│           ├── [x] incremento-1-diseno.md                 # Diseño del 1er incremento (IMPLEMENTADO)
│           └── [x] incremento-2-diseno.md                 # Diseño LOD dinámico v1.1 (IMPLEMENTADO)
│
├── tests/                         # Tests de integración (incl. determinismo)
│   ├── [x] determinism.rs         # misma semilla → misma salida + guardias
│   ├── [x] persistence_pg.rs      # escribe run+métricas en BD (#[ignore]; corre con --ignored)
│   └── [x] terreno.rs             # planeta: incr. 1 (uniforme) + incr. 2 (LOD: 2:1, conservación, convergencia…)
│
└── output/                        # Volcados JSON locales (IGNORADO por git)
    └── [ ] .gitkeep
```

---

## Estado actual — motor con persistencia + terreno con LOD dinámico (2026-11-04)

Existe el binario `abm` (pirámide invertida): un único paquete Cargo, aún **no** el workspace de
cinco crates. Sus módulos anticipan esa separación, de modo que el futuro refactor será básicamente
mover carpetas. Cerradas hasta hoy las fases **«inicial»** (motor de forrajeo determinista → JSON),
**«persistencia»** (sink a PostgreSQL/TimescaleDB + Grafana aprovisionado) y, de la fase
**«terreno»**, los incrementos **1** (sustrato físico esférico: clima, agua, vegetación) y **2** (LOD
dinámico: la malla se subdivide/colapsa sola). El terreno es un binario aparte, `terreno`:

```
abm/
├── [x] Cargo.toml · rust-toolchain.toml · .gitignore
├── config/[x] default.toml · terreno.toml (con [lod])
├── docs/design/[x]                           # una carpeta por fase (CLAUDE §8.1):
│      inicial/ (README + minimal-model) · persistencia/ (README + design)
│      terreno/ (README + visión madre + incremento-1-diseno + incremento-2-diseno v1.1) — IMPLEMENTADOS
├── infra/migrations/[x] V1__initial.sql      # simulation_runs + metrics (hypertable)
├── tests/[x] determinism.rs · persistence_pg.rs (BD, #[ignore]) · terreno.rs
└── src/
    ├── [x] main.rs · lib.rs · config.rs · error.rs
    ├── [x] bin/terreno.rs                    # binario de la fase terreno (CLI aparte)
    ├── core/        → futuro abm-core        (rng, time, schedule, world_setup)
    ├── domain/      → futuro abm-ruleset     (components, grid, params, buffers,
    │                    systems/{regrow, order, forage, reproduce, mortality, apply})
    ├── metrics/     → futuro abm-metrics     (schema, collect)
    ├── persistence/ → futuro abm-persistence (sink [trait], json_sink, pg_sink, migrations)
    └── [x] terreno/  → sustrato físico esférico (fase «terreno», incrementos 1 y 2):
             config · geometry (cubo esférico equiangular, claves u64, vecino por tramo) ·
             noise (Perlin 3D) · statics (estáticos = función pura de la celda) ·
             world (SoA + Topology multinivel con vecindad CSR) · hydro (priority-flood, drenaje,
             ríos/lagos, distancia al mar) · lod (vecindad multinivel, reparto conservativo, driver,
             equilibrio 2:1, paso de topología) · worldgen (génesis + refinamiento inicial) ·
             systems (clima, nieve, balance hídrico, difusión simétrica, agua superficial,
             vegetación, interfaz de agentes) · metrics (esquema v2) · mod (run + world_dump)
```

Estado: determinista (misma semilla → mismo JSON byte a byte, verificado también entre procesos y
entre `debug` y `release`); `cargo fmt` / `clippy -D warnings` / `test` en verde (11 unitarios +
forrajeo 3 + terreno 14; el test de BD es `#[ignore]`). Terreno: balance de agua a ~1e-12 también
con LOD activo, 9 biomas emergentes, ríos y lagos; con la config por defecto el LOD deja ~190 000
celdas (21,5 % de la malla fina uniforme) y la malla converge (sin parpadeo).
El motor de forrajeo ya **escribe en la BD** además del JSON: el destino se elige por config
(`[persistence] sink = "json" | "postgres" | "both"`); la conexión se toma del entorno
(`DATABASE_URL` o `POSTGRES_*` de `infra/.env`, autocargado). Infraestructura `infra/`
(Postgres/TimescaleDB + Grafana) disponible.

**Modelo de forrajeo calibrado:** con los parámetros por defecto la población hace un *boom* inicial
(~300 → ~5000) y se **estabiliza** en una capacidad de carga sana (~5000 de 10 000 celdas) con
nacimientos y muertes continuos, gracias a la relación `g < m < apetito` (regeneración por celda <
metabolismo < apetito; `m = 2 > g = 1`). Detalle en `docs/design/inicial/`.

**Pendiente (fases siguientes):** terreno incremento 3 (erosión y pedogénesis, relieve vivo);
refactor a los cinco crates; tablas `events` y `agent_snapshots` (LOD micro) con su sink; volcar
las métricas del terreno a la BD; acoplar los agentes al planeta. El diseño de las tablas pendientes
ya está en `DATABASE_SCHEMA.md`.

**Infraestructura (2026-11-04):** BD en `localhost:25432` y Grafana en `http://localhost:23000`
(puertos en `infra/.env`; los habituales están ocupados en esta máquina) y red Docker con subred fija
`10.88.42.0/24` (chocaba con una red de otra instancia WSL). Verificado: motor → BD, Grafana → BD y
la consulta del dashboard devuelven datos. Falta solo mirarlo en el navegador (con la contraseña de
Grafana del responsable).
