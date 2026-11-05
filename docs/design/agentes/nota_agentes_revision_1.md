# Nota de diseño: revisión 1 de la biología de los agentes

**Modifica:** `docs/design/agentes/diseno_biologia_agentes.md` (v1.0 → v1.1)
**Estado:** Las cuatro decisiones están aprobadas por el director. Lista para implementar.
**Naturaleza:** Plano teórico. Sin código.
**Origen:** Hallazgos de la implementación del incremento 1 de agentes (commit ece0868).

Resumen de lo que cambia:
| # | Cambio | Sección afectada del diseño |
|---|---|---|
| 1 | Apetito en energía aprovechable y referido al gasto total del día (D1 + hiperfagia) | §6.4, §6.5 |
| 2 | Consumo con respuesta funcional Holling tipo II, encendida por defecto (D16) | §6.4, §6.9 |
| 3 | Leche según la condición de la madre + destete forzoso + dimorfismo sexual en el preset humano | §5.2, §6.5, §6.7, §6.8, §6.9 |
| 4 | Edad de esterilidad individual | §5, §6.7, §6.3, §6.9 |

Los cuatro cambios se pueden apagar por separado en configuración, para comparar con la salida del incremento 1.

---

## 1. Apetito en energía aprovechable (D1) y referido al gasto total

### Problema
En la v1.0 el apetito era energía bruta: `Ap = k_ap · B · PAL`, con `k_ap = 2` y `ε = 0,5`. Como mucho se asimilaba `2·G·0,5 = G`, exactamente lo gastado, así que nunca había sobrante para rehacer reservas ni para criar. Fue un error de diseño.

### Regla nueva
```
G_total,i = G_i del día (mantenimiento y actividad + crecimiento + gestación + lactancia)
            evaluado con el estado al INICIO del tick (sin circularidad)

Ap_asim,i = k_ap · G_total,i                       (máximo de energía ASIMILADA por día, kcal)

Demanda bruta:  d_i = min( Ap_asim,i , max(0, E_max(m_i) − E_i + G_total,i) ) / ε
```
- El apetito se expresa en energía que el cuerpo aprovecha. La demanda bruta (forraje a retirar del terreno) es esa cantidad dividida por la eficiencia digestiva `ε`.
- Al referirse al gasto total, las madres gestantes y lactantes quieren comer más (**hiperfagia de la lactancia**, un fenómeno real), sin tocar `k_ap`.
- Cría dependiente: su demanda se sigue multiplicando por `(1 − φ_leche(a))`, porque esa es su capacidad de comer sólido (ver §3).
- Modo población: misma regla por clase, con `ē_k` y el `G_total` medio de la clase.

### Validación
- Con forraje abundante, un adulto rehace reservas: `E/E_max` sube hacia 1.
- Una madre lactante con forraje abundante mantiene su condición.

---

## 2. Respuesta funcional Holling tipo II (D16)

### Problema
Con el reparto de la v1.0 (§6.4), cada animal come lo que pide mientras quede forraje, así que nadie nota la escasez hasta que la comida se acaba. Resultado: la población supera la capacidad, agota el forraje y se extingue (13 años con clima constante).

### Fenómeno real
La **respuesta funcional** es cómo cambia lo que come un animal según la comida disponible. Cuando hay poco forraje, encontrarlo y comerlo cuesta más tiempo, así que cada animal come menos antes de que se acabe. Eso regula la población.

- Sin este efecto, o cuando el forraje rebrota muy despacio, en la realidad hay sobrepasamiento y colapso: los renos de la isla de St. Matthew pasaron de 29 a ~6 000 y cayeron a 42 en dos décadas.
- La forma estándar es la de Holling tipo II: lo comido crece con el forraje disponible y se satura.

### Regla nueva (encendida por defecto)
```
x_i = d_i · V_c / ( V_c + V_½ )                    (ingesta bruta antes del tope)
V_½ = h_½ · K_c                                    (forraje al que se come la mitad de lo deseado)

Tope de reparto justo (se mantiene de la v1.0):
si Σ_j x_j > F :   x_i ← x_i · F / Σ_j x_j        (F = V_c · A_celda · ρ_veg)
```
- **Individuos:** `V_c` es el de su celda fina.
- **Poblaciones:** `V_c` es la densidad media de la celda de nivel 0, es decir, el stock total de sus descendientes entre su área total.

