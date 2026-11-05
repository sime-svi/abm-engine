# Adenda al relieve tectónico: balance de la corteza continental

Documento de diseño. Complementa a diseno_relieve_tectonico.md (incremento 4 del terreno).

Estado: versión 2. Diseño cerrado; las preguntas del §11 ya están respondidas.

Esta adenda sustituye cualquier calibración de η_col pensada para llegar a f_cont = 0,40. A partir de ahora, f_cont es un resultado que se valida, no un objetivo que se fuerza.

Cambios respecto a la versión 1:
- Las tasas de los arcos y de la erosión por subducción pasan de "por km de fosa" a "por km² de placa que se hunde". Así no dependen de la longitud ni de la forma de los bordes.
- La regla 6.7 ya no añade una ley de erosión nueva: corrige el destino del sedimento de la erosión que ya existe.
- Se añaden objetivos de área subducida y de longitud de bordes medida en la misma malla.

Unidades:
- 1 AU (unidad Armstrong) = 1 km³/año = 10⁶ km³/Myr.
- Las tasas por área van en km³ de corteza por km² de placa que se hunde (km³/km²). Equivalen a "km de espesor".

Los símbolos nuevos se adaptan a los nombres del documento principal allí donde ya existan, por ejemplo el espesor de corteza continental o la densidad del sedimento.

---

## 0. Requisito previo: conservación exacta al mover las placas

Cumplido. La fuga es de 2·10⁻¹⁵ del volumen en 500 Myr, y hay un test y una métrica de la batería que la vigilan.

Regla. Toda cantidad por unidad de área que viaje con las placas conserva exactamente su total al pasar de un paso al siguiente:

Σ_i Q_i · A_i (después) = Σ_i Q_i · A_i (antes)

Esto vale para el espesor de corteza continental, el sedimento y cualquier otra cantidad equivalente. Nunca se copia material: solo se reparte el volumen calculado.

---

## 1. Fenómeno real y objetivo

La corteza continental se crea y se destruye a la vez.

Se crea sobre todo en los arcos volcánicos de las zonas de subducción. Además, los arcos oceánicos se pegan a los continentes cuando chocan con ellos.

Vuelve al manto por cuatro vías:
- el sedimento que se hunde en las fosas;
- la erosión por subducción: la placa que baja raspa el borde y la base de la placa de arriba;
- la delaminación: la base de la corteza demasiado gruesa se transforma en una roca más densa que el manto y se desprende;
- la corteza que se hunde en los choques entre continentes.

En la Tierra actual ambos lados rondan 3–5 AU y están en equilibrio aproximado, o con una ligera pérdida neta. El balance cambia con el ciclo de supercontinentes: se crea más corteza cuando un supercontinente se rompe, porque nacen zonas de subducción nuevas, y se destruye más cuando se junta, por los choques.

El control principal de la producción de los arcos es cuánta placa se hunde, es decir, la velocidad de convergencia. La erosión por subducción también crece con la convergencia.

Objetivo: que el volumen de corteza continental y f_cont salgan solos de procesos con tasas medidas. Deben mantenerse aproximadamente estables durante 500 Myr, con las oscilaciones del ciclo de supercontinentes.

---

## 2. Alcance y nivel de detalle

Sí se modela:
- Conservación exacta del remuestreo (§0, ya hecho).
- Producción de los arcos por km² de placa que se hunde, según el tipo de arco.
- Régimen de cada margen: erosivo o acrecionario.
- Erosión por subducción por km² de placa que se hunde.
- Sedimento en la fosa: una parte se pega al margen y el resto se hunde.
- Choques con dos eficacias distintas: arco contra continente y continente contra continente.
- Delaminación de la corteza engrosada.
- Destino del sedimento de la erosión que ya existe durante la deriva: acaba en el mar, en una capa mínima de sedimento. Hoy se pierde.
- Contabilidad del balance por proceso en cada paso.

