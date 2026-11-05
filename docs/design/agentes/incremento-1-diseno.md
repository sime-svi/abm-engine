# Agentes — Incremento 1: el individuo en un hábitat de prueba

**Estado:** diseño técnico de Claude Code sobre el documento del responsable (`diseno_biologia_agentes.md`, v1.0). Las decisiones de interpretación (§3) están marcadas para su revisión. **Revisión 1** (`nota_agentes_revision_1.md`, implementada el 2026-11-05, §8 de la nota): confirma D1, sustituye D16 por la respuesta funcional de Holling y añade la leche según la condición de la madre, el destete forzoso, el dimorfismo sexual y la edad de esterilidad individual.
**Fecha:** 2026-11-05.

---

## 1. Objetivo y alcance

El documento del responsable describe la biología completa: individuos y poblaciones, Foco, forraje sobre el planeta, eventos y métricas. Siguiendo la pirámide invertida (CLAUDE §4), se construye por capas. Este incremento implementa la **capa de base: el individuo**, con su energía, crecimiento, reproducción sexual y mortalidad, sobre un **hábitat de prueba** sin planeta. Así la biología se valida aislada antes de tocar el terreno.

**Entra** (secciones del documento madre):
- §5 / §6.0: catálogo de especies por masa (alometría) con el preset humano, y la población inicial.
- §6.1, solo la parte del stock: forraje logístico por celda y especie, con la PPN del modelo Miami a partir de un clima **constante** por zona.
- §6.4: demanda y reparto justo. §6.5: energética. §6.6: crecimiento (West). §6.7: reproducción sexual. §6.8: mortalidad (Siler, inanición, edad máxima, orfandad). Envejecimiento. §6.10: escritura del consumo.
- §6.11: azar por contador e identificadores deterministas.
- §8: métricas, incluido el **balance energético global**. §7: eventos, como recuentos por tick.
- §10: las comprobaciones que no necesitan el planeta.

**Queda para después:**

| Incremento | Contenido |
|---|---|
| 2 | Modo población (cohortes, §6.9) y conversión individuo ↔ población (§6.3), con la prueba de coherencia entre modos (≥ 10 semillas, 5 %, incluida la sequía). Incluye la versión de poblaciones de los cambios de la revisión 1 (nota §8.7). |
| 3 | Acople con el planeta: forraje en las celdas del terreno con medias móviles y factor estacional (§6.1 completo), LOD del forraje, Foco (§6.2) como disparador del LOD del terreno y génesis de agentes en el planeta. |
| 4 | Registro de eventos individuales (§7) y volcado a la base de datos (tablas `events` y `agent_snapshots`). |

## 2. Dónde vive el código

- `abm-ruleset/src/biologia/`: el modelo. Configuración, especies, azar, componentes, hábitat, génesis, sistemas y orden del tick. El modelo de forrajeo antiguo sigue intacto hasta que este lo sustituya.
- `abm-metrics/src/biologia.rs`: el observador. Recuentos por tick, balance y tablas de vida anuales.
- `abm-persistence/src/schema_bio.rs`: el formato de salida.
- `abm-runner`: `run_biologia` y el binario `agentes`, con `config/agentes.toml`.

## 3. Decisiones de interpretación

