# Hundimiento de los márgenes, puntos calientes y succión de las losas

2026-11-08, quinta ronda. Rama `terreno-margenes-calientes-succion` (sin fusionar). `main` = 473b805: la rama de la isostasia fusionada, con la regla relativa del tiempo y los márgenes como entidades. Diseño: `incremento-4-diseno.md` §75–§77.

## Resumen

- **Fusión y regla del tiempo.** La rama de la isostasia está en `main`.
  - La batería compara ahora el planeta con la versión anterior, las dos a la vez: no más de un 5 % más lento, con un techo de 330 s.
  - Al fusionar: +4,2 % frente al `main` de antes.
- **Las tres notas hacen lo que dicen**, cada una medida con 5 semillas tras implementarla:

  | | Medida | Objetivo |
  |---|---|---|
  | Márgenes (McKenzie) | se hunden 1,5 km con β 2–3 y 1,6 km con β > 3 | McKenzie: 1,57 y 1,77 km |
  | Puntos calientes | magmatismo intraplaca oceánico de 1,41 AU y 4,1 grandes erupciones por 100 Myr | 1,3–1,7 AU y ~4,3 |
  | Succión | el tirón, el 58 % de la potencia; las fosas retroceden en el 66 % de su longitud | ~50 % y 62–78 % |

- **Pero el planeta empeora con la succión.** Las masas continentales se acercan, chocan más (30 000 km de choque, frente a 23 000 en la Tierra) y los choques destruyen corteza:

  | | Con los puntos calientes, sin succión | Con todo |
  |---|---|---|
  | Pérdida de volumen continental | −6 % | −13 % |
  | `f_cont` | 0,33 | 0,30 |
  | Nivel del mar | −100 m | −202 m |
  | Altura media de la tierra | 800 m | 963 m (la Tierra 671) |

- **Los choques no se agotan nunca.** Convergen a 1,1–1,4 cm/año incluso con más de 80 Myr, así que las placas casi no se sueldan (0,98 soldaduras por 100 Myr). Es el caso del §11.3 de la nota de succión: **falla la resistencia de los choques**. Te lo traigo (§7).
- **Supercontinente:** no se forma.
  - Las masas se acercan (−1,7 km/Myr) y se cierran algo más océanos de rift (1,2 por semilla, a los ~190 Myr).
  - Pero el mayor bloque de una sola placa reúne solo el 14 % del continente (el criterio pide pasar del 60 % una vez).
- **Tiempo:** cumple la regla, justo. El planeta tarda 259,5 s y `main`, a la vez, 247,2 (+5,0 %; techo 330). Lo que más crece es la succión (~7 s directos) y lo que trae consigo: más sedimento y más flujo lateral (§9).
- **Decisiones para ti** en el §12.

## 1. La fusión y la regla del tiempo

- **La regla, en la batería** (`herramientas/escenarios.py`).
  - El planeta corre a la vez que su versión anterior: por defecto `main`, o el commit anterior si se está en `main`. Se compila en un worktree de `output/referencia/` y se corre sin la caché de la génesis.
  - Uno tras otro no servía: el segundo salía un 2–5 % más lento, porque la máquina cambia de un momento a otro.
  - El techo de 330 s se comprueba con ese tiempo, algo mayor que el del planeta solo.
- **La fusión.** Medidas a la vez, la rama tardaba 302,7 s y `main` 287,3 (+5,4 %).
  - El recuento de los márgenes por celdas de costa y su perfil pasaron a calcularse cada 25 Myr y en el último paso (solo se usan sus valores finales). El modelo no cambia: la salida es idéntica salvo esas medidas.
  - Con eso: 251,2 s frente a 241,0 (+4,2 %). Fusionada con `main` sin conflictos (473b805).
- **Los márgenes como entidades** (pedido por ti): uno por lado de cada rift.
  - Nace cuando se rompe.
  - Termina cuando la mitad de sus celdas tiene cerca una fosa, una convergencia difusa o un choque, o cuando le queda menos del 10 % de las celdas que llegó a tener.