Se abstrae o se deja para más adelante:
- **Material disuelto que se fija en la corteza oceánica (≈0,4 AU).** Se ignora: es química, no relieve.
- **Erosión por subducción distinta según el régimen del margen.** Se usa una sola tasa media por área, porque la erosión actúa en todos los márgenes. Ampliación futura.
- **Levantamiento al desprenderse la raíz densa.** En la realidad, al caerse la raíz de eclogita la superficie sube. Aquí la corteza tiene una sola densidad y eso no se reproduce. Ampliación futura.
- **Flujo lateral de la corteza inferior** (las mesetas altas se desparraman hacia los lados). No entra. Si tras esta adenda f_cont sigue bajo, es el primer candidato.
- **Reparto fino del sedimento en el mar** (plataforma, talud, llanura abisal), su compactación, las mesetas oceánicas y los montes submarinos. Van en la nota del fondo oceánico; aquí la capa de sedimento es la mínima necesaria para cerrar el balance.
- **Reciclaje del sedimento subducido a través del magma del arco.** No se modela por separado; la tasa medida de los arcos ya es la producción neta observada.
- **Forma de los bordes de placa.** Se trata en la nota siguiente. Esta adenda no depende de ella.

---

## 3. Entidades y componentes

La adenda no añade entidades.

Datos nuevos por celda de la malla tectónica:
- espesor de sedimento S;
- marca de origen de arco a.

Datos nuevos por tramo de borde convergente:
- régimen r;
- sedimento que llega a la fosa S_fosa;
- área de placa consumida A_sub.

Datos globales nuevos: los flujos por proceso y el volumen continental.

---

## 4. Variables de estado

| Símbolo | Significado | Tipo | Rango / unidades |
|---|---|---|---|
| h_c,i | Espesor de corteza de tipo continental en la celda i. Incluye lo que un arco ha añadido sobre litosfera oceánica (solo lo añadido, no los 7 km basálticos de base). | real | 0–80 km |
| S_i | Espesor de sedimento en la celda i, como roca compactada | real | 0–15 km |
| a_i | Origen de arco: 1 si la corteza continental de la celda la formó un arco que aún no se ha unido a un continente | booleano | 0 / 1 |
| z_i | Altura sobre el nivel del mar (ya existe) | real | km |
| A_i | Área de la celda (ya existe) | real | km² |
| A_sub,j | Área de la placa que baja consumida en la fosa j durante el paso. La da exacta el remuestreo conservativo. | real | km² |
| v_j | Velocidad de convergencia perpendicular al borde en el tramo j. Solo se usa para decidir el régimen del margen. | real | cm/año |
| r_j | Régimen del margen j | estado | erosivo / acrecionario |
| S_fosa,j | Espesor medio de sedimento de la placa que llega a la fosa j | real | km |
| V_cont | Volumen global de material continental = Σ_i (h_c,i + S_i) · A_i | real | km³ |
| F_arc, F_SE, F_sed, F_col, F_del, F_den | Flujo global de cada proceso en el paso (§6) | real | AU |

Todo el sedimento cuenta como material continental, porque en este modelo su única fuente es la erosión de la tierra.

---

## 5. Parámetros ajustables

"Medido" significa que el valor viene de la Tierra y no se toca. "Calibrado" significa que no se puede medir directamente y se ajusta contra los objetivos del §10.

