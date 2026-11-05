# Diseño técnico — Terreno · Incremento 2: LOD dinámico adaptativo

**Sistema:** Resolución adaptativa (subdivisión/colapso) sobre el sustrato del incremento 1.
**Versión:** 1.1 (2026-11-04). Revisa la v1.0 (2026-07-15) tras contrastarla con el código real del incremento 1.
**Documentos previos:** [`diseno_terreno_espacio_geografia.md`](./diseno_terreno_espacio_geografia.md) §2, §6.1 (visión madre) · [`incremento-1-diseno.md`](./incremento-1-diseno.md) (base uniforme ya implementada).
**Destinatario:** Claude Code para implementación.
**Naturaleza:** Plano teórico del incremento. Fija la matemática y las reglas de determinismo; el código queda a criterio de quien programe.

> **Notación** heredada. "Hoja" = celda activa (nodo del quadtree sin subdividir). "Familia" = las 4 hermanas hijas de una misma madre. "Extensivo" se suma, "intensivo" se promedia (§ notación del madre). `a_i` = área de la hija `i`, `A` = área de la madre. Agua en mm de lámina; volúmenes en m³ (`mm/1000 · m²`).

---

## Cambios respecto a la v1.0 (por qué hay v1.1)

La v1.0 daba por hechas propiedades que el código del incremento 1 **no** cumple. Cada fila es un hueco detectado y la decisión que lo cierra (las decisiones `D…` se desarrollan en las secciones indicadas):

| # | Hueco en la v1.0 | Decisión v1.1 | § |
|---|---|---|---|
| 1 | La elevación se normaliza con el min/max del ruido de **toda la malla** (`worldgen.rs`): no es función pura de la posición; cambiar la malla cambia todas las alturas. | **D2a** — normalización **congelada** (min/max del nivel base, calculados una vez) + recorte a `[0,1]`. | 3.1 |
| 2 | Pendiente, textura, `W_cap`, `k_perm`, `F0` dependen de los **vecinos** (mínimos cuadrados); el recurso se sortea con un RNG **secuencial** por id. | **D2b** — pendiente por **diferencias centrales de la propia función de altura**; **D2c** — recurso con **RNG por contador** (hash de semilla + celda). | 3.2–3.4 |
| 3 | Al afinar la costa, hijas que eran tierra pasan a mar (y viceversa). Nada decía qué hacer con su agua, nieve y biomasa; el balance solo cuenta tierra → el agua se perdería sin que la métrica lo viera. | **D6** — reparto **solo entre hijas de tierra**; lo que no cabe o cae en mar va al **término nuevo `lod_al_mar`** del balance global. | 6 |
| 4 | "La red de ríos no se recomputa": falso, los receptores apuntan a celdas que dejan de existir. El criterio "toca río/costa" no tenía histéresis → parpadeo. | **D3** — los **derivados globales se recalculan** tras cada cambio de topología; **D7** — **marca hidrológica pegajosa** (lo que fue río o lago no se colapsa) + **anticipación** con **ensayo en seco** (no se colapsa si la madre volvería a subdividirse). | 4, 8 |
| 5 | Los vecinos eran una tabla fija de 8 por dirección de Moore; con niveles mezclados hay hasta 12. La difusión usaba `W_cap`·área de la celda de **id menor** → entre gruesa y fina el intercambio cambiaba ~4× según cuál tuviera el id menor. | **D4** — vecindad por **anillo al nivel L+1** con tipo de contacto lado/esquina, **simétrica por construcción** (lista CSR); **D5** — difusión **simétrica** (media armónica). | 5, 7 |
| 6 | Con 6 octavas fijas el relieve no tiene detalle bajo ~120 km: refinar más allá del nivel 2 (~50 km) no descubre relieve nuevo. | **D2a** — **una octava más por nivel** (`octaves_per_level`), con normalización fija para que añadir octavas solo añada detalle. | 3.1 |
| — | "Gradiente relativo" sin definir (y ∇T por latitud existe en todas partes). | **D7** — umbrales **físicos** separados: `dh_sub` (m), `dt_sub` (°C), `dtheta_sub` (–). | 8 |
| — | Distancia al mar (BFS × lado fijo) y longitud de ríos (nº celdas × lado fijo) asumen celdas iguales. | **D3** — distancia por **Dijkstra con distancias reales**; longitud de río = Σ lado real de cada celda-río. | 4, 11 |
| — | Se mezclaban *determinismo* e *independencia del camino*. | **D10** — se separan los dos conceptos y sus tests. | 0, 10 |
| — | No regresión "byte a byte frente al incremento 1". | **D11** — D2b/D3/D5 cambian la física uniforme (nueva línea base). La no regresión pasa a ser: **LOD que no dispara ≡ LOD apagado**, byte a byte. | 10 |

