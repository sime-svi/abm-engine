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
