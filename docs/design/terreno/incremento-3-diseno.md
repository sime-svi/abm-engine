# Diseño técnico — Terreno · Incremento 3: Relieve vivo (erosión, sedimento y suelo)

**Sistema:** Evolución del relieve y del suelo sobre el sustrato de los incrementos 1 y 2.
**Versión:** 1.1 (2026-11-05) — implementado. Las 4 decisiones de §14 fueron confirmadas por el responsable el 2026-11-04; la v1.1 incorpora lo aprendido al implementar (§16).
**Documentos previos:** [`diseno_terreno_espacio_geografia.md`](./diseno_terreno_espacio_geografia.md) §6.7, §6.8, §9, §10 (visión madre) · [`incremento-1-diseno.md`](./incremento-1-diseno.md) · [`incremento-2-diseno.md`](./incremento-2-diseno.md) (LOD dinámico, ya implementado).
**Destinatario:** Claude Code para implementación.
**Naturaleza:** Plano teórico del incremento. Fija la matemática y las reglas de determinismo y conservación; el código queda a criterio de quien programe.

> **Notación** heredada. `h0` = elevación "de nacimiento" (ruido, función pura de la celda, incr. 2 §3); `e` = erosión/depósito acumulado (m, negativo = rebajado); `h = h0 + e`. `a_i` = área de la hoja `i` (m²); `L_i` = distancia entre los centros de `i` y su receptor (m); `P̄` = precipitación media anual (m/año); `T̄` = temperatura media anual (°C). Tasas en **unidades por año**, no por tick (§2).

---

## 0. Objetivo

Que el relieve **deje de ser fijo**: los ríos excavan valles, el sedimento rellena las hondonadas, las laderas se suavizan y el suelo se forma y se pierde. Debe emerger, sin pintarlo, lo que pide la visión madre (§1): **valles fluviales**, **cuencas hidrográficas coherentes** y **montañas redondeadas**.

Hay además un **motivo práctico inmediato**. Con 9 octavas (nivel 3), el relieve de ruido sin erosión deja miles de **hondonadas cerradas artificiales**: unas 9 700 celdas-lago frente a 34 en la malla uniforme (incr. 2 §13). Para los futuros agentes, toda celda de agua es infranqueable y cuenta como fuente de agua, así que esos lagos falsos distorsionarían su movimiento y su acceso al agua. La erosión es el mecanismo físico que los elimina: el sedimento rellena las hondonadas y la incisión rebaja sus bordes.

---

## 1. Alcance

### Entra
- Relieve dinámico compatible con el LOD: `h = h0 + e` (§3).
- **Incisión fluvial** por potencia de arroyo, con esquema **implícito** incondicionalmente estable (§4).
- **Sedimento**: transporte aguas abajo y **relleno de depresiones**; lo que llega al mar sale del sistema (§5).
- **Difusión de ladera** conservativa sobre la malla multinivel (§6).
- **Suelo**: profundidad `z` dinámica (meteorización, pérdida por erosión, ganancia por depósito), y capacidad de campo `W_cap` proporcional a `z` (§7).
- **Dos regímenes de tiempo** con el mismo código: **génesis geológica** antes de empezar y **erosión en ejecución** durante la simulación (§2, §8, §9).
- Métricas y tests (§11, §13).

### NO entra (→ incremento 4 y siguientes, §15)
- **Lagos dinámicos** (volumen, hipsometría, desborde, evaporación; madre §6.5). Siguen siendo lagos estáticos (depresión rellenada), solo que ahora hay muchos menos.
- **D-infinito / flujo multidireccional**: el esquema implícito de §4 exige **un receptor por celda**.
- **Fertilidad dinámica** (madre §6.8): su motor principal es el uso agrícola, que necesita agentes.
- **Depósito en el mar** (deltas, plataforma continental): el mar es un sumidero en esta versión.
- Erodibilidad variable por tipo de roca, y registro de **eventos** discretos (madre §7).

---

## 2. Dos regímenes de tiempo (D1)

Excavar un valle lleva del orden de 10⁵–10⁶ años; el tick es un día. Un único ritmo no sirve para ambas cosas, así que el mismo paso de erosión (§4–§7) se ejecuta con un **intervalo `dt` en años** que decide quien lo llama:

