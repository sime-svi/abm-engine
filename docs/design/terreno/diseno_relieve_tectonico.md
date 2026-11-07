# Documento de Diseño: Relieve tectónico con deriva de placas

**Sistema:** Terreno, génesis del relieve (incremento 4 del terreno)
**Versión:** 1.0
**Destinatario:** Claude Code para implementación
**Naturaleza:** Plano teórico. Fenómeno real, lógica de estados y ecuaciones. Sin código.
**Origen:** `docs/design/terreno/propuesta_relieve_realista.md` (diagnóstico de Claude Code) y decisión S1 de la nota de sombra pluviométrica.
**Implementación (2026-11-05):** `incremento-4-diseno.md` (decisiones del responsable, correcciones de realismo, interpretaciones, calibraciones, resultados frente a la Tierra y hallazgos).

**Decisiones cerradas por el director:**
1. Opción C, directamente: el relieve sale de la tectónica de placas, sin pasar antes por un arreglo del generador actual.
2. Física de la Tierra y valores terrestres por defecto. La geografía cambia con la semilla; el agua total y la cantidad de corteza continental tienen los valores de la Tierra y siguen siendo parámetros.
3. **Se simula la deriva de las placas** durante cientos de millones de años antes de la génesis geológica. No es una foto fija: la historia (cordilleras viejas, suturas, márgenes pasivos, edades del fondo oceánico) sale sola.
4. Referencia: estadísticas de ETOPO 2022 y otros conjuntos de datos reales, calculadas a nuestra resolución y guardadas en el repositorio.

**Principio que guía este diseño (CLAUDE §2.8):** las alturas, las cordilleras, las mesetas y las plataformas deben salir de sus causas físicas. Los parámetros con valor físico medible (densidades, espesores, curva edad-profundidad) llevan su valor real. Solo se calibran los que no se pueden medir directamente (la intensidad de las fuerzas que mueven las placas), y siempre contra un resultado observado (la velocidad real de las placas).

**Notación:** `·` multiplicación, `^` exponente, `√` raíz. Myr = millón de años. Las distancias sobre la esfera son geodésicas (por la superficie).

---

## 1. Fenómeno real y objetivo

### Qué se modela
La superficie de un planeta rocoso con agua está hecha de dos tipos de corteza que "flotan" sobre el manto:
- **Corteza continental:** gruesa (~35-40 km de media, hasta ~70 km bajo el Tíbet) y ligera (~2,8 g/cm³). Flota alta y forma los continentes.
- **Corteza oceánica:** fina (~7 km) y más densa (~2,9 g/cm³). Flota baja y forma los fondos oceánicos.

Ese equilibrio de flotación se llama **isostasia**: un bloque de corteza más grueso se hunde más en el manto y también sobresale más, como un témpano de hielo más grande.

La superficie está dividida en **placas** rígidas que se mueven unos centímetros al año:
- **Donde se separan** (bordes divergentes) nace corteza oceánica nueva: las dorsales. Al envejecer, la corteza se enfría, se contrae y se hunde. Por eso el fondo oceánico es más profundo cuanto más viejo.
- **Donde convergen**, la placa oceánica más densa se hunde bajo la otra (subducción). Aparecen una fosa profunda y un arco volcánico que engrosa la corteza (Andes, Japón).
- **Donde chocan dos continentes**, ninguno se hunde: la corteza se apila y se engrosa. Salen cordilleras y, si el choque es grande, mesetas altas (Himalaya y Tíbet).
- **Los continentes se rompen** (rifts) y se abren océanos nuevos (Atlántico), y **se vuelven a juntar** (ciclo de los supercontinentes, de ~300-600 Myr).
- **Las cordilleras viejas se gastan** por erosión durante cientos de millones de años (Apalaches, Urales).

### Qué debe emerger (sin imponerse)
- **Curva de alturas bimodal:** continentes cerca de 0-1 km y fondos oceánicos hacia −4/−5 km.
- **Tierra emergida ~29 %, altura media de la tierra ~800 m y profundidad media del océano ~3 700 m**, como consecuencia del agua total, la corteza continental y la edad del fondo oceánico.
- **Dorsales oceánicas** como lomos de ~2 500 m de profundidad, y fondos viejos de ~5 500-6 000 m.
- **Cordilleras alargadas** en los bordes de placa: costeras tipo Andes, de choque tipo Himalaya, con **mesetas** detrás (Tíbet, Altiplano).
- **Fosas** de 6-11 km frente a las zonas de subducción, y **arcos de islas**.
- **Cordilleras viejas y bajas dentro de los continentes:** suturas de choques antiguos.
- **Plataformas continentales** en los márgenes pasivos.
- Como consecuencia: **laderas realistas**, para que `k_oro = 40` dé 2-3 veces más lluvia en barlovento sin tocarlo, y **ríos realistas**.

### Producto de datos añadido
La deriva genera una **historia geológica del planeta**: rifts, choques, supercontinentes, océanos que se abren y se cierran. Se registra como eventos (§7). Es un dato de salida en sí mismo.

