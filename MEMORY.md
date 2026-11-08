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
  (`505ff11`), se fusionó `terreno` en `main` (fast-forward: incremento 1 + borrador) y se reescribió el
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
- **Estado tras el cambio:** Diseño v1.1 cerrado y aprobado; `main` = `terreno` en `505ff11`.

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
  Commits: `4e42afa` (terreno incr. 2) y el de esta entrada.
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
    byte** al binario de `a7e4ba0`, con erosión y sin ella.
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
  - **No regresión:** con los cuatro interruptores apagados, salida **idéntica byte a byte** al binario de f6e34e6 (JSON con erosión y sin ella, y volcado por celda), compilado en un worktree del scratchpad.
  - **Tests nuevos:** `la_suma_se_corta_en_el_mar_y_cuenta_la_costa`, `escenario_de_juguete_de_la_nota`, `en_un_mundo_plano_la_sombra_pluviometrica_no_cambia_nada`; `la_lluvia_no_depende_de_la_resolucion` comprueba también `w`; el de la caché compara `w` y `c_P`.
  - **Comprobaciones:** `fmt`, `clippy --workspace --all-targets -D warnings` y `cargo test --workspace` en verde (terreno 14 + 15 + 15).
  - **Planeta frente a la referencia:**
    - lluvia sobre tierra +0,8 %; desierto 5,60 → 5,95 %, selva 2,07 → 2,79 %, bosque templado 6,83 → 5,61 %; lagos 51 → 57;
    - a sotavento de las cordilleras grandes en latitudes medias: lluvia 688 → 231 mm y desierto 7,6 → 15,8 %;
    - PPN (Miami) y capacidad de carga −2,9 %.
- **Estado tras el cambio:** sombra pluviométrica activa. Pendientes del responsable: S1–S5 (nota §6.7). Siguiente: revisión 1 de la biología de los agentes.

## [2026-11-05] Revisión 1 de la biología de los agentes: apetito por gasto total, Holling, lactancia y dimorfismo, esterilidad

- **Qué:** Se implementó `docs/design/agentes/nota_agentes_revision_1.md` en la rama `agentes-revision1`, solo para individuos. Cuatro cambios, cada uno con su interruptor:
  1. `energetica.apetito_gasto_total`: `Ap = k_ap · G_total`. D1 queda confirmada.
  2. `forraje.respuesta_funcional`: Holling II con `h_½ = (k_ap − 1)/2` por especie. **Sustituye** a `forraje.respuesta`, D16; la variante proporcional y `agentes_proporcional.toml` se retiran.
  3. Lactancia y sexos:
     - `leche_por_condicion` (`r_lact`, `r_aband`);
     - `destete_forzoso` (`f_dest_min`), con la causa de muerte nueva "destete forzoso";
     - `dimorfismo`: masa, grasa y crecimiento por sexo; el preset humano: 56/64 kg y 25/15 % de grasa.
  4. `esterilidad_individual`: `a_est ~ Normal(38, 4)` truncada, por contador, fijada al nacer.

  Otros cambios:
  - **Métricas opcionales** (`[metricas] demografia`): fecundidad por grupos de 5 años, edad del último parto, mortalidad por grupo de edad y sexo, y condición de las lactantes. Todos los campos nuevos de la salida son opcionales y no se escriben con su interruptor apagado (formato v1 intacto).
  - **Configuración:** `config/experiments/agentes_incremento1.toml` con todo apagado.
  - **Modo población:** su versión de cada cambio queda anotada para el incremento 2 (nota §8.7).
- **Por qué:** los hallazgos del incremento 1: la población no se regulaba, las hambrunas acababan con las hembras y la fecundidad humana era de ~10 hijos.
- **Decisiones / problemas:**
  - **Holling regula.** Sin él, el herbívoro se extingue siempre en el año 13; con todo, 1,23·`N_max` (la nota pide 0,7–1,2) y humanos 0,90. Se mantienen la densidad frente a la PPN en 4 biomas, la equivalencia energética 10/300 kg (cociente 1,000) y Damuth (12,8). Con un liquen (τ_c = 20 años): sobrepasamiento de 2,5× y colapso.
  - **El cambio 1 solo se nota con comida escasa** (con abundancia, el apetito antiguo ya pagaba la leche). En equilibrio quita el hambre (1 045 muertes/año → 0), pone `V/K = 0,50` y es lo que evita el colapso de hembras del herbívoro en la sequía (31 % de adultas frente al 0,04 %).
  - **El cambio 3 casi no actúa en el herbívoro:** su sobremortalidad femenina viene de la gestación. Subir `r_aband` a 0,25 no cambia nada. En humanos, el dimorfismo hace que en una hambruna mueran menos mujeres que hombres (0,08 frente a 0,30 al año), como en la realidad.
  - **Hallazgo: hambre sincronizada.** Con Holling, lo que come cada uno depende del forraje que queda y no de cuántos comen. En una sequía, todos los de una celda mueren a la vez, y las celdas pequeñas (o con la sequía durante el sobrepasamiento) se extinguen enteras.
  - **Fecundidad humana:** con `μ_est = 38` salen 6,6 hijos y el último parto a los 36,7 años (el objetivo era 37–40), con una caída por edad más temprana que la de Coale-Trussell. Con 40 sale la forma, pero 7,3 hijos. Ningún valor cumple los tres objetivos.
  - **`e0` humana de 21–25 años** en equilibrio: el 29 % de las muertes, por hambre.
  - **Optimización:** sortear `a_est` cada día costaba un 35 %; se fija al nacer (`Repro.a_fin`) y la salida sale idéntica byte a byte.
  - **Decisiones pendientes del responsable:** A1–A7 (nota §8.9).
- **Verificación:**
  - **No regresión:** con todo apagado, salida **idéntica byte a byte** al binario de f6e34e6 (30 años por defecto).
  - **Tests:** 2 unitarios nuevos y 7 de simulación nuevos o reescritos. `fmt`, `clippy --workspace --all-targets -D warnings` y `cargo test --workspace` en verde (biología: 8 unitarios + 15 de simulación, 45 s en debug).
  - **Medidas:** campaña de 30 corridas (cada cambio solo, todos, todos menos uno, sequías, biomas, equivalencia, liquen, barrido de `μ_est`), en la nota §8.
- **Estado tras el cambio:** biología con la revisión 1. Siguiente: decisiones A1–A7; después, el incremento 2 (poblaciones).

## [2026-11-05] Herramienta de no regresión (primer paso de los 5 puntos de escalabilidad)

- **Qué:** `herramientas/no_regresion.sh REF BINARIO CONFIG_REF CONFIG_ACTUAL`:
  - compila el binario del commit `REF` en un worktree temporal;
  - ejecuta cada binario en su propia carpeta (sin mezclar `output/` ni cachés);
  - compara los JSON byte a byte, salvo el nombre de la corrida, y sale con 0 si coinciden y con 1 si no.

  Probada con las dos notas del día (`f6e34e6` agentes, `6031705` terreno): idénticas. Con la config encendida, detecta la diferencia.
- **Por qué:** el responsable preguntó si el método escalará con ~100 sistemas y pidió soluciones para los cinco puntos que salieron: interruptores permanentes, no regresión, interacciones entre sistemas, herramientas de medida y rendimiento. Esta es la parte que no necesita decisión suya: la comprobación manual de hoy, ya repetible en un solo comando.
- **Estado:** el resto de los cinco puntos queda propuesto al responsable. Retirar los interruptores tras cada decisión y crear una batería de escenarios de referencia tocan las convenciones de CLAUDE.md y necesitan su confirmación.

## [2026-11-05] Realismo primero y retirada de los interruptores decididos

- **Qué:**
  - CLAUDE.md recoge, confirmado por el responsable:
    - **§2.8 realismo primero:** si una calibración no llega a un objetivo realista, primero se duda de la base, no se fuerza un parámetro;
    - **§5:** interruptores solo mientras se mide; tras la decisión, Claude los retira sin preguntar; todo lo medido se puede consultar;
    - **§10.5:** batería de escenarios de referencia;
    - **§2.1:** herramientas fuera del motor en Python o bash.
  - Se retiraron todos los interruptores ya decididos:
    - terreno: `moisture_from_coast`, `agotamiento_humedad`, `conservar_lluvia`, `mar_plano` (queda `tol_lluvia`); `d_oro_km` ≥ 10;
    - agentes: `apetito_gasto_total`, `respuesta_funcional`, `leche_por_condicion`, `destete_forzoso`, `dimorfismo`, `esterilidad_individual` y `metricas.demografia`.
  - Los campos nuevos de la salida pasan a ser normales (siempre presentes). Se borran las configs "todo apagado" de `config/experiments/`.
- **Por qué:** el responsable no quiere mantener ni decidir interruptores ("si no es imprescindible y molesta, se va fuera"). La referencia del pasado es el commit anterior, con `herramientas/no_regresion.sh`.
- **Verificación:** la salida por defecto es **idéntica byte a byte** a la de `b15dfee` (agentes y terreno). `fmt`, `clippy --workspace --all-targets -D warnings` y `cargo test --workspace` en verde; el test de biología del diseño v1.0, que ya no tiene camino, se retira (14 de simulación).

## [2026-11-05] Decisiones del responsable sobre la sombra pluviométrica (S1–S5) y la revisión 1 de agentes (A1–A7)

- **Qué:** criterio del responsable: lo más realista (CLAUDE §2.8).
  - **Terreno** (nota de sombra §7):
    - `k_oro` vuelve a su valor físico, **40**: el problema es el relieve, no `k_oro`;
    - `L_recarga` = **3 000 km**;
    - el secado del aire usa la **altura de saturación**: el aire solo pierde vapor al subir por encima de lo que ya subió, y las lomas no lo secan;
    - el mar plano se mantiene;
    - `c_P` queda como **apaño provisional** hasta calibrar la lluvia con datos de la Tierra.
  - **Agentes** (nota de revisión §9):
    - `N_max` cuenta la **población real**: gasto medio de la población estable, crías incluidas, sustituyendo a D6;
    - `f_com` del herbívoro pasa de 0,004 a **0,00324**, para que la densidad de equilibrio siga la ley de Damuth;
    - fecundidad humana: `μ_est` = **40**, `p_conc` = **0,005**;
    - la variante "proporcional" (D16) queda retirada.
  - **Propuesta de relieve realista** (`docs/design/terreno/propuesta_relieve_realista.md`), con diagnóstico y opciones; recomienda tectónica de placas.
  - **Pendientes obligatorios de realismo** en TREE.md: relieve; competencia con jerarquía; coste del celo y estacionalidad; otras causas de muerte; fecundabilidad por edad; individuos persistentes (P6).
- **Por qué:** el responsable pidió no construir nada irreal aunque cumpla un objetivo escrito: "habría que haber dudado del relieve desde el principio".
- **Decisiones / problemas:**
  - **El relieve no es realista.**
    - La tierra está a 407 m de media, frente a ~840 en la Tierra.
    - El 68 % está por debajo de 200 m, frente a ~25–30 %.
    - Las laderas de barlovento tienen una pendiente de ~0,009, frente a 0,02–0,05.
    - La curva `lf³` aplasta los continentes, y el ruido igual en todas direcciones da manchas en vez de cordilleras.
    - Con `k_oro` físico, el barlovento recibe solo ×1,37 de lluvia (objetivo ×2–3). Se deja así a propósito, hasta arreglar el relieve.
  - **La altura de saturación** aumenta la lluvia de barlovento de las cordilleras grandes (408 → 548 mm con `k_oro` = 150) y reduce el secado global (−19,8 → −16 %).
  - **Planeta definitivo:**
    - lluvia en tierra +0,2 %, `c_P` 1,207;
    - a sotavento de las cordilleras grandes, 688 → 251 mm y desierto del 7,6 al 16 %;
    - PPN −3,3 %, lagos 51 → 42.
  - **Independencia de la malla:** lluvia +0,7 % de 208 a 104 km y −0,13 % de 52 a 26. La erosión en la prueba de independencia del camino sube a 3,9 % · 4,4 %, y su tolerancia pasa del 4 al 5 %.
  - **N_max real:**
    - gasto medio / gasto adulto: 0,953 en el herbívoro y 0,871 en humanos;
    - con `f_com` 0,004, la densidad de equilibrio del herbívoro quedaba un 23 % por encima de Damuth; con 0,00324, 4,04/km² (Damuth: 3,97);
    - `N/N_max` en equilibrio: herbívoro 1,17, humanos 0,96 (0,309/km² a PPN 700).
  - **Fecundidad humana:**
    - 6,83 hijos, último parto a 38,2 años, caída por edad 1,00 / 0,87 / 0,94 / 0,68 / 0,30 / 0,03 (Coale-Trussell: 1,00 / 0,94 / 0,86 / 0,70 / 0,36 / 0,05);
    - bajar a ~6 hijos sin estropear la forma pide la fecundabilidad por edad y los abortos (Q1, Q3).
  - **Hambruna humana:**
    - la mortalidad adulta de las mujeres es 0,07 al año, frente a 0,54 la de los hombres;
    - la población pierde un 35 % y se recupera a ~0,6 % al año, en ~50 años;
    - el test `en_la_hambruna_las_mujeres_resisten_mejor` pasa a exigir "no colapsa y vuelve a crecer", porque la recuperación al 80 % en 20 años no es realista con 6,8 hijos.
- **Verificación:**
  - `fmt`, `clippy --workspace --all-targets -D warnings` y `cargo test --workspace` en verde (biología: 8 unitarios + 15 de simulación, con el nuevo `en_equilibrio_la_densidad_sigue_la_ley_de_damuth`).
  - Medidas del planeta y de 100 años de agentes, en las notas.
- **Estado tras el cambio:** decisiones S1–S5 y A1–A7 aplicadas. Siguiente: las herramientas de escalabilidad (batería de escenarios, tiempos por sistema, medidas en `herramientas/`). Después, el relieve realista, cuando el responsable apruebe la propuesta.

## [2026-11-05] Herramientas de escalabilidad: ajustes `--set`, tiempos por sistema, diagnóstico del planeta, batería de escenarios y `verificar.sh`

