# Informe: la Tierra real en la regla del modelo, para los continentes (2026-11-08, segunda ronda)

Encargo del responsable: primero, meter el grosor real de la corteza (CRUST1.0) y su sedimento en la
isostasia del modelo, con su nivel del mar, y comparar el relieve que sale con ETOPO sin hielo (altura
media de la tierra, por debajo de 200 m, por encima de 2 000 m, pendiente y plataforma). Medir también
el grosor de la corteza continental del planeta frente a CRUST1.0. «Si con la corteza real no sale el
relieve real, para y tráemelo.»

**No sale, así que la ronda se para aquí.** Quedan en espera, sin empezar, la nota de demografía de
placas v2 (rama D y D6), la nota de rifts v2, la tabla de cierre y la medida del tiempo (§7).

- Herramienta: `herramientas/analisis/tierra_corteza_regla.py` (~1,5 min con las cachés hechas).
- Resultados de la Tierra: `config/referencias/tierra_corteza.toml`.
- Rama `terreno-demografia-placas`. El motor no cambia.

## Resumen

- **Con la corteza real, la regla del modelo da una tierra el doble de alta que la de la Tierra.**
  - Altura media: 1 389 m, frente a 671.
  - Por encima de 2 000 m: el 19,7 % de la tierra, frente al 5,3 %.
  - Por debajo de 200 m: el 4,7 %, frente al 28,3 %.
  - La plataforma mide un tercio de la real: 17 km, frente a 43 (54 a la misma resolución). La plataforma real emerge: el 69 % queda en tierra.
  - La pendiente sale 1,65 veces la real a la misma resolución.
- **Con la corteza real sale una tierra más alta que la del propio planeta** (1 136 m y 15 % por encima de 2 km). El exceso de relieve del planeta viene sobre todo de la regla, la base, y no de su corteza.
- **Dónde falla.**
  - En la regla, cada km de corteza sube la tierra 152 m y cada km de sedimento, 273 m.
  - En la Tierra, con la corteza de CRUST1.0: unos 65–75 m por km de corteza, y el sedimento no la sube (−17 m por km).
  - Los escudos y las plataformas viejas (el 61 % de la tierra) quedan 1 km por debajo de la regla. Las zonas jóvenes y calientes (rifts, Cuenca y Cordillera, Kenia) quedan por encima.
- **No lo arreglan** las densidades reales de cada capa de CRUST1.0 (1 393 m), mover el cero de la regla (se hunden los continentes) ni el grosor de la litosfera de LITHO1.0 (explica poco).
- **Lo que falta es lo que hay debajo de la corteza:** el manto litosférico, su temperatura y su composición. Es la isostasia térmica (Lachenbruch y Morgan 1990; Hasterok y Chapman 2007). La regla del modelo es la de una corteza joven sobre un manto caliente: acierta en los arcos (581 m, frente a 600) y yerra en los escudos.
- **La corteza del planeta frente a la real.**
  - La mediana en tierra es la misma: 37,7 km, frente a 37,3.
  - Pero tiene el doble de corteza de más de 45 km (20 %, frente a 9 %).
  - Tiene un tercio del sedimento continental (0,45 km, frente a 1,24).
  - Y un tercio de la corteza continental sumergida (márgenes y plataformas): el 3,5 % del planeta, frente al 9,2 %.

## 1. Cómo se mide

**La regla del modelo** (`tectonica/isostasia.rs: altura`, con los parámetros de `config/terreno.toml`):
- **Corteza continental:** flotación `B = (ρ_m − ρ_c)·(h − h_nivel) + (ρ_m − ρ_s)·s`.
  - `h` es la corteza cristalina y `s` el sedimento; `ρ_c` 2 800, `ρ_m` 3 300, `ρ_s` 2 400 y `ρ_w` 1 030 kg/m³; `h_nivel` 32 km.
  - Al aire sube `B/ρ_m`: 152 m por km de corteza y 273 m por km de sedimento. Bajo el agua, `B/(ρ_m − ρ_w)`: 220 y 396 m.
