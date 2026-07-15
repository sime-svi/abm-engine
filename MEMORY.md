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
