# Modo Tierra real — diseño técnico de la Fase 1

Nota del responsable: `diseno_modo_tierra.md`. Este documento responde a su §11 antes de implementar (§1), fija el cómo (§2) y recoge la validación de su §10 (§3) y los hallazgos (§4).

Estado: Fase 1 implementada y validada (rama `terreno-modo-tierra`, 2026-11-08), salvo los ríos frente a HydroSHEDS: la red del responsable bloquea su servidor de datos. Hallazgos que decide el responsable: la hidrología sobre el relieve real (§4.1) y la vegetación (§4.2).

---

## 1. Respuestas al §11

### 1.1 Qué campos consume hoy el resto del motor del terreno

Es la interfaz del proveedor de mundo. Lo que el motor recibe del mundo se reduce a dos grupos:

- los **campos de nacimiento** de cada celda: función pura de su dirección (cara, nivel, i, j), que es lo que hoy da el generador de estáticos (`statics.rs`);
- el **clima de cada tick**, que hoy da el sistema de clima (`systems.rs: climate`).

Todo lo demás lo deriva el motor, igual en los dos modos.

| Campo | Unidades | Quién lo lee | Planeta inventado | Tierra |
|---|---|---|---|---|
| Altura de nacimiento `h0` de la celda, a su nivel | m | todo: tierra o mar, hidrología, LOD, métricas | ruido o deriva de placas + detalle | ETOPO 2022 superficie, media de la celda |
| Hielo: fracción de la celda | 0–1 | vegetación, bioma, habitabilidad | no hay (el bioma «hielo» sale de T̄ < −10 °C) | ETOPO superficie − roca > 0 |
| Textura del suelo: `w_cap0`, `k_perm`, `fert` | mm, 0–1 | balance de agua, vegetación | ruido de textura + pendiente | lo mismo, provisional hasta SoilGrids |
| Recurso `resource` | — | agentes (capacidad de alimento) | sorteo por contador | lo mismo, provisional |
| Temperatura del tick `temp` | °C | nieve, evapotranspiración, vegetación, bioma, LOD, métricas | latitud + altura + seno estacional | CHELSA + anomalía de CRU TS, interpolada entre mitades de mes |
| Precipitación del tick `precip` | mm/día | nieve, balance de agua, ríos, bioma, métricas | franja × humedad × orografía × agotamiento × `c_P` | CHELSA × cociente de CRU TS, repartida por igual en el mes |

Lo que el motor deriva en los dos modos:

- **De la altura:**
  - la pendiente, el gradiente y la orientación (diferencias centrales de la función de altura a un paso de celda);
  - el relleno de depresiones y las direcciones de flujo;
  - el área de drenaje;
  - los ríos y los lagos;
  - la distancia al mar.
- **Del estado:** la malla dinámica (LOD), la nieve, el balance de agua, la difusión lateral, el agua superficial, la vegetación y los biomas.

Solo existe en el modo inventado, y el modo Tierra no lo usa:

- la deriva de placas y su historia;
- la génesis geológica (la erosión y el suelo de un millón de años) y la erosión durante la simulación;
- el gradiente orográfico a escala `d_oro`, el agotamiento de humedad `w` y `c_P` (CHELSA ya lleva la lluvia de las montañas y su sombra).

El modo Tierra da la profundidad del suelo uniforme (`z_init`), también provisional hasta SoilGrids.

El motor de agentes (`abm-ruleset`) **no lee hoy nada del terreno**: vive en un hábitat de prueba de zonas con clima fijo y su producción de Miami. Su lectura futura (`systems.rs` §6.9: `coste_movimiento`, `habitabilidad`, `agua_disponible`, `capacidad_alimento`) lee el estado del mundo (pendiente, humedad del suelo, vegetación, ríos y lagos, temperatura, recurso), no el proveedor: funciona igual en los dos modos.

### 1.2 A qué nivel de malla se cargan los datos y cuánto ocupan

- **Nivel de carga:** el 3, el más fino del motor por defecto (`l_max` = 3: celdas de ~26 km de lado, 884 736 en toda la esfera).
  - Los niveles 0–2 (208, 104 y 52 km) se calculan en el motor como media ponderada por área de sus cuatro hijas: la conservación entre niveles es exacta, salvo el redondeo.
  - Una celda más fina que el nivel de carga toma el valor de su antecesora (el reparto conservativo del motor).