---

## 0. Objetivo

Que la resolución **siga a la relevancia**: las celdas se **subdividen** en 4 hijas un nivel más finas donde hace falta detalle (costas, ríos, lagos, relieve abrupto, contrastes de temperatura o humedad; en el futuro, agentes y eventos) y se **colapsan** de vuelta cuando la zona deja de importar. La física del incremento 1 corre sobre la malla no uniforme resultante con dos cambios acotados (vecindad y difusión, §7).

**Dos propiedades distintas** (la v1.0 las mezclaba):
- **Determinismo**: misma `(semilla, config)` → misma salida bit a bit. Se garantiza porque **todo** (criterios, orden, operadores, recálculos) es función determinista del estado, iterado en orden canónico. No depende de la propiedad siguiente.
- **Independencia del camino** (consistencia física): subdividir y colapsar de inmediato devuelve el estado original; y el valor de un campo estático en una celda no depende de la historia de la malla. Es lo que evita que el LOD fabrique artefactos. **No** significa que dos historias de refinamiento distintas den el mismo futuro: mientras una zona está fina, su dinámica es más detallada y diverge legítimamente.

Es la pieza que hace tratable un planeta entero: a nivel uniforme fino no cabría; con LOD solo se paga detalle donde aporta.

---

## 1. Alcance

### Entra
- **Bosque de quadtrees** (uno por cara), con hojas a niveles mezclados entre `l_min` y `l_max`.
- **Estáticos como función pura** de la dirección de la celda (§3) y **derivados globales recalculables** (§4).
- **Vecindad multinivel** simétrica con **árbol balanceado 2:1** (§5).
- **Operadores de subdivisión y colapso** conservativos, con tierra↔mar contabilizado (§6).
- **Difusión lateral simétrica** (§7).
- **Driver de refinamiento** con histéresis y anticipación, y **refinamiento inicial** en la génesis (§8, §9).
- **Métricas** del LOD y **tests** (§10, §11).

### NO entra (diferido)
- Disparadores de **agentes** y **eventos** (no existen aún): el único punto de entrada es el cálculo de marcas del driver (§8); hoy sin productores.
- **Erosión / pedogénesis** (incremento 3): el relieve sigue estático.
- Reparto por afinidad distinta del área/capacidad.
- Sub-pasos temporales por nivel (la difusión mantiene la semántica de relajación por celda del incremento 1, §7).

---

## 2. Árbol y almacenamiento

### Direccionamiento jerárquico
- Una celda (a cualquier nivel) es `A = (c, L, i, j)`, con `N_L = n0·2^L` celdas por arista de cara.
- **Padre** de `(c,L,i,j)` = `(c, L−1, ⌊i/2⌋, ⌊j/2⌋)`. **Hijas** = `(c, L+1, 2i+di, 2j+dj)`, `di,dj ∈ {0,1}`.
- La geometría (`center_dir`, `cell_area`, `lat_lon`) es función pura de `(c, i, j, N_L)`. Las 4 hijas teselan exactamente a la madre (las aristas de celda son círculos máximos y la rejilla equiangular anida).
- **Clave** `id: u64` = `c` (3 bits, 61–63) · `L` (5 bits, 56–60) · `j` (28 bits) · `i` (28 bits). Exige `n0·2^(l_max+1) ≤ 2^28` (el `+1` es por el anillo de §5).

