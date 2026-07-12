# TREE.md — Estructura del repositorio

> Este archivo refleja la estructura **objetivo** y marca qué está implementado.
> Leyenda: `[x]` existe y funciona · `[~]` existe parcialmente · `[ ]` planificado, aún no creado.
> Regenerar tras cualquier cambio estructural (ver CLAUDE.md §8).

Última actualización: 2026-07-11 — primer binario mínimo funcionando

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
│   ├── [x] default.toml           # Parámetros base de simulación
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
│   ├── [x] docker-compose.yml     # Postgres/TimescaleDB + Grafana
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
│       └── inicial/               # Una carpeta por fase (ver CLAUDE §8.1)
│           ├── [x] README.md          # Explicación llana de la fase + consideraciones
│           └── [x] minimal-model.md   # Diseño técnico: modelo de forrajeo (v1)
│
├── tests/                         # Tests de integración (incl. determinismo)
│   └── [x] determinism.rs         # misma semilla → misma salida + guardias
│
└── output/                        # Volcados JSON locales (IGNORADO por git)
    └── [ ] .gitkeep
```

---

## Estado actual — primer binario mínimo (2026-07-11)

Existe ya el **primer binario mínimo** `abm` (pirámide invertida): un único paquete Cargo, aún **no**
el workspace de cinco crates. Sus módulos anticipan esa separación, de modo que el futuro refactor
será básicamente mover carpetas:

```
abm/
├── [x] Cargo.toml · rust-toolchain.toml · .gitignore
├── config/[x] default.toml
├── docs/design/[x]                           # una carpeta por fase (CLAUDE §8.1):
│      inicial/ (README + minimal-model) · persistencia/ (README + design)
├── infra/migrations/[x] V1__initial.sql      # simulation_runs + metrics (hypertable)
├── tests/[x] determinism.rs · persistence_pg.rs (BD, #[ignore])
└── src/
    ├── [x] main.rs · lib.rs · config.rs · error.rs
    ├── core/        → futuro abm-core        (rng, time, schedule, world_setup)
    ├── domain/      → futuro abm-ruleset     (components, grid, params, buffers,
    │                    systems/{regrow, order, forage, reproduce, mortality, apply})
    ├── metrics/     → futuro abm-metrics     (schema, collect)
    └── persistence/ → futuro abm-persistence (sink [trait], json_sink, pg_sink, migrations)
```

Estado: determinista (misma semilla → mismo JSON byte a byte, verificado también entre procesos);
`cargo fmt` / `clippy -D warnings` / `test` en verde. La infraestructura `infra/`
(Postgres/TimescaleDB + Grafana) sigue disponible, pero el motor **aún no escribe en BD** (vuelca a
`output/*.json`).

**Nota de calibración:** con los parámetros por defecto la población satura la cuadrícula y se
congela; ajustar `config` para dinámicas auge-caída es el siguiente paso de *modelado*. El salto a
los cinco crates y a la BD llega cuando se decida subir de capa.
