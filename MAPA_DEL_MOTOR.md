# MAPA_DEL_MOTOR.md — Qué calcula el motor, en qué orden y dónde

> Para orientarse en el motor sin leer el código: qué hace cada binario, en qué orden corren sus
> sistemas, qué reglas aplica cada uno, dónde está su código y su diseño, qué parámetros mandan y en
> qué estado está cada pieza.
>
> Complementa a `TREE.md` (qué archivos hay) y a los documentos de `docs/design/` (el porqué de cada
> regla). Se actualiza en cada ronda que añade, quita o reordena una regla.

Última actualización: 2026-11-08 (quinta ronda) — `main` = 473b805 (la isostasia con litosfera, la regla relativa del tiempo y los márgenes como entidades). Rama `terreno-margenes-calientes-succion`, sin fusionar: hundimiento térmico de McKenzie en los márgenes, puntos calientes (mesetas, trazas, abombamientos) y succión de las losas profundas y de los restos de losa, con la soldadura tras un choque agotado.

Leyenda del estado de un parámetro (CLAUDE §2.8):
- **medido**: lleva su valor físico real;
- **calibrado**: no se mide directamente y se ajusta contra una observación real, que se cita;
- **supuesto**: un valor razonable dentro de un rango, a falta de una observación con que calibrarlo;
- **provisional**: un apaño marcado, con la causa real que hay que corregir.

---

## 1. Los tres binarios

| Binario | Qué simula | Paso de tiempo | Config | Diseño |
|---|---|---|---|---|
| `abm` | Forrajeo mínimo en una cuadrícula (fase «inicial») | tick | `config/default.toml` | `docs/design/inicial/` |
| `agentes` | Biología de los individuos por días (fase «agentes») | día | `config/agentes.toml` | `docs/design/agentes/` |
| `terreno` | El planeta: relieve, clima, agua, vegetación, suelo | día; la génesis, en Myr | `config/terreno.toml` | `docs/design/terreno/` |

Todos corren en un solo hilo con un orden fijo: misma semilla y misma config, misma salida bit a bit
(CLAUDE §6). Cada uno escribe su JSON en `output/` y un `<nombre>_tiempos.json` con lo que tarda
cada sistema. Aceptan ajustes `--set ruta=valor` sin tocar la config.

---

## 2. El planeta (`terreno`)

### 2.1 Una corrida

1. **Génesis** (`worldgen.rs`), en la caché `output/cache` si ya se calculó con la misma semilla, la
   misma config y el mismo código:
   1. **relieve de nacimiento**: la deriva de placas de 500 Myr (§2.2) o, con
      `worldgen.relieve = "ruido"`, el ruido de los incrementos 1–3;
   2. malla del cubo esférico, estáticos y derivados (drenaje, ríos, lagos, distancia al mar);
   3. refinamiento inicial del LOD;
   4. **génesis geológica** (incremento 3): erosión y suelo durante ~1 millón de años.
2. **Ticks diarios** (`lib.rs`, `systems.rs`), en este orden:

| # | Sistema | Qué hace | Diseño |
|---|---|---|---|
| 1 | `lod` | Subdivide o colapsa celdas según dónde hace falta detalle (cada `n_lod` ticks) | incremento 2 |
| 2 | `clima` | Temperatura y lluvia del día, con la lluvia orográfica y la sombra pluviométrica | incremento 1; notas de lluvia y de sombra |
| 3 | `nieve` | Acumulación y fusión | incremento 1 |
| 4 | `balance_agua` | Suelo, evapotranspiración, escorrentía | incremento 1 |
| 5 | `difusion_lateral` | El agua del suelo entre celdas vecinas | incremento 2 |
| 6 | `agua_superficial` | Ríos, lagos y lo que llega al mar | incremento 1 |
| 7 | `vegetacion` | Biomas y vegetación (cada `n_veg` ticks) | incremento 1 |
| 8 | `contabilidad`, `metricas` | Balance de agua (residual ~10⁻¹²) y métricas del tick | — |
| — | `erosion` | Erosión en ejecución, cada `run_every` ticks | incremento 3 |

3. **Salida**: `output/<nombre>_terreno.json` (esquema v7). Incluye `run.tectonica`: la historia de
   la deriva (eventos, métricas de cada Myr, cuentas de la corteza) y el balance medio.