| Parámetro | Significado | Por defecto | Unidades | Origen |
|---|---|---|---|---|
| p_arc,oc | Corteza que fabrica un arco sobre placa oceánica por cada km² de placa que se hunde | 1,2 | km³/km² | Medido: 96 km³/Myr por km de arco (Clift et al. 2009) dividido por la convergencia media terrestre, ≈80 km/Myr (unos 3,4 km²/año repartidos en ~43 500 km de margen convergente) |
| p_arc,cont | Ídem para un arco sobre margen continental | 0,8 | km³/km² | Medido: 64 km³/Myr por km (Clift et al. 2009), con la misma conversión |
| p_SE | Erosión por subducción por cada km² de placa que se hunde | 0,4 | km³/km² | Medido: 1,3–1,7 AU globales repartidos entre 2,7–3,4 km²/año subducidos |
| v_ero | Convergencia por encima de la cual el margen tiende a ser erosivo | 6 | cm/año | Medido (Clift y Vannucchi 2004) |
| S_ero | Sedimento en la fosa por debajo del cual el margen tiende a ser erosivo | 1 | km | Medido (Clift y Vannucchi 2004) |
| f_acr | Fracción del sedimento que se pega a un margen acrecionario | 0,3 (inicial) | adimensional | Calibrado: acreción global ≈0,3 AU frente a 1,0–1,65 AU subducidos |
| η_col | Fracción de la corteza solapada que se conserva en un choque continente–continente | 0,5 | adimensional | Medido: en India–Asia falta en torno al 50 % (40–50 % según otro estudio; dato discutido) |
| η_arco | Fracción conservada en un choque arco–continente | 0,9 | adimensional | Medido: el arco de Luzón se acreciona casi un 90 % en Taiwán |
| h_del | Espesor a partir del cual la base de la corteza se vuelve eclogita y se desprende | 50 | km | Físico (presiones de 1,2–1,5 GPa); rango 45–55 |
| τ_del | Tiempo característico de la delaminación del exceso | 50 (inicial) | Myr | Calibrado: delaminación global ≈1,1 AU y espesor máximo de 70–80 km |
| τ_ero | Tiempo de la erosión de la génesis (ya existe en el documento principal) | 150 | Myr | Calibrado junto con τ_del: relajación a 35–38 km en 100–200 Myr y exportación al mar ≈1,8 AU |
| ε_cons | Error relativo de conservación admitido por paso | igual que la malla del terreno | adimensional | Numérico |

---

## 6. Reglas por paso tectónico

Δt es el paso tectónico en Myr. Todos los volúmenes van en km³.

### 6.0 Movimiento

Remuestreo conservativo de h_c, S, a y del resto de cantidades que viajan con la placa (§0). El remuestreo registra además, para cada fosa j, el área de la placa que baja que se ha consumido allí en el paso: A_sub,j.

### 6.1 Producción de los arcos

Para cada tramo de fosa j, con el tipo de arco que corresponda (oceánico o continental):

ΔV_arc,j = p_arc(tipo_j) · A_sub,j

La producción es proporcional a cuánta placa se hunde, que es el control principal observado.

Además, el área consumida apenas depende de lo dentado que esté el borde. Para placas rígidas, el área neta que cruza un borde entre dos puntos es la misma sea el borde recto o en zigzag; solo los dientes muy exagerados la inflan. Por eso los flujos no se multiplican aunque las fosas del modelo midan más que las reales.

ΔV_arc,j se suma a h_c en las celdas de arco de la placa de arriba, según la regla de colocación del documento principal. Si el arco crece sobre litosfera oceánica, esas celdas pasan a a_i = 1.

F_arc = Σ_j ΔV_arc,j / (10⁶ · Δt)

### 6.2 Régimen de cada margen

r_j = erosivo si v_j > v_ero y S_fosa,j < S_ero

r_j = acrecionario en cualquier otro caso

### 6.3 Erosión por subducción

Actúa en todos los márgenes convergentes donde la placa de arriba tiene corteza de tipo continental o de arco junto a la fosa, sea cual sea su régimen:

ΔV_SE,j = p_SE · A_sub,j

Ese volumen se resta de h_c en la celda de la placa de arriba más cercana a la fosa. Si esa celda se queda sin corteza continental, se sigue restando en la siguiente hacia el interior. Así el margen retrocede, como en los márgenes erosivos reales, a ritmos de pocos km por millón de años.

El orden de reparto es fijo: por distancia a la fosa y, en caso de empate, por índice de celda. No hay azar.

F_SE = Σ_j ΔV_SE,j / (10⁶ · Δt), y es un sumidero: ese volumen pasa al manto.

### 6.4 Sedimento que llega a la fosa

V_llega,j es el volumen de sedimento (S_i · A_i) de las celdas de la placa que baja consumidas en la fosa j durante el paso.

