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


## Nota de sombra pluviométrica (05-11-2026)

El responsable pidió que la lluvia de montaña se notara de verdad. En la nota anterior salió que, con el valor que tenía `k_oro`, **las sombras de lluvia no existían**: en el 98 % de la tierra la montaña cambiaba la lluvia menos de un 1 %. La nota está en `nota_terreno_sombra_pluviometrica.md`, y su §6 cuenta lo hecho y lo medido.

**Vocabulario:**
- **Barlovento:** la cara de la montaña que mira al viento. El aire sube, se enfría y suelta su agua: llueve más.
- **Sotavento:** la cara de detrás. El aire baja y se calienta: llueve menos.
- **`k_oro`:** cuánto reacciona la lluvia a la pendiente que el viento tiene que subir. Es como el volumen de un altavoz: la pendiente es la música y `k_oro` dice cuánto se oye.

### Qué se hizo

1. **Subir `k_oro`.** La nota proponía 40 y pedía comprobarlo con un objetivo: en las laderas de barlovento de las cordilleras grandes, que llueva de 2 a 3 veces más. Con 40 solo llovía 1,4 veces más. Nuestras montañas, vistas a la escala a la que la lluvia "ve" el relieve (25 km), son unas cuatro veces más suaves que las de la Tierra. Hizo falta **150** para llegar a 2,1–2,2 veces.
2. **El aire se seca al cruzar montañas (agotamiento de humedad).** El aire que subió una cordillera ya soltó su agua, y no la recupera al bajar: detrás llueve menos durante cientos de km, hasta que el aire vuelve a cargarse de humedad sobre el mar o, poco a poco, sobre la tierra. Cada punto de tierra mira hacia atrás por el camino del viento, de 25 en 25 km, y suma cuántos metros ha tenido que subir el aire recientemente; cuanto más ha subido, menos humedad le queda. Como el viento del modelo solo sopla hacia el este o el oeste, ese camino es el propio paralelo.
3. **Mantener la lluvia total.** Secar el aire quita un 16 % de la lluvia de toda la tierra. La nota pide que el total no cambie más de un 5 %, así que, al acabar de generar el planeta, se calcula un factor `c_P` (aquí 1,18) que reparte esa lluvia de vuelta sobre toda la tierra.
4. **Una corrección mía: el mar es plano para el aire.** Con un `k_oro` grande salió que el fondo del mar contaba como montaña: movía más de un 10 % la lluvia del 86 % del océano y la de las costas. El aire está en la superficie del agua, no en el fondo. Ahora, sobre el mar no hay efecto de montaña, y en la costa la pendiente se mide desde el nivel del mar. Se puede apagar.

Cada cosa tiene su interruptor en `config/terreno.toml`. Con los cuatro apagados (`config/experiments/terreno_sin_sombra.toml`), el planeta sale **exactamente igual que antes**, bit a bit.

### Qué cambia en el planeta

- **Aparecen desiertos detrás de las montañas.** En latitudes medias, a 200–500 km detrás de las cordilleras de más de 2 000 m, la lluvia baja de 688 a 231 mm al año y el desierto pasa del 7,6 % al 15,8 % de esa zona. La pradera de esa zona casi desaparece. Es lo que buscaba la visión madre (la estepa patagónica, la Gran Cuenca de EE. UU.).
- **En el resto llueve algo más**, porque el factor `c_P` devuelve el agua: la selva crece un 35 % (del 2,1 % al 2,8 % del planeta) y el bosque templado baja (del 6,8 % al 5,6 %).
- **Para los futuros agentes, algo menos de comida.** La productividad de las plantas, medida con la misma fórmula que usan los agentes (el modelo Miami), baja un 2,9 % en el total del planeta, y lo mismo la población de herbívoros que podría mantener. Pasa aunque la lluvia total sea la misma: las plantas aprovechan peor el agua si unas zonas reciben de más y otras de menos.
- **Coste:** generar el planeta tarda lo mismo (~43 s la primera vez). Con la génesis guardada, un año de simulación pasa de 8 a 10 s.

### Consideraciones y hallazgos

- **El contraste entre las dos caras de las cordilleras es menor de lo esperado:** en el planeta, el barlovento recibe 2,2 veces la lluvia del sotavento; la nota esperaba de 5 a 20. Hay dos motivos:
  - el aire también se seca *mientras* sube la propia ladera, así que lo alto del barlovento ya recibe poco;
  - detrás de las montañas el aire se recarga bastante deprisa: a 200–500 km recibe el 42–54 % de la lluvia de antes, y la nota esperaba el 20–40 %.

  Con un "juguete" (una cordillera de 3 000 m dibujada a mano) sí sale un contraste de 7. Ajustar los dos parámetros del secado lo acercaría al objetivo. Lo decide el responsable, con los datos de la nota (§6.3).