---

## 2. Alcance y LOD

### Escalas de tiempo
- **Génesis tectónica:** pasos de `Δt_tect = 1 Myr` durante `T_deriva = 500 Myr`. Cubre al menos un ciclo de supercontinente y recicla todo el fondo oceánico inicial, porque en la Tierra el fondo oceánico más viejo tiene ~180 Myr. Así, la condición inicial arbitraria queda borrada.
- **Génesis geológica** (prerrodaje de erosión, ya existente): ~1 Myr, después de la tectónica.
- **Simulación (ticks de un día):** **sin tectónica.** A 5 cm/año, una placa se mueve 50 m en mil años, así que es despreciable a la escala de la simulación. Es el nivel de detalle correcto, no un apaño. El estado tectónico final queda como datos estáticos.

### Escala espacial
- La tectónica corre en una **malla de referencia** de lado `d_tect` (por defecto, el nivel 0 de la malla del terreno, ~78 km). Es suficiente para placas de miles de km.
- Los campos tectónicos resultantes (tipo de corteza, espesor, edad, distancias a bordes) se interpolan para cualquier celda más fina, y encima va el ruido fino (§6.8). Así el relieve de una celda fina es función pura de (estado tectónico final, posición, semilla) y respeta las reglas de la malla adaptativa.

### Se modela
Placas rígidas que giran sobre la esfera, dinámica simplificada por fuerzas (tirón de la placa que subduce, empuje de dorsal, rozamiento basal, resistencia de colisión), creación de corteza en dorsales, subducción, arcos, colisión continental con meseta, rifts, suturas (fusión de placas), erosión tectónica de cordilleras, sedimento en márgenes, isostasia, nivel del mar por volumen de agua y rebote isostático en la erosión.

### Se abstrae
- **Fuerzas:** el manto no se simula; se representa con tres fuerzas y un rozamiento.
- **Placas rígidas:** sin deformación interna, salvo en las franjas de borde.
- **Litosfera bajo los continentes:** su efecto queda incluido en la relación empírica espesor-altura.

### Ampliación futura (no implementar ahora)
Puntos calientes y plumas del manto (Hawái, Islandia, grandes provincias ígneas); cambios del nivel del mar en el tiempo geológico; glaciaciones; acoplamiento clima-erosión durante la deriva; vulcanismo activo durante la simulación; deriva polar verdadera.

### Precedente técnico
Cortial et al. (2019), "Procedural Tectonic Planets" (Computer Graphics Forum, Eurographics), muestra que una tectónica simplificada de este tipo es viable para generar planetas. La diferencia aquí es que se calibra contra datos reales.

---

## 3. Entidades y componentes (ECS)

### Entidad `Placa` (solo durante la génesis tectónica)
| Componente | Contenido |
|---|---|
| `Id` | identificador determinista |
| `Rotacion` | orientación acumulada de la placa (rotación 3D sobre el centro del planeta) |
| `VelocidadAngular` | `ω_p`: vector de rotación (eje y rapidez, en rad/Myr) |
| `MapaCorteza` | la corteza que transporta, guardada **en el sistema de referencia propio de la placa** (ver nota) |
| `Historial` | edad desde el último rift o fusión; lista de suturas internas |

**Nota de representación (recomendación, no obligación):** como las placas son rígidas, lo natural es que cada placa guarde su corteza en sus propias coordenadas, y que el mundo se consulte aplicando la rotación inversa: "¿qué placa tiene corteza en este punto?". Así el movimiento es exacto y no se arrastran errores de interpolación paso a paso.

### Propiedades de cada celda de corteza (dentro del `MapaCorteza` de su placa)
| Propiedad | Significado |
|---|---|
| `tipo` | {CONTINENTAL, OCEANICA} |
| `H` | espesor de la corteza (km) |
| `edad_oc` | edad de la corteza oceánica (Myr; solo oceánica) |
| `edad_orog` | tiempo desde el último engrosamiento tectónico (Myr; para la erosión tectónica) |
| `dH_dt` | ritmo de engrosamiento actual (km/Myr; para el levantamiento en el prerrodaje) |

### Componentes estáticos nuevos en las celdas del terreno (resultado final)
| Componente | Contenido |
|---|---|
| `Tectonica` | id de placa, tipo de corteza, `H`, `edad_oc`, `edad_orog`, `dH_dt` |
| `Borde` | tipo del borde más cercano {DIVERGENTE, SUBDUCCION, COLISION, TRANSFORMANTE}, distancia a él y lado (placa que subduce / que cabalga) |

### Recursos globales
| Recurso | Contenido |
|---|---|
| `NivelMar` | `ζ`, calculado a partir del volumen de agua |
| `HistoriaGeologica` | registro de eventos tectónicos (§7) |

---

## 4. Variables de estado

