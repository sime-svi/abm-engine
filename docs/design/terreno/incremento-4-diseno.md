# Incremento 4 del terreno: relieve tectónico (diseño técnico y resultados)

**Estado:** primera implementación (2026-11-05), con interruptor `worldgen.relieve = "tectonico" | "ruido"`
mientras se mide, más la adenda de balance v2 (§9). **Diseño del responsable:** `diseno_relieve_tectonico.md`
y `adenda_balance_corteza.md` (mandan sobre este documento).
**Código:** `crates/abm-terreno/src/tectonica/` (`malla`, `deriva`, `isostasia`, `historia`, `mod`), más
`statics.rs` (G5), `erosion.rs` y `lod.rs` (G6), `worldgen.rs`, `cache.rs` (`ABMGEN04`) y `metrics.rs`
(esquema v6; v7 con el balance de la corteza). **Referencia de la Tierra:** `config/referencias/tierra_relieve.toml`
(`herramientas/analisis/referencia_tierra.py`), `tierra_bordes.toml` (`bordes_bird.py`) y `tierra_lluvia.toml`
(`lluvia_observada.py`).

---

## 1. Decisiones del responsable (2026-11-05)

| | Decisión |
|---|---|
| R1 | La tectónica corre en el **nivel 2 del cubo del terreno** (~52 km con `n0 = 48`, 221 184 celdas): los rasgos de 150 km tienen ~3 celdas. Si el coste lo impidiera, avisar antes de bajar de resolución (no hizo falta: ~35 s por deriva). |
| R2 | **Choque de continentes:** va abajo la placa que subducía antes (la India bajo Asia); si ninguna subducía, la de corteza más fina en el contacto. La de arriba conserva la superficie; la corteza solapada de la de abajo la engruesa (η_col), y el resto se pierde al manto. |
| R3 | **Arranque:** empujón al azar con velocidades del rango real (1–10 cm/año por placa); un rodaje de 50 Myr (`t_rodaje_myr`) no cuenta para la validación. |
| R4 | **Arcos sobre fondo oceánico:** la flotación por espesor se **suma** a la profundidad por edad. |
| R5 | **Ruido fino:** empieza por debajo de la malla tectónica y su amplitud continúa sin saltos el espectro del relieve real; se valida con las pendientes de ETOPO. |
| R6 | **Referencia:** la roca sin hielo con el rebote isostático completo (Paxman, Austermann y Hollyday, 2022) y ETOPO 2022 fuera de los casquetes. El agua del planeta es la del océano actual mientras no haya casquetes. «Casquetes polares» va a los pendientes. |
| R7 | **Corteza continental:** no se calibra η_col para llegar a 0,40; f_cont es un resultado a validar. Llegará una adenda con los sumideros reales que faltan (erosión por subducción y sedimento subducido) y η_col del choque India–Asia. |
| R8 | Ante cualquier hueco: la opción más fiel a la realidad; lo que falte para el realismo, a pendientes, en orden. |
| — | CLAUDE §1: la geopolítica y los conflictos **se quedan** en el alcance. |

## 2. Correcciones de realismo (avisadas al responsable)

- **C1. Isostasia con agua encima.** La fórmula del §6.7, `z = (H − H_nivel)·(1 − ρ_c/ρ_m)`, vale al aire. Una columna sumergida tiene agua encima, no aire: sube `(ρ_m − ρ)/(ρ_m − ρ_w)` por km de espesor (0,220 la continental, 0,176 la oceánica) en vez de `(ρ_m − ρ)/ρ_m` (0,152 y 0,121). `isostasia::con_flotacion` aplica cada factor a su tramo y es continua al asomar. Un margen de 25 km queda a −1,54 km (el documento daba −1,06).
- **C2. El sedimento no es corteza.** Va en una capa propia (`sed`) de densidad `ρ_s = 2 400 kg/m³`, conservando la masa: la roca erosionada (ρ_c) se deposita como sedimento (ρ_s) y flota con `(ρ_m − ρ_s)/(ρ_m − ρ_w)`. Por tonelada, levanta el fondo ~2 veces más que la «corteza» del documento. Cuando un margen se engruesa (arco o choque), su sedimento pasa a corteza (metamorfismo), con su masa.
- **C3. Velocidad real de las placas:** `v_obj = 4,39 cm/año`, la media cuadrática de toda la litosfera en el marco sin rotación neta (NNR-MORVEL56, Argus et al., 2011). El documento ponía 5.

## 3. Interpretaciones y decisiones técnicas

