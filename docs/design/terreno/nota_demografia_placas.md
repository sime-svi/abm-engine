# Nota de diseño: demografía de placas y exceso de fosas

Documento de diseño. Complementa a diseno_relieve_tectonico.md (incremento 4 del terreno), a nota_ciclo_fosas.md (versión 6), a nota_rifts.md y a adenda_balance_corteza.md (versión 9).

Estado: versión 2. Diagnóstico hecho; se cierran las ramas y se añade la muerte de placas por subducción de dorsales (rama D).

Cambios de la versión 2:
- **Rama A.** Se retira la soldadura por borde inactivo (A1): en el modelo no actúa nunca, porque las placas grandes siempre se mueven unas respecto a otras. Se mantienen la captura de microplacas (A2) y la retirada de las placas fantasma, que antes seguían "vivas" sin área.
- **Rama D, nueva.** Cuando una dorsal llega a una fosa, deja de crear fondo y la placa que había entre ambas se rompe y es capturada. Es como murieron en la Tierra las placas oceánicas grandes (Farallón, Kula), y es lo que el diagnóstico echa en falta: en el modelo las placas grandes nacen en los rifts (~5 por 100 Myr) y solo mueren en las suturas (~3).
- **Objetivo de tierra por encima de 2 km:** 5,3 %, la cifra de ETOPO sin hielo, que es lo que representa el modelo (antes ~11 %, con el hielo de Antártida y Groenlandia).
- **Ramas B y C:** se descartan, porque el diagnóstico no da señal para ninguna de las dos.

---

## 0. Diagnóstico (primero)

Todo con 5 semillas: cada 50 Myr a lo largo de la deriva, y al final.

**D1. Placas.**
- Número de placas.
- Distribución acumulada de sus áreas, frente a las 52 placas de Bird proyectadas en la malla.
- Nacimientos de placas por causa (rift, invasión, nacimiento espontáneo, otras) y muertes por causa (consumida entera, soldada en un choque, absorbida), por cada 100 Myr.

**D2. Fosas.**
- Longitud total a lo largo del tiempo.
- Balance por causa: longitud creada por nacimiento inducido, por nacimiento espontáneo, por crecimiento en la invasión y por división de placas; longitud perdida por muerte sin convergencia y por choque.
- Distribución de edades de las fosas y de su convergencia.

**D3. Convergencia.** Reparto de la longitud de borde convergente entre fosa activa, convergencia difusa y choque, frente a Bird: SUB ≈ 51 300 km, OCB ≈ 17 400 km y CCB ≈ 23 000 km.

**D4. Bordes inactivos.** Longitud de borde entre placas cuya velocidad relativa es menor que 0,2 cm/año, y cuánto tiempo llevan así.

**D5. Por placa.** Área que se hunde y longitud de fosa de cada placa, distinguiendo las placas nacidas de rifts, de invasiones y de nacimientos espontáneos.

**D6. Encuentros entre dorsales y fosas** (versión 2). Cuántas veces por 100 Myr un tramo de dorsal llega a menos de d_dc de una fosa donde se hunde una de sus dos placas, y qué pasa después.

---

## 1. Fenómeno real y objetivo

### Cómo es en la Tierra

**Reparto de la convergencia.** La Tierra tiene unos 51 000 km de fosa, que hunden unos 2,7 km² de fondo al año. Además tiene unos 17 400 km de convergencia sin fosa, donde las placas se acercan despacio y se arrugan, y unos 23 000 km de choques entre continentes (Bird 2003).

**Las placas también nacen y mueren.**
- **Nacen** al partirse un continente, al separarse microplacas en los extremos de las fosas y al separarse los márgenes.
- **Mueren** de tres formas:
  - consumidas enteras en una fosa, como fue desapareciendo la placa de Farallón al dividirse;
  - soldadas a una vecina cuando el borde entre ambas deja de moverse, porque un borde que no se mueve ya no es un borde;
  - capturadas por la vecina: en California, restos de Farallón que se quedaron sin fosa activa pasaron a moverse con la placa del Pacífico.

**El número de placas no crece sin límite.** Bird distingue 52 placas: unas pocas grandes que ocupan casi todo el planeta y muchas pequeñas, cuyo número sigue una ley de potencia con el tamaño. Una ley de potencia quiere decir que hay muchas más placas pequeñas que grandes, en una proporción fija.

**La invasión es estrecha.** La invasión de un océano por una fosa se ha observado a lo largo de márgenes transformantes, donde el puente continental es estrecho: las Antillas Menores y el arco de Escocia.

### Qué pasa hoy en el modelo

