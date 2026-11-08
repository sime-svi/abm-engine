# Isostasia con litosfera, erosión recalibrada y diagnóstico del supercontinente

2026-11-08, cuarta ronda. Rama `terreno-demografia-placas`. Nota: `docs/design/terreno/nota_isostasia_litosfera.md`. Diseño: `incremento-4-diseno.md` §72–§73.

## Resumen

- **R4 está en el motor**, con la edad tectonotérmica de cada celda (envejece, se reinicia con rifts, arcos, choques y delaminación, y se mezcla por volumen). Las edades de partida son las de la Tierra de hoy menos 500 Myr, con las más viejas en el interior de cada continente.
- **El relieve mejora mucho** (5 semillas, frente a la última tabla de cierre, con la isostasia vieja):

  | | Antes | Ahora | Tierra |
  |---|---|---|---|
  | Altura media de la tierra | 1 136 m | **781 m** | 671 |
  | Tierra por encima de 2 000 m | 15,0 % | **7,0 %** | 5,3 % |
  | Tierra por debajo de 200 m | 8,1 % | **16,2 %** | 28,3 % |
  | Nivel del mar | −98 m | **−140 m** | ≥ −150 |

- **La erosión**, recalibrada con la isostasia nueva: `E_1` pasa de 0,0075 a 0,02 (India–Asia relaja en 159 Myr) y `n_ero` sigue en 2,5. La exportación al mar sigue en 4,4 AU, frente a ~1,8. No se puede calibrar con ella: a la larga se exporta lo que añaden los arcos y los choques.
- **No cumplen:**
  - **los márgenes apenas se hunden**: 0,38 km en 100 Myr, frente a 2–3 km (McKenzie 1978). La causa está en la regla, y hay que decidir (§6);
  - la plataforma (10 km, frente a 43) y la tierra baja;
  - la clase de 100–500 Myr (697 m, frente a 326 ± 300).
- **Edad del fondo:** 84 Myr de media, dentro del objetivo de ≤ 88. **No cuesta nivel del mar:** con las edades de la Tierra, el mar estaría 57 m más bajo, no más alto (§7).
- **Supercontinente** (solo diagnóstico):
  - los océanos de rift se abren (~13 por semilla, hasta ~3 900 km) y uno de sus márgenes se activa a los ~70 Myr, **pero casi nunca se cierran** (0,8 por semilla);
  - los continentes no se acercan entre sí: su distancia crece despacio (+1,1 km/Myr).
- **Tiempo: 283 s, se pasa del presupuesto de 275.** Pero la versión anterior, medida hoy igual, tarda 288,6 s: la máquina va hoy más lenta que cuando se midieron 256 s. Con R4 el planeta tarda 5,6 s menos (§10).
- **Determinismo:** una semilla salió distinta en la novena cifra de la génesis porque el binario se recompiló mientras corría. Con el binario quieto, 8 de 8 corridas son idénticas (§11).

## 1. La comprobación del §0

Se repite el ajuste de R4 sin las celdas donde la regla de Lamb et al. (2020) deja más de 300 m de residuo. Queda el 33 % de la tierra (5 735 de 19 710 celdas de 1°). Herramienta: `herramientas/analisis/nota_isostasia_s0.py`; salida: `config/referencias/tierra_isostasia_s0.toml`.

| Ajuste | ρ_u | ρ_l | z_t | Δρ_ma | Δρ arcaico | L_min | τ_L | H_0 | Error medio: toda la tierra / celdas del ajuste |
|---|---|---|---|---|---|---|---|---|---|
| Estudio | 2 750 | 3 000 | 15 | 20 | 10 | 40 | 100 | 3 005 | 432 / 432 m |
| §0, sin la topografía dinámica | 2 750 | 3 000 | 15 | 20 | 15 | 50 | 100 | 3 051 | 435 / 288 m |
| **§0, con el sedimento como corteza (el motor)** | 2 750 | 3 000 | 15 | **25** | **15** | **50** | 100 | 3 077 | 449 / 285 m |

- **Salen del borde:** `Δρ_ma` (25), el manto arcaico (15) y `L_min` (50). Se usan esos valores.
- **Siguen en el borde:** las densidades de la corteza (2 750 arriba y 3 000 abajo) y la profundidad del paso entre ellas (15 km). Se anota en TREE.md como limitación conocida.
- Validación cruzada entre mitades: 468 m en América con la Antártida y 444 m en Eurasia, África y Australia.

