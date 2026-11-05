# Herramientas

Programas fuera del motor para verificar, medir y consultar las corridas (CLAUDE §2.1: pueden usar Python o bash). Las métricas que sostienen una decisión salen de aquí o de la propia salida JSON, nunca de cálculos sueltos (CLAUDE §5).

| Herramienta | Para qué |
|---|---|
| `verificar.sh` | Toda la verificación de CLAUDE §10 en un comando: formato, clippy, tests, batería de escenarios y, con `--no-regresion REF`, no regresión byte a byte frente a un commit |
| `escenarios.py` + `escenarios.toml` | Batería de escenarios de referencia: corridas fijas con unas pocas métricas clave, sus márgenes y su presupuesto de tiempo |
| `no_regresion.sh` | Compara byte a byte la salida actual con la de un commit anterior (para los cambios que no deben alterar nada) |
| `analisis/agentes.py` | Resume una corrida de agentes: población frente a `N_max`, fecundidad, esperanza de vida, hambre, mortalidad por sexo |
| `analisis/terreno.py` | Resume y compara corridas del planeta: biomas, diagnóstico, `c_P` y, con el volcado por celda, la sombra pluviométrica |

## Ajustes desde la línea de comandos

Los binarios `agentes` y `terreno` aceptan `--set ruta=valor` (varias veces), que cambia la config sin editar el archivo:

```bash
target/release/terreno --set climate.k_oro=60 --set run.name=k60
target/release/agentes --set run.total_ticks=36500 --set habitat.sequia.inicio_dia=7300 \
                       --set habitat.sequia.duracion_dias=365 --set habitat.sequia.factor=0.5
target/release/agentes --set 'especies=[{ nombre = "humano", preset = "humano" }]'
```

- La ruta va por puntos, y un número elige un elemento de una lista: `especies.1.ajustes.p_conc`.
- El valor es un literal TOML. Si no lo es, se toma como texto.
- Una clave mal escrita da error (las configs rechazan los campos desconocidos).

## Informe de tiempos por sistema

Cada corrida de `agentes` y `terreno` escribe, aparte de su salida, `output/<nombre>_tiempos.json`: cuánto tarda cada sistema del tick, en total, como fracción del tiempo y por vez. La consola muestra las etapas que más tardan. No forma parte de la salida del modelo, porque cambia de una corrida a otra (CLAUDE §6), y `no_regresion.sh` lo ignora.

Primera medida (2026-11-05):
- **Planeta, un año:** el LOD se lleva el 53 % del tiempo, la difusión lateral el 20 % y el clima el 11 %.
- **Agentes, 100 años:** la demanda se lleva el 32 %, la energética el 22 % y las métricas el 17 %.

## Diagnóstico del planeta en la salida

Desde el esquema v5, la salida del planeta (`<nombre>_terreno.json`) trae un bloque `diagnostico`, con las cifras que se comparan con la Tierra:
- tierra emergida;
- altura media y mediana de la tierra, y la curva hipsométrica por franjas;
- profundidad media del océano;
- lluvia sobre tierra y sobre el mar;
- tierra con el aire seco (`w` < 0,5);
- PPN de Miami.

La sombra pluviométrica (sotavento y barlovento de las cordilleras) se mide aún con `analisis/terreno.py` sobre el volcado por celda (`run.dump_world` y `run.dump_climate`).

## La batería de escenarios

```bash
herramientas/escenarios.py            # compila y pasa todo (~1 min)
herramientas/escenarios.py --listar
herramientas/escenarios.py --solo planeta,humanos_hambruna
```

Escribe las salidas en `output/escenarios/` y el informe en `output/escenarios/informe.json`. Por cada métrica guarda el valor, el margen y si pasa; también el tiempo y las etapas que más tardan.

**Escenarios:**
- **`planeta`:** el planeta por defecto (relieve, lluvia, biomas, ríos, sombra pluviométrica, conservación del agua).
- **`agentes_equilibrio`:** 100 años de agentes (población frente a `N_max`, fecundidad humana, esperanza de vida, balance de energía).
- **`agentes_sequia`:** una sequía al 50 % durante un año.
- **`humanos_hambruna`:** solo humanos, hambruna al 30 %.

**Márgenes.** Rodean el valor actual: avisan cuando un cambio mueve algo que emerge al juntar sistemas, no son objetivos de realismo. Si un cambio intencionado saca una métrica de su margen:
1. se documenta en la nota de diseño del cambio;
2. se actualiza el margen en `escenarios.toml`, con su nota.

Cada nota dice qué vigila la métrica y, cuando se sabe, el valor real de referencia. Los valores irreales conocidos están marcados como pendientes, como la tierra emergida o la caída del herbívoro en la sequía.

**Añadir una métrica:**
1. si no existe, escribir su función en `MEDIDAS` (`escenarios.py`), apoyándose en `analisis/`;
2. añadir un `[[escenario.metrica]]` con su margen y su nota.