- **La lluvia no depende del zoom donde importa.** Al pasar de 52 a 26 km, que es lo que hará el Foco de los agentes, la lluvia de la zona cambia un 0,00 %. A escalas mucho más gruesas (cientos de km) la lluvia sí cambia algo más que antes con la resolución (hasta un 6,7 % en la prueba con celdas de 834 km), porque esas celdas usan su propia pendiente y ahora `k_oro` es grande.
- **Decisiones pendientes del responsable** (nota §6.7): el valor de `k_oro` (150 calibrado, frente al 40 propuesto), los dos parámetros del secado, cómo contar la subida de la propia ladera, el mar plano y cómo se calcula `c_P`.

### Decisiones del responsable: realismo primero (05-11-2026)

El responsable fijó un principio nuevo para todo el proyecto (CLAUDE §2.8): **si algo no sale realista, primero se duda de la base, no se fuerza un número**.

- **`k_oro` vuelve a 40,** su valor físico. Subirlo a 150 cumplía el objetivo de lluvia en las montañas, pero compensaba otro problema: **el relieve no es realista**. Dos tercios de los continentes son llanuras casi a nivel del mar (en la Tierra, una cuarta parte), y las montañas son manchas sueltas en vez de cordilleras. La propuesta para arreglarlo, con tectónica de placas, está en `propuesta_relieve_realista.md`, a falta de que el responsable la apruebe. Mientras, las montañas apenas aumentan la lluvia: es lo honesto con este relieve.
- **El aire se seca de forma más realista.** Antes cada loma del camino le quitaba humedad. Ahora solo la pierde cuando sube por encima de lo que ya había subido: al bajar se calienta, pero conserva su humedad. Sobre las mesetas altas lleva menos vapor, como en la realidad.
- **El aire tarda más en recuperar la humedad sobre tierra** (3 000 km en vez de 1 000): con eso, detrás de una cordillera de juguete llueve un 35–38 % de lo de delante, en el rango de lo real.
- **El factor `c_P`,** que devuelve la lluvia que quita el secado, queda como **apaño provisional**. Lo realista será calibrar la lluvia con datos de la Tierra cuando el relieve sea realista.
- **Resultado en el planeta:** detrás de las cordilleras grandes la lluvia baja de 688 a 251 mm y el desierto se duplica. La comida del planeta, medida con el modelo Miami, baja un 3,3 %.

## Incremento 4: el relieve sale de la tectónica de placas (05-11-2026)

**Qué es.** Hasta ahora las montañas y los océanos salían de un «ruido»: un dibujo al azar, suave, al que se daba forma con una curva. Ahora salen de su causa real, la **tectónica de placas**.

La superficie del planeta está partida en **placas**: trozos rígidos de corteza que flotan sobre el manto y se mueven unos centímetros al año.
- Donde dos placas se separan, sale lava y nace **fondo oceánico** nuevo: la **dorsal**.
- Donde chocan, una se hunde bajo la otra (**subducción**), con una **fosa** profunda y una cadena de volcanes (un **arco**, como los Andes).
- Si chocan dos continentes, ninguno se hunde del todo: la corteza se apila y sale una **cordillera**, y detrás una **meseta** (el Himalaya y el Tíbet).

El planeta simula **500 millones de años** de esta deriva antes de empezar. En ese tiempo los continentes se rompen y se vuelven a juntar, y las cordilleras viejas se gastan.

Al final, la altura de cada sitio sale de la **isostasia**, la flotación: una corteza más gruesa sobresale más, como un témpano más grande. El **nivel del mar** sale de cuánta agua hay, la del océano actual de la Tierra. Encima va un «detalle fino» con la rugosidad del relieve real, medida en los datos de la Tierra. Después llega la erosión de un millón de años que ya existía, ahora con dos cosas nuevas: las montañas activas siguen subiendo, y la roca que la erosión quita alivia la corteza, que rebota hacia arriba.

