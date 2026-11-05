# Fase «Terreno» — el escenario físico del mundo

> Explicación en lenguaje llano (CLAUDE §8.1). Aquí se cuenta **qué** se está construyendo y **por qué**; el **cómo** formal está en los documentos de diseño técnico de al lado.

## ¿Qué es esta fase?

Hasta ahora el motor tenía un tablero plano y abstracto: una cuadrícula donde unos agentes buscaban "recurso". Esta fase construye debajo de ellos un **mundo físico de verdad**: un planeta esférico con montañas y valles, un clima que cambia con la latitud, la altura y las estaciones, lluvia, nieve, ríos, lagos, y vegetación que crece donde puede.

La idea central es que **nada de eso se pinta a mano**. No colocamos "aquí un desierto, allí un bosque". Ponemos las leyes físicas (cómo baja la temperatura con la altura, cómo la montaña frena las nubes, cómo el agua resbala y se acumula) y dejamos que el paisaje **emerja solo** a partir de una semilla. Cambias la semilla y sale otro planeta, siempre reproducible: misma semilla → mismo mundo, hasta el último bit.

**Vocabulario mínimo** (cada término, traducido al momento):
- **Emergencia**: que un patrón complejo (un desierto) aparezca solo, como consecuencia de reglas simples, sin haberlo dibujado.
- **Bioma**: un tipo de paisaje vivo (desierto, selva, tundra…). Aquí sale de combinar temperatura y humedad.
- **Sombra pluviométrica**: el desierto que se forma "detrás" de una cordillera, porque la montaña se queda con toda la lluvia en la cara que da al viento.
- **Determinismo**: que la simulación se pueda repetir exactamente, sin azar incontrolado. Esencial para que los resultados sean científicos.
- **Cubo esférico** (*cubed sphere*): truco para cubrir una esfera con celdas cuadradas sin que los polos se deformen. Se toma un cubo de 6 caras y se "infla" hasta que es una esfera.
- **LOD** (*Level of Detail*, nivel de detalle): calcular con lupa solo donde importa, y con brocha gorda donde no, para que un planeta entero quepa en un ordenador.

## ¿Por qué en incrementos?

La metodología del proyecto (la "pirámide invertida", CLAUDE §4) prohíbe construir el tejado antes que los cimientos: no se sube de nivel hasta que lo de abajo es **estable y verificado**. La visión completa del terreno (el documento [`diseno_terreno_espacio_geografia.md`](./diseno_terreno_espacio_geografia.md)) es enorme: incluye ríos que tallan valles a lo largo de milenios y una resolución que se afina sola donde hay acción. Hacer todo eso de golpe sería frágil.

Por eso la fase se parte en incrementos. El primero está diseñado en [`incremento-1-diseno.md`](./incremento-1-diseno.md) y el segundo en [`incremento-2-diseno.md`](./incremento-2-diseno.md).

## Incremento 1 — qué logra y qué deja fuera

**Logra** (sobre una esfera real, reproducible):
- El planeta esférico con relieve, y de él salen la temperatura, las estaciones (con hemisferios opuestos: cuando en el norte es verano, en el sur es invierno), la lluvia con sombra pluviométrica, la nieve, la humedad del suelo, los ríos y los lagos, y la vegetación con sus biomas.
- Una prueba de que **el agua se conserva**: lo que entra como lluvia, menos lo que se evapora, menos lo que llega al mar, cuadra. Es la señal de que el modelo no "fabrica" ni "pierde" agua por error.

**Deja fuera a propósito** (para incrementos siguientes):
- **El relieve vivo** (erosión): en el incremento 1 las montañas no se desgastan ni los ríos tallan valles; el relieve es fijo. Eso llega en el incremento 3.
- **La resolución que se afina sola** (LOD adaptativo): las estructuras de datos ya están **preparadas** para ello, pero de momento el mundo corre a un detalle uniforme. Encender el afinado dinámico es el incremento 2.

## Consideraciones y decisiones tomadas (13-07-2026)

