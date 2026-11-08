# Estudio: por qué la tierra sale demasiado alta (isostasia continental)

Tipo: protocolo de estudio previo al diseño. No cambia el motor. Lo hace Claude Code con herramientas de análisis y datos reales; el resultado vuelve aquí, y de él sale la nota de diseño.

---

## 0. Por qué un estudio y no un arreglo directo

La prueba con CRUST1.0 demostró que la regla de flotación del modelo falla incluso con la corteza real de la Tierra. Predecir la altura de los continentes es un problema muy estudiado en geofísica, y hay modelos publicados que lo resuelven bastante bien. Por eso se sigue el método científico:

1. reproducir primero un resultado publicado con nuestros propios datos y herramientas;
2. comparar varias reglas candidatas, de la más sencilla a la más completa;
3. validar la elegida con datos que no se usaron para ajustarla;
4. solo después, diseñar e implementar la elegida.

---

## 1. Lo que dice la ciencia

**Isostasia de toda la litosfera** (Lamb et al. 2020). La altura de un continente no depende solo de su corteza, sino también del manto litosférico, la capa de manto rígido pegada debajo de la corteza.
- El manto litosférico es más denso que la corteza, en unos 300–550 kg/m³.
- Es solo un poco más denso que la astenosfera, el manto blando sobre el que flota, en unos 20–40 kg/m³ netos. Es frío, lo que lo hace más denso, pero también está empobrecido en hierro, lo que lo aligera.
- Con estos valores, la variación del grosor de la litosfera controla, junto con la corteza, la altura de los continentes. Las alturas del planeta se explican sin topografía dinámica (empujes del manto profundo) por encima de unos cientos de metros.
- Sus datos están publicados en Zenodo (registro 4031646).

**Isostasia térmica** (Hasterok y Chapman 2007). En Norteamérica, la temperatura de la litosfera explica cerca de la mitad de la altura de cada lugar, y el resto lo explican sobre todo la composición y el grosor. La meseta del Colorado (~1 800 m) y las Grandes Llanuras (~300 m) tienen rocas parecidas en profundidad; la diferencia es que la base de la corteza está más caliente bajo la meseta.

**Grosor de la litosfera según la edad** (Artemieva 2006, modelo TC1). El grosor crece con la edad del último evento tectonotérmico, es decir, el último rift, arco u orogenia que calentó la zona: z ≈ 0,04 · t + 93,6 km, con t en Myr. En los cratones arcaicos llega a unos 200 km, y en algunos a más de 250.

**Hundimiento de los márgenes tras un rift** (McKenzie 1978). Al estirarse, la litosfera se adelgaza y se calienta; después, al enfriarse, se hunde de forma parecida al fondo oceánico. Ese hundimiento crea las cuencas y las plataformas, que luego rellena el sedimento. El sedimento no levanta la tierra: ocupa el hueco que deja la litosfera al hundirse.

---

## 2. Datos

- ETOPO 2022 sin hielo y con rebote (ya en el repositorio).
- CRUST1.0: grosor y densidad de la corteza y del sedimento (ya en el repositorio).
- Grosor de la litosfera: TC1 (Artemieva, rejilla de 1°) y LITHO1.0 (Pasyanos et al. 2014).
- Edad tectonotérmica de cada zona: las provincias de TC1.
- Datos de Lamb et al. 2020 (Zenodo 4031646).

---

## 3. Reglas candidatas

Todas se escriben como el equilibrio de una columna de litosfera. Para una columna con corteza h_c (densidad media ρ_c), sedimento h_s (ρ_s) y manto litosférico h_m = L − h_c − h_s (densidad neta ρ_a + Δρ_ma), la altura sobre el nivel del mar, si sale positiva, es:

H = [ (ρ_a − ρ_c) · h_c + (ρ_a − ρ_s) · h_s − Δρ_ma · h_m ] / ρ_a − H_0

Bajo el mar, la profundidad se multiplica por ρ_a / (ρ_a − ρ_w), porque el agua también pesa. H_0 es la constante de referencia, que se fija con el nivel del mar.

| Regla | Qué cambia | Parámetros |
|---|---|---|
| R0 | La regla actual: solo corteza y sedimento, con las densidades del modelo (Δρ_ma = 0) | Ninguno nuevo. Es la línea base |
| R1 | R0 con la densidad de la corteza creciendo con la profundidad: superior ~2 700–2 750 kg/m³, inferior ~2 900–3 000; media mundial ~2 830 (Christensen y Mooney 1995) | Densidades por capa, dentro de los rangos publicados |
| R2 | Litosfera completa con el grosor L observado (TC1 o LITHO1.0) | Δρ_ma en 20–40 kg/m³ |
| R3 | Como R2, pero con L calculada a partir de la edad tectonotérmica, que es lo que el motor puede llevar. Se prueban dos formas: (a) la recta de Artemieva; (b) la recta más una recuperación rápida tras el evento, L(t) = L_min + (L_lin(t) − L_min) · (1 − exp(−t / τ_L)) | L_min, τ_L y Δρ_ma, dentro de rangos físicos |
| R4 | R3 con la densidad de la corteza de R1, solo si R1 aporta | Las de R1 y R3 |