- **Qué:** las soluciones aprobadas por el responsable para los cinco puntos de escalabilidad (el 1 y el 2 ya estaban: retirar interruptores, CLAUDE §5, y `no_regresion.sh`).
  - **Ajustes desde la línea de comandos:** `agentes` y `terreno` aceptan `--set ruta=valor`, que cambia la config sin editarla.
    - `abm_core::config::cargar` crea las tablas que falten y admite índices de lista.
    - Una clave mal escrita da error, porque las configs rechazan los campos desconocidos.
  - **Tiempos por sistema** (punto 5):
    - `abm_core::cronometro::Cronometro`, y `abm_core::time::Programa`: los sistemas del tick en orden, cada uno en su propio `Schedule` y cronometrado;
    - la biología pasa de `bio_systems()` a `bio_programa()`;
    - el terreno anota cada paso de `step`, el LOD, la erosión, las métricas y la génesis;
    - cada corrida escribe `<nombre>_tiempos.json` aparte de su salida, porque cambia de una corrida a otra (CLAUDE §6); `no_regresion.sh` lo ignora.
  - **Diagnóstico del planeta en la salida** (punto 4, opción d; esquema del terreno **v5**). Bloque `diagnostico`:
    - tierra emergida, altura media y mediana, curva hipsométrica por franjas, profundidad del océano;
    - lluvia en tierra y en el mar, tierra con `w` < 0,5 y PPN de Miami.

    La sombra pluviométrica sigue en Python, sobre el volcado por celda.
  - **Medidas en `herramientas/analisis/`** (punto 4): `agentes.py` y `terreno.py`, los análisis de la campaña pasados del scratchpad al repositorio y reutilizables.
  - **Batería de escenarios de referencia** (punto 3; `herramientas/escenarios.py` + `escenarios.toml`):
    - 4 escenarios: `planeta`, `agentes_equilibrio` (100 años), `agentes_sequia` y `humanos_hambruna`;
    - 37 métricas con margen y nota, y un presupuesto de tiempo por escenario;
    - informe en `output/escenarios/informe.json`; tarda ~1 min.
  - **`herramientas/verificar.sh`:** CLAUDE §10 en un comando: fmt, clippy, tests, escenarios y, con `--no-regresion REF`, la no regresión. `herramientas/README.md` lo explica todo.
- **Por qué:** el responsable quiere que el método escale a ~100 sistemas y que "se pueda consultar todo".
- **Decisiones / problemas:**
  - **Programa frente a `.chain()`:** correr cada sistema en su propio `Schedule` equivale a encadenarlos. La salida de `agentes` y de `terreno` es **idéntica byte a byte** a la de `main` (`04cad03`), antes de añadir el diagnóstico.
  - **Los márgenes rodean el valor actual,** no son objetivos de realismo. Cada nota dice el valor real cuando se conoce, y los irreales quedan marcados como pendientes: tierra emergida, altura media, barlovento.
  - **Hallazgo:**
    - con la config real, una sequía de un año al 50 % deja al herbívoro en el **2,7 %** de su población, y 29 años después está en el 37 %. En las mortandades reales muere un 20–60 %;
    - es el hambre sincronizada (A5): la competencia con jerarquía, pendiente obligatorio, debe subir esa cifra;
    - en humanos, la misma sequía quita un 12 %.
  - **Primeros tiempos:**
    - planeta: el LOD se lleva el 53 %, la difusión lateral el 20 % y el clima el 11 %;
    - agentes: la demanda el 32 %, la energética el 22 % y **las métricas el 17 %**. El observador es caro: candidato a optimizar.
  - El primer `verificar.sh` cazó un aviso de clippy (`sort_by` → `sort_by_key`).
- **Verificación:** `herramientas/verificar.sh` en verde: fmt, clippy, todos los tests (nuevos: carga con ajustes, cronómetro, argumentos, diagnóstico) y la batería (37/37 métricas y 4/4 presupuestos).
- **Estado tras el cambio:** los cinco puntos tienen solución en marcha. Quedan, a medida que hagan falta:
  - pasar a Rust las medidas que aún son de Python (sombra pluviométrica);
  - el paralelismo determinista, cuando el informe de tiempos lo pida;
  - el LOD de agentes, que es el incremento 2.

## [2026-11-05] Traspaso a la siguiente sesión: el relieve realista

- **Estado:**
  - main en `733262f` (más este traspaso). `herramientas/verificar.sh` en verde: fmt, clippy, 76 tests y la batería de escenarios (37/37 métricas).
  - Las decisiones S1–S5 y A1–A7 están aplicadas. Las herramientas de escalabilidad, hechas (`herramientas/README.md`).
- **Decisión del responsable:** el relieve va primero. Está escribiendo su documento de diseño del relieve y lo dejará en `docs/temp/`.
- **Qué hacer al empezar:**
  1. Leer CLAUDE.md, TREE.md y MEMORY.md, y también `docs/design/terreno/propuesta_relieve_realista.md`: el diagnóstico frente a la Tierra y las opciones. El documento del responsable manda sobre esa propuesta.
  2. Archivar su documento con `git mv` en `docs/design/terreno/` y borrar `docs/temp/`. Si deja algo ambiguo en el modelo o las ecuaciones, preguntar antes de codificar (CLAUDE §10).
  3. Rama propia e implementación con el método:
     - interruptor mientras se mide, si hay que comparar con el relieve actual;
     - no regresión byte a byte con el interruptor apagado (`herramientas/no_regresion.sh`);
     - medidas frente a la Tierra con el bloque `diagnostico` de la salida del planeta y `herramientas/analisis/terreno.py`;
     - la batería: las métricas del escenario `planeta` se moverán a propósito, así que se actualizan sus márgenes con nota;
     - `verificar.sh`, documentación, commit y main al día.
  4. Con el relieve realista:
     - comprobar que `k_oro` = 40 da 2–3 veces más lluvia en barlovento sin tocarlo;
     - calibrar la lluvia de base con datos de la Tierra (~1 000 mm/año en todo el planeta y ~800 sobre tierra);
     - retirar `c_P`;
     - comparar los biomas con los de la Tierra.

  La génesis en caché se recalcula sola: su clave incluye la huella del código.
- **Pendiente de confirmar por el responsable (propuestas para CLAUDE.md, aún sin escribir):**
  - **§1, visión:** "poder seguir a un único individuo (p. ej., un humano desde que nace hasta que muere) y subir de escala hasta ver toda su población, sin perder su trazabilidad". Además: ¿siguen dentro del alcance la "geopolítica" y los "conflictos" de §1? El responsable dudó de querer "hacer civilizaciones".
  - **§2.8, tipos de parámetros** (nota que el responsable reenvió el 2026-11-05):
    - **(a) con valor físico medible** (`k_oro`, `H_v`, Kleiber): se quedan en su valor real; si no cuadra, se revisa la base;
    - **(b) no medibles directamente** (`f_com`, `L_recarga`): se calibran contra datos observados reales, citando la observación (p. ej., la ley de Damuth), nunca contra otra irrealidad del modelo;
    - **(c) apaños provisionales** (`c_P`): se marcan como tales, con la causa real que hay que corregir.
  - **§10:** la verificación completa se pasa con `herramientas/verificar.sh`.
- **Pendiente de medir** (nota reenviada por el responsable): en el escenario `agentes_sequia` muere el 97 % de los herbívoros. Hay que medir cuánto mueve cada causa antes de elegir.
  1. **Vegetación: descartada,** medido el 2026-11-05.
     - La sequía multiplica el rebrote por 0,5, así que la producción cae justo a la mitad.
     - El forraje en pie solo baja del 50 al 38 % de K.
     - Aun así, el 90 % de los herbívoros muere en ~4 meses, comiendo ~87 % de su gasto: con `k_ap` = 2 y Holling, `2·0,385/0,885`. La condición media de partida es 0,40.
  2. **Ahorro metabólico al pasar hambre: no existe en el modelo.**
     - El gasto usa la masa estructural (`sistemas.rs`, `bio_demanda`), las reservas no lo bajan y el `PAL` es fijo.
     - En la realidad, en el experimento de Minnesota (Keys et al., 1950) el basal cae ~40 % en total (~15–20 % más de lo que explica la pérdida de masa), y los ciervos bajan su metabolismo en invierno (Arnold et al., 2004).
  3. **Competencia con jerarquía (A5-c):** pendiente obligatorio.
  4. **Posible cuarta causa:** las reservas son solo grasa (`e_max`). Un animal real también consume proteína antes de morir.

  Los mecanismos 2–4 necesitan nota de diseño antes de codificarse (CLAUDE §2.7).
- **Pendientes obligatorios de realismo:** en TREE.md.

## [2026-11-05] Relieve tectónico (incremento 4 del terreno): deriva de placas, referencia de la Tierra y primeros resultados

- **Qué:** el responsable dejó `diseno_relieve_tectonico.md` en `docs/temp/`. Se archivó en `docs/design/terreno/` (no estaba bajo git: se movió y se añadió) y `docs/temp` se borró. Rama `terreno-tectonica`.
  - **Módulo `abm-terreno/src/tectonica/`:**
    - `malla`: nivel 2 del cubo y cuaterniones;
    - `deriva`: placas en su propio sistema, fuerzas, huecos y solapes, arcos, choques, erosión tectónica y sedimento, rifts, suturas, absorciones y saltos de dorsal;
    - `isostasia`: altura con el agua encima, Parsons y Sclater, fosas y nivel del mar por volumen;
    - `historia`: eventos, métricas de cada Myr y cuentas de la corteza;
    - `mod`: estado final e interpolación continua a las celdas del terreno.
  - **Integración:**
    - `StaticGen` toma la altura de la base tectónica más un ruido de detalle (G5);
    - la génesis corre la deriva (G1–G4), acondiciona los hoyos del ruido (G6) y hace el prerrodaje con levantamiento y rebote isostático (`erosion::step_con`, partidas `uplift_m3` y `rebound_m3`);
    - la caché pasa a `ABMGEN04` con el estado tectónico;
    - la salida pasa al esquema v6 (`run.tectonica` y `diagnostico.relieve`), y el volcado lleva los datos de la placa.
  - **Interruptor:** `worldgen.relieve = "tectonico" | "ruido"`; la config pone `"tectonico"` y el código, por defecto, `"ruido"` (tests rápidos). `config/experiments/terreno_relieve_ruido.toml` sirve para la no regresión.
  - **Referencia de la Tierra:** `herramientas/analisis/referencia_tierra.py`, solo biblioteca estándar, con el lector mínimo de HDF5 `h5mini.py`. Usa ETOPO 2022, Paxman et al. (2022) v3 leído por peticiones parciales, Seton et al. (2020), Bird (2003) y Argus et al. (2011), a las resoluciones del planeta. Escribe `config/referencias/tierra_relieve.toml`.
  - CLAUDE.md, confirmado por el responsable: §1 visión (del individuo a la población; la geopolítica y los conflictos **se quedan**), §2.8 tipos de parámetros (a/b/c) y §10 `verificar.sh`.
- **Decisiones del responsable** (preguntas al empezar):
  - malla tectónica en el nivel 2 (~52 km);
  - en un choque va abajo la que subducía antes, o la de corteza más fina;
  - empujón inicial de 1–10 cm/año y 50 Myr de rodaje;
  - la flotación por espesor de los arcos se suma a la edad;
  - ruido fino que continúe el espectro real, validado con ETOPO;
  - referencia: roca rebotada sin hielo (Paxman), con el agua del océano actual;
  - no calibrar η_col (llegará una adenda con los sumideros reales);
  - y, en general, «siempre la opción más fiel a la realidad; si falta algo, a pendientes, en orden». Guardado como memoria.
- **Correcciones de realismo avisadas** (el responsable pidió que le dijera lo que contradice la realidad):
  - isostasia con agua encima bajo el nivel del mar (0,220 en vez de 0,152 por km);
  - sedimento con su densidad (2 400 kg/m³) y su masa;
  - `v_obj` 4,39 cm/año (NNR-MORVEL56) en vez de 5;
  - **apuntados como pendientes:** reciclado en las fosas, acortamiento andino, microplacas, ciclo de Wilson y flexión de la litosfera.
- **Problemas resueltos en la implementación** (`incremento-4-diseno.md` §3):
  - borrar las celdas locales «ocultas» roía los continentes (2·10⁹ km³ de corteza «subducida» sin choque): ahora se conservan;
  - la polaridad decidida celda a celda dejaba bordes moteados, 2–3 veces más largos, y disparaba las fuerzas: ahora la polaridad y los papeles de un choque son de cada par de placas;
  - el empate «id menor» de los huecos hacía asimétrica la expansión: ahora un sorteo sin sesgo;
  - la resistencia de colisión del documento (con la convergencia del paso anterior) es inestable con la fuerza de India–Asia: ahora implícita, en un sistema acoplado entre placas;
  - el engrosamiento de un choque convertía en continental el fondo oceánico de la placa de arriba: ahora solo se apila la corteza continental;
  - con el ruido fino calibrado aparecían miles de hoyos: acondicionamiento G6 con balance exacto.
- **Calibraciones** (tipo b):
  - `c_b` a la velocidad real de las placas;
  - `c_col` = 90 000 con un escenario India–Asia: 14,4 → 4,8–5,2 cm/año, 1/3;
  - ruido de detalle con el espectro de ETOPO: `p` = 0,77 y σ por altura respecto al dato.
- **Resultados** (la Tierra: roca rebotada a 26 km):
  - velocidad de las placas 4,36 cm/año (4,39);
  - Forsyth y Uyeda: +0,46 velocidad–fosa y −0,55 velocidad–continente;
  - océano de 3 945 m (3 667), ley de Hack 0,57 y edad mediana del fondo 60 Myr (55);
  - **pero** la tierra emergida es del 33,1 % (28,4 %) y la altura media de la tierra de 1 459 m (671);
  - plataformas de 0,6 km (43), fondo de más de 200 Myr un 15 % (~0,1 %) y corteza continental 0,40 → 0,334.
  - El clima, en cascada: `c_P` 1,99, lluvia en tierra 559 mm, PPN 551 y hielo 7 %.
