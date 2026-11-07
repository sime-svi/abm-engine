# Herramientas

Programas fuera del motor para verificar, medir y consultar las corridas (CLAUDE §2.1: pueden usar Python o bash). Las métricas que sostienen una decisión salen de aquí o de la propia salida JSON, nunca de cálculos sueltos (CLAUDE §5).

| Herramienta | Para qué |
|---|---|
| `verificar.sh` | Toda la verificación de CLAUDE §10 en un comando: formato, clippy, tests, batería de escenarios y, con `--no-regresion REF`, no regresión byte a byte frente a un commit |
| `escenarios.py` + `escenarios.toml` | Batería de escenarios de referencia: corridas fijas con unas pocas métricas clave, sus márgenes y su presupuesto de tiempo |
| `no_regresion.sh` | Compara byte a byte la salida actual con la de un commit anterior (para los cambios que no deben alterar nada) |
| `analisis/agentes.py` | Resume una corrida de agentes: población frente a `N_max`, fecundidad, esperanza de vida, hambre, mortalidad por sexo |
| `analisis/terreno.py` | Resume y compara corridas del planeta: biomas, diagnóstico, `c_P`, relieve tectónico y, con el volcado por celda, la sombra pluviométrica y las laderas de barlovento de pie a cresta |
| `analisis/referencia_tierra.py` | Calcula las tablas de la Tierra con las que se compara el relieve (`config/referencias/tierra_relieve.toml`) |
| `analisis/h5mini.py` | Lector mínimo de HDF5 (sin librerías): lee filas sueltas de rejillas contiguas por peticiones parciales |
| `analisis/lluvia_observada.py` | Mide la lluvia observada de la Tierra (WorldClim y CHELSA) a la escala y con las definiciones del planeta, y la compara con la del planeta (`config/referencias/tierra_lluvia.toml`) |
| `analisis/tiffmini.py` | Lector mínimo de GeoTIFF (sin librerías): una banda en tiras, con Deflate y predictor; también fila a fila por peticiones parciales |
| `analisis/bordes_bird.py` | Mide los bordes de placa de la Tierra (Bird 2003) en la malla tectónica, con el método y la regla de clasificación del modelo, y la convergencia de sus fosas (SUB y OCB tramo a tramo y en la malla; `config/referencias/tierra_bordes.toml`) |

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

También mide la **convergencia de las fosas** (diagnóstico de `nota_ciclo_fosas.md` §0): la componente perpendicular al borde (cm/año) por clases, su media ponderada por longitud y el área que se hunde. Lo hace tramo a tramo con la tasa de divergencia de Bird (clases SUB, fosas, y OCB, convergencia sin fosa) y en la malla con la regla de la nota. Fundir las placas pequeñas quita sobre todo fosas de microplacas de arco: las de las 15 mayores hunden 1,5 km²/año de los 2,7 de todas.

## La deriva en varias semillas

```bash
SEMILLAS=42,1,2,3,4 VARIANTES="tectonica.lambda_esp=0;tectonica.lambda_esp=2e-7" \
  cargo test --release -p abm-terreno --test tectonica deriva_en_varias_semillas -- --ignored --nocapture
E1=0.005,0.01,0.02 N_ERO=2,3 \
  cargo test --release -p abm-terreno --test tectonica calibracion_relajacion -- --ignored --nocapture
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
- las cuatro medidas de fuerzas del §10 de la nota: oblicuidad, tamaños de las placas, potencia de cada fuerza y área creada y consumida.

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
