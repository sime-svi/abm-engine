# TREE.md — Estructura del repositorio

> Este archivo refleja la estructura **objetivo** y marca qué está implementado.
> Leyenda: `[x]` existe y funciona · `[~]` existe parcialmente · `[ ]` planificado, aún no creado.
> Regenerar tras cualquier cambio estructural (ver CLAUDE.md §8).

Última actualización: 2026-11-05 — relieve tectónico (incremento 4 del terreno, rama `terreno-tectonica`): deriva de placas, referencia de la Tierra, diagnóstico v6

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
│   ├── [x] agentes.toml           # Biología de los agentes (especies, energética, forraje, hábitat)
│   ├── [x] terreno.toml           # Parámetros del planeta ([lod], [erosion], [soil], [tectonica]…)
│   ├── referencias/
│   │   ├── [x] tierra_relieve.toml # Tablas de la Tierra (ETOPO 2022, Paxman 2022, Seton 2020, Bird 2003) a las resoluciones del planeta
│   │   ├── [x] tierra_bordes.toml  # Bordes de placa de Bird (2003) medidos en la malla tectónica como los del modelo (+ convergencia de las fosas)
│   │   ├── [x] tierra_nivel_mar.toml # La Tierra real con la regla del modelo: nivel del mar, zócalo por edad y curvas de placa ajustadas; escalones de edad (zonas de fractura)
│   │   └── [x] tierra_lluvia.toml  # Lluvia observada (WorldClim, CHELSA) a 26 km con las definiciones del planeta
│   └── experiments/
│       └── [x] terreno_relieve_ruido.toml # Relieve de ruido: referencia de la no regresión mientras se mide (se borra con el interruptor)
│
├── crates/                        # Un paquete por responsabilidad; las flechas apuntan a abm-core
│   ├── abm-core/                  # EL MOTOR (agnóstico al dominio; no depende de nadie)
│   │   ├── [x] Cargo.toml
│   │   └── src/
│   │       ├── [x] lib.rs
│   │       ├── [x] time.rs         # Tick + run_ticks + Programa (sistemas en orden, cada uno cronometrado)
│   │       ├── [x] schedule.rs     # Planificador de un solo hilo (deterministic)
│   │       ├── [x] rng.rs          # RNG raíz determinista sembrado (ChaCha8)
│   │       ├── [x] config.rs       # Carga de la config con ajustes `ruta=valor` (los `--set`)
│   │       ├── [x] cronometro.rs   # Tiempo por sistema (solo informa; CLAUDE §6)
│   │       └── [x] error.rs        # ConfigError (común a todos los paquetes)
│   │
│   ├── abm-ruleset/               # LAS REGLAS: forrajeo original + biología de los agentes (→ abm-core)
│   │   ├── [x] Cargo.toml
│   │   └── src/
│   │       ├── [x] lib.rs           # + VERSION (versión del ruleset)
│   │       ├── biologia/            # Biología por días (fase «agentes», incremento 1)
│   │       │   ├── [x] mod.rs · config.rs      # BioConfig (config/agentes.toml)
│   │       │   ├── [x] especie.rs   # Especie por masa: Kleiber, West, Siler, leche, N_max, sexos, a_est (+ tests)
│   │       │   ├── [x] azar.rs      # Azar por contador H(semilla, id, tick, propósito, índice) + SumaExacta
│   │       │   ├── [x] componentes.rs · recursos.rs · habitat.rs   # Individuo (ECS), libro, hábitat de prueba (Miami)
│   │       │   ├── [x] sistemas.rs  # T4 forraje · A3 demanda (Holling)/reparto · A4 energética · A5–A9 (destete forzoso)
│   │       │   └── [x] genesis.rs · schedule.rs    # Población inicial y orden del tick
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
│   │   │   ├── [x] erosion · cache · worldgen · systems
│   │   │   ├── [x] metrics.rs       # Métricas por tick + diagnóstico del planeta (v5) + diagnóstico del relieve (v6)
│   │   │   ├── [x] humedad.rs       # Agotamiento de humedad a favor del viento (anillos, Λ, w, c_P)
│   │   │   └── tectonica/           # Relieve tectónico (incremento 4): deriva de placas → relieve de nacimiento
│   │   │       ├── [x] mod.rs       # Génesis tectónica, estado final, interpolación a las celdas del terreno
│   │   │       ├── [x] malla.rs     # Malla tectónica (nivel 2 del cubo) y cuaterniones
│   │   │       ├── [x] deriva.rs    # Placas, fuerzas, huecos y solapes, dientes, fosas (arcos, erosión por subducción, prisma), choques, delaminación, flujo lateral cúbico (adenda v6), erosión, sedimento (solo al océano mundial), eventos, márgenes pasivos
│   │   │       ├── [x] deriva/suavizado.rs # Borde suavizado (nota de bordes): pertenencia φ, normal, tipo de tramo, suavizado de la pertenencia, islas
│   │   │       ├── [x] deriva/fosas.rs     # Ciclo de las fosas (nota de fosas v4): tramos difusos y activos, nacimientos (espontáneos solo en transformantes), invasión de transformantes y márgenes pasivos, polaridad, losa, rotura de la losa tras un choque, convergencia difusa
│   │   │       ├── [x] isostasia.rs # Altura isostática (con el agua), Parsons y Sclater, fosas, nivel del mar
│   │   │       └── [x] historia.rs  # Eventos, métricas de cada Myr y cuentas de la corteza continental
│   │   └── tests/
│   │       ├── [x] terreno.rs       # incr. 1 (uniforme) + incr. 2 (LOD: 2:1, conservación…) + lluvia y w vs resolución + diagnóstico
│   │       ├── [x] erosion.rs       # incr. 3 (material, hondonadas, camino, tope, caché…) + relieve orográfico + mundo plano
│   │       └── [x] tectonica.rs     # incr. 4: determinismo, giro rígido, sin continentes, India–Asia, rift, caché (+ exploratorias: deriva_en_varias_semillas, calibraciones, diagnostico_pilas, diagnostico_fondo_viejo)
│   │
│   ├── abm-metrics/               # OBSERVADORES + LOD de almacenamiento (→ core, ruleset, persistence)
│   │   ├── [x] Cargo.toml
│   │   └── src/ [x] lib.rs · collect.rs · biologia.rs   # biologia: recuentos, balance, tablas de vida, demografía
│   │
│   ├── abm-persistence/           # SALIDA (JSON + Postgres tras un trait; sin deps del proyecto)
│   │   ├── [x] Cargo.toml
│   │   └── src/
│   │       ├── [x] lib.rs           # Recurso Sink + write_json
│   │       ├── [x] schema.rs        # TickMetrics, RunMeta, MetricsDoc
│   │       ├── [x] schema_bio.rs    # BioDoc (salida de la biología)
│   │       ├── [x] sink.rs          # Trait MetricsSink (destino intercambiable)
│   │       ├── [x] json_sink.rs     # Volcado a JSON
│   │       ├── [x] pg_sink.rs       # Sink a PostgreSQL/TimescaleDB (RunRecord + COPY)
│   │       ├── [x] migrations.rs    # Aplicador de infra/migrations/ (embebidas)
│   │       └── [x] error.rs         # PersistenceError
│   │
│   └── abm-runner/                # BINARIOS + CLI (→ todos)
│       ├── [x] Cargo.toml           # default-run = "abm"
│       ├── src/
│       │   ├── [x] lib.rs           # run_simulation / run_biologia_completa, argumentos (--set), informe de tiempos
│       │   ├── [x] main.rs          # binario `abm` (forrajeo; JSON y/o BD)
│       │   ├── [x] bin/agentes.rs   # binario `agentes` (biología por días; + <nombre>_tiempos.json)
│       │   └── [x] bin/terreno.rs   # binario `terreno` (planeta; + <nombre>_tiempos.json)
│       └── tests/
│           ├── [x] biologia.rs      # balance, ayuno, determinismo, orden, cohorte + revisión 1 (Holling, sequía, fecundidad…)
│           ├── [x] determinism.rs   # misma semilla → misma salida + guardias
│           └── [x] persistence_pg.rs # escribe run+métricas en BD (#[ignore]; corre con --ignored)
│
├── herramientas/                  # Herramientas fuera del motor (Python y bash; ver su README)
│   ├── [x] README.md              # Qué hay y cómo se usa
│   ├── [x] verificar.sh           # CLAUDE §10 en un comando: fmt, clippy, tests, escenarios (+ no regresión)
│   ├── [x] escenarios.py          # Batería de escenarios de referencia (márgenes + presupuesto de tiempo)
│   ├── [x] escenarios.toml        # Los escenarios, sus métricas, márgenes y notas
│   ├── [x] no_regresion.sh        # No regresión byte a byte frente a un commit anterior (worktree temporal)
│   ├── [x] planeta_semillas.py    # El planeta con varias semillas: tabla de cierre de la tectónica frente a ETOPO
│   └── analisis/
│       ├── [x] agentes.py         # Resumen de una corrida de agentes (N/N_max, fecundidad, e0, mortalidad…)
│       ├── [x] terreno.py         # Resumen y comparación de planetas (biomas, diagnóstico, tectónica, sombra pluviométrica)
│       ├── [x] referencia_tierra.py # Tablas de la Tierra para el relieve (config/referencias/tierra_relieve.toml)
│       ├── [x] h5mini.py          # Lector mínimo de HDF5 por peticiones parciales (rejillas de Paxman et al.)
│       ├── [x] bordes_bird.py     # Bordes de Bird (2003) en la malla tectónica, medidos y clasificados como el modelo; convergencia de SUB, OCB y CCB
│       ├── [x] tierra_regla_modelo.py # La Tierra real (Seton, GlobSed) con la curva de profundidad y la isostasia del modelo: nivel del mar y zonas de fractura
│       ├── [x] lluvia_observada.py # Lluvia observada de la Tierra a 26 km, frente a la del planeta
│       └── [x] tiffmini.py        # Lector mínimo de GeoTIFF (tiras, Deflate) también por peticiones parciales
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
│       │   ├── [x] nota_terreno_lluvia_orografica.md      # Lluvia independiente de la resolución (IMPLEMENTADA, §8)
│       │   ├── [x] nota_terreno_sombra_pluviometrica.md   # Agotamiento de humedad + c_P (IMPLEMENTADA, §6; decisiones §7)
│       │   ├── [x] propuesta_relieve_realista.md          # Diagnóstico del relieve (DECIDIDA: opción C con deriva)
│       │   ├── [x] diseno_relieve_tectonico.md            # Diseño del responsable: relieve tectónico con deriva de placas (v1.0)
│       │   ├── [x] incremento-4-diseno.md                 # Diseño técnico, calibraciones, resultados frente a la Tierra y hallazgos (IMPLEMENTADO, en medida)
│       │   ├── [x] adenda_balance_corteza.md              # Adenda del responsable (v5): balance de la corteza, flujo lateral, erosión con la altura (IMPLEMENTADA; incremento-4-diseno.md §9, §13, §23 y §28; E_1 y n_ero provisionales)
│       │   ├── [x] nota_bordes_placa.md                   # Nota del responsable: borde suavizado, dientes, 20°, islas (IMPLEMENTADA; incremento-4-diseno.md §12)
│       │   └── [x] nota_ciclo_fosas.md                    # Nota del responsable (v2): nacimiento, vida y muerte de las fosas (IMPLEMENTADA; incremento-4-diseno.md §24–§27; fosa ×1,55: medidas de fuerzas, espera decisión)
│       ├── paquetes/              # Fase: código dividido en el workspace de paquetes
│       │   └── [x] README.md          # Qué es cada paquete, nudos deshechos, antes → ahora
│       └── agentes/               # Fase: biología de los agentes por días (EN CURSO)
│           ├── [x] README.md                      # Explicación llana + hallazgos
│           ├── [x] diseno_biologia_agentes.md     # Diseño del responsable v1.0 (madre)
│           ├── [x] incremento-1-diseno.md         # Individuos en un hábitat de prueba (IMPLEMENTADO)
│           └── [x] nota_agentes_revision_1.md     # Revisión 1 (IMPLEMENTADA para individuos, §8; decisiones §9)
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
  de años en caché). El **4** (relieve tectónico) está implementado y en medida, con interruptor.
