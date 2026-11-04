# Diseño técnico — Terreno · Incremento 2: LOD dinámico adaptativo

**Sistema:** Resolución adaptativa (subdivisión/colapso) sobre el sustrato del incremento 1.
**Versión:** 1.0
**Documentos previos:** [`diseno_terreno_espacio_geografia.md`](./diseno_terreno_espacio_geografia.md) §2, §6.1 (visión madre) · [`incremento-1-diseno.md`](./incremento-1-diseno.md) (base uniforme ya implementada).
**Destinatario:** Claude Code (Opus 4.8) para implementación.
**Naturaleza:** Plano teórico del incremento. Fija la matemática y las reglas de determinismo; el código queda a criterio de quien programe.

> **Notación** heredada. "Hoja" = celda activa (nodo del quadtree sin subdividir). "Extensivo" se suma, "intensivo" se promedia (§ notación del madre).

---

## 0. Objetivo

Que la resolución **siga a la relevancia**: las celdas se **subdividen** en 4 hijas un nivel más finas donde hace falta detalle (costas, ríos, gradientes fuertes; en el futuro, agentes y eventos) y se **colapsan** de vuelta cuando la zona deja de importar. La física del incremento 1 corre **sin cambios** sobre la malla no uniforme resultante (es invariante a la escala). El determinismo se mantiene por **independencia del camino**: el resultado no depende de qué celdas estuvieron finas ni cuándo.

Es la pieza que hace tratable un planeta entero: a nivel uniforme fino (`L_max`) no cabría (miles de millones de celdas); con LOD, solo se paga detalle donde aporta.

---

## 1. Alcance

### Entra
- **Bosque de quadtrees** (uno por cara), con hojas a niveles mezclados entre `L_min` y `L_max`.
- **Operadores de subdivisión y colapso** conservativos y **path-independent**.
- **Vecindad entre niveles** con **árbol balanceado 2:1** (hojas adyacentes difieren como mucho un nivel).
- **Driver de refinamiento**: criterios (gradiente de h/T/θ, cercanía a costa/río/lago) evaluados cada `N_lod` ticks, con histéresis para no oscilar.
- Adaptación de los 5 sistemas del incremento 1 a la malla no uniforme (casi todo ya es invariante a escala).
- **Tests**: independencia del camino, conservación bajo subdivisión/colapso, invariante 2:1, determinismo, y que las métricas globales apenas cambian al activar el LOD.

### NO entra (diferido)
- Disparadores de **agentes** y **eventos** (no existen aún): el gancho se deja, sin productores.
- **Erosión / pedogénesis** (incremento 3): el relieve sigue estático, así que la red de ríos no se recomputa.
- Reparto por afinidad distinta del área/capacidad (se usa el reparto conservativo simple, §3).

---

## 2. Árbol y almacenamiento

### Direccionamiento jerárquico
- Una celda (a cualquier nivel) es `A = (c, L, i, j)`, con `N_L = N0·2^L` celdas por arista de cara, `L ∈ [L_min, L_max]`.
- **Padre** de `(c,L,i,j)` = `(c, L−1, i/2, j/2)`. **Hijas** = `(c, L+1, 2i+di, 2j+dj)`, `di,dj ∈ {0,1}`.
- La geometría (`center_dir`, `cell_area`, `lat_lon`) ya funciona a cualquier `N_L`: la posición y el área de una celda son función pura de su dirección `(c,L,i,j)`. **No hay que interpolar nada.**
- Cada dirección se codifica en un `id: u64` estable: `c` (3 bits) · `L` (5 bits) · `j` (28 bits) · `i` (28 bits). Es la clave canónica.

### Layout SoA con reconstrucción en cada paso de topología
- Las hojas activas se guardan en columnas `Vec` (SoA, como el incremento 1), alineadas con una **lista de hojas ordenada por `id`** (`leaves: Vec<u64>`). Ese orden canónico fija las reducciones deterministas.
- La topología **solo cambia cada `N_lod` ticks**. En ese paso: se aplican subdivisiones/colapsos, se **reconstruye** `leaves` ordenada y las columnas SoA, y se recalcula el mapa `index_of: id → slot`. Entre pasos de topología, las columnas son fijas y la física itera un orden estable.
- `index_of` es un `HashMap` usado **solo para localizar vecinos** (nunca se itera → su desorden no afecta al determinismo). Toda iteración de física/reducción va por `leaves` (orden de `id`).

> Regla de oro de determinismo: **iterar siempre `leaves` (ordenado por id); usar `index_of` solo para consultas puntuales de vecindad.**

