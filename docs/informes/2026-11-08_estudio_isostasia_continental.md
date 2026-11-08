# Informe: estudio de isostasia continental (2026-11-08, tercera ronda)

Protocolo del responsable: `docs/design/terreno/estudio_isostasia_continental.md`. Es un estudio con la Tierra real: **no cambia el motor**.

- Herramienta: `herramientas/analisis/estudio_isostasia.py` (~5 min con las cachés).
- Resultados: `config/referencias/tierra_isostasia.toml`.

## Resumen

- **La reproducción de Lamb et al. (2020) sale.**
  - Sus constantes salen de sus rejillas: α = 7,8 y β = 93 (en el 69 % de las celdas el cociente es exactamente ese).
  - Con la referencia de su texto (32 km de corteza a nivel del mar con 100 km de litosfera), CRUST1.0 y CAM2016:
    - en la tierra continental, nuestra anomalía se correlaciona 0,92 con la suya, con 307 m de diferencia RMS (la suya varía ±490 m);
    - su reducción a 100 km de litosfera coincide al metro con su tabla por continentes.
- **Ninguna regla cumple todo el §5.** La mejor es R4:
  - litosfera completa con su grosor de la edad tectonotérmica (con recuperación tras el evento), un manto arcaico algo más ligero, la densidad de la corteza creciente con la profundidad y el sedimento que rellena el hueco en vez de levantar la tierra;
  - **cumple** la altura media de la tierra (648 m, objetivo 671 ± 100) y el criterio de error: 451 m de error medio en las mitades de prueba, un 33 % **mejor** que R2 con la litosfera observada (677);
  - **no cumple** la tierra por debajo de 200 m (15,3 %, objetivo 28 ± 5), la de por encima de 2 000 m (3,2 %, justo debajo de 3,3–7,3), la plataforma (25,0 km, objetivo ≥ 26,8 a la misma resolución) ni los tres casos con nombre.
- **El grosor de la litosfera sacado de la edad funciona tan bien como el observado** (R3b 675 m, R2 677). El motor puede llevarlo.
- **Reparto del error de la regla actual** (Shapley, sobre el RMS):
  - la densidad de la corteza, ~45 %;
  - el sedimento que levanta, ~1/3;
  - la falta de manto litosférico, ~1/4.
- **Lo que no explica ninguna regla:**
  - las grandes llanuras bajas (la llanura rusa: R4 da 813 m, real 127);
  - el alto de Kenia (549 m, real 1 745);
  - el Tíbet (3 669 m, real 5 153).
  - Son, de cientos de metros a más de 1 km, más de lo que el §5 atribuye a la topografía dinámica. Las anomalías del propio Lamb en esos lugares son −483, +404 y +1 100 m.
- **Los parámetros ajustados acaban en los bordes de sus rangos publicados:** Δρ_ma 20, L_min 40, ρ_u 2 750, ρ_l 3 000, z_t 15. Los datos piden más de lo que esas leyes permiten: falta otra física (topografía dinámica, flexión).

## 1. Lo pedido

1. Reproducir la isostasia de litosfera completa de Lamb et al. (2020) con sus datos (Zenodo 4031646) y, si no se parece, corregir nuestra implementación antes de seguir.
2. Evaluar las reglas R0–R4 con la Tierra real: tabla de relieve, error por celda, por provincia y casos con nombre.
3. Ajustar solo lo que no tiene valor medido, con validación cruzada entre mitades de continentes.
4. Elegir la regla más sencilla que cumpla el §5 con variables que el motor puede calcular.
5. Repartir el error de R0 entre el manto litosférico, la densidad de la corteza y el sedimento.

## 2. Datos