| Régimen | Cuándo | `dt` por paso | Para qué |
|---|---|---|---|
| **Génesis geológica** | Una vez, antes del tick 0 | `geo_dt_years` (p. ej. 5 000 años), `geo_steps` pasos (p. ej. 200 → 1 millón de años) | Que el mundo arranque **ya tallado**: valles, cuencas y sin hondonadas artificiales. |
| **Erosión en ejecución** | Cada `run_every` ticks | `run_every / T_año · run_accel` años | Que el relieve siga vivo durante la simulación. Con `run_accel = 1` (realista) apenas cambia a escala humana, como en la realidad. Con `run_accel ≫ 1`, sirve para experimentos. |

**Unidades por año.** Los parámetros de erosión y suelo se expresan por año (m/año, m²/año), no por tick. Así no dependen de la longitud del tick y no hace falta otro reescalado ×48/365 como el del incremento 1. Los valores de la visión madre están por tick, con 48 ticks al año, y son ~10³ veces más rápidos que los reales. Aquí se parte de órdenes de magnitud reales y se calibra (§12).

---

## 3. Relieve con LOD: `h = h0 + e` (D2)

El incremento 2 se apoya en que los estáticos son función pura de la celda. Con erosión, la elevación deja de serlo. Se resuelve separando dos partes:

- `h0(c, L, i, j)`: la elevación de nacimiento. **Pura** (incr. 2 §3) y recalculada al crear cada celda, como hasta ahora.
- `e`: lo erosionado o depositado (m). **Dinámico e intensivo** (espesor por unidad de área). El LOD lo trata como la nieve: **copia** al subdividir y **media por área** al colapsar. Así se conserva `Σ e·a` (volumen de material) y la ida y vuelta es la identidad (incr. 2 §6.3).
- `h = h0 + e`, y la clasificación de océano es `h < nivel_mar`.

**Gradiente.** `∇h = ∇h0 + ∇e`:
- `∇h0`: diferencias centrales de la función pura (incr. 2 §3.2), guardadas por celda.
- `∇e`: mínimos cuadrados sobre los vecinos (campo dinámico, como `T` o `θ`).

La pendiente y la orientación de la física (escorrentía, orografía) usan `∇h`. La **textura del suelo** sigue derivando de `∇h0`, porque es un estático puro. Con `e ≡ 0` todo se reduce **exactamente** al incremento 2.

**La tierra nunca baja del nivel del mar por erosión** (no hay transferencias de agua tierra→mar por esta vía):
- La incisión implícita (§4) es una combinación convexa entre `h_i` y un nivel de base que es `≥ nivel_mar` (desembocadura = nivel del mar).
- La difusión (§6) solo actúa entre celdas de tierra y, bajo su condición de estabilidad, cumple el principio del máximo.
- El depósito solo sube.

**Acondicionamiento hidrológico de las hojas nuevas** (añadido en la v1.1, §16). El detalle fino que añade una subdivisión posterior (las octavas extra de `h0`) **no** está erosionado, y sus hondonadas son un artefacto de bajar de escala. Además, un colapso puede dejar encerrada a una vecina. Regla: **un cambio de malla no puede crear hondonadas**. Una hoja solo queda en depresión (`h < filled_h`) si su origen lo estaba: la madre para una hija, alguna de las hermanas para una madre, ella misma para una hoja que no cambia. Si no, se rellena hasta la cota de derrame, como habría hecho el sedimento. El volumen se apunta en `fill_m3`, un término propio del balance de material. Solo se aplica con el relieve vivo activo; sin él, todo queda como en el incremento 2.

---

## 4. Incisión fluvial: potencia de arroyo implícita (D3)

**Caudal efectivo, ponderado por lluvia** (D9):
```
Q_i = P̄_i·a_i + Σ_{donantes d de i} Q_d          (m³/año; se acumula aguas arriba→abajo)
```
`P̄` es la precipitación media anual de la celda:
- **En la génesis**, sale de forma analítica de la fórmula del clima (incr. 1). El término estacional promedia a cero cuando `seas_precip_amp ≤ 1`, así que `P̄ = p_ref·banda(|φ|)·m_agua·m_oro`, con `m_oro` evaluado con `∇h` actual.
- **En ejecución**, es la media acumulada `precip_sum / sample_count · T_año`.