## 2. Nota de hundimiento de los márgenes (McKenzie)

- **Qué se hizo:** cada muestra guarda su estiramiento al romper el rift (`β = h_0/h_c`) y el tiempo desde la rotura. La isostasia R4 suma la carga de McKenzie (primer término) menos lo que R4 ya hunde desde la rotura.
- **Prueba del rift aislado:** sigue la curva de McKenzie exactamente (902 muestras, desvío 0).
- **En el planeta**, el basamento de los márgenes distales se hunde entre 0–20 y 80–120 Myr desde la rotura:

  | β | Planeta | McKenzie, para el β del medio de la clase |
  |---|---|---|
  | 1–2 | 1 217 m | 859 (β 1,5; la clase se acerca a 2) |
  | 2–3 | 1 504 m | 1 573 |
  | > 3 | 1 639 m | 1 769 |

  Antes, 0,38 km en cualquier margen.
- **Lo que no mejora:**
  - la plataforma sigue en 9–11 km (la Tierra 43);
  - el sedimento solo rellena los márgenes hasta ~2–2,5 km de agua: los más viejos tienen el basamento a 4 km;
  - el mar baja 48 m (de −140 a −188), porque los márgenes hondos guardan más agua.

## 3. Nota de puntos calientes

- **Qué se hizo:** 12 puntos calientes fijos en el marco del manto, de 150 Myr de vida, con:
  - abombamiento sobre el océano;
  - trazas que engrosan la placa que pasa (cadenas como Hawái);
  - grandes erupciones al nacer algunos: mesetas en el océano, basaltos en los continentes.
- Las mesetas flotan y se acrecionan como los arcos, pero **sin pasar a continentales** (§8, respuesta 2). `F_int` entra en el balance de la corteza.
- **Pruebas:** la traza sale recta y tan larga como lo que avanzó la placa, con el caudal exacto; el abombamiento no viaja con la placa; una meseta en la fosa se acreciona un 90 %; el balance cuadra a 10⁻⁹; misma semilla, mismos puntos calientes.
- **En el planeta** (5 semillas, sin succión):
  - el mar sube de −188 a −100 m y la tierra baja de 849 a 800 m;
  - la plataforma pasa de 9,3 a 11,1 km;
  - el océano tiene media menos mediana de −367 m (ETOPO, −414);
  - 4,1 grandes erupciones por 100 Myr (objetivo ~4,3);
  - magmatismo oceánico: 1,20 AU con el caudal de partida y 1,41 con el recalibrado (1,25·10⁵ km³/Myr por traza), dentro de 1,3–1,7.
- **Lo que no cuadra:** los basaltos continentales dan 0,03 AU, frente a ~0,12 (Clift et al. 2009).
  - Para llegar haría falta `h_CFB` ≈ 12 km, fuera de su rango (1–5). No se fuerza (CLAUDE §2.8).
  - Con estas áreas, solo ~1 de cada 3 erupciones cae en un continente. Los 0,12 de Clift deben de incluir otro magmatismo continental.

## 4. Nota de succión de las losas

- **Qué se hizo:**
  - la parte de cada losa que pasa de `L_losa`, y los restos de las fosas que mueren o se rompen, tiran de la base de las placas cercanas (a menos de 1 500 km);
  - la tracción entra como un par más en el equilibrio de fuerzas;
  - la soldadura de siempre (las suturas) pide ahora la losa rota y 20 Myr con la convergencia agotada.