- **Fondo oceánico:** la curva de profundidad adoptada (placa ajustada a la mediana del zócalo real, §46 del diseño) y la flotación de su sedimento. Con las edades de Seton et al. (2020) y el sedimento de GlobSed, como en `tierra_regla_modelo.py`.
- **Nivel del mar:** el que llena las cuencas con el agua del modelo (`gel_m` = 2 617 m).

**La corteza real:** CRUST1.0 (Laske et al. 2013), en celdas de 1°.
- `h` son sus tres capas cristalinas y `s` sus tres capas de sedimento. El agua y el hielo no cuentan: el modelo representa la roca sin hielo con su rebote.
- Se interpola en las celdas de nivel 3 (~26 km), bilinealmente entre los cuatro centros de 1° que rodean cada celda.
- En las celdas continentales se interpola solo entre celdas de tipo continental, para no mezclar el fondo oceánico en la costa.
- **Qué es continental:** lo que no tiene edad de Seton en al menos la mitad de la celda, como en `tierra_regla_modelo.py`.

**Las medidas** son las de la tabla de cierre, con la definición de la referencia (`referencia_tierra.py`).
- La tierra son las celdas con su media por encima del nivel del mar.
- La pendiente se mide a ±25 km.
- La plataforma es el mar de menos de 200 m de profundidad, entre la longitud de costa.

**La resolución.** CRUST1.0 tiene celdas de ~111 km. La pendiente y la plataforma dependen de la resolución, así que se comparan también con ETOPO llevado a celdas de 1° y vuelto a interpolar igual (columna «Tierra a 1°»).

**Diagnóstico:** el grosor del manto litosférico (la base de la litosfera menos el Moho) de LITHO1.0 (Pasyanos et al. 2014), y los tipos de corteza de CRUST1.0.

## 2. El relieve con la corteza real

| | Tierra (ETOPO sin hielo, 26 km) | Tierra a 1° | Regla: solo continentes (océano real) | **Regla: toda la Tierra** | Planeta (5 semillas, ronda anterior) |
|---|---|---|---|---|---|
| Nivel del mar respecto al dato (m) | 0 | 0 | −84 | **−221** | −98 |
| Tierra emergida | 0,284 | 0,282 | 0,313 | **0,323** | 0,333 |
| Altura media de la tierra (m) | 671 | 658 | 1 293 | **1 389** | 1 136 |
| Altura mediana de la tierra (m) | 404 | 409 | 1 160 | **1 280** | — |
| Tierra por debajo de 200 m | 0,283 | 0,268 | 0,065 | **0,047** | 0,081 |
| Tierra por encima de 2 000 m | 0,053 | 0,046 | 0,167 | **0,197** | 0,150 |
| Pendiente media de la tierra (±25 km) | 0,0052 | 0,0029 | 0,0046 | **0,0048** | 0,0153 |
| Anchura media de la plataforma (km) | 42,8 | 53,5 | 19,6 | **17,4** | 8,8 |
| Profundidad media del océano (m) | 3 667 | 3 650 | 3 810 | **3 864** | 3 952 |

- **La regla, con la corteza real, no da el relieve real en ninguna fila.**
- La pendiente de la regla con CRUST1.0 hay que compararla con la Tierra a 1°: 0,0048, frente a 0,0029. La regla convierte las diferencias de grosor en el doble de desnivel.
- **La plataforma real emerge.**
  - La plataforma real es el 4,9 % del planeta, con 28,8 km de corteza y 2,5 km de sedimento de media. Con la regla, el 69 % de ella queda en tierra (a +343 m de media), el 21 % más hondo que 200 m y solo el 10 % sigue siendo plataforma.
  - La costa se va al talud, donde el fondo cae deprisa. Por eso la plataforma se queda en un tercio y la tierra emergida sube del 28,4 al 32,3 %.
  - Es el sedimento: con la regla, sus 2,5 km levantan la columna más de lo que la hunde su corteza fina.
- El nivel del mar baja 221 m con la regla: unos 150 por el fondo oceánico (§46) y el resto por la corteza continental (con el océano real, −84 m).

**Sensibilidad** (toda la Tierra con la regla; cambia solo lo que se indica):

