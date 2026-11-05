# Nota de diseño: corrección de la dependencia de escala de la lluvia orográfica

**Sistema:** Terreno, clima (visión madre §6.2, factor `m_oro`)
**Estado:** Decisión cerrada por el director. Lista para implementar.
**Naturaleza:** Plano teórico. Sin código.

---

## 1. Problema medido

En la prueba de independencia del camino, la física base dependía de la resolución: con la zona fina llueve más (+4 % si está fina todo el rato; +3 % ya en el incremento 2). Con más detalle, las pendientes son más fuertes, y el factor orográfico sube la lluvia en barlovento más de lo que la baja en sotavento.

## 2. Por qué hay que corregirlo (no es solo un artefacto numérico)

1. **Física.** La lluvia orográfica no responde a la pendiente local de 1 km. Entre que el aire asciende, condensa y la gota cae, el viento lo desplaza del orden de 10-50 km. La lluvia "ve" el relieve suavizado a esa escala. Calcular `m_oro` con la pendiente fina es físicamente incorrecto.
2. **Bucle artificial con los agentes.** El zoom se activa donde hay actividad de agentes → allí la malla es fina → llueve más → más humedad → más vegetación y forraje → más agentes → más zoom. Es un sesgo que se realimenta justo en las zonas observadas y contaminaría los datos de salida.

## 3. Regla nueva

El factor orográfico se calcula con un **relieve de referencia suavizado a una escala fija** `d_oro`, independiente del nivel de la celda:

```
h_oro(celda) = Σ_j w_j · h_j / Σ_j w_j
   sobre las celdas j cuyo centro está a distancia geodésica ≤ d_oro del centro de la celda
   con w_j = A_j (área real de la celda j)

m_oro = clamp( 1 + k_oro · ( ∇h_oro · viento_unitario ) , m_oro_min , m_oro_max )
```

- `∇h_oro` es el gradiente del relieve suavizado, evaluado con la distancia de suavizado `d_oro` como paso (no con el lado `d` de la celda fina).
- Si la celda es más gruesa que `d_oro` (lado `d ≥ d_oro`), se usa su propia elevación: ya está suavizada a una escala mayor.

**Requisito de invariancia (el objetivo de la corrección):** `h_oro` debe dar el mismo valor en un punto dado del planeta tanto si la zona está en malla fina como en malla gruesa. La forma de garantizarlo es calcularlo siempre desde el mismo nivel de malla de referencia `L_oro`, el nivel cuyo lado es el más cercano a `d_oro`. Para celdas finas, `h_oro` se toma (interpolado) del nivel `L_oro`. Así no depende de que exista o no detalle fino. Es una función pura de (estado del nivel `L_oro`, posición), así que respeta las reglas de determinismo.

## 4. Qué cambia y qué NO

| Proceso | Escala de la pendiente | Motivo |
|---|---|---|
| Lluvia orográfica (`m_oro`) | **Referencia fija `d_oro`** | Su escala física (10-50 km) es mayor que la celda fina |
| Escorrentía (`c_esc`, vía `tan S`) | Local (celda) | El agua corre por la ladera real que pisa |
| Coste de movimiento | Local (celda) | El agente nota la pendiente local |
| Erosión (`K·A^m·S^n`) | Local, pero **vigilar** | Medir si su dependencia de la resolución es relevante (la prueba de independencia dio 0,3 %, aceptable de momento) |

## 5. Parámetros nuevos

| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `d_oro` | escala de suavizado del relieve para la lluvia orográfica | 25 | km |

## 6. Validación

- **Repetir la prueba de independencia del camino** (fina siempre / gruesa→fina→gruesa / gruesa siempre). Objetivo: la diferencia de lluvia en la zona baja del +4 % a ruido numérico (< 0,5 %).
- **Nueva línea base de no-regresión:** el test "erosión apagada = incremento 2" pasa a ser "**erosión y corrección orográfica apagadas** = incremento 2" (0 diferencias). Con la corrección encendida se fija una nueva salida de referencia.
- **Comprobación física:** la sombra pluviométrica sigue apareciendo a sotavento de las cordilleras grandes. Si desaparece, `k_oro` debe recalibrarse: con relieve suavizado los gradientes son menores, así que puede necesitar un valor mayor.

## 7. Pregunta abierta

- **Recalibrar `k_oro`.** Al suavizar el relieve, los gradientes bajan. Hay que comprobar que la lluvia media global y el contraste barlovento/sotavento siguen en rangos realistas. Si no, ajustar `k_oro` para recuperar los totales globales del incremento 2, y medirlo.

---

## 8. Implementación y medidas (Claude Code, 2026-11-05)

### 8.1 Qué se ha implementado

