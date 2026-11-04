# Diseño técnico — Terreno · Incremento 1

**Sistema:** Sustrato espacial físico (subconjunto ejecutable de la visión completa)
**Versión:** 1.0
**Documento madre:** [`diseno_terreno_espacio_geografia.md`](./diseno_terreno_espacio_geografia.md) (la visión completa; este documento **recorta** un primer incremento estable de ella).
**Destinatario:** Claude Code (Opus 4.8) para implementación.
**Naturaleza:** Plano teórico del incremento. Fija la matemática, las estructuras conceptuales y las reglas de determinismo. No contiene código; las decisiones de implementación quedan a criterio de quien programe, respetando lo aquí escrito.

> **Notación** (heredada del documento madre): `·` multiplicación, `√` raíz, `^` exponente, `_` subíndice. Un *tick* `t ∈ ℕ` es el paso de tiempo entero del motor. "Extensivo" = se suma al juntar celdas (agua, nieve, biomasa); "intensivo" = se promedia (temperatura, humedad relativa).

---

## 0. Decisiones que fija este incremento

Este incremento materializa cuatro decisiones tomadas con el responsable (13-07-2026):

| Eje | Decisión | Consecuencia para este documento |
|---|---|---|
| **Geometría** | Cubo esférico **completo** (6 caras, adyacencia real, lat/lon reales) | Se especifica la proyección, el direccionamiento y la vecindad entre caras (§2). |
| **Resolución** | Árbol **LOD-ready** corriendo a **nivel uniforme fijo**; la subdivisión/colapso dinámica **no dispara todavía** | Las estructuras se diseñan para LOD (§3), pero el sistema 6.1 del documento madre queda **fuera** (§10). |
| **Procesos** | Solo los **rápidos** (clima, nieve, balance hídrico, agua superficial, vegetación). **Relieve estático**. | Entran §6.0–§6.6 del madre; **erosión (6.7) y pedogénesis (6.8) quedan fuera** (§10). |
| **Tick** | **Fino, ~365 ticks/año** (≈1 tick por día) | Todas las tasas por tick se reescalan (§7). |

**Objetivo del incremento:** que de la sola combinación de `(semilla, config)` emerja, sobre una esfera real y de forma reproducible bit a bit, la cadena causal **elevación → clima → agua → vegetación**, con biomas, sombra pluviométrica, ríos y lagos observables, y con el **balance de agua global cerrando** dentro de tolerancia numérica. Nada de relieve vivo, nada de resolución adaptativa: eso son los incrementos 2 y 3.

---

## 1. Alcance: qué entra y qué NO

### Entra (subconjunto de la visión)
- **Geometría del cubo esférico** completa: 6 caras, proyección equiangular, lat/lon reales, área exacta por celda, vecindad intra e inter-cara, esquinas del cubo resueltas (cierra P8 para este incremento).
- **Estructura de datos LOD-ready**: árbol por cara y direccionamiento `(cara, L, i, j)`, pero **poblado a un único nivel uniforme** `L_unif` (config, defecto 0).
- **World-gen estático (§6.0 del madre, sin el bucle de erosión)**: elevación por ruido 3D coherente, clasificación de superficie, pendiente/orientación sobre la esfera, textura de suelo y sus derivados (`W_cap`, `k_perm`, `F0`), rellenado de depresiones + cuencas de lago, red de ríos por acumulación de flujo, recurso genérico mínimo.
- **Sistemas por tick**: 6.2 clima (T, P con efecto orográfico y estaciones), 6.3 nieve, 6.4 balance hídrico (modelo de cubo + difusión lateral), 6.5 agua superficial (mar/ríos/lagos), 6.6 vegetación (idoneidad + logístico + bioma de Whittaker).
- **Interfaz con agentes (§6.9)**: cálculo bajo demanda de `H`, `c_mov`, `A_disp`, `K_alim`. La **cola de escrituras** de agentes se deja como *hook* vacío (no hay agentes todavía).
- **Métricas y eventos (§7–§8 del madre)** que alimentan la tubería `metrics`/`persistence` ya existente.
- **Tests de determinismo, continuidad esférica y conservación**.