- **Hallazgos** (`incremento-4-diseno.md` §6):
  1. el nivel del mar queda 1 079 m bajo el dato: con Parsons y Sclater y edades como las de la Tierra las cuencas caben más que el océano actual, así que el «ζ ≈ 0» del diseño no sale ni en la Tierra. Faltan sedimento abisal, mesetas y márgenes, y el inicio de subducción en márgenes viejos. No se fuerza `H_nivel`;
  2. no hay plataformas;
  3. **el relieve real de la Tierra a 26 km da ×1,36 de lluvia de barlovento con `k_oro` = 40**: el objetivo ×2–3 no lo cumple ni la Tierra a esa escala;
  4. el ritmo de rifts del §6.5 es ~5–10 veces menor que el de la ruptura de Pangea;
  5. la corteza continental baja un 17 % (choques y sedimento perdido);
  6. remuestreo de un 5 % en 500 Myr;
  7. la Tierra a 26 km tiene el 10,4 % de la tierra en cuencas cerradas, y el planeta un 4,4 %: el planeta de ruido estaba más drenado que la Tierra.
- **Verificación:**
  - `herramientas/verificar.sh` en verde: fmt, clippy y **89 tests** (6 nuevos de la tectónica + 3 unitarios de malla + 4 de isostasia; 5 ignorados: 4 exploratorios y el de BD);
  - batería: todas las métricas y presupuestos dentro (el escenario `planeta`, rehecho con 34 métricas (51 en total) y notas frente a la Tierra; 13 s con la génesis en caché, ~95 s sin ella);
  - **no regresión:** con `worldgen.relieve = "ruido"` (`config/experiments/terreno_relieve_ruido.toml`), salida idéntica byte a byte a la de `main`; agentes, sin cambios.
- **Estado tras el cambio:** relieve tectónico implementado y medido, por defecto en la config, con interruptor. Pendiente del responsable: la adenda de sumideros y las decisiones de `incremento-4-diseno.md` §7.

## [2026-11-05] Conservación exacta de la deriva y respuestas a la adenda de balance de la corteza

- **Qué:** el responsable respondió al resumen anterior y entregó `docs/design/terreno/adenda_balance_corteza.md`, que llegó directamente a esa carpeta con un `Zone.Identifier` de Windows, borrado.
  - **Prioridad 0, el 5 % de remuestreo:** es una fuga de conservación. Ahora mover las placas conserva la corteza a error de redondeo (2·10⁻¹⁵ del volumen en 500 Myr).
    - los agujeros de los mapas se tapan solo con corteza nueva, nunca copiando;
    - `en_celda` reparte un volumen exacto por áreas, y choques y sedimento solo en la corteza continental;
    - `fundir` traslada cada muestra a la celda libre más cercana.
  - Test nuevo: `mover_las_placas_conserva_exactamente_la_corteza`. Métrica nueva de la batería: `tect_remuestreo_rel`.
  - Las tres preguntas del §11 de la adenda, respondidas en `incremento-4-diseno.md` §8:
    1. sí hay erosión durante la deriva (τ_orog), pero pierde el sedimento como sumidero sin nombre y relaja despacio (~345 Myr);
    2. los arcos van a 40 km³/Myr/km, constantes;
    3. la corteza de arco no cuenta hasta 20 km, y entonces entra entera: no hay `a_i`.
- **Hallazgo nuevo:** las fosas del modelo miden ~100 000 km (la Tierra, ~43 500 de márgenes convergentes oceánicos) y las dorsales ~101 500 (la Tierra ~65 000). Con las tasas por km de la adenda, los flujos de fosa saldrían ~2,3 veces los reales. Hay que decidir antes de implementarla.
- **Decisiones del responsable:**
  - sí al inicio de subducción en márgenes viejos y sí al fondo oceánico real (llegan en notas próximas);
  - lluvia: no cambiar el objetivo; el objetivo debe salir de lluvia observada (WorldClim o CHELSA) a 25 km, y revisar `c_P`. Aclarado: el ×1,36 es el modelo de lluvia sobre el relieve real, no lluvia observada;
  - rifts: se revisan en la próxima nota;
  - el interruptor del relieve lo decide Claude: quitarlo cuando el relieve tectónico pase la comparación con la Tierra;
  - la polaridad por par vale de momento, anotada en TREE.md (en la Tierra puede cambiar a lo largo de un borde).
- **Efecto en el planeta:**
  - tierra emergida 29,8 % (la Tierra 28,4 %); altura media de la tierra 1 544 m (671); océano 3 753 m (3 667);
  - corteza continental 0,40 → 0,304 (antes 0,334: las copias la inflaban); nivel del mar −1 184 m.
  - Márgenes de la batería actualizados, con nota.
- **Estado tras el cambio:** deriva con conservación exacta. Pendiente: decidir la longitud de los bordes, implementar la adenda, medir la lluvia observada a 25 km, y las notas próximas (inicio de subducción, fondo oceánico, rifts).

## [2026-11-05] Adenda de balance v2: balance de la corteza continental, bordes de la Tierra medidos igual y lluvia observada

- **Qué:** el responsable entregó la versión 2 de `docs/design/terreno/adenda_balance_corteza.md` (tasas por km² de placa que se hunde) y pidió:
  - implementarla sin esperar a corregir los bordes;
  - medir los bordes de Bird (2003) en la malla de 52 km igual que los del modelo, y decir si el modelo clasifica transformantes;
  - para la erosión, el sedimento al mar validado con la prueba de relajación;
  - para la lluvia, la observada (WorldClim/CHELSA) a 25 km con la misma definición de barlovento, y revisar `c_P`.

  Rama `terreno-balance-corteza`. Detalle técnico en `incremento-4-diseno.md` §9–§11; explicación llana en el README de la fase.
  - **Adenda implementada entera, sin interruptor** (cambio de mecanismo; la referencia es el commit anterior):
    - en cada muestra, `hc` (corteza de tipo continental) y `arco` (`a_i`); sedimento también sobre el fondo oceánico; `V_cont = Σ (hc + S·ρ_s/ρ_c)·A`;
    - el remuestreo apunta el fondo consumido de cada tramo de fosa;
    - régimen erosivo o acrecionario; arcos `p_arc·A_sub`; erosión por subducción `p_SE·A_sub`; prisma con `f_acr`; choques con `η_col` 0,5 y `η_arco` 0,9; delaminación exponencial; erosión que lleva el sedimento al mar o a cuencas cerradas;
    - balance comprobado en cada paso (error de 10⁻¹⁴);
    - métricas por paso (§8) y medias en `run.tectonica.balance`; eventos `inicio_choque`, `fin_choque`, `cambio_regimen` y `retroceso_margen`; salida en esquema v7.
  - **Decisiones técnicas mías** (avisadas, `incremento-4-diseno.md` §9.2):
    - **tramos:** el consumo se agrupa en la celda de borde de la placa de arriba;
    - **arcos:** cada placa recibe exactamente `Σ A_sub`, repartido por su franja con el peso de la convergencia del borde. Con un reparto por tramo, la mitad se quedaba sin franja en bordes dentados;
    - **sedimento:** inundación por prioridad hasta el mar; las cuencas cerradas se llenan hasta su desagüe; en el mar llena hasta la plataforma (−130 m) y sigue mar adentro;
    - **choques:** lo que no cabe en la cuenca de un frente pasa al resto de la placa;
    - **erosión por subducción:** hasta `d_arco + w_arco/2`; la celda que se queda sin corteza continental pasa a fondo oceánico.
  - **`H_max` sube de 65 a 80 km** (lo más grueso de la Tierra, objetivo de la adenda) y vale para toda corteza: arcos, prisma y choques. Hubo que comprobarlo **por muestra receptora**: con la comprobación por celda del mundo, algunas muestras llegaban a 120–150 km, porque `en_celda` reparte entre muestras de menos área y una muestra puede recibir por dos celdas.
  - **`τ_ero` = 63 Myr**, calibrado con la prueba de relajación (India–Asia y detener la convergencia): baja de 38 km en 148 Myr, el centro de 100–200. Con 150 Myr eran 322. `τ_del` y `f_acr` se quedan en sus valores iniciales: sus objetivos globales dependen de la base, que falla.
  - **Bordes de Bird** (`herramientas/analisis/bordes_bird.py` → `config/referencias/tierra_bordes.toml`):
    - la malla no alarga las líneas, las acorta (×0,90): el método del gradiente alisa los zigzag finos;
    - con la regla del modelo, la Tierra da dorsales 103 000, fosas 53 000, choques 39 000 y transformantes 38 500 km (52 placas); con 15 placas, 87 000 / 39 500 / 25 600 / 29 600;
    - el modelo, con 14–15 placas: 121 000 / 118 000 / 11 000 / 62 000.
  - **El modelo sí clasifica transformantes** (I4: a menos de 15°; Bird separa a ~20°). La malla convierte la mitad de las transformantes oceánicas reales en dorsales.
  - **Lluvia observada:** la midió un subagente (`lluvia_observada.py`, `tiffmini.py`, `tierra_lluvia.toml`; `referencia_tierra.py` y `terreno.py` refactorizados sin cambiar sus salidas). Con nuestra definición de barlovento, la Tierra da ×0,7–1,2, no ×2–3, porque la definición coge mesetas (el Tíbet, Irán y el Altiplano son un tercio del área). El planeta falla en la sombra pluviométrica, el perfil con la altura y el reparto por latitud.
- **Por qué:** realismo primero. `f_cont` es un resultado; antes de tocar parámetros se revisa la base.
- **Resultados** (planeta, 450 Myr sin rodaje; entre paréntesis, la Tierra):
  - **área subducida 6,3 km²/año (2,7–3,4): el doble;**
  - arcos 6,9 AU (2,7–5); erosión por subducción 2,4 (1,3–1,7); sedimento subducido 1,04 (1,0–1,65); acrecionado 0,41 (≈0,3);
  - **choques 4,7 (0,4–0,7)**; delaminación 1,18 (≈1,1); sedimento al mar 4,9 (≈1,8);
  - espesor máximo 79 km (70–80); `V_cont` −16 % (±10 %); `f_cont` 0,31 (0,40).
  - **Diagnóstico:** el 26 % del fondo consumido y el 31 % de la corteza continental solapada caen en bordes divergentes y transformantes (dientes de bordes que se deslizan). El modelo tiene 1,7 veces más borde y 3 veces más fosa que una Tierra de 15 placas medida igual. El remuestreo no es la causa: un borde en zigzag consume lo mismo que uno recto (1,029 frente a 1,022 del teórico).
  - **Planeta:** altura media de la tierra 1 544 → 1 277 m; nivel del mar −1 184 → −619 m; edad media del fondo 97 → 79 Myr; lluvia sobre tierra 595 → 710 mm (`c_P` 2,04 → 1,78); sotavento entre resto 1,24 → 0,79 (la Tierra 0,58).
- **Verificación:**
  - 6 tests nuevos (relajación, planeta sin fosas, reglas del margen y de la delaminación, borde dentado frente a recto, balance por paso y traslados exactos en la prueba de conservación); `sin_cratones_solo_los_arcos_hacen_corteza_continental` sustituye a la prueba que prohibía la corteza continental sin cratones, porque los arcos reales la hacen en decenas de Myr;
  - batería: `planeta` con 46 métricas (11 nuevas del balance; márgenes movidos con nota);
  - `verificar.sh` en verde; no regresión de los agentes idéntica byte a byte; la génesis tarda ~140 s (antes ~90).
- **Pendiente:** la nota del responsable sobre los bordes, el inicio de subducción, el fondo oceánico y los rifts; después, calibrar `τ_del` y revisar `f_acr`. La definición de barlovento y el objetivo de la lluvia los decide el responsable.

## [2026-11-06] Nota de bordes de placa, adenda v3 (flujo lateral sin tope), qué apilaba corteza y barlovento observado

- **Qué:** el responsable entregó `docs/design/terreno/nota_bordes_placa.md` y la adenda v3 (`H_max` deja de ser regla; lo sustituye el flujo lateral §6.6b).
  - Pidió:
    - implementar la nota antes de calibrar nada de la adenda;
    - investigar qué apilaba corteza hasta 150 km;
    - volver a medir la exportación de sedimento;
    - medir el barlovento con su regla: laderas del pie a la cresta, sin mesetas, frente a la llanura del pie.
  - Dio por buenas mis decisiones anteriores (arco por placa, plataforma a −130 m, prueba de los arcos, `τ_ero` = 63). Pidió no recalibrar `c_P`.

  Rama `terreno-bordes-placa`; detalle en `incremento-4-diseno.md` §12–§15 y explicación llana en el README de la fase.
  - **Nota de bordes**, en `tectonica/deriva/suavizado.rs` y `deriva.rs`:
    - pertenencia suavizada `φ` a `L_borde` = 150 km (radio efectivo de al menos 1,5 celdas en mallas gruesas) y normal del borde suavizado;
    - transformante a 20° (`theta_tr_grados` sustituye a `f_transformante`);
    - solo se consume en tramos convergentes;
    - los dientes se recolocan en el hueco más cercano que toca su placa (antes de repartir los huecos);
    - creación residual; suavizado de la pertenencia oceánica.
  - **Extensiones (decisiones mías):**
    - islas de menos de `L_borde²` absorbidas por la placa que las rodea, también continentales;
    - una muestra suelta (`φ` < 0,15) no gana un solape;
    - los dientes sin hueco se reparten por la vecindad;
    - lo que se calcula para una celda va a la muestra que el mundo ve en ella (más las ocultas cuyo centro cae allí);
    - el fondo de una cuenca cerrada no se erosiona.
  - **Adenda v3:**
    - fuera `h_max_km`; alarma de 80 km en las pruebas y en la batería;
    - flujo lateral por la malla del mundo, también a través de las suturas de los choques continentales (desviación del texto "de la misma placa", avisada);
    - `D_flujo` = 3 000 km²/Myr, calibrado con India–Asia (máximo 72–77 km).
  - **Qué apilaba corteza** (sin tope y sin flujo, 527 km en India–Asia y 1 525 km en el planeta):
    1. la franja de Voronoi de cada frente de choque, a veces de una sola celda;
    2. islas de la placa de arriba que ganaban solapes y se comían la de abajo;
    3. el flujo por la rejilla girada de la placa, donde las muestras gruesas estaban sueltas;
    4. astillas de la placa de arriba rodeadas de otras placas;
    5. dientes e islas repartidos en 1–3 celdas;
    6. una montaña de sedimento por mezclar la muestra que se ve con la del centro.

    Ahora el máximo de toda la deriva es 113 km, y pasan de 80 solo 3 pasos de 500.
  - **Bird** (`bordes_bird.py`, regla de la nota): la Tierra de 15 placas da dorsales 86 700, fosas 37 300, choques 23 600, transformantes 34 900 y perímetro 182 600 km.
  - **Barlovento** (subagente; `terreno.laderas_pie_cresta()`): la Tierra observada ×0,90 (WorldClim) y ×1,23 (CHELSA); el planeta ×1,05. El objetivo ~1,2 sustituye al ×2–3.
