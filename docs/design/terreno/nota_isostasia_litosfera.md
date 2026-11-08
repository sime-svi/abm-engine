# Nota de diseño: isostasia con litosfera (edad tectonotérmica)

Documento de diseño. Complementa a diseno_relieve_tectonico.md (incremento 4 del terreno) y aplica al motor la regla R4 del estudio de isostasia continental (docs/informes/2026-11-08, estudio de isostasia).

Estado: diseño cerrado salvo las preguntas del §11. Antes de implementar, hacer la comprobación del §0.

---

## 0. Comprobación previa: que lo que falta no deforme el ajuste

En el estudio, varios parámetros de R4 quedaron en el borde de sus rangos publicados. Eso suele indicar que el ajuste intenta absorber algo que la regla no incluye; aquí, la topografía dinámica, es decir, los empujes del manto profundo, como el que levanta África oriental.

Comprobación: repetir el ajuste de R4 excluyendo las celdas donde la propia regla de Lamb et al. (2020) deja un residuo de más de 300 m.
- Si los parámetros salen del borde de sus rangos, se usan esos valores.
- Si siguen en el borde, se mantienen los del estudio, y se anota en TREE.md como limitación conocida.

---

## 1. Fenómeno real y objetivo

Bajo la corteza de los continentes hay una capa de manto rígido, el manto litosférico, cuyo grosor depende de cuánto tiempo hace del último gran calentamiento de la zona (un rift, un arco o una orogenia): su edad tectonotérmica.
- **Zonas jóvenes:** la capa es fina y caliente. Pesa poco y la zona queda alta, como Kenia.
- **Zonas viejas:** la capa es gruesa y fría. Pesa más y la zona queda baja, como la llanura rusa.

Además:
- La corteza es más densa cuanto más profunda.
- El sedimento rellena las cuencas que crea la litosfera al hundirse, en lugar de levantar la tierra.
- Los márgenes de los rifts, adelgazados y calientes, se hunden al enfriarse (McKenzie 1978), y sobre ellos se forman las plataformas continentales.

El estudio mostró que, con la Tierra real, la regla R4 (corteza con densidad según la profundidad, manto litosférico con grosor según la edad, manto arcaico algo más ligero y el tratamiento del sedimento como relleno) mejora a la regla de Lamb et al. (2020) con la litosfera observada. Además, sacar el grosor de la litosfera de la edad da el mismo error que usar el grosor observado. Por eso al motor le basta con llevar la edad tectonotérmica de cada zona.

**Objetivo:** que el relieve de los continentes del planeta, y su hundimiento en los márgenes, salgan de esta regla, con valores y formas terrestres.

---

## 2. Alcance y nivel de detalle

Sí se modela:
- Edad tectonotérmica por celda y el reloj que la reinicia.
- La regla R4, tal como la fija el estudio (ecuación, formas y coeficientes del informe).
- Edades iniciales realistas de los continentes al empezar la deriva.
- Hundimiento térmico de los márgenes tras un rift, que sale solo de la regla al envejecer.

Se abstrae o se deja para más adelante:
- **Topografía dinámica** (plumas y superelevaciones del manto, como la de África): hasta unos cientos de metros. Ampliación futura, junto con los puntos calientes.
- **Calentamiento parcial.** Un evento reinicia la edad solo de las celdas donde actúa, nunca de forma parcial.
- **Engrosamiento del manto litosférico en los choques** antes de que se desprenda: queda resumido en el reinicio por delaminación.

---

## 3. Entidades y componentes

Nuevo dato por celda de corteza continental: la edad tectonotérmica t_tt. Viaja con la placa como el resto de datos.

---

## 4. Variables de estado

| Símbolo | Significado | Tipo | Rango / unidades |
|---|---|---|---|
| t_tt,i | Edad tectonotérmica de la celda i: tiempo desde su último reinicio | real | 0–4 000 Myr |
| L_i | Grosor de la litosfera, calculado a partir de t_tt,i con la forma R3b del estudio | real | km |
| H_i | Altura o profundidad que da la regla R4 | real | m |

---

## 5. Parámetros ajustables

Los de R4 (densidades por capa, contraste del manto, ligereza del manto arcaico, forma y constantes de L(t), tratamiento del sedimento) toman los valores del estudio, o los de la comprobación del §0. Nuevos:

| Parámetro | Significado | Por defecto | Unidades | Origen |
|---|---|---|---|---|
| t_arc | Edad por encima de la cual el manto se trata como arcaico | 2 500 | Myr | Límite del Arcaico; coherente con el estudio |
| ε_del | Delaminación mínima en un paso para reiniciar la edad | 1 | km de corteza retirada | Supuesto; evita reiniciar por pérdidas mínimas |

---

## 6. Reglas por paso tectónico

### 6.1 Envejecimiento

En cada paso, t_tt,i ← t_tt,i + Δt.

### 6.2 Reinicio (t_tt,i ← 0)

