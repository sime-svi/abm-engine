# Documento de Diseño: Espacio y Geografía (El Terreno)

**Sistema:** Sustrato espacial físico del motor ABM
**Versión:** 1.0
**Destinatario:** Claude Code (Opus 4.8, effort máximo) para implementación
**Naturaleza de este documento:** Es el plano teórico. Contiene fenómeno real, lógica de estados y ecuaciones. NO contiene código; las decisiones de implementación (estructuras de datos, lenguaje, optimización) quedan a criterio de quien lo programe, siempre respetando la matemática y las reglas de determinismo aquí fijadas.

**Notación:**
- `·` multiplicación, `√` raíz, `^` exponente, `_` subíndice.
- Un "tick" es el paso de tiempo entero `t ∈ ℕ` del motor. Es adimensional (ver Pregunta Abierta P1 sobre su calibración).
- "Extensivo" = magnitud que se suma al juntar celdas (volumen de agua, masa de nieve, biomasa). "Intensivo" = magnitud que se promedia (temperatura, humedad relativa).

---

## 1. Fenómeno real y objetivo

### Qué se modela
El escenario físico del mundo: una esfera planetaria con relieve (montañas, valles, llanuras), un clima que cambia con la latitud, la altura y las estaciones, un suelo con distinta composición y humedad, masas de agua (océanos, lagos, ríos) y vegetación. Y, sobre todo, un escenario que **evoluciona con el tiempo**: los ríos tallan valles, las montañas se erosionan, el suelo se forma, los lagos crecen y menguan.

### Qué debe emerger (no se pinta a mano, sale solo de la cadena causal)
- **Biomas**: desiertos, bosques, tundras, praderas, aparecen como consecuencia de la combinación de temperatura y humedad, no se colocan manualmente.
- **Sombra pluviométrica (rain shadow)**: desiertos detrás de las cordilleras, en la cara a sotavento del viento dominante.
- **Cuencas hidrográficas**: ríos que nacen en las alturas, confluyen y desembocan en el mar o en lagos endorreicos.
- **Valles fluviales y deltas**: esculpidos por la erosión a lo largo de muchos ticks.
- **Ciclo estacional**: inviernos y veranos, con hemisferios opuestos.
- **Franjas climáticas**: trópicos húmedos, desiertos subtropicales, zonas templadas, casquetes polares.

### Rol del terreno
El terreno es **instrumental**: existe para servir a los agentes. Exporta hacia ellos, celda a celda, cuatro cosas medibles: habitabilidad, coste de movimiento, agua disponible y capacidad de alimento. Además, los agentes pueden **escribir** sobre el terreno (deforestar, regar, urbanizar). Ese doble sentido (leer y escribir) es lo que cierra el bucle de comportamiento emergente.

### La cadena causal (esqueleto lógico del sistema)
```
semilla → ELEVACIÓN → (PENDIENTE, ORIENTACIÓN)
latitud + elevación + estación → TEMPERATURA
franja de latitud + relieve + viento → PRECIPITACIÓN
precipitación + temperatura + suelo + pendiente → HUMEDAD DEL SUELO
escorrentía + drenaje + relieve → AGUA SUPERFICIAL (ríos, lagos, mar)
humedad + temperatura + fertilidad → VEGETACIÓN / CAPACIDAD DE CARGA
agua superficial + pendiente + relieve → EROSIÓN → (modifica ELEVACIÓN) [bucle largo]
clima + humedad + roca → PEDOGÉNESIS → (modifica SUELO) [bucle muy largo]
todo lo anterior → habitabilidad, coste de movimiento, recursos → decisiones de los agentes
```

---

## 2. Alcance y LOD (Nivel de Detalle)

### Sustrato espacial: esfera con cubo esférico (cubed sphere)
El mundo es una **esfera real**. Para representarla con celdas cuadradas sin la singularidad grave de los polos de una rejilla latitud-longitud, se usa un **cubo esférico**: un cubo de 6 caras cuadradas proyectado sobre la esfera. Cada cara es una rejilla cuadrada independiente con su propio árbol de subdivisión.

- Una celda se identifica por `(cara c ∈ {0..5}, nivel L, i, j)`.
- La adyacencia **dentro de una cara** es trivial (vecinos i±1, j±1).
- La adyacencia **entre caras** se resuelve con un mapa fijo de las 12 aristas del cubo (qué borde de qué cara toca con qué borde de qué otra). Cruzar un borde de cara equivale a "cruzar el horizonte" del globo, incluido pasar por encima de un polo: se sale por una cara y se entra por la contigua de forma continua.
- Las 8 esquinas del cubo son puntos donde concurren 3 caras; son singularidades suaves (ver Pregunta Abierta P8).
- De la posición 3D del centro de cada celda se derivan su **latitud φ** y **longitud λ** (necesarias para el clima).
- Cada celda tiene un **área real** `A_celda` (varía ligeramente por la proyección; las celdas cercanas a las esquinas del cubo son algo menores). Esa área se usa para **ponderar toda agregación** entre niveles, garantizando la conservación.

Esto sustituye limpiamente al modelo cilíndrico: el enrollado este-oeste y el paso por los polos son ya intrínsecos a la topología del cubo, sin casos especiales de "muro polar".