- **Pruebas:** una fosa con losa profunda arrastra a la placa de arriba hacia ella; un resto atrae a las placas cercanas hasta `τ_hund` (183 Myr) y después no; un choque agotado suelda sus placas sin perder corteza (y a 3 cm/año, no).
- **Un fallo, corregido:** al principio, la tracción de cada celda gruesa (~37 000 km²) iba a la placa de su centro, y una microplaca podía disparar su velocidad. Ahora el par se suma por las celdas finas, cada una con su placa y su área.
- **Calibración de `ε_suc`** (3 semillas por valor):

  | `ε_suc` | Tirón / succión / empuje | Retroceso de las fosas | Cambio de volumen | Nivel del mar |
  |---|---|---|---|---|
  | 0,05 | 0,54 / 0,29 / 0,18 | 68 %, 1,13 cm/año | −16 % | −374 m |
  | 0,1 | 0,48 / 0,37 / 0,16 | 66 %, 1,01 | −30 % | −466 |
  | 0,2 | 0,31 / 0,58 / 0,11 | 67 %, 1,05 | −28 % | −585 |

  - Se queda **0,05**: el tirón queda cerca del ~50 % de Conrad y Lithgow-Bertelloni (2002) y es el que menos corteza destruye.
  - El retroceso de las fosas cumple en fracción (62–78 %), pero no en velocidad (1,3–1,5 cm/año): sale ~1,0.
- **Por qué empeora el planeta con la succión** (cuentas de 3 semillas, con 0,05):
  - las masas se acercan y chocan más: el solape de los choques pasa de 18,4 a 22,0 AU, la delaminación de 3,1 a 4,0 y el sedimento subducido de 0,74 a 1,0;
  - el continente pierde volumen, `f_cont` baja y, con menos tierra, la misma agua se reparte en más océano: el mar baja;
  - es la base, no la succión: los choques destruyen corteza de más (la pérdida en los choques ya era de 1,2 AU, frente a 0,4–0,7) y no se agotan (§7).
- **Los restos de losa:** hay ~1 900 a la vez. Son sobre todo tramos de fosa que mueren y renacen: cada uno deja su losa entera. Y `L_j` crece sin tope en una fosa vieja.

## 5. Recalibración (5 semillas)

- **`E_1`:** se queda en 0,02. La relajación del orógeno no cambia (159 Myr): las notas no tocan ese escenario.
  - La tierra sale más alta (963 m), pero por la corteza más gruesa de los choques (85,8 km al final) y el mar más bajo, no por la erosión. Subir `E_1` rompería la relajación.
- **`ε_suc`:** 0,05 (§4).
- **El arrastre basal:** se recalibra solo a la velocidad real de las placas. Queda en 4,47 cm/año (4,39 ± 25 %).
- **El caudal de las trazas:** 1,25·10⁵ km³/Myr (antes 1,0·10⁵), con lo que el magmatismo oceánico pasa de 1,20 a 1,41 AU.

## 6. Tabla de cierre (5 semillas; la fosa y el área que se hunde, con 10)

