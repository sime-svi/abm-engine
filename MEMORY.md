# MEMORY.md — Histórico del proyecto

> Memoria persistente entre sesiones. **Append-only**: se añaden entradas al final, nunca se reescribe el pasado.
> Cada entrada documenta qué se hizo, por qué, y qué decisiones o problemas surgieron.
> Actualizar tras cada cambio significativo (ver CLAUDE.md §8).

Formato de entrada:

```
## [AAAA-MM-DD] Título breve
- Qué: ...
- Por qué: ...
- Decisiones / problemas: ...
- Estado tras el cambio: ...
```

---

## [2026-07-11] Definición de la visión y del stack

- **Qué:** Se fija el objetivo del proyecto (motor ABM masivo, comportamiento emergente, escala macro y micro) y el stack estricto: Rust + ECS puro, headless, salida a JSON/PostgreSQL.
- **Por qué:** Rust por seguridad de memoria y concurrencia; ECS por rendimiento de caché; headless porque el producto son los datos, no una UI.
- **Decisiones:** Paradigma ECS puro (prohibida la POO para entidades). Desacoplamiento obligatorio Core/Ruleset. LOD tanto en cómputo como en almacenamiento. Metodología de pirámide invertida.
- **Estado:** Visión acordada. Sin código.

## [2026-07-11] Entorno de desarrollo (WSL2 dedicado)

- **Qué:** Se crea una instancia WSL2 dedicada y aislada (`abm-engine`, Ubuntu 24.04 LTS) separada de la instancia de trabajo, mediante `wsl --install ... --name`.
- **Por qué:** Aislamiento total del entorno laboral; entorno Linux para toolchain de Rust y contenedores de datos.
- **Decisiones / problemas:**
  - La instancia arrancaba como `root` (comportamiento de las instancias `--name`). Se configura un usuario normal (`simes`) como usuario por defecto vía `/etc/wsl.conf`, necesario también porque Claude Code no permite `--dangerously-skip-permissions` como root.
  - `.wslconfig` es global a todas las instancias WSL2 (no per-instancia); se tiene en cuenta al asignar CPU/RAM.
  - Arranque en `$HOME` en lugar de `/mnt/c` resuelto con condición en `.bashrc`.
  - Regla de rendimiento fijada: el código vive en el filesystem de Linux (`~/abm`), nunca en `/mnt/c`.
- **Estado:** Instancia operativa con usuario normal.

## [2026-07-11] Infraestructura de datos (Docker)

- **Qué:** Docker Engine nativo instalado en WSL2 (no Docker Desktop). `docker-compose.yml` con PostgreSQL/TimescaleDB (`timescale/timescaledb:latest-pg16`) y Grafana.
- **Por qué:** Postgres+TimescaleDB elegido sobre MongoDB por ser datos de serie temporal con necesidad de SQL analítico; Grafana como visualización inmediata sin escribir frontend. Docker por reproducibilidad y aislamiento.
- **Decisiones / problemas:**
  - Credenciales movidas a `infra/.env` (ignorado por git); el compose las referencia por variable. Contraseñas simples en local, pero fuera del control de versiones.
  - systemd habilitado en `/etc/wsl.conf` para que el demonio de Docker arranque solo.
  - Grafana no conectaba a Postgres usando `localhost`: entre contenedores hay que usar el nombre de servicio (`db:5432`) y SSL en `disable`. Resuelto.
- **Estado:** Postgres y Grafana operativos y conectados.

## [2026-07-11] Cimientos documentales

- **Qué:** Creados los cuatro documentos vivos: `CLAUDE.md`, `DATABASE_SCHEMA.md`, `TREE.md` y este `MEMORY.md`. Se define el esquema de BD escalable (runs, agent_dim, metrics, events, agent_snapshots) con LOD de almacenamiento, y la estructura objetivo del workspace de crates.
- **Por qué:** Establecer los cimientos y el protocolo de documentación viva antes de escribir una sola línea de código Rust.
- **Decisiones:** Multi-ejecución de primera clase (todo cuelga de `run_id`); reproducibilidad por semilla + versiones; no almacenar estado de cada agente en cada tick (snapshots muestreados + reconstrucción por eventos).
- **Estado:** Cimientos documentales completos. **Próximo paso:** inicializar el repositorio Git y el primer binario mínimo de Rust (cuadrícula + agentes), aún sin BD, volcando a JSON.

## [2026-07-11] Preferencia de comunicación con el responsable

- **Qué:** El responsable (Simeón) es nuevo en Rust y en programación de sistemas. Pide que toda comunicación use lenguaje sencillo: se pueden y deben usar términos técnicos, pero SIEMPRE explicados al momento, en lenguaje llano, sin dar por conocido el vocabulario.
- **Por qué:** Está aprendiendo mientras dirige el proyecto; la claridad es más importante que la concisión técnica.
- **Aplicable a:** cualquier agente que interactúe con él. Introduce el término técnico y justo detrás su traducción en cristiano (ej. "crate = paquete de código, una caja de piezas con una función concreta").
- **Estado:** Directriz activa.

## [2026-07-11] Primer binario mínimo del motor (forrajeo en cuadrícula)

- **Qué:** Implementado el primer binario mínimo `abm` en Rust siguiendo la pirámide invertida:
  cuadrícula 2D toroidal con recurso que regenera + agentes que se mueven, consumen, metabolizan, se
  reproducen y mueren (modelo tipo *Sugarscape*, formalizado antes de codificar en
  `docs/design/minimal-model.md`). ECS con `bevy_ecs` 0.19 headless. Salida a JSON en `output/`.
  Todos los parámetros en `config/default.toml`.
- **Por qué:** Establecer una base matemáticamente estable, determinista y verificada antes de subir
  de capa (BD, snapshots, workspace de crates).
- **Decisiones / problemas:**
  - **Motor ECS = `bevy_ecs` desde el día 1** (decisión del responsable frente a un SoA a mano). Su
    planificador puede paralelizar, lo que chocaría con el determinismo bit-a-bit (§6). Blindaje de
    triple cinturón: feature `multi_threaded` **no compilada** (`default-features = false`) +
    `SingleThreadedExecutor` explícito + `.chain()` (orden total de sistemas). A nivel de datos:
    orden canónico por `AgentId`, spawn/despawn diferidos a un sistema **exclusivo** (muertes→
    nacimientos), grid como `Vec` plano (sin `HashMap`), RNG `ChaCha8Rng` sembrado usado **solo** en
    la génesis (cero sorteos en el bucle; desempate por orden fijo), y sumas en `f64` en orden fijo.
  - **Verificado:** dos corridas con la misma semilla producen un JSON **idéntico byte a byte**, y
    también entre procesos distintos (no solo dentro del mismo). Tests: `misma_semilla_misma_salida`,
    `semilla_distinta_salida_distinta`, `no_es_trivialmente_vacio`.
  - **Estructura pre-crates:** un único paquete con módulos `src/core` (→ `abm-core`), `src/domain`
    (→ `abm-ruleset`), `src/metrics`, `src/persistence`. El refactor futuro a workspace será mover
    carpetas, sin reescribir lógica. El trait `MetricsSink` es la costura para meter Postgres luego.
  - **Versiones verificadas hoy en crates.io** (varias posteriores al corte de conocimiento del
    agente): `bevy_ecs` 0.19.0 (¡`ExecutorKind` fue eliminado → `set_executor`!), `rand` 0.10.2 (los
    métodos núcleo `next_u32/u64` viven en el trait `rand::Rng`), `toml` 1.1, `thiserror` 2. Edition
    2024; Rust fijado a 1.97.0 vía `rust-toolchain.toml`.
  - **Hallazgo de dinámica (no es un bug):** con los parámetros por defecto la población **satura
    toda la cuadrícula** (10000) y se congela en equilibrio (recurso ≈ 0, energía media ~13 < umbral
    de reproducción 20), **sin muertes en 200 ticks** (los agentes envejecen hasta ~185; morirían por
    vejez justo después). El motor es correcto y determinista; es cuestión de **calibrar parámetros**
    (trabajo del responsable, §5). Pendiente sugerido: subir el metabolismo por encima de la
    regeneración por celda para obtener ciclos auge-caída y ejercitar la mortalidad.