- **Esfera completa desde el principio**, no una versión plana de prueba: el responsable prefirió pagar ya la complejidad geométrica del cubo esférico a cambio de no tener que rehacerlo. Las esquinas del cubo (donde se juntan 3 caras) se resuelven de forma sencilla y sin artefactos.
- **Estructuras preparadas para el futuro, pero corriendo simple**: se decidió construir el andamiaje del detalle adaptativo (LOD) desde ya —para no reescribir la física después— pero sin activarlo todavía. Lo difícil del LOD no es la estructura, es la dinámica; y esa se deja para cuando la física esté probada.
- **Tick fino (~1 día, 365 al año)**: se eligió realismo estacional a cambio de recalibrar todos los ritmos del modelo. Consecuencia anotada: la escala de vida de los agentes actuales (~200 ticks) habrá que reajustarla cuando se acoplen a este mundo.
- **Los números por defecto son un punto de partida, no dogma**: el criterio de que el modelo "está bien" no son los valores concretos, sino que el balance de agua cierre y que los biomas aparezcan en las franjas de latitud esperadas.

## Estado: incremento 1 **implementado y verificado** (2026-07-13)

Existe el módulo `abm::terreno` (en `src/terreno/`), el binario `terreno` (`cargo run --bin terreno`), su config `config/terreno.toml` y sus tests (`tests/terreno.rs`). Se ejecuta así:

```
cargo run --bin terreno            # usa config/terreno.toml → output/terreno_terreno.json
cargo test --test terreno          # tests de la fase
```

**Qué se logró de verdad**, con la config por defecto (planeta de 13 824 celdas, un año de 365 ticks):

- Un planeta esférico reproducible con relieve tipo Tierra (tierra a ~490 m de media, océano ~61 %).
- Emergen **los 9 biomas** (desde selva ecuatorial hasta hielo polar), la **sombra pluviométrica**, ~1500 celdas de **río** y decenas de **lagos**, con la temperatura bien graduada del ecuador (~23 °C) al polo (~−19 °C).
- **El agua se conserva** con precisión de máquina (residual relativo ~1e-13): la prueba clave de que el modelo no inventa ni pierde agua.
- **Reproducibilidad total**: misma semilla → mismo JSON byte a byte, incluso entre procesos distintos.

### Hallazgos y decisiones durante la implementación

- **Bug de geometría cazado a tiempo**: un desajuste en el orden de los ejes de las caras del cubo hacía que dos caras se solaparan (¡y el área total seguía sumando bien, enmascarándolo!). Lo delató un test de que los vecinos ortogonales sean recíprocos. Lección: el test de "área = 4πR²" no basta para validar el cubo esférico; hace falta también el de vecindad.
- **La conservación del agua es exigente**: al principio se escapaba ~0.01 % del agua. La causa era sutil: la difusión de humedad movía "milímetros de lámina de agua" iguales entre celdas de área ligeramente distinta (el cubo esférico no tiene todas las celdas del mismo tamaño), y milímetros iguales sobre áreas distintas no son el mismo volumen. Corregido transfiriendo volumen: ahora cierra a 1e-13.
- **Los números por defecto del diseño eran demasiado fríos y secos**: la elevación repartida de forma lineal dejaba la tierra altísima (mundo helado), y la lluvia de referencia no llegaba para selva tropical. Se añadió una curva de relieve realista (`land_exp`) y se subió la lluvia de referencia (`p_ref`). Todo por configuración, sin tocar código.
- **Alcance recortado con honestidad**: los ríos usan un modelo de "un solo receptor" (más simple que el D-infinito del plano), y los lagos son estáticos (sin volumen dinámico todavía). Son simplificaciones conscientes, irrelevantes mientras no haya erosión, y están documentadas en el diseño técnico (§12).

Lo que **quedaba para los siguientes incrementos**: la resolución que se afina sola (LOD adaptativo, incr. 2, ya hecho: ver abajo) y el relieve vivo que talla valles (erosión y pedogénesis, incr. 3).

## Incremento 2 — la resolución que se afina sola

### Qué logra

Hasta ahora el planeta se dividía en celdas todas del mismo tamaño (unos 200 km de lado). Eso es tosco donde pasan cosas (una costa, un río) y derrochador donde no pasa nada (el océano abierto). Ahora **cada celda puede partirse en 4 más pequeñas** cuando la zona lo merece, y esas 4 pueden volver a juntarse cuando deja de merecerlo. Hasta 3 veces: de ~200 km a ~26 km.