| Variante | Nivel del mar (m) | Tierra | Altura media (m) | < 200 m | > 2 000 m |
|---|---|---|---|---|---|
| La regla | −221 | 0,323 | 1 389 | 0,047 | 0,197 |
| Con la densidad de cada capa de CRUST1.0 (corteza en tierra 2 816; sedimento 2 309) | −303 | 0,305 | 1 393 | 0,042 | 0,192 |
| Con la flotación de la corteza oceánica de más de 7 km (mesetas) | −142 | 0,318 | 1 332 | 0,056 | 0,179 |
| Sin el sedimento continental | −377 | 0,281 | 1 350 | 0,045 | 0,156 |
| Con el sedimento contado como corteza | −277 | 0,302 | 1 348 | 0,048 | 0,157 |
| Moviendo el cero (`h_nivel` 47 km, hacia la altura media real) | −1 110 | 0,098 | 928 | 0,176 | 0,093 |

- Las densidades reales de las capas apenas cambian nada: la corteza de CRUST1.0 en tierra tiene 2 816 kg/m³ de media, casi la del modelo.
- Quitar el sedimento baja la tierra por encima de 2 km al 15,6 %, pero la altura media apenas baja: el mar baja con los márgenes.
- **Mover el cero no sirve.** Si `h_nivel` sube, se hunden también los márgenes y las plataformas: con el agua del modelo, la tierra se queda en el 10 % del planeta y aún sobra tierra alta. Lo que está mal es la pendiente de la regla, no su cero.

## 3. Dónde falla la regla

### 3.1 Por grosor de la corteza (tierra y plataforma continental de la Tierra real)

| `h` desde (km) | Área | Altura real (m) | Regla (m) | Regla con las densidades de CRUST1.0 (m) |
|---|---|---|---|---|
| 20 | 3,5 % | 33 | −221 | −364 |
| 25 | 12,0 % | 96 | 217 | 60 |
| 30 | 9,4 % | 263 | 581 | 502 |
| 32,5 | 13,3 % | 466 | 871 | 866 |
| 35 | 18,8 % | 536 | 1 199 | 1 097 |
| 37,5 | 17,3 % | 536 | 1 525 | 1 355 |
| 40 | 10,8 % | 567 | 1 856 | 1 680 |
| 42,5 | 6,1 % | 756 | 2 189 | 2 047 |
| 45 | 3,1 % | 969 | 2 511 | 2 480 |
| 47,5 | 1,6 % | 1 290 | 2 872 | 2 910 |
| 50 | 1,2 % | 2 358 | 3 397 | 3 636 |
| 55 | 0,6 % | 3 538 | 4 224 | 4 557 |
| 60 | 0,5 % | 4 130 | 4 943 | 5 423 |
| 70 | 0,4 % | 4 869 | 6 319 | 7 031 |

- **En la Tierra, de 32 a 45 km de corteza la tierra apenas sube:** de 470 a 760 m. Ahí está el 60 % de la tierra: los escudos y las plataformas.
- Con la regla, ese mismo tramo sube de 870 a 2 500 m.
- Por encima de 47 km (los orógenos) la Tierra sube deprisa, pero la regla sigue 1–1,5 km por encima.
- Con la regla, el planeta se comporta igual: en su tierra, 1 040 m a 37,5–40 km y 1 969 m a 45–47,5 km (§4).

### 3.2 Por tipo de corteza de CRUST1.0 (tierra real)

| Tipo | Tierra | `h` (km) | `s` (km) | Altura real (m) | Regla (m) | Real − regla (m) | > 2 km real | > 2 km regla |
|---|---|---|---|---|---|---|---|---|
| Escudos y plataformas (Arcaico y Proterozoico) | 61,4 % | 37,7 | 1,3 | 456 | 1 437 | −981 | 0,5 % | 18,3 % |
| Fanerozoico | 7,6 % | 35,6 | 1,2 | 431 | 1 074 | −643 | 0,2 % | 7,6 % |
| Orógenos | 16,6 % | 44,2 | 0,9 | 1 657 | 2 326 | −669 | 26,0 % | 55,3 % |
| Arcos y antearcos | 5,2 % | 32,7 | 1,2 | 600 | 581 | +19 | 5,4 % | 6,0 % |
| Corteza estirada y rifts | 6,3 % | 32,3 | 0,9 | 723 | 460 | +263 | 6,2 % | 2,2 % |
| Plataformas, taludes y márgenes | 2,6 % | 30,8 | 1,6 | 245 | 383 | −138 | 1,6 % | 2,9 % |

