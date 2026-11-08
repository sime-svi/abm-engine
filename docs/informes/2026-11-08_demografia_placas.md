# Informe: demografía de placas y exceso de fosas (2026-11-08)

Nota de diseño: `docs/design/terreno/nota_demografia_placas.md`. Rama `terreno-demografia-placas`
(sin fusionar con `main`). Todo con 5 semillas (42, 1, 2, 3, 4) y la configuración por defecto
(`config/terreno.toml`). Diagnóstico: `cargo test --release -p abm-terreno --test tectonica --
--ignored diagnostico_demografia_placas --nocapture`.

## Resumen

- **El diagnóstico apunta a H1:** las placas se acumulan.
  - Hay ~20 placas de al menos el 1 % del planeta, frente a las 13 de Bird (1,5 veces).
  - Nacen ~21 placas por 100 Myr y mueren ~19.
  - Las señales de H2 (invasión) y H3 (convergencia difusa) no se cumplen.
- **La rama A no lo arregla.**
  - La soldadura por borde inactivo (A1) no actúa nunca, con ningún valor de su rango: los bordes de las placas grandes nunca se quedan quietos.
  - La captura de microplacas (A2) retira las placas fantasma, pero no reduce las placas grandes.
  - La fosa baja un 7 % (96 300 → 89 800 km), dentro de la dispersión entre semillas.
- **Tras la rama A ninguna rama encaja** (B y C no tienen señal), así que la ronda se para aquí, como pide la nota. La causa que muestra el diagnóstico no la cubre ninguna rama:
  - los rifts crean placas grandes (5 por 100 Myr), y solo mueren en las suturas (3 por 100 Myr);
  - las placas oceánicas grandes nunca se consumen enteras.
- **La rama A hace el planeta más lento:** 298 s solo, frente a 243 sin ella y un presupuesto de 275. No se fusiona con `main`.

## 1. Lo pedido

1. Diagnóstico del §0 (D1–D5) y respuestas al §11.
2. Aplicar las ramas del árbol del §2 que correspondan (A, B, C, en ese orden, midiendo tras cada una) y calibrar sus parámetros contra el §10.
3. Si el diagnóstico apunta a H4, parar y traerlo.
4. Tabla de cierre completa, con las filas de relieve.

## 2. Respuestas al §11

1. **La velocidad relativa ya se calcula a lo largo de cada borde, pero no se guardaba.**
   - `clasificar_bordes` calcula en cada celda de borde `v_rel = v_p − v_q`, con las rotaciones de las dos placas (y el empujón de apertura de un rift joven).
   - De ella guardaba solo la componente normal (`v_n`). El módulo servía para clasificar (por debajo de 0,05 cm/año el tramo pasa a transformante) y se descartaba.
   - Ahora se guarda en cada celda (`Borde::v_rel`). El máximo por par de placas, que pide `v_rel,pq`, se calcula en la regla A1 y en el diagnóstico.
2. **Las protecciones de las microplacas, además de la de la losa:**
   - toda placa de menos de `A_min` (0,5 % de la esfera, 2,55 Mkm²) se absorbía en el mismo paso en la vecina con más borde común; la única excepción era tener una losa que tira (`absorciones`);
   - `partir` no deja nacer mitades de menos de `A_min`, en rifts y saltos de dorsal;
   - la separación de un margen invadido (nota de fosas v4) sí crea microplacas desde (3·`L_borde`)² ≈ 0,2 Mkm², que viven mientras tienen losa.
   - No había ninguna otra protección. Además, **una placa consumida entera no moría**: sin área no tiene borde ni vecina, así que seguía «viva». Llegaba a haber 29 al final de la deriva, casi todas vacías.
3. **Para H4**: los datos de D1–D5 están en los §3–§6 de este informe. El reparto de potencia de las fuerzas, actualizado, va en el §3.6.

## 3. Diagnóstico del §0 (antes de tocar nada; código de `main`, 7c54bbd)

### 3.1 D1, placas

Placas con área, cada 50 Myr (media de 5 semillas). Bird (2003) proyectado en la malla tiene 58 placas: 13 de al menos el 1 % del planeta, 17 del 0,5 % y 26 del 0,1 %. Sus 15 mayores cubren el 95,8 %.