- **Resultados** (planeta; entre paréntesis, la Tierra):
  - consumo en bordes que no convergen 26 → 3,0 % (< 2 %); solape continental en ellos 31 → 4,5 %;
  - `A_sub` 6,3 → 3,64 km²/año (2,7–3,4);
  - perímetro 312 → 206 mil km (183 ± 25 %); dorsales 74 600 (86 700); **fosas 74 000 (37 300)**; choques 6 800; transformantes 51 200;
  - edad media del fondo 132 Myr (64), mediana 59 (55), el 24 % pasa de 200 Myr;
  - `f_cont` 0,363; `V_cont` −6,6 %;
  - arcos 3,9 AU; erosión por subducción 1,4; sedimento subducido 0,38; choques 3,0; delaminación 0,05; al mar 3,1;
  - tierra 36,6 % y 989 m de media (671).
  - **Barrido de `L_borde`** 100/150/300 km: fosas 80 200 / 74 000 / 58 200 km. Ni con 300 bajan de 1,5 veces la Tierra: según la nota §11.2, la causa son las fuerzas de las placas.
  - **Exportación al mar: no depende de `τ_ero`.** 3,07 AU tanto con 63 Myr (relajación 136 Myr) como con 110 (257 Myr). En equilibrio, la erosión se lleva lo que añaden arcos y choques. Lo que falta es la delaminación: 0,05 AU frente a ≈1,1, porque el flujo reparte la corteza antes de los 50 km de `h_del`. Una ley no lineal cambiaría dónde se erosiona, no el total. Pregunta para el responsable: ¿delaminación de las raíces de los arcos?
  - **§11.3:** el 44,5 % del área subducida cae en tramos sin franja propia; el arco se sigue repartiendo por placa.
- **Verificación:**
  - pruebas nuevas:
    - deslizamiento puro (consumo y creación exactamente 0 en 100 Myr; celdas de borde 1 536 → 768);
    - separación oblicua (solo nace placa);
    - un gran choque no apila por encima de 80 km;
  - India–Asia como en `main` (velocidad tras el choque 5,8 frente a 5,5 cm/año);
  - conservación a 10⁻¹⁴; batería del planeta con 53 métricas (márgenes movidos con nota); no regresión de los agentes idéntica byte a byte; `verificar.sh`.
  - La génesis tarda ~160 s (presupuesto del escenario: 200).
- **Pendiente:** las fuerzas de las placas (fosas ×2); la exportación al mar (delaminación de los arcos o erosión no lineal: decide el responsable); la nota del inicio de subducción, el fondo oceánico y los rifts; los 3 pasos sobre 80 km; después, `τ_del` y `f_acr`; la nota de la lluvia por latitudes.

## [2026-11-07] Diagnóstico de la nota de fosas, adenda v4 (raíz de los arcos), uniones triples y varias semillas

- **Qué:** el responsable entregó `docs/design/terreno/nota_ciclo_fosas.md` y la adenda v4 (raíz de los arcos con `h_del,arco` = 40 km; delaminación como objetivo débil).
  - Pidió:
    - el diagnóstico del §0 antes de implementar la nota: si el exceso de fosa es lento, implementarla; si es rápido, traérselo;
    - responder al §11;
    - no calibrar nada (`τ_del`, `f_acr`, `X_ini`, `λ_esp`) hasta la nota de fosas;
    - mirar si el consumo en bordes no convergentes y los picos de 113 km salen de las uniones triples;
    - decir cuánto tiempo añade la nota.
  - Aprobó todas las decisiones de la entrega anterior.
- **Diagnóstico del §0: el exceso de fosa es rápido.** No se implementa la nota hasta que decida el responsable.
  - El modelo tiene 74 000 km de fosa a 5,1 cm/año, con el 18,6 % por debajo de 2 cm/año.
  - Bird: SUB, 51 309 km a 5,3 cm/año (21,4 %); OCB, 81 % lenta. En la malla con la regla de la nota: 51 163 km (31 %) con todas las placas y 37 347 (36 %) con las 15 mayores.
  - La fosa lenta del modelo (13 800 km) es la de la Tierra; todo el exceso está por encima de 2 cm/año.
- **El 7–9 cm/año de la nota no es el de Bird.**
  - Mezcla el área que se hunde en toda la Tierra con la fosa de las 15 mayores, que solo hunde 1,48 km²/año: al fundir las placas pequeñas se van las fosas de las microplacas de arco.
  - Referencia coherente para un modelo sin microplacas: la fosa total, ~51 000 km a 5,3 cm/año y 2,7 km²/año. Contra ella, el exceso es ×1,45, casi todo de área (el modelo hunde 3,6 km²/año).
  - `bordes_bird.py` mide ahora esa convergencia tramo a tramo y en la malla (`tierra_bordes.toml`). En Bird, las dorsales crean 3,01 km²/año a 4,47 cm/año.
- **§11:**
  - el tirón de la losa no depende de su longitud (tira entero desde el primer paso);
  - la división de placas existe (`partir`, rifts y saltos de dorsal), pero corta por un círculo, y un margen pasivo necesita cortar por el tipo de corteza;
  - no se guardan zonas de fractura.
- **Adenda v4, implementada sin calibrar.**
  - Regla: `h_del_arco_km` = 40; la muestra que recibe arco lleva `en_franja_arco` hasta la delaminación del paso; `delaminacion_arcos_au` en la salida.
  - Fuente: Jagoutz y Behn 2013 (Nature 504): más densa que el manto a más de 40 km, discontinuidad a 38–42 km. El 35–45 que di antes era una estimación mía.
  - Con 5 semillas: la delaminación pasa de 0,07 a 0,31 AU; **la exportación al mar no baja** (3,13 → 3,06); el volumen continental cae más (−8,4 → −9,5 %).
  - Con `h_del_arco_km` = 50, la tectónica es la de `main`.
- **Corrección de la entrega anterior: la exportación sí depende de `τ_ero`.** Con 5 semillas: 3,06 AU con 63 y 2,38 con 110, y baja en todas. Lo de antes era una deriva por valor en un planeta caótico (±0,5 AU entre semillas).
  - Un solo `τ_ero` no cumple a la vez la relajación (135 y 256 Myr con la v4) y ~1,8 AU: vuelve la erosión no lineal, que decide el responsable.
  - La mitad de la erosión potencial sale de columnas de más de 10 km de exceso; el 27 %, de llanuras (menos de 5 km).
- **Uniones triples: no son la causa.**
  - Del 3,2 % de consumo en bordes no convergentes, solo el 11 % está en uniones triples. El 86 % está a menos de 150 km de una fosa del mismo par: son finales de fosa que la cuenta asigna a la transformante vecina.
  - De los picos de más de 80 km, el 88 % ya era visible el paso anterior y el 3,5 % son muestras ocultas que reaparecen. Los sube sobre todo el reparto de dientes y muestras sueltas al rehacer los mapas (58 %), y después los choques.
  - Las muestras ocultas llegan a 180 km porque no fluyen.
- **Herramientas:**
  - traza de diagnóstico en `dar_paso` (tiempo y espesor máximo por etapa; apagada no cuesta nada);
  - `consumo_en_borde_paso` y `solape_en_borde_paso`;
  - prueba exploratoria `deriva_en_varias_semillas` (SEMILLAS, H_DEL_ARCO, TAU_ERO; ~4 min con 10 derivas en paralelo).
- **Tiempo:**
  - por etapas, la erosión con el sedimento es el 31 %, rasterizar el 11 % y los solapes el 10 %;
  - la nota de fosas añadiría del 3 al 5 % (~5 s; el planeta tarda 164 de 200 s).
- **Verificación:**
  - batería: el planeta de la semilla 42 es otra realización con la v4, así que 25 márgenes se mueven con nota; 3 métricas nuevas (delaminación de los arcos, convergencia media y fracción lenta de las fosas); 56 en total;
  - no regresión de los agentes byte a byte; terreno igual a `main` con `h_del_arco_km` = 50, salvo los campos nuevos.
- **Pendiente (`incremento-4-diseno.md` §22):** decisión sobre la nota de fosas y las fuerzas (área ×1,3); la ley de erosión; el volumen continental (−9,5 %); los picos; después, calibrar con varias semillas.

## [2026-11-07] Adenda v5 (base), nota de fosas v2 implementada, medidas de fuerzas y erosión con la altura

- **Qué pidió el responsable.**
  - Nota de fosas v2: implementarla, con el tirón que crece con la losa. Si la fosa sigue más de 1,25 veces la referencia (Bird SUB, ~51 000 km), las cuatro medidas de fuerzas del §10, antes de tocar nada.
  - Adenda v5: retirar la raíz de los arcos; erosión `E = E_1·(z/1 km)^n_ero` (calibrar con relajación y ~1,8 AU); toda calibración con 5 semillas; finales de fosa a su fosa; muestras ocultas en el flujo; revisar el reparto de dientes y sueltas.
  - Orden: base, nota y erosión al final.
- **Base.**
  - Raíz de los arcos retirada.
  - Finales de fosa: consumo no convergente del 3,2 % al 0,4–0,6 %.
  - Ocultas en el flujo: máximo de una oculta de 86–180 a 69–92 km.
  - Dientes:
    - segunda pasada a 3·L_borde hacia huecos de tramo transformante o convergente;
    - los que siguen sin hueco esperan ocultos en su placa en vez de repartirse (movían ~5,8 AU de corteza continental y la mitad engrosaba bordes que se deslizan);
    - picos de 605 a 27 celdas·paso y exportación de 3,22 a 2,55 AU.
  - La erosión miraba la cuenca cerrada en la celda del centro de la muestra, no en la que la ve: una columna alta junto a una depresión no se erosionaba.
- **Nota de fosas** (`deriva/fosas.rs`).
  - Estado por celda de borde en un mapa de cada placa, rehecho cada paso y heredado a dos anillos; tramos = componentes conexas del par.
  - Difuso con absorción en `W_dif` = `L_borde`; nacimiento con `X_ini`; polaridad fija (sustituye a la polaridad por par); tirón × `min(1, L/L_losa)`; muerte por `τ_muerte` y por choque.
  - Espontáneos en márgenes pasivos (`partir_por_corteza`) y en transformantes.
  - `activar_fosas` para los escenarios con subducción en marcha; 5 pruebas nuevas e India–Asia igual.
  - Decisiones mías:
    - flota = continental (> 20 km): con «`hc > 0`», el sedimento convertido en roca mataba las fosas cada 2 Myr;
    - una fosa espontánea nace con `L = X_ini`;
    - el tirón sigue mientras la fosa vive.
- **Calibrado `λ_esp` = 1,7·10⁻⁷** (edad media del fondo 64 Myr; más de 180 Myr no baja del 4 % con ningún valor).
- **Resultado** (5 semillas):
  - fosa 79 300 km a 5,7 cm/año, ×1,55 la referencia;
  - difusa 10 300 km a 1,9 cm/año (OCB 16 200 a 1,3);
  - se hunden 4,4 km²/año.
  - Con `λ_esp` = 0, fosa 58 700 km (×1,15), se hunden 2,92 + 0,28 km²/año y la placa mayor ocupa el 25 %, pero el fondo es demasiado viejo (media 103 Myr). El exceso viene casi entero de los nacimientos en márgenes pasivos, que convierten márgenes enteros en fosa.
- **Las cuatro medidas de fuerzas** (incremento-4-diseno.md §27):
  - oblicuidad casi uniforme, frente a la Tierra con bordes de frente;
  - placa mayor del 41 % (Pacífico 22 %);
  - tirón 75 % y empuje de dorsal 25 % de la potencia motriz (Tierra ~90/10);
  - área creada 4,9 km²/año (2,7–3,4).
- **Erosión con la altura:** `τ_ero` retirado; `n_ero` = 2 y `E_1` = 0,01 provisionales (relajación 144 Myr, tasas de m/Myr a 0,1–0,25 km/Myr); al mar 4,2 ± 1,0 AU. La calibración conjunta espera a la base, para no compensar el exceso de subducción. La tierra baja a 921 m de media y el mar queda en el dato.
- **Hallazgos:**
  - la convergencia difusa mete 1,1 km³/año de basalto bajo los continentes (la nota dice «ambas placas»), que los levanta y la erosión gasta;
  - los picos de más de 80 km vuelven con la nota (máximo visible 107–129 km), sobre todo en los choques.
- **Herramientas:**
  - `deriva_en_varias_semillas` con `VARIANTES` (como `--set`) y las medidas de fuerzas;
  - `calibracion_relajacion` con `E1` y `N_ERO`;
  - `bordes_bird.py`: clase de Bird → fosa con la regla de la nota, oblicuidad y tamaños.
- **Tiempo:** deriva 112 s (antes ~100); planeta 187,6 s de 200.
- **Batería:** 34 márgenes movidos con nota y 4 métricas nuevas de fosas (59 en el planeta).
- **Pendiente (incremento-4-diseno.md §30):** cómo consumir el fondo viejo sin rodear márgenes de fosa; el basalto bajo los continentes; los picos; luego, la calibración conjunta de la erosión.

## [2026-11-07] Última ronda de la tectónica: nota de fosas v3, flujo cúbico (adenda v6), sedimento al mar abierto, calibración conjunta y tabla de cierre

- **Qué pidió el responsable.**
  - Nota de fosas v3: nacimiento espontáneo solo donde el fondo pasa de `a_min` y solo en ese tramo; en la convergencia difusa el basalto nunca pasa a corteza continental. Acepta mis tres decisiones de la v2.
  - Adenda v6: flujo lateral `D_0·(c/c_ref)³` con `c = h_c − h_flujo`. Antes de calibrar, comprobar si los picos de 107–129 km eran columnas reales.
  - Calibración conjunta con 5 semillas (`D_0`, `E_1`, `n_ero`, `τ_del`, `f_acr`, `X_ini`, `λ_esp`, `a_min`) y la tabla de cierre frente a ETOPO. Avisar si el tiempo pasaba de 200 s.
