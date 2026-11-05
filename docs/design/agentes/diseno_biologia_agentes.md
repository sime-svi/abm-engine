# Documento de Diseño: Biología de los Agentes

**Sistema:** Energía, crecimiento, reproducción, mortalidad y representación individuo-población de los agentes
**Versión:** 1.0
**Destinatario:** Claude Code para implementación
**Naturaleza:** Plano teórico. Fenómeno real, lógica de estados y ecuaciones. Sin código.
**Documentos relacionados:** Diseño del terreno (visión madre) y Nota de lluvia orográfica.

**Decisiones cerradas por el director:**
- 1 tick = 1 día.
- Especie genérica definida por su masa corporal (leyes de escala). Los humanos son un preset con valores directos.
- Representación mixta: individuos donde hay foco (zoom) y poblaciones agregadas en el resto, con las dos reglas de conservación de la malla.
- Energía en kilocalorías reales.
- Reproducción sexual (hembras y machos, gestación, lactancia, destete).
- Foco = regiones fijadas en la configuración + foco automático por actividad, con tope de coste.

**Notación:** `·` multiplicación, `^` exponente, `_` subíndice. `dias_anio = 365`, y debe coincidir con `T_anio` del terreno. Las tasas "por año" se pasan a "por día" dividiendo por `dias_anio`. "Extensivo" = se suma al juntar (nº de individuos, energía total, biomasa); "intensivo" = se promedia.

---

## 1. Fenómeno real y objetivo

### Qué se modela
Organismos que obtienen energía de la vegetación del terreno, la gastan en mantenerse vivos (metabolismo), guardan el sobrante como reservas de grasa, crecen desde cría hasta adulto, se reproducen sexualmente (la hembra paga gestación y lactancia), envejecen y mueren, ya sea de hambre, por causas intrínsecas o por edad.

### Qué debe emerger (no se impone)
- **Densidades que siguen a la productividad del terreno.** Más individuos en sabanas y bosques productivos que en desiertos y tundras. Y, entre especies, la relación real entre tamaño corporal y densidad (ley de Damuth): las especies grandes son escasas y las pequeñas, abundantes.
- **Capacidad de carga** por bioma, sin fijarla a mano.
- **Pirámides de población** con forma realista: mortalidad infantil, estructura por edades y sexos.
- **Hambrunas con las sequías.** Cae la producción vegetal, se agotan las reservas, sube la mortalidad (sobre todo de crías y viejos), cae la fertilidad, y después llega la recuperación.
- **Fertilidad dependiente de la energía:** las hembras en mala condición conciben menos y abortan más.
- **Extinciones locales y recolonización.**

### Relación con el terreno
El terreno produce el **forraje** (comida) a partir de su clima y su suelo. Los agentes lo consumen, y ese consumo se escribe de vuelta en el terreno. Es la conexión que cierra el bucle.

### Traducción de los parámetros actuales (config/default.toml)
| Parámetro actual | Valor | Lo sustituye |
|---|---|---|
| Vida máxima | 200 ticks | `a_max` por especie (años → días) + mortalidad por edad (Siler) |
| Metabolismo | 2 | Gasto diario `G = B(m)·PAL` (Kleiber, kcal/día) |
| Apetito | 4 | `Ap = k_ap · G` (ingesta máxima diaria) |
| Rebrote 1, máx. 8 | por casilla | Forraje logístico por unidad de área: el "8" pasa a ser `K_c` (stock máximo) y el "1" la regeneración máxima, derivados de la productividad real |
| Energía para reproducirse | 20 | Umbrales de condición corporal `r_min`, `r_opt` sobre `E/E_max` |
| Energía cedida al hijo | 50 % | Coste de gestación + reserva inicial de la cría + lactancia |
| Energía inicial | 5-25 | `E0 = r_0 · E_max(m0)` para las crías; población inicial en la génesis (§6.0) |

**La regla de estabilidad de julio (rebrote < metabolismo < apetito) se generaliza** en dos condiciones:
1. `k_ap > 1`: el apetito supera al gasto, así que un agente puede rehacer reservas.
2. Comida accesible finita, que acota la población. La población máxima sostenible de una celda es:
```
N_max_sostenible ≈ ( f_com · PPN_anual · A_celda · ρ_veg · ε ) / ( dias_anio · G_medio )
```
En el mundo antiguo, "rebrote < metabolismo" significaba que una casilla alimentaba a menos de un agente. En el planeta, una celda alimenta a miles, y lo que limita es esta capacidad de carga.

---

## 2. Alcance y LOD

### Tiempo
Tick = 1 día. Todo proceso biológico se expresa por día.

### Especie: genérica por masa, con anclas
Cada especie se define por su masa adulta `M` (kg) y su tipo térmico (endotermo = sangre caliente, ectotermo = sangre fría). Sus parámetros salen de **leyes de escala alométricas** (cómo cambia una propiedad biológica con el tamaño) ancladas en una especie de referencia:
```
Q(M) = Q_ref · (M / M_ref)^b_Q
```
- `Q_ref` es el valor de la propiedad en la especie de referencia (`M_ref` = 70 kg, un mamífero herbívoro tipo ciervo) y `b_Q` es su exponente de escala.
- Un **preset** (como "humano") puede sobrescribir cualquier valor derivado.

### Dieta
Consumidor primario: come vegetación del terreno (forraje). Cada especie accede a su propio stock de forraje (sin competencia entre especies en esta versión).

### Dos representaciones (LOD de agentes)
- **Individuo:** dentro del **Foco**. Cada organismo es una entidad con edad, sexo, masa, reservas y estado reproductivo.
- **Población (cohortes):** fuera del Foco. Una entidad por (celda gruesa de nivel 0, especie), con recuentos enteros por sexo y clase de edad y energía total por clase.
- **Regla de exclusividad:** en una celda y para una especie existe **una sola representación** a la vez.