- **I1. Representación (§3, nota).** Cada placa guarda su corteza en una copia de la malla en su propio sistema (`mapa`: celda local → muestra). Gira con un cuaternión renormalizado en cada paso. El mundo se «rasteriza» cada paso: para cada celda del mundo se gira su centro al sistema de cada placa candidata (casquete envolvente) y se mira si allí hay corteza. Una placa rígida que gira 100 Myr conserva su forma (menos del 1 % de celdas de borde distintas; test `una_placa_rigida_que_gira_no_se_deforma`).
- **I2. Mapas al día sin remuestrear lo que no cambia.** Una celda local sigue si representa alguna celda del mundo de su placa. Si ninguna la representa este paso («oculta», por la desalineación de las dos rejillas), también sigue: cuando una celda del mundo la vuelva a ver, decide el solape. Borrarlas roía los continentes por los bordes (corregido: la corteza «subducida» sin choque pasó de 2·10⁹ km³ a 0). Las celdas del mundo sin placa reciben corteza oceánica nueva, y se tapan los agujeros del mapa local alrededor. Los cambios calculados en el mundo (arcos, choques, sedimento) llegan a todas las celdas locales cuyo centro cae en la celda del mundo.
- **I3. Bordes.** Celdas del mundo con alguna vecina de otra placa. La normal y la longitud salen del gradiente por mínimos cuadrados del indicador de la otra placa sobre las 8 vecinas: `L = 2·A·|∇χ|`, exacto en bordes rectos de cualquier orientación.
- **I4. Tipo de borde.** Convergencia `v_n = (v_p − v_q)·n̂`. Es transformante si `|v_n| < 0,26·|v_rel|` (a menos de 15° del desgarre puro) o si la velocidad relativa es menor de 0,05 cm/año; si no, divergente o convergente, y entonces subducción (la oceánica, o la del par) o colisión (las dos continentales). La velocidad incluye el empuje de apertura de un rift recién hecho, para que la dorsal empuje desde el primer paso.
- **I5. Fuerzas** (§6.2) en las celdas de borde, con la clasificación del paso anterior. **Resistencia de colisión implícita:** el documento usa la convergencia del paso anterior (`F_col = −c_col·v_conv`). Con la fuerza que pide India–Asia (frenar a 1/3), ese esquema explícito es inestable: cada paso invierte y amplifica la velocidad del anterior. Se resuelve igual de física (resistencia proporcional a la convergencia), pero con la velocidad del propio paso, en un sistema lineal acoplado entre las placas en contacto (`3N × 3N`, eliminación gaussiana). La resistencia es `c_col·c_b·v_conv` por km de frente: el rozamiento basal de `c_col` km² de placa.
- **I6. Huecos** (§6.3): cada celda sin placa va a la placa de la celda cubierta más cercana. El empate «id menor» del documento hacía la expansión asimétrica: con una dorsal de una celda de ancho casi siempre hay empate, y toda la corteza nueva iba a la misma placa. Se rompe con un sorteo determinista sin sesgo (test `un_continente_se_parte_y_abre_un_oceano_simetrico`).
- **I7. Polaridad por par.** «Subduce la más vieja» decidido celda a celda dejaba bordes moteados de varias celdas de ancho, porque la edad varía de celda en celda. Eso alargaba los bordes 2–3 veces y disparaba las fuerzas. En la Tierra la polaridad es de todo el tramo de fosa. La polaridad entre dos cortezas oceánicas, y los papeles de un choque (R2), se deciden por par de placas en el primer contacto, con la media de cada lado en los solapes del paso. Se olvidan tras 20 Myr sin contacto.
- **I8. Arcos:** cada tramo de fosa reparte `q_arco·L·Δt` km³ en las celdas de su placa (la que cabalga) a `d_arco ± w_arco/2`, medidas sin salir de la placa (Dijkstra). La corteza oceánica que pasa de `H_cont_min` se vuelve continental con su volumen entero.
- **I9. Choques** (§6.4, R2): la corteza de la de abajo que se consume en una celda engruesa la placa de arriba desde esa celda hacia dentro. Se reparte uniforme en `W_min_orog`, con tope `H_max`, y el exceso sigue hacia dentro en orden de distancia (la meseta se ensancha). **Solo la corteza continental de la de arriba se apila;** antes también se engrosaba su fondo oceánico, que se volvía continental a manchas. Lo que no cabe se apunta (`choque_sin_sitio`).
- **I10. Suturas:** si la convergencia media del frente de un par es menor que `v_sutura` durante `t_sutura`, la mayor absorbe a la menor (su corteza pasa al sistema de la mayor, una vez).
- **I11. Rifts** (§6.5): círculo máximo por el centroide continental, con orientación sorteada y deformado por ruido. La corteza continental a menos de `w_margen` **a cada lado** se adelgaza (`H ← f_rift·H`). Las hijas comparten el sistema de la madre (sin remuestreo) y abren a `v_rift` en el centro durante el primer paso. Saltos de dorsal: igual, en la placa con más fondo oceánico y sin adelgazar.
- **I12. Erosión tectónica y sedimento** (§6.6): `H_0` global. El sedimento va al margen más cercano de la misma placa (continental, más hondo que −130 m y a menos de `w_margen` de la costa), hasta la plataforma; lo que sobra pasa a los demás márgenes de la placa, y el resto sale al fondo oceánico (se pierde, como dice el documento). La costa usa el nivel del mar del paso, calculado por volumen.
- **I13. `dH/dt`** es el engrosamiento del paso (arcos y choques); se pone a 0 al empezar cada paso.
- **I14. Fosas:** distancia firmada al borde de subducción (positiva en la placa que subduce), medida sin salir de cada placa; la fosa se evalúa con la distancia interpolada, así que es estrecha también en las celdas finas.
- **I15. Interpolación a las celdas del terreno:** núcleo de Wendland de soporte 1,2 lados de celda sobre el vecindario 3×3 de la celda que contiene el punto. Es continuo (toda celda a menos del soporte está en el vecindario) y se aplica a la altura de base, la distancia a la fosa, la σ del detalle y `dH/dt`. La altura de una celda del terreno es función pura de su posición, como pide el LOD.
- **I16. Ruido de detalle** (G5, R5): primera onda de 2 lados de la malla tectónica (104 km) y una octava más por cada nivel del LOD más fino que ella, con razón `p` entre octavas. En cada nivel solo entran las ondas de al menos dos lados de celda, así que la altura de una celda es la media de su zona, como en ETOPO. La σ depende de la altura isostática **respecto al dato**: es función del espesor de la corteza, como la rugosidad del §6.8, y no se deja engañar por un nivel del mar desplazado.
- **I17. G6 acondicionamiento:** el ruido de detalle tiene la rugosidad real pero no la red de drenaje. Antes del prerrodaje se rellenan, con material del borde y balance exacto, las hondonadas que no existen en la base tectónica sin ruido; las cuencas de la tectónica se conservan. Prerrodaje: levantamiento `U = 0,152·dH/dt` y rebote `(ρ_c/ρ_m)·Ē` en `r_flex` sobre la tierra, como partidas propias (`uplift_m3`, `rebound_m3`) fuera del balance de sedimento. El rebote sigue durante la simulación. G7: el nivel del mar se recalcula en la malla del planeta y se informa (`nivel_mar_tras_prerrodaje_m`), sin aplicarlo.