### Layout SoA ordenado por clave
- Las hojas activas viven en columnas `Vec` (SoA, como el incremento 1), alineadas con `keys: Vec<u64>` **ordenada**. El orden `(c, L, j, i)` es el canónico: fija todas las reducciones. A nivel uniforme coincide con el orden `c·n² + j·n + i` del incremento 1.
- **Localizar una hoja** = búsqueda binaria en `keys` (`O(log N)`). **No hay `HashMap`** (ni siquiera de consulta): menos piezas con desorden interno.
- La topología solo cambia en los **pasos de topología** (cada `n_lod` ticks). En ese paso se reconstruyen `keys`, las columnas y la vecindad. Entre pasos, las columnas son fijas: sin asignaciones por tick (CLAUDE §5).

> Regla de oro de determinismo: **iterar siempre por posición en `keys` (orden de clave)**.

---

## 3. Estáticos: función pura de `(c, L, i, j; semilla, config)`

Un estático se **recalcula** en la dirección de la celda cada vez que la celda aparece (subdivisión o colapso), nunca se interpola. Para que eso sea coherente, cada estático debe depender **solo** de la dirección y el nivel de la celda, no de la malla vecina.

### 3.1 Elevación (D2a)
```
oct(L)   = noise_octaves + octaves_per_level·(L − l_min)
raw(x,L) = Σ_{k<oct(L)} p^k · ruido(f0·λ^k · x)  /  norm0,     norm0 = Σ_{k<noise_octaves} p^k
frac     = clamp( (raw − raw_min) / raw_span , 0, 1 )
h        = hipsometría(frac)            (misma curva del incremento 1: océano lineal, tierra frac^land_exp)
```
- `raw_min`, `raw_span` se calculan **una sola vez** en la génesis (min/max de `raw` en los centros de la malla uniforme `l_min`) y quedan **congelados**: son constantes del mundo, función de `(semilla, config)`.
- **Normalización fija `norm0`**: añadir octavas no reescala las anteriores, solo suma detalle. Con `λ = 2` y una octava por nivel, la razón *tamaño de celda / detalle más fino* es la misma en todos los niveles.
- En `L = l_min` la elevación es **idéntica bit a bit** a la del incremento 1 (mismas operaciones).

### 3.2 Pendiente y orientación (D2b)
Diferencias centrales de la función `h(·, L)` a un paso de celda `δ_L = (π/2)/N_L` (radianes) sobre la esfera, en la base local este/norte `(ê, n̂)` de la celda:
```
p_e± = cos δ_L · x ± sin δ_L · ê ,   p_n± = cos δ_L · x ± sin δ_L · n̂
∂h/∂e = (h(p_e+) − h(p_e−)) / (2 δ_L R) ,   ∂h/∂n = (h(p_n+) − h(p_n−)) / (2 δ_L R)
S = atan |∇h| ,   asp = atan2(∂h/∂n, ∂h/∂e)
```
Sustituye al ajuste por mínimos cuadrados sobre vecinos del incremento 1 (que dependía de la malla).

### 3.3 Suelo
Textura, `W_cap`, `k_perm`, `F0`: mismas fórmulas del incremento 1, evaluadas con el ruido de textura en `x` y con la pendiente de §3.2 → funciones puras.

### 3.4 Recurso (D2c)
Densidad de recurso definida sobre el **ancestro de nivel `l_min`** de la celda, con un **RNG por contador** (hash SplitMix64 de `(semilla, clave del ancestro, flujo)`), y nula en el mar:
```
u1, u2 = hash01(semilla ⊕ 0xACE1, clave_ancestro, 0|1)
recurso = (no_mar(celda) ∧ u1 < resource_density) ? 100·u2 : 0
```
Es intensivo (copiar al subdividir y promediar por área al colapsar da lo mismo que recalcular). Se sigue derivando de la semilla (CLAUDE §6): solo cambia que el valor no depende del **orden** del sorteo.

---

## 4. Derivados globales (D3): se recalculan tras cada cambio de topología

Dependen de la malla entera, así que se recalculan sobre las hojas nuevas al final de cada paso de topología (y en la génesis). Coste `O(N log N)`, cada `n_lod` ticks.