### Valor por defecto de `h_½` (derivado, con posibilidad de override)
En equilibrio, cada animal debe asimilar lo que gasta. Si desea asimilar `k_ap · G`, el equilibrio llega cuando `V/(V + V_½) = 1/k_ap`, es decir:
```
V_eq = V_½ / (k_ap − 1)
```
Para que el equilibrio caiga en el punto de máximo rebrote del forraje (`V = K_c/2`), lo que da la mayor población sostenible:
```
h_½ = (k_ap − 1) / 2          (con k_ap = 2 → h_½ = 0,5)
```
Con costes de reproducción, el gasto medio supera algo al de mantenimiento, así que el equilibrio real queda algo por encima de `K_c/2` y la población algo por debajo de `N_max_sostenible`. Es esperable.

### Parámetros
| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `respuesta_funcional` | activa la respuesta funcional | true | — |
| `h_½` | semisaturación relativa a `K_c` | `(k_ap − 1)/2` = 0,5 | — |

### Validación
- **Clima constante, 100 años:** sin extinción. La población converge (se admiten oscilaciones amortiguadas) a `N_eq / N_max_sostenible` entre 0,7 y 1,2.
- **Densidad frente a la productividad** en desierto, tundra, templado y selva, y **equivalencia energética** entre masas distintas (10 y 300 kg). Repetir las pruebas que ya pasó la versión proporcional.
- **Prueba cualitativa de sobrepasamiento:** con un forraje de rebrote muy lento (`τ_c` grande, tipo liquen, ~20 años), debe poder aparecer un sobrepasamiento seguido de colapso. El modelo debe poder producir ambos regímenes según la ecología, no imponer uno.

---

## 3. Lactancia según la condición de la madre, destete forzoso y dimorfismo sexual

### Problema
Con una sequía del 50 % durante un año quedan 2 hembras frente a 3 159 machos. La gestante que pasa hambre aborta, pero la lactante da leche hasta morir. En la realidad ocurre lo contrario:
- En las hambrunas mueren antes las crías que las madres. En especies longevas, la madre prioriza su supervivencia y desteta o abandona.
- En humanos, las mujeres sobreviven a las hambrunas mejor que los hombres (Zarulli et al., 2018, PNAS).

### 3.1 Leche graduada por la condición de la madre
```
r = E_madre / E_max(m_madre)
g_leche(r) = clamp( (r − r_aband) / (r_lact − r_aband) , 0 , 1 )

Leche que recibe cada cría:   L_c = φ_leche(a_c) · g_leche(r) · G_cría
Coste para la madre:          Σ_crías L_c / ε_leche
```
- Con la madre en buena condición (`r ≥ r_lact`), la leche es completa. Por debajo, baja de forma lineal.
- **La cría no compensa la leche que falta comiendo más:** su capacidad de comer sólido sigue siendo `(1 − φ_leche(a))` (§1). La falta de leche la sufre la cría, que nota el hambre antes que la madre. Es lo realista.

### 3.2 Destete forzoso
```
si r < r_aband :   la lactancia termina (destete forzoso)
   cada cría con a ≥ f_dest_min · d_destete  → sobrevive destetada (φ_leche = 0, come solo sólido sin límite de capacidad)
   cada cría con a <  f_dest_min · d_destete → muere con probabilidad p_huerf (la misma regla de los huérfanos)
   la madre pasa a DISPONIBLE (su fertilidad será baja por f_E hasta que se recupere)
```
- Se evalúa en el paso de reproducción (A6), con la condición de la madre tras la energética del día.
- Evento nuevo: **destete forzoso** (madre, crías destetadas, crías muertas).

### 3.3 Modo población
Por cada bin de `Lq`:
```
madres que abandonan:  D_ab ~ Binomial( madres(Lq[j]) , F_Γ( r_aband · E_max ; forma = 1/cv_E² , escala = ē_F · cv_E² ) )
sus crías (en proporción): si la edad del bin a_j ≥ f_dest_min · d_destete → pasan a destetadas
                           si no → mueren con Binomial( · , p_huerf )
leche del resto: se usa g_leche( ē_F / E_max ) de la media de las lactantes
```
- La leche se calcula con la condición media de las lactantes, no con la media de cada condición individual. Es una aproximación de nivel de detalle, aceptable y cubierta por la prueba de coherencia entre modos.

### 3.4 Dimorfismo sexual (solo preset humano; en el genérico, apagado)
Cada sexo tiene su propia masa adulta y su propia fracción máxima de grasa. Kleiber, la curva de crecimiento, `E_max` y el apetito se calculan por sexo con su masa.

| Símbolo | Mujer | Hombre | Unidades |
|---|---|---|---|
| `M` | 56 | 64 | kg |
| `f_grasa_max` | 0,25 | 0,15 | — |
| `f_neo` (masa al nacer / masa de la madre) | 0,059 (≈ 3,3 kg) | — | — |