| Myr | Placas (sin área) | 15 mayores | ≥ 1 % / ≥ 0,5 % / ≥ 0,1 % |
|---|---|---|---|
| −450 | 16,2 (1,4) | 0,967 | 15,8 / 15,8 / 16,2 |
| −350 | 21,6 (3,8) | 0,891 | 19,6 / 20,8 / 21,2 |
| −250 | 24,4 (9,2) | 0,875 | 20,6 / 23,4 / 24,2 |
| −150 | 27,4 (15,2) | 0,872 | 21,6 / 24,4 / 27,0 |
| −50 | 26,6 (24,4) | 0,879 | 19,0 / 23,2 / 25,2 |
| 0 | 26,0 (29,2) | 0,881 | 18,4 / 22,8 / 24,2 |

| Por 100 Myr, tras el rodaje | Nacen | | Mueren |
|---|---|---|---|
| Rift | 5,8 ± 1,2 | Consumidas enteras | 6,3 ± 1,7 |
| Invasión (separación de un margen) | 15,5 ± 4,7 | Soldadas en un choque (sutura) | 3,1 ± 1,2 |
| Salto de dorsal | 0 | Absorbidas | 9,9 ± 3,6 |
| Nacimiento espontáneo | 0 | | |
| **Total** | **21,3** | **Total** | **19,3** |

- **Sobran placas grandes:** ~20 de al menos el 1 % frente a 13 (1,5 veces). Las de menos tamaño están en número terrestre (~25 de al menos el 0,1 %, frente a 26).
- **Nacen más de las que mueren** durante los primeros 300 Myr: el número pasa de 16 a ~27 y se estabiliza en los últimos 150 Myr.
- **Las placas que nacen por invasión son microplacas de vida corta,** el 73 % de los nacimientos. Mueren pronto: las consumen o las absorben.
- **Las grandes solo mueren en las suturas** (3 por 100 Myr), frente a 5,8 rifts.
- **Placas fantasma:** las consumidas enteras siguen «vivas» sin área (29 al final). Casi todas están vacías; alguna guarda una muestra oculta con corteza.

### 3.2 D2, fosas

Balance de la fosa activa tras el rodaje (km por 100 Myr). Se corrigió la longitud del evento de nacimiento inducido, que contaba las celdas de los dos lados del borde.

| Creada | | Perdida | |
|---|---|---|---|
| Nacimiento inducido | 111 400 ± 32 000 | Muerte sin convergencia | 52 200 ± 12 900 |
| Nacimiento espontáneo | 76 | Muerte por choque | 13 900 ± 4 400 |
| Invasión de transformantes | 4 800 ± 1 200 | Resto: acortamiento, geometría, divisiones y fusiones | 60 600 ± 13 600 |
| Separación de márgenes invadidos | 15 400 ± 4 000 | | |

- **Los nacimientos inducidos crean el 85 % de la fosa**, con mucho recambio: nacen ~215 tramos por 100 Myr y mueren ~200 sin convergencia.
- **La invasión aporta el 15 %**, por debajo del 25 % de la señal de H2.
- Edades al final: <10 Myr, 13 %; 10–25, 12 %; 25–50, 24 %; 50–100, 19 %; 100–200, 18 %; ≥200, 14 %.
- Convergencia: <1 cm/año, 10 %; 1–2, 17 %; 2–4, 27 %; 4–6, 19 %; 6–10, 17 %; ≥10, 11 %.

### 3.3 D3, convergencia (medias tras el rodaje)

| | Modelo | Bird | Razón |
|---|---|---|---|
| Fosa activa (SUB) | 96 300 ± 13 800 km | 51 300 | 1,88 |
| Convergencia difusa (OCB) | 13 000 ± 600 km | 17 400 | 0,75 |
| Choque (CCB) | 21 200 ± 1 800 km | 23 000 | 0,92 |

La convergencia difusa es más de la mitad de la de Bird: no hay señal de H3.

### 3.4 D4, bordes inactivos

- Entre pares de placas con la velocidad relativa máxima de todo su borde común por debajo de 0,2 cm/año: 1 000 ± 1 000 km de media, y casi siempre menos de 10 Myr seguidos.
- Con 0,5 cm/año, 0–14 000 km; con 1 cm/año, 3 000–20 000 km. Siempre de poca duración.
- **No hay «mucha longitud de borde inactivo».**

### 3.5 D5, por el origen de la placa (al final)