### Foco (zoom)
Unión de regiones fijadas en la configuración y foco automático por actividad, con un presupuesto máximo de individuos. **Sustituye** el disparador del terreno "hay agentes en la celda" por "la celda está en Foco". Si no, con poblaciones en todo el planeta, la malla se afinaría en todas partes.

### Fuera de alcance (otros documentos)
- Comportamiento: movimiento, búsqueda de comida y pareja, migración. Aquí se deja un hueco en el orden de ejecución.
- Vida social, cultura y agricultura humana.

### Ampliación futura (marcado, no implementar ahora)
Depredación y cadena trófica; competencia entre especies por el mismo forraje; enfermedades; genética y herencia de rasgos; estacionalidad reproductiva (celo ligado al clima); coste térmico de los endotermos (incluido como opción, apagado por defecto); dependencia prolongada de los niños humanos y reparto de comida; individuos persistentes (con nombre) que sobrevivan al paso a modo población; carroña.

---

## 3. Entidades y componentes (ECS)

### Catálogo `Especie` (datos de configuración, no entidad dinámica)
Masa adulta `M`, tipo térmico, preset, y todos los parámetros de la Sección 5. Los valores derivados (§6.0) se calculan una vez al cargar.

### Entidad `Individuo` (solo dentro del Foco)
| Componente | Contenido |
|---|---|
| `Id` | identificador determinista (§6.11) |
| `EspecieRef` | especie |
| `Sexo` | {HEMBRA, MACHO} |
| `Edad` | `a` en días |
| `Masa` | `m` en kg |
| `Reserva` | `E` en kcal (extensivo) |
| `EstadoRepro` | máquina de estados (§6.7) + contador de días restantes |
| `Parentesco` | `id_madre` (si es dependiente) y lista de `crias_dependientes` (si es madre lactante) |
| `Posicion` | celda (la más fina existente en su ubicación) |

### Entidad `Poblacion` (fuera del Foco; una por celda de nivel 0 y especie)
| Componente | Contenido |
|---|---|
| `Id` | identificador determinista |
| `Cohortes` | `N[s][k]` (entero, nº de individuos por sexo `s` y clase de edad `k`) y `E_tot[s][k]` (kcal, extensivo) |
| `ColaGestacion` | `Gq[j]`: nº de hembras gestantes con `j` bins de días restantes hasta el parto |
| `ColaLactancia` | `Lq[j] = (madres, crías)`: nº de madres lactantes y de crías dependientes vivas, con `j` bins de días restantes hasta el destete |
| `Estadisticas` | contadores de la ventana para el foco automático (§6.2) |

### Componentes nuevos en las celdas del terreno
| Componente | Contenido |
|---|---|
| `Forraje` | `V_c[esp]`: stock de forraje accesible para cada especie (g de materia seca/m²) |
| `ClimaMedio` | `T̄` (°C) y `P̄` (mm/año): medias móviles anuales, y `S̄` (media móvil del factor de crecimiento estacional) |
| `Foco` | booleano (solo en celdas de nivel 0; las hijas lo heredan) |

---

## 4. Variables de estado

| Símbolo | Significado | Tipo | Rango / Unidades |
|---|---|---|---|
| `a` | edad del individuo | entero | días; [0, a_max] |
| `m` | masa actual | real | kg; [m0, M] |
| `E` | reserva energética | real | kcal; [0, E_max(m)] |
| `s` | sexo | discreto | {HEMBRA, MACHO} |
| `estado` | estado reproductivo | discreto | hembra: {INMADURA, DISPONIBLE, GESTANTE, LACTANTE, POSTREPRODUCTIVA}; macho: {INMADURO, MADURO} |
| `dias_rest` | días hasta el parto o el destete | entero | días |
| `N[s][k]` | individuos por sexo y clase (modo población) | entero | ≥ 0 |
| `E_tot[s][k]` | energía total por sexo y clase | real | kcal; ≥ 0 |
| `Gq[j]`, `Lq[j]` | colas de gestación y lactancia | entero | ≥ 0 |
| `V_c` | stock de forraje accesible | real | g/m²; [0, K_c] |
| `PPN_anual` | productividad primaria neta | real | g/m²/año; [0, 3000] |
| `T̄`, `P̄` | temperatura y precipitación medias anuales móviles | real | °C, mm/año |
| `Foco` | celda observada con individuos | booleano | {0,1} |

---

## 5. Parámetros ajustables

### 5.1 Especie de referencia y exponentes de escala
| Símbolo | Significado | Valor ancla (M_ref = 70 kg) | Exponente `b` | Unidades |
|---|---|---|---|---|
| `B0` | constante de Kleiber (metabolismo basal) | 70 | (fórmula, ver §6.0) | kcal/(día·kg^0.75) |
| `PAL` | factor de actividad (gasto real / basal) | 2.5 | 0 | — |
| `a_max` | edad máxima | 25 | 0.20 | años |
| `a_crec90` | edad al 90 % de la masa adulta | 1.5 | 0.25 | años |
| `a_rep` | edad de madurez reproductiva | 1.5 | 0.25 | años |
| `d_gest` | duración de la gestación | 200 | 0.25 | días |
| `d_destete` | duración de la lactancia | 120 | 0.25 | días |
| `Δa` | anchura de clase de edad (modo población) | 365 | 0.25 | días |
| `camada` | crías por parto (media) | 1 | — | crías |
| `f_neo` | masa de la cría al nacer / masa adulta | 0.06 | 0 | — |
| `f_grasa_max` | fracción máxima de masa en grasa | 0.15 | 0 | — |
| `k_ap` | apetito / gasto | 2.0 | 0 | — |
| `ε` | eficiencia digestiva del forraje | 0.5 | 0 | — |
| `f_com` | fracción de la PPN accesible a la especie | 0.004 | 0 | — |
| `p_conc` | probabilidad diaria de concepción (hembra disponible, con pareja, en buena condición) | 0.03 | — | 1/día |
| `ρ_par` | machos maduros necesarios por hembra disponible (modo población) | 0.1 | — | — |
| `a_fin_rep` | edad de fin de la reproducción | = a_max | — | años |
| `c_dup` | tiempo de duplicación de la mortalidad / a_max | 0.08 | — | — |

