# Fase «inicial» — el primer motor mínimo

> Explicación en lenguaje llano de qué se logra en esta fase. El diseño técnico y las fórmulas
> exactas están en [`minimal-model.md`](./minimal-model.md), en esta misma carpeta.

## En una frase

Un mundo con forma de cuadrícula (como un tablero) donde muchos **agentes** (bichitos) buscan comida,
la comen, se reproducen y mueren; y el programa **anota en un archivo** lo que va pasando, número a
número.

## Qué hemos conseguido

- Un **motor de simulación** que funciona y **produce datos** (no dibuja nada en pantalla: eso es lo
  que significa *headless*, «sin cabeza», sin interfaz gráfica).
- Que sea **reproducible**: si arrancas con la misma *semilla* (el número que inicializa el azar),
  obtienes **exactamente** el mismo resultado, bit a bit. Esto es lo que convierte la simulación en
  algo científico y no en una casualidad.
- Que **todo se controle desde un archivo de texto** (`config/default.toml`): el tamaño del tablero,
  cuántos bichitos, cuánta comida, etc. Se experimenta cambiando ese archivo, **sin tocar el código**.

## Por qué importa (la «pirámide invertida»)

Construimos primero lo más pequeño que funcione bien, y solo subimos de nivel cuando esa base es
sólida. Esta fase es esa base: todo lo que venga después (base de datos, más detalle, más escala) se
apoya en que esto sea **estable y reproducible**. Y lo es.

## Cómo funciona, paso a paso

El tiempo avanza en **ticks** (un *tick* es un paso de tiempo, como un latido del reloj de la
simulación; es un número entero, no el reloj de verdad). En cada tick, para cada bichito y en este
orden:

1. **Crece la comida** en cada casilla (hasta un máximo).
2. **Se mueve** a la casilla vecina con más comida que esté libre (o se queda donde está).
3. **Come** lo que puede de esa casilla.
4. **Gasta energía** (metabolismo) y **envejece** un poco.
5. **Se reproduce** si tiene bastante energía: nace un hijo en una casilla libre al lado, y la madre
   le cede parte de su energía.
6. **Muere** si se queda sin energía o si es demasiado viejo.

Al terminar el tick se apuntan los totales del momento (cuántos hay, energía media, comida total,
nacimientos, muertes).

## Tres palabras técnicas, traducidas

- **ECS** (usamos la librería `bevy_ecs`): una forma de guardar los datos de los agentes en **listas
  paralelas** (una para las posiciones, otra para las energías…) en vez de en «objetos». Va más
  rápido porque la CPU lee memoria seguida.
- **Determinismo**: mismo punto de partida → mismo final, exacto. Lo cuidamos mucho (un solo hilo de
  ejecución, orden fijo de todo, y el azar sembrado y controlado).
- **LOD** (*Level of Detail*, nivel de detalle): guardar solo lo que vale la pena. Aquí guardamos los
  **totales por tick**, no la ficha de cada bichito en cada instante (eso sería una montaña de datos
  inútil).

## Consideraciones (lo que conviene saber)

- **Dinámica sana de «capacidad de carga».** Con los valores por defecto la población hace un *boom*
  inicial (de ~300 a ~5000 en los primeros 100 ticks) y luego se **estabiliza** en torno a la mitad
  del tablero (~5000), con nacimientos y muertes ocurriendo sin parar: es un equilibrio **vivo**, no
  un número congelado. Hacia el tick 200 verás un pico de muertes —la primera generación nació a la
  vez, envejece a la vez y muere casi de golpe— y después el sistema se reequilibra solo.
- **La clave del ajuste** (por si tocas el `config`): el *metabolismo* (lo que un agente gasta por
  tick, `2.0`) es **mayor** que lo que una casilla regenera de comida (`1.0` por tick). Por eso, cuando
  se juntan demasiados, la comida no da abasto y algunos pasan hambre y mueren; eso es justo lo que
  impide que llenen todo el tablero y lo que crea el equilibrio. La regla es `regeneración < metabolismo
  < apetito`: si el metabolismo igualara a la regeneración, la población saturaría el tablero y se
  congelaría; si igualara al apetito, no podría crecer ni en la mejor casilla.
- **Todavía no hay base de datos.** Los resultados salen a archivos **JSON** en la carpeta `output/`.
  La base de datos (PostgreSQL) llega en una fase posterior.
- Es **un único programa**, aún no la estructura final de cinco «cajas» de código (*crates*). Pero ya
  está organizado por dentro para que ese cambio sea sencillo.
- **Fuera de alcance por ahora:** fotos detalladas de agentes (*snapshots*), eventos individuales, la
  base de datos, y correr en paralelo.

## Para trastear

```bash
# Ejecuta la simulación (genera output/baseline.json)
cargo run --release -- config/default.toml

# Comprueba los tests (incluye el de reproducibilidad)
cargo test
```

Cambia valores en `config/default.toml` y vuelve a ejecutar para ver cómo cambia el comportamiento.