1. Clasificación base: `OCEANO ⟺ h < nivel_mar`.
2. **Priority-flood** desde el océano sobre el grafo de hojas (`filled_h`, `receptor`, orden de extracción) con desempate por posición en `keys`.
3. **Área de drenaje** (m², invariante a la escala) acumulada aguas arriba→abajo.
4. **Distancia al mar**: Dijkstra multi-fuente desde las hojas de océano, arista = distancia de círculo máximo entre centros (km), desempate por posición.
5. Etiquetas **RIO** (área de drenaje > umbral) y **LAGO** (`filled_h > h + 1 m`).

Solo el punto 1 decide tierra/mar, y depende solo de `h` (estático) → **una hoja nunca cambia de tierra a mar salvo al cambiar la malla** (§6). Río/lago/tierra son todos "no mar" para la física: cambiar entre ellos no mueve agua.

---

## 5. Vecindad entre niveles y balance 2:1 (D4)

### Vecino al mismo nivel cruzando arista
Dentro de la cara es trivial. Al cruzar **una** arista, el vecino es la celda de la cara contigua que **comparte el tramo de arista** correspondiente: se localiza el punto situado a `ε = 10⁻⁶` celdas **más allá** de la arista, en el **centro del tramo** (no el centro de la celda fantasma, que cerca de las esquinas de cara cae en la fila equivocada por la curvatura de la rejilla). Las parametrizaciones equiangulares de dos caras coinciden **sobre** la arista común, así que el tramo es el mismo. Cruzar **dos** aristas a la vez = esquina del cubo → sin vecino (7 vecinos).

### Anillo al nivel L+1
Para la hoja `A = (c, L, i, j)`:
1. Se toman sus 4 **hijas virtuales** al nivel `L+1` y, para cada una, sus 8 vecinos de Moore al nivel `L+1` (regla anterior). Se descartan las que son hijas de `A`. Queda el **anillo**: ≤ 12 direcciones, cada una marcada **lado** (alcanzada por un movimiento ortogonal desde alguna hija → comparte arista con `A`) o **esquina** (solo por diagonal → contacto puntual).
2. Cada dirección `R` del anillo se resuelve a hoja: `R` si es hoja (vecino **más fino**), si no su padre (vecino **del mismo nivel**), si no su abuelo (vecino **más grueso**). Con balance 2:1 no hay más casos; si no se resuelve, el árbol está desequilibrado (error de invariante).
3. Tipo del par `(A, B)`: **lado** si alguna dirección del anillo resuelta a `B` es lado; si no, **esquina**.

A nivel uniforme el anillo reproduce los 8 vecinos de Moore (7 en las 24 celdas-esquina del cubo) con su tipo ortogonal/diagonal.

### Simetría por construcción
Se recogen los pares `(min, max, tipo)` desde **todas** las hojas, se ordenan y se fusionan (tipo = lado si cualquiera de los dos lados lo ve así). De los pares se construye la **lista CSR** (offsets + vecinos ordenados) → si `A` ve a `B`, `B` ve a `A`. Peso geométrico del par: `g = 1` (lado), `g = 1/√2` (esquina), como el incremento 1.

### Balance 2:1
Hojas vecinas difieren como mucho un nivel. Tras marcar subdivisiones (§8) se propaga en **cascada**: si un par acabaría con diferencia ≥ 2, se subdivide también el más grueso (basta un nivel por hoja y paso: el árbol de partida está balanceado). Una familia **no** se colapsa si algún vecino externo acabaría más fino que el nivel de las hijas (`nivel_final(vecino) > L_hijas`). Las colapsas de familias distintas no se estorban entre sí (dos hojas a distancia ≥ 2 niveles no pueden ser vecinas en el árbol de partida).

---

## 6. Subdivisión y colapso del estado dinámico (D6)

Estáticos y geometría: se **recalculan** (§3). Intensivos climáticos (`T`, `P`, acumuladores de medias para biomas): se **copian** a las hijas y se **promedian por área** al colapsar. Bioma: se recalcula (océano, o Whittaker de los acumuladores). Los extensivos de agua y biomasa siguen estas reglas, donde `L_t` = hijas de tierra (`h ≥ nivel_mar`) y `a_t = Σ_{L_t} a_i`.