### NO entra (diferido, ver §10)
- Subdivisión/colapso dinámico (LOD adaptativo, 6.1 del madre) — **incremento 2**.
- Erosión/sedimentación (6.7) y pedogénesis (6.8): el relieve `h`, la pendiente `S` y `z_suelo`/`F` **no cambian** tras el world-gen — **incremento 3**.
- Todo lo ya marcado "ampliación futura" en el madre: corrientes oceánicas, inercia térmica marina, monzones/estocásticos, tectónica, insolación por orientación, química de suelos, deriva climática.

---

## 2. Geometría del cubo esférico

El mundo es una esfera de radio `R_planeta`. Se representa como un **cubo de 6 caras cuadradas proyectado sobre la esfera** (cubed sphere). Se elige la variante **equiangular** (gnomónica de ángulos iguales), no la gnomónica ingenua, porque reparte el área mucho más uniformemente (razón área máx/mín ≈ 1.5 frente a ≈ 5.2), lo que reduce el sesgo en toda agregación ponderada por área.

### 2.1 Caras y ejes locales
Seis caras identificadas por su normal exterior:

| Cara `c` | Normal `n̂` | Eje local `â` (i crece →) | Eje local `b̂` (j crece →) |
|---|---|---|---|
| 0 | +X | +Y | +Z |
| 1 | +Y | −X | +Z |
| 2 | −X | −Y | +Z |
| 3 | −Y | +X | +Z |
| 4 | +Z (polo N) | +Y | −X |
| 5 | −Z (polo S) | +Y | +X |

`(â, b̂, n̂)` es dextrógiro en cada cara. Las cuatro caras ecuatoriales comparten `b̂ = +Z`; las polares eligen tangentes que cierran la continuidad con las ecuatoriales (la elección exacta de signos se valida con el test de continuidad de §9).

### 2.2 Direccionamiento y centro 3D de una celda
Una celda es `(cara c, nivel L, i, j)` con `i,j ∈ [0, N)` y `N = N0 · 2^L` celdas por arista de cara. Su centro:
```
ξ_i = −π/4 + (i + 0.5) · (π/2)/N            (ángulo equiangular en â)
η_j = −π/4 + (j + 0.5) · (π/2)/N            (ángulo equiangular en b̂)
P⃗   = n̂ + tan(ξ_i)·â + tan(η_j)·b̂          (punto en el plano de la cara, normal a 1)
r̂    = P⃗ / |P⃗|                              (dirección unitaria sobre la esfera)
x⃗    = R_planeta · r̂                         (posición física del centro)
φ    = asin(r̂_z)                             (latitud)
λ    = atan2(r̂_y, r̂_x)                       (longitud)
```
Como `x⃗` es función pura de `(c, L, i, j)`, subdividir una celda **recalcula** las posiciones finas exactas: la geometría es coherente a cualquier nivel (propiedad que el incremento 2 explotará).