La regla acierta en los arcos, donde la corteza es joven y está sobre un manto caliente. Se queda corta en los rifts. Y deja los escudos, las plataformas y los orógenos 0,6–1 km demasiado altos.

### 3.3 Algunos lugares (celda de 1° de CRUST1.0; altura de su celda, de ETOPO1)

| Lugar | Tipo | Altura real (m) | Corteza (km) | Sedimento (km) | Regla (m) |
|---|---|---|---|---|---|
| Plataforma rusa | Proterozoico | 110 | 41,6 | 2,5 | 2 133 |
| Amazonía | Arcaico tardío | 40 | 39,1 | 1,9 | 1 595 |
| Cuenca del Congo | Arcaico temprano | 420 | 36,0 | 6,0 | 2 242 |
| Siberia occidental | Fanerozoico | 50 | 34,0 | 3,0 | 1 121 |
| Llanura del Misisipi | Plataforma | 70 | 25,5 | 8,0 | 1 197 |
| Australia central | Proterozoico tardío | 440 | 42,7 | 1,0 | 1 898 |
| Tíbet | Orógeno | 4 920 | 72,1 | 0,1 | 6 103 |
| Altiplano andino | Orógeno | 3 760 | 58,6 | 8,0 | 6 209 |
| Cuenca y Cordillera (Nevada) | Corteza estirada | 2 070 | 33,6 | 0,1 | 265 |
| Rift de África oriental (Kenia) | Rift | 2 260 | 33,7 | 0,3 | 339 |

Las grandes llanuras y cuencas de la Tierra quedan con la regla a 1–2 km de altura. Las zonas calientes, a unos cientos de metros, cuando están a más de 2 km.

### 3.4 El sedimento (tierra real)

| Sedimento desde (km) | Tierra | Real − regla (m) |
|---|---|---|
| 0 | 49,0 % | −470 |
| 0,5 | 14,7 % | −644 |
| 1 | 14,7 % | −920 |
| 2 | 13,9 % | −1 175 |
| 4 | 7,0 % | −1 580 |
| 8 | 0,9 % | −2 103 |

Cuanto más sedimento, más se pasa la regla. En la Tierra, las cuencas sedimentarias son bajas porque la litosfera bajo ellas se hundió: subsidencia térmica tras un rift, flexión o el empuje del manto. Su sedimento rellena ese hueco y no levanta la tierra. En la regla, cada km de sedimento la sube 273 m.

### 3.5 El manto litosférico (LITHO1.0)

| Manto litosférico desde (km) | Tierra | Real − regla (m) | Altura real (m) |
|---|---|---|---|
| 0 | 40,5 % | −434 | 807 |
| 50 | 11,6 % | −902 | 939 |
| 100 | 11,0 % | −993 | 592 |
| 150 | 19,0 % | −962 | 485 |
| 200 | 16,9 % | −994 | 443 |
| 250 | 1,0 % | −974 | 354 |

- Bajo 50 km de manto litosférico, la regla se pasa la mitad que con más (−434 m, frente a −900/−1 000).
- Pasados 50 km, el grosor sísmico de LITHO1.0 ya no ordena el residuo.
- La isostasia térmica pura daría unos −13 m por km de manto litosférico, y la recta da −2,3 (§3.6). Los −13 salen con α = 3·10⁻⁵ K⁻¹ y una geoterma lineal de ~500 °C en el Moho a 1 330 °C en la base. Dos razones posibles:
  - el manto de los escudos es más ligero por su composición, y eso compensa su frío (hipótesis isopícnica de Jordan, 1978);
  - la base sísmica de la litosfera mide mal su estado térmico.

### 3.6 Rectas de la altura real en la tierra (mínimos cuadrados ponderados por área)

| Recta | Ordenada (m) | Por km de corteza (m) | Por km de sedimento (m) | Otros términos | r² |
|---|---|---|---|---|---|
| La regla (al aire) | — | 152 | 273 | — | — |
| Real, con `h` y `s` | −2 095 | 74 | −21 | — | 0,39 |
| Real, con el manto litosférico `L` | −1 926 | 75 | −11 | −2,3 por km de `L` | 0,44 |
| Real, con un término por tipo de corteza | −1 985 | 65 | −17 | Fanerozoico +107; orógenos +765; arcos +467; rifts y estirada +611; márgenes +241 (respecto a escudos y plataformas) | 0,53 |