- **Las anomalías de CRU TS se quedan en su rejilla** (0,5°, unos 55 km: el nivel 2 de la malla). Guardarlas por celda de nivel 3 ocuparía ~10 GB (884 736 celdas × 124 años × 12 meses × 2 variables). Cada celda de nivel 3 apunta a la celda de CRU que contiene su centro, o a la de tierra más cercana. Cada mes se arma el mapa del mes en el nivel 3 y se sube a los niveles gruesos con las mismas medias por área.
- **Lo que ocupa:**

  | Qué | Dónde | Tamaño |
  |---|---|---|
  | Fuentes descargadas | `output/referencias/` (ignorado por git) | ETOPO a 2', ~0,35 GB (ya estaba); CHELSA, 24 mapas mensuales, 4,3 GB; CRU TS, 1,1 GB comprimido |
  | Caché de la proyección, por nivel de carga | `output/cache/tierra_n3_*.bin` | 0,51 GB (medido): 0,10 del nivel 3 (altura, hielo, 12 + 12 valores de clima, cobertura y celda de CRU) y 0,40 de las series de CRU TS (67 420 celdas de tierra × 1 488 meses × 2 variables, en décimas) |
  | Caché del mundo construido | `output/cache/genesis_*.bin` | 0,09 GB (medido; ~194 000 celdas) |
  | Memoria durante la simulación | — | 1,0 GB con las cachés; 1,3 GB la primera vez, al proyectar las fuentes (medido) |
  | Tiempo de un año simulado | — | 7,4 s con las cachés; 26 s la primera vez (medido; la proyección de los 24 GeoTIFF tarda ~16 s con un hilo por archivo) |

### 1.3 Licencia y descarga de cada fuente

Nada va a git: las fuentes se bajan a `output/referencias/`, que git ignora, con `herramientas/tierra/descargar.sh`. A git solo van las herramientas y las tablas resumen, como ya se hacía con ETOPO.

| Fuente | Licencia | Descarga |
|---|---|---|
| ETOPO 2022 v1 (NOAA NCEI) | Dominio público (obra del Gobierno de EE. UU.); se cita doi:10.25921/fd45-gt74 | Ya está: OPeNDAP a 2', `referencia_tierra.py` |
| CHELSA V2.1, climatologías 1981–2010 (Karger et al. 2017; datos: Karger et al. 2021, EnviDat doi:10.16904/envidat.228) | CC0 1.0 («Creative Commons Zero – No Rights Reserved», ficha del conjunto en chelsa-climate.org); se cita igualmente | 24 GeoTIFF (`tas` y `pr`, 12 meses; 113 y 230 MB cada uno) del almacén `os.zhdk.cloud.switch.ch/chelsav2/GLOBAL/climatologies/1981-2010/`, sin registro |
| CRU TS 4.09 (Harris et al. 2020) | Open Government Licence v3 (Reino Unido); se atribuye a «Climatic Research Unit (University of East Anglia) and Met Office» | 2 NetCDF-3 comprimidos (`tmp` 443 MB, `pre` 693 MB) de `crudata.uea.ac.uk`, sin registro. En CEDA (el archivo oficial) piden cuenta |
| HydroSHEDS: HydroRIVERS v1.0 (Lehner et al. 2008; Lehner y Grill 2013) | «Freely available for scientific, educational and commercial use», con el acuerdo de licencia de HydroSHEDS (documentación técnica) y la cita obligatoria | `HydroRIVERS_v10_shp.zip` (544 MB) de `data.hydrosheds.org`. Solo para validar |
| MODIS MOD17A3HGF v6.1 (Running y Zhao; doi:10.5067/MODIS/MOD17A3HGF.061) | Política de datos de NASA EOSDIS: uso libre, sin restricciones, con cita | El original (500 m) pide cuenta de Earthdata. Se usa la versión a 0,5° del ICDC (Universidad de Hamburgo, doi:10.25592/uhhfdm.14635, CC BY 4.0): un zip de 48 MB con 2001–2023, sin registro. Solo para validar |

### 1.4 ¿La vegetación del motor da la producción de MODIS sin recalibrar?

**No, y por construcción.**