Notas:
- El exponente de la vida es 0.20 porque es el valor empírico en mamíferos (Calder): 11,6·M^0,2 años da ~27 años para 70 kg. La teoría metabólica predice 0.25. Ambos son defendibles; se deja parametrizado.
- `f_com = 0.004` está calibrado para que la densidad de equilibrio reproduzca la ley de Damuth (ver §10). Por la "equivalencia energética" (densidad · gasto ≈ constante con la masa), el mismo `f_com` vale para cualquier masa.

### 5.2 Preset "humano" (sobrescribe lo derivado)
| Símbolo | Valor | Fuente / motivo |
|---|---|---|
| `M` | 60 kg | adulto cazador-recolector medio |
| `PAL` | 1.8 | gasto de cazadores-recolectores |
| `f_grasa_max` | 0.20 | |
| `a_max` | 110 años | |
| `a_crec90` | 16 años | (ver Pregunta Abierta P1: curva de crecimiento humana) |
| `a_rep` | 19 años | edad media al primer parto en cazadores-recolectores |
| `a_fin_rep` | 50 años | menopausia |
| `d_gest` | 270 días | |
| `d_destete` | 900 días (~2,5 años) | destete en cazadores-recolectores |
| `f_neo` | 0.055 | 3,3 kg al nacer |
| `camada` | 1 | |
| `ε` | 0.85 | dieta de mayor calidad (carne, tubérculos, frutos) |
| `f_com` | 1.1·10⁻⁴ | calibrado a ~0,3 personas/km² con PPN ~700 g/m²/año |
| `p_conc` | 0.007 /día | fecundabilidad ~0,2 por ciclo de 28 días |
| `ρ_par` | 1.0 | |
| `ρ_sexo` | 0.512 | proporción de varones al nacer |
| `β3` | 0.085 /año | duplicación de la mortalidad adulta cada ~8 años (Gompertz) |
| `p_huerf` | 0.5 | los huérfanos lactantes sobreviven a veces gracias a otros cuidadores |

### 5.3 Energética, crecimiento y reproducción (todas las especies)
| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `ρ_grasa` | energía por kg de grasa | 9 000 | kcal/kg |
| `η` | eficiencia de almacenar el sobrante | 0.85 | — |
| `c_crec` | coste energético de crear tejido nuevo | 3 000 (rango 2 000-6 000) | kcal/kg |
| `r_crec_min` | condición mínima (E/E_max) para crecer | 0.2 | — |
| `k_gest` | sobrecoste diario de la gestación / basal adulto | 0.2 | — |
| `ε_leche` | eficiencia de convertir reservas en leche | 0.8 | — |
| `r_0` | reserva inicial de la cría / su E_max | 0.5 | — |
| `r_min`, `r_opt` | condición (E/E_max) mínima y óptima para concebir | 0.3 / 0.6 | — |
| `r_abort` | condición bajo la cual se pierde la gestación | 0.1 | — |
| `ρ_sexo` | proporción de machos al nacer | 0.5 | — |
| `p_huerf` | prob. de que muera una cría lactante huérfana | 1.0 | — |
| `B0_ecto` | constante metabólica de ectotermos a T_ref | 7 (= B0/10, orientativo) | kcal/(día·kg^0.75) |
| `E_a` | energía de activación (teoría metabólica) | 0.65 | eV |
| `T_ref` | temperatura de referencia de ectotermos | 20 | °C |

### 5.4 Mortalidad (modelo de Siler)
| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `α1`, `β1` | mortalidad juvenil (decrece con la edad) | calibrar | 1/año, 1/año |
| `α2` | mortalidad de fondo constante | calibrar | 1/año |
| `α3`, `β3` | mortalidad senescente (crece con la edad) | calibrar; `β3 = ln2/(c_dup·a_max)` | 1/año, 1/año |

Los `α` se calibran contra los objetivos de la §10 (tabla de vida).

### 5.5 Forraje (en el terreno)
| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `ρ_veg` | energía bruta de la materia vegetal seca | 4.4 | kcal/g |
| `τ_c` | años de producción que caben en el stock máximo | 0.5 | años |
| `g0_frac` | rebrote mínimo (banco de semillas), fracción de la regeneración máxima | 0.01 | — |
| `T_0`, `T_opt_v` | temperatura de inicio y de plenitud del crecimiento vegetal | 0 / 20 | °C |
| `θ_w`, `θ_c` | humedad de marchitez y de crecimiento pleno | 0.15 / 0.5 | — |

### 5.6 Foco y modo población
| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `N_ind_max` | presupuesto máximo de individuos en foco (todo el planeta) | 200 000 | individuos |
| `u_on`, `u_off` | umbral de activación y desactivación del foco automático | 1.0 / 0.5 | — |
| `w1..w4` | pesos de la puntuación de actividad | 1 / 2 / 1 / 3 | — |
| `t_min_foco` | permanencia mínima en foco (o fuera) | 90 | días |
| `ventana_foco` | ventana para medir la actividad | 30 | días |
| `cv_E` | dispersión de reservas dentro de una clase (coef. de variación) | 0.3 | — |
| `b_q` | anchura de bin de las colas de gestación/lactancia | 7 | días |
| `fraccion_inicial` | población inicial / capacidad estimada | 0.5 | — |

---

## 6. Reglas y ecuaciones por tick

### 6.0 Derivación de la especie y población inicial (una vez)