Cifras resultantes (verificadas), con `PAL` 1,8:
| | Basal | Gasto diario | Reserva máxima | Días de ayuno |
|---|---|---|---|---|
| Mujer | 1 433 kcal | 2 579 kcal | 126 000 kcal | ~49 |
| Hombre | 1 584 kcal | 2 851 kcal | 86 400 kcal | ~30 |

En el genérico, por defecto `M_macho = M_hembra = M` y la misma grasa: el dimorfismo queda desactivado. Es parametrizable por especie (ver Pregunta abierta Q2).

### Parámetros
| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `r_lact` | condición por debajo de la cual la leche empieza a bajar | 0,3 | — |
| `r_aband` | condición por debajo de la cual hay destete forzoso | 0,1 | — |
| `f_dest_min` | fracción de `d_destete` a partir de la cual la cría sobrevive destetada | 0,5 | — |
| `M_macho`, `M_hembra` | masa adulta por sexo | = M (genérico) | kg |
| `f_grasa_max_macho`, `f_grasa_max_hembra` | grasa máxima por sexo | = f_grasa_max (genérico) | — |

### Validación
- **Sequía del 50 % durante un año (el mismo escenario):** la mortalidad debe ordenarse así: crías > viejos > adultos. En el preset humano, la mortalidad adulta femenina debe ser menor o igual que la masculina. La proporción de sexos adultos debe quedar razonable (sin colapso de hembras) y la población debe recuperarse tras la sequía.
- **Ayuno humano:** ~49 días las mujeres y ~30 los hombres.

---

## 4. Edad de esterilidad individual

### Problema
Las humanas tienen ~10 hijos frente a ~6 en cazadores-recolectores (rango real: ~4,7 los !Kung, ~6,2 los Hadza, ~8 los Aché), porque la fertilidad no baja con la edad antes de los 50.

### Por qué la esterilidad y no una curva de fecundidad
El intervalo entre partos lo dominan la gestación (270 días) y la lactancia (900 días). La espera hasta concebir es solo de ~140 días. Por eso, multiplicar la probabilidad diaria de concebir por una curva de edad apenas cambia el total.

Cálculo de comprobación (Monte Carlo simplificado: `a_rep` 19, `p_conc` 0,007, gestación 270, lactancia 900, ~20 % de mortalidad infantil que acorta la lactancia):
| Variante | Hijos por mujer (si sobrevive a la edad reproductiva) |
|---|---|
| Actual (fértil hasta los 50) | 9,7 (cuadra con los ~10 medidos) |
| Solo curva de fecundidad por edad (forma de Coale-Trussell) | 8,4 |
| **Edad de esterilidad ~ Normal(38, 4)** | **~6,0** |

Lo que en la realidad corta la fertilidad es que **cada mujer queda estéril a una edad concreta**: en poblaciones sin anticoncepción, el último parto llega en promedio en torno a los 40 años.

### Regla nueva
```
Al nacer cada hembra:  a_est ~ Normal( μ_est , σ_est ) truncada a [a_rep , a_fin_rep]
   (generador por contador: H(semilla, id, "esterilidad"))
Concepción permitida solo si  a < a_est
Al alcanzar a_est: si está GESTANTE o LACTANTE, termina su ciclo; después pasa a POSTREPRODUCTIVA
```

**Valores:**
- Humano: `μ_est = 38 años`, `σ_est = 4 años`. Es el valor inicial; la calibración final debe hacerse sobre el modelo completo con el objetivo de ~6 hijos por mujer.
- Genérico (derivado): `μ_est = a_rep + 0,7 · (a_fin_rep − a_rep)`; `σ_est = 0,1 · (a_fin_rep − a_rep)`.

### Modo población
La fracción fértil de la clase `k` se calcula sin guardar nada nuevo:
```
fert_k = [ Φ((a_fin_rep − μ)/σ) − Φ((a_k − μ)/σ) ] / [ Φ((a_fin_rep − μ)/σ) − Φ((a_rep − μ)/σ) ]
         (con a_k ≥ a_rep;  Φ = función de distribución acumulada de la normal estándar)

F_disp = max( 0 , Σ_k N_F,k · fert_k − Σ Gq − Σ madres(Lq) )
```

### Conversión individuo ↔ población
- **Agregación:** la `a_est` individual se pierde. El modo población usa la distribución.
- **Desagregación:** cada hembra sortea su `a_est` de la distribución completa, de forma independiente de su edad. En media, la fracción fértil resultante coincide con `fert_k`. Una gestante o lactante creada con `a_est < a` es coherente: concibió antes de quedar estéril.

### Parámetros
| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `μ_est` | edad media de esterilidad | humano 38; genérico derivado | años |
| `σ_est` | desviación típica | humano 4; genérico derivado | años |

