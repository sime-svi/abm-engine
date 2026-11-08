# Informe: nota de demografía de placas v2 y nota de rifts v2 (2026-11-08, tercera ronda)

Encargo del responsable:
- implementar la nota de demografía v2: rama D (subducción de dorsales) y medida D6, sin la soldadura A1;
- implementar la nota de rifts v2: el reloj del supercontinente;
- calibrar `λ_A` y `τ_ais` con el número de placas grandes (~13);
- medir placas y fosas, sin la tabla de cierre del relieve (la isostasia la va a cambiar);
- presupuesto, 275 s.

Notas: `docs/design/terreno/nota_demografia_placas.md` (v2) y `nota_rifts.md` (v2). Rama `terreno-demografia-placas`, commit `cf1b87d`. Medidas con 5 semillas (42, 1, 2, 3, 4), tras el rodaje: `cargo test --release -p abm-terreno --test tectonica -- --ignored diagnostico_demografia_placas --nocapture` (SEMILLAS, VARIANTES).

## Resumen

- **Con la rama D «de frente» y el reloj de los rifts, las placas y las fosas se acercan a la Tierra:**
  - las placas de al menos el 1 % del planeta son 13,8 de media (Bird, 13), frente a 18,3 antes;
  - la fosa mide 71 800 km (×1,40 de Bird), frente a 89 800 (×1,75);
  - se hunden 3,55 km² de fondo al año (objetivo 2,7–3,4), frente a 4,48;
  - la velocidad es de 4,55 cm/año y las 15 placas mayores cubren el 97 % del planeta (Bird, 95,8 %).
- **El fondo oceánico envejece:** 87 Myr de media en la deriva y 110 al final, frente a 72 y 86 (objetivo: no peor que 88; Tierra, 64). Con menos rupturas hay menos océano nuevo, y el fondo viejo que ninguna fosa consume (§56) pesa más.
- **La rama D con la letra de la nota actúa sobre todo en los puntos triples.**
  - Cualquier dorsal que toca una fosa por su costado cumple «a menos de `d_dc`»: la regla se come cada paso una franja de hasta 300 km a lo largo de la fosa (538 veces por 100 Myr, 132 000 km de fosa apagada).
  - Se implementa la lectura de la nota, la franja **entre** la dorsal y la fosa: la fosa tiene que estar delante de la dorsal (`θ_dc` = 45°). Así actúa 316 veces por 100 Myr y deja mejor las placas, la fosa y el fondo que se hunde.
- **D6.** Sin la regla hay unos 400 encuentros entre dorsales y fosas por 100 Myr, que duran ~3 Myr. El 94 % acaba con la placa viva: la dorsal se aleja o la fosa muere.
- **Calibración.** `λ_A` = 4·10⁻⁴ y `τ_ais` = 100 Myr, los valores de la nota, dan las ~13 placas grandes; ninguna vecina lo mejora.
  - No se forma ningún supercontinente: el bloque continental mayor no pasa del 46 % del continente.
  - Por eso la duración de los supercontinentes no se puede medir y `τ_ais` queda en su supuesto.
- **Tiempo:** el planeta tarda 256 s, también en la batería (presupuesto 275). Ninguna etapa se pasa.
- **La batería:** 35 métricas del planeta salen de sus márgenes (otra realización del planeta). No se mueven hasta después de la isostasia, y `main` no se toca.

## 1. Lo hecho

**A1 fuera.**
- Se retiran la regla, su contador, `v_fus`, `τ_fus` y sus dos pruebas. Se quedan A2 y la retirada de las placas fantasma.
- Como A1 no actuaba nunca, la deriva sin la rama D ni el reloj reproduce la ronda anterior: fosa 89 758 km, 4,48 km²/año, 4,73 cm/año.

**Rama D** (`subduccion_de_dorsales`, en los eventos de placas tras los rifts; `d_dc` 300 km, `θ_dc` 45°, interruptor `subduccion_dorsales`):
- **Cuándo actúa.** Un tramo de dorsal entre `p` y `r` está en encuentro si `p` tiene una fosa activa bajo una **tercera** placa `u` que está delante del tramo (a menos de `θ_dc` de la dirección de apertura hacia dentro de `p`) y a menos de `d_dc`, medido por dentro de `p`.
- **Qué hace.** La franja de `p` entre ese tramo y la fosa pasa a `r`, con la conservación de `fundir`:
  - la dorsal ya no separa dos placas;
  - la fosa de enfrente se apaga: `r` no hereda su estado ni a `p` como antepasada;
  - el borde nuevo con `u` se clasifica en el paso siguiente según su movimiento;
  - si `p` se queda sin muestras, se retira.
