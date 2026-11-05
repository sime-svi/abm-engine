# Adenda al relieve tectónico: balance de la corteza continental

Documento de diseño. Complementa a diseno_relieve_tectonico.md (incremento 4 del terreno).

Estado: diseño cerrado salvo las preguntas del §11, que debe responder quien implemente.

Esta adenda sustituye cualquier calibración de η_col pensada para llegar a f_cont = 0,40. A partir de ahora, f_cont es un resultado que se valida, no un objetivo que se fuerza.

Unidades:
- 1 AU (unidad Armstrong) = 1 km³/año = 10⁶ km³/Myr.
- Las tasas por longitud de borde van en km³ por millón de años y por km de fosa (km³/Myr/km).

Los símbolos nuevos se adaptan a los nombres del documento principal allí donde ya existan, por ejemplo el espesor de corteza continental o la densidad del sedimento.

---

## 0. Requisito previo: conservación exacta al mover las placas

El remuestreo de la deriva pierde hoy un 5 % de la corteza. Eso es una fuga de conservación y rompe la primera regla de la malla: conservación en cada corte. Es mayor que el flujo físico más pequeño que hay que medir; los choques continentales mueven 0,4–0,7 AU, en torno al 3–5 % del volumen continental en 500 Myr.

Regla. Toda cantidad por unidad de área que viaje con las placas debe conservar exactamente su total al pasar de un paso al siguiente:

Σ_i Q_i · A_i (después) = Σ_i Q_i · A_i (antes)

Esto vale para el espesor de corteza continental, el sedimento y cualquier otra cantidad equivalente. La tolerancia es la misma que en la malla del terreno (error de redondeo).

La forma natural de conseguirlo es el reparto por solape de áreas: cada celda de destino recibe de cada celda de origen exactamente la fracción de área que se solapa. El método concreto lo elige quien implemente; el requisito es el resultado exacto.

Sin esto, nada de lo que sigue se puede validar.

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

Objetivo: que el volumen de corteza continental y f_cont salgan solos de procesos con tasas medidas. Deben mantenerse aproximadamente estables durante 500 Myr, con las oscilaciones del ciclo de supercontinentes.

---

## 2. Alcance y nivel de detalle

Sí se modela:
- Conservación exacta del remuestreo (§0).
- Producción de los arcos, por km de fosa, según el tipo de arco y la velocidad de convergencia.
- Régimen de cada margen: erosivo o acrecionario.
- Erosión por subducción, por km de fosa.
- Sedimento en la fosa: una parte se pega al margen y el resto se hunde.
- Choques con dos eficacias distintas: arco contra continente y continente contra continente.
- Delaminación de la corteza engrosada.
- Denudación de largo plazo durante la deriva, solo si hoy no existe (§11, pregunta 1). El material erosionado acaba en el mar, en una capa mínima de sedimento.
- Contabilidad del balance por proceso en cada paso.

Se abstrae o se deja para más adelante:
- **Material disuelto que se fija en la corteza oceánica (≈0,4 AU).** Se ignora: es química, no relieve.
- **Erosión por subducción distinta según el régimen del margen.** Se usa una sola tasa media, porque la erosión actúa en todos los márgenes. Ampliación futura.
- **Levantamiento al desprenderse la raíz densa.** En la realidad, al caerse la raíz de eclogita la superficie sube. Aquí la corteza tiene una sola densidad y eso no se reproduce. Ampliación futura.
- **Flujo lateral de la corteza inferior** (las mesetas altas se desparraman hacia los lados). No entra. Si tras esta adenda f_cont sigue bajo, es el primer candidato.
- **Reparto fino del sedimento en el mar** (plataforma, talud, llanura abisal), su compactación, las mesetas oceánicas y los montes submarinos. Van en la nota del fondo oceánico; aquí la capa de sedimento es la mínima necesaria para cerrar el balance.
- **Reciclaje del sedimento subducido a través del magma del arco.** No se modela por separado; la tasa medida de los arcos ya es la producción neta observada.

---

## 3. Entidades y componentes

La adenda no añade entidades.

Datos nuevos por celda de la malla tectónica:
- espesor de sedimento S;
- marca de origen de arco a.