La visión madre usa el área de drenaje: es el caso particular de `P̄` uniforme. Ponderar por lluvia hace que la erosión dependa del clima: las zonas de sombra pluviométrica se erosionan menos.

**Potencia de arroyo con `n = 1`** (madre: `m_ero = 0.5`, `n_ero = 1`): `E = K·Q^m·S`.

**Esquema implícito** (Braun y Willett, 2013). Se recorren las celdas desde el nivel de base hacia arriba, en el **orden de extracción del priority-flood**, donde todo receptor va antes que sus donantes:
```
h_i' = ( h_i + F_i · b_i ) / ( 1 + F_i ) ,     F_i = K · Q_i^m · dt / L_i
```
El nivel de base `b_i` depende del receptor `r` de `i`:
- **`r` es océano:** `b_i = nivel_mar` (la desembocadura).
- **`r` está en una depresión** (`h_r < filled_h_r`): `b_i = filled_h_r` (la superficie del lago, que es el nivel de base local).
- **En otro caso:** `b_i = h_r'` (la elevación de `r` ya actualizada en este paso).

Propiedades:
- **Estabilidad incondicional y monotonía.** `h_i'` queda entre `b_i` y `h_i` sea cual sea `dt`: no hay oscilaciones, no hay NaN y no se excava por debajo del nivel de base.
- **Las celdas en depresión** (`h_i < filled_h_i`) **no se excavan**: reciben sedimento (§5).
- **Espesor erosionado** `ε_i = h_i − h_i' ≥ 0`, con volumen `ε_i·a_i`.

---

## 5. Sedimento y relleno de depresiones (D4)

El material excavado viaja aguas abajo por la red de receptores, en orden de flujo de arriba abajo:
```
Qs_i = ε_i·a_i + Σ_{donantes} Qs_d                   (m³ de sedimento que pasa por i)
si i está en depresión:  d_i = min(Qs_i, (filled_h_i − h_i)·a_i) ;  h_i += d_i/a_i ;  Qs_i −= d_i
si el receptor es océano (o no hay receptor):  V_mar += Qs_i
```
- **Balance exacto**: `Σ ε·a = Σ d + V_mar` en cada paso. Es la métrica de validación del incremento (§11), análoga al balance de agua.
- El relleno nunca sobrepasa la cota de derrame, así que no crea depresiones nuevas. Las hondonadas pequeñas se rellenan antes que las grandes porque necesitan menos volumen; la incisión del borde (§4) completa el trabajo.
- **Sin depósito por capacidad en laderas** (el `C_sed` de la madre). A 26–200 km no se resuelven llanuras aluviales, y depositar en laderas podría formar presas artificiales. Queda para el futuro, junto con los deltas.

---

## 6. Difusión de ladera (D5)

Suaviza crestas y rellena irregularidades. Es un flujo de volumen **por pares** (exactamente conservativo) entre hojas de **tierra que comparten arista** (pares `g = 1` del incr. 2 §5):
```
F_ij = κ · dt · (h_i − h_j) · ℓ_ij / d_ij        (m³; ℓ_ij = √min(a_i, a_j), d_ij = distancia entre centros)
h_i −= F_ij / a_i ,   h_j += F_ij / a_j
```
- **Explícita, con subpasos automáticos.** Si `c_i = κ·dt/a_i · Σ_j ℓ_ij/d_ij` supera ½ en alguna hoja, el paso se divide en `⌈2·max c_i⌉` subpasos. Así se mantienen la estabilidad y el principio del máximo.
- `κ` es una **difusividad efectiva** (m²/año), que representa procesos por debajo de la celda. Es mucho mayor que el reptado real (~0.01 m²/año), que a 26 km sería invisible. Se calibra (§12).
- **Orden dentro del paso de erosión:** incisión (§4) → sedimento (§5) → difusión → suelo (§7). Entre pasos se recalculan los derivados globales (incr. 2 §4).

---

## 7. Suelo (pedogénesis) (D6)