- **Dos decisiones de realismo, con sus medidas:**
  - **La tercera placa.** Una dorsal y una fosa entre las mismas dos placas son un solo borde que cambia de tipo cerca de su polo de rotación, no una dorsal que llega a una fosa.
  - **La fosa delante.** Con la letra de la nota («a menos de `d_dc` del tramo»), en un punto triple la dorsal toca la fosa por su costado. La regla se comería cada paso una franja de hasta 300 km a lo largo de la fosa: el punto triple correría a ~30 cm/año, cuando los reales migran a unos cm/año. La franja «entre la dorsal y la fosa» de la nota (Farallón) es la de delante. Con `θ_dc` = 180°, la letra.

**D6.** Los encuentros de cada paso, también con la regla apagada (`Deriva::dorsal_fosa_paso`; `dorsal_fosa_n` y `dorsal_fosa_km` en las métricas).

**Rifts v2** (nota de rifts v2 §6.1):
- Cada muestra lleva el Myr del último reinicio de su bloque (`reinicio_myr`).
- El reloj de un bloque es el tiempo desde su reinicio más reciente. Al juntarse dos bloques se toma el menor, y la corteza nueva no lo cambia.
- `p = 1 − exp(−λ_A·A·min(1, t/τ_ais)·Δt)`.
- Lo reinician un rift (los dos trozos) y una sutura que junta dos continentes (los bloques de las dos placas que tocan el frente).
- Los cratones de partida empiezan con un reloj al azar en [0, `τ_ais`]: la nota no lo fija, y así no se sincronizan sus rupturas.

**Métricas nuevas:**
- en el balance, los supercontinentes que se forman y se rompen, su duración, las subducciones de dorsal, las placas retiradas y la fosa apagada;
- en el diagnóstico, D6, las rupturas, las placas de al menos el 1 % (media de los pasos) y el bloque continental mayor.

**Pruebas nuevas:**
- una dorsal que llega a la fosa muere y su franja pasa a la placa del otro lado, conservando la corteza;
- con la rama D apagada solo se miden los encuentros;
- un bloque recién unido no se rompe y uno viejo sí, y sus trozos empiezan el reloj al romperse.

India–Asia apaga la rama D en su escenario, como ya apagaba la invasión: en las esquinas de sus meridianos artificiales la regla movía la rotura de la losa de 22 a 25 Myr.

## 2. Cada cambio por separado y juntos (5 semillas, tras el rodaje)

| | Sin D ni reloj (≈ ronda anterior) | Solo el reloj | D con la letra y reloj | **D de frente y reloj (por defecto)** | Tierra u objetivo |
|---|---|---|---|---|---|
| Placas de al menos el 1 % (media de los pasos) | 18,3 ± 2,9 | 14,9 ± 2,8 | 15,1 ± 4,8 | **13,8 ± 2,4** | ~13 (Bird) |
| Rupturas por 100 Myr | 5,0 | 3,6 | 4,1 | 3,5 | 4–7 en la ruptura de un supercontinente; menos fuera |
| Fosa (km) | 89 758 | 77 851 | 86 657 | **71 807** | 51 309 ± 25 % |
| Fondo que se hunde (km²/año) | 4,48 | 3,79 | 3,99 | **3,55** | 2,7–3,4 |
| Convergencia difusa (km) | 11 033 | 9 910 | 13 842 | 9 789 | 17 447 ± 50 % |
| Choques (km) | 17 759 | 17 295 | 19 308 | 16 561 | 23 003 ± 25 % |
| Velocidad (cm/año) | 4,73 | 4,40 | 4,44 | 4,55 | 4,39 ± 25 % |
| Edad media del fondo: en la deriva / al final (Myr) | 72 / 86 | 85 / 109 | 80 / 102 | **87 / 110** | no peor que 88 (Tierra 64) |
| Forsyth–Uyeda (fosa) | 0,51 | 0,52 | 0,47 | 0,53 | se mantiene |
| Nacimientos inducidos (km de fosa por 100 Myr) | 106 271 | 91 660 | 144 290 | 83 863 | — |
| Subducciones de dorsal por 100 Myr | 0 | 0 | 538 | 316 | — |
| Fosa que apagan (km por 100 Myr) | 0 | 0 | 132 063 | 71 748 | — |
| Placas retiradas por la rama D por 100 Myr | 0 | 0 | 0,04 | 0 | — |
| Bloque continental mayor: media / máximo | — | 0,29 / 0,41 | 0,27 / 0,39 | 0,30 / 0,46 | Pangea, ~0,9 |

