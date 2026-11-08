# Adenda 1 al modo Tierra: ríos y lagos reales, una sola vegetación, glaciares y variabilidad

Documento de diseño. Complementa a diseno_modo_tierra.md (Fase 1) y responde a las decisiones del informe 2026-11-08_cierre_y_modo_tierra.md (§9).

Estado: diseño cerrado. Es la segunda ronda de la capa "modo Tierra, Fase 1"; según la regla de CLAUDE.md, queda como mucho una más.

Principio de esta adenda: en el modo Tierra, lo que existe medido se usa como dato. El motor modela solo lo que cambia con el tiempo: los caudales, la vegetación y los lagos de la Fase 2.

---

## 1. Ríos y lagos: la red es un dato, el caudal sale del modelo

### Por qué

La hidrología del motor calcula la red rellenando las depresiones desde el mar. Sobre el relieve real promediado a 26 km falla por tres motivos:
- las gargantas estrechas, como la del Congo, desaparecen al promediar;
- no hay cuencas cerradas;
- la tierra por debajo del nivel del mar cuenta como mar.

En la Tierra, la red de ríos está medida (HydroSHEDS, a partir de un relieve de 90 m). Los modelos hidrológicos globales no la recalculan a escala gruesa: la llevan a su rejilla con métodos de reescalado que conservan las cuencas reales (Eilander et al. 2021).

### Reglas

1. **Fuentes:**
   - direcciones de flujo de HydroSHEDS, a 15″;
   - cuencas de HydroBASINS, que marcan las cuencas cerradas;
   - lagos de HydroLAKES (Messager et al. 2016).
   
   Si no se puede descargar HydroSHEDS, la alternativa es MERIT Hydro (Yamazaki et al. 2019).
2. **Reescalado a la malla (nivel 3), por píxel de salida,** como en Eilander et al. (2021):
   - para cada celda de la malla, su píxel de salida es el de mayor área drenada dentro de la celda;
   - desde él se sigue el camino de alta resolución río abajo hasta llegar al píxel de salida de otra celda, que pasa a ser la celda de aguas abajo;
   - si esa celda no es vecina, se toma la vecina por la que pasa el camino.
   
   El resultado es una dirección de flujo por celda. Los niveles 0–2 se derivan del nivel 3.
3. **Cuencas cerradas:** las que terminan tierra adentro (Chad, Djouf, Tarim, Asia central, Gran Cuenca) acaban en su celda terminal. Allí el agua se evapora o forma su lago; nunca rebosa hacia otra cuenca.
4. **Mar:** solo es mar lo que tiene profundidad y está conectado con el océano mundial. Las depresiones interiores (El Fayum, Qattara, Turfán) son tierra, salvo donde HydroLAKES marca un lago, como el Caspio o el mar Muerto.
5. **Lagos:** fracción de lago de cada celda, sacada de HydroLAKES por área. Un lago es agua y no es tierra habitable. En la Fase 1, el área de los lagos es fija.
6. **Caudal:** la escorrentía que calcula el motor (lluvia menos lo que se evapora y lo que guarda el suelo) se encamina cada día por la red fija. Así, los ríos llevan más o menos agua según el año: en una sequía real, baja el caudal.
7. **La hidrología del motor** se mantiene para el planeta inventado, sin cambios.

**Fase 2 (anotado):** el balance de agua de los lagos de las cuencas cerradas, que suben hasta que lo que evaporan iguala lo que reciben. Hará falta para los grandes lagos del pasado, como el Mega-Chad.

### Validación

| Magnitud | Objetivo | Referencia |
|---|---|---|
| Área de las cuencas de los 20 ríos mayores | ± 20 % | HydroSHEDS |
| Desembocaduras | en su sitio: el Congo, el Nilo, el Amur, el Lena y el Yukón incluidos | HydroSHEDS |
| Área total de lagos | la de HydroLAKES en la malla, ± 10 % | HydroLAKES |
| Cuencas cerradas | ninguna rebosa a otra | HydroBASINS |
| Caudal que llega al mar desde la tierra | ≈ 37 000–40 000 km³/año | Dai y Trenberth (2002) |
| Caudal medio del Amazonas y del Congo | del orden de 6 000 y 1 300 km³/año | Datos de aforo publicados |