### 2.3 Área real de la celda
Los 4 vértices de la celda se obtienen con la misma fórmula usando los ángulos de esquina `ξ ∈ {−π/4 + i·(π/2)/N, −π/4 + (i+1)·(π/2)/N}` (análogo en `η`). Proyectados a la esfera, el área es el **ángulo sólido** del cuadrilátero esférico por exceso angular (l'Huilier sobre sus dos triángulos):
```
A_celda = R_planeta² · Ω(v0,v1,v2,v3)
```
Test de sanidad: `Σ_celdas A_celda ≈ 4π·R_planeta²` dentro de tolerancia (§9). Esta `A_celda` pondera **toda** agregación entre niveles y el cierre del balance de agua.

### 2.4 Vecindad (intra-cara, inter-cara y esquinas)
- **Intra-cara:** trivial, `i±1`, `j±1` (4 ortogonales + 4 diagonales), con el orden de offsets **fijo** (misma disciplina que la rejilla 2D actual, `grid.rs`).
- **Inter-cara (las 12 aristas del cubo):** en vez de tabular a mano las 12 transformaciones (fuente clásica de errores de signo/intercambio de ejes), se resuelve **por geometría, precalculado una sola vez** en world-gen:
  1. Para un vecino que se sale de la cara (p. ej. `i = −1`), se evalúa su dirección `r̂` con la misma parametrización extendida.
  2. Se determina **qué cara** lo contiene (la del eje con mayor `|componente|` de `r̂`).
  3. Se invierte la proyección en esa cara para obtener `(i', j')`.
  Resultado: una **tabla de vecinos estática** (`Vec` indexado por id lineal de celda → hasta 8 ids vecinos). En tiempo de ejecución la vecindad es un acceso O(1); las transformaciones fiddly se calculan una vez y de forma determinista.
- **Esquinas del cubo (8 puntos donde concurren 3 caras) — cierre de P8 para este incremento:** una celda de esquina tiene **7 vecinos** en lugar de 8 (le falta una diagonal). No se inventa una celda fantasma: el estárcil de difusión (§6.4, §2 del madre) **renormaliza sus pesos sobre los vecinos existentes**. Como el área de cada celda se calcula exactamente (§2.3), no hace falta ningún parche de área. Es una resolución determinista y sin artefactos, suficiente para este incremento; el tratamiento fino de esquinas queda para cuando (si) se necesite.

### 2.5 Distancias y difusión (correcciones de rejilla cuadrada, del madre)
- **Distancia real entre centros:** distancia sobre la esfera (cuerda o arco) entre los `x⃗`, no `d` nominal. Movimiento, rangos y gradientes la usan.
- **Difusión casi isótropa:** en cualquier difusión lateral, vecino ortogonal pesa `1`, diagonal `1/√2`, y se **normaliza** (incluidas las esquinas, §2.4).
- **Flujo D-infinito:** la bajada del agua se reparte entre las dos celdas que rodean la dirección real de máxima pendiente (elimina ríos artificialmente rectos).

---

## 3. Estructura de datos: LOD-ready pero uniforme

**Principio rector:** las estructuras se diseñan como si el LOD adaptativo existiera, pero se pueblan a un único nivel `L_unif`. Así, activar la subdivisión en el incremento 2 será encender un sistema, **no reescribir la física**.

- **Celda = entidad ECS** (coherente con CLAUDE §2 y con el motor de forrajeo actual). Sus componentes son los del §3 del madre (estáticos: `Posicion`, `Geometria`, `Elevacion`, `Pendiente`, `Orientacion`, `Suelo`, `Roca`; dinámicos: `AguaSuelo`, `Clima`, `Nieve`, `Vegetacion`, `Fertilidad`, `Recursos`, `TipoSuperficie`). `bevy_ecs` los almacena por columnas (SoA), cumpliendo la orientación a datos de CLAUDE §5.
- **Topología = recurso global** (`Resource` de bevy): los 6 árboles por cara + la **tabla de vecinos** precalculada + el mapa `id_lineal → (cara,L,i,j)` y su inverso. El árbol nace a profundidad uniforme `L_unif`; los nodos guardan nivel/madre/hijas (vacías en este incremento).
- **Id lineal de celda** = clave estable y canónica (como `AgentId` en el motor actual): fija el **orden de iteración determinista** de reducciones y difusiones. Ninguna reducción depende del orden interno del ECS.
- **Overlays** (del madre): `RedRios` (grafo dirigido de tramos con caudal `Q`) y `Cuenca` (por depresión: `V_lago`, `h_derrame`, curva hipsométrica simplificada). No son estado pesado por celda.
- **Sin asignaciones en el hot loop** (CLAUDE §5): buffers de trabajo (escorrentía `R`, drenaje `D`, `P_liq`, acumuladores de difusión) se reservan una vez y se reutilizan por tick, como en `buffers.rs` del motor actual.

> **Ganancia concreta del enfoque:** la física de §6 se escribe contra la API de topología (`vecinos(id)`, `area(id)`, `dist(id_a, id_b)`), que es **invariante a la escala**. Da igual que el árbol esté uniforme (hoy) o adaptativo (mañana): el código de clima/agua/vegetación no cambia.

---

## 4. Generación del mundo (t = 0) — §6.0 del madre, sin bucle de erosión

Todo depende solo de `semilla` (RNG ChaCha sembrado, como el motor actual). Misma semilla + misma config → mismo mundo.

**a) Elevación por ruido 3D coherente multi-octava.** Clave para la continuidad en la esfera: se evalúa **ruido 3D del centro `x⃗`** (no ruido 2D por cara), de modo que la elevación es **automáticamente continua al cruzar aristas y esquinas** (no hay costuras que parchear).
```
h(x⃗) = Σ_{k=0}^{oct-1} a_k · ruido3D( f_k · x⃗ , semilla )
a_k = persistencia^k ,  f_k = f_0 · lacunaridad^k
```
Se normaliza y reescala a `[h_min, h_max]`. Parámetros: `oct=6`, `persistencia=0.5`, `lacunaridad=2.0`, `f_0`, `h_min=−11000 m`, `h_max=9000 m`.

