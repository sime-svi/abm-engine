# Nota de diseño: bordes de placa (dientes, transformantes y forma)

Documento de diseño. Complementa a diseno_relieve_tectonico.md (incremento 4 del terreno) y a adenda_balance_corteza.md (versión 3).

Estado: diseño cerrado salvo las preguntas del §11.

Esta nota corrige la base del balance de corteza. Hasta que esté implementada, no se calibra ningún parámetro de la adenda (f_acr, τ_del, D_flujo, τ_ero): hacerlo compensaría una base irreal.

---

## 1. Fenómeno real y objetivo

### Cómo son los bordes en la Tierra

En la Tierra hay tres tipos de borde entre placas:
- **Dorsales:** las placas se separan y nace fondo oceánico.
- **Fosas y choques:** las placas se acercan; una se hunde, o las dos se engruesan si son continentes.
- **Fallas transformantes:** las placas se deslizan una junto a la otra, como en la falla de San Andrés. Aquí no nace ni se destruye placa: el área que cruza el borde es cero.

Su forma también es característica:
- Las fosas son arcos suaves, de cientos a miles de km de radio.
- Las dorsales son tramos perpendiculares a la dirección de separación, cortados por fallas transformantes paralelas a esa dirección.
- La forma de un borde es estable a escalas de unos 100–200 km, porque la litosfera es rígida. Un borde real no tiene dientes del tamaño de una celda.

### Qué pasa hoy en el modelo (medido)

- **Dientes en los bordes.** Los bordes están dibujados con celdas, así que tienen dientes. Al girar las placas, los dientes de una se meten en la otra también donde solo se deslizan o se separan, y el remuestreo lo trata como si una se hundiera.
  - El 26 % del fondo oceánico consumido cae en bordes divergentes o transformantes.
  - El 31 % de la corteza continental que se solapa también cae en ellos.
  - En la Tierra, ambos serían cero.
- **Bordes demasiado largos.** Frente a la Tierra con 15 placas medida con la misma regla, el perímetro total del modelo es 1,7 veces mayor, y la longitud de fosa, 3 veces mayor.
- **Se hunde el doble de fondo.** Se consumen 6,3 km²/año de fondo oceánico, frente a 2,7–3,4 en la Tierra. Arcos, erosión por subducción y sedimento al mar salen altos por esa razón.

### Objetivo

Que solo se consuma placa donde las placas se acercan de verdad, y que los bordes tengan una longitud total y por tipo parecida a la de la Tierra.

---

## 2. Alcance y nivel de detalle

Sí se modela:
- Clasificación de cada tramo de borde con un borde suavizado, no con la escalera de celdas.
- Umbral de transformante a 20°, el que separa las clases en los datos de Bird.
- Compensación de dientes: en tramos transformantes y divergentes, el solape no se consume, sino que se recoloca en el hueco de su propia placa.
- Suavizado de la pertenencia en los bordes oceánicos, a la escala de rigidez de la litosfera.
- Métricas de consumo y creación por tipo de tramo.

Se abstrae o se deja para más adelante:
- **Segmentación dorsal–transformante más fina que una celda.** Con celdas de 52 km, una dorsal con saltos cortos se ve como una dorsal oblicua. Se acepta a este nivel de detalle.
- **Microplacas.** Bird tiene 52 placas; el modelo, 14–15. Se compara con las 15 mayores de la Tierra. Ampliación futura.
- **Polaridad de subducción por tramo.** Hoy se decide por par de placas; ya está anotado en TREE.md.
- **Saltos de dorsal como eventos explícitos.** El suavizado reproduce los reajustes pequeños; los saltos grandes quedan para más adelante.
- **Corteza continental.** No cambia de placa por suavizado. Sus bordes los deciden los choques y, en la nota siguiente, los rifts.

---

## 3. Entidades y componentes

No hay entidades nuevas.

Datos temporales por celda, calculados en cada paso:
- fracción suavizada de pertenencia a cada placa vecina;
- normal del borde suavizado;
- tipo de tramo local.

---

## 4. Variables de estado