## 2. Lo implementado

- **La regla** (`isostasia.rs`):
  - la corteza (con su sedimento) pesa 2 750 kg/m³ hasta 15 km y 3 000 debajo;
  - el manto litosférico pesa 25 kg/m³ más que la astenosfera (15 en el arcaico, de más de 2 500 Myr);
  - su grosor sale de la edad: `L = 50 + (93,6 + 0,04·t − 50)·(1 − e^(−t/100))` km;
  - bajo el mar, el agua también pesa.
- **Lo que cambia con ella:**
  - lo que baja la superficie al erosionar (0,086 km por km en una corteza gruesa, antes 0,15);
  - el sedimento que cabe en una cuenca;
  - el levantamiento de la génesis geológica.
  - Se retiran `h_nivel_km` y la regla vieja.
- **La edad tectonotérmica** (`Corteza::edad_tt`): envejece al principio de cada paso.
  - Vuelve a 0 en la franja estirada de un rift al romper, en las celdas de arco que reciben corteza, en la franja que engruesa un choque y donde la delaminación quita más de 1 km en el paso.
  - Se mezcla por volumen al juntar corteza y viaja con el flujo lateral.
- **Medidas nuevas por paso:** la tierra por clase de edad, los márgenes por tiempo desde la rotura y el área reiniciada por causa. También un diagnóstico del supercontinente cada 25 Myr, que solo mide (§8).
- **Pruebas nuevas:**
  - la regla da los números de la herramienta del estudio (siete columnas, a 0,1 m);
  - un margen que envejece se hunde lo que dice la regla;
  - los márgenes de un rift cuentan su edad desde la rotura;
  - la mezcla de edades no toca ningún volumen;
  - las edades de partida son deterministas y más viejas por dentro.
  - Todas en verde.

## 3. Recalibración de `E_1` y `n_ero` (5 semillas)

Para cada `n_ero`, el `E_1` que relaja el orógeno India–Asia en ~160–175 Myr (la prueba pide 100–200):

| | R4 con la erosión vieja | n 2, E_1 0,03 | **n 2,5, E_1 0,02** | n 3, E_1 0,01 |
|---|---|---|---|---|
| Relajación (Myr) | 411 | 166 | **159** | 174 |
| Altura media de la tierra (m; 671) | 1 192 | 736 | **781** | 948 |
| Tierra < 200 m (0,283) | 0,070 | 0,189 | **0,162** | 0,128 |
| Tierra > 2 000 m (0,053) | 0,161 | 0,070 | **0,070** | 0,101 |
| Pendiente (0,0052) | 0,0163 | 0,0113 | **0,0119** | 0,0134 |
| Plataforma (km; 43) | 5,4 | 10,5 | **9,7** | 8,6 |
| Nivel del mar (m; ≥ −150) | −463 | −172 | **−140** | −229 |
| Sedimento al mar (AU; ≈1,8) | 2,8 | 4,7 | **4,4** | 3,7 |
| Cambio del volumen continental (±10 %) | −12,8 % | −11,7 % | **−7,4 %** | −8,8 % |
| Corteza máxima al final (≤ 80 km) | 80 | 85 | **78** | 79 |
| Clases de edad dentro de ±300 m | 3 de 5 | 5 de 5 | **4 de 5** | 3 de 5 |

- **Sin recalibrar, R4 deja la tierra alta.** Un engrosamiento levanta la tierra algo más de la mitad que antes (0,086 frente a 0,15 km por km), así que la erosión la gasta más despacio y la corteza se queda gruesa.
- **Se elige n 2,5 con `E_1` 0,02.**
  - n 2 acerca un poco más el relieve: la altura media entra en 671 ± 100 y las cinco clases cuadran;
  - pero deja más corteza máxima y más pérdida de volumen, y el mar a −172 m;
  - las diferencias entre n 2 y n 2,5 quedan dentro de la dispersión entre semillas (por ejemplo, la altura media 736 ± 104 frente a 781 ± 67 m). Por eso se queda el centro del rango.