- Si r_j es acrecionario: f_acr · V_llega,j se suma a h_c de la celda de margen de la placa de arriba (pasa a formar parte del continente), y (1 − f_acr) · V_llega,j pasa al manto.
- Si r_j es erosivo: todo V_llega,j pasa al manto.

F_sed es la suma de lo que pasa al manto.

### 6.5 Choques

Cuando la corteza de tipo continental de la placa de abajo se solapa con la de arriba, sea V_sol el volumen solapado en el paso. Se elige la eficacia según el origen de esa corteza:

- η = η_arco si la corteza solapada es de arco (a_i = 1);
- η = η_col si es continental.

η · V_sol se suma a la placa de arriba con la regla de engrosamiento del documento principal. (1 − η) · V_sol pasa al manto y forma F_col.

Tras un choque arco–continente, la corteza acrecionada pasa a a_i = 0: ya es continente.

### 6.6 Delaminación

En toda celda con h_c,i > h_del:

h_c,i ← h_del + (h_c,i − h_del) · exp(−Δt / τ_del)

Lo retirado pasa al manto y forma F_del. Se usa la forma exponencial exacta porque es estable para cualquier Δt.

### 6.7 Destino del sedimento de la erosión existente

La génesis ya erosiona durante la deriva con la regla del documento principal (tiempo τ_ero). No se añade otra ley de erosión. Lo que cambia es adónde va el material. Hoy solo cabe en los márgenes continentales y el resto se pierde (≈1,28 AU), lo que rompe la conservación.

Nueva regla:
- Si la erosión actúa sobre una celda con sedimento, lo retira primero de S_i y, cuando no queda, de h_c,i.
- El volumen erosionado baja por la pendiente máxima de la malla tectónica hasta la primera celda marina y se suma a su S.
- Si el camino acaba en una cuenca cerrada (un mínimo local en tierra), el sedimento se queda allí, como ocurre en las cuencas sin salida al mar reales.

Este proceso no pierde volumen: es un traslado interno. F_den solo se registra como métrica, como volumen que llega al mar.

### 6.8 Isostasia y nivel del mar

Se aplican las reglas del documento principal, con el sedimento con su propia densidad (ya implementado).

### 6.9 Comprobación del balance

En cada paso debe cumplirse:

V_cont(t + Δt) − V_cont(t) = (F_arc − F_SE − F_sed − F_col − F_del) · 10⁶ · Δt

El error relativo debe ser como mucho ε_cons. Si no se cumple, el test falla. El remuestreo, la erosión con su sedimento y la acreción no aparecen en la ecuación porque solo trasladan volumen.

---

## 7. Eventos discretos

- **Inicio y fin de un choque:** paso, placas implicadas, tipo (arco–continente o continente–continente), η usado, volumen solapado acumulado y volumen perdido.
- **Cambio de régimen de un margen:** paso, placa, longitud de fosa afectada, régimen nuevo. Se agrega por paso para no generar ruido.
- **Retroceso de margen:** paso, celda que deja de tener corteza continental por erosión por subducción.

---

## 8. Métricas por paso

Se registran en cada paso tectónico, o cada N Myr, con N como parámetro:
- F_arc, F_SE, F_sed, F_col, F_del y F_den, en AU;
- V_cont y f_cont;
- espesor máximo de corteza y espesor medio de la corteza continental;
- área total subducida y área creada en las dorsales (km²/año);
- longitud total de fosas, dorsales y fallas transformantes (km), y fracción de fosa en régimen erosivo;
- volumen total de sedimento y espesor medio de sedimento sobre el fondo oceánico;
- error de conservación del paso.

---

## 9. Interacción con otros sistemas y orden dentro del paso tectónico

Orden dentro del paso:

1. Remuestreo conservativo, con registro de A_sub,j (6.0).
2. Clasificación de bordes del documento principal.
3. Régimen de cada margen (6.2).
4. Producción de los arcos (6.1).
5. Erosión por subducción (6.3).
6. Sedimento en la fosa (6.4).
7. Choques (6.5).
8. Delaminación (6.6).
9. Erosión del documento principal, con el destino del sedimento de 6.7.
10. Isostasia y nivel del mar (6.8).
11. Comprobación del balance y métricas (6.9 y §8).