| | Modelo | Tierra u objetivo | |
|---|---|---|---|
| Tierra emergida | 0,303 | 0,284 | — |
| Altura media de la tierra | 963 ± 60 m | 671 ± 100 | ✗ |
| Tierra < 200 m / > 2 000 m | 0,146 / 0,123 | 0,283 / 0,053 | ✗ / ✗ |
| Pendiente media | 0,0138 | 0,0052 | ✗ |
| Plataforma | 9,3 km | 42,8 | ✗ |
| Nivel del mar | −202 ± 108 m | ≥ −150 (0 ± 75 en la nota de puntos calientes) | ✗ |
| Profundidad media del océano; media − mediana | 3 789 m; −315 m | 3 667 ± 150; −414 | ✓ / ✗ |
| Corteza máxima al final | 85,8 km | 70–80 | ✗ |
| Edad media del fondo; más de 180 Myr | 90 ± 21 Myr; 15 % | ≤ 88 (64); < 1 % | ✗ |
| `f_cont`; cambio de volumen | 0,295; −13,3 % | 0,40; ±10 % | ✗ / ✗ |
| Velocidad de las placas | 4,47 cm/año | 4,39 | ✓ |
| Sedimento al mar | 5,9 AU | ≈1,8 | ✗ |
| **Longitud de fosa (10 semillas)** | **82 900 ± 7 000 km** | 51 000 ± 25 % | ✗ (×1,6) |
| **Área que se hunde (10 semillas)** | **3,68 ± 0,38 km²/año** | 2,7–3,4 | ✗ |
| Choques; convergencia difusa | 30 000 km; 15 200 km | CCB 23 000; OCB 17 400 | ✗ (+30 %) / ✓ |
| Placas de ≥ 1 %; las 15 mayores | 21,4; 0,86 | 13; 0,958 | ✗ / ✓ |
| Pérdida en choques | 1,31 AU | 0,4–0,7 | ✗ |
| Rupturas por 100 Myr | 4,3 | ~4–7 | ✓ |
| **Márgenes pasivos, como entidades:** vida de los que terminan; edad media y máxima al final | 86 Myr; 197 y 415 Myr | 130–180; ~104 y ~180 | ✗ |
| … nacidos y terminados (por semilla); cómo terminan | 33 y 23; invasión 52 %, choque 45 %, desaparecen 3 % | — | — |
| Altura por clase de edad (0–100 / 100–500 / 500–1 600 / 1 600–2 500 / > 2 500 Myr) | 1 670 / 787 / 528 / 433 / 463 m | 1 247 / 326 / 491 / 421 / 543 ± 300 | 3 de 5 |
| Fracción de la tierra por clase | 0,50 / 0,28 / 0,06 / 0,06 / 0,10 | 0,28 / 0,17 / 0,17 / 0,23 / 0,16 | demasiado joven |
| **La tierra de menos de 500 Myr, por lo que reinició su edad** | arcos 54 %, choques 40 %, grandes erupciones 2,4 %, rifts 1,7 %, delaminación 1,2 %, ninguno 0,8 % | — | — |
| Hundimiento de los márgenes (todos los distales) | 1,70 km | 2–3 | ✗ (por poco) |
| Magmatismo intraplaca oceánico; basaltos continentales | 1,41 AU; 0,03 AU | 1,3–1,7; ~0,12 | ✓ / ✗ |
| Grandes erupciones por 100 Myr; mesetas acrecionadas | 4,1; 0,18 AU | ~4,3; hasta ~1,1 | ✓ / — |
| Potencia: tirón / succión / empuje | 0,58 / 0,23 / 0,19 | tirón ~0,50 | ✓ (±10 %) |
| Fosas que retroceden; velocidad media | 66 %; 1,0 cm/año | 62–78 %; 1,3–1,5 | ✓ / ✗ |
| Supercontinente: mayor bloque de una placa (media); mayor masa conexa (media y máximo) | 0,14; 0,59 y 0,83 | el bloque de una placa > 0,6 alguna vez | ✗ |
| Tiempo (10 a la vez; en la batería, a la vez con `main`, §9) | 324 s; 259,5 s frente a 247,2 | ≤ +5 % y ≤ 330 s | ✓ (+5,0 %) |

- **La tierra joven:** el 94 % de la de menos de 500 Myr la reiniciaron los arcos (54 %) y los choques (40 %). Cada Myr ponen a cero ~9 millones de km² de continente cada uno, las mismas franjas una y otra vez. Con el exceso de fosa (×1,6) y de choques (+30 %), el continente queda demasiado joven: la mitad de la tierra tiene menos de 100 Myr (la Tierra, el 28 %).
- **La fosa y el área que se hunde, con 10 semillas:** varían ±8 % y ±10 % de una semilla a otra; estas 5 salieron más parecidas por azar (±4 % y ±3 %). La media de 10 tiene un error de ~±3 %: las dos siguen lejos de la Tierra.

## 7. Los choques no se agotan (§11.3 de la nota de succión)

La convergencia media de los frentes de choque, por la edad del choque (10 semillas):