**Profundidad de suelo** `z` (m) por hoja: dinámica e intensiva. El LOD la copia y la promedia por área sobre las hijas de tierra. En el océano `z = 0`.

**Meteorización** (madre §6.8):
```
dz = k_weath · f_clima · exp(−z / z_ref) · dt
f_clima = clamp(T̄, 0, ∞)/T_ref · min(1, P̄ / P_ref_suelo)
```
La madre usa `θ`. Aquí se usa `P̄`, que está disponible en la génesis sin simular la hidrología y es coherente en ambos regímenes. `T̄` también es analítica en la génesis: `t_ecuador − k_lat·|φ| − lapse·h_tierra`, porque el término estacional promedia a cero.

**Erosión y depósito sobre el suelo:**
- **Incisión:** se lleva primero el suelo y luego la roca: `z −= min(z, ε)`.
- **Depósito:** se convierte en suelo: `z += d/a`.
- **Difusión:** mueve suelo: `z += Δh_dif`, con `z ≥ 0` (si se agota, aflora la roca).

**Capacidad de campo** (acotada en la v1.1, §16):
```
W_cap = W_cap0 · clamp( z / z_init , f_rock , z_root / z_init )
```
- `W_cap0` es el valor por textura del incremento 2 (puro). Con `z = z_init` el factor vale exactamente 1: es **idéntico, bit a bit**, al incremento 2. Equivale a la `W_cap = z·θ_fc` de la madre con `θ_fc = W_cap0/z_init`.
- **Techo `z_root`** (profundidad de raíces): solo retiene agua útil la zona de raíces. Sin techo, los lagos rellenados con cientos de metros de sedimento daban capacidades absurdas (de hasta 136 m de agua).
- **Suelo `f_rock`:** una celda de decenas de km nunca es solo roca, siempre quedan grietas y suelo en vaguadas. Sin este suelo, la roca desnuda (θ = 0) junto a suelo profundo disparaba el criterio de humedad del LOD por todas partes.

**Consecuencias:**
- **Roca desnuda** (`z → 0`): `W_cap → f_rock·W_cap0`. El agua escurre casi toda y la vegetación sufre. Emergen las cumbres peladas.
- **Si `W_cap` encoge** por debajo de `W`, el exceso se drena en el siguiente balance hídrico, por el mismo mecanismo del incremento 1. Se conserva y no hace falta un término nuevo.
- **Corrección obligada del reparto del LOD** (incr. 2 §6.1). Hoy, si las hijas de tierra suman capacidad 0, todo va al mar, incluidas la nieve y la biomasa. Debe separarse: la nieve y la biomasa van a las hijas de tierra aunque `W_cap = 0`, y solo el agua del suelo que no cabe va al mar.

---

## 8. Génesis geológica (D7)

```
1. Malla uniforme l_min → estáticos → derivados → refinamiento LOD inicial (incr. 2 §9).
2. Para cada ronda r = 1..geo_rounds:
     a. geo_steps/geo_rounds pasos de erosión con dt = geo_dt_years, malla CONGELADA;
        en cada paso: derivados globales → Q → incisión → sedimento → difusión → suelo → ∇e.
     b. Reiniciar la marca hidrológica (incr. 2 §8) y refinar el LOD hasta converger.
3. Estado dinámico inicial (incr. 1) sobre la malla final; t = 0.
```
- **Por qué en rondas.** La erosión fija los ríos en sus valles, así que tras una ronda la red de ríos es más estable que la del ruido. La siguiente ronda erosiona también las celdas finas que han aparecido siguiendo esos ríos.
- **Por qué reiniciar la marca hidrológica.** Las marcas de la génesis sin erosión clavaban refinamiento sobre lagos falsos y sobre posiciones viejas de los ríos.
- **Coste estimado** con la config por defecto (~190 000 hojas): ~50 ms por paso en `release` (dominado por el priority-flood), es decir ~10 s para 200 pasos, más las rondas de LOD. En `debug` los tests usan mundos pequeños.

---

## 9. Erosión en ejecución (D8)

Cada `run_every` ticks, después de la vegetación (orden de la madre §9, pasos 7–8):
1. Un paso de erosión (§4–§7) con `dt = run_every / T_año · run_accel` años, usando `P̄` y `T̄` acumulados.
2. Recalcular los derivados globales, `∇h`, `W_cap` y los biomas que dependan de ello.