**Metabolismo basal (ley de Kleiber):**
```
Endotermo:  B(m) = B0 · m^(3/4)                                        (kcal/día)
Ectotermo:  B(m,T) = B0_ecto · m^(3/4) · exp( −(E_a/k_B) · ( 1/(T+273,15) − 1/(T_ref+273,15) ) )
            k_B = 8,617·10⁻⁵ eV/K
```
- El de sangre fría depende de la temperatura de su celda: más calor, más ritmo vital.

**Reservas máximas:** `E_max(m) = f_grasa_max · m · ρ_grasa`

**Tiempos y masas derivadas:** `Q(M) = Q_ref · (M/M_ref)^b_Q` (tabla 5.1), salvo override del preset. Masa al nacer `m0 = f_neo · M`.

**Constante de crecimiento (modelo ontogenético de West).** Se calcula para que la cría llegue al 90 % de la masa adulta a la edad `a_crec90` (en días):
```
c_W = ( 4 · M^(1/4) / a_crec90 ) · ln( (1 − (m0/M)^(1/4)) / (1 − 0,9^(1/4)) )
```

**Mortalidad senescente:** `β3 = ln 2 / (c_dup · a_max)` (o valor directo del preset).

**Población inicial (génesis de agentes).** En cada celda de nivel 0 habitable:
```
N_inicial = fraccion_inicial · N_max_sostenible(celda)
```
- Edades repartidas según la curva de supervivencia `l(a) = exp(−∫ μ)` (la distribución estable aproximada), mitad hembras y mitad machos (`ρ_sexo`), `E = 0,5 · E_max(m(a))`, colas de gestación y lactancia vacías.
- Si la celda está en Foco, se desagrega a individuos (§6.3).

### 6.1 Sistema de forraje (terreno, cada tick, después de la vegetación)

**Medias climáticas móviles** (memoria de un año; se inicializan en la génesis con las medias climatológicas, para no necesitar calentamiento):
```
T̄(t+1) = T̄ + ( T − T̄ ) / dias_anio
P̄(t+1) = P̄ + ( P · dias_anio − P̄ ) / dias_anio            (P del tick anualizada, mm/año)
```

**Productividad primaria neta anual (modelo Miami, Lieth 1975)** — lo que crecen las plantas por m² y año, limitado por el calor o por el agua, el que sea más escaso:
```
f_T(T̄) = 3000 / ( 1 + e^(1,315 − 0,119·T̄) )
f_P(P̄) = 3000 · ( 1 − e^(−0,000664·P̄) )
PPN_anual = min( f_T , f_P )                                  (g de materia seca/m²/año)
```

**Factor estacional** (las plantas crecen cuando hace temperatura y hay agua en el suelo), normalizado para que su media anual sea 1:
```
g_T = clamp( (T − T_0) / (T_opt_v − T_0) , 0 , 1 )
g_θ = clamp( (θ − θ_w) / (θ_c − θ_w) , 0 , 1 )
S̄(t+1) = S̄ + ( g_T·g_θ − S̄ ) / dias_anio
s(t) = g_T·g_θ / S̄        (si S̄ ≈ 0, s = 0)
```

**Stock de forraje por especie (crecimiento logístico calibrado a la productividad real):**
```
K_c = τ_c · f_com · PPN_anual                                  (stock máximo, g/m²)
r_c = 4 / ( τ_c · dias_anio )                                  (1/día)
g_0 = g0_frac · f_com · PPN_anual / dias_anio                  (rebrote mínimo)
V_c(t+1) = clamp( V_c + s(t) · [ g_0 + r_c · V_c · (1 − V_c/K_c) ] − consumo_c , 0 , K_c )
```
- Con estas constantes, la regeneración máxima (que se da a media carga) es exactamente `r_c·K_c/4 = f_com·PPN_anual/dias_anio`. El stock produce de media lo que dice la productividad real.
- `consumo_c` llega de los agentes (§6.10), en g/m².
- Una sequía baja `θ`, con ella `s(t)`, y el rebrote se frena.

**LOD del forraje (las dos reglas):** el total de la celda (`V_c · A_celda`) es extensivo.
- Al subdividir, las hijas reciben densidad proporcional a su `K_c`, normalizada para que `Σ V_c,i · A_i = V_c · A`.
- Al colapsar, la madre toma la media ponderada por área.

### 6.2 Sistema de Foco — cada `N_lod` ticks (antes del LOD del terreno)

**Puntuación de actividad** de cada celda de nivel 0 con población, medida en la `ventana_foco`:
```
u = w1 · |ΔN/N|  +  w2 · f_inanicion  +  w3 · |flujo_migratorio_neto|/N  +  w4 · evento
```
- `ΔN/N`: cambio relativo de población. `f_inanicion`: fracción que ha muerto de hambre. `flujo_migratorio_neto`: entradas menos salidas. `evento`: 1 si ocurrió un evento marcado (hambruna, extinción local, conflicto futuro), 0 si no.

**Reglas de entrada y salida (con histéresis y permanencia mínima, para evitar parpadeos):**
```
entra en foco  ⟺  u > u_on   y   lleva ≥ t_min_foco fuera
sale del foco  ⟺  u < u_off  y   lleva ≥ t_min_foco dentro   y   no es región fija
```

**Presupuesto (determinista):**
1. Las regiones fijas de la configuración entran siempre.
2. Las candidatas automáticas se ordenan por `u` descendente (empate: índice de celda ascendente) y entran mientras `Σ N_estimado ≤ N_ind_max`.
3. Si las regiones fijas solas superan el presupuesto, se avisa en el registro y se respetan igualmente (decisión del director).

**Acople con el terreno:** el disparador de subdivisión del terreno "hay agentes en la celda" pasa a ser **"la celda pertenece a una celda de nivel 0 en Foco"**. Dentro del Foco, el terreno se afina hasta el nivel de individuos `L_ind` (por defecto `L_max`). Fuera, siguen solo los disparadores físicos (gradientes, ríos, costas).

### 6.3 Conversión individuo ↔ población — cada `N_lod` ticks (después del LOD del terreno)

