# Herramientas

Programas fuera del motor para verificar, medir y consultar las corridas (CLAUDE §2.1: pueden usar Python o bash). Las métricas que sostienen una decisión salen de aquí o de la propia salida JSON, nunca de cálculos sueltos (CLAUDE §5).

| Herramienta | Para qué |
|---|---|
| `verificar.sh` | Toda la verificación de CLAUDE §10 en un comando: formato, clippy, tests, batería de escenarios y, con `--no-regresion REF`, no regresión byte a byte frente a un commit |
| `escenarios.py` + `escenarios.toml` | Batería de escenarios de referencia: corridas fijas con unas pocas métricas clave, sus márgenes y su presupuesto de tiempo |
| `no_regresion.sh` | Compara byte a byte la salida actual con la de un commit anterior (para los cambios que no deben alterar nada) |
| `planeta_semillas.py` | El planeta entero con varias semillas a la vez y la tabla de cierre de la tectónica: media, desviación y extremos frente a ETOPO y a los objetivos de la adenda y la nota de fosas |
| `analisis/agentes.py` | Resume una corrida de agentes: población frente a `N_max`, fecundidad, esperanza de vida, hambre, mortalidad por sexo |
| `analisis/terreno.py` | Resume y compara corridas del planeta: biomas, diagnóstico, `c_P`, relieve tectónico y, con el volcado por celda, la sombra pluviométrica y las laderas de barlovento de pie a cresta |
| `analisis/referencia_tierra.py` | Calcula las tablas de la Tierra con las que se compara el relieve (`config/referencias/tierra_relieve.toml`) |
| `analisis/h5mini.py` | Lector mínimo de HDF5 (sin librerías): lee filas sueltas de rejillas contiguas por peticiones parciales |
| `analisis/lluvia_observada.py` | Mide la lluvia observada de la Tierra (WorldClim y CHELSA) a la escala y con las definiciones del planeta, y la compara con la del planeta (`config/referencias/tierra_lluvia.toml`) |
| `analisis/tiffmini.py` | Lector mínimo de GeoTIFF (sin librerías): una banda en tiras, con Deflate y predictor; también fila a fila por peticiones parciales |
| `analisis/bordes_bird.py` | Mide los bordes de placa de la Tierra (Bird 2003) en la malla tectónica, con el método y la regla de clasificación del modelo, y la convergencia de sus fosas y de sus choques (SUB, OCB y CCB tramo a tramo, y en la malla; `config/referencias/tierra_bordes.toml`) |

## Ajustes desde la línea de comandos

Los binarios `agentes` y `terreno` aceptan `--set ruta=valor` (varias veces), que cambia la config sin editar el archivo:

```bash
target/release/terreno --set climate.k_oro=60 --set run.name=k60
target/release/agentes --set run.total_ticks=36500 --set habitat.sequia.inicio_dia=7300 \
                       --set habitat.sequia.duracion_dias=365 --set habitat.sequia.factor=0.5
target/release/agentes --set 'especies=[{ nombre = "humano", preset = "humano" }]'
```

- La ruta va por puntos, y un número elige un elemento de una lista: `especies.1.ajustes.p_conc`.
- El valor es un literal TOML. Si no lo es, se toma como texto.
- Una clave mal escrita da error (las configs rechazan los campos desconocidos).

## Informe de tiempos por sistema

Cada corrida de `agentes` y `terreno` escribe, aparte de su salida, `output/<nombre>_tiempos.json`: cuánto tarda cada sistema del tick, en total, como fracción del tiempo y por vez. La consola muestra las etapas que más tardan. No forma parte de la salida del modelo, porque cambia de una corrida a otra (CLAUDE §6), y `no_regresion.sh` lo ignora.

Primera medida (2026-11-05):
- **Planeta, un año:** el LOD se lleva el 53 % del tiempo, la difusión lateral el 20 % y el clima el 11 %.
- **Agentes, 100 años:** la demanda se lleva el 32 %, la energética el 22 % y las métricas el 17 %.