**Invariante:** las hojas de océano tienen `W = SWE = V = 0` (el océano es un depósito infinito fuera del balance).

### 6.1 Subdivisión (madre → 4 hijas)
- **Madre de océano**: hijas de océano a cero; hijas de **tierra nueva**: `W = 0`, `SWE = 0`, `V = v_init` (biomasa semilla, fuera de todo balance).
- **Madre de tierra sin hijas de tierra**: todo su almacenamiento va al mar: `lod_al_mar += (W + SWE)·A/1000`; la biomasa se pierde.
- **Madre de tierra con hijas de tierra**:
```
SWE_i = SWE·A/a_t ,   V_i = V·A/a_t                         (i ∈ L_t; conserva Σ X·a)
V_w = W·A ,  C = Σ_{L_t} W_cap_i·a_i                         (en mm·m²)
si V_w ≤ C:  W_i = W_cap_i · V_w/C                           (misma saturación en todas)
si V_w > C:  W_i = W_cap_i ,  lod_al_mar += (V_w − C)/1000   (exceso de capacidad)
```

### 6.2 Colapso (4 hijas → madre)
- **Madre de océano**: `lod_al_mar += Σ_{L_t} (W_i + SWE_i)·a_i/1000`; biomasa perdida; madre a cero.
- **Madre de tierra sin hijas de tierra**: tierra nueva (`W = SWE = 0`, `V = v_init`).
- **Madre de tierra con hijas de tierra**:
```
W = Σ_{L_t} W_i·a_i / A ,  SWE = Σ_{L_t} SWE_i·a_i / A ,  V = Σ_{L_t} V_i·a_i / A
si W > W_cap:  lod_al_mar += (W − W_cap)·A/1000 ,  W = W_cap
```

### 6.3 Independencia del camino (ida y vuelta)
Madre de tierra con al menos una hija de tierra y sin exceso de capacidad; subdividir y colapsar sin ticks intermedios:
- Estáticos: recalculados de la misma función → idénticos.
- `SWE`, `V`: `Σ_{L_t} (X·A/a_t)·a_i / A = X`. Identidad.
- `W`: `Σ_{L_t} W_cap_i·(V_w/C)·a_i / A = V_w/A = W`; y `W ≤ W_cap` de la madre (lo era antes) → sin exceso al colapsar. Identidad.

Las excepciones (madre sin hijas de tierra, exceso de capacidad) **no** son identidad, pero quedan **contabilizadas** en `lod_al_mar`.

### 6.4 Balance de agua con LOD
El paso de topología corre al **inicio** del tick. El almacenamiento previo se mide **antes** del paso de topología, y el residual incluye el término nuevo:
```
residual = P − ET − salida_al_mar − lod_al_mar − Δalmacenamiento    (debe ≈ 0)
```
Así la métrica de validación clave del incremento 1 también verifica la conservación del LOD.

---

## 7. La física sobre la malla no uniforme (D5)

| Sistema | Cambio |
|---|---|
| Clima, nieve, balance hídrico, vegetación | **Ninguno**: locales por celda (lat, h, θ, S de la propia hoja). |
| Difusión lateral | **Simétrica** sobre los pares de §5 (abajo). |
| Agua superficial (ríos) | Ninguno: receptor y orden de flujo vienen de §4 (posiciones en `keys`). |
| Gradiente/pendiente | Ya no usa vecinos (§3.2). |
| Interfaz de agentes | `agua_disponible` recorre la lista CSR. |

**Difusión simétrica.** Para cada par `(i, j)` (orden de pares), con `X = W_cap·A`:
```
H_ij = 2·X_i·X_j / (X_i + X_j)                     (media armónica, mm·m²)
q    = k_lat_flow · g_ij · (θ_i − θ_j) · H_ij       (mm·m², positivo = de i a j)
q    ← limitado: no vaciar al donante ni rebasar la capacidad del receptor (holguras ≥ 0)
W_i −= q / A_i ,   W_j += q / A_j
```
- **Simétrica**: intercambiar `i` y `j` da el mismo flujo con signo opuesto (la v1.0 dependía de quién tenía el id menor).
- **Conservativa** exacta: se mueve volumen.
- Entre celdas iguales reduce a la fórmula del incremento 1 salvo que usa la media armónica de `W_cap·A` de las dos (antes, la de la celda de id menor).
- Mantiene la semántica de **relajación por celda** del incremento 1 (fracción de capacidad intercambiada por tick independiente del tamaño): estable a cualquier nivel sin sub-pasos.