### Validación
- **Humano:** hijos por mujer ≈ 6 (aceptable 5-7); edad media del último parto ≈ 37-40.
- **Forma de la fertilidad por edades** (tasas por grupo de 5 años) comparable con la curva "natural" de Coale-Trussell: máximo a los 20-29 y caída fuerte a partir de los 35. Valores relativos de referencia: 20-24: 1,00; 25-29: 0,94; 30-34: 0,86; 35-39: 0,70; 40-44: 0,36; 45-49: 0,05.

---

## 5. Orden de ejecución (sin cambios estructurales)
- La hiperfagia (§1) y la respuesta funcional (§2) actúan en A3 (demanda y reparto).
- La leche graduada actúa en A4 (energética). El destete forzoso y la esterilidad actúan en A6 (reproducción).
- Las muertes por destete forzoso se resuelven en A7, con la regla de huérfanos.

## 6. Pruebas generales tras la revisión
- Las pruebas existentes de la v1.0 se mantienen: balance energético, independencia del orden de proceso, determinismo bit a bit y calibración de Kleiber y del ayuno.
- **Con los cuatro cambios apagados**, la salida debe ser idéntica a la del commit ece0868.
- La prueba de coherencia entre modos (individuos frente a poblaciones) debe incluir el escenario de sequía, porque el destete forzoso y la esterilidad tienen versión en los dos modos.

## 7. Preguntas abiertas
- **Q1 — Fecundabilidad decreciente con la edad.** Es un efecto real pero secundario (en el cálculo, de 6,1 a 6,0 hijos). Queda como opción futura.
- **Q2 — Dimorfismo en otras especies.** En muchos mamíferos los machos son mayores (ciervos ~1,3-1,5×). ¿Activarlo por especie cuando haya varias?
- **Q3 — Pérdidas fetales crecientes con la edad.** Sus efectos ya están absorbidos en `μ_est`; se separarían solo si se necesitan los abortos espontáneos como dato de salida.

---

## 8. Implementación y medidas (Claude Code, 2026-11-05)

Implementado para **individuos**. La versión de poblaciones de cada cambio queda anotada para el incremento 2 (§8.7).

### 8.1 Qué se ha implementado

| Cambio | Interruptor (`config/agentes.toml`) | Parámetros | Dónde actúa |
|---|---|---|---|
| 1. Apetito por gasto total | `energetica.apetito_gasto_total` | `k_ap` | A3 |
| 2. Holling II | `forraje.respuesta_funcional` | `h_medio` por especie (defecto `(k_ap − 1)/2`) | A3 |
| 3.1 Leche según la condición | `energetica.leche_por_condicion` | `r_lact`, `r_aband` | A3 (cálculo), A4 (leche) |
| 3.2 Destete forzoso | `energetica.destete_forzoso` | `r_aband`, `f_dest_min`, `p_huerf` | A6 (destete), A7 (muertes) |
| 3.4 Dimorfismo | `energetica.dimorfismo` | `masa_kg_hembra/macho`, `f_grasa_max_hembra/macho` | Toda la energética |
| 4. Esterilidad individual | `energetica.esterilidad_individual` | `mu_est_anios`, `sigma_est_anios` | A6 y génesis |
| Métricas nuevas (no cambian el modelo) | `metricas.demografia` | `ancho_fecundidad_anios` (5) | Salida |

- **Opción del incremento 1 sustituida.** `forraje.respuesta` (D16, `"total"`/`"proporcional"`) desaparece: la nota la sustituye por la respuesta funcional, así que la propuesta proporcional ya no existe.
- **Experimento con todo apagado.** `config/experiments/agentes_incremento1.toml` (sustituye a `agentes_proporcional.toml`) tiene todo apagado y reproduce el incremento 1 byte a byte.
- **Campos de salida opcionales.** Los campos nuevos solo se escriben si su interruptor está encendido:
  - por registro: destetes forzosos, crías destetadas y crías muertas, y la condición de las lactantes;
  - por año: muertes por destete forzoso, fecundidad por grupos de 5 años de la madre, edad media del último parto y mortalidad por grupo de edad (crías, jóvenes, adultos, viejos) y sexo;
  - por especie: masa y ayuno de cada sexo, `h_½`, `μ_est` y `σ_est`, y los límites de los grupos.

**Interpretaciones** (para revisar):