## Diagnóstico del planeta en la salida

Desde el esquema v5, la salida del planeta (`<nombre>_terreno.json`) trae un bloque `diagnostico`, con las cifras que se comparan con la Tierra. Con el relieve tectónico (esquema v6) añade `diagnostico.relieve` (océano, pendientes a 25 km, plataforma, ley de Hack, cuencas cerradas, espectro por escala) y la historia geológica en `run.tectonica` (eventos, métricas de cada Myr, cuentas de la corteza continental). `run.diagnostico_relieve = true` lo calcula también con el relieve de ruido. Lo básico:
- tierra emergida;
- altura media y mediana de la tierra, y la curva hipsométrica por franjas;
- profundidad media del océano;
- lluvia sobre tierra y sobre el mar;
- tierra con el aire seco (`w` < 0,5);
- PPN de Miami.

La sombra pluviométrica (sotavento y barlovento de las cordilleras) se mide aún con `analisis/terreno.py` sobre el volcado por celda (`run.dump_world` y `run.dump_climate`).

## Referencia de la Tierra

```bash
python3 -I herramientas/analisis/referencia_tierra.py   # ~15 min la primera vez (descargas); después, ~10 min
```

Descarga una vez a `output/referencias/` (ignorado por git) y escribe `config/referencias/tierra_relieve.toml` (sí va al repositorio: solo tablas resumen, con nombre, versión y fecha de cada conjunto de datos). Solo usa la biblioteca estándar de Python.
- **ETOPO 2022** (NOAA): superficie y roca bajo el hielo, por OPeNDAP, cada 2 minutos de arco.
- **Paxman, Austermann y Hollyday (2022):** roca de la Antártida y Groenlandia sin hielo y con su rebote. Son HDF5 de 1,4–1,5 GB: `h5mini.py` lee solo 1 de cada 10 o 30 filas con peticiones parciales.
- **Seton et al. (2020):** edades del fondo oceánico (NetCDF-3).
- **Bird (2003):** áreas de las 52 placas.
- **Argus et al. (2011):** velocidad de la litosfera (de la publicación).

Calcula a las resoluciones del planeta (niveles 0 a 3 del cubo, ~208 a 26 km), con la media de los píxeles de cada celda:
- tierra emergida, altura media y mediana, y curvas de alturas de la tierra y del océano;
- espectro del relieve por escala y por tipo (lo que continúa el ruido de detalle);
- pendientes a 25 km y factor de lluvia de barlovento con la regla del modelo;
- plataforma continental y tierra en cuencas cerradas;
- edades del fondo y tamaños de las placas.

La versión de referencia es la **roca sin hielo con el rebote isostático** (decisión del responsable): roca desnuda como la del planeta, con el agua del océano actual.

## Lluvia observada

```bash
python3 -I herramientas/analisis/lluvia_observada.py   # ~40 s con las cachés; la primera vez, unos minutos más
```

El objetivo de la lluvia de barlovento sale de lluvia observada, a la misma escala (~26 km) y con la misma regla que el planeta: las laderas de barlovento de pie a cresta (decisión del responsable, 2026-11-05). El ×1,36 de la referencia del relieve es la regla del modelo sobre el relieve real, no lluvia medida. El programa escribe `config/referencias/tierra_lluvia.toml`, solo con la Tierra. Al lado muestra lo mismo medido en el volcado del planeta con la lluvia del modelo (`output/escenarios/planeta_world.json`, de `escenarios.py --solo planeta`).

Datos, descargados una vez a `output/referencias/`:
- **WorldClim 2.1:** lluvia mensual de 1970–2000 a 10' (~7 MB). Interpola pluviómetros, así que se queda corto en las montañas, y en los casquetes de hielo no sirve.
- **CHELSA 2.1:** bio12, la lluvia anual de 1981–2010 a 30''. No tiene reducciones internas: `tiffmini.py` lee 1 de cada 8 filas con peticiones parciales (~20 MB). Modela el viento y el relieve; satura en 6 553,5 mm.
- **El relieve:** el de `referencia_tierra.py` (roca rebotada sin hielo), guardado en caché en `output/cache/`.