## 4. Calibraciones (tipo b, contra observaciones)

- **`c_b`**: a la velocidad media cuadrática real (4,39 cm/año) en el primer paso con fuerzas y cada 50 Myr. Media tras el rodaje: 4,36.
- **`c_col` = 90 000 km** (antes 5 000, puesto a ojo). Escenario India–Asia en la malla de 52 km: el océano viejo subduce bajo Asia y tira de la India a 14,4 cm/año (la India real iba a 15–18). Al chocar (frente de ~2 200 km), la convergencia baja a 4,8–5,2 cm/año, 1/3 como en India–Asia (de ~15 a ~5). Con `c_col = 0`, Asia se traga la India entera en 24 Myr sin frenarla. El frenazo tarda 2–4 Myr, no 10–20, porque aquí el frente se forma de golpe.
- **Ruido de detalle:** `p = 0,77`, la razón de σ entre los niveles 1→2 y 2→3 de ETOPO en las clases de interior. La σ por altura se toma de la de ETOPO entre 52 y 26 km por clase, dividida por 0,8, la fracción que captura el ruido. En las clases de interior el planeta queda a ±30 % de la Tierra (llanuras a 500–1 000 m: 155 frente a 136 m; abisal: 173 frente a 165 m).

## 5. Resultados (planeta por defecto, semilla 42; la Tierra: roca rebotada a 26 km)

| | Planeta | Tierra | Banda del diseño |
|---|---|---|---|
| Tierra emergida | 33,1 % | 28,4 % | 25–33 % |
| Altura media / mediana de la tierra | 1 459 / 1 490 m | 671 / 404 m | 650–950 / 300–550 m |
| Tierra por debajo de 200 m / de 1 000 a 2 000 m | 0,9 / 81 % | 28 / 16 % | — |
| Profundidad media del océano | 3 945 m | 3 667 m | 3 400–4 000 m |
| Océano de menos de 200 m (plataformas) | 0,1 % | 7,0 % | — |
| Anchura media de la plataforma | 0,6 km | 43 km | — |
| Pendiente a 25 km: tierra / sobre 2 000 m (mediana) | 0,014 (media) / 0,016 | 0,005 / 0,010 | — |
| `m_oro` medio en barlovento (k_oro = 40) | 1,52 | **1,36** | ×2–3 |
| Ley de Hack | 0,57 | 0,5–0,6 | 0,45–0,65 |
| Tierra en cuencas cerradas | 4,4 % | 10,4 % | — |
| Fondo oceánico: edad media / mediana / máxima | 104 / 60 / 627 Myr | 64 / 55 / 339 Myr | máx. 120–250 |
| Fondo de más de 200 Myr | 15 % | ~0,1 % | — |
| Velocidad de las placas | 4,36 cm/año | 4,39 | por calibración |
| Forsyth y Uyeda: velocidad–fosa / velocidad–continente | +0,46 / −0,55 | positiva / negativa | — |
| Placas al final / la mayor | 12 / 25 % | 7 grandes (81 %), 13 > 1 % / 20,5 % | — |
| Corteza continental | 0,40 → 0,334 | ~0,40 estable | resultado (R7) |
| Nivel del mar respecto al dato | −1 079 m | — | ~0 |