| Origen | Placas | Área | Fosa propia | Fondo que hunde |
|---|---|---|---|---|
| Inicial | 12,6 | 0,535 | 62 000 km | 1,63 km²/año |
| Rift | 11,6 | 0,462 | 41 300 km | 1,58 km²/año |
| Invasión | 1,8 | 0,003 | 1 200 km | 0,03 km²/año |

El exceso de fosa se reparte entre las placas iniciales y las de rift. Las microplacas de la invasión apenas cuentan.

### 3.6 Fuerzas (nota de fosas v2, medida 3, al final de la deriva)

- Tirón de la losa: 75 ± 2 % de la potencia motriz. Empuje de dorsal: 25 ± 2 %.
- Arrastre basal: 80 ± 7 % de la resistente. Choques: 20 ± 7 %.

## 4. Qué rama corresponde (§2)

| Hipótesis | Señal | Medido | ¿Se cumple? |
|---|---|---|---|
| H1 | Más de 1,5 veces las placas de Bird de su tamaño, nacimientos que superan a las muertes, o mucho borde inactivo | ~20 de ≥1 % frente a 13 (×1,5–1,6); nacen 21,3 y mueren 19,3; borde inactivo ~1 000 km | **Sí** (las dos primeras) |
| H2 | La invasión aporta más del 25 % de la fosa creada | 15 % | No |
| H3 | Difusa por debajo de la mitad de OCB y nacimientos inducidos dominantes | 0,75 de OCB (dominan los inducidos) | No |

**Rama A.**

## 5. Rama A: implementación, medida y calibración

**Implementación** (`deriva.rs: soldaduras`, `capturas`; al final del paso, tras los rifts):

- **A1.** Por cada par de placas vecinas, el máximo de `v_rel` a lo largo de todo su borde común.
  - Si está por debajo de `v_fus`, su contador `T_in` (en la historia del par) suma Δt; si no, vuelve a 0.
  - Con `T_in ≥ τ_fus`, sin fosa activa ni rift que se estire en su borde común, la mayor absorbe a la menor con `fundir`, que conserva la corteza. Evento `soldadura`.
- **A2.** Cada placa de menos de `A_micro` sin fosa activa propia suma Δt a `T_sf`; con `T_sf ≥ τ_micro`, pasa a la vecina con más borde común (empates, por índice). Evento `captura_microplaca`.
  - Sustituye a la absorción de toda placa de menos de `A_min`.
  - Una placa sin área ni borde pasa a la placa bajo sus muestras ocultas o, si está vacía, se retira.
- **Pruebas nuevas:**
  - `dos_placas_que_se_mueven_igual_se_sueldan_conservando_la_corteza`;
  - `dos_placas_con_una_fosa_activa_no_se_sueldan`;
  - `una_microplaca_sin_fosa_la_captura_la_vecina_con_mas_borde`.
  - India–Asia sigue pasando.

**Medida y calibración** (5 semillas):

| | §0 | A por defecto (0,2; 20; 0,5; 10) | `v_fus` 0,5, `τ_fus` 10 | `A_micro` 1, `τ_micro` 5 | Las dos | Objetivo (§10) |
|---|---|---|---|---|---|---|
| Soldaduras por borde inactivo / 100 Myr | — | 0 | **0** | 0 | 0 | — |
| Microplacas capturadas / 100 Myr | — | 0,3 | 0,3 | 0,5 | 0,5 | — |
| Fosa (km) | 96 300 | 89 800 | 89 800 | 94 400 | 94 400 | 51 300 ± 25 % |
| Fondo que se hunde (km²/año) | 4,64 | 4,48 | 4,48 | 4,52 | 4,52 | 2,7–3,4 |
| Difusa (km) | 13 000 | 11 000 | 11 000 | 12 500 | 12 500 | 17 400 ± 50 % |
| Choques (km) | 21 200 | 17 800 | 17 800 | 20 800 | 20 800 | 23 000 ± 25 % |
| Velocidad (cm/año) | 4,65 | 4,73 | 4,73 | 4,61 | 4,61 | 4,39 ± 25 % |
| Edad media del fondo al final (Myr) | 88 | 86 | 86 | 79 | 79 | no peor que 88 |
| Forsyth–Uyeda (fosa) | 0,58 | 0,51 | 0,51 | 0,50 | 0,50 | se mantiene |

