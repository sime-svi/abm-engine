# Nota de diseño: puntos calientes, mesetas oceánicas y trazas volcánicas

Documento de diseño. Complementa a diseno_relieve_tectonico.md, a adenda_balance_corteza.md (versión 9) y a nota_isostasia_litosfera.md.

Estado: diseño cerrado salvo las preguntas del §11.

---

## 1. Fenómeno real y objetivo

### En la Tierra

Del manto profundo suben columnas de roca caliente, las plumas. Donde llegan a la superficie hay un punto caliente, que se mueve poco respecto al manto mientras las placas pasan por encima. Tienen tres efectos:

**1. Abombamientos (swells).** El fondo marino alrededor de un punto caliente se levanta 500–1 200 m, en un abombamiento de 1 000–1 500 km de ancho (Crough 1983). Se queda sobre el punto caliente, no viaja con la placa.

**2. Trazas volcánicas.** Al pasar la placa por encima, el punto caliente va construyendo una cadena de volcanes, cada vez más viejos cuanto más lejos, como la cadena de Hawái y del Emperador. Engrosa la corteza oceánica a lo largo de la traza.

**3. Grandes erupciones (grandes provincias ígneas).** Cuando llega la cabeza de una pluma nueva, en 1–5 Myr sale una cantidad enorme de magma:
- en el océano, forma una meseta oceánica: la de Ontong Java ocupa unos 1,86 millones de km², y su corteza tiene unos 30–35 km de grosor, frente a los 7 normales;
- en un continente, forma un basalto de inundación, como el Decán.

Hubo unos 11 episodios en los últimos 250 Myr, uno cada ~23 Myr (Rampino y Stothers).

**Magnitudes.** El magmatismo del interior de las placas oceánicas, sumando trazas y mesetas, es un ~5 % de todo el magma de la Tierra: unos 1,3–1,7 km³/año de media a largo plazo (Crisp 1984). El continental, menos.

**Consecuencias.**
- **Océano menos hondo.** Abombamientos, montes submarinos y mesetas ocupan volumen en las cuencas oceánicas. Con la Tierra real, la curva de profundidad del modelo ajustada a la mediana del fondo deja el mar a −150 m; ajustada a la media, a −25 m. Esos ~125 m de diferencia son, sobre todo, estos relieves.
- **Fuente de corteza.** Las mesetas son gruesas y flotan: no se hunden en las fosas y se pegan a los continentes. Si se acrecionan, aportan del orden de 1,1 km³/año de corteza (Clift et al. 2009).

### En el modelo

No existen. Por eso:
- el océano es demasiado hondo: 3 881 m de media, frente a 3 667;
- el nivel del mar queda a −140 m, y toda la tierra se mide unos 170 m más alta;
- falta una fuente de corteza: f_cont es 0,33, frente a 0,40.

### Objetivo

Que los puntos calientes, con sus abombamientos, trazas y mesetas, aparezcan con ritmos y tamaños terrestres, y que de ellos salgan solos un océano con la profundidad real, el nivel del mar en su sitio, islas volcánicas y una fuente más de corteza.

---

## 2. Alcance y nivel de detalle

Sí se modela:
- Puntos calientes fijos en el marco del manto, con vida limitada.
- Abombamiento solo sobre litosfera oceánica.
- Trazas que engrosan la corteza de la placa que pasa por encima.
- Grandes erupciones al nacer algunos puntos calientes: mesetas en el océano y basaltos en los continentes.
- Las mesetas flotan y se acrecionan como los arcos.

Se abstrae o se deja para más adelante:
- **Abombamiento sobre los continentes**, como el de África. Es parte de la topografía dinámica, ya en ampliación futura. R4 se ajustó con la tierra real, que ya lo lleva de forma estadística.
- **Rifts provocados por plumas**, como en el Atlántico central o Afar.
- **Movimiento de los puntos calientes**, y su relación con las dorsales (Islandia).
- **Efectos climáticos de las grandes erupciones.**

---

## 3. Entidades y componentes