- **«paquetes»**: el workspace.

En curso: **«agentes»** (biología por días), incremento 1 hecho (individuos en un hábitat de prueba).

Estado:
- **Determinismo:** misma semilla → mismo JSON byte a byte, verificado también entre procesos y entre
  `debug` y `release`.
- **Comprobaciones:** `herramientas/verificar.sh` pasa toda la lista de CLAUDE §10 en un comando:
  - `cargo fmt`, `clippy --workspace -D warnings` y `cargo test` en verde: 89 tests (unitarios: núcleo 3,
    terreno 21, biología 8, métricas 1, runner 1; forrajeo 3; biología 15; terreno 16; erosión 15;
    tectónica 6 + 4 exploratorios `#[ignore]`; el test de BD es `#[ignore]` y pasa con `--ignored`);
  - la **batería de escenarios de referencia** (`herramientas/escenarios.toml`): 4 escenarios y 51
    métricas con sus márgenes y presupuestos de tiempo, ~1–2 min;
  - con `--no-regresion REF`, la no regresión byte a byte frente a un commit.
- **Consultable:**
  - los binarios aceptan ajustes `--set ruta=valor` sin editar la config;
  - escriben un informe de tiempos por sistema (`<nombre>_tiempos.json`). En el planeta, el LOD se
    lleva el 53 % del tiempo; en los agentes, la demanda el 32 % y las métricas el 17 %;
  - la salida del planeta trae un diagnóstico (relieve, lluvia, PPN) para compararlo con la Tierra.