**b) Clasificación de superficie.** `h < nivel_mar → OCEANO`; si no, `TIERRA` (lagos y ríos se marcan en e/f).

**c) Pendiente y orientación sobre la esfera.** Con las alturas de los vecinos y las **distancias reales** entre centros, se estima el gradiente en el **plano tangente local** (base este-norte-arriba, ENU, derivada de φ,λ):
```
∇h en la base ENU local → S = arctan(|∇h|) ,  asp = atan2(∇h_norte, ∇h_este)
```
`asp` (orientación) se calcula y almacena, pero su uso (insolación) queda para futuro (P9 = futuro).

**d) Textura de suelo y derivados.** Fracciones (arena, limo, arcilla) por ruido sembrado sesgado por relieve (laderas empinadas → más arenoso; llanuras bajas → más limo/arcilla). De ahí:
```
W_cap = z_suelo · θ_fc(textura)   [mm]   ;   k_perm = permeabilidad(textura)   ;   F0 = fertilidad_base(textura)
```

**e) Rellenado de depresiones y cuencas de lago.** **Priority-flood** (Barnes 2014) sobre el grafo global de la esfera, con **desempate determinista por id de celda**. Cada depresión se inunda hasta su cota de derrame `h_derrame`; se registra una `Cuenca` con volumen inicial y curva hipsométrica simplificada (P5: **paramétrica** en este incremento). Las cuencas sin salida al mar son endorreicas.

**f) Red de ríos.** Dirección de bajada D-infinito por celda; **acumulación de flujo** (`A_drenaje`) por **orden topológico del DAG de drenaje** (Kahn, desempate por id → determinista). `A_drenaje > umbral_rio → RIO`, enlazado en `RedRios` desde nacimientos a desembocaduras (mar o lago).

**g) Recursos.** Un **único recurso genérico** colocado por proceso Poisson sembrado, sesgado por geología (P4 permanece abierta: multi-mineral es futuro).

**Nota (relieve estático):** en este incremento, tras el world-gen, `h`, `S`, `asp`, `z_suelo`, textura y `RedRios` **no vuelven a cambiar**. El bucle vivo erosión↔relieve es el incremento 3.

---

## 5. Sistemas por tick (§6.2–§6.6 del madre)

Se implementan **exactamente** las ecuaciones del documento madre §6.2–§6.6 y §6.9. No se reproducen aquí para no duplicar la fuente de verdad; este documento solo fija **lo específico del incremento**:

1. **Clima (6.2):** `T` por latitud/altura + oscilación estacional sinusoidal con hemisferios opuestos; `P` = base por franja de latitud · efecto orográfico `m_oro` · modulación estacional. El viento es **campo fijo por franja de latitud** (P2 = asumido fijo).
2. **Nieve (6.3):** partición lluvia/nieve por `T_cong`, fusión por grados-día. `P_liq` alimenta el balance hídrico.
3. **Balance hídrico (6.4):** modelo de cubo (escorrentía `R`, infiltración, ET proxy lineal, drenaje profundo `D`) + difusión lateral casi isótropa (con el estárcil renormalizado en esquinas, §2.4). Produce `R` y `D`.
4. **Agua superficial (6.5):** mar como sumidero y fuente infinita de humedad; ríos = suma de `R+D` aguas arriba propagada por `RedRios` (que aquí es **fija**, no se recomputa: P6 no aplica sin erosión); lagos = balance de volumen por cuenca con desbordamiento por `h_derrame`.
5. **Vegetación (6.6, cada `N_veg` ticks):** idoneidad (campanas de humedad/temperatura × fertilidad), capacidad de carga `K` y crecimiento logístico; **bioma etiquetado por Whittaker** a partir de `T̄` y `P̄` (emerge, no se pinta).
6. **Interfaz con agentes (6.9):** `H`, `c_mov`, `A_disp`, `K_alim` bajo demanda. La cola de escrituras es un **hook vacío** (sin productores todavía) pero deja el slot del bucle listo para cuando lleguen los agentes.