Cumple las **dos reglas**: conserva los extensivos y es función pura de (estado, posición, semilla, tick).

**Agregación (la celda sale del Foco: individuos → población):**
```
N[s][k]    = nº de individuos de sexo s con edad en la clase k
E_tot[s][k] = Σ E_i de esos individuos                         (suma exacta)
Gq[j]      = nº de gestantes cuyo dias_rest cae en el bin j
Lq[j]      = (nº de lactantes cuyo dias_rest cae en el bin j , nº total de sus crías dependientes vivas)
```
- Se conservan exactamente: nº de individuos por sexo y clase, energía total y nº de gestantes y lactantes.
- Se pierden la identidad, la posición fina y el retraso de crecimiento de cada individuo (ver P7).

**Desagregación (la celda entra en Foco: población → individuos):**
```
Para cada (s,k), crear N[s][k] individuos:
  edad   a_i = inicio_k + floor( u_i · Δa )                    u_i ∈ [0,1) del generador (§6.11)
  masa   m_i = curva de crecimiento evaluada en a_i (§6.6, sin retraso)
  reserva: x_i ~ Gamma(forma = 1/cv_E², escala = 1) ;  w_i = x_i / Σ x_j
           E_i = E_tot[s][k] · w_i                              (Σ E_i = E_tot exacto)
Estados reproductivos:
  - se eligen Σ Gq hembras en edad reproductiva → GESTANTE, con dias_rest del bin
  - se eligen Σ madres(Lq) hembras de las restantes → LACTANTE, con dias_rest del bin
  - las Σ crías(Lq) crías más jóvenes de la clase 0 se asignan a las lactantes de cada bin
    (cada bin reparte sus crías por igual entre sus madres);
    si sobran crías sin madre: se aplica la regla de huérfanos (§6.8)
  - el resto de hembras: INMADURA / DISPONIBLE / POSTREPRODUCTIVA según la edad
Posición:
  - cada madre con sus crías dependientes va junta
  - celda fina elegida con probabilidad ∝ K_c(celda) · A_celda (calidad del hábitat)
```
- La elección de qué hembras son gestantes, el reparto de reservas y la posición salen del generador determinista con claves fijas. Así la desagregación es una función pura.

**Fronteras del Foco (flujos de migración, cuyas reglas pertenecen al documento de comportamiento):**
- Un individuo que sale del Foco hacia una celda en modo población queda **absorbido**: se suma a sus cohortes y colas.
- Un subgrupo que emigra de una población hacia una celda en Foco se **emite**: se desagrega con la regla anterior.

### 6.4 Demanda y reparto justo del forraje — cada tick

**Demanda de cada consumidor** (no come más de lo que le cabe, para no destruir comida del terreno inútilmente):
```
Individuo:   d_i = min( Ap(m_i) , max(0, E_max(m_i) − E_i + G_i) / ε )
             cría dependiente: multiplicar por (1 − φ_leche(a))   (el resto le llega como leche)
Población:   d_k = N_k · min( Ap(m_k) , max(0, E_max(m_k) − ē_k + G_k) / ε )     con ē_k = E_tot_k / N_k
Ap(m) = k_ap · B(m) · PAL
```

**Oferta de la celda** (kcal brutas): `F = V_c · A_celda · ρ_veg`

**Reparto proporcional** (independiente del orden en que se procesan los consumidores, condición de determinismo):
```
x_i = d_i · min( 1 , F / Σ_j d_j )
```
- Si hay comida de sobra, cada uno come lo que pide. Si falta, todos reciben la misma proporción de lo que piden.
- Una población (nivel 0) consume del forraje total de todas las celdas descendientes de su celda. El consumo se descuenta de cada descendiente en proporción a su stock.

### 6.5 Energética individual — cada tick

**Gasto del día:**
```
G_i = B(m_i) · PAL                                   (mantenimiento + actividad)
    + c_crec · Δm_i                                   (crecimiento, §6.6)
    + [GESTANTE]  k_gest · B(M)                       (gestación)
    + [LACTANTE]  Σ_crías φ_leche(a_c) · G_c / ε_leche (leche para las crías)
φ_leche(a) = max( 0 , 1 − a / d_destete )            (la leche se sustituye por comida de forma lineal hasta el destete)
```

**Balance de reservas:**
```
I_asim = ε · x_i                                      (energía asimilada)
si I_asim ≥ G_i:   E_i ← E_i + η · (I_asim − G_i)     (el sobrante se guarda con pérdidas)
si I_asim < G_i:   E_i ← E_i − (G_i − I_asim)         (el déficit sale de las reservas)
E_i ← min( E_i , E_max(m_i) )
Cría dependiente: además recibe φ_leche(a)·G_c como leche (lo paga la madre).
```
- Si `E_i ≤ 0` → **muerte por inanición** (§6.8).

**Coste térmico de endotermos (opcional, apagado por defecto):** `+ C_term · m^(1/2) · max(0, T_lc − T)`. Requiere la temperatura crítica inferior `T_lc` y la conductancia `C_term` de cada especie (ver P10).

### 6.6 Crecimiento — cada tick (individuos con m < M)

```
si E_i / E_max(m_i) ≥ r_crec_min:
    Δm = c_W · m^(3/4) · ( 1 − (m/M)^(1/4) )         (kg/día; modelo de West)
si no:
    Δm = 0                                             (retraso del crecimiento por hambre)
m ← min( m + Δm , M )
```
- La curva "sin retraso" que se usa para asignar masa por edad es la solución exacta: `(m/M)^(1/4) = 1 − (1 − (m0/M)^(1/4)) · e^(−c_W·a / (4·M^(1/4)))`.

### 6.7 Reproducción sexual — máquinas de estados (cada tick)

