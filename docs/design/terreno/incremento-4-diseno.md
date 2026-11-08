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

## 12. Nota de bordes de placa: implementación (2026-11-06)

`nota_bordes_placa.md`, del responsable. Código: `tectonica/deriva/suavizado.rs` (borde suavizado) y `deriva.rs` (solapes, huecos, dientes, islas). Sin interruptor: cambio de mecanismo; la referencia del comportamiento anterior es el commit anterior.

### 12.1 Reglas y adaptaciones a la representación (§11.1 de la nota)

- **Borde suavizado (§6.2).** `φ_p,i` = fracción del área a menos de `L_borde` (150 km) que reclama la placa `p`. Un solape cuenta la mitad para cada placa; un hueco, nada. Las vecindades se calculan una vez, en formato CSR.
  - Radio efectivo de al menos 1,5 lados de celda: en las mallas gruesas de las pruebas (~200 km) un radio de 150 km no llega a las vecinas.
  - La normal es el gradiente de `φ_q − φ_p` por mínimos cuadrados sobre las 8 vecinas.
- **Tipo de tramo (§6.3):** con `θ_tr` = 20° (`theta_tr_grados`, que sustituye a `f_transformante` = 0,26 ≈ 15°). `clasificar_bordes` usa también la normal suavizada; la longitud se sigue midiendo con el gradiente del indicador, el método con que se mide la Tierra.
- **Solapes (§6.4).** En un tramo convergente, el torneo de siempre: se consume y se registra `A_sub`. En uno transformante o divergente, la celda es de la placa de más `φ` (desempate sembrado) y la muestra de la otra es un **diente**.
- **Dientes (decisiones técnicas).** Se recolocan antes de repartir los huecos, para que llenen el que su placa deja detrás. Destino: el hueco más cercano que **toca su placa**, de cualquier tipo, a menos de `r_busca = L_borde`.
  - Interpretar "el hueco de su propia placa" como "del lado de su placa según `φ`" dejaba sin hueco a un diente continental metido en el lado de la otra placa. Iba a una sola celda de su placa y la apilaba paso a paso: 1 525 km.
  - Sin hueco, el diente se reparte por las celdas de su placa a menos de `r_busca` si son al menos un cuarto de la vecindad; si no, por todas las de la vecindad. En dos o tres celdas, el "engruesa un poco" de la nota apilaba corteza.
- **Huecos:** los de dorsal, como siempre. Los de tramos transformantes o convergentes que no llenó ningún diente, a la placa de más `φ`, con fondo nuevo (creación residual).
- **Suavizado de la pertenencia (§6.5), como la nota:** solo celdas sin corteza continental ni de arco.
  - Si la otra placa ya tiene una muestra oculta en ese sitio, se ve la suya (si también es solo oceánica); si no, la muestra pasa a la otra placa con su volumen.
- **Extensiones (decisiones técnicas):**
  - **Islas.** Un trozo suelto de una placa (componente conexa que no es la mayor) de menos de `L_borde²` pasa a la placa que más lo rodea, también si es continental. Es la absorción de placas pequeñas del documento principal, pero por trozos. A 52 km, un trozo de 100 km de una placa dentro de otra no es un borde.
  - **Muestras sueltas.** La de una placa con `φ` < 0,15 en la celda (`PHI_SUELTA`; una celda de las ~26 de la vecindad da ~0,04) no gana un solape. Si perdiera también como diente, la corteza de la placa de abajo que quedó oculta bajo la de arriba (corteza metida por debajo) se sumaba entera a la de arriba en poco sitio. Si pierde, se consume como siempre.

### 12.2 Pruebas (§10 de la nota)

| Prueba | Resultado |
|---|---|
| Deslizamiento puro, borde con dientes de una celda, 100 Myr | consumo 0 y creación 0 (exactos); celdas de borde 1 536 → 768 (los dientes desaparecen) |
| Separación oblicua (45°), 30 Myr | consumo 0 en la dorsal; nacen 1,55·10⁷ km² (lo que da la cinemática) |
| India–Asia | velocidad de la India 14,7 → 5,3–6,4 cm/año; antes de la nota, 14,7 → 4,8–6,7 (media tras el choque 5,8 frente a 5,5): mismo comportamiento, no idéntico (los bordes laterales usan 20° y la normal suavizada) |
| Conservación | balance de cada paso a 7·10⁻¹⁵ |

### 12.3 Resultados (planeta, semilla 42, sin el rodaje)

| Magnitud | Antes de la nota | Con la nota | Objetivo (§10) |
|---|---|---|---|
| Fondo consumido en tramos divergentes y transformantes | 26 % | 3,0 % | < 2 % |
| Corteza continental solapada en ellos | 31 % | 4,5 % | < 2 % |
| Fondo consumido / creado (km²/año) | 6,29 / 6,44 | 3,64 / 4,06 | 2,7–3,4 |
| Dorsales (km) | 120 900 | 74 600 | 86 700 ± 25 % |
| Fosas | 117 600 | 74 000 | 37 300 ± 25 % |
| Choques | 11 100 | 6 800 | 23 600 ± 25 % |
| Transformantes | 62 300 | 51 200 | 34 900 ± 25 % |
| Perímetro | 311 800 | 205 700 | 182 600 ± 25 % |
| Edad media del fondo (Myr) | 79 | 132 (mediana 59) | 63,9 (mediana 55,2) |

- **Referencia de la Tierra.** Es la de `tierra_bordes.toml` con la regla de la nota (normal suavizada a 150 km y 20°), 15 placas mayores. Con 52 placas: 100 900 / 51 200 / 35 400 / 47 300 / 234 700 km.
- **Barrido de `L_borde`** (el único parámetro que la nota deja ajustar):

  | `L_borde` (km) | fosas (km) | perímetro (km) | `A_sub` (km²/año) | consumo no convergente | edad media (Myr) |
  |---|---|---|---|---|---|
  | 100 | 80 200 | 226 800 | 4,07 | 4,9 % | 108 |
  | 150 | 74 000 | 205 700 | 3,64 | 3,0 % | 132 |
  | 300 | 58 200 | 161 800 | 2,85 | 2,3 % | 218 |

  Aun con 300 km las fosas son 1,56 veces las de la Tierra. Según la nota (§10 y §11.2), la causa no está en la forma de los bordes, sino en las **fuerzas que mueven las placas**. Se queda en 150 km: con él, el perímetro y las dorsales entran en su margen.
- **Edad del fondo.** La mediana (59 Myr) ya es la de la Tierra. La media sube porque el 24 % del fondo pasa de 200 Myr (la Tierra, ~0,1 %). Antes, los dientes reciclaban fondo joven y la escondían. Falta el inicio de subducción en márgenes viejos (nota siguiente).
- **§11.3 (¿arco por tramo?).** Con los bordes suavizados, el 44,5 % del área subducida sigue cayendo en tramos sin franja propia. El arco se sigue repartiendo por placa.

## 13. Adenda de balance v3: flujo lateral sin tope (2026-11-06)

`H_max` deja de ser una regla (se quita `h_max_km` de la configuración). En las pruebas queda la alarma de 80 km, `ALARMA_ESPESOR_KM`: en el choque India–Asia (malla de 104 km, con un 10 % de holgura por la resolución) y en la batería del planeta (`tect_bal_espesor_max_todo_km`).

**Flujo lateral (§6.6b), adaptaciones (decisiones técnicas):**
- **Va por la malla del mundo**, entre celdas vecinas de lado de la misma placa y con la muestra que ve cada una. La rejilla propia de una placa está girada respecto a la del mundo; la corteza nueva o recolocada queda en celdas sueltas de esa rejilla, sin vecinas con las que fluir. Por la rejilla de la placa, las muestras gruesas no se vaciaban.
- **También fluye a través de un borde de choque continente–continente.** El orógeno abarca las dos placas; si no, una astilla de la placa de arriba rodeada de otras placas se queda con toda la corteza que se mete debajo. Es una desviación del texto de la adenda ("de la misma placa") y se avisa al responsable.
- Subpasos con `D·Δt_sub/d² ≤ 1/16`, porque una muestra puede verla más de una celda. Conserva el volumen por construcción.

**Calibración de `D_flujo` (tipo b)** con India–Asia (malla de 52 km), hecha tras implementar la nota de bordes:

| D (km²/Myr) | 0 | 1 000 | 2 000 | 2 500 | **3 000** | 4 000 | 10 000 |
|---|---|---|---|---|---|---|---|
| Espesor máximo a 30–60 Myr (km) | 219–527 | 91–115 | 76–85 | 72–81 | **72–77** | 68–73 | 58–64 |
| Ancho de la meseta de más de 55 km (km) | 185–230 | 244–334 | 244–351 | 244–366 | **254–284** | 232–280 | 42–268 |

`D_flujo` = 3 000 km²/Myr, el centro de 70–80 km. Equivale a un canal de corteza inferior de 15 km con viscosidad ~10²⁰ Pa·s, dentro de lo estimado para el Tíbet. La meseta (cientos de km) no llega a los ~1 000 km del Tíbet; la limitan la erosión lineal, que gasta igual una meseta que un frente empinado, y la delaminación (§15). Con el flujo y la nota de bordes, la relajación tarda 136 Myr: sigue en 100–200 con `τ_ero` = 63.

### 13.1 Qué apilaba corteza (investigación pedida por el responsable)

Sin tope y sin flujo, el choque India–Asia apila 124 km en 10 Myr, 203 en 20 y 527 en 60. En el planeta llegó a 1 525 km. El tope de 80 km lo ocultaba. Mecanismos encontrados, en orden de importancia, y su arreglo:

1. **La franja de cada frente de choque (Voronoi).** Con un frente de dos filas, las celdas de frente de dentro se quedaban con una sola celda y la apilaban (hasta 18 km por paso). Arreglo: el volumen de cada zona de choque se reparte por igual en su franja de `W_min_orog`, la regla del documento principal ("uniforme en `W_min_orog`"), sobre toda la corteza de la placa de arriba. Que fuese solo la continental era una decisión mía de la primera implementación.
2. **Islas de la placa de arriba** (muestras ocultas que reaparecen dentro de la otra placa). En un choque continental ganaban el solape y se comían, paso a paso, la corteza que les pasaba por debajo. Arreglo: una muestra suelta no gana (§12.1).
3. **El flujo lateral por la rejilla girada de la placa**, donde las muestras gruesas estaban sueltas. Arreglo: por la malla del mundo.
4. **Astillas de la placa de arriba rodeadas de otras placas.** Arreglo: el flujo cruza las suturas de los choques continentales.
5. **Dientes e islas repartidos en 1–3 celdas.** Arreglo: reparto por la vecindad y absorción de islas.
6. **Una montaña de sedimento.** La muestra que el mundo ve en una celda recibía el sedimento de la depresión vecina, donde caía su centro. La depresión seguía siéndolo y la montaña crecía sin fin; además, la erosión se llevaba el sedimento de una cuenca cerrada y volvía a ella. Arreglos: lo que se calcula para una celda va a la muestra que se ve en ella (más las ocultas cuyo centro cae allí), y el fondo de una cuenca cerrada no se erosiona.

**Resultado:** espesor máximo de toda la deriva 1 525 → **113 km**; media de los máximos 64 km; al final, 63. Pasan de 80 km **3 pasos de 500**, por traslados de dientes e islas en uniones de placas. Queda como hallazgo.

## 14. Barlovento con la regla del responsable (2026-11-06)

La regla: laderas que suben de cara al viento dominante, del pie a la cresta; fuera las mesetas (celdas con terreno más alto a barlovento). Se comparan con la llanura del pie. Una sola función para la Tierra y el planeta, `terreno.laderas_pie_cresta()`, del subagente de la lluvia.

| | Tierra, WorldClim | Tierra, CHELSA | Planeta (antes de la nota) | Planeta (ahora) |
|---|---|---|---|---|
| Lluvia de ladera / lluvia del pie (cociente de medias) | 0,90 | 1,23 | 1,12 | 1,05 |

El objetivo es lo que da la Tierra observada con la misma regla: ~1,2 con CHELSA, la más fiel en montaña. No es ×2–3. El 67 % de las laderas del planeta sube directamente desde el mar (la Tierra, 6 %); esas se omiten, como en la Tierra.

## 15. Exportación de sedimento y la ley de erosión (2026-11-06)

Medido tras la nota de bordes (relajación: choque India–Asia de 50 Myr, malla de 104 km):

| `τ_ero` (Myr) | relajación a menos de 38 km | sedimento al mar (planeta) |
|---|---|---|
| 63 | 136 Myr | 3,07 AU |
| 110 | 257 Myr | 3,07 AU |

- **La exportación al mar no depende de `τ_ero`.** En equilibrio, la erosión se lleva lo que añaden los arcos y los choques; `τ_ero` solo decide lo altas que llegan a estar las montañas. Una ley no lineal con la altura cambiaría dónde se erosiona (frentes empinados frente a mesetas) y la anchura de las mesetas, pero no el total exportado.
- **Lo que sobra es lo que falta en la delaminación:** 0,05 AU frente a ≈1,1. El flujo lateral reparte la corteza engrosada antes de que llegue a `h_del` = 50 km (los dos umbrales son iguales, `h_flujo` = `h_del`). En la Tierra, las raíces de los arcos se desprenden: la eclogita de los arcos se forma desde ~35–45 km. En el modelo, ese material acaba erosionado y en el mar.
- **Pregunta para el responsable:** ¿delaminación de las raíces de los arcos (y `τ_del`) antes que la erosión no lineal? `τ_del` y `f_acr` siguen sin calibrar.

> **Corrección (2026-11-07, §19).** Cada fila de la tabla es una sola deriva, y el planeta es caótico: entre semillas, la exportación varía ±0,5 AU. Con 5 semillas, la exportación **sí depende de `τ_ero`**: 3,06 AU con 63 Myr y 2,38 con 110, y baja en todas las semillas. La caída de las raíces de los arcos no la baja (3,13 → 3,06). Los dos puntos de arriba quedan refutados.

## 16. Pendientes (2026-11-06; sustituida por §22)

1. **Fuerzas de las placas** (nota de bordes §11.2): las fosas siguen 2 veces las de la Tierra con cualquier `L_borde` razonable; mirar el reparto de convergencia por placa.
2. **Exportación al mar** (3,1 AU, objetivo ≈1,8): no depende de `τ_ero` (§15); lo que falta es la delaminación (0,05 AU, ≈1,1). Decidir entre la delaminación de las raíces de los arcos y la erosión no lineal con la altura (adenda v3 §11.4), que cambiaría dónde se erosiona pero no el total.
3. Nota siguiente del responsable: inicio de subducción en márgenes viejos (el 24 % del fondo pasa de 200 Myr), fondo oceánico (reparto del sedimento, mesetas, montes submarinos) y rifts que estiren conservando.
4. Los 3 pasos de 500 que pasan la alarma de 80 km (dientes e islas en uniones de placas).
5. Con la base arreglada, calibrar `τ_del` y `f_acr` (sedimento acrecionado 0,14 y subducido 0,38 AU, objetivos ≈0,3 y 1,0–1,65; llega poco sedimento a las fosas).
6. Lluvia: reparto por latitudes (nota propia, tras cerrar la tectónica); `c_P` sin recalibrar.
7. Acortamiento andino, ciclo de Wilson, flexión de la litosfera, microplacas; casquetes polares.

## 17. Diagnóstico de la nota de fosas (§0, 2026-11-07)

Se mide la convergencia perpendicular al borde a lo largo de las fosas, ponderada por longitud:
- **En el modelo**, la componente normal de la velocidad relativa (con la normal del borde suavizado) en cada celda de fosa del lado que se hunde, tras el rodaje. Está en la salida (`convergencia_media_fosas_cm_anio`, `fosa_lenta_frac` y `fosas_por_convergencia_frac` del balance) y en la batería.
- **En Bird (2003)**, la tasa de divergencia de cada tramo de las clases SUB (fosa) y OCB (convergencia oceánica sin fosa), y también en la malla con la regla de la nota (`bordes_bird.py`, `tierra_bordes.toml`).

| | fosa (km) | convergencia media (cm/año) | por debajo de 2 cm/año | se hunden (km²/año) |
|---|---|---|---|---|
| Modelo, semilla 42 (entrega anterior) | 74 000 | 5,10 | 18,6 % | 3,64 |
| Modelo, semilla 42, con la adenda v4 | 75 800 | 4,91 | 20,9 % | 3,60 |
| Bird SUB, tramo a tramo | 51 309 | 5,32 | 21,4 % | 2,73 |
| Bird OCB, tramo a tramo | 17 447 | 1,29 | 81,0 % | 0,23 |
| Tierra en la malla, regla de la nota, 58 placas | 51 163 | 4,74 | 31,3 % | 2,42 |
| Tierra en la malla, regla de la nota, 15 mayores | 37 347 | 3,97 | 36,2 % | 1,48 |

Reparto por clases (% de la longitud; 0–1, 1–2, 2–3, 3–4, 4–5, 5–6, 6–8, 8–10, 10–15 y más de 15 cm/año):

| | 0–1 | 1–2 | 2–3 | 3–4 | 4–5 | 5–6 | 6–8 | 8–10 | 10–15 | >15 |
|---|---|---|---|---|---|---|---|---|---|---|
| Modelo (semilla 42, entrega anterior) | 5,3 | 13,3 | 14,2 | 13,7 | 12,0 | 9,9 | 14,0 | 7,9 | 7,6 | 2,0 |
| Bird SUB | 9,1 | 12,3 | 6,9 | 9,3 | 8,7 | 9,6 | 27,1 | 10,6 | 5,1 | 1,3 |
| Bird OCB | 66,6 | 14,4 | 8,4 | 2,5 | 4,2 | 0,6 | 1,8 | 1,4 | 0,2 | 0,0 |
| Tierra en la malla, 15 mayores | 22,8 | 13,3 | 7,7 | 6,7 | 8,8 | 8,4 | 27,9 | 2,5 | 1,7 | 0,0 |

**El exceso de fosa es rápido.**
- La fosa lenta (menos de 2 cm/año) del modelo mide 13 800 km. La de la Tierra mide 11 000 km (SUB tramo a tramo) y 13 500–16 000 en la malla.
- Todo el exceso está por encima de 2 cm/año: el modelo tiene 60 200 km, frente a 40 300 (SUB), 35 100 (malla, 58 placas) y 23 800 (malla, 15 mayores).
- La convergencia media del modelo es la de las fosas de Bird, no la mitad.
- El nacimiento inducido de la nota quitaría como mucho la fosa lenta (~19 %), y el modelo seguiría con más de 1,6 veces la de las 15 mayores. Según el §0 de la nota, se trae al responsable antes de implementarla.

**El dato de 7–9 cm/año del §0 y del §10 de la nota no es el de Bird.**
- La convergencia media de las fosas de Bird es de 5,3 cm/año tramo a tramo, y 4,0–4,7 en la malla.
- Los 7–9 salen de dividir el área que se hunde en toda la Tierra (2,7–3,4 km²/año) entre la longitud de fosa de las 15 placas mayores (37 300 km).
- Pero esas fosas solo hunden 1,48 km²/año. Al fundir las placas pequeñas con la vecina de más borde se van sobre todo las fosas de las microplacas de arco (Tonga, Marianas, Nuevas Hébridas…), cuyo borde más largo es la propia fosa.
- El modelo no tiene microplacas, así que sus fosas hacen el trabajo de todas las de la Tierra. La referencia coherente es la fosa total de la Tierra: ~51 000 km a 5,3 cm/año, con 2,7 km²/año.
- Contra ella, el modelo tiene 1,45 veces la longitud. Casi todo es exceso de área: hunde 3,6 km²/año frente a 2,73 de las fosas de Bird (×1,33) o 2,96 con la convergencia sin fosa (×1,23).
- En Bird, las dorsales oceánicas crean 3,01 km²/año en 67 335 km, a 4,47 cm/año de media. En total crea y destruye 3,51 km²/año, también en los rifts y en los choques entre continentes.
- La pregunta para las fuerzas es por qué las placas del modelo crean y hunden un 20–30 % más de fondo que la Tierra con la misma velocidad media (4,4 cm/año).