- **La exportación al mar no se calibra:** a la larga se exporta lo que añaden los arcos (4,5 AU en el modelo) y los choques. Ajustarla a ~1,8 sería forzar `E_1` contra otra irrealidad (CLAUDE §2.8; §63 del diseño).
  - n 3 exporta menos (3,7 AU), pero con la tierra a 948 m y un 10 % por encima de 2 000 m.

## 4. Tabla de cierre (5 semillas, planeta entero; `herramientas/planeta_semillas.py`)

| | Modelo | Tierra u objetivo | |
|---|---|---|---|
| Tierra emergida | 0,320 ± 0,012 | 0,284 | — |
| Altura media de la tierra | 781 ± 67 m | 671 ± 100 (nota §10) | ✗ (por 10 m) |
| Tierra por debajo de 200 m | 0,162 ± 0,065 | 0,283; no peor que R4 con la Tierra (0,222) | ✗ |
| Tierra por encima de 2 000 m | 0,070 ± 0,011 | 0,053; no peor que R4 con la Tierra (0,031) | ✓ |
| Pendiente media (±25 km) | 0,0119 | 0,0052 | ✗ (×2,3) |
| Plataforma | 9,7 ± 1,4 km | 42,8; no peor que R4 con la Tierra (33,0) | ✗ |
| Nivel del mar | −140 ± 60 m | ≥ −150 | ✓ |
| Profundidad media del océano | 3 881 ± 67 m | 3 667 ± 150 | ✗ |
| Corteza máxima al final / en toda la deriva | 78 / 117 km | 70–80 | ✓ / ✗ |
| Edad media del fondo; más de 180 Myr | 84 ± 3 Myr; 12 % | ≤ 88 (64,2); < 1 % | ✓ (✗ con 64) / ✗ |
| `f_cont`; cambio de volumen | 0,326; −7,4 % | 0,40; ±10 % | ✗ / ✓ |
| Velocidad de las placas | 4,67 cm/año | 4,39 | ✓ |
| Sedimento al mar | 4,4 ± 1,1 AU | ≈1,8 | ✗ |
| Longitud de fosa; área que se hunde | 85 200 km; 4,20 km²/año | 51 000; 2,7–3,4 | ✗ |
| Placas de ≥ 1 %; las 15 mayores | 18,8 ± 2,6; 0,91 | 13; 0,958 | ✓ (blando) |
| Choques; convergencia difusa | 17 700 km; 11 100 km | CCB 23 000; OCB 17 400 | ✓ |
| Márgenes pasivos: vida; edad media y máxima al final | 101 Myr; 197 y 619 Myr | 130–180; ~104 y ~180 | ✗ |
| Pérdida en choques | 1,19 AU | 0,4–0,7 | ✗ |
| Rupturas por 100 Myr | 3,9 | ~4–7 | ✗ (por poco) |
| Tiempo (cinco a la vez; solo, §10) | 373 s; 283 s | 275 | ✗ |
| **Altura media de la tierra por clase de edad** (0–100 / 100–500 / 500–1 600 / 1 600–2 500 / > 2 500 Myr) | 1 484 / **697** / 450 / 401 / 482 m | 1 247 / 326 / 491 / 421 / 543 ± 300 | 4 de 5 |
| Fracción de la tierra por clase | 0,41 / 0,31 / 0,09 / 0,09 / 0,10 | 0,28 / 0,17 / 0,17 / 0,23 / 0,16 | blando: demasiado joven |
| **Hundimiento del basamento de los márgenes en ~100 Myr** | 0,38 ± 0,25 km | 2–3 km | ✗ |

- **Las placas de ≥ 1 %** varían mucho entre ajustes de la erosión (13,8 con n 2; 18,8 con n 2,5; con 5 semillas, ±2,6–4,7). No se ha medido de dónde viene la diferencia con la ronda anterior (13,8).
- **Al final, el 71 % de la tierra tiene menos de 500 Myr** (en la Tierra, el 45 %).
  - Cada Myr, los arcos vuelven a poner a 0 unos 8 millones de km² de continente, y los choques unos 7. Son las mismas franjas, una y otra vez.
  - Una corteza tan joven encaja con el exceso de fosa (×1,7) y de choques activos, pero no se ha medido cuánto pone cada cosa.

## 5. La altura por clase de edad (§11.3 de la nota)

La tierra al final, por clase (media de 5 semillas; prueba `diagnostico_nivel_del_mar_y_edad_del_fondo`):