- **Agua:** la erosión no mueve agua. El exceso de `W` sobre una `W_cap` menor se drena en el tick siguiente, y la tierra no pasa a mar (§3). El balance de agua sigue cerrando sin términos nuevos.
- **Con `run_accel ≫ 1`** los ríos pueden migrar, y la marca hidrológica pegajosa refinaría de más. Habrá que darle **caducidad**; queda anotado para cuando se use (incr. 2 §8).

---

## 10. Orden dentro del tick

```
1. Paso de topología LOD           — si t > 0 y t mod n_lod = 0     (incr. 2)
2. Clima → 3. Nieve → 4. Balance hídrico → difusión de humedad → 5. Agua superficial
6. Vegetación                      — cada n_veg ticks
7. Erosión + sedimento + suelo     — cada run_every ticks (§9)       [NUEVO]
8. Métricas
```

---

## 11. Determinismo, conservación y validación (tests)

- **Determinismo**: misma semilla + config → misma salida byte a byte, con génesis geológica y erosión en ejecución.
- **No regresión**: con `geo_steps = 0` y la erosión en ejecución apagada, la salida es **idéntica** a la del incremento 2.
- **Conservación de material**: `Σ ε·a = Σ d + V_mar` en cada paso, a tolerancia de coma flotante. Y, a nivel global, `Σ e·a + V_mar_acumulado = 0`, incluidos los pasos del LOD.
- **Estabilidad del esquema implícito**: un paso con `dt` enorme no produce NaN, no baja ninguna celda de su nivel de base y deja toda la tierra `≥ nivel_mar`.
- **Relleno**: una hondonada sintética que recibe sedimento suficiente queda a su cota de derrame y no la rebasa.
- **LOD**: la ida y vuelta conserva `e` y `z` (identidad), y el reparto con `W_cap = 0` conserva nieve y biomasa.
- **Difusión**: conserva el volumen y respeta el principio del máximo (no crea extremos nuevos).
- **Génesis** (calibración, con umbrales a fijar al implementar):
  - las celdas-lago bajan al menos un **80 %** frente al incremento 2;
  - la fracción de tierra cambia **±2 puntos** como mucho;
  - la elevación media de la tierra baja **menos de un 25 %**;
  - siguen emergiendo los 9 biomas;
  - la malla del LOD converge.
- **Casos límite**:
  - **mundo plano**: `S = 0`, no hay incisión;
  - **sin lluvia**: `P̄ = 0`, `Q = 0`, no hay incisión (la difusión sí actúa);
  - **una sola montaña**: se forman valles radiales y la cima se redondea.

**Criterios de "hecho"** (CLAUDE §10): `fmt` + `clippy -D warnings` limpios; `cargo test` en verde; documentos vivos actualizados; parámetros en config.

---

## 12. Parámetros nuevos

Todos son propuestas de partida, a calibrar contra los objetivos de §11.

### Sección nueva `[erosion]`
| Clave | Significado | Defecto (propuesto) |
|---|---|---|
| `geo_steps` | pasos de la génesis geológica (0 = sin génesis) | 200 |
| `geo_dt_years` | años por paso de la génesis | 5000 |
| `geo_rounds` | rondas erosión ↔ LOD de la génesis | 2 |
| `k_ero` | erodibilidad `K` (m^(1−3m)·año^(m−1)) | 1e-6 |
| `m_ero` | exponente del caudal (`n = 1` fijo) | 0.5 |
| `kappa_dif` | difusividad de ladera efectiva (m²/año) | 10 |
| `run_every` | cada cuántos ticks se erosiona en ejecución (0 = nunca) | 365 |
| `run_accel` | acelerador del tiempo geológico en ejecución | 1.0 |

### Añadidos a `[soil]`
| Clave | Significado | Defecto (propuesto) |
|---|---|---|
| `z_init` | profundidad de suelo inicial (m) | 1.0 |
| `k_weath` | meteorización máxima (m/año) | 1e-4 (calibrado; propuesta inicial 5e-5) |
| `z_ref` | profundidad de referencia de la producción (m) | 0.5 |
| `t_ref` | temperatura de referencia (°C) | 15 |
| `p_ref_soil` | lluvia de referencia (mm/año) | 1000 |
| `z_root` | profundidad de raíces: techo de la capacidad (m) | 1.5 |
| `f_rock` | fracción de capacidad de la roca desnuda | 0.3 |