**Con qué se compara.** Con la Tierra de verdad. Un programa nuevo (`herramientas/analisis/referencia_tierra.py`) descarga el relieve medido de toda la Tierra (ETOPO 2022), la roca de la Antártida y de Groenlandia sin su hielo (Paxman et al., 2022), las edades del fondo oceánico (Seton et al., 2020) y las placas reales (Bird, 2003). Calcula las mismas cifras que el planeta y a la misma resolución, y las guarda en `config/referencias/tierra_relieve.toml`.

### Qué sale bien

- **Las placas se comportan como las de la Tierra.**
  - Se mueven a 4,4 cm/año de media, la velocidad medida.
  - Las que tienen mucha fosa van deprisa y las muy continentales van despacio, como observaron Forsyth y Uyeda en 1975.
  - Hay rifts, choques, suturas y un continente que se junta y se separa.
- **Un choque como el de la India y Asia frena como el real:** de 14 a 5 cm/año.
- **El océano tiene la profundidad de la Tierra** (3 945 m de media, la Tierra 3 667) y su edad mediana es de 60 Myr (la Tierra 55).
- **Los ríos** siguen la ley de Hack de los ríos reales.

### Qué no cuadra todavía (hallazgos)

- **La tierra está demasiado alta:** 1 459 m de media, frente a 671 en la Tierra. Pasa porque el agua llena las cuencas oceánicas y aún sobra cuenca: el mar queda ~1 km más bajo de lo que esperaba el diseño. Hay dos causas, y ninguna es un número mal puesto:
  - al fondo oceánico del modelo le faltan cosas que levantan el fondo real (sedimento, mesetas y montes submarinos);
  - sobrevive fondo viejísimo que en la Tierra ya se habría hundido, porque falta el mecanismo que inicia nuevas fosas en los bordes viejos de los océanos.

  Forzar un número para bajar la tierra iría contra el principio de realismo, así que se ha dejado así y anotado.
- **No hay plataformas continentales** (la franja de mar poco hondo junto a la costa): los continentes nacen con el borde cortado.
- **Una sorpresa sobre la lluvia de montaña.** El relieve real de la Tierra, a la escala a la que la ve nuestra lluvia (25 km), solo da ×1,4 de lluvia en las laderas de barlovento, no ×2–3. Ese ×1,4 era nuestra regla de lluvia puesta sobre el relieve real; la lluvia medida de verdad se cuenta más abajo.
- **La corteza continental baja un 17 %** en los 500 Myr. Lo trata la adenda de balance, más abajo.

Todo está medido en la batería de escenarios (escenario `planeta`, 34 métricas), con su nota frente a la Tierra, y explicado en `incremento-4-diseno.md`. Mientras se mide, el relieve anterior sigue disponible: `worldgen.relieve = "ruido"`.

### Balance de la corteza continental (adenda del responsable, 05-11-2026)

**El fenómeno.** La corteza continental se fabrica y se destruye a la vez.
- **Se fabrica en los arcos volcánicos:** donde una placa se hunde bajo otra, el agua que lleva funde el manto y sube magma que engorda la placa de arriba. Además, los arcos que nacen en medio del mar acaban pegándose a los continentes.
- **Vuelve al manto por cuatro vías.**
  - **El sedimento** que se hunde en las fosas.
  - **La erosión por subducción:** la placa que baja raspa el borde de la de arriba.
  - **La delaminación:** la base de una corteza demasiado gruesa se transforma, a mucha presión, en una roca más densa que el manto (eclogita) y se desprende.
  - **Los choques entre continentes:** parte de la corteza que se mete debajo se pierde.

**Lo que se hizo.** Cada uno de esos procesos tiene ahora su tasa medida en la Tierra.
- El arco y la erosión por subducción son proporcionales a **cuánta placa se hunde**: el área de fondo oceánico que desaparece en cada fosa, que el modelo mide exacta.
- Un margen es **erosivo** (raspa) si converge deprisa y le llega poco sedimento, y **acrecionario** si no. En uno acrecionario, parte del sedimento se queda pegado y forma un **prisma** (una cuña de sedimento apretado).
- Lo que la erosión quita en tierra baja por las laderas hasta el mar y se deposita; si cae en una cuenca sin salida, se queda allí hasta llenarla. Antes se perdía.
- Cada paso de un millón de años comprueba el balance: lo que cambia la corteza es exactamente lo que fabrican los arcos menos lo que se va al manto, hasta el error de redondeo del ordenador.