| Símbolo | Significado | Tipo | Rango / Unidades |
|---|---|---|---|
| `ω_p` | velocidad angular de la placa p | vector real | rad/Myr |
| `v(x)` | velocidad de la superficie en x: `R · (ω_p × x̂)` | vector real | km/Myr (1 km/Myr = 0,1 cm/año) |
| `tipo(x)` | tipo de corteza | discreto | {CONTINENTAL, OCEANICA} |
| `H(x)` | espesor de corteza | real | km; [H_oc, H_max] |
| `edad_oc(x)` | edad del fondo oceánico | real | Myr; ≥ 0 |
| `edad_orog(x)` | tiempo desde el último engrosamiento | real | Myr |
| `z(x)` | altura respecto al dato de referencia (antes del nivel del mar) | real | m |
| `ζ` | nivel del mar respecto al dato | real | m |
| `h(x)` | altura final sobre el nivel del mar: `z − ζ` | real | m |
| `N_placas` | número de placas | entero | — |

---

## 5. Parámetros ajustables

### 5.1 Planeta y agua (valores terrestres)
| Símbolo | Significado | Defecto | Unidades | Origen |
|---|---|---|---|---|
| `GEL` | capa global de agua: volumen del océano / superficie del planeta | 2 617 | m | Tierra: 1,335·10⁹ km³ / 5,10·10⁸ km² |
| `f_cont` | fracción de la superficie con corteza continental (incluidas las plataformas) | 0,40 | — | Tierra: ~40 % |

### 5.2 Corteza e isostasia (valores físicos medidos)
| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `ρ_c` | densidad de la corteza continental | 2 800 | kg/m³ |
| `ρ_m` | densidad del manto superior | 3 300 | kg/m³ |
| `H_oc` | espesor de la corteza oceánica | 7 | km |
| `H_0` | espesor de la corteza continental estable | 35 (± ruido `σ_H0` = 3) | km |
| `H_max` | espesor máximo antes del colapso gravitatorio | 65 | km |
| `H_nivel` | espesor continental que queda al nivel del mar (relación empírica terrestre; ver §6.6) | 32 | km |
| `H_cont_min` | espesor a partir del cual un arco oceánico pasa a contar como continental | 20 | km |

### 5.3 Fondo oceánico (Parsons y Sclater, 1977; fondo viejo ajustado a la Tierra)
| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `d_r` | profundidad de la cresta de la dorsal | 2 500 | m |
| `c_hs` | subsidencia por enfriamiento (fondo joven) | 350 | m/Myr^½ |
| `d_∞`, `A_pl`, `τ_pl` | modelo de placa (fondo viejo), ajustado a la mediana del zócalo real (decisión del responsable, 2026-11-07; antes, Parsons y Sclater: 6 400 / 3 200 / 62,8) | 5 310 / 2 373 / 31 | m / m / Myr |
| `t_x` | edad de cambio de un modelo al otro | 20 | Myr |
| `D_fosa`, `w_fosa` | sobreprofundidad y anchura de la fosa | 3 000 / 50 | m / km |

### 5.4 Bordes de placa
| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `q_arco` | crecimiento de corteza en arcos (sección añadida por km de fosa y Myr) | 40 | km²/Myr |
| `d_arco`, `w_arco` | distancia del arco a la fosa y anchura del arco | 200 / 150 | km |
| `η_col` | fracción de la corteza solapada en un choque que se queda engrosando la placa | 0,35 | — |
| `W_min_orog` | anchura mínima de una cordillera de choque | 150 | km |
| `w_margen` | anchura del margen continental adelgazado (plataforma y talud) | 150 | km |
| `f_rift` | factor de adelgazamiento de la corteza en un rift | 0,7 | — |

### 5.5 Dinámica de placas
| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `f_sp` | tirón de la placa que subduce (por km de fosa) | 1 (referencia) | unidad de fuerza relativa |
| `f_rp` | empuje de dorsal (por km de dorsal) | 0,2 · `f_sp` | idem |
| `t_sat` | edad a la que el tirón de la placa se satura | 80 | Myr |
| `c_col` | resistencia de colisión continental | calibrar: un choque debe frenar la convergencia a ~1/3 en 10-20 Myr (India-Asia pasó de ~15 a ~5 cm/año) | idem·Myr/km |
| `κ_cont` | rozamiento basal extra bajo corteza continental (raíces profundas) | 3 | — |
| `c_b` | rozamiento basal | **calibrado** para que la velocidad media sea `v_obj` | — |
| `v_obj` | velocidad media cuadrática de las placas (objetivo de calibración) | 5 | cm/año |

### 5.6 Eventos de placas
| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `N_p0` | placas iniciales | 12 | — |
| `N_cratones` | bloques continentales iniciales | 8 | — |
| `N_min` | mínimo de placas (por debajo se abre una dorsal nueva) | 8 | — |
| `A_min` | área mínima de una placa (por debajo, se absorbe en su vecina) | 0,5 % de la superficie | — |
| `τ_rift` | tiempo medio hasta el rift de un continente que contiene toda la corteza continental | 150 | Myr |
| `v_rift` | velocidad inicial de apertura de un rift | 2 | cm/año |
| `v_sutura`, `t_sutura` | velocidad de convergencia y duración por debajo de las cuales dos placas en colisión se fusionan | 0,5 / 10 | cm/año / Myr |
| `τ_orog` | tiempo de decaimiento del engrosamiento por erosión tectónica | 150 | Myr |

