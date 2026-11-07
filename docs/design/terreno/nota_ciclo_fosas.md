# Nota de diseño: ciclo de vida de las fosas (nacimiento, vida y muerte de la subducción)

Documento de diseño. Complementa a diseno_relieve_tectonico.md (incremento 4 del terreno), a nota_bordes_placa.md y a adenda_balance_corteza.md (versión 5).

Estado: versión 3. Diseño cerrado.

Cambios de la versión 3:
- El nacimiento espontáneo ya no convierte en fosa todo un margen: solo nace donde el fondo es viejo (más de a_min), y la fosa ocupa solo el tramo de fondo viejo (6.4).
- En la convergencia difusa, la corteza oceánica no pasa a corteza continental (6.2).
- Se aceptan tres decisiones de implementación: la corteza que flota es la continental de más de 20 km; una fosa espontánea nace con L = X_ini; el tirón sigue mientras la fosa vive.

---

## 0. Diagnóstico previo (hecho)

La hipótesis era que el exceso de fosa del modelo estaba en tramos que convergen muy despacio. El diagnóstico la descarta:

| | Fosa (km) | Convergencia media (cm/año) | Por debajo de 2 cm/año | Se hunden (km²/año) |
|---|---|---|---|---|
| Modelo (semilla 42) | 74 000 | 5,1 | 18,6 % | 3,6 |
| Bird SUB, tramo a tramo | 51 300 | 5,3 | 21 % | 2,73 |
| Bird OCB, tramo a tramo | 17 400 | 1,3 | 81 % | 0,23 |
| La Tierra en la malla, 15 placas mayores | 37 300 | 4,0 | 36 % | 1,48 |

Conclusiones:
- **El exceso está en fosas rápidas.** La fosa lenta del modelo (13 800 km) es como la de la Tierra.
- **El objetivo de 7–9 cm/año de la versión 1 era un error.** Repartía el área que se hunde en toda la Tierra entre las fosas de las 15 placas mayores, que no incluyen las fosas de las microplacas de arco (Tonga, Marianas…).
- **La referencia coherente es la fosa total de Bird.** Como el modelo no tiene microplacas, hay que comparar con unos 51 000 km a 5,3 cm/año.
- **Frente a esa referencia, el modelo tiene 1,45 veces la longitud.** Casi todo se explica porque hunde más área: 3,6 km²/año frente a 2,7–3,0.

La nota se implementa igualmente, por tres motivos:
- resuelve el fondo oceánico demasiado viejo;
- añade el crecimiento del tirón con la longitud de la losa, que hoy es pleno desde el primer paso y es una causa posible del exceso de fosas rápidas;
- convierte en convergencia difusa los tramos que no llegan a X_ini, como los bordes OCB de la Tierra.

Si tras implementarla la fosa sigue más de 1,25 veces por encima de la referencia, se estudian las fuerzas con las medidas del §10.

---

## 1. Fenómeno real y objetivo

### Cómo funcionan las fosas en la Tierra

Una zona de subducción no aparece en cuanto dos placas se empujan. Tiene una vida propia: nace, dura decenas o cientos de millones de años y muere.

**Nacimiento inducido.** Dos placas empujadas una contra otra a lo largo de una zona débil no forman una fosa enseguida. Primero hace falta que una se meta bajo la otra una distancia acumulada:
- unos 100–150 km en una zona de fractura;
- típicamente unos 200 km en modelos más recientes.

Lo que manda es esa convergencia acumulada, no la velocidad. Hasta entonces, la convergencia se absorbe engrosando la zona del borde, que es lo que Bird clasifica como bordes convergentes sin fosa.

**Nacimiento espontáneo.** La litosfera oceánica vieja y densa junto a una zona débil puede empezar a hundirse por sí sola; el ejemplo clásico es Izu-Bonin-Marianas. Es raro.

**Dónde nacen.** En zonas de fractura antiguas, fallas transformantes, dorsales apagadas y detrás de fosas existentes (cambio de polaridad). Una placa homogénea, sin zonas débiles, no forma una subducción que se mantenga sola.

**Con qué frecuencia.** Muchas zonas de subducción actuales son jóvenes, así que el nacimiento de fosas es un proceso continuo.

**Muerte.**
- Cuando llega corteza que flota y no se hunde: un continente o un arco (ya modelado como choque).
- Cuando las placas dejan de converger, por ejemplo al llegar una dorsal a la fosa; la fosa puede sustituirse por una falla transformante, como ocurrió en California.