**Lo que sale bien.**
- El sedimento que se hunde (1,0 AU) y la delaminación (1,2 AU) caen en los valores de la Tierra. **AU** es la unidad de estos flujos: 1 km³ de roca al año.
- La corteza más gruesa llega a 79 km, como el Tíbet.
- Una cordillera que deja de crecer vuelve a su espesor normal en ~150 millones de años, como las cordilleras viejas reales. Así se ha fijado el ritmo de la erosión.
- El planeta mejora en varias cosas:
  - la tierra baja a 1 277 m de media (antes 1 544; la Tierra 671);
  - el fondo oceánico rejuvenece;
  - llueve más sobre la tierra;
  - detrás de las cordilleras llueve menos que en el resto, como en la Tierra (antes era al revés).

**Lo que no cuadra, y por qué.**
- **En el modelo se hunde el doble de placa que en la Tierra:** 6,3 km² al año, frente a 2,7–3,4. Por eso salen altos los arcos, la erosión por subducción y el sedimento que llega al mar.
- **Los choques destruyen de 7 a 12 veces más corteza que en la Tierra.**
- La causa está en la base, no en los números: **los bordes de placa del modelo son demasiado largos y dentados**, y sus placas se deslizan poco de lado.
  - Comparado con la Tierra medida igual, con el mismo número de placas, el modelo tiene 1,7 veces más borde y 3 veces más fosa.
  - Un tercio del consumo ocurre en bordes que en la Tierra no consumen nada: los dientes de dos placas que se deslizan una junto a otra chocan entre sí.
  - El responsable diseña la corrección de los bordes en su próxima nota. Hasta entonces no se ajusta ningún parámetro para compensarlo.

**La lluvia, medida de verdad.** Con datos de lluvia observada (WorldClim y CHELSA) a la misma escala y con la misma definición de "ladera de barlovento", la Tierra **no** llueve más en esas laderas: ×0,7–1,2. La definición coge mesetas altas y secas, como el Tíbet; hay que revisarla. Lo que sí falla en la lluvia del modelo es:
- la sombra de lluvia detrás de las cordilleras, aún floja (×0,79, la Tierra ×0,58);
- el reparto por latitudes: subtrópicos demasiado secos y latitudes medias demasiado húmedas.

Detalle técnico en `incremento-4-diseno.md` §9–§11; la adenda, en `adenda_balance_corteza.md`.

### Bordes de placa limpios y flujo de la corteza (nota de bordes y adenda v3, 06-11-2026)

**El problema.** Los bordes entre placas están dibujados con celdas, así que tienen dientes. Al moverse las placas, los dientes de una se metían en la otra también donde solo se deslizan (una **falla transformante**, como la de San Andrés) o donde se separan (una **dorsal**). El modelo lo trataba como si una placa se hundiera bajo la otra. Un cuarto del fondo oceánico que "se hundía" lo hacía en sitios donde en la Tierra no se hunde nada, así que el modelo hundía el doble de placa que la Tierra.

**Lo que se hizo.**
- **Borde suavizado.** El tipo de cada tramo se decide mirando el borde a 150 km, como lo ve una litosfera rígida, no la escalera de celdas. Es **transformante** si las placas se deslizan a menos de 20° del borde, el umbral que separa las clases en los datos reales de Bird.
- **Dientes recolocados.** Solo se hunde placa donde las placas se acercan de verdad. El diente de un borde que se desliza no se destruye: se recoloca en el hueco que su placa deja detrás.
- **Islas absorbidas.** Los trocitos sueltos de una placa dentro de otra pasan a la placa que los rodea, como un terreno que se pega a un continente.
- **Flujo lateral.** La corteza muy gruesa ya no tiene tope. Su parte de abajo, caliente y blanda, se desparrama hacia los lados, como una masa de pan, y forma mesetas en vez de agujas.

**Qué se consigue.**
- El fondo que se hunde en bordes que no convergen baja del 26 al 3 %.
- Se hunden 3,6 km² de fondo al año, frente a 2,7–3,4 en la Tierra; antes eran 6,3. Con ello, los arcos y la erosión por subducción caen en los valores de la Tierra.
- La longitud total de los bordes y la de las dorsales se parecen a las de la Tierra medida igual.
- Un choque como el de la India y Asia hace una meseta de unos 75 km de espesor, como el Tíbet, sin tope.
- La tierra baja a 989 m de media (la Tierra, 671).

**Lo que se descubrió al quitar el tope.** Sin tope, la corteza se apilaba hasta 1 525 km en algunos sitios. Había seis causas, todas por cómo está dibujado el modelo, no por la física. La principal: todo lo que un choque mete debajo iba a una sola celda. Ahora el máximo es de 113 km, y solo 3 de cada 500 pasos pasan de 80.

