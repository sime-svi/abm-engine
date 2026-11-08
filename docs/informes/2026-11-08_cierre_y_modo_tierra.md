# Cierre del planeta inventado v1 y modo Tierra real (Fase 1)

2026-11-08, sexta ronda.

- **`main` = 42c70f9:** el planeta inventado congelado, con la succión apagada.
- **Rama `terreno-modo-tierra`, sin fusionar:** el modo Tierra.
- **Diseño:**
  - `docs/design/terreno/cierre_planeta_inventado_v1.md`;
  - `docs/design/terreno/modo-tierra-diseno.md`, el técnico, que tiene las respuestas al §11, el cómo, la validación y los hallazgos.

## Resumen

- **Planeta inventado v1 congelado y en `main`:**
  - `ε_suc` = 0, con el código de la succión dentro;
  - el documento de cierre, con su tabla (5 semillas, sin succión) y la ampliación futura en tu orden;
  - las decisiones aceptadas, anotadas;
  - la regla de las tres rondas, en `CLAUDE.md` §4.
- **Respuestas al §11 de la nota del modo Tierra**, antes de implementar (§2): qué lee el motor del mundo, el nivel de carga y lo que ocupa, las licencias y la descarga, y la vegetación frente a MODIS.
- **Fase 1 implementada.** Funciona con `--set mundo.modo=tierra`:
  - una corrida tarda 7 s con las cachés y 26 s la primera vez, y ocupa 1 GB;
  - el modo inventado sale **idéntico byte a byte** frente a 42c70f9.
- **La validación del §10:**

  | | Resultado |
  |---|---|
  | Relieve: tierra y altura media; conservación entre niveles; hielo | ✓ |
  | Hipsometría | ± 1,4 % en las dos franjas más bajas (objetivo ± 1 %); el resto ≤ 0,3 % |
  | Clima medio sobre la tierra frente a CHELSA medido aparte | ✓ (temperatura −0,002 °C; lluvia −0,01 %) |
  | Variabilidad de la lluvia anual por región (13) | ✓ |
  | El Sahel en 1984 | ✓ |
  | Autocorrelación de un año al siguiente | ✗ en 5 de 10 regiones |
  | Ríos frente a HydroSHEDS | pendiente: tu red bloquea su servidor de datos. Pero ya se ve que fallan (§5) |
  | Vegetación del motor frente a MODIS | ✗ (r = 0,17) |
  | Producción de Miami frente a MODIS | 61 frente a 53 PgC/año, r = 0,80 |

- **Lo que decides tú:** la hidrología sobre el relieve real, la vegetación, los glaciares de montaña y la autocorrelación (§9).

## 1. Congelación y fusión

- **La succión, apagada:** `ε_suc` = 0 en `config.rs` y `terreno.toml`, con el 0,05 calibrado en el comentario. La prueba del resto de losa la enciende con su valor.
- **Los basaltos continentales:** el objetivo de `planeta_semillas.py` es ahora el aceptado, 0,03 AU. La nota dice que la cifra de Clift incluye otro magmatismo continental.
- **Batería:**
  - el planeta, 236,2 s frente a 244,8 de `main` (473b805) a la vez: −3,5 %;
  - 27 márgenes movidos con su nota, porque la realización del planeta es otra;
  - todo dentro.
- **Fusión:** `terreno-margenes-calientes-succion` → `main` (42c70f9), en rápido y subida.

## 2. Respuestas al §11 de la nota del modo Tierra

1. **Qué campos consume hoy el resto del motor del terreno** (la interfaz del proveedor de mundo). Son dos grupos:
   - **Los campos de nacimiento de cada celda,** función pura de su dirección:
     - la altura;
     - el hielo (nuevo);
     - la textura del suelo (capacidad de campo, permeabilidad, fertilidad);
     - el recurso.
   - **El clima de cada día:** la temperatura y la precipitación.

   El motor deriva de la altura la pendiente, el relleno, el flujo, el drenaje, los ríos, los lagos y la distancia al mar. El resto (nieve, agua, vegetación, biomas, LOD) sale del estado del mundo. El motor de agentes no lee hoy nada del terreno. Tabla completa: `modo-tierra-diseno.md` §1.1.