Datos nuevos por tramo de borde convergente:
- régimen r;
- sedimento que llega a la fosa S_fosa.

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
| L_j | Longitud del tramo de fosa j | real | km |
| v_j | Velocidad de convergencia perpendicular al borde en el tramo j | real | cm/año |
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
| q_arc,oc | Producción de un arco sobre placa oceánica | 96 | km³/Myr/km | Medido (Clift et al. 2009); rango publicado 50–125 |
| q_arc,cont | Producción de un arco sobre margen continental | 64 | km³/Myr/km | Medido (Clift et al. 2009) |
| e_SE | Erosión por subducción de la placa de arriba | 33 | km³/Myr/km | Medido: 1,3–1,7 AU globales entre ~43 500 km de margen |
| v_ero | Convergencia por encima de la cual el margen tiende a ser erosivo | 6 | cm/año | Medido (Clift y Vannucchi 2004) |
| S_ero | Sedimento en la fosa por debajo del cual el margen tiende a ser erosivo | 1 | km | Medido (Clift y Vannucchi 2004) |
| f_acr | Fracción del sedimento que se pega a un margen acrecionario | 0,3 (inicial) | adimensional | Calibrado: acreción global ≈0,3 AU frente a 1,0–1,65 AU subducidos |
| η_col | Fracción de la corteza solapada que se conserva en un choque continente–continente | 0,5 | adimensional | Medido: en India–Asia falta en torno al 50 % (40–50 % según otro estudio; dato discutido) |
| η_arco | Fracción conservada en un choque arco–continente | 0,9 | adimensional | Medido: el arco de Luzón se acreciona casi un 90 % en Taiwán |
| h_del | Espesor a partir del cual la base de la corteza se vuelve eclogita y se desprende | 50 | km | Físico (presiones de 1,2–1,5 GPa); rango 45–55 |
| τ_del | Tiempo característico de la delaminación del exceso | 50 (inicial) | Myr | Calibrado: delaminación global ≈1,1 AU y espesor máximo de 70–80 km |
| κ_den | Coeficiente de denudación de largo plazo (§6.7) | calibrar | km^(1−n)/Myr | Calibrado: exportación al mar ≈1,8 AU y relajación a 35–38 km en 100–200 Myr |
| n_den | Exponente de la denudación (la erosión crece más que proporcional con la altura) | calibrar (inicial 2) | adimensional | Calibrado con los mismos dos objetivos |
| ε_cons | Error relativo de conservación admitido por paso | igual que la malla del terreno | adimensional | Numérico |

---

## 6. Reglas por paso tectónico

Δt es el paso tectónico en Myr. Todos los volúmenes van en km³.

### 6.0 Movimiento

Remuestreo conservativo de h_c, S, a y del resto de cantidades que viajan con la placa (§0).

### 6.1 Producción de los arcos

Para cada tramo de fosa j, con el tipo de arco que corresponda (oceánico o continental):

ΔV_arc,j = q_arc(tipo_j) · (v_j / v̄) · L_j · Δt

v̄ es la convergencia media de todas las fosas del planeta en ese paso. La producción crece con la velocidad de convergencia, que es el control principal observado. Al normalizar por la media del propio planeta, la producción global sigue siendo la de los valores medidos.

ΔV_arc,j se suma a h_c en las celdas de arco de la placa de arriba, según la regla de colocación del documento principal. Si el arco crece sobre litosfera oceánica, esas celdas pasan a a_i = 1.

F_arc = Σ_j ΔV_arc,j / (10⁶ · Δt)

### 6.2 Régimen de cada margen

r_j = erosivo si v_j > v_ero y S_fosa,j < S_ero

r_j = acrecionario en cualquier otro caso

### 6.3 Erosión por subducción

Actúa en todos los márgenes convergentes donde la placa de arriba tiene corteza de tipo continental o de arco junto a la fosa, sea cual sea su régimen:

ΔV_SE,j = e_SE · L_j · Δt

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

### 6.7 Denudación de largo plazo

Esta regla solo se añade si hoy la génesis no erosiona durante los 500 Myr (§11, pregunta 1).

En cada celda con tierra emergida (z_i > 0), la tasa de erosión es:

E_i = κ_den · z_i^n_den (km/Myr)

El espesor retirado en el paso es Δh_i = E_i · Δt. Se resta primero de S_i y, cuando no queda sedimento, de h_c,i.

El volumen retirado (Δh_i · A_i) baja por la pendiente máxima de la malla tectónica hasta la primera celda marina, y se suma a su S. Si el camino acaba en una cuenca cerrada (un mínimo local en tierra), el sedimento se queda allí, como ocurre en las cuencas sin salida al mar reales.

Este proceso no pierde volumen: es un traslado interno. F_den solo se registra como métrica, como volumen que llega al mar.

### 6.8 Isostasia y nivel del mar

Se aplican las reglas del documento principal, con el sedimento con su propia densidad (ya implementado).

### 6.9 Comprobación del balance

En cada paso debe cumplirse:

V_cont(t + Δt) − V_cont(t) = (F_arc − F_SE − F_sed − F_col − F_del) · 10⁶ · Δt

El error relativo debe ser como mucho ε_cons. Si no se cumple, el test falla. El remuestreo, la denudación y la acreción no aparecen en la ecuación porque solo trasladan volumen.

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
- longitud total de fosas (km) y fracción de esa longitud en régimen erosivo;
- volumen total de sedimento y espesor medio de sedimento sobre el fondo oceánico;
- error de conservación del paso.

---

## 9. Interacción con otros sistemas y orden dentro del paso tectónico

Orden dentro del paso:

1. Remuestreo conservativo (6.0).
2. Clasificación de bordes del documento principal.
3. Régimen de cada margen (6.2).
4. Producción de los arcos (6.1).
5. Erosión por subducción (6.3).
6. Sedimento en la fosa (6.4).
7. Choques (6.5).
8. Delaminación (6.6).
9. Denudación, si aplica (6.7).
10. Isostasia y nivel del mar (6.8).
11. Comprobación del balance y métricas (6.9 y §8).