**Lo que no cuadra todavía.**
- **Las fosas miden el doble que en la Tierra.** Probando el suavizado entre 100 y 300 km siguen por encima de 1,5 veces. No es la forma de los bordes: son las fuerzas que mueven las placas.
- **El fondo oceánico es demasiado viejo en parte.** La edad típica (mediana) ya es la de la Tierra, pero una cuarta parte pasa de 200 millones de años, porque falta el mecanismo que abre fosas nuevas en los bordes viejos de los océanos.
- **La erosión manda al mar 3,1 km³ al año, frente a ~1,8 en la Tierra.** No es cosa del ritmo de la erosión: con uno casi el doble de lento, al mar llega lo mismo, porque a la larga la erosión se lleva lo que los arcos y los choques añaden. Lo que falta es que la base de la corteza engrosada se desprenda hacia el manto (la delaminación): en la Tierra se lleva ~1,1 km³ al año, en el modelo casi nada.

**La lluvia de las laderas, medida bien.** Con la regla del responsable (laderas del pie a la cresta, sin mesetas, comparadas con la llanura de su pie), la Tierra observada da ×0,9–1,2 y el planeta ×1,05. El modelo ya está en el rango real.

### Antes de que las fosas nazcan y mueran: el diagnóstico, y la raíz de los arcos (07-11-2026)

**Qué es una fosa.** Es el borde donde una placa oceánica se hunde bajo otra. La **convergencia** es lo rápido que se acercan las dos placas de frente, en centímetros al año. La nota del responsable sobre el ciclo de vida de las fosas propone que una fosa no aparezca en cuanto dos placas se empujan. Primero tendrían que juntarse unos 150 km; hasta entonces, el borde solo se engruesa. Antes de implementarla, el responsable pidió comprobar si el exceso de fosas del modelo está en los bordes que convergen despacio, que son los que esa regla quitaría.

**Lo que salió: el exceso está en las fosas rápidas.**
- El modelo tiene casi la misma longitud de fosa lenta (por debajo de 2 cm/año) que la Tierra, unos 14 000 km.
- Lo que le sobra son fosas que convergen deprisa.
- La convergencia media de sus fosas (5,1 cm/año) es la de las fosas reales (5,3 según Bird).
- La regla de la nota quitaría como mucho una quinta parte de las fosas. Por eso se lleva el resultado al responsable antes de implementarla, como pedía la nota.

**Un dato de la nota que no cuadra.**
- La nota esperaba que las fosas reales convergieran a 7–9 cm/año. Salía de repartir todo lo que se hunde en la Tierra entre las fosas de una Tierra simplificada a 15 placas.
- Al simplificar, desaparecen sobre todo las fosas de las placas pequeñas de los arcos de islas, como Tonga o las Marianas. Las fosas de esa Tierra simplificada solo hunden la mitad.
- El modelo no tiene placas pequeñas, así que sus fosas deben hacer el trabajo de todas las de la Tierra.
- Comparado con todas, el modelo tiene 1,45 veces más fosa. Casi todo se explica porque crea y hunde un 20–30 % más de fondo oceánico que la Tierra. Esa es la pregunta para las fuerzas que mueven las placas.

**La raíz de los arcos.** Bajo un arco volcánico se acumulan rocas muy densas. Cuando la corteza pasa de unos 40 km de espesor, esa base pesa más que el manto de debajo y se hunde en él (la **delaminación**). Está medido en el arco fósil de Kohistán (Jagoutz y Behn, 2013).
- Se implementó así, sin calibrar nada.
- La delaminación se multiplica por 4, pero el sedimento que llega al mar no baja. Lo que se desprende sale de los continentes, que pierden un 9,5 % de su volumen en 450 millones de años.
- La idea, que fue mía, era que el sedimento que sobraba en el mar era la delaminación que faltaba. Era errónea.

**Una corrección importante.** Dije que el ritmo de la erosión no cambiaba el sedimento que llega al mar. Lo había medido con una sola historia del planeta para cada ritmo, y el planeta es **caótico**: un cambio pequeño cambia qué placas chocan y cuándo, y el resultado varía mucho de una historia a otra.
- Con cinco historias distintas (semillas), una erosión más lenta sí manda menos sedimento al mar.
- Ningún ritmo único cumple a la vez que las montañas viejas se gasten en 100–200 millones de años y que llegue al mar lo de la Tierra.
- Desde ahora, las calibraciones se miden con varias semillas.