### 5.7 Malla, tiempo y detalle
| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `d_tect` | lado de la malla tectónica | = d(0) ≈ 78 | km |
| `Δt_tect` | paso de la génesis tectónica | 1 | Myr |
| `T_deriva` | duración de la deriva | 500 | Myr |
| `a_llano`, `a_orog`, `a_abisal` | amplitud del ruido fino en llanuras, cordilleras y fondos abisales | 150 / 1 500 / 200 | m |
| `r_flex` | radio de flexión para el rebote isostático | 100 | km |

---

## 6. Reglas y ecuaciones

El orden completo de la génesis está en §9. Todo azar sale del generador por contador `H(semilla, paso, id_placa, propósito, índice)`.

### 6.1 Estado inicial (t = −T_deriva)

1. **Placas:** `N_p0` placas como regiones de Voronoi esféricas a partir de puntos sembrados, con distancia deformada por ruido para que los bordes no sean rectos.
2. **Cratones:** `N_cratones` manchas de corteza continental (ruido deformado), con área total `f_cont · A_planeta`, repartidas entre placas. Espesor `H_0 ± σ_H0`.
3. **Resto oceánico:** `H = H_oc`, `edad_oc` aleatoria en [0, 150] Myr. Es arbitraria a propósito: con `T_deriva = 500` Myr, todo ese fondo se recicla por subducción antes del final.
4. **Velocidades iniciales:** se calculan con §6.2.

### 6.2 Dinámica: velocidad de cada placa por equilibrio de fuerzas (cada paso)

En la Tierra, las placas se mueven sobre todo **tiradas por su borde que subduce**. Las placas con mucha fosa son las rápidas, como la Pacífica, y las que llevan mucho continente son lentas (Forsyth y Uyeda, 1975). Se modela así:

**Fuerzas en los bordes** (por km de borde, tangentes a la superficie; `n̂` es la normal al borde que apunta hacia fuera de la placa):
```
Tirón de la placa (en sus bordes donde subduce):
    F_sp = f_sp · min( √(edad_oc / t_sat) , 1 ) · n̂            (hacia la fosa; más fuerte si la placa es vieja y densa)
Empuje de dorsal (en sus bordes divergentes):
    F_rp = − f_rp · n̂                                           (alejándola de la dorsal)
Resistencia de colisión (en sus bordes de choque continental):
    F_col = − c_col · max( 0 , v_conv ) · n̂                     (v_conv = convergencia del paso anterior)
```

**Rozamiento basal** (repartido por toda la placa, proporcional a la velocidad, mayor bajo continente):
```
f_b(x) = − c_b · (1 + κ_cont · [x es continental]) · v(x)
```

**Equilibrio de momentos** (pares de fuerzas respecto al centro del planeta). Para cada placa:
```
M_p · ω_p = Σ_bordes ∫ x̂ × F dl                 (suma en orden fijo de celdas de borde: determinista)
M_p = c_b · R · ∫_placa (1 + κ_cont·[cont]) · ( I − x̂ x̂ᵀ ) dA          (matriz 3×3)
ω_p = M_p⁻¹ · Σ momentos
```
- `I − x̂ x̂ᵀ` es la proyección sobre el plano tangente: solo cuenta el movimiento a lo largo de la superficie.
- **Marco sin rotación neta:** se resta a todas las placas la rotación media de la litosfera ponderada por área, como en los modelos geodésicos de la Tierra.
- **Calibración:** `c_b` se ajusta una sola vez (al inicio y, si hace falta, cada 50 Myr) para que la velocidad media cuadrática de la superficie sea `v_obj` = 5 cm/año. Las fuerzas absolutas del manto no se pueden medir; la velocidad de las placas sí.

### 6.3 Movimiento y resolución de huecos y solapes (cada paso)

**Mover:** `Rotacion_p ← Rot(ω_p · Δt_tect) · Rotacion_p`.

Después, para cada celda de la malla del mundo se cuenta cuántas placas tienen corteza en ella:

**0 placas (hueco) → borde divergente: nace corteza oceánica.**
```
tipo = OCEANICA ; H = H_oc ; edad_oc = 0
se asigna a la placa más cercana de las que se separan (empate: id menor)
```
Así aparecen las dorsales y la edad del fondo crece de forma simétrica desde ellas.

**2 o más placas (solape) → borde convergente.** Para cada par, se decide quién se hunde:
| Par | Resultado |
|---|---|
| oceánica + continental | la oceánica subduce (se elimina de su placa) |
| oceánica + oceánica | subduce la más vieja (más fría y densa); empate: id mayor |
| continental + continental | **colisión**: ninguna subduce (§6.4) |