2. **Nivel y tamaño.**
   - **Nivel 3**, el `l_max` del motor: 884 736 celdas de ~26 km. Los niveles 0–2 son medias por área de sus hijas.
   - **CRU TS se queda en su rejilla** (0,5°, ~el nivel 2). Por celda de nivel 3 ocuparía ~10 GB.
   - **Medido:** la caché de la proyección ocupa 0,51 GB (0,10 de las tablas y 0,40 de las series de CRU) y la del mundo, 0,09 GB.
3. **Licencias y descarga** (`herramientas/tierra/descargar.sh`, a `output/referencias/`; nada va a git):

   | Fuente | Licencia | Descarga |
   |---|---|---|
   | ETOPO 2022 | dominio público | ya estaba |
   | CHELSA V2.1 | CC0 1.0 | 4,3 GB, sin registro |
   | CRU TS 4.09 | Open Government Licence v3 | 1,1 GB, de la UEA sin registro; CEDA pide cuenta |
   | HydroRIVERS | licencia de HydroSHEDS: uso científico, educativo y comercial, con cita | 0,5 GB, solo para validar |
   | MODIS MOD17A3HGF v6.1 | NASA, sin restricciones | se usa la versión a 0,5° del ICDC (CC BY 4.0, 48 MB), solo para validar |

4. **¿Da la vegetación del motor la producción de MODIS sin recalibrar?** No, y se ve en la fórmula:
   - es una biomasa sin unidades;
   - su campana de temperatura (óptimo 22 °C, σ 12 °C) deja casi sin nada al bosque boreal.

   Como pide la nota, no he tocado nada: lo he medido (§6) y te lo traigo (§9).

## 3. Cómo es la Fase 1

- **Configuración:** `[mundo] modo` y `[tierra]` en `config/terreno.toml`. En el modo Tierra no corren la deriva, la génesis geológica, la erosión ni el agotamiento de humedad. El año es el real: 365 días, con sus meses.
- **Carga** (`tierra/carga.rs`): el motor lee las fuentes en bruto (no toca la red), con dos lectores mínimos en Rust:
  - GeoTIFF con Deflate, con `miniz_oxide`, un Deflate en Rust puro y la única dependencia nueva;
  - NetCDF-3 en flujo desde el gzip.

  Proyecta cada fuente al nivel 3, con cada punto pesando `cos φ`. CHELSA usa todos sus píxeles, con un hilo por archivo; el resultado no depende de los hilos.
- **Clima de cada día** (`tierra/clima.rs`):
  - el mes es la climatología de CHELSA más la anomalía del año real de CRU TS: diferencia en la temperatura y cociente en la lluvia, con tope `r_max`;
  - la temperatura se interpola entre las mitades de los meses, y la lluvia es la del mes, repartida por igual entre sus días;
  - el año real sale de bloques de 10 años seguidos, con el RNG por contador.
- **Hielo:** una columna nueva del mundo. Quita la vegetación (también la de partida) y la habitabilidad, y es bioma «hielo». En el modo inventado vale 0 y sus factores son 1 exacto.

**Decisiones mías, con sus datos:**

- **La regla del hielo se aparta de la letra de la nota.** La nota dice «superficie − roca > 0».
  - En el Ártico, las dos versiones de ETOPO salen de fuentes distintas y difieren unos metros sobre el mar y en la costa de Groenlandia. Con la regla literal salían 1,2 Mkm² de «hielo» sobre el mar y Groenlandia con 3,07 Mkm² (su casquete tiene 1,71).
  - La regla nueva pide la superficie sobre el nivel del mar y al menos 10 m de espesor (`hielo_min_m`). Así salen la Antártida con 13,9 Mkm² y Groenlandia con 1,8.
