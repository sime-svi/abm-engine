# Nota de diseño: hundimiento térmico de los márgenes (McKenzie)

Documento de diseño. Complementa a nota_isostasia_litosfera.md (regla R4) y a adenda_balance_corteza.md (§6.10: estiramiento de los márgenes).

Estado: diseño cerrado salvo las preguntas del §11.

---

## 1. Fenómeno real y objetivo

Cuando un continente se estira y se rompe, la litosfera de la zona estirada se adelgaza y sube manto caliente y fértil a ocupar su sitio. El margen se hunde en dos fases (McKenzie 1978):
1. **Al estirarse:** la corteza adelgazada flota menos. El modelo ya lo reproduce: unos 1,8 km al romper.
2. **Después, durante decenas de Myr:** el manto caliente se enfría, se contrae y el margen sigue hundiéndose, como el fondo oceánico joven. Sobre esa cuenca que se hunde se acumula el sedimento y se forma la plataforma continental.

En el modelo, la segunda fase casi no existe: 0,38 km en 100 Myr, frente a unos 2 km para un estiramiento de 3. El motivo es que el manto litosférico de R4 es el de los continentes viejos, empobrecido y ligero, y no el manto fértil que sube bajo un rift.

**Objetivo:** que los márgenes estirados se hundan con la curva de McKenzie según su estiramiento, sin cambiar la regla R4 en el resto de los continentes.

---

## 2. Alcance y nivel de detalle

Sí se modela:
- El hundimiento térmico tras la rotura, con el primer término de la solución de McKenzie, según el estiramiento de cada celda.
- Su encaje con R4, sin contar dos veces lo que R4 ya hunde.

Se abstrae o se deja para más adelante:
- **Estiramiento distinto de la corteza y del manto** (estiramiento no uniforme).
- **Calentamiento durante el propio rift**, que puede levantar los hombros del rift.
- **Márgenes volcánicos**, con mucho magma añadido al romper.

---

## 3. Entidades y componentes

Nuevos datos por celda de la franja estirada:
- β, el factor de estiramiento;
- t_r, el tiempo desde la rotura.

---

## 4. Variables de estado

| Símbolo | Significado | Tipo | Rango / unidades |
|---|---|---|---|
| β_i | Factor de estiramiento de la celda al romper: β = h_0 / h_c, con h_0 el grosor antes del rift (adenda §6.10). Es 1 fuera de los márgenes | real | 1 – h_0/h_rot |
| t_r,i | Tiempo desde la rotura | real | Myr |
| S_McK,i | Hundimiento térmico de McKenzie con agua encima | real | km |
| S_R4,i | Hundimiento con agua encima que R4 ya produce desde la rotura, por el crecimiento de L(t) | real | km |
| Δm_i | Masa añadida al manto de la columna | real | kg/m² |

---

## 5. Parámetros ajustables

| Parámetro | Significado | Por defecto | Unidades | Origen |
|---|---|---|---|---|
| E_0 | Escala del hundimiento térmico | 3,2 | km | Medido/físico: McKenzie (1978), con a = 125 km, ρ = 3 330 kg/m³, α = 3,28·10⁻⁵ /°C y T = 1 333 °C. Rango 2,4–3,2; el límite inferior es el hundimiento total del fondo oceánico del modelo (2,37 km) |
| τ | Tiempo característico del enfriamiento | 62,8 | Myr | Físico: a²/(π²·κ), con κ = 8·10⁻⁷ m²/s (McKenzie 1978). Rango 50–63 |

---

## 6. Reglas por paso tectónico

### 6.1 Al romper un rift

Cada celda de la franja estirada guarda β_i y pone t_r,i = 0.

### 6.2 En cada paso

t_r,i ← t_r,i + Δt, y el hundimiento de McKenzie, con agua encima, es:

S_McK,i = E_0 · (β_i / π) · sin(π / β_i) · (1 − e^(−t_r,i / τ))

Ejemplos con los valores por defecto: con β = 3, unos 2,1 km a los 100 Myr y unos 2,65 km al final; con β = 1,5, unos 1,05 km a los 100 Myr y unos 1,3 km al final.

### 6.3 Encaje con R4 (sin contar dos veces)

R4 ya hunde algo el margen al crecer L(t) desde la rotura (S_R4). Solo se añade lo que falta:

Δm_i = max(0, S_McK,i − S_R4,i) · (ρ_m − ρ_w)

Esa masa se suma al manto de la columna en la isostasia R4. Así, la isostasia calcula la altura correcta con cualquier relleno encima, sea agua, sedimento o aire, y el sedimento hunde además la cuenca con su propio peso.

### 6.4 Fin del efecto

Si la celda sufre después un nuevo evento que reinicia su edad tectonotérmica (arco, choque o delaminación), se pone β_i = 1 y el término desaparece: el nuevo evento manda.

### 6.5 Mezcla

Si en una celda se junta corteza de procedencias distintas, se conservan β y t_r de la parte con más volumen.

---

## 7. Eventos discretos

No genera eventos nuevos.

---

## 8. Métricas por paso

- Hundimiento del basamento de los márgenes, con agua encima, por tiempo desde la rotura y por clase de β.
- Anchura media de la plataforma continental.

---

## 9. Interacción con otros sistemas y orden dentro del paso tectónico

- β y t_r se guardan al romper el rift (adenda §6.10).
- El término se aplica dentro de la isostasia R4, en su mismo lugar del paso.
- Viajan con la placa como el resto de datos de la celda.

---

## 10. Validación y determinismo

| Magnitud | Objetivo | Hoy |
|---|---|---|
| Rift aislado (prueba): hundimiento del basamento con agua encima, por tiempo desde la rotura | La curva de McKenzie para su β, ± 15 % | — |
| Planeta: hundimiento térmico de los márgenes distales en ~100 Myr | ~2 km para β ≈ 3 (2–3 km) | 0,38 km |
| Plataforma | no peor que R4 con la Tierra real (33 km); objetivo 43 | 9,7 km |
| Tierra por debajo de 200 m | no peor que R4 con la Tierra real (22 %); objetivo 28 % | 16,2 % |

Pruebas:
1. **Rift aislado:** sigue la curva de McKenzie para su β.
2. **No regresión:** una celda sin estirar (β = 1) da la misma altura que hoy.
3. **Determinismo:** misma semilla, mismo resultado.

---

## 11. Preguntas abiertas

1. ¿Cómo se calcula S_R4 exactamente, a partir del cambio del término del manto de R4 desde la rotura?
2. ¿Se conserva bien el grosor previo h_0 de cada celda para calcular β?

---

## Referencias

- McKenzie, D. (1978). Some remarks on the development of sedimentary basins. Earth and Planetary Science Letters 40.