**Punto caliente (entidad nueva).** Datos:
- posición en el marco del manto (latitud y longitud);
- edad y vida;
- caudal de su traza;
- altura y anchura de su abombamiento.

**Por celda:** la fracción a_i se amplía a "fracción de corteza flotante no continental", que incluye arcos y mesetas. Sigue igual para todo lo demás (adenda §6.5).

---

## 4. Variables de estado

| Símbolo | Significado | Tipo | Rango / unidades |
|---|---|---|---|
| x_k | Posición del punto caliente k en el marco del manto | lat, lon | — |
| e_k / T_k | Edad / vida del punto caliente k | real | Myr |
| q_k | Caudal de corteza de su traza | real | km³/Myr |
| H_k / W_k | Altura / anchura de su abombamiento | real | km |
| LIP activa | Área, grosor añadido y tiempo restante de una gran erupción en curso | estado | — |

---

## 5. Parámetros ajustables

| Parámetro | Significado | Por defecto | Unidades | Origen |
|---|---|---|---|---|
| N_hs | Número de puntos calientes activos | 12 | — | Supuesto: unos 8 tienen un caudal de flotabilidad de más de 1 Mg/s, y hay más débiles. Rango 8–20 |
| T_hs | Vida de un punto caliente | 150 | Myr | Hawái–Emperador, más de 80 Myr; Kerguelen, ~120. Rango 80–200 |
| λ_LIP | Ritmo de grandes erupciones en todo el planeta | 1 / 23 | 1/Myr | 11 episodios en 250 Myr (Rampino y Stothers) |
| f_cabeza | Fracción de nacimientos de puntos calientes con gran erupción | λ_LIP · T_hs / N_hs (≈ 0,54) | — | Derivado, para que salga λ_LIP |
| A_LIP | Área de una gran erupción | log-uniforme entre 0,2 y 2 | Mkm² | La mayor conocida, Ontong Java, 1,86 Mkm² |
| h_LIP | Grosor de corteza que añade en el océano | 15 (10–25) | km | La corteza de las mesetas tiene 20–35 km, frente a 7 |
| h_CFB | Grosor que añade en un continente | 3 (1–5) | km | Calibrado con el aporte continental, ~0,12 km³/año (Clift et al. 2009) |
| Δt_LIP | Duración de la erupción | 2 (1–5) | Myr | La mayor parte del volumen sale en menos de 1–5 Myr |
| q | Caudal de cada traza | calibrar | km³/Myr | Con el total intraplaca oceánico, 1,3–1,7 km³/año (Crisp 1984) |
| r_tr | Radio de la zona de la traza bajo el punto caliente | 100 | km | Anchura de las dorsales volcánicas tipo Hawái, ~150–250 km. Rango 50–150 |
| H_sw / W_sw | Altura / anchura de los abombamientos | uniformes en 0,5–1,2 km / 1 000–1 500 km | km | Crough (1983) |

---

## 6. Reglas por paso tectónico

### 6.1 Vida de los puntos calientes

- **Al empezar:** hay N_hs puntos calientes, en posiciones al azar en la esfera y con edades al azar entre 0 y T_hs.
- **En cada paso:** e_k ← e_k + Δt.
- **Muerte y nacimiento:** si e_k ≥ T_hs, el punto caliente muere y nace otro en una posición al azar. Con probabilidad f_cabeza, el nuevo empieza con una gran erupción.

Todo usa el generador sembrado, en un orden fijo.

### 6.2 Gran erupción

Durante Δt_LIP, en un disco de área A_LIP centrado en el punto caliente, en el marco del manto:
- **en las celdas oceánicas:** se añade corteza hasta sumar h_LIP en total, y esa corteza cuenta como flotante no continental;
- **en las celdas continentales:** se añade h_CFB de corteza, y la edad tectonotérmica vuelve a 0.

### 6.3 Traza

En cada paso, cada punto caliente añade q_k · Δt de corteza, repartida en las celdas a menos de r_tr de su posición. Son celdas de la placa que esté encima en ese momento, así que la traza se dibuja sola al moverse la placa. Esa corteza cuenta como flotante no continental solo donde la corteza total supera el umbral de flotación (20 km); los montes submarinos más pequeños se hunden en las fosas, como en la Tierra.