| # | Punto del documento | Decisión | Motivo |
|---|---|---|---|
| **D1** | §6.4 `d = min(Ap, (E_max − E + G)/ε)` | **El apetito `Ap = k_ap·B·PAL` se mide en energía asimilable:** `d = min(Ap, E_max − E + G) / ε`. | Leído tal cual, `Ap` sería forraje bruto. Con `ε = 0,5` y `k_ap = 2`, el genérico asimilaría como mucho `B·PAL`: no podría pagar crecimiento, gestación ni lactancia, ni rehacer reservas, y se extinguiría. La regla de estabilidad del propio documento (`k_ap > 1` ⇒ se rehacen reservas, §1 y §10) solo se cumple con esta lectura. **Confirmada en la revisión 1 §1**, que además refiere el apetito al gasto total del día: `Ap = k_ap·G_total` (`energetica.apetito_gasto_total`). |
| D2 | §6.5/§6.6, el gasto de crecimiento depende de `Δm` | `Δm` se decide al principio del tick con la condición de partida (`E/E_max ≥ r_crec_min`). Su coste entra en el gasto del día y la masa crece en A5. | Evita la circularidad entre gasto, ingesta y crecimiento. |
| D3 | §6.5, leche | La madre paga `Σ φ(a_c)·G_c / ε_leche`, con `G_c` el gasto propio de cada cría (mantenimiento + crecimiento). | `G_c` de la cría no incluye leche. |
| D4 | §6.5/§6.8, inanición | Si el déficit supera las reservas, `E` queda negativa ese día y el individuo muere. Su reserva (negativa) entra en "reservas de los muertos". | Cierra el balance exactamente sin repartir el déficit. |
| D5 | §6.5, tope `E ≤ E_max` | Lo que se recortara se anota como **excedente**. Con D1 es cero salvo redondeo. | El balance sigue cerrado aunque el tope actúe. |
| D6 | §6.0, `N_max` | `G_medio` = gasto del adulto, `B(M)·PAL`. | Es el usado en la tabla de calibración del §10 (3,9 ciervos/km²). **Sustituida en la revisión 1 (A1):** `G_medio` es el gasto medio de la población estable, crías incluidas, y `f_com` del herbívoro pasa a 0,00324 para que la densidad de equilibrio siga la ley de Damuth (nota de revisión §9.2). |
| D7 | §6.0, población inicial | Edades ∝ `l(a)` de Siler (sin inanición). Masa por la curva de crecimiento, `E = reserva_inicial·E_max` (0,5 por defecto) y colas vacías. Las crías iniciales no tienen madre: comen solas (`φ = 0`). | Lo que dice §6.0. Lo de las crías se sigue de "colas vacías". |
| D8 | §6.7, `H_macho` | Basta un macho maduro de la especie en la misma celda del hábitat. | En este incremento no hay celdas más finas. |
| D9 | §6.7, fin de la vida reproductiva | Una hembra gestante o lactante que alcanza `a_fin_rep` termina el ciclo y después pasa a postreproductiva. | Lo pide el documento ("termina antes el ciclo"). |
| D10 | §6.7, destete | Una cría deja de depender al cumplir `d_destete` (`φ = 0`). Su madre pasa a disponible cuando se le acaba la cuenta (destete) o cuando ya no le queda ninguna cría viva. | Las dos cuentas empiezan el día del parto y coinciden. |
| D11 | §5.4, Siler del genérico | Las tasas se expresan **relativas a `a_max`** (`α·a_max`, `β·a_max`), así escalan con la masa. Se calibran en la especie de referencia (§4). | `β3 = ln2/(c_dup·a_max)` ya sigue esta forma; el documento quiere la especie definida por su masa. |
| D12 | §6.11, azar | `H` = cadena de mezcladores SplitMix64 sobre (semilla, id, tick, propósito, índice). Uniforme de 53 bits. | Función pura, sin estado. |
| D13 | §10, "barajar el orden de proceso" | Todas las sumas que vuelven al modelo o a la salida (demanda por celda, consumo, libro de energía, métricas) son **sumas exactas en punto fijo** (`i128`, 2⁻³² kcal): no dependen del orden. | Se prueba barajando el orden de creación de la población inicial: la salida es idéntica byte a byte. |
| D14 | §7, eventos | Recuentos por tick y por especie. El registro individual va al incremento 4. | Acotar el tamaño del registro. |
| D15 | §6.1, factor estacional | `s(t) = 1` (clima constante), salvo una **sequía** opcional: `s(t) = factor` durante una ventana de días. | El factor real necesita `T` y `θ` del terreno (incremento 3). La sequía permite probar las hambrunas (§10). |
| **D16** | §6.4, oferta `F = V·A·ρ_veg` | Por defecto, el diseño tal cual (`forraje.respuesta = "total"`). Como **propuesta**, `"proporcional"`: la demanda se multiplica por `V/K`. | Con todo el stock accesible, la población no nota la escasez hasta agotarlo y colapsa (§10). Con la propuesta se regula sola cerca de `N_max`. **Decidida en la revisión 1 §2:** respuesta funcional de Holling tipo II, `d·V/(V + h_½·K)`, encendida por defecto (`forraje.respuesta_funcional`). La opción `respuesta` y la variante proporcional se retiran. |