### 2.2 La deriva de placas: un paso (Δt = 1 Myr; `tectonica/deriva.rs`, `Deriva::dar_paso`)

El estado son las **placas rígidas**. Cada una guarda sus muestras de corteza en su propio sistema
de referencia y gira sobre la esfera con un cuaternión. Las cantidades que viajan con las placas se
conservan exactamente: la deriva nunca copia corteza.

| # | Etapa | Qué hace | Código | Diseño |
|---|---|---|---|---|
| 1 | Velocidades | Equilibrio de fuerzas por placa: tirón de la losa, empuje de dorsal, succión de las losas profundas y de los restos (`τ = ε_suc·B·(1 − d/W)/W`, en la malla gruesa), rozamiento basal (recalibrado a `v_obj`), resistencia de los choques | `velocidades`, `succion.rs: pares_de_succion` | principal §6.2; nota de fosas v2; nota de succión §6.3 |
| 0 | Edad tectonotérmica, puntos calientes y restos de losa | Toda la corteza envejece Δt; la edad vuelve a 0 donde la calientan un rift al romper, un arco, un choque, una delaminación de más de `ε_del` o una gran erupción, y se mezcla por volumen al juntar corteza. Los puntos calientes envejecen y, al morir, nace otro (con gran erupción, a veces). Los restos de losa envejecen y dejan de tirar a los `τ_hund` | `envejecer_tt`, `vida_de_puntos_calientes`, `envejecer_restos` | nota de isostasia con litosfera §6.1–6.3; nota de puntos calientes §6.1; nota de succión §6.2 |
| 2 | Giro | Cada placa gira con su velocidad angular (y el empujón de apertura de un rift) | `dar_paso` | principal §6.1 |
| 3 | Rasterizar, solapes, dientes, huecos | Qué placa ve cada celda del mundo; los solapes se consumen (fosa) o chocan; los dientes de borde se recolocan; los huecos se rellenan con fondo nuevo | `rasterizar`, `resolver_solapes`, `recolocar_dientes`, `rellenar_huecos`, `reconstruir_mapas` | nota de bordes |
| 4 | Rifts que se estiran | La abertura de un rift joven se rellena estirando la corteza de sus bordes en rampa hasta la rotura | `rifts.rs: estirar_rifts` | adenda v8 §6.10 |
| 5 | Bordes suavizados | Pertenencia suavizada a `L_borde`, islas absorbidas y convergencia difusa | `suavizado.rs`; `fosas.rs: absorber_difusa` | nota de bordes; nota de fosas §6.2 |
| 5b | Magmatismo intraplaca | Grandes erupciones (mesetas en el océano, basaltos en los continentes) y trazas de los puntos calientes; la corteza flotante de mesetas y trazas no pasa la columna a continental; `F_int` entra en el balance | `calientes.rs: magmatismo_intraplaca` | nota de puntos calientes §6.2–6.3 y §6.6 |
| 6 | Clasificar bordes | Tipo de cada celda de borde (dorsal, fosa, choque, transformante, difusa), su velocidad relativa y el estado de su fosa | `clasificar_bordes`, `aplicar_estado_fosas` | nota de bordes; nota de fosas |
| 7 | Procesos de fosa | Régimen del margen; arcos, erosión por subducción y prisma de acreción | `procesos_de_fosa` | adenda §6.1–6.4 |
| 8 | Choques | Lo que se solapa en un choque se queda arriba con `η = a·η_arco + (1 − a)·η(h)` y engruesa el frente | `reconstruir_mapas`, `engrosar_choques` | adenda §6.5 (v7 y v9) |
| 9 | Delaminación | La raíz de más de `h_del` se desprende | `delaminar` | adenda §6.6 |
| 10 | Flujo lateral | La corteza inferior de más de `h_flujo` se desparrama: `D = D_0·(c/c_ref)³` | `flujo_lateral` | adenda v6 §6.6b |
| 11 | Erosión y sedimento | `E = E_1·z^n_ero` en la tierra emergida; las alturas con la isostasia de la litosfera en los continentes (regla R4, más el hundimiento térmico de McKenzie en los márgenes estirados) y con la curva de la placa y el abombamiento de los puntos calientes en el océano; nivel del mar con el agua de la Tierra; el sedimento va a las cuencas cerradas y al mar abierto, hasta la plataforma | `erosion_y_envejecimiento`, `isostasia.rs: altura_mundo`, `nivel_del_mar`, `sedimento_al_mar` | adenda v5 §6.7; nota de isostasia con litosfera §6.4; nota de hundimiento §6.3; nota de puntos calientes §6.4 |
| 12 | Eventos de placas | Suturas (soldadura tras un choque agotado: con la losa ya rota, a menos de `v_sold` durante `τ_sold`; reinician el reloj de los bloques que se juntan), rifts (por bloques y por suturas, con la probabilidad que crece con el tiempo que el bloque lleva unido), subducción de dorsales (una dorsal con la fosa delante a menos de `d_dc`: su franja pasa a la placa del otro lado y la fosa se apaga), captura de microplacas, saltos de dorsal | `suturas`, `rifts.rs: rifts`, `subduccion_de_dorsales`, `capturas`, `saltos_de_dorsal` | principal §6.4–6.5; nota de rifts v2; nota de demografía de placas v2 (A2 y rama D) |
| 13 | Fosas | Estado de cada tramo convergente: nacimiento inducido, muerte, rotura de la losa (la losa de lo que muere o se rompe pasa a un resto); invasión de transformantes y márgenes pasivos; nacimientos espontáneos | `fosas.rs: avanzar_fosas`, `invadir`, `nacimientos_espontaneos`; `succion.rs: nacen_restos` | nota de fosas v6; nota de succión §6.2 |
| 14 | Métricas | Métricas del Myr, balance de la corteza (error ~10⁻¹⁵), márgenes pasivos y su perfil, bloques continentales, la tierra por clase de edad tectonotérmica y los márgenes estirados por tiempo desde la rotura; cada 25 Myr, la foto del diagnóstico del supercontinente (solo mide) | `metricas_del_paso`, `diagnostico.rs` | adenda §8; notas; nota de isostasia con litosfera §8 |