- Se hunden 4,6 km² de fondo al año, frente a 2,7–3,4.
- Las fosas miden 1,89 veces las de la Tierra. El salto, desde 1,37, llegó justo al añadir los rifts, que multiplican las placas y los bordes.
- Hay más arcos y montañas de la cuenta (el 28 % de la tierra está por encima de 2 km, frente al ~11 % real) y llega demasiado sedimento al mar (5,1 AU frente a ~1,8).

### Objetivo

Que las placas y las fosas nazcan y mueran a un ritmo terrestre, de modo que su número, su longitud y el fondo que se hunde salgan como en la Tierra sin tocar las fuerzas a mano.

---

## 2. Hipótesis y árbol de decisión

| Hipótesis | Señal en el §0 | Rama |
|---|---|---|
| H1. Las placas se acumulan: nacen más de las que mueren | Más de 1,5 veces las placas de Bird de su mismo tamaño, o nacimientos que superan a las muertes de forma sostenida, o mucha longitud de borde inactivo (D4) | A |
| H2. La invasión alarga demasiado las fosas | La invasión aporta más del 25 % de la longitud de fosa creada (D2) | B |
| H3. Los nacimientos inducidos convierten en fosa la convergencia que en la Tierra queda difusa | La convergencia difusa es menos de la mitad de la de Bird (OCB) y los nacimientos inducidos dominan (D2, D3) | C |
| H4. Ninguna de las anteriores: placas y fosas en número terrestre, pero se hunde demasiado | — | Se trae aquí con el §0 para diseñar las fuerzas |

Si se cumplen varias, las ramas se aplican en el orden A, B, C, midiendo después de cada una.

---

## 3. Entidades y componentes

No hay entidades nuevas. Por cada par de placas vecinas hace falta un contador de tiempo con el borde inactivo, y por cada placa, un contador de tiempo sin fosa activa propia.

---

## 4. Variables de estado

| Símbolo | Significado | Tipo | Rango / unidades |
|---|---|---|---|
| v_rel,pq | Velocidad relativa máxima a lo largo de todo el borde común entre las placas p y q | real | cm/año |
| T_in,pq | Tiempo seguido con v_rel,pq < v_fus | real | Myr |
| T_sf,p | Tiempo seguido que la placa p lleva sin ninguna fosa activa propia, ni como placa que baja ni como placa de arriba | real | Myr |
| A_p | Área de la placa p | real | Mkm² |

---

## 5. Parámetros ajustables

| Parámetro | Significado | Por defecto | Unidades | Origen |
|---|---|---|---|---|
| d_dc | Distancia entre una dorsal y la fosa donde se hunde una de sus placas a partir de la cual la dorsal muere | 300 | km | Supuesto: las microplacas que quedaron frente a California tenían unos cientos de km de anchura al ser capturadas; rango 150–500 |
| A_micro | Área por debajo de la cual una placa es microplaca | 0,5 | Mkm² | Supuesto, del orden de las microplacas de Bird; rango 0,2–1 |
| τ_micro | Tiempo sin fosa activa tras el que una microplaca es capturada | 10 | Myr | Supuesto; rango 5–30 |
| X_ini | Convergencia acumulada para el nacimiento inducido (ya existe) | 150 | km | Rango 100–200 (Gurnis et al. 2004; Li y Gurnis 2023) |

Se calibran contra los objetivos del §10, con 5 semillas.

---

## 6. Reglas por paso tectónico

### Rama A: muerte de placas

**A1. Soldadura por borde inactivo.** Retirada en la versión 2: no actuaba nunca.

**A2. Captura de microplacas.**
- Si A_p < A_micro y la placa p no tiene fosa activa propia, T_sf,p ← T_sf,p + Δt; si la tiene, T_sf,p ← 0.
- Si T_sf,p ≥ τ_micro, la placa p pasa a la vecina con la que comparte más longitud de borde, con la misma conservación que A1.

Esta regla sustituye a la protección actual de las microplacas, que no se absorben mientras tienen losa. Una microplaca cuya losa ya se rompió o murió queda libre para ser capturada.

### Rama B: invasión solo por bordes transformantes

La invasión (nota_ciclo_fosas.md, 6.4) avanza solo a lo largo de fallas transformantes entre placas, no por márgenes pasivos dentro de una placa, como en los casos observados. El cierre de los océanos viejos queda a cargo sobre todo de los rifts y de la reorganización de las placas, que son lo que hizo bajar la edad del fondo de 119 a 88 Myr.

### Rama D: subducción de dorsales y muerte de placas oceánicas (versión 2)