---

## 8. Driver de refinamiento (D7)

Se evalúa al inicio de cada paso de topología sobre el estado actual. Para cada hoja `X` (variaciones solo entre vecinos **de tierra**, y solo si `X` es de tierra; en el océano no hay física):
```
Δh(X) = max_{Y∈vec(X), tierra} |h_Y − h_X| ,   ΔT(X), Δθ(X) análogos
rasgo(X) = refine_features ∧ ( X ∈ {RIO, LAGO} ∨ X costa )       (costa: tierra con vecino mar o mar con vecino tierra)
```
**Subdividir** `X` (si `L < l_max`):
```
rasgo(X) ∨ Δh > dh_sub ∨ ΔT > dt_sub ∨ Δθ > dtheta_sub   [∨ demanda de agentes/eventos, futuro]
```
**Colapsar** la familia `F` (las 4 hermanas hojas, `L > l_min`) si:
1. **Calma con histéresis** en las 4: ninguna es rasgo ni lleva la **marca hidrológica** (abajo), y `Δh ≤ h_hist·dh_sub`, `ΔT ≤ h_hist·dt_sub`, `Δθ ≤ h_hist·dtheta_sub`.
2. **Anticipación**, en dos niveles. *Barata*: la madre `M` que resultaría **no** cumpliría el criterio de subdividir frente a los vecinos externos de la familia (`h_M` de §3.1; `T_M` media por área; `θ_M` del agua agregada; costa si `M` y algún vecino externo difieren en tierra/mar). *Exacta* (**ensayo en seco**): el paso completo se aplica en una **copia** del mundo, se recalculan los derivados globales (§4) y se **vetan** las familias cuya madre pediría subdividirse en esa copia; si no hay vetos, la copia es el resultado. La barata no ve los ríos (dependen de toda la red: al juntar 4 hijas, la madre recoge el drenaje de las 4 y puede pasar a ser río); la exacta sí.
3. Balance 2:1 (§5).

**Marca hidrológica pegajosa.** Toda hoja que es RIO o LAGO queda **marcada para siempre**; las hijas heredan la marca y una familia con alguna hija marcada no se colapsa. Motivo: ríos y lagos dependen de la red de drenaje de **toda** la malla (receptor único), y la malla depende de ellos. Sin memoria, ese acoplamiento no converge: un colapso desvía un río, el río nuevo obliga a subdividir en otra parte, eso cambia la red... (medido en el planeta por defecto: ~270 subdivisiones y ~270 colapsos por paso, indefinidamente). Con el relieve estático, el conjunto marcado **solo crece y está acotado** → la malla **converge** (cero cambios tras ~13 pasos). La costa no necesita marca: depende solo de `h` (estático) y converge sola. *Límite consciente*: con relieve vivo (incremento 3) la marca necesitará caducidad.

Las variaciones dependen de la resolución (a más detalle, vecinos más cercanos y diferencias menores): el refinamiento **se detiene solo** donde el campo ya está resuelto.

---

## 9. Paso de topología y orden dentro del tick

**Paso de topología** (función pura del estado):
1. Marcas del driver (§8).
2. Subdivisiones + cascada 2:1; colapsos con veto 2:1 (§5).
3. Nueva lista de claves (ordenada) y nuevas columnas: copiar las hojas que no cambian; recalcular estáticos/geometría de las nuevas; transferir el estado dinámico (§6).
4. Reconstruir la vecindad (§5) y recalcular los derivados globales (§4); recalcular biomas de las hojas nuevas.

**Orden dentro del tick** (madre §9, paso 1): si `t > 0` y `t mod n_lod = 0` → paso de topología; después los sistemas del incremento 1.