**Comprobación de órdenes de magnitud** (`K = 1e-6`, `m = 0.5`): en un río con cuenca de 10⁴ km² y `P̄ = 0.5 m/año`, `Q ≈ 5·10⁹ m³/año`. Con pendiente 0.002, `E ≈ 1e-6 · 7·10⁴ · 0.002 ≈ 0.14 mm/año`, del orden real. En un millón de años da del orden de cien metros de valle; más donde la pendiente es mayor.

---

## 13. Métricas

- **Por tick:**
  - profundidad media de suelo en tierra;
  - en los ticks con erosión: volumen erosionado, depositado y enviado al mar, y el residual de material.
- **En los metadatos de la corrida (génesis):**
  - pasos y años simulados;
  - volúmenes erosionado, depositado y al mar;
  - celdas-lago, elevación media de la tierra y fracción de tierra, antes y después de la génesis.

---

## 14. Decisiones a confirmar por el responsable

1. **Dos regímenes de tiempo** (§2): génesis geológica + erosión realista en ejecución, con acelerador opcional. *Recomendado.* La alternativa es solo erosión acelerada en ejecución, que es lo que sugieren los valores de la madre: más sencilla, pero el mundo arranca con los lagos falsos y el relieve cambia a ritmo irreal mientras viven los agentes.
2. **Reparto del alcance**: incremento 3 = relieve vivo + profundidad de suelo; incremento 4 = lagos dinámicos, flujo multidireccional, fertilidad dinámica y eventos. *Recomendado:* son piezas separables, y la pirámide pide pasos pequeños y verificables.
3. **Caudal ponderado por lluvia** (§4) en vez de solo el área de drenaje de la madre. *Recomendado:* la erosión pasa a depender del clima, y el caso de la madre queda como caso particular.
4. **El mar como sumidero de sedimento** (§5), sin deltas, en esta versión. *Recomendado:* los deltas piden procesos costeros y de plataforma que no existen aún.

---

## 15. Fuera de alcance → siguientes incrementos

| Diferido | Dónde |
|---|---|
| Lagos dinámicos (volumen, hipsometría, desborde, evaporación) | incremento 4 |
| Flujo multidireccional (D-infinito/MFD) | incremento 4 (exige un solver iterativo) |
| Fertilidad dinámica (aporte vegetal, lixiviación, uso agrícola) | incremento 4 / con agentes |
| Depósito por capacidad en laderas, deltas y plataforma continental | futuro |
| Erodibilidad por tipo de roca | futuro |
| Eventos discretos (nace/muere un río, se seca un lago, erosión notable…) | con la tabla `events` (DATABASE_SCHEMA §2.4) |
| Caducidad de la marca hidrológica (para `run_accel ≫ 1`) | cuando se use |

---

## 16. Estado de implementación (2026-11-05)

**Implementado y verificado.**
- **Código:** módulo nuevo `src/terreno/erosion.rs` (paso de erosión, medias de clima, gradiente `∇h0 + ∇e`, capacidad de campo, génesis geológica). Cambios en:
  - `world.rs`: `h0`, `e`, `z_soil`, `grad0_*`, `w_cap0`;
  - `lod.rs`: reparto de `e` y `z`, acondicionamiento, reparto con `W_cap = 0`, anticipación con `h0 + e`;
  - `hydro.rs`: recálculo sin Dijkstra tras erosionar;
  - `systems.rs`: medias anuales de clima reutilizables;
  - `metrics.rs`: esquema v3;
  - `worldgen.rs` y `mod.rs`: génesis y erosión en ejecución.
- **Config:** `[erosion]` y los parámetros nuevos de `[soil]`.
- **Tests:** `tests/erosion.rs` (9) y 1 unitario nuevo de reparto. Los de los incrementos 1 y 2 corren con el relieve estático.

