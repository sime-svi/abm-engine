# CLAUDE.md — Instrucciones maestras del proyecto

> Este archivo es la fuente de verdad para cualquier agente (Claude Code) que trabaje en el repositorio. Léelo entero al inicio de cada sesión. Si algo aquí entra en conflicto con una petición puntual, señálalo antes de actuar.

---

## 1. Visión del proyecto

Motor de simulación masiva basada en agentes (ABM) orientado a **comportamiento emergente**. No es un videojuego: es un entorno de cálculo puro que evalúa interacciones a escala macroscópica (geopolítica, demografía, conflictos) y microscópica (sistemas biológicos, fisiología, enfermedades), aplicando estrictamente Nivel de Detalle (LOD) computacional para evitar la explosión combinatoria.

La salida son datos: registros históricos, métricas y eventos estructurados, volcados a archivos locales (JSON) y/o a PostgreSQL para su análisis y visualización asíncrona posterior.

---

## 2. Principios inquebrantables

Estos principios no se negocian sin discusión explícita con el responsable del proyecto:

1. **Lenguaje: Rust.** Sin excepciones para el motor. Elegido por seguridad de memoria y concurrencia masiva. Las herramientas fuera del motor (análisis, medidas, verificación; carpeta `herramientas/`) pueden usar Python o bash *(confirmado por el responsable el 2026-11-05)*.
2. **Paradigma: ECS puro (Entity Component System).** Prohibida la POO tradicional para las entidades. Todo se modela como datos en arrays contiguos (Struct of Arrays) para maximizar el aprovechamiento de la caché de CPU. Referencia: Bevy ECS en modo headless o Flecs.
3. **Headless al 100%.** Cero renderizado, cero UI en tiempo real. La única salida es el volcado de datos.
4. **Determinismo y reproducibilidad.** Toda ejecución debe poder replicarse bit a bit a partir de su semilla y su versión de ruleset. Ver sección 6.
5. **LOD / Lazy Evaluation.** El detalle se calcula y se almacena solo cuando aporta valor. Lo que no se observa a alto detalle se resuelve con agregados estadísticos.
6. **Desacoplamiento Core / Ruleset.** El motor (tiempo, memoria, scheduling, concurrencia) es agnóstico a las reglas. Las reglas (ecuaciones, parámetros, lógica de estados) no conocen los detalles internos del motor.
7. **Abstracción matemática primero.** Todo comportamiento complejo se reduce a fórmulas y estados transaccionales **antes** de codificarse. El diseño matemático vive en `docs/design/` y precede a la implementación.
8. **Realismo primero.** El objetivo es un mundo real, no cumplir cifras escritas. Si una calibración no alcanza un objetivo realista, **primero se duda de la base** (los datos de entrada, el relieve, las condiciones iniciales, la propia fórmula) y solo después de un parámetro. Nunca se fuerza un parámetro hasta un valor irreal para compensar otra irrealidad: construir sobre bases irreales y añadir capas encima acaba en un sistema irreal. Si hace falta un apaño provisional, se marca como tal y se anota la causa real que hay que corregir. *(Confirmado por el responsable el 2026-11-05.)*

---

## 3. Arquitectura del repositorio (workspace de crates)

El proyecto es un **workspace de Cargo** con crates de responsabilidad única. Esto materializa el desacoplamiento y permite compilar/testear cada capa por separado.

- **`abm-core`** — El motor. Bucle de tiempo (tick), scheduler de sistemas, gestión de mundo/entidades, RNG determinista, paralelismo. No contiene ninguna regla del dominio.
- **`abm-ruleset`** — Las reglas. Ecuaciones, parámetros, lógica de estados, componentes del dominio (recursos, salud, población, etc.). Depende de `abm-core` pero no al revés.
- **`abm-terreno`** — El sustrato físico del planeta: cubo esférico con LOD dinámico, clima, agua, vegetación, erosión y suelo. Paquete propio (no dentro de `abm-ruleset`) porque es grande, tiene su propio ejecutable y no depende del motor de agentes. Depende solo de `abm-core`. *(Añadido el 2026-11-05 con confirmación del responsable.)*
- **`abm-persistence`** — La salida. Serialización a JSON y sink a PostgreSQL. Aislada tras un trait para poder cambiar el destino sin tocar el motor.
- **`abm-metrics`** — Observadores y agregación. Aplica el LOD de almacenamiento: decide qué se registra a nivel de agente y qué se colapsa en métricas agregadas.
- **`abm-runner`** — Los binarios ejecutables (`abm`, el motor de agentes, y `terreno`, el planeta). Lee configuración, monta core + ruleset + metrics + persistence, ejecuta la simulación y gestiona la CLI.