## 4. Calibración de la mortalidad (Siler)

Probabilidad diaria `p = 1 − exp(−μ(a)/dias_anio)`, con `a` en años al empezar el día. Tabla de vida calculada con esa misma regla diaria:

**Humano** (preset): parámetros medios de cazadores-recolectores de Gurven y Kaplan (2007): `α1 = 0,422`, `β1 = 1,131`, `α2 = 0,013`, `α3 = 1,47·10⁻⁴` (1/año), con el `β3 = 0,085` del preset.

| Objetivo (§10) | Modelo |
|---|---|
| Supervivencia a los 15 años ≈ 55–60 % | 56,4 % |
| Edad modal de muerte adulta ≈ 70 años | 69 años |
| Esperanza de vida al nacer ≈ 30–35 años | 30,9 años |

**Genérico de 70 kg** (`a_max = 25` años, `β3 = ln2/(0,08·25) = 0,347`/año). Tres reglas y un ajuste:
- `α2 = −ln(0,9) = 0,1054`/año: mortalidad adulta de fondo del 10 %.
- `α3`: la senescencia iguala al fondo a `0,48·a_max`, como en el humano (52 de 110 años). Da `α3 = 1,646·10⁻³`.
- `β1·d_destete ≈ 2,83`, como en el humano: la mortalidad juvenil decae al ritmo del destete. Da `β1 = 8,61`.
- `α1` se ajusta para que `l(1 año) = 0,6`: `α1 = 3,433`.

En relativos a `a_max` (lo que va a la configuración): `A1 = 85,83`, `B1 = 215,2`, `A2 = 2,634`, `A3 = 0,04116`.

| Objetivo (§10) | Modelo |
|---|---|
| Supervivencia del primer año ≈ 0,6 | 0,600 |
| Mortalidad adulta anual ≈ 0,1 | 0,104–0,120 (de 2 a 8 años) |
| (informativo) esperanza de vida al nacer | 4,8 años |

## 5. Hábitat de prueba

- Zonas con nombre, nº de celdas, área por celda y clima constante (`t_media`, `p_media_mm`). `PPN = min(f_T, f_P)` (Miami, §6.1).
- Por celda y especie: `K_c = τ_c·f_com·PPN`, `r_c = 4/(τ_c·dias_anio)` y `g_0 = g0_frac·f_com·PPN/dias_anio`. El stock empieza lleno (`V_c = K_c`).
- Las celdas no tienen vecindad: no hay movimiento (el comportamiento es otro documento). Cada celda es una población cerrada.
- Para los ectotermos, la temperatura del metabolismo es la `t_media` de la celda.

## 6. Orden del tick

```
T4  forraje: rebrote logístico (· s(t))
A1  (conversión individuo ↔ población: incremento 2)
A2  (comportamiento: documento aparte)
A3  demanda: Δm previsto, gasto propio, leche de las crías, demanda y reparto justo por celda
A4  energética: ingesta, leche recibida, balance de reservas, libro de energía
A5  crecimiento: m ← min(m + Δm, M)
A6  reproducción: madurez, aborto, parto, destete, concepción; las crías nacen al final del paso
A7  mortalidad: inanición, edad máxima, Siler, orfandad; se retiran los muertos
A8  envejecimiento: edad + 1; las crías que cumplen d_destete dejan de depender
A9  escrituras: el consumo se descuenta del stock
M   métricas
```