- **El tipo de corteza explica más que el grosor de la litosfera.** Sin él, r² = 0,39; con LITHO1.0, 0,44; con el tipo, 0,53.
  - El tipo hace de edad termotectónica: el tiempo desde el último rift, arco u orógeno.
  - Las zonas jóvenes quedan 0,5–0,8 km por encima de los escudos con la misma corteza.
- **Aun dentro de cada tipo, la pendiente real es 65 m por km, no 152.**
  - Una parte puede venir del error de `h` en CRUST1.0, que aplana cualquier recta.
  - La prueba directa (§2) no depende de eso: la altura media de la regla es el doble.

### 3.7 Lo más que puede arreglar una corrección por tipo

| | Tierra | Regla | Regla + el residuo medio de su tipo | Recta real por tipo (océano real) |
|---|---|---|---|---|
| Nivel del mar (m) | 0 | −221 | −244 | +163 |
| Tierra emergida | 0,284 | 0,323 | 0,276 | 0,283 |
| Altura media (m) | 671 | 1 389 | 904 | 563 |
| < 200 m | 0,283 | 0,047 | 0,139 | 0,211 |
| > 2 000 m | 0,053 | 0,197 | 0,072 | 0,029 |
| Pendiente (±25 km; la Tierra a 1°, 0,0029) | 0,0052 | 0,0048 | 0,0060 | 0,0030 |
| Plataforma (km; la Tierra a 1°, 53,5) | 42,8 | 17,4 | 19,3 | 37,7 |

- **Una corrección perfecta por la edad termotectónica arreglaría más o menos la mitad.** Es como la de la columna «regla + el residuo medio de su tipo», que es lo más que puede dar un término por tipo:
  - la tierra por encima de 2 km bajaría del 19,7 % al 7,2 %;
  - la altura media, de 1 389 a 904 m.
- Quedarían la tierra baja (14 %, frente a 28 %), la pendiente (el doble) y la plataforma: lo que depende de la pendiente dentro de cada tipo y del sedimento.
- La recta real (la última columna) es una regresión: por construcción se queda corta en los extremos (2,9 % por encima de 2 km).

## 4. La corteza continental del planeta frente a CRUST1.0

Planeta: las 5 semillas de la ronda anterior (rama A), del volcado por celda al final de la corrida. CRUST1.0: la corteza sin edad de Seton (tierra) y la sumergida de tipo continental (bajo el mar).

| En tierra | Planeta (media ± desv.) | CRUST1.0 |
|---|---|---|
| Área (fracción del planeta) | 0,295 ± 0,027 | 0,283 |
| `h` media (km) | 39,4 ± 0,5 | 37,8 |
| `h` p10 / p25 / p50 / p75 / p90 (km) | 32,0 / 34,6 / 37,7 / 43,0 / 50,1 | 30,6 / 34,1 / 37,3 / 40,6 / 44,6 |
| Más de 40 km | 36,4 % | 28,6 % |
| **Más de 45 km** | **20,0 %** | **9,1 %** |
| **Más de 50 km** | **10,1 %** | **3,7 %** |
| Más de 60 km | 2,2 % | 1,6 % |
| Más de 70 km | 0,2 % | 0,4 % |
| **Sedimento medio (km)** | **0,45 ± 0,07** | **1,24** |

| Bajo el mar (corteza continental) | Planeta | CRUST1.0 (tipo continental) |
|---|---|---|
| **Área (fracción del planeta)** | **0,035 ± 0,005** | **0,092** |
| `h` media (km) | 25,7 | 26,2 |
| Sedimento medio (km) | 1,02 | 2,49 |

- **La mitad de abajo de la distribución es la real; la de arriba, no.** En tierra, la mediana es la misma, pero el planeta tiene más del doble de corteza de más de 45 km y casi el triple de más de 50.
  - Es el engrosamiento de los choques y de los arcos que no se va: con la regla, cada uno de esos km sube la tierra 152 m.