### Resolución adaptativa (multi-resolución jerárquica, tipo quadtree)
La resolución **no es fija**: sigue a la relevancia.
- **Nivel base (L=0)**: toda la esfera cubierta por celdas gruesas (por defecto ~78 km de lado, ver parámetros).
- **Subdivisión bajo demanda**: cualquier celda puede partirse en 4 hijas del nivel siguiente cuando hace falta detalle (presencia de agentes, gradiente físico fuerte, cercanía a río o costa, evento activo). Se baja hasta `L_max` (por defecto ~1-2 km de lado).
- **Colapso**: cuando una zona vuelve a quedar inactiva, sus 4 hijas se funden de nuevo en la celda madre.
- **Física invariante a la escala**: las ecuaciones de este documento son las mismas en todos los niveles. Lo único que cambia es el parámetro `d` (lado de la celda) y `A_celda`.

### Reparto estático vs dinámico
- **Estático** (se genera una vez en t=0 y solo cambia por procesos lentos de erosión/pedogénesis): elevación, pendiente, orientación, latitud/longitud, área, textura base del suelo, erodibilidad de la roca madre.
- **Dinámico** (se recalcula en cada tick o en su cadencia): humedad del suelo, temperatura, precipitación instantánea, manto de nieve, agua superficial (caudal de ríos, volumen de lagos), vegetación, fertilidad efectiva, recursos.

### LOD temporal (cadencias): cada proceso corre a su ritmo
| Sistema | Cadencia (por defecto) | Motivo físico |
|---|---|---|
| Clima (T, P) | cada tick | rápido, gobierna todo lo demás |
| Nieve | cada tick | responde a la temperatura del tick |
| Balance hídrico (humedad θ) | cada tick | responde a la lluvia del tick |
| Agua superficial (ríos, lagos) | cada tick | transporta la escorrentía del tick |
| Vegetación | cada `N_veg` ticks (=4) | crece en semanas/meses |
| LOD (subdivisión/colapso) | cada `N_lod` ticks (=8) | la topología no cambia cada instante |
| Erosión y sedimentación | cada `M_ero` ticks (=64) | esculpe en escalas largas |
| Pedogénesis (formación de suelo) | cada `M_ped` ticks (=256) | milenios |

### Correcciones adoptadas para la rejilla cuadrada
- **Distancia euclídea real**: el vecino diagonal está a `d·√2`, no a `d`. Movimiento y rangos usan esta distancia.
- **Difusión casi isótropa**: en cualquier proceso de difusión lateral (humedad, calor y, en el futuro, líquidos y enfermedades), los 4 vecinos ortogonales pesan `1`, los 4 diagonales pesan `1/√2`, y se normaliza. Así la expansión es casi circular, no romboidal.
- **Flujo de agua D-infinito**: la dirección de bajada del agua puede repartirse entre las dos celdas que rodean la dirección real de máxima pendiente, en proporción al ángulo. Elimina el sesgo de ríos artificialmente rectos del método clásico de 8 direcciones.

### Qué se modela, qué se abstrae y qué se ignora a propósito
- **Se modela explícitamente**: relieve, pendiente/orientación, temperatura estacional, precipitación con efecto orográfico, humedad del suelo (modelo de cubo), nieve, agua superficial (mar por nivel, lagos por cuenca, ríos por red), vegetación por idoneidad, erosión por potencia de arroyo, pedogénesis por meteorización, interfaz con agentes.
- **Se abstrae (versión simplificada)**: el viento es un campo fijo por franja de latitud (no dinámica atmosférica real); la evapotranspiración es un proxy lineal con la temperatura; el suelo se resume en tres números derivados (capacidad de campo, permeabilidad, fertilidad).
- **Ampliación futura (marcado, no implementar ahora)**: corrientes oceánicas y clima marino; inercia térmica del mar sobre la costa; monzones y eventos meteorológicos estocásticos (sequías, tormentas); tectónica de placas; ajuste fino por orientación de ladera (insolación); química de suelos detallada; deriva climática de largo plazo (glaciaciones).

---

## 3. Entidades y componentes implicados (modelo ECS)

**Entidad principal:** la **celda** (a cualquier nivel del árbol). Cada celda es una entidad con los componentes de abajo.

**Estructura de árbol (no es una entidad física, es organización):** cada nodo del quadtree por cara guarda su nivel, sus 4 hijas (si está subdividida) y su madre.

### Componentes ESTÁTICOS (se fijan en world-gen; solo los tocan erosión/pedogénesis)
| Componente | Contenido |
|---|---|
| `Posicion` | cara c, índices (i,j), nivel L; latitud φ, longitud λ derivadas |
| `Geometria` | lado `d` de la celda, área real `A_celda` |
| `Elevacion` | altura `h` sobre un cero de referencia |
| `Pendiente` | `S` (magnitud) y `dir_bajada` (dirección D-infinito) |
| `Orientacion` | `asp` (hacia dónde mira la ladera) — reservado, uso futuro |
| `Suelo` | fracciones (arena, limo, arcilla); derivados `W_cap`, `k_perm`, `F0` |
| `Roca` | erodibilidad `k_ero_local` de la roca madre; profundidad de suelo `z_suelo` |