- **A1 no actúa nunca**, ni en los extremos del rango (`v_fus` 0,5 cm/año, `τ_fus` 10 Myr): la variante da los mismos números bit a bit que la de por defecto. Las placas de este modelo siempre se mueven unas respecto a otras: cada una tiene su tirón o su empuje.
- **A2 retira las placas fantasma** (29 → ~1,5 al final) y captura unas pocas microplacas. Pero, como ya no se absorbe al momento toda placa de menos de 2,55 Mkm², las microplacas de la invasión viven más (1,8 → 4–5 al final). Las placas de al menos el 0,1 % pasan de 24 a 27; Bird tiene 26.
- **La fosa y el fondo que se hunde bajan algo** (−7 % y −3 %), dentro de la dispersión entre semillas (±10 %). **Las placas de al menos el 1 % no bajan** (~20).
- Los parámetros se quedan en los valores de la nota: no hay un valor del rango que acerque más al §10.

## 6. Después de A: ninguna rama encaja

- H1 sigue: ~20 placas de ≥1 % y nacimientos por encima de las muertes.
- H2: la invasión aporta el 16 % de la fosa creada, por debajo del 25 %.
- H3: la difusa es 0,63 de OCB, por encima de la mitad.

Así que no corresponden B ni C. Las reglas de A tampoco actúan sobre la causa que muestra el diagnóstico. **Se para aquí** (nota, cabecera: «si ninguna rama encaja, se trae aquí con los datos del §0»).

## 7. Tabla de cierre (5 semillas, planeta entero, con la rama A)

| | Modelo | Tierra u objetivo | `main` (ronda g) | |
|---|---|---|---|---|
| Tierra emergida | 0,333 ± 0,023 | 0,284 | 0,326 | ✗ |
| Altura media de la tierra | 1 136 ± 100 m | 671 m | 1 235 | ✗ |
| Tierra por debajo de 200 m | 0,081 ± 0,007 | 0,283 | 0,087 | ✗ |
| **Tierra por encima de 2 000 m** | **0,150 ± 0,026** | **0,053** (roca sin hielo; con el hielo, 0,107) | — | ✗ |
| Pendiente media (±25 km) | 0,0153 | 0,0052 | 0,0167 | ✗ |
| Anchura de la plataforma | 8,8 ± 1,3 km | 42,8 km | 8,1 | ✗ |
| Nivel del mar al final de la deriva | −98 ± 83 m | 0 (la Tierra real con la regla: −150) | −164 | — |
| Profundidad media del océano | 3 952 ± 131 m | 3 667 m | 3 915 | ✗ |
| Corteza máxima al final / en toda la deriva | 77 ± 3 / 125 ± 10 km | 70–80 km | 80 / 123 | ✓ / ✗ |
| Edad media del fondo; más de 180 Myr | 86 ± 9 Myr; 13,7 % | 64,2; < 1 % | 88; 13,1 % | ✗ |
| `f_cont`; cambio de volumen | 0,330; −7,0 % | 0,40; ±10 % | 0,323; −8,0 % | ✗ / ✓ |
| Velocidad de las placas | 4,73 ± 0,18 cm/año | 4,39 | 4,65 | ✓ |
| Sedimento al mar | 4,85 ± 1,18 AU | ≈1,8 | 5,06 | ✗ |
| Longitud de fosa | 89 800 ± 9 200 km | ~51 000 ± 25 % | 96 300 | ✗ (×1,75) |
| Área que se hunde | 4,48 ± 0,48 km²/año | 2,7–3,4 | 4,64 | ✗ |
| Choques: longitud | 17 800 ± 2 700 km | CCB 23 000 ± 25 % | 21 200 | ✓ |
| Convergencia difusa: longitud | 11 000 ± 1 200 km | OCB 17 400 ± 50 % | 13 000 | ✓ |
| Placas al final | 28,2 ± 5,9 | Bird 58 en la malla | — | — |
| Placas de al menos el 1 % | 19,6 ± 2,9 | 13 | — | ✗ |
| Las 15 placas mayores | 0,890 del planeta | 0,958 ± 0,10 | — | ✓ |
| Márgenes pasivos: vida; edad media y máxima al final | 101 Myr; 196 y 637 Myr | 130–180; ~104 y ~180 | 103; 182 y 646 | ✗ |
| Choques: pérdida total | 1,20 ± 0,21 AU | 0,4–0,7 | 1,38 | ✗ |
| Rupturas por 100 Myr; las que siguen una sutura | 5,0; 23 % | ~4–7; apreciable | 5,8; 29 % | ✓ |
| Ciclos de supercontinente en 450 Myr | 2,4 ± 2,1 | ~1,1 | 3,6 | ✗ (blando) |
| Tiempo del planeta | 319 ± 21 s con cinco a la vez; **298 s solo** | 275 s | 264; 243 | ✗ |

