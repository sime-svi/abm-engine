# Fase «agentes» — la biología de los seres vivos, por días

> Explicación en lenguaje llano (CLAUDE §8.1). El diseño completo del responsable está en `diseno_biologia_agentes.md`; el diseño técnico del primer paso, en `incremento-1-diseno.md`.

## ¿Qué es esta fase?

Los "bichitos" del modelo inicial vivían en un tablero, comían, se reproducían y morían con reglas inventadas: vivían 200 ticks, gastaban 2 unidades de energía por tick, comían 4... Cuando el planeta pasó a contar el tiempo en **días** (1 tick = 1 día), esas cifras dejaron de tener sentido: un animal que vive 200 días y se reproduce cuando "tiene 20 de energía" no se parece a nada real.

El responsable rediseñó la biología entera con **cifras reales**:
- la energía se mide en **kilocalorías** (kcal), las mismas de las etiquetas de los alimentos;
- cada especie se define por su **peso adulto**. De él salen, con las **leyes de escala** de la biología (reglas medidas en cientos de especies, como "un animal 10 veces más pesado vive unas 1,6 veces más"), cuánto gasta al día, cuánto vive, cuánto tarda en crecer o cuánto dura el embarazo;
- los **humanos** son un caso especial con sus propios valores (60 kg, 110 años como máximo, embarazo de 270 días, lactancia de 2,5 años...);
- hay **dos sexos**: la hembra paga el embarazo y la leche, y las crías dependen de su madre hasta el destete.

## Incremento 1 — el individuo, en un hábitat de prueba

Como el diseño es grande, se construye por pasos (la "pirámide invertida" del proyecto). El primero pone a vivir a **individuos** (cada animal es una entidad con su edad, sexo, peso, reservas de grasa y estado reproductivo) en un **hábitat de prueba**: unas cuantas parcelas con un clima fijo, sin planeta todavía. Así la biología se comprueba sola antes de mezclarla con el terreno.

Lo que hace cada animal cada día:
1. **Gasta** energía en mantenerse vivo y moverse: es su **metabolismo**, que crece con el peso según la **ley de Kleiber** (un animal de 70 kg gasta unas 4 200 kcal al día). Si está creciendo, gasta además en fabricar tejido. Si está preñada, en el embarazo. Si amamanta, en la leche.
2. **Come** el **forraje** (la vegetación que le sirve de comida) de su parcela. El forraje vuelve a crecer cada día según lo productiva que sea la zona, calculada a partir de su temperatura y su lluvia (el **modelo Miami**, una fórmula clásica de ecología). Si no hay comida para todos, todos reciben la misma proporción de lo que piden: es el **reparto justo**.
3. **Guarda o tira de reservas.** Lo que sobra se guarda como grasa (con pérdidas). Lo que falta sale de la grasa. Si la grasa se acaba, muere de hambre.
4. **Crece** hasta el peso adulto, siguiendo una curva medida en animales reales (la de West).
5. **Se reproduce:** una hembra madura y en buena forma, con un macho en su parcela, puede quedarse preñada. Si pasa demasiada hambre, aborta. Tras el parto amamanta hasta el destete.
6. **Puede morir:** de hambre, de vieja (al llegar a la edad máxima) o por las causas de cada edad. Estas siguen el **modelo de Siler**: mucha mortalidad en las crías, una mortalidad de fondo en la edad adulta y otra que crece con la vejez. Si muere una madre que amamanta, sus crías pueden morir con ella (**orfandad**).

### Para qué sirve

Para comprobar que la biología **por sí sola** da cifras realistas antes de soltarla en el planeta:
- Un humano de 60 kg quema 1 509 kcal al día en reposo y ~2 700 con su actividad. Aguanta unos 40 días sin comer.
- Con la mortalidad calibrada, la **esperanza de vida al nacer** de los humanos es de 31 años. El 56 % llega a los 15, y la edad más común de muerte adulta es 69 años. Son las cifras de los cazadores-recolectores actuales.
- El herbívoro de 70 kg: el 60 % sobrevive a su primer año y el 10 % de los adultos muere cada año.
- La energía **cuadra al céntimo**: lo que se come menos lo que se gasta, se pierde y se llevan los muertos es exactamente lo que cambian las reservas de los vivos (diferencia de millonésimas de kcal por mes).
- Con la misma semilla, la simulación da **exactamente** lo mismo, aunque cambie el orden en que el programa recorre a los animales.

### Qué queda fuera (para los siguientes pasos)

- **Paso 2, las poblaciones.** Lejos de donde se mira, los animales no se simularán uno a uno, sino como recuentos por edad y sexo, mucho más baratos.
- **Paso 3, el planeta.** El forraje saldrá del clima y del suelo reales del terreno, y el **Foco** (las zonas que se observan de cerca) decidirá dónde hay individuos.
- **Paso 4, los registros.** Nacimientos y muertes uno a uno, y su paso a la base de datos.
- **Otros documentos.** El movimiento y la búsqueda de comida o pareja (por eso, de momento, cada parcela es un mundo cerrado), la vida social y la agricultura.