- **Nota v3** (`deriva/fosas.rs`).
  - Candidatas de margen junto a fondo de más de `a_min`, con `p = 1 − exp(−λ·((a − a_min)/100)·L·Δt)` por bloque.
  - `partir_por_fosa`: separa el océano que está delante del tramo, con un camino mínimo de varias fuentes.
  - Transformantes solo en el tramo viejo.
  - Difusa: se retira el basalto, y `hc` y sedimento van a las celdas continentales de la franja.
  - Prueba 1 adaptada (conserva el sedimento a 10⁻⁹).
- **Adenda v6.**
  - Flujo cúbico con subpasos adaptativos al canal más grueso.
  - Red compacta (`RedFlujo`), idéntica byte a byte al recorrido celda a celda: el flujo pasa de 23 a 18 s.
  - Las parejas de ocultas que no pueden fluir no entran.
- **Los picos (incremento-4-diseno.md §33).**
  - Por muestras distintas, eran regiones gruesas, no duplicados.
  - Pero las alimentaba un **fallo de base**: `sedimento_al_mar` dejaba sin tope en la desembocadura lo que no cabía en un mar cerrado pequeño ya lleno. Muestras de 160–1 835 km de sedimento, que un choque o un arco convertían en corteza: picos de 688–2 068 km y exportación de hasta 10 AU.
  - Arreglo: solo es salida un mar abierto (`area_mar_abierto_km2` = 5·10⁵ km²). Los mares cerrados pequeños son cuencas que se llenan hasta su desagüe. Como seguro, lo que no cabe sigue hasta el mar más cercano con sitio (`sedimento_desbordado_km3`, 0,005–0,03 AU).
  - Quedan agujas pasajeras de sumar muestras (islas y fusiones).
- **Calibración (§34).**
  - `D_0` = 50 km²/Myr: India–Asia a 72 km; con la base corregida, manda `D_0` y no `τ_del`. Antes del arreglo daba 70–79 km por el sedimento apilado.
  - `τ_del` = 25 Myr: delaminación 0,71 AU.
  - `n_ero` = 2,5 y `E_1` = 0,005: relajación 135 Myr.
  - `f_acr` = 0,2: acrecionado/subducido 0,24.
  - `λ_esp` = 4·10⁻⁶: la edad no responde entre 1·10⁻⁶ y 1,6·10⁻⁵.
  - `X_ini` 150 y `a_min` 140 sin cambio: no mueven ni la fosa ni la edad.
  - Decisión mía: no subir `D_0` para aplanar las columnas de más de 100 km del planeta, que vienen del exceso de los choques (CLAUDE §2.8).
- **Tabla de cierre (§35; `herramientas/planeta_semillas.py`, 5 semillas, planeta entero).**
  - Cumplen:
    - fosa 59 470 km (×1,17);
    - área que se hunde 3,14 km²/año;
    - velocidad 4,49 cm/año;
    - corteza máxima al final 76 km;
    - tiempo 181,3 s de 200.
  - No cumplen:
    - relieve frente a ETOPO: tierra emergida 0,33 (0,28), altura media 1 150 m (671), por debajo de 200 m 3,9 % (28 %), pendiente ×2,4, plataforma 4,7 km (43);
    - edad media del fondo 99 Myr (64,2), con un 18,5 % de más de 180 Myr;
    - `f_cont` 0,33;
    - ΔV_cont −13 ± 8 %;
    - al mar 2,5 AU;
    - corteza máxima de toda la deriva 101 km.
- **Causas (§36).**
  - La v3 deja sin consumir el fondo viejo de los márgenes pasivos (el 44–68 % está a menos de 300 km de su orilla continental). Las cuencas se ahondan y el mar baja 0,5–1 km, así que las plataformas emergen y la tierra queda alta y sin llanuras. Con la v2 el mar quedaba en el dato, pero la fosa medía ×1,55.
  - Los choques destruyen 2,9 AU (como la Tierra de hoy con India–Asia, pero siempre; la media a largo plazo es 0,4–0,7) en 8 500 km de borde (Bird CCB 21 200). Es el mayor sumidero del volumen continental y el probable origen del exceso de relieve y de exportación.
- **Herramientas:**
  - `planeta_semillas.py` (la tabla de cierre);
  - `diagnostico_pilas` (dos pasadas: sigue etapa a etapa la muestra que pica);
  - `diagnostico_fondo_viejo`;
  - en `deriva_en_varias_semillas`, los flujos de sedimento y su máximo;
  - `D0` en `calibracion_relajacion`; `TAU_DEL` en `calibracion_flujo_lateral`.
- **Batería:** 39 márgenes del planeta movidos con nota (otra realización). Agentes idénticos byte a byte frente a `main`.
- **Pendiente (§38):**
  - cómo se cierra un océano de tipo Atlántico (propagación de fosas por el margen, invasión desde el océano vecino), decisión del responsable;
  - la duración y la velocidad de los choques;
  - las agujas de sumar muestras;
  - recalibrar la erosión cuando cambie la base.

## [2026-11-07] Nota de fosas v4: invasión, rotura de la losa, diagnóstico de los choques y tabla de cierre

- **Qué pidió el responsable.**
  - Nota de fosas v4: los márgenes pasivos no se hunden solos; los océanos viejos se cierran por invasión, una fosa activa que se alarga desde su extremo a `v_inv` por transformantes y márgenes con fondo de más de `a_inv`. El tramo nuevo hereda la polaridad y la losa; si separa el océano, este es una placa nueva. El nacimiento espontáneo solo queda en transformantes.
  - Rotura de la losa tras un choque (`t_rot` 10–25 Myr según la edad). Antes, contar cómo funcionaba el tirón al morir una fosa por choque.
  - Calibrar `v_inv` y `a_inv` (40–100) con la vida de los márgenes pasivos y la edad del fondo (5 semillas).
  - Diagnóstico de los choques frente a Bird CCB, sin tocar `c_col` ni `η_col`.
  - Sedimento al «océano mundial» si es sencillo. Repetir la tabla de cierre.
- **El tirón antes:** desaparecía al instante al llegar el continente, borde a borde y por tramo (incremento-4-diseno.md §39).
- **Rotura de la losa** (§40).
  - La fosa que choca sigue activa y tira también en los bordes de choque, con la edad de la losa del inicio del choque, hasta `t_rot`.
  - La zona comparte reloj y edad; la losa rota queda marcada.
  - Fallos encontrados por el camino:
    - el reloj de un choque vecino hacía chocar a tramos enteros (fosas de ~30 Myr y ~600 roturas por 100 Myr);
    - la edad de la losa se tomaba de celdas de borde recién creadas (se usa el fondo de detrás).
  - India–Asia: la losa tira 21–25 Myr y se rompe; la India va a la mitad de su velocidad mientras cuelga y a un tercio después; 68–77 km de espesor máximo y relajación en 140 Myr.
- **Invasión** (§40).
  - El extremo de una fosa es su último borde convergente vivo; no lo son las transformantes con el estado heredado ni el borde de un choque.
  - Por transformantes: camino 0–1 a `v_inv`, heredando la losa.
  - Por márgenes: un frente sin cinemática, en el borde de continentes de al menos (3·L_borde)², ni arcos ni trozos sueltos (separaban placas de 3,9 y 11 Mkm²).
  - Cada 3·L_borde se intenta separar el océano de delante. Decisión mía: con microplacas desde (3·L_borde)² que no se absorben mientras tienen losa, porque esa franja mide ~270 km y nunca llegaba a A_min.
  - Solo invade una fosa que converge.
  - Nuevo `tapar_huecos_del_mundo` tras los eventos de placas (un fallo latente que destapó la prueba sin cratones).
- **Calibración (§41).**
  - `v_inv` = 25 km/Myr y `a_inv` = 60 apenas mueven la edad del fondo (89–101 Myr) y la de los márgenes (vida 91–124 Myr con 0–150 km/Myr).
  - Las microplacas mejoran el balance: ΔV_cont de −22 a −15 % y pérdida en choques de 4,3 a 3,5 AU.
- **Choques frente a Bird CCB (§42).**
  - Modelo: 8 700 km a 1,89 cm/año. CCB: 23 000 km a 1,87.
  - El modelo converge menos área que la Tierra de hoy, pero la regla de `η_col` = 0,5 aplicada a CCB daría 7–9 AU (la Tierra pierde 0,4–0,7 a largo plazo).
  - Duración de 71 Myr; dos tercios de la pérdida ocurren con la losa colgando.
- **Sedimento:** solo es mar abierto el océano mundial (se retira `area_mar_abierto_km2`).
- **Tabla de cierre (§43).**
  - Cumplen: corteza máxima final 75 km, velocidad 4,55 cm/año, convergencia de los choques y tiempo (170,9 s de 200).
  - Fosa ×1,34 y área ×1,31: a la ampliación futura, como pidió el responsable.
  - No cumplen: el relieve, la edad del fondo (112 Myr al final), los márgenes (vida 117 Myr; al final, edad media 212 y máxima 650), `f_cont` 0,31 y ΔV_cont −15 %.
- **Pruebas nuevas:**
  - invasión de un margen;
  - margen aislado que no se hunde;
  - rotura de la losa en India–Asia;
  - India–Asia sin invasión (escenario del choque).
- **Herramientas:**
  - `bordes_bird.py`, también la convergencia de CCB;
  - `planeta_semillas.py`, con márgenes y choques;
  - en el informe de varias semillas: márgenes, invasión, intentos de separar, roturas y choques;
  - traza con los huecos del mundo.
- **Batería:** 27 márgenes movidos. Agentes idénticos a `main`.
- **Pendiente (§45):**
  - el ciclo de Wilson y el estado inicial (márgenes que ninguna fosa alcanza);
  - la regla de la pérdida en los choques;
  - fosa y área a la ampliación futura.


## [2026-11-07] La Tierra real en la regla del modelo, adenda v7 (η según el grosor) y nota de fosas v5 (zonas de fractura), sin calibrar

- **Qué pidió el responsable.**
  - Primero, antes de cambiar nada: meter la Tierra real en la regla del modelo y dar el resultado antes de calibrar. Las edades de Seton, el sedimento de GlobSed y el volumen de agua actual, con la curva de profundidad y la isostasia del modelo, frente al nivel 0 y los 3 667 m. Comprobar también si la profundidad deja de crecer pasados 80–100 Myr.
  - Adenda v7: `η(h)` de `η_min` (corteza de menos de 20 km) a 1 (de más de 35). Calibrar `η_min` en 0–0,5 con la pérdida de 0,4–0,7 AU.
  - Nota de fosas v5: zonas de fractura (escalones de edad de más de `Δa_fz` dentro de una placa) como candidatas al nacimiento espontáneo.
  - Calibrar `η_min`, `λ_esp` y `Δa_fz` con 5 semillas y repetir la tabla de cierre.
  - Se pararon Grafana y la base de datos (`docker compose stop`; los volúmenes se conservan).
- **La Tierra real con la regla** (`herramientas/analisis/tierra_regla_modelo.py`, `config/referencias/tierra_nivel_mar.toml`; incremento-4-diseno.md §46).
  - GlobSed viene de la copia de CaltechDATA, porque NOAA retiró la página en mayo de 2025.
  - Con la roca real, el volumen del modelo da −14 m. Con la regla del modelo, −386 m y 3 992 m de profundidad media.
  - El zócalo real deja de bajar hacia los 80 Myr: mediana de 5,0–5,4 km de 70 a 180 Myr. Parsons y Sclater sigue hasta 6,1 km a los 150.
  - Dos tercios del desfase vienen del fondo de más de 80 Myr. GDH1 da −340 m.
  - Una placa ajustada a la mediana (`d_∞` 5 310 m, `τ` 31 Myr) da −150 m; ajustada a la media, −25 m. La diferencia son las mesetas, los montes submarinos y los puntos calientes.
- **Adenda v7** (§47).
  - `eta_col` pasa a `eta_min`, `h1_choque_km` y `h2_choque_km` (`TectonicaCfg::eta_choque`).
  - Con `h_1 = h_2 = 10⁹` el planeta sale idéntico byte a byte a 524153b.
  - Falla `un_gran_choque_no_apila_corteza_por_encima_de_80_km`: 89 km, porque la India uniforme de 35 km se queda entera. Se deja así hasta que decida el responsable.
- **Nota v5** (§48).
  - Detección, sorteo y nacimiento con partición de la placa por la zona de fractura (`partir_por_zona_fractura`: la fuente más cercana por el océano, continentes enteros). Decisión mía: el corte sigue la dirección del escalón hasta los bordes.
  - Métricas: `nacimientos_zf`, `separar_zf`, `zf_escalon_km` y `zf_candidata_km`.
  - Dos pruebas nuevas.
  - Con `Δa_fz` = 10⁹ y la v7 apagada, la salida es igual sin las claves nuevas.
- **Lo que hacen en el planeta** (§49; 5 semillas, valores iniciales, sin calibrar).
  - La v7 baja la pérdida en choques de 3,5 a 1,0 AU, pero apila la corteza: ×28 celdas de más de 80 km, delaminación de 2,3 AU y sedimento al mar de 3,9.
  - Con `η_min` = 0,5 no se llega a 0,4–0,7.
  - Causa: en el modelo no hay márgenes adelgazados (0,1 % de lo que choca tiene menos de 20 km). El 75–81 % de lo que se pierde es corteza de tipo continental sobre fondo oceánico que no cuenta como arco.
  - La v5 rejuvenece el fondo (71 Myr de media en la deriva frente a 89) y los márgenes (147 Myr al final frente a 212). Pero la fosa sube a 86 500 km.
  - Sus candidatos (100 000–160 000 km, frente a 417 en la Tierra) son las costuras del fondo viejo del modelo, líneas de más de 1 000 km.
  - `Δa_fz` es redundante (todo candidato tiene un escalón de más de 70 Myr).
- **Herramientas nuevas:**
  - `diagnostico_zonas_fractura`;
  - el reparto de lo que choca por grosor en `deriva_en_varias_semillas`;
  - filas nuevas en `planeta_semillas.py`: nivel del mar, profundidad media, pérdida total en choques y nacimientos en zonas de fractura.
- **No se ha calibrado nada,** como pidió el responsable. `main` sigue en 524153b porque la rama tiene una prueba que falla a propósito.
- **Pendiente (§50), en orden, antes de calibrar:**
  1. la curva de profundidad;
  2. la regla de la v5;
  3. márgenes adelgazados en los rifts y la `η` de la corteza continental sobre fondo oceánico;
  4. después, calibrar y repetir la tabla de cierre.