### Orden de ejecución dentro del tick (subconjunto del §9 del madre)
```
1. Clima (T, P)
2. Nieve (SWE, P_liq)
3. Balance hídrico (θ; produce R y D)
4. Agua superficial (ríos, lagos; consume R y D)
5. Vegetación (V, bioma)              — cada N_veg ticks
6. Interfaz con agentes (exporta H, c_mov, A_disp, K_alim)
7. Aplicación de escrituras encoladas — hook vacío en este incremento
```
Se omiten del orden del madre los pasos 1 (LOD), 7 (erosión) y 8 (pedogénesis): fuera de alcance.

---

## 6. Interfaz con agentes (estado en este incremento)

Se calcula la **cara de lectura** completa (§6.9 del madre): coste de movimiento, habitabilidad, agua disponible, capacidad de alimento. Es lo que un futuro sistema de agentes leerá. La **cara de escritura** (deforestar, regar, urbanizar) se define estructuralmente —hay una cola que se drena al final del tick— pero está **vacía**: sin agentes no hay quien encole. Esto mantiene la forma del bucle correcta y evita un refactor cuando se acoplen los agentes.

---

## 7. Calibración del tick y reescalado de parámetros

El tick pasa de "~1 semana" (`T_anio=48` del madre) a "**~1 día**" (`T_anio=365`). Regla única y uniforme (respeta la "convención de tasas" del §6 del madre):

> **Toda tasa expresada *por tick* se multiplica por `48/365 ≈ 0.1315`. Toda cadencia expresada *en ticks* se multiplica por `365/48 ≈ 7.6`. Los parámetros adimensionales o intensivos por unidad física (no por tick) NO cambian.**

### 7.1 Parámetros que SÍ se reescalan (valor madre → valor incremento 1)
| Símbolo | Significado | Madre (≈semana) | Incremento 1 (≈día) |
|---|---|---|---|
| `T_anio` | duración del año | 48 ticks | **365 ticks** |
| `P_ref` | precipitación de referencia | 20 mm/tick | **≈2.6 mm/tick** |
| `k_fus` | fusión de nieve por grado | 2 mm/(°C·tick) | **≈0.26 mm/(°C·tick)** |
| `k_et` | ET potencial por grado | 1.0 mm/(°C·tick) | **≈0.13 mm/(°C·tick)** |
| `r_veg` | crecimiento vegetal | 0.2 /tick | **≈0.026 /tick** |
| `k_lat_flow` | difusión lateral de humedad | 0.05 | **≈0.0066** |
| `N_veg` | cadencia de vegetación | 4 ticks | **≈30 ticks** |

### 7.2 Parámetros que NO cambian
Adimensionales, intensivos por grado de latitud, umbrales y geometría: `T_ecuador`, `k_lat` (°C/°), `Γ` (°C/km), `A_est_polo`, `T_cong`, `c_esc_base`, `k_pend`, `k_sat`, `θ_opt/σ_θ`, `T_opt/σ_T`, `K_max`, `nivel_mar`, `W_cap_*`, exponentes, etc.

### 7.3 Advertencias de calibración (no son valores sagrados)
- Los defaults del madre eran un **punto de partida grueso** con ritmos mezclados; el reescalado mecánico los deja **coherentes entre sí**, pero cada uno debe pasar un **sanity check contra tasas reales diarias** (p. ej. `k_fus` diario cae cerca de factores grados-día realistas, ~0.3–6 mm/°C·día; `k_et·T` a T=25 °C ≈ 3.3 mm/día, plausible). El **criterio de aceptación real** no son los números sino: (i) el balance de agua cierra (§9) y (ii) los biomas emergen en las franjas esperadas.
- **Anti-aliasing estacional:** `T_anio=365 ≥ 24` ✔ (holgado).
- **Estabilidad de difusión (tipo CFL, §10 del madre):** `coef·Δt/d² ≤ ½`. Con el tick fino el `Δt` relativo es menor → la difusión es **más** estable; el reescalado a la baja de `k_lat_flow` refuerza el margen.
- **Desfase con la escala de vida de agentes (~200 ticks < 1 año):** queda **anotado, no resuelto**. No afecta a esta fase (los agentes corren en su propia fase); habrá que reescalar la vida de los agentes al acoplarlos.

Todos estos valores viven en `config/*.toml` (nada a fuego, CLAUDE §5). Se propone un bloque `[terreno]` nuevo (o un `config/experiments/terreno.toml`) con toda esta parametrización.