**Las uniones de tres placas no son el problema.**
- El poco consumo de placa que queda en bordes que no convergen está sobre todo en los extremos de las fosas, donde el borde gira y pasa a deslizarse.
- Las columnas de más de 80 km suelen salir de los trozos de borde que se recolocan y de los choques.

**El tiempo.** La regla de la nota añadiría del 3 al 5 % al cálculo, unos 5 segundos sobre 164 (el límite es 200).

### Fosas con vida propia, una base más limpia y erosión con la altura (07-11-2026, segunda parte)

**Primero, la base.** Se arreglaron cuatro cosas de cómo está dibujado el planeta:
- **Los extremos de las fosas.** Donde una fosa gira y el borde pasa a deslizarse, lo que se hunde se contaba en el tramo equivocado. Ahora cuenta para la fosa.
- **Las muestras ocultas.** Son trozos de placa que el mundo no ve en un paso. Ya se reparten hacia los lados como el resto de la corteza gruesa, y no se apilan.
- **Los dientes del borde.** Son los trozos de placa que, por el dibujo en celdas, se meten en la placa vecina donde las dos solo se deslizan. Antes, si no encontraban sitio, se repartían como engrosamiento: unos 1–3 km³ de corteza continental al año, tanto como los arcos volcánicos, en bordes que en la realidad no engordan. Ahora esperan en su placa al paso siguiente. Los picos de más de 80 km casi desaparecieron y la exportación al mar bajó un 20 %.
- **Un fallo de la erosión.** Una columna alta junto a una depresión no se erosionaba nunca, porque se miraba la cuenca en la celda equivocada.

**Fosas que nacen, viven y mueren** (la nota del responsable).
- Ahora un borde donde dos placas se juntan no es una fosa desde el primer momento. Es **convergencia difusa**: el fondo se arruga y se engruesa a los dos lados.
- Solo cuando se han juntado 150 km nace la fosa, con una **polaridad** que ya no cambia (cuál de las dos placas se hunde).
- Su **losa** (el trozo de placa ya hundido) tira cada vez más a medida que se alarga.
- La fosa muere si deja de converger 10 millones de años o si le llega un continente.
- También pueden nacer fosas solas en los bordes de los continentes donde el fondo es viejo (**nacimiento espontáneo**). Su ritmo se ajustó para que la edad media del fondo oceánico sea la de la Tierra, 64 millones de años.

**Lo que salió.**
- La convergencia difusa se parece a la de la Tierra: unos 10 000 km de borde, que se acercan unos 2 cm al año.
- Sin nacimientos espontáneos, la longitud de las fosas, el fondo que se hunde y el tamaño de las placas son los de la Tierra. Pero el fondo envejece demasiado: una cuarta parte pasa de 180 millones de años.
- Con los nacimientos necesarios para rejuvenecerlo, cada uno convierte en fosa todo el borde de un continente. Las fosas miden 1,55 veces las de la Tierra, se hunde un 50 % más de fondo, quedan placas gigantes y sigue habiendo fondo muy viejo (8 %).
- Como pidió el responsable, se midieron las fuerzas que mueven las placas antes de tocar nada:
  - el tirón de las losas da el 75 % del empuje (en la Tierra, del orden del 90 %);
  - los bordes del modelo están orientados al azar respecto al movimiento, mientras que en la Tierra las dorsales y las fosas tienden a abrirse y cerrarse de frente;
  - la placa mayor ocupa el 40 % del planeta (el Pacífico, el 22 %).
- La causa principal del exceso parece estar en cómo nacen las fosas espontáneas, más que en las fuerzas.

**La erosión ahora depende de la altura.** Como en la Tierra, las cordilleras se gastan mucho más deprisa que las llanuras: a 300 m se pierde ~1 m por millón de años; a 4 km, 0,15 km.
- Con los valores provisionales, una cordillera vieja se gasta en 144 millones de años.
- La tierra baja: la altura media pasa de 1 200 a 920 m (la Tierra, 670), y ya no sobra tierra entre 1 y 2 km.
- El ritmo exacto (cuánto sedimento llega al mar) no se ha ajustado todavía: con la base que hunde un 50 % de más, ajustarlo sería tapar un error con otro.

**Lo que queda por decidir.**
- Cómo se consume el fondo viejo sin rodear de fosa los continentes.
- Si el basalto de la convergencia difusa debe ir también bajo los continentes, donde los levanta.
- Por qué vuelven picos de corteza de más de 100 km en los choques.