**Hembra:**
```
INMADURA ──(a ≥ a_rep)──────────────────────────────→ DISPONIBLE
DISPONIBLE ──(concepción)───────────────────────────→ GESTANTE (dias_rest = d_gest)
GESTANTE ──(dias_rest = 0)──→ [PARTO] ──────────────→ LACTANTE (dias_rest = d_destete)
GESTANTE ──(E/E_max < r_abort)──→ [ABORTO] ─────────→ DISPONIBLE   (la energía invertida se pierde)
LACTANTE ──(dias_rest = 0  ó  mueren todas sus crías)→ DISPONIBLE
cualquier estado ──(a ≥ a_fin_rep)──────────────────→ POSTREPRODUCTIVA (si está gestante o lactante, termina antes el ciclo)
```

**Macho:** `INMADURO ──(a ≥ a_rep)──→ MADURO`. Sin coste energético directo de reproducción en esta versión.

**Concepción** (hembra DISPONIBLE, cada día):
```
p = p_conc · f_E( E/E_max ) · H_macho
f_E(r) = clamp( (r − r_min) / (r_opt − r_min) , 0 , 1 )      (fertilidad según la condición corporal)
H_macho = 1 si hay al menos un macho MADURO en su misma celda fina; si no, 0
```
- La búsqueda activa de pareja pertenece al documento de comportamiento.

**Parto:**
```
n_crías = floor(camada) + Bernoulli( camada − floor(camada) )
cada cría:  sexo = MACHO con prob ρ_sexo
            m = m0 ;  E0 = r_0 · E_max(m0)
la madre paga: E_madre ← E_madre − Σ E0
si E_madre no alcanza: las crías reciben lo que haya, repartido por igual (pueden morir pronto: emergente)
```

### 6.8 Mortalidad — cada tick

Se evalúan en este orden, y la primera que ocurre mata:
1. **Inanición:** `E ≤ 0` → muerte.
2. **Edad máxima:** `a ≥ a_max` → muerte.
3. **Intrínseca (modelo de Siler).** Riesgo anual por edad, con tres componentes: infantil decreciente, fondo constante y senescente creciente:
```
μ(a) = α1 · e^(−β1·a)  +  α2  +  α3 · e^(β3·a)          (a en años, μ en 1/año)
p_muerte_dia = 1 − exp( −μ(a) / dias_anio )
```
4. **Huérfanos:** si muere una madre LACTANTE, cada cría dependiente muere con probabilidad `p_huerf`. Las que sobreviven pasan a alimentarse solas (`φ_leche = 0`).

**Energía de los muertos:** su reserva sale del sistema (se contabiliza como "pérdida por mortalidad" en el balance energético; como carroña es ampliación futura).

### 6.9 Modo población (cohortes) — cada tick

Mismas reglas que el individuo, pero aplicadas a clases de edad. Con recuentos enteros y extracciones binomiales, se conserva la **estocasticidad demográfica**: el azar de los números pequeños, que permite extinciones locales.

**Clases de edad:** anchura `Δa` días, `K = ceil(a_max_días / Δa)` clases. Edad representativa `a_k` = punto medio; masa `m_k` = curva de crecimiento sin retraso en `a_k`.

**Energía por clase:**
```
ē_k = E_tot_k / N_k
x̄_k = reparto justo de §6.4 dividido entre N_k
Δ_k = G_k − ε·x̄_k                                    (déficit medio por individuo, si > 0)
```

**Inanición (aproximación de campo medio coherente con la desagregación).** Se supone que las reservas dentro de la clase siguen una Gamma de media `ē_k` y coeficiente de variación `cv_E`, la misma que usa la desagregación:
```
si Δ_k > 0:
   f_inan = F_Γ( Δ_k ; forma = 1/cv_E² , escala = ē_k·cv_E² )        (fracción cuya reserva no cubre el déficit)
   D_inan ~ Binomial( N_k , f_inan )
   energía que se llevan: D_inan · ē_k · F_Γ(Δ_k ; forma+1 , escala) / F_Γ(Δ_k ; forma , escala)
   (media condicionada de quienes estaban por debajo del déficit)
```
- `F_Γ` es la función de distribución acumulada Gamma, una función matemática estándar.

**Actualización de energía de los supervivientes:** misma regla de §6.5 aplicada a la media, multiplicada por los supervivientes, con tope `N_k · E_max(m_k)`.

**Mortalidad intrínseca:** `D_int ~ Binomial( N_supervivientes , p_muerte_dia(a_k) )`, y se llevan `ē_k` cada uno.

**Reproducción con colas:**
```
F_disp = max( 0 , hembras en edad [a_rep, a_fin_rep) − Σ Gq − Σ madres(Lq) )
H̄_macho = min( 1 , N_machos_maduros / (ρ_par · F_disp) )
ē_F = energía media de las hembras reproductivas
C ~ Binomial( F_disp , p_conc · f_E(ē_F/E_max(M)) · H̄_macho )      → se suman a Gq en el bin de d_gest
Abortos: fracción F_Γ( r_abort·E_max(M) ; ... ) de las gestantes → Binomial → vuelven a disponibles
Partos: el bin de Gq que llega a 0 → nacimientos = Σ crías (Bernoulli como en §6.7),
        sexos ~ Binomial( nacimientos , ρ_sexo ), se suman a la clase 0, E0 sale de E_tot de las hembras,
        las madres pasan a Lq en el bin de d_destete
Destetes: el bin de Lq que llega a 0 → esas madres vuelven a disponibles
```
- **Costes de gestación y lactancia:** `Σ Gq · k_gest · B(M)` y `Σ_j crías(Lq[j]) · φ_leche(a_j) · G_c(a_j) / ε_leche` (con `a_j` la edad de las crías de ese bin) se descuentan de la `E_tot` de las clases de hembras reproductivas, en proporción a su número.
- **Coherencia de las colas con la mortalidad** (para que los recuentos no se descuadren):
  - las muertes de hembras en edad reproductiva se reparten entre disponibles, gestantes y lactantes en proporción a su número (extracciones binomiales sucesivas sobre los bins de `Gq` y `madres(Lq)`);
  - las muertes de crías de clase 0 menores de `d_destete` se descuentan de los contadores `crías(Lq)` en proporción;
  - un bin de `Lq` que se queda sin crías libera a sus madres, que vuelven a disponibles. Es la misma regla que en individuos: si mueren todas las crías, termina la lactancia.