- **El mar tiene clima.** CHELSA V2.1 trae valores también sobre el océano (los de ERA5, hasta 84° N), así que el mar no queda vacío.

## 4. Validación del §10 (`herramientas/tierra/validar.py`)

| Magnitud | Modelo | Referencia | Objetivo | |
|---|---|---|---|---|
| Fracción de tierra | 0,2905 | ETOPO a 26 km: 0,2912 | ± 1 % | ✓ |
| Altura media de la tierra | 792,4 m | 791,1 | ± 1 % | ✓ |
| Hipsometría | de 0 a 200 m, 0,276; de 200 a 500 m, 0,284 | 0,280 / 0,280 | ± 1 % | ± 1,4 % en esas dos franjas (las celdas gruesas de las llanuras); el resto ≤ 0,3 % |
| Conservación entre niveles | 1,1·10⁻¹⁴ | — | redondeo | ✓ |
| Hielo | 15,6 Mkm² | ~15,6 | — | ✓ (sin los glaciares de montaña) |
| Temperatura de la tierra | 8,896 °C | CHELSA medido aparte: 8,898 | ± 2 % | ✓ |
| Lluvia de la tierra | 861,2 mm | 861,2 | ± 2 % | ✓ |
| Variabilidad de la lluvia anual (coeficiente de variación, 13 regiones) | — | CRU TS 1901–2024 | ± 10 % | ✓ (de −4 % a +5 %) |
| Autocorrelación de un año al siguiente | 0,87 veces la de CRU de media | CRU TS | ± 10 % | ✗ en 5 de 10 |
| Años secos (< 75 % de la media) | p. ej., Sahel 0,028, Nordeste de Brasil 0,108 | 0,032 y 0,089 | ± 15 % | dentro del ruido: son 2–11 años de 124 |
| El Sahel en 1984 | 304 mm, 0,733 de lo normal | CRU TS: 308, 0,723 | coincide | ✓ |
| Ríos | — | HydroSHEDS | ± 20 % y en su sitio | pendiente; fallan (§5) |
| Producción vegetal | Miami, 61 PgC/año | MODIS: 53 (65 contando la celda de 0,5° entera) | 50–60 | Miami, un 2 % por encima del techo |
| Mapa de la producción | vegetación del motor r = 0,17; Miami r = 0,80 | MODIS | r > 0,8 | ✗ / en el límite |
| Agentes | — | batería | siguen pasando | ✓ |

Pruebas:
- el modo inventado sale idéntico byte a byte frente a 42c70f9;
- con la misma semilla sale la misma secuencia de años;
- el Sahel de 1984;
- y tres pruebas con datos inventados (`tests/tierra.rs`), que no necesitan las fuentes:
  - la interpolación y la conservación del clima entre niveles;
  - el periodo base;
  - la altura y el hielo.

## 5. Hallazgo: los ríos

**La hidrología del motor no da los ríos de la Tierra.** Rellena las depresiones desde el mar, sin balance de agua en los lagos. Con el relieve real a 26 km salen **9,4 Mkm² de lagos**, ~6 veces los reales a esta escala, y ríos grandes por donde no son:

- **El Congo** sale por el Ogooué (Gabón), con 3,8 Mkm²: su garganta bajo Kinshasa es más estrecha que una celda y, al promediar, la tapa.
- **El Nilo** acaba en la depresión de El Fayum (−43 m): para el motor, la tierra bajo el nivel del mar es mar.
- **Las cuencas cerradas se desbordan:**
  - el lago Chad, por el río Cross, junto al Níger (4,3 Mkm²);
  - el Djouf, por el Senegal (2,9 Mkm²);
  - Asia central, en el Caspio por Turkmenistán (4,6 Mkm²);
  - el Tarim, en Turfán.