| Dato | Fuente | Resolución | Uso |
|---|---|---|---|
| Relieve | ETOPO 2022 sin hielo, con el rebote de Paxman et al. (2022) | celdas de 26 km | la altura real |
| Corteza y sedimento | CRUST1.0 (Laske et al. 2013) | 1° | `h`, `s`, densidades y tipos de corteza |
| Litosfera de Lamb | CAM2016Litho (Priestley y McKenzie; EarthScope) | 2° | R2 y la reproducción |
| Litosfera térmica | TC1 (Artemieva 2006; isoterma de 1 300 °C) | 5° (la de 1° no es libre) | R2 |
| Litosfera sísmica | LITHO1.0 (Pasyanos et al. 2014) | 1° | R2 |
| Lamb et al. (2020) | Zenodo 4031646: ds02 (continentes), ds03 (anomalía), ds06 y ds07 (perturbaciones) | 1° | reproducción |
| Edad tectonotérmica | el tipo de corteza de CRUST1.0, alineado con el mapa de edades de Artemieva y Mooney (tabla `EDAD_TIPO`) | 1° | R3 y R4 |
| Océano | Seton et al. (2020), GlobSed | 26 km | qué es continente; el océano real en la tabla de relieve |

La edad de cada tipo de corteza es un supuesto del estudio: el punto medio de la edad que describe el tipo. Por ejemplo, Arcaico temprano 3 300 Myr; Proterozoico temprano y medio 1 750; Fanerozoico 300; orógenos activos 20–30; rifts 20; márgenes 120–150.

**Las mitades.** A son Eurasia, África, Australia y el resto. B son América (de 170° O a 30° O, con Groenlandia) y la Antártida.

## 3. Reproducción de Lamb et al. (2020)

**La regla de Lamb** (su figura 1): ΔH = ΔC·(ρ_m − ρ_c)/ρ_a − ΔL·(ρ_m − ρ_a)/ρ_a = ΔC/α − ΔL/β.

**Sus constantes, de sus rejillas.** La perturbación que anula cada anomalía, dividida entre la anomalía, da α = 7,8 (grosor de corteza) y β = 93 (grosor de litosfera) en el 69 % de las celdas.
- Con Δρ_ma = 35 kg/m³ (la «altura reducida» de su texto), ρ_a ≈ 3 255 y ρ_m − ρ_c ≈ 417 kg/m³ (ρ_c ≈ 2 873).
- La referencia es 32 km de corteza a nivel del mar con 100 km de litosfera, es decir, H = (C − 32)/7,8 − (L − 100)/93 en km.

**Comparación nodo a nodo** (sus nodos de grado entero; nuestra altura, la media de ETOPO en la caja de 1°):

| | Nodos | Correlación | Anomalía media, nuestra / suya (m) | Desviación, nuestra / suya (m) | Diferencia RMS (m) |
|---|---|---|---|---|---|
| **Tierra continental** | 14 729 | **0,924** | +16 / −158 | 625 / 492 | **307** |
| Todos, también el océano | 50 062 | 0,264 | −846 / +107 | 982 / 445 | 1 356 |

- En la tierra continental la reproducción es buena. Queda un desfase de ~170 m y algo más de dispersión: su altura observada (ETOPO1) y su litosfera no son exactamente las nuestras.
- Con la corteza solo cristalina (sin el sedimento), la correlación baja a 0,81: Lamb cuenta el sedimento como corteza.
- Sus anomalías del océano usan otra regla para el agua, que no publican; el estudio es de continentes.
- **Su tabla por continentes** (ds02): la altura reducida a 100 km que calculamos coincide con la suya en los once continentes. Su propia regla deja anomalías de −250 a +550 m en las medias continentales (por ejemplo, Norteamérica 290 m observados frente a 37 calculados).

## 4. Las reglas

H = [(ρ_a − ρ_c)·h + k_s·(ρ_a − ρ_s)·s − Δρ_ma·(L − h − s)]/ρ_a − H_0 sobre el mar. Bajo el mar, la profundidad se multiplica por ρ_a/(ρ_a − ρ_w).

