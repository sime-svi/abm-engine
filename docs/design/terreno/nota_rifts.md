# Nota de diseño: rifts (ruptura de continentes)

Documento de diseño. Complementa a diseno_relieve_tectonico.md (incremento 4 del terreno), a nota_ciclo_fosas.md (versión 6) y a adenda_balance_corteza.md (versión 9, §6.10: estiramiento de los márgenes).

Estado: diseño cerrado salvo las preguntas del §11. Es la última nota de la tectónica; tras implementarla se congela la capa.

---

## 1. Fenómeno real y objetivo

### Cómo se rompen los continentes en la Tierra

**Frecuencia.** Los continentes se parten con frecuencia. Pangea empezó a romperse hace unos 200 Myr y en ~150 Myr se abrieron varios océanos y mares por etapas: el Atlántico central, el Atlántico sur, el Índico entre África, India, Madagascar, Antártida y Australia, el Atlántico norte y, más tarde, el mar Rojo. Son del orden de 6–10 rupturas grandes en ese tiempo.

**Los continentes grandes se rompen más.** Un supercontinente aísla el manto de debajo y está rodeado de fosas que tiran de sus bordes. Por eso los supercontinentes se rompen, y los continentes pequeños rara vez.

**Se rompen por sus cicatrices.** Las rupturas aprovechan zonas débiles antiguas, sobre todo las suturas de choques anteriores. El Atlántico se abrió aproximadamente por donde se había cerrado un océano anterior, Iapetus. Es la observación que dio nombre al ciclo de Wilson: los océanos se cierran y vuelven a abrirse por el mismo sitio.

**Consecuencias.** Cada ruptura crea un océano nuevo y joven, dos márgenes pasivos estirados (sobre los que se forman las plataformas) y una reorganización de las placas. Los océanos se abren y se cierran en ciclos: unos 1,5 ciclos de supercontinente en 630 Myr.

### Qué pasa hoy en el modelo

- Hay 1–4 rifts en 500 Myr (τ_rift = 150 Myr), frente a 6–10 solo en la ruptura de Pangea.
- Por eso casi no hay márgenes estirados: la regla de estiramiento (adenda §6.10) funciona, pero casi no tiene dónde actuar. Los márgenes pasivos tienen 35 km de grosor en la costa y una franja estirada de 65 km, frente a 100–300 km en la Tierra.
- Sin océanos nuevos, el fondo envejece (119 Myr de media, 22 % por encima de 180 Myr), y apenas un 0–6 % de ese fondo viejo viene del estado inicial: lo crea la deriva y luego nadie lo consume.

### Objetivo

Que los continentes se rompan a un ritmo terrestre, sobre todo los grandes y por sus suturas. Así deberían salir solos los océanos jóvenes, los márgenes estirados con su plataforma, un fondo de edad realista y el ciclo de supercontinentes.

---

## 2. Alcance y nivel de detalle

Sí se modela:
- Probabilidad de ruptura proporcional al área de cada bloque continental.
- Trazado de la ruptura por la sutura más larga del bloque, si la tiene.
- Tamaño mínimo de los trozos.
- Fase de rift con estiramiento y después rotura (adenda §6.10, ya implementada).

Se abstrae o se deja para más adelante:
- **Plumas del manto** como desencadenantes (Afar, las grandes erupciones al romperse Pangea). Ampliación futura, junto con los puntos calientes.
- **Rifts fallidos** (aulacógenos), que se abren y se detienen sin crear océano. Ampliación futura.
- **Aislamiento térmico** del supercontinente según el tiempo que lleve unido. Queda implícito en que la probabilidad crece con el área.
- **Rifts tras los arcos** (cuencas de trasarco). Van con el retroceso de las fosas, ya en ampliación futura.

---

## 3. Entidades y componentes

No hay entidades nuevas.
- **Bloque continental:** región conexa de corteza continental (más de 20 km) dentro de una placa. Se calcula en cada paso.
- **Suturas:** ya existen en el modelo, de los choques anteriores. Hace falta poder recorrerlas como líneas dentro de un bloque.

---

## 4. Variables de estado

| Símbolo | Significado | Tipo | Rango / unidades |
|---|---|---|---|
| A_k | Área del bloque continental k | real | Mkm² |
| S_k | Sutura más larga contenida en el bloque k, si la hay | línea de celdas | — |
| p_k | Probabilidad de ruptura del bloque k en el paso | real | 0–1 |

---

## 5. Parámetros ajustables

| Parámetro | Significado | Por defecto | Unidades | Origen |
|---|---|---|---|---|
| λ_A | Probabilidad de ruptura por Mkm² de continente y por Myr | 2,5·10⁻⁴ (inicial) | 1/(Mkm²·Myr) | Calibrado: ~4–7 rupturas por 100 Myr con unos 200 Mkm² de corteza continental, que es el ritmo de la ruptura de Pangea |
| A_trozo,min | Área mínima de cada trozo tras la ruptura | 0,5 | Mkm² | Físico: Madagascar, unos 0,6 Mkm², es de los trozos más pequeños separados por un rift |
| L_sut,min | Longitud mínima de una sutura para guiar la ruptura | 3 · L_borde | km | Numérico |