### Componentes DINÁMICOS (cada uno a su cadencia)
| Componente | Contenido |
|---|---|
| `AguaSuelo` | `W` (agua almacenada, extensivo) → deriva `θ = W/W_cap` |
| `Clima` | `T` (temperatura del tick), `P` (precipitación del tick) |
| `Nieve` | `SWE` (equivalente en agua de la nieve acumulada, extensivo) |
| `Vegetacion` | `V` (biomasa/densidad, extensivo), bioma etiquetado |
| `Fertilidad` | `F` (fertilidad efectiva actual, intensivo) |
| `Recursos` | depósitos minerales / recurso extraíble (extensivo) |
| `TipoSuperficie` | estado discreto: TIERRA / OCEANO / LAGO / RIO / HIELO |

### Entidades superpuestas (overlays)
- **Red de ríos** `RedRios`: un grafo dirigido de tramos (nodo = celda-río; arista = hacia dónde fluye). Cada tramo guarda caudal `Q`. No se guarda como estado pesado por celda, sino como red.
- **Cuencas de lago** `Cuenca`: para cada depresión, su volumen de agua `V_lago`, su cota de derrame `h_derrame` y la curva hipsométrica (relación profundidad-área-volumen) simplificada.

### Componente de interfaz con agentes
| `InterfazTerreno` (calculado bajo demanda, no almacenado) | Exporta: habitabilidad `H`, coste de movimiento `c_mov`, agua disponible `A_disp`, capacidad de alimento `K_alim` |

---

## 4. Variables de estado

| Símbolo | Significado | Tipo | Rango / Unidades |
|---|---|---|---|
| `t` | tick actual | entero | `ℕ` (adimensional) |
| `h` | elevación de la celda | real | metros (m); rango [-11000, 9000] típico |
| `S` | pendiente (magnitud) | real | [0, π/2] rad (o su tangente) |
| `asp` | orientación de ladera | real | [0, 2π) rad (0 = norte) |
| `φ`, `λ` | latitud, longitud | real | φ ∈ [-90, 90]°, λ ∈ [-180, 180]° |
| `d` | lado de la celda | real | m (depende del nivel L) |
| `A_celda` | área real de la celda | real | m² |
| `W` | agua almacenada en el suelo | real | mm (extensivo); [0, W_cap] |
| `θ` | humedad relativa del suelo | real | [0, 1] (intensivo); θ = W/W_cap |
| `T` | temperatura de la celda | real | °C; [-60, 55] típico |
| `P` | precipitación del tick | real | mm/tick; ≥ 0 |
| `SWE` | nieve acumulada (equiv. agua) | real | mm (extensivo); ≥ 0 |
| `V` | biomasa / densidad vegetal | real | [0, K] (extensivo, unidades rel.) |
| `K` | capacidad de carga vegetal | real | [0, K_max] |
| `F` | fertilidad efectiva | real | [0, 1] (intensivo) |
| `z_suelo` | profundidad del suelo | real | m; ≥ 0 |
| `Q` | caudal de un tramo de río | real | mm·m²/tick (volumen/tick); ≥ 0 |
| `V_lago` | volumen de agua de una cuenca | real | m³ (extensivo); ≥ 0 |
| `TipoSuperficie` | estado de la celda | discreto | {TIERRA, OCEANO, LAGO, RIO, HIELO} |
| `bioma` | bioma etiquetado | discreto | {desierto, pradera, bosque, selva, tundra, taiga, ...} |

---

## 5. Parámetros ajustables

Todos con valor por defecto y unidades. Ninguno se fija "a fuego" en la lógica; van a configuración. Los valores por defecto son un punto de partida coherente tipo Tierra, pensados para calibrar.

### Geometría y LOD
| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `R_planeta` | radio del planeta | 6371 | km |
| `N0` | celdas por arista de cara en L=0 | 128 | celdas |
| `L_max` | niveles máximos de subdivisión | 6 | — |
| `N_lod` | cadencia de revisión de subdivisión/colapso | 8 | ticks |
| `grad_sub` | umbral de gradiente para subdividir | 0.15 | (relativo) |

Relación de escala: lado de celda `d(L) = arista_cara / (N0 · 2^L)`, con `arista_cara ≈ (π/2)·R_planeta ≈ 10007 km`. Con los defectos: `d(0) ≈ 78 km`, `d(6) ≈ 1.2 km`.

### Clima y estaciones
| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `T_ecuador` | temperatura media en el ecuador a nivel del mar | 30 | °C |
| `k_lat` | caída de temperatura por grado de latitud | 0.6 | °C/° |
| `Γ` | gradiente térmico vertical (lapse rate) | 6.5 | °C/km |
| `A_est_polo` | amplitud estacional máxima (en el polo) | 25 | °C |
| `T_anio` | duración de un año | 48 | ticks |
| `T_cong` | umbral de congelación (lluvia→nieve) | 0 | °C |
| `k_fus` | tasa de fusión de nieve por grado | 2 | mm/(°C·tick) |
| `P_ref` | precipitación de referencia (banda húmeda) | 20 | mm/tick |
| `viento_franja` | dirección de viento dominante por franja de latitud | tabla | rad |

### Suelo y balance hídrico
| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `nivel_mar` | cota por debajo de la cual hay océano | 0 | m |
| `c_esc_base` | coeficiente base de escorrentía | 0.10 | — |
| `k_pend` | sensibilidad de la escorrentía a la pendiente | 0.5 | — |
| `k_sat` | sensibilidad de la escorrentía a la saturación | 0.4 | — |
| `k_et` | proxy de evapotranspiración por grado | 1.0 | mm/(°C·tick) |
| `k_lat_flow` | coeficiente de difusión lateral de humedad | 0.05 | — |
| `W_cap_min`, `W_cap_max` | capacidad de campo mín/máx según textura | 50 / 300 | mm |

