# Incremento 4 del terreno: relieve tectónico (diseño técnico y resultados)

**Estado:** primera implementación (2026-11-05), con interruptor `worldgen.relieve = "tectonico" | "ruido"`
mientras se mide. **Diseño del responsable:** `diseno_relieve_tectonico.md` (manda sobre este documento).
**Código:** `crates/abm-terreno/src/tectonica/` (`malla`, `deriva`, `isostasia`, `historia`, `mod`), más
`statics.rs` (G5), `erosion.rs` y `lod.rs` (G6), `worldgen.rs`, `cache.rs` (`ABMGEN04`) y `metrics.rs`
(esquema v6). **Referencia de la Tierra:** `config/referencias/tierra_relieve.toml`
(`herramientas/analisis/referencia_tierra.py`).

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
6. **Remuestreo del 5 %** del volumen continental en 500 Myr: copias de corteza al tapar agujeros de los mapas junto a márgenes con dorsal. Es del orden de lo que crean los arcos; hay que reducirlo antes de calibrar el balance.
7. **Cuencas cerradas:** con el relieve tectónico, el 4,4 % de la tierra está en cuencas cerradas (la Tierra, 10,4 % a 26 km). El planeta de ruido anterior (42 lagos) estaba más drenado que la Tierra.

## 7. Pendientes (en orden)

1. Adenda del responsable: sumideros reales de corteza (erosión por subducción, sedimento subducido) y η_col de India–Asia (Ingalls et al., 2016: ~50 % reciclado).
2. Inicio de subducción en márgenes pasivos viejos (cierra el fondo de más de 200 Myr).
3. Fondo oceánico real: sedimento abisal, mesetas y montes submarinos (puntos calientes, P2 del diseño), y márgenes continentales sumergidos. Con 1–3 debería volver ζ ≈ 0 sin forzar `H_nivel`.
4. Acortamiento andino (el §6.3 cuenta con él); ciclo de Wilson (rifts por suturas viejas); flexión de la litosfera (cuencas de antepaís); microplacas (Bird).
5. Reducir el remuestreo (hallazgo 6).
6. Revisar el objetivo de la lluvia de barlovento (hallazgo 3) y, con el relieve ya realista, calibrar la lluvia de base con datos de la Tierra y retirar `c_P`.
7. Casquetes polares (R6); con ellos, el agua total pasa a ser océano + hielo.