| Regla | Qué es | Parámetros ajustados (con todos los continentes) |
|---|---|---|
| R0 | La regla del modelo (ρ_a 3 300, ρ_c 2 800, ρ_s 2 400, sin manto). «Tal cual» con su cero (`h_nivel` 32 km) y con H_0 ajustado | H_0 |
| R1 | R0 con ρ_u hasta z_t y ρ_l por debajo (Christensen y Mooney 1995) | ρ_u 2 750, ρ_l 3 000, z_t 15 km (los tres en el borde de su rango) |
| R2_lamb | Las constantes de Lamb, sin ajustar nada | — |
| R2 | Litosfera completa con L observado: CAM2016, TC1, LITHO1.0 (ρ_a 3 255) | ρ_c 2 890, Δρ_ma 20 (bordes), H_0 |
| R3a | Como R2 con L = 0,04·t + 93,6 km (Artemieva) | ρ_c 2 890, Δρ_ma 20 |
| R3b | L = L_min + (L_lin − L_min)·(1 − e^(−t/τ_L)) | ρ_c 2 890, Δρ_ma 20, L_min 40, τ_L 50 |
| R3b_relleno | R3b con el sedimento que rellena el hueco y no levanta (k_s = 0; McKenzie 1978, §1 del estudio) | los de R3b |
| R3c | R3b_relleno con un Δρ_ma propio del manto arcaico, el más empobrecido (variante opcional del §3: Δρ_ma según la edad) | Δρ_ma 20 y 15 (arcaico), L_min 40, τ_L 50 |
| **R4** | R3c con la densidad de la corteza de R1 (R1 aporta) | ρ_u 2 750, ρ_l 3 000, z_t 15; Δρ_ma 20 y 10 (arcaico); L_min 40, τ_L 100; H_0 3 005 m |

Rangos: ρ_u 2 700–2 750, ρ_l 2 900–3 000, z_t 15–25 km, ρ_c 2 800–2 890, Δρ_ma 20–40 (el arcaico, 0–20), L_min 40–80 km y τ_L 20–200 Myr.

## 5. La tabla del §4

**Relieve a 26 km**, con el océano real, el agua del modelo y su nivel del mar. Los parámetros son los ajustados con todos los continentes. La Tierra a 1° es la referencia para la pendiente y la plataforma (CRUST1.0 es de 1°).

| | Tierra | Tierra a 1° | R0 tal cual | R0 | R1 | R2_lamb | R2 CAM2016 | R2 TC1 | R2 LITHO1.0 | R3a | R3b | R3b_relleno | R3c | **R4** |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Altura media de la tierra (m) | 671 | 658 | 1 293 | 1 108 | 744 | 831 | 800 | 852 | 925 | 864 | 862 | 847 | 847 | **648** |
| Tierra < 200 m | 0,283 | 0,268 | 0,065 | 0,075 | 0,151 | 0,128 | 0,144 | 0,142 | 0,115 | 0,156 | 0,155 | 0,128 | 0,107 | **0,153** |
| Tierra > 2 000 m | 0,053 | 0,046 | 0,167 | 0,106 | 0,037 | 0,057 | 0,043 | 0,056 | 0,070 | 0,063 | 0,066 | 0,063 | 0,061 | **0,032** |
| Pendiente (±25 km) | 0,0052 | 0,0029 | 0,0046 | 0,0045 | 0,0036 | 0,0040 | 0,0040 | 0,0040 | 0,0049 | 0,0045 | 0,0047 | 0,0045 | 0,0042 | **0,0033** |
| Plataforma (km) | 42,8 | 53,5 | 19,6 | 20,7 | 27,5 | 24,3 | 27,2 | 26,7 | 18,5 | 22,9 | 21,6 | 17,8 | 18,1 | **25,0** |
| Nivel del mar (m) | 0 | 0 | −84 | −233 | +7 | −94 | −21 | −46 | −49 | −38 | −34 | −85 | −94 | +31 |

**Error por celda de tierra continental a 1°** (m; error medio absoluto, y entre paréntesis el sesgo). Ajuste en una mitad y prueba en la otra:

| Regla | Ajuste A → prueba B | Ajuste B → prueba A | **Prueba, media** | Todo (RMS) |
|---|---|---|---|---|
| R0 | 723 → 807 (−251) | 769 → 738 (+243) | 772 | 746 (943) |
| R1 | 635 → 634 (−186) | 620 → 646 (+184) | 640 | 631 (805) |
| R2_lamb (sin ajustar) | — → 701 (−403) | — → 532 (+32) | 616 | 593 (771) |
| R2 CAM2016 | 550 → 718 (−380) | 624 → 635 (+349) | **677** | 601 (769) |
| R2 TC1 | 625 → 709 (−280) | 651 → 650 (+267) | 680 | 645 (816) |
| R2 LITHO1.0 | 607 → 899 (−553) | 742 → 732 (+499) | 816 | 692 (876) |
| R3a | 640 → 699 (−272) | 644 → 661 (+262) | 680 | 650 (825) |
| R3b | 626 → 698 (−278) | 641 → 651 (+265) | 675 | 641 (816) |
| R3b_relleno | 507 → 555 (−141) | 546 → 522 (+141) | 539 | 523 (677) |
| R3c | 481 → 558 (−134) | 540 → 500 (+137) | 529 | 507 (661) |
| **R4** | 409 → 464 (−103) | 463 → 437 (+106) | **451** | 432 (564) |

- **Sobreajuste.** La prueba sale como mucho un 13 % peor que el ajuste en las reglas con relleno, y R4 un 9 %. R2 con LITHO1.0 sí se sobreajusta (607 → 899).
- **El sesgo entre mitades.** Con el mismo cero, América y la Antártida quedan ~100–400 m por debajo de lo que se ajusta en Eurasia, África y Australia, y al revés. Es la diferencia de altura media entre los dos grupos que ninguna regla explica.

## 6. Error por provincia (m: sesgo / error medio; parámetros de todo)

| Provincia | Tierra | Real | R0 | R2 CAM2016 | R3b | R3c | **R4** |
|---|---|---|---|---|---|---|---|
| Escudo arcaico | 15,6 % | 543 | +134 / 602 | −95 / 526 | −498 / 706 | −83 / 468 | **+96 / 398** |
| Plataforma proterozoica | 46,5 % | 417 | +255 / 742 | +185 / 600 | +190 / 656 | +54 / 524 | **+12 / 413** |
| Cinturón fanerozoico | 18,8 % | 849 | −125 / 685 | −18 / 570 | +92 / 581 | +90 / 490 | **+55 / 411** |
| Rift o zona joven | 6,5 % | 698 | −1 122 / 1 163 | −502 / 654 | −397 / 620 | −356 / 524 | **−154 / 443** |
| Orógeno activo y arco | 10,7 % | 1 641 | −348 / 822 | −351 / 749 | −46 / 612 | −30 / 519 | **−213 / 608** |
| Margen pasivo | 1,9 % | 198 | −363 / 791 | +103 / 505 | +137 / 564 | −163 / 460 | **+90 / 336** |

Con la litosfera de la edad, los escudos arcaicos quedaban 500 m bajos (R3b). Un manto arcaico algo más ligero los corrige (R3c, R4): es lo que se espera de su empobrecimiento (Jordan 1978; Poudjom Djomani et al. 2001).

## 7. Casos con nombre (celda de 1°; m)

| Caso (criterio) | Real | R0 tal cual | R2_lamb | R2 CAM2016 | R3b | R3c | **R4** |
|---|---|---|---|---|---|---|---|
| Llanura rusa (< 400) | 127 | 2 076 | **391** | 836 | 1 141 | 1 072 | 813 |
| Kenia (> 1 200) | 1 745 | 88 | 520 | 205 | 183 | 376 | 549 |
| Tíbet (4 000–5 500) | 5 153 | 5 835 | 3 497 | 3 580 | **4 575** | **4 845** | 3 669 |
| Meseta del Colorado | 1 883 | 1 674 | 1 641 | 1 534 | 1 376 | 1 071 | 1 043 |
| Grandes Llanuras | 483 | 1 256 | −169 | 327 | 512 | 391 | 327 |

Ninguna regla cumple los tres criterios a la vez.
- La llanura rusa solo baja de 400 m con las constantes de Lamb.
- Kenia no pasa de 600 m con ninguna regla: su corteza es de 32 km y su litosfera de 56 km (CAM2016). Su altura la sostiene la pluma de África oriental.
- El Tíbet cumple con R3b y R3c, pero R4 lo deja en 3,7 km: su corteza inferior densa (3 000 kg/m³ desde 15 km) pesa mucho en una corteza de 70 km.