## 18. Respuestas a las preguntas de la nota de fosas (§11)

1. **Tirón de la losa.**
   - El documento principal (§6.2) lo calcula por km de fosa: `F_sp = f_sp·min(√(edad/t_sat), 1)·n̂`, con `t_sat` = 80 Myr. Depende de la edad del fondo que se hunde, no de la longitud de la losa (`velocidades()`).
   - Una fosa tira entera desde el primer paso, así que hace falta el factor `min(1, L_j/L_losa)` del 6.5, con la memoria de la fosa.
   - Es una de las causas posibles del exceso de fosas rápidas, sin medir: cualquier tramo que pasa a fosa empuja enseguida su placa hacia él.
2. **División de placas.**
   - Existe: `partir()` la usan los rifts (§6.5, `τ_rift` = 150 Myr) y los saltos de dorsal.
   - Corta la placa por un círculo máximo deformado con ruido y crea dos hijas con el giro de la madre. En los rifts, adelgaza la corteza continental junto a la línea (`f_rift`) y da el empuje de apertura (`v_rift`).
   - Para un margen pasivo hay que cortar por el tipo de corteza en lugar de por un círculo: el lado de cada muestra se decide aparte. El resto (hijas, dueños, eventos, conservación) se reutiliza.
3. **Zonas de fractura.** No se guardan.
   - Cada muestra lleva su edad, si es continental, si es de arco, sus espesores, su sedimento y la edad de su último engrosamiento.
   - Los saltos de edad de una dorsal escalonada quedan en `edad_oc`, pero las dorsales del modelo no se escalonan a la escala real (celdas de 52 km).
   - Para empezar, los candidatos son los márgenes pasivos y las transformantes actuales.
4. **Diagnóstico del §0:** §17.

## 19. Adenda v4: caída de la raíz de los arcos (2026-11-07)

**Regla.**
- En una columna que recibe corteza de un arco activo en el paso, o que es corteza de arco (`a_i` = 1), la base se desprende por encima de `h_del,arco` = 40 km (`h_del_arco_km`). En el resto, por encima de `h_del` = 50 km.
- `τ_del` es el mismo para las dos (50 Myr, sin calibrar).
- La marca de la franja de arco viaja con la muestra (`en_franja_arco`): la pone el arco al añadir su corteza y la quita la delaminación del mismo paso.
- La salida separa la parte de los arcos (`delaminacion_arcos_au`).
- Con `h_del_arco_km` = 50 el modelo es el de antes, igual salvo los campos nuevos del JSON.

**Fuente.** Jagoutz, O. y Behn, M. D. (2013), *Foundering of lower island-arc crust as an explanation for the origin of the continental Moho*, Nature 504, 131–134. Según el resumen:
- la corteza inferior del arco de Kohistán pesa más que el manto empobrecido de debajo a más de 40 km;
- su caída deja una discontinuidad sísmica a ~38–42 km.

El rango 35–45 km que propuse antes era una estimación mía (la aparición del granate en los cumulados máficos hacia 1,0–1,3 GPa), no de una fuente concreta. La fuente concreta sostiene 38–42 km.

**Resultados.** 5 semillas (42, 1, 2, 3 y 4), 450 Myr sin el rodaje; media [extremos]:

| | `h_del,arco` = 50 (como antes), `τ_ero` = 63 | 40, `τ_ero` = 63 | 40, `τ_ero` = 110 |
|---|---|---|---|
| Sedimento al mar (AU; ≈1,8) | 3,13 [2,81–3,76] | 3,06 [2,38–3,59] | 2,38 [1,99–2,69] |
| Delaminación (AU) | 0,069 [0,05–0,09] | 0,305 [0,17–0,40] | 0,422 [0,26–0,57] |
| – de ella, raíces de arco | 0,014 | 0,254 [0,15–0,33] | 0,348 [0,21–0,47] |
| Cambio de V_cont (±10 %) | −8,4 % [−12,5; −5,3] | −9,5 % [−14,6; −1,7] | −9,5 % [−12,4; −5,6] |
| `f_cont` final (0,40 ± 0,03) | 0,351 | 0,346 | 0,344 |
| Arcos / choques (AU) | 3,76 / 3,09 | 3,80 / 3,10 | 3,76 / 3,04 |
| Se hunden (km²/año) | 3,54 | 3,58 | 3,54 |
| Espesor máximo de una muestra (km) | 162 [132–194] | 140 [121–160] | 142 [125–162] |

La relajación (prueba India–Asia, determinista) tarda 135 Myr con `τ_ero` = 63 y 256 con 110, con la v4 (antes, 136 y 257).

**Conclusiones.**
- **La caída de las raíces de los arcos no baja la exportación al mar.** Multiplica por 4 la delaminación (0,07 → 0,31 AU) y lo que se lleva sale del volumen continental (−8,4 → −9,5 %), no del sedimento al mar. Ese material no acababa en el mar, sino en la propia corteza. La inferencia del §15, mía, era errónea.
- **La exportación depende de `τ_ero`**, al contrario de lo que se dijo en el §15 con una sola deriva: con 110 Myr baja en todas las semillas (3,06 → 2,38) y sube la delaminación (la corteza está más gruesa).
- **Un solo `τ_ero` no cumple a la vez** la relajación (100–200 Myr) y ~1,8 AU. Es el caso en que el responsable dijo pasar a una erosión no lineal con la altura; la adenda v4 lo había descartado por el dato erróneo.
- **Erosión potencial por exceso sobre `h0` = 35 km** (5 semillas, `τ_ero` = 63): la mitad sale de columnas de más de 10 km de exceso, y el 27 % de las de menos de 5 km (llanuras y mesetas bajas). Por bandas: 12 % (0–2 km), 15 % (2–5), 24 % (5–10), 38 % (10–20) y 11 % (más de 20). Una ley lineal gasta las llanuras con el mismo tiempo característico que las cordilleras.
- **Método.** Entre semillas, la exportación varía ±0,5 AU y el volumen continental ±5 puntos. Comparar una sola deriva, como en el §15, puede dar una conclusión falsa. Las calibraciones se miden con la media de varias semillas (prueba `deriva_en_varias_semillas`, ~4 min con 10 derivas en paralelo).

## 20. Uniones triples, finales de fosa y picos de espesor (2026-11-07)

Medido en las mismas 5 semillas con la prueba `deriva_en_varias_semillas`. Una celda está en una unión triple si hay tres placas o más a menos de `L_borde` (150 km). Las uniones triples son el 4,7 % de la longitud de borde.

- **Consumo en bordes que no convergen** (3,2 % del consumo en la entrega anterior):
  - Solo 0,36 puntos (11 %) están en uniones triples.
  - El 86 % (medido con la adenda v4) está a menos de 150 km de una fosa del mismo par de placas: son **finales de fosa**. El solape se decide convergente en su celda, pero la cuenta lo asigna a la celda de borde más cercana (hasta 3 celdas), que ya es el tramo transformante donde la fosa gira.
  - Por oblicuidad, ese consumo «transformante» está a 15–20° en un 56 %, a 10–15° en un 25 % y a menos de 10° en un 19 %.
- **Solape continental en bordes que no convergen** (4,6 %): 0,21 puntos (4,5 %) en uniones triples.
- **Picos de más de 80 km** (entrega anterior): 605 celdas·paso en las 5 semillas, en 81 de 2 250 pasos tras el rodaje.
  - 209 están en uniones triples, pero 204 son un solo episodio de la semilla 2; sin él, el 1 %.
  - Su muestra ya era visible el paso anterior en el 88 %; era una muestra oculta gruesa que reaparece en el 3,5 %. Con la adenda v4: 230 celdas·paso, 79 % y 7 %.
  - En los pasos con picos, el máximo lo sube sobre todo la reconstrucción de los mapas (58 %), donde los dientes sin hueco y las muestras sueltas se suman a celdas de su placa o de la placa en la que están. Le siguen los choques (16 %).
  - El espesor visible máximo tras el rodaje va de 89 a 113 km (de 91 a 135 con la v4).
  - Las muestras ocultas llegan a 86–180 km: se erosionan y delaminan, pero no fluyen (el flujo lateral solo mueve las visibles).

## 21. Coste por etapa y lo que añadiría la nota de fosas (2026-11-07)

Traza de diagnóstico de `dar_paso`: cada etapa da su tiempo de pared y el espesor máximo al acabarla. Está apagada por defecto y entonces no cuesta nada.

La deriva del planeta tarda ~100–110 s por semilla, con 10 a la vez. El escenario del planeta, solo, tarda 164 s de 200, porque la génesis incluye más cosas que la deriva.

| Etapa | % del paso |
|---|---|
| Erosión, nivel del mar y sedimento (dos inundaciones por prioridad) | 31 |
| Rasterizar | 11–12 |
| Solapes (con la clasificación de reclamos) | 10 |
| Suavizado de la pertenencia | 8 |
| Métricas | 8 |
| Clasificar bordes | 5 |
| Procesos de fosa | 5 |
| Islas | 4,5 |
| Velocidades | 3,6 |
| Choques | 3,2 |
| Dientes / flujo lateral / eventos de placas / mapas | 2–3 cada una |
| Delaminación y huecos | 0,6 |

Lo que añadiría la nota de fosas:
- **Estado de cada tramo convergente** (difuso o activo, `X_j`, `L_j`, `T_j`): lo que una parte de clasificar bordes, menos del 0,5 %.
- **Convergencia difusa:** repartir el solape en discos de `W_dif` a cada lado, como los dientes (2,6 %), sobre menos celdas. De 1 a 2 %.
- **Nacimientos y muertes:** casi nada.
- **Placas nuevas en los márgenes pasivos:** cada corte cuesta lo que un rift, y cada placa de más añade ~1 % al paso (8 placas: 100 s; 15 placas: 107 s).

En total, del 3 al 5 % de la deriva: de 4 a 6 s más en el escenario del planeta, que quedaría en ~170 de 200 s. Si hiciera falta sitio, la etapa mayor es la erosión con el sedimento: calcula dos inundaciones por prioridad por paso.

## 22. Pendientes (2026-11-07; sustituida por §30)

1. **Decisión del responsable sobre la nota de fosas.** El exceso de fosa es rápido (§17); la nota quitaría como mucho el 19 %.
   - La referencia coherente para un modelo sin microplacas es la fosa total de la Tierra (51 000 km a 5,3 cm/año), no la de las 15 mayores. Contra ella, el exceso es ×1,45 y casi todo es área: el modelo crea y hunde un 20–30 % más de fondo que la Tierra con la misma velocidad media.
   - Mirar las fuerzas, incluido el tirón pleno desde el primer paso (§18.1).
2. **Erosión:** un solo `τ_ero` no cumple a la vez la relajación y la exportación (§19). ¿Ley no lineal con la altura? Las llanuras (menos de 5 km de exceso) dan el 27 % de la erosión potencial.
3. **Volumen continental:** −9,5 % de media con la raíz de los arcos (una semilla, −14,6 %); el margen es ±10 %.
4. **Picos de más de 80 km:** los dientes y muestras sueltas que se suman al rehacer los mapas, y los choques (§20). Las muestras ocultas no fluyen.
5. **Finales de fosa** asignados a la transformante vecina (3 % del consumo): es una cuenta, no un consumo en un borde que no converge.
6. Calibrar `τ_del`, `f_acr`, `X_ini` y `λ_esp` tras la nota de fosas, con la media de varias semillas.
7. Lluvia: reparto por latitudes (nota propia, tras cerrar la tectónica); barlovento con objetivo ~1,2 (CHELSA); `c_P` sin recalibrar.
8. Acortamiento andino, ciclo de Wilson, flexión de la litosfera, microplacas; casquetes polares.

## 23. Adenda v5: la base (2026-11-07, segunda entrega)

En el orden del responsable: primero la cuenta de los finales de fosa y las muestras ocultas, después la nota de fosas y al final la erosión. Todo se mide con 5 semillas (42, 1, 2, 3 y 4) y su dispersión.

- **Se retira la raíz de los arcos** (`h_del_arco_km`, la marca de la franja del arco y su cuenta): las tasas medidas de los arcos ya son netas.
- **Finales de fosa:**
  - Un frente de consumo a menos de `L_borde` de una fosa del mismo par se asigna a esa fosa (`tramos_en_el_borde`), con su arco y sus procesos.
  - El consumo en bordes que no convergen baja del 3,2 % a 0,4–0,6 %.
  - El solape continental en esos bordes (4,5 %) es solo una cuenta y no se toca.
- **Las muestras ocultas fluyen** (`flujo_lateral`):
  - Cada oculta intercambia con la visible de la celda donde cae su centro, con la misma ley y el acoplamiento de un lado de celda.
  - El máximo de una oculta baja de 86–180 km a 69–92.
- **Dientes:**
  - Los que no encuentran hueco a menos de `L_borde` lo buscan, a menos de 3·`L_borde`, entre los huecos de tramo transformante o convergente que tocan su placa (los de dorsal no: no se pone corteza continental en una dorsal).
  - Los que siguen sin hueco y tienen su placa alrededor esperan en ella, ocultos, al paso siguiente, sin repartirse.
  - Las muestras sueltas, sin su placa alrededor, siguen pasando a la placa donde están.
  - **Hallazgo:** los dientes movían ~5,8 AU de corteza continental al año. La mitad se repartía como engrosamiento en bordes que se deslizan, unos 1–3 AU, del orden de los arcos.
- **Erosión en cuencas cerradas:** la cuenca se mira en la celda que ve la muestra, no en la de su centro (a menudo la vecina). Con la ley de la altura, una muestra alta junto a una depresión no se erosionaba nunca. El fallo venía de antes.

Efecto, antes de la nota (5 semillas, erosión lineal):

| | entrega anterior | + finales de fosa y ocultas | + segunda pasada de dientes | + dientes que esperan |
|---|---|---|---|---|
| Picos de más de 80 km (celdas·paso, 5 semillas) | 605 | 307 | 584 | 27 |
| Espesor visible máximo tras el rodaje (km) | 89–113 | 85–106 | 85–157 | 76–88 |
| Sedimento al mar (AU) | 3,13 | 3,22 ± 0,31 | 2,98 ± 0,58 | 2,55 ± 0,40 |

El engrosamiento de los dientes también alimentaba la erosión: sin él, la exportación al mar baja 0,7 AU.

## 24. Nota de fosas v2: implementación (2026-11-07)

`deriva/fosas.rs`.

**Estado del tramo.**
- Cada celda de borde de un tramo convergente de corteza oceánica guarda un estado en el mapa de su placa (celda local → estado, en las dos orillas): difuso con `X`, o fosa activa con su polaridad, la longitud de la losa `L`, su edad y el tiempo sin converger.
- Viaja con la placa. Cada paso se rehace con los bordes de ahora: el estado se hereda de la celda o de sus vecinas a uno y dos anillos, y el que ya no está en un borde con su par se descarta.
- Un tramo es una componente conexa de los bordes convergentes del par, con las dos orillas.
- Tras partirse o fundirse una placa, el estado que miraba a ella mira a sus hijas o a la fundida (`ancestros`).

**Reglas.**
- **6.1–6.2.** Un tramo nuevo es difuso con `X = 0`. Sus solapes no se consumen: la muestra que pierde se quita (la placa pierde área) y su corteza, su base basáltica y su sedimento se reparten como espesor entre las celdas de las dos placas a menos de `W_dif` = `L_borde`, por área. Se conserva el volumen; no hay arcos, ni `A_sub`, ni sumideros. `X` crece con la convergencia perpendicular media del tramo.
- **6.3.** Con `X ≥ X_ini` (150 km) nace la fosa. Polaridad: se hunde el océano frente a corteza que flota; entre dos océanos, el más viejo; si empatan, sorteo sembrado.
- **6.4.** Nacimiento espontáneo en márgenes pasivos (sin otra placa a menos de `L_borde`) y en transformantes entre fondos oceánicos con contraste de edad. Se trocean en bloques de unos `L_borde` y se hace una tirada sembrada por bloque y paso. En un margen, la parte oceánica conexa se separa como placa nueva (`partir_por_corteza`) y todo el borde nuevo es la fosa.
- **6.5.** Una fosa activa consume con su polaridad fija (también en el torneo de dos océanos). `L` crece con la convergencia y el tirón se multiplica por `min(1, L/L_losa)`, con `L_losa` = 800 km.
- **6.6.** Muere a los `τ_muerte` = 10 Myr sin converger, o por choque. Una muerte cuenta cuando el grupo de celdas que muere no toca celdas activas vivas de su par; si las toca, la fosa se acorta. Tras un choque, el tramo vuelve a difuso con `X = 0`.
- **Sustituye** a la polaridad de dos océanos decidida por par de placas.

**Decisiones de quien implementa** (huecos de la nota).
- **Corteza que flota = corteza continental** (`h > h_cont_min` = 20 km; un arco más grueso ya es continental).
  - Con «`hc > 0`», una capa fina de corteza de tipo continental (el sedimento que `sumar_hc` convierte en roca, el material de una muestra suelta) bastaba para matar la fosa por choque.
  - En el planeta, las fosas vivían 2–3 Myr y casi toda la convergencia era difusa: 69 000 km, 3 km²/año, exportación de 11 AU.
- **Una fosa espontánea nace con la losa que ya cuelga, `L = X_ini`:** con `L = 0` no tiraría y moriría a los 10 Myr.
- **El tirón sigue mientras la fosa vive**, también en los pasos en que sus placas se deslizan.
- **Para los escenarios que empiezan con una subducción en marcha**, `activar_fosas(L)` pone fosas maduras en los tramos convergentes: India–Asia y el borde dentado.

**Pruebas** (§10 de la nota), en verde: convergencia lenta y corta (no nace, se engruesa y conserva), convergencia suficiente (nace a los 3 Myr y se hunde la vieja), polaridad fija, muerte tras `τ_muerte` con el tramo divergente, y nacimiento espontáneo en un margen pasivo con placa nueva y balance exacto. India–Asia sigue igual.

## 25. Calibración de `λ_esp` y resultados de la nota (2026-11-07)

`λ_esp` se calibra con la edad del fondo (Seton et al. 2020), con 5 semillas y la erosión lineal anterior:

| `λ_esp` (1/(km·Myr)) | edad media del fondo (Myr) | fondo de más de 180 Myr | fosa activa (km) | se hunden (km²/año) | al mar (AU) |
|---|---|---|---|---|---|
| 0 | 103 ± 11 | 26 % | 58 700 ± 5 800 | 2,92 | 3,9 |
| 1·10⁻⁷ | 71 ± 4 | 7,0 % | 72 300 | 4,17 | 4,7 |
| 2·10⁻⁷ | 62 ± 6 | 6,1 % | 81 100 | 4,60 | 4,9 |
| 4·10⁻⁷ | 56 ± 7 | 4,2 % | 92 000 | 5,27 | 5,3 |
| 1·10⁻⁶ | 52 ± 9 | 4,7 % | 100 300 | 5,69 | 5,4 |

- La edad media de la Tierra (64,2 Myr) sale con `λ_esp` = 1,7·10⁻⁷, el valor por defecto.
- La fracción por encima de 180 Myr no baja del 4 % con ningún `λ_esp`: hay fondo viejo en el interior de placas oceánicas grandes que ningún nacimiento alcanza.

Con `λ_esp` = 1,7·10⁻⁷ y la erosión con la altura provisional (§28), 5 semillas:

| | modelo | Tierra |
|---|---|---|
| Fosa activa | 79 300 ± 7 400 km a 5,7 cm/año | ~51 000 km a 5,3 (Bird SUB; 49 300 en la malla) |
| Convergencia difusa | 10 300 ± 1 100 km a 1,9 cm/año; absorbe 0,31 km²/año | 16 200–17 400 km a 1,3 (OCB); ~0,2 km²/año |
| Se hunden / se crean (km²/año) | 4,4 ± 0,5 / 4,9 | 2,7 + 0,2 / 2,7–3,4 |
| Edad media del fondo; más de 180 Myr | 68 ± 8 Myr; 7,8 % | 64,2; menos del 1 % |
| Edad media de las fosas; fracción del tirón | 67 Myr; 0,82 | — |
| Velocidad de las placas; Forsyth–Uyeda (fosa) | 4,56 cm/año; 0,68 | 4,39; positiva |
| Nacimientos por 100 Myr | 174 inducidos (muchos son trozos que alargan una fosa), 5,5 en margen, 0,1 en transformante | — |
| Arcos / choques / al mar (AU) | 4,5 / 3,1 / 4,2 ± 1,0 | ~1,8 al mar |
| V_cont; `f_cont` | −7,4 ± 4,1 %; 0,37 | ±10 %; 0,40 |

- **La fosa sigue 1,55 veces por encima de la referencia.** Según el §10 de la nota, se miden las fuerzas y se traen al responsable antes de tocar nada (§27).
- **Sin nacimientos espontáneos** (`λ_esp` = 0), fosa, área y tamaños de placa son los de la Tierra: 58 700 km (×1,15), se hunden 2,92 + 0,28 km²/año, y la placa mayor ocupa el 25 % de la esfera. Pero el fondo es demasiado viejo.
- Los nacimientos en márgenes pasivos que hacen falta para la edad del fondo añaden unos 20 000 km de fosa y un 50 % de área: cada uno convierte en fosa todo el margen contiguo, crea una placa y lleva a más choques y suturas.
- **La convergencia difusa mete basalto bajo los continentes:** 1,1 ± 0,1 km³/año. Lo pide la nota («ambas placas»), pero los levanta, y la erosión se lleva corteza continental por arriba. En un escenario India–Asia dejó 15 km de basalto bajo el borde de Asia.
- **Los picos de más de 80 km vuelven con la nota:** 3 400–11 700 celdas·paso y un máximo visible de 107–129 km, también con `λ_esp` = 0. Los sube sobre todo el engrosamiento de los choques.

## 26. Referencia de Bird para la nota (2026-11-07)

`bordes_bird.py` da ahora lo que la regla de la nota llama fosa en cada clase de Bird (todas las placas): SUB 37 120 km, OCB 10 006, OTF 3 141, CCB 629 y OSR 197.
- Las clases en la malla miden SUB 49 275 km y OCB 16 196.
- Con las 15 mayores, SUB da 25 852 km de fosa.
- La referencia de la nota, «la de Bird SUB en la malla» (≈51 000 km), corresponde a la clase SUB, 49 275 km, o al total que la regla llama fosa, 51 163.

## 27. Las cuatro medidas de fuerzas (nota de fosas §10)

Con la configuración de §25 (5 semillas; la Tierra, de Bird en la malla):

1. **Oblicuidad** (% de la longitud por clases de 10°, de 0° = desgarre puro a 90° = de frente).
   - Convergentes: modelo 13,8 / 11,6 / 10,3 / 10,6 / 10,6 / 10,6 / 10,7 / 10,9 / 11,1, casi uniforme; Bird 10,1 / 10,8 / 8,4 / 7,3 / 7,5 / 9,2 / 13,6 / 16,0 / 16,9. En la Tierra, el 46,5 % converge a más de 60° (en el modelo, el 33 %).
   - Divergentes: modelo casi uniforme (32 % a más de 60°); Bird, 42 %, con el 17,5 % a 80–90°: las dorsales se abren de frente.
2. **Tamaños de las placas** (% de la esfera, al final).
   - La mayor del modelo ocupa el 23–53 % (media 41 %) y quedan 8–14 placas.
   - Bird, las 15 mayores: 21,8 / 11,5 / 11,4 / 10,9 / 10,1 / 9,8 / 9,0 / 3,7 / 3,7 / 2,5 / 1,8 / 1,1 / 1,0 / 1,0 / 0,6.
   - Con `λ_esp` = 0, la mayor ocupa de media el 25 %, como el Pacífico.
3. **Potencia de cada fuerza** (`τ·ω` sumado en todas las placas; en equilibrio suman cero).
   - Motriz: tirón de la losa 74–76 % y empuje de dorsal 24–26 %. Las estimaciones de la Tierra dan la mayor parte a las losas, del orden del 90 % (Lithgow-Bertelloni y Richards 1998), con el empuje de dorsal mucho menor.
   - Resistente: arrastre basal 81–84 % y choques 16–19 %. Igual con `λ_esp` = 0.
4. **Área creada y consumida.**
   - Se crean 4,2–5,5 km²/año y se consumen 3,7–4,9 más 0,3 difusos. La Tierra está en 2,7–3,4: Bird crea 3,01 km²/año en las dorsales oceánicas y 3,51 en total.
   - Con `λ_esp` = 0: 3,2–3,65 creados y 2,7–3,1 + 0,3 consumidos.

**Lectura** (sin tocar nada).
- La diferencia de fuerzas con la Tierra es sobre todo de forma: oblicuidad uniforme frente a bordes de frente, y un empuje de dorsal del 25 %.
- Pero el exceso de fosa y de área de la configuración calibrada viene casi entero de los nacimientos espontáneos en márgenes pasivos. Sin ellos, longitud, área y tamaños son los de la Tierra.
- Lo que hay que decidir es cómo se consume el fondo viejo sin rodear márgenes enteros de fosa.

## 28. Erosión con la altura (adenda v5 §6.7, 2026-11-07)

**Ley.**
- `E = E_1·(z/1 km)^n_ero` km/Myr donde la columna asoma (`z > 0` sobre el mar), primero el sedimento y luego la corteza de tipo continental (la base basáltica no se erosiona).
- Lo retirado en un paso no baja la columna del nivel del mar (con el rebote isostático); en las cuencas cerradas no hay erosión.
- Se retira `τ_ero` (`tau_orog_myr`) con la ley lineal.

**Relajación** (India–Asia, 50 Myr de choque, tiempo para bajar de 38 km), tras el arreglo de la cuenca cerrada:

| `n_ero` | `E_1` = 0,005 | 0,01 | 0,02 | 0,04 km/Myr |
|---|---|---|---|---|
| 2 | 286 Myr | 144 | 80 | 46 |
| 3 | 71 | 38 | 21 | 10 |

**Valores provisionales:** `n_ero` = 2 (el inicial) y `E_1` = 0,01 km/Myr.
- La relajación tarda 144 Myr.
- Las tasas cumplen la comprobación: ~1 m/Myr a 300 m, 10 m/Myr a 1 km, 0,09–0,25 km/Myr a 3–5 km (0,1–1 en cordilleras).
- **No se calibran con la exportación al mar:** con la base actual (se hunde un 50 % más de fondo y hay 4,5 AU de arcos), ajustarlos a ~1,8 AU sería forzar un parámetro para compensar otra irrealidad (CLAUDE §2.8). Con estos valores, el planeta exporta 4,2 ± 1,0 AU.
- En la semilla 42 la tierra baja a 921 m de media (antes 1 203; la Tierra 671), la franja de 1–2 km pasa del 64 % al 21 % (la Tierra 16 %) y el nivel del mar queda en el dato (antes, 700 m por debajo).

## 29. Coste (2026-11-07)

- La deriva tarda 112 s por semilla, frente a ~100 sin la nota.
  - Fosas y nacimientos espontáneos: 5,5–6,2 %.
  - Convergencia difusa: 0,8 %.
  - Flujo de las ocultas: el flujo lateral sube del 2,5 al 5,5 %.
- El escenario del planeta tarda 187,6 s de 200.
- Con `λ_esp` = 1·10⁻⁶, la deriva llegaba a 117–380 s: la búsqueda del sedimento en el mar recorre cada vez más mar cuando la exportación es grande y las plataformas se llenan.

## 30. Pendientes (2026-11-07 b, en orden; sustituida por §38)

1. **Decisión del responsable sobre el consumo del fondo viejo** (§25, §27). Los nacimientos en márgenes pasivos que piden la edad del fondo convierten márgenes enteros en fosa y suben fosa y área un 50 %; sin ellos, fosa, área y tamaños son los de la Tierra. Además, el fondo de más de 180 Myr no baja del 4 %.
2. **Basalto de la convergencia difusa bajo los continentes** (1,1 km³/año): la nota dice «ambas placas». ¿Solo la placa oceánica?
3. **Picos de más de 80 km con la nota** (máximo visible 107–129 km): sobre todo el engrosamiento de los choques.
4. **Calibración conjunta de la erosión** (`E_1`, `n_ero`) con la relajación y ~1,8 AU, cuando la base esté corregida.
5. **Tiempo:** el planeta está en 187,6 s de 200; la etapa mayor es la erosión con el sedimento (~30 %).
6. Calibrar `τ_del`, `f_acr` y `X_ini` (100–200) tras decidir 1–3, con varias semillas.
7. Lluvia: reparto por latitudes (nota propia); barlovento con objetivo ~1,2 (CHELSA).
8. Acortamiento andino, ciclo de Wilson, flexión de la litosfera, microplacas; casquetes polares.

## 31. Nota de fosas v3: implementación (2026-11-07 c)

`deriva/fosas.rs`. El responsable aceptó las tres decisiones de §24 (flota la corteza de más de 20 km, una fosa espontánea nace con `L = X_ini`, el tirón sigue mientras la fosa vive).

**Nacimiento espontáneo solo donde el fondo es viejo (6.4).**
- Candidatas de margen: las celdas continentales cuya vecina oceánica más vieja de su placa tiene más de `a_min` (140 Myr), sin otra placa a menos de `L_borde`. Se trocean en bloques de unos `L_borde`, con una tirada sembrada por bloque y paso: `p = 1 − exp(−λ_esp·((a − a_min)/100 Myr)·L·Δt)`.
- Al nacer, el tramo de la fosa es el conjunto conexo de celdas candidatas del bloque, no todo el margen.
- `partir_por_fosa` (sustituye a `partir_por_corteza`): un camino mínimo de varias fuentes por el fondo oceánico de la placa, desde las celdas de mar de su orilla. Las que tocan el tramo llevan la marca 1. La placa nueva es el océano con la marca 1 (el que está delante del tramo). Las fosas se ponen solo en los bordes nuevos que tienen corteza continental a algún lado.
- En una transformante: solo el tramo con fondo de más de `a_min` y contraste de edad (más de la mitad de la edad del viejo) pasa a fosa, y se hunde el lado viejo.

**Convergencia difusa (6.2): el basalto nunca pasa a corteza continental.** El área oceánica solapada se retira sin arco ni sumideros. La corteza de tipo continental o de arco y el sedimento de la muestra se reparten como espesor a menos de `W_dif` = `L_borde`, por área: en las celdas continentales de las dos placas si las hay; si no, entre todas las de las dos placas. Se conserva el volumen continental (`hc` más el sedimento con su masa).

**Prueba 1 de la nota, adaptada a la v3.** El texto de su §10 («la corteza se engruesa y el volumen se conserva») es anterior a la regla del basalto. Ahora las dos placas llevan 0,5 km de sedimento. A 1 cm/año durante 10 Myr no nace fosa, no se hunde nada como fosa, se retira el fondo que converge y el sedimento se acumula en la franja (más de 0,55 km). El inventario continental se conserva a 10⁻⁹. Las otras cuatro pruebas y India–Asia siguen en verde.

## 32. Adenda v6: flujo lateral con el cubo del canal (2026-11-07 c)

**Ley.** `V_ik = D_0·(c_i/c_ref)³·(h_c,i − h_c,k)/d_ik·ℓ_ik·Δt`, con `c_i = h_c,i − h_flujo` (el canal débil por encima de 50 km) y `c_ref` = 10 km (normalización). Va de la columna más gruesa a la vecina y usa el `D` de la de arriba. En un canal, el caudal crece con el cubo de su espesor (Clark y Royden 2000), así que un engrosamiento excesivo se aplana mucho más deprisa que uno moderado. Se retira `d_flujo_km2_myr` (la ley lineal).

**Subpasos adaptativos.** Cada subpaso se adapta al canal más grueso de ese momento, con `Δt_sub = min(resto, d_min²/(16·D(h_max)))`. La muestra más gruesa del planeta fija el número de subpasos. Por eso el coste del flujo depende de que no haya agujas.

**Red compacta** (`RedFlujo`).
- Las muestras que intervienen se guardan en un índice ordenado por placa y celda local, junto con los lados de cada celda fuente y las parejas de visible y oculta.
- La red se rehace solo cuando una celda pasa a fuente al recibir.
- Cada subpaso hace las mismas cuentas, en el mismo orden, que recorrer el mundo celda a celda. La salida es idéntica byte a byte, comprobado con la semilla 42 y `D_0` = 1600.
- Las parejas de ocultas que no pueden fluir (las dos por debajo de `h_flujo`) no entran; se añaden si su celda pasa a fuente.
- El flujo pasa de 23 a 18 s por deriva: el coste no estaba en las tablas sino en el número de subpasos.

**Calibración de `D_0` con India–Asia** (adenda v6: espesor máximo de 70–80 km; mesetas de cientos a ~1 000 km). Espesor máximo a los 50 Myr de choque, a 52 km, con la base ya corregida (§33):

| `D_0` (km²/Myr) | 25 | 50 | 100 | 200 | 400 | 800 | 1 600 |
|---|---|---|---|---|---|---|---|
| `τ_del` = 25 Myr | 74 | 72 | 69 | — | 64 | 62 | 61 |
| `τ_del` = 50 Myr | — | 74 | 71 | 68 | 65 | — | 61 |
| `τ_del` = 100 Myr | — | — | 72 | — | 66 | — | — |

- Manda `D_0`; `τ_del` apenas cambia nada en India–Asia.
- La meseta de más de 4 km mide 245–320 km de ancho y 0,23–0,30 Mkm². El Tíbet mide ~1 000 km y 2,5 Mkm²: el escenario es pequeño (un choque de 2 000 km de frente durante 50 Myr) y no llega a mesetas tan grandes con ningún `D_0`.
- Antes del arreglo del sedimento, el mismo barrido daba 70–79 km: parte de ese espesor venía del sedimento apilado en el mar que se cierra entre India y Asia.
- `D_0` = 50 km²/Myr equivale a ~2·10²⁰ Pa·s en un canal de 10 km, dentro del rango de Clark y Royden (2000): 10¹⁸ en los bordes suaves del Tíbet y 10²¹ en los abruptos.

## 33. Los picos de 107–129 km: ¿columnas reales? (2026-11-07 c)

**Primera comprobación, por muestras distintas** (antes de los arreglos; 5 semillas). De las celdas·paso de más de 100 km, tienen al menos 3 **otras** muestras de más de 80 km alrededor:
- el 95–96 % en las semillas 42 y 1;
- el 63 % en la 2, el 32 % en la 3 y el 46 % en la 4.

Las agujas (vecinas de menos del 70 %) son el 1, 1, 26, 67 y 0 %. Entre ellas, las que ve más de una celda o tienen el centro en otra son pocas. No son la misma muestra vista varias veces: son regiones gruesas de verdad.

**Pero de dónde salen esas regiones** (`diagnostico_pilas`, dos pasadas de la misma deriva).
- Antes del paso del pico, la muestra era oceánica y llevaba encima **160, 229, 648, 900, 1 376 y hasta 1 835 km de sedimento**.
- Un choque, un arco, una convergencia difusa o la reconstrucción de los mapas lo convertía en corteza (`sumar_hc`), con picos de 156 a 1 592 km.
- El flujo lateral lo repartía por las vecinas en el mismo paso y dejaba regiones de 60–75 km.
- Eran esos picos los que llegaban a 688–2 068 km en algunas semillas, y también inflaban la exportación al mar, que llegó a 10 AU.

**La causa, en `sedimento_al_mar`.** El sedimento de cada desembocadura llenaba el mar más cercano hasta la plataforma (−130 m). Si ese mar ya estaba lleno, todo lo que sobraba se quedaba en la desembocadura, sin tope. El comentario decía que eso no pasaba en una deriva normal, y sí pasa. Lo producen los mares cerrados pequeños rodeados de montañas que se erosionan:
- el resto de un océano que se cierra entre dos continentes que chocan;
- una depresión de rift por debajo del nivel del mar.

**Arreglo** (dos partes):
1. **Solo es salida un mar abierto.** Una masa de mar conectada de al menos `area_mar_abierto_km2` = 5·10⁵ km² (y la mayor, si ninguna llega).
   - Un mar cerrado más pequeño es una cuenca cerrada para la inundación por prioridad. Se rellena hasta su cota de desagüe y lo que sobra sigue cuesta abajo hasta el mar abierto, como en el Caspio, con unos 20 km de sedimento en su cuenca sur.
   - Es un criterio de la malla (a ~52 km un estrecho puede no verse), no un parámetro físico.
2. **Seguro:** si el mar abierto de una desembocadura ya está lleno hasta la plataforma, lo que sobra sigue hasta el mar más cercano con sitio, aunque cruce tierra. Se cuenta aparte (`sedimento_desbordado_km3`).

**Efecto** (5 semillas):
- El sedimento máximo de una muestra baja de 160–1 835 km a 34–71 km.
- Lo desbordado queda en 0,005–0,02 AU, y 0,45–0,56 AU se quedan en cuencas cerradas.
- Ya no hay picos de cientos de km.
- Quedan agujas de 110–150 km durante una sola etapa. Salen de sumar varias muestras en una al absorber islas (`absorber_islas`) o al fundir placas, y el flujo las aplana en el mismo paso.

**Respuesta.** Los picos eran columnas reales en el sentido de la pregunta (no muestras duplicadas). Pero los alimentaba un fallo de la base: el sedimento apilado sin tope.

## 34. Calibración conjunta con 5 semillas (2026-11-07 c)

Semillas 42, 1, 2, 3 y 4 (`deriva_en_varias_semillas` con `VARIANTES`), con la base de §33. Media ± desviación entre semillas.

**Erosión** (`λ_esp` = 1,6·10⁻⁵, `f_acr` = 0,2; cada pareja `E_1`–`n_ero` da una relajación de ~140 Myr):

| `n_ero`, `E_1` (km/Myr), `D_0` | al mar (AU) | ΔV_cont | `f_cont` | fosa (km) | edad del fondo; > 180 Myr | máximo visible (km) |
|---|---|---|---|---|---|---|
| 2; 0,01; 1 600 | 2,85 ± 0,69 | −13,2 % | 0,337 | 67 300 | 75 Myr; 13 % | 98 |
| 2,5; 0,005; 1 600 | 2,28 ± 0,28 | −8,2 % | 0,351 | 60 900 | 78; 17 % | 106 |
| 3; 0,0025; 1 600 | 2,16 ± 0,51 | −10,1 % | 0,334 | 69 700 | 73; 13 % | 95 |
| 2,5; 0,005; 400 | 2,39 ± 0,29 | −10,1 % | 0,342 | 64 900 | 74; 14 % | 96 |

**Parámetros de las fosas y la delaminación** (`λ_esp` = 4·10⁻⁶, `n_ero` = 2,5, `E_1` = 0,005):

| variante | al mar | delaminación (AU) | ΔV_cont | fosa (km) | difusa (km) | edad; > 180 Myr | máximo visible |
|---|---|---|---|---|---|---|---|
| `D_0` 1 600, `τ_del` 50 | 2,48 | 0,26 | −10,4 % | 59 500 | 8 170 | 79; 14,5 % | 113 ± 24 |
| + `X_ini` = 200 km | 2,38 | 0,28 | −9,7 % | 59 700 | 9 680 | 83; 15,5 % | 101 |
| + `a_min` = 100 Myr | 2,38 | 0,25 | −8,6 % | 63 200 | 8 830 | 78; 16 % | 101 |
| + `τ_del` = 25 Myr | 2,33 | 0,43 | −9,7 % | 63 300 | 8 350 | 81; 15,6 % | 90 ± 11 |
| + `τ_del` = 15 | 2,42 | 0,74 | −15,2 % | 64 600 | 7 980 | 75; 11,6 % | 96 |
| + `τ_del` = 10 | 2,30 | 0,81 | −13,6 % | 64 300 | 8 760 | 76; 13,9 % | 105 ± 38 |
| `D_0` 100, `τ_del` 25 | 2,24 | 0,62 | −10,6 % | 63 700 | 8 680 | 77; 13 % | 96 |
| `D_0` 50, `τ_del` 25 | 2,49 | 0,71 | −13,4 ± 7,8 % | 59 500 | 7 100 | 83; 18 % | 93 |
| `D_0` 50, `τ_del` 50 | 2,49 | 0,46 | −11,3 % | 59 800 | 7 920 | 83; 19 % | 106 |