Relación con otros trabajos:
- **Nota siguiente: fondo oceánico, inicio de subducción, rifts y forma de los bordes.** Usará la capa S de esta adenda y definirá cómo se reparte en el mar, su compactación, las mesetas oceánicas y los montes submarinos. Las mesetas acrecionadas a los continentes serán una fuente adicional de corteza. Las fosas nuevas aumentan la producción de los arcos y los sumideros de las fosas; es una de las vías por las que el balance cambia con el ciclo de supercontinentes.
- **Erosión del terreno a resolución fina (incremento 3).** Sigue igual durante la simulación. La erosión de la génesis tectónica es su versión de bajo nivel de detalle para los 500 Myr. Ambas deben mandar su sedimento al mar.

---

## 10. Validación y determinismo

### Objetivos contra la Tierra

| Magnitud | Tierra | Fuente |
|---|---|---|
| Producción de los arcos | 2,7–5 AU | Scholl y von Huene 2009; Stern y Scholl 2010 |
| Sedimento subducido | 1,0–1,65 AU | von Huene y Scholl 1991; Clift et al. 2009 |
| Erosión por subducción | 1,3–1,7 AU | Clift et al. 2009; Stern 2011 |
| Delaminación | ≈1,1 AU (mal acotada) | Clift et al. 2009 |
| Pérdida en choques continentales | 0,4–0,7 AU | Clift et al. 2009; Stern y Scholl 2010 |
| Sedimento acrecionado | ≈0,3 AU | Clift et al. 2009 |
| Sedimento exportado al mar | ≈1,8 AU (media del Cenozoico) | Clift et al. 2009 |
| Creación total / destrucción total | 3,2–5 / 3,2–5,3 AU | Stern y Scholl 2010 |
| Volumen de corteza continental | ≈7·10⁹ km³ | Cogley 1984 |
| Cambio de V_cont en 500 Myr | dentro de ±10 % (criterio propuesto: equilibrio aproximado o ligera pérdida) | Derivado de Clift et al. 2009 y Stern y Scholl 2010 |
| f_cont al final | 0,40 ± 0,03 (criterio propuesto) | — |
| Relajación de la corteza engrosada | vuelve a 35–38 km en 100–200 Myr | Clift et al. (arcos y continentes) |
| Espesor máximo de corteza | 70–80 km | Tíbet, Andes centrales |
| Área de fondo oceánico que se hunde (igual a la que se crea en las dorsales) | 2,7–3,4 km²/año | Cogné y Humler 2006; Rowley 2002; Bird 2003 |
| Longitud de bordes por tipo (fosas, dorsales, fallas transformantes) | La de los bordes de Bird (2003) proyectados sobre la misma malla y medidos con el mismo método | Bird 2003 |
| Ciclo de supercontinentes | más creación al romperse, más destrucción al juntarse | Stern y Scholl 2010 |

La longitud de bordes no se compara con cifras publicadas en km, porque una malla de 52 km alarga cualquier línea al dibujarla en escalera. Se compara con la Tierra medida igual.

Comprobación de orden de magnitud para la erosión de la génesis: en las cordilleras activas la erosión ronda de 0,1 a 1 km por millón de años, y en los escudos antiguos, metros por millón de años.

### Pruebas

1. **Conservación.** Hecha: la fuga es error de redondeo.
2. **Relajación.** Se usa el escenario India–Asia ya calibrado. Se detiene la convergencia y se mide cuánto tarda el espesor máximo en bajar de 38 km. Objetivo: 100–200 Myr. Hoy, solo con erosión, tarda ~345 Myr.
3. **Balance completo.** Corrida de 500 Myr. Cada flujo debe caer en su rango, el área subducida en 2,7–3,4 km²/año, V_cont dentro de ±10 % y f_cont dentro del criterio.
4. **Casos límite:**
   - planeta sin fosas: no hay arcos ni sumideros de fosa, y solo actúan la delaminación y la erosión;
   - margen con sedimento muy grueso: siempre acrecionario;
   - corteza sin exceso sobre h_del: no delamina;
   - paso tectónico grande: la delaminación sigue siendo estable gracias a la forma exponencial;
   - borde muy dentado frente a borde recto entre las mismas placas: A_sub total prácticamente igual.