## 8. Reparto del error de R0 (§4.5; Shapley sobre las 8 combinaciones, cada una ajustada)

| | R0 | Con las tres cosas | Manto litosférico | Densidad de la corteza | Sedimento que no levanta |
|---|---|---|---|---|---|
| RMS (m) | 943 | 614 | 74 (22 %) | 149 (45 %) | 106 (32 %) |
| Error medio (m) | 746 | 459 | 69 (24 %) | 123 (43 %) | 95 (33 %) |

Además del reparto de la forma, R0 tal cual tiene un sesgo de +620 m en la altura media: su cero (`h_nivel` 32 km) es el de una corteza joven sobre un manto caliente.

## 9. Los criterios del §5

Objetivos:
- altura media de la tierra, 571–771 m;
- tierra por debajo de 200 m, 23–33 %;
- tierra por encima de 2 000 m, 3,3–7,3 %;
- plataforma, ≥ 26,8 km (la mitad de la de ETOPO a 1°);
- error medio, como mucho 1,3 veces el de R2 con L observado (677 m → ≤ 880 m);
- casos con nombre: la llanura rusa por debajo de 400 m, Kenia por encima de 1 200 m y el Tíbet entre 4 y 5,5 km.

| Regla | Altura media | < 200 m | > 2 000 m | Plataforma | Error ≤ 1,3·R2 | Rusa | Kenia | Tíbet | Cumple |
|---|---|---|---|---|---|---|---|---|---|
| R0 tal cual | ✗ | ✗ | ✗ | ✗ | — (sesgo de +620 m) | ✗ | ✗ | ✗ | 0 de 7 |
| R0 (H_0 ajustado) | ✗ | ✗ | ✗ | ✗ | ✓ (772) | ✗ | ✗ | ✓ | 2 de 8 |
| R1 | ✓ | ✗ | ✓ | ✓ | ✓ | ✗ | ✗ | ✗ | 4 de 8 |
| R2_lamb | ✗ | ✗ | ✓ | ✗ | ✓ | ✓ | ✗ | ✗ | 3 de 8 |
| R2 CAM2016 | ✗ (800) | ✗ | ✓ | ✓ | (referencia) | ✗ | ✗ | ✗ | 2 de 7 |
| R3b | ✗ | ✗ | ✓ | ✗ | ✓ | ✗ | ✗ | ✓ | 3 de 8 |
| R3c | ✗ | ✗ | ✓ | ✗ | ✓ | ✗ | ✗ | ✓ | 3 de 8 |
| **R4** | ✓ | ✗ | ✗ (3,2) | ✗ (25,0) | ✓ (451) | ✗ | ✗ | ✗ | 2 de 8 |

**Ninguna regla cumple el §5.**
- Lo que más se resiste es la tierra baja: ninguna regla pasa del 17,6 % frente al 28 %. Ese 17,6 % es la R4 sin el manto arcaico; la R4 final da 15,3 %. Y los casos con nombre.
- R4 es la que más se acerca en la forma: el menor error por celda, sin sesgo por provincias y la altura media.
- R1 cumple más casillas (4) con menos física, pero su error es mayor (640 m) y deja los rifts 700 m bajos.

## 10. Lo que dicen los datos

1. **La isostasia de litosfera completa explica mucho, pero no todo.**
   - Con la litosfera observada (CAM2016), el error medio baja de 772 m (R0) a 677.
   - Con la litosfera de la edad, el relleno de las cuencas, el manto arcaico y la corteza densa abajo, baja a 451.
2. **Lo que queda son 450–560 m de error medio por celda**, con sesgos que no dependen del tipo de corteza:
   - la llanura rusa y las Grandes Llanuras, demasiado altas;
   - Kenia, la meseta del Colorado y el Tíbet, demasiado bajos;
   - América y la Antártida, más bajas que el resto con el mismo cero.
   - Es la escala de la topografía dinámica: el manto que sube bajo África oriental y baja bajo las cuencas cratónicas. También la de la flexión. Pero en los casos con nombre pasa de 1 km, más que los «unos cientos de metros» del §5.