## [2026-11-07] Curva de profundidad real, nota de fosas v6 (fuera las zonas de fractura), adenda v8 (rifts que estiran), procedencia del fondo y calibración

- **Qué pidió el responsable.**
  - Responder siempre en español (añadido a CLAUDE.md §5).
  - Adoptar la placa ajustada a la mediana del zócalo real (`d_∞` 5 310, `A` 2 373, `τ` 31) y anotar en TREE las mesetas, los montes submarinos y los puntos calientes (~150 m).
  - Nota de fosas v6: quitar las zonas de fractura y diagnosticar, sin cambiar nada, (a) de dónde salen los escalones de edad enormes y (b) cuánto fondo de más de 180 Myr y cuántos márgenes viejos vienen del estado inicial.
  - Adenda v8 §6.10: rifts que estiran los bordes en rampa conservando el volumen, y una India con su margen estirado.
  - Diagnosticar (c) qué es la corteza sobre fondo oceánico que se pierde en los choques.
  - Calibrar `η_min` y `λ_esp` con 5 semillas y repetir la tabla de cierre, con el nivel del mar y la profundidad media.
- **Hecho** (incremento-4-diseno.md §51–§58).
  - Curva nueva, con su prueba.
  - v5 retirada: código, `da_fz_myr`, métricas y pruebas.
  - Procedencia de cada muestra (`Corteza.proc`: `nace`, `lote` y `vias`, que nada lee). Sin la v5 y con la procedencia, el planeta es igual al de 17b9ce2, sin contar las claves retiradas.
  - v8 en `deriva/rifts.rs`, con prueba de conservación. Se retira `f_rift` y su sumidero `F_rift`; `w_margen_km` pasa a ser `W_m` y entra `h_rot_km`.
  - India con su margen estirado.
- **Escenarios India–Asia** (§52).
  - La curva nueva sube su nivel del mar de −1 890 a −1 255 m (su océano viejo es enorme y tiene el agua de la Tierra). El orógeno sobresale menos y se erosiona más despacio.
  - El margen estirado apenas cambia el máximo.
  - `D_0` recalibrado contra su observación (70–80 km): de 50 a 400, ~2,5·10¹⁹ Pa·s.
  - Quedan en rojo la relajación del orógeno (231–250 Myr frente a 100–200; `E_1` y `n_ero` se calibraron con aquel mar irreal) y el frenado tras romperse la losa (5,8 frente a 5,75). No se tocan sin el responsable.
- **Diagnósticos.**
  - (c) De lo perdido en los choques, el 78 % es corteza de tipo continental sobre fondo oceánico. Dentro de ella: arcos que dejaron de contar como arco al mezclarse (44 %), engrosamiento de choques (28 %), dientes, es decir astillas (12 %), y sedimento hecho corteza (12 %). Sedimento acrecionado y difusa, ~1 % cada uno.
  - La corteza continental fina casi no choca, ni con la v8: los márgenes que chocan son sobre todo los del estado inicial, sin estirar.
  - (a) Los escalones: relleno de huecos (33–48 %), clase «dorsal» del mismo linaje sin evento (20–44 %), suavizado, dientes e islas (4–23 %), invasión (4–12 %). Fusiones, saltos y rifts, ≤ 6 %; estado inicial, ≤ 8 % (domina solo al principio).
  - (b) Del fondo de más de 180 Myr (10–21 % del planeta) solo el 0–6 % es inicial, y de los márgenes viejos, el 1–11 %.
- **Márgenes estirados:** la v8 funciona, pero en el planeta casi no hay rifts (1–4 en 500 Myr con `τ_rift` = 150). Los márgenes pasivos tienen 35 km en la costa y una franja estirada de 65 km (Tierra: 100–300). Va con el ciclo de Wilson.
- **Calibración** (§55).
  - `η_min` = 0,5, el techo: la pérdida queda en 1,1 AU y no llega a 0,4–0,7.
  - `λ_esp` = 4·10⁻⁶ sin cambios: ni ×100 mueve la edad del fondo.
- **Tabla de cierre** (§57).
  - Mar a −305 m (antes −500/−1 000), océano de 3 952 m, plataforma de 8,2 km.
  - Pérdida en choques 1,1 AU (antes 3,5), ΔV_cont −10 %, área que se hunde 3,37 ✓.
  - Siguen mal la edad del fondo (119 Myr; 22 % de más de 180), los márgenes, el relieve, el sedimento (3,5 AU) y la fosa (×1,37).
  - **El tiempo:** 230 s solo, frente a un presupuesto de 200. No se ha subido.
- **Batería:** 28 márgenes del planeta movidos con su nota; el presupuesto de tiempo, no. Agentes idénticos a `main`.
- **Herramientas:**
  - `diagnostico_procedencia_del_fondo`;
  - en el informe de varias semillas, el reparto de la pérdida por vía y el perfil de los márgenes pasivos con la anchura de la franja estirada;
  - en `planeta_semillas.py`, las filas de nivel del mar, profundidad media y pérdida total.
- **`main` sigue en 524153b:** en la rama fallan a propósito las dos pruebas de escenario, hasta que decida el responsable.
- **Pendiente (§58), en orden:**
  1. las dos pruebas y el tiempo;
  2. el origen de arco por fracción;
  3. la representación que fabrica escalones y fondo viejo;
  4. las mesetas y los puntos calientes;
  5. lo que sigue de §45.

## [2026-11-07] Licencia propietaria y repositorio en GitHub

- **Qué pidió el responsable:** una licencia que deje claro que el proyecto es solo suyo, y subir el repositorio a `https://github.com/sime-svi/abm-engine.git` (remoto `origin`).
- **Hecho.**
  - `LICENSE` con «todos los derechos reservados», en español y con traducción al inglés: no se concede ningún permiso de uso, copia ni distribución sin autorización escrita.
  - Los datos de terceros de los que derivan las tablas de referencia (ETOPO, Paxman et al., Seton et al., GlobSed, Bird, CHELSA, WorldClim) conservan sus licencias.
  - Commit en `main` (a5b6583), sobre 524153b, y fusionado en la rama `terreno-cierre-tectonica` sin reescribir su historia.
  - Antes de subir se comprobó que el historial no tiene secretos: solo `infra/.env.example`, con marcadores; el `.env` real está ignorado y nunca se commiteó.
  - Se suben `main` y la rama de trabajo; las ramas antiguas ya están dentro de ellas.
- **A tener en cuenta:** si el repositorio de GitHub es público, sus condiciones de servicio dejan a otros verlo y bifurcarlo dentro de GitHub, diga lo que diga la licencia. Para que nadie lo vea, hay que hacerlo privado.

## [2026-11-07] Historia reescrita: los commits, solo a nombre del responsable

- **Qué pidió el responsable:** que en git no quede rastro de la herramienta como contribuidora, subirlo forzando y apuntar en CLAUDE.md que nunca se añada a los commits.
- **Hecho.**
  - Se quitaron de todos los commits de todas las ramas las líneas de coautoría y de enlace de sesión (29 de 38 commits).
  - Árboles, autores y fechas quedan idénticos: se comprobó commit a commit.
  - Cambian los identificadores de esos 29 commits. Las referencias de los documentos se actualizaron: por ejemplo, 5c8ef1c → 524153b, bd45fa6 → 17b9ce2 y la licencia 2a15c2b → a5b6583.
  - Se borró la historia vieja del almacén local y se subieron `main` y `terreno-cierre-tectonica` forzando.
  - CLAUDE.md §9: la herramienta nunca se añade a los commits (ni autor, ni coautor, ni enlaces de sesión, ni firmas), y el remoto `origin` va por SSH.
- **A tener en cuenta:** GitHub puede seguir mostrando un tiempo los commits viejos si alguien tiene su identificador exacto, hasta su limpieza interna. Su gráfico de contribuidores tarda en actualizarse.

## [2026-11-08] Última ronda de la tectónica: adenda v9 (arco por fracción), nota de rifts (por área y por suturas), erosión recalibrada y flujo lateral sin rehacer su red

- **Qué pidió el responsable.**
  - Contestar el §11 de la nota de rifts con referencias al código e implementarla, sustituyendo a `τ_rift`; calibrar `λ_A` con ~4–7 rupturas por 100 Myr y 5 semillas.
  - Implementar la adenda v9 (el origen de arco como fracción `a_i` mezclada por volumen, `η = a_i·η_arco + (1 − a_i)·η(h)`) y recalibrar `η_min` con 0,4–0,7 AU.
  - Aceptar `D_0` = 400; ampliar la tolerancia de la India tras la rotura para que entren 5,8 cm/año, sin tocar `c_col`.
  - Recalibrar `E_1` y `n_ero` con el mar nuevo (relajación en 100–200 Myr y exportación de ~1,8 AU) y decir si no se cumplen a la vez.
  - Subir el presupuesto del planeta a 250 s y, si era sencillo, hacer implícito el flujo lateral.
  - Anotar en TREE los escalones de edad del relleno de huecos y de las muestras que reaparecen como artefactos de representación.
  - Repetir la tabla de cierre y dejar `main` al día si pasan las pruebas. Después, congelar la capa.
- **Respuestas al §11** (`incremento-4-diseno.md` §59).
  - `τ_rift` era global, repartido por placa.
  - Las suturas no se guardaban como líneas (solo el evento y el `lote` de la procedencia).
  - `partir` solo cortaba por un círculo con ruido, pero generalizarlo costaba poco.
  - Además: con `p ∝ A`, el ritmo global no depende de lo juntos que estén los continentes.
- **Hecho** (§60).
  - **v9:** `Corteza.arco` pasa a fracción, con `de_arco()` y `continente()`. Se mezcla en `sumar_hc`, el flujo lateral y el estiramiento de los rifts, y `η` sale de la fórmula de la adenda.
  - **Decisión de realismo que se aparta de la letra de la adenda:** lo que deja un choque fuera de un continente conserva la fracción de arco de lo solapado; sobre un continente, 0. Con la letra, 0,93 AU de arco al año pasaban a «continental» y se perdían en el choque siguiente.
  - **Rifts** (`deriva/rifts.rs`):
    - suturas marcadas en las muestras al fundirse dos placas (`Corteza.sutura`);
    - bloques continentales, con una tirada por bloque `1 − exp(−λ_A·A·Δt)`;
    - trazado por la sutura más larga (de al menos 450 km) con la partición por fuente más cercana, o un círculo por el centro del bloque, con trazado alternativo;
    - `partir_por_lados` para cualquier corte;
    - eventos `rift` (trozos, sutura) y `rotura_rift`, y métricas de bloques y del perfil de los márgenes;
    - se retiran `τ_rift` y la regla vieja.
  - Tres pruebas nuevas de la nota: la sutura, el bloque pequeño y el determinismo.
  - **No regresión:** con los interruptores de medida apagados, el planeta era idéntico al de 1f8814d sin las claves nuevas. Después se retiraron los interruptores.
- **El flujo lateral.** El perfil mostró que no costaban los subpasos (~110 por paso), sino rehacer la red ~50 veces por paso: el 60 % del flujo. Un paso implícito no ahorraría (`D·Δt/d²` ≈ 7–10: tantas iteraciones de gradiente conjugado como subpasos). Se hacen cuatro ahorros idénticos byte a byte:
  - la red se amplía en vez de rehacerse;
  - solo se apuntan para mirar después las celdas que aún no son fuente;
  - se aplica por la posición de la muestra;
  - el nivel del mar y el sedimento comparten las alturas, y la bisección de `sed_hasta` sale cuando ya no puede estrecharse.
  Además, los bloques y el perfil de los márgenes calculan una vez qué celdas son de continente. El flujo pasa de ~60 a ~20 s por deriva.
- **Calibración** (5 semillas; §61–§63).
  - `λ_A` = 4·10⁻⁴: 4,9 ± 1,7 rupturas por 100 Myr en la deriva y 5,8 en el planeta.
  - `η_min` = 0,5, el techo: con 0,4 la pérdida sube.
  - **`E_1` = 0,0075** (con `n_ero` = 2,5): relajación en 171 Myr. **No se cumplen a la vez:** todas las combinaciones que relajan en 100–200 Myr exportan 5,1–5,5 AU. Las dos metas dependen de la erosión por encima de 2–3 km, y la causa es la base: el fondo que se hunde va a 4,6 km²/año y la tierra es demasiado alta.
- **India–Asia:** la tolerancia tras la rotura pasa a 0,55 veces la velocidad de antes (5,8 cm/año entra; la India real siguió frenando, queda como causa pendiente). La relajación cumple con `E_1` nuevo.
- **Tabla de cierre** (§64).
  - Mejoran: la edad media del fondo (119 → 88 Myr), el fondo de más de 180 Myr (22 → 13 %), el nivel del mar (−305 → −164 m), los choques (21 200 km, como Bird) y ΔV_cont (−8 %).
  - Empeoran: la fosa (×1,89), el área que se hunde (4,6), el sedimento al mar (5,1 AU), la pérdida en choques (1,4 AU) y el relieve.
  - Los márgenes de rift se estiran (95 km de franja), pero son el 10 % de la costa: el resto es del estado inicial.
  - **Planeta solo: 243 s; en la batería, con los agentes a la vez, 250,4 s: se pasa del presupuesto de 250 por 0,4 s** (con cinco a la vez, 264).
- **Batería:** 38 márgenes del planeta movidos con su nota (otra realización) y dos métricas nuevas (rupturas por 100 Myr y fracción por sutura). El presupuesto del planeta, a 250 s. Agentes sin cambios.
- **Herramientas:**
  - en `deriva_en_varias_semillas`: rifts, márgenes de rift, ciclos de supercontinente, coste del flujo y de la erosión y las métricas, y el arco que dejan los choques;
  - en `planeta_semillas.py`, las filas de la nota de rifts.
- **`main` no se actualizó en el primer commit de la ronda** (1c54153): el planeta se pasaba del presupuesto de 250 s por 0,4 s.
- **Después decidió el responsable** (2026-11-08):
  - acepta la fracción de arco fuera de los continentes, `E_1` = 0,0075, la India como pendiente y `λ_A` = 4·10⁻⁴;
  - sube el presupuesto del planeta a 275 s;
  - pide actualizar `main` y subir cada commit a GitHub en cuanto se haga (CLAUDE.md §9);
  - la capa no se congela todavía: antes se arregla el exceso de fosas (nota de demografía de placas).