Celdas·paso de más de 100 km (5 semillas): 21 con `D_0` = 1 600, 325 con 100 y 2 331 con 50.

**`λ_esp`**: con 1·10⁻⁶, 4·10⁻⁶ y 1,6·10⁻⁵ nacen 4, 8 y 15 fosas en márgenes por 100 Myr, y la edad media del fondo apenas cambia (74–84 Myr; más de 180 Myr, 13–18 %).

**Valores elegidos:**

| Parámetro | Valor | Por qué |
|---|---|---|
| `D_0` | 50 km²/Myr | India–Asia, el caso limpio, da 72 km (§32; objetivo 70–80). En el planeta, un `D_0` alto aplana mejor las columnas (21 celdas·paso de más de 100 km con 1 600, frente a 2 331), pero sus choques engruesan de más (§36): subirlo para eso sería tapar una irrealidad con otra (CLAUDE §2.8) |
| `τ_del` | 25 Myr | Delaminación de 0,71 AU, la más cercana a ≈1,1; con 50 Myr, 0,46 AU y máximo visible de 106 km. India–Asia no cambia. La caída convectiva de una raíz eclogítica tarda 10–30 Myr |
| `n_ero`, `E_1` | 2,5; 0,005 km/Myr | La relajación tarda 135 Myr (100–200; con 0,004, 177 Myr; con 0,006, 125). Con n = 3 la exportación es casi la misma (2,16 frente a 2,28 AU) y las llanuras bajan de 1 m/Myr. Con n = 2 se exportan 2,85 AU |
| `f_acr` | 0,2 | Acrecionado/subducido = 0,24 (Tierra 0,3/1,0–1,65 = 0,18–0,30) |
| `λ_esp` | 4·10⁻⁶ /(km·Myr) | ~8 nacimientos en márgenes por 100 Myr. La edad no responde, así que no se sube (la nota: el nacimiento espontáneo es raro) |
| `X_ini` | 150 km (igual) | 200 km no acorta la fosa y alarga la convergencia difusa |
| `a_min` | 140 Myr (igual) | 100 Myr da más nacimientos (13 por 100 Myr) y más fosa, pero no rejuvenece el fondo |

**Tasas de la erosión** con `n_ero` = 2,5 y `E_1` = 0,005: 0,25 m/Myr a 300 m, 0,9 a 500 m, 5 a 1 km; 0,08 km/Myr a 3 km, 0,16 a 4 km, 0,28 a 5 km. Las llanuras se quedan por debajo de los metros por Myr de los escudos (1–10 m/Myr), y las cordilleras dentro de 0,1–1 km/Myr por encima de ~3,5 km.

## 35. Tabla de cierre (2026-11-07 c)

`herramientas/planeta_semillas.py`: el planeta entero (génesis, relieve y un año de clima) con las semillas 42, 1, 2, 3 y 4. Media ± desviación [mínimo–máximo]. La Tierra del relieve es ETOPO 2022 con la roca sin hielo y su rebote, a 26 km.

| | Modelo | Tierra u objetivo | |
|---|---|---|---|
| Tierra emergida | 0,330 ± 0,034 [0,276–0,368] | 0,284 | ✗ |
| Altura media de la tierra | 1 150 ± 188 m [884–1 400] | 671 m | ✗ |
| Tierra por debajo de 200 m | 0,039 ± 0,012 [0,026–0,053] | 0,283 | ✗ |
| Pendiente media de la tierra (±25 km) | 0,0124 ± 0,0016 | 0,0052 | ✗ (×2,4) |
| Anchura media de la plataforma | 4,7 ± 0,7 km | 42,8 km | ✗ |
| Corteza máxima al final | 76 ± 4,5 km [70–81] | 70–80 km | ✓ |
| Corteza máxima de toda la deriva | 101 ± 2,7 km [98–104] | 70–80 km | ✗ |
| Edad media del fondo al final | 99 ± 16,5 Myr [74–115] (mediana 64) | 64,2 Myr | ✗ |
| Fondo de más de 180 Myr | 18,5 ± 4,3 % | menos del 1 % | ✗ |
| `f_cont` | 0,333 ± 0,038 | 0,40 ± 0,03 | ✗ |
| Cambio del volumen continental | −13,4 ± 7,8 % [−25,6 a −4,4] | ±10 % | ✗ |
| Velocidad de las placas | 4,49 ± 0,23 cm/año | 4,39 | ✓ |
| Sedimento al mar | 2,49 ± 0,37 AU | ≈1,8 | ✗ |
| Longitud de fosa | 59 470 ± 6 030 km [50 370–66 020] | ~51 000 ± 25 % | ✓ (×1,17) |
| Área que se hunde | 3,14 ± 0,35 km²/año | ≈2,7 (Bird SUB); 2,7–3,4 | ✓ (×1,16) |
| Tiempo de cálculo del planeta | 181,3 s solo (batería); 180–204 s con las cinco a la vez | 200 s | ✓ |

Otras medidas: la convergencia media de las fosas es de 5,46 cm/año (Bird SUB 5,3). La convergencia difusa mide 7 100 km y absorbe 0,26 km²/año (OCB: 17 400 km y ~0,2 km²/año). La placa mayor ocupa el 27 % (el Pacífico, el 20,5 %). Al final de la deriva, el nivel del mar queda entre 500 y 1 000 m por debajo del dato y el océano tiene 3,93 km de profundidad media (3,67 km en la Tierra).

La longitud de fosa y el área que se hunde cumplen, así que no van a la ampliación futura.

## 36. Lo que no cuadra, y su causa

1. **El fondo viejo baja el mar, y eso deforma el relieve.**
   - **Qué pasa con la nota v3.** Solo nace fosa en el tramo de fondo viejo, así que la fosa y el área que se hunde son las de la Tierra. Pero el fondo viejo de los márgenes pasivos no se consume. Envejece toda la deriva: 79–83 Myr de media a lo largo de ella y 99 Myr al final.
   - **Cómo baja el mar.** Las cuencas son más hondas y el océano ocupa más área (`f_cont` de 0,33). El mar baja de 0,5 a 1 km.
   - **Qué hace eso al relieve:**
     - las plataformas y los taludes quedan en seco;
     - emerge el 33 % del planeta;
     - la tierra queda a 1 150 m de media;
     - casi no hay tierra por debajo de 200 m;
     - la plataforma mide 4 km;
     - la pendiente media se dobla;
     - la franja de 1–2 km guarda el 45 % de la tierra (Tierra 16 %).
   - **Con la v2** (todo el margen pasaba a fosa), el mar quedaba en el dato y la tierra a 921 m, pero la fosa medía ×1,55 y el área que se hunde +50 %.
   - **No es cosa de la erosión.** Bajar la tierra a fuerza de erosión sería compensar una irrealidad con otra.
   - **Dónde está ese fondo** (`diagnostico_fondo_viejo`, 5 semillas):
     - del 44 al 68 % está a menos de 300 km de la orilla continental de su propia placa;
     - del 0 al 1 % está en placas sin fosa;
     - del 21 al 48 % está a más de 3 000 km de la fosa de su placa;
     - la placa mayor (22–43 % del planeta) guarda del 32 al 71 %.
2. **Los choques destruyen 2,9 ± 1,0 AU.**
   - Es lo que hace la Tierra de hoy en pleno choque India–Asia (~3 AU), pero en el modelo es permanente. La media terrestre a largo plazo es de 0,4–0,7 AU.
   - El borde de choque del modelo mide 8 500 km; el de Bird (CCB), 21 200.
   - Es el mayor sumidero del volumen continental, y probablemente el origen del relieve y de la exportación de más: 2,5 frente a 1,8 AU.
   - También explica que el planeta haga columnas de más de 100 km con el `D_0` de India–Asia.
   - No estaba en la lista de calibración: `η_col` es medido.
3. **El sedimento que llega a las fosas es poco:** 0,49 AU subducidas y 0,12 acrecionadas, frente a 1,0–1,65 y 0,3. De los 2,5 AU que salen al mar, la mayor parte se queda en el fondo junto a los márgenes pasivos, el mismo fondo viejo del punto 1.
4. **La delaminación queda en 0,71 AU** (≈1,1, objetivo débil).
5. **Corteza máxima de toda la deriva, 101 km.** Son agujas pasajeras de sumar muestras (islas y fusiones) y columnas de los choques que un `D_0` bajo tarda en aplanar. La máxima al final queda en 76 km.
6. **Sedimento máximo de una muestra: 44–61 km** (en la Tierra, unos 20 km). Son cuencas cerradas llenas hasta su desagüe y muestras ocultas bajo otras. Hay que revisarlo.

## 37. Coste (2026-11-07 c)

- El planeta tarda 181,3 s de 200 con la génesis recalculada (la génesis, el 92 %). Con `D_0` = 1 600 tardaba 171–176 s, y antes de esta ronda, 187,6 s.
- Con cinco planetas a la vez, cada uno tarda 180–204 s (comparten la máquina).
- La red compacta del flujo ahorra un 22 % de su coste (§32). Al quitar las pilas de sedimento desaparecen las agujas que forzaban subpasos diminutos.

## 38. Pendientes (2026-11-07 c, en orden; sustituida por §45)

1. **Decisión del responsable: cómo se cierra un océano de tipo Atlántico.** El fondo viejo pegado a márgenes pasivos no se consume con la v3 (§36.1), y de eso salen el nivel del mar y el relieve frente a ETOPO. Con la v2 la edad cuadraba, pero la fosa y el área no. Faltan los mecanismos de la Tierra: la propagación de una fosa a lo largo del margen o la invasión de la subducción desde el océano vecino (Antillas Menores, Scotia, Gibraltar).
2. **Los choques destruyen de 4 a 7 veces la media terrestre** (§36.2): cuánto duran y a qué velocidad convergen (`c_col`, suturas).
3. Agujas de sumar muestras (islas y fusiones) y sedimento máximo de 44–61 km.
4. Volver a calibrar la erosión con la exportación cuando cambie la base (1 y 2).
5. Lluvia: reparto por latitudes (nota propia); barlovento con objetivo ~1,2 (CHELSA).
6. Acortamiento andino, ciclo de Wilson, flexión de la litosfera, microplacas; casquetes polares.

## 39. Cómo funcionaba el tirón al morir una fosa por choque (antes de la v4)

El responsable lo pidió antes de la rotura de la losa. No había losa colgando: el tirón desaparecía en el mismo paso en que llegaba el continente.
- **Celda a celda:** un borde con corteza continental a los dos lados es un choque (`clasificar_bordes`). El tirón solo se aplicaba en bordes de fosa activa, así que ese borde dejaba de tirar y en su lugar actuaba el acoplamiento del choque (`c_col`).
- **El tramo entero:** cuando la mitad de la orilla que se hunde flotaba, la fosa del tramo moría por choque al final del paso, aunque le quedara fondo oceánico. Volvía a difusa con X = 0 y podía renacer a los X_ini.
- **Calibración:** `c_col` estaba calibrado con esa rotura instantánea (un choque frena a la India a un tercio).

## 40. Nota de fosas v4: implementación (2026-11-07 d)

**Rotura de la losa (6.6).**
- El estado de cada borde de fosa guarda el tiempo desde el inicio del choque (`choque_myr`) y la edad de la losa (`edad_fondo_myr`).
- Un tramo choca cuando la mitad de su orilla que se hunde flota. Un borde que ya es continente contra continente también choca.
- Mientras la losa cuelga, la fosa sigue activa: consume el fondo que le queda y su losa tira también en los bordes de choque, junto al acoplamiento de `c_col`, con la edad de la losa del inicio del choque.
- A los `t_rot = 10 + 15·min(1, a/100)` Myr la losa se rompe. Se apunta `rotura_losa` (con el tiempo desde el inicio del choque) y la muerte por choque. El tramo vuelve a difuso desde X = 0 y puede renacer.
- **Decisiones de quien implementa:**
  - Toda la zona de choque comparte reloj y edad de losa (los más avanzados de sus bordes vecinos), para que se rompa a la vez.
  - La losa rota queda marcada en sus bordes. Un borde de choque nuevo junto a ella ya no cuelga, pero uno que nace junto a la fosa viva de los lados sí: la rotura avanza a lo largo del frente, como un desgarro.
  - La edad de la losa es la del fondo más viejo junto a cada celda de la orilla. La de las celdas del borde mismo engaña, porque suelen ser corteza recién puesta al rellenar huecos.
  - El reloj de un choque vecino que hereda alguna celda no hace chocar a todo el tramo. Así la parte oceánica de la fosa sigue hundiéndose, como el Tetis a los lados de la India. Al principio esto fallaba: las fosas vivían ~30 Myr y salían ~600 roturas por 100 Myr.
- **India–Asia:** la losa del Tetis (88 Myr) tira 21–25 Myr tras el choque. Mientras cuelga, la India avanza a ~6,5 cm/año (la mitad de antes del choque); tras romperse, a ~4 cm/año (un tercio, como en la calibración de `c_col`). El espesor máximo sigue en 69–72 km (con `D_0` = 50) y la meseta se ensancha de ~280 a ~380 km.

**Invasión (6.4).** Cada paso, después de avanzar los tramos (`invadir`).
- **Extremo de una fosa:** un borde convergente de fosa activa, sin choque y que converge en el paso, con algún borde vecino de su placa fuera de la fosa.
  - Las transformantes que heredan el estado de la fosa de al lado (la herencia es a dos anillos) no cuentan como fosa.
  - El borde de un choque no es un extremo: la losa sigue a los lados.
- **Por una transformante de la placa que se hunde:**
  - Avanza si su lado es oceánico y de más de `a_inv` (60 Myr) y se hunde bajo un continente, un arco o un fondo más joven.
  - El frente avanza celda a celda desde el extremo (camino de coste 0–1: lo que ya tiene el estado de la fosa no cuesta).
  - Avanza `n_paso` celdas por paso, las que pasa `v_inv·t` de un múltiplo de la celda al siguiente. Así la media es exacta sin guardar nada por frente.
  - El tramo nuevo hereda la losa y la edad de la fosa.
- **Por un margen pasivo:**
  - Un margen pasivo es el borde de un continente o un microcontinente de su placa, de al menos `(3·L_borde)²` (~0,2 Mkm²; Madagascar, 0,59), con fondo oceánico suyo de más de `a_inv` al lado.
  - No lo son ni una celda continental suelta ni un arco de islas: el primero hizo saltar una placa de 3,9 Mkm² en una unión triple; el segundo, una de 11 Mkm².
  - El frente avanza desde las celdas de margen más cercanas a un extremo (a menos de dos anillos), como mucho `n_paso` celdas nuevas por paso y frente. Se guarda en la placa (`invadidas`) y no mueve nada: la placa aún no se ha roto ahí.
  - Cada `√A_min` (~1 600 km) de margen invadido, o cuando el margen viejo se acaba, se intenta separar el océano de delante como placa nueva (`partir_por_fosa`). La fosa ocupa el tramo invadido, con el océano bajo el continente y la losa de la fosa que invade.
  - Un frente que ninguna fosa toca en `τ_muerte` se olvida.
- **Solo invade una fosa que se hunde de verdad** (que converge en el paso): si no, una fosa moribunda seguía viva rasgando las transformantes de sus extremos.

**Nacimiento espontáneo:** solo en transformantes con contraste de edad (la parte de los márgenes se retira). Lo que se separa por invasión se cuenta en `nacimientos[1]`.

**De paso:**
- **Huecos del mundo tras un evento de placas.** `recalcular_mundo` solo mueve las muestras, así que una celda podía quedarse sin muestra hasta el paso siguiente. Era un fallo latente, que la v4 destapó en la prueba sin cratones. Ahora la celda ve la muestra de su primera vecina con corteza (`tapar_huecos_del_mundo`), sin crear corteza.
- **Salida del sedimento:** solo es mar abierto el océano mundial, la mayor masa de mar conectada (se retira `area_mar_abierto_km2`).

**Pruebas** (nota v4 §10), en verde:
- 5: una fosa invade un margen pasivo, avanza a `v_inv` y separa su océano conservando la corteza.
- 5b: un margen sin fosa cerca no se hunde nunca, ni con `λ_esp` = 1.
- 6: en India–Asia, la losa tira tras el choque y se rompe a los `t_rot`.
- India–Asia adaptada: el frenazo a menos de la mitad se mide tras la rotura.
- El paso cinemático de las pruebas lleva ya la invasión y los nacimientos espontáneos.

## 41. Calibración de `v_inv` y `a_inv` (5 semillas, 2026-11-07 d)

**Primera versión** (el océano de delante de un margen invadido solo se separa si llega a `A_min` = 2,55 Mkm²):

| `v_inv` / `a_inv` | edad del fondo; > 180 Myr | márgenes: vida de los que acaban / edad media al final / máxima | separaciones por 100 Myr | pérdida en choques, con losa + sin ella (AU) | ΔV_cont |
|---|---|---|---|---|---|
| 0 | 89; 24 % | 119 / 220 / 640 | 0 | 2,71 + 1,65 | −22 % |
| 50 / 60 | 96; 27 % | 122 / 230 / 648 | 0,4 | 2,57 + 1,49 | −21 % |
| 150 / 60 | 86; 17 % | 119 / 218 / 642 | 1,8 | 2,70 + 1,62 | −22 % |
| 50 / 40 | 90; 21 % | 124 / 223 / 647 | 0,6 | 2,79 + 1,81 | −24 % |

- Los frentes avanzan (1 000–2 200 km/Myr de margen en todo el planeta), pero el 95 % de los intentos de separar fallan y casi todo el frente acaba olvidado.
- La causa: el océano que queda delante de un tramo invadido es una franja de ~270 km (0,02–1,4 Mkm² para tramos de 300–5 800 km).
- Además, al repartir el océano entre márgenes contaban también los trozos continentales sueltos y los arcos del océano. Ahora solo cuentan los continentes.

**Versión con microplacas** (decisión de quien implementa). El océano separado puede ser una microplaca desde `(3·L_borde)²` (~0,2 Mkm², como Juan de Fuca), que no se funde con su vecina mientras tiene una losa que tira; se intenta separar cada `3·L_borde` de margen invadido. Una franja unida a la placa que se hunde se movería con su giro y se deslizaría a lo largo de la costa en vez de hundirse.

| `v_inv` / `a_inv` | edad del fondo; > 180 Myr | márgenes: vida / edad media al final | separaciones por 100 Myr | pérdida en choques (AU) | ΔV_cont | fosa (km) |
|---|---|---|---|---|---|---|
| 25 / 60 | 89; 19 % | 117 / 212 | 18 | 2,40 + 1,14 | −15 % | 68 400 |
| 50 / 60 | 101; 27 % | 112 / 238 | 40 | 1,88 + 1,13 | −13 % | 62 800 |
| 150 / 60 | 93; 22 % | 91 / 239 | 71 | 1,97 + 0,82 | −8 % | 68 400 |
| 50 / 100 | 89; 22 % | 111 / 213 | 27 | 2,42 + 1,21 | −16 % | 67 800 |