Regla de dependencias: las flechas apuntan hacia `abm-core`. Nada en `abm-core` puede depender de `abm-ruleset`, `abm-terreno`, `abm-persistence` ni `abm-metrics`. Si necesitas que el core "sepa" algo del dominio, es señal de que la abstracción está mal y hay que revisarla.

> **Nota de arranque incremental:** NO crees los cinco crates el primer día. Empieza con un único binario que tenga cuadrícula + agentes mínimos funcionando. Refactoriza hacia esta estructura solo cuando la base sea estable (ver metodología). El TREE.md marca qué existe hoy frente al objetivo. *(Hecho el 2026-11-05: el código vive en `crates/`, un paquete por responsabilidad; ver `docs/design/paquetes/`.)*

---

## 4. Metodología: Pirámide Invertida

Desarrollo estrictamente incremental. Se empieza por lo mínimo y solo se sube de capa cuando la base es **matemáticamente estable** (métricas coherentes, sin NaN/overflow, comportamiento reproducible).

Nunca implementes una capa N+1 si la capa N no produce datos estables y verificados. Ante la duda, pregunta.

---

## 5. Convenciones de código

- **Formato:** `cargo fmt` obligatorio antes de cada commit. Sin excepciones.
- **Linter:** `cargo clippy -- -D warnings` debe pasar limpio. Los warnings son errores.
- **Orientación a datos:** favorece Struct of Arrays sobre Array of Structs. Evita `Box<dyn Trait>` en los bucles calientes; el dispatch dinámico rompe la caché.
- **`unsafe`:** prohibido salvo justificación escrita en comentario y aprobación explícita. Casi nunca hará falta.
- **Gestión de errores:** `Result` + `thiserror` para errores de librería; `anyhow` solo en el binario `abm-runner`. Nada de `unwrap()`/`panic!` fuera de tests y del arranque.
- **Configuración externa:** todos los parámetros de simulación viven en archivos `config/*.toml`, nunca hardcodeados. El responsable dirige experimentos cambiando config, no código.
- **Sin asignaciones en el hot loop:** reserva buffers una vez y reutilízalos por tick. Perfila antes de optimizar, pero no introduzcas `Vec::new()` por tick alegremente.
- **Interruptores de cambios del modelo:** un cambio nuevo puede llevar un interruptor en la config mientras se desarrolla y se mide (cada cambio por separado y todos juntos). Cuando el responsable decide, Claude **retira sin preguntar** los interruptores que no sean imprescindibles, con el código viejo que encendían. La referencia del comportamiento anterior es el commit anterior (`herramientas/no_regresion.sh`), no una rama del código. Solo se conserva una opción si es una alternativa científica que se quiere comparar de forma permanente, documentada y con su test. *(Confirmado por el responsable el 2026-11-05.)*
- **Todo lo medido se puede consultar:** las métricas que sostienen una decisión van a la salida (JSON) o a un script reproducible de `herramientas/`, no a cálculos sueltos.

---

## 6. Determinismo y reproducibilidad (crítico)

Sin esto, los resultados no son científicos. Reglas:

- **RNG sembrado y explícito.** Un único generador raíz sembrado desde la config. Nada de `rand::thread_rng()` ni fuentes de entropía del sistema en la lógica de simulación.
- **Prohibido el reloj de pared en la lógica.** El tiempo de la simulación es el `tick`, un entero. `SystemTime`/`Instant` solo para medir rendimiento, jamás para decisiones del modelo.
- **Orden de iteración estable.** Si el paralelismo introduce no-determinismo en el orden de reducción, usa acumuladores asociativos o fija el orden. Documenta cualquier punto donde el paralelismo pueda alterar resultados.
- **Versionado.** Cada ejecución registra en la BD su `engine_version` y `ruleset_version`. `rust-toolchain.toml` fija la versión del compilador. Una ejecución = (semilla, versión, config) reproducible.

---

## 7. Persistencia (evolución por fases)

- **Fase inicial:** volcado a JSON en `output/`. Simple, inspeccionable, sin dependencia de BD para validar el motor.
- **Fase madura:** sink a PostgreSQL/TimescaleDB. Detrás de un trait (`Sink`) en `abm-persistence`, para poder tener ambos destinos y elegir por config.
- **Esquema de BD:** definido en `DATABASE_SCHEMA.md` y aplicado mediante migraciones versionadas en `infra/migrations/`. Nunca modifiques el esquema de la BD a mano; escribe una migración.