**Vocabulario mínimo:**
- **Árbol de cuatro** (*quadtree*): la estructura que resulta de ir partiendo celdas en 4. Cada cara del cubo es un árbol; las celdas que existen de verdad en cada momento son las **hojas** del árbol.
- **Subdividir / colapsar**: partir una celda en 4 hijas / juntar 4 hermanas en su madre.
- **Regla 2:1**: dos celdas vecinas nunca se llevan más de un nivel de tamaño (una celda no puede tocar a otra 4 veces más pequeña). Mantiene la malla ordenada y simplifica quién es vecino de quién.
- **Histéresis**: margen para no ir y venir. Se subdivide cuando una diferencia supera un umbral, pero solo se colapsa cuando baja **claramente** por debajo (la mitad).

**Dónde se afina** (todo regulable en `config/terreno.toml`, sección `[lod]`): costas, ríos y lagos; relieve abrupto (más de 500 m de diferencia con un vecino); y contrastes fuertes de temperatura (4 °C) o de humedad del suelo.

**Lo más importante: no se inventa ni se pierde nada al cambiar de tamaño.**
- Lo que es fijo del terreno (altura, tipo de suelo…) se **recalcula** para cada celda nueva a partir de la semilla, no se copia de la madre. Por eso la celda pequeña tiene su altura "verdadera" con más detalle: cada nivel añade una capa más de detalle al relieve (una **octava** más de ruido).
- Lo que cambia con el tiempo (agua del suelo, nieve, vegetación) se **reparte** entre las hijas de forma que el total se conserva exactamente. Y si se parte una celda y se vuelve a juntar enseguida, queda como estaba (**independencia del camino**).
- Cuando al afinar la costa resulta que un trozo era mar, su agua se manda al mar y **se apunta** en el balance. El balance de agua del planeta sigue cuadrando: el error es de unas pocas billonésimas.

### Cómo se ejecuta

```
cargo run --release --bin terreno    # usa config/terreno.toml (con LOD) → output/terreno_terreno.json
cargo test --test terreno            # tests de la fase (incrementos 1 y 2)
```

Con el LOD el planeta tiene ~190 000 celdas en lugar de 13 824, así que conviene `--release` (modo optimizado): un año simulado tarda ~13 s. Para volver al mundo uniforme del incremento 1 basta poner `l_max = 0` en `[lod]`.

### Consideraciones y hallazgos (04-11-2026)

- **Primero se revisó el diseño contra el código** (versión 1.1 del diseño). El de julio daba por hechas cosas que el código del incremento 1 no hacía: por ejemplo, la altura dependía de toda la malla y no solo de la posición, y nada decía qué hacer con el agua de una celda que al afinarse resulta ser mar. Se cerraron seis huecos en el papel antes de programar.
- **Los ríos y la malla se influyen mutuamente, y eso daba vueltas sin fin.** El río se calcula sobre la malla (cada celda manda su agua a su vecina más baja, la que llamamos su **receptor**), y la malla se afina siguiendo al río. Al juntar 4 celdas, la grande podía recoger el agua de las 4 y "convertirse en río", lo que obligaba a volver a partirla. En el planeta completo había un vaivén permanente de ~270 cambios por revisión. Se resolvió con dos ideas:
  - un **ensayo en seco**: antes de juntar celdas, se prueba en una copia y se descarta lo que provocaría que se volvieran a partir;
  - una **marca pegajosa**: lo que alguna vez fue río o lago ya no se vuelve a juntar.
  Con el relieve fijo, la malla deja de cambiar en ~13 revisiones. Lo cazó un test escrito a propósito para eso.
- **El resultado global apenas cambia**, que es lo que se buscaba: misma temperatura media (9,13 °C), misma fracción de tierra (39 %) y biomas parecidos. Lo que gana es detalle local: costas y ríos dibujados a 26 km en vez de a 200.
- **Aparecen muchos lagos pequeños** (~9 700 celdas): con más detalle, el relieve de ruido tiene muchas hondonadas cerradas, porque todavía no hay erosión que las vacíe. Se espera que el incremento 3 lo corrija.
- **Ajustes a la física del incremento 1**, necesarios para que todo sea coherente al cambiar de tamaño: la pendiente, la distancia al mar, el reparto de humedad entre vecinas y el sorteo del recurso se calculan ahora de otra forma. Se comprobó que en la malla uniforme apenas mueven los resultados (los biomas cambian menos de medio punto).

Lo que **queda para los siguientes incrementos**: el relieve vivo que talla valles (erosión y pedogénesis, incr. 3), y que la malla se afine también donde haya agentes o eventos, cuando existan.