| Clase (Myr) | Fracción de la tierra | Altura media | Corteza con su sedimento | Litosfera | Tierra: altura / corteza con sedimento |
|---|---|---|---|---|---|
| 0–100 | 0,41 | 1 480 m | 43,4 km | 60 km | 1 247 m / 40,7 km |
| 100–500 | 0,31 | 691 m | 37,6 km | 99 km | 326 m / 37,2 km |
| 500–1 600 | 0,09 | 445 m | 38,3 km | 138 km | 491 m / 38,0 km |
| 1 600–2 500 | 0,09 | 398 m | 41,1 km | 175 km | 421 m / 39,6 km |
| > 2 500 | 0,10 | 476 m | 39,2 km | 215 km | 543 m / 39,5 km |

- **Las clases viejas sí cuadran**, a menos de 70 m de la Tierra. La pregunta del §11.3 no hace falta para ellas.
- **La que no cuadra es la de 100–500 Myr** (697 m en la tabla de cierre). Su corteza es como la de la Tierra (37,6 frente a 37,2 km), así que no viene del estado inicial. Vienen dos cosas:
  - **la regla:** con la corteza real, R4 ya pone esa clase 120–160 m alta (450–490 m frente a 326; comprobación del §0);
  - **el nivel del mar:** el del planeta queda a −140 m del dato, frente a +34 m en la prueba con la Tierra. Toda la tierra se mide ~170 m más alta.
- Toda la tierra de menos de 500 Myr nació o se reinició durante la deriva: las edades de partida tienen más de 500 Myr al final.

## 6. El hundimiento de los márgenes (no cumple)

El basamento de los márgenes estirados distales, sin su sedimento y con agua encima (lo que mide el «backstripping»), por tiempo desde la rotura:

| Myr desde la rotura | 0–20 | 20–50 | 50–80 | 80–120 | 120–200 | > 200 |
|---|---|---|---|---|---|---|
| Basamento (m bajo el mar) | 1 839 | 1 999 | 2 120 | 2 218 | 2 270 | 2 276 |
| Superficie, con el sedimento (m) | 1 713 | 1 624 | 1 533 | 1 473 | 1 251 | 880 |

- **El hundimiento inicial está bien:** ~1,8 km al romper, del orden de lo que da McKenzie (1978) con un estiramiento de ~3 (1,5–2 km).
- **El hundimiento térmico se queda corto:** 0,38 km en 100 Myr, frente a ~2 km de McKenzie con un estiramiento de 3 (2–3 km en la parte distal). La regla da 0,34 km: `Δρ_ma·ΔL/(ρ_a − ρ_w)` = 25 kg/m³ × 30 km de litosfera que crece (de 50 a 80 km) / 2 225.
- **La causa está en la regla, no en un parámetro.**
  - El manto litosférico de R4 es el continental, empobrecido: su ligereza química compensa casi todo su peso térmico, y por eso pesa solo 25 kg/m³ más que la astenosfera.
  - Bajo un rift, en cambio, el manto viejo se adelgaza y sube astenosfera caliente y fértil. Al enfriarse se contrae como el fondo oceánico: ~70 kg/m³ de media, desde casi 0 hasta ~100 km.
  - Subir `Δρ_ma` o bajar `L_min` sería forzar dos parámetros ajustados a la Tierra real (CLAUDE §2.8).
- **Opciones (decide el responsable):**
  1. **Un manto térmico para la litosfera nueva:** bajo una franja estirada, el manto que se forma tras la rotura se hunde con la curva del fondo oceánico, escalada por el estiramiento (`1 − 1/β`, McKenzie). Con el tiempo pasa al manto continental de R4.
  2. **R4 con dos capas de manto:** una térmica (fértil, que crece con √t) y otra química (empobrecida, que solo tienen los mantos viejos).
  3. Dejarlo así y anotarlo como limitación.
- **La superficie se hace menos honda con el tiempo:** el sedimento rellena más deprisa de lo que el margen se hunde. Aun así, la plataforma es estrecha (10 km, frente a 43); no se ha medido la relación entre las dos cosas.

## 7. Lo que cuesta en nivel del mar la edad del fondo

Prueba `diagnostico_nivel_del_mar_y_edad_del_fondo`: el nivel del mar al final de la deriva con las edades del fondo del modelo y con las de la Tierra (Seton et al. 2020), en los mismos cuantiles de área.