Mide, en la Tierra y en el planeta, sobre las celdas de nivel 3:
- **las laderas de barlovento de pie a cresta**, la regla del objetivo (`terreno.laderas_pie_cresta`, la misma función en los dos mundos):
  - suben de cara al viento del modelo hasta una cresta de más de 2000 m, y contra el viento, hasta el pie, todo es más bajo que ellas: quedan fuera las mesetas y los sotaventos;
  - se camina por el paralelo cada 10 km, contra el viento hasta 500 km o hasta el mar; el pie es el primer punto a menos de 100 m de lo más bajo de ese camino;
  - se compara la lluvia de cada ladera con la del llano de su pie, también por franjas de altura; en la Tierra se omiten las de pie en el mar, y en el planeta se dan también con la lluvia del mar;
  - `terreno.py` da el cociente del planeta (`barlovento_ladera_ratio`, cociente de las medias por área) y su área (`barlovento_ladera_area_km2`).
- la lluvia de las laderas de barlovento de las cordilleras grandes, con la selección de `referencia_tierra.pendientes` y `terreno.py`;
- el factor de barlovento observado, lo que en el modelo es `m_oro`: la lluvia de cada celda de barlovento entre la de la tierra baja (h < 300 m) a 50–300 km contra el viento, o entre la de la tierra baja de su franja de 1° de latitud;
- el sotavento de las cordilleras grandes en latitudes medias y el resto, como `terreno.py`;
- la lluvia media de la tierra, con y sin la Antártida y Groenlandia, y por franjas de 10° (para revisar `c_P`).

## Bordes de placa de la Tierra en la malla del modelo

```bash
python3 -I herramientas/analisis/bordes_bird.py   # ~3 min; baja PB2002_steps de Bird la primera vez
```

Una malla de 52 km no puede dibujar una línea más fina que sus celdas, así que la longitud de los bordes del modelo se compara con la Tierra medida igual (adenda de balance §10). El programa reconstruye las placas de Bird (2003) como regiones de la malla tectónica (nivel 2) y mide cada borde con el método de `clasificar_bordes` (`L = 2·A·|∇χ|`). Da el tipo de dos maneras:
- **por la clase de Bird** del tramo más cercano (dorsal, rift, fosa, convergencia oceánica, choque, transformante oceánica y continental);
- **con la regla del modelo:** la normal y la velocidad relativa del tramo de Bird. La regla vigente es la de `nota_bordes_placa.md` (`regla_nota`): la normal del borde suavizado a 150 km y transformante a menos de 20° (el umbral que separa las clases de Bird). Se miden también las reglas anteriores (normal sin suavizar, 15° y 20°). La corteza oceánica es la que tiene edad en Seton et al. (2020).

Lo hace con las 52 placas de Bird y con una Tierra reducida a las 15 mayores (las pequeñas, fundidas con su vecina de más borde), como el modelo.

Para la nota de fosas da también lo que su regla llama fosa en cada clase de Bird (la referencia de la longitud de fosa es la fila SUB), la oblicuidad de los bordes y los tamaños de las placas.

## La Tierra real con la regla del modelo (nivel del mar)

```bash
python3 -I herramientas/analisis/tierra_regla_modelo.py   # ~2 min con los datos de referencia_tierra.py
```

Responde a una pregunta: si se pone la Tierra real en la regla del modelo, ¿sale el nivel del mar de la Tierra? Toma las edades reales del fondo (Seton et al. 2020), el grosor real del sedimento oceánico (GlobSed, Straume et al. 2019) y el volumen de agua del modelo (`gel_m`). Con la curva de profundidad por edad del modelo y la isostasia de su sedimento calcula el nivel del mar y la profundidad media del océano, y los compara con la Tierra (nivel 0, 3 667 m). El resto del planeta (continentes, plataformas y mares sin edad) conserva su roca real. También prueba GDH1, la raíz de la edad sin límite, un sedimento menos denso y la regla aplicada solo al fondo de menos de 80 Myr.