- **Biología de los agentes (incremento 1 + revisión 1):** individuos con energía en kcal, crecimiento de West,
  reproducción sexual y mortalidad de Siler calibrada (humano: e0 30,9 años; genérico: l(1) = 0,6),
  sobre un hábitat de prueba. Balance de energía cerrado a 10⁻⁶ kcal. Con la revisión 1:
  - la respuesta funcional de Holling regula las poblaciones: herbívoro 1,17·`N_max`, con `N_max` contando la población real y su densidad en la ley de Damuth; humanos 0,96;
  - el apetito se refiere al gasto total;
  - hay destete forzoso y dimorfismo (en una hambruna mueren menos mujeres que hombres);
  - cada mujer tiene su edad de esterilidad (~40 años): 6,8 hijos por mujer, el último hacia los 38.
- **Lluvia independiente de la resolución** (nota de lluvia orográfica): relieve de referencia a
  25 km para el efecto orográfico y distancia de humedad medida desde la costa. Al afinar una zona a
  la resolución real, su lluvia cambia un 0,03 % (antes +3,9 %).
- **Sombra pluviométrica** (nota de sombra pluviométrica, decisiones del 2026-11-05):
  - `k_oro = 40`, su valor físico;
  - agotamiento de humedad con la altura de saturación del aire y `L_recarga` = 3 000 km;
  - `c_P = 1,21`, provisional;
  - el aire ve el mar plano.

  A sotavento de las cordilleras grandes, en latitudes medias, la lluvia baja de 688 a 251 mm y el desierto se duplica. El barlovento queda flojo porque el **relieve no es realista** (propuesta pendiente).
