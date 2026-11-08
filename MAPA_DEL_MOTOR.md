# MAPA_DEL_MOTOR.md — Qué calcula el motor, en qué orden y dónde

> Para orientarse en el motor sin leer el código: qué hace cada binario, en qué orden corren sus
> sistemas, qué reglas aplica cada uno, dónde está su código y su diseño, qué parámetros mandan y en
> qué estado está cada pieza.
>
> Complementa a `TREE.md` (qué archivos hay) y a los documentos de `docs/design/` (el porqué de cada
> regla). Se actualiza en cada ronda que añade, quita o reordena una regla.

Última actualización: 2026-11-08 (tercera ronda) — rama `terreno-demografia-placas`, sin fusionar con `main`: nota de demografía de placas v2 (fuera la soldadura A1; subducción de dorsales, rama D, con la fosa delante de la dorsal) y nota de rifts v2 (reloj de cada bloque continental). Estudio de isostasia continental (Lamb et al. 2020 y reglas R0–R4) con la Tierra real, sin cambiar el motor: decide el responsable. En `main` (7c54bbd) la etapa 12 tiene la absorción de toda placa de menos de `A_min` sin losa y los rifts sin reloj.

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
| 1 | Velocidades | Equilibrio de fuerzas por placa: tirón de la losa, empuje de dorsal, rozamiento basal (recalibrado a `v_obj`), resistencia de los choques | `velocidades` | principal §6.2; nota de fosas v2 |
| 2 | Giro | Cada placa gira con su velocidad angular (y el empujón de apertura de un rift) | `dar_paso` | principal §6.1 |
| 3 | Rasterizar, solapes, dientes, huecos | Qué placa ve cada celda del mundo; los solapes se consumen (fosa) o chocan; los dientes de borde se recolocan; los huecos se rellenan con fondo nuevo | `rasterizar`, `resolver_solapes`, `recolocar_dientes`, `rellenar_huecos`, `reconstruir_mapas` | nota de bordes |
| 4 | Rifts que se estiran | La abertura de un rift joven se rellena estirando la corteza de sus bordes en rampa hasta la rotura | `rifts.rs: estirar_rifts` | adenda v8 §6.10 |
| 5 | Bordes suavizados | Pertenencia suavizada a `L_borde`, islas absorbidas y convergencia difusa | `suavizado.rs`; `fosas.rs: absorber_difusa` | nota de bordes; nota de fosas §6.2 |
| 6 | Clasificar bordes | Tipo de cada celda de borde (dorsal, fosa, choque, transformante, difusa), su velocidad relativa y el estado de su fosa | `clasificar_bordes`, `aplicar_estado_fosas` | nota de bordes; nota de fosas |
| 7 | Procesos de fosa | Régimen del margen; arcos, erosión por subducción y prisma de acreción | `procesos_de_fosa` | adenda §6.1–6.4 |
| 8 | Choques | Lo que se solapa en un choque se queda arriba con `η = a·η_arco + (1 − a)·η(h)` y engruesa el frente | `reconstruir_mapas`, `engrosar_choques` | adenda §6.5 (v7 y v9) |
| 9 | Delaminación | La raíz de más de `h_del` se desprende | `delaminar` | adenda §6.6 |
| 10 | Flujo lateral | La corteza inferior de más de `h_flujo` se desparrama: `D = D_0·(c/c_ref)³` | `flujo_lateral` | adenda v6 §6.6b |
| 11 | Erosión y sedimento | `E = E_1·z^n_ero` en la tierra emergida; nivel del mar con el agua de la Tierra; el sedimento va a las cuencas cerradas y al mar abierto, hasta la plataforma | `erosion_y_envejecimiento`, `nivel_del_mar`, `sedimento_al_mar` | adenda v5 §6.7 |
| 12 | Eventos de placas | Suturas (dos placas en choque lento se funden; reinician el reloj de los bloques que se juntan), rifts (por bloques y por suturas, con la probabilidad que crece con el tiempo que el bloque lleva unido), subducción de dorsales (una dorsal con la fosa delante a menos de `d_dc`: su franja pasa a la placa del otro lado y la fosa se apaga), captura de microplacas, saltos de dorsal | `suturas`, `rifts.rs: rifts`, `subduccion_de_dorsales`, `capturas`, `saltos_de_dorsal` | principal §6.4–6.5; nota de rifts v2; nota de demografía de placas v2 (A2 y rama D) |
| 13 | Fosas | Estado de cada tramo convergente: nacimiento inducido, muerte, rotura de la losa; invasión de transformantes y márgenes pasivos; nacimientos espontáneos | `fosas.rs: avanzar_fosas`, `invadir`, `nacimientos_espontaneos` | nota de fosas v6 |
| 14 | Métricas | Métricas del Myr, balance de la corteza (error ~10⁻¹⁵), márgenes pasivos y su perfil, bloques continentales | `metricas_del_paso` | adenda §8; notas |

**Parámetros que mandan** (`[tectonica]` de `config/terreno.toml`, donde cada uno lleva su origen):