**Polaridad.** Cuál de las dos placas se hunde se decide al nacer y se mantiene toda la vida de la fosa:
- el océano se hunde bajo el continente;
- entre dos océanos, se hunde el más viejo y denso.

**Consecuencias medibles.**
- La edad media del fondo oceánico es de 64,2 Myr.
- El fondo de océano abierto más viejo ronda los 180 Myr. Solo quedan bolsas pequeñas más viejas, de unos 280 Myr, en el Mediterráneo oriental.
- Los océanos se abren y se cierran en ciclos: unos 1,5 ciclos de supercontinente en 630 Myr.

### Qué pasa hoy en el modelo

- Todo tramo convergente es fosa desde el primer paso, y la polaridad se decide por par de placas en cada paso.
- La longitud de fosa es 1,45 veces la terrestre, con una velocidad media parecida: el modelo hunde más área que la Tierra (§0).
- El 24 % del fondo oceánico tiene más de 200 Myr. No nacen fosas nuevas que cierren los océanos viejos.

### Objetivo

Las fosas pasan a ser entidades con vida propia:
- nacen solo tras suficiente convergencia acumulada en una zona débil, o espontáneamente en márgenes viejos;
- mantienen su polaridad;
- mueren cuando deja de haber convergencia o cuando llega corteza que flota.

Así deberían salir solas una longitud de fosa y una velocidad de convergencia terrestres, un fondo oceánico de edad realista y ciclos de apertura y cierre de océanos.

---

## 2. Alcance y nivel de detalle

Sí se modela:
- Estado de cada tramo convergente: difuso (aún sin fosa) o fosa activa, con memoria entre pasos.
- Convergencia acumulada en los tramos difusos y nacimiento inducido al superar X_ini.
- Convergencia difusa: el área que converge sin fosa se absorbe engrosando la corteza a ambos lados.
- Polaridad fijada al nacer. Sustituye a la decisión por par de placas que figura como provisional en TREE.md.
- Nacimiento espontáneo en márgenes pasivos viejos y en fallas transformantes con fuerte contraste de edad, con una probabilidad que crece con la edad del fondo.
- Muerte por falta de convergencia sostenida. La muerte por choque ya existe.
- Tirón de la losa creciente desde el nacimiento. El documento principal no lo tiene.

Se abstrae o se deja para más adelante:
- **Retroceso de la fosa y apertura de cuencas tras el arco (back-arc).** Son reales e importantes; ampliación futura.
- **Propagación lateral de fosas**, incluida la invasión de océanos viejos desde fosas existentes, como los arcos de Escocia y del Caribe, que avanzan hacia el Atlántico. Aquí la fosa nace de una vez en el tramo de fondo viejo. Ampliación futura.
- **Inicio de subducción por plumas del manto.** Ampliación futura.
- **Rotura de la losa y saltos de fosa tras un choque de arco.** Se reproducen solo en parte, porque un tramo que sigue convergiendo tras un choque vuelve a acumular convergencia y puede renacer.

---

## 3. Entidades y componentes

**Fosa:** una entidad nueva, o un componente de las celdas de margen; quien implemente elige. Datos:
- identificador;
- placa que baja y placa de arriba;
- estado;
- edad de la fosa;
- convergencia acumulada desde el nacimiento;
- tiempo acumulado sin convergencia.

El estado viaja con la celda de margen de la placa de arriba, como el resto de cantidades de la placa.

**Tramo difuso:** un componente de las celdas de borde convergente sin fosa, con su convergencia acumulada.

---

## 4. Variables de estado

| Símbolo | Significado | Tipo | Rango / unidades |
|---|---|---|---|
| e_j | Estado del tramo convergente j | estado | difuso / activa |
| X_j | Convergencia acumulada en un tramo difuso | real | 0–X_ini km |
| s_j | Placa que se hunde (polaridad), fijada al nacer | identificador | — |
| t_j | Edad de la fosa | real | Myr |
| L_j | Convergencia acumulada desde el nacimiento (longitud de losa) | real | km |
| T_j | Tiempo acumulado con convergencia nula o negativa | real | Myr |
| v_n,j | Convergencia perpendicular al borde en el paso | real | cm/año |
| a_j | Edad de la litosfera oceánica del lado que se hundiría | real | Myr |

