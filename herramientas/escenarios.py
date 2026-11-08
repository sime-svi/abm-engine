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
margen. Un escenario puede llevar, en lugar de un presupuesto fijo, una **regla relativa**
(`presupuesto_relativo` y `techo_s`; decisión del responsable del 2026-11-08 para el planeta): no
puede ser más de esa fracción más lento que la versión anterior medida en la misma tanda, ni pasar
del techo con el escenario solo. La versión anterior (`--referencia`; por defecto `main`, o el
commit anterior si se está en `main`) se compila en un worktree aparte (`output/referencia/`). Los
dos escenarios corren a la vez, sin la caché de la génesis y sin nada más en marcha: así ven las
mismas condiciones de la máquina (uno tras otro, el segundo salía hasta un 2–5 % más lento). El
techo se comprueba con ese mismo tiempo, que es algo mayor que el del escenario solo: si cabe así,
cabe solo.

Uso:
  herramientas/escenarios.py                  # compila en release y pasa toda la batería
  herramientas/escenarios.py --solo planeta   # solo algunos (separados por comas)
  herramientas/escenarios.py --listar         # lista los escenarios
  herramientas/escenarios.py --sin-compilar   # usa los binarios ya compilados
  herramientas/escenarios.py -j 1             # de uno en uno (tiempos más limpios)
  herramientas/escenarios.py --referencia ba3609c   # la versión anterior para la regla relativa
  herramientas/escenarios.py --sin-referencia # sin la versión anterior (solo el techo absoluto)