Esta regla sustituye a τ_rift.

---

## 6. Reglas por paso tectónico

### 6.1 Probabilidad de ruptura

Para cada bloque continental k que no esté ya en fase de rift:

p_k = 1 − exp(−λ_A · A_k · Δt)

Con el generador sembrado, recorriendo los bloques en un orden fijo, se decide si el bloque se rompe en el paso.

### 6.2 Trazado

- **Si el bloque contiene una sutura S_k de al menos L_sut,min**, la ruptura sigue esa sutura y se prolonga en línea recta desde sus extremos hasta la costa del bloque.
- **Si no la tiene**, se usa el trazado actual del modelo: un círculo máximo que pasa por el centro del bloque, con orientación sorteada.
- **Si algún trozo queda por debajo de A_trozo,min**, se prueba el trazado alternativo (el del caso sin sutura, con otra orientación sorteada). Si tampoco vale, no hay ruptura en este paso.

### 6.3 Ruptura

El bloque y su placa se dividen por el trazado, con la maquinaria actual de división, generalizada para cortar por una línea cualquiera y no solo por un círculo. El borde nuevo entra en fase de rift y la abertura se rellena estirando los márgenes hasta la rotura (adenda §6.10). Las fuerzas del documento principal deciden después cómo se mueve cada trozo.

---

## 7. Eventos discretos

- **Inicio de rift:** paso, placa, bloque, área de cada trozo, si sigue una sutura y su longitud.
- **Rotura** (fin de la fase de rift, nace el océano): paso, rift, extensión acumulada.

---

## 8. Métricas por paso

- Rupturas por cada 100 Myr, y fracción que sigue una sutura.
- Número de bloques continentales y área del mayor, como fracción del total continental: un índice de supercontinente a lo largo del tiempo.
- Anchura media de la franja estirada de los márgenes y grosor de la corteza en la costa.
- Edad media de los márgenes pasivos que existen, y vida media de los que terminan.

---

## 9. Interacción con otros sistemas y orden dentro del paso tectónico

La ruptura se decide al final del paso, después de choques, delaminación, flujo y erosión. Así la placa nueva se mueve desde el paso siguiente con las fuerzas del documento principal.

Relación con otros trabajos:
- **Estiramiento de márgenes** (adenda §6.10): actúa en cada ruptura.
- **Fosas** (nota_ciclo_fosas.md): los márgenes nuevos pueden ser invadidos más adelante, y la reorganización de placas crea fosas nuevas.
- **Suturas:** las crean los choques (adenda §6.5). Los rifts las reutilizan, y así se cierra el ciclo de Wilson.

---

## 10. Validación y determinismo

| Magnitud | Objetivo | Hoy |
|---|---|---|
| Rupturas por 100 Myr | ~4–7 (ritmo de la ruptura de Pangea) | 0,2–0,8 |
| Anchura de la franja estirada | 100–300 km por lado | 65 km |
| Anchura media de la plataforma | ≈43 km (ETOPO) | 8,2 km |
| Vida media de los márgenes pasivos que terminan | 130–180 Myr (Bradley 2008) | — |
| Edad media del fondo oceánico | 64,2 Myr (Seton et al. 2020) | 119 Myr |
| Ciclo de supercontinentes | del orden de 1,5 ciclos en 630 Myr (criterio blando) | — |
| Rupturas que siguen una sutura | una parte apreciable (criterio blando, sin cifra) | — |

λ_A se calibra con el número de rupturas por 100 Myr, con 5 semillas, y se comprueba con el resto de la tabla.

### Pruebas

1. **Bloque grande con sutura:** a λ_A alto (configuración de prueba), se rompe por la sutura y conserva el volumen de corteza.
2. **Bloque pequeño:** por debajo de 2 · A_trozo,min no se rompe nunca.
3. **Determinismo:** misma semilla, mismas rupturas.

### Determinismo

Una tirada por bloque y paso, en un orden fijo de bloques. Los trazados sorteados usan el mismo generador.

---

## 11. Preguntas abiertas

1. ¿Qué forma tiene hoy la regla de τ_rift (por placa, por continente o global)?
2. ¿Las suturas se guardan como líneas de celdas que se puedan recorrer?
3. ¿La maquinaria de división puede cortar por una línea cualquiera? Si no, ¿cuánto cuesta generalizarla?

---

## Referencias

- Bradley, D. C. (2008). Passive margins through earth history. Earth-Science Reviews 91.
- Seton, M. et al. (2020). A global data set of present-day oceanic crustal age and seafloor spreading parameters. Geochemistry, Geophysics, Geosystems 21.
- Stern, R. J. y Scholl, D. W. (2010). Yin and yang of continental crust creation and destruction by plate tectonic processes. International Geology Review 52.
- Wilson, J. T. (1966). Did the Atlantic close and then re-open? Nature 211.