- Las colas avanzan un bin cada `b_q` días.

**Envejecimiento de cohortes:** cada vez que `t mod Δa = 0`, todas las clases se desplazan una posición (`N[s][k] → N[s][k+1]`, y su energía con ellas). La última clase muere por edad máxima. Los nacidos se acumulan en la clase 0 entre desplazamientos.

### 6.10 Escritura sobre el terreno — al final del tick

```
consumo_c(celda) = Σ x_i (kcal brutas consumidas en la celda) / ( ρ_veg · A_celda )      (g/m²)
```
- Se aplica al stock `V_c` en el paso de escrituras del final del tick (cola). Como el reparto justo garantiza `Σ x ≤ F`, el stock nunca baja de 0.
- Las demás escrituras de agentes previstas en la visión madre (deforestar, regar, urbanizar) siguen la misma cola.

### 6.11 Azar determinista e identificadores

- **Generador basado en contador** (tipo Philox o un hash criptográfico rápido): cada número aleatorio es una función pura `H(semilla, id_entidad, tick, propósito, índice)`. No hay un "estado del generador" que avance. Consecuencias:
  - el resultado no depende del orden en que se procesen las entidades, ni del paralelismo;
  - la desagregación es reproducible, sea cuando sea que ocurra.
- `propósito` distingue cada uso (concepción, mortalidad, sexo, edad en desagregación...), para que dos decisiones distintas no compartan número.
- **Identificadores deterministas:**
  - recién nacido: `id = H(id_madre, nº de parto de la madre, índice en la camada)`;
  - individuo desagregado: `id = H(id_poblacion, tick, s, k, i)`;
  - colisiones: con 64 bits o más son despreciables; detectar y registrar si ocurrieran.
- Las extracciones Binomial y Gamma se generan a partir de este mismo generador.

---

## 7. Eventos discretos (para registrar)

Cada evento: `tipo`, `tick`, `celda`, `ids implicados`, `datos`. En modo población se registran **recuentos agregados por celda y tick** (no un evento por individuo), para acotar el tamaño del registro.
- **Nacimiento** (madre, crías, sexos, masa y reserva iniciales).
- **Muerte**, con causa: inanición / intrínseca / edad máxima / orfandad; edad y reserva al morir.
- **Concepción**, **aborto**, **destete**.
- **Entrada / salida de Foco** de una celda de nivel 0 (nº de individuos convertidos, motivo: región fija o puntuación).
- **Absorción / emisión** en la frontera del Foco.
- **Extinción local:** la población de una celda de nivel 0 llega a 0.
- **Hambruna:** `f_inanicion` de una celda por encima de un umbral (parámetro, defecto 1 % en la ventana).
- **Explosión demográfica:** crecimiento relativo por encima de un umbral en la ventana.

## 8. Métricas agregadas (por tick y por año)

- Población total, por modo (individuos / poblaciones), por sexo y por especie.
- Pirámide de edades (por clase y sexo).
- Tasas brutas de natalidad y mortalidad (por 1 000 habitantes y año); mortalidad por causa.
- Esperanza de vida al nacer (tabla de vida del periodo, cada año), tasa global de fecundidad (hijos por mujer) e intervalo medio entre nacimientos.
- Condición corporal media `E/E_max` y fracción en inanición.
- Densidad por bioma frente a la PPN, para comparar con la ley de Damuth.
- Presión de herbivoría: consumo / producción de forraje, por bioma.
- **Balance energético global (control de conservación):**
```
Σ asimilada del forraje
 − Σ gasto propio (mantenimiento y actividad, crecimiento, gestación)
 − Σ pérdidas por almacenamiento (fracción 1−η del sobrante)
 − Σ pérdidas en la conversión a leche (fracción 1−ε_leche de la leche pagada)
 − Σ reservas de los muertos
 = Δ(Σ E de todos los vivos)                                   (a tolerancia numérica)
```
Las transferencias internas (leche recibida, reserva inicial de las crías) se cancelan entre madre y cría, y no aparecen en el balance.
- Individuos en Foco frente a `N_ind_max` (coste).

## 9. Interacción con otros sistemas y orden de ejecución dentro del tick

```
TERRENO (orden de la visión madre)
 T1. Foco (cada N_lod)                         ← nuevo, decide qué celdas de nivel 0 están en foco
 T2. LOD del terreno (cada N_lod)              ← disparador "hay agentes" sustituido por "en Foco"
 T3. Clima → Nieve → Balance hídrico → Agua superficial → Vegetación
 T4. Forraje (PPN, factor estacional, rebrote)  ← nuevo (§6.1)
 T5. Erosión / Pedogénesis (en su cadencia)
 T6. Interfaz del terreno para agentes
AGENTES
 A1. Conversión individuo ↔ población (cada N_lod, tras T2, porque necesita las celdas finas ya creadas)
 A2. Comportamiento / movimiento                ← documento aparte (hueco reservado)
 A3. Demanda y reparto justo del forraje (§6.4)
 A4. Energética: ingesta, gasto, gestación, lactancia (§6.5 / §6.9)
 A5. Crecimiento (§6.6)
 A6. Reproducción: concepciones, abortos, partos, destetes (§6.7 / §6.9)
 A7. Mortalidad: inanición, edad máxima, intrínseca, orfandad (§6.8 / §6.9)
 A8. Envejecimiento: edad + 1 día; desplazamiento de cohortes cuando t mod Δa = 0
 A9. Escrituras al terreno: consumo de forraje y demás escrituras encoladas (§6.10)
```
- `dias_anio` (agentes) = `T_anio` (terreno) = 365.
- El terreno expone a los agentes: `T`, `θ`, `V_c`, `K_c`, `A_celda`, `Foco` y la interfaz (habitabilidad, coste de movimiento, agua disponible).
- El forraje se calcula antes de que coman los agentes. Su consumo se aplica al final del tick y el rebrote del día siguiente parte del stock ya consumido.