- **Estado:** Base estable y verificada. `cargo fmt` + `cargo clippy -- -D warnings` + `cargo test`
  en verde; sin `unwrap`/`panic`/`unsafe`/reloj de pared en la lógica. Listo para calibrar el modelo
  o para subir de capa.

## [2026-07-13] Convención de documentación por fases + reorganización de `docs/design/`

- **Qué:** Añadida al `CLAUDE.md` la sección **§8.1**: al completar una fase se crea en `docs/design/`
  una carpeta con el nombre de la fase, se mueve ahí el diseño técnico/matemático y se añade un
  `README.md` en lenguaje llano (qué se logra, para qué, consideraciones). Aplicado a la fase
  **inicial**: `docs/design/minimal-model.md` movido con `git mv` (historial conservado) a
  `docs/design/inicial/minimal-model.md`, y creado `docs/design/inicial/README.md`.
- **Por qué:** Trazabilidad. Separar el "cómo" formal (diseño técnico) del "qué/por qué" en lenguaje
  sencillo, una carpeta por fase, para seguir la evolución leyendo los `README.md` en orden. Pedido
  por el responsable.
- **Decisiones:** El `README.md` de fase es el documento "para humanos"; el markdown técnico convive
  con él en la misma carpeta. Convención registrada en `CLAUDE.md §8.1` (cambio confirmado por el
  responsable, como exige §8).
- **Estado:** Convención activa y aplicada a la fase inicial.

## [2026-07-13] Calibración del modelo: dinámica de capacidad de carga

- **Qué:** Ajustado `config/default.toml`: `metabolism` 1.0 → **2.0** y `total_ticks` 200 → 500. Con
  ello el modelo pasa de una trayectoria degenerada (la población saturaba el tablero de 10 000 celdas
  y se congelaba, sin muertes) a una **dinámica sana de capacidad de carga**: *boom* inicial
  (~300 → ~5000) y equilibrio estable ~5000 (media población del tablero) con nacimientos y muertes
  continuos, recurso oscilando (~5500), sin saturar ni extinguirse.
- **Por qué:** El fallo estaba en que `metabolism (1) = regrowth (1)`: en un tablero lleno cada agente
  empataba energía y nadie moría → congelación. Regla de calibración descubierta: **`g < m < apetito`**
  (regeneración por celda < metabolismo < apetito). El metabolismo por encima de la regeneración hace
  que el hacinamiento provoque inanición (capacidad de carga); por debajo del apetito permite ganar
  energía en celdas ricas (crecimiento).
- **Cómo se eligió:** Barrido empírico de `m ∈ {2.0, 2.5, 3.0}` en corridas de 800 ticks (las tres son
  sanas; a mayor `m`, menor población de equilibrio y más recurso libre). Elegido `m = 2.0` por el
  ratio limpio 2:1 y una densidad de equilibrio ~50 %.
- **Verificación:** determinismo intacto (dos corridas idénticas byte a byte), `cargo test` en verde
  (los 3 tests; ahora también se ejercita la mortalidad). Sin cambios de código: solo `config` + docs
  de la fase (`docs/design/inicial/`) actualizados.
- **Estado:** Modelo de la fase inicial **calibrado y correcto**.

## [2026-07-13] Fase «persistencia»: sink a PostgreSQL/TimescaleDB

- **Qué:** El motor ya persiste en la BD (PostgreSQL 16 + TimescaleDB), además del JSON. Migración
  `V1__initial.sql` con `simulation_runs` (registro de corridas) y `metrics` (hypertable por `tick`,
  formato largo). Sink elegible por config (`[persistence] sink = json|postgres|both`, por defecto
  `json`). Nuevos módulos `persistence::pg_sink` (write_run) y `persistence::migrations` (aplicador).
- **Por qué:** Cerrar el hueco "infra levantada pero muerta": activar la BD + Grafana, que es el valor
  del proyecto (datos consultables y graficables). La costura (trait `MetricsSink`) ya existía.
- **Decisiones:**
  - Cliente `postgres` **síncrono** (no `sqlx`): la BD es salida, no toca el determinismo; se evita un
    runtime asíncrono. Escritura atómica: INSERT run + `COPY` de métricas en una transacción.
  - **Migraciones aplicadas por el propio binario** (embebidas con `include_str!`, tabla
    `schema_migrations`), porque **no hay `psql`** instalado. Coincide con DATABASE_SCHEMA §5.
  - **Conexión desde el entorno** (`DATABASE_URL` o `POSTGRES_*`), nunca en código/config (§9).
  - Fan-out en `main.rs`, no en el bucle: `run_simulation` sigue devolviendo el `MetricsDoc`; el runner
    reparte a los destinos. Los tests de determinismo siguen **herméticos** (sink por defecto `json`).
  - `run_id` = UUID por ejecución (identificador de corrida, no parte del estado reproducible);
    `started_at/finished_at` los pone la BD con `now()` (nada de reloj de pared en Rust).
  - Docker: el usuario no está en el grupo `docker`, pero los contenedores corren y el puerto 5432 es
    accesible, así que el motor conecta directo sin el CLI de Docker.
- **Verificación:** test de integración `tests/persistence_pg.rs` (`#[ignore]`; corre con la BD y
  `--ignored`): escribe una corrida y comprueba 1 fila en `simulation_runs` + 3500 en `metrics`
  (500 ticks × 7 métricas). Corrida real con `sink=both` OK (run_id impreso). `cargo test` hermético
  sigue 3/3 con el test de BD ignorado. `fmt` / `clippy -D warnings` limpios.
- **Dependencias nuevas:** `postgres` 0.19 (features `with-serde_json-1`, `with-uuid-1`) y `uuid` 1 (v4).
- **Estado:** Fase «persistencia» completa y verificada. Pendiente (fases siguientes): `events`,
  `agent_snapshots` (LOD micro), dashboards de Grafana, refactor a los 5 crates.

## [2026-07-13] Grafana aprovisionado (visualización automática)

- **Qué:** Aprovisionamiento de Grafana por ficheros en `infra/grafana/`: fuente de datos PostgreSQL
  (`datasource.yml`, apuntando a `db:5432`) y un dashboard inicial (`abm-overview.json`, 4 paneles
  *Trend* de la última ejecución: población, nacimientos/muertes, energía media y recurso). El
  `docker-compose.yml` monta el aprovisionamiento en el contenedor de Grafana y le pasa las `POSTGRES_*`.
- **Por qué:** Que la visualización aparezca sola y reproducible, sin configurar Grafana a mano. Cierra
  el círculo datos → gráficas.
- **Decisiones:** La contraseña NO va en los ficheros (se interpola de `$POSTGRES_PASSWORD`, que compose
  pasa desde `infra/.env`). Paneles tipo *Trend* con eje X = `tick` (no tiempo de reloj), coherente con
  que el tiempo de la simulación es un entero.
- **Verificación:** JSON y YAML validados localmente. **Pendiente**: que el responsable recree el
  contenedor de Grafana (`docker compose up -d`) para que cargue el aprovisionamiento — requiere Docker
  (el usuario no está en el grupo docker: usar `sudo`).
- **Estado:** Ficheros listos y commiteables; falta recrear el contenedor y comprobar en el navegador.

## [2026-07-13] Cierre de fase: README + autocarga de infra/.env

