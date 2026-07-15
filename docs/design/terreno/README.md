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

Por eso la fase se parte en incrementos. El primero está diseñado en [`incremento-1-diseno.md`](./incremento-1-diseno.md).

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

Lo que **queda para los siguientes incrementos**: la resolución que se afina sola (LOD adaptativo, incr. 2) y el relieve vivo que talla valles (erosión y pedogénesis, incr. 3).