| Semilla | 42 | 1 | 2 | 3 | 4 | Media |
|---|---|---|---|---|---|---|
| Edad media del fondo (Myr) | 82,1 | 84,8 | 86,6 | 80,4 | 87,8 | 84,4 (Tierra 63,9) |
| Nivel del mar con sus edades (m) | −197 | −89 | −85 | −211 | −118 | −140 |
| Con las de la Tierra (m) | −296 | −178 | −132 | −228 | −151 | −197 |

- **La edad del fondo no cuesta nivel del mar: lo sube 57 m** (de 17 a 99 según la semilla).
- **Por qué, si el fondo del modelo es más viejo:** la profundidad del fondo crece con la edad cada vez más despacio, y a partir de ~100 Myr casi no cambia. Con la curva del modelo, por clases de edad:
  - el fondo de más de 160 Myr de más (14 %, frente al 2,6 % de la Tierra) **hunde** el fondo medio **75 m**;
  - pero el modelo tiene menos fondo de 60–160 Myr (30 %, frente al 44 %) y algo más de 0–20 Myr (24 %, frente al 21 %), que lo **suben 120 m**;
  - en neto, el fondo medio queda 46 m más somero que con las edades de la Tierra.
- **Con la isostasia nueva, el nivel del mar ya cumple** (−140 m, frente a −150).
- El fondo viejo de más, que dejan los artefactos de la deriva (§56 del diseño), hunde el fondo medio ~75 m. Si el resto de las edades fuera como en la Tierra, costaría del orden de 70 m de nivel del mar.

## 8. Diagnóstico del supercontinente (5 semillas, cada 25 Myr; sin cambiar el modelo)

Herramienta: `herramientas/analisis/supercontinente.py`, sobre las fotos que toma la deriva (`deriva/diagnostico.rs`, que solo mide: la tectónica sale idéntica con y sin él).

**1. El bloque mayor.**
- La mayor masa continental conexa reúne de media el 56 % del continente, y como mucho el 88 % (83–98 % según la semilla). Esta medida cruza los bordes de placa y cuenta también los arcos.
- El mayor bloque de una sola placa reúne solo el 22 % de media.
- Así que los continentes llegan a tocarse en masas grandes, pero siguen en placas distintas.

**2. ¿Se acercan o se alejan los bloques?**
- Las masas de al menos el 1 % del continente están, de media, a 11 700 km unas de otras.
- Con las velocidades de cada foto, esa distancia **crece despacio: +1,1 ± 1,1 km/Myr**.
- Se acercan el 47 % de los pares (pesados por su área), y en el 37 % de las fotos el conjunto se acerca.
- No hay una tendencia a juntarse: los continentes deambulan.

**3. Los océanos que abren los rifts** (los dos márgenes de cada rift ya roto):

| Por semilla | Media |
|---|---|
| Océanos que se abren más de 1 000 km | 13,4 |
| Anchura máxima | ~3 900 km |
| Con un margen activo (una fosa, convergencia difusa o choque cerca del 25 % de sus celdas) | 12,6, a los ~69 Myr de edad |
| Que se cierran (la mitad de los dos márgenes vuelve a menos de 300 km) | **0,8** (4 en total, en dos semillas, a los 191 y 362 Myr de edad media) |
| Cuyo margen desaparece sin cerrarse | 0,8 |
| Edad al final de los que siguen abiertos | 298 Myr |

- **Los océanos interiores se abren y casi nunca se cierran.**
  - La subducción llega a uno de sus márgenes a los ~70 Myr. En la Tierra, el Atlántico tiene ~180 Myr y solo tiene dos fosas pequeñas: las Antillas y Scotia.
  - Pero no los cierra: desde que se activa un margen, 34 de 56 océanos siguen ensanchándose (+1,6 km/Myr de mediana). Pasan de 2 460 km de media a un máximo de 4 210, y acaban en 3 300.
  - No se ha medido por qué; lo esperable es que su dorsal siga abriéndolos.

