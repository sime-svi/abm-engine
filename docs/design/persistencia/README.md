# Fase «persistencia» — guardar los datos en una base de datos

> Explicación en lenguaje llano. El detalle técnico está en `design.md` (misma carpeta) y el esquema
> completo en `DATABASE_SCHEMA.md` (raíz del repo).

## En una frase

Hasta ahora la simulación escupía sus resultados a un archivo JSON. Ahora, además, puede **guardarlos
en una base de datos de verdad** (PostgreSQL con TimescaleDB), pensada para datos que van con el
tiempo, que puedes **consultar con SQL** y **pintar en Grafana** sin escribir nada de interfaz.

## Qué hemos conseguido

- El motor guarda cada ejecución en dos tablas:
  - **`simulation_runs`**: una fila por corrida (su nombre, semilla, versión del motor, la config
    entera, cuándo empezó y acabó, cuántos ticks). Es el "índice" de experimentos.
  - **`metrics`**: los números por tick (población, nacimientos, muertes, energía, recurso…), en
    formato "largo" (una fila por métrica y tick), listos para graficar.
- Puedes tener **muchas ejecuciones guardadas a la vez** y compararlas (cada una con su `run_id`).
- Eliges el destino en el `config`: `sink = "json"`, `"postgres"` o `"both"` (ambos).

## Palabras técnicas, traducidas

- **TimescaleDB / hypertable**: TimescaleDB es un añadido a PostgreSQL para datos temporales. Una
  *hypertable* es una tabla que por dentro se parte en trozos por tiempo (aquí, por `tick`) para que
  insertar y consultar sea rápido aunque haya millones de filas. Por fuera la usas como una tabla
  normal.
- **Migración**: un archivo `.sql` numerado que crea o cambia tablas. En vez de tocar la BD a mano se
  escribe una migración; así el esquema queda versionado y reproducible. El binario las aplica solo al
  arrancar (y anota cuáles ya aplicó, para no repetirlas).
- **Sink**: el "desagüe" o destino de los datos. Cambiar de JSON a base de datos es cambiar el sink,
  sin tocar el motor.

## Cómo probarlo

```bash
# 1) Levanta la BD si no está corriendo:   cd infra && docker compose up -d
# 2) Pon en el config [persistence] sink = "postgres" (o "both") y ejecuta desde la raíz:
cargo run --release -- config/default.toml
# (el binario lee infra/.env solo; no hace falta exportar nada)
```

El binario imprime el `run_id`. A partir de ahí, Grafana (http://localhost:23000 (puerto cambiado el 2026-11-04; antes 3000)) puede leer las tablas
`simulation_runs` y `metrics` para hacer gráficas.

## Consideraciones

- La **conexión no vive en el código ni en la config** (son credenciales): se toma de variables de
  entorno (`DATABASE_URL` o `POSTGRES_*`), que están en `infra/.env` (ignorado por git).
- Por defecto el sink es **`json`**, para que los tests no dependan de tener una BD encendida.
- De momento se guardan **métricas agregadas**. El detalle por agente (*snapshots*) y los eventos son
  fases siguientes; el esquema ya los tiene diseñados en `DATABASE_SCHEMA.md`.