- **Las microplacas mejoran el balance:** la pérdida en choques baja de ~4,3 a 2,8–3,6 AU y ΔV_cont de −22 % a −8/−16 %.
- **`v_inv` y `a_inv` apenas mueven lo que debían calibrar.** Ni la edad del fondo (89–101 Myr) ni la de los márgenes que quedan (~210–240 Myr de media y 630–650 de máxima, frente a 104 y ~180). La vida de los que acaban es de 91–124 Myr con cualquier valor, también sin invasión (119): la mayoría no los acaba una invasión.
- **Valores:**
  - `v_inv` = 25 km/Myr: la vida de los márgenes que acaban más cercana al objetivo (117 Myr). Es físico: las Antillas Menores y Escocia avanzaron ~1 000–1 500 km en 40–50 Myr.
  - `a_inv` = 60 Myr (el supuesto de la nota).
  - `λ_esp` = 4·10⁻⁶ (solo transformantes: 0,2–0,6 nacimientos por 100 Myr, raros).

## 42. Choques frente a Bird CCB (solo diagnóstico, sin tocar `c_col` ni `η_col`)

Bird CCB (`bordes_bird.py`, ahora también para esta clase): 23 000 km a 1,87 cm/año de media (el 64 % por debajo de 2 cm/año), es decir, 0,43 km²/año de convergencia.

| | modelo (5 semillas) | Bird CCB |
|---|---|---|
| Longitud | 8 700 km | 23 000 km |
| Convergencia media | 1,89 cm/año | 1,87 cm/año |
| Área que converge | ~0,16 km²/año | 0,43 km²/año |
| Duración, ponderada por la corteza solapada | 71 Myr | choques activos de decenas de Myr (India–Asia ~50–55, Arabia–Eurasia ~25–35) |
| Pérdida de corteza | 3,5 AU: 2,4 con la losa colgando (68 %) y 1,1 tras la rotura (32 %) | 0,4–0,7 de media; ~3 en un gran choque |

- **El modelo no choca de más.** Converge a la velocidad de la Tierra en un borde más corto: menos área que la Tierra de hoy.
- **Lo que sobra es la pérdida.** La regla del modelo (se pierde la mitad de la corteza que se solapa, `η_col` = 0,5) aplicada a la convergencia de Bird CCB daría 7–9 AU para la Tierra de hoy, de 10 a 20 veces la media de largo plazo.
- **Lectura:** el 50 % de India–Asia (la corteza que «falta») no se puede aplicar a toda la convergencia continental. Parte se acorta y se extruye, no se pierde.
- **La rotura de la losa** reparte la pérdida: dos tercios ocurren mientras la losa cuelga. Con la rotura instantánea de antes, las fosas morían al instante y la pérdida era de ~2,9 AU.

## 43. Tabla de cierre con la v4 (5 semillas, planeta entero)

| | Modelo | Tierra u objetivo | |
|---|---|---|---|
| Tierra emergida | 0,321 ± 0,032 | 0,284 | ✗ |
| Altura media de la tierra | 1 207 ± 151 m | 671 m | ✗ |
| Tierra por debajo de 200 m | 0,037 ± 0,004 | 0,283 | ✗ |
| Pendiente media (±25 km) | 0,0120 ± 0,0014 | 0,0052 | ✗ |
| Anchura de la plataforma | 4,7 ± 0,4 km | 42,8 km | ✗ |
| Corteza máxima al final / en toda la deriva | 75 ± 4 / 99 ± 11 km | 70–80 km | ✓ / ✗ |
| Edad media del fondo al final; > 180 Myr | 112 ± 23 Myr; 18,9 % | 64,2; < 1 % | ✗ |
| `f_cont`; cambio de volumen | 0,313; −14,9 ± 5,4 % | 0,40; ±10 % | ✗ |
| Velocidad de las placas | 4,55 ± 0,36 cm/año | 4,39 | ✓ |
| Sedimento al mar | 2,56 ± 0,47 AU | ≈1,8 | ✗ |
| Longitud de fosa | 68 400 ± 7 100 km | ~51 000 ± 25 % | ✗ (×1,34) → ampliación futura |
| Área que se hunde | 3,55 ± 0,52 km²/año | ≈2,7 | ✗ (×1,31) → ampliación futura |
| Vida media de los márgenes pasivos que acaban | 117 ± 10 Myr | 130–180 | ✗ |
| Edad de los márgenes pasivos al final: media / máxima | 212 / 650 Myr | ~104 / ~180 | ✗ |
| Choques: longitud, convergencia, duración | 8 700 km; 1,89 cm/año; 71 Myr | CCB 23 000 km; 1,87 | — |
| Choques: pérdida con la losa colgando / tras la rotura | 2,40 / 1,14 AU | — | — |
| Tiempo de cálculo del planeta | 170,9 s solo; 191–203 s con cinco a la vez | 200 s | ✓ |

## 44. Lo que no cuadra, y su causa (v4)

1. **El fondo y los márgenes viejos no se consumen.**
   - Al final quedan márgenes pasivos de 650 Myr: los del principio de la deriva, a los que no llega ningún extremo de fosa.
   - La invasión solo avanza desde los extremos de las fosas, y por un margen solo hasta que se acaba su fondo viejo.
   - En la Tierra los márgenes actuales tienen como mucho ~180 Myr porque todos nacieron al romperse Pangea; los anteriores se cerraron al formarse.
   - El planeta del modelo no ha pasado por un ciclo así (no forma un supercontinente y lo rompe). Con el fondo viejo, el mar queda bajo y el relieve igual que en la v3.
2. **La pérdida de los choques** (§42): la regla de `η_col` aplicada a toda la convergencia continental pierde de más.
3. **Fosa y área que se hunde** (×1,34 y ×1,31): suben con la losa que cuelga tras el choque y con las microplacas que separa la invasión. Van a la ampliación futura, como pidió el responsable.

## 45. Pendientes (2026-11-07 d, en orden)

1. **Decisión del responsable: el ciclo de Wilson.** Cómo se cierran los océanos con márgenes de cientos de Myr que ninguna fosa alcanza, o si el estado inicial (márgenes y fondo de partida) debe ser otro. De eso dependen el nivel del mar y el relieve.
2. **Decisión del responsable: la regla de la pérdida en los choques** (`η_col` sobre toda la convergencia continental frente a 0,4–0,7 AU), con el diagnóstico de §42.
3. **Ampliación futura:** longitud de fosa y área que se hunde.
4. Agujas de sumar muestras (islas y fusiones) y sedimento máximo de 35–65 km.
5. Volver a calibrar la erosión con la exportación cuando cambie la base.
6. Lluvia: reparto por latitudes (nota propia); barlovento con objetivo ~1,2 (CHELSA).
7. Acortamiento andino, flexión de la litosfera, retroceso de fosas y cuencas tras el arco; casquetes polares.


## 46. La Tierra real con la regla del modelo (2026-11-07 e)

Pregunta del responsable, antes de cambiar nada: si se pone la Tierra real en la regla del modelo, ¿sale su nivel del mar? Herramienta: `herramientas/analisis/tierra_regla_modelo.py`, con salida en `config/referencias/tierra_nivel_mar.toml`.

**Cómo se mide.**
- Entradas:
  - las edades del fondo de Seton et al. (2020);
  - el grosor del sedimento de GlobSed (Straume et al. 2019), de la copia de CaltechDATA (NOAA retiró la página en mayo de 2025);
  - el volumen de agua del modelo (`gel_m` = 2 617 m).
- Celdas de nivel 3 (~26 km).
- El fondo con edad (57,4 % del planeta) toma la curva de profundidad del modelo (Parsons y Sclater: `t_x` 20, `d_∞` 6 400 m, `τ` 62,8 Myr) y la isostasia de su sedimento: sube `S·(ρ_m − ρ_s)/(ρ_m − ρ_w)`, con ρ_s = 2 400 (0,40·S).
- El resto conserva la roca real sin hielo con su rebote.

| | Nivel del mar | Profundidad media del océano | Mar |
|---|---|---|---|
| Tierra (roca sin hielo con rebote), a nivel 0 | 0 | 3 667 m | 71,6 % |
| Tierra con el volumen de agua del modelo | −14 m | 3 683 m | — |
| **Regla del modelo** | **−386 m** | **3 992 m** | 65,5 % |
| Regla sin sedimento | −579 m | 4 051 m | 64,6 % |
| Regla con ρ_s = 2 000 (sedimento pelágico poco compactado) | −302 m | 3 968 m | 66,0 % |
| Regla solo en el fondo de menos de 80 Myr | −134 m | 3 882 m | 67,4 % |
| GDH1 (Stein y Stein 1992) | −340 m | 3 978 m | 65,8 % |
| Raíz de la edad sin límite | −523 m | 4 036 m | 64,8 % |
| Placa ajustada a la **mediana** del zócalo real: `d_∞` 5 310 m, `A` 2 373 m, `τ` 31 Myr (rms 62 m) | −150 m | 3 893 m | 67,2 % |
| Placa ajustada a la **media** del zócalo real: `d_∞` 5 070 m, `A` 1 809 m, `τ` 34 Myr (rms 85 m) | −25 m | 3 706 m | 70,6 % |

Las curvas ajustadas conservan la raíz del modelo hasta `t_x` = 20 Myr (`2 500 + 350·√t`, que ya coincide con el fondo joven real). Después son `d_∞ − A·e^(−t/τ)`, continuas en `t_x`, con `d_∞` y `τ` por mínimos cuadrados ponderados por área en las clases de 20 a 180 Myr. Con una curva ajustada, la densidad del sedimento apenas cambia el nivel del mar: la misma isostasia descarga el zócalo al medirlo y lo carga en la regla.

Zócalo (el fondo descargado de su sedimento) por edad, en m:

| Edad (Myr) | Área del planeta | Mediana | Media | Sedimento medio | Modelo | GDH1 | Raíz |
|---|---|---|---|---|---|---|---|
| 0–10 | 5,9 % | 3 308 | 3 242 | 145 | 3 283 | 3 416 | 3 283 |
| 20–30 | 5,7 % | 4 214 | 4 150 | 248 | 4 251 | 4 417 | 4 250 |
| 40–50 | 4,2 % | 4 727 | 4 569 | 365 | 4 837 | 4 943 | 4 848 |
| 60–70 | 3,9 % | 5 023 | 4 788 | 448 | 5 263 | 5 245 | 5 322 |
| 80–90 | 2,9 % | 5 165 | 4 963 | 548 | 5 573 | 5 418 | 5 727 |
| 100–110 | 2,6 % | 5 315 | 5 158 | 935 | 5 799 | 5 517 | 6 086 |
| 120–130 | 2,5 % | 5 205 | 4 950 | 1 120 | 5 963 | 5 574 | 6 413 |
| 140–150 | 1,4 % | 5 266 | 4 982 | 1 210 | 6 082 | 5 607 | 6 715 |
| 160–170 | 0,8 % | 5 012 | 4 674 | 1 040 | 6 169 | 5 626 | 6 996 |

**Lo que sale.**
1. El volumen de agua del modelo es el de la Tierra: con la roca real, el nivel queda a −14 m.
2. La regla del modelo hace las cuencas demasiado hondas incluso con las edades reales: el nivel baja 386 m y la profundidad media sube a 3 992 m.
3. **El zócalo real deja de bajar hacia los 80 Myr.** Entre 70 y 180 Myr, la mediana se queda en 5,0–5,4 km y la media en 4,7–5,2 km, sin tendencia.
4. **La curva del modelo sí se aplana, pero tarde y hondo.**
   - No sigue la raíz de la edad sin límite: la raíz daría −523 m.
   - Sigue bajando 556 m entre 80 y 150 Myr.
   - En el fondo de más de 100 Myr queda 0,6–1,5 km más honda que la media real.
5. Hasta los 30 Myr, la curva del modelo coincide con la mediana real (±40 m).
6. Dos tercios del desfase vienen del fondo de más de 80 Myr: con la regla solo en el más joven, el nivel queda a −134 m.
7. GDH1 apenas mejora (−340 m). Aplana a 5,65 km, pero en el fondo joven es 100–200 m más honda que la Tierra.
8. La densidad del sedimento pesa unos 85 m: con 2 000 kg/m³, que es la del sedimento pelágico de su primer kilómetro, el nivel queda a −302 m.
9. En el fondo viejo, la media real es 200–300 m más somera que la mediana. Son las mesetas, los montes submarinos y los abombamientos de los puntos calientes, que el modelo no tiene.

10. **Con una curva de placa ajustada a la mediana real, el desfase baja de −386 a −150 m.** Los 125 m que quedan hasta la media (−25 m) son el volumen de las mesetas, los montes submarinos y los abombamientos de los puntos calientes, que el modelo no tiene.

**Consecuencia para el modelo.** El océano del modelo es viejo: 112 Myr de media al final de la v4, frente a 64 en la Tierra. Con la curva de Parsons y Sclater, el fondo viejo sigue hundiéndose y agranda el problema de la edad. Con el aplanamiento real, un fondo de 150 Myr estaría casi a la misma profundidad que uno de 90.

## 47. Adenda v7: la pérdida en los choques según el grosor (implementación)

**Regla.**
- `η_col` pasa a ser `η(h) = η_min + (1 − η_min)·clamp((h − h_1)/(h_2 − h_1), 0, 1)`, en `TectonicaCfg::eta_choque`.
- Parámetros: `eta_min` 0,5 (inicial; se calibra dentro de 0–0,5), `h1_choque_km` 20 y `h2_choque_km` 35.
- `h` es el espesor de la corteza cristalina (`h`) de la muestra de la placa de abajo que se consume en el solape: toda la de un continente; en el fondo oceánico, sus 7 km y lo que han añadido los arcos o la acreción.
- Los arcos siguen con `η_arco` = 0,9.
- La `η` de cada choque viaja con él (`Choque.eta`) para la cuenta del engrosamiento.

**No regresión.** Con `h_1 = h_2 = 10⁹`, la regla es la vieja (η = 0,5 siempre). Así, el planeta de `config/terreno.toml` sale idéntico byte a byte al de 524153b (5,57 MB).

**Prueba nueva.** `en_un_choque_se_pierde_la_corteza_fina_y_se_queda_la_gruesa`.

**Consecuencia: falla `un_gran_choque_no_apila_corteza_por_encima_de_80_km`.**
- En esa prueba, la India es un bloque uniforme de 35 km. Con la v7 tiene η = 1, así que se queda entera y apila en Asia el doble que antes.
- El máximo llega a 89,0 km; el límite de la prueba es 88 (80 más un 10 % por la resolución).
- Causas posibles, que decide el responsable:
  - la base del escenario: la India del escenario no tiene el margen norte adelgazado que tuvo la Gran India real;
  - `τ_del` y `D_0`, que se calibraron con la mitad de la corteza perdida.
- No se toca la prueba ni ningún parámetro antes de calibrar.

## 48. Nota de fosas v5: zonas de fractura (implementación)

**Detección (cada paso).**
- Pares de celdas vecinas de lado de una misma placa, las dos de fondo oceánico, con un escalón de edad de más de `Δa_fz` (`da_fz_myr` = 10).
- Son candidatos los que cumplen la regla de las transformantes: lado viejo de más de `a_min` y escalón de más de la mitad de su edad.

**Sorteo.**
- Por placa y bloque de `L_borde`, con `p = 1 − exp(−λ_esp·((a − a_min)/100)·L·Δt)`, como las transformantes.
- Tiene su propia corriente del generador (`P_ESPONTANEO_ZF`).

**Nacimiento.**
- La fosa sigue el escalón: todos los pares candidatos de la placa conexos con los del bloque.
- La placa se parte por él (`partir_por_zona_fractura`):
  - cada celda oceánica va con el lado del escalón que tiene más cerca por el océano de la placa;
  - más allá de los extremos, el corte sigue la dirección del escalón hasta los bordes existentes. Una zona de fractura es la traza de una transformante y la continúa (decisión de quien implementa: «la misma maquinaria que la invasión», que también corta por la fuente más cercana);
  - cada continente va entero con el lado que más lo rodea.
- El lado viejo se hunde y puede ser una microplaca, desde (3·L_borde)². El joven queda arriba y tiene al menos `A_min`.
- La fosa se pone en los bordes del escalón con `L = X_ini`; el resto del corte es transformante.
- Una placa se parte una vez por paso como mucho.

**Métricas.**
- Por paso: `nacimientos_zf`, `separar_zf` (intentos y fallidos), `zf_escalon_km` y `zf_candidata_km`.
- En el balance: `nacimientos_zf_100myr`, `separar_zf_fallidos_frac`, `zf_escalon_km` y `zf_candidata_km`.

**Pruebas nuevas.**
- `una_zona_de_fractura_con_fondo_viejo_hace_nacer_una_fosa_y_parte_la_placa`: 200 frente a 50 Myr. Nace la fosa, la placa se parte, el lado viejo se hunde y tira, el continente va entero con el joven y la corteza se conserva.
- `un_escalon_pequeno_en_fondo_viejo_no_hace_nacer_ninguna_fosa`: 200 frente a 160 Myr es zona de fractura, pero no candidata.

**La regla frente a la Tierra.** Escalones de edad entre celdas vecinas de 52 km en Seton (`tierra_nivel_mar.toml`, sección `zonas_fractura`). Toda la Tierra suma 1,31·10⁷ km de lados con edad.

| Escalón | Longitud |
|---|---|
| Más de 5 Myr | 798 310 km |
| Más de 10 Myr | 200 255 km |
| Más de 20 Myr | 39 770 km |
| Más de 40 Myr | 8 027 km |
| Más de 70 Myr | 1 616 km |
| **Regla v5** (viejo > 140 Myr, contraste > ½·viejo, escalón > Δa_fz) | **417 km** |
| Sin el contraste (viejo > 140 Myr, escalón > Δa_fz) | 18 347 km |
| Con `a_min` = 100 y el contraste | 2 189 km |

**Lo que sale.**
- Con `a_min` = 140 Myr y el contraste de más de la mitad de la edad vieja, todo candidato tiene ya un escalón de más de 70 Myr. Así, `Δa_fz` (5–20) nunca decide nada y no se puede calibrar.
- En la Tierra real, la regla deja 417 km de candidatos en todo el planeta: unas 8 celdas.
- Las zonas de fractura reales del fondo viejo separan fondos con decenas de Myr de diferencia, no con más de la mitad de su edad.
- Además, con el aplanamiento real (§46), dos fondos de más de 80 Myr apenas difieren en profundidad (ni en densidad).

**No regresión.** Con `Δa_fz` = 10⁹ (sin zonas de fractura) y la v7 apagada, la salida del planeta es igual a la de 524153b, sin contar las claves nuevas de las métricas. Se comprobó leyendo los dos JSON.

## 49. Lo que hacen la v7 y la v5 en el planeta (5 semillas, sin calibrar)

Valores iniciales: `η_min` 0,5, `λ_esp` 4·10⁻⁶ y `Δa_fz` 10. «Las dos apagadas» es la v4.

| | Las dos apagadas (v4) | Solo v7 | Solo v5 | Las dos | Las dos, `η_min` = 0 | Tierra u objetivo |
|---|---|---|---|---|---|---|
| Pérdida en choques (AU) | 3,54 | 0,97 | 3,57 | 1,42 | 2,10 | 0,4–0,7 |
| Delaminación (AU) | 0,67 | 2,30 | 0,55 | 1,88 | 1,51 | ≈1,1 |
| Sedimento al mar (AU) | 2,56 | 3,86 | 2,16 | 3,25 | 2,94 | ≈1,8 |
| Celdas·paso de más de 80 km | 2 655 | 75 581 | 2 830 | 61 233 | 58 618 | 0 |
| Muestra más gruesa de la deriva (km) | 146 | 221 | 143 | 205 | 205 | 70–80 |
| Edad media del fondo en la deriva (Myr) | 89 | 95 | 71 | 71 | 73 | 64 |
| Fondo de más de 180 Myr al final | 19 % | 24 % | 7 % | 9 % | 12 % | < 1 % |
| Márgenes pasivos: vida de los que acaban (Myr) | 117 | 115 | 97 | 94 | 99 | 130–180 |
| Márgenes al final: edad media / máxima (Myr) | 212 / 650 | 237 / 646 | 147 / 613 | 159 / 620 | 165 / 628 | ~104 / ~180 |
| Longitud de fosa (km) | 68 400 | 62 300 | 86 500 | 80 900 | 79 700 | ~51 000 |
| Área que se hunde (km²/año) | 3,55 | 3,18 | 4,72 | 4,25 | 4,20 | 2,7–3,4 |
| ΔV_cont | −15 % | −12 % | −8 % | −6 % | −8 % | ±10 % |
| `f_cont` | 0,313 | 0,322 | 0,338 | 0,355 | 0,347 | 0,40 |
| Nacimientos en zonas de fractura (por 100 Myr) | — | — | 11,4 | 13,0 | 11,3 | — |
| Candidatos a zona de fractura (km) | — | — | 105 700 | 111 600 | 101 700 | 417 (Tierra) |