| # | Punto | Decisión |
|---|---|---|
| R1 | §1, `G_total` | Gasto propio (mantenimiento y actividad, crecimiento, gestación) + leche pagada, con el estado al empezar el tick (como D2 y D3). En la cría, su propio gasto. |
| R2 | §2, `h_½` | Derivado `(k_ap − 1)/2`, recortado a ≥ 0 (con `k_ap < 1` saldría negativo). `V_½ = h_½·K` de la celda y la especie. La respuesta multiplica la demanda; después actúa el tope del reparto justo. |
| R3 | §3.1, momento | `g_leche` usa la condición de la madre al empezar el tick (en A3, porque su demanda la necesita). La leche se transfiere en A4. La cría no compensa comiendo más. |
| R4 | §3.2 | Se mira en A6, después del destete natural y del "sin crías vivas". Causa de muerte nueva, "destete forzoso", distinta de la orfandad. Las que sobreviven comen solas. La madre pasa a disponible (no concibe ese día: su condición está bajo `r_min`). |
| R5 | §3.4 | Masa, grasa y `c_W` de West por sexo. Masa al nacer `m0 = f_neo·M` = 3,3 kg en humanos (el 0,059·56 = 3,30 kg de la nota: lo mismo). Reserva inicial de cada cría con la grasa de su sexo. Gestación con `B(M_hembra)`. `N_max` con el gasto adulto medio de los dos sexos. |
| R6 | §4 | Normal truncada por rechazo (Box–Muller con el azar por contador `H(semilla, id, "esterilidad")`). Se calcula una vez, al nacer o en la génesis, y se guarda en el estado reproductivo (sortearla cada día costaba un 35 % del tiempo). |

### 8.2 No regresión y pruebas

**No regresión.** Con todos los interruptores apagados (`agentes_incremento1.toml`), los 30 años por defecto dan una salida **idéntica byte a byte** a la del binario de ece0868, compilado en un worktree. Se repite con `herramientas/no_regresion.sh ece0868 agentes config/agentes.toml config/experiments/agentes_incremento1.toml`. Tras la optimización de R6, la salida con todo encendido es idéntica a la de antes.

**Tests unitarios nuevos:**
- `el_dimorfismo_da_las_cifras_de_la_nota`: 1 433 / 2 579 / 126 000 kcal y 48,8 días de ayuno en las mujeres; 1 584 / 2 851 / 86 400 kcal y 30,3 días en los hombres. Sin dimorfismo, las tablas por sexo son las de la especie, bit a bit.
- `la_edad_de_esterilidad_sigue_una_normal_truncada`.

**Tests de simulación nuevos:**
- `con_respuesta_funcional_la_poblacion_se_regula`;
- `con_forraje_de_rebrote_lento_hay_sobrepasamiento_y_colapso`;
- `con_forraje_abundante_se_rehacen_reservas_y_la_lactante_mantiene_su_condicion`;
- `en_equilibrio_el_apetito_por_gasto_total_evita_el_hambre`;
- `en_la_sequia_las_hembras_no_colapsan`;
- `en_la_hambruna_las_mujeres_resisten_mejor`;
- `la_fecundidad_humana_baja_con_la_edad`;
- `sin_los_cambios_de_la_revision_no_hay_regulacion` (el hallazgo A del incremento 1).

**Tests que se mantienen (§6):** balance energético, determinismo, independencia del orden, tabla de vida y ayuno, todos con los cuatro cambios encendidos.

### 8.3 Cada cambio por separado y todos juntos

Hábitat por defecto: 16 celdas de 100 km², templado y clima constante. 100 años; medias de los años 40–100.

**Herbívoro de 70 kg** (`N_max` = 10 158):

| Variante | `N/N_max` | Condición | Lactantes | `V/K` | Muertes por hambre/año | `e0` |
|---|---|---|---|---|---|---|
| Ninguno (incr. 1) | extinto en el año 13 | | | | | |
| Solo 1, solo 3 o solo 4 | extinto en el año 13 | | | | | |
| Solo 2 | 1,19 | 0,57 | 0,49 | 0,60 | 210 | 3,4 |
| **Todos** | **1,23** (cv 1 %) | **0,40** | **0,60** | **0,50** | **0** | **4,2** |
| Todos menos 1 | 1,19 | 0,54 | 0,49 | 0,59 | 1 045 | 2,4 |
| Todos menos 2 | extinto en el año 13 | | | | | |
| Todos menos 3 | 1,23 | 0,40 | 0,60 | 0,50 | 0 | 4,2 |
| Todos menos 4 | 1,23 | 0,40 | 0,60 | 0,50 | 0 | 4,2 |

**Humanos** (`N_max` = 740):