Cuentas de la corteza continental en 500 Myr (km³):
- añaden: arcos +3,2·10⁸ y acreción +0,6·10⁸;
- se pierden en los choques: −10,6·10⁸ al manto y −1,5·10⁸ sin sitio;
- se pierde sedimento al fondo oceánico: −6,4·10⁸ (de 6,7·10⁸ erosionados);
- adelgazan los rifts: −0,5·10⁸;
- y el remuestreo añade +3,8·10⁸ (5,3 %).

Coste: deriva ~35 s; génesis completa ~95 s; con la caché, un año de simulación ~20 s.

Cascada en el clima (frente al relieve de ruido):
- temperatura media 7,2 °C (antes 9,35);
- lluvia en tierra 559 mm (antes 687) con `c_P` = 1,99;
- `w < 0,5` en el 64 % de la tierra;
- PPN 551 (antes 704) y hielo 7,0 % del planeta (antes 3,9 %).

La tierra demasiado alta lo explica casi todo.

## 6. Hallazgos

1. **El nivel del mar queda 1 079 m bajo el dato, y por eso la tierra está ~800 m alta.**
   - Con la curva de Parsons y Sclater y una distribución de edades como la de la Tierra, las cuencas tienen 4,9 km de media y caben más que todo el océano actual: «ζ ≈ 0» (§6.7) no puede salir ni en la Tierra.
   - A la Tierra real la levantan cosas que el modelo aún no tiene: sedimento abisal, mesetas oceánicas y montes submarinos, y márgenes continentales sumergidos.
   - Además, sobrevive fondo de hasta 627 Myr (el 15 % del fondo tiene más de 200), porque falta el **inicio de subducción en márgenes viejos**: unos 200 m del desfase.
   - Recalibrar `H_nivel` lo «arreglaría» forzando un parámetro (§2.8), así que no se ha tocado.
2. **No hay plataformas continentales** (0,6 km frente a 43). Los cratones nacen con bordes cortados y quedan altos, así que no se inundan; solo los rifts hacen márgenes.
3. **El ×2–3 de lluvia en barlovento con `k_oro = 40` no lo da ni la Tierra:** su relieve real a 26 km da ×1,36 con la regla del modelo (pendiente de barlovento ~9 m/km). El realce real de la lluvia debe venir de escalas menores de 25 km o de otra física. Hay que revisar el objetivo o la escala de la lluvia orográfica, no el relieve.
4. **El ritmo de rifts del §6.5 es bajo:** `p_rift ∝ A_cont,p/A_cont,total` da un rift cada ~150 Myr en total, sea cual sea la fragmentación (3 en 500 Myr). La ruptura de Pangea tuvo ~6–10 rifts en 150 Myr.
5. **La corteza continental baja un 17 % en 500 Myr**, sobre todo por los choques (2,1 km³/año al manto) y por el sedimento que no cabe en los márgenes (1,3 km³/año). Pendiente de la adenda (R7).
6. **Remuestreo del 5 %** (resuelto el mismo día: conservación exacta, §7) del volumen continental en 500 Myr: copias de corteza al tapar agujeros de los mapas junto a márgenes con dorsal. Es del orden de lo que crean los arcos; hay que reducirlo antes de calibrar el balance.
7. **Cuencas cerradas:** con el relieve tectónico, el 4,4 % de la tierra está en cuencas cerradas (la Tierra, 10,4 % a 26 km). El planeta de ruido anterior (42 lagos) estaba más drenado que la Tierra.

## 7. Conservación exacta de la deriva (adenda de balance §0, 2026-11-05)

El responsable pidió arreglar el 5 % de remuestreo antes de evaluar el balance: es una fuga de conservación. Tenía tres causas, todas corregidas:
- **Tapar agujeros copiando corteza.** Al crear corteza nueva, los agujeros del mapa de la placa se tapaban con una copia de la corteza vecina, que podía ser continental. Ahora solo se tapan con corteza nueva cuando la celda del mundo era un hueco; nunca se copia.
- **Repartos por celda del mundo.** Arcos, choques y sedimento ponían un espesor en todas las muestras locales que caían en la celda, y el volumen repartido dependía de cuántas fueran. Ahora se reparte exactamente el volumen calculado, por áreas. Choques y sedimento van solo a la corteza continental: antes también engrosaban muestras oceánicas ocultas, que no cuentan en el inventario.
- **Fundir placas.** Antes se copiaba la corteza vista desde el mundo. Ahora cada muestra pasa a la celda libre más cercana de la otra placa, con su volumen.

El inventario de corteza continental de las placas (todas sus muestras) cierra a error de redondeo: 2·10⁻¹⁵ del volumen en 500 Myr (test `mover_las_placas_conserva_exactamente_la_corteza` y métrica `tect_remuestreo_rel` de la batería). La diferencia con el volumen que ve el mundo (muestras ocultas o repetidas) es del orden del 0,1 %.