Con subducción activa, el borde se marca como SUBDUCCION. La placa que subduce recibe el tirón `F_sp`, y en la que cabalga crece el arco:
```
en la franja de la placa que cabalga a distancia d_arco ± w_arco/2 de la fosa:
    ΔH = q_arco · Δt_tect / w_arco        (km por paso; mismo reparto a lo largo del arco)
    dH_dt = q_arco / w_arco ;  edad_orog = 0
si la corteza es oceánica y H supera H_cont_min → pasa a CONTINENTAL (los continentes crecen por acreción de arcos)
```
Comprobación: 40 km²/Myr durante 50 Myr, repartidos en 150 km, engrosan ~13 km. Sumado al acortamiento, sale una corteza andina de ~50-70 km.

### 6.4 Colisión continental y mesetas (conservando la corteza)

Cuando dos cortezas continentales se solapan, el volumen solapado de la placa que llega se transfiere a la otra como engrosamiento, con una eficiencia `η_col`. El resto se pierde hacia el manto (subducción de la corteza inferior, extrusión lateral).
```
Volumen a añadir en el paso (por km de borde):  A_x = η_col · Σ ( H · área solapada )
Se reparte en la placa que recibe, desde la sutura hacia dentro:
    1. engrosar las celdas más cercanas a la sutura
    2. ninguna celda puede pasar de H_max: el exceso pasa a la siguiente celda hacia el interior
       (colapso gravitatorio: la corteza demasiado gruesa se desparrama lateralmente)
    3. anchura mínima de la franja: W_min_orog
edad_orog = 0 ;  dH_dt = ΔH / Δt_tect en las celdas engrosadas
```
**La meseta sale sola:** cuando el engrosamiento llega al tope `H_max`, la franja se ensancha en vez de crecer en altura.

Comprobación con India-Asia: acortamiento de ~2 500 km en 50 Myr con `H_0` = 35 km y `η_col` = 0,35. El ancho de la franja engrosada a `H_max` sale de ~1 000 km, y su altura isostática de ~5 km (§6.6), como el Tíbet.

**Sutura (fusión de placas):** si la convergencia entre dos placas en colisión es menor que `v_sutura` durante más de `t_sutura`, se funden en una sola. La línea de choque se registra como sutura con su edad.

### 6.5 Rifts, placas pequeñas y equilibrio del número de placas (cada paso)

**Rift continental.** Un continente grande aísla el manto de debajo, que se calienta y acaba rompiéndolo. Probabilidad de rift por paso para la placa p:
```
p_rift = ( Δt_tect / τ_rift ) · ( A_cont,p / A_cont,total )
```
Un supercontinente que contenga toda la corteza continental se rompe de media en ~150 Myr. En la Tierra, Pangea duró del orden de 130-150 Myr ya formada.

Cuando ocurre:
- Línea de rift: arco de círculo máximo que pasa por el centroide continental de la placa, con orientación sorteada y deformada por ruido.
- Se crean dos placas hijas con ids deterministas. A lo largo de la línea, la corteza se adelgaza (`H ← f_rift · H`) en una franja de `w_margen`. Ahí nacerán los márgenes pasivos con su plataforma.
- Las hijas reciben una velocidad relativa inicial de apertura `v_rift`, perpendicular a la línea. Después manda la dinámica de §6.2.

**Placas pequeñas:** una placa con área menor que `A_min` se absorbe en la vecina con la que comparte más borde.

**Mínimo de placas:** si `N_placas < N_min`, se abre una dorsal nueva en la placa oceánica más grande (salto de dorsal), dividiéndola en dos.

### 6.6 Erosión tectónica, sedimento y envejecimiento (cada paso)

**Envejecimiento:** `edad_oc += Δt_tect` y `edad_orog += Δt_tect`.

**Las cordilleras se gastan.** La erosión quita roca de arriba y el rebote isostático sube la raíz, así que, a escala geológica, lo que decae es el exceso de espesor:
```
si H > H_0:   H ← H_0 + (H − H_0) · e^( −Δt_tect / τ_orog )
dH_dt (de erosión) no cuenta como levantamiento
```
Comprobación con `τ_orog` = 150 Myr: una cordillera de choque al tope (`H` = 65 km, exceso de 30 km) baja en 300 Myr a un exceso de ~4 km de espesor, es decir, ~0,6 km sobre la llanura de alrededor. Encaja con los Apalaches y los Urales.

**El sedimento se conserva:** el volumen de corteza erosionado en cada paso se deposita en las celdas de margen continental de la misma placa más cercanas (dentro de `w_margen` de la costa), engrosándolas hasta llevarlas como mucho a la profundidad de plataforma (`H` tal que `z` ≈ −130 m). El sobrante sale al fondo oceánico y no se sigue: los sedimentos abisales son ampliación futura.

### 6.7 Altura isostática y nivel del mar (al final de la deriva)