| Edad del choque (Myr) | 0–10 | 10–20 | 20–40 | 40–80 | > 80 |
|---|---|---|---|---|---|
| Convergencia (cm/año) | 1,56 | 1,42 | 1,40 | 1,36 | 1,26 |

- **Bajan muy despacio y nunca de `v_sold`** (0,5 cm/año). En cada paso, solo el 10–12 % de los frentes de choque está por debajo.
- Las soldaduras son 0,8–1,0 por 100 Myr, tras 55–61 Myr de choque de media.
- **En la Tierra, los choques se frenan mucho más:**
  - la India pasó de ~15 a ~4–5 cm/año en unos 10 Myr y se ha mantenido así 50 Myr (Molnar y Stock 2009);
  - los choques viejos (los Urales, los Apalaches) se pararon del todo: sus placas se soldaron.
- **Por qué** (lectura de las cuentas, no medido aparte):
  - la resistencia de los choques (`c_col`) se calibró para que la India frene a 1/3, no para pararla;
  - las losas que siguen tirando de los dos lados (tirón y ahora succión) los mantienen convergiendo.
- **Consecuencias:** las placas no se sueldan, el bloque mayor de una placa no crece y el reloj de los rifts casi nunca ve un supercontinente. Además, los choques, que no paran, engruesan y destruyen corteza: la pérdida en choques es de 1,3 AU, frente a 0,4–0,7.

## 8. Respuestas a los §11

### Nota de hundimiento de los márgenes

1. **¿Cómo se calcula `S_R4`?** Es el cambio del término del manto de R4 desde la rotura, con agua encima:
   - `S_R4 = [Δρ(t)·max(0, L(t) − h) − Δρ(0)·max(0, L(0) − h)]/(ρ_a − ρ_w)`, con `t` la edad tectonotérmica (0 al romper) y `h` la corteza con su sedimento;
   - en un margen distal (`h` < 50 km) es `Δρ·(L(t) − L_min)/(ρ_a − ρ_w)`: 0,34 km a los 100 Myr;
   - se suma al manto `max(0, S_McK − S_R4)·(ρ_a − ρ_w)`: con agua encima, la columna se hunde lo de McKenzie, y con sedimento, además, su peso.
2. **¿Se conserva bien `h_0`?** Se guarda por lado del rift, no por celda: la corteza continental media a menos de `W_m` del corte al empezar (`LadoRift::h0_km`).
   - El estiramiento reparte en rampa el volumen de la franja, así que `β = h_0/h_c` es coherente con él: de 1 en el borde de dentro a ~3,5 junto al eje.
   - Una celda que tuviera más corteza que la media queda con `β` = 1.

### Nota de puntos calientes

1. **¿Hay un marco del manto fijo?** Sí: el de la malla del mundo, en el que `velocidades` quita cada paso la rotación neta de las placas.
2. **¿Trata bien el umbral de 20 km las mesetas?** No: pasa a continental una columna de más de 20 km.
   - Con la isostasia de los continentes, una meseta de 22 km quedaría a ~0,6–0,9 km bajo el mar. Las reales están a 1,7–3 km.
   - Por eso la corteza de mesetas y trazas que flota se añade como corteza flotante no continental, sin pasar la columna a continental. Con la isostasia del océano, la meseta queda a ~2,6 km.
   - En las fosas se acreciona un 90 %, como un arco.
   - El umbral solo decide cuándo una traza deja de hundirse en las fosas.
3. **¿Qué coste añade?** Poco: 0,6 s por planeta en su etapa (el 0,3 % de la deriva). El abombamiento, en las alturas, añade ~1 s más entre la erosión y el nivel del mar (§9).

### Nota de succión de las losas

1. **¿Cómo se suman las fuerzas?** En cada placa, `K·ω = T/c_b`.
   - `K` es el arrastre basal, más el acoplamiento de los choques.
   - `T` es el par del tirón de las losas y del empuje de las dorsales; la succión entra en `T` como un par más.
   - `c_b` se recalibra a la velocidad real, así que el arrastre se reajusta solo.