| Grupo | Parámetros | Estado |
|---|---|---|
| Planeta y agua | `gel_m` 2 617 m, `f_cont` 0,40 al empezar | medido; supuesto |
| Isostasia y fondo | densidades; curva de profundidad (`d_inf` 5 310, `a_pl` 2 373, `τ_pl` 31) | medido; ajustado al zócalo real (Seton, GlobSed) |
| Isostasia continental | `h_nivel` 32 km (corteza a nivel del mar), con `ρ_c`, `ρ_s` y `ρ_m` | supuesto (relación empírica). **Con la corteza real de CRUST1.0 no da el relieve real** (la tierra al doble de altura). El estudio de isostasia continental (tercera ronda del 2026-11-08) propone una regla de litosfera completa (R4: litosfera de la edad tectonotérmica, corteza más densa abajo y sedimento de relleno), que aún no cumple todos sus criterios: decide el responsable |
| Fuerzas | `f_sp`, `f_rp`; `c_col` 90 000; `v_obj` 4,39 cm/año | supuesto; calibrado (India–Asia); medido (NNR-MORVEL56) |
| Bordes | `θ_tr` 20°; `L_borde` 150 km | medido (Bird); calibrado (longitudes de Bird) |
| Fosas | `X_ini` 150 km; `τ_muerte` 10; `λ_esp` 4·10⁻⁶; `v_inv` 25 km/Myr; `a_inv` 60; `t_rot` 10–25 Myr | supuesto (rango de modelos); supuesto; calibrado (vida de los márgenes) |
| Arcos y erosión por subducción | `p_arc_oc` 1,2, `p_arc_cont` 0,8, `p_se` 0,4; `f_acr` 0,2 | medido (Clift et al. 2009); calibrado |
| Choques | `η_arco` 0,9; `η_min` 0,5 (techo), `h_1` 20, `h_2` 35 | medido (Luzón–Taiwán); calibrado; físico |
| Delaminación y flujo | `h_del` 50; `τ_del` 25; `h_flujo` 50; `D_0` 400 | físico; calibrado; físico; calibrado (India–Asia, 70–80 km) |
| Erosión | `E_1` 0,0075; `n_ero` 2,5 | calibrado (relajación de un orógeno en 100–200 Myr); rango de Montgomery y Brandon |
| Rifts | `λ_A` 4·10⁻⁴; `τ_ais` 100 Myr (reloj del bloque, nota de rifts v2); `A_trozo,min` 0,5 Mkm²; `L_sut,min` 450 km; `W_m` 150; `h_rot` 10 | calibrado (placas de ≥ 1 %: 13,8 frente a 13 de Bird); supuesto (no se forma ningún supercontinente con que calibrarlo); físico; numérico; físico |
| Muerte de placas | `A_micro` 0,5 Mkm², `τ_micro` 10 Myr (A2); `d_dc` 300 km y `θ_dc` 45° (rama D; interruptor `subduccion_dorsales`) | supuestos; `θ_dc`, geométrico (la fosa delante de la dorsal, no a su costado) |

### 2.3 Del estado final de la deriva al relieve

`tectonica/mod.rs: estado_final` interpola la corteza, el sedimento, la edad y las fosas de la malla
tectónica (nivel 2, ~52 km) a las celdas del terreno y calcula la altura isostática. El ruido de
detalle fino continúa el espectro real (ETOPO), según la altura.

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
  - `analisis/estudio_isostasia.py` (estudio de isostasia continental: Lamb et al. 2020 y las reglas R0–R4);
  - `analisis/`.
- **Diagnósticos de la tectónica:** las pruebas `#[ignore]` de `crates/abm-terreno/tests/tectonica.rs`:
  - `deriva_en_varias_semillas`;
  - `diagnostico_demografia_placas`;
  - `diagnostico_procedencia_del_fondo`;
  - `calibracion_*`.
- **Diseño y resultados:** `docs/design/<fase>/`, con su `README.md` en lenguaje llano y la nota técnica de cada incremento.
- **Informes de cada ronda:** `docs/informes/`.

---

## 6. Estado (2026-11-08)

- **Forrajeo y persistencia:** cerrados.
- **Agentes:** incremento 1 y revisión 1 hechos; siguen las cohortes y los individuos persistentes.
- **Terreno:** incrementos 1–3 cerrados.
  - El 4 (relieve tectónico) está en medida: antes de congelarlo se arregla el exceso de fosas (nota de demografía de placas).
  - **La base de la isostasia continental falla con la corteza real** (segunda ronda del 2026-11-08). El estudio de isostasia continental (tercera ronda) da la regla que más se acerca, sin cumplir todos sus criterios: decide el responsable.
  - Nota de demografía v2 y nota de rifts v2 implementadas (tercera ronda): placas grandes en número terrestre (13,8), fosa ×1,40 y 3,55 km²/año; el fondo envejece (110 Myr al final). Sin la tabla de cierre del relieve, que espera a la isostasia.
  - Lejos de la Tierra quedan la longitud de fosa y el fondo que se hunde, el relieve demasiado alto, el sedimento al mar, la pérdida en los choques y los márgenes pasivos del estado inicial.
  - Ver `docs/design/terreno/incremento-4-diseno.md` §65 y el informe de la ronda en `docs/informes/`.