## 7. Conservación y determinismo

- **Libro de energía por tick:** asimilada, leche recibida, gasto propio, leche pagada, pérdida al almacenar, excedente, reservas de los muertos, y las reservas al principio y al final. El residuo `ΔΣE − (asimilada + recibida − propio − pagada − pérdida − excedente − muertos)` debe ser cero a redondeo. La pérdida en la conversión a leche es `pagada − recibida`. El balance del §8 se obtiene agrupando términos.
- **Azar** por contador (D12). **Identificadores:**
  - población inicial: `H(semilla, celda·2¹⁶ + especie, 0, ID_INICIAL, i)`;
  - cría: `H(semilla, id_madre, nº de parto, ID_CRÍA, índice)`.
  - Las colisiones se cuentan en las métricas (deben ser 0).
- **Sumas exactas** (D13).

## 8. Métricas

- **Por registro** (cada `metrics_every` ticks, con los recuentos acumulados desde el anterior) y por especie:
  - población: total, hembras, machos, crías dependientes, gestantes y lactantes;
  - eventos: nacimientos, muertes por causa, concepciones, abortos y destetes;
  - estado: condición media `E/E_max`, fracción en déficit y reservas totales;
  - forraje: stock, stock máximo, consumo y rebrote.
  - Además, el balance de energía del intervalo y las colisiones de id.
- **Por año** y por especie:
  - población media y densidad frente a `N_max`;
  - nacimientos, muertes por causa y tasas brutas (‰);
  - esperanza de vida al nacer (tabla de vida del periodo, por clases de `Δa`), tasa global de fecundidad e intervalo medio entre partos;
  - pirámide por clase y sexo al final del año, y presión de herbivoría (consumo / rebrote).

## 9. Pruebas

**Unitarias** (`abm-ruleset`):
- tabla de calibración del §10 (Kleiber, gasto, ayuno, gestación, Damuth, Miami);
- tabla de vida de Siler (§4);
- curva de crecimiento (solución exacta frente a la integración diaria);
- azar por contador;
- sumas exactas independientes del orden.

**De simulación** (`abm-runner/tests/biologia.rs`):
- balance energético cerrado;
- sin forraje, extinción en ~`E_max/G` días;
- sin machos, sin nacimientos;
- con `k_ap ≤ 1` no se rehacen reservas;
- determinismo;
- independencia del orden de proceso;
- tabla de vida simulada de una cohorte frente a la analítica.

**Medidas** (informe, no test): densidad de equilibrio frente a la productividad, hambruna por sequía y equivalencia energética entre especies.

## 10. Resultados y hallazgos (2026-11-05)

### 10.1 Verificación

- **Tests:** 6 unitarios en `abm-ruleset`, 1 en `abm-metrics` y 8 de simulación, todos en verde. Los de simulación tardan 12 s en debug.
- **Balance de energía:** residuo máximo de 2·10⁻⁶ kcal por registro de 30 días, frente a ~10¹⁰ kcal de flujos.
- **Determinismo:** salida idéntica byte a byte entre procesos y entre debug y release. También con la población inicial creada en otro orden.
- **Tabla de vida simulada** (cohorte de 2 000 recién nacidos del genérico): `l(1)` y `e0` dentro del error de muestreo de la analítica.

### 10.2 Hallazgo A: con el reparto del diseño, la población no se regula

Corrida por defecto: 16 celdas de 100 km², templado (PPN 1 115 g/m²/año), 30 años.
- **Herbívoro de 70 kg** (`N_max` = 10 158):
  - crece bien alimentado (condición 0,95) de 5 072 a 10 782 en 7 años, por encima de `N_max`, mientras el stock baja despacio (consumo / rebrote ≈ 1,02–1,09);
  - el stock se agota y llega la hambruna: 1 688, 6 577 y 3 261 muertes por inanición en los años 8 a 10;
  - **extinción en el año 13**.