---

## 5. Parámetros ajustables

| Parámetro | Significado | Por defecto | Unidades | Origen |
|---|---|---|---|---|
| X_ini | Convergencia acumulada para el nacimiento inducido | 150 | km | Modelos numéricos: 100–150 km en zona de fractura (Gurnis et al. 2004), ~200 km (Li y Gurnis 2023). Se puede mover dentro de 100–200 |
| W_dif | Anchura de la franja donde se reparte el engrosamiento difuso a cada lado | igual a L_borde (150) | km | Del orden de la anchura de los bordes difusos reales |
| τ_muerte | Tiempo con convergencia nula o negativa tras el que muere una fosa | 10 | Myr | Supuesto; ajustable |
| λ_esp | Probabilidad de nacimiento espontáneo por km de tramo candidato y por Myr, por cada 100 Myr de edad por encima de a_min | calibrar | 1/(km·Myr) | Calibrado contra la distribución de edades del fondo (Seton et al. 2020) |
| a_min | Edad mínima del fondo para un nacimiento espontáneo | 140 | Myr | Supuesto físico: solo el fondo viejo y denso se hunde por sí solo. Se puede mover dentro de 100–180 al calibrar |
| L_losa | Longitud de losa con la que el tirón alcanza su valor pleno | 800 | km | Geométrico: losa que llega a la base del manto superior (660 km) con inclinaciones de 45–60°; rango 660–1 000 |

---

## 6. Reglas por paso tectónico

### 6.1 Tramos convergentes nuevos

Todo tramo clasificado como convergente (nota_bordes_placa.md, 6.3) que no pertenezca a una fosa activa empieza en estado difuso con X_j = 0.

### 6.2 Convergencia difusa

En un tramo difuso no se consume placa. El área que converge en el paso (el solape del tramo) se absorbe así:
- la corteza continental o de arco y el sedimento de las celdas solapadas se reparten como espesor a menos de W_dif del borde, en proporción al área: lo continental, en el lado continental; si los dos lados son oceánicos, el sedimento se reparte entre ambos;
- la corteza oceánica (el basalto) nunca pasa a corteza continental. El área oceánica solapada se retira como en una fosa, pero sin arco ni sumideros de la adenda. En la Tierra, el fondo que converge sin fosa se pliega y se engruesa un poco, y a este nivel de detalle su efecto en el relieve es pequeño;
- se conserva el volumen;
- no hay arcos, ni A_sub, ni sumideros.

Al mismo tiempo, se acumula la convergencia:

X_j ← X_j + v_n,j · Δt

con v_n,j convertida a km/Myr (1 cm/año = 10 km/Myr).

Si el tramo deja de converger, X_j se conserva pero no crece. Si el tramo desaparece, se descarta.

Si en un tramo difuso las dos placas tienen corteza continental en el borde, el tramo pasa a ser un choque, que ya está modelado, y no una fosa.

### 6.3 Nacimiento inducido

Cuando X_j ≥ X_ini, el tramo pasa a fosa activa. Se fija entonces su polaridad s_j:
- océano frente a continente o arco: se hunde el océano;
- océano frente a océano: se hunde el lado más viejo (a mayor);
- empate: sorteo sembrado.

Además: t_j = 0, L_j = 0 y T_j = 0. Se registra el evento de nacimiento.

### 6.4 Nacimiento espontáneo

Tramos candidatos:
- **márgenes pasivos**, es decir, transiciones de océano a continente dentro de una misma placa;
- **fallas transformantes entre placas** con contraste de edad entre lados mayor que la mitad de la edad del lado viejo.

Solo cuentan los segmentos donde el fondo del lado que se hundiría es viejo: a > a_min. En la Tierra, el fondo joven no se hunde solo, y ni siquiera el fondo de ~180 Myr del Atlántico ha empezado a hundirse junto a sus márgenes pasivos: el nacimiento espontáneo es raro y exige litosfera vieja y densa.

Se divide cada candidato en segmentos de longitud L_borde. Para cada segmento con a > a_min:

p = 1 − exp(−λ_esp · ((a − a_min) / 100 Myr) · L_borde · Δt)

Con el generador sembrado se decide si nace una fosa en ese segmento.