- **Terreno:** balance de agua a ~1e-12 con LOD y erosión; balance de material cerrado (el
  acondicionamiento también, exacto); 9 biomas emergentes, ríos y ~50 lagos; ~195 000–205 000 celdas
  con la config por defecto.
- **Génesis en caché** (`output/cache`): ~43 s la primera vez, ~10 s después.
- **Base de datos:** el motor de forrajeo escribe en la BD además del JSON (`[persistence] sink =
  "json" | "postgres" | "both"`; conexión desde `infra/.env`).

**Modelo de forrajeo calibrado:** con los parámetros por defecto la población hace un *boom* inicial
(~300 → ~5000) y se **estabiliza** en una capacidad de carga sana (~5000 de 10 000 celdas) con
nacimientos y muertes continuos, gracias a la relación `g < m < apetito` (regeneración por celda <
metabolismo < apetito; `m = 2 > g = 1`). Detalle en `docs/design/inicial/`.

- **Relieve tectónico (incremento 4, `docs/design/terreno/incremento-4-diseno.md`):**
  - deriva de placas de 500 Myr en la malla de nivel 2 (~52 km), ~35 s, en la caché de la génesis (`ABMGEN04`);
  - placas a 4,36 cm/año (la Tierra 4,39), con las correlaciones de Forsyth y Uyeda; India–Asia frena a 1/3 (`c_col` calibrado);
  - océano de 3 945 m (la Tierra 3 667), pero tierra demasiado alta (1 459 m, la Tierra 671): el mar queda 1 079 m bajo el dato;
  - **balance de la corteza continental** (adenda v3, `incremento-4-diseno.md` §9 y §13): arcos y erosión por subducción por km² de placa que se hunde, régimen del margen, prisma, choques con `η_col`/`η_arco`, delaminación, flujo lateral sin tope (`D_flujo` = 3 000 km²/Myr), sedimento al mar (y a cuencas cerradas) y balance exacto por paso; `τ_ero` = 63 Myr calibrado con la relajación;
  - **bordes de placa** (nota de bordes, §12): borde suavizado a 150 km, transformantes a 20°, dientes recolocados, islas absorbidas; se hunden 3,6 km²/año (la Tierra 2,7–3,4); el consumo en bordes que no convergen baja del 26 al 3 %;
  - referencia de la Tierra en `config/referencias/tierra_relieve.toml`;
  - interruptor `worldgen.relieve`: con `"ruido"`, salida idéntica byte a byte a la de `main`.

