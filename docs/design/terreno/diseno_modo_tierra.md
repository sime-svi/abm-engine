# Documento de diseño: modo Tierra real

Documento de diseño. Abre un segundo modo de mundo: el motor arranca sobre la Tierra real, con su relieve y su clima medidos, en lugar de generar un planeta. El planeta inventado se congela tal como está y queda como el otro modo.

Estado: Fase 1 (Tierra actual) diseñada y lista para implementar. Fase 2 (Tierra del pasado) solo esbozada (§12).

---

## 1. Fenómeno real y objetivo

El objetivo del proyecto es la vida: seguir a un individuo de que nace hasta que muere y poder subir hasta ver toda su población. Para eso, el mundo tiene que dar a los agentes lo que en la realidad condiciona su vida: dónde hay tierra y mar, a qué altura, qué temperatura y cuánta lluvia hay en cada mes, cuánto varía de un año a otro (las sequías), por dónde van los ríos y cuánta vegetación crece.

En el modo Tierra, todo eso viene de mediciones reales. El realismo es máximo por construcción, y la validación se reduce a comprobar que los datos llegan bien a la malla del motor.

**Objetivo:** que el resto del motor reciba del modo Tierra los mismos campos que recibe del planeta inventado, con valores reales, sin saber de qué modo vienen.

---

## 2. Alcance y nivel de detalle

Sí se modela (Fase 1):
- **Relieve y batimetría** reales (ETOPO 2022) en la malla del motor, conservando las medias entre niveles de la malla.
- **Hielo:** los casquetes y glaciares actuales, como zonas sin vegetación ni vida terrestre.
- **Clima medio mensual real:** temperatura y precipitación (CHELSA, 1981–2010).
- **Variabilidad de un año a otro real**, remuestreando años medidos completos (CRU TS, 1901 hasta hoy). Así salen sequías con la frecuencia, la extensión y la duración reales.
- **Ríos y lagos**, calculados por el motor a partir del relieve y la lluvia, y comparados con los reales.
- **Vegetación** calculada por el modelo del motor a partir del clima, y comparada con la producción vegetal medida por satélite.
- **El relieve no cambia durante la simulación:** a escala de miles de años, la erosión es despreciable.

Se abstrae o se deja para más adelante:
- **La Tierra del pasado:** clima y nivel del mar cambiantes. Es la Fase 2.
- **Suelos reales** (SoilGrids).
- **Reparto diario de la lluvia en días de lluvia y días secos.** En la Fase 1, la lluvia de cada mes se reparte de forma uniforme entre sus días.
- **Cambio climático causado por los humanos.** La climatología base es de 1981–2010; como opción, se puede corregir a un periodo más antiguo (§5).

---

## 3. Entidades y componentes

**Proveedor de mundo:** una interfaz común con dos implementaciones, el planeta inventado (la génesis actual) y la Tierra. El resto del motor solo ve la interfaz.

**Campos por celda de la malla:**
- altura o profundidad;
- máscara de hielo;
- 12 valores mensuales de temperatura y precipitación media;
- red de ríos y lagos (derivada);
- y, en cada año simulado, el año real elegido para la variabilidad.

---

## 4. Variables de estado

| Símbolo | Significado | Tipo | Unidades |
|---|---|---|---|
| z_i | Altura (positiva) o profundidad (negativa) de la celda i | real | m |
| hielo_i | Fracción de la celda cubierta de hielo | real | 0–1 |
| T̄_i,m / P̄_i,m | Temperatura / precipitación media del mes m (climatología) | real | °C / mm |
| a_año | Año real elegido para el año simulado actual | entero | año |
| ΔT_i,m / r_i,m | Anomalía de temperatura / cociente de precipitación del mes m en ese año real, respecto a su climatología | real | °C / — |
| T_i,d / P_i,d | Temperatura / precipitación del día d | real | °C / mm |

---

## 5. Parámetros ajustables

| Parámetro | Significado | Por defecto | Unidades | Origen |
|---|---|---|---|---|
| fuente_relieve | Relieve de partida | ETOPO 2022, superficie con hielo | — | Ya en el repositorio |
| fuente_clima | Climatología mensual | CHELSA v2.1, 1981–2010 | — | La más fiel en montaña, según la comparación ya hecha |
| fuente_variabilidad | Series mensuales para la variabilidad | CRU TS 4 (0,5°, 1901–hoy) | — | Harris et al. 2020 |
| periodo_base | Periodo al que se refiere el clima | 1981–2010 | años | Opción: corregir la temperatura y la lluvia a 1901–1930 con CRU TS, para quitar parte del calentamiento reciente |
| L_bloque | Longitud de los bloques de años reales remuestreados | 10 | años | Supuesto: conserva las sequías de varios años, como la del Sahel en los setenta y ochenta. Rango 5–20 |
| r_max | Tope del cociente de precipitación | 5 | — | Numérico: evita valores extremos en zonas casi sin lluvia |
| nivel_carga | Nivel más fino de la malla al que se cargan los datos | el del motor por defecto | — | Por debajo, se usa el reparto con conservación del motor |

---

## 6. Reglas

### 6.1 Carga (una vez, al arrancar)

1. Se proyectan ETOPO, CHELSA y CRU TS a la malla del motor por media ponderada por área. Así se respeta la regla de conservación entre niveles.
2. Se calcula la máscara de hielo como superficie con hielo menos roca, cuando la diferencia es mayor que 0.
3. Se calculan las anomalías de cada año real de CRU TS respecto a su propia climatología del periodo base: diferencia para la temperatura y cociente para la lluvia, con tope r_max.
4. Se calculan los ríos y los lagos con la hidrología del motor.
5. Todo se guarda en caché, por nivel de malla.