**La v7.**
- Baja la pérdida en los choques de 3,5 a 1,0 AU, pero la corteza que se queda se apila. Las celdas de más de 80 km se multiplican por 28.
- La delaminación sube a 2,3 AU, el doble de su objetivo, y el sedimento al mar a 3,9 AU.
- `τ_del` y `D_0` se calibraron con la mitad de la corteza perdida.
- Con `η_min` = 0,5, el techo del rango, la pérdida es de 1,0 AU (1,4 con la v5). El objetivo de 0,4–0,7 AU no se alcanza dentro de 0–0,5.

**La v5.**
- Hace lo que se buscaba: el fondo rejuvenece (71 Myr de media en la deriva, 7 % de más de 180 al final) y también los márgenes (147 Myr de media al final).
- Pero la fosa sube a 86 500 km (×1,7 de la Tierra) y el área que se hunde a 4,7 km²/año.
- Y los márgenes que acaban viven menos (97 Myr).

**Qué son los candidatos a zona de fractura en el modelo** (`diagnostico_zonas_fractura`, semillas 42 y 1, cada 100 Myr).
- Crecen con la deriva: 19 000–77 000 km a los 100 Myr y 85 000–160 000 km a los 300–500.
- El lado joven tiene sobre todo 20–100 Myr y el viejo 140–300.
- Entre el 40 y el 75 % de su longitud está en líneas de más de 1 000 km.
- Solo el 1–8 % son celdas jóvenes sueltas («agujeros») y el 5–15 % están junto a otra placa.
- **Son las costuras del fondo viejo del propio modelo:** fondos de 140–300 Myr pegados a fondos jóvenes dentro de una placa.
- La Tierra casi no tiene fondo de más de 180 Myr ni escalones así: 1 616 km de más de 70 Myr en todo el planeta.
- Así, la v5 cierra el océano viejo del modelo por un camino que en la Tierra no existe: el disparo lo pone la propia irrealidad que se quiere corregir.

**Qué corteza entra en los choques** (nuevas cuentas `choque_solapado_por_grosor_km3` y `choque_perdida_por_grosor_km3`, sin la de arco; fracción de toda la deriva, 5 semillas, `η_min` = 0,5):

| Corteza que entra | Solapada (las dos / solo v7) | Perdida (las dos / solo v7) |
|---|---|---|
| Continental de menos de 20 km | 0,1 % / 0,1 % | 0,4 % / 0,4 % |
| Continental de 20–35 km | 14 % / 14 % | 18 % / 25 % |
| Continental de más de 35 km | 64 % / 73 % | 0 / 0 |
| De tipo continental sobre fondo oceánico, no de arco | 22 % / 13 % | 81 % / 75 % |

- **En el modelo no hay márgenes adelgazados.** Los rifts parten los continentes sin estirar su corteza (`F_rift` es provisional, §45 y TREE). Así, al choque casi no llega corteza continental de menos de 20 km, y la regla de `η_min` no tiene sobre qué actuar.
- **La pérdida que queda, 1,0–1,4 AU, sale sobre todo de columnas oceánicas con corteza de tipo continental que no cuenta como arco (75–81 %).**
  - Es la que ponen en el fondo oceánico el engrosamiento de los choques, el flujo lateral o la absorción de la convergencia difusa.
  - Basta que entre algo así en una columna sin corteza continental para que deje de ser de arco (`sumar_hc`).
  - Su grosor (7 km más `hc`) queda por debajo de 20 km, así que pierde `η_min`.
- La corteza de grosor normal (64–73 % de lo solapado) se queda entera y se apila. De ahí las celdas de más de 80 km, la delaminación y el sedimento al mar de más.

## 50. Pendientes (2026-11-07 e, en orden; decide el responsable antes de calibrar)

1. **La curva de profundidad (§46), que es la base.** Opciones:
   - mantener Parsons y Sclater (la Tierra real daría −386 m);
   - GDH1 (−340 m);
   - la placa ajustada a la mediana real, que es el fondo normal medido (`d_∞` 5 310 m, `A` 2 373 m, `τ` 31 Myr; −150 m). Los 125 m que faltan son las mesetas, los montes submarinos y los puntos calientes, mecanismos que faltan;
   - la placa ajustada a la media real (−25 m), que mete esos relieves en la curva.
   - **Recomendación:** la mediana, con las mesetas y los puntos calientes en la lista de lo que falta.
2. **La regla de la v5 (§48–§49).**
   - `Δa_fz` es redundante con el contraste de ½·viejo y `a_min` = 140, así que no se puede calibrar.
   - En la Tierra la regla deja 417 km de candidatos; en el modelo, 100 000–160 000 km.
   - Esos candidatos son las costuras de su fondo viejo, que la Tierra no tiene. La v5 rejuvenece el océano, pero alarga la fosa a ×1,7 de la Tierra.
3. **La regla de la v7 sobre la base actual (§47, §49).**
   - Sin márgenes adelgazados no se alcanza el objetivo de 0,4–0,7 AU con `η_min` en 0–0,5.
   - La corteza normal que se queda se apila (×28 celdas de más de 80 km).
   - La prueba India–Asia pasa de 80 km: su India no tiene margen adelgazado.
   - Hacen falta dos decisiones:
     - si los rifts deben estirar y adelgazar la corteza de los márgenes;
     - qué `η` corresponde a la corteza de tipo continental sobre fondo oceánico que no es de arco.
   - Después habrá que volver a calibrar `τ_del` y `D_0` con la v7.
4. Después: calibrar `η_min`, `λ_esp` y `Δa_fz` (si sigue en la regla) con 5 semillas, y repetir la tabla de cierre. Ya tiene filas para el nivel del mar, la profundidad media, la pérdida total en los choques y los nacimientos en zonas de fractura.
5. Siguen de §45: el ciclo de Wilson y el estado inicial; la ampliación futura de fosa y área; las agujas de sumar muestras y el sedimento máximo; la erosión con la exportación; la lluvia por latitudes; el acortamiento andino, la flexión, el retroceso de fosas y los casquetes.

## 51. Decisiones del responsable y su implementación (2026-11-07 f)

**Curva de profundidad.**
- Se adopta la placa ajustada a la mediana del zócalo real: `d_∞` 5 310 m, `A_pl` 2 373 m, `τ_pl` 31 Myr, con la raíz de Parsons y Sclater hasta `t_x` = 20 Myr (§46).
- Las mesetas oceánicas, los montes submarinos y los puntos calientes quedan como mecanismo pendiente: ~150 m de nivel del mar en la Tierra real (TREE).
- La prueba de la curva comprueba que es continua y que baja menos de 200 m entre 80 y 150 Myr.

**Nota de fosas v6: se retiran las zonas de fractura.**
- Se quita su código: detección, sorteo, nacimiento y partición de la placa.
- Se quitan también `da_fz_myr`, sus métricas, sus dos pruebas, su línea del informe y su fila de la tabla de cierre.
- El nacimiento espontáneo vuelve a ser solo de transformantes.

**Procedencia de cada muestra** (diagnóstico de la v6 y de los choques; `Corteza.proc`, que ninguna regla lee).
- `nace`: cómo nació la muestra (estado inicial, dorsal, hueco de creación residual, rift estirado, continente que perdió su corteza).
- `lote`: el último evento que juntó su corteza con la de otra placa o la separó de su madre (sutura, absorción, salto de dorsal o rift; `Deriva::lotes`).
- `vias`: la corteza de tipo continental de la columna por la vía que la puso allí (arco, acreción, sedimento, choque, flujo lateral, difusa, dientes, islas, fusión, inicial, rift).
  - Se reparte en proporción cuando se quita corteza y se escala cuando una muestra cambia de celda.
  - En los choques se cuenta lo solapado y lo perdido por clase y por vía (`Deriva::diag_choque_km3`).
- Diagnóstico nuevo: `diagnostico_procedencia_del_fondo`.
- **No regresión:** sin la v5 y con la procedencia, el planeta (curva vieja y `f_rift`) es igual al de 17b9ce2 con `Δa_fz` = 10⁹, sin contar las claves retiradas de la v5. Se comprobó leyendo los dos JSON.

**Adenda v8 §6.10: rifts que estiran los bordes** (`deriva/rifts.rs`).
- Al partirse una placa por un rift, cada lado con corteza continental junto al corte apunta su franja de partida.
  - Es la corteza continental a menos de `W_0 = W_m·(h_0 + h_rot)/(2·h_0)` del corte, por las celdas de la placa.
  - `h_0` es el espesor medio de la corteza continental a menos de `W_m` del corte.
- En cada paso, la corteza nueva de la abertura junto a la franja pasa a corteza continental de la franja (`nace` = rift).
- El volumen de la franja se reparte en una rampa por la distancia al eje:
  - la franja mide `w = W_0 + e`, siendo `e` el área abierta en ese lado entre la longitud del rift;
  - el espesor junto al eje es `h_0·(2·W_0/w − 1)`, y como mínimo `h_rot`.
- Solo se mueve corteza entre las muestras de la franja, en orden fijo, así que el volumen se conserva exactamente.
- Cuando `e` llega a `W_m − W_0`, ese lado se rompe y lo que se abre después es fondo oceánico.
- **Se retira `f_rift`** (`H ← 0,7·H` a menos de `w_margen` del corte), con su sumidero provisional `F_rift`: estirar conserva el volumen. `w_margen_km` pasa a ser `W_m` (150 km) y se añade `h_rot_km` (10 km).
- Prueba nueva: `un_rift_estira_sus_bordes_en_rampa_conservando_el_volumen_y_despues_abre_el_oceano`.
  - Durante la fase de rift no nace fondo oceánico.
  - El volumen continental cambia menos de 10⁻⁹.
  - Al romperse, la rampa va de menos de 20 km junto al océano nuevo a más de 33 km a 200–300 km.
- **La India de las pruebas de choque tiene su margen estirado:** una rampa de 35 km a `h_rot` en los `W_m` más cercanos a su borde.

## 52. Lo que la base nueva cambia en los escenarios India–Asia

Escenario de `un_gran_choque_no_apila_corteza_por_encima_de_80_km` (malla de 104 km, 50 Myr) y de `dos_continentes_chocan_frenan_y_levantan_una_cordillera` (208 km):

| | Corteza máxima | India tras romperse la losa |
|---|---|---|
| Curva vieja, India uniforme | 88,3 km | 5,7 cm/año |
| Curva nueva, India uniforme | 93,6 km | 6,0 |
| Curva vieja, India estirada | 89,6 km | 5,7 |
| Curva nueva, India estirada | 92,0 km | 6,0 |

- El margen estirado apenas cambia el máximo (±1,5 km): la mayor parte de la India tiene corteza normal y con la v7 se queda entera.
- **Lo que mueve las pruebas es la curva nueva, por el nivel del mar del escenario.**
  - El escenario tiene un 29 % de continente, un océano de 80 y 150 Myr y el agua de la Tierra.
  - Con Parsons y Sclater, su mar quedaba a −1 890 m; con la curva nueva, a −1 255 m. Asia está a +130–300 m sobre el dato.
  - Así el orógeno sobresale menos sobre el mar y se erosiona más despacio (la erosión crece con la altura sobre el mar elevada a 2,5).

**`D_0`, recalibrado contra su observación** (India–Asia, 70–80 km; en la v6 se eligió por 72 km a los 50 Myr). No es para tapar picos:

| `D_0` (km²/Myr) | Máximo | A los 50 Myr | Relajación por debajo de 38 km |
|---|---|---|---|
| 50 | 92,0 | 87,1 | 231 Myr |
| 100 | 88,8 | 81,8 | 250 |
| 200 | 84,0 | 78,6 | 243 |
| 400 | 79,5 | 75,5 | 242 |
| 800 | 75,3 | 72,4 | 239 |
| 1 600 | 71,8 | 69,8 | — |

Con `τ_del` = 10 Myr (y `D_0` = 50), 83,6 km. Pero bajar `τ_del` aleja aún más la delaminación de su objetivo: 2,3 AU con la v7 frente a ≈1,1.

**Dos pruebas quedan fuera por la base nueva, y no se tocan sus parámetros:**
1. **`la_relajacion_de_un_orogeno_tarda_entre_100_y_200_myr`: 231–250 Myr con cualquier `D_0`** (149 con la curva vieja).
   - `E_1` y `n_ero` se calibraron con esta prueba cuando el mar del escenario estaba a −1 890 m, una base irreal que inflaba la erosión.
   - Subir la erosión para recuperarla aumentaría el sedimento al mar del planeta, que ya pasa del objetivo.
   - Decide el responsable. Las opciones son:
     - revisar el objetivo: los Urales conservan raíces de ~50 km tras 250 Myr;
     - añadir el colapso posorogénico;
     - o recalibrar la erosión.
2. **`dos_continentes_chocan_frenan_y_levantan_una_cordillera`: tras romperse la losa, la India va a 5,8–6,0 cm/año frente al límite de 5,75** (la mitad de 11,5). Ya estaba al borde (5,7).
   - Mientras la losa cuelga, la India va a 3,9 cm/año, un tercio, como la India real.
   - Tras la rotura se acelera, por el giro que le dan las fosas de los lados (§40), mientras la real siguió frenando.
   - `c_col` no puede arreglar las dos fases a la vez, y el responsable pidió no tocarlo (v4).

## 53. `D_0` en el planeta (5 semillas, curva nueva, v7 y v8, `η_min` = 0,5)

| `D_0` (km²/Myr) | 50 | 400 | 800 | Objetivo |
|---|---|---|---|---|
| Celdas·paso de más de 80 km | 84 480 | 18 659 | 7 036 | 0 |
| Muestra más gruesa de la deriva (km) | 194 | 171 | 192 | 70–80 |
| Pérdida en choques (AU) | 0,98 | 1,10 | 1,06 | 0,4–0,7 |
| Delaminación (AU) | 2,59 | 2,26 | 2,44 | ≈1,1 |
| Sedimento al mar (AU) | 3,33 | 3,51 | 3,68 | ≈1,8 |
| ΔV_cont | −11,7 % | −10,2 % | −11,5 % | ±10 % |
| Edad media del fondo en la deriva; más de 180 Myr al final | 93; 22 % | 93; 22 % | 90; 22 % | 64; < 1 % |
| Márgenes que acaban: vida (Myr) | 110 | 111 | 116 | 130–180 |
| Flujo lateral en el coste de la deriva (15 derivas a la vez) | 15–18 % | 29–36 % | 49–52 % | — |

Se elige `D_0` = 400:
- en India–Asia da 79,5 km de máximo y 75,5 a los 50 Myr, dentro de 70–80;
- con 800 el flujo pasaría a la mitad del coste de la deriva, y el planeta, de los 200 s.

## 54. Qué corteza se pierde en los choques (diagnóstico (c) del responsable)

Mientras tanto se le aplica `η(h)` según su grosor (`h` = 7 km + `hc`, casi siempre `η_min`). Reparto de lo perdido en toda la deriva (5 semillas, `η_min` = 0,5; igual con `D_0` 50, 400 y 800):

| Corteza que entra | Solapada | Perdida |
|---|---|---|
| Continental de menos de 20 km | 0,2 % | 1 % |
| Continental de 20–35 km | 9–12 % | 20–21 % |
| Continental de más de 35 km | 75–79 % | 0 |
| De tipo continental sobre fondo oceánico, no de arco | 12–14 % | **78–79 %** |

**La corteza sobre fondo oceánico que se pierde, por la vía que la puso allí:**

| Vía | Fracción |
|---|---|
| Arco | 44 % |
| Engrosamiento de un choque | 28 % |
| Dientes (astillas de borde que pasan a otra placa) | 12–13 % |
| Sedimento que pasó a corteza al sumarle corteza encima | 12–13 % |
| Acreción, convergencia difusa y flujo lateral | ~1 % cada una |

- **No es sedimento acrecionado ni restos de la convergencia difusa** (~1 % cada uno). Las astillas de borde son el 12–13 %.
- **Casi la mitad es corteza de arco que dejó de contar como arco.**
  - En `sumar_hc`, una columna es de arco solo si toda su corteza continental lo es. Basta con que le llegue algo que no es de arco (un choque, un diente, la difusa) para que toda la columna cuente como continental.
  - Su grosor (7 km más `hc`) la deja por debajo de `h_1`, así que pierde con `η_min`.
  - En la Tierra, esos arcos se acrecionarían casi enteros (`η_arco` = 0,9; Luzón en Taiwán).
- **El 28 % es corteza que el engrosamiento de un choque puso en el fondo oceánico de la placa de arriba:** el frente reparte en cualquier corteza a menos de `W_min_orog`.
- **La corteza continental fina (menos de 20 km) apenas entra en los choques, también con la v8** (0,2 % de lo solapado).
  - Sí aparece la de los márgenes estirados: un 35 % de lo que se pierde de esa clase viene de los rifts.
  - Pero la mayoría de los márgenes que chocan son los del estado inicial. Sus continentes tienen espesor normal hasta la costa y nunca se estiraron.

## 55. Calibración de `η_min` y `λ_esp` (5 semillas; curva nueva, v7, v8 y `D_0` = 400)

| | `η_min` 0,5; `λ_esp` 4·10⁻⁶ | `η_min` 0,25 | `λ_esp` 4·10⁻⁵ | `λ_esp` 4·10⁻⁴ | Objetivo |
|---|---|---|---|---|---|
| Pérdida en choques (AU) | 1,10 | 1,50 | 1,15 | 0,94 | 0,4–0,7 |
| Nacimientos espontáneos por 100 Myr | 0,2 | 0,4 | 2,2 | 20,8 | raros |
| Edad media del fondo en la deriva (Myr) | 93 | 89 | 90 | 98 | 64 |
| Fondo de más de 180 Myr al final | 22 % | 19 % | 21 % | 25 % | < 1 % |
| Márgenes que acaban: vida (Myr) | 111 | 116 | 109 | 115 | 130–180 |
| Márgenes al final: edad media (Myr) | 227 | 213 | 210 | 213 | ~104 |

- **`η_min` = 0,5, el techo de su rango.** La pérdida sube al bajarlo (0,25: 1,5 AU; 0: 2,1 AU en la ronda anterior), así que 0,4–0,7 AU no se alcanza en 0–0,5. El 78 % de lo que se pierde es la corteza sobre fondo oceánico de §54, sobre todo arcos que dejaron de contar como arco.
- **`λ_esp` = 4·10⁻⁶, sin cambios.** Con 10 y 100 veces más nacimientos (2 y 21 por 100 Myr), ni la edad del fondo ni la de los márgenes se mueven más que la dispersión entre semillas (74–112 Myr de edad media). Los nacimientos en transformantes no llegan al fondo viejo del interior de las placas. Se deja el valor raro, como en la Tierra: pocos casos, como Izu-Bonin-Marianas.

## 56. De dónde sale el fondo viejo y sus escalones (diagnóstico de la nota de fosas v6)

`diagnostico_procedencia_del_fondo`, 5 semillas, al final de la deriva (500 Myr). Los escalones enormes son los de antes: fondo de más de `a_min` junto a fondo de menos de la mitad de su edad, dentro de una misma placa; 105 000–274 000 km.

**(a) Su causa**, por el lote más reciente de los dos lados, si no llegaron juntos a la placa:

| Causa | Fracción de la longitud |
|---|---|
| Relleno de huecos con fondo nuevo (creación residual, en tramos transformantes o convergentes) | 33–48 % |
| «Dorsal»: los dos lados nacieron en dorsales, en el mismo lote | 20–44 % |
| Suavizado de la pertenencia de los bordes oceánicos | 1–13 % |
| Separación de un océano por invasión | 4–12 % |
| Diente recolocado | 2–8 % |
| Isla absorbida | 0–2 % |
| Fusiones de placas (sutura, absorción) | 0–5 % |
| Rift | 0–2 % |
| Salto de dorsal | 0 % |
| Fondo del estado inicial junto a fondo nuevo | 0–8 % (71–92 % a los 100 Myr) |

- **Casi todo sale de la representación de las placas, no de procesos de la Tierra.**
  - El relleno de huecos pone fondo de edad 0 junto a fondo viejo donde no hay dorsal.
  - Los dientes, islas y suavizado mueven una muestra vieja a otro sitio.
  - Las fusiones, los saltos de dorsal y los rifts suman menos del 6 %.
- **La clase «dorsal» no tiene ningún evento marcado.** Dos fondos de dorsal del mismo linaje no deberían diferir en más de la mitad de su edad. Lo más probable son muestras ocultas bajo otra placa que reaparecen junto a fondo joven, también de la representación (sin medir).
- **El estado inicial solo pesa al principio:** su campo de edades (ruido, sin dorsales) pone fondo de 0–150 Myr junto a las dorsales iniciales.

**(b) Qué parte del fondo y de los márgenes viejos viene del estado inicial:**

| | Total | Del estado inicial |
|---|---|---|
| Fondo de más de 180 Myr | 51–109 Mkm² (10–21 % del planeta) | 0–6 % (una semilla, 21 %) |
| Márgenes pasivos de más de 180 Myr | 2 500–6 600 celdas | 1–11 % |
| Todos los márgenes pasivos | 6 400–9 950 celdas | 1–5 % |

- **El fondo y los márgenes viejos no son la herencia del estado inicial.** Los crea la deriva y sobreviven demasiado: en la Tierra, menos del 1 % del fondo pasa de 180 Myr.
- Los márgenes de 600–650 Myr que quedan sí son del estado inicial, pero son pocos.

## 57. Tabla de cierre (5 semillas, planeta entero; curva nueva, v7, v8, `D_0` = 400, `η_min` = 0,5)

| | Modelo | Tierra u objetivo | v4 (§43) | |
|---|---|---|---|---|
| Tierra emergida | 0,334 ± 0,015 | 0,284 | 0,321 | ✗ |
| Altura media de la tierra | 1 143 ± 158 m | 671 m | 1 207 | ✗ |
| Tierra por debajo de 200 m | 0,068 ± 0,006 | 0,283 | 0,037 | ✗ |
| Pendiente media (±25 km) | 0,0128 | 0,0052 | 0,0120 | ✗ |
| Anchura de la plataforma | 8,2 ± 1,3 km | 42,8 km | 4,7 | ✗ |
| **Nivel del mar al final de la deriva** | **−305 ± 144 m** | 0 (la Tierra real con la regla: −150) | −500 a −1 000 | — |
| **Profundidad media del océano** | **3 952 ± 88 m** | 3 667 m | — | ✗ |
| Corteza máxima al final / en toda la deriva | 79 ± 3 / 117 ± 14 km | 70–80 km | 75 / 99 | ✓ / ✗ |
| Edad media del fondo al final; más de 180 Myr | 119 ± 28 Myr; 21,8 % | 64,2; < 1 % | 112; 18,9 % | ✗ |
| `f_cont`; cambio de volumen | 0,318; −10,3 ± 4,6 % | 0,40; ±10 % | 0,313; −14,9 % | ✗ (por poco) |
| Velocidad de las placas | 4,40 ± 0,39 cm/año | 4,39 | 4,55 | ✓ |
| Sedimento al mar | 3,51 ± 0,71 AU | ≈1,8 | 2,56 | ✗ |
| Longitud de fosa | 69 800 ± 5 900 km | ~51 000 ± 25 % | 68 400 | ✗ (×1,37) → ampliación futura |
| Área que se hunde | 3,37 ± 0,30 km²/año | 2,7–3,4 | 3,55 | ✓ |
| Márgenes pasivos: vida de los que acaban | 111 ± 6 Myr | 130–180 | 117 | ✗ |
| Márgenes al final: edad media / máxima | 228 / 643 Myr | ~104 / ~180 | 212 / 650 | ✗ |
| Choques: longitud, convergencia, duración | 12 100 km; 1,76 cm/año; 63 Myr | CCB 23 000 km; 1,87 | 8 700; 1,89; 71 | — |
| Choques: pérdida con la losa / tras la rotura / total | 1,06 / 0,04 / 1,10 AU | total 0,4–0,7 | 2,40 / 1,14 / 3,54 | ✗ |
| Tiempo de cálculo del planeta | 246 ± 24 s con cinco a la vez | 200 s | 191–203 con cinco | ✗ |

**Lo que mejora.**
- El nivel del mar sube de −500/−1 000 a −305 m.
- La tierra baja de 200 m pasa del 3,7 al 6,8 % y la plataforma de 4,7 a 8,2 km, por la curva nueva y los márgenes estirados.
- La pérdida en los choques baja de 3,5 a 1,1 AU y el cambio de volumen de −15 a −10 %.
- El área que se hunde y la velocidad de las placas cumplen.

**Los márgenes estirados casi no se ven en el planeta.** Perfil de los márgenes pasivos al final (`deriva_en_varias_semillas`, 5 semillas):

| Distancia a la costa | 0–50 km | 50–100 | 100–150 | 150–200 | 200–300 |
|---|---|---|---|---|---|
| Espesor de la corteza | 35,2 km | 38,9 | 40,1 | 40,3 | 40,1 |

- La franja estirada mide 65 km (25–75), frente a 100–300 por lado en la Tierra.
- **La causa es que casi no hay rifts.** Con `τ_rift` = 150 Myr para todo el planeta, salen 1–4 rifts en 500 Myr (uno en la semilla 42). Su corteza estirada existe: 94 muestras de 15 km de media a los 400 Myr. Pero casi ningún margen pasivo del modelo nace de un rift.
- En la Tierra casi todos los márgenes pasivos son de rift, de la rotura de Pangea. Es el ciclo de Wilson pendiente (§45.1): el planeta no forma un supercontinente ni lo rompe.

**Lo que sigue sin cuadrar.**
- El fondo viejo: no viene del estado inicial (§56).
- El relieve.
- `f_cont`.
- El sedimento al mar, más alto que antes porque las cordilleras retienen más corteza (v7).
- La pérdida en los choques (§54).
- **El tiempo**: el flujo lateral con `D_0` = 400 y la corteza que retiene la v7 cuestan un 25 % más.

## 58. Pendientes (2026-11-07 f, en orden; decide el responsable)

1. **Dos pruebas de escenario fuera de su objetivo por la base nueva** (§52). Son la razón de que `main` no avance.
   - La relajación del orógeno (231–250 Myr frente a 100–200). Su calibración (`E_1`, `n_ero`) se hizo con un mar del escenario a −1 890 m, una base irreal. Opciones: revisar el objetivo, el colapso posorogénico o recalibrar la erosión, aunque subirla empeora el sedimento al mar.
   - El frenado de la India tras romperse la losa (5,8–6,0 cm/año frente a 5,75). La India se acelera tras la rotura; `c_col` no puede arreglar las dos fases y el responsable pidió no tocarlo.
2. **El tiempo del planeta: 230 s solo (246 con cinco a la vez), frente a un presupuesto de 200.** Lo sube el flujo lateral de `D_0` = 400 con la corteza que retiene la v7. No se ha tocado el presupuesto.
3. **La corteza de arco que deja de contar como arco al mezclarse** (§54). Es el 44 % de la pérdida sobre fondo oceánico, el 78 % del total. Hace falta una regla de origen por fracción en vez de todo o nada. Con eso, `η_min` podría calibrarse contra 0,4–0,7 AU.
4. **Los escalones de edad y el fondo viejo salen de la representación de las placas** (§56): relleno de huecos con fondo nuevo, movimientos de muestras sueltas y separaciones. No vienen del estado inicial. El fondo de más de 180 Myr (22 %) lo crea la deriva y ninguna fosa lo consume.
5. **Mecanismo pendiente: mesetas oceánicas, montes submarinos y puntos calientes** (~150 m de nivel del mar en la Tierra real).
6. **Casi no hay rifts** (1–4 en 500 Myr con `τ_rift` = 150 Myr): los márgenes estirados de la v8 casi no aparecen (franja de 65 km frente a 100–300). Va con el ciclo de Wilson.
7. Siguen de §45:
   - el ciclo de Wilson;
   - la ampliación futura de fosa (×1,37) y área;
   - las agujas de sumar muestras y el sedimento máximo;
   - la erosión con la exportación (3,5 AU frente a ≈1,8);
   - la lluvia por latitudes;
   - el acortamiento andino, la flexión, el retroceso de fosas y los casquetes.

## 59. Respuestas a las preguntas de la nota de rifts (§11, 2026-11-07 g)

1. **La regla de `τ_rift` era global, repartida por placa** (`rifts()` en `deriva.rs`).
   - Cada placa con corteza continental se partía en el paso con probabilidad `Δt/τ_rift · A_cont,p/A_cont,total`. El planeta entero tenía, de media, `Δt/τ_rift` rupturas por paso: 0,67 por 100 Myr con 150 Myr, se repartiera como se repartiera la corteza.
   - `A_cont,p` era toda la corteza continental de la placa (`c.cont`, también arcos que pasaron a continental), aunque estuviera en trozos separados.
   - El corte era un círculo máximo por su centro de masas, con orientación al azar.
   - Si una mitad quedaba por debajo de `A_min` (0,5 % del planeta, 2,55 Mkm²), no había ruptura ni se reintentaba.
2. **Las suturas no se guardaban como líneas.**
   - `suturas()` fundía la placa menor en la mayor y emitía el evento `sutura` con la mitad de la longitud del frente y un punto. El borde desaparecía y nada marcaba dónde estuvo.
   - La única huella era `proc.lote`, que marca toda la placa absorbida (no la línea), es diagnóstico y se sobrescribe con el siguiente evento.
   - El diseño principal sí lo pedía («La línea de choque se registra como sutura con su edad», §6.5; «lista de suturas internas», `Historial`).
3. **La división no podía cortar por una línea cualquiera, pero generalizarla costaba poco.**
   - `partir` decidía el lado de cada muestra con el signo de `dot(x, polo) + ruido`. Es lo único que dependía de la forma del corte; crear las hijas, heredar fosas e invasiones, el tamaño mínimo y los dueños del mundo, no.
   - La partición por fuente más cercana de la v5 (`partir_por_zona_fractura`, en 17b9ce2) sirve de base.
   - El estiramiento de la v8 (`iniciar_estiramiento`) ya valía para cualquier corte.

**Dos observaciones antes de calibrar.**
- Con `p_k ∝ A_k`, el ritmo global es `λ_A·A_cont` se reparta como se reparta la corteza. Un supercontinente no se rompe más por km² que la misma corteza en varios continentes; solo quedan fuera los bloques de menos de `2·A_trozo,min`. El aislamiento térmico de la nota (§2) no «queda implícito» por km². Si sale un ciclo de supercontinentes, saldrá del trazado por las suturas. No se cambia la nota.
- Una placa de menos de `A_min` se absorbe en su vecina en el mismo paso (`absorciones`), lo que desharía el rift. Además de `A_trozo,min` en cada trozo del bloque, se exige `A_min` a cada mitad de la placa; si no se cumple, se prueba el trazado alternativo.

## 60. Decisiones del responsable y su implementación (2026-11-07 g, la última ronda de la tectónica)

**Decisiones.**
- Adenda v9: el origen de arco como fracción `a_i` (0–1), mezclada por volumen, y `η = a_i·η_arco + (1 − a_i)·η(h)`; después, recalibrar `η_min` con la pérdida a largo plazo (0,4–0,7 AU).
- Nota de rifts: la ruptura por bloques con probabilidad `1 − exp(−λ_A·A_k·Δt)` y el trazado por la sutura más larga, en lugar de `τ_rift`; calibrar `λ_A` con ~4–7 rupturas por 100 Myr.
- India–Asia: se acepta `D_0` = 400. La tolerancia de la velocidad de la India tras romperse la losa se amplía para que entren 5,8 cm/año, sin tocar `c_col`.
- Recalibrar `E_1` y `n_ero` con el mar nuevo: relajación en 100–200 Myr y exportación de ~1,8 AU, y decir si no se cumplen a la vez.
- El presupuesto del planeta sube a 250 s; si es sencillo, hacer implícito el flujo lateral para que no necesite subpasos.
- Los escalones de edad del relleno de huecos y de las muestras que reaparecen se anotan en TREE como artefactos de representación, para más adelante.
- Repetir la tabla de cierre con 5 semillas. Después se congela la capa.

**Adenda v9: la fracción de arco** (`Corteza.arco`, de 0 a 1; `de_arco()` si pasa de 0,5, `continente()` la corteza continental que no es sobre todo de arco).
- `sumar_hc` mezcla por volumen: `a ← (a·h_c + f·dh)/(h_c + roca + dh)`, con el sedimento que pasa a corteza a fracción 0.
  - `f` = 1 lo que hace un arco.
  - `f` = 0 la acreción de sedimento.
  - Los traslados (dientes, islas, fusiones, convergencia difusa) llevan la fracción de la corteza que llega.
- El flujo lateral transporta el volumen de arco con el volumen (cada flujo lleva la fracción de su fuente), y el estiramiento de un rift deja en su franja la fracción media.
- En los choques, `η = a_i·η_arco + (1 − a_i)·η(h)`. El diagnóstico por grosor y por vía cuenta la parte que no es de arco.
- Donde el modelo preguntaba «continental y no de arco» (régimen de los arcos, márgenes pasivos, cuerpos de la invasión), ahora pregunta `continente()`.
- **Lo que deja un choque: una decisión de realismo que se aparta de la letra de la adenda.**
  - La adenda dice que la corteza de un choque se mezcla con fracción 0. Así es sobre un continente: la corteza de arco que llega se une a él, que es justo lo que define `a_i` («de un arco que aún no se ha unido a un continente»).
  - Pero el frente de un choque reparte lo que retiene por toda la placa de arriba a menos de `W_min_orog`, también sobre fondo oceánico o sobre otro arco. Ahí la corteza de arco no se ha unido a ningún continente.
  - Medido (5 semillas, sin rifts nuevos): de la corteza de arco que retienen los choques, 1,1 AU queda sobre un continente y 0,93 AU fuera de él. Con fracción 0, esa corteza pasaría a «continental» y se perdería con `η_min` en el siguiente choque: es la misma pérdida por cambio de etiqueta que la v9 quería quitar.
  - **Regla adoptada:** sobre un continente, fracción 0; fuera de él, la fracción de arco de lo solapado. Comparación en §62.
- **No regresión:** con la regla de todo o nada (interruptor de medida, ya retirado), el planeta era idéntico al de 1f8814d sin las claves nuevas de la salida.

**Nota de rifts** (`deriva/rifts.rs`).
- **Suturas como líneas** (`Corteza.sutura` = `2·id + lado`). Al fundirse dos placas, `marcar_sutura` marca las muestras de los dos lados del frente de choque (bordes `Colision` entre las dos), con el lado de cada una. Viajan con su muestra y su Myr queda en `Deriva::suturas_hechas`.
- **Bloques continentales:** regiones conexas, por las ocho vecinas, de celdas del mundo con `continente()` y el mismo dueño; en orden de su primera celda.
- **Regla (§6.1):** cada bloque de al menos `2·A_trozo,min` que no tenga celdas en una franja de rift que aún se estira se rompe con probabilidad `1 − exp(−λ_A·A_k·Δt)`. Hay una tirada por bloque, con el generador por contador (placa, primera celda).
- **Trazado (§6.2).**
  - La sutura más larga del bloque con sus dos lados dentro, si mide al menos `L_sut,min` (450 km). Su longitud es la del lado más corto, en celdas por lado de celda.
  - El corte por la sutura (`lados_por_sutura`) reparte cada celda de la placa al lado de la fuente más cercana entre las celdas de los dos lados (camino mínimo por las celdas de la placa). Más allá de los extremos, la frontera entre las dos regiones sigue recta hasta la costa y luego por el océano de la placa. Lo que no se alcanza (muestras ocultas bajo otra placa) toma el lado más cercano por toda la esfera.
  - Sin sutura, un círculo máximo por el centro del bloque con orientación sorteada.
  - Si algún trozo queda con menos de `A_trozo,min` de continente, o una mitad de la placa con menos de `A_min`, se prueba el trazado alternativo: un círculo con otra orientación. Si tampoco vale, no hay ruptura.
  - Una sutura que se reabre deja de serlo.
- **División (§6.3):** `partir_por_lados` parte por cualquier corte. El empujón de apertura gira alrededor del eje a 90° del centro en el plano de corte; para una sutura, el polo es la diferencia de los centros de sus dos lados. Lo demás es la v8: `iniciar_estiramiento` y `estirar_rifts`.
- **Eventos (§7):** `rift` con el área del bloque, la de cada trozo (`trozos_km2`), la clase (`sutura` o `circulo`) y la longitud de la sutura (`sutura_km`); `rotura_rift` cuando un lado se rompe y nace el océano, con su extensión (`extension_km`).
- **Métricas (§8):**
  - por paso: bloques que pueden romperse y fracción del continente en el mayor, lados que se estiran, y en los márgenes pasivos el espesor en la costa y la anchura de la franja estirada;
  - en el resumen: rupturas, las que siguen una sutura y roturas por 100 Myr, y ciclos de supercontinente (el índice pasa de menos de 0,5 a 0,7 o más).
- **Se retiran `τ_rift` y la regla vieja.**
- **Pruebas (§10):**
  - `un_bloque_grande_se_rompe_por_su_sutura_y_conserva_la_corteza`: una sutura curva y descentrada de ~3 800 km. Se rompe por ella, no por un círculo por el centro, con menos del 1 % de celdas del lado equivocado, y el volumen continental cambia menos de 10⁻⁹ al partir y al estirar.
  - `un_bloque_pequeno_no_se_rompe_nunca`: 0,48 Mkm² no se rompe en 20 Myr, y 1,4 Mkm² sí.
  - `la_misma_semilla_da_las_mismas_rupturas`.
- **No regresión:** con la regla de `τ_rift` (interruptor de medida, ya retirado), el planeta era idéntico al de 1f8814d sin las claves nuevas.

**El flujo lateral: lo que costaba no eran los subpasos.** El perfil (5 semillas) lo muestra:
- De media, ~110 subpasos por paso, con el espesor máximo de la red casi siempre entre 70 y 90 km, sobre una red de ~7 000–9 000 muestras.
- La red se **rehacía entera ~50 veces por paso**, cada vez que una celda pasaba de `h_flujo` al recibir. Rehacerla se llevaba el 60 % del coste del flujo (99 s de 160, con 20 derivas a la vez).
- **Un paso implícito no ahorraría.** El sistema es poco rígido: `D·Δt/d²` vale ~7–10 con 70–90 km. Un gradiente conjugado necesitaría tantas iteraciones como subpasos tiene hoy, cada una del coste de un subpaso. Además, la ley cúbica pediría iterarlo para no aplastar de más los picos.
- **Se hacen dos cambios que no alteran ningún bit:**
  - la red se amplía con las fuentes nuevas, en vez de rehacerse: dentro del paso no cambian los dueños, los representantes ni las muestras, así que los lados de una fuente son siempre los mismos;
  - solo se apuntan, para mirar después, las celdas que aún no son fuente.
- **Resultado:** idéntica byte a byte. Rehacer la red pasa de 99 a 2 s y los subpasos de 61 a 28 s (con 15–20 derivas a la vez).

## 61. Cada cambio por separado y todos juntos (5 semillas, deriva sola, `λ_A` = 2,5·10⁻⁴)

