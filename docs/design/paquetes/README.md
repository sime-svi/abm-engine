# Fase «paquetes» — el código dividido en piezas

> Explicación en lenguaje llano (CLAUDE §8.1). Esta fase no cambia lo que simula el proyecto: cambia **cómo está organizado** el código. Al final hay un breve apartado técnico.

## ¿Qué es esta fase?

Hasta ahora todo el código vivía en un único **paquete** de Rust (en la jerga de Rust, un *crate*: una caja de código que se compila de una vez). Funcionaba, pero mezclaba cosas de naturaleza muy distinta: el motor que hace avanzar el tiempo, las reglas de los bichitos, el planeta, las métricas y la base de datos.

Ahora el proyecto es un **workspace** (un conjunto de paquetes que se compilan juntos, cada uno con una sola responsabilidad), tal como pedía desde el principio `CLAUDE.md` §3:

| Paquete | Qué contiene, en una frase |
|---|---|
| `abm-core` | **El motor:** el reloj de ticks, el azar sembrado y el planificador que ejecuta los sistemas de uno en uno y siempre en el mismo orden. No sabe nada de bichitos ni de planetas. |
| `abm-ruleset` | **Las reglas del modelo de forrajeo:** los bichitos, el tablero, cómo comen, se reproducen y mueren, y en qué orden pasa cada cosa en un tick. |
| `abm-terreno` | **El planeta:** la esfera, la malla que se afina sola, el clima, el agua, la vegetación, la erosión y el suelo. |
| `abm-metrics` | **Los observadores:** cada tick resumen el estado del modelo en unas pocas cifras (población, energía…). |
| `abm-persistence` | **La salida:** el formato de esas cifras y los destinos donde se guardan (archivo JSON o base de datos). |
| `abm-runner` | **Los programas que ejecutas:** `abm` (los bichitos) y `terreno` (el planeta). Leen la configuración, juntan las piezas y arrancan. |

## ¿Para qué sirve?

- **Cada pieza se puede compilar y probar por separado**, y un cambio en una no obliga a recompilar las que no dependen de ella.
- **Es imposible saltarse la regla de oro del proyecto:** el motor no puede saber nada de las reglas. Antes era una norma que había que vigilar; ahora la impone el compilador, porque `abm-core` ni siquiera tiene acceso a los demás paquetes.
- **Prepara el siguiente paso:** juntar los bichitos con el planeta. Los dos mundos ya comparten el mismo motor y pueden encajarse sin enredarse.

## Consideraciones y decisiones (05-11-2026)

- **El planeta tiene paquete propio** (decisión del responsable). Otra opción era meterlo dentro de las reglas. Se eligió separarlo porque es grande, tiene su propio programa y no depende del motor de bichitos. Se añadió a `CLAUDE.md` §3.
- **Hubo que deshacer tres nudos** que en un solo paquete pasaban desapercibidos:
  1. *El motor conocía las reglas.* Montaba el orden de los sistemas del forrajeo y creaba el mundo con sus bichitos. Eso pasó a `abm-ruleset`. En el motor quedó solo lo genérico: el reloj, el azar, el planificador y el bucle de ticks.
  2. *Métricas y persistencia se necesitaban mutuamente.* Las métricas escribían en el destino de la persistencia, y la persistencia usaba el formato de las métricas. Entre paquetes, ese círculo está prohibido. Se rompió poniendo el formato y el "enchufe" de salida en `abm-persistence`, como ya decía `CLAUDE.md` §7.
  3. *La base de datos recibía la configuración entera del forrajeo.* Ahora recibe solo los datos que guarda (nombre, semilla, versión, la config como texto), así que sirve para cualquier modelo.
- **Cada paquete tiene sus propios errores.** El motor solo conserva los de leer una configuración, que comparten todos.
- **Nada cambia por fuera:** los dos programas dan una salida **idéntica byte a byte** a la de antes de la división (comprobado). Los comandos siguen siendo los mismos (`cargo run --release -- config/default.toml`, `cargo run --release --bin terreno`, `cargo test`).
- **Un detalle práctico:** Cargo ejecuta los tests desde la carpeta de cada paquete, así que los tests que leen `config/default.toml` lo buscan ahora con la ruta del workspace.

## Apartado técnico

**Dependencias entre paquetes** (comprobadas con `cargo tree`; las flechas apuntan hacia `abm-core`, que no depende de ninguno):

```
abm-core         ← (nada)
abm-ruleset      → abm-core
abm-terreno      → abm-core
abm-persistence  → (ninguno del proyecto)
abm-metrics      → abm-core, abm-ruleset, abm-persistence
abm-runner       → todos
```

**Dónde quedó cada cosa** (antes → ahora):

| Antes (un paquete) | Ahora |
|---|---|
| `src/core/{time,rng}.rs` | `crates/abm-core/src/` (+ `run_ticks`, el bucle de tiempo) |
| `src/core/schedule.rs` | genérico en `abm-core` (`schedule::deterministic`); el orden del modelo, en `abm-ruleset/src/schedule.rs` |
| `src/core/world_setup.rs`, `src/config.rs`, `src/domain/**` | `crates/abm-ruleset/src/` |
| `src/metrics/collect.rs` | `crates/abm-metrics/src/` |
| `src/metrics/schema.rs`, `src/persistence/**` | `crates/abm-persistence/src/` |
| `src/terreno/**`, `build.rs`, `tests/{terreno,erosion}.rs` | `crates/abm-terreno/` |
| `src/lib.rs`, `src/main.rs`, `src/bin/terreno.rs`, `tests/{determinism,persistence_pg}.rs` | `crates/abm-runner/` |
| `src/error.rs` | `ConfigError` en `abm-core`; `PersistenceError`, `TerrenoError` y `SimError` en sus paquetes |

Los ficheros se movieron con `git mv`, así que conservan su historial. Las versiones de las dependencias externas están fijadas una sola vez en el `Cargo.toml` raíz (`[workspace.dependencies]`), y `bevy_ecs` sigue compilado sin la parte multihilo en todo el workspace.