**Pendiente (fases siguientes):**
- **Pendientes obligatorios de realismo** (CLAUDE §2.8; responsable, 2026-11-05), en orden:
  - **relieve tectónico, lo que falta para que sea el de la Tierra** (hallazgos de `incremento-4-diseno.md` §6–§7):
    1. ~~adenda de balance~~ y ~~nota de bordes~~ hechas. **Siguen:** las fuerzas de las placas (las fosas miden 2 veces las de la Tierra con cualquier `L_borde` de 100–300 km; nota de bordes §11.2); la exportación al mar (3,1 AU, ≈1,8): no depende de `τ_ero`; falta la delaminación (0,05 AU, ≈1,1): ¿raíces de los arcos o erosión no lineal? (decide el responsable); rifts que estiren conservando (`F_rift` provisional); los 3 pasos de 500 que pasan la alarma de 80 km; después, calibrar `τ_del` y revisar `f_acr`;
    2. inicio de subducción en márgenes pasivos viejos (hoy sobrevive un 15 % de fondo de más de 200 Myr);
    3. fondo oceánico real (sedimento abisal, mesetas y montes submarinos, puntos calientes) y márgenes continentales sumergidos: con 1–3, el nivel del mar debería volver al dato sin forzar `H_nivel`. **Medido (2026-11-07, `tierra_regla_modelo.py`):** con las edades y el sedimento reales, la curva del modelo deja el mar a −386 m: el zócalo real deja de bajar hacia los 80 Myr y la curva de Parsons y Sclater no. Una curva de placa ajustada a la mediana real lo deja a −150 m. Los 125 m que faltan hasta la media son las mesetas, los montes submarinos y los puntos calientes;
    4. acortamiento andino, ciclo de Wilson (rifts por suturas viejas), flexión de la litosfera (cuencas de antepaís), microplacas;
    5. ~~reducir el remuestreo de la deriva~~ hecho: conservación exacta (2·10⁻¹⁵); la polaridad de la subducción y los papeles de un choque se deciden **por par de placas**, provisional: en la Tierra quién se hunde puede cambiar a lo largo de un mismo borde;
    5b. ~~umbral de transformantes a 20°~~ hecho (`theta_tr_grados`, nota de bordes);
    6. lluvia: barlovento medido con la regla del responsable (laderas del pie a la cresta, sin mesetas): la Tierra observada ×0,90 (WorldClim) – ×1,23 (CHELSA), el planeta ×1,05; queda el reparto por latitudes (nota propia, tras cerrar la tectónica; `c_P` sin recalibrar);
  - **casquetes polares** (relieve de hielo; con ellos, el agua total pasa a ser océano + hielo);
  - **competencia con jerarquía** entre agentes (los fuertes comen primero; en las hambrunas mueren antes
    crías y viejos);
  - **coste del celo** de los machos y estacionalidad reproductiva (P3), y dimorfismo por especie (Q2);
  - **otras causas de muerte** (enfermedad, accidentes, violencia) y recalibrar Siler (P4);
  - **fecundabilidad por edad** y abortos espontáneos (Q1, Q3);
  - **individuos persistentes** (P6): seguir a un humano desde que nace hasta que muere y subir a su
    población (visión del responsable), requisito del incremento 2;
  - **la sequía que mata al 97 % del herbívoro** (escenario `agentes_sequia`). No es la vegetación: el
    forraje en pie solo baja del 50 al 38 % de K. Hay que medir antes de elegir:
    - el ahorro metabólico al pasar hambre (hoy el gasto no baja);
    - la competencia con jerarquía;
    - si las reservas deben incluir proteína, además de grasa;
- biología de los agentes: incremento 2 (cohortes y conversión, con la versión de poblaciones de la
  revisión 1 y P6), incremento 3 (planeta, forraje del terreno y Foco), incremento 4 (eventos y BD);
- terreno incremento 4 (lagos dinámicos, flujo multidireccional, fertilidad dinámica, eventos);
- **siguiente:** la adenda del responsable sobre los sumideros de corteza y las decisiones de
  `incremento-4-diseno.md` §7; después, retirar el interruptor del relieve;
- tablas `events` y `agent_snapshots` (LOD micro) con su sink;
- volcar las métricas del terreno a la BD.

El diseño de las tablas pendientes ya está en `DATABASE_SCHEMA.md`.

**Infraestructura (2026-11-04):** BD en `localhost:25432` y Grafana en `http://localhost:23000`
(puertos en `infra/.env`; los habituales están ocupados en esta máquina), y red Docker con subred
fija `10.88.42.0/24` (chocaba con una red de otra instancia WSL). Verificado: motor → BD, Grafana →
BD y la consulta del dashboard devuelven datos.
