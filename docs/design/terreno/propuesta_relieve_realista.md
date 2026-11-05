# Propuesta: un relieve realista

**Estado:** propuesta de Claude Code (2026-11-05). **Decidida por el responsable:** opción C con deriva de placas (`diseno_relieve_tectonico.md`), implementada en el incremento 4 (`incremento-4-diseno.md`).
**Origen:** decisión S1 de la nota de sombra pluviométrica.
- En vez de forzar `k_oro` hasta 150 para que lloviera lo esperado en barlovento, hay que dudar del relieve (CLAUDE §2.8, realismo primero).
- Con el valor físico de `k_oro` (40), las laderas de barlovento reciben solo ×1,37 de lluvia, frente a ×2–3 en la Tierra, porque nuestras montañas son demasiado suaves.

---

## 1. Diagnóstico: el planeta frente a la Tierra

Medido sobre el planeta por defecto, tras la génesis geológica. Las cifras de la Tierra son aproximadas (de la curva hipsométrica terrestre). Para calibrar habría que usar una tabla de referencia de un modelo digital del relieve terrestre (p. ej., ETOPO).

| | Planeta | Tierra (aprox.) |
|---|---|---|
| Tierra emergida | 38,8 % | ~29 % |
| Altura media de la tierra | 407 m | ~840 m |
| Altura mediana de la tierra | 53 m | varios cientos de metros |
| Tierra por debajo de 200 m | 68 % | ~25–30 % |
| Tierra de 1 000 a 2 000 m | 6,3 % | ~15–20 % |
| Tierra por encima de 2 000 m | 5,4 % | ~10 % |
| Profundidad media del océano | 3 153 m | ~3 700 m |
| Pendiente media de barlovento de las cordilleras de más de 2 000 m, a escala de 25 km | ~0,009 (9 m/km) | ~0,02–0,05 (Rocosas, Alpes, Andes, Himalaya) |

**Por qué.**
1. **La curva de alturas** (`h = lf³ · h_max`, con `land_exp` = 3) aplasta casi toda la tierra contra el nivel del mar: dos terceras partes de los continentes son llanuras costeras. Faltan las tierras medias y las mesetas.
2. **El ruido es igual en todas direcciones.** Las montañas salen como manchas sueltas, no como cordilleras alargadas. En la Tierra, las cordilleras nacen donde chocan las placas (Andes, Himalaya), son estrechas y largas, y tienen mesetas detrás (Tíbet, Altiplano).
3. La génesis geológica (1 millón de años de erosión) suaviza algo más lo poco que hay.

Todo lo que se construya encima hereda esa irrealidad: la lluvia de montaña, los ríos, los biomas, el forraje y, más adelante, dónde vivirán los agentes.

## 2. Opciones

| Opción | Qué es | Realismo | Coste |
|---|---|---|---|
| **A. Recalibrar el generador actual** | Cambiar la curva de alturas (`land_exp`) y el peso de cada octava del ruido para reproducir la curva hipsométrica de la Tierra | Alturas medias correctas, pero las montañas siguen siendo manchas, sin cordilleras ni mesetas | Bajo |
| **B. Ruido "de crestas" para las montañas** | Añadir un ruido que forma crestas alargadas (técnica habitual en terrenos procedurales), más la calibración de A | Cordilleras alargadas, pero colocadas al azar, sin lógica geológica | Medio |
| **C. Tectónica de placas simplificada** | Generar placas (regiones en la esfera) con su movimiento. Donde chocan, cordilleras (o mesetas si chocan dos continentes); donde se separan, rifts y dorsales oceánicas; costas con plataforma continental. Encima, el ruido actual para el detalle fino y la erosión que ya existe | Las montañas salen donde saldrían en la Tierra: cordilleras costeras tipo Andes, mesetas tipo Tíbet, plataformas continentales. La curva de alturas sale de la física de la corteza, no de un exponente | Alto: un incremento propio del terreno |

La visión madre ya marca la tectónica de placas como ampliación futura (§2, "Ampliación futura").

## 3. Recomendación

**La C, como incremento 4 del terreno ("relieve tectónico"), antes de que los agentes pasen al planeta (incremento 3 de agentes).** Es la única que no fabrica cordilleras a mano, y es la base de todo lo demás (CLAUDE §2.8).

**Objetivos de calibración** (contra datos reales de la Tierra):
- tierra emergida ~29 %;
- curva hipsométrica de la tierra por franjas de altura;
- altura media de la tierra ~800 m;
- océano de ~3 700 m de profundidad media, con plataformas continentales;
- cordilleras de 100–400 km de ancho, crestas de 3–7 km y laderas de 0,02–0,05 a 25 km;
- alguna meseta alta detrás de las cordilleras de choque.

**Después, en el mismo incremento:**
- comprobar que `k_oro` = 40 da 2–3 veces más lluvia en barlovento sin tocarlo;
- calibrar la lluvia de base con datos reales (~1 000 mm/año en todo el planeta y ~800 sobre tierra);
- retirar `c_P`;
- comparar la proporción de biomas con la de la Tierra.

## 4. Preguntas para el responsable

1. **¿C, o primero A como paso rápido?** A mejora las alturas en poco tiempo, pero habría que rehacerla al llegar C.
2. **¿Planeta "tipo Tierra"** (mismas proporciones de tierra y océano) o con proporciones propias por semilla? Lo segundo es más general, pero calibrar contra la Tierra exige tener una referencia.
3. **Tiempo geológico:** ¿las placas se generan en una foto fija (posición y movimiento actuales) o se simula su deriva durante millones de años antes de la génesis geológica? La foto fija es mucho más barata y da las cordilleras; la deriva da además la historia (cordilleras viejas y erosionadas).
4. **Datos de referencia:** ¿usamos estadísticas de un modelo digital de la Tierra (ETOPO u otro) como objetivo? Habría que guardarlas en el repositorio (`docs/` o `config/`).