## Consideraciones y hallazgos (05-11-2026)

- **Una corrección al diseño (D1, a confirmar).** El "apetito" se lee como energía que el cuerpo **aprovecha**, no como hierba en bruto. Leído al pie de la letra, el herbívoro, que aprovecha solo la mitad de lo que come, no podría pagar ni el crecimiento ni las crías y se extinguiría. Así se cumple la regla del propio diseño: "si el apetito supera al gasto, se pueden rehacer reservas".
- **El hallazgo grande: con el reparto tal como está, las poblaciones no se regulan.** Todo el forraje de la parcela está disponible, así que los animales comen hasta hartarse mientras quede algo. Nadie nota la escasez hasta que la comida se acaba de golpe. La población crece bien alimentada, supera lo que la parcela puede mantener, se come las reservas de forraje y se hunde. En el hábitat de prueba, sin movimiento entre parcelas, **el herbívoro se extingue en 13 años** con un clima que no cambia nunca.
- **Una propuesta medida (D16, la decide el responsable).** Si cada animal consigue comer **en proporción a lo que queda** de forraje (con la mitad, la mitad), la escasez se nota antes y la población **se regula sola**:
  - el herbívoro se queda estable durante 50 años en ~1,2 veces la capacidad calculada (las crías comen menos que un adulto);
  - la **densidad sigue a la productividad** en los cuatro biomas probados: desierto 1,3, tundra 1,6, templado 7,8 y selva 17 animales por km²;
  - dos especies de 10 y 300 kg en el mismo sitio **consumen lo mismo por km²**, y su densidad sigue la ley de Damuth, como en la naturaleza.

  Está en la configuración como `respuesta = "proporcional"` (experimento en `config/experiments/agentes_proporcional.toml`). Mientras el responsable no decida, el valor por defecto es el del diseño (`"total"`).
- **Las hambrunas matan sobre todo a las hembras.** Con una sequía que reduce el rebrote a la mitad durante un año, mueren todas las crías primero, como dice el diseño. Pero también casi todas las hembras: de 5 452 quedan 2, frente a 3 159 machos. Pagan embarazo y leche, y el diseño hace abortar a la preñada que pasa hambre, pero no prevé que la que amamanta abandone a la cría: sigue dando leche hasta morir. Sin hembras, la población no se recupera. Los humanos, con más reservas, sí se recuperan. **Propuesta:** un "destete forzoso" equivalente al aborto, a decidir.
- **Los humanos tienen demasiados hijos.** La tasa de fecundidad sale de ~10 hijos por mujer, frente a 5–6 en cazadores-recolectores reales. El modelo no hace que la fertilidad baje con la edad antes de la menopausia (a los 50). Queda anotado para la calibración.
- **Rendimiento.** 30 años con ~10 000 animales tardan ~4 s.

## Revisión 1 (05-11-2026) — las decisiones del responsable sobre los hallazgos

El responsable decidió qué hacer con los hallazgos del incremento 1 en `nota_agentes_revision_1.md`, y la sección 8 de esa nota cuenta lo hecho y lo medido. Son cuatro cambios, cada uno con su interruptor en `config/agentes.toml`. Con los cuatro apagados (`config/experiments/agentes_incremento1.toml`), la simulación da **exactamente** lo mismo que antes.

1. **El apetito tiene en cuenta todo lo que se gasta ese día**, también criar. Es lo que pasa en la naturaleza: las madres que amamantan comen mucho más (**hiperfagia**: comer más de lo normal).
2. **Cuando escasea la comida, cada animal come menos antes de que se acabe.** Encontrar comida cuesta más cuando hay poca, así que nadie llena la tripa del todo. Es la **respuesta funcional de Holling**, la forma clásica de describirlo en ecología. Sustituye a la propuesta "proporcional" del incremento 1.
3. **Las madres se protegen y los sexos son distintos:**
   - una madre en mala condición da menos leche;
   - si su condición cae mucho, desteta a la fuerza: las crías mayorcitas sobreviven comiendo solas y las más pequeñas pueden morir;
   - en los humanos, las mujeres pesan 56 kg con un 25 % de grasa y los hombres 64 kg con un 15 %. Ellas aguantan sin comer unos 49 días y ellos unos 30.
4. **Cada mujer deja de ser fértil a una edad suya**, en torno a los 38 años (unos más tarde, otras antes), y no todas a los 50 como antes.

### Qué se ha conseguido

- **Las poblaciones se regulan solas y ya no se extinguen.** El herbívoro, que antes desaparecía en 13 años, aguanta 100 años estable, con la comida a la mitad de su máximo (justo donde más rebrota). Los humanos se quedan en un 90 % de lo que la tierra puede mantener. Lo consigue el cambio 2. El cambio 1 hace que, en esa situación, criar salga pagado: el herbívoro deja de morir de hambre.
- **Siguen saliendo las leyes de la ecología:** hay más animales donde la tierra produce más, y una especie de 300 kg y otra de 10 kg en el mismo sitio comen exactamente lo mismo por km² (ley de Damuth).
- **El modelo también sabe colapsar cuando toca.** Con un alimento que tarda 20 años en rebrotar (como el liquen de los renos de la isla de St. Matthew), la población se dispara al doble y medio de lo sostenible y después se hunde.
- **Las hambrunas ya no acaban con las hembras.**
  - Con una sequía, en el herbívoro quedan un 30 % de hembras adultas; antes, casi ninguna.
  - En los humanos, ahora mueren **menos mujeres que hombres**, como en las hambrunas reales, gracias a sus reservas de grasa.
  - Las crías mueren antes que los adultos, y la población se recupera.
