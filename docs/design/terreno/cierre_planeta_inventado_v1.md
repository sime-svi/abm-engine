# Cierre del planeta inventado v1

Documento de cierre (2026-11-08). El responsable congela el planeta inventado tal como está y el proyecto pasa al modo Tierra real (`diseno_modo_tierra.md`). El planeta inventado sigue siendo el otro modo del motor: no se toca más allá de lo que pida la interfaz del proveedor de mundo, y debe salir idéntico byte a byte.

## 1. Qué es el planeta inventado v1

Un planeta que nace de una deriva de placas de 500 Myr y después vive un año de clima, en la malla del cubo esférico con LOD (niveles 0–3, ~190 a ~24 km). Está documentado paso a paso en `incremento-4-diseno.md` (§1–§77) y en `MAPA_DEL_MOTOR.md`. En el planeta inventado v1 entran:

- **Las placas y sus fuerzas:** tirón de la losa, empuje de dorsal, rozamiento basal calibrado a la velocidad real y resistencia de los choques. La succión de las losas está programada pero apagada (`eps_suc` = 0).
- **Los bordes y las fosas:**
  - bordes suavizados;
  - fosas que nacen, invaden márgenes pasivos, se rompen tras un choque y mueren;
  - subducción de dorsales;
  - captura de microplacas.
- **La corteza continental, con balance exacto:** arcos, erosión por subducción, prisma, choques con su pérdida según el grosor y el origen de arco, delaminación, flujo lateral, erosión con la altura y sedimento al mar.
- **Los rifts:** por bloques y por suturas, con el reloj del bloque, márgenes que se estiran y se hunden como en McKenzie, y márgenes pasivos como entidades.
- **La isostasia de toda la litosfera (R4)**, con la edad tectonotérmica de cada celda.
- **Los puntos calientes:** mesetas oceánicas que flotan sin pasar a continentales, trazas volcánicas, abombamientos y grandes erupciones.
- **El terreno del planeta** (incrementos 1–3): clima estacional, lluvia orográfica con su sombra, agua, ríos y lagos, vegetación y suelo.

## 2. Tabla de cierre (5 semillas: 42, 1, 2, 3 y 4; sin succión; `herramientas/planeta_semillas.py`)

| | Modelo | Tierra u objetivo | |
|---|---|---|---|
| Tierra emergida | 0,322 ± 0,009 | 0,284 | — |
| Altura media de la tierra | 902 ± 27 m | 671 ± 100 | ✗ |
| Tierra por debajo de 200 m / por encima de 2 000 m | 0,126 / 0,099 | 0,283 / 0,053 | ✗ / ✗ |
| Pendiente media (±25 km) | 0,0134 | 0,0052 | ✗ |
| Plataforma | 10,0 ± 2,0 km | 42,8 | ✗ |
| Nivel del mar al final de la deriva | −106 ± 82 m | ≥ −150 | ✓ |
| Profundidad media del océano; media − mediana | 3 894 m; −322 m | 3 667 ± 150; −414 | ✗ (por 77 m) / ✗ |
| Corteza máxima al final / en toda la deriva | 81 / 115 km | 70–80 | ✗ (por poco) / ✗ |
| Edad media del fondo; más de 180 Myr | 91 ± 12 Myr; 15 % | ≤ 88 (la Tierra 64); < 1 % | ✗ |
| `f_cont`; cambio de volumen continental | 0,319; −6,3 % | 0,40; ±10 % | ✗ / ✓ |
| Velocidad de las placas | 4,74 cm/año | 4,39 | ✓ |
| Sedimento al mar | 5,0 AU | ≈1,8 | ✗ |
| Longitud de fosa; área que se hunde | 85 600 km; 4,0 km²/año | 51 000 ± 25 %; 2,7–3,4 | ✗ (×1,7) / ✗ |
| Choques; convergencia difusa | 23 400 km; 12 800 km | CCB 23 000 ± 25 %; OCB 17 400 ± 50 % | ✓ / ✓ |
| Placas de ≥ 1 %; las 15 mayores | 23,2; 0,82 | 13; 0,958 ± 0,10 | ✗ / ✗ |
| Pérdida en choques | 1,33 AU | 0,4–0,7 | ✗ |
| Rupturas por 100 Myr | 3,9 | ~4–7 | ✗ (por poco) |
| Márgenes pasivos (entidades): vida de los que terminan; edad media y máxima al final | 79 Myr; 173 y 378 Myr | 130–180; ~104 y ~180 | ✗ |
| Altura por clase de edad (0–100 / 100–500 / 500–1 600 / 1 600–2 500 / > 2 500 Myr) | 1 568 / 734 / 434 / 423 / 533 m | 1 247 / 326 / 491 / 421 / 543 ± 300 | 3 de 5 |
| Fracción de la tierra por clase | 0,48 / 0,32 / 0,07 / 0,06 / 0,07 | 0,28 / 0,17 / 0,17 / 0,23 / 0,16 | demasiado joven |
| La tierra de menos de 500 Myr, por lo que reinició su edad | arcos 56 %, choques 39 %, erupciones 2,7 % | — | — |
| Hundimiento de los márgenes, β 2–3 / > 3 | 1 470 / 1 661 m | McKenzie 1 573 / 1 769 ± 15 % | ✓ / ✓ |
| Magmatismo intraplaca oceánico; basaltos continentales | 1,36 AU; 0,033 AU | 1,3–1,7; 0,03 (aceptado)¹ | ✓ / ✓ |
| Grandes erupciones por 100 Myr | 4,1 | ~4,3 | ✓ |
| Potencia que mueve las placas: tirón / empuje | 0,76 / 0,24 | tirón ~0,50 con la succión | — (succión apagada) |
| Fosas que retroceden; velocidad media | 65 %; 1,05 cm/año | 62–78 %; 1,3–1,5 | ✓ / ✗ |
| Soldaduras por 100 Myr; mayor bloque de una placa | 0,44; 0,14 | — ; > 0,6 alguna vez (supercontinente) | ✗ |
| Tiempo del planeta (cinco a la vez) | 267 s | regla relativa: ≤ +5 % frente a la versión anterior, ≤ 330 s | ✓ |

