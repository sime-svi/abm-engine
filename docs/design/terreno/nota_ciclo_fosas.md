# Nota de diseño: ciclo de vida de las fosas (nacimiento, vida y muerte de la subducción)

Documento de diseño. Complementa a diseno_relieve_tectonico.md (incremento 4 del terreno), a nota_bordes_placa.md y a adenda_balance_corteza.md (versión 4).

Estado: diseño cerrado salvo las preguntas del §11. Antes de implementarla hay que hacer el diagnóstico del §0.

---

## 0. Diagnóstico previo (antes de implementar)

Hipótesis: el modelo convierte en fosa cualquier tramo convergente desde el primer paso, incluidos los que convergen muy despacio. En la Tierra, esos tramos absorben la convergencia engrosándose, sin fosa.

Los números la apoyan:
- **En la Tierra**, 2,7–3,4 km²/año de fondo se hunden en ~37 300 km de fosa (medidos con la misma regla). La convergencia media en las fosas es de unos 7–9 cm/año.
- **En el modelo**, 3,6 km²/año se hunden en ~74 000 km. La media es de unos 4,9 cm/año.

El modelo tiene, por tanto, el doble de fosa yendo a la mitad de velocidad.

Medida pedida:
- Distribución de la convergencia perpendicular a lo largo de las fosas del modelo.
- La misma distribución en las fosas de Bird (clase SUB) y en sus bordes convergentes sin fosa (clase OCB).
- Fracción de longitud de fosa por debajo de 2 cm/año en cada caso.

Si el exceso del modelo está en tramos lentos, la hipótesis se confirma. Si el exceso está en tramos rápidos, la causa está en las fuerzas que mueven las placas, y se trae aquí antes de seguir.

La nota se implementa en ambos casos, porque además resuelve el fondo oceánico demasiado viejo (§1).

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
- La longitud de fosa es el doble de la terrestre, con la mitad de velocidad media (§0).
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
- Tirón de la losa creciente desde el nacimiento, si el documento principal no lo tiene ya (§11, pregunta 1).

Se abstrae o se deja para más adelante:
- **Retroceso de la fosa y apertura de cuencas tras el arco (back-arc).** Son reales e importantes; ampliación futura.
- **Propagación lateral gradual de una fosa nueva.** La fosa nace de una vez en su tramo candidato.
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
| λ_esp | Probabilidad de nacimiento espontáneo por km de tramo candidato y por Myr, para fondo de 100 Myr | calibrar | 1/(km·Myr) | Calibrado contra la distribución de edades del fondo (Seton et al. 2020) |
| L_losa | Longitud de losa con la que el tirón alcanza su valor pleno | 800 | km | Geométrico: losa que llega a la base del manto superior (660 km) con inclinaciones de 45–60°; rango 660–1 000 |

---

## 6. Reglas por paso tectónico

### 6.1 Tramos convergentes nuevos

Todo tramo clasificado como convergente (nota_bordes_placa.md, 6.3) que no pertenezca a una fosa activa empieza en estado difuso con X_j = 0.

### 6.2 Convergencia difusa

En un tramo difuso no se consume placa. El área que converge en el paso (el solape del tramo) se absorbe así:
- la corteza y el sedimento de las celdas solapadas se reparten como espesor entre las celdas de ambas placas a menos de W_dif del borde, en proporción a su área;
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

Se divide cada candidato en segmentos de longitud L_borde. Para cada segmento, con a la edad del fondo del lado que se hundiría:

p = 1 − exp(−λ_esp · (a / 100 Myr) · L_borde · Δt)

Con el generador sembrado se decide si nace una fosa en ese segmento.

Si nace en un margen pasivo:
- La parte oceánica se separa de la placa como placa nueva, con la misma maquinaria que usen los rifts.
- La fosa ocupa todo el tramo candidato contiguo de esa placa, hasta los bordes existentes, que se cierran con fallas transformantes.
- Su polaridad es la del océano bajo el continente.

Si nace en una falla transformante: el tramo pasa a fosa activa, y se hunde el lado viejo.

En ambos casos: t_j = 0, L_j = 0 y T_j = 0. Se registra el evento.

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
| Longitud de fosa activa (Bird, 15 placas, misma regla) | 37 300 km ± 25 % | 74 000 |
| Convergencia media en las fosas | 7–9 cm/año (derivado de Bird y del área subducida) | ~4,9 |
| Fondo que se hunde | 2,7–3,4 km²/año | 3,6 |
| Edad media del fondo oceánico | 64,2 Myr (Seton et al. 2020) | — (mediana 59) |
| Fondo con más de 180 Myr | menos del 1 % del área oceánica | 24 % con más de 200 |
| Velocidad media de las placas | 4,39 cm/año, sin reajustar fuerzas | 4,36 |
| Placas rápidas = las de mucha fosa (Forsyth y Uyeda) | se mantiene | sí |
| Ciclos de supercontinente | del orden de 1,5 en 630 Myr (criterio blando) | — |

λ_esp se calibra con la distribución de edades del fondo: la media y la fracción por encima de 180 Myr. X_ini solo puede moverse dentro de 100–200 km, para acercar la longitud de fosa.

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

## 11. Preguntas abiertas

1. **Tirón de la losa.** ¿Cómo calcula hoy el documento principal (§6.2) el tirón de la losa? ¿Crece con la longitud de la losa? Si no, se añade el factor de 6.5.
2. **División de placas.** ¿Existe ya maquinaria para dividir placas, la de los rifts? El nacimiento espontáneo en márgenes pasivos la necesita.
3. **Zonas de fractura.** ¿Guarda el modelo las trazas de antiguas fallas transformantes dentro de las placas? Si no, los candidatos espontáneos son solo los márgenes pasivos y las transformantes actuales entre placas, que es suficiente para empezar.
4. **Resultado del diagnóstico del §0.**

---

## Referencias

- Bird, P. (2003). An updated digital model of plate boundaries. Geochemistry, Geophysics, Geosystems 4.
- Coltice, N. et al. (2012). Dynamic causes of the relation between area and age of the ocean floor. Science 336.
- Gurnis, M., Hall, C. y Lavier, L. (2004). Evolving force balance during incipient subduction. Geochemistry, Geophysics, Geosystems 5.
- Li, Y. y Gurnis, M. (2023). A simple force balance model of subduction initiation. Geophysical Journal International 232.
- Seton, M. et al. (2020). A global data set of present-day oceanic crustal age and seafloor spreading parameters. Geochemistry, Geophysics, Geosystems 21.
- Stern, R. J. y Scholl, D. W. (2010). Yin and yang of continental crust creation and destruction by plate tectonic processes. International Geology Review 52.