**Efecto en el planeta** (antes → ahora):
- corteza continental final 0,334 → 0,304: las copias la inflaban;
- tierra emergida 33,1 → 29,8 % (la Tierra 28,4 %);
- altura media de la tierra 1 459 → 1 544 m;
- océano 3 945 → 3 753 m;
- nivel del mar −1 079 → −1 184 m.

## 8. Respuestas a las preguntas de la adenda de balance (§11)

1. **¿Hay erosión durante la deriva? Sí:** la erosión tectónica del diseño principal (§6.6). El exceso de espesor sobre `H_0` decae con `τ_orog` = 150 Myr, en toda corteza continental más gruesa que `H_0`, esté o no emergida.
   - Arranca ~1,3 AU, del orden de la exportación al mar de la adenda (≈1,8 AU).
   - El sedimento solo cabe en los márgenes continentales: en 500 Myr se depositaron 0,06 AU y se perdieron 1,28 AU como sumidero sin nombre. Según la adenda, ese material debe acabar en el mar como capa S (traslado interno, no pérdida).
   - La relajación es lenta: de 65 a 38 km tarda `150·ln 10` ≈ 345 Myr (objetivo 100–200). Con la delaminación de la adenda (por encima de 50 km, τ = 50 Myr) bajará; se valida con la prueba de relajación.
   - Su forma (exceso de espesor) no es la de la regla 6.7 (`κ·z^n`), que hace crecer la erosión con la altura, como en la Tierra.
2. **Tasa de los arcos hoy: 40 km³/Myr por km de fosa**, constante, sin depender de la velocidad de convergencia ni del tipo de arco. La adenda pide 96 (oceánico) y 64 (continental), escalados por `v/v̄`. **Ojo con la longitud de las fosas:** las del modelo miden ~100 000 km de media (la Tierra, ~43 500 km de márgenes convergentes oceánicos) y las dorsales ~101 500 km (la Tierra ~65 000). Con tasas por km, todos los flujos de fosa saldrían ~2,3 veces los reales. Los bordes son demasiado largos (ondulados por la deformación inicial y dentados por el crecimiento en las dorsales).
3. **¿Cuenta la corteza de arco sobre litosfera oceánica? No.** Mientras no pasa de `H_cont_min` (20 km) es oceánica y no cuenta; al pasar, la columna entera (con los 7 km basálticos) entra en el volumen continental. No hay marca de origen de arco: hay que crear `a_i` y contar solo lo añadido (`h_c`), como pide la adenda.

## 9. Adenda de balance v2: implementación (2026-11-05)

El responsable entregó la versión 2 de `adenda_balance_corteza.md`. Las tasas de los arcos y de la erosión por subducción pasan a ser **por km² de placa que se hunde**, el área consumida que da exacta el remuestreo. Se implementó entera, sin interruptor: es un cambio de mecanismo, y la referencia del comportamiento anterior es el commit anterior.

### 9.1 Estado nuevo de cada muestra

- `hc`: corteza de tipo continental (km). Toda la de un continente; en el fondo oceánico, solo lo que añaden los arcos y la acreción, sin los 7 km basálticos. `0 ≤ hc ≤ h`.
- `arco`: la marca `a_i`. La columna es de arco si toda su corteza continental la hizo una zona de subducción. Al unirse a corteza continental sin marca (un choque, un arco que crece sobre un continente) pasa a 0.
- `sed`: el sedimento, ya existente, ahora también sobre el fondo oceánico.
- `V_cont = Σ (hc + sed·ρ_s/ρ_c)·A` sobre todas las muestras. Que un arco oceánico pase de `H_cont_min` (pasa a `cont`) no cambia `V_cont`: ya contaba desde que se creó.

### 9.2 Orden del paso (§9 de la adenda) y decisiones técnicas

1. **Remuestreo con `A_sub`.** Cada muestra oceánica consumida en un solape apunta su área, su sedimento y su edad a la celda del mundo donde se perdió (el frente). Si lleva corteza continental (arco o prisma), esa parte va al choque (6.5). **Tramo de fosa:** el consumo de cada frente se agrupa en la celda de borde más cercana de la placa de arriba, sin salir de ella. Cuando la placa de abajo pierde dos filas en un paso, las dos son de la misma fosa.
2. Clasificación de los bordes (la del documento principal).
3. **Régimen** (6.2), por tramo. `v_j` es la convergencia normal del borde del tramo, y `S_fosa,j` el espesor medio del sedimento consumido allí (`margen_erosivo`).
4. **Arcos** (6.1), en la franja `d_arco ± w_arco/2`. Cada celda usa `p_arc,cont` si su corteza es continental sin marca de arco y `p_arc,oc` si no.
   - **Reparto (decisión técnica).** En un borde dentado, las celdas de franja se las quedan los tramos de los dientes; con un reparto por tramo, la mitad de la producción se quedaba sin franja. Por eso cada placa de arriba recibe exactamente `Σ A_sub` de sus tramos, repartido por su franja con el peso de la convergencia normal `v_n` del tramo de cada celda.
   - Un peso `A_sub/L` concentraba el arco hasta 223 km, porque las celdas de esquina tienen `L` casi nula. Se descartó.