Da además la profundidad del zócalo (el fondo descargado de su sedimento) por clases de edad de 10 Myr, en mediana y en media, frente a las curvas. Así se ve dónde deja de bajar el fondo real. Por último, mide los escalones de edad entre celdas vecinas a la escala de la deriva (nivel 2), con las condiciones del nacimiento espontáneo en zonas de fractura de la nota de fosas v5. Escribe `config/referencias/tierra_nivel_mar.toml`.

GlobSed se lee de `output/referencias/globsed/globsed-v3.asc`. Es la copia en ASCII de ArcGIS que guarda CaltechDATA (doi:10.22002/k4070-ngc79, dominio público), porque NOAA retiró su página en mayo de 2025. Hay que bajar `GlobSed.zip` (65 MB, MD5 `4e3f112e0cdc820bd74d3b261414e2cd`) y sacar de él `globsed-v3.asc`.

También mide la **convergencia de las fosas** (diagnóstico de `nota_ciclo_fosas.md` §0): la componente perpendicular al borde (cm/año) por clases, su media ponderada por longitud y el área que se hunde. Lo hace tramo a tramo con la tasa de divergencia de Bird (clases SUB, fosas, y OCB, convergencia sin fosa) y en la malla con la regla de la nota. Fundir las placas pequeñas quita sobre todo fosas de microplacas de arco: las de las 15 mayores hunden 1,5 km²/año de los 2,7 de todas.

## La Tierra real con la regla del modelo (los continentes)

```bash
python3 -I herramientas/analisis/tierra_corteza_regla.py   # ~1,5 min con las cachés; la primera vez, unos minutos más
python3 -I herramientas/analisis/tierra_corteza_regla.py --planeta output/semillas/semilla_42_world.json
```

Responde a la misma pregunta para los continentes: con el grosor real de la corteza y de su sedimento (CRUST1.0, Laske et al. 2013), la isostasia continental del modelo (`isostasia::altura`), el océano con la regla de `tierra_regla_modelo.py` y el nivel del mar del modelo, ¿sale el relieve de ETOPO sin hielo?
- **Qué mide:** las filas del relieve de la tabla de cierre (tierra emergida, altura media y mediana, tierra por debajo de 200 m y por encima de 2 000 m, pendiente a ±25 km, plataforma), con la misma definición que `referencia_tierra.py`.
- **Cómo:** CRUST1.0 es de 1°. Se interpola bilinealmente a las celdas de nivel 3; en las continentales, solo con celdas de tipo continental. La pendiente y la plataforma se comparan también con ETOPO llevado a 1° (`tierra_1grado`).
- **Variantes:**
  - la regla solo en los continentes;
  - toda la Tierra con la regla;
  - con la densidad de cada capa de CRUST1.0;
  - con las mesetas oceánicas;
  - sin el sedimento continental o contándolo como corteza;
  - con el cero movido;
  - y dos cotas: la regla más el residuo medio de su tipo de corteza, y la recta real por tipo.
- **Diagnósticos:**
  - la altura real y la de la regla por grosor de la corteza, por tipo de corteza de CRUST1.0 y por grosor del sedimento;
  - el residuo frente al manto litosférico de LITHO1.0 (Pasyanos et al. 2014);
  - las rectas de la altura real;
  - dónde pone la regla la plataforma real.
- **El planeta:** con `--planeta` (por defecto, `output/semillas/semilla_*_world.json`), el grosor de la corteza continental del planeta frente a CRUST1.0: en tierra, bajo el mar y por percentiles.