- **Qué:** Reescrito el `README.md` de la raíz (estaba obsoleto: solo describía la infra). Ahora
  documenta el motor, la puesta en marcha (JSON y BD+Grafana), la config, la estructura y los docs
  vivos. Además, el binario **autocarga `infra/.env`** al arrancar (dep `dotenvy`), así conectar a la
  BD ya no requiere el ritual `set -a; . infra/.env; set +a` (no sobreescribe variables ya definidas).
- **Por qué:** Cerrar la fase con la documentación coherente y quitar fricción al responsable.
- **Verificación:** corrida con `sink=both` desde un shell **sin** variables `POSTGRES_*` → conecta y
  escribe (run_id impreso). `fmt` / `clippy -D warnings` / `test` en verde.
- **Estado:** Primera fase (motor + persistencia + Grafana) **cerrada**. Pendiente solo el commit.

## [2026-07-13] Sincronización de TREE.md con el estado real (auditoría de fases)

- **Qué:** Auditadas las fases «inicial» y «persistencia» (docs de fase + docs vivos + código). Las dos
  carpetas `docs/design/{inicial,persistencia}/` estaban completas y coherentes con el código, la
  config y la migración; `fmt` / `clippy -D warnings` / `test` (3/3 herméticos, el de BD `#[ignore]`)
  en verde. Pero `TREE.md` había quedado desincronizado: el commit de persistencia actualizó su bloque
  de árbol pero dejó la prosa anterior a la calibración y a la BD. Corregido.
- **Por qué:** CLAUDE §8 exige que los cuatro documentos vivos reflejen el estado real; `TREE.md`
  contradecía a los otros tres (y a sí mismo: el árbol ya citaba la persistencia, pero la prosa la
  negaba).
- **Decisiones / problemas:** Cambios solo en `TREE.md` (nada de código): (1) fecha/título de «Última
  actualización» y de «Estado actual» → 2026-07-13; (2) añadidas `docs/design/persistencia/` y
  `tests/persistence_pg.rs` al árbol objetivo; (3) prosa «el motor **aún no escribe en BD**» → ahora
  escribe (sink `json`/`postgres`/`both`, conexión desde el entorno); (4) «Nota de calibración»
  (población satura la cuadrícula y se congela) → «Modelo calibrado» (capacidad de carga sana,
  `g < m < apetito`, `m = 2 > g = 1`); (5) listado explícito de lo pendiente (5 crates,
  `events`/`agent_snapshots`, verificar Grafana en el navegador).
- **Estado tras el cambio:** Los cuatro documentos vivos coherentes entre sí y con el código. Únicos
  pendientes reales: **verificar Grafana en el navegador** (requiere Docker) y, en fases siguientes, el
  refactor a los cinco crates y las tablas de LOD micro (`events`, `agent_snapshots`).

## [2026-07-13] Diseño de la fase «terreno» — incremento 1 (sustrato físico)

- **Qué:** Leída la visión completa del terreno (`docs/design/terreno/diseno_terreno_espacio_geografia.md`,
  esfera + clima + agua + vegetación + erosión + pedogénesis + LOD adaptativo). Como es enorme, se
  recortó un **primer incremento estable** y se documentó: nuevo `incremento-1-diseno.md` (diseño
  técnico) + `README.md` de la fase (explicación llana). Aún **sin código**: es fase de diseño.
- **Por qué:** La pirámide invertida (CLAUDE §4) prohíbe implementar la visión entera de golpe. El
  documento madre es el plano; el incremento 1 es la primera capa verificable.
- **Decisiones (acordadas con el responsable vía 4 preguntas):**
  1. **Geometría:** cubo esférico **completo** desde el principio (6 caras, adyacencia real, lat/lon).
     Esquinas del cubo (P8) resueltas con estárcil renormalizado sobre 7 vecinos → cerrada para el incr.
  2. **Resolución:** estructuras **LOD-ready** pero corriendo a **nivel uniforme fijo**; la
     subdivisión/colapso dinámica (6.1) se difiere al incremento 2. Recomendación mía (punto intermedio,
     no era ninguna de las dos opciones ofrecidas): construir el árbol ya, activarlo después, para no
     reescribir la física (que es «invariante a la escala»).
  3. **Procesos:** solo los **rápidos** (clima, nieve, balance hídrico, agua superficial, vegetación).
     **Relieve estático**: erosión (6.7) y pedogénesis (6.8) → incremento 3.
  4. **Tick:** **fino ~365/año** (≈1 día). Regla de reescalado uniforme: tasas por tick ×(48/365),
     cadencias en ticks ×(365/48). P1 cerrada. Anotado el desfase con la vida de agentes (~200 ticks).
- **Alcance geométrico concreto fijado:** proyección **equiangular** (mejor reparto de área que la
  gnomónica ingenua), área por celda vía ángulo sólido, vecindad inter-cara **precalculada por
  geometría** (evita tabular a mano las 12 aristas), ruido de elevación **3D** (continuidad automática
  entre caras), priority-flood + acumulación de flujo con **desempate por id** (determinista).
- **Preguntas abiertas tras el incremento:** P1 cerrada (tick=365); P8 cerrada (esquinas); P4 = recurso
  genérico único; P5 = hipsometría paramétrica; P6 no aplica (relieve estático); P2/P3/P7/P9 = futuro.
- **Estado tras el cambio:** Fase «terreno» **diseñada, no implementada**. Documentos vivos: creados
  `terreno/README.md` y `terreno/incremento-1-diseno.md`; `TREE.md` actualizado (árbol objetivo + prosa
  + fecha). Sin tocar código, config, esquema de BD ni migraciones. Próximo paso natural: implementar el
  incremento 1 (o refinar el diseño si el responsable lo revisa).

## [2026-07-13] Implementación de la fase «terreno» — incremento 1

- **Qué:** Implementado el sustrato físico esférico (módulo `abm::terreno` en `src/terreno/`, binario
  `terreno`, `config/terreno.toml`, `tests/terreno.rs`). Genera un planeta (cubo esférico equiangular)
  y corre los sistemas rápidos por tick (clima con estaciones y sombra pluviométrica, nieve, balance
  hídrico tipo cubo, agua superficial/ríos/lagos, vegetación con biomas de Whittaker) de forma
  determinista. Salida: métricas por tick a JSON (+ snapshot opcional del mundo). Independiente del
  motor de forrajeo (no lo toca).
- **Por qué:** Primer incremento estable de la fase (pirámide invertida), según el diseño acordado.
- **Decisiones / problemas resueltos:**
  - **Celdas como columnas `Vec` SoA** (no entidades ECS), igual que el recurso `Grid` del forrajeo:
    mejor caché y orden de iteración trivialmente estable para ~100k celdas. Desviación documentada.
  - **Bug de geometría**: el orden de la tupla de ejes de cara en `face_basis` no coincidía con el
    desdoblado `(a,b,n)` → dos caras se solapaban. El test de "área = 4πR²" NO lo detectó (el solapamiento
    suma igual); lo cazó un test nuevo de reciprocidad de vecinos ortogonales. Corregido.
  - **Conservación de agua**: fugaba ~1e-4 relativo. Causa: la difusión lateral movía mm iguales entre
    celdas de área desigual (el cubo esférico no es uniforme) → no conservaba volumen. Reescrita para
    transferir volumen (razón de áreas) con limitador de flujo. Ahora cierra a **1e-13** (verificado).
  - **Realismo (calibración por config, no código)**: la elevación lineal dejaba la tierra a ~4500 m
    (mundo helado) → añadida curva hipsométrica `land_exp=3` (media ~490 m). `p_ref` reescalado (2.6)
    daba <1000 mm/año → calibrado a 5.0 para que emerja la selva. Emergen los 9 biomas.
  - **Alcance recortado con honestidad**: flujo de agua de **receptor único** (D8) en vez de D-infinito;
    **lagos estáticos** (sin volumen dinámico/desborde). Irrelevante sin erosión; documentado en §12 del
    diseño. D-infinito y lagos dinámicos → incrementos 2/3.
  - **Esquinas del cubo (P8)**: celdas de esquina con 7 vecinos, estárcil de difusión renormalizado.
  - **Tick fino (P1)**: `T_anio=365`; tasas por tick ×48/365, cadencias ×365/48 (tabla en config).