- **Humanos** (`N_max` = 740): crecen despacio. Hay hambrunas en los años 27, 35, 50 y 60, y la población va a menos (306 en el año 60).

**Causa estructural.** Con `F = V·A·ρ_veg`, cada uno come lo que pide hasta que el stock no da ni para un día. Nadie nota la escasez a tiempo: la fecundidad (`f_E`) solo baja cuando baja la condición, y la condición solo baja cuando el forraje ya se ha acabado. Con el stock por debajo de `K/2`, el rebrote logístico cae y el hundimiento se acelera. En celdas aisladas (sin migración ni recolonización) el final es la extinción.

### 10.3 Propuesta D16: consumo proporcional al stock

Con `respuesta = "proporcional"` (la demanda × `V/K`), al bajar el stock se come menos. Como el apetito permite comer hasta `k_ap = 2` veces el gasto, el equilibrio cae en `V ≈ K/2`, justo donde el rebrote es máximo.

| Medida (años 20–40) | Herbívoro 70 kg | Humano |
|---|---|---|
| Desierto (25 °C, 100 mm; PPN 193) | 1,32/km² = 1,21·`N_max` | 0,063/km² = 0,78·`N_max` |
| Tundra (−10 °C, 300 mm; PPN 227) | 1,56/km² = 1,21·`N_max` | 0,085/km² = 0,90·`N_max` |
| Templado (10 °C, 700 mm; PPN 1 115) | 7,79/km² = 1,23·`N_max` | 0,42/km² = 0,91·`N_max` |
| Selva (25 °C, 2 500 mm; PPN 2 430) | 17,0/km² = 1,23·`N_max` | 0,89/km² = 0,89·`N_max` |

- **Estabilidad:** el herbívoro se mantiene en 12 450 ± 0,5 % del año 10 al 60, con condición ~0,57 y ~200 muertes por hambre al año (mortalidad dependiente de la densidad). Su `e0` baja de 4,8 a 3,5 años.
- **Por qué supera `N_max`:** `N_max` se calcula con el gasto del adulto, y las crías gastan menos.
- **Equivalencia energética** (especies de 10 y 300 kg juntas, años 30–60): consumen 4 478 y 4 460 kg/km²/año (cociente 1,004). Sus densidades, 33,6 y 2,6/km², guardan un cociente de 12,9 ≈ 30^¾ (Damuth).
- **Test:** `con_respuesta_proporcional_la_poblacion_se_regula`.

### 10.4 Hallazgo B: las hambrunas matan sobre todo a las hembras

Sequía con la propuesta D16 (rebrote al 50 % durante el año 21):
- **Herbívoro:** 8 682 muertes por hambre y la población pasa de 12 408 a 2 739. Mueren primero **todas las crías** (la clase de edad 0 queda vacía), como pide el §10.
- **Pero también casi todas las hembras:** de 5 452 quedan 2, frente a 3 159 machos. Sin hembras no hay recuperación: 37 animales en el año 35.
- **Ya en equilibrio** la población tiene un 56 % de machos.
- **Causa:** las hembras pagan gestación y lactancia. El diseño hace abortar a la gestante en mala condición (`r_abort`), pero no tiene equivalente en la lactancia: la madre sigue pagando leche hasta morir.
- **Propuesta:** un destete forzoso cuando `E/E_max < r_abort`, a decidir por el responsable.
- **Con rebrote al 30 %**, el herbívoro se extingue. Los humanos caen de 532 a 232 y se recuperan despacio (371 en el año 45).

### 10.5 Otras observaciones

- **Fecundidad humana alta:** la tasa global de fecundidad sale de ~10 hijos por mujer (intervalo entre partos ~3 años de los 19 a los 50), frente a 5–6 en cazadores-recolectores. Falta que la fecundabilidad baje con la edad antes de la menopausia. Se suma a la pregunta abierta P1 de la calibración humana.
- **Rendimiento:** 30 años con ~10 000 individuos, ~4 s en release.