---

## 2. Vegetación: una sola producción, con unidades reales

### Por qué

La vegetación del terreno no tiene unidades y su campana de temperatura (óptimo de 22 °C) deja casi vacío el bosque boreal: su mapa se parece poco al de MODIS (r = 0,17). La producción de Miami (Lieth 1975), la que ya usan los agentes, da con el clima de CHELSA 61 PgC/año, frente a 53 de MODIS, con r = 0,80. Recalibrar una fórmula sin unidades sería un apaño; Miami es un modelo publicado, con unidades y medido.

### Reglas (solo en el modo Tierra; el planeta inventado sigue igual, porque está congelado)

1. La capacidad anual de la vegetación de cada celda es la producción de Miami con la temperatura y la lluvia de ese año (climatología más la anomalía del año real), en gramos de materia seca por m² y año.
2. **Reparto dentro del año:** cada mes recibe una parte proporcional a la producción de Miami calculada con la temperatura de ese mes y su lluvia multiplicada por 12, normalizada para que la suma sea la anual. Así la vegetación no crece en los meses fríos ni en los secos.
3. Los agentes comen de esa misma producción, con la conversión a kcal que ya existe. El motor tiene así una sola producción vegetal.
4. La respuesta fina a la sequía (el agua del suelo y la paja que queda) llegará con la nota de sequías de los agentes.

### Validación

| Magnitud | Objetivo | Referencia |
|---|---|---|
| Producción total de la tierra | 50–65 PgC/año | MODIS 53; otras estimaciones más altas; Miami tiende a pasarse un poco |
| Mapa | r ≥ 0,8 | MODIS |
| Bosque boreal (50–70° N) | producción dentro de ± 30 % de MODIS | MODIS |
| Desiertos | cerca de 0 | MODIS |

---

## 3. Glaciares de montaña

Se añaden los glaciares del Randolph Glacier Inventory 7.0 (CC BY 4.0), en fracción de celda, a la máscara de hielo. Objetivo: unos 0,7 Mkm² más de hielo, en su sitio (Patagonia, Karakórum, Alaska, Alpes).

---

## 4. Variabilidad de un año a otro

1. **Fondo de años:** solo se remuestrean los años en que CRU TS tiene estaciones en al menos el 90 % de la tierra. Quien implemente fija el rango; probablemente desde los años 30 hasta ~2020. Los años sin estaciones vuelven a la climatología y no tienen variabilidad propia.
2. **Autocorrelación:** se acepta la del remuestreo por bloques (0,87 veces la real) si las sequías de varios años siguen bien. Comprobación: frecuencia, por región, de rachas de 2 o más años seguidos por debajo del 75 % de la lluvia media, frente a CRU TS. Si sale dentro del ruido, se acepta y se anota. Si no, se trae aquí.

---

## 5. Otras decisiones

- **Hipsometría** a ± 1,4 % en las dos franjas más bajas: se acepta. Es el efecto de promediar las llanuras en celdas gruesas.
- **Regla del hielo** (superficie sobre el mar y al menos 10 m de espesor): se acepta.
- **Fusión:** la rama del modo Tierra se fusiona ya con main. El modo es opcional y el planeta inventado sale idéntico.

---

## Referencias

- Dai, A. y Trenberth, K. E. (2002). Estimates of freshwater discharge from continents: latitudinal and seasonal variations. Journal of Hydrometeorology 3.
- Eilander, D. et al. (2021). A hydrography upscaling method for scale-invariant parametrization of distributed hydrological models. Hydrology and Earth System Sciences 25.
- Lehner, B., Verdin, K. y Jarvis, A. (2008). New global hydrography derived from spaceborne elevation data. Eos 89.
- Lieth, H. (1975). Modeling the primary productivity of the world. En: Primary Productivity of the Biosphere. Springer.
- Messager, M. L. et al. (2016). Estimating the volume and age of water stored in global lakes using a geo-statistical approach. Nature Communications 7.
- RGI 7.0 Consortium (2023). Randolph Glacier Inventory, version 7.0.
- Yamazaki, D. et al. (2019). MERIT Hydro: a high-resolution global hydrography map based on latest topography datasets. Water Resources Research 55.