3. **Los parámetros se van a los bordes de sus rangos.**
   - Δρ_ma al mínimo (20).
   - La corteza inferior a lo más denso (3 000) y desde lo más somero (15 km).
   - L_min al mínimo (40 km).
   - Con valores dentro de lo publicado, las reglas no pueden dar más.
4. **La edad tectonotérmica sirve:** R3b (de la edad) da el mismo error que R2 con la litosfera observada. Para el motor basta con llevar la edad, no el grosor de la litosfera.
5. **El sedimento rellena:** quitar su flotación es el cambio que más mejora R3 (675 → 539 m). Es lo que dice el §1 del estudio: ocupa el hueco que deja la litosfera al hundirse.

## 11. Para la nota de diseño (§7 del estudio; decide el responsable)

Ninguna regla cumple el §5, así que el estudio no puede elegir una sola. Las opciones:

1. **Adoptar R4 como base** y llevar lo que no explica a la topografía dinámica (ampliación futura).
   - Es la más realista de las medidas y usa solo variables que el motor puede llevar:
     - la edad tectonotérmica de cada columna (la reinician los rifts, los arcos, los choques y la delaminación);
     - si es manto arcaico (los cratones de partida);
     - la densidad de la corteza por profundidad;
     - el sedimento como relleno.
   - Daría una tierra de altura media terrestre y sin sesgo por provincias, pero con un 15 % de tierra baja (Tierra 28 %) y un 3 % por encima de 2 km (Tierra 5,3 %).
2. **Igual, pero con la tierra alta.** R4 sin el manto arcaico propio dejaba un 3,3 % por encima de 2 km y un 17,6 % por debajo de 200 m, con algo más de error (468 m).
3. **Añadir una topografía dinámica simple** (subida sobre los rifts y las zonas calientes, bajada en las cuencas cratónicas), con su propio estudio. Es lo que falta para la tierra baja y los casos con nombre.
4. **Revisar los criterios del §5** para los casos que ni la regla de Lamb cumple (Kenia, la llanura rusa), que son de topografía dinámica.

En cualquiera de ellas, después hay que recalibrar la erosión (`E_1`, `n_ero`) y el nivel del mar, y dar a los continentes de partida edades terrestres (cratones viejos).

## 12. Estado

- Sin cambios en el motor por el estudio. Son nuevos:
  - la herramienta `herramientas/analisis/estudio_isostasia.py`;
  - su salida, `config/referencias/tierra_isostasia.toml`;
  - este informe.
- Datos en `output/referencias/` (ignorado por git): `cam2016/`, `tc1/` y `lamb2020/`.
- Rama `terreno-demografia-placas`, commit `cf1b87d` (con la nota de demografía v2 y la de rifts v2; ver su informe).

## Referencias

- Artemieva, I. M. (2006). Global 1°×1° thermal model TC1 for the continental lithosphere. Tectonophysics 416.
- Christensen, N. I. y Mooney, W. D. (1995). Seismic velocity structure and composition of the continental crust: a global view. Journal of Geophysical Research 100.
- Hasterok, D. y Chapman, D. S. (2007). Continental thermal isostasy. Journal of Geophysical Research 112.
- Jordan, T. H. (1978). Composition and development of the continental tectosphere. Nature 274.
- Lamb, S., Moore, J. D. P., Perez-Gussinye, M. y Stern, T. (2020). Global whole lithosphere isostasy. Geochemistry, Geophysics, Geosystems 21; datos en Zenodo, doi:10.5281/zenodo.4031646.
- Laske, G. et al. (2013). Update on CRUST1.0. Geophysical Research Abstracts 15.
- McKenzie, D. (1978). Some remarks on the development of sedimentary basins. Earth and Planetary Science Letters 40.
- Pasyanos, M. E. et al. (2014). LITHO1.0. Journal of Geophysical Research: Solid Earth 119.
- Poudjom Djomani, Y. H., O'Reilly, S. Y., Griffin, W. L. y Morgan, P. (2001). The density structure of subcontinental lithosphere through time. Earth and Planetary Science Letters 184.
- Priestley, K. y McKenzie, D. (CAM2016Litho, EarthScope EMC): grosor térmico de la litosfera a 2°.
