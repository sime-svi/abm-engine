# Nota de diseño: succión de las losas y cierre de los océanos

Documento de diseño. Complementa a diseno_relieve_tectonico.md (§6.2, fuerzas), a nota_ciclo_fosas.md (versión 6), a nota_rifts.md (versión 2) y a nota_demografia_placas.md (versión 2).

Estado: diseño cerrado salvo las preguntas del §11.

---

## 1. Fenómeno real y objetivo

### En la Tierra

**Dos formas de tirar.** Las losas mueven las placas de dos maneras (Conrad y Lithgow-Bertelloni 2002):
- **Tirón:** la losa que sigue unida a su placa en el manto superior tira de ella. Es cerca de la mitad de la fuerza que mueve las placas.
- **Succión:** la losa que ya se hundió hasta el manto inferior arrastra el manto a su alrededor. Ese flujo empuja la base de las placas cercanas hacia la fosa, tanto la que se hunde como la de arriba.

**Las fosas retroceden y la placa de arriba las sigue.** Entre el 62 % y el 78 % de los tramos de fosa retroceden, a una media de 1,3–1,5 cm/año (Schellart et al. 2008). La placa de arriba va detrás de su fosa. Así se cierran los océanos y se juntan los continentes.

**Las losas siguen tirando después de morir.** Una losa que se separa sigue hundiéndose en el manto inferior a ~1,2 cm/año (van der Meer et al. 2010), y tarda del orden de 150–200 Myr en llegar al fondo. Mientras tanto, sigue arrastrando el manto. Los continentes tienden así a juntarse sobre los cementerios de losas antiguas, que es como se forman los supercontinentes.

**Tras un choque, las placas se sueldan.** Cuando la convergencia de un choque se agota, el borde deja de moverse y las dos placas pasan a ser una. Así, Pangea funcionó como una sola gran placa continental.

### En el modelo (diagnóstico)

- No hay succión. Un continente solo va hacia una fosa si su propia placa se hunde en ella.
- Los océanos de los rifts se abren, uno de sus márgenes se activa a los ~70 Myr, pero casi nunca se cierran (0,8 por semilla) y siguen ensanchándose.
- Los continentes no se juntan: su distancia media crece despacio (+1,1 km/Myr).
- Las masas continentales llegan a tocarse en grupos grandes (hasta el 83–98 % del continente), pero siguen en placas distintas: el bloque mayor de una sola placa reúne solo el 22 %. Por eso el reloj de los rifts casi nunca ve un supercontinente.

### Objetivo

Que la succión de las losas, activas y muertas, y la soldadura tras los choques hagan que los océanos se cierren, que los continentes se junten y que se formen supercontinentes de forma natural, con fuerzas y movimientos de fosa terrestres.

---

## 2. Alcance y nivel de detalle

Sí se modela:
- La parte profunda de cada losa activa y los restos de losa que siguen hundiéndose, en el marco del manto.
- La succión como una tracción en la base de las placas, dirigida hacia la losa.
- La soldadura de placas tras un choque continental agotado.

Se abstrae o se deja para más adelante:
- **El flujo completo del manto** (convección en 3D).
- **Interacción entre plumas y losas.**
- **Deriva polar verdadera.**

---

## 3. Entidades y componentes

**Losa profunda:** se calcula en cada paso para cada fosa activa, a partir de su longitud de losa.

**Resto de losa (entidad nueva):** una línea en el marco del manto, con su flotabilidad negativa por unidad de longitud y su edad desde que se separó.

**Por borde de choque:** un contador del tiempo con convergencia agotada.

---

## 4. Variables de estado

| Símbolo | Significado | Tipo | Rango / unidades |
|---|---|---|---|
| L_j | Longitud de losa de la fosa j (ya existe, nota de fosas 6.5) | real | km |
| L_inf,j | Parte de la losa en el manto inferior: max(0, L_j − L_losa) | real | km |
| B | Peso de la losa por unidad de longitud de fosa, el que no compensa su flotación | real | N/m |
| t_s,k | Tiempo desde que el resto k se separó | real | Myr |
| T_c,pq | Tiempo seguido con la convergencia agotada en el borde de choque entre p y q | real | Myr |

---

## 5. Parámetros ajustables

| Parámetro | Significado | Por defecto | Unidades | Origen |
|---|---|---|---|---|
| L_losa | Longitud de la losa en el manto superior (ya existe) | 800 | km | Nota de fosas |
| d_off | Distancia horizontal entre la fosa y su losa profunda, hacia la placa de arriba | 500 | km | Geométrico: la losa llega a 660 km de profundidad con inclinaciones de 45–65°. Rango 300–800 |
| W_suc | Alcance lateral de la succión | 1 500 | km | Del orden de la profundidad del manto inferior. Rango 1 000–3 000 |
| ε_suc | Fracción del peso de la losa profunda que llega a las placas como tracción | calibrar | — | Contra el reparto de fuerzas y el retroceso de las fosas (§10) |
| v_hund | Velocidad de hundimiento de las losas en el manto inferior | 1,2 | cm/año | Medido: 1,2 ± 0,3 cm/año (van der Meer et al. 2010) |
| τ_hund | Tiempo hasta que un resto deja de tirar | ~2 200 km / v_hund ≈ 180 | Myr | Derivado. Rango 120–250 |
| v_sold | Convergencia por debajo de la cual un choque se considera agotado | 0,5 | cm/año | Supuesto. Rango 0,2–1 |
| τ_sold | Tiempo con la convergencia agotada tras el que las placas se sueldan | 20 | Myr | Supuesto. Rango 10–40 |

---

## 6. Reglas por paso tectónico

