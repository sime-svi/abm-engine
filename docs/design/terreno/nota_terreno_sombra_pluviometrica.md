# Nota de diseño: calibración de k_oro y sombra pluviométrica extensa

**Sistema:** Terreno, clima (visión madre §6.2; nota de lluvia orográfica)
**Estado:** Decisión aprobada por el director. Lista para implementar.
**Naturaleza:** Plano teórico. Sin código.
**Origen:** Hallazgos del commit bb6a8c0. Con `k_oro = 0,5`, el factor orográfico queda entre 0,99 y 1,01 en el 98 % de la tierra: no hay sombra de lluvia.

---

## 1. Diagnóstico

1. **`k_oro` estaba mal dimensionado en la visión madre.** Multiplica un gradiente adimensional: metros que sube el terreno por metro recorrido. El relieve suavizado a 25 km tiene gradientes pequeños. Una cordillera de 3 000 m con laderas de 75 km da 0,04, y con `k_oro = 0,5` el efecto es de un 2 %. En la realidad, las laderas de barlovento reciben del orden de 2-3 veces la lluvia de la llanura previa.
2. **Aunque se suba `k_oro`, la fórmula local no puede producir los desiertos de detrás de las cordilleras,** que son un objetivo explícito de la visión madre (§1). La fórmula actual solo seca la propia ladera de bajada. En la realidad, la sombra se extiende cientos de km (estepa patagónica, Gran Cuenca de EE. UU.), porque el aire soltó su vapor al subir y no lo recupera al bajar.

## 2. Parte A — Calibrar `k_oro`

Orden de magnitud: para que el barlovento de una cordillera de gradiente 0,04 reciba ~2,5 veces la lluvia, hace falta `k_oro · 0,04 ≈ 1,5`, es decir `k_oro ≈ 37`. Valor inicial: **40**, que coincide con lo medido por Claude Code. Se mantienen los topes `m_oro_min = 0,2` y `m_oro_max = 3,0`.

**Calibración por objetivos (no fijar el número a ciegas):**
- En las laderas de barlovento de las cordilleras grandes (relieve suavizado > 2 000 m), la media de `m_oro` debe estar entre 2 y 3.
- La lluvia total sobre tierra debe quedar dentro de ±5 % de la anterior. Si se desvía más, se aplica un factor global constante `c_P` sobre `P_base`, calculado una vez en la génesis para recuperar el total.

## 3. Parte B — Agotamiento de humedad a favor del viento

### Fenómeno
El aire húmedo que sube por una cordillera se enfría (~6 °C por km en aire saturado). Al enfriarse, puede contener menos vapor: alrededor de un 7 % menos por cada °C. Subir ~2,5 km enfría unos 15 °C, y el aire puede llevar entonces unas 2,9 veces menos vapor; es decir, conserva ~35 % (≈ e^−1). Esa agua cae en barlovento. Al bajar por sotavento, el aire se calienta pero no recupera el agua. Solo se recarga poco a poco con la evaporación sobre tierra, o de golpe al pasar sobre el mar.

### Regla
Para cada celda de tierra, se recorre el **camino del viento hacia atrás** (en sentido contrario a `viento_franja` en cada punto) en pasos de `d_oro`, hasta encontrar mar o alcanzar `L_traj`. Sobre ese camino:
```
Λ(celda) = Σ_j  max( 0 , Δh_oro,j ) · e^( −s_j / L_recarga )
     Δh_oro,j = subida del relieve suavizado en el tramo j, en el sentido del viento
     s_j      = distancia desde el tramo j hasta la celda
     la suma se corta en el primer punto de mar (el aire se recarga: Λ empieza en 0 desde la costa)

w(celda) = e^( −Λ / H_v )                                  (fracción de humedad que le queda al aire)

P(celda, t) = c_P · P_base(φ, dist_costa) · m_oro(celda) · w(celda) · m_est(φ, t)
```
- `Λ` son los "metros de subida que el aire ha sufrido recientemente". Las subidas lejanas pesan menos porque el aire se ha ido recargando por el camino.
- Una segunda cordillera detrás de la primera recibe menos lluvia en su barlovento, porque `w < 1` multiplica también a `m_oro`. Es físicamente coherente.
- El factor de distancia a la costa de `P_base` y `w` son mecanismos distintos: secado continental sobre llano, y pérdida por ascenso. Se mantienen los dos, y el posible doble secado se absorbe en la calibración global `c_P`.

### Invariancia de escala y coste
- `Λ` y `w` se calculan sobre el **nivel de referencia `L_oro`** (el mismo del que sale `h_oro`), y las celdas finas interpolan `w`. Así sigue cumpliéndose el requisito de la nota anterior: el resultado no depende de la malla.
- Como el viento es fijo por franja de latitud, `w` es un campo casi estático: se precalcula en la génesis (después del prerrodaje de erosión) y se recalcula en la cadencia de la erosión (`M_ero`), o solo cuando `h_oro` cambie por encima de un umbral.
- Es una función pura de (`h_oro`, campo de viento, máscara de mar), así que es determinista.

### Parámetros
| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `k_oro` | sensibilidad de la lluvia a la pendiente suavizada | 40 (calibrar) | — |
| `agotamiento_humedad` | activa el factor `w` | true | — |
| `H_v` | subida que deja al aire con ~37 % de su vapor | 2 500 | m |
| `L_recarga` | distancia de recarga de humedad sobre tierra | 1 000 | km |
| `L_traj` | longitud máxima del camino hacia atrás | 2 000 | km |
| `c_P` | factor global de conservación de la lluvia total | calculado en la génesis | — |

## 4. Validación

- **Escenario de juguete:** una cordillera norte-sur de 3 000 m con vientos del oeste.
  - El pie de barlovento recibe ~2-3 veces la lluvia de la llanura previa.
  - La llanura de sotavento, a 200-500 km, recibe ~0,2-0,4 veces.
  - Contraste barlovento/sotavento del orden de 5-20 veces. Referencia real: Andes del sur, con 2 000-5 000 mm en Chile frente a ~200 mm en la estepa patagónica.
  - Cálculo de comprobación: `w ≈ e^(−3000/2500) ≈ 0,30`, y con `m_oro ≈ 2,5` el contraste es ≈ 8.
- **Mundo plano:** `w = 1` y `m_oro = 1` en todas partes, con salida idéntica a la de antes de esta nota.
- **Invariancia de malla:** la prueba fina / gruesa→fina→gruesa / gruesa sigue pasando para la lluvia.
- **Biomas:** aparecen desiertos o estepas a sotavento de las cordilleras grandes en latitudes medias. Es la comprobación del objetivo §1 de la visión madre.
- **Total global:** la lluvia sobre tierra queda dentro de ±5 % (o se ajusta con `c_P`).
- **No-regresión:** con `k_oro = 0,5` y `agotamiento_humedad = false`, la salida es idéntica byte a byte a la del commit bb6a8c0.

## 5. Efecto aguas abajo (para tenerlo en cuenta)
Cambia la distribución de la lluvia, así que cambian la humedad del suelo, los biomas, la productividad vegetal (modelo Miami) y, con ella, el forraje y la capacidad de carga de los agentes. Hay que esperar que la densidad de agentes caiga a sotavento de las cordilleras. Es el comportamiento buscado.

---

## 6. Implementación y medidas (Claude Code, 2026-11-05)

### 6.1 Qué se ha implementado

**Interruptores** (`[climate]` de `config/terreno.toml`; cada cambio se apaga por separado):

| Interruptor | Por defecto | Apagado | Qué hace |
|---|---|---|---|
| `k_oro` | 150 | 0,5 | Parte A: sensibilidad de la lluvia a la pendiente suavizada (§6.2) |
| `conservar_lluvia` (+ `tol_lluvia = 0,05`) | `true` | `false` | Parte A: factor global `c_P` |
| `agotamiento_humedad` (+ `h_v_m`, `l_recarga_km`, `l_traj_km`) | `true` | `false` | Parte B: factor `w` |
| `mar_plano` | `true` | `false` | Decisión técnica nueva (§6.6, hallazgo 5): el aire ve el mar plano |

`config/experiments/terreno_sin_sombra.toml` lleva los cuatro apagados y reproduce la salida de antes de la nota.

**Parte A.**
- `k_oro` sigue siendo un parámetro. El valor por defecto sale de la calibración por objetivos (§6.2).
- `c_P` (`humedad::factor_conservacion`) se calcula una vez, al acabar la génesis, y queda fijo durante la corrida:
  - referencia: la lluvia media anual sobre tierra **sin efecto orográfico ni agotamiento** (franja de latitud y distancia a la costa). La orografía de antes de la nota (`k_oro = 0,5`) la dejaba a un 0,3 %, así que es "la anterior";
  - si la lluvia actual se desvía de ella más de `tol_lluvia`, `c_P = referencia / actual`; si no, `c_P = 1`;
  - solo multiplica la lluvia de la **tierra**: ni la orografía (con el mar plano) ni el agotamiento tocan el mar.
- La desviación y `c_P` van a los metadatos de la salida (`run.lluvia`).

**Parte B** (módulo nuevo `crates/abm-terreno/src/humedad.rs`):
- **Anillos.** El viento del modelo es zonal: solo sopla hacia el este o el oeste, según la franja. Por eso el camino del viento hacia atrás de un punto es su propio paralelo. `Λ` se calcula sobre anillos: paralelos a paso `d_oro` en latitud, con puntos a paso `d_oro` a lo largo de cada uno (816 000 puntos con 25 km). Es exacto con este viento. Si algún día el viento tiene componente norte-sur, habrá que cambiarlos por caminos de verdad.
- **Relieve de cada punto.** El de referencia de la nota de lluvia orográfica: el relieve de nacimiento al nivel `L_oro` más la erosión vista a ese nivel. Así `Λ` no depende de la malla.
- **Mar.** Un punto es mar si su relieve de referencia queda bajo el nivel del mar (no se usa la máscara de la malla, que cambia al afinar). La suma se corta ahí, y el tramo de la costa cuenta la subida desde el nivel del mar.
- **Distancia de cada tramo** `s_j`: la de su punto medio, `(j + ½)·paso`.
- **Celdas.**
  - La de nivel `L_oro` o más fina toma `w` interpolado en su centro, solo con los puntos de tierra de los anillos. Mezclar el mar, con `w = 1`, daría a la tierra de la costa humedad del mar.
  - La más gruesa toma la media de `w` en los centros de sus descendientes de nivel `L_oro` (8×8 como mucho). Lo que a esa escala es mar cuenta con `w = 1`.
  - El mar tiene `w = 1`.
- **Cuándo se calcula.** Al acabar la génesis, como dice la nota ("después del prerrodaje de erosión"): durante la génesis geológica, `w = 1`. Después se rehace en cada paso de erosión y en cada cambio de malla, porque las hojas nuevas necesitan su `w`.
- **Coste.** Construir los anillos lleva ~0,3 s y rehacer `w`, 0,05–0,2 s (más cuanto más erosión hay que mirar). La génesis tarda lo mismo que antes (~43 s la primera vez). Con la caché, la corrida de un año pasa de 8,4 a 10,4 s: hay un 3 % más de celdas y `w` se rehace en cada cambio de malla.
- **Caché.** Pasa al formato `ABMGEN03` y guarda `w` y `c_P`. Los anillos no se guardan: se rehacen al primer uso.
- **Volcado por celda.** Con `run.dump_climate = true`, el volcado añade el factor orográfico, `w` y la lluvia media anual de cada celda.

### 6.2 Calibración de `k_oro` (Parte A)

Objetivo de la nota (§2): en las laderas de barlovento de las cordilleras grandes (relieve suavizado > 2 000 m), `m_oro` medio entre 2 y 3. Se midió con dos definiciones de "ladera de barlovento", con todo lo demás encendido:
- **(i)** celdas de tierra a más de 2 000 m cuyo terreno sube en la dirección del viento;
- **(ii)** celdas que suben con el viento, a más de 300 m, con una cresta de más de 2 000 m a menos de 150 km por delante.

Medias por área.

| `k_oro` | `m_oro` medio (i) | (ii) | Tierra con `m_oro` < 0,6 / > 1,5 | Contraste barlovento / sotavento (lat. medias) |
|---|---|---|---|---|
| 0,5 (antes) | 1,00 | 1,00 | 0 % / 0 % | 0,9 |
| 40 (propuesta) | 1,37 | 1,34 | 1,4 % / 0,9 % | 1,1 |
| 80 | 1,72 | 1,67 | 3,6 % / 2,8 % | 1,5 |
| 120 | 2,02 | 1,96 | 5,5 % / 4,3 % | 1,9 |
| **150 (elegido)** | **2,18** | **2,13** | **6,7 % / 5,3 %** | **2,2** |
| 160 | 2,23 | 2,18 | 7,1 % / 5,6 % | 2,3 |
| 240 | 2,46 | 2,44 | 9,8 % / 7,8 % | 2,9 |

- **Por qué hace falta más de 40.** A la escala de 25 km, las montañas de este planeta son más suaves que las reales. En las laderas de barlovento de más de 2 000 m suben de media ~9 m por km (0,009), frente a los 40 m por km (0,04) del razonamiento de la nota. Con 40, la media se queda en 1,37.
- Entre 120 y 250 se cumple el objetivo con las dos definiciones; **150** lo cumple con margen (2,13–2,18).
- **La lluvia total sobre tierra** (con `k_oro` solo, sin agotamiento) sube un +3,0 % con 150. Cabe en el ±5 % de la nota: el tope inferior del factor (0,2) recorta más el sotavento que el superior (3) el barlovento.
- Percentiles 1 / 10 / 50 / 90 / 99 del factor en tierra con 150: 0,20 / 0,76 / 1,00 / 1,22 / 2,92.

**Pendiente del responsable:** el valor final (decisión S1).

### 6.3 Escenario de juguete (§4)

Test `humedad::tests::escenario_de_juguete_de_la_nota`. El perfil:
- mar;
- 500 km de llanura a 200 m;
- ladera de barlovento de 3 000 m en 75 km;
- bajada en 75 km a una meseta de 500 m;
- vientos del oeste.

Lluvia relativa a la de la llanura de barlovento:

| | Pie de barlovento → cresta | Sotavento a 200 / 300 / 400 / 500 km | Contraste |
|---|---|---|---|
| Objetivo de la nota | ~2–3 | ~0,2–0,4 | 5–20 |
| `k_oro = 150` | 3,04 · 2,11 · 1,47 · cresta 0,45 | 0,42 · 0,46 · 0,50 · 0,54 | 7,2 |
| `k_oro = 40` | 1,77 · 1,75 · 1,22 · cresta 0,37 | igual | 4,2 |

- **Al subir la ladera llueve menos**, porque el aire ya se secó: `Λ` incluye la subida del propio tramo. Con pasos de 25 km, una ladera de 75 km tiene solo tres puntos.
- **El sotavento queda por encima del objetivo** porque el aire se recarga: a 200–500 km de la cresta, las subidas pesan `e^(−0,2…−0,5)`.

Sensibilidad del sotavento y del contraste (`k_oro = 150`):

| `H_v` | `L_recarga` | Sotavento 200 / 500 km | Contraste |
|---|---|---|---|
| **2 500 m** | **1 000 km** | **0,42 / 0,54** | **7,2** |
| 2 000 m | 1 000 km | 0,34 / 0,46 | 8,9 |
| 1 500 m | 1 000 km | 0,24 / 0,36 | 12,8 |
| 2 500 m | 2 000 km | 0,38 / 0,44 | 8,0 |
| 2 500 m | 3 000 km | 0,36 / 0,40 | 8,4 |

Con `k_oro = 40`, no contar en `Λ` la subida del último tramo (el aire que *llega* a la celda) subiría el pie de barlovento de 1,77 a 2,53. Con 150 da igual, porque el factor ya está en su tope.

### 6.4 Efecto de cada cambio sobre el planeta

Config por defecto (un año) frente a la salida de referencia de bb6a8c0, en tres tablas.

**Variantes:**
- T1: solo `k_oro = 150`;
- T2: solo agotamiento;
- T3: agotamiento + `c_P`;
- T4: `k_oro` + mar plano;
- T5: todo (por defecto);
- T6: todo menos el mar plano;
- T7: todo con `k_oro = 40`.

**Lluvia y factores**

| | Lluvia tierra | Lluvia mar | `c_P` (desvío) | `m_oro` tierra p1 / p99 | Tierra con `w` < 0,5 |
|---|---|---|---|---|---|
| Ref. | 1,356·10¹⁴ m³ | — | — | 0,99 / 1,01 | 0 % |
| T1 | +2,95 % | +2,70 % | — | 0,20 / 2,92 | 0 % |
| T2 | −15,98 % | +0,05 % | 1 (−16,0 %) | 0,99 / 1,01 | 12,2 % |
| T3 | −0,08 % | −0,01 % | 1,190 (−16,0 %) | 0,99 / 1,01 | 12,2 % |
| T4 | +3,02 % | +0,06 % | — | 0,20 / 2,92 | 0 % |
| **T5** | **+0,80 %** | **+0,07 %** | **1,180 (−15,2 %)** | **0,20 / 2,92** | **12,5 %** |
| T6 | +0,67 % | +2,79 % | 1,180 (−15,2 %) | 0,20 / 2,92 | 12,5 % |
| T7 | +0,12 % | +0,02 % | 1,189 (−15,9 %) | 0,53 / 1,49 | 12,1 % |

**Biomas, lagos y capacidad de carga**

| | Desierto | Pradera | Sabana | Bosque templado | Selva | Lagos | PPN media (Miami) |
|---|---|---|---|---|---|---|---|
| Ref. | 5,60 % | 3,35 % | 5,10 % | 6,83 % | 2,07 % | 51 | 727,9 g/m²/año |
| T1 | 6,14 | 3,17 | 5,02 | 6,19 | 2,40 | 56 | −1,3 % |
| T2 | 7,75 | 2,93 | 5,37 | 5,31 | 1,58 | 54 | −11,1 % |
| T3 | 5,69 | 2,96 | 5,99 | 5,77 | 2,55 | 52 | −2,4 % |
| T4 | 6,13 | 3,18 | 4,99 | 6,21 | 2,44 | 56 | −1,2 % |
| **T5** | **5,95** | **3,02** | **5,59** | **5,61** | **2,79** | **57** | **−2,8 %** |
| T6 | 6,00 | 3,01 | 5,57 | 5,57 | 2,74 | 57 | −3,0 % |
| T7 | 5,71 | 2,98 | 5,85 | 5,79 | 2,61 | 42 | −2,4 % |

Los biomas van en % de la superficie del planeta.

**Sotavento de las cordilleras en latitudes medias**

Tierra a 30–60° de latitud, por debajo de 2 000 m, con una cresta de más de 2 000 m a 200–500 km a barlovento (2,05·10⁶ km²):

| | Lluvia | `w` | Desierto | Pradera | Taiga | Tundra |
|---|---|---|---|---|---|---|
| Ref. | 688 mm | 1,00 | 7,6 % | 8,2 % | 19,5 % | 43,0 % |
| T1 | 496 mm | 1,00 | 11,7 % | 4,0 % | 14,1 % | 47,0 % |
| T2 | 285 mm | 0,39 | 15,8 % | 0 % | 10,6 % | 51,6 % |
| **T5** | **231 mm** | **0,37** | **15,8 %** | **0,1 %** | **7,9 %** | **53,0 %** |

En el resto de la franja, con T5 llueve 864 mm (803 en la referencia) y el desierto baja del 2,4 % al 2,1 %.

**Más detalles:**
- **La capacidad de carga** del herbívoro de 70 kg, calculada con el modelo Miami sobre las medias anuales de cada celda, baja lo mismo que la PPN (−2,9 % con T5). La PPN se satura con la lluvia: repartir la misma agua con más contraste da menos productividad total.
- **Malla:** 210 078 celdas al final del año (204 435 en la referencia), porque hay más contrastes de humedad que afinar.
- **Ríos:** 199 879 km (201 730 en la referencia).

### 6.5 Validación (§4) y pruebas

**No regresión.** Con los cuatro interruptores apagados, la salida es **idéntica byte a byte** a la del binario de ece0868 (el terreno no cambió desde bb6a8c0):
- el JSON, con erosión y sin ella;
- el volcado por celda.

Se repite con `herramientas/no_regresion.sh bb6a8c0 terreno config/terreno.toml config/experiments/terreno_sin_sombra.toml`.

**Mundo plano.** Test `en_un_mundo_plano_la_sombra_pluviometrica_no_cambia_nada`, con toda la tierra a 0 m:
- `w = 1` y `c_P = 1` exactos;
- la corrida es idéntica a la de `k_oro = 0`.

Frente al código de antes de la nota no puede salir idéntica: allí el fondo marino contaba como relieve y movía (poco, con 0,5) la lluvia de la costa.

**Invariancia de malla de `w`.** En `la_lluvia_no_depende_de_la_resolucion`, en la tierra que ya existía, `w` cambia un −2·10⁻⁵ al afinar.

**Lluvia de la zona al afinar.** Cara 0, resolución real:

| | 208 → 104 km | 52 → 26 km (la del Foco) |
|---|---|---|
| Antes de la nota | +0,03 % | −0,20 % |
| `k_oro = 150` | +0,38 % | +0,15 % |
| Todo encendido | +0,58 % | +0,00 % |

- Con el agotamiento, la tierra que aparece en la costa al afinar es más húmeda que un interior ahora más seco. Antes, ese +1,2 % de la costa se anulaba con el −1,2 % del interior.
- El test admite < 0,5 % sin agotamiento y < 1 % con él.

**Independencia del camino.** Test `independencia_del_camino_con_dinamica`, con celdas de 834/417 km; (b)/(c) · (a)/(b) · (a)/(c):

| | Lluvia | Erosionado |
|---|---|---|
| Antes de la nota | 1,4 · 2,3 · 3,6 % | 1,1 · 1,9 · 3,0 % |
| Todo con `k_oro = 40` | 1,0 · 2,0 · 3,0 % | 0,7 · 3,2 · 3,9 % |
| **Todo con `k_oro = 150`** | **2,3 · 4,5 · 6,7 %** | **0,2 · 2,0 · 2,1 %** |

- A esa escala todas las celdas son más gruesas que `d_oro` y usan su propia pendiente, que cambia con la resolución (regla de la nota de lluvia orográfica §3). Con un `k_oro` grande, eso pesa.
- Tolerancias ajustadas a estas cifras, con margen.

**Caché.** `la_cache_reproduce_la_genesis_bit_a_bit` compara ahora también `w` y `c_P`, y cruza un paso de LOD para que el mundo leído de la caché reconstruya los anillos.

**Comprobaciones de CLAUDE §10:** `fmt`, `clippy --workspace --all-targets -D warnings` y `cargo test --workspace` en verde.

### 6.6 Hallazgos

1. **`k_oro = 40` se queda corto en este planeta.** El relieve sintético es ~4 veces más suave que el real a 25 km. Para el objetivo de la nota hace falta ~150 (§6.2).
2. **El agotamiento también seca el barlovento.**
   - `Λ` suma todas las subidas del camino, incluida la de la propia ladera. Por eso, al subir una cordillera, cada punto recibe menos que el anterior.
   - En el planeta, las laderas de barlovento de las cordilleras grandes reciben 477 mm, frente a 563 en la referencia y 1 303 con `k_oro = 150` sin agotamiento.
   - El contraste barlovento/sotavento en latitudes medias queda en 2,2. La nota esperaba 5–20, y el juguete da 7,2.
3. **El sotavento a 200–500 km recibe 0,42–0,54 de la lluvia previa** (objetivo de la nota: 0,2–0,4), porque el aire se recarga sobre tierra (`L_recarga = 1 000 km`). Se llega al objetivo con `H_v ≈ 1 500 m` o `L_recarga ≈ 3 000 km` (§6.3).
4. **Sin `c_P`, el agotamiento quita el 16 % de la lluvia de la tierra.** `c_P = 1,18` la recupera. A cambio, la tierra que no está a sotavento recibe un 18 % más (selva +35 %).
5. **El fondo marino contaba como relieve** (la nota no lo trata).
   - Con `k_oro = 150`, la lluvia del 86 % del océano cambiaba más de un 10 %, y la del mar en total un +2,7 %.
   - Pasaba también en la costa: la pendiente se medía desde el fondo marino.
   - `mar_plano` lo corrige: sobre el mar `m_oro = 1`, y en la costa la pendiente se mide desde el nivel del mar. Sobre la tierra apenas cambia nada (+0,13 % de lluvia).
6. **Aparecen desiertos a sotavento**, el objetivo de la visión madre (§1):
   - en latitudes medias, tras las cordilleras grandes, el desierto pasa del 7,6 % al 15,8 % y la pradera casi desaparece;
   - esa franja es fría (el planeta tiene 9,4 °C de media), así que buena parte de ese sotavento es tundra, que no cambia de bioma con la lluvia.
7. **Capacidad de carga del planeta: −2,9 %.** Habrá menos agentes a sotavento y más en las costas y barloventos húmedos.
8. **La erosión y la lluvia dependen algo más de la resolución a escalas de cientos de km** (§6.5). A la escala del Foco (52 → 26 km), la lluvia no cambia (0,00 %).

### 6.7 Decisiones para el responsable

| # | Decisión | Opción aplicada (más fiel a la nota) | Datos |
|---|---|---|---|
| S1 | Valor de `k_oro` | **150**, calibrado por el objetivo de la nota (40 no lo cumple) | §6.2 |
| S2 | ¿Bajar `H_v` o subir `L_recarga` para que el sotavento quede en 0,2–0,4? | **Valores de la nota** (2 500 m, 1 000 km): sotavento 0,42–0,54 | §6.3 |
| S3 | ¿Contar en `Λ` la subida del último tramo, que seca la propia ladera? | **Sí**, como la fórmula de la nota | §6.3, §6.6.2 |
| S4 | `mar_plano`, que no está en la nota | **Encendido** (decisión técnica de Claude Code, reversible) | §6.6.5 |
| S5 | `c_P`: referencia y alcance | **Sin orografía ni agotamiento y solo sobre la tierra** | §6.1 |