Si nace en un margen pasivo:
- La fosa ocupa solo el tramo contiguo de margen donde el fondo de al lado sigue teniendo más de a_min, no todo el margen.
- En sus extremos, la fosa se une a los bordes existentes con fallas transformantes, como los bordes que en la Tierra rasgan la placa en el extremo de una fosa.
- La parte oceánica que queda entre la fosa y esos bordes se separa como placa nueva. Se reutiliza la maquinaria de división de los rifts, cortando por el límite entre corteza oceánica y continental, no por un círculo.
- Su polaridad es la del océano bajo el continente.

Si nace en una falla transformante: solo el tramo con fondo de más de a_min pasa a fosa activa, y se hunde el lado viejo.

En ambos casos: t_j = 0, L_j = X_ini y T_j = 0, porque una losa que se hunde por su propio peso tira desde el principio. Se registra el evento.

### 6.5 Vida de la fosa

Mientras está activa, la fosa consume placa en su tramo, con su polaridad fija. Aquí se aplican los procesos de la adenda: régimen, arcos, erosión por subducción y sedimento.

En cada paso:

L_j ← L_j + max(0, v_n,j) · Δt

t_j ← t_j + Δt

Si el documento principal no escala ya el tirón de la losa con su longitud (§11, pregunta 1), el tirón de esta fosa se multiplica por:

min(1, L_j / L_losa)

Una fosa recién nacida apenas tira y va ganando fuerza a medida que su losa se alarga.

### 6.6 Muerte de la fosa

- **Si v_n,j ≤ 0:** T_j ← T_j + Δt. Si v_n,j > 0, T_j vuelve a 0.
- **Si T_j ≥ τ_muerte:** la fosa muere. El tramo se reclasifica según su movimiento (transformante o divergente) y se registra el evento.
- **Si llega corteza continental o de arco:** el choque se resuelve como ya está modelado, y se registra la muerte por choque. Si después el tramo sigue convergiendo, vuelve al estado difuso desde X_j = 0, y puede renacer con otra polaridad, como en un cambio de polaridad real tras el choque de un arco.

---

## 7. Eventos discretos

- **Nacimiento de fosa:** paso, tipo (inducido o espontáneo), placas, polaridad, longitud del tramo, edad del fondo que se hunde.
- **Muerte de fosa:** paso, causa (sin convergencia o choque), edad de la fosa, longitud de losa acumulada.
- **Placa nueva por nacimiento espontáneo:** paso, placa de origen, área de la placa nueva.

---

## 8. Métricas por paso

- Longitud de fosa activa y de tramo difuso (km).
- Convergencia media en las fosas activas, ponderada por longitud (cm/año).
- Distribución de edades de las fosas activas.
- Nacimientos y muertes por cada 100 Myr, por tipo y causa.
- Distribución de edades del fondo oceánico: media, mediana y fracción por encima de 180 y de 200 Myr.
- Área absorbida por convergencia difusa (km²/año).
- Número de placas.

---

## 9. Interacción con otros sistemas y orden dentro del paso tectónico

1. Mover las placas.
2. Borde suavizado, normales y tipo de tramo (nota_bordes_placa.md).
3. Actualizar el estado de los tramos convergentes: nuevos tramos difusos (6.1) y muertes (6.6).
4. Nacimientos inducidos (6.3) y espontáneos (6.4).
5. Solapes y huecos (nota_bordes_placa.md, 6.4). Los solapes en fosas activas se consumen y los de tramos difusos se absorben con 6.2.
6. Procesos de la adenda.
7. Fuerzas para el paso siguiente, con el tirón de cada fosa según 6.5.
8. Isostasia, nivel del mar y métricas.

Relación con otros trabajos:
- **Fuerzas (§6.2 del documento principal).** Al desaparecer fosas falsas, el tirón total puede bajar. Si la velocidad media de las placas cae por debajo de la terrestre, se revisa la física del tirón (densidad y longitud de la losa); no se reajusta a mano.
- **Rifts (nota siguiente).** El nacimiento espontáneo en márgenes pasivos usa la misma maquinaria de división de placas. Con fosas que nacen y rifts que rompen continentes, deberían salir los ciclos de apertura y cierre de océanos.

---

## 10. Validación y determinismo

### Objetivos

Las tolerancias son propuestas.