### Vegetación
| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `θ_opt`, `σ_θ` | humedad óptima y tolerancia | 0.6 / 0.25 | — |
| `T_opt`, `σ_T` | temperatura óptima y tolerancia | 22 / 12 | °C |
| `K_max` | capacidad de carga máxima | 100 | (rel.) |
| `r_veg` | tasa de crecimiento vegetal | 0.2 | 1/tick |
| `N_veg` | cadencia del sistema de vegetación | 4 | ticks |

### Erosión y pedogénesis
| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `k_ero` | constante de erosión (potencia de arroyo) | 1e-5 | — |
| `m_ero`, `n_ero` | exponentes de área y pendiente | 0.5 / 1.0 | — |
| `k_cap` | constante de capacidad de transporte de sedimento | 2e-5 | — |
| `κ_dif` | difusión de ladera (soil creep) | 0.01 | m²/tick |
| `M_ero` | cadencia de erosión | 64 | ticks |
| `k_weath` | tasa de meteorización (formación de suelo) | 0.001 | m/tick |
| `z_ref` | profundidad de referencia de producción de suelo | 0.5 | m |
| `M_ped` | cadencia de pedogénesis | 256 | ticks |

### Semilla
| Símbolo | Significado | Defecto | Unidades |
|---|---|---|---|
| `semilla` | semilla global del generador determinista | 0 | entero |

### Constantes secundarias (introducidas en la Sección 6, también parametrizables)
Para respetar la regla "nada a fuego", la Sección 6 usa además estas constantes, todas con valor por defecto a fijar en configuración: parámetros del ruido de elevación (`oct=6`, `persistencia=0.5`, `lacunaridad=2.0`, `f_0`, `h_min=-11000 m`, `h_max=9000 m`); efecto orográfico (`k_oro=0.5`, `m_oro_min=0.2`, `m_oro_max=3.0`); `umbral_rio` (área de drenaje mínima para ser río); pesos del coste de movimiento (`c_base=1`, `k_c_pend=1.0`, `k_c_barro=0.5`, `k_c_veg=0.3`); agua disponible (`β_rio=0.5`, `β_lago=0.5`); pedogénesis/fertilidad (`aporte_veg`, `lixiviacion`, `uso_agricola`, `T_ref=25 °C`).

---

## 6. Reglas y ecuaciones por tick

Toda aleatoriedad proviene de un generador sembrado a partir de `semilla`. El tiempo es el entero `t`. Se describe primero la generación del mundo (una vez), luego cada sistema en su orden de ejecución.

**Convención de tasas (importante para la consistencia temporal).** Todas las constantes de tasa (`k_ero`, `k_weath`, `r_veg`, `κ_dif`, `k_lat_flow`) se expresan **por tick**, y en los sistemas de cadencia lenta se **multiplican por su cadencia** (`M_ero`, `M_ped`, `N_veg`) para obtener el cambio acumulado en el intervalo. Así, si se afina el tick (ver P1), basta reescalar las tasas de forma uniforme y la física se mantiene proporcional.

**Convención de unidades de volumen.** El agua se lleva internamente en mm de lámina de agua (mm = litros/m²). Al convertir a volumen absoluto para ríos y lagos se multiplica por el área: `volumen (m³) = (mm/1000) · A_celda (m²)`. Todos los términos de caudal y volumen deben reducirse a esta misma unidad antes de sumarse.

### 6.0 Generación del mundo (una sola vez, t = 0)

**a) Elevación por ruido coherente multi-octava (fractal).** Para el centro 3D `x⃗` de cada celda:
```
h(x⃗) = Σ_{k=0}^{oct-1} a_k · ruido( f_k · x⃗ , semilla )
con  a_k = persistencia^k ,  f_k = f_0 · lacunaridad^k
```
- `ruido(...)`: una función de ruido coherente (tipo gradiente/simplex), continua y determinista sobre la esfera. Da valores suaves en [-1, 1].
- `oct` (octavas): número de capas de detalle (defecto 6). Cada octava añade detalle más fino.
- `persistencia` (defecto 0.5): cuánto pesa cada octava sucesiva (<1, las finas pesan menos).
- `lacunaridad` (defecto 2.0): cuánto sube la frecuencia por octava.
- El resultado se normaliza y se reescala al rango [h_min, h_max].
- Como `h` es función pura de la posición y la semilla, subdividir una celda **recalcula la elevación fina exacta** (no se interpola): la elevación es coherente a cualquier nivel.

**b) Clasificación de superficie.** Si `h < nivel_mar` → `TipoSuperficie = OCEANO`; si no, `TIERRA` (provisional; lagos y ríos se marcan después).

**c) Pendiente y orientación.** A partir de las alturas de los vecinos (con distancias reales sobre la esfera), gradiente:
```
dz/dx ≈ (h_este − h_oeste) / (2·d)
dz/dy ≈ (h_norte − h_sur) / (2·d)
S    = arctan( √( (dz/dx)² + (dz/dy)² ) )        (pendiente)
asp  = arctan2( dz/dy , dz/dx )                   (orientación)
```
- `S` mide lo empinado. `asp` mide hacia dónde cae la ladera.