| Variante | `N/N_max` (cv) | Lactantes | Hambre/año | `e0` | Hijos por mujer | Mujeres entre los adultos | Último parto |
|---|---|---|---|---|---|---|---|
| Ninguno | 0,28 (1,14: quedan 8) | 1,00 | 14 | 34,3 | 5,0 | 62 % | 48,6 años |
| Solo 2 | 1,10 (0,02) | 0,67 | 7 | 22,2 | 10,7 | **24 %** | 48,8 |
| Solo 3 | extinto en el año 80 | | | | | 0 % | |
| Solo 4 | 0,49 (0,32) | 1,00 | 10 | 29,1 | 6,3 | 41 % | 36,0 |
| **Todos** | **0,90** (0,08) | **0,90** | **7** | **24,8** | **6,6** | **54 %** | **36,7** |
| Todos menos 1 | 0,84 (0,06) | 0,83 | 8 | 22,3 | 6,8 | 49 % | 35,9 |
| Todos menos 2 | 0,48 (0,33) | 1,00 | 10 | 29,1 | 6,4 | 41 % | 36,1 |
| Todos menos 3 | 0,88 (0,07) | 0,90 | 4 | 26,0 | 6,5 | 47 % | 36,7 |
| Todos menos 4 | 1,25 (0,05) | 0,82 | 27 | 15,2 | 10,7 | 67 % | 49,1 |

**Qué aporta cada cambio:**
- **Cambio 2 (Holling):** es lo que regula.
  - Sin él, el herbívoro se extingue siempre y los humanos se hunden.
  - Con él, el forraje queda en `K/2` (`V/K` = 0,50 con el cambio 1), donde la nota quería el equilibrio.
- **Cambio 1:** en equilibrio, criar sale pagado.
  - El herbívoro deja de morir de hambre (de 1 045 al año a 0), las lactantes pasan de 0,49 a 0,60 de condición y su `e0` de 2,4 a 4,2 años.
  - Con forraje abundante no cambia nada: el apetito del incremento 1 ya paga la leche (0,97 de condición con los dos).
- **Cambio 3:** en clima constante casi no se nota (el herbívoro no tiene dimorfismo). Actúa en las hambrunas (§8.4).
- **Cambio 4:** baja la fecundidad humana de 10,7 a 6,6 hijos por mujer (§8.5).

### 8.4 Sequía (validación de §3)

**Herbívoro.** Rebrote al 50 % el año 21 (el escenario del incremento 1 §10.4), 50 años, mismo hábitat:

| Variante | Población: antes → año de la sequía → siguiente → año 50 | Hembras / machos adultos después | Mortalidad en la sequía (1/año): crías · jóvenes · adultas / adultos · viejos |
|---|---|---|---|
| Solo 2 | 11 985 → 6 099 → 2 923 → 150 | **1 / 2 668** | 6,1 · 3,9 · 3,25 / 0,59 · 3,1 |
| 2 + 1 | 12 034 → 3 410 → 986 → 8 952 | 185 / 427 | 5,5 · 6,3 · 4,61 / 3,80 · 0,7 |
| 2 + 3 | 12 062 → 5 774 → 2 600 → 59 | **3 / 2 360** | 8,5 · 4,1 · 3,15 / 0,62 · 3,1 |
| **Todos** | **12 041 → 3 350 → 905 → 6 613** | **193 / 430** | **8,7 · 7,2 · 4,56 / 3,76 · 0,6** |

**Humanos.** Rebrote al 30 % el año 21, 60 años, solo humanos:

| Variante | Antes → sequía → siguiente → año 60 | Mujeres / hombres adultos después | Mortalidad adulta mujeres / hombres | Niñas / niños | Viejas / viejos |
|---|---|---|---|---|---|
| Solo 2 | 544 → 485 → 216 → 368 | 36 / 68 | **0,62 / 0,36** | 0,49 / 0,73 | 0,58 / 0,35 |
| 2 + 1 | 544 → 490 → 235 → 444 | 53 / 61 | 0,36 / 0,42 | 0,37 / 0,83 | 0,56 / 0,39 |
| 2 + 3 | 544 → 485 → 223 → 389 | 54 / 61 | 0,46 / 0,34 | 0,71 / 1,10 | 0,37 / 0,31 |
| Todos menos 3 | 441 → 432 → 342 → 520 | 94 / 109 | 0,13 / 0,09 | 0,31 / 0,70 | 0,18 / 0,12 |
| **Todos** | **441 → 433 → 321 → 425** | **103 / 75** | **0,08 / 0,30** | **0,24 / 0,79** | **0,16 / 0,32** |

**Lo que cumple:**
- **Humanos.**
  - Con todo encendido, la mortalidad adulta de las mujeres es menor que la de los hombres (0,08 frente a 0,30).
  - En las mujeres, mueren más las niñas que las viejas, y más las viejas que las adultas.
  - La población vuelve al 96 % en 40 años.
  - Lo decide el dimorfismo (cambio 3): sin él, ellas mueren más.
- **Herbívoro.**
  - Ya no colapsan las hembras: son el 31 % de los adultos tras la sequía (0,04 % solo con Holling).
  - Mueren más las crías que los adultos, y la población se recupera (×7 en 30 años).
  - Lo que lo consigue es el **cambio 1**, no el 3: con el apetito por gasto total, las madres llegan a la sequía con las reservas pagadas.