Variante opcional de R3: Δρ_ma que también depende de la edad (manto más caliente, menos denso, en zonas jóvenes), por si L(t) sola no basta.

---

## 4. Protocolo

1. **Reproducir lo publicado.** Aplicar R2 con los datos de Lamb y comparar con su resultado. Si no se parece, el fallo está en nuestra implementación, y se corrige antes de seguir.
2. **Evaluar cada regla con los datos reales**, sin el planeta simulado. Se calcula la altura de cada celda y se compara con ETOPO:
   - la tabla de relieve: altura media de la tierra, tierra por debajo de 200 m y por encima de 2 000 m, pendiente a la misma resolución y anchura de la plataforma;
   - el error medio por celda;
   - el error por tipo de provincia: escudo arcaico, plataforma proterozoica, cinturón fanerozoico, rift o zona joven, orógeno activo y margen pasivo;
   - casos con nombre: llanura rusa (~110 m), Kenia (~2 260 m), Tíbet (~4,9 km), meseta del Colorado y Grandes Llanuras.
3. **Ajustar solo lo que no tiene valor medido.** Se ajustan H_0 y, dentro de sus rangos publicados, Δρ_ma, las densidades y las constantes de L(t). Validación cruzada: se ajusta con la mitad de los continentes (Eurasia, África y Australia) y se comprueba con la otra mitad (América y la Antártida sin hielo), y después al revés. Si el error en la mitad de comprobación es mucho mayor que en la de ajuste, la regla está sobreajustada: se ha adaptado a los datos en lugar de explicarlos.
4. **Elegir la regla más sencilla** que cumpla el §5 y que use solo variables que el motor puede calcular. Si R2 cumple pero R3 no, el problema está en cómo se calcula L a partir de la edad, y se mejora eso; no se usan datos fijos de la Tierra.
5. **Repartir el error de R0.** Medir qué parte del error de la regla actual viene de no tener manto litosférico, qué parte de la densidad de la corteza y qué parte del sedimento.

---

## 5. Criterios de aceptación (con los datos reales)

| Magnitud | Objetivo |
|---|---|
| Altura media de la tierra | 671 ± 100 m |
| Tierra por debajo de 200 m | 28 ± 5 % |
| Tierra por encima de 2 000 m | 5,3 ± 2 % |
| Plataforma | al menos la mitad de la de ETOPO, a la misma resolución |
| Error medio por celda de la regla elegida (R3 o R4) | como mucho un 30 % peor que R2 con L observado |
| Casos con nombre | llanura rusa por debajo de 400 m; Kenia por encima de 1 200 m; Tíbet entre 4 y 5,5 km |

Lo que ninguna regla explique se atribuye a la topografía dinámica, hasta unos cientos de metros, y va a ampliación futura.

---

## 6. Qué no hacer

- Mover densidades u otros valores medidos fuera de sus rangos publicados.
- Elegir una regla que necesite datos que el motor no puede producir.
- Tocar el motor durante este estudio.

---

## 7. Después del estudio

Con la regla elegida se escribe la nota de diseño. Tendrá que decir:
- qué lleva cada celda: edad tectonotérmica y, si hace falta, el grosor de la litosfera;
- qué reinicia el reloj: rifts, arcos, choques y delaminación;
- el hundimiento de los márgenes tras un rift;
- el estado inicial: con qué edades empiezan los continentes, con una distribución parecida a la de la Tierra, con cratones viejos;
- qué hay que recalibrar después: la erosión (E_1 y n_ero) y el nivel del mar, que se calibraron sobre la regla actual.

---

## Referencias

- Artemieva, I. M. (2006). Global 1°×1° thermal model TC1 for the continental lithosphere: implications for lithosphere secular evolution. Tectonophysics 416.
- Christensen, N. I. y Mooney, W. D. (1995). Seismic velocity structure and composition of the continental crust: a global view. Journal of Geophysical Research 100.
- Hasterok, D. y Chapman, D. S. (2007). Continental thermal isostasy: 1. Methods and sensitivity; 2. Application to North America. Journal of Geophysical Research 112.
- Lamb, S., Moore, J. D. P., Perez-Gussinye, M. y Stern, T. (2020). Global whole lithosphere isostasy: implications for surface elevations, structure, strength, and densities of the continental lithosphere. Geochemistry, Geophysics, Geosystems 21.
- McKenzie, D. (1978). Some remarks on the development of sedimentary basins. Earth and Planetary Science Letters 40.
- Pasyanos, M. E. et al. (2014). LITHO1.0: an updated crust and lithospheric model of the Earth. Journal of Geophysical Research 119.