Descarga la primera vez a `output/referencias/crust1/` los ficheros de CRUST1.0 que usa (`crust1.bnds`, `crust1.rho`, `CNtype1-1.txt` y `CNtype1_key.txt`, sacados por su nombre de los tar de igppweb.ucsd.edu) y a `output/referencias/litho1/` el tar de LITHO1.0 (12 MB), que se lee sin extraerlo. Guarda en `output/cache/` las celdas de nivel 3 de Seton y de GlobSed, y la litosfera en la malla de 1°. Escribe `config/referencias/tierra_corteza.toml`, solo con la Tierra.

## La deriva en varias semillas

```bash
SEMILLAS=42,1,2,3,4 VARIANTES="tectonica.lambda_esp=0;tectonica.lambda_esp=2e-7" \
  cargo test --release -p abm-terreno --test tectonica deriva_en_varias_semillas -- --ignored --nocapture
E1=0.005,0.01,0.02 N_ERO=2,3 D0=1600 \
  cargo test --release -p abm-terreno --test tectonica calibracion_relajacion -- --ignored --nocapture
D0=100,400,1600 \
  cargo test --release -p abm-terreno --test tectonica calibracion_flujo_lateral -- --ignored --nocapture
SEMILLA=3 VARIANTE="tectonica.n_ero=3" UMBRAL=140 \
  cargo test --release -p abm-terreno --test tectonica diagnostico_pilas -- --ignored --nocapture
SEMILLAS=42,1,2,3,4 \
  cargo test --release -p abm-terreno --test tectonica diagnostico_fondo_viejo -- --ignored --nocapture
```

`VARIANTES` son ajustes de la config como los de `--set`: separados por comas dentro de una variante y por `;` entre variantes. Sin ella, la config tal cual.

La deriva del planeta es caótica: un cambio pequeño cambia qué placas chocan y cuándo, y la semilla 42 de la batería da solo una realización. Entre semillas, la exportación de sedimento al mar varía ±0,5 AU y el cambio del volumen continental ±5 puntos. Para decidir una calibración se mira la media de varias semillas. Esta prueba exploratoria corre una deriva por cada semilla y valor, todas a la vez (~4 min con 10).

Por cada una da:
- el balance de la corteza;
- el consumo y el solape por tipo de borde, en las uniones triples y en los finales de fosa;
- los picos de más de 80 km y si su muestra estaba oculta el paso anterior;
- la erosión potencial por exceso de espesor;
- el tiempo de cada etapa del paso, con la traza de `Deriva::dar_paso`;
- las fosas de la nota de fosas: longitud activa y difusa, edad, tirón, nacimientos y muertes;
- la edad del fondo;
- las cuatro medidas de fuerzas del §10 de la nota: oblicuidad, tamaños de las placas, potencia de cada fuerza y área creada y consumida;
- el sedimento: el máximo de una muestra, el que no cupo en el mar de su desembocadura y el que queda en cuencas cerradas;
- la nota de fosas v4: los márgenes pasivos (vida de los que acaban, edad media y máxima al final), la invasión (km/Myr por transformantes y por márgenes, intentos de separar y fallos), las roturas de losa y los choques (convergencia, duración ponderada por la corteza solapada y pérdida con la losa colgando y sin ella).

Las otras pruebas exploratorias:
- `calibracion_relajacion`: cuánto tarda la corteza engrosada de India–Asia en volver a 38 km, con cada `E_1`, `n_ero` y `D_0`;
- `calibracion_flujo_lateral`: espesor máximo y meseta de India–Asia con cada `D_0`;
- `diagnostico_pilas`: dos pasadas de la misma deriva. La primera halla los pasos en que alguna etapa deja una muestra por encima de `UMBRAL`, aunque otra la baje en el mismo paso. La segunda sigue esa muestra etapa a etapa, con su placa, su borde más cercano y sus vecinas;
- `diagnostico_fondo_viejo`: dónde queda al final el fondo de más de 180 Myr (en qué placa, a qué distancia de su fosa y de su orilla continental);
- `diagnostico_procedencia_del_fondo` (nota de fosas v6): cada 100 Myr, de dónde salen los escalones de edad enormes y cuánto del fondo de más de 180 Myr y de los márgenes pasivos viejos viene del estado inicial. Usa la procedencia de cada muestra (`Corteza.proc`: cómo nació, con qué evento llegó a su placa y por qué vía le llegó su corteza continental), que el modelo no lee.