**Lo que no cumple:**
- **El orden en el herbívoro es crías > jóvenes > adultos > viejos.** Los viejos (más de 12 años) son los que menos mueren, y la nota esperaba crías > viejos > adultos.
- **Las hembras del herbívoro siguen muriendo más que los machos** (4,6 frente a 3,8 al año).
  - La causa es la gestación, no la lactancia: pasan gestantes 200 de cada 336 días, y el aborto solo llega con la condición en `r_abort` = 0,1.
  - Subir `r_aband` de 0,1 a 0,25 no cambia nada (hembras adultas tras la sequía: 30 %, 30 %, 29 %), aunque los destetes forzosos pasan de 5 a 71.
- **La recuperación es lenta.** Con 4 celdas de 50 km², la población cae al 6–15 % y tarda ~25 años en volver al 80 %.
- **El hambre llega a la vez a todos** (§8.8, hallazgo 5): en hábitats pequeños, o con la sequía durante el sobrepasamiento inicial, se extinguen celdas enteras.

### 8.5 Fecundidad humana (validación de §4)

Fecundidad por grupos de edad de la madre, relativa a la de 20–24 años; años 40–100:

| | 20–24 | 25–29 | 30–34 | 35–39 | 40–44 | 45–49 | Hijos por mujer | Último parto |
|---|---|---|---|---|---|---|---|---|
| Coale-Trussell (referencia de la nota) | 1,00 | 0,94 | 0,86 | 0,70 | 0,36 | 0,05 | ~6 | 37–40 años |
| Sin esterilidad (todos menos 4) | 1,00 | 0,98 | 0,99 | 0,98 | 0,99 | 0,96 | 10,7 | 49,1 |
| **Con esterilidad (todos)** | 1,00 | 0,87 | 0,83 | 0,58 | 0,18 | 0,01 | **6,6** | **36,7** |

Barrido de `μ_est` (solo humanos, todo encendido, 150 años; años 50–150):

| `μ_est` | Hijos por mujer | Último parto | Relativa 35–39 / 40–44 / 45–49 | `e0` |
|---|---|---|---|---|
| 36 | 6,06 | 34,7 | 0,40 / 0,09 / 0,01 | 22,9 |
| **38 (la nota)** | **6,68** | **36,6** | 0,55 / 0,19 / 0,03 | 20,6 |
| 40 | 7,34 | 38,9 | **0,76 / 0,34 / 0,06** | 19,5 |
| 42 | 7,97 | 40,6 | 0,84 / 0,46 / 0,14 | 17,3 |

**Ningún valor cumple a la vez los tres objetivos.**
- Con 38: ~6 hijos (6,7), pero el último parto llega un poco pronto (36,6, el objetivo es 37–40) y la fecundidad cae antes que en la curva natural.
- Con 40: la forma de Coale-Trussell y el último parto (38,9), pero 7,3 hijos.

Hay que bajar algo la fecundidad de las edades centrales: la fecundabilidad con la edad (pregunta Q1 de la nota) o la espera entre partos. Se mantiene 38 (decisión A4).

### 8.6 Resto de validaciones de §2

**Densidad frente a la productividad** (todo encendido, años 15–40; entre paréntesis, la propuesta proporcional del incremento 1):

| Bioma | Herbívoro (/km²) | Humanos (/km²) |
|---|---|---|
| Desierto | 1,36 = 1,24·`N_max` (1,32) | 0,048 = 0,60·`N_max` (0,063) |
| Tundra | 1,59 = 1,23·`N_max` (1,56) | 0,061 = 0,65·`N_max` (0,085) |
| Templado | 7,80 = 1,23·`N_max` (7,79) | 0,309 = 0,67·`N_max` (0,42) |
| Selva | 16,97 = 1,23·`N_max` (17,0) | 0,646 = 0,64·`N_max` (0,89) |

**Equivalencia energética** (10 y 300 kg juntas, años 30–60):
- consumen 4 567 y 4 567 kg/km²/año (cociente 1,000);
- sus densidades, 33,5 y 2,61/km², guardan un cociente de 12,8 = 30^¾ (ley de Damuth).

**Sobrepasamiento** (forraje de rebrote lento, τ_c = 20 años):
- con Holling: pico de 2,47·`N_max` en el año 15 y extinción hacia el año 30;
- sin Holling: pico de 3,49 en el año 18 y extinción.

El modelo produce los dos regímenes (regulación con τ_c = 0,5; sobrepasamiento y colapso con τ_c = 20), como pedía la nota. Las celdas están aisladas, así que el colapso acaba en extinción.