**d) Textura del suelo.** Se asignan fracciones (arena, limo, arcilla), sumando 1, mediante ruido sembrado sesgado por relieve (laderas empinadas → más pedregoso/arenoso; llanuras bajas → más limo/arcilla). De la textura se derivan:
```
W_cap = z_suelo · θ_fc(textura)          (capacidad de campo, en mm)
k_perm = permeabilidad(textura)          (arena alta, arcilla baja)
F0     = fertilidad_base(textura)        (limo/arcilla > arena)
```
- `θ_fc(textura)`: fracción de agua que el suelo retiene; arcilla alta, arena baja.

**e) Rellenado de hoyos y cuencas de lago.** Se detectan las depresiones (celdas sin vecino más bajo). Cada depresión se inunda hasta su **cota de derrame** `h_derrame` (la altura mínima del borde por la que rebosaría). Se registra una `Cuenca` con su volumen inicial y su curva hipsométrica. Las cuencas sin salida al mar son endorreicas.

**f) Red de ríos.** Se calcula la dirección de bajada (D-infinito) de cada celda de tierra y se acumula el flujo aguas arriba (`flow accumulation`), obteniendo el área de drenaje `A_drenaje` de cada celda. Las celdas con `A_drenaje > umbral_rio` pasan a `RIO` y se enlazan en la `RedRios` (grafo dirigido desde nacimientos hasta desembocaduras en mar o lago).

**g) Recursos.** Depósitos minerales colocados por un proceso sembrado tipo Poisson, sesgado por geología (roca madre). Versión mínima; ver Pregunta Abierta P4.

Todo lo anterior depende solo de `semilla`, así que **el mundo es reproducible**: misma semilla → mismo mundo.

### 6.1 Sistema de subdivisión/colapso (LOD adaptativo) — cada `N_lod` ticks

**Disparadores para subdividir una celda de nivel L < L_max:**
```
subdividir  ⟺  ( hay agentes en la celda )
             ∨ ( gradiente local de h, T o θ > grad_sub )
             ∨ ( la celda es o toca RIO / OCEANO / LAGO )
             ∨ ( hay un evento activo en la celda )
```

**Regla de subdivisión (crea 4 hijas del nivel L+1). Debe cumplir dos condiciones:**

1. **Datos estáticos → función pura de la posición.** La elevación, textura, etc. de cada hija se recalculan directamente del ruido sembrado en su posición fina. No se interpola desde la madre; se obtiene el valor "verdadero" fino.

2. **Datos dinámicos extensivos → reparto que conserva.** El agua `W`, la nieve `SWE`, la biomasa `V` de la madre se reparten entre las 4 hijas con pesos deterministas `w_i` que suman 1:
```
X_hija_i = X_madre · w_i ,   con  Σ_i w_i = 1
w_i = ρ_i / Σ_j ρ_j        (ρ_i = afinidad de la hija i, p.ej. su W_cap relativa)
```
   Las variables intensivas (θ, T) de las hijas se derivan de sus propios extensivos y capacidades.

**Regla de colapso (funde 4 hijas en la madre):**
```
Extensivos:  X_madre = Σ_i X_hija_i                       (suma exacta)
Intensivos:  Y_madre = (Σ_i A_i · Y_hija_i) / (Σ_i A_i)   (media ponderada por área)
```
- `A_i` = área real de la hija. La media ponderada por área garantiza que no se distorsione al colapsar celdas de área desigual.

**Ambas operaciones son funciones puras de (estado, posición, semilla).** Consecuencia clave: el resultado es **independiente del camino**. Que una zona haya estado fina todo el rato, o gruesa y luego fina, da resultados consistentes y reproducibles. Esto es lo que preserva el determinismo pese a que la resolución cambie sobre la marcha.

### 6.2 Sistema de clima — cada tick

**Temperatura.** Media local (por latitud y altura sobre el mar):
```
T_media(celda) = T_ecuador − k_lat·|φ| − Γ · max(0, (h − nivel_mar)/1000)
```
- El término de altura solo aplica a tierra emergida; `h` en m, `Γ` en °C/km (de ahí el /1000).

Oscilación estacional (sinusoidal), con hemisferios opuestos:
```
T(celda, t) = T_media(celda) + s(φ) · A_est(|φ|) · sin( 2π · t / T_anio )
A_est(|φ|) = A_est_polo · (|φ| / 90)
s(φ) = +1 si φ ≥ 0 (hemisferio norte) ,  −1 si φ < 0 (hemisferio sur)
```
- `A_est` crece con la latitud (estaciones marcadas en los polos, nulas en el ecuador).
- `s(φ)` invierte la fase: cuando el norte está en verano, el sur está en invierno.

**Precipitación.** Base por franja de latitud (húmedo en el ecuador, seco en subtrópicos ±30°, húmedo templado ±50°, seco polar) modulada por dos factores:
```
P(celda, t) = P_base(φ, dist_agua) · m_oro(celda) · m_est(φ, t)
```
- `P_base(φ, dist_agua)`: mapa base por franja de latitud y distancia a fuente de humedad (mar/lago); más lejos del agua, más seco.
- `m_oro(celda)` (efecto orográfico / sombra pluviométrica): se compara la dirección del viento dominante `viento_franja(φ)` con la pendiente. Si el terreno **sube** contra el viento (barlovento) → `m_oro > 1` (llueve más); si **baja** a sotavento → `m_oro < 1` (sombra seca).
```
m_oro = clamp( 1 + k_oro · (∇h · viento_unitario) , m_oro_min , m_oro_max )
```
- `m_est(φ, t)`: modulación estacional húmeda/seca, sinusoide análoga a la de temperatura (opcional; defecto suave).