---

## 3. Subdivisión (hoja → 4 hijas, L → L+1)

Como en el incremento 1 todos los campos se guardan como **densidades** (mm de lámina de agua, nieve en mm, biomasa por celda) y **Σ áreas de las 4 hijas = área de la madre**, el reparto conservativo es simple:

**a) Estáticos (h, pendiente, textura, W_cap, k_perm, F0):** se **recalculan desde el ruido/posición** en la dirección fina de cada hija (el valor "verdadero" fino, §6.1(1) del madre). No se interpola desde la madre.

**b) Nieve `SWE` y biomasa `V` (densidades sin límite superior duro):** cada hija hereda el valor de la madre:
```
SWE_hija = SWE_madre ,  V_hija = V_madre
```
Conserva el total porque `Σ (X_madre · a_hija) = X_madre · A_madre` (densidad constante).

**c) Agua del suelo `W` (densidad con tope `W_cap` que cambia al refinar):** como las hijas tienen `W_cap` recomputado (distinto del de la madre), copiar mm podría superar el tope. Se reparte por **volumen**, proporcional a la capacidad de cada hija:
```
V_w = (W_madre/1000)·A_madre                      (volumen de agua de la madre, m³)
W_hija_i = V_w·1000 · W_cap_i / Σ_j (W_cap_j·a_j)   (mm; ≤ W_cap_i si hay capacidad total suficiente)
```
Conserva `V_w` exactamente: `Σ_i (W_hija_i/1000)·a_i = V_w · Σ(W_cap_i·a_i)/Σ(W_cap_j·a_j) = V_w`. ✓

---

## 4. Colapso (4 hijas → madre, L+1 → L)

**Estáticos:** se **recalculan** en la dirección de la madre (función pura del ruido) → idénticos vengan de donde vengan.

**Densidades (W, SWE, V):** media ponderada por área (conserva el total):
```
X_madre = Σ_i (X_hija_i · a_i) / A_madre
```

**Independencia del camino (test clave).** Subdividir y luego colapsar debe devolver el estado original:
- Estáticos: se recalculan del ruido en ambos sentidos → idénticos. ✓
- SWE, V: subdividir copia `v` a las 4 hijas; colapsar promedia por área → `Σ v·a_i/A = v`. Identidad. ✓
- W: subdividir da `W_i = V_w·1000·W_cap_i/Σ(W_cap·a)`; colapsar → `Σ W_i·a_i/A = V_w·1000/A = W_madre`. Identidad. ✓

Todas las operaciones son función pura de `(estado, posición, semilla)` ⇒ **el resultado es independiente del camino** (§6.1 y §10 del madre).

---

## 5. Vecindad entre niveles y balance 2:1

Se impone un **árbol balanceado 2:1**: dos hojas adyacentes difieren como mucho un nivel. Es el estándar de mallas adaptativas y acota los casos de vecindad a tres.

**Consulta de vecino** de la hoja `(c,L,i,j)` en una dirección de Moore:
1. Se calcula la dirección del vecino **al mismo nivel** `(c',L,i',j')` (intra-cara trivial; inter-cara con el mismo mecanismo geométrico del incremento 1, pero a resolución `N_L`).
2. Se resuelve a hoja real:
   - Si `(c',L,i',j')` es hoja activa → **vecino al mismo nivel**.
   - Si su **padre** es hoja activa → **vecino más grueso** (un nivel menos).
   - Si está subdividido → **vecinos más finos**: las 2 hijas de ese lado (1 hija si la dirección es diagonal).
3. Con balance 2:1, no hay más casos.

**Balanceo:** tras marcar subdivisiones/colapsos y antes de reconstruir, se propaga: si al subdividir una hoja aparece un vecino que quedaría a distancia >1 nivel, se subdivide también ese vecino (cascada acotada). Un colapso se **veta** si rompería el balance 2:1.

---

## 6. La física sobre la malla no uniforme (qué cambia y qué no)

Casi nada cambia, porque la física del incremento 1 ya se escribió contra una API invariante a la escala:

| Sistema | Cambio necesario |
|---|---|
| Clima, nieve | **Ninguno**: son locales por celda (usan lat, h, θ de la propia hoja). |
| Balance hídrico (difusión lateral) | La difusión ya transfiere **volumen** por pares con razón de áreas `A_i/A_j` (incr. 1). Es conservativa para **cualquier** par, incluso de niveles distintos. Solo cambia el enumerador de vecinos (ahora puede devolver hojas de otro nivel). |
| Agua superficial (ríos) | Receptor = vecino-hoja de menor `filled_h` (mezcla niveles sin problema). Acumulación de flujo sobre las hojas. |
| Vegetación | Local por celda. Ninguno. |
| Gradiente/pendiente (world-gen y clima) | El ajuste por mínimos cuadrados usa desplazamientos y Δh de los vecinos-hoja; admite cualquier conjunto (mezcla de niveles, varios vecinos por lado). Ninguno estructural. |
| Interfaz de agentes | Ninguno. |

⇒ El grueso del incremento 2 es el **árbol + operadores + vecindad + driver**, no la física.

---

## 7. Driver de refinamiento (cada `N_lod` ticks)

**Subdividir** una hoja con `L < L_max` si:
```
subdividir ⟺ ( |∇h|,|∇T| o |∇θ| relativo > grad_sub )
            ∨ ( la hoja es o toca RIO / OCEANO / LAGO )
            ∨ ( [futuro] agentes o evento activos )
```
**Colapsar** las 4 hojas hermanas → madre si **ninguna** cumple el criterio de subdivisión **con histéresis** (se colapsa solo si están **claramente** por debajo: umbral de colapso = `grad_sub · h_hist`, con `h_hist < 1`, p. ej. 0.5), y si no rompe el balance 2:1.

La histéresis evita el "parpadeo" (subdividir/colapsar en ticks alternos). `N_lod` (cadencia) es grande porque la topología no cambia cada instante.

---

## 8. Determinismo, conservación y validación (tests)

- **Independencia del camino** (test central): partiendo de un mundo uniforme, subdividir una región y colapsarla de vuelta debe reproducir el estado uniforme celda a celda dentro de tolerancia. Y: dos corridas que refinan por caminos distintos (p. ej. `N_lod` distinto) convergen en las métricas globales dentro de tolerancia.
- **Conservación bajo subdivisión/colapso**: el volumen total de agua (suelo + nieve) y la biomasa total no cambian al subdividir ni al colapsar (a épsilon de máquina).
- **Balance de agua global** sigue cerrando (métrica del incremento 1) con LOD activo.
- **Invariante 2:1**: en todo momento, hojas adyacentes difieren ≤ 1 nivel.
- **Determinismo**: misma semilla + misma config → misma salida byte a byte, con y sin LOD.
- **No regresión**: con LOD activado pero criterios que no disparan (o `L_max = L_min`), la salida es **idéntica** a la del incremento 1 (misma malla uniforme).

**Criterios de "hecho"** (CLAUDE §10): `fmt` + `clippy -D warnings` limpios; `cargo test` en verde (tests nuevos incluidos); documentos vivos actualizados; parámetros en config.

---

## 9. Parámetros nuevos (config `[lod]`)

| Símbolo | Significado | Defecto (propuesto) |
|---|---|---|
| `l_min` | nivel base uniforme de arranque | 0 |
| `l_max` | nivel máximo de subdivisión | 3 |
| `n_lod` | cadencia de revisión de topología (ticks) | 60 |
| `grad_sub` | umbral de gradiente relativo para subdividir | 0.15 |
| `h_hist` | factor de histéresis para colapsar (< 1) | 0.5 |
| `refine_costa` | refinar celdas de costa/río/lago | true |

(El `l_unif` del incremento 1 pasa a ser `l_min`; con `l_max = l_min` se recupera exactamente el incremento 1.)

---

## 10. Fuera de alcance → siguientes incrementos

| Diferido | Incremento |
|---|---|
| Disparadores de agentes y eventos | cuando existan los agentes |
| Erosión/sedimentación y pedogénesis (relieve vivo) + recomputar `RedRios` | **3** |
| Afinidad de reparto no trivial, criterios de refinamiento avanzados | futuro |

---

## 11. Decisiones de implementación propuestas (por defecto, a confirmar)

1. **Árbol balanceado 2:1** (estándar; acota la vecindad a 3 casos).
2. **Reconstrucción de las columnas SoA ordenadas por `id`** en cada paso de topología (cada `N_lod`), no mantenimiento incremental con free-list: más simple y trivialmente determinista; el coste es amortizable porque la topología cambia poco.
3. **Reparto conservativo por área/capacidad** (§3–§4), sin afinidad exótica.
4. **Driver sin agentes**: refinar por gradiente físico + costa/río/lago (lo que da detalle geográfico útil hoy).
5. `l_max = 3` por defecto (planeta manejable en debug); subir por config para experimentos.