5. **Erosión por subducción** (6.3): `p_SE·A_sub` desde la celda de la placa de arriba más cercana a la fosa hacia dentro, en orden de distancia y por índice.
   - "Junto a la fosa" es a menos de `d_arco + w_arco/2`. Lo que no encuentra corteza continental ahí no se erosiona: 0,15 AU, en fosas intraoceánicas sin arco.
   - Una celda que se queda sin corteza continental pasa a fondo oceánico, con la edad del fondo que se hunde en ese tramo (evento `retroceso_margen`).
   - Los tramos de muesca erosionan desde el tramo con franja más cercano.
6. **Sedimento en la fosa** (6.4). En un margen acrecionario, `f_acr·V_llega` forma el **prisma**: desde la celda del frente hacia dentro, por la región del tramo y sin pasar de `H_max`. El resto, y todo el de un margen erosivo, pasa al manto (`F_sed`).
7. **Choques** (6.5). Se usa `η_arco` si la corteza solapada tiene marca de arco y `η_col` si no.
   - La corteza de un continente solo engruesa corteza continental de la placa de arriba; la de un arco o un prisma, cualquier corteza.
   - Lo que no cabe en la cuenca de su frente pasa al resto de la placa de arriba, de lo más cercano a lo más lejano. Antes se perdía: 1,2 AU de "sin sitio" que solo eran un artefacto.
8. **Delaminación** (6.6): `tras_delaminar`, la forma exponencial exacta, en todas las muestras.
9. **Erosión** (§6.6 del principal, con 6.7). El exceso del espesor equivalente sobre `H_0` decae con `τ_ero`, primero el sedimento y luego `hc`. El sedimento va así (decisión técnica; la adenda pide "la primera celda marina" y el resto llega en la nota del fondo):
   - **Hasta el mar:** inundación por prioridad desde la costa (Barnes et al. 2014). Da a cada celda de tierra su cota de desagüe y la celda hacia la que drena.
   - **En una cuenca cerrada** se queda hasta llenarla y el resto sigue hacia el desagüe. Una cuenca no se convierte en una torre de sedimento.
   - **En el mar**, desde cada desembocadura, llena las celdas más cercanas por el mar hasta la plataforma (−130 m) y sigue mar adentro. Echarlo todo en la primera celda marina apilaba la erosión de un continente en una sola celda.
10. Nivel del mar (una vez por paso, antes del sedimento, como antes).
11. **Balance del paso** (6.9): `|ΔV_cont − (F_arc − F_SE − F_sed − F_col − F_del − F_rift)| / V_cont`. `F_col` incluye lo que no cupo bajo `H_max`. `F_rift` es el adelgazamiento de los rifts, sumidero provisional hasta la nota de rifts (en la Tierra estirar conserva el volumen).

**Tope `H_max` (decisión de realismo, avisada).** El documento principal lo tenía en 65 km y solo para los choques. La adenda pide un espesor máximo de 70–80 km. `H_max` pasa a **80 km**, lo más grueso de la Tierra (Tíbet), y vale para toda corteza: arcos, prisma y choques. Es el tope del colapso gravitatorio, que no distingue el origen de la corteza.
- Se comprueba con la muestra receptora más gruesa, y lo planeado se lleva por muestra. `en_celda` reparte entre las muestras que caen en la celda, cuya área puede ser hasta la mitad de la celda del mundo, y una muestra puede recibir por dos celdas. Con la comprobación por celda del mundo, algunas muestras llegaban a 120–150 km.
- Al fundir placas, una muestra sin sitio libre va a la más delgada de las cercanas.

**Calibración de `τ_ero` (tipo b).** La prueba de relajación de la adenda (§10, prueba 2) usa el choque India–Asia de 50 Myr y luego detiene la convergencia (`paso_sin_movimiento`):

| τ_ero (Myr) | 150 | 100 | 80 | 65 | **63** | 60 |
|---|---|---|---|---|---|---|
| bajar de 38 km (Myr) | 322 | 225 | 184 | 153 | **148** | 142 |

El objetivo es 100–200 Myr; se toma su centro, `τ_ero = 63 Myr`, que no depende de la duración del choque (30 Myr dan 147). Con él, una cordillera de 70 km se erosiona a 0,56 km/Myr y un escudo de 36 km a 16 m/Myr: los órdenes de magnitud de la adenda. `τ_del` (50) y `f_acr` (0,3) **no se calibran**: sus objetivos son globales y dependen del área subducida, que es el doble de la real (§9.3). Calibrarlos ahora compensaría una base irreal.

### 9.3 Resultados (planeta por defecto, semilla 42, 450 Myr sin el rodaje)