| Símbolo | Significado | Tipo | Rango / unidades |
|---|---|---|---|
| φ_p,i | Fracción suavizada de la placa p alrededor de la celda i: media de la pertenencia a p dentro de un radio L_borde, ponderada por área | real | 0–1 |
| n̂_i | Normal del borde suavizado en la celda i, apuntando de la placa p a la placa q | vector tangente a la esfera | unitario |
| v_rel,i | Velocidad de la placa q respecto de la placa p en la celda i | vector | cm/año |
| α_i | Ángulo entre v_rel,i y la línea del borde suavizado (0° = deslizamiento puro, 90° = choque o separación frontal) | real | 0–90° |
| tipo_i | Tipo de tramo local | estado | convergente / divergente / transformante |
| solape_i, hueco_i | Celda reclamada por dos placas tras mover / celda que no reclama ninguna | marcas | — |

---

## 5. Parámetros ajustables

| Parámetro | Significado | Por defecto | Unidades | Origen |
|---|---|---|---|---|
| L_borde | Radio del suavizado del borde | 150 | km | Calibrado: longitud por tipo y perímetro total contra Bird (15 placas) medido igual. El orden de magnitud físico es la escala de flexión de la litosfera, 100–300 km |
| θ_tr | Ángulo por debajo del cual un tramo es transformante | 20 | grados | Medido: separación de clases en los datos de Bird (hoy el modelo usa 15) |
| r_busca | Distancia máxima para recolocar un diente en un hueco de su propia placa | igual a L_borde | km | Numérico |

---

## 6. Reglas por paso tectónico

### 6.1 Mover las placas

Se mantiene el remuestreo conservativo actual. Tras mover, quedan marcadas las celdas en solape (dos placas las reclaman) y en hueco (ninguna las reclama).

### 6.2 Borde suavizado

Para cada celda i cerca de un borde y para cada placa p presente en su entorno:

φ_p,i = Σ_k A_k · [k pertenece a p] / Σ_k A_k, sumando sobre las celdas k a menos de L_borde de i

Una celda en solape cuenta como media celda para cada una de sus dos placas, y un hueco no cuenta para ninguna.

La normal n̂_i es la dirección en que más crece (φ_q,i − φ_p,i), calculada por diferencias entre celdas vecinas y normalizada a longitud 1.

### 6.3 Tipo de tramo local

α_i = 90° − ángulo entre v_rel,i y n̂_i

- Si α_i < θ_tr, el tramo es **transformante**.
- Si no, y v_rel,i · n̂_i < 0 (las placas se acercan), es **convergente**.
- En el resto de casos es **divergente**.

### 6.4 Resolución de solapes y huecos

**Solape en tramo convergente.** Se consume como ahora, y se registra A_sub para la adenda. Es la única vía por la que se consume placa.

**Hueco en tramo divergente.** Nace fondo oceánico nuevo, como ahora. Es la única vía por la que nace placa.

**Solape en tramo transformante o divergente.** No se consume nada.
- La celda queda para la placa con mayor φ en ella, es decir, la del lado del borde suavizado donde cae.
- La corteza (y el sedimento) de la otra placa se traslada al hueco de su propia placa más cercano, a menos de r_busca. Si hay varios a la misma distancia, se desempata con el sorteo sembrado que ya usa la tectónica.
- Si no hay ningún hueco a menos de r_busca, se suma a la celda más cercana de su propia placa, que se engruesa un poco. Así se conserva el volumen.
- Este traslado se registra como "dientes recolocados".

**Hueco en tramo transformante o convergente.**
- Se llena primero con los dientes recolocados de la regla anterior.
- Si sigue vacío, se asigna a la placa con mayor φ y se rellena con fondo oceánico nuevo.
- Este relleno se registra como "creación residual". En un tramo transformante corresponde a la pequeña componente de separación real del borde.

### 6.5 Suavizado de la pertenencia

Se aplica a cada celda a menos de L_borde de un borde que solo tenga corteza oceánica, es decir, sin corteza de tipo continental ni de arco.

Si su placa actual p tiene φ_p,i < 0,5 y otra placa q tiene φ_q,i > 0,5, la celda pasa a la placa q.

La corteza no se mueve: solo cambia de placa, y a partir de ahí viaja con q. El volumen se conserva por construcción.

Esto reproduce los pequeños reajustes de los bordes reales y evita que los dientes se acumulen paso a paso. El área reasignada se registra como métrica y debe ser pequeña.

### 6.6 Procesos de la adenda

Los procesos de la adenda se aplican después, sin cambios. A_sub sale solo de los solapes en tramos convergentes (6.4).

---

## 7. Eventos discretos

No genera eventos nuevos. Su efecto se sigue con las métricas del §8.

---

## 8. Métricas por paso