**Parámetros que mandan** (`[tectonica]` de `config/terreno.toml`, donde cada uno lleva su origen):

| Grupo | Parámetros | Estado |
|---|---|---|
| Planeta y agua | `gel_m` 2 617 m, `f_cont` 0,40 al empezar | medido; supuesto |
| Isostasia y fondo | densidades; curva de profundidad (`d_inf` 5 310, `a_pl` 2 373, `τ_pl` 31) | medido; ajustado al zócalo real (Seton, GlobSed) |
| Isostasia continental (R4) | `ρ_ast` 3 255; `ρ_c_sup` 2 750 hasta `z_t` 15 km y `ρ_c_inf` 3 000 debajo; `Δρ_ma` 25 (arcaico 15, desde `t_arc` 2 500 Myr); `L = L_min + (93,6 + 0,04·t − L_min)·(1 − e^(−t/τ_L))` con `L_min` 50 y `τ_L` 100; `H_0` 3 077 m; el sedimento cuenta como corteza | ajustados a la Tierra real (Lamb et al. 2020, TC1 y CRUST1.0) sin las celdas de topografía dinámica (comprobación del §0 de la nota). **`ρ_c_sup`, `ρ_c_inf` y `z_t` quedan en el borde de sus rangos publicados** (limitación conocida, TREE.md). La topografía dinámica, ampliación futura |
| Edad tectonotérmica | `ε_del` 1 km; edades de partida: la Tierra de hoy de más de 500 Myr, menos 500, las más viejas en el interior de cada continente | supuesto; medido (tipos de CRUST1.0 con las provincias de Artemieva y Mooney) |
| Hundimiento de los márgenes | `E_0` 3,2 km, `τ` 62,8 Myr; `β = h_0/h_c` al romper | medido/físico (McKenzie 1978) |
| Puntos calientes | `N_hs` 12, `T_hs` 150 Myr; una gran erupción cada 23 Myr, 0,2–2 Mkm², `h_LIP` 15 km, `h_CFB` 3 km, 2 Myr; traza `q` 1,25·10⁵ km³/Myr en `r_tr` 100 km; abombamiento 0,5–1,2 km × 1 000–1 500 km | supuesto (8–20); medido; medido (Rampino y Stothers); calibrado (magmatismo intraplaca oceánico, Crisp 1984); medido (Crough 1983) |
| Succión de las losas | `d_off` 500 km, `W_suc` 1 500 km, `ε_suc` 0,05; `v_hund` 1,2 cm/año, `τ_hund` = 2 200 km / `v_hund` | geométrico; supuesto; calibrado (el tirón, ~50 % de la potencia); medido (van der Meer et al. 2010) |
| Soldadura | `v_sold` 0,5 cm/año (`v_sutura_cm_anio`), `τ_sold` 20 Myr (`t_sutura_myr`), con la losa rota | supuestos |
| Fuerzas | `f_sp`, `f_rp`; `c_col` 90 000; `v_obj` 4,39 cm/año | supuesto; calibrado (India–Asia); medido (NNR-MORVEL56) |
| Bordes | `θ_tr` 20°; `L_borde` 150 km | medido (Bird); calibrado (longitudes de Bird) |
| Fosas | `X_ini` 150 km; `τ_muerte` 10; `λ_esp` 4·10⁻⁶; `v_inv` 25 km/Myr; `a_inv` 60; `t_rot` 10–25 Myr | supuesto (rango de modelos); supuesto; calibrado (vida de los márgenes) |
| Arcos y erosión por subducción | `p_arc_oc` 1,2, `p_arc_cont` 0,8, `p_se` 0,4; `f_acr` 0,2 | medido (Clift et al. 2009); calibrado |
| Choques | `η_arco` 0,9; `η_min` 0,5 (techo), `h_1` 20, `h_2` 35 | medido (Luzón–Taiwán); calibrado; físico |
| Delaminación y flujo | `h_del` 50; `τ_del` 25; `h_flujo` 50; `D_0` 400 | físico; calibrado; físico; calibrado (India–Asia, 70–80 km) |
| Erosión | `E_1` 0,02; `n_ero` 2,5 | calibrado con la isostasia nueva (relajación de India–Asia en 159 Myr; 100–200); `n_ero`, rango de Montgomery y Brandon (2 y 3 dan lo mismo dentro de la dispersión entre semillas) |
| Rifts | `λ_A` 4·10⁻⁴; `τ_ais` 100 Myr (reloj del bloque, nota de rifts v2); `A_trozo,min` 0,5 Mkm²; `L_sut,min` 450 km; `W_m` 150; `h_rot` 10 | calibrado (placas de ≥ 1 %: 13,8 frente a 13 de Bird); supuesto (no se forma ningún supercontinente con que calibrarlo); físico; numérico; físico |
| Muerte de placas | `A_micro` 0,5 Mkm², `τ_micro` 10 Myr (A2); `d_dc` 300 km y `θ_dc` 45° (rama D, de frente, aceptada por el responsable) | supuestos; `θ_dc`, geométrico (la fosa delante de la dorsal, no a su costado) |