**4. El margen pasivo que pasa a ser activo.** Del margen pasivo de cada foto, 25 Myr después:
- sigue pasivo: 44 %;
- activo por invasión (una fosa o convergencia difusa cerca): 4,0 %;
- activo por un choque: 4,3 %;
- ya no es margen, sin borde convergente cerca: 42 %. Es sobre todo el vaivén de la costa en la malla: celdas que dejan de ser costa porque cambia la placa o la corteza de al lado;
- su muestra ya no está: 4,9 %.
- **Cada 25 Myr se activa el ~8 % del margen pasivo, mitad por invasión y mitad por choque.** Es un ~30 % en 100 Myr.

**5. Las fuerzas y las fosas.**
- Hay 6,6 placas con al menos el 5 % del continente. El 90 % tiene losa propia: tiran de ellas sus fosas.
- **Su motor:** el tirón de la losa, el 77 % de la potencia; el empuje de las dorsales, el 23 %.
- **Su freno:** el arrastre basal, el 79 %; los choques, el 21 %.
- Van a 3,1 cm/año, frente a 4,7 del conjunto de las placas.
- **Van hacia las fosas, pero poco:** el 59 % del continente se acerca a la fosa activa más cercana, a +0,63 cm/año de media.
  - Para el 51 % del continente, esa fosa es una donde se hunde su propia placa: el continente va pegado a la losa que tira de él, como la India.
  - Para el 38 %, su placa cabalga sobre esa fosa.
- **El modelo no tiene succión de la fosa sobre la placa de arriba**, ni arrastre del manto que mueva los continentes.
  - En la Tierra, las placas de arriba se mueven hacia sus fosas cuando la losa retrocede. Así se cierran los océanos y se juntan los continentes.
  - Aquí, un continente solo va hacia una fosa si su propia placa se hunde en ella.

## 9. Respuestas al §11 de la nota

1. **¿Cómo trata R4 el sedimento?**
   - En el estudio, R4 lo trata como relleno: no da flotación propia (`k_s` = 0) y solo ocupa el sitio del manto en `L − h − s`.
   - El motor no puede hacerlo así. Su reparto del sedimento calcula cuánto cabe en una cuenca hasta una cota, y con `k_s` = 0 cabría infinito.
   - **El motor cuenta el sedimento como corteza**, con la densidad de la capa en que está. Con la Tierra real se ajusta casi igual: 449 frente a 435 m de error medio en toda la tierra, y 285 frente a 288 en las celdas del ajuste. Y da el mejor relieve: 22 % de tierra por debajo de 200 m y 33 km de plataforma.
   - La regla del motor da los números de la herramienta del estudio con esta variante, a 0,1 m (prueba `la_isostasia_con_litosfera_da_los_numeros_del_estudio`).
2. **¿Qué cuesta calcular L y R4 en cada paso?** Poco. En la traza de la deriva de la semilla 42, la nueva y la anterior a la vez:
   - la erosión, +1,2 s (13,0 frente a 11,8);
   - las alturas para el nivel del mar, +0,8 s;
   - las métricas nuevas y las fotos del diagnóstico, ~2,9 s.
   - El reparto del sedimento, en cambio, baja 14 s (21,6 frente a 35,7). No se ha medido por qué; cambia todo el relieve.
3. **Si las clases viejas no cuadran, ¿viene del estado inicial o de la regla?** Cuadran: a menos de 70 m de la Tierra (§5). La que falla es la de 100–500 Myr: un poco por la regla (120–160 m, también con la Tierra real) y sobre todo por el nivel del mar, 170 m más bajo que con la Tierra real.

## 10. Tiempo (presupuesto 275 s)

| | Planeta solo (semilla 42) | Génesis | Deriva (traza, las dos a la vez) |
|---|---|---|---|
| Anterior (ba3609c), medido hoy | 288,6 s | 270,0 s | 188 s |
| **Ahora** | **283,0 s** | 265,0 s | 197 s |
| En la batería, con los agentes a la vez | 307,5 s | — | — |

- **Se pasa del presupuesto por 8 s (3 %), pero no por R4.**
  - La versión anterior, medida hoy en las mismas condiciones (la máquina en reposo), tarda 288,6 s, frente a los 256 s que se midieron en la ronda anterior. La máquina va hoy un ~13 % más lenta.
  - Con R4 y la erosión nueva, el planeta tarda 5,6 s menos que antes.