---

## 8. Protocolo de documentación viva (OBLIGATORIO)

Este repositorio mantiene cuatro documentos vivos. **Mantenerlos actualizados es parte de cada tarea, no un extra.** Tras cualquier cambio significativo, antes de cerrar la tarea:

- **`MEMORY.md`** — Añade una entrada al histórico (formato en el propio archivo) describiendo qué se hizo, por qué, y qué decisiones o problemas surgieron. Es append-only: no reescribas el pasado, añade al final. Es la memoria entre sesiones.
- **`TREE.md`** — Si añadiste, moviste o eliminaste archivos/carpetas relevantes, regenera el árbol y marca qué está implementado.
- **`DATABASE_SCHEMA.md`** — Si tocaste el esquema (nueva tabla, columna, índice, hypertable), actualízalo para que refleje el estado real, y crea la migración correspondiente.
- **`CLAUDE.md`** (este archivo) — Solo se actualiza cuando cambian principios, arquitectura o convenciones, y siempre con confirmación del responsable.

Regla de oro: si alguien lee estos cuatro archivos, debe entender el estado completo del proyecto sin mirar el código. Si al terminar una tarea eso ya no es cierto, la tarea no está terminada.

### 8.1. Documentación por fases en `docs/design/`

Cada **fase** completada (en el sentido de la pirámide invertida, §4) deja su rastro documentado en `docs/design/<nombre-de-fase>/`. Al cerrar una fase, **antes de darla por terminada**:

1. Crea la carpeta `docs/design/<nombre-de-fase>/` con el nombre de la fase (ej. `docs/design/inicial/`).
2. **Mueve** ahí el/los documento(s) de diseño técnico o matemático de esa fase (ej. `minimal-model.md`). Usa `git mv` para conservar el historial.
3. **Crea** dentro un `README.md` que explique, **en lenguaje humano y sencillo** (sin dar por conocido el vocabulario técnico: introduce cada término y tradúcelo al momento), qué se está logrando en la fase, para qué sirve y qué queda fuera, junto con algunas **consideraciones** (decisiones tomadas, limitaciones y hallazgos).

Así, en cada fase conviven el **diseño técnico** (el "cómo" formal, para implementar) y la **explicación llana** (el "qué" y el "por qué", para entender). Cualquiera debe poder seguir la evolución del proyecto leyendo en orden los `README.md` de las fases.

---

## 9. Git

- Ramas por funcionalidad, `main` siempre compilable y con tests en verde.
- Commits pequeños y descriptivos, en imperativo (ej. "Añade sistema de consumo de recursos").
- `.gitignore` debe excluir siempre: `/target`, `/output`, `infra/.env`, y cualquier secreto.
- Nunca subir credenciales. Las contraseñas viven en `infra/.env` (ignorado); el `docker-compose.yml` las referencia por variable.

---

## 10. Verificación (antes de dar por hecha una tarea)

Una tarea no está completa hasta que:

1. `cargo fmt` y `cargo clippy -- -D warnings` pasan limpios.
2. `cargo test` en verde (incluye tests de determinismo: misma semilla → misma salida).
3. Los documentos vivos (sección 8) están actualizados.
4. Si aplica, la migración de BD existe y el esquema documentado coincide con el real.
5. La **batería de escenarios de referencia** pasa (escenarios fijos con unas pocas métricas clave y sus márgenes, que vigilan lo que emerge al juntar sistemas; ver `herramientas/`). No se exige que la salida sea idéntica byte a byte salvo cuando un cambio no debe alterar nada. Si un cambio intencionado saca una métrica de su margen, se documenta en su nota y se actualiza el margen.

Ante cualquier ambigüedad sobre el modelo, las ecuaciones o la escala, **pregunta al responsable antes de codificar**. El diseño matemático manda sobre la implementación.

---

## 11. Tras finalizar una tarea

Una vez finalizada, se ha de actualizar el MEMORY.md siempre, haciendo un resumen de lo ocurrido en la sesión. Y si es necesario, también TREE.md , DATABASE_SCHEMA.md y cualquiera que haya parecido, siguiendo el formato que ya tienen.
Tras esto, debes sugerir un nombre de commit, y decir como se puede comprobar lo que se acaba de hacer.