**Corteza continental (isostasia de Airy con relación empírica terrestre):**
```
z_cont = ( H − H_nivel ) · ( 1 − ρ_c / ρ_m )            (km → m)
```
Con `1 − 2,8/3,3 = 0,152`:
| H | z |
|---|---|
| 25 km (margen estirado) | −1,06 km |
| 35 km (continente estable) | +0,45 km |
| 45 km | +1,97 km |
| 65 km (meseta) | +5,0 km (Tíbet ~4,5-5 km) |

`H_nivel` (32 km) es una relación empírica de la Tierra (espesor ↔ altura), no una medida directa. Es el ancla que se recalibra si la curva de alturas no cuadra (ver §10).

**Corteza oceánica (enfriamiento con la edad, Parsons y Sclater 1977):**
```
d(t) = d_r + c_hs · √t                                  si t ≤ t_x     (enfriamiento del semiespacio)
d(t) = d_∞ − A_pl · e^( −t / τ_pl )                     si t > t_x     (modelo de placa)
z_oc = − d(edad_oc)
```
Las dos fórmulas coinciden a 20 Myr con 8 m de diferencia, así que el cambio es continuo. Resultados: dorsal a 2 500 m, fondo de 40 Myr a ~4 700 m, fondo de 150 Myr a ~6 100 m.

**Fosas** (en el lado de la placa que subduce de los bordes SUBDUCCION activos al final):
```
z_oc ← z_oc − D_fosa · exp( −( dist_fosa / w_fosa )² )
```

**Márgenes:** la transición continente-océano sale sola del adelgazamiento de los márgenes (rifts, §6.5) y del sedimento (§6.6). La plataforma es la corteza continental entre ~0 y −200 m.

**Nivel del mar por volumen de agua** (la tierra emergida no se impone):
```
encontrar ζ tal que   Σ_celdas max(0, ζ − z) · A_celda = GEL · A_planeta        (bisección; determinista)
h = z − ζ
```
Con los valores terrestres debería salir ζ ≈ 0, tierra emergida ≈ 29 % y profundidad media ≈ 3 700 m. Son comprobaciones, no objetivos forzados.

### 6.8 Detalle fino según el contexto tectónico

El ruido actual pasa a ser solo **detalle**, con amplitud según la tectónica:
```
rugosidad r(x) = clamp( (H − H_0) / (H_max − H_0) , 0 , 1 )        (0 en llanuras, 1 en cordilleras al tope)
continental:  h += ( a_llano + (a_orog − a_llano) · r ) · ruido_fractal(x, semilla)
oceánica:     h += a_abisal · ruido_fractal(x, semilla)               (colinas abisales)
```
- El ruido tiene media cero, así que no desplaza la curva de alturas.
- Se retira la curva `h = lf³ · h_max` (`land_exp`).
- Para las celdas finas: campos tectónicos interpolados de la malla tectónica, más este ruido evaluado en su posición fina, más el acondicionamiento hidrológico que ya existe. Es una función pura.

### 6.9 Cambios en la erosión (prerrodaje y simulación)

**a) Levantamiento tectónico en el prerrodaje.** Las zonas activas al final de la deriva (arcos y choques) siguen subiendo durante el millón de años de erosión:
```
U(x) = ( 1 − ρ_c / ρ_m ) · dH_dt(x)                      (km/Myr = mm/año)
∂h/∂t = U − K·A^m·S^n + κ·∇²h + rebote
```
El equilibrio entre levantamiento y erosión es lo que da a las cordilleras activas sus laderas reales: la pendiente se ajusta hasta que la erosión iguala al levantamiento.

**b) Rebote isostático** (se añade al prerrodaje y a la erosión de la simulación). Cuando la erosión quita roca, la corteza descargada sube, igual que un barco al descargarlo. Por eso las montañas tardan cientos de millones de años en desaparecer:
```
E_neto(x) = erosión − depósito en el paso (espesor)
Ē(x) = media de E_neto ponderada por área en un radio r_flex           (la litosfera es rígida y reparte la carga)
Δh_rebote(x) = ( ρ_c / ρ_m ) · Ē(x)
```
- Se apunta en una partida propia de las cuentas ("rebote isostático"), separada del balance de sedimento, que sigue cerrando como hasta ahora.
- Efecto esperado: la bajada de la altura media de la tierra en el prerrodaje (hoy 85 m) se reduce a ~15 %, porque el ~85 % de lo erosionado se compensa con rebote.

**c) Nivel del mar:** se recalcula con §6.7 al terminar el prerrodaje, porque el sedimento llevado al mar cambia el volumen de las cuencas.

### 6.10 Durante la simulación
Sin tectónica (ver §2). Los campos `Tectonica` y `Borde` quedan como datos estáticos. Servirán más adelante para volcanes, terremotos y recursos minerales asociados a bordes de placa (ampliación futura).

---

## 7. Eventos discretos (historia geológica, registrados durante la génesis)