**Criterio "`N_eq/N_max` entre 0,7 y 1,2"**:
- humanos: 0,90, cumple;
- herbívoro: 1,23, se pasa un poco. `N_max` se calcula con el gasto del adulto y las crías gastan menos (decisión A1).

### 8.7 Modo población (pendiente del incremento 2, sin adelantar)

| Cambio | Lo que hará falta |
|---|---|
| 1 | Misma regla por clase: `Ap = k_ap · Ḡ_total,k`, con la `ē_k` de la clase (§1). |
| 2 | `V_c` = densidad media de la celda de nivel 0: stock total de sus descendientes entre su área (§2). |
| 3.1 | La leche, con la condición media de las lactantes, `g_leche(ē_F/E_max)` (§3.3). |
| 3.2 | Madres que abandonan: `Binomial(madres(Lq[j]), F_Γ(r_aband·E_max; 1/cv_E², ē_F·cv_E²))`. Sus crías pasan a destetadas o mueren con `Binomial(·, p_huerf)` según la edad del bin (§3.3). |
| 3.4 | Clases separadas por sexo (ya lo están: `N[s][k]`) con la masa y la grasa de su sexo. |
| 4 | La fracción fértil `fert_k` con `Φ` (§4), sin guardar `a_est`. Al desagregar, cada hembra sortea su `a_est`. |

La prueba de coherencia entre modos debe incluir la sequía (§6), y las sumas deberán ser exactas como en individuos.

### 8.8 Hallazgos

1. **La respuesta funcional regula, y el apetito por gasto total la completa.** Juntas ponen el equilibrio en `K/2` sin hambre (herbívoro) y pagan la cría.
2. **El cambio 1 solo se nota con comida escasa.** Con forraje abundante, el apetito del incremento 1 ya cubría la leche. Con la respuesta funcional, en equilibrio cada uno come la mitad de lo que querría, y ahí sí importa.
3. **Las hambrunas ya no acaban con las hembras,** pero en el herbívoro siguen muriendo más que los machos. La causa es la gestación, y el destete forzoso no la toca (§8.4).
4. **Las mujeres resisten mejor las hambrunas,** como en la realidad, gracias al dimorfismo: 49 días de ayuno frente a 30.
5. **El hambre sincronizada.**
   - Con la respuesta funcional, lo que come cada uno depende solo del forraje que queda, no de cuántos comen.
   - En una sequía, todos los de una celda entran en déficit a la vez y mueren casi a la vez, antes de que el forraje se recupere.
   - Las celdas son poblaciones cerradas, así que muchas se extinguen enteras (con 25 km² y la sequía en pleno sobrepasamiento, todas).
   - No es un fallo del código, sino una consecuencia de la regla. Con movimiento entre celdas (otro documento) habrá recolonización.
6. **La esperanza de vida humana baja a 21–25 años en equilibrio.** El 29 % de las muertes son por hambre: la población se regula así. La calibración de Siler (30,9 años) supone comida suficiente (P4 del diseño).
7. **Ningún `μ_est` cumple los tres objetivos de fecundidad** (§8.5).
8. **Coste:**
   - 30 años con todo encendido tardan 15 s (con el herbívoro vivo, ~12 000 individuos), frente a 16 s solo con Holling y ~4 s del incremento 1, en el que el herbívoro se extinguía en el año 13;
   - los tests de biología tardan 45 s en debug (antes 12 s).

### 8.9 Decisiones para el responsable

| # | Decisión | Opción aplicada | Datos |
|---|---|---|---|
| A1 | `N/N_max` del herbívoro es 1,23 (la nota pide ≤ 1,2). ¿Se acepta, o se calcula `N_max` con el gasto medio de la población? | Se acepta (D6 sin cambios) | §8.3, §8.6 |
| A2 | Las hembras del herbívoro mueren más en las hambrunas por la gestación. ¿Subir `r_abort`, bajar `k_gest` o aceptarlo? | Sin cambios | §8.4 |
| A3 | En el herbívoro los viejos son los que menos mueren en una sequía. ¿Es un objetivo para la especie genérica? | Sin cambios | §8.4 |
| A4 | Fecundidad humana: `μ_est` 38 (≈ 6 hijos) o 40 (forma y edad del último parto), y ¿añadir la fecundabilidad por edad (Q1)? | 38, el valor inicial de la nota | §8.5 |
| A5 | Hambre sincronizada: ¿algo de heterogeneidad en el acceso al forraje, o esperar al movimiento? | Sin cambios (no está en la nota) | §8.8.5 |
| A6 | `e0` humana de 21–25 años en equilibrio | Sin cambios (P4) | §8.8.6 |
| A7 | La opción `"proporcional"` (D16) se retira, sustituida por Holling | Sustituida | §8.1 |