- **El reloj de los rifts es lo que baja las placas grandes** (18,3 → 14,9): hay menos rupturas (5,0 → 3,6 por 100 Myr).
- **La rama D de frente baja la fosa y el fondo que se hunde.** Apaga fosas de placas estrechas entre su dorsal y su fosa, y con ello las placas grandes llegan a 13,8. No mata placas enteras: sus franjas son trozos.
- **Con la letra, la rama D apaga el doble de fosa**, pero nacen 1,6 veces más fosas inducidas (144 000 km por 100 Myr): recambio en los puntos triples, sin ganancia.
- **El fondo envejece con menos rupturas:** de 86 a 109–110 Myr al final.

**Placas y fosas cada 50 Myr** (por defecto, media de las semillas):

| Myr | Placas con área | Las 15 mayores | ≥ 1 % / ≥ 0,5 % / ≥ 0,1 % | Fosa / difusa / choque (km) |
|---|---|---|---|---|
| −450 | 14,2 | 0,999 | 13,6 / 14,0 / 14,2 | 71 329 / 1 452 / 20 792 |
| −350 | 14,6 | 0,999 | 13,6 / 13,6 / 13,8 | 66 904 / 5 424 / 13 747 |
| −250 | 17,0 | 0,975 | 14,0 / 15,4 / 16,0 | 73 409 / 6 289 / 15 773 |
| −150 | 19,2 | 0,985 | 14,0 / 15,2 / 16,4 | 70 534 / 13 305 / 18 672 |
| −50 | 18,8 | 0,971 | 14,0 / 16,0 / 17,6 | 77 650 / 9 170 / 16 540 |
| 0 | 18,8 | 0,966 | 14,2 / 16,8 / 18,2 | 79 250 / 13 532 / 19 931 |

Bird en la malla: 58 placas; las 15 mayores, 0,958; de ≥ 1 %, 13; de ≥ 0,5 %, 17; de ≥ 0,1 %, 26. Faltan placas pequeñas (18 de ≥ 0,1 %, frente a 26).

## 3. D6: encuentros entre dorsales y fosas

| | Empiezan por 100 Myr | Duran (Myr) | Acaban por la regla | Con la placa de la franja muerta | Con ella viva | Dorsal en encuentro (km) |
|---|---|---|---|---|---|---|
| Sin la regla, medidos con la letra | 385–460 | 4,5 | — | 7–8 % | 92–93 % | 4 800–6 000 |
| Sin la regla, medidos de frente | 405 | 2,9 | — | 6 % | 94 % | 1 990 |
| Con la regla, la letra | 882 | 1,4 | 63 % | 4 % | 33 % | 4 890 |
| **Con la regla, de frente** | 435 | 0,7 | 73 % | 2 % | 25 % | 790 |

- **Sin la regla, los encuentros son contactos breves** (~3 Myr) y casi siempre acaban solos: la dorsal se aleja o la fosa muere.
- **En la Tierra de hoy hay unos pocos**, que duran decenas de Myr: Chile, Cascadia (Juan de Fuca) y Woodlark, entre otros. El modelo tiene muchos más: tiene más placas pequeñas y más fosa.
- **Con la regla de frente**, el 73 % de los encuentros acaba por la regla en menos de un paso. Ninguna placa se retira entera.

## 4. Calibración de `λ_A` y `τ_ais` (5 semillas, rama D de frente)

