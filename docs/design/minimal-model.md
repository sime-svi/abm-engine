# Modelo mínimo — Forrajeo en cuadrícula (tipo *Sugarscape*)

> Diseño matemático del **primer binario mínimo** (CLAUDE.md §2.7: "abstracción matemática
> primero"). Este documento **precede y manda** sobre la implementación. El código de `src/` implementa
> exactamente lo aquí descrito. Cualquier cambio de modelo se decide aquí primero.
>
> Estado: **v1 — hito inicial**. Alcance: métricas agregadas por tick → JSON. Sin BD, sin snapshots.

---

## 1. Espacio

- Cuadrícula 2D de `W × H` celdas. Índice lineal de una celda `(x, y)`:  **`i = y·W + x`**, con `0 ≤ x < W`, `0 ≤ y < H`.
- **Topología toroidal** (los bordes se envuelven): la celda a la derecha de `x = W−1` es `x = 0`, etc. Elimina los efectos de borde (un agente en una esquina no queda artificialmente atrapado). *Asunción por defecto, revisable en config (`grid.toroidal`).*
- **Vecindad de Moore (8 celdas)**: las 8 adyacentes (incluye diagonales). Con toroide, los desplazamientos vecinos se calculan con aritmética modular:
  `x' = (x + dx + W) mod W`, `y' = (y + dy + H) mod H`, para `(dx, dy) ∈ {−1,0,1}² \ {(0,0)}`.
  *Revisable en config (`grid.neighborhood = 8 | 4`; 4 = Von Neumann, sin diagonales).*
- **Ocupación única** (`single_occupancy = true`): como máximo **un agente por celda**. Define qué celdas están "libres" para moverse o para nacer. *Revisable en config.*

## 2. Estado

**Celda `i`:** un recurso `R_i ∈ [0, R_max]` (número real, `f32`).

**Agente `k`:** posición `(x_k, y_k)`, energía `E_k ≥ 0` (`f32`), edad `a_k ∈ ℕ` (ticks vividos), e
identificador único monotónico `AgentId_k` (entero que nunca se reutiliza; fija el **orden canónico**
de actualización).

## 3. Tiempo y dinámica de un tick

El tiempo es el **tick** `t ∈ ℕ` (entero; jamás el reloj de pared — CLAUDE §6). Cada tick ejecuta,
**en este orden fijo** (el orden *es* el modelo):

### (0) Regeneración del recurso
Para toda celda `i`:  **`R_i ← min(R_i + g, R_max)`**  (`g` = `resource.regrowth`).

### (1) Orden canónico
Se recolectan los agentes vivos y se **ordenan por `AgentId` ascendente**. Todos los pasos siguientes
recorren los agentes en ese orden (**actualización asíncrona**: el que va antes actúa —y coge
recurso— primero). Garantiza reproducibilidad con independencia del layout interno del motor ECS.

### (2) Movimiento + consumo + metabolismo (por agente, en orden canónico)
Para el agente `k` en su celda `c`:

1. **Mover.** Candidatas = `{c} ∪ {vecinas de c que estén libres}`. Se elige la de **mayor `R`**.
   Empate → **orden de desplazamiento fijo** (la celda actual `c` va primero, luego los offsets en
   orden fijo) ⇒ ante empate el agente **se queda**, y no hay sorteo en el bucle caliente. Se
   actualiza la ocupación (libera `c`, ocupa la elegida `c'`). *Desempate revisable (`movement.tiebreak`).*
2. **Consumir.**  `consumo = min(R_{c'}, apetito)`;  `E_k ← E_k + consumo`;  `R_{c'} ← R_{c'} − consumo`  (`apetito` = `agents.appetite`).
3. **Metabolizar y envejecer.**  `E_k ← E_k − m`;  `a_k ← a_k + 1`  (`m` = `agents.metabolism`).

### (3) Reproducción (por agente, en orden canónico)
Si `E_k ≥ θ` (`θ` = `reproduction.energy_threshold`) **y** existe una celda vecina **libre y no
reservada** este tick:

- energía del hijo:  `E_hijo = φ · E_k`   (`φ` = `reproduction.child_energy_fraction`, `0 ≤ φ ≤ 1`);
- la madre conserva:  `E_k ← (1 − φ) · E_k`;
- se **reserva** la celda hija (para que dos madres no elijan la misma) y se registra la intención de
  nacimiento (el spawn real ocurre en el paso 5). El hijo nace con `a = 0`.

El reparto es determinista (sin RNG). Un recién nacido **no actúa** hasta el tick siguiente (no está en
el orden canónico de este tick).

### (4) Mortalidad (por agente, en orden canónico)
El agente `k` muere si  **`E_k ≤ 0`  o  `a_k > a_max`**  (`a_max` = `agents.max_age`). Se registra la
intención de muerte (el despawn real ocurre en el paso 5).

### (5) Aplicación (spawn/despawn, orden determinista)
En un único paso con acceso exclusivo al mundo:
1. **Muertes primero**, en orden canónico: liberan su celda y se eliminan.
2. **Nacimientos después**, en orden de `AgentId` de la madre: se crea el hijo, se le asigna el
   siguiente `AgentId` (contador monotónico), se ocupa su celda reservada.

Aplicar muertes antes que nacimientos hace la asignación de identidades reproducible.

### (6) Métricas
Se agregan las métricas del tick (§5) y se registran en el sink (JSON) si `t mod metrics_every = 0`.

## 4. Determinismo (contrato)

Misma `(semilla, versión de motor, config)` ⇒ **misma salida byte a byte**. Se sostiene sobre:
orden canónico por `AgentId`; sumas en `f64` en orden fijo (índice de celda / `AgentId`); RNG
`ChaCha8Rng` sembrado, **usado solo en la génesis del mundo** (posiciones y energías iniciales, en
orden de `AgentId`), **cero sorteos en el bucle** con el desempate por orden fijo; nada de `HashMap`,
`SystemTime`/`Instant` ni `thread_rng` en la lógica.

## 5. Métricas agregadas (por tick)

| Métrica | Definición |
|---|---|
| `population` | nº de agentes vivos tras el paso (5) |
| `births` | nacimientos aplicados en el tick |
| `deaths` | muertes aplicadas en el tick |
| `energy_total` | `Σ_k E_k` (en `f64`, orden de `AgentId`) |
| `energy_mean` | `energy_total / population` (0 si población = 0) |
| `age_mean` | `(Σ_k a_k) / population` (0 si población = 0) |
| `resource_total` | `Σ_i R_i` (en `f64`, orden de índice de celda) |

## 6. Invariantes (deben cumplirse siempre)

- `0 ≤ R_i ≤ R_max` para toda celda.
- A lo sumo un agente por celda (si `single_occupancy`).
- `E_k` y todas las métricas son **finitas** (nunca `NaN`/`inf`).
- `population ≥ 0`; sin `AgentId` repetidos.

## 7. Parámetros por defecto (corrida pequeña e inspeccionable)

Definidos en `config/default.toml`. Valores base: `W = H = 100`, `total_ticks = 200`,
`initial_count = 300`, `R_max = 8`, `g = 1`, `apetito = 4`, `m = 1`, `a_max = 200`, `θ = 20`,
`φ = 0.5`, `seed = 42`. Elegidos para que la población **ni explote ni colapse** de inmediato y el
comportamiento sea verificable a ojo. Se calibran por config, nunca por código (CLAUDE §5).

## 8. Fuera de alcance (fases posteriores)

Snapshots de agente (micro), eventos discretos, sink a PostgreSQL/TimescaleDB, capacidad por celda
("montañas de azúcar"), genética/linaje más allá de `parent_id`, y paralelismo. El diseño de datos ya
deja la costura preparada (trait `MetricsSink`, `AgentId`, grid como `Vec` plano).