- **Le falta sedimento continental** (un tercio del real).
- **Le faltan los márgenes sumergidos** (un tercio del área real). Ya se sabía: las franjas estiradas de los márgenes miden 65 km, frente a 100–300.
- **La corteza continental total** es el 33 % del planeta (`f_cont` 0,330), frente al ~40 % de la Tierra.

## 5. Lo que dicen los datos sobre la causa

1. **La regla del modelo es la isostasia de Airy de la corteza sola, con un cero fijo.**
   - El cero dice que una corteza de 32 km queda a nivel del mar.
   - Es el mismo para todos los continentes, y vale para una corteza joven sobre un manto caliente: acierta en los arcos y casi en los márgenes.
2. **En la Tierra cuenta también lo que hay debajo de la corteza: el manto litosférico.**
   - Bajo los escudos y las plataformas viejas es frío, grueso y denso, y los hunde ~1 km respecto a la regla.
   - Bajo los rifts y las zonas estiradas es caliente y fino, y los levanta.
   - Es la isostasia térmica (Lachenbruch y Morgan 1990; Hasterok y Chapman 2007). La depleción del manto de los escudos la compensa en parte (Jordan 1978).
3. **Las cuencas sedimentarias son bajas porque su litosfera se hundió** (subsidencia térmica tras el rift, McKenzie 1978; flexión). Su sedimento no las levanta.
4. **Con la corteza del modelo pasa lo mismo:**
   - la regla convierte su corteza de 37–40 km en tierra de ~1 km;
   - y su cola de más de 45 km, en el 15 % de la tierra por encima de 2 km.
   - Por eso, recalibrar la erosión o el engrosamiento para bajar el relieve sería forzar parámetros contra una base irreal (CLAUDE §2.8).
5. **La pendiente del planeta (0,0153, el triple de la real)** no la explica la regla con la corteza real (0,0048). Queda por medir si viene de cómo cambia de grosor la corteza del planeta en distancias cortas o del detalle fino. Va después de arreglar la base.
6. **Las plataformas.** Con la regla, el sedimento de los márgenes los levanta por encima del mar (§2). En la Tierra, los márgenes se hunden al enfriarse tras el rift (la subsidencia térmica), y el sedimento rellena ese hueco hasta la plataforma. Es la misma física que falta en los escudos y en las cuencas.

## 6. Opciones (decide el responsable)

1. **Recomendada, la más realista: isostasia térmica de la litosfera continental según su edad termotectónica.**
   - **Qué es.** Que la flotación de cada columna continental tenga un término de enfriamiento, igual que el fondo oceánico tiene su curva de profundidad por edad.
     - La litosfera continental se calienta y adelgaza en cada rift, arco u orógeno.
     - Después se enfría, se engrosa y baja durante cientos de Myr, hasta ~1 km en los escudos.
     - Las cuencas que nacen en los rifts se hunden así y se llenan de sedimento.
   - **Qué hace falta:**
     - una edad termotectónica por muestra: el tiempo desde el último evento térmico. El modelo tiene `edad_orog`; hay que ver si la reinician también los rifts y los arcos;
     - una curva de amplitud y tiempo característico, calibrada con la Tierra: el residuo por edad termotectónica de CRUST1.0 y ETOPO (§3.2, §3.6) y, si hace falta, el flujo de calor (Lucazeau 2019) o las edades de Artemieva (2006);
     - revisar después el cero `h_nivel`, que pasaría a ser el de la litosfera joven.
   - **Qué daría.** Según los datos, como mucho lo del §3.7: un 7 % de tierra por encima de 2 km y 900 m de altura media. Lo que quede (la tierra baja y la pendiente dentro de cada tipo) se mide después.
2. **Corteza inferior más densa al engrosarse (eclogita), para los orógenos** (Tíbet: regla 6,1 km, real 4,9). Puede ir con la 1. El modelo ya tiene la delaminación de la raíz de más de 50 km, pero no la densificación.
3. **Provisional (tipo c): la relación medida en lugar de Airy** (`z = −1 985 + 65·h − 17·s` más un término por tipo).
   - Da una hipsometría parecida (§3.7, última columna), pero no es física.
   - Habría que marcarla como apaño, con la causa real que hay que corregir (la 1).