Relación con otros trabajos:
- **Nota del fondo oceánico (próxima).** Usará la capa S de esta adenda y definirá cómo se reparte en el mar, su compactación, las mesetas oceánicas y los montes submarinos. Las mesetas acrecionadas a los continentes serán una fuente adicional de corteza.
- **Inicio de subducción en márgenes viejos (próxima).** Al crear fosas nuevas, aumenta la producción de los arcos y los sumideros de las fosas. Es una de las vías por las que el balance cambia con el ciclo de supercontinentes.
- **Erosión del terreno a resolución fina (incremento 3).** Sigue igual durante la simulación. La regla 6.7 es su versión de bajo nivel de detalle para los 500 Myr de génesis.

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
| Longitud de márgenes convergentes oceánicos | ≈43 500 km | von Huene y Scholl 1991 |
| Ciclo de supercontinentes | más creación al romperse, más destrucción al juntarse | Stern y Scholl 2010 |

Comprobación de orden de magnitud para la regla 6.7: en las cordilleras activas la erosión ronda de 0,1 a 1 km por millón de años, y en los escudos antiguos, metros por millón de años.

### Pruebas

1. **Conservación.** La fuga del remuestreo pasa a ser nula, dentro de ε_cons.
2. **Relajación.** Se usa el escenario India–Asia ya calibrado. Se detiene la convergencia y se mide cuánto tarda el espesor máximo en bajar de 38 km. Objetivo: 100–200 Myr.
3. **Balance completo.** Corrida de 500 Myr. Cada flujo debe caer en su rango, V_cont dentro de ±10 % y f_cont dentro del criterio.
4. **Casos límite:**
   - planeta sin fosas: no hay arcos ni sumideros de fosa, y solo actúan la delaminación y la denudación;
   - margen con sedimento muy grueso: siempre acrecionario;
   - corteza sin exceso sobre h_del: no delamina;
   - paso tectónico grande: la delaminación sigue siendo estable gracias a la forma exponencial.

### Determinismo

Esta adenda no usa azar. Todas las reglas son funciones puras del estado. Los repartos entre celdas (6.3 y 6.4) siguen un orden geométrico fijo con desempate por índice.

### Si un objetivo no se cumple

Primero se revisa la base: un mecanismo que falte o un valor medido mal aplicado. Solo los parámetros calibrados (f_acr, τ_del, κ_den, n_den) se ajustan.

Los parámetros medidos pueden moverse dentro de su rango publicado, porque estas tasas tienen incertidumbres de hasta el doble. Fuera de ese rango, no.

---

## 11. Preguntas abiertas (a responder antes de implementar)

1. **¿Hay erosión durante la deriva?** ¿Aplica hoy la génesis tectónica alguna erosión durante los 500 Myr?
   - Si sí: no se añade 6.7, y la erosión existente se valida contra los objetivos de relajación y exportación al mar.
   - Si no: se añade 6.7. Sospecha: sin erosión, las cordilleras de toda la historia siguen enteras, y eso podría explicar parte de la altura excesiva de la tierra.
2. **¿Qué tasa de crecimiento de arcos usa hoy el documento?** Expresada en km³/Myr por km de fosa, para compararla con q_arc.
3. **¿Cuenta hoy en f_cont la corteza de arco sobre litosfera oceánica?** La adenda pide contarla como material continental desde que se crea. ¿Existe ya alguna marca de origen de arco o hay que crear a_i?

---

## Referencias

- Clift, P. y Vannucchi, P. (2004). Controls on tectonic accretion versus erosion in subduction zones. Reviews of Geophysics 42, RG2001.
- Clift, P. D., Vannucchi, P. y Phipps Morgan, J. (2009). Crustal redistribution, crust–mantle recycling and Phanerozoic evolution of the continental crust. Earth-Science Reviews 97, 80–104.
- Clift, P. D., Schouten, H. y Vannucchi, P. (2009). Arc–continent collisions, sediment recycling and the maintenance of the continental crust. Geological Society Special Publication 318.
- Cogley, J. G. (1984). Continental margins and the extent and number of the continents. Reviews of Geophysics 22.
- Ingalls, M., Rowley, D. B., Currie, B. y Colman, A. S. (2016). Large-scale subduction of continental crust implied by India–Asia mass-balance calculation. Nature Geoscience 9.
- Replumaz, A. et al. (2010). Crustal mass budget and recycling during the India/Asia collision. Tectonophysics 492.
- Scholl, D. W. y von Huene, R. (2009). Implications of estimated magmatic additions and recycling losses at the subduction zones of accretionary and collisional orogens. Geological Society Special Publication 318.
- Stern, C. R. (2011). Subduction erosion: rates, mechanisms, and its role in arc magmatism and the evolution of the continental crust and mantle. Gondwana Research 20.
- Stern, R. J. y Scholl, D. W. (2010). Yin and yang of continental crust creation and destruction by plate tectonic processes. International Geology Review 52.
- von Huene, R. y Scholl, D. W. (1991). Observations at convergent margins concerning sediment subduction, subduction erosion, and the growth of continental crust. Reviews of Geophysics 29.