- **Las mujeres tienen unos 6,6 hijos** (antes ~10), en el rango de los cazadores-recolectores reales (5–7), y el último hijo llega hacia los 37 años.

### Consideraciones y hallazgos

- **El hambre llega a todos a la vez.** Con la nueva regla, lo que come cada animal depende de cuánta comida queda, no de cuántos comen. En una sequía todos los de una parcela pasan hambre a la vez y mueren casi a la vez, antes de que la comida se recupere. Como cada parcela está aislada, muchas se quedan vacías. Cuando los animales puedan moverse (otro documento), se volverán a llenar.
- **En el herbívoro, las hembras siguen muriendo más que los machos en las hambrunas.** No es por la leche, sino por el embarazo: pasan preñadas 200 de cada 336 días. Que la madre deje antes de amamantar no lo arregla (se probó).
- **Los humanos viven menos de media:** de 31 años de esperanza de vida a 21–25. Cuando la población está en el límite de lo que la tierra da, el 29 % de las muertes son por hambre.
- **La fecundidad humana no cuadra del todo.** Con la edad de esterilidad a los 38, sale el número de hijos correcto, pero el último llega un poco pronto. Con 40, la forma por edades es la real, pero salen demasiados hijos. Haría falta otro ajuste (por ejemplo, que la probabilidad de quedarse embarazada baje con la edad).
- **Coste:** 30 años de simulación tardan ~15 s, porque ahora el herbívoro sobrevive (antes se extinguía en el año 13 y la corrida tardaba 4 s).
- **Queda para el paso 2 (poblaciones):** la versión "por grupos" de cada cambio, ya escrita en la nota.
- **Decisiones del responsable:** siete (A1–A7, nota §8.9), con sus datos.

## Cómo se ejecuta

```bash
cargo run --release --bin agentes                                                 # config/agentes.toml (con la revisión 1)
cargo run --release --bin agentes -- config/experiments/agentes_incremento1.toml   # el incremento 1 tal cual
```

Escribe `output/<nombre>_agentes.json` con:
- los parámetros derivados de cada especie;
- un registro cada 30 días: población, nacimientos, muertes por causa, condición, forraje y balance de energía; con la revisión 1, también destetes forzosos y condición de las lactantes;
- un resumen por año: esperanza de vida, fecundidad, intervalo entre partos, pirámide de edades y densidad frente a la capacidad; con la revisión 1, también fecundidad por grupos de edad, edad del último parto y mortalidad por edad y sexo.

### Decisiones del responsable: realismo primero (05-11-2026)

Las siete decisiones pendientes se tomaron con un criterio: **lo que más se parezca a la realidad** (CLAUDE §2.8).

- **La capacidad de carga cuenta la población real.** Antes se calculaba como si todos los animales fueran adultos, y no lo son: las crías gastan menos. Ahora se usa el gasto medio de una población con sus crías, jóvenes y viejos. Al hacerlo, salió que el herbívoro vivía un 23 % por encima de la densidad real de los animales de su tamaño (la ley de Damuth), y se corrigió su acceso a la comida para que cuadre.
- **La fecundidad humana se parece a la real:** cada mujer deja de ser fértil hacia los 40 años, con una probabilidad de quedarse embarazada algo menor que antes (la de las cazadoras-recolectoras, que tienen menos reservas que una población bien alimentada). Resultado:
  - 6,8 hijos por mujer;
  - el último hacia los 38 años;
  - una caída de la fertilidad con la edad casi igual a la de los pueblos sin anticoncepción.
- **En las hambrunas humanas mueren menos mujeres que hombres** (0,07 frente a 0,54 al año), y la población se recupera despacio, en unos 50 años, como en la realidad.
- **Quedan apuntados, obligatorios, porque hoy no pueden hacerse bien:**
  - la **competencia con jerarquía**: en la realidad los fuertes comen primero, y en las hambrunas mueren antes crías y viejos, en vez de todos a la vez;
  - el **coste del celo** de los machos, que en los ciervos reales los hace morir más en los inviernos duros;
  - **otras causas de muerte**: enfermedad, accidentes, violencia;
  - la **fertilidad que baja con la edad** y los abortos espontáneos;
  - los **individuos persistentes**: la visión del proyecto es poder seguir a un humano desde que nace hasta que muere y subir hasta ver toda su población, así que alguien a quien se sigue no puede "disolverse" en una población cuando su zona deja de observarse de cerca. Pasa a ser un requisito del paso 2.