- **Verificación:** `fmt` + `clippy -D warnings` limpios; `cargo test` en verde (forrajeo 3/3 intactos +
  terreno 3 unit + 6 integración: determinismo, semillas distintas, continuidad esférica, conservación
  de agua, caso sin lluvia, ríos). Determinismo byte a byte entre procesos confirmado.
- **Estado tras el cambio:** Fase «terreno» incremento 1 **cerrada**. Documentos vivos actualizados
  (este MEMORY, TREE, diseño §12, README de la fase). Pendiente: incrementos 2 (LOD dinámico) y 3
  (erosión/pedogénesis); opcionalmente, verter las métricas de terreno a Postgres/Grafana.

## [2026-07-15] Diseño de la fase «terreno» — incremento 2 (LOD dinámico)

- **Qué:** Redactado `docs/design/terreno/incremento-2-diseno.md`, el diseño técnico del LOD adaptativo
  (subdivisión/colapso), que es el siguiente paso según el mapa del incremento 1 (§10). Aún **sin
  código**: se respeta la regla de la documentación (CLAUDE §7/§4) de que el diseño matemático precede
  a la implementación, sobre todo en la pieza más sensible al determinismo de todo el terreno.
- **Por qué:** El LOD dinámico es el incremento 2 planificado; su corrección (independencia del camino,
  conservación de extensivos, vecindad entre niveles) exige cerrarse en papel antes de programar.
- **Decisiones de diseño (con demostraciones en el doc):**
  - **Bosque de quadtrees** por cara; hojas = celdas activas; columnas SoA reconstruidas y ordenadas por
    `id` en cada paso de topología (cada `N_lod`) → determinismo trivial; `index_of` solo para vecindad.
  - **Reparto conservativo**: estáticos recalculados del ruido (función pura); densidades (SWE, V)
    copiadas al subdividir y promediadas por área al colapsar; agua `W` repartida por volumen∝capacidad.
    Se demuestra la **independencia del camino** (subdividir+colapsar = identidad) para los tres.
  - **Árbol balanceado 2:1**: vecinos difieren ≤1 nivel → 3 casos de vecindad (igual/grueso/2 finas).
  - **La física apenas cambia**: ya es invariante a escala; la difusión del incr. 1 ya transfiere
    volumen por pares con razón de áreas → conservativa entre niveles distintos sin tocar nada.
  - **Driver sin agentes**: refinar por gradiente físico + costa/río/lago, con histéresis anti-parpadeo.
  - Nuevos parámetros `[lod]` (l_min/l_max/n_lod/grad_sub/h_hist); con `l_max=l_min` se recupera el
    incremento 1 exacto (test de no-regresión).
- **Estado tras el cambio:** Incremento 2 **diseñado, no implementado**. TREE actualizado. Próximo paso:
  implementarlo (árbol + operadores + vecindad multinivel + driver), con tests de independencia del
  camino, conservación e invariante 2:1. Pendiente confirmación del responsable para arrancar el código.

## [2026-11-04] Revisión del diseño del incremento 2 (v1.1) + git + comprobación de Grafana