`deriva_en_varias_semillas` da además:
- lo que choca y lo que se pierde en los choques por la clase de la corteza que entra y por la vía que la puso allí;
- el perfil de los márgenes pasivos al final, con la anchura de su franja estirada (adenda v8).

## La tabla de cierre del planeta

```bash
herramientas/planeta_semillas.py                        # semillas 42,1,2,3,4, a la vez (~5 min)
herramientas/planeta_semillas.py --semillas 7,8 --set tectonica.e1_km_myr=0.004
```

Corre el planeta entero (génesis, relieve y un año de clima) con cada semilla y lo analiza como la batería (`analisis/terreno.py`). Da la media, la desviación y los extremos de cada fila:
- **frente a ETOPO** (la roca sin hielo con su rebote, a 26 km): tierra emergida, altura media de la tierra, fracción por debajo de 200 m, pendiente media y anchura de la plataforma;
- **el nivel del mar al final de la deriva y la profundidad media del océano**, frente a la Tierra (0 y 3 667 m; la Tierra real con la regla del modelo da −150 m, `tierra_regla_modelo.py`);
- **frente a los objetivos de la adenda y de la nota de fosas:** corteza máxima, edad del fondo, `f_cont`, cambio de volumen, velocidad de las placas, sedimento al mar, longitud de fosa, área que se hunde, vida y edad de los márgenes pasivos (Bradley 2008) y tiempo;
- **los choques, frente a Bird CCB:** longitud, convergencia, duración y pérdida antes y después de la rotura de la losa; la pérdida total, frente a 0,4–0,7 AU (adenda v7).

Escribe cada corrida en `output/semillas/` y el resumen en `output/semillas/cierre.json`. El tiempo de cada semilla está medido con las cinco a la vez: el de una sola es el de la batería.

## La batería de escenarios

```bash
herramientas/escenarios.py            # compila y pasa todo (~1 min)
herramientas/escenarios.py --listar
herramientas/escenarios.py --solo planeta,humanos_hambruna
```

Escribe las salidas en `output/escenarios/` y el informe en `output/escenarios/informe.json`. Por cada métrica guarda el valor, el margen y si pasa; también el tiempo y las etapas que más tardan.

**Escenarios:**
- **`planeta`:** el planeta por defecto, con el relieve tectónico (relieve frente a la Tierra, deriva de placas, lluvia, biomas, ríos, sombra pluviométrica, conservación del agua).
- **`agentes_equilibrio`:** 100 años de agentes (población frente a `N_max`, fecundidad humana, esperanza de vida, balance de energía).
- **`agentes_sequia`:** una sequía al 50 % durante un año.
- **`humanos_hambruna`:** solo humanos, hambruna al 30 %.

**Márgenes.** Rodean el valor actual: avisan cuando un cambio mueve algo que emerge al juntar sistemas, no son objetivos de realismo. Si un cambio intencionado saca una métrica de su margen:
1. se documenta en la nota de diseño del cambio;
2. se actualiza el margen en `escenarios.toml`, con su nota.

Cada nota dice qué vigila la métrica y, cuando se sabe, el valor real de referencia. Los valores irreales conocidos están marcados como pendientes, como la tierra emergida o la caída del herbívoro en la sequía.

**Añadir una métrica:**
1. si no existe, escribir su función en `MEDIDAS` (`escenarios.py`), apoyándose en `analisis/`;
2. añadir un `[[escenario.metrica]]` con su margen y su nota.