- **Pendiente** (§65, en orden): la fosa y el área que se hunde; el relieve alto; los márgenes del estado inicial; la pérdida en choques; los artefactos de representación; la India que se acelera; el mar del escenario de relajación; las ampliaciones de la nota de rifts; lo que sigue de §58; el tiempo (243 s solo, 250,4 en la batería).

## [2026-11-08] Nota de demografía de placas: diagnóstico D1–D5, rama A y punto de decisión (sin fusionar)

- **Qué pidió el responsable.**
  - Subir el presupuesto del planeta a 275 s, actualizar `main` con la ronda anterior y subir la rama.
  - Aceptó la fracción de arco fuera de los continentes, `E_1` = 0,0075, la India como pendiente y `λ_A` = 4·10⁻⁴.
  - En CLAUDE.md §9: cada commit se sube en el acto, y Claude commitea cuando lo considera.
  - Con la nota de demografía de placas: diagnóstico del §0 (D1–D5, 5 semillas) y respuestas al §11; aplicar las ramas A, B y C que correspondan, midiendo tras cada una, y calibrarlas contra el §10. Si apunta a H4, parar y traer el diagnóstico.
  - Tabla de cierre con las filas de relieve, informe en `docs/informes/` y `MAPA_DEL_MOTOR.md`.
- **Hecho.**
  - `main` = 7c54bbd (la ronda g con el presupuesto de 275 s), verificado en verde y subido.
  - Rama nueva `terreno-demografia-placas`.
  - Diagnóstico `diagnostico_demografia_placas` (055da72), con lo que necesita en el modelo:
    - `Borde::v_rel`;
    - `Deriva::hundido_paso`;
    - `fosas_activas()`;
    - la longitud del evento de nacimiento inducido, que contaba los dos lados del borde.
  - Rama A (`soldaduras`, `capturas`; parámetros `v_fus`, `τ_fus`, `A_micro`, `τ_micro`) con tres pruebas.
  - Informe `docs/informes/2026-11-08_demografia_placas.md`.
  - `MAPA_DEL_MOTOR.md` creado (no existía).
- **Diagnóstico.**
  - Sobran placas grandes: ~20 de ≥1 % frente a 13 (H1). Nacen 21,3 por 100 Myr y mueren 19,3.
  - La invasión aporta el 15 % de la fosa creada (sin H2) y la difusa es ×0,75 de OCB (sin H3).
  - Borde inactivo, ~1 000 km. Las placas consumidas enteras seguían «vivas»: 29 al final.
- **Rama A.**
  - A1 no actúa nunca, ni en los extremos de su rango.
  - A2 retira las placas fantasma pero deja vivir más microplacas de la invasión.
  - La fosa baja un 7 %, dentro de la dispersión.
  - El planeta tarda 298 s solo, frente a 243: el reparto del sedimento al mar se encarece en todas las semillas, sin perfilar.
- **Se para:** tras A, ninguna rama encaja con la causa. Las placas grandes nacen en los rifts (~5 por 100 Myr) y solo mueren en las suturas (~3); las oceánicas grandes no se consumen enteras. Opciones para el responsable, en el informe (§8).
- **Tabla de cierre:** la tierra por encima de 2 000 m es el 15 %, frente al 5,3 % de la roca de ETOPO sin hielo (el ~11 % de la nota incluye el hielo).
- **`main` no se toca:** la rama A no mejora el exceso y se pasa del presupuesto de tiempo. Decide el responsable.

## [2026-11-08] La Tierra real en la regla del modelo, para los continentes: no sale el relieve real y la ronda se para

- **Qué pidió el responsable** (encargo de la segunda ronda del día).
  - **Primero:** meter el grosor real de la corteza (CRUST1.0) y su sedimento en la isostasia del modelo, con su nivel del mar, y comparar con ETOPO sin hielo: altura media de la tierra, por debajo de 200 m, por encima de 2 000 m, pendiente y plataforma. Medir también el grosor de la corteza continental del planeta frente a CRUST1.0. «Si con la corteza real no sale el relieve real, para y tráemelo.»
  - **Después** (en espera): la nota de demografía de placas v2 (fuera A1; A2 y las fantasma se quedan; rama D de subducción de dorsales con `d_dc` 300 km; D6) y la nota de rifts v2 (reloj por bloque con `τ_ais` 100 Myr; recalibrar `λ_A` y `τ_ais` con las ~13 placas de ≥ 1 % y la duración de los supercontinentes).
  - El objetivo de la tierra por encima de 2 000 m pasa a 5,3 %. Presupuesto de 275 s. Tabla de cierre con 5 semillas, informe y mapa.
- **Hecho.**
  - Herramienta `herramientas/analisis/tierra_corteza_regla.py`, con salida en `config/referencias/tierra_corteza.toml`.
  - Usa CRUST1.0 interpolado de 1° a nivel 3, solo entre celdas continentales; LITHO1.0 para el manto litosférico; el océano con la regla de §46, y medidas como `referencia_tierra.py`, también a 1°.
  - Informe `docs/informes/2026-11-08_corteza_real_en_la_regla.md`; diseño §67–§68; README de la fase; mapa del motor; TREE (pendiente 0 de realismo); README de herramientas.
- **Resultado: no sale.**
  - Con la corteza real, la regla da 1 389 m de altura media de la tierra (671), el 19,7 % por encima de 2 km (5,3 %), el 4,7 % por debajo de 200 m (28,3 %), una plataforma de 17 km (43; 54 a 1°) y la pendiente ×1,65 a 1°.
  - Sale más alta que el propio planeta (1 136 m, 15 %): el exceso de relieve del planeta es sobre todo de la regla.
- **Por qué.**
  - La regla da 152 m por km de corteza y 273 por km de sedimento. En la Tierra, con CRUST1.0: 65–75 m por km de corteza y −17 a −21 por km de sedimento.
  - Por tipo (real − regla): escudos y plataformas, −981 m (el 61 % de la tierra); Fanerozoico, −643; orógenos, −669; arcos, +19; rifts y corteza estirada, +263.
  - Ejemplos: la plataforma rusa daría 2,1 km (real 110 m); Kenia, 0,34 km (real 2,26).
  - La plataforma real emerge: el 69 % queda en tierra con la regla.
  - No lo arreglan las densidades de las capas de CRUST1.0 (2 816 en tierra, casi la del modelo), quitar el sedimento ni mover `h_nivel` (con 47 km la tierra queda en el 10 % del planeta).
  - LITHO1.0 explica poco: −2,3 m por km de manto litosférico.
  - Lo que falta es la isostasia térmica del manto litosférico según la edad termotectónica, y la subsidencia de cuencas y márgenes. Un término perfecto por tipo daría como mucho un 7,2 % por encima de 2 km y 904 m de media.
- **La corteza del planeta frente a CRUST1.0** (en tierra).
  - Misma mediana (37,7 y 37,3 km), pero el doble de más de 45 km (20 %, frente a 9 %) y casi el triple de más de 50.
  - Un tercio del sedimento (0,45 km, frente a 1,24) y un tercio de la corteza continental sumergida (3,5 % del planeta, frente a 9,2 %).
- **Decisión que espera** (opciones en el §6 del informe):
  1. isostasia térmica por edad termotectónica (recomendada, la más realista);
  2. densificación de la corteza inferior gruesa;
  3. la relación medida como apaño provisional;
  4. seguir con la regla.
  - La rama D y los rifts v2 no dependen de la isostasia y podrían ir antes.
- **Las notas v2 del responsable** se commitean tal como llegaron, sin implementar.
- **Estado:**
  - el motor no cambia;
  - la rama `terreno-demografia-placas` sigue sin fusionar, con la rama A;
  - `main` sigue en 7c54bbd.

## [2026-11-08] Estudio de isostasia continental; nota de demografía de placas v2 (rama D, D6) y nota de rifts v2 (reloj del bloque)

- **Qué pidió el responsable.**
  - «Bien parado.» Un estudio, sin tocar el motor (`estudio_isostasia_continental.md`):
    - reproducir la isostasia de litosfera completa de Lamb et al. (2020) con sus datos (Zenodo 4031646);
    - evaluar las reglas R0–R4 con la Tierra real, con validación cruzada entre mitades de continentes;
    - dar la tabla del §4 y si se cumplen los criterios del §5, con su informe.
  - Mientras tanto, implementar la nota de demografía v2 (rama D y D6, sin A1) y la de rifts v2 (reloj), calibrar `λ_A` y `τ_ais` con las placas grandes (~13), medir placas y fosas, y no hacer la tabla de cierre del relieve. Presupuesto, 275 s.
- **El estudio** (`herramientas/analisis/estudio_isostasia.py`, `config/referencias/tierra_isostasia.toml`, informe `docs/informes/2026-11-08_estudio_isostasia_continental.md`, diseño §70).
  - **Datos:** el PDF del artículo solo tenía imágenes, pero sus constantes salen de sus rejillas (α 7,8 y β 93). La litosfera que usaron es CAM2016Litho (EarthScope). TC1 solo es libre a 5°.
  - **Reproducción:** correlación 0,92 en la tierra continental (en el océano usan otra regla para el agua).
  - **R4** (litosfera de la edad tectonotérmica con recuperación, manto arcaico más ligero, corteza densa abajo y sedimento de relleno): error medio 451 m en las mitades de prueba, frente a 677 de R2 con la litosfera observada; altura media 648 m.
  - **Ninguna regla cumple el §5:** la tierra baja (15 % frente a 28 %) y los casos con nombre (llanura rusa, Kenia, Tíbet) son topografía dinámica de más de unos cientos de metros. Los parámetros acaban en los bordes de sus rangos.
  - **Reparto del error de R0:** densidad de la corteza ~45 %, sedimento ~1/3, manto litosférico ~1/4.
- **La ronda de placas** (informe `docs/informes/2026-11-08_demografia_placas_v2.md`, diseño §69, commit cf1b87d).
  - A1 fuera.
  - **Rama D:** una dorsal con la fosa delante (`θ_dc` 45°) a menos de `d_dc`, bajo una tercera placa, pasa su franja a la placa del otro lado y apaga la fosa.
    - **Decisión contra la letra:** a la letra actuaba en los puntos triples, comiéndose 300 km de fosa por paso (538 veces por 100 Myr). Se midieron las dos.
  - **Rifts v2:** reloj por bloque (`reinicio_myr`), reiniciado en los rifts y en las suturas entre continentes. Los cratones de partida empiezan con un reloj al azar en [0, `τ_ais`].
  - **Resultado:** placas de ≥ 1 % 13,8 (antes 18,3); fosa 71 800 km (×1,40; antes ×1,75); 3,55 km²/año; el fondo envejece (110 Myr al final, antes 86).
  - **Calibración:** `λ_A` 4·10⁻⁴ y `τ_ais` 100 se quedan. No se forma ningún supercontinente: el bloque mayor es como mucho del 46 %.
  - **Tiempo:** 256 s, también en la batería (presupuesto 275). 35 márgenes del planeta fuera: no se mueven hasta la isostasia; `main` sin tocar.
  - India–Asia apaga la rama D en las esquinas artificiales de su escenario.
- **Decisiones que esperan:**
  - la regla de isostasia (R4 como base o estudiar la topografía dinámica);
  - la rama D, de frente o con la letra;
  - la edad del fondo;
  - después, la tabla de cierre y los márgenes de la batería.

## [2026-11-08] Isostasia con litosfera (R4 y edad tectonotérmica), erosión recalibrada y diagnóstico del supercontinente

- **Qué pidió el responsable.**
  - Los informes, siempre como archivo de `docs/informes/` (el texto de la terminal le llega cortado).
  - Isostasia: acepta R4; la topografía dinámica va a ampliación futura (TREE.md). Implementar `nota_isostasia_litosfera.md`:
    - primero su §0: repetir el ajuste de R4 sin las celdas con más de 300 m de residuo de Lamb, y usar esos parámetros si salen del borde de sus rangos;
    - luego, la edad tectonotérmica por celda y R4 en lugar de la isostasia actual, y responder a su §11.
  - Acepta la rama D «de frente».
  - Después: recalibrar `E_1` y `n_ero` con 5 semillas, la tabla de cierre completa con las filas nuevas, el diagnóstico del supercontinente (sin cambiar nada; 5 semillas, cada 25 Myr) y lo que cuesta en nivel del mar la edad del fondo. Presupuesto, 275 s.
- **§0** (`herramientas/analisis/nota_isostasia_s0.py`, `config/referencias/tierra_isostasia_s0.toml`).
  - Queda el 33 % de la tierra. Salen del borde `Δρ_ma` (25 con el sedimento como corteza), el manto arcaico (15) y `L_min` (50): se usan.
  - Siguen en el borde ρ_u 2 750, ρ_l 3 000 y z_t 15: limitación anotada en TREE.md.
- **Implementación** (diseño §72).
  - R4 en `isostasia.rs`, con el sedimento contado como corteza: la variante que el reparto del sedimento puede llevar (§11.1), casi igual de buena con la Tierra real.
  - `Corteza::edad_tt`: envejece; se reinicia con un rift al romper, los arcos, los choques y la delaminación de más de 1 km; se mezcla por volumen en `sumar_hc` y en el flujo lateral.
  - Edades de partida: la Tierra de hoy de más de 500 Myr, menos 500, por la distancia a la costa.
  - La erosión baja la superficie con la derivada de R4; `f_levantamiento` también. Fuera `h_nivel_km`.
  - Se retiró el interruptor de la rama D.
  - Métricas nuevas, `rift_id` y el diagnóstico del supercontinente (`deriva/diagnostico.rs`, solo mide), con su herramienta `supercontinente.py`.
  - Dos pruebas exploratorias: el nivel del mar con las edades de la Tierra y la huella de la deriva.
- **Erosión** (5 semillas, para cada `n_ero` el `E_1` que relaja en ~160–175 Myr): `E_1` 0,02 con `n_ero` 2,5 (159 Myr).
  - Con R4 y la erosión vieja, la tierra queda a 1 192 m y relaja en 411 Myr.
  - n 2 y n 3 quedan dentro de la dispersión entre semillas.
  - La exportación (4,4 AU) no se calibra: la limita la base.