| | Rupturas por 100 Myr | Placas ≥ 1 % (media) | Fosa (km) | Se hunden (km²/año) | Edad del fondo, deriva / final (Myr) | Bloque mayor, media / máximo |
|---|---|---|---|---|---|---|
| `λ_A` 2·10⁻⁴, `τ_ais` 100 | 1,6 | 11,8 ± 2,5 | 65 450 | 3,19 | 93 / 125 | 0,26 / 0,36 |
| `λ_A` 3·10⁻⁴, `τ_ais` 100 | 2,9 | 13,9 ± 3,6 | 72 067 | 3,50 | 86 / 110 | 0,28 / 0,42 |
| **`λ_A` 4·10⁻⁴, `τ_ais` 100** | 3,5 | **13,8 ± 2,4** | 71 807 | 3,55 | 87 / 110 | 0,30 / 0,46 |
| `λ_A` 4·10⁻⁴, `τ_ais` 50 | 4,2 | 16,4 ± 4,1 | 83 411 | 3,96 | 73 / 95 | 0,25 / 0,40 |
| `λ_A` 4·10⁻⁴, `τ_ais` 200 | 3,1 | 14,1 ± 3,7 | 72 307 | 3,63 | 84 / 105 | 0,29 / 0,42 |

- **`λ_A` se queda en 4·10⁻⁴ y `τ_ais` en 100 Myr.**
  - Es la combinación más cercana a las 13 placas grandes y la de menos dispersión entre semillas.
  - Con 3·10⁻⁴ sale lo mismo, dentro de la dispersión. Con 2·10⁻⁴ se queda corta (11,8). Con `τ_ais` 50 sobran placas (16,4) y con 200 no mejora (14,1).
- **La duración de los supercontinentes no se puede usar:** no se forma ninguno.
  - El bloque continental mayor es como mucho el 36–46 % del continente; Pangea reunía ~90 %.
  - El índice de supercontinente mide continentes que se tocan, no soldados, y oscila de 0,3 a 0,9 en 10 Myr. Sus «supercontinentes» duran 9–42 Myr.
  - `τ_ais` queda como supuesto de la nota (100 Myr), el mejor de los tres probados para las placas grandes.
- Con menos rupturas el fondo envejece: el número de placas grandes y la edad del fondo tiran en sentidos opuestos.

## 5. Tiempo

| | Planeta solo (semilla 42) | En la batería | Génesis |
|---|---|---|---|
| `main` (7c54bbd) | 243 s | 250,4 s | — |
| Ronda anterior (rama A) | 298 s | — | — |
| **Esta ronda** | **256 s** | **256,7 s** | 239 s (93 %) |

No se pasa el presupuesto de 275 s.

## 6. La batería

- Agentes, sin cambios.
- Planeta: tiempo dentro, pero **35 métricas fuera de sus márgenes**. Es otra realización del planeta: placas, fosas, edad del fondo, relieve, lluvia, biomas.
  - Las de placas y fosas van hacia la Tierra: fosa 67 190 km, área que se hunde 3,30 km²/año, 16 placas al final.
  - La edad del fondo se aleja: 111 Myr al final, frente a un margen de 67–75.
- Los márgenes no se mueven ahora: la isostasia continental va a cambiar el relieve y, con él, la erosión, el sedimento y el clima. Por eso la rama no se fusiona con `main`.

## 7. Lo que queda (decide el responsable)

1. **La rama D: la versión de frente** (`θ_dc` 45°, la recomendada y por defecto) o la letra (`θ_dc` 180°). La de frente es la que da la Tierra en placas, fosa y fondo que se hunde.
2. **La edad del fondo** (110 Myr al final, objetivo ≤ 88). Con las placas grandes en su número, lo que la sostiene es el fondo viejo que ninguna fosa consume: los artefactos de representación de §56 y la falta de nacimientos de fosa en los márgenes viejos.
3. **Los interruptores** `subduccion_dorsales` y `θ_dc` se retiran cuando decida (CLAUDE §5).
4. **La tabla de cierre del relieve y los márgenes de la batería, tras la isostasia** (estudio de isostasia continental).

## 8. Estado

- Rama `terreno-demografia-placas`, commit `cf1b87d`, subida; `main` sigue en 7c54bbd.
- Pruebas en verde (`cargo test --workspace`), `cargo fmt` y `cargo clippy -D warnings` limpios. La batería, con el tiempo dentro y 35 márgenes del planeta por mover (§6).