| Magnitud | Modelo | Tierra (adenda §10) | |
|---|---|---|---|
| Conservación (remuestreo; balance del peor paso) | 1·10⁻¹⁴; 9,5·10⁻¹⁵ | error de redondeo | sí |
| Área subducida | **6,29 km²/año** (creada 6,44) | 2,7–3,4 | **el doble** |
| Arcos | 6,92 AU | 2,7–5 | alto |
| Erosión por subducción | 2,39 AU | 1,3–1,7 | alto |
| Sedimento subducido | 1,04 AU | 1,0–1,65 | sí |
| Sedimento acrecionado | 0,41 AU | ≈0,3 | cerca |
| Pérdida en choques | **4,72 AU** | 0,4–0,7 | **7–12 veces** |
| Delaminación | 1,18 AU | ≈1,1 | sí |
| Sedimento al mar | 4,93 AU | ≈1,8 | alto |
| Espesor máximo | 78,8 km (tope 80); el 1,9 % de la corteza continental pasa de 70 km | 70–80 | sí |
| Cambio de `V_cont` | −15,9 % | ±10 % | no |
| `f_cont` final | 0,310 | 0,40 ± 0,03 | no |
| Fracción de fosa erosiva | 34 % del área subducida | — | |

Los choques terminados son 108, 71 de arco contra continente y 37 de continente contra continente, con una eficacia efectiva media de 0,76.

**Dónde ocurre el consumo (diagnóstico nuevo, `cuentas.*_por_borde`).** En la Tierra todo entra por las fosas y los choques.

| Tipo del borde del tramo | sin borde | divergente | subducción | colisión | transformante |
|---|---|---|---|---|---|
| Fondo oceánico consumido (km²/año) | 0,14 | 0,56 | 4,57 | 0,11 | 0,92 |
| Corteza continental solapada (AU) | 1,10 | 1,22 | 3,66 | 6,11 | 1,99 |

- El 26 % del fondo consumido y el 31 % de la corteza continental solapada caen en bordes que en la Tierra no consumen nada. Son los dientes de bordes que se deslizan: una placa rígida con un borde dentado que se mueve de lado abre huecos en una cara y se solapa en la otra.
- La geometría del remuestreo no es la causa. La prueba `un_borde_dentado_consume_la_misma_area_que_uno_recto` da 1,022 veces el área teórica con un borde recto y 1,029 con uno en zigzag de dientes a 45°: cuando todos los dientes convergen, el área es la misma.
- Lo que sobra es cinemática y forma. Los bordes del modelo miden ~310 000 km con 14 placas, frente a 182 000 en la Tierra de 15 placas medida igual (§10). Además las placas se mueven poco de lado: el área subducida en fosas, 4,57 km²/año, ya es 1,3–1,7 veces la real.
- Los arcos, la erosión por subducción y la erosión de la génesis siguen al área subducida: por eso salen altos. Es un problema de base, para la nota de los bordes.

**Efecto en el planeta**, frente a la conservación exacta (§7):
- tierra 31,0 % (la Tierra 28,4);
- altura media de la tierra 1 544 → 1 277 m (671);
- océano 3 853 m (3 667);
- nivel del mar −1 184 → −619 m: el sedimento llena las cuencas;
- edad media del fondo 97 → 79 Myr, más de 200 Myr 15 → 8 %;
- lluvia sobre tierra 595 → 710 mm, con `c_P` 2,04 → 1,78;
- la sombra pluviométrica pasa a ir en el sentido real: sotavento entre resto de latitudes medias, 1,24 → 0,79 (la Tierra 0,58, §11).

## 10. Bordes de la Tierra medidos igual que los del modelo (2026-11-05)

`herramientas/analisis/bordes_bird.py` → `config/referencias/tierra_bordes.toml`. Bird (2003), PB2002: 5 819 tramos y 52 placas. Las placas se reconstruyen como regiones de la malla de 52 km, y la longitud se mide con el mismo método que `clasificar_bordes` (`L = 2·A·|∇χ|`).

**La malla no alarga los bordes, los acorta.** El método del gradiente no cuenta la escalera, y una malla de 52 km alisa los zigzag más finos que sus celdas. La afirmación de la adenda v2 §10 ("una malla de 52 km alarga cualquier línea") no se cumple con este método. La comparación con la Tierra medida igual sigue siendo la correcta.

| Clase de Bird | OSR | CRB | SUB | OCB | CCB | OTF | CTF | total |
|---|---|---|---|---|---|---|---|---|
| Vectorial (km) | 67 335 | 27 470 | 51 309 | 17 447 | 23 003 | 47 785 | 26 133 | 260 483 |
| En la malla (km) | 51 443 | 26 091 | 49 275 | 16 196 | 21 203 | 46 488 | 24 413 | 235 109 |

Por tipo del modelo. Dorsal = OSR+CRB; fosa = SUB+OCB (convergencia entre placas oceánicas); choque = CCB; transformante = OTF+CTF.

| km | dorsales | fosas | choques | transformantes | total |
|---|---|---|---|---|---|
| Tierra, clases de Bird en la malla (52 placas) | 77 534 | 65 471 | 21 203 | 70 900 | 235 109 |
| Tierra, regla del modelo, 15° (52 placas) | 103 189 | 53 293 | 39 049 | 38 509 | 234 039 |
| Tierra, regla del modelo, 15° (15 placas mayores) | 87 446 | 39 521 | 25 596 | 29 626 | 182 188 |
| Modelo, media sin el rodaje (14–15 placas) | 120 882 | 117 552 | 11 075 | 62 250 | 311 760 |