- La vegetación del terreno (`systems.rs: vegetation`) es una biomasa logística **sin unidades**: capacidad `k_max · f_hum(θ) · f_temp(T̄) · fert`, con `k_max` = 100. No produce carbono, así que no puede dar los 50–60 PgC/año de MODIS sin una conversión que no existe.
- Su campana de temperatura (óptimo 22 °C, σ 12 °C) deja casi sin nada al bosque boreal: `f_temp(−5 °C)` = 0,006. MODIS le da 200–400 gC/m²/año.
- El motor sí tiene una producción con unidades: la de Miami (Lieth 1975) de los agentes, g de materia seca/m²/año, a partir de T̄ y P̄ anuales. Es la base de su forraje, y el diagnóstico del terreno ya la calcula (`ppn_miami_tierra`).

Como pide la nota, **no se toca nada**:

- se mide la correlación del mapa de la vegetación del motor con MODIS;
- se miden el total y la correlación de Miami con el clima real;
- y se trae al responsable (§3 y §4).

**Medido (§3):**
- el mapa de la vegetación del motor tras un año se parece poco al de MODIS: r = 0,17;
- Miami con el clima de CHELSA da 61 PgC/año (con 0,47 g C por g de materia seca), frente a 53 de MODIS, y su mapa se parece al de MODIS: r = 0,80.

---

## 2. Diseño técnico

### 2.1 Configuración

- `[mundo] modo = "inventado" | "tierra"`, por defecto `"inventado"`. El modo inventado sale idéntico byte a byte.
- `[tierra]`:
  - las rutas de las fuentes;
  - `nivel_carga` (3);
  - `l_bloque` (10 años);
  - `r_max` (5);
  - `periodo_base` (1981–2010);
  - `anio_forzado` (0 = remuestreo).

En el modo Tierra:

- la deriva, la génesis geológica, el agotamiento de humedad y la erosión no corren;
- el año tiene 365 ticks (días), con los meses reales: 31, 28, 31…

### 2.2 Carga (una vez; caché por nivel de carga)

El motor lee las fuentes en bruto y las proyecta sobre su malla de nivel 3:

- GeoTIFF en tiras con Deflate y predictor 2: un lector mínimo en `tierra/geotiff.rs`, con `miniz_oxide`, un Deflate en Rust puro;
- NetCDF-3 clásico, leído por registros desde el gzip: `tierra/netcdf3.rs`.

La red no la toca nunca.