| Evento | Datos |
|---|---|
| **Rift** | Myr, placa madre, placas hijas, longitud de la línea, área continental partida |
| **Inicio de subducción** en un borde | Myr, placas, longitud |
| **Inicio de colisión continental** | Myr, placas, longitud del frente |
| **Sutura** (fusión de placas) | Myr, placas fusionadas, línea de sutura |
| **Absorción** de una placa pequeña | Myr, placa absorbida, placa receptora |
| **Salto de dorsal** (placa oceánica dividida) | Myr, placa, placas nuevas |
| **Acreción de arco** (oceánica → continental) | Myr, área |
| **Supercontinente formado / roto** | Myr, cuando la mayor masa continental conectada pasa por encima o por debajo del 70 % de toda la corteza continental |

## 8. Métricas agregadas

**Cada Myr de la deriva:** nº de placas; velocidad media cuadrática y por placa; longitud total de dorsales, fosas y frentes de colisión; edad media y máxima del fondo oceánico; volumen de corteza continental; fracción de la mayor masa continental (índice de supercontinente).

**Al final de la génesis (diagnóstico del planeta, comparado con la referencia):**
- tierra emergida;
- curva de alturas de tierra y océano por franjas;
- altura media y mediana de la tierra;
- profundidad media del océano;
- distribución de áreas del fondo oceánico por edad;
- pendientes a 25 km en zonas de montaña;
- anchura de plataformas;
- lluvia en barlovento frente a llanura (con `k_oro` = 40);
- exponente de la ley de Hack de los ríos.

**Cuentas de conservación:**
- superficie total de corteza = superficie del planeta en cada paso (exacto);
- volumen de corteza continental: Δ = + arcos − pérdidas por colisión (1 − η_col) ± erosión-sedimento, con el error de remuestreo por debajo de una tolerancia;
- volumen de agua = `GEL · A_planeta`.

## 9. Orden de ejecución (génesis completa)

```
G1. Estado inicial (§6.1)
G2. Bucle de deriva, T_deriva / Δt_tect pasos; en cada paso:
     a. velocidades por equilibrio de fuerzas (§6.2), marco sin rotación neta, calibración de c_b si toca
     b. mover placas (§6.3)
     c. huecos → corteza oceánica nueva (§6.3)
     d. solapes → subducción y arcos (§6.3) / colisión y mesetas (§6.4)
     e. erosión tectónica, sedimento a márgenes, envejecimiento (§6.6)
     f. eventos de placas: suturas, rifts, absorciones, saltos de dorsal (§6.4, §6.5)
     g. métricas y eventos del paso
G3. Altura isostática: continental, oceánica, fosas (§6.7)
G4. Nivel del mar por volumen de agua (§6.7)
G5. Detalle fino por contexto tectónico (§6.8)
G6. Acondicionamiento hidrológico + prerrodaje de erosión con U y rebote isostático (§6.9)
G7. Nivel del mar recalculado (§6.9c)
G8. Resto de la génesis existente: suelo, ríos, lagos, medias climáticas, forraje
```
- **Caché de la génesis:** G1-G7 dependen solo de la semilla, de los parámetros y del código, así que entran en la caché que ya existe.
- **Durante la simulación:** nada de esto corre.

## 10. Validación y determinismo

### Datos de referencia (a calcular por Claude Code y guardar en el repositorio)
Script en `herramientas/analisis/`. Se guardan solo **tablas resumen** con nombre, versión y fecha del conjunto de datos (no los datos brutos) en `config/referencias/tierra_relieve.toml`, **calculadas a las mismas resoluciones que el planeta** (~78 km y ~25 km, por promedio ponderado por área). Las pendientes y las alturas dependen de la resolución, y solo así la comparación es justa.

| Referencia | Conjunto de datos | Qué se extrae |
|---|---|---|
| Relieve y batimetría | ETOPO 2022 (NOAA NCEI) | tierra emergida, curva de alturas por franjas, media y mediana de la tierra (ETOPO1 dio ~797 m y ~420 m), profundidad media del océano, pendientes a 25 km en montaña, anchura de plataformas |
| Edad del fondo oceánico | Malla de edades de Müller et al. (EarthByte) | edad máxima (~180 Myr) y distribución de áreas por edad |
| Velocidad de placas | MORVEL (DeMets et al., 2010) o equivalente | velocidad media y rango (~1-10 cm/año) |
| Tamaño de placas | Modelo de placas de Bird (2003) | distribución de áreas (pocas grandes y muchas pequeñas) |

### Objetivos (bandas)
| Métrica | Referencia terrestre aproximada | Banda aceptable inicial |
|---|---|---|
| Tierra emergida | ~29 % | 25-33 % |
| Altura media de la tierra | ~800 m | 650-950 m |
| Altura mediana de la tierra | ~420 m | 300-550 m |
| Profundidad media del océano | ~3 700 m | 3 400-4 000 m |
| Curva de alturas | bimodal (máximos cerca de 0-1 km y de −4/−5 km) | forma bimodal |
| Fondo oceánico más viejo | ~180 Myr | 120-250 Myr |
| Velocidad media de placas | ~4-5 cm/año | por calibración |
| Mesetas de colisión (si hay colisión activa) | 4,5-5 km | 4-5,5 km |
| Lluvia en barlovento con `k_oro` = 40 | ×2-3 | ×2-3 **sin tocar `k_oro`** |
| Ley de Hack (longitud ∝ área^h) | h ≈ 0,5-0,6 | 0,45-0,65 |