## Incremento 3 — el relieve vivo

### Qué logra

Hasta ahora las montañas y los valles del planeta eran fijos: salían de la semilla y no cambiaban nunca. Ahora el relieve **evoluciona**:
- **Los ríos excavan.** El agua que corre arranca material, más cuanto más caudal lleva y más empinado es el terreno. Así se forman valles.
- **El sedimento viaja río abajo** y rellena las hondonadas que encuentra; lo que llega al mar se pierde.
- **Las laderas se suavizan** poco a poco (el material "resbala" de lo alto a lo bajo).
- **El suelo se forma y se pierde.** La roca se convierte en suelo, más rápido donde hace calor y llueve; la erosión se lo lleva. Donde se agota, aflora la roca, que retiene poca agua.

**Vocabulario mínimo:**
- **Génesis geológica:** antes de empezar la simulación, se "envejece" el planeta un millón de años de golpe, en pasos de 5 000 años. El mundo arranca ya tallado.
- **Esquema implícito:** una forma de calcular la erosión que no se descontrola aunque el paso de tiempo sea enorme (miles de años). Sin él, la génesis sería inviable.
- **Hondonada (depresión):** un hueco sin salida donde el agua se queda. Con el ruido de la semilla salían miles, y la erosión las vacía.

### El problema que resuelve

Con la malla fina del incremento 2 aparecían **unos 9 700 lagos falsos**: hondonadas del ruido que en un planeta real se habrían rellenado hace mucho. Para los futuros agentes eso habría sido un desastre, porque el agua es infranqueable y cuenta como fuente de agua. Tras la génesis geológica quedan **unos 50**, del orden de los lagos grandes de la Tierra.

### Cómo se ejecuta

```
cargo run --release --bin terreno    # génesis geológica + un año → output/terreno_terreno.json
cargo test --test erosion            # tests del relieve vivo
```

La primera ejecución tarda unos 37 s en modo optimizado, porque la génesis simula un millón de años. Luego la génesis se guarda en `output/cache` y las siguientes tardan unos **8 s**. La caché se rehace sola si cambias la semilla, cualquier parámetro o el código. Todo se regula en `config/terreno.toml`, en las secciones `[erosion]` y `[soil]`. Con `geo_steps = 0` y `run_every = 0` vuelves exactamente al relieve fijo del incremento 2.

### Resultados

- **Lagos falsos:** de 9 681 a unos 50.
- **Relieve:** la tierra queda de media 85 m más baja (de 491 a 406 m) y la fracción de tierra apenas cambia (de 39,3 % a 39,0 %). Siguen saliendo los 9 biomas.
- **Clima:** el planeta se calienta 0,2 °C de media, porque con la tierra más baja hace menos frío.
- **Durante la simulación** la erosión sigue a su ritmo real: unos 18 000 millones de m³ de sedimento al año en todo el planeta. Es del mismo orden que lo que llevan al mar los ríos de la Tierra de verdad.
- **Material:** se puede comprobar como el agua: lo erosionado es igual a lo depositado más lo que llega al mar. Cuadra con un error de 31 m³ sobre 18 000 billones de m³.

### Consideraciones y hallazgos (05-11-2026)

- **Las cuatro decisiones del diseño se tomaron como se recomendaba:** génesis geológica + erosión realista; lagos dinámicos y lo demás para el incremento 4; erosión según la lluvia que recoge cada río; y el mar como sumidero del sedimento (sin deltas).
- **El suelo necesitaba dos límites.** Con la primera fórmula, la roca desnuda no guardaba nada de agua y los lagos rellenados de sedimento guardaban cantidades absurdas. Esa diferencia de humedad entre vecinas hacía que la malla se afinara sin parar (14 500 subdivisiones de golpe). Se limitó a lo físico: solo retiene agua útil la zona de raíces (1,5 m) y la roca conserva algo (30 %), porque una celda de decenas de kilómetros nunca es solo roca.
- **Al afinar la malla durante la simulación volvían a salir lagos falsos.** El detalle fino nuevo no está erosionado. Ahora un cambio de malla no puede crear hondonadas: si aparece una donde no la había, se rellena como lo habría hecho el sedimento, y ese volumen se apunta aparte en el balance.
- **Con la erosión apagada, el resultado es idéntico al del incremento 2**, comprobado valor a valor contra el programa antiguo.

### Revisión y ajustes (05-11-2026)