| | Nada (1f8814d) | v9 | Rifts | v9 + rifts | v9 que conserva el arco | Conserva + rifts | Objetivo |
|---|---|---|---|---|---|---|---|
| Rupturas por 100 Myr | 0,49 | 0,58 | 3,1 | 3,6 | 0,62 | 4,1 | 4–7 |
| Las que siguen una sutura (por 100 Myr) | 0 | 0 | 0,49 | 0,71 | 0 | 0,84 | apreciable |
| Pérdida en choques (AU) | 1,10 | 0,94 | 1,40 | 1,26 | 0,84 | 1,11 | 0,4–0,7 |
| Delaminación (AU) | 2,26 | 2,86 | 2,67 | 2,82 | 2,56 | 2,77 | ≈1,1 |
| Sedimento al mar (AU) | 3,51 | 4,00 | 3,90 | 4,07 | 3,80 | 3,91 | ≈1,8 |
| ΔV_cont | −10,2 % | −13,7 % | −12,6 % | −12,6 % | −10,5 % | −9,1 % | ±10 % |
| Edad media del fondo en la deriva (Myr) | 93 | 95 | 88 | 81 | 92 | 76 | 64 |
| Fondo de más de 180 Myr al final | 22 % | 25 % | 23 % | 15 % | 21 % | 15 % | < 1 % |
| Márgenes que acaban: vida (Myr) | 111 | 111 | 103 | 106 | 117 | 108 | 130–180 |
| Arco que retienen los choques, sobre un continente / fuera (AU) | 0,46 / 0,57 | 1,11 / 0,93 | 0,52 / 0,62 | 1,29 / 1,06 | 1,19 / 1,50 | 1,53 / 1,99 | — |

- **La v9 sola baja la pérdida** (1,10 → 0,94 AU), pero las cordilleras retienen más arco y engordan. Suben la delaminación (2,3 → 2,9) y el sedimento al mar (3,5 → 4,0), y el volumen continental pasa de −10 a −14 %.
- **Conservar el arco fuera de los continentes** (§60) acerca todo lo que se mide: pérdida 0,84, delaminación 2,6 y ΔV_cont −10,5 %. Con los rifts, la pérdida queda en 1,11 y ΔV_cont en −9,1 %.
- **Los rifts rejuvenecen el fondo** (93 → 76 Myr de media en la deriva; fondo de más de 180 Myr, 22 → 15 %), pero alargan las fosas (§62) y meten corteza fina en los choques: la pérdida sube.
- **Con la v9, lo que se pierde sobre fondo oceánico ya no es arco.** Es sedimento que pasó a corteza (33–48 %), lo que dejan los choques sobre el fondo de la placa de arriba (23–34 %) y astillas de borde (11–13 %). El arco baja del 44–45 % al 10–18 %.

## 62. `λ_A` y `η_min` (5 semillas; v9 que conserva el arco y rifts por bloques)

| | `λ_A` 2,5·10⁻⁴ | **4·10⁻⁴** | 6·10⁻⁴ | 4·10⁻⁴ y `η_min` 0,4 | Objetivo |
|---|---|---|---|---|---|
| Rupturas por 100 Myr | 4,1 ± 0,7 | **4,9 ± 1,7** | 7,5 ± 0,9 | 4,9 ± 0,5 | 4–7 |
| Las que siguen una sutura | 21 % | 22 % | 17 % | 19 % | apreciable |
| Roturas de un lado por 100 Myr | 5,8 | 6,9 | 10,1 | 7,1 | — |
| Suturas por 100 Myr | 2,6 | 2,8 | 4,4 | 2,2 | — |
| Edad media del fondo en la deriva (Myr) | 76 | 75 | 65 | — | 64 |
| Fondo de más de 180 Myr al final | 15 % | 16 % | 12 % | — | < 1 % |
| Márgenes que acaban: vida (Myr) | 108 | 100 | 95 | 109 | 130–180 |
| Longitud de fosa (km) | 87 200 | 88 500 | 105 500 | 83 800 | ~51 000 |
| Área que se hunde (km²/año) | 4,15 | 4,28 | 4,91 | 4,01 | 2,7–3,4 |
| Pérdida en choques (AU) | 1,11 | 1,16 | 1,36 | 1,23 | 0,4–0,7 |

- **`λ_A` = 4·10⁻⁴**, en el centro del objetivo. Es más que el 2,5·10⁻⁴ de la nota porque el modelo tiene menos continente (`f_cont` 0,32 frente a 0,40).
- **Más rifts rejuvenecen el fondo, pero alargan las fosas.** Cada rift deja dos márgenes nuevos, y la invasión de la v4 los convierte pronto en fosas (550–600 km/Myr de margen invadido). La fosa (×1,37 antes) y el área que se hunde ya pasaban del objetivo: es la ampliación futura de fosa.
- **`η_min` se queda en 0,5, el techo de su rango.** Con 0,4 la pérdida sube (1,16 → 1,23 AU). La pérdida que queda es corteza fina sobre fondo oceánico (61–64 %), sobre todo sedimento hecho corteza y lo que los choques reparten por el fondo de la placa de arriba. `η_min` no puede bajarla.

## 63. `E_1` y `n_ero`: relajación y exportación con el mar nuevo

**Relajación en India–Asia** (`calibracion_relajacion`; el mar del escenario, a −1 255 m):

| `n_ero` \ `E_1` (km/Myr) | 0,005 | 0,006 | 0,0075 | 0,008 | 0,009 | 0,011 | 0,012 | 0,016 |
|---|---|---|---|---|---|---|---|---|
| 2,0 | — | — | — | 281 | — | — | 200 | 153 |
| 2,5 | 242 | 210 | **171** | — | 140 | 121 | — | — |
| 3,0 | 132 | 111 | 94 | — | 77 | 65 | — | — |

**Exportación en el planeta** (5 semillas, `λ_A` = 4·10⁻⁴) de las combinaciones que cumplen la relajación:

| `n_ero`; `E_1` | 2,5; 0,005 (antes) | **2,5; 0,0075** | 3,0; 0,005 | 2,0; 0,014 | Objetivo |
|---|---|---|---|---|---|
| Relajación (Myr) | 242 ✗ | 171 ✓ | 132 ✓ | ~175 ✓ | 100–200 |
| Sedimento al mar (AU) | 3,94 | 5,06 | 5,54 | 5,37 | ≈1,8 |
| Delaminación (AU) | 2,74 | 2,41 | 2,01 | 2,60 | ≈1,1 |
| ΔV_cont | −8,7 % | −8,0 % | −7,2 % | −10,1 % | ±10 % |
| Pérdida en choques (AU) | 1,16 | 1,38 | 1,38 | 1,38 | 0,4–0,7 |

**No se cumplen a la vez.** La relajación pide más erosión y la exportación, la mitad de la de hoy. Las dos dependen de lo mismo: entre el 62 y el 70 % de la erosión del planeta sale de la tierra a más de 3 km sobre el mar, y ahí está también la cola lenta de la relajación (2–4 km). `n_ero` no puede separarlas.

**La causa está en la base, no en `E_1`** (CLAUDE §2.8):
- Entra demasiada corteza. El fondo que se hunde va a 4,0–4,9 km²/año (Tierra, 2,7–3,4) y los arcos suman 4,4–5,4 AU, porque la fosa mide 1,7–1,9 veces la de la Tierra.
- La tierra es demasiado alta: el 27–29 % pasa de 2 km, frente a ~11 % en la Tierra.
- Además, el mar del propio escenario de relajación está a −1 255 m (Asia, a 1,7 km sobre el mar). Con un mar realista, el orógeno sobresaldría menos y haría falta todavía más erosión.

**Se elige `E_1` = 0,0075 con `n_ero` = 2,5.** Es el cambio mínimo que cumple la relajación (171 Myr) y el de menos exportación de los tres. Bajar `E_1` hasta ~1,8 AU dejaría las cordilleras sin gastarse en 400 Myr: forzar un parámetro para tapar otra irrealidad.
- `E = E_1·z^2,5` da 7,5 m/Myr a 1 km, 117 a 3 km y 420 a 5 km. No es alto: el Himalaya se erosiona a 1–5 km/Myr.
- Con `n_ero` = 3 baja la delaminación (2,0) y el pico de la deriva, pero exporta algo más. La diferencia entre las dos queda dentro de la dispersión entre semillas (±0,3–0,5 AU).

## 64. Tabla de cierre (5 semillas, planeta entero; v9, rifts por bloques, `λ_A` = 4·10⁻⁴, `E_1` = 0,0075)

| | Modelo | Tierra u objetivo | Ronda f (§57) | |
|---|---|---|---|---|
| Tierra emergida | 0,326 ± 0,025 | 0,284 | 0,334 | ✗ |
| Altura media de la tierra | 1 235 ± 160 m | 671 m | 1 143 | ✗ |
| Tierra por debajo de 200 m | 0,087 ± 0,025 | 0,283 | 0,068 | ✗ |
| Pendiente media (±25 km) | 0,0167 | 0,0052 | 0,0128 | ✗ |
| Anchura de la plataforma | 8,1 ± 1,7 km | 42,8 km | 8,2 | ✗ |
| Nivel del mar al final de la deriva | −164 ± 123 m | 0 (la Tierra real con la regla: −150) | −305 | — |
| Profundidad media del océano | 3 915 ± 136 m | 3 667 m | 3 952 | ✗ |
| Corteza máxima al final / en toda la deriva | 80 ± 10 / 123 ± 13 km | 70–80 km | 79 / 117 | ✓ / ✗ |
| Edad media del fondo al final; más de 180 Myr | 88 ± 23 Myr; 13,1 % | 64,2; < 1 % | 119; 21,8 % | ✗ |
| `f_cont`; cambio de volumen | 0,323; −8,0 ± 4,6 % | 0,40; ±10 % | 0,318; −10,3 % | ✗ / ✓ |
| Velocidad de las placas | 4,65 ± 0,25 cm/año | 4,39 | 4,40 | ✓ |
| Sedimento al mar | 5,06 ± 1,04 AU | ≈1,8 | 3,51 | ✗ |
| Longitud de fosa | 96 300 ± 13 800 km | ~51 000 ± 25 % | 69 800 | ✗ (×1,89) |
| Área que se hunde | 4,64 ± 0,48 km²/año | 2,7–3,4 | 3,37 | ✗ |
| Márgenes pasivos: vida de los que acaban | 103 ± 11 Myr | 130–180 | 111 | ✗ |
| Márgenes al final: edad media / máxima | 182 / 646 Myr | ~104 / ~180 | 228 / 643 | ✗ |
| Choques: longitud, convergencia, duración | 21 200 km; 1,45 cm/año; 63 Myr | CCB 23 000 km; 1,87 | 12 100; 1,76; 63 | — |
| Choques: pérdida con la losa / tras la rotura / total | 1,23 / 0,15 / 1,38 AU | total 0,4–0,7 | 1,06 / 0,04 / 1,10 | ✗ |
| **Rupturas por 100 Myr** | **5,8 ± 1,2** | ~4–7 | 0,2–0,8 | ✓ |
| Rupturas que siguen una sutura | 29 % (13–65 %) | apreciable | — | ✓ (blando) |
| Roturas de un lado (nace el océano) por 100 Myr | 8,5 ± 3,4 | — | — | — |
| Márgenes pasivos: franja estirada; corteza en la costa | 55 km; 35,7 km | 100–300 por lado | 65; 35,2 | ✗ |
| Mayor bloque continental (media) | 0,18 del continente | — | — | — |
| Ciclos de supercontinente (de < 0,5 a ≥ 0,7) en 450 Myr | 3,6 ± 2,4 | ~1,5 en 630 Myr (~1,1) | — | ✗ (blando) |
| Tiempo de cálculo del planeta | 264 ± 9 s con cinco a la vez; **243 s solo** y 250,4 s en la batería (semilla 42) | 250 s | 246 con cinco; 230 solo | ✗ (por 0,4 s) |

**Lo que mejora.**
- **Rupturas a ritmo terrestre** (5,8 por 100 Myr), una de cada tres o cuatro por una sutura.
- **El fondo rejuvenece:** de 119 a 88 Myr de media al final (68 de media en la deriva), y el de más de 180 Myr, del 22 al 13 %.
- **El nivel del mar sube a −164 m**, casi el de la Tierra real con la regla del modelo (−150), y el océano tiene 3 915 m.
- **Los choques duran y miden como los de la Tierra** (21 200 km frente a los 23 000 de Bird) y ΔV_cont cumple (−8 %).

**Lo que empeora, y su causa.**
- **La fosa (×1,89) y el área que se hunde (4,6 km²/año).** Cada rift deja dos márgenes que la invasión convierte pronto en fosas, con más placas (28 en la semilla 42). Ya pasaban del objetivo (ampliación futura de fosa).
- **El sedimento al mar (5,1 AU) y la pérdida en choques (1,4 AU),** por la erosión más fuerte que pide la relajación (§63) y porque entra más corteza: los arcos crecen con el área que se hunde.
- **El relieve sigue alto** (1 235 m de media; el 27–29 % de la tierra por encima de 2 km).
- **Los márgenes pasivos no se parecen a los de la Tierra.** Los nacidos de un rift sí se estiran: con la calibración elegida, 28 km de corteza en la costa y una franja de 95 ± 27 km (5 semillas; 95–115 en las otras variantes), casi en 100–300. Pero son el 8–12 % de la costa de los márgenes. El resto son los bordes de los continentes del estado inicial, que nunca se estiraron, y viven más de lo real (edad máxima 646 Myr). En la Tierra, casi todos vienen de la rotura de Pangea.
- **Los ciclos de supercontinente** (3,6 frente a ~1,1): el índice oscila entre 0,3 y 0,9 sin un supercontinente que dure. Con `p ∝ A` el ritmo de ruptura no depende de lo juntos que estén los continentes (§59).

**El tiempo.**
- Con la red ampliada y los demás ahorros, todos idénticos byte a byte, el flujo lateral pasa de ~60 a ~20 s por deriva.
- Las placas y bordes de más que traen los rifts cuestan casi lo mismo. El planeta solo tarda 243 s; en la batería, con los escenarios de agentes a la vez, 250,4 s: **se pasa del presupuesto de 250 por 0,4 s**.
- Por la regla del responsable, el presupuesto no se sube sin preguntar. Por eso `main` no se actualiza en esta ronda: decide el responsable.
- Lo que más cuesta ahora es el reparto del sedimento al mar (22 s por deriva), la rasterización (19 s), el flujo (20 s), la erosión (10 s), las métricas (13 s) y, fuera de la deriva, la génesis geológica.

## 65. Pendientes al congelar la capa (2026-11-08, en orden)

La capa tectónica queda congelada: no se le añaden más reglas. Lo que sigue queda como trabajo futuro, en orden de lo que más aleja el planeta de la Tierra.

1. **La longitud de fosa y el área que se hunde** (×1,89 y 4,6 km²/año): las fuerzas de las placas y la invasión de los márgenes (550–700 km/Myr de margen invadido). Arrastran los arcos, el sedimento al mar y la pérdida en los choques.
2. **El relieve demasiado alto** (1 235 m; el 27–29 % de la tierra por encima de 2 km): es la base de la exportación al mar (§63). Con él, `E_1` no puede cumplir a la vez la relajación y ~1,8 AU.
3. **Los márgenes del estado inicial,** que nunca se estiraron y viven 600 Myr: un estado inicial más terrestre (un supercontinente que se rompe) o una deriva más larga.
4. **La pérdida en los choques** (1,4 AU frente a 0,4–0,7): corteza fina sobre fondo oceánico, sobre todo sedimento hecho corteza y lo que el frente de un choque reparte por el fondo de la placa de arriba (`W_min_orog`).
5. **Los artefactos de representación de la deriva** (TREE): el relleno de huecos con fondo de edad 0, las muestras que reaparecen y los movimientos de muestras sueltas, que fabrican escalones de edad y fondo viejo (§56).
6. **La India que se acelera tras romperse la losa** (5,8 cm/año; la real siguió frenando): el giro que le dan las fosas de los lados. La prueba admite hasta 0,55 veces la velocidad de antes (decisión del responsable).
7. **El mar del escenario de relajación** (−1 255 m; Asia a 1,7 km sobre el mar): con un mar realista, el orógeno sobresaldría menos y haría falta más erosión.
8. **Ampliaciones de la nota de rifts:** plumas del manto, rifts fallidos, aislamiento térmico de los supercontinentes y cuencas de trasarco.
9. **Lo que sigue de §58:** mesetas oceánicas, montes submarinos y puntos calientes (~150 m de nivel del mar); el acortamiento andino, la flexión, el retroceso de las fosas y los casquetes; la lluvia por latitudes.
10. ~~El tiempo~~ decidido: el presupuesto del planeta sube a 275 s (243 s solo, 250,4 en la batería). Lo siguiente que se podría abaratar es el reparto del sedimento al mar (dos inundaciones por prioridad en cada paso), la rasterización y la génesis geológica.

**Decisiones del responsable (2026-11-08).**
- Acepta la fracción de arco de lo que deja un choque fuera de un continente, `E_1` = 0,0075, la India como pendiente y `λ_A` = 4·10⁻⁴.
- Sube el presupuesto del planeta a 275 s y actualiza `main` con esta ronda.
- **La capa no se congela todavía:** antes se arregla el exceso de fosas (nota de demografía de placas, §66 en adelante).

## 66. Nota de demografía de placas: diagnóstico, rama A y punto de decisión (2026-11-08)

Informe completo, con todas las tablas: `docs/informes/2026-11-08_demografia_placas.md`. Rama `terreno-demografia-placas`, sin fusionar con `main`.

- **Respuestas al §11.**
  - La velocidad relativa ya se calculaba en cada celda de borde (`clasificar_bordes`), pero solo se guardaba su componente normal; ahora también el módulo (`Borde::v_rel`).
  - Las microplacas solo se libraban de la absorción inmediata (toda placa de menos de `A_min`) si tenían una losa que tiraba.
  - Una placa consumida entera no moría: sin área ni borde seguía «viva» (29 al final).
- **Diagnóstico del §0** (`diagnostico_demografia_placas`, 5 semillas):
  - **D1:** ~20 placas de al menos el 1 % del planeta frente a las 13 de Bird; nacen 21,3 por 100 Myr (rift 5,8; invasión 15,5) y mueren 19,3.
  - **D2:** el nacimiento inducido crea el 85 % de la fosa y la invasión el 15 %. Se corrigió la longitud de sus eventos, que contaba los dos lados del borde.
  - **D3:** fosa ×1,88, difusa ×0,75 y choque ×0,92 de Bird.
  - **D4:** ~1 000 km de borde inactivo, siempre durante poco tiempo.
  - **D5:** el exceso de fosa está en las placas iniciales y en las de rift.
  - **Señales:** H1 sí (rama A); H2 y H3 no.
- **Rama A** (`soldaduras`, `capturas`; tres pruebas de la nota):
  - A1 no actúa nunca, ni con `v_fus` 0,5 y `τ_fus` 10: los bordes de las placas grandes nunca se quedan quietos;
  - A2 retira las placas fantasma, pero deja vivir más microplacas de la invasión;
  - la fosa baja un 7 % (dentro de la dispersión) y las placas grandes no bajan;
  - el planeta tarda 298 s solo, frente a 243 (presupuesto 275), porque el reparto del sedimento al mar se encarece en todas las semillas por razones aún sin perfilar.
- **Tras A, ninguna rama encaja; se para y decide el responsable.** La causa que muestra el diagnóstico es otra: las placas grandes nacen en los rifts (~5 por 100 Myr) y solo mueren en las suturas (~3). Opciones en el §8 del informe:
  - una muerte de placas oceánicas grandes;
  - menos rifts fuera de los supercontinentes;
  - medir la rama C sin su señal;
  - revisar A.
- **Tabla de cierre con las filas de relieve** (§7 del informe). La tierra por encima de 2 000 m es el 15 %, frente al 5,3 % de la roca de ETOPO sin hielo. El ~11 % de la nota incluye el hielo de Antártida y Groenlandia.