- **Resultados** (informe `docs/informes/2026-11-08_isostasia_litosfera.md`, diseño §73).
  - La tierra a 781 m (la Tierra 671; antes 1 136); un 7 % por encima de 2 000 m (5,3); un 16 % por debajo de 200 m (28); nivel del mar −140 m.
  - Las clases de edad viejas cuadran; la de 100–500 Myr sale alta (697 m, frente a 326) por la regla y por el nivel del mar.
  - **Los márgenes apenas se hunden al enfriarse:** 0,38 km en 100 Myr, frente a 2–3. El manto de R4 es el continental empobrecido; bajo un rift sube astenosfera fértil. Decide el responsable.
  - **La edad del fondo** (84 Myr) no cuesta nivel del mar: con las de la Tierra el mar estaría 57 m más bajo, porque el modelo tiene también más fondo joven.
  - **Supercontinente:** los océanos de rift se abren (13,4 por semilla), un margen se activa a los ~69 Myr, pero casi nunca se cierran (0,8). Los continentes no convergen. Sus placas las mueve el tirón de sus losas; no hay succión de la fosa sobre la placa de arriba.
- **Incidencias.**
  - **Determinismo:** una semilla salió distinta en la novena cifra porque `target/release/terreno` se recompiló a mitad de una tanda. Con el binario quieto, 8 de 8 corridas idénticas y la huella de la deriva igual en 4 procesos. `planeta_semillas.py` corre ahora una copia del binario.
  - **Tiempo:** 283 s solo (275). La versión anterior tarda hoy 288,6 s en las mismas condiciones: la máquina va más lenta que en la ronda anterior (256).
  - **Batería:** 28 márgenes del planeta movidos con su nota; todo dentro salvo el tiempo.
  - **Tolerancia ampliada** (para que la revise el responsable): la rotura de la losa de India–Asia respecto al frente visible del choque, de ±3 a ±4 Myr.
- **Decisiones que esperan:**
  - el hundimiento térmico de los márgenes;
  - el presupuesto de tiempo;
  - el supercontinente (¿nota sobre la succión de la fosa?);
  - la corteza demasiado joven y el exceso de fosa;
  - la tolerancia ampliada;
  - fusionar con `main`.

## [2026-11-08] Fusión con main, regla relativa del tiempo; notas de hundimiento de los márgenes, de puntos calientes y de succión de las losas

- **Qué pidió el responsable.**
  - Fusionar la rama de la isostasia con `main`.
  - El presupuesto de tiempo, como regla relativa: no más de un 5 % más lento que la versión anterior medida en la misma tanda, con un techo de 330 s.
  - Acepta la tolerancia de ±4 Myr de la prueba de la losa.
  - Tres notas en orden, midiendo con 5 semillas tras cada una, y responder a su §11: hundimiento de McKenzie en las franjas estiradas; puntos calientes (abombamiento, trazas, grandes erupciones, mesetas que flotan y se acrecionan, `F_int` en el balance); succión de las losas profundas y de los restos, y soldadura tras un choque agotado.
  - La vida de los márgenes, como entidades que nacen en cada rift.
  - Recalibrar (`E_1`, `ε_suc`, el arrastre basal y los caudales de los puntos calientes) y la tabla de cierre completa, con la tierra joven que reinician arcos y choques, y la fosa y el área que se hunde con 10 semillas.
- **Fusión y regla** (`main` = 473b805).
  - La batería corre el planeta a la vez que su versión anterior (worktree en `output/referencia/`). Uno tras otro, el segundo salía un 2–5 % más lento.
  - La rama pasaba en +5,4 %. Se dejó en +4,2 % calculando cada 25 Myr el recuento de márgenes por celdas de costa y su perfil (salida del modelo idéntica salvo esas medidas).
  - Los márgenes, como entidades `MargenRift`.
- **Las notas** (rama `terreno-margenes-calientes-succion`: fde4e69, f15c5b4 y 7153f47; diseño §75–§77; informe `docs/informes/2026-11-08_margenes_calientes_succion.md`).
  - **McKenzie:** `β` y `t_rot` por muestra; la carga `max(0, S_McK − S_R4)` en R4. El rift aislado sigue la curva exacta; en el planeta, 1,5–1,6 km con β 2–3 y > 3. La plataforma no mejora; el mar baja 48 m.
  - **Puntos calientes:** 12, con mesetas que no pasan a continentales (con R4 quedarían a ~0,7 km bajo el mar). El mar sube 88 m. Magmatismo oceánico 1,41 AU con `q` 1,25·10⁵; 4,1 grandes erupciones por 100 Myr. Los basaltos continentales dan 0,03 AU frente a ~0,12: no se fuerza `h_CFB`.
  - **Succión:** calculada en la malla gruesa, con el par repartido por las celdas finas. Hubo un fallo: una microplaca disparaba su velocidad.
    - `ε_suc` 0,05: el tirón es el 58 % de la potencia y las fosas retroceden en el 66 %, a 1,0 cm/año (objetivo 1,3–1,5).
    - La soldadura (suturas) pide ahora la losa rota y 20 Myr.
- **El hallazgo: los choques no se agotan nunca.** Convergen a 1,56 → 1,26 cm/año con la edad, nunca por debajo de 0,5; solo hay ~0,9 soldaduras por 100 Myr.
  - Con la succión, los continentes se acercan y chocan más (30 000 km): la corteza se destruye (−13 %), el mar baja a −202 m y la tierra sube a 963 m.
  - El mayor bloque de una placa, 14 %: no hay supercontinente.
- **Otros números.**
  - Fosa 82 900 ± 7 000 km y 3,68 ± 0,38 km²/año (10 semillas).
  - La tierra de menos de 500 Myr la reiniciaron los arcos (54 %) y los choques (40 %).
  - Los márgenes (entidades) viven 86 Myr.
  - Tiempo +5,0 % frente a `main` (la regla, justo); la succión, ~7 s directos.
  - La batería: 29 márgenes movidos con su nota.
- **Decisiones que esperan:**
  - la resistencia de los choques;
  - dejar o apagar la succión;
  - el retroceso de las fosas y los restos de losa;
  - los basaltos continentales;
  - fusionar la rama.

## [2026-11-08] Congelación del planeta inventado v1 y fusión con main

- **Qué decidió el responsable.**
  - Congelar el planeta inventado tal como está y pasar al modo Tierra real (`docs/design/terreno/diseno_modo_tierra.md`).
  - La succión de las losas, apagada por defecto (`ε_suc` = 0); el código se queda.
  - Acepta las decisiones de la ronda anterior:
    - las mesetas, como corteza flotante no continental;
    - los márgenes, como entidades;
    - el caudal de las trazas;
    - los basaltos continentales, a 0,03 AU (la cifra de Clift incluye otro magmatismo continental).
  - Nueva regla en `CLAUDE.md` §4: cada capa tiene un máximo de tres rondas y se juzga por lo que necesita la capa siguiente.
- **Hecho.**
  - `ε_suc` = 0 en `config.rs` y `terreno.toml`, con su valor calibrado (0,05) en el comentario. La prueba del resto de losa la enciende con ese valor.
  - La fila de los basaltos de `planeta_semillas.py`, con el objetivo aceptado.
  - Documento de cierre: `docs/design/terreno/cierre_planeta_inventado_v1.md`, con la tabla de cierre sin succión (5 semillas), las decisiones fijadas y la ampliación futura en orden:
    1. la resistencia de los choques;
    2. la succión;
    3. el supercontinente;
    4. el exceso de fosas;
    5. la corteza joven;
    6. la plataforma;
    7. el retroceso de las fosas;
    8. la topografía dinámica.
  - **La tabla, sin succión:**
    - tierra a 902 m;
    - mar a −106 m;
    - fondo de 91 Myr;
    - ΔV −6,3 %;
    - fosa ×1,7;
    - choques de 23 400 km;
    - márgenes como McKenzie (1,47 y 1,66 km);
    - magmatismo oceánico de 1,36 AU;
    - 4,1 grandes erupciones por 100 Myr.
  - **Batería:** 238,0 s frente a 246,4 de `main` (473b805) a la vez, −3,4 %. 27 márgenes del planeta movidos con su nota (otra realización del planeta).
  - Fusión de `terreno-margenes-calientes-succion` con `main`.

## [2026-11-08] Modo Tierra real, Fase 1 (rama terreno-modo-tierra)

- **Qué pidió el responsable:** responder al §11 de `diseno_modo_tierra.md` antes de implementar, implementar la Fase 1, validar con su §10 (el modo inventado, idéntico byte a byte) y dejar el informe en `docs/informes/`.
- **§11:**
  - **Interfaz del proveedor de mundo:** los campos de nacimiento de cada celda (altura, hielo, textura del suelo, recurso) y el clima de cada día (temperatura y precipitación). El resto lo deriva el motor. Los agentes no leen hoy nada del terreno.
  - **Carga:** en el nivel 3 (26 km); CRU TS se queda en su rejilla. La caché ocupa 0,51 GB.
  - **Licencias:**
    - ETOPO, dominio público;
    - CHELSA, CC0;
    - CRU TS, OGL v3 (de la UEA, sin registro);
    - HydroRIVERS, libre con cita;
    - MODIS, NASA libre (versión a 0,5° del ICDC, CC BY 4.0).
  - **Vegetación frente a MODIS:** no, por construcción (biomasa sin unidades).
- **Hecho** (`crates/abm-terreno/src/tierra/`):
  - lectores mínimos de GeoTIFF y de NetCDF-3 en flujo, con `miniz_oxide`, la única dependencia nueva;
  - proyección de ETOPO, CHELSA (todos los píxeles, un hilo por archivo) y CRU TS al nivel 3, y los niveles gruesos por medias;
  - años reales por bloques con el RNG por contador;
  - mapas mensuales con las anomalías, y temperatura interpolada entre mitades de mes;
  - la columna `hielo`;
  - la salida v8 (`run.tierra`, `diagnostico.tierra`).
  - Se activa con `--set mundo.modo=tierra`: 7 s por año con las cachés y 1 GB.
- **Decisiones con datos:**
  - **La regla del hielo** se aparta de la nota: hace falta la superficie sobre el mar y ≥ 10 m de espesor (`hielo_min_m`). Con «> 0», el ruido entre las dos versiones de ETOPO daba 1,2 Mkm² de hielo sobre el mar del Ártico.
  - **CHELSA V2.1 tiene clima sobre el mar,** hasta 84° N.
- **Validación** (`herramientas/tierra/validar.py`):
  - ✓ relieve, conservación entre niveles (10⁻¹⁴), hielo (15,6 Mkm²), clima medio frente a CHELSA medido aparte, coeficiente de variación de la lluvia anual en 13 regiones y el Sahel de 1984;
  - ✗ la autocorrelación de un año al siguiente (×0,87) y la vegetación del motor frente a MODIS (r = 0,17);
  - Miami con el clima real: 61 PgC/año y r = 0,80;
  - los ríos frente a HydroSHEDS quedan pendientes: Vodafone bloquea `data.hydrosheds.org` (Cloudflare).
- **Hallazgo grande:** la hidrología del motor sobre el relieve real da 9,4 Mkm² de lagos y ríos por sitios que no son. Hay tres causas:
  - gargantas tapadas al promediar (el Congo sale por el Ogooué);
  - cuencas cerradas que se desbordan (el Chad sale por el Níger);
  - la tierra bajo el mar cuenta como mar (el Nilo acaba en El Fayum).
- **No regresión:** el planeta inventado sale idéntico byte a byte frente a 42c70f9.
- **Decisiones que esperan:**
  - la hidrología;
  - la vegetación (¿Miami?);
  - los glaciares de montaña (RGI 7.0);
  - la autocorrelación;
  - fusionar la rama.

## [2026-11-08] Adenda 1 del modo Tierra, en curso (rama terreno-modo-tierra-adenda1)

- **Qué pidió el responsable:**
  - fusionar `terreno-modo-tierra` con `main` (hecho: `main` = ca6ddbe);
  - `adenda_modo_tierra_1.md`, segunda ronda de la capa (queda una como mucho):
    - ríos y lagos como dato (HydroSHEDS reescalado por píxel de salida, cuencas cerradas que no rebosan, mar solo lo conectado, lagos de HydroLAKES y caudal encaminado);
    - vegetación = producción de Miami del año, repartida por meses;
    - glaciares del RGI 7.0;
    - remuestreo solo de años con estaciones en el 90 % de la tierra, y rachas de años secos;
    - acepta la regla del hielo y la hipsometría a ±1,4 %.
- **Hecho y probado** (las pruebas pasan, clippy limpio; planeta inventado y agentes idénticos byte a byte frente a `main`):
  - **Remuestreo:** `cobertura_min` 0,9, con 1938–2010 (73 años). Las rachas de 2 o más años bajo el 75 % y las del quinto seco quedan dentro del ruido de Poisson en las 13 regiones: se acepta la autocorrelación.
  - **Una sola producción:** `abm_terreno::vegetacion::ppn_miami`, que reexporta el hábitat de los agentes (`abm-ruleset` depende ahora de `abm-terreno`). En el modo Tierra:
    - `veg` es la producción del año;
    - `prod_mes`, la del mes;
    - con el año de la semilla 42 (1987), 59,3 PgC/año.
  - **Glaciares:** la tabla del RGI 7.0 (réplica de OGGM; NSIDC pide cuenta), repartida por centro y área. +0,54 Mkm², hielo 16,1 Mkm²; regiones a ±2 %.
  - **Hidrología** (`tierra/hidro.rs`, `tierra/lagos.rs`, lector general de GeoTIFF con LZW y teselas):
    - el reescalado, el mar conectado, los lagos rasterizados y la red de la malla de niveles mezclados (`hydro.rs: red_de_la_tierra`);
    - la escorrentía encaminada, con las cuencas cerradas evaporando;
    - el LOD afina las celdas que mezclan cuencas.

    Probado con TIFF inventados: sin ciclos y sin cuencas que rebosen.
- **Lo que falta:** los datos de HydroSHEDS. Vodafone bloquea `data.hydrosheds.org` (Cloudflare, por LaLiga): 16 intentos de 21:40 a 23:08, todos bloqueados.
  - El responsable se ofreció a bajarlos desde otra red.
  - Hasta tenerlos, el modo Tierra no arranca (los pide la carga).
  - Después:
    1. proyectar;
    2. validar con las tablas de la adenda;
    3. márgenes de la batería;
    4. informe, mapa, README y TREE.