### 6.3 Sistema de nieve — cada tick
```
si T(celda,t) < T_cong:   la precipitación cae como nieve:  SWE += P ;  P_liq = 0
si no:                    lluvia:  P_liq = P
Fusión:  si T > T_cong:   fusion = min( SWE , k_fus · (T − T_cong) )
                          SWE −= fusion
                          P_liq += fusion
```
- `P_liq` = agua líquida que llega al suelo este tick (lluvia + deshielo). Es la entrada del balance hídrico.
- Si `SWE > 0`, `TipoSuperficie` puede marcarse `HIELO` para la interfaz de agentes.

### 6.4 Sistema de balance hídrico (modelo de cubo) — cada tick

**Escorrentía (agua que resbala sin infiltrarse):**
```
c_esc = clamp( c_esc_base + k_pend·tan(S) + k_sat·(W/W_cap) , 0 , 1 )
R = P_liq · c_esc                         (escorrentía → va al agua superficial)
P_inf = P_liq − R                          (lo que se infiltra)
```
- Más pendiente y más saturación → más escorrentía.

**Evapotranspiración (agua que se pierde al aire):**
```
PET = k_et · max(0, T)                      (evapotranspiración potencial, proxy lineal)
ET  = min( PET , W + P_inf )                (real: no puede evaporarse más de lo disponible)
```
- (Nota: `PET` puede refinarse con Thornthwaite; ampliación futura.)

**Actualización del almacén y drenaje profundo:**
```
W_temp = W(t) + P_inf − ET
D = max( 0 , W_temp − W_cap )               (drenaje profundo → baseflow al agua superficial)
W(t+1) = clamp( W_temp − D , 0 , W_cap )
θ(t+1) = W(t+1) / W_cap
```

**Flujo lateral de humedad (difusión, casi isótropa) — opcional, tras lo anterior:**
```
Para cada par de vecinos i,j:
  q_ij = k_lat_flow · peso_ij · ( θ_i − θ_j )      (peso_ij = 1 ortogonal, 1/√2 diagonal)
  W_i −= q_ij·W_cap_i ;  W_j += q_ij·W_cap_i        (conservando el total)
```

La escorrentía `R` y el drenaje `D` de todas las celdas se acumulan y se pasan al sistema de agua superficial.

### 6.5 Sistema de agua superficial (mar, ríos, lagos) — cada tick

**Mar.** Nivel fijo `nivel_mar` (puede ser un parámetro global de deriva muy lenta; por defecto constante). Las celdas OCEANO son sumideros: absorben todo el caudal que les llega y son fuente infinita de humedad para el clima.

**Ríos.** El caudal de cada tramo es la suma de la escorrentía y el drenaje acumulados aguas arriba a lo largo de la red:
```
Q(tramo) = Σ_{celdas aguas arriba} ( R + D ) · A_celda
```
- Se propaga por la `RedRios` desde nacimientos hacia desembocadura. Un río desemboca en un lago o en el mar. El caudal alimenta la erosión (6.7) y da agua a las celdas colindantes (interfaz).

**Lagos.** Cada cuenca actualiza su volumen:
```
V_lago(t+1) = V_lago(t) + entra − sale − evap
entra = escorrentía + drenaje + caudal de ríos que entran a la cuenca
evap  = PET_lago · area_superficie_lago                     (evaporación del espejo de agua)
sale  = si V_lago > V_derrame:  el exceso rebosa por h_derrame y baja como río
        si no: 0
```
- La `area_superficie_lago` y la profundidad se obtienen del volumen mediante la curva hipsométrica de la cuenca. Un lago crece en época húmeda y encoge (o se seca) en sequía; las cuencas endorreicas nunca "salen", solo evaporan.

### 6.6 Sistema de vegetación — cada `N_veg` ticks

**Idoneidad** como producto de factores de tolerancia (curvas tipo campana, cada uno en [0,1]):
```
f_hum(θ)  = exp( −((θ − θ_opt)/σ_θ)² )
f_temp(T) = exp( −((T̄ − T_opt)/σ_T)² )        (T̄ = temperatura media reciente)
f_suelo(F)= F                                   (fertilidad, ya en [0,1])
Idoneidad = f_hum · f_temp · f_suelo
```
- Demasiado seco/húmedo o demasiado frío/cálido → factor bajo.

**Capacidad de carga y crecimiento logístico:**
```
K = K_max · Idoneidad
V(t+N_veg) = V + r_veg · V · (1 − V/K) · N_veg      (si K>0; si K→0, V decae hacia 0)
```

**Bioma etiquetado** (para salida/registro) según el diagrama de Whittaker, a partir de la temperatura media anual y la precipitación anual de la celda: desierto, pradera, sabana, bosque templado, selva, taiga, tundra, etc. Emerge de `T̄` y `P̄`, no se pinta.

### 6.7 Sistema de erosión y sedimentación — cada `M_ero` ticks