1. **ETOPO 2022 superficie** (cada 2'): media ponderada por `cos φ` de los puntos que caen en cada celda.
2. **Hielo:** fracción del área de la celda con la superficie sobre el nivel del mar y al menos `hielo_min_m` (10 m) por encima de la roca. La roca solo difiere de la superficie bajo los casquetes, así que se lee en las franjas polares (latitud < −60° y > 59°).
   - **Se aparta de la letra de la nota** («superficie − roca > 0»), con datos: en el Ártico las dos versiones de ETOPO vienen de fuentes distintas (BedMachine frente a GEBCO e IBCAO) y difieren unos metros sobre el mar y en la costa sin hielo de Groenlandia.
   - Con la regla literal salían 1,2 Mkm² de «hielo» sobre el mar y Groenlandia con 3,07 Mkm² (su casquete tiene 1,71).
   - Con la nueva, Groenlandia da 1,80 Mkm² (el casquete y sus glaciares de alrededor) y la Antártida 13,9.
   - Un hielo cuya superficie quede bajo el mar no existe: hasta el que flota asoma ~1/9 de su espesor.
3. **CHELSA** (30'', todos los píxeles): media ponderada por `cos φ` de los píxeles de cada celda y mes, y su cobertura (la fracción de la celda con píxeles).
   - CHELSA V2.1 tiene valores también sobre el mar (los de ERA5 bajados de escala), hasta 84° N. El mar tiene así clima medido, aunque la Fase 1 no lo use.
   - Las celdas al norte de 84° N (todas de mar; el 0,27 % de la esfera) toman el clima de la celda con dato más cercana en su meridiano.
4. **CRU TS 4.09** (0,5°, 1901–2024): las series mensuales de `tmp` y `pre` de cada celda de tierra, en décimas, y la celda de CRU de cada celda de nivel 3.
5. **Caché:** las tablas del nivel de carga y las series se guardan en `output/cache/tierra_n<L>_<huella>.bin`. La huella combina la del código, `n0`, el nivel, `hielo_min_m` y la ruta absoluta, el tamaño y la fecha de cada fuente. La caché del mundo construido suma esta huella a su clave.
6. **Niveles gruesos:** en memoria, medias por área de las hijas; el clima, ponderado además por la cobertura.

### 2.3 El mundo

- **Génesis:** la misma de siempre sobre la altura de ETOPO: malla del nivel base, estáticos, vecindad, hidrología del motor y refinado inicial del LOD (con los mismos criterios de relieve, costas, ríos y lagos).
- **Lo que se salta:** la deriva, el acondicionamiento del detalle, la génesis geológica y el agotamiento de humedad. El relieve queda tal cual: el motor no lo erosiona ni rellena sus hondonadas.
- **Hielo:** una columna nueva del mundo, `hielo` (0 en el modo inventado). Tres factores `1 − hielo` (que en el modo inventado valen 1 exacto, así que no cambian ningún bit):
  - multiplica la capacidad de la vegetación;
  - multiplica la vegetación de partida;
  - multiplica la habitabilidad.

  El bioma es «hielo» si `hielo` ≥ 0,5.

### 2.4 El clima de cada día

- **Año real de cada año simulado:**
  - el año simulado `y` (tick / 365) pertenece al bloque `b = y / L_bloque`;
  - el bloque sortea su año de inicio con el RNG por contador (semilla, `b`), uniforme entre 1901 y 2024 − L + 1;
  - `a_y = inicio_b + y mod L_bloque`;
  - antes del año 0 va el año real anterior a `a_0` (o el propio `a_0`, si es 1901);
  - con `anio_forzado`, siempre ese año.
- **Anomalías:** respecto a la climatología de CRU TS del periodo base de cada celda de CRU:
  - `ΔT = T(a, m) − T̄_base(m)`;
  - `r = min(P(a, m) / P̄_base(m), r_max)`, con `r` = 1 si `P̄_base` = 0.
  - Con otro periodo base, la climatología se corrige con CRU TS: `T̄ + (T̄_CRU,base − T̄_CRU,1981–2010)` y `P̄ · P̄_CRU,base / P̄_CRU,1981–2010`.
- **Mapa de un mes** `(a, m)`, en el nivel 3:
  - `T̄(m) + ΔT`;
  - `P̄(m) · r`;
  - y sube a los niveles gruesos con las medias. Se arma solo cuando hace falta: hay como mucho tres a la vez.
- **Día `d`:**
  - la temperatura se interpola linealmente entre las mitades de los dos meses que lo rodean, del año real que toque (el diciembre anterior o el enero siguiente pueden ser de otro año real);
  - la precipitación es la del mes del día, entre sus días.

  Las dos son lineales en el mapa del mes: la media de las hijas es la de la madre también cada día.

### 2.5 Salida

- `run.tierra`: las fuentes y sus versiones, los parámetros y el año real de cada año simulado (el evento del §7).
- `diagnostico`: el clima es el de la Tierra (T̄ y P̄ anuales de la climatología).
- `diagnostico.tierra`:
  - la fracción con hielo;
  - las medias sobre la tierra;
  - las mayores cuencas que llegan al mar, con su desembocadura y su área.
- En el modo Tierra, las medias de temperatura de cada tick son sobre la tierra (el mar no tiene clima).

---

## 3. Validación (§10 de la nota)

Herramienta: `herramientas/tierra/validar.py`; resultados en `output/tierra_validacion/validacion.json`. El modo Tierra corre un año con la semilla 42 (año real 1990).

| Magnitud | Modelo | Referencia | Objetivo | |
|---|---|---|---|---|
| Fracción de tierra | 0,2905 | ETOPO a 26 km: 0,2912 | ± 1 % | ✓ (−0,2 %) |
| Altura media de la tierra | 792,4 m | 791,1 m | ± 1 % | ✓ (+0,2 %) |
| Hipsometría, franjas 0–200 / 200–500 m | 0,276 / 0,284 | 0,280 / 0,280 | ± 1 % | ✗ por poco (± 1,4 %, 0,4 puntos) |
| Hipsometría, el resto de franjas | — | — | ± 1 % | ✓ (≤ 0,3 %) |
| Conservación entre niveles (Σ z·A) | 1,1·10⁻¹⁴; malla frente a nivel 3, 1,6·10⁻¹⁵ | — | redondeo | ✓ |
| Hielo | 15,6 Mkm² (Antártida 13,9, Groenlandia 1,8) | casquetes ~15,6 | — | ✓ (sin los glaciares de montaña, §4.3) |
| Temperatura media de la tierra | 8,896 °C | CHELSA medido aparte (con una muestra de sus píxeles, sobre la tierra de la malla): 8,898 | ± 2 % | ✓ |
| Lluvia media de la tierra | 861,2 mm | 861,2 | ± 2 % | ✓ (−0,01 %) |
| Variabilidad de la lluvia anual por región (13 regiones; coeficiente de variación) | 0,037–0,204 | CRU TS 1901–2024: 0,038–0,194 | ± 10 % | ✓ en las 13 (de −4 % a +5 %) |
| Autocorrelación de un año al siguiente | 0,87 veces la de CRU de media (África oriental 0,14 frente a 0,22) | CRU TS | ± 10 % | ✗ en 5 de las 10 regiones donde no es casi nula; Siberia, justo en el −10 % (§4.4) |
| Años secos (< 75 % de la media) | Sahel 0,028, Nordeste de Brasil 0,108, Australia 0,011… | 0,032, 0,089, 0,016… | ± 15 % | dentro del ruido (± 1 año en 124) |
| Un año concreto: el Sahel en 1984 | 304 mm (0,733 de su clima) | CRU TS: 308 mm (0,723) | coincide | ✓ |
| Ríos: las 20 mayores cuencas | — | HydroSHEDS | ± 20 %, desembocaduras en su sitio | pendiente de HydroRIVERS, pero ya se ve que no (§4.1) |
| Producción vegetal total | Miami, 61 PgC/año (la vegetación del motor no tiene unidades) | MODIS 2001–2023: 53 (con sus píxeles válidos; 65 contando la celda de 0,5° entera) | 50–60 | Miami, un 2 % por encima del techo (§4.2) |
| Mapa de la producción | r = 0,17 la vegetación del motor; r = 0,80 Miami | MODIS | r > 0,8 | ✗ el motor; Miami, en el límite |
| Agentes | — | batería | siguen pasando | ✓ |

Pruebas:
1. **Modo inventado intacto:** el planeta sale idéntico byte a byte frente a 42c70f9, el planeta congelado (`herramientas/no_regresion.sh`).
2. **Determinismo:** misma semilla, misma secuencia de años reales (`tests/tierra.rs` y `tierra/clima.rs`).
3. **El Sahel en 1984:** con el año forzado, su anomalía es la de CRU TS: 0,733 frente a 0,723 (`forzando_1984_la_lluvia_del_sahel_es_la_de_cru_ts`).

## 4. Hallazgos y decisiones pendientes

### 4.1 La hidrología del motor no da los ríos de la Tierra (decide el responsable)

Sobre el relieve real a 26 km, la hidrología del motor (relleno de depresiones desde el mar, sin balance de agua en los lagos) da **9,4 Mkm² de lagos**. La Tierra tiene ~2,7 Mkm² de lagos de más de 10 ha y, a esta resolución, ~1,5. Y saca **ríos grandes por sitios que no son**:

| Río | Dónde desemboca en el modelo | Por qué |
|---|---|---|
| Congo | por el Ogooué (Gabón), con 3,8 Mkm² | la garganta del Congo bajo Kinshasa es más estrecha que una celda: al promediar, la celda queda alta y tapa la salida |
| Nilo | en la depresión de El Fayum (−43 m), que el motor toma por mar | la tierra bajo el nivel del mar es «mar» para el motor (`h < 0`) |
| Lago Chad | por el río Cross (Nigeria), junto al Níger, con 4,3 Mkm² | el motor rellena las cuencas cerradas hasta que desbordan; en la Tierra el Chad es endorreico (evapora lo que recibe) |
| Djouf (Sáhara) | por el Senegal, con 2,9 Mkm² | lo mismo |
| Asia central (Aral, Amu Daria, Sir Daria) | en el Caspio, por la costa de Turkmenistán (4,6 Mkm²) | lo mismo |
| Tarim | en la depresión de Turfán (−154 m), como mar | las dos cosas |
| Amur | junto a Vladivostok | garganta o divisoria al promediar |
| Lena | por el Anabar | lo mismo |
| Yukón | por el Kuskokwim | lo mismo |
| Ob | en su sitio, con 4,5 Mkm² (HydroSHEDS ~3,0) | recoge cuencas cerradas de Kazajistán |

Salen bien el Amazonas (5,62 Mkm², en su sitio), el Paraná, el Mackenzie, el Yangtsé y el Volga (en el Caspio, como en la Tierra).

Para los agentes importa: un lago es agua, sin tierra habitable, así que el 6 % de la tierra está bajo lagos que no existen.

Tres causas, cada una con su arreglo posible:
1. **Las gargantas y valles estrechos se tapan al promediar.** Se podría encaminar el agua por la altura más baja de cada celda (no la media), o grabar la red real de HydroSHEDS en la malla («stream burning»). En el segundo caso los ríos dejan de calcularse a partir del relieve y pasan a ser datos.
2. **No hay cuencas cerradas.** Un lago debería subir solo hasta que su evaporación iguale lo que recibe. Es un balance de agua nuevo para los lagos, y hará falta para la Fase 2 (el Mega-Chad del Holoceno).
3. **La tierra bajo el nivel del mar no es mar.** Habría que llamar mar solo a lo que está conectado con el océano. Pero sin el punto 2, el Caspio se llenaría hasta desbordar al mar Negro.

### 4.2 La vegetación (se trae aquí, sin tocar nada)

- La vegetación del terreno no tiene unidades y su mapa se parece poco al de MODIS (r = 0,17): la campana de temperatura, de óptimo 22 °C, deja casi sin nada al bosque boreal.
- La producción de Miami de los agentes, con el clima de CHELSA, da 61 PgC/año y r = 0,80 con MODIS.

Opciones:
- que la capacidad de la vegetación del terreno sea la producción de Miami, la misma fórmula que ya usan los agentes, con lo que el motor tendría una sola producción vegetal;
- o recalibrar la vegetación actual contra MODIS.

### 4.3 Los glaciares de montaña

- ETOPO solo tiene la roca bajo los dos casquetes, la Antártida y Groenlandia. En Patagonia y en el Karakórum, roca y superficie coinciden (comprobado).
- Faltan así ~0,7 Mkm² de glaciares y casquetes pequeños: el Ártico canadiense, Svalbard, Alaska, Patagonia, el Himalaya e Islandia. Están en el Randolph Glacier Inventory 7.0 (CC BY 4.0).
- Para los agentes importa poco: su clima de CHELSA ya es frío y la vegetación del motor ahí es casi nula.

### 4.4 La autocorrelación de un año al siguiente

- La serie de la malla en el orden real reproduce la de CRU TS: en África oriental, 0,219 frente a 0,216.
- El remuestreo por bloques la baja: de media, a 0,87 veces la de CRU.
  - Una parte es el corte entre bloques: 1 de cada `L` años.
  - Otra parte es que los años de los extremos de la serie salen menos, y la persistencia de algunas regiones (África oriental, Siberia, la Amazonia) viene de cambios de varias décadas y de la cobertura cambiante de estaciones.
  - Con bloques de 20 años no mejora: África oriental baja a 0,12 y Siberia sigue en 0,43.
- El coeficiente de variación y los años secos sí salen.

### 4.5 La cobertura de CRU TS

- Donde no hay estaciones, CRU TS vuelve a su climatología: ese año no tiene variabilidad propia allí.
- Sin estaciones de lluvia queda el 29 % de la tierra de CRU en 1901–1910, el 3 % en 1961–1990 y el 18 % en 2021–2024.
- Los años de los extremos de la serie tienen, por tanto, menos variabilidad real.
- Va en la salida: `run.tierra.cru_sin_estaciones_por_decada`.

## Referencias

- Harris, I., Osborn, T. J., Jones, P. y Lister, D. (2020). Version 4 of the CRU TS monthly high-resolution gridded multivariate climate dataset. Scientific Data 7, 109.
- Karger, D. N. et al. (2017). Climatologies at high resolution for the earth's land surface areas. Scientific Data 4, 170122.
- Lehner, B., Verdin, K. y Jarvis, A. (2008). New global hydrography derived from spaceborne elevation data. Eos 89(10), 93–94.
- Lehner, B. y Grill, G. (2013). Global river hydrography and network routing: baseline data and new approaches to study the world's large river systems. Hydrological Processes 27(15), 2171–2186.
- Lieth, H. (1975). Modeling the primary productivity of the world. En Lieth y Whittaker (eds.), Primary Productivity of the Biosphere, 237–263.
- NOAA National Centers for Environmental Information (2022). ETOPO 2022 15 Arc-Second Global Relief Model. doi:10.25921/fd45-gt74.
- Running, S. y Zhao, M. (2021). MODIS/Terra Net Primary Production Gap-Filled Yearly L4 Global 500 m SIN Grid V061. doi:10.5067/MODIS/MOD17A3HGF.061.