**La tierra por encima de 2 000 m.** La nota da ~11 % para ETOPO: es la superficie **con el hielo** (Antártida y Groenlandia, a 2–4 km). El modelo no tiene casquetes, así que lo comparable es la roca con su rebote isostático: 5,3 %. El planeta tiene casi tres veces eso. El 28 % de la nota era otra medida: la altura isostática al final de la deriva, antes de la génesis geológica y del clima.

**El tiempo.** La regla A apenas cuesta (~1,5 s por deriva). Lo que crece, en las cinco semillas, es el reparto del sedimento al mar (21,9 → 37,7 s por deriva en la semilla 42), sin que desborde sedimento. Sin perfilador en la máquina no se ha podido ver por qué. Con la rama A, el planeta no cabe en el presupuesto de 275 s.

## 8. Lo que dice el diagnóstico sobre la causa

- **Las placas grandes se acumulan porque casi no mueren.**
  - Los rifts crean ~5 por 100 Myr, el ritmo de la ruptura de Pangea.
  - Las grandes solo mueren en las suturas (~3 por 100 Myr).
  - Las placas oceánicas grandes no se consumen enteras: lo que muere consumido son las microplacas de la invasión.
- **En la Tierra, las placas oceánicas grandes sí desaparecen en las fosas.**
  - Farallón se fue partiendo en trozos (Juan de Fuca, Cocos, Nazca), que se consumen.
  - Kula e Izanagi se hundieron enteras.
  - En el modelo, una placa con dorsal siempre recibe fondo nuevo, y su borde no deja nunca de moverse: A1 no tiene sobre qué actuar.
- **Por eso sobra fosa.** Con más placas grandes hay más borde: el perímetro de la semilla 42 es de 359 000 km, frente a los 260 000 de Bird (+38 %). Además, una parte mayor del borde es fosa activa: el 27 %, frente al 20 %.
  - Los nacimientos inducidos crean el 85 % de la fosa y la renuevan deprisa: unos 215 tramos nuevos por 100 Myr y otros tantos que mueren sin convergencia.

**Opciones para el diseño** (decide el responsable):
1. **Una muerte de placas grandes** que encaje con la Tierra.
   - Una placa que pierde su dorsal y se hunde por todos sus bordes, como Kula o Farallón, se parte en trozos o se consume.
   - O una placa oceánica que baja de cierto tamaño pasa a la vecina que la arrastra.
2. **Menos nacimientos de placas grandes.** La regla de rifts está calibrada con el ritmo de la ruptura de Pangea, que fue un momento de dispersión. Una tasa media más baja, o una que dependa del aislamiento térmico (nota de rifts §2), daría menos placas nuevas fuera de los supercontinentes.
3. **La fracción del borde que es fosa** (27 % frente a 20 %). La rama C (`X_ini` hasta 200 km) no tiene señal, pero haría más exigente el nacimiento inducido, que domina la creación de fosa. Se puede medir sin cambiar el diseño.
4. **Revisar o quitar la rama A.** A1 no actúa en este modelo. A2 retira las placas fantasma, que conviene conservar, pero deja vivir más microplacas de la invasión y el planeta va más lento.

## 9. Estado del código

- `main` (7c54bbd) no tiene la rama A.
- La rama `terreno-demografia-placas` tiene:
  - el diagnóstico (D1–D5, `diagnostico_demografia_placas`) y la longitud corregida de los nacimientos inducidos (055da72);
  - la rama A con sus tres pruebas, el informe y el mapa del motor (este commit).
- Las pruebas pasan. La batería no: con la rama A, el planeta se pasa del presupuesto de tiempo y los márgenes del planeta no están puestos al día. Por eso no se fusiona con `main` hasta que decida el responsable.