Escribe las salidas en `output/escenarios/` y el informe en `output/escenarios/informe.json`. Sale con
0 si todo está dentro de márgenes y presupuestos, con 1 si algo no lo está y con 2 si falla una
corrida.
"""
import argparse
import functools
import json
import math
import os
import shutil
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


def comando(esc, binario=None, salida=SALIDA):
    binario = binario or os.path.join(RAIZ, 'target', 'release', esc['binario'])
    args = [binario, esc['config'], '--set', f'run.name="{esc["nombre"]}"',
            '--set', f'run.output_path="{salida}"']
    for a in esc.get('ajustes', []):
        args += ['--set', a]
    # Con la regla relativa de tiempo, siempre sin la caché de la génesis: se mide la génesis entera.
    if 'presupuesto_relativo' in esc:
        args += ['--set', 'run.genesis_cache=""']
    return args


def correr(esc, binario=None, cwd=RAIZ, salida=SALIDA):
    os.makedirs(os.path.join(cwd, salida), exist_ok=True)
    log = os.path.join(cwd, salida, f"{esc['nombre']}.log")
    inicio = time.monotonic()
    with open(log, 'w') as f:
        r = subprocess.run(comando(esc, binario, salida), cwd=cwd, stdout=f, stderr=subprocess.STDOUT)
    return r.returncode, time.monotonic() - inicio, log


def referencia_por_defecto():
    """La versión anterior: `main` desde otra rama; en `main`, el último commit si hay cambios sin
    commitear y el anterior si no."""
    if git('rev-parse', '--abbrev-ref', 'HEAD') != 'main':
        return 'main'
    return 'HEAD' if git('status', '--porcelain', '--untracked-files=no') else 'HEAD~1'


def preparar_referencia(ref, binarios):
    """Compila los binarios de `ref` en su worktree (`output/referencia/<commit>/arbol`), una sola vez
    por commit. Devuelve (commit, carpeta del worktree)."""
    sha = git('rev-parse', '--short', ref)
    if not sha:
        raise RuntimeError(f'no encuentro la referencia {ref}')
    base = os.path.join(RAIZ, 'output', 'referencia', sha)
    arbol = os.path.join(base, 'arbol')
    hechos = all(os.path.exists(os.path.join(base, b)) for b in binarios)
    if not hechos:
        if not os.path.isdir(arbol):
            os.makedirs(base, exist_ok=True)
            subprocess.run(['git', 'worktree', 'add', '--detach', '--quiet', arbol, sha], cwd=RAIZ, check=True)
        destino = os.path.join(RAIZ, 'output', 'referencia', 'target')
        cmd = ['cargo', 'build', '--release', '--quiet'] + [x for b in binarios for x in ('--bin', b)]
        print(f'Compilando la versión anterior ({ref} = {sha}) ...', flush=True)
        subprocess.run(cmd, cwd=arbol, check=True, env=dict(os.environ, CARGO_TARGET_DIR=destino))
        for b in binarios:
            shutil.copy2(os.path.join(destino, 'release', b), os.path.join(base, b))
    return sha, arbol


def evaluar(esc, segundos_pared, ref=None):
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
    ok_tiempo = not math.isnan(total_s) and total_s <= presupuesto
    relativo = None
    if 'presupuesto_relativo' in esc:
        # Regla relativa: el techo absoluto (solo) y, con la versión anterior, la fracción de más.
        presupuesto = esc['techo_s']
        ok_tiempo = not math.isnan(total_s) and total_s <= presupuesto
        if ref is not None:
            maximo = (1.0 + esc['presupuesto_relativo']) * ref['total_s']
            ok_tiempo = ok_tiempo and total_s <= maximo
            relativo = {'commit': ref['commit'], 'total_s': ref['total_s'],
                        'cambio_frac': total_s / ref['total_s'] - 1.0,
                        'maximo_frac': esc['presupuesto_relativo']}
    return {
        'nombre': esc['nombre'], 'binario': esc['binario'], 'descripcion': esc.get('descripcion', ''),
        'segundos_pared': segundos_pared, 'total_s': total_s, 'presupuesto_s': presupuesto,
        'referencia': relativo,
        'ok_tiempo': ok_tiempo,
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
    ap.add_argument('--referencia', help='versión anterior para la regla relativa de tiempo (por defecto main, '
                    'o el commit anterior en main)')
    ap.add_argument('--sin-referencia', action='store_true', help='sin la versión anterior: solo el techo absoluto')
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

    # Los de regla relativa, primero: la versión anterior y la actual a la vez, sin nada más.
    relativos = [e for e in escenarios if 'presupuesto_relativo' in e]
    refs = {}
    corridas = {}
    ref = sha = arbol = None
    if relativos and not a.sin_referencia:
        ref = a.referencia or referencia_por_defecto()
        sha, arbol = preparar_referencia(ref, sorted({e['binario'] for e in relativos}))
    for e in relativos:
        trabajos = [lambda e=e: correr(e)]
        if sha is not None:
            print(f"Corriendo {e['nombre']} y el de la versión anterior ({ref} = {sha}) a la vez ...", flush=True)
            binario = os.path.join(RAIZ, 'output', 'referencia', sha, e['binario'])
            salida = os.path.join('..', 'salida')
            trabajos.append(lambda e=e: correr(e, binario, cwd=arbol, salida=salida))
        else:
            print(f"Corriendo {e['nombre']} ...", flush=True)
        with ThreadPoolExecutor(max_workers=len(trabajos)) as ex:
            res = list(ex.map(lambda f: f(), trabajos))
        corridas[e['nombre']] = res[0]
        if sha is not None:
            rc, _, log = res[1]
            if rc != 0:
                print(f'La corrida {e["nombre"]} de la versión anterior falló; mira {log}')
                return 2
            with open(os.path.join(arbol, salida, f"{e['nombre']}_tiempos.json")) as f:
                refs[e['nombre']] = {'commit': f'{ref} = {sha}', 'total_s': json.load(f)['total_s']}
    resto = [e for e in escenarios if 'presupuesto_relativo' not in e]
    print(f'Corriendo {len(resto)} escenarios ...', flush=True)
    with ThreadPoolExecutor(max_workers=a.j or max(len(resto), 1)) as ex:
        for e, r in zip(resto, ex.map(correr, resto)):
            corridas[e['nombre']] = r
    fallos = [(n, log) for n, (rc, _, log) in corridas.items() if rc != 0]
    if fallos:
        for n, log in fallos:
            print(f'La corrida {n} falló; mira {log}')
        return 2

    informe = {
        'commit': git('rev-parse', '--short', 'HEAD'),
        'cambios_sin_commit': bool(git('status', '--porcelain', '--untracked-files=no')),
        'escenarios': [evaluar(e, corridas[e['nombre']][1], refs.get(e['nombre'])) for e in escenarios],
    }
    informe['ok'] = all(r['ok_tiempo'] and all(m['ok'] for m in r['metricas']) for r in informe['escenarios'])
    with open(os.path.join(RAIZ, SALIDA, 'informe.json'), 'w') as f:
        json.dump(informe, f, indent=2, ensure_ascii=False)

    for r in informe['escenarios']:
        marca_t = '✓' if r['ok_tiempo'] else '✗'
        etapas = ', '.join(f"{e['nombre']} {100 * e['fraccion']:.0f} %" for e in r['etapas_principales'][:3])
        print(f"\n== {r['nombre']} ({r['binario']}): {r['descripcion']}")
        if r.get('referencia'):
            ref = r['referencia']
            regla = (f"versión anterior {ref['commit']}, a la vez: {ref['total_s']:.1f} s, {100 * ref['cambio_frac']:+.1f} %; "
                     f"regla: ≤ +{100 * ref['maximo_frac']:.0f} % y ≤ {r['presupuesto_s']:.0f} s")
        elif 'presupuesto_relativo' in next(e for e in escenarios if e['nombre'] == r['nombre']):
            regla = f"sin la versión anterior; techo {r['presupuesto_s']:.0f} s solo"
        else:
            regla = f"presupuesto {r['presupuesto_s']:.0f} s"
        print(f"   {marca_t} tiempo {r['total_s']:.1f} s ({regla}; {etapas})")
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