- Consumo de placa por tipo de tramo (convergente, divergente, transformante), en km²/año.
- Corteza continental solapada por tipo de tramo, en AU.
- Dientes recolocados y creación residual, en km²/año.
- Área reasignada por el suavizado, en km²/año.
- Longitud de borde por tipo y perímetro total, en km, medidos con el mismo método que la Tierra.
- Área total consumida y área total creada, en km²/año.
- Edad media del fondo oceánico.

---

## 9. Interacción con otros sistemas y orden dentro del paso tectónico

1. Mover las placas (6.1).
2. Borde suavizado y normales (6.2).
3. Tipo de tramo (6.3). Sustituye a la clasificación actual con 15°.
4. Solapes y huecos (6.4).
5. Suavizado de la pertenencia (6.5).
6. Procesos de la adenda, en su orden: régimen, arcos, erosión por subducción, sedimento en la fosa, choques, delaminación y flujo lateral, erosión.
7. Isostasia y nivel del mar.
8. Comprobación del balance y métricas.

Relación con otros trabajos:
- La adenda de la corteza usa A_sub, que ahora solo sale de tramos convergentes.
- La nota siguiente (inicio de subducción, rifts y fondo oceánico) parte de bordes ya limpios.

---

## 10. Validación y determinismo

### Objetivos

Los criterios de tolerancia son propuestos.

| Magnitud | Objetivo | Hoy |
|---|---|---|
| Fondo consumido en tramos divergentes y transformantes | menos del 2 % del total | 26 % |
| Corteza continental solapada en tramos no convergentes | menos del 2 % | 31 % |
| Fondo consumido = fondo creado | 2,7–3,4 km²/año | 6,3 |
| Dorsales (Bird, 15 placas, medido igual) | 87 400 km ± 25 % | 120 900 |
| Fosas | 39 500 km ± 25 % | 117 600 |
| Choques | 25 600 km ± 25 % | 11 100 |
| Transformantes | 29 600 km ± 25 % | 62 300 |
| Perímetro total | 182 200 km ± 25 % | 311 800 |
| Edad media del fondo oceánico | la de Seton et al. (2020) en la misma malla, ya en el repositorio | 79 Myr |

### Pruebas

1. **Deslizamiento puro.** Dos placas que solo se deslizan, con el borde en zigzag, durante 100 Myr. El consumo y la creación deben ser nulos (error de redondeo) y no deben aparecer dientes nuevos.
2. **Separación oblicua.** Dos placas que se separan en diagonal: solo nace placa, no se consume nada.
3. **Choque frontal.** El escenario India–Asia debe dar los mismos resultados que hoy (no regresión).
4. **Conservación.** El volumen de corteza y de sedimento se conserva en cada paso, como ya exige la adenda.

### Determinismo

El suavizado y la clasificación son funciones puras del estado. Los empates se resuelven con el sorteo sembrado que ya usa la tectónica.

### Si un objetivo no se cumple

Solo se ajusta L_borde, porque es el único parámetro calibrado de esta nota; θ_tr es un valor medido. Si con un L_borde razonable (100–300 km) las fosas siguen más de 1,5 veces por encima de la Tierra, la causa no está en la forma de los bordes, sino en las fuerzas que mueven las placas (§11, pregunta 2).

---

## 11. Preguntas abiertas

1. **Representación de las placas.** Las reglas están escritas por celda. Si las placas se representan con varias muestras por celda, quien implemente las adapta, manteniendo la conservación exacta.
2. **Si las fosas siguen demasiado largas.** Si tras esta nota la longitud de fosa sigue por encima de 1,5 veces la de la Tierra, hay que mirar las fuerzas que mueven las placas (§6.2 del documento principal) con el reparto de convergencia por placa. Se traería aquí para diseñar la corrección.
3. **Arco por placa.** Con los bordes ya suavizados, ¿sigue haciendo falta repartir el arco por placa, o puede volver a repartirse por tramo, que es más fiel? Lo decide quien implemente, con los datos.

---

## Referencias

- Bird, P. (2003). An updated digital model of plate boundaries. Geochemistry, Geophysics, Geosystems 4.
- Cogné, J.-P. y Humler, E. (2006). Trends and rhythms in global seafloor generation rate. Geochemistry, Geophysics, Geosystems 7.
- Rowley, D. B. (2002). Rate of plate creation and destruction: 180 Ma to present. GSA Bulletin 114.
- Seton, M. et al. (2020). A global data set of present-day oceanic crustal age and seafloor spreading parameters. Geochemistry, Geophysics, Geosystems 21.