- **El Amur, el Lena y el Yukón** salen a ríos vecinos.

Salen bien el Amazonas (5,62 Mkm², en su sitio), el Paraná, el Mackenzie, el Yangtsé y el Volga.

**Para los agentes importa:** un lago es agua sin tierra habitable, y el 6 % de la tierra queda bajo lagos que no existen.

**Tres causas y sus arreglos** (`modo-tierra-diseno.md` §4.1):
1. **Gargantas tapadas al promediar.** Se puede encaminar el agua por la altura más baja de cada celda, o grabar la red de HydroSHEDS, con lo que los ríos pasarían a ser datos.
2. **Falta el balance de agua de los lagos**, que suben hasta que evaporan lo que reciben. Hará falta para la Fase 2 (el Mega-Chad).
3. **El mar debería ser solo lo conectado con el océano.** Pero sin el punto 2, el Caspio desbordaría al mar Negro.

La comparación exacta con HydroSHEDS espera al servidor. Vodafone lo bloquea («Por causas ajenas a Vodafone…»), con el bloqueo de las direcciones de Cloudflare. La herramienta la hace en cuanto el archivo está en `output/referencias/hydrosheds/`.

## 6. La vegetación (sin tocar nada)

- **La del motor:**
  - tras un año, r = 0,17 con MODIS;
  - no tiene unidades;
  - el bosque boreal se queda casi vacío.
- **Miami** (la producción de los agentes) con el clima de CHELSA:
  - 61 PgC/año (0,47 g C por g de materia seca), frente a 53 de MODIS 2001–2023 (Zhao y Running publican 53–56);
  - r = 0,80.

**Opciones:**
- que la capacidad de la vegetación del terreno sea la producción de Miami, con lo que el motor tendría una sola producción vegetal y la de los agentes vendría del terreno;
- o recalibrar la vegetación actual contra MODIS.

## 7. Otras medidas

- **La autocorrelación.**
  - La serie de la malla en el orden real reproduce la de CRU (África oriental: 0,219 frente a 0,216).
  - El remuestreo la baja a 0,87 veces de media: cortes entre bloques y años de los extremos de la serie menos representados. La persistencia de varias regiones viene de cambios de décadas.
  - Con bloques de 20 años no mejora (África oriental 0,12).
- **La cobertura de CRU TS:** sin estaciones de lluvia queda el 29 % de su tierra en 1901–1910, el 3 % en 1961–1990 y el 18 % en 2021–2024. Allí CRU vuelve a su climatología, y esos años tienen menos variabilidad propia. Va en la salida.
- **Los glaciares de montaña** (~0,7 Mkm²) no están en ETOPO: comprobado en Patagonia y en el Karakórum. Están en el Randolph Glacier Inventory 7.0 (CC BY 4.0).

## 8. Batería y verificación

- `herramientas/verificar.sh --no-regresion 42c70f9`: formato, clippy, pruebas, batería y no regresión.
- **La batería** tiene un escenario nuevo, `tierra`: un año del modo Tierra con 11 métricas (relieve, hielo, clima, conservación, determinismo, la mayor cuenca y los lagos). Necesita las fuentes.
- **No regresión:** el planeta inventado sale idéntico byte a byte frente a 42c70f9.

## 9. Lo que decides tú

1. **La hidrología** sobre el relieve real: el encaminado por la altura mínima o la red de HydroSHEDS, el balance de agua de los lagos y el mar conectado (§5).
2. **La vegetación:** atar su capacidad a la producción de Miami, o recalibrarla contra MODIS (§6).
3. **Los glaciares de montaña:** añadir el RGI 7.0, o dejarlos fuera en la Fase 1.
4. **La autocorrelación de un año al siguiente:** aceptar la del remuestreo por bloques, o cambiar el método.
5. **Fusionar** `terreno-modo-tierra` con `main`.