**Incisión por potencia de arroyo (stream power):**
```
E = k_ero · A_drenaje^m_ero · S^n_ero · M_ero        (material erosionado, en espesor)
```
- `A_drenaje`: área de drenaje (cuánta agua pasa). `S`: pendiente. Más caudal y más pendiente → más erosión. `M_ero` convierte la tasa "por tick" en el intervalo entre pasos.

**Transporte y depósito de sedimento.** Capacidad de transporte del flujo:
```
C_sed = k_cap · A_drenaje · S
si carga_sedimento > C_sed:  se deposita el exceso (Dep) aquí
si carga_sedimento < C_sed:  se erosiona más (hasta la disponibilidad de suelo)
```

**Difusión de ladera (soil creep), que suaviza el relieve, con laplaciano corregido:**
```
Δh_creep = κ_dif · ∇²h · M_ero
∇²h ≈ Σ_vecinos peso · (h_vecino − h) ,  peso = 1 ortogonal, 1/√2 diagonal (normalizado)
```

**Actualización de la elevación y realimentación:**
```
h(t+M_ero) = h − E + Dep + Δh_creep
```
- Tras modificar `h`, se **recalculan** pendiente, orientación y direcciones de flujo de las celdas afectadas, y periódicamente la `RedRios`. Aquí está el bucle vivo: erosión → cambia `h` → cambia `S` y el flujo → cambia la erosión. Los valles se tallan, las montañas se redondean.

### 6.8 Sistema de pedogénesis (formación de suelo) — cada `M_ped` ticks

**Meteorización (la roca se convierte en suelo).** Función de producción de suelo, más rápida cuando el suelo es fino y el clima cálido-húmedo:
```
tasa_prod = k_weath · f_clima(T, θ) · exp( −z_suelo / z_ref )
f_clima(T, θ) = clamp(T,0,∞)/T_ref · θ         (cálido y húmedo acelera; frío/seco frena)
z_suelo(t+M_ped) = z_suelo + (tasa_prod − erosion_de_suelo) · M_ped
```
- El suelo se forma más despacio a medida que engorda (`exp(−z/z_ref)`). Donde la erosión gana, aflora la roca (`z_suelo → 0`).

**Evolución de la fertilidad.** Aporte orgánico de la vegetación, pérdida por lavado (lixiviación) con lluvia alta, y consumo agrícola:
```
F(t+M_ped) = clamp( F + aporte_veg·V − lixiviacion·P̄ − uso_agricola , 0 , 1 )
```
- Como `W_cap = z_suelo · θ_fc`, al cambiar `z_suelo` cambia la capacidad de campo: suelo más profundo retiene más agua.

### 6.9 Interfaz con agentes — calculada bajo demanda cada tick

**Coste de movimiento** (lo caro que es cruzar la celda):
```
c_mov = c_base · (1 + k_c_pend·tan(S)) · (1 + k_c_barro·θ) · (1 + k_c_veg·(V/K_max))
```
- OCEANO/LAGO/RIO: coste muy alto o intransitable salvo reglas de cruce (puentes, barcos) definidas por el sistema de agentes.

**Habitabilidad** (lo bueno que es vivir aquí, en [0,1]):
```
H = f_hum(θ) · f_temp(T) · g_agua(A_disp)
```

**Agua disponible** (humedad + cercanía a agua superficial):
```
A_disp = θ + β_rio·(hay río adyacente) + β_lago·(hay lago adyacente)
```

**Capacidad de alimento:** `K_alim = V` (biomasa vegetal) o el recurso extraíble de la celda.

**Escritura de los agentes sobre el terreno (bucle de realimentación).** Las acciones de los agentes se **encolan durante el tick** y se **aplican al final del tick** (para no romper el determinismo por orden de proceso):
- Deforestar: `V → bajo`.
- Cultivar/regar: sube `θ` (riego) y consume `F` (`uso_agricola`).
- Urbanizar/sellar: `k_perm → 0`, `V → 0` (la ciudad impermeabiliza el suelo, aumentando la escorrentía).

---

## 7. Eventos discretos que genera (para registrar)

Cada evento se registra con: `tipo`, `tick`, `celda(s) o cuenca/tramo`, `datos`.
- **Subdivisión / colapso** de celda: nivel origen y destino.
- **Nacimiento / desaparición de río**: el caudal cruza `umbral_rio` al alza o a la baja.
- **Formación / desecación de lago**: `V_lago` cruza umbral por arriba/abajo.
- **Desbordamiento de lago**: la cuenca supera `V_derrame` y empieza a rebosar.
- **Cambio de bioma** de una celda: cruza una frontera de Whittaker.
- **Congelación / deshielo**: `SWE` aparece o desaparece (`TipoSuperficie` ↔ HIELO).
- **Evento de erosión notable**: `|Δh|` en un paso supera un umbral (deslizamiento, incisión fuerte).
- **Sequía / inundación local**: `θ` cruza extremos (≈0 o ≈1) mantenidos varios ticks.

---

## 8. Métricas agregadas que produce (por tick)

- Temperatura media global y por franja de latitud.
- Precipitación total y `θ` medio global.
- Superficie por bioma (nº de celdas o área real por tipo).
- Longitud total de ríos, nº de lagos, volumen total de agua superficial.
- Elevación media y tasa media de erosión.
- Cobertura de nieve/hielo (área con `SWE > 0`).
- Nº de celdas activas (subdivididas) por nivel → indicador de coste computacional.
- **Balance de agua global** (control de conservación): `entrada (lluvia) − salida (evaporación total) − flujo neto al océano ≈ 0` dentro de tolerancia numérica. Es la métrica de validación clave.