- **Qué:** Al retomar la sesión (casi 4 meses después) se contrastó el diseño v1.0 del LOD dinámico con
  el código real del incremento 1, y aparecieron seis huecos. Se commiteó el borrador v1.0 tal cual
  (`d0e100c`), se fusionó `terreno` en `main` (fast-forward: incremento 1 + borrador) y se reescribió el
  diseño como **v1.1** (`docs/design/terreno/incremento-2-diseno.md`, con tabla "cambios respecto a la
  v1.0"). El responsable aprobó todas las propuestas ("sí a todo").
- **Por qué:** La v1.0 suponía cosas que el código no cumplía: (1) la altura se normalizaba con el
  min/max de toda la malla (no era función pura de la posición); (2) pendiente/suelo dependían de los
  vecinos y el recurso de un RNG secuencial; (3) nada decía qué hacer con el agua de hijas que al afinar
  la costa resultan ser mar; (4) "la red de ríos no se recomputa" era falso y faltaba histéresis en los
  rasgos; (5) vecinos en tabla fija de 8 y difusión asimétrica (usaba la celda de id menor: ~4× de error
  entre gruesa y fina); (6) con 6 octavas fijas, refinar más allá del nivel 2 no añadía relieve.
- **Decisiones (v1.1):** clave `u64` y búsqueda binaria (sin `HashMap`); estáticos = función pura de
  `(cara, nivel, i, j)` (normalización congelada, **una octava más por nivel**, pendiente por diferencias
  centrales, recurso por hash SplitMix64); derivados globales recalculados en cada cambio de malla
  (distancia al mar por Dijkstra); vecindad por **anillo al nivel L+1** (simétrica, CSR, peso lado/
  esquina); difusión **simétrica** (media armónica); reparto solo entre hijas de tierra con término
  **`lod_al_mar`** en el balance; umbrales físicos `dh_sub`/`dt_sub`/`dtheta_sub`; se separan
  determinismo e independencia del camino; no regresión = "LOD que no dispara ≡ LOD apagado".
- **Grafana (pendiente arrastrado):** no se pudo verificar. Los contenedores `abm-db`/`abm-grafana`
  llevan parados desde julio, y el puerto **5432 está ocupado por un proceso ajeno al proyecto** (no
  visible desde esta instancia; probablemente otra instancia WSL, que comparte red). Arrancar la BD
  del proyecto chocaría. El usuario ya está en el grupo `docker`. **Cuidado:** con `sink=postgres` el
  motor intentaría conectar a ese otro Postgres.
- **Estado tras el cambio:** Diseño v1.1 cerrado y aprobado; `main` = `terreno` en `d0e100c`.

## [2026-11-04] Implementación de la fase «terreno» — incremento 2 (LOD dinámico)

- **Qué:** Implementado el LOD adaptativo según el diseño v1.1. Módulos nuevos `statics.rs`,
  `hydro.rs` y `lod.rs`; reescritos `geometry.rs`, `world.rs`, `worldgen.rs`, `metrics.rs` (esquema
  v2) y `mod.rs`; difusión simétrica en `systems.rs`; sección `[lod]` en `config/terreno.toml`
  (`l_max = 3`), más `worldgen.octaves_per_level` y `vegetation.v_init` (`planet.l_unif` pasa a ser
  `lod.l_min`). La malla se subdivide en costas, ríos, lagos, relieve abrupto y contrastes de T/θ, y
  se colapsa con histéresis; equilibrio 2:1 con cascada y veto.
- **Por qué:** Incremento 2 de la pirámide del terreno: detalle donde aporta, sin pagar la malla fina
  entera.
- **Decisiones / problemas (cazados por los tests nuevos):**
  - **Parpadeo**: una familia colapsaba y su madre, al recoger el drenaje de las 4 hijas (185 151 km²
    frente a un umbral de 80 000), pasaba a ser río y se volvía a partir cada 2 pasos → **ensayo en
    seco**: el paso se aplica en una copia del mundo y se vetan los colapsos cuya madre volvería a
    subdividirse.
  - **No convergencia en el planeta completo** (~270 subdivisiones + ~270 colapsos por paso,
    indefinidamente): la red de ríos (receptor único) depende de la malla y la malla de los ríos →
    **marca hidrológica pegajosa** (lo que fue río/lago no se colapsa; las hijas la heredan).
    Converge en 13 pasos. Se retiró el colchón espacial de 1 celda de la v1.1 (redundante).
    Límite: con erosión (incr. 3) la marca necesitará caducidad.
  - **Génesis**: la tierra nueva nace seca y disparaba el criterio de θ → el refinamiento inicial usa
    solo criterios estáticos.
  - `gen` es palabra reservada en Rust 2024 → el campo se llama `statgen`.
- **Verificación:** `fmt` + `clippy -D warnings` limpios; `cargo test` en verde (11 unitarios +
  forrajeo 3 + terreno 14; BD `#[ignore]`). Salida idéntica byte a byte entre dos procesos y entre
  debug y release. Balance de agua con LOD a 2.7e-12. Config por defecto: génesis con **190 278
  celdas** (21,5 % de la malla uniforme fina), ~13 s y ~340 MB por año en release. Frente al
  incremento 1 (compilado aparte desde `cfc82ce`): misma T media (9,13 °C) y misma fracción de tierra
  (39 %); la nueva línea base uniforme deja la altura bit a bit igual y mueve los biomas menos de
  0,4 puntos.
- **Hallazgo de dinámica:** a nivel 3 (9 octavas) el relieve de ruido sin erosión tiene muchas
  hondonadas cerradas → ~9 700 celdas-lago (34 a nivel uniforme). Son lagos estáticos (tierra
  saturada para la física); se espera que la erosión del incremento 3 los reduzca.
- **Estado tras el cambio:** Incremento 2 **implementado y verificado** en la rama `terreno` (sin
  commitear). Documentos actualizados: diseño §8/§9/§13, nota en el diseño del incremento 1, README
  de la fase, README raíz, TREE. Pendiente: commit; incremento 3 (erosión/pedogénesis); Grafana
  (bloqueado por el puerto 5432).

## [2026-11-04] Infra en puertos nuevos + subred fija: Grafana verificado de punta a punta

- **Qué:** BD publicada en `localhost:25432` y Grafana en `http://localhost:23000` (antes 5432/3000),
  configurables en `infra/.env` (`PGPORT`, `GRAFANA_PORT`; plantilla en `.env.example`). El motor ya
  leía `PGPORT` (su valor por defecto pasa a 25432) y el test de BD también lo lee ahora. La red de
  compose tiene una **subred fija `10.88.42.0/24`**. Además, `Cargo.toml` declara `default-run = "abm"`.
  Commits: `94bc0d0` (terreno incr. 2) y el de esta entrada.
- **Por qué:** El responsable tiene varios puertos ocupados (5432, 5433, 5435, 55432, 55433, por
  otros proyectos o instancias) y pidió un puerto "random": se eligieron 25432/23000, libres y por
  debajo del rango efímero del sistema (32768–60999). Al levantar los contenedores salió un segundo
  problema: Grafana daba "connection reset" y ni la BD lo alcanzaba. Causa: **otra instancia WSL
  tenía una red de Docker con la misma subred (172.18.0.0/16)**. Las instancias de WSL2 comparten la
  red del kernel, así que había dos rutas iguales y el tráfico se iba a la otra red. Se fijó una
  subred fuera de los rangos por defecto de Docker. Lo de `default-run`: desde el binario `terreno`
  (julio), el `cargo run --release -- config/default.toml` del README fallaba ("could not determine
  which binary to run").
- **Verificación:** Grafana sano (v13.1.0), dashboard aprovisionado (logs) y Grafana → `db:5432`
  alcanzable; `cargo test --test persistence_pg -- --ignored` en verde contra el puerto 25432; corrida
  real con `sink = "both"` (run_id `45500c55…`) y la consulta del panel «Población» devuelve sus 500
  ticks (395 → 6697 → 4976). No se pudo entrar en la API autenticada de Grafana: la contraseña de
  admin ya no es la de fábrica (la cambió el responsable). Falta solo verlo en el navegador.
- **Estado tras el cambio:** Infra operativa en los puertos nuevos. `fmt` / `clippy` / `test` en verde.

## [2026-11-04] Diseño de la fase «terreno» — incremento 3 (relieve vivo), borrador v1.0

- **Qué:** Redactado `docs/design/terreno/incremento-3-diseno.md`: erosión fluvial, sedimento, difusión
  de ladera y suelo dinámico, compatibles con el LOD del incremento 2. Aún **sin código**. Además, `main`
  quedó al día con `terreno` (el responsable delegó los commits: "de los commits te encargas tú").
- **Por qué:** Es el siguiente incremento del plan de la fase, y arregla un defecto que perjudicaría a
  los agentes: ~9 700 celdas-lago falsas (hondonadas del ruido). Para la interfaz de agentes, el agua
  es infranqueable y cuenta como fuente.
- **Decisiones de diseño (propuestas):**
  - `h = h0 + e`: `h0` puro (incr. 2) y `e` dinámico, que el LOD copia y promedia (conserva `Σ e·a`).
  - Incisión por **potencia de arroyo implícita** (Braun y Willett 2013), incondicionalmente estable
    y monótona, con nivel de base = mar o superficie del lago.
  - Sedimento que rellena depresiones (balance exacto erosionado = depositado + al mar).
  - Difusión de ladera por pares con subpasos automáticos.
  - Suelo `z` con `W_cap = W_cap0·z/z_init` (bit a bit el incr. 2 cuando `z = z_init`).
  - Parámetros en unidades por año, no por tick.
  - Prueba: la tierra nunca baja del nivel del mar por erosión, así que la erosión no mueve agua.
- **Hallazgo a corregir al implementar:** el reparto del LOD (`split_state`) manda al mar nieve y
  biomasa si las hijas de tierra suman `W_cap = 0`. Con suelo dinámico (roca desnuda) eso pasaría.
- **Pendiente de confirmar por el responsable (§14 del diseño):**
  1. dos regímenes de tiempo (génesis geológica de ~1 millón de años + erosión realista en ejecución);
  2. reparto incr. 3 / incr. 4 (lagos dinámicos, D-infinito, fertilidad dinámica, eventos → incr. 4);
  3. caudal ponderado por lluvia;
  4. mar como sumidero de sedimento (sin deltas).
- **Estado tras el cambio:** Incremento 3 **diseñado, no implementado**. TREE y README de la fase
  actualizados. Próximo paso: confirmar las 4 decisiones e implementar.

## [2026-11-05] Implementación de la fase «terreno» — incremento 3 (relieve vivo)

- **Qué:** Implementado el relieve vivo según el diseño, cuyas 4 decisiones confirmó el responsable
  ("sí a todo"):
  - módulo nuevo `src/terreno/erosion.rs`: incisión por potencia de arroyo implícita, sedimento que
    rellena depresiones, difusión de ladera con subpasos y suelo dinámico;
  - **génesis geológica** (200 pasos × 5 000 años = 1 millón de años, en 2 rondas erosión ↔ LOD);
  - erosión en ejecución cada 365 ticks a ritmo real;
  - `h = h0 + e` repartido por el LOD (copia/media que conserva `Σ e·a`);
  - config `[erosion]` y nuevos parámetros de `[soil]`; esquema de salida v3;
  - `tests/erosion.rs` (9 tests).
- **Por qué:** Incremento 3 del plan. Elimina los ~9 700 lagos falsos del ruido (quedan 30), que
  habrían distorsionado a los agentes.
- **Decisiones / problemas (cazados por las corridas de calibración y los tests):**
  - **La capacidad `W_cap ∝ z` sin límites rompía el LOD.** La roca desnuda (10 % de la tierra) daba
    θ = 0, y los lagos rellenados, hasta 136 m de agua. El criterio θ pidió 14 510 subdivisiones
    de golpe. → `W_cap = W_cap0·clamp(z/z_init, f_rock, z_root/z_init)`, con `z_root = 1.5 m` y
    `f_rock = 0.3`; calibrado `k_weath = 1e-4` m/año.
  - **El detalle fino nuevo no está erosionado.** Cada subdivisión en ejecución traía hondonadas →
    lagos → rasgos → más afinado (805 → 5 100 lagos en un año). → **Acondicionamiento**: un cambio
    de malla no puede crear hondonadas; se rellenan hasta el derrame y se apuntan en `fill_m3`.
    Solo con el relieve vivo activo.
  - **Faltaba apuntar el relleno del refinado inicial** en el balance de material: lo cazó el test
    del balance global (1,7 % de descuadre).
  - **El veto 2:1 es conservador** con colapsos vecinos simultáneos (necesita varias pasadas): es
    seguro, y el test lo tiene en cuenta.
  - Corregido `split_state`: con `W_cap = 0` la nieve y la biomasa ya no se pierden en el mar.
- **Verificación:** `fmt` + `clippy -D warnings` limpios; `cargo test` en verde (12 unitarios +
  forrajeo 3 + terreno 14 + erosión 9; BD ignorado y verde con `--ignored`).
  - **No regresión:** con la erosión apagada, 0 diferencias en 7 300 valores frente al binario del
    incremento 2.
  - **Determinismo:** idéntico byte a byte entre procesos.
  - **Conservación:** residual de material de la génesis de 31 m³ sobre 1,8·10¹⁶; agua ≤ 5·10⁻¹² en
    3 años.
  - **Resultados:** elevación media de la tierra 491 → 406 m; tierra 39,3 → 39,0 %; T +0,2 °C; 9
    biomas; ~1,8·10¹⁰ m³/año de sedimento en ejecución (orden de la Tierra real); malla que converge
    en 3 años.
  - **Coste:** ~38 s y ~390 MB por génesis + año en release.
- **Estado tras el cambio:** Incremento 3 **implementado y verificado**. Siguiente del plan: incremento
  4 (lagos dinámicos, flujo multidireccional, fertilidad dinámica, eventos) o los agentes en el
  planeta.

## [2026-11-05] Revisión externa del incremento 3: relleno exacto, suelo mínimo por nivel, independencia del camino, tope y caché

- **Qué:** El responsable trajo una revisión del incremento 3 ("tres ajustes antes de seguir", dos
  protecciones y dos notas). Hecho:
  1. **Acondicionamiento exacto.** El material del relleno sale del borde de cada grupo de hondonadas
     (bajado por igual, sin pasar de la cota de derrame); si el borde no alcanza, se rellena en
     proporción. Solo actúa con relieve ya erosionado. Lo no compensado es redondeo (−7,9·10⁻⁶ m³
     sobre 2,2·10¹³ m³).
  2. **Test de independencia del camino** con dinámica: (a) fina, (b) gruesa→fina→gruesa,
     (c) gruesa.
  3. **Suelo mínimo por nivel**: `z_mín(L) = z_init·f_rock·2^(−β·L)` (β = 0.5).
  4. **Tope de subdivisiones** por revisión (`lod.max_sub_frac = 0.02`, las más urgentes primero).
  5. **Caché de la génesis** (`run.genesis_cache`, `src/terreno/cache.rs`; huella del código en
     `build.rs`): 36,9 s → 8,3 s, y el mundo se recupera bit a bit.
  - Ya existían: la histéresis (`h_hist = 0.5`) y la formación de suelo decreciente con la
    profundidad.
- **Por qué:** Conservación exacta en cada corte (regla 1 del responsable), demostrar que la malla
  adaptativa no deja huella, y quitar el artefacto del 30 % de roca a escala fina.
- **Hallazgos:**
  - **A. El ajuste 3 chocaba con la prueba 2.** Como "capacidad mínima", al subdividir se perdía agua
    que no volvía al colapsar (refinar la cara 0 bajaba su capacidad un 10,5 %). → Reformulado como
    **profundidad mínima de suelo** que las hijas heredan.
  - **B. Efecto de escala de la física base.** Con la zona fina llueve +4,0 % (a) / +1,4 % (b) frente
    a (c), por la asimetría del factor orográfico con pendientes más fuertes; la incisión arranca más
    suelo en el tramo fino. (b) vuelve a comportarse como (c): no es huella de los operadores. Ya
    existía en el incremento 2 (+3 % de lluvia). Arreglarlo (parametrizaciones de pendiente a escala
    de referencia) cambiaría resultados del incremento 2 → **pendiente de decisión del responsable**.
  - Tolerancias del test fijadas sobre lo medido: (b)/(c) agua < 1 %, material < 0,1 %, suelo < 4 %,
    erosión < 1 %; (a)/(b) agua < 6 %, suelo < 5 %.
- **Verificación:** `fmt` + `clippy -D warnings` limpios; `cargo test` en verde (12 + 3 + 14 +
  erosión 13). No regresión frente al binario del incremento 2: 7 300 valores, 0 diferencias.
  Resultados: 52 lagos tras la génesis, tierra 39,3 → 38,9 %, ~204 700 celdas al final del año,
  agua ≤ 2,6·10⁻¹².
- **Estado tras el cambio:** Terreno listo para recibir agentes. Próximos pasos acordados:
  - el responsable rehace la **biología de los agentes por días** (vida, metabolismo, apetito,
    rebrote: es su diseño);
  - Claude divide el código en **paquetes** (CLAUDE §3).

## [2026-11-05] Fase «paquetes»: el código dividido en el workspace de crates (CLAUDE §3)

- **Qué:** El paquete único `abm` pasa a ser un **workspace de seis paquetes** en `crates/`:
  - `abm-core`: `Tick` + `run_ticks`, planificador de un solo hilo, `RootRng`, `ConfigError`;
  - `abm-ruleset`: reglas del forrajeo, config, génesis, orden del tick;
  - `abm-terreno`: el planeta, con su `build.rs` y sus tests;
  - `abm-metrics`: `collect_metrics`;
  - `abm-persistence`: esquema de métricas, trait `MetricsSink`, JSON, PostgreSQL, migraciones;
  - `abm-runner`: binarios `abm` y `terreno`, `run_simulation`, tests de determinismo y de BD.

  Versiones de dependencias externas fijadas una vez en `[workspace.dependencies]`. Ficheros movidos
  con `git mv` (conservan historial). `CLAUDE.md` §3 actualizado con `abm-terreno`, opción
  confirmada por el responsable ("sí, me parece bien"). Nueva carpeta `docs/design/paquetes/`.
- **Por qué:** La base es estable y el siguiente paso es juntar agentes y planeta. La regla de
  dependencias pasa de norma a vigilar a garantía del compilador.
- **Decisiones / problemas — tres nudos que el paquete único escondía:**
  1. **El motor conocía el dominio:** montaba el orden de los sistemas del forrajeo y creaba el mundo
     con sus agentes y su `Sink`. → El orden del modelo y la génesis van a `abm-ruleset`; el motor
     da `schedule::deterministic()` y `run_ticks`; el `Sink` lo inserta `abm-runner`.
  2. **Ciclo métricas ↔ persistencia** (las métricas escribían en el `Sink` de la persistencia y la
     persistencia usaba el esquema de las métricas). → Esquema y trait en `abm-persistence`, como
     decía CLAUDE §7.
  3. **La BD recibía la `Config` entera del forrajeo.** → `pg_sink::write_run(&RunRecord, &doc)`,
     con solo los datos que guarda; la versión del ruleset sale de `abm_ruleset::VERSION`.

  Además:
  - un error por paquete (`ConfigError` en core; `PersistenceError`, `TerrenoError`, `SimError`);
  - los tests leen `config/default.toml` con una ruta anclada al workspace, porque Cargo los corre
    desde la carpeta del paquete;
  - `cargo run --release -- config/default.toml` sigue lanzando `abm` (`default-run` del runner);
  - la huella del código del terreno cambió con las rutas, así que la caché de la génesis se rehace
    una vez.
- **Verificación:** `fmt` + `clippy --workspace --all-targets -D warnings` limpios; `cargo test`
  desde la raíz recorre los seis paquetes en verde (12 + 3 + 14 + 13; BD `#[ignore]`, que pasa con
  `--ignored`). Dependencias comprobadas con `cargo tree` (todas apuntan a `abm-core`, que no depende
  de nadie). Los dos binarios dan una salida **idéntica byte a byte** a la de antes de la división
  (`baseline.json` y `terreno_terreno.json`).
- **Estado tras el cambio:** Workspace operativo. Siguiente: el responsable rehace la biología de los
  agentes por días; después, acoplar agentes y planeta.

## [2026-11-05] Nota de lluvia orográfica: relieve de referencia a 25 km y distancia de humedad desde la costa

- **Qué:** El responsable entregó dos documentos en `docs/temp/`. Se archivaron con git y `docs/temp`
  se borró:
  - `nota_terreno_lluvia_orografica.md` → `docs/design/terreno/`;
  - `diseno_biologia_agentes.md` → `docs/design/agentes/` (se trabaja en la entrada siguiente).

  Se implementó la nota:
  - `climate.d_oro_km = 25` y nivel de referencia `L_oro`, el de lado más cercano a `d_oro`.
  - Relieve de referencia en un punto = relieve de nacimiento del nivel `L_oro` + erosión `e` vista a
    ese nivel (media por área de los descendientes del antepasado `L_oro`). Diferencias centrales
    con paso `d_oro`.
  - Las celdas más gruesas que la referencia usan su propia pendiente.
  - Columnas `oro0_e/n` (parte pura, al crear la celda) y `oro_e/n` (con la erosión). Se refrescan
    dentro de `erosion::step` y tras cada cambio de malla con `e ≠ 0`. La caché pasa al formato
    `ABMGEN02`.
  - Función pública `erosion::orographic_gradient_at`.
- **Por qué:** En el incremento 3 la lluvia dependía de la resolución (+4 % con la zona fina). Con
  los agentes, eso cerraría un bucle artificial (Foco → malla fina → más lluvia → más forraje → más
  agentes → más Foco).
- **Decisiones / problemas:**
  - **Hallazgo 1, mi diagnóstico del incremento 3 era erróneo.** Con `k_oro = 0` la diferencia es
    idéntica. La causa era la distancia de humedad, medida hasta el **centro** de la celda de mar:
    sobraba media celda y, al afinar, la costa "se acercaba" (+16,9 / +8,4 / +3,1 % de lluvia por
    área de tierra al afinar de 834/417/208 km). Corregido con `climate.moisture_from_coast = true`:
    la distancia se mide desde la línea de costa. Es una decisión técnica mía, reversible por config.
    A la resolución real queda +0,03 % en la lluvia de la zona (−0,7 % por área). Nota corregida en
    `incremento-3-diseno.md` §16.
  - **Hallazgo 2, con `k_oro = 0,5` no hay sombra pluviométrica:** el factor queda en 0,99–1,01
    para el 98 % de la tierra. Barrido: con `k_oro = 40`, percentiles 1–99 de 0,36–1,66 y la
    lluvia total +0,14 %. Propuesta: `k_oro ≈ 40`, **pendiente del responsable** (pregunta abierta
    §7 de la nota). Se mantiene 0,5.
  - **Hallazgo 3, la erosión depende de la resolución:** 1,1 / 1,9 / 3,0 % en la prueba de
    independencia. El 0,3 % de antes era una compensación con el exceso de lluvia. Queda vigilada
    (nota §4), sin cambios.
  - Con la config por defecto, `L_oro = 3 = l_max`. La corrección orográfica apenas cambia nada hoy;
    actuará cuando el Foco afine por debajo de 26 km.
  - **Fallo latente:** `serde_json` no leía los `f64` de forma exacta, y el resumen de la génesis
    guardado en la caché podía volver con 1 ulp de diferencia. Lo destapó
    `la_cache_reproduce_la_genesis_bit_a_bit`. Se activó `float_roundtrip` en el workspace.
  - Coste: la génesis por defecto pasa de 36 a 42 s, una sola vez gracias a la caché.
- **Verificación:**
  - **No regresión:** con `d_oro_km = 0` y `moisture_from_coast = false`, salida **idéntica byte a
    byte** al binario de `d7bd4a3`, con erosión y sin ella.
  - **Tests nuevos:** `el_relieve_orografico_no_depende_de_la_malla` (invariancia < 1e-9) y
    `la_lluvia_no_depende_de_la_resolucion` (+0,03 %, antes +3,86 %).
  - La prueba de independencia del camino mide ahora la lluvia de la zona: (b)/(c) 6,2 → 1,4 %;
    (a)/(c) 16 → 3,6 %. Tolerancias ajustadas y documentadas.
  - `fmt`, `clippy --workspace --all-targets -D warnings` y `cargo test` en verde (12 + 3 + 15 + 14).
  - Nueva salida de referencia: lluvia +0,40 %, desierto −1,6 %, selva +3,9 %.
- **Estado tras el cambio:** Terreno con la lluvia independiente de la resolución. Pendiente del
  responsable: `k_oro`. Siguiente: biología de los agentes (incremento 1).

## [2026-11-05] Fase «agentes», incremento 1: la biología por días con individuos en un hábitat de prueba

- **Qué:** El responsable entregó el diseño `diseno_biologia_agentes.md` (v1.0), ahora en
  `docs/design/agentes/`. Se partió en cuatro incrementos y se hizo el 1, con su diseño técnico
  (`incremento-1-diseno.md`) y el README llano de la fase. Código nuevo:
  - `abm-ruleset/src/biologia/`:
    - especie por masa con leyes de escala y preset humano (Kleiber, West, Siler, leche, `N_max`);
    - azar por contador `H(semilla, id, tick, propósito, índice)` y `SumaExacta` (punto fijo `i128`);
    - componentes del individuo y hábitat de prueba (zonas de clima constante, PPN de Miami, forraje
      logístico por celda y especie);
    - sistemas T4 y A3–A9 (forraje, demanda y reparto justo, energética, crecimiento, reproducción
      sexual con aborto/parto/destete, mortalidad con inanición/edad máxima/Siler/orfandad,
      envejecimiento, escrituras), génesis y orden del tick.
  - `abm-metrics/src/biologia.rs`: recuentos por registro, balance de energía, tablas de vida del
    periodo (e0), TGF, intervalo entre partos y pirámides anuales.
  - `abm-persistence/src/schema_bio.rs`: `BioDoc`.
  - `abm-runner`: `run_biologia(_con_orden)`, binario `agentes`, `tests/biologia.rs`.
  - `config/agentes.toml` y `config/experiments/agentes_proporcional.toml`.

  El modelo de forrajeo antiguo sigue intacto.
- **Por qué:** Con 1 tick = 1 día, la biología antigua (vida de 200 ticks, metabolismo 2, apetito 4,
  rebrote 1) no tenía sentido. Pirámide invertida: la biología se valida sola antes de tocar el
  planeta.
- **Decisiones / problemas:**
  - **D1 (a confirmar):** el apetito `Ap = k_ap·B·PAL` se lee en energía asimilable,
    `d = min(Ap, E_max − E + G)/ε`. Al pie de la letra (forraje bruto), con `ε·k_ap = 1` el genérico
    no podría crecer, criar ni rehacer reservas, y la regla `k_ap > 1` del diseño no se cumpliría.
  - **Calibración de Siler** (la dejaba "a calibrar"):
    - humano: Gurven y Kaplan (2007) con `β3 = 0,085` → e0 30,9 años, l(15) 0,564, moda adulta 69;
    - genérico: tasas relativas a `a_max` (escalan con la masa), l(1) = 0,600 y mortalidad adulta
      0,104–0,120.
  - Otras interpretaciones D2–D15 en el diseño del incremento (gasto de crecimiento decidido al
    principio del tick, leche de la madre, reserva negativa del que muere de hambre, `N_max` con el
    gasto adulto, edades iniciales ∝ l(a), etc.).
  - **Hallazgo A (importante):** con el reparto del diseño (§6.4, todo el stock accesible), la
    población crece bien alimentada, supera `N_max`, agota el forraje y colapsa. El herbívoro se
    extingue en 13 años con clima constante.
  - **Propuesta D16** (`forraje.respuesta = "proporcional"`, demanda × `V/K`): la población se regula
    sola en ~1,2·`N_max` (las crías gastan menos que el adulto). La densidad sigue a la productividad
    en 4 biomas, y la equivalencia energética entre 10 y 300 kg sale con cociente 1,004 (Damuth). **Por
    defecto se mantiene el diseño (`"total"`) hasta que decida el responsable.**
  - **Hallazgo B:** las hambrunas matan sobre todo a las hembras (de 5 452 quedan 2, frente a 3 159
    machos, con una sequía del 50 % durante un año), y la población no se recupera. No hay
    "destete forzoso" equivalente al aborto. En equilibrio, 56 % de machos. Propuesta para el
    responsable.
  - La fecundidad humana sale de ~10 hijos por mujer (real 5–6): falta que baje con la edad antes de
    los 50.
- **Verificación:**
  - `fmt`, `clippy --workspace --all-targets -D warnings` y `cargo test` en verde: biología 6
    unitarios + 8 de simulación, métricas 1, y el resto sin cambios.
  - Balance de energía cerrado a ~2·10⁻⁶ kcal por registro.
  - Salida idéntica entre procesos y entre debug y release, también con la población creada en otro
    orden.
  - Tabla de vida simulada de una cohorte de 2 000 dentro del error de muestreo.
  - 30 años con ~10 000 individuos, ~4 s.
- **Estado tras el cambio:** Incremento 1 de «agentes» hecho. Pendiente del responsable: D1, D16,
  destete forzoso y fecundidad humana por edad (y `k_oro` del terreno). Siguiente: incremento 2
  (cohortes y conversión individuo ↔ población).

## [2026-11-05] Nota de sombra pluviométrica: k_oro calibrado, agotamiento de humedad a favor del viento y c_P

- **Qué:** El responsable entregó dos notas en `docs/temp/`. Se archivaron y `docs/temp` se borró. No estaban bajo git: se movieron y se añadieron en su destino.
  - `nota_terreno_sombra_pluviometrica.md` → `docs/design/terreno/` (esta entrada);
  - `nota_agentes_revision_1.md` → `docs/design/agentes/` (entrada siguiente).

  Rama `terreno-sombra`. Se implementó la nota con un interruptor por cambio en `[climate]`:
  - **Parte A:** `k_oro`, por defecto **150**, calibrado por el objetivo de la nota (la nota proponía 40).
  - **Parte A:** `conservar_lluvia` + `tol_lluvia`. Factor `c_P` fijado al acabar la génesis, solo sobre la tierra. Su referencia es la lluvia sin efecto orográfico ni agotamiento.
  - **Parte B:** `agotamiento_humedad` + `h_v_m`, `l_recarga_km`, `l_traj_km`. Módulo nuevo `abm-terreno/src/humedad.rs`:
    - `Λ` sobre **anillos**: paralelos y puntos cada `d_oro`, porque el viento del modelo es zonal;
    - el relieve es el de referencia del nivel `L_oro`, el mar se toma donde ese relieve queda bajo el nivel del mar, y el tramo de la costa cuenta desde el nivel del mar;
    - cada celda recibe `w = e^(−Λ/H_v)`: interpolado con puntos de tierra en las de nivel `L_oro` o más finas, y la media sobre la huella (8×8) en las gruesas;
    - `w` se calcula al acabar la génesis y se rehace con cada paso de erosión y cada cambio de malla.
  - **`mar_plano`** (decisión técnica mía): el aire ve el mar plano.

  Otros cambios:
  - caché `ABMGEN03`, que guarda `w` y `c_P`;
  - `run.dump_climate`: el volcado añade `m_oro`, `w` y la lluvia anual;
  - `run.lluvia` en los metadatos;
  - `config/experiments/terreno_sin_sombra.toml` con todo apagado.
- **Por qué:** con `k_oro = 0,5` no había sombra de lluvia. La fórmula local tampoco podía crear los desiertos de detrás de las cordilleras, que son un objetivo de la visión madre.
- **Decisiones / problemas:**
  - **`k_oro = 40` no llega al objetivo** (media de `m_oro` 2–3 en las laderas de barlovento de las cordilleras de más de 2 000 m). Da 1,37: a 25 km las montañas del planeta suben ~9 m/km, frente a los 40 de las reales. Barrido 40–240 con dos definiciones de ladera: 150 da 2,13–2,18. Lo decide el responsable (S1).
  - **El agotamiento también seca el barlovento,** porque `Λ` incluye la subida de la propia ladera:
    - en el planeta, el contraste barlovento/sotavento en latitudes medias es 2,2 (la nota esperaba 5–20);
    - en el escenario de juguete de la nota es 7,2: pie ×3,0 y sotavento a 200–500 km ×0,42–0,54 (la nota esperaba 0,2–0,4, por la recarga con `L_recarga` = 1 000 km);
    - sensibilidad en la nota §6.3: con `H_v = 1 500` m se cumpliría. Decisiones S2 y S3.
  - **Sin `c_P`, el agotamiento quita el 16 % de la lluvia de la tierra;** `c_P = 1,18` la recupera.
  - **Hallazgo: el fondo marino contaba como relieve.** Con `k_oro` grande movía más de un 10 % la lluvia del 86 % del océano (+2,7 % en total) y la de las costas. `mar_plano` lo arregla; sobre la tierra cambia +0,1 %. Decisión S4.
  - **La lluvia al afinar** (208 → 104 km) cambia +0,58 % con todo encendido (antes +0,03 %), pero `w` es invariante (−2·10⁻⁵ en el interior): el cambio sale de la costa. De 52 a 26 km (la escala del Foco), 0,00 %. En la prueba de independencia del camino (celdas de 834/417 km) la lluvia difiere hasta un 6,7 %, porque esas celdas usan su propia pendiente. Tolerancias de los tests ajustadas y documentadas.
- **Verificación:**
  - **No regresión:** con los cuatro interruptores apagados, salida **idéntica byte a byte** al binario de ece0868 (JSON con erosión y sin ella, y volcado por celda), compilado en un worktree del scratchpad.
  - **Tests nuevos:** `la_suma_se_corta_en_el_mar_y_cuenta_la_costa`, `escenario_de_juguete_de_la_nota`, `en_un_mundo_plano_la_sombra_pluviometrica_no_cambia_nada`; `la_lluvia_no_depende_de_la_resolucion` comprueba también `w`; el de la caché compara `w` y `c_P`.
  - **Comprobaciones:** `fmt`, `clippy --workspace --all-targets -D warnings` y `cargo test --workspace` en verde (terreno 14 + 15 + 15).
  - **Planeta frente a la referencia:**
    - lluvia sobre tierra +0,8 %; desierto 5,60 → 5,95 %, selva 2,07 → 2,79 %, bosque templado 6,83 → 5,61 %; lagos 51 → 57;
    - a sotavento de las cordilleras grandes en latitudes medias: lluvia 688 → 231 mm y desierto 7,6 → 15,8 %;
    - PPN (Miami) y capacidad de carga −2,9 %.
- **Estado tras el cambio:** sombra pluviométrica activa. Pendientes del responsable: S1–S5 (nota §6.7). Siguiente: revisión 1 de la biología de los agentes.