- **El modelo sí clasifica fallas transformantes**, desde el primer día: I4, `|v_n| < 0,26·|v_rel|`, es decir, a menos de 15° del desgarre puro. Son el 20 % de su longitud. Bird separa sus clases a ~20°: el percentil 95 de la oblicuidad de sus transformantes es 17,8° y el percentil 5 del resto, 24,4°. Con 20°, la Tierra da 48 400 km de transformantes.
- **Con la regla del modelo, la malla convierte la mitad de las transformantes oceánicas de la Tierra en dorsales.** Los escalones dorsal–transformante más cortos que una celda se alisan en un borde oblicuo con componente de apertura. De los 46 500 km de OTF en la malla, 24 700 salen dorsal, 4 900 fosa y 15 800 transformante.
- **La Tierra de 15 placas**, como el modelo, tiene 182 000 km de bordes. El modelo tiene 312 000: 1,7 veces más borde para el mismo número de placas.
  - Dorsales ×1,4.
  - **Fosas ×3,0.**
  - Transformantes ×2,1.
  - Choques ×0,43: los continentes del modelo chocan menos tiempo y en frentes más cortos, aunque consumen más por km (§9.3).
  - El exceso no viene de que falten las transformantes, sino de bordes más largos para su número de placas y, sobre todo, de más fosa.

## 11. Lluvia observada a 25 km (2026-11-05)

`herramientas/analisis/lluvia_observada.py` → `config/referencias/tierra_lluvia.toml`. Usa WorldClim 2.1 a 10' y CHELSA 2.1 bio12, sobre las mismas celdas de nivel 3 (~26 km) y con las mismas definiciones que el planeta (`laderas_barlovento` y `sotavento_y_resto`, comprobadas iguales en las dos herramientas).

El ×1,36 era **nuestra regla de lluvia sobre el relieve real**, no lluvia medida.

| | WorldClim | CHELSA | Planeta (tras la adenda) |
|---|---|---|---|
| Lluvia de barlovento (mm) | 629 | 907 | — |
| Barlovento / tierra baja de su franja de 1° (cociente de medias) | 0,72 | 0,96 | — |
| Barlovento / tierra baja a 50–300 km contra el viento | 0,90 (27 % del área) | 1,18 | — |
| Sotavento / resto de latitudes medias | 0,58 | 0,57 | 0,79 |
| Lluvia media de la tierra (mm; sin Antártida ni Groenlandia) | 744 (794) | 874 (924) | 710 |

Las columnas del planeta se midieron con el volcado anterior a la adenda; la tabla completa está en `tierra_lluvia.toml`. Antes de la adenda, el planeta daba barlovento entre franja 1,00, sotavento entre resto 1,24 y 595 mm.

- **Con nuestra definición de barlovento, la lluvia observada no aumenta en las laderas: ×0,7–1,2.** La definición coge mesetas altas y secas: la cresta de más de 2 000 m a menos de 150 km siempre se cumple en una meseta. El Tíbet, Irán y el Altiplano son 2,8 de los 8,7 millones de km² de barlovento. El objetivo ×2–3 no tiene apoyo observacional con esta definición; hay que revisarla (decisión del responsable).
- **Lo que falla de verdad en nuestra lluvia** son la sombra pluviométrica (×1,24 antes de la adenda, ×0,79 ahora; la Tierra ×0,58), el perfil con la altura (×1,7 en laderas bajas, la Tierra ×1,0–1,1) y el reparto por latitud (subtrópicos 3–4 veces más secos y latitudes medias del norte el doble de húmedas). La lluvia media de la tierra se queda corta aun con `c_P`.
- **CHELSA es más fiel en montaña** (modela el viento sobre el relieve). WorldClim interpola pluviómetros de valle y es una cota inferior.

## 12. Pendientes (en orden)

1. Nota siguiente del responsable: forma de los bordes (datos del §10 y del diagnóstico del §9.3), inicio de subducción en márgenes viejos, fondo oceánico (reparto del sedimento, compactación, mesetas y montes submarinos) y rifts (que estiren conservando: hoy `F_rift` es un sumidero provisional de 0,11 AU).
2. Con la base arreglada: calibrar `τ_del` (delaminación ≈1,1 AU y espesor máximo 70–80 km) y revisar `f_acr`, la exportación al mar, `V_cont` y `f_cont`.
3. Lluvia: revisar la definición de barlovento (mesetas), el reparto por latitud y la lluvia de base con la media observada (CHELSA ~920 mm sin casquetes), y retirar `c_P`.
4. ~~Adenda del responsable: sumideros reales de corteza~~: hecho, §9.
5. Acortamiento andino (el §6.3 cuenta con él); ciclo de Wilson (rifts por suturas viejas); flexión de la litosfera (cuencas de antepaís); microplacas (Bird).
6. ~~Reducir el remuestreo~~: hecho, §7.
7. Casquetes polares (R6); con ellos, el agua total pasa a ser océano + hielo.