**Refinamiento inicial (génesis).** Tras generar la malla uniforme `l_min`, se aplican pasos de topología con **solo los criterios estáticos** (relieve y rasgos; los de `T` y `θ` se desactivan: aún no hay dinámica, y el reparto de las pasadas daría humedades artificiales, porque la tierra nueva nace seca) **hasta que no haya cambios** (como mucho `8·(l_max − l_min) + 8` pasadas). Después se fija el **estado dinámico inicial** del incremento 1 sobre la malla final (`W = 0.5·W_cap`, `V = v_init` en tierra). La génesis no genera transferencias al mar.

---

## 10. Determinismo, conservación y validación (tests)

- **Determinismo**: misma semilla + config → misma salida byte a byte, con LOD activo.
- **No regresión (D11)**: LOD activo con criterios que nunca disparan ≡ LOD apagado (`l_max = l_min`), métricas y mundo byte a byte.
- **Independencia del camino**: (a) operadores: ida y vuelta = identidad (§6.3) en familias sin excepciones, a épsilon de máquina; (b) mundo: subdividir todas las hojas y colapsarlas devuelve el estado celda a celda, y lo que no vuelve está en `lod_al_mar`; (c) estáticos: la elevación de una hoja no depende de la historia de la malla.
- **Conservación**: el balance global (§6.4) cierra a tolerancia de coma flotante en todos los ticks con LOD activo y pasos de topología frecuentes.
- **Invariante 2:1** tras cada paso; **vecindad simétrica**; a nivel uniforme, 8 vecinos (7 en las 24 esquinas del cubo).
- **Sin parpadeo**: con solo criterios estáticos, la topología converge (los últimos pasos no cambian nada). *Este test cazó dos fallos reales durante la implementación (§13).*
- **Métricas globales estables**: activar el LOD no cambia mucho la temperatura media ni la fracción de océano.

**Criterios de "hecho"** (CLAUDE §10): `fmt` + `clippy -D warnings` limpios; `cargo test` en verde; documentos vivos actualizados; parámetros en config.

---

## 11. Parámetros y métricas

### Config `[lod]` (nueva sección)
| Clave | Significado | Defecto (propuesto, a calibrar) |
|---|---|---|
| `l_min` | nivel base uniforme (sustituye a `planet.l_unif`) | 0 |
| `l_max` | nivel máximo; `l_max = l_min` ⇒ LOD apagado | 3 |
| `n_lod` | cadencia del paso de topología (ticks) | 60 |
| `dh_sub` | variación de altura con vecinos que dispara subdividir (m) | 500 |
| `dt_sub` | ídem temperatura (°C) | 4 |
| `dtheta_sub` | ídem humedad relativa del suelo (–) | 0.3 |
| `h_hist` | factor de histéresis para colapsar (< 1) | 0.5 |
| `refine_features` | refinar costa, ríos y lagos | true |

Otros nuevos: `worldgen.octaves_per_level` (1) y `vegetation.v_init` (1.0, biomasa inicial de la tierra, antes fija en el código).

### Métricas por tick (esquema de salida v2)
Nº de hojas y **hojas por nivel**; subdivisiones y colapsos del tick; `lod_al_mar` (m³); área de tierra; longitud de ríos con el lado real de cada celda (`√área`). Se mantienen las del incremento 1.

---

## 12. Fuera de alcance → siguientes incrementos

| Diferido | Incremento |
|---|---|
| Disparadores de agentes y eventos | cuando existan los agentes |
| Erosión/sedimentación y pedogénesis (relieve vivo), D-infinito, lagos con volumen dinámico | **3** |
| Afinidad de reparto no trivial, sub-pasos temporales por nivel | futuro |

---

## 13. Estado de implementación (2026-11-04)