### 6.4 Abombamiento

Sobre la litosfera oceánica a menos de W_k/2 del punto caliente, el fondo sube:

U(d) = H_k · cos²(π · d / W_k)

Se aplica en la isostasia como una masa negativa en el manto, Δm = −U · (ρ_m − ρ_w), que sigue al punto caliente y no a la placa. Sobre la litosfera continental no se aplica (§2).

### 6.5 Acreción

La corteza flotante de las mesetas se trata en las fosas y en los choques como la de los arcos: η = a_i · η_arco + (1 − a_i) · η(h) (adenda §6.5).

### 6.6 Balance

Se añade una fuente al balance de la adenda (§6.9): F_int, el magmatismo intraplaca (trazas y grandes erupciones).

---

## 7. Eventos discretos

- **Nacimiento y muerte de un punto caliente:** paso, posición, si nace con gran erupción.
- **Gran erupción:** paso, posición, área, tipo (oceánica o continental) y volumen.

---

## 8. Métricas por paso

- F_int, en AU, separado en trazas, mesetas y basaltos continentales.
- Área y número de mesetas oceánicas presentes de más de 0,2 Mkm².
- Volumen de mesetas acrecionado, en AU.
- Profundidad media del océano y diferencia entre su media y su mediana.
- Nivel del mar.
- Área de islas volcánicas emergidas.

---

## 9. Interacción con otros sistemas y orden dentro del paso tectónico

1. Vida de los puntos calientes (6.1), al principio del paso.
2. Grandes erupciones (6.2) y trazas (6.3), después de mover las placas y antes de los procesos de la adenda.
3. Abombamiento (6.4), dentro de la isostasia.
4. Acreción (6.5), dentro de los choques y las fosas.

---

## 10. Validación y determinismo

| Magnitud | Objetivo | Hoy |
|---|---|---|
| Profundidad media del océano | 3 667 ± 150 m | 3 881 |
| Media menos mediana de la profundidad del océano | la de ETOPO, medida igual | — |
| Nivel del mar | 0 ± 75 m | −140 |
| Magmatismo intraplaca oceánico (trazas y mesetas), media a largo plazo | 1,3–1,7 km³/año (Crisp 1984) | 0 |
| Aporte de los basaltos continentales | ~0,12 km³/año (Clift et al. 2009) | 0 |
| Grandes erupciones | ~1 cada 23 Myr | 0 |
| Mesetas acrecionadas | hasta ~1,1 km³/año (Clift et al. 2009; criterio blando) | 0 |
| f_cont | acercarse a 0,40 | 0,33 |
| Abombamientos | 0,5–1,2 km de alto, 1 000–1 500 km de ancho | — |

Pruebas:
1. **Traza.** Un punto caliente bajo una placa que se mueve deja una cadena recta, más vieja cuanto más lejos.
2. **Abombamiento.** No viaja con la placa.
3. **Meseta.** Al llegar a una fosa no se hunde y se acreciona ~90 %.
4. **Conservación.** El volumen añadido cuadra con F_int en el balance.
5. **Determinismo.** Misma semilla, mismos puntos calientes y erupciones.

---

## 11. Preguntas abiertas

1. ¿El modelo tiene un marco del manto fijo, sin rotación neta de la litosfera? Los puntos calientes se colocan en ese marco.
2. ¿El umbral de flotación de 20 km ya trata bien la corteza de las mesetas?
3. ¿Qué coste añade?

---

## Referencias

- Clift, P. D., Vannucchi, P. y Phipps Morgan, J. (2009). Crustal redistribution, crust–mantle recycling and Phanerozoic evolution of the continental crust. Earth-Science Reviews 97.
- Crisp, J. A. (1984). Rates of magma emplacement and volcanic output. Journal of Volcanology and Geothermal Research 20.
- Crough, S. T. (1983). Hotspot swells. Annual Review of Earth and Planetary Sciences 11.
- Rampino, M. R. y Stothers, R. B. (1988). Flood basalt volcanism during the past 250 million years. Science 241.