### 2.3 Del estado final de la deriva al relieve

`tectonica/mod.rs: estado_final` interpola la corteza, el sedimento, la edad y las fosas de la malla
tectónica (nivel 2, ~52 km) a las celdas del terreno y calcula la altura isostática. El ruido de
detalle fino continúa el espectro real (ETOPO), según la altura. El levantamiento de la génesis
geológica usa la derivada de R4 en corteza gruesa (`(ρ_ast − ρ_c_inf + Δρ_ma)/ρ_ast` = 0,086).

---

## 3. Los agentes (`agentes`): un día (`abm-ruleset/src/biologia/schedule.rs`)

| # | Sistema | Qué hace |
|---|---|---|
| 1 | `inicio` | Libro de energía a cero, reservas de partida |
| 2 | `forraje` | Rebrote del forraje (T4) |
| 3 | `demanda` | Gasto del día, demanda con la respuesta funcional de Holling y reparto (A3) |
| 4 | `energetica` | Ingesta, leche, reservas (A4) |
| 5 | `crecimiento` | Crecimiento de West (A5) |
| 6 | `reproduccion`, `nacimientos` | Madurez, concepción, aborto, parto, destete (A6) |
| 7 | `mortalidad`, `aplicar_muertes` | Inanición, edad máxima, Siler, orfandad (A7) |
| 8 | `envejecimiento` | Edad y fin de la dependencia (A8) |
| 9 | `escrituras` | El consumo se descuenta del forraje (A9) |

Diseño: `docs/design/agentes/` (incremento 1 y revisión 1).

---

## 4. El forrajeo (`abm`)

Rebrote, orden, forrajeo, reproducción, mortalidad y aplicación de cambios
(`abm-ruleset/src/systems/`). Escribe en JSON o en PostgreSQL (`[persistence] sink`). Diseño:
`docs/design/inicial/` y `docs/design/persistencia/`.