### Determinismo

Esta adenda no usa azar. Todas las reglas son funciones puras del estado. Los repartos entre celdas (6.3 y 6.4) siguen un orden geométrico fijo con desempate por índice.

### Si un objetivo no se cumple

Primero se revisa la base: un mecanismo que falte o un valor medido mal aplicado. Solo los parámetros calibrados (f_acr, τ_del, τ_ero) se ajustan.

Los parámetros medidos pueden moverse dentro de su rango publicado, porque estas tasas tienen incertidumbres de hasta el doble. Fuera de ese rango, no.

---

## 11. Preguntas abiertas (respondidas)

1. **Erosión durante la deriva.** Sí existe (τ_ero = 150 Myr). No se añade otra ley: se corrige el destino del sedimento (6.7) y se valida con la prueba de relajación. Hoy tarda ~345 Myr en bajar de 65 a 38 km; la delaminación debería acercarlo a 100–200 Myr.
2. **Producción de los arcos.** Hoy es fija, 40 km³/Myr por km de fosa, sin depender de la convergencia ni del tipo de arco. Se sustituye por 6.1.
3. **Corteza de arco en f_cont.** Hoy no cuenta hasta pasar de 20 km, y entonces entra entera, con sus 7 km de basalto. Se crea la marca a_i y se cuenta solo lo añadido, desde que se crea.

Queda abierta una cuestión que no bloquea esta adenda: las fosas y las dorsales del modelo miden más que las de la Tierra (unos 100 000 y 101 500 km). Con las tasas por área de la versión 2, los flujos no dependen de esa longitud. La forma de los bordes se trata en la nota siguiente.

---

## Referencias

- Bird, P. (2003). An updated digital model of plate boundaries. Geochemistry, Geophysics, Geosystems 4.
- Clift, P. y Vannucchi, P. (2004). Controls on tectonic accretion versus erosion in subduction zones. Reviews of Geophysics 42, RG2001.
- Clift, P. D., Vannucchi, P. y Phipps Morgan, J. (2009). Crustal redistribution, crust–mantle recycling and Phanerozoic evolution of the continental crust. Earth-Science Reviews 97, 80–104.
- Clift, P. D., Schouten, H. y Vannucchi, P. (2009). Arc–continent collisions, sediment recycling and the maintenance of the continental crust. Geological Society Special Publication 318.
- Cogley, J. G. (1984). Continental margins and the extent and number of the continents. Reviews of Geophysics 22.
- Cogné, J.-P. y Humler, E. (2006). Trends and rhythms in global seafloor generation rate. Geochemistry, Geophysics, Geosystems 7.
- Ingalls, M., Rowley, D. B., Currie, B. y Colman, A. S. (2016). Large-scale subduction of continental crust implied by India–Asia mass-balance calculation. Nature Geoscience 9.
- Replumaz, A. et al. (2010). Crustal mass budget and recycling during the India/Asia collision. Tectonophysics 492.
- Rowley, D. B. (2002). Rate of plate creation and destruction: 180 Ma to present. GSA Bulletin 114.
- Scholl, D. W. y von Huene, R. (2009). Implications of estimated magmatic additions and recycling losses at the subduction zones of accretionary and collisional orogens. Geological Society Special Publication 318.
- Stern, C. R. (2011). Subduction erosion: rates, mechanisms, and its role in arc magmatism and the evolution of the continental crust and mantle. Gondwana Research 20.
- Stern, R. J. y Scholl, D. W. (2010). Yin and yang of continental crust creation and destruction by plate tectonic processes. International Geology Review 52.
- von Huene, R. y Scholl, D. W. (1991). Observations at convergent margins concerning sediment subduction, subduction erosion, and the growth of continental crust. Reviews of Geophysics 29.