---

## 8. Métricas y eventos

Alimentan la tubería `metrics`/`persistence` ya existente (JSON y/o Postgres por config).

**Métricas por tick (§8 del madre aplicables):** T media global y por franja; P total y `θ` medio; superficie por bioma; longitud total de ríos, nº de lagos, volumen de agua superficial; elevación media (constante en este incremento); cobertura de nieve/hielo; **balance de agua global** (métrica de validación clave). Se omiten las de erosión/LOD (fuera de alcance).

**Eventos discretos (§7 del madre aplicables):** nacimiento/desaparición de río (fija en este incremento salvo por sequía extrema que anule caudal), formación/desecación de lago, desbordamiento de lago, cambio de bioma, congelación/deshielo, sequía/inundación local. Se omiten subdivisión/colapso y erosión notable.

---

## 9. Determinismo, conservación y validación (plan de tests)

Extiende el enfoque de `tests/determinism.rs` actual (misma semilla → misma salida byte a byte, también entre procesos).

**Determinismo**
- World-gen reproducible: hash del volcado del mundo idéntico para `(semilla, config)` fijas, en dos procesos distintos.
- Métricas por tick idénticas byte a byte entre corridas.
- Priority-flood, acumulación de flujo y toda reducción/difusión con **desempate y orden por id de celda** (nunca orden interno del ECS, nunca reloj de pared).

**Continuidad esférica** (test nuevo, específico del cubo)
- Muestrear `h`, `T`, `θ` a lo largo de un camino que **cruce una arista** y una **esquina** de cara; asertar que no hay discontinuidad más allá del gradiente vecinal esperado (valida los signos de ejes locales de §2.1 y las esquinas de §2.4).

**Conservación**
- **Balance de agua global** (métrica clave §8): `entrada(lluvia) − salida(ET total) − flujo neto al océano ≈ 0` dentro de tolerancia de coma flotante, sostenido a lo largo de la corrida.
- **Área:** `Σ A_celda ≈ 4π·R_planeta²`; razón área máx/mín acotada (~1.5, propia de la proyección equiangular).

**Casos límite esperados (§10 del madre)**
- Mundo plano (`h` constante): sin ríos, franjas climáticas puras por latitud, sin sombra pluviométrica.
- Una montaña aislada: aparece sombra pluviométrica a sotavento; nieve en la cima si `T < T_cong`.
- Sin lluvia (`P_base=0`): `θ→0`, la vegetación muere, ríos y lagos se secan.
- Lluvia intensa constante: lagos se llenan y desbordan, red de ríos activa, `θ→1`.

**Criterios de "hecho"** (CLAUDE §10): `cargo fmt` + `cargo clippy -D warnings` limpios; `cargo test` en verde (incluidos los tests nuevos de continuidad y conservación); documentos vivos (MEMORY, TREE, y este diseño) actualizados; parámetros en `config`, no en código.

---

## 10. Fuera de alcance (mapa de los siguientes incrementos)

| Diferido | Qué es | Incremento previsto |
|---|---|---|
| **LOD dinámico (6.1)** | subdivisión/colapso según relevancia, con reparto conservativo e independencia del camino | **2** |
| **Erosión/sedimentación (6.7)** | potencia de arroyo, transporte, soil creep; **modifica `h`** y realimenta el flujo | **3** |
| **Pedogénesis (6.8)** | producción de suelo, evolución de `z_suelo` y fertilidad `F` | **3** |
| Recomputar `RedRios` (P6) | reconstruir la red tras cambiar el relieve | **3** (con erosión) |
| Multi-mineral (P4) | geología de varios recursos | futuro |
| Hipsometría real de lagos (P5) | curva profundidad-área-volumen del relieve real | futuro |
| Corrientes/clima marino (P7), inercia térmica (P3), insolación (P9), viento dinámico (P2) | ampliaciones del madre | futuro |

---

## 11. Preguntas abiertas — estado tras este incremento