---

## 5. Dónde está cada cosa

- **Parámetros:** `config/*.toml`; los experimentos, con `--set`.
- **La Tierra para comparar:** `config/referencias/`: relieve (ETOPO, Paxman), bordes de Bird medidos en la malla, nivel del mar con la regla del modelo, la corteza real (CRUST1.0) con la regla del modelo y lluvia.
- **Medir y verificar:** `herramientas/`:
  - `verificar.sh` (CLAUDE §10);
  - `escenarios.py` (batería);
  - `no_regresion.sh`;
  - `planeta_semillas.py` (tabla de cierre con 5 semillas);
  - `analisis/tierra_regla_modelo.py` y `analisis/tierra_corteza_regla.py` (la Tierra real con la regla del modelo: el océano y los continentes);
  - `analisis/estudio_isostasia.py` (estudio de isostasia continental: Lamb et al. 2020 y las reglas R0–R4) y `analisis/nota_isostasia_s0.py` (su ajuste sin la topografía dinámica, la Tierra por clase de edad y las edades de partida);
  - `analisis/supercontinente.py` (diagnóstico del supercontinente, sobre las corridas de `planeta_semillas.py`, con los márgenes como entidades);
  - `analisis/`.
- **Diagnósticos de la tectónica:** las pruebas `#[ignore]` de `crates/abm-terreno/tests/tectonica.rs`:
  - `deriva_en_varias_semillas`;
  - `diagnostico_demografia_placas`;
  - `diagnostico_procedencia_del_fondo`;
  - `diagnostico_nivel_del_mar_y_edad_del_fondo` (el nivel del mar con las edades del fondo del modelo y con las de la Tierra);
  - `calibracion_*`.
- **Diseño y resultados:** `docs/design/<fase>/`, con su `README.md` en lenguaje llano y la nota técnica de cada incremento.
- **Informes de cada ronda:** `docs/informes/`.

---

## 6. Estado (2026-11-08)

- **Forrajeo y persistencia:** cerrados.
- **Agentes:** incremento 1 y revisión 1 hechos; siguen las cohortes y los individuos persistentes.
- **Terreno:** incrementos 1–3 cerrados.
  - El 4 (relieve tectónico) está en medida: antes de congelarlo se arregla el exceso de fosas (nota de demografía de placas).
  - **Hundimiento de los márgenes, puntos calientes y succión** (quinta ronda del 2026-11-08, sin fusionar): cada uno hace lo que pide su nota (los márgenes se hunden como McKenzie; magmatismo intraplaca y grandes erupciones terrestres; el tirón, ~50 % de la potencia). Pero con la succión los continentes chocan más y **los choques no se agotan nunca** (1,1–1,4 cm/año aun con más de 80 Myr): las placas no se sueldan, se destruye corteza (−13 %), el mar baja a −202 m y la tierra sube a 963 m. Decide el responsable (informe `docs/informes/2026-11-08_margenes_calientes_succion.md`).
  - **Isostasia con litosfera** (cuarta ronda del 2026-11-08, en `main`): la regla R4 con la edad tectonotérmica de cada celda y la erosión recalibrada (`E_1` 0,02). La tierra baja a 781 m de media (la Tierra 671) y a un 7 % por encima de 2 000 m (5,3 %); el nivel del mar, a −140 m. **Los márgenes estirados casi no se hunden al enfriarse** (0,38 km en 100 Myr, frente a 2–3): decide el responsable.
  - Diagnóstico del supercontinente (cuarta ronda): los océanos de rift se abren pero casi nunca se cierran, y los continentes no convergen; el modelo no tiene succión de la fosa sobre la placa de arriba.
  - Nota de demografía v2 y nota de rifts v2 implementadas (tercera ronda).
  - Lejos de la Tierra quedan la longitud de fosa (×1,7) y el fondo que se hunde, la tierra baja y la plataforma, el sedimento al mar, la pérdida en los choques, los márgenes pasivos y una corteza demasiado joven (el 71 % de la tierra de menos de 500 Myr).
  - Tiempo: regla relativa (≤ +5 % frente a la versión anterior en la misma tanda, techo 330 s). Con las tres notas, 259,5 s frente a 247,2 de `main` a la vez: +5,0 %, justo en el límite.
  - Ver `docs/design/terreno/incremento-4-diseno.md` §73–§74 y el informe de la ronda en `docs/informes/`.
