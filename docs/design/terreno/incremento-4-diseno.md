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

## 22. Pendientes (2026-11-07, en orden)

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