## 10. Validación y determinismo

### Comprobaciones de calibración (ya verificadas numéricamente)
| Comprobación | Valor del modelo | Referencia real |
|---|---|---|
| Metabolismo basal humano 60 kg (Kleiber) | 1 509 kcal/día | ~1 400-1 600 medido |
| Gasto diario humano (PAL 1,8) | ~2 716 kcal/día | cazadores-recolectores, del orden de 2 500-3 000 |
| Ayuno humano (20 % grasa, 60 kg) | 108 000 kcal → ~40 días | supervivencia sin comer del orden de 1-2 meses |
| Coste total de la gestación humana | 0,2 · 1 509 · 270 ≈ 81 500 kcal | estimaciones ~77 000-85 000 kcal |
| Densidad de herbívoro de 70 kg (ley de Damuth: ≈ 96·M^(−3/4) ind/km², M en kg) | ≈ 3,9 /km² con f_com = 0,004 y PPN 700 | ley de Damuth (mamíferos) |
| Densidad humana (f_com 1,1·10⁻⁴, PPN 700) | ≈ 0,3 /km² | cazadores-recolectores 0,1-1 /km² |
| PPN selva (25 °C, 2 500 mm) | ≈ 2 430 g/m²/año | ~2 200 (Whittaker y Likens) |
| PPN desierto (25 °C, 100 mm) | ≈ 190 | ~90 (matorral desértico) |
| PPN tundra (−10 °C, 300 mm) | ≈ 230 | ~140 |
| PPN templado (10 °C, 700 mm) | ≈ 1 100 | ~1 200 (bosque templado) |

### Objetivos de calibración de la mortalidad (con comida suficiente)
- **Humanos (cazadores-recolectores, Gurven y Kaplan 2007):** supervivencia hasta los 15 años ≈ 55-60 %; edad modal de muerte adulta ≈ 70 años; esperanza de vida al nacer ≈ 30-35 años.
- **Genérico de 70 kg:** supervivencia del primer año ≈ 0,6; mortalidad adulta anual ≈ 0,1.
- Los `α` de Siler se ajustan para cumplir estos objetivos **en una corrida sin inanición**. La inanición se suma después en tiempos difíciles (ver P4).

### Pruebas de conservación y coherencia
- **Balance energético global** (§8) cerrado a tolerancia de coma flotante.
- **Entrar y salir del Foco repetidamente no deriva nada:** nº de individuos por sexo y clase exacto (enteros); energía total a redondeo.
- **Coherencia entre modos:** el mismo escenario simulado todo en individuos y todo en poblaciones, con al menos 10 semillas. Población media N(t), pirámide de edades, natalidad y mortalidad deben coincidir dentro de una tolerancia estadística (por defecto 5 %). Es la prueba de independencia del camino aplicada a los agentes.
- **Equivalencia energética:** dos especies de masas muy distintas en el mismo hábitat deben consumir aproximadamente la misma energía por km² en equilibrio (densidad · gasto ≈ constante).

### Determinismo
- Misma semilla + mismos parámetros → corrida idéntica bit a bit.
- **Prueba de orden:** barajar internamente el orden de proceso de las entidades debe dar un resultado idéntico. Comprueba el reparto justo y el generador por contador.

### Casos límite
- Sin forraje → extinción en ~`E_max / G` días (≈ 40 en humanos, ≈ 22 en el genérico de 70 kg).
- Sin machos maduros → sin nacimientos.
- Sequía prolongada → baja PPN → hambruna → mueren primero crías y viejos y bajan las concepciones → recuperación al volver la lluvia.
- Población inicial muy pequeña → posible extinción por azar demográfico, aunque haya comida.
- `k_ap ≤ 1` → los agentes no pueden rehacer reservas: la regla de estabilidad generalizada se incumple, y la prueba debe detectarlo.

## 11. Preguntas abiertas

- **P1 — Curva de crecimiento humana.** El modelo de West ajustado al 90 % a los 16 años da ~7 g/día de crecimiento al nacer, frente a los ~25-30 g/día reales, y no tiene el estirón adolescente. ¿Curva tabulada para el preset humano?
- **P2 — Dependencia prolongada humana.** Los niños humanos dependen de los adultos hasta ~15 años (reparto de comida). Es un tema del documento social. Mientras tanto, se alimentan solos tras el destete.
- **P3 — Estacionalidad reproductiva.** Celo ligado al clima: crías en primavera, conectado con la temperatura del terreno.
- **P4 — Calibración de Siler.** Los datos reales de mortalidad incluyen muertes por hambre. ¿Calibramos con comida suficiente (opción por defecto, sobrestima algo la mortalidad total en tiempos malos) o separamos la fracción no energética?
- **P5 — Varias especies y competencia por el mismo forraje.**
- **P6 — Individuos persistentes** (con nombre, líderes) que conserven su identidad aunque su celda pase a modo población.
- **P7 — Masa y retraso de crecimiento al agregar.** Se pierden al pasar a modo población. ¿Guardar un déficit medio de masa por clase?
- **P8 — Carroña:** destino de la energía de los muertos (cadena trófica futura).
- **P9 — Calibrar el Foco automático:** presupuesto `N_ind_max` y pesos `w1..w4`.
- **P10 — Coste térmico de endotermos:** activarlo requiere `T_lc` y `C_term` por especie (y, en humanos, ropa y fuego).
- **P11 — Humanos y forraje.** Para humanos, `f_com` representa caza y recolección. La agricultura cambiará esta fuente de comida (documento propio).