- **Parámetro** `climate.d_oro_km = 25` (`config/terreno.toml`). `d_oro_km = 0` apaga la corrección.
- **Nivel de referencia** `L_oro`: el nivel cuyo lado de celda (en el centro de una cara) es el más cercano a `d_oro`. Con la config por defecto (`n0 = 48`: lados de 208, 104, 52 y 26 km), `L_oro = 3`.
- **Relieve de referencia en un punto** = relieve de nacimiento del nivel `L_oro` evaluado en ese punto + erosión acumulada `e` vista a ese nivel.
  - El relieve de nacimiento es la misma función pura que da la elevación de una celda de nivel `L_oro`. Ya está suavizado a esa escala, porque cada nivel añade una octava de detalle.
  - La erosión es la de la hoja que contiene el punto. Si esa hoja es más fina que `L_oro`, se toma la media por área de los descendientes de su antepasado de nivel `L_oro`. Al subdividir, las hijas copian `e`; al colapsar, la madre toma la media. Por eso esa media no depende del detalle que haya por debajo de `L_oro`.
  - **Diferencia con §3:** en vez de promediar sobre un disco de radio `d_oro`, se usa el relieve del nivel `L_oro`, que ya está a esa escala. Es más barato y cumple igual el requisito de invariancia.
- **Gradiente** `∇h_oro`: diferencias centrales con paso `d_oro`, en la base este/norte de la celda.
- **Celdas más gruesas que la referencia** (nivel `< L_oro`): usan su propia pendiente, como dice §3.
- **Coste.** La parte del relieve de nacimiento se calcula una vez, al crear cada celda (columnas `oro0_e`/`oro0_n`). La de la erosión se rehace tras cada paso de erosión y cada cambio de malla. Las dos se guardan en la caché de la génesis (formato `ABMGEN02`; las cachés antiguas se descartan solas).

### 8.2 Hallazgo 1: el +4 % no venía de la orografía

Al repetir la medida para validar la corrección, la diferencia de lluvia **no cambió**. Con `k_oro = 0` (sin ningún efecto orográfico) también sale idéntica. El diagnóstico del incremento 3 (§16, hallazgo B) estaba equivocado; ya lleva una nota de corrección.

**Causa real: la distancia de humedad.** El factor `m_agua = exp(−d/1500 km)` medía `d` desde el centro de la celda de tierra hasta el **centro** de la celda de mar más cercana. A cada distancia le sobraba media celda de mar. Al afinar, esa media celda encoge, la costa "se acerca" y llueve más. Es el mismo bucle con los agentes que describe §2, y era más fuerte que el orográfico.

Medida: lluvia de un tick sobre la tierra de la cara 0, antes y después de afinar la cara un nivel.

| Resolución (gruesa → fina) | Distancia desde el centro del mar (hasta el incr. 3) | Igual, con `k_oro = 0` | Sin distancia (humedad uniforme) | **Distancia desde la costa (nuevo)** |
|---|---|---|---|---|
| 834 → 417 km | +16,9 % | +16,9 % | −0,4 % | **+2,6 %** |
| 417 → 208 km | +8,4 % | +8,4 % | −0,4 % | **+1,1 %** |
| 208 → 104 km (la resolución real) | +3,1 % | +3,1 % | −0,1 % | **−0,7 %** |

(Lluvia por unidad de área de tierra. Contando también el cambio de la tierra de la zona, el total a la resolución real pasa de +3,9 % a +0,03 %.)

**Corrección añadida** (decisión técnica de Claude Code, reversible por config): `climate.moisture_from_coast = true`. La distancia se mide desde la **línea de costa**: una celda de tierra que toca el mar empieza a la distancia de su centro al borde compartido, y desde ahí se propaga por la tierra como antes. Lo que queda es real o casi: al afinar cambia la propia costa (la tierra de la zona cambia un +0,8 %), y dentro de celdas enormes la exponencial se curva.

### 8.3 Validación (§6)

- **Prueba de independencia del camino** (cara 0, `n0 = 12`, celdas de 834 y 417 km). Lluvia sobre la tierra de la zona:

  | Comparación | Antes | Ahora |
  |---|---|---|
  | (b) gruesa→fina→gruesa frente a (c) gruesa siempre | 6,2 % | 1,4 % |
  | (a) fina siempre frente a (b) | 10,5 % | 2,3 % |
  | (a) frente a (c) | 16 % | 3,6 % |

  A esta resolución de juguete no se llega al 0,5 %: la costa de celdas de 834 km cambia mucho al afinar (la tierra de la zona, un 1,7 %).