La edad vuelve a 0 en las celdas donde actúa alguno de estos eventos:
- **Rift:** las celdas de la franja estirada (adenda §6.10), al romper.
- **Arco:** las celdas de la franja de arco que reciben corteza en el paso.
- **Choque:** las celdas de la franja de engrosamiento del choque.
- **Delaminación:** las celdas que pierden más de ε_del en el paso.

### 6.3 Mezcla

Cuando el remuestreo, el flujo lateral, la acreción o un choque juntan corteza de varias procedencias en una celda, la edad resultante es la media de las edades ponderada por volumen. Un aporte de arco o de choque llega con edad 0.

### 6.4 Altura

En cada paso, la altura de cada celda de corteza continental se calcula con R4 a partir de su corteza, su sedimento y L(t_tt). El océano mantiene su curva de profundidad según la edad del fondo. El nivel del mar se sigue calculando con el volumen de agua.

Los márgenes estirados empiezan con t_tt = 0 y corteza fina: quedan bajos y siguen hundiéndose a medida que envejecen, como en la Tierra.

### 6.5 Estado inicial

Al empezar la deriva, hace 500 Myr, se asignan edades a la corteza continental inicial:
1. **Distribución:** la de TC1 (área de corteza continental por edad tectonotérmica), solo con las edades de más de 500 Myr, restándoles 500 Myr. Es decir, cómo era aproximadamente la Tierra entonces.
2. **Reparto en el espacio:** dentro de cada continente, las edades más viejas van a las celdas más interiores y las más jóvenes a las más cercanas a la costa. Así se reproduce cómo crecen los continentes, añadiendo corteza por los bordes. El reparto se hace por rangos de distancia a la costa, con desempate por índice.

---

## 7. Eventos discretos

No genera eventos nuevos. Los reinicios se cuentan como métrica.

---

## 8. Métricas por paso

- Distribución del área continental por edad tectonotérmica.
- Altura media de la tierra por clase de edad: 0–100, 100–500, 500–1 600, 1 600–2 500 y más de 2 500 Myr.
- Profundidad de los márgenes estirados según el tiempo desde la rotura.
- Área reiniciada por cada tipo de evento.

---

## 9. Interacción con otros sistemas y orden dentro del paso tectónico

- El envejecimiento va al principio del paso.
- Los reinicios se aplican en los procesos que los causan: rifts, arcos, choques y delaminación.
- La mezcla se aplica en el remuestreo, el flujo, la acreción y los choques.
- La altura (6.4) sustituye a la isostasia actual, en su mismo lugar del paso, antes del nivel del mar.

---

## 10. Validación y determinismo

Con el planeta simulado y 5 semillas:

| Magnitud | Objetivo | Origen |
|---|---|---|
| Altura media de la tierra | 671 ± 100 m | ETOPO sin hielo |
| Tierra por debajo de 200 m y por encima de 2 000 m | no peor que R4 con la Tierra real | Estudio |
| Plataforma | no peor que R4 con la Tierra real | Estudio |
| Altura media por clase de edad | la de la Tierra (TC1 y ETOPO) ± 300 m en cada clase | TC1 y ETOPO |
| Hundimiento de los márgenes | ~2–3 km en 100 Myr en la parte distal, con forma parecida a la del fondo oceánico | McKenzie 1978 |
| Distribución de edades tectonotérmicas al final | parecida a la de TC1 (criterio blando) | TC1 |
| Nivel del mar | no peor que −150 m, la referencia sin mesetas | Prueba de la Tierra real |

### Pruebas

1. **Regla aislada.** Con las cortezas y edades de la Tierra real, R4 dentro del motor da los mismos números que el estudio.
2. **Margen.** Un rift aislado: el margen queda bajo y se hunde con el tiempo según la curva esperada.
3. **Conservación.** La mezcla de edades no cambia ningún volumen.
4. **Determinismo.** Misma semilla, mismas edades.

### Después

Recalibrar con 5 semillas la erosión (E_1 y n_ero: relajación y exportación) y comprobar el nivel del mar. Después, tabla de cierre completa.

---

## 11. Preguntas abiertas

1. ¿Cómo trata exactamente R4 el sedimento? Que la implementación en el motor sea la misma que en el estudio.
2. ¿Qué coste añade calcular L y R4 en cada paso?
3. Si la altura por clase de edad no cuadra en las clases viejas, ¿viene del estado inicial o de la regla? Comprobarlo con la Tierra real por clases.

---

## Referencias

- Artemieva, I. M. (2006). Global 1°×1° thermal model TC1 for the continental lithosphere: implications for lithosphere secular evolution. Tectonophysics 416.
- Lamb, S. et al. (2020). Global whole lithosphere isostasy. Geochemistry, Geophysics, Geosystems 21.
- McKenzie, D. (1978). Some remarks on the development of sedimentary basins. Earth and Planetary Science Letters 40.