2. **¿Hay un marco del manto fijo?** El mismo de los puntos calientes.
3. **¿Bajan los choques de `v_sold`?** No: ver el §7. Te lo traigo, como pide la nota.

## 9. Tiempo (regla relativa)

| | Planeta (batería, a la vez con `main`) | Deriva de la semilla 42 (traza, a la vez) |
|---|---|---|
| `main` (473b805) | 247,2 s | 166 s |
| **Esta rama** | **259,5 s (+5,0 %)** | 187 s (+13 %) |

- **Cumple la regla, justo:** +5,0 % (el máximo es +5 %) y por debajo del techo de 330 s.
- **Lo que crece en la deriva** (+21 s):
  - la succión, en las velocidades: de 3,7 a 10,7 s;
  - el reparto del sedimento al mar: de 19,5 a 28,6 s. Hay más sedimento que repartir, porque la tierra es más alta y se erosiona más;
  - el flujo lateral: de 21,7 a 27,3 s, por la corteza más gruesa de los choques;
  - los puntos calientes, 0,6 s; el hundimiento de McKenzie, inapreciable.
- El resto de la génesis compensa en parte: el planeta entero crece solo 12 s.
- **Si se quiere margen:**
  - la tracción de la succión se podría calcular cada 2–5 Myr (las losas se mueven despacio);
  - los restos de losa (~1 900) se podrían juntar por zonas.

## 10. Batería y verificación

- **Batería:** el tiempo cumple la regla. 29 métricas del planeta salían de sus márgenes por la realización nueva (relieve, lluvia, biomas, lagos, ríos y tectónica). Se movieron con su nota. Con ellas, todo dentro.
- **Pruebas:** todas en verde (`cargo test --workspace`); `cargo fmt` y `cargo clippy -D warnings`, limpios. Hay 11 pruebas nuevas:
  - tres de los márgenes: la regla aislada, el rift aislado y la mezcla;
  - cinco de los puntos calientes: la traza, el abombamiento, la meseta en la fosa, y el balance y el determinismo en una sola;
  - tres de la succión: la fosa que retrocede, el resto de losa y el choque agotado.
- **Pruebas que cambian:**
  - `sin_cratones…` compara el fondo desnudo con la curva de edad más el abombamiento;
  - `la_losa_sigue_tirando…` se corre sin succión, porque mide la rotura de la losa; con la succión, la India llega por otro camino y su tramo rompe a los 25 Myr.

## 11. Lo que queda abierto

- Los restos de losa son muchos (~1 900) y las losas profundas no tienen tope (`L_j` crece mientras la fosa vive). La nota lo define así; si se quiere acotar, sería otra decisión.
- La India no se acelera tras romperse su losa con `ε_suc` 0,05 (la prueba pasa). Con 1 sí, a 25 cm/año.

## 12. Lo que decides tú

1. **La resistencia de los choques** (§7): los choques no se agotan y destruyen corteza. ¿Cómo debe frenarse un choque? Lo realista es que la convergencia caiga hasta pararse en unas decenas de Myr: la India, de 15 a 5 cm/año en ~10 Myr, y los orógenos viejos, parados del todo.
2. **La succión empeora el planeta** mientras los choques no se arreglen: menos continente, mar más bajo, tierra más alta. ¿Se deja encendida (`ε_suc` 0,05) o se apaga hasta arreglar los choques?
3. **El retroceso de las fosas** va a 1,0 cm/año, frente a 1,3–1,5. Subir `ε_suc` lo acerca, pero destruye más corteza.
4. **Los basaltos continentales** dan 0,03 AU frente a ~0,12 (Clift). ¿Se revisa el objetivo o el área de las erupciones continentales?
5. **Fusionar esta rama con `main`**, cuando lo anterior esté decidido.