4. **Dejar la regla y seguir con las placas** (rama D y rifts v2), que no dependen de la isostasia.
   - Las filas del relieve de la tabla de cierre seguirían mal por la base.
   - Igual que la erosión y el sedimento al mar, que dependen de la altura.

Las opciones 1 y 2 tocan la base sobre la que se calibraron `E_1` (la relajación de un orógeno) y el nivel del mar. Habría que volver a mirarlas después.

## 7. Lo que queda en espera (encargo de esta ronda, sin empezar)

- **Nota de demografía de placas v2:** quitar la soldadura por borde inactivo (A1); mantener la captura de microplacas (A2) y la retirada de las placas fantasma; rama D, subducción de dorsales (`d_dc` 300 km), y D6, encuentros entre dorsales y fosas.
- **Nota de rifts v2:** probabilidad de ruptura con el reloj de cada bloque, `p = 1 − exp(−λ_A·A·min(1, t/τ_ais)·Δt)` con `τ_ais` = 100 Myr. Recalibrar `λ_A` y `τ_ais` con las placas de al menos el 1 % (~13) y con la duración de los supercontinentes.
- **Objetivo de tierra por encima de 2 000 m: 5,3 %.** Ya es el valor con el que compara la tabla de cierre (`planeta_semillas.py` toma la roca sin hielo de `tierra_relieve.toml`).
- **El tiempo:** con la rama A, el planeta tardaba 298 s solo (presupuesto 275). Crece el reparto del sedimento al mar (21,9 → 37,7 s por deriva) y hay que perfilarlo.
- **Tabla de cierre completa con 5 semillas** y el mapa del motor al día.

La rama D y los rifts v2 no dependen de la isostasia. Si el responsable quiere, pueden ir antes que el arreglo de la base. La tabla de cierre del relieve, no.

## 8. Estado del código y de las ramas

- **Sin cambios en el motor.** Son nuevos:
  - la herramienta `herramientas/analisis/tierra_corteza_regla.py`;
  - su salida, `config/referencias/tierra_corteza.toml`;
  - este informe.
- **Datos descargados** (en `output/referencias/`, ignorado por git):
  - `crust1/`: `crust1.bnds`, `crust1.rho`, `CNtype1-1.txt` y `CNtype1_key.txt`, sacados por su nombre de los tar de igppweb.ucsd.edu;
  - `litho1/litho1.0.tar.gz`, que se lee sin extraerlo.
- **Cachés** (en `output/cache/`): las celdas de nivel 3 de Seton y de GlobSed, y la litosfera de LITHO1.0 en la malla de 1°.
- **Las notas v2 del responsable** (`nota_demografia_placas.md`, `nota_rifts.md`) se guardan en este commit tal como llegaron, sin implementar.
- **Ramas:**
  - `terreno-demografia-placas`, sin fusionar, con la rama A;
  - `main` sigue en 7c54bbd.

## Referencias

- Artemieva, I. M. (2006). Global 1°×1° thermal model TC1 for the continental lithosphere. Tectonophysics 416.
- Hasterok, D. y Chapman, D. S. (2007). Continental thermal isostasy: 1. Methods and sensitivity; 2. Application to North America. Journal of Geophysical Research 112.
- Jordan, T. H. (1978). Composition and development of the continental tectosphere. Nature 274.
- Lachenbruch, A. H. y Morgan, P. (1990). Continental extension, magmatism and elevation; formal relations and rules of thumb. Tectonophysics 174.
- Laske, G., Masters, G., Ma, Z. y Pasyanos, M. (2013). Update on CRUST1.0, a 1-degree global model of Earth's crust. Geophysical Research Abstracts 15, EGU2013-2658.
- Lucazeau, F. (2019). Analysis and mapping of an updated terrestrial heat flow data set. Geochemistry, Geophysics, Geosystems 20.
- McKenzie, D. (1978). Some remarks on the development of sedimentary basins. Earth and Planetary Science Letters 40.
- Pasyanos, M. E., Masters, T. G., Laske, G. y Ma, Z. (2014). LITHO1.0: An updated crust and lithospheric model of the Earth. Journal of Geophysical Research: Solid Earth 119.