---

## 9. Interacción con otros sistemas y orden de ejecución dentro del tick

El orden importa porque cada sistema consume la salida del anterior:

```
1. LOD (subdivisión/colapso)      — cada N_lod ticks
2. Clima (T, P)
3. Nieve (SWE, P_liq)
4. Balance hídrico (θ; produce R y D)
5. Agua superficial (ríos, lagos; consume R y D)
6. Vegetación (V, bioma)          — cada N_veg ticks
7. Erosión y sedimentación (modifica h; recalcula S, flujo, RedRios) — cada M_ero ticks
8. Pedogénesis (z_suelo, F; recalcula W_cap) — cada M_ped ticks
9. Interfaz con agentes (exporta H, c_mov, A_disp, K_alim)
10. Aplicación de las escrituras encoladas por los agentes (al final del tick)
```

- La **erosión va después del agua** porque necesita el caudal/área de drenaje del tick.
- Tras erosión y pedogénesis se **recalculan los componentes estáticos derivados** afectados (pendiente, orientación, capacidad de campo).
- Los sistemas de **agentes** (que corren en su propia fase, fuera de este documento) leen la `InterfazTerreno` producida en el paso 9 y encolan escrituras que se aplican en el paso 10.

---

## 10. Validación y determinismo

**Determinismo:**
- `semilla` → world-gen reproducible. Misma `semilla` + mismos parámetros → corrida idéntica bit a bit.
- Toda aleatoriedad (ruido, colocación de recursos, perturbaciones) sale del generador sembrado. Jamás se usa el reloj de pared.
- Subdivisión/colapso son funciones puras → **independencia del camino** (test: subdividir y colapsar una región debe devolver un estado equivalente al de no haberla tocado, dentro de tolerancia numérica).

**Conservación (tests):**
- Balance de agua global cierra a tolerancia de coma flotante (métrica de la sección 8).
- El reparto en subdivisión y la suma en colapso conservan los extensivos (agua, nieve, biomasa).
- La ponderación por área en la media intensiva evita sesgos entre celdas de área desigual.

**Casos límite esperados:**
- Mundo plano (relieve nulo): sin ríos, franjas climáticas puras por latitud, sin sombra pluviométrica.
- Una única montaña: aparece sombra pluviométrica en la cara a sotavento; nieve en la cima si supera `T_cong`.
- Sin lluvia (`P_base=0`): `θ→0`, la vegetación muere, lagos y ríos se secan.
- Lluvia constante alta: lagos se llenan y desbordan, se forma red de ríos, θ→1.
- Esfera: los campos (T, θ, h) son **continuos al cruzar bordes de cara y polos**; el enrollado este-oeste no muestra discontinuidad.

**Estabilidad numérica (condiciones a respetar al calibrar):**
- **Aliasing estacional**: `T_anio ≥ 12` ticks (idealmente ≥ 24) para que la sinusoide de estaciones no salga a trompicones (mínimo de muestras por ciclo).
- **Difusión estable**: los coeficientes de difusión deben cumplir `coef · Δt / d² ≤ 1/2` aprox. (condición tipo CFL) para no oscilar. Aplica a `k_lat_flow` y `κ_dif`.
- **Balance no negativo**: los `clamp` y el `ET = min(PET, disponible)` impiden que el agua se vuelva negativa aunque el tick sea grande.

---

## 11. Preguntas abiertas (a cerrar antes o durante la implementación)

- **P1 — Calibración del tick (`ticks_por_año`).** El motor mantiene el tick abstracto (`t ∈ ℕ`), pero en cuanto conviven dos ritmos reales (evapotranspiración anual vs estaciones, erosión vs clima) hace falta un parámetro de calibración `T_anio` que los mantenga en proporción. Además, la escala actual (una vida ≈ 200 ticks → ~1 tick cada varios meses) es demasiado gruesa para estaciones: habrá que **afinar el tick** (subir mucho los ticks por año) o correr el clima en un sub-ciclo más fino. Decisión de modelo pendiente (tuya).
- **P2 — Viento.** ¿Campo de viento fijo por franja de latitud (lo asumido aquí) o algo más dinámico? Afecta al efecto orográfico.
- **P3 — Inercia térmica del mar.** ¿Suavizamos la temperatura de las costas por cercanía al agua? Marcado como futuro.
- **P4 — Recursos minerales.** Nivel de detalle de los depósitos: ¿solo "fertilidad + un recurso genérico" o varios minerales con geología?
- **P5 — Hipsometría de lagos.** ¿Curva profundidad-área-volumen simple (paramétrica) o derivada del relieve real de la cuenca?
- **P6 — Recomputar la red de ríos.** ¿Cada cuánto reconstruir `RedRios` tras la erosión, dado su coste? ¿Cada `M_ero`, o con menos frecuencia?
- **P7 — Corrientes oceánicas / clima marino.** Confirmado como ampliación futura.
- **P8 — Esquinas del cubo.** Cómo resolver exactamente las 8 esquinas donde concurren 3 caras (vecindad y área) sin artefactos.
- **P9 — Orientación de ladera en la temperatura.** ¿Activamos ya el ajuste por insolación (`asp`) o lo dejamos como futuro (por defecto, futuro)?
```