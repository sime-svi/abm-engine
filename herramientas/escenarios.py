#!/usr/bin/env python3
"""Batería de escenarios de referencia (CLAUDE §10.5; punto 3 de escalabilidad).

Cada escenario es una corrida fija: un binario, su config de `config/` y unos ajustes `--set`. De su
salida se sacan unas pocas métricas clave, que vigilan lo que emerge al juntar sistemas, y se
comprueba que caen dentro de sus márgenes. Los escenarios, las métricas, los márgenes y el porqué de
cada uno están en `herramientas/escenarios.toml`. Ahí se cambian, con una nota, cuando un cambio
intencionado del modelo mueve una métrica.

No exige salidas idénticas byte a byte: para eso está `herramientas/no_regresion.sh`.

Además vigila el presupuesto de tiempo de cada escenario (punto 5). Es el tiempo de simulación que
mide el propio binario en `<nombre>_tiempos.json`, y depende de la máquina: los presupuestos dejan
margen.

Uso:
  herramientas/escenarios.py                  # compila en release y pasa toda la batería
  herramientas/escenarios.py --solo planeta   # solo algunos (separados por comas)
  herramientas/escenarios.py --listar         # lista los escenarios
  herramientas/escenarios.py --sin-compilar   # usa los binarios ya compilados
  herramientas/escenarios.py -j 1             # de uno en uno (tiempos más limpios)

Escribe las salidas en `output/escenarios/` y el informe en `output/escenarios/informe.json`. Sale con
0 si todo está dentro de márgenes y presupuestos, con 1 si algo no lo está y con 2 si falla una
corrida.
"""
import argparse
import functools
import json
import math
import os
import subprocess
import sys
import time
import tomllib
from concurrent.futures import ThreadPoolExecutor

RAIZ = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(RAIZ, 'herramientas', 'analisis'))
import agentes as an_a  # noqa: E402
import terreno as an_t  # noqa: E402

SALIDA = 'output/escenarios'


class Salida:
    """La salida de un escenario, con sus documentos cargados una sola vez."""

    def __init__(self, nombre):
        self.dir = os.path.join(RAIZ, SALIDA)
        self.nombre = nombre

    @functools.cached_property
    def agentes(self):
        return an_a.cargar(self.dir, self.nombre)

    @functools.cached_property
    def terreno(self):
        return an_t.analizar(self.dir, self.nombre)

    @functools.cached_property
    def tiempos(self):
        with open(f'{self.dir}/{self.nombre}_tiempos.json') as f:
            return json.load(f)


def _pob(s, especie):
    return an_a.serie_poblacion(s.agentes, especie)


# Cada medida recibe la salida y los parámetros de la métrica en escenarios.toml.
MEDIDAS = {
    # Planeta (salida JSON; las de sotavento y barlovento necesitan el volcado por celda).
    'terreno.valor': lambda s, campo: s.terreno[campo],
    'terreno.bioma_pct': lambda s, bioma: s.terreno['biomas_pct'].get(bioma, 0.0),
    'terreno.hipsometria_frac': lambda s, franja: s.terreno['hipsometria_frac'][franja],
    'terreno.sotavento': lambda s, campo: s.terreno['sotavento'][campo],
    'terreno.resto_latitudes_medias': lambda s, campo: s.terreno['resto_latitudes_medias'][campo],
    # Agentes.
    'agentes.resumen': lambda s, especie, campo, desde=0, hasta=None:
        an_a.resumen(s.agentes, especie, desde, hasta)[campo],
    'agentes.fecundidad_rel': lambda s, especie, grupo, desde=0, hasta=None:
        an_a.resumen(s.agentes, especie, desde, hasta)['fecundidad_rel'][grupo],
    'agentes.mortalidad': lambda s, especie, sexo, grupo, desde=0, hasta=None:
        an_a.resumen(s.agentes, especie, desde, hasta)[f'mortalidad_{sexo}'][grupo],
    # Población mínima en [desde, hasta) frente a la del año `antes`.
    'agentes.caida': lambda s, especie, antes, desde, hasta=None:
        min(_pob(s, especie)[desde:hasta]) / _pob(s, especie)[antes],
    # Población del año `anio` frente a la del año `antes`.
    'agentes.relativa': lambda s, especie, antes, anio: _pob(s, especie)[anio] / _pob(s, especie)[antes],
    # Población del año `anio` frente al mínimo de [desde, anio]: cuánto se ha recuperado.
    'agentes.recuperacion': lambda s, especie, desde, anio:
        _pob(s, especie)[anio] / min(_pob(s, especie)[desde:anio + 1]),
    'agentes.hembras_adultas': lambda s, especie, anio: an_a.hembras_adultas(s.agentes, especie, anio),
    'agentes.residuo_energia': lambda s: an_a.residuo_energia(s.agentes),
}


def comando(esc):
    binario = os.path.join(RAIZ, 'target', 'release', esc['binario'])
    args = [binario, esc['config'], '--set', f'run.name="{esc["nombre"]}"',
            '--set', f'run.output_path="{SALIDA}"']
    for a in esc.get('ajustes', []):
        args += ['--set', a]
    return args


def correr(esc):
    os.makedirs(os.path.join(RAIZ, SALIDA), exist_ok=True)
    log = os.path.join(RAIZ, SALIDA, f"{esc['nombre']}.log")
    inicio = time.monotonic()
    with open(log, 'w') as f:
        r = subprocess.run(comando(esc), cwd=RAIZ, stdout=f, stderr=subprocess.STDOUT)
    return r.returncode, time.monotonic() - inicio, log


def evaluar(esc, segundos_pared):
    s = Salida(esc['nombre'])
    filas = []
    for m in esc.get('metrica', []):
        params = {k: v for k, v in m.items() if k not in ('medida', 'min', 'max', 'nota')}
        try:
            valor = float(MEDIDAS[m['medida']](s, **params))
            error = None
        except Exception as e:  # una medida que no se puede calcular cuenta como fallo
            valor, error = float('nan'), f'{type(e).__name__}: {e}'
        ok = error is None and not math.isnan(valor) and m['min'] <= valor <= m['max']
        filas.append({'medida': m['medida'], 'params': params, 'valor': valor, 'min': m['min'],
                      'max': m['max'], 'ok': ok, 'nota': m.get('nota', ''), 'error': error})
    try:
        total_s = s.tiempos['total_s']
        etapas = sorted(s.tiempos['etapas'], key=lambda e: -e['segundos'])
    except (FileNotFoundError, KeyError):
        total_s, etapas = float('nan'), []
    presupuesto = esc.get('presupuesto_s', float('inf'))
    return {
        'nombre': esc['nombre'], 'binario': esc['binario'], 'descripcion': esc.get('descripcion', ''),
        'segundos_pared': segundos_pared, 'total_s': total_s, 'presupuesto_s': presupuesto,
        'ok_tiempo': not math.isnan(total_s) and total_s <= presupuesto,
        'etapas_principales': [{'nombre': e['nombre'], 'fraccion': e['fraccion']} for e in etapas[:5]],
        'metricas': filas,
    }


def etiqueta(fila):
    p = ', '.join(f'{k}={v}' for k, v in fila['params'].items())
    return f"{fila['medida']}({p})" if p else fila['medida']


def git(*args):
    try:
        return subprocess.run(['git', *args], cwd=RAIZ, capture_output=True, text=True).stdout.strip()
    except OSError:
        return ''


def main():
    ap = argparse.ArgumentParser(description='Batería de escenarios de referencia (CLAUDE §10.5).')
    ap.add_argument('--solo', help='escenarios a pasar, separados por comas')
    ap.add_argument('--listar', action='store_true', help='lista los escenarios y sale')
    ap.add_argument('--sin-compilar', action='store_true', help='no compila los binarios')
    ap.add_argument('-j', type=int, default=0, help='escenarios a la vez (por defecto, todos)')
    a = ap.parse_args()

    with open(os.path.join(RAIZ, 'herramientas', 'escenarios.toml'), 'rb') as f:
        escenarios = tomllib.load(f)['escenario']
    if a.listar:
        for e in escenarios:
            print(f"{e['nombre']:<22} {e['binario']:<8} {len(e.get('metrica', []))} métricas · {e.get('descripcion', '')}")
        return 0
    if a.solo:
        pedidos = set(a.solo.split(','))
        desconocidos = pedidos - {e['nombre'] for e in escenarios}
        if desconocidos:
            print(f"Escenarios desconocidos: {', '.join(sorted(desconocidos))}")
            return 2
        escenarios = [e for e in escenarios if e['nombre'] in pedidos]

    if not a.sin_compilar:
        bins = sorted({e['binario'] for e in escenarios})
        cmd = ['cargo', 'build', '--release', '--quiet'] + [x for b in bins for x in ('--bin', b)]
        print('Compilando', ', '.join(bins), '...', flush=True)
        if subprocess.run(cmd, cwd=RAIZ).returncode != 0:
            return 2

    print(f'Corriendo {len(escenarios)} escenarios ...', flush=True)
    with ThreadPoolExecutor(max_workers=a.j or len(escenarios)) as ex:
        corridas = list(ex.map(correr, escenarios))
    fallos = [(e['nombre'], log) for e, (rc, _, log) in zip(escenarios, corridas) if rc != 0]
    if fallos:
        for n, log in fallos:
            print(f'La corrida {n} falló; mira {log}')
        return 2

    informe = {
        'commit': git('rev-parse', '--short', 'HEAD'),
        'cambios_sin_commit': bool(git('status', '--porcelain', '--untracked-files=no')),
        'escenarios': [evaluar(e, seg) for e, (_, seg, _) in zip(escenarios, corridas)],
    }
    informe['ok'] = all(r['ok_tiempo'] and all(m['ok'] for m in r['metricas']) for r in informe['escenarios'])
    with open(os.path.join(RAIZ, SALIDA, 'informe.json'), 'w') as f:
        json.dump(informe, f, indent=2, ensure_ascii=False)

    for r in informe['escenarios']:
        marca_t = '✓' if r['ok_tiempo'] else '✗'
        etapas = ', '.join(f"{e['nombre']} {100 * e['fraccion']:.0f} %" for e in r['etapas_principales'][:3])
        print(f"\n== {r['nombre']} ({r['binario']}): {r['descripcion']}")
        print(f"   {marca_t} tiempo {r['total_s']:.1f} s (presupuesto {r['presupuesto_s']:.0f} s; {etapas})")
        for m in r['metricas']:
            marca = '✓' if m['ok'] else '✗'
            extra = f"  [{m['error']}]" if m['error'] else ''
            print(f"   {marca} {etiqueta(m)} = {m['valor']:.4g}  (margen {m['min']:g}–{m['max']:g}){extra}")
    n_mal = sum(not m['ok'] for r in informe['escenarios'] for m in r['metricas'])
    n_t = sum(not r['ok_tiempo'] for r in informe['escenarios'])
    print(f"\n{'TODO DENTRO DE MÁRGENES' if informe['ok'] else 'FUERA DE MÁRGENES'}: "
          f"{n_mal} métricas y {n_t} presupuestos fuera. Informe -> {SALIDA}/informe.json")
    return 0 if informe['ok'] else 1


if __name__ == '__main__':
    sys.exit(main())