### 6.2 Al empezar cada año simulado

Se elige el año real a_año remuestreando por bloques. Al empezar cada bloque, se sortea con el generador sembrado un año real de inicio, y después se recorren L_bloque años seguidos. Se usa siempre el mapa completo de ese año real, para conservar cómo se reparten de verdad las sequías en el espacio: regiones que se secan a la vez y otras que se mojan.

### 6.3 Cada día (tick)

Para el día d, que cae entre la mitad de un mes y la mitad del siguiente:
- **Temperatura:** interpolación lineal entre los valores de los dos meses de T̄ + ΔT.
- **Precipitación:** la del mes, P̄ · r, repartida por igual entre sus días.

### 6.4 Lo que no cambia

El relieve, el nivel del mar y el hielo son fijos en la Fase 1.

---

## 7. Eventos discretos

- **Año real elegido:** año simulado, año real usado.

---

## 8. Métricas

- Fracción de tierra, altura media de la tierra, fracción con hielo.
- Temperatura y precipitación medias sobre la tierra.
- Cuenca de los grandes ríos.
- Producción vegetal global y por regiones.
- Frecuencia de años secos por región.

---

## 9. Interacción con otros sistemas

- Sustituye, en este modo, a la génesis, la tectónica, la lluvia orográfica, el ciclo estacional sinusoidal y la erosión durante la simulación.
- Los agentes, la vegetación y la hidrología leen los mismos campos que antes, a través del proveedor de mundo.
- La elección de modo va en la configuración. El planeta inventado sigue funcionando igual, byte a byte.

---

## 10. Validación y determinismo

| Magnitud | Objetivo | Referencia |
|---|---|---|
| Fracción de tierra, altura media de la tierra, hipsometría | iguales a ETOPO a la misma resolución, ± 1 % | ETOPO 2022 |
| Conservación entre niveles de la malla | error de redondeo | Regla de la malla |
| Precipitación y temperatura medias sobre la tierra | iguales a CHELSA a la misma resolución, ± 2 % | CHELSA |
| Variabilidad de un año a otro, por región | desviación típica de la lluvia anual y autocorrelación de un año a otro, ± 10 % de CRU TS | CRU TS |
| Sequías | frecuencia de años con menos del 75 % de la lluvia media, por región, ± 15 % de CRU TS | CRU TS |
| Ríos | área de las cuencas de los 20 ríos mayores, ± 20 % al nivel por defecto; desembocaduras en su sitio | HydroSHEDS (Lehner et al. 2008) |
| Producción vegetal | total de la tierra del orden de 50–60 PgC/año; mapa con correlación espacial > 0,8 | MODIS MOD17 |
| Agentes | las pruebas actuales (Damuth y demografía) siguen pasando | Batería |

Pruebas:
1. **Modo inventado intacto:** sale idéntico byte a byte.
2. **Determinismo:** misma semilla, misma secuencia de años reales.
3. **Un año concreto:** forzando un año real (por ejemplo, 1984), la lluvia del Sahel en la malla coincide con CRU TS de ese año.

---

## 11. Preguntas abiertas

1. ¿Qué campos exactos consume hoy el resto del motor del terreno? Esa lista define la interfaz del proveedor de mundo.
2. ¿A qué nivel de malla se cargan los datos, y cuánto ocupan en caché?
3. ¿Qué licencia tiene cada fuente, y cómo se descargan sin guardarlas en git?
4. ¿La vegetación del motor da la producción de MODIS sin recalibrar? Si no, se trae aquí antes de tocar nada.

---

## 12. Fase 2 (esbozo): la Tierra del pasado

Para poder simular la salida de los humanos de África, el mundo tiene que ser el de entonces:
- **Clima del pasado:**
  - Beyer, Krapp y Manica (2020): últimos 120 000 años, rejilla de 0,5°, pasos de 500 años, temperatura y lluvia mensuales;
  - Krapp et al. (2021): últimos 800 000 años, con temperatura, lluvia y producción vegetal.
  
  Se interpolan en el tiempo y se les suma la misma variabilidad de años reales. Que la variabilidad del pasado fuera como la actual es una suposición, y se anota.
- **Nivel del mar del pasado:** se aplica una curva del nivel del mar a la batimetría de ETOPO. Así aparecen y desaparecen los puentes de tierra: Beringia, la plataforma de la Sonda y Sahul.
- **Casquetes de hielo** reconstruidos.
- **Validación climática:** los corredores verdes entre el norte de África y Arabia que encontraron Timmermann y Friedrich (2016): húmedos hace 115 000–75 000 años, secos hace 71 000–60 000 y húmedos otra vez hace 60 000–47 000.

---

## Referencias

- Beyer, R. M., Krapp, M. y Manica, A. (2020). High-resolution terrestrial climate, bioclimate and vegetation for the last 120,000 years. Scientific Data 7.
- Harris, I. et al. (2020). Version 4 of the CRU TS monthly high-resolution gridded multivariate climate dataset. Scientific Data 7.
- Karger, D. N. et al. (2017). Climatologies at high resolution for the earth's land surface areas (CHELSA). Scientific Data 4.
- Krapp, M. et al. (2021). A statistics-based reconstruction of high-resolution global terrestrial climate for the last 800,000 years. Scientific Data 8.
- Lehner, B., Verdin, K. y Jarvis, A. (2008). New global hydrography derived from spaceborne elevation data (HydroSHEDS). Eos 89.
- Timmermann, A. y Friedrich, T. (2016). Late Pleistocene climate drivers of early human migration. Nature 538.