- **A la resolución real** (test `la_lluvia_no_depende_de_la_resolucion`, 208 → 104 km): **+3,86 % → +0,03 %**. Objetivo (< 0,5 %) cumplido.
- **Invariancia del relieve de referencia** (test `el_relieve_orografico_no_depende_de_la_malla`): en los mismos puntos, con la zona un nivel más fina, el gradiente coincide a < 10⁻⁹ (relativo), con erosión de por medio.
- **No regresión:** con `d_oro_km = 0` y `moisture_from_coast = false`, la salida es **idéntica byte a byte** a la del commit anterior, con erosión y sin ella. Se comprobó con el binario anterior y la config por defecto.
- **Nueva salida de referencia** (config por defecto, correcciones encendidas, un año), frente a la anterior: lluvia total +0,40 %; tierra −0,01 %; ríos +0,4 %; lagos 51 → 51; desierto 5,70 → 5,60 % del planeta; selva 1,99 → 2,07 %; pradera 3,32 → 3,35 %; resto de biomas < 0,5 %. Las costas reciben algo más de humedad.

### 8.4 Hallazgo 2: con `k_oro = 0,5` no hay sombra pluviométrica

Con el valor de la visión madre, el factor orográfico de las celdas de tierra queda casi pegado a 1: el 98 % está entre 0,99 y 1,01. Ocurría igual antes de la corrección. Las pendientes a 26–200 km son del orden de 10⁻³–10⁻², y `0,5 · 0,01 = 0,005`. **La sombra pluviométrica nunca ha existido en el modelo.** Barrido (config por defecto, con las correcciones):

| `k_oro` | Factor orográfico en tierra (percentiles 1 / 10 / 50 / 90 / 99) | Tierra con < 0,6 / > 1,5 | Lluvia total | Desierto / selva / sabana / pradera (% del planeta) | Lagos |
|---|---|---|---|---|---|
| **0,5** (actual) | 0,99 / 1,00 / 1,00 / 1,00 / 1,01 | 0 % / 0 % | 3,531·10¹⁴ m³/año | 5,60 / 2,07 / 5,10 / 3,35 | 51 |
| 20 | 0,67 / 0,93 / 1,00 / 1,07 / 1,33 | 0,5 % / 0,2 % | +0,07 % | 5,62 / 2,11 / 5,17 / 3,35 | 62 |
| 40 | 0,36 / 0,86 / 1,00 / 1,14 / 1,66 | 3,2 % / 2,1 % | +0,14 % | 5,69 / 2,16 / 5,15 / 3,31 | 40 |

La lluvia total del planeta apenas se mueve (< 0,2 %), así que "recuperar los totales del incremento 2" (§7) se cumple con cualquier valor. Lo que cambia es el contraste local. **Propuesta:** `k_oro ≈ 40`. Daría un barlovento de ×1,5–3 y un sotavento de ×0,2–0,6 en las cordilleras grandes, en el rango de lo real. Es la pregunta abierta de §7: **la decide el responsable**. Mientras tanto, se mantiene `k_oro = 0,5`.

### 8.5 Hallazgo 3: la erosión sí depende de la resolución

Quitado el sesgo de lluvia, la erosión de la prueba de independencia difiere un 1,1 % en (b)/(c), un 1,9 % en (a)/(b) y un 3,0 % en (a)/(c). Antes difería un 0,2–0,3 %, pero solo porque el exceso de lluvia de la malla fina compensaba que, con la misma lluvia, erosiona menos. Con `n0 = 48`, (b)/(c) da un 3,3 % sobre todo el planeta. El relieve fino tiene más detalle (una octava más por nivel) y la potencia de arroyo responde a la pendiente y al área drenada de cada celda. Queda **vigilado** (§4), sin cambios. Las tolerancias de la prueba se ajustaron a estas cifras, que están documentadas en el propio test.

### 8.6 Qué cambia hoy y qué cambiará con los agentes

Con la config por defecto, `L_oro = 3 = l_max`. Las celdas de nivel 0 a 2 (208–52 km) son más gruesas que `d_oro` y conservan su pendiente. Solo las de nivel 3 (26 km) usan la referencia, y está a su misma escala. Por eso **la corrección orográfica apenas cambia nada hoy**: actuará cuando el Foco de los agentes afine por debajo de 26 km. La corrección de la distancia sí cambia ya los resultados (§8.3).

### 8.7 Otros detalles

- **Coste.** La génesis por defecto tarda unos 42 s en vez de 36 s: el relieve de referencia se rehace tras cada paso de erosión. Es una sola vez, porque después se lee de la caché. Si molestara, el muestreo de los cuatro puntos se puede precalcular por malla.
- **Fallo latente arreglado.** Al probar la caché salió que `serde_json`, por defecto, no lee los `f64` de forma exacta. El resumen de la génesis, que la caché guarda en JSON, podía volver con una diferencia de 1 ulp, y el test `la_cache_reproduce_la_genesis_bit_a_bit` lo detectó. Se activó la opción `float_roundtrip` de `serde_json` en el `Cargo.toml` del workspace. Solo afecta a la lectura: las salidas no cambian.