| Magnitud | Objetivo | Hoy |
|---|---|---|
| Longitud de fosa activa | La de Bird SUB proyectada en la malla con la misma regla (≈51 000 km tramo a tramo), ± 25 % | 74 000 |
| Convergencia media en las fosas | ≈5,3 cm/año (Bird SUB) | 5,1 |
| Longitud de convergencia difusa | La de Bird OCB en la malla (≈17 400 km tramo a tramo), a ≈1,3 cm/año | 0 |
| Fondo que se hunde en fosas | ≈2,7 km²/año (Bird SUB), más ≈0,2 absorbidos en convergencia difusa | 3,6 |
| Edad media del fondo oceánico | 64,2 Myr (Seton et al. 2020) | — (mediana 59) |
| Fondo con más de 180 Myr | menos del 1 % del área oceánica | 24 % con más de 200 |
| Velocidad media de las placas | 4,39 cm/año, sin reajustar fuerzas | 4,36 |
| Placas rápidas = las de mucha fosa (Forsyth y Uyeda) | se mantiene | sí |
| Ciclos de supercontinente | del orden de 1,5 en 630 Myr (criterio blando) | — |

λ_esp se calibra con la distribución de edades del fondo: la media y la fracción por encima de 180 Myr. X_ini solo puede moverse dentro de 100–200 km, para acercar la longitud de fosa. Todo se mide con la media de al menos 5 semillas.

### Si la fosa sigue demasiado larga

Si tras implementar la nota la fosa sigue más de 1,25 veces por encima de la referencia, la causa está en las fuerzas. Se miden y se traen aquí antes de tocar nada:
1. La distribución de la longitud de borde por oblicuidad (de choque frontal a deslizamiento puro), frente a la de Bird.
2. La distribución de tamaños de las placas, frente a las 15 mayores de Bird.
3. Para cada placa, cuánto aporta cada fuerza a su movimiento: tirón de la losa, empuje de dorsal, resistencia de choque y arrastre en la base.
4. El área creada y consumida en total, frente a 2,7–3,4 km²/año.

### Pruebas

1. **Convergencia lenta y corta.** Dos placas oceánicas que convergen a 1 cm/año durante 10 Myr (100 km < X_ini): no nace fosa, la corteza se engruesa en la franja y el volumen se conserva.
2. **Convergencia suficiente.** Las mismas placas a 5 cm/año: la fosa nace a los 3 Myr (150 km), y se hunde el lado más viejo.
3. **Polaridad fija.** Tras nacer, la polaridad no cambia aunque cambien las edades a ambos lados.
4. **Muerte.** Una fosa cuyas placas pasan a separarse muere tras τ_muerte, y el tramo pasa a divergente.
5. **Nacimiento espontáneo.** Con λ_esp alto (configuración de prueba), un margen pasivo viejo se separa como placa nueva con su fosa, conservando la corteza.
6. **No regresión.** El escenario India–Asia sigue igual.

### Determinismo

Todas las reglas son funciones puras del estado. El nacimiento espontáneo usa el generador sembrado, una tirada por segmento candidato y paso, en un orden fijo de segmentos.

---

## 11. Preguntas abiertas (respondidas)

1. **Tirón de la losa.** Depende de la edad del fondo, no de la longitud de la losa. Se añade el factor de 6.5.
2. **División de placas.** Existe (la usan los rifts y los saltos de dorsal), pero corta por un círculo. Para los márgenes pasivos se corta por el tipo de corteza (6.4); el resto se reutiliza.
3. **Zonas de fractura.** No se guardan. Los candidatos espontáneos son los márgenes pasivos y las transformantes actuales entre placas.
4. **Diagnóstico del §0.** Hecho: el exceso está en fosas rápidas. Objetivos del §10 corregidos.

---

## Referencias

- Bird, P. (2003). An updated digital model of plate boundaries. Geochemistry, Geophysics, Geosystems 4.
- Coltice, N. et al. (2012). Dynamic causes of the relation between area and age of the ocean floor. Science 336.
- Gurnis, M., Hall, C. y Lavier, L. (2004). Evolving force balance during incipient subduction. Geochemistry, Geophysics, Geosystems 5.
- Li, Y. y Gurnis, M. (2023). A simple force balance model of subduction initiation. Geophysical Journal International 232.
- Seton, M. et al. (2020). A global data set of present-day oceanic crustal age and seafloor spreading parameters. Geochemistry, Geophysics, Geosystems 21.
- Stern, R. J. y Scholl, D. W. (2010). Yin and yang of continental crust creation and destruction by plate tectonic processes. International Geology Review 52.