**Verificación:**
- `fmt` y `clippy -D warnings` limpios; `cargo test` en verde.
- **No regresión** con la erosión apagada, contra el binario del incremento 2 (commit `8dab382`): 7 300 valores comparados (365 ticks × 20 campos), **0 diferencias**.
- **Determinismo:** dos procesos con la génesis activa dan una salida idéntica byte a byte.
- **Conservación:**
  - material de la génesis: residual de 31 m³ sobre 1,8·10¹⁶ m³ erosionados;
  - balance global `Σ e·a + al_mar − relleno = 0` verificado en los tests;
  - agua con erosión en ejecución: ≤ 5·10⁻¹² relativo en tres años simulados.

**Resultados con la config por defecto** (semilla 42, un año tras la génesis):

| | Incremento 1 | Incremento 2 (LOD) | Incremento 3 (relieve vivo) |
|---|---|---|---|
| Lagos (celdas) | 34 | 9 681 | **30** |
| Elevación media de la tierra (m) | 490 | 489 | 406 |
| Fracción de tierra | 0.392 | 0.393 | 0.389 |
| T media anual (°C) | 9.13 | 9.13 | 9.35 |
| θ media anual | 0.354 | 0.374 | 0.401 |
| Selva / desierto (% del planeta) | 0.98 / 6.47 | 1.50 / 5.91 | 1.97 / 5.70 |
| Celdas | 13 824 | 190 278 → 190 695 | 192 618 → 199 164 |

- **Génesis:** 200 pasos, 1 millón de años. Lagos 3 477 → 30 (el "antes" ya incluye el acondicionamiento del refinado inicial). Elevación media de la tierra 491 → 406 m (−17 %). Tierra 39,3 % → 39,0 %. Siguen los 9 biomas. Sedimento al mar: 1,8·10¹⁶ m³. Relleno de acondicionamiento: 3·10¹³ m³ (0,2 % de lo erosionado).
- **Erosión en ejecución** (realista): ~1,8·10¹⁰ m³/año en todo el planeta, del orden del sedimento que los ríos de la Tierra llevan al mar (~10¹⁰ m³/año).
- **Clima:** el planeta se calienta 0,2 °C de media porque la tierra está más baja (gradiente térmico vertical).
- **Malla:** en tres años crece cada vez menos (+6 500, +3 500 y +400 celdas): converge.
- **Coste:** génesis + un año ≈ 38 s y ~390 MB en `release` (antes ~13 s).

**Hallazgos durante la implementación** (ya corregidos):
1. **Capacidad de campo sin límites.** Con `W_cap ∝ z`, la roca desnuda (~10 % de la tierra tras la génesis) tenía capacidad 0, y los lagos rellenados de sedimento, hasta 136 m de agua. Los contrastes de humedad resultantes disparaban el criterio θ del LOD: 14 510 subdivisiones en la primera revisión, frente a ~10 en el incremento 2. → Techo `z_root` y suelo `f_rock` (§7), con `f_rock` calibrado a 0,3 y `k_weath` a 1e-4.
2. **Hondonadas del detalle fino sin erosionar.** Cada subdivisión en ejecución traía detalle nuevo con hondonadas. Esos lagos falsos, al ser rasgos, pedían más afinado, que traía más detalle, y así en bucle (805 → 5 100 lagos en un año). → **Acondicionamiento hidrológico** (§3).
3. **Balance de material incompleto.** El relleno del refinado inicial, anterior a la génesis, no se apuntaba. Lo cazó el test del balance global (1,7 % de descuadre).
4. **Veto 2:1 conservador.** Si una familia y su vecina colapsan en el mismo paso, el veto no lo sabe y deja la segunda para la pasada siguiente. Es seguro y converge en pocas pasadas. No es un fallo; el test de ida y vuelta lo tiene en cuenta.

**Desviaciones respecto a §1–§15:**
- `z` se arrastra también en el océano, sin uso, con la misma regla de copia y media que `e`. Así los operadores del LOD no tienen casos especiales.
- La distancia al mar no se recalcula tras un paso de erosión, porque la tierra no pasa a mar (§3).
- La erosión en ejecución corre cada `run_every = 365` ticks: en una corrida de un año no llega a actuar, la primera vez es en el tick 365.