En la Tierra, una placa oceánica grande muere cuando la dorsal que la alimenta llega a la fosa que la consume. El fondo junto a la dorsal es tan joven y caliente que flota y no se hunde bien: la dorsal deja de abrirse en ese tramo, la fosa se apaga y la placa que quedaba entre ambas se rompe en microplacas, que la placa del otro lado de la dorsal acaba capturando. Así desapareció la placa de Farallón frente a Norteamérica: sus restos pasaron a la placa del Pacífico, y la fosa se convirtió en la falla de San Andrés. Así desaparecieron también Kula y otras placas del Pacífico.

Regla, para cada tramo de dorsal activa entre las placas p y r:
- Si a menos de d_dc del tramo hay una fosa activa en la que se hunde p, es decir, si la franja de p entre la dorsal y la fosa es más estrecha que d_dc:
  1. el tramo de dorsal muere y deja de crear fondo;
  2. la franja de p entre la dorsal muerta y la fosa se separa y pasa a la placa r, con la misma conservación que A2;
  3. el tramo de fosa frente a ella muere, y el borde nuevo entre r y la placa de arriba se clasifica según su movimiento (a menudo, transformante).
- Si p se queda sin área, se retira como placa, igual que las fantasma.

Todo se recorre en un orden fijo de tramos.

### Rama C: nacimiento inducido más exigente

Se mueve X_ini dentro de 100–200 km, hasta que la longitud de convergencia difusa se acerque a la de Bird. Si con 200 km no basta, se trae aquí.

---

## 7. Eventos discretos

- **Soldadura de placas:** paso, placas, longitud del borde que desaparece, tiempo que llevaba inactivo.
- **Captura de microplaca:** paso, microplaca, placa que la captura, área.

---

## 8. Métricas por paso

Las de D1–D5, de forma continua.

---

## 9. Interacción con otros sistemas y orden dentro del paso tectónico

Las reglas A1 y A2 se aplican al final del paso, después de los rifts, para que una placa recién nacida no se suelde en el mismo paso. Los contadores T_in y T_sf viajan con las placas; cuando dos placas se funden, el contador de la placa resultante es el de la mayor.

---

## 10. Validación y determinismo

| Magnitud | Objetivo | Hoy |
|---|---|---|
| Longitud de fosa | ≈51 000 km ± 25 % (Bird SUB, medida igual) | ×1,89 |
| Fondo que se hunde | 2,7–3,4 km²/año | 4,6 |
| Convergencia difusa | ≈17 400 km ± 50 % (Bird OCB) | — |
| Choques | ≈23 000 km ± 25 % (Bird CCB) | 21 200 |
| Placas | Distribución acumulada de áreas parecida a la de Bird en la malla: las 15 mayores, una fracción del planeta parecida ± 10 puntos | — |
| Tierra por encima de 2 km | 5,3 % (ETOPO sin hielo) | 15 % |
| Placas de al menos el 1 % del planeta | ~13 (Bird) | ~20 |
| Velocidad media de las placas | 4,39 cm/año ± 25 %, sin reajustar fuerzas | — |
| Edad media del fondo | no peor que 88 Myr (Tierra, 64) | 88 |
| Placas rápidas = las de mucha fosa (Forsyth y Uyeda) | se mantiene | sí |

### Pruebas

1. **Dos placas que se mueven igual durante τ_fus** se sueldan, y el volumen de corteza y sedimento se conserva.
2. **Dos placas con una fosa activa entre ellas** no se sueldan aunque la fosa vaya despacio.
3. **Una microplaca sin fosa durante τ_micro** es capturada por la vecina con más borde común.
4. **No regresión:** India–Asia sigue igual.

### Determinismo

Las reglas son funciones puras del estado. Los empates en la vecina con más borde se resuelven por el índice de placa.

---

## 11. Preguntas abiertas

1. ¿Se calcula ya la velocidad relativa a lo largo de cada borde, o hay que añadirla?
2. ¿Qué protecciones tienen hoy las microplacas, además de la de la losa?
3. Si el §0 apunta a H4, traer aquí D1–D5 junto con el reparto de potencia de las fuerzas de la ronda de la nota de fosas v2, actualizado.

---

## Referencias

- Bird, P. (2003). An updated digital model of plate boundaries. Geochemistry, Geophysics, Geosystems 4.
- Duarte, J. C. et al. (2024). Gibraltar subduction zone is invading the Atlantic. Geology.
- Gurnis, M., Hall, C. y Lavier, L. (2004). Evolving force balance during incipient subduction. Geochemistry, Geophysics, Geosystems 5.
- Li, Y. y Gurnis, M. (2023). A simple force balance model of subduction initiation. Geophysical Journal International 232.