**Implementado y verificado.** Código en `src/terreno/`: `statics.rs` (generador de estáticos, §3), `hydro.rs` (derivados globales, §4), `lod.rs` (vecindad multinivel, operadores de reparto, driver, paso de topología, §5–§9), `geometry.rs` (claves `u64` y vecino por tramo compartido), `world.rs` (topología con claves ordenadas y vecindad CSR), `systems.rs` (difusión simétrica, balance con `lod_al_mar`), `metrics.rs` (esquema v2), `worldgen.rs` (génesis + refinamiento inicial). Config `[lod]` en `config/terreno.toml`. Tests: 6 unitarios nuevos (claves, teselado de hijas, vecindad recíproca con 24 esquinas, hash por contador, elevación pura, octavas) + 3 de operadores (ida y vuelta = identidad, conservación con mar y exceso, tierra nueva) + 8 de integración nuevos (`tests/terreno.rs`: vecindad uniforme de Moore, determinismo con LOD, no regresión, equilibrio 2:1, conservación con pasos de topología, ida y vuelta del mundo entero, convergencia sin parpadeo, refinado de rasgos + métricas globales).

**Verificación:** `fmt` + `clippy -D warnings` limpios; `cargo test` en verde; salida idéntica byte a byte entre dos procesos y entre `debug` y `release`; balance de agua con LOD a **2.7·10⁻¹²** relativo (peor tick del año).

**Resultados con la config por defecto** (`n0 = 48`, `l_max = 3`, 365 ticks, semilla 42):

| | Incremento 1 | Código nuevo, malla uniforme | LOD `l_max = 3` |
|---|---|---|---|
| Celdas | 13 824 | 13 824 | 190 278 (génesis) → 190 695 (fin) |
| T media anual (°C) | 9.128 | 9.128 | 9.125 |
| θ media anual | 0.354 | 0.349 | 0.374 |
| Precipitación anual (10¹⁵ m³) | 0.336 | 0.333 | 0.342 |
| Fracción de tierra | 0.392 | 0.392 | 0.393 |
| Lagos (celdas) | 34 | 34 | 9 681 |
| Selva / desierto (% del planeta) | 0.98 / 6.47 | 1.00 / 6.62 | 1.50 / 5.91 |

- La **nueva línea base uniforme** (D2b, D3, D5, D2c) apenas cambia la física: la elevación es bit a bit la misma y los biomas varían < 0.4 puntos.
- El LOD deja **190 278 hojas** = 21.5 % de la malla uniforme fina equivalente (884 736): hojas al nivel 3 ≈ 124 200, sobre todo **costas** (el ruido con 9 octavas da costas muy recortadas), sus familias y los ríos. Coste: ~13 s y ~340 MB por año simulado en `release` (en `debug`, bastante más lento: usar `--release`).
- **Lagos**: a nivel 3 el relieve de ruido (sin erosión) tiene miles de hondonadas cerradas → ~9 700 celdas-lago. Son lagos estáticos (la física los trata como tierra saturada); desaparecerán en buena parte con la erosión del incremento 3.

**Hallazgos durante la implementación** (cazados por los tests, ya corregidos):
1. **Parpadeo por ríos**: una familia se colapsaba y su madre, al recoger el drenaje de las 4 hijas (185 151 km² > umbral de 80 000), pasaba a ser río y se volvía a subdividir cada dos pasos. La anticipación barata no lo ve → se añadió el **ensayo en seco** (§8).
2. **No convergencia en el planeta completo**: incluso con el ensayo en seco, el acoplamiento malla ↔ red de ríos mantenía un vaivén de ~270 cambios por paso → **marca hidrológica pegajosa** (§8). Con ella, convergencia en 13 pasos. El colchón espacial de 1 celda de la v1.1 se retiró: con la marca y el ensayo en seco ya no aporta y encarecía la malla.
3. **Humedades artificiales en la génesis**: la tierra nueva (nace seca) disparaba el criterio de `θ` en las pasadas iniciales → la génesis usa **solo criterios estáticos** (§9).

**Desviaciones respecto al texto de §1–§12:** ninguna de fondo; los tres hallazgos ya están incorporados arriba (§8, §9). Los umbrales por defecto (§11) se mantienen tras la calibración.

> **Revisado en el incremento 3 (2026-11-05).** Con el suelo dinámico (roca desnuda), el reparto de §6.1 se amplía: si las hijas de tierra suman capacidad de agua 0, la nieve y la biomasa van igualmente a ellas y solo el agua del suelo va al mar. Además, con el relieve vivo se añade el acondicionamiento hidrológico de las hojas nuevas. Ver [`incremento-3-diseno.md`](./incremento-3-diseno.md) §3 y §16.