Las bandas se ajustarán con las tablas reales de ETOPO cuando estén calculadas.

### Qué se puede recalibrar si algo no cuadra (en este orden)
1. `H_nivel`: relación empírica, si falla la altura media o la mediana de la tierra.
2. `c_b`: por definición calibrado a la velocidad de las placas.
3. `η_col`, `τ_orog`, `q_arco`: proceden de estimaciones con mucha dispersión en la literatura. Se pueden mover dentro de sus rangos reales, documentándolo.

**No se tocan para cuadrar resultados:** densidades, `H_oc`, la curva edad-profundidad ni `GEL`. Si con ellos no cuadra, el fallo está en la mecánica y hay que revisarla (CLAUDE §2.8).

### Pruebas de comportamiento
- **Un solo continente:** se rompe en algún momento, se abre un océano con dorsal central y edades simétricas, y aparecen márgenes pasivos con plataforma.
- **Dos continentes en convergencia:** el océano entre ellos subduce, aparece un arco, chocan, se forma una cordillera con meseta si el choque es grande, se suturan y la cordillera envejece y baja.
- **Planeta sin corteza continental** (`f_cont` = 0): todo océano, con profundidad según la edad.
- **Ciclo de supercontinente:** en corridas largas (≥ 1 000 Myr) el índice de supercontinente oscila con periodos del orden de cientos de Myr.
- **Correlación de Forsyth y Uyeda:** las placas con más fosa son más rápidas y las más continentales, más lentas.

### Determinismo
- Generador por contador con claves (semilla, paso, id de placa, propósito). Ids de placas hijas: `H(id_madre, paso, índice)`.
- Sumas de momentos y volúmenes en orden fijo de celdas, para obtener resultados idénticos bit a bit.
- Prueba: dos génesis con la misma semilla deben dar salidas idénticas, incluidos los eventos.
- No-regresión: este incremento **sustituye** el relieve, así que la salida cambia a propósito. Se fija una nueva línea base y se actualizan las bandas de los escenarios de referencia.

### Efecto en cascada (esperado)
Cambian los ríos, los lagos, la lluvia (ahora con laderas reales), los biomas, el forraje y la capacidad de carga. Como pedía la propuesta, en el mismo incremento conviene:
- comprobar `k_oro` = 40;
- calibrar la lluvia de base con datos reales;
- retirar el apaño `c_P`;
- comparar la proporción de biomas con la de la Tierra.

## 11. Preguntas abiertas

- **P1 — Fase del ciclo de supercontinente.** Según la semilla, el planeta puede acabar en fase dispersa (como la Tierra actual) o con un supercontinente (como hace 250 Myr). Las dos son reales. ¿Se acepta lo que salga, o se elige `T_deriva` por semilla para terminar en fase dispersa? Recomendación: aceptar lo que salga, que es lo realista, y registrar la fase como dato.
- **P2 — Puntos calientes** (Hawái, Islandia, mesetas volcánicas): primera ampliación natural.
- **P3 — Nivel del mar en el tiempo geológico:** ahora solo se calcula al final. Calcularlo en cada paso permitiría registrar las transgresiones (mares que inundan continentes), pero no cambia el relieve final.
- **P4 — Resolución tectónica:** si el coste de 500 pasos a ~78 km fuera alto, se puede usar una malla de ~150 km para la deriva. La geometría de los bordes se refina después con las distancias.
- **P5 — Clima durante la deriva:** en la realidad la erosión depende del clima (más lluvia, más erosión). Aquí `τ_orog` es uniforme. Acoplarlo es una ampliación futura.

---

### Referencias citadas
- Parsons, B. y Sclater, J. G. (1977). An analysis of the variation of ocean floor bathymetry and heat flow with age. *JGR* 82(5).
- Forsyth, D. y Uyeda, S. (1975). On the relative importance of the driving forces of plate motion. *Geophys. J. R. Astr. Soc.* 43.
- Cortial, Y., Peytavie, A., Galin, É. y Guérin, É. (2019). Procedural Tectonic Planets. *Computer Graphics Forum* 38(2).
- Amante, C. y Eakins, B. W. (2009). ETOPO1 Global Relief Model (NOAA); ETOPO 2022 (NOAA NCEI).
- Müller, R. D. et al. Malla de edades del fondo oceánico (EarthByte).
- DeMets, C., Gordon, R. G. y Argus, D. F. (2010). Geologically current plate motions (MORVEL). *GJI* 181.
- Bird, P. (2003). An updated digital model of plate boundaries. *G³* 4(3).