¹ Los ~0,12 km³/año de Clift et al. (2009) incluyen otro magmatismo continental, además de los basaltos de las grandes erupciones (decisión del responsable, 2026-11-08).

## 3. Decisiones que quedan fijadas

- **Isostasia:** R4, con el sedimento contado como corteza y los parámetros de su §0. Los márgenes estirados suman el hundimiento de McKenzie sin contar dos veces.
- **Mesetas oceánicas:** corteza flotante no continental, con la isostasia del océano. Se acrecionan como los arcos.
- **Márgenes pasivos:** medidos como entidades que nacen en cada rift.
- **Caudal de las trazas:** 1,25·10⁵ km³/Myr. Los basaltos continentales dan 0,03 AU.
- **Succión de las losas:** programada y calibrada (`ε_suc` 0,05), pero apagada (`ε_suc` = 0) hasta arreglar los choques.
- **Tiempo:** regla relativa (≤ +5 % frente a la versión anterior medida en la misma tanda, techo de 330 s).

## 4. Ampliación futura

En orden de lo que más cambia el planeta:

1. **La resistencia de los choques.** Deben frenarse hasta pararse en decenas de Myr:
   - la India pasó de ~15 a ~5 cm/año en unos 10 Myr (Molnar y Stock 2009);
   - los orógenos viejos (los Urales, los Apalaches) están parados.
   - En el modelo convergen a 1,4–1,6 cm/año durante toda su vida y casi no se sueldan. Por eso destruyen corteza (1,3 AU, frente a 0,4–0,7) y no se forman supercontinentes.
2. **La succión de las losas** (`deriva/succion.rs`): lista y calibrada, apagada hasta arreglar los choques. Con ella encendida los continentes se acercan, pero mientras los choques no paran, la corteza se destruye (−13 %) y el mar baja.
3. **El supercontinente.** Los océanos de rift se abren pero casi nunca se cierran y el mayor bloque de una placa no pasa del 14 % del continente. Depende de 1 y 2.
4. **El exceso de fosas:** ×1,7 en longitud y un 20 % más de área que se hunde.
5. **La corteza demasiado joven:** la mitad de la tierra tiene menos de 100 Myr (en la Tierra, el 28 %). La reinician los arcos y los choques: depende de 1 y 4.
6. **La plataforma estrecha:** 10 km, frente a 43. El sedimento no rellena los márgenes, que ya se hunden como en McKenzie.
7. **El retroceso de las fosas, lento:** 1,0 cm/año, frente a 1,3–1,5 (Schellart et al. 2008).
8. **La topografía dinámica:** los empujes del manto profundo (plumas, la superelevación africana), de hasta unos cientos de metros. Va junto con los puntos calientes, que ya existen sobre el océano.

Y los que ya estaban en `TREE.md`: el sedimento al mar (5 AU, ≈1,8), el fondo viejo que ninguna fosa consume (15 % de más de 180 Myr) y los artefactos de representación de la deriva (§56 del diseño).

## 5. Dónde está todo

- **Diseño y resultados:** `incremento-4-diseno.md` (§1–§77) y las notas del responsable de `docs/design/terreno/`.
- **Informes de las rondas:** `docs/informes/`. Las últimas:
  - `2026-11-08_isostasia_litosfera.md`;
  - `2026-11-08_margenes_calientes_succion.md`;
  - `2026-11-08_cierre_y_modo_tierra.md`.
- **Mapa de etapas y parámetros:** `MAPA_DEL_MOTOR.md`.
- **Herramientas:** `herramientas/planeta_semillas.py` (tabla de cierre), `herramientas/analisis/supercontinente.py` y la batería (`herramientas/escenarios.py`).