| Pregunta del madre | Estado en el incremento 1 |
|---|---|
| **P1** calibración del tick | **Cerrada**: `T_anio=365`, reescalado uniforme (§7). |
| **P2** viento | Asumido **fijo por franja** (§5). Sin cambios. |
| **P3** inercia térmica del mar | Futuro. |
| **P4** recursos minerales | **Un recurso genérico** (§4g). Multi-mineral, futuro. |
| **P5** hipsometría de lagos | **Paramétrica** (§4e). Real, futuro. |
| **P6** recomputar red de ríos | **No aplica** (relieve estático); vuelve con la erosión (incr. 3). |
| **P7** corrientes oceánicas | Futuro. |
| **P8** esquinas del cubo | **Cerrada para este incremento**: estárcil renormalizado sobre 7 vecinos, área exacta (§2.4). |
| **P9** orientación de ladera en T | Futuro; `asp` se **calcula y guarda**, no se usa (§4c). |

---

## 12. Estado de implementación y desviaciones (lo que se construyó de verdad)

Implementado en `src/terreno/` (módulo `abm::terreno`), binario `terreno`, config `config/terreno.toml`,
tests `tests/terreno.rs`. **Verificado**: `fmt` + `clippy -D warnings` limpios; determinismo byte a byte
entre procesos; **balance de agua a 1e-13 relativo** (conservación exacta); continuidad esférica de la
elevación; caso límite sin lluvia (suelo se seca, vegetación muere). Emergen los 9 biomas, la sombra
pluviométrica, ríos (~1500 celdas) y lagos, con bandas térmicas graduadas por latitud.

Desviaciones conscientes respecto al plano teórico (todas documentadas, ninguna afecta a la
conservación ni al determinismo):

| Punto | Diseño teórico | Implementado en el incremento 1 | Motivo |
|---|---|---|---|
| **Celda** | "entidad ECS" (§3) | **columnas `Vec` SoA** en structs (como el recurso `Grid` del motor de forrajeo, no entidades) | 100k celdas numéricas: SoA es más rápido, más cacheable y de orden de iteración trivialmente estable. Coherente con el precedente del repo. |
| **Flujo del agua** | D-infinito (reparto entre 2 celdas) | **receptor único** (D8 sobre el grafo de vecinos), asignado por el priority-flood | Mucho más simple y 100% determinista; los ríos algo más rectos son irrelevantes sin erosión (fuera de alcance). D-infinito queda para el incremento 3. |
| **Lagos** | volumen dinámico + hipsometría + desbordamiento (6.5) | **lagos estáticos** (celdas de depresión rellenada, etiquetadas `LAGO`); sin volumen dinámico | Un modelo de volumen/desborde por cuenca chocaba con un balance de agua limpio en el primer pase. Diferido (sub-item de incr. 2). La física los trata como tierra saturada. |
| **Difusión lateral** | `W_i −= q·W_cap_i` (§6.4 madre, asume áreas iguales) | **transferencia de volumen** con razón de áreas `A_i/A_j` y limitador de flujo | En el cubo esférico las áreas difieren ligeramente; mover mm iguales entre áreas desiguales **no conservaba volumen** (residual ~1e-4). Con la corrección, cierra a 1e-13. |
| **Elevación** | normalizada lineal a `[h_min,h_max]` (§4a) | + **curva hipsométrica** `land_exp` sobre la tierra emergida | La normalización lineal dejaba la tierra a ~4500 m de media (mundo helado). Con `land_exp=3` la media baja a ~490 m (tipo Tierra). |
| **`p_ref`** | 2.6 mm/tick (reescalado mecánico de 20·48/365) | **5.0 mm/tick** (calibrado) | 2.6 daba ~950 mm/año en el ecuador, por debajo del umbral de selva tropical. 5.0 (~1825 mm/año) hace emerger la selva sin secar los desiertos subtropicales. |

Nada de esto está "a fuego": `land_exp`, `p_ref` y el resto viven en `config/terreno.toml`.
> **Revisado en el incremento 2 (2026-11-04).** Para que el LOD dinámico sea coherente, el incremento 2 cambió cuatro piezas de esta base uniforme: la **pendiente** se calcula por diferencias centrales de la propia función de altura (no por mínimos cuadrados sobre vecinos); la **distancia al mar** es un Dijkstra con distancias reales (no BFS × lado fijo); la **difusión lateral** es simétrica (media armónica de `W_cap·A`, no la celda de id menor); y el **recurso** usa un RNG por contador. La elevación sigue siendo bit a bit la misma y las métricas apenas cambian. Ver [`incremento-2-diseno.md`](./incremento-2-diseno.md) §3, §4, §7 y §13.