Una revisión externa propuso tres ajustes antes de seguir. Se aplicaron los tres, con dos matices que salieron al medir:
- **El relleno de hondonadas ya no crea material.** Lo que sube en una hondonada lo pierden sus vecinas del borde, que bajan todas lo mismo y nunca por debajo del nivel de desagüe. Así tampoco aparecen hondonadas nuevas en el borde. Si el borde no tiene suficiente, la hondonada se queda como un lago poco hondo. Las cuentas cuadran hasta el redondeo del ordenador.
- **El suelo mínimo depende del tamaño de la celda.** Una celda de 200 km siempre conserva bolsas de suelo (30 %); una de 26 km, menos (11 %); a 1 km un acantilado puede ser solo roca. *Matiz:* expresado como "capacidad mínima", al afinar la malla se perdía agua que no volvía al juntar las celdas; expresado como **profundidad mínima de suelo**, que las celdas hijas heredan, ese problema desaparece.
- **Prueba de "independencia del camino":** una misma zona (un sexto del planeta) se simula fina todo el rato, gruesa→fina→gruesa y gruesa todo el rato. El material y la erosión coinciden al 0,3 %. *Matiz:* cuando una zona está fina **llueve algo más** (+4 % si lo está todo el rato). Con más detalle se ven pendientes más fuertes, y las montañas aumentan la lluvia más de lo que la reducen. No es un fallo de la malla: es un efecto de escala de la física, que ya estaba en el incremento 2. Corregirlo cambiaría resultados anteriores, así que queda como decisión pendiente.
- **Ya estaban:** la histéresis (solo se juntan celdas cuando la diferencia baja de la mitad del umbral), que la formación de suelo baje con la profundidad, y que el agua que no cabe al afinar se vaya al mar contabilizada.
- **Nuevos:** un **tope** de subdivisiones por revisión (el 2 % de las celdas, las más urgentes primero) y la **caché** de la génesis.

Lo que **queda para los siguientes incrementos**: los lagos de verdad (que se llenan, se vacían y se desbordan), el agua repartida entre varias celdas de bajada, la fertilidad que cambia con el tiempo y el registro de eventos (incremento 4); y que la malla se afine también donde haya agentes, cuando existan.

## Nota de lluvia orográfica (05-11-2026)

El responsable pidió corregir un efecto que salió en el incremento 3: con la malla fina **llovía más** que con la gruesa (+4 %). Era peligroso para lo que viene. Donde haya agentes, la malla se afinará; si eso trae más lluvia, habrá más comida y más agentes, y la malla se afinará aún más. Es un bucle artificial. La nota está en `nota_terreno_lluvia_orografica.md` y su §8 cuenta lo hecho.

- **Lo que pedía la nota.** La lluvia de montaña (la que aumenta al subir el aire por una ladera, el **efecto orográfico**) ya no mira la pendiente de cada celda. Mira el relieve **suavizado a 25 km**, la escala a la que el viento arrastra las nubes antes de que llueva. Ese relieve sale siempre del mismo nivel de malla, así que da igual que la zona esté fina o gruesa. Hecho y comprobado con un test.
- **La sorpresa.** La culpa del +4 % no era de la orografía (me equivoqué en el diagnóstico del incremento 3). Era de la **distancia al mar**, que decide cuánta humedad llega tierra adentro. Se medía hasta el centro de la celda de mar más cercana, así que siempre sobraba media celda de mar. Al afinar, esa media celda encoge y la costa parece "acercarse". Ahora se mide **hasta la línea de costa**. A la resolución real, el +3,9 % baja a +0,03 %.
- **Otra sorpresa.** Con el valor actual del parámetro (`k_oro = 0,5`), la lluvia de montaña apenas se notaba: en el 98 % de la tierra cambia la lluvia menos de un 1 %. **Las sombras de lluvia (el lado seco de las montañas) no existían.** Para que existan, el parámetro tendría que valer unos 40. Lo decide el responsable.
- **Y una tercera.** Sin el exceso de lluvia que lo tapaba, se ve que la erosión depende algo de la resolución (1–3 % en la prueba). Queda vigilada.
- **Nada se rompe.** Con las dos correcciones apagadas, el programa da exactamente la misma salida que antes. Con ellas encendidas, el planeta cambia poco: llueve un 0,4 % más en total (algo más en las costas), hay un 1,6 % menos de desierto y un 4 % más de selva.