- **Qué crece en la deriva** (+9 s): los eventos de placas (+3,9 s), las métricas (+3,9 s; ~2,9 de las medidas nuevas y las fotos), la rasterización (+2,7 s), el flujo lateral (+2,4 s), la clasificación de bordes (+2,0 s), las fosas (+1,8 s) y sus procesos (+1,6 s). El reparto del sedimento baja 14 s.
- Casi todo lo que crece es por la realización del planeta: más fosa (85 200 km, frente a 71 800 en la ronda anterior) y más placas. Lo que añade R4 en sí son ~2 s.
- **Con la erosión vieja (0,0075), el flujo lateral se disparaba** (de 30 a 71 s): la corteza quedaba más gruesa y hacían falta 182 subpasos por paso, frente a 108. Con `E_1` 0,02 vuelve a lo de antes.
- **El resto de la génesis** (lo que no es la deriva: el prerrodaje y la génesis geológica) baja ~14 s, por diferencia: 68 s frente a 82. Hay menos tierra que erosionar: el 30 % del planeta frente al 35 %, y 3,7·10¹⁶ m³ erosionados frente a 4,8·10¹⁶.

## 11. Determinismo

- En la tanda final, la semilla 2 salió distinta de la corrida anterior en la novena cifra de la génesis geológica (el residuo de su erosión). La tectónica era idéntica.
- **La causa:** `target/release/terreno` se reescribió a las 15:46, con la tanda en marcha (15:44–15:52), y ese proceso cargó a medias el binario nuevo.
- **Con el binario quieto, todo es idéntico:**
  - la deriva de la semilla 2 da la misma huella de todo su estado, paso a paso, en 4 procesos (prueba `diagnostico_huella_de_la_deriva`);
  - el planeta entero de la semilla 2 da lo mismo, bit a bit, en 8 corridas.
- **Arreglo:** `planeta_semillas.py` corre ahora una copia del binario. En la tabla de cierre, esa semilla difiere en la cifra 12: no cambia nada.

## 12. Batería y verificación

- **Batería:**
  - 28 métricas del planeta salían de sus márgenes, todas por la realización nueva del planeta: relieve, lluvia, biomas, lagos, ríos y tectónica. Se actualizan con su nota (el valor nuevo y el cambio), como pide CLAUDE §10.5.
  - Los agentes, sin cambios.
  - Con los márgenes nuevos, todo queda dentro **salvo el tiempo del planeta**: 307,5 s en la batería, frente a 275. El presupuesto no se sube sin el responsable.
- **Pruebas:** todas en verde (`cargo test --workspace`); `cargo fmt` y `cargo clippy -D warnings`, limpios.
- **Una tolerancia ampliada, para que la revises:** en `la_losa_sigue_tirando_tras_el_choque_y_se_rompe_a_los_t_rot`, la rotura de la losa respecto al frente visible del choque (tres bordes de choque) pasa de ±3 a ±4 Myr.
  - Con `E_1` 0,02, ese frente se adelanta 4 Myr al inicio del choque del modelo (el primer solape continental); con 0,0075 pasaba igual con R4.
  - La rotura respecto al inicio del choque del modelo sigue a 22 ± 2 Myr.

## 13. Lo que decide el responsable

1. **Los márgenes** (§6): un manto térmico para la litosfera nueva bajo los rifts, R4 con dos capas de manto, o dejarlo como limitación. Puede pesar en la plataforma y en la tierra baja, pero no se ha medido.
2. **El tiempo:** 283 s, frente a 275. La versión anterior tarda hoy 288,6 s, así que no es cosa de R4: la máquina va más lenta. ¿Subir el presupuesto, o medirlo en otro momento? Si hiciera falta, tomar las medidas nuevas cada 5 Myr en vez de cada paso ahorraría ~2–3 s.
3. **El supercontinente** (§8): los océanos de rift no se cierran y los continentes no se juntan. En el modelo falta la succión de la fosa sobre la placa de arriba. ¿Nota propia?
4. **La corteza demasiado joven** (§4): el 71 % de la tierra tiene menos de 500 Myr, frente al 45 % en la Tierra. Encaja con el exceso de fosa (×1,7), que ya estaba pendiente; no se ha medido cuánto pone cada cosa.
5. **La tolerancia ampliada** de la prueba de la losa (§12): ±4 Myr en lugar de ±3.
6. **Fusionar con `main`**, cuando lo anterior esté decidido.