### 6.1 Peso de las losas profundas

Para cada fosa activa j:

B_j = Δρ_s · g · h_s · L_inf,j

Δρ_s y h_s son los mismos que usa el tirón de la losa en el documento principal, según la edad del fondo que se hunde. La losa profunda se coloca a lo largo de la fosa, desplazada d_off hacia la placa de arriba, y se recalcula en cada paso, así que sigue a la fosa.

### 6.2 Restos de losa

Cuando una fosa muere, o su losa se rompe tras un choque, la losa entera (longitud L_j) pasa a ser un resto, fijo en el marco del manto en la posición de su losa profunda en ese momento. Su peso se va apagando mientras baja:

B_k(t) = B_k,0 · max(0, 1 − t_s,k / τ_hund)

### 6.3 Tracción de succión

Para cada celda de cualquier placa, a una distancia d menor que W_suc de una losa profunda o de un resto (en el marco del manto), la base de la placa recibe una tracción horizontal hacia el punto más cercano de esa línea:

τ_suc = ε_suc · B · (1 − d / W_suc) / W_suc

Se suman las contribuciones de todas las losas. La tracción entra en el equilibrio de fuerzas de cada placa como una fuerza más que la mueve, junto al tirón de la losa y el empuje de las dorsales. El arrastre basal ya existente la frena.

### 6.4 Soldadura tras un choque

En cada borde de choque entre continentes, una vez rota la losa (nota de fosas 6.6):
- si la convergencia es menor que v_sold, T_c,pq ← T_c,pq + Δt; si no, T_c,pq ← 0;
- si T_c,pq ≥ τ_sold, las dos placas se funden en una, con la conservación de la demografía de placas (A2). La mayor absorbe a la menor y la sutura queda dentro de la placa.

### 6.5 Recalibración de las fuerzas

Si al añadir la succión la velocidad media de las placas sube, se reajusta el coeficiente de arrastre basal, que es un valor calibrado contra la velocidad. No se tocan los valores físicos de las losas.

---

## 7. Eventos discretos

- **Resto de losa nuevo:** paso, fosa, longitud, peso inicial.
- **Soldadura tras un choque:** paso, placas, longitud de la sutura, duración del choque.

---

## 8. Métricas por paso

- Reparto de la potencia que mueve las placas: tirón, succión y empuje de las dorsales.
- Movimiento de las fosas respecto al manto: fracción de longitud que retrocede y velocidad media.
- Velocidad de la placa de arriba hacia su fosa.
- Número de restos de losa y peso total.
- Convergencia de los choques a lo largo de su vida, y soldaduras.
- Las del diagnóstico del supercontinente: bloque mayor de una sola placa y conectado, distancia entre bloques y océanos de rift que se cierran.

---

## 9. Interacción con otros sistemas y orden dentro del paso tectónico

- La tracción se calcula en el paso de fuerzas, junto al tirón y al empuje de las dorsales.
- Los restos se crean al morir las fosas o romperse las losas.
- La soldadura se comprueba al final del paso, junto a la captura de microplacas.

---

## 10. Validación y determinismo

| Magnitud | Objetivo | Origen |
|---|---|---|
| Parte del tirón de la losa en la potencia que mueve las placas | ~50 % | Conrad y Lithgow-Bertelloni 2002 |
| Velocidad media de las placas | 4,39 cm/año ± 25 % | Ya en la batería |
| Fosas que retroceden respecto al manto | 62–78 % de la longitud | Schellart et al. 2008 |
| Velocidad media de las fosas | 1,3–1,5 cm/año de retroceso | Schellart et al. 2008 |
| Océanos de rift que se cierran | una parte apreciable (criterio blando) | — |
| Vida de los márgenes pasivos, medidos como entidades que nacen en cada rift y no por celdas de costa | 130–180 Myr | Bradley 2008 |
| Supercontinente | al menos una vez en 500 Myr, el bloque mayor de una sola placa pasa del 60 % del continente, y después se rompe (criterio blando) | Pangea |
| Placas rápidas = las de mucha fosa (Forsyth y Uyeda) | se mantiene | Ya en la batería |
| India–Asia | no se acelera tras romperse la losa | Pendiente anterior |

ε_suc se calibra con el reparto de potencia y el retroceso de las fosas, con 5 semillas.

Pruebas:
1. **Fosa que retrocede.** La placa de arriba se mueve hacia ella.
2. **Resto de losa.** Aun sin fosa, atrae las placas cercanas mientras t_s < τ_hund, y después deja de tirar.
3. **Choque agotado.** Tras τ_sold con la convergencia agotada, las placas se sueldan y se conserva todo.
4. **Determinismo.**

---

## 11. Preguntas abiertas

1. ¿Cómo se suman hoy las fuerzas de cada placa (§6.2 del documento principal)? La tracción tiene que entrar en el mismo equilibrio.
2. ¿Hay un marco del manto fijo? Es el mismo que necesitan los puntos calientes.
3. Si los choques nunca bajan de v_sold, mide su convergencia a lo largo de su vida y tráemelo, porque entonces falla la resistencia de los choques.

---

## Referencias

- Bradley, D. C. (2008). Passive margins through earth history. Earth-Science Reviews 91.
- Conrad, C. P. y Lithgow-Bertelloni, C. (2002). How mantle slabs drive plate tectonics. Science 298.
- Schellart, W. P., Stegman, D. R. y Freeman, J. (2008). Global trench migration velocities and slab migration induced upper mantle volume fluxes. Earth-Science Reviews 88.
- van der Meer, D. G. et al. (2010). Towards absolute plate motions constrained by lower-mantle slab remnants. Nature Geoscience 3.
