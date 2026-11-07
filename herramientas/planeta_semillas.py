#!/usr/bin/env python3
"""El planeta por defecto con varias semillas y la tabla de cierre de la tectónica (media ± desviación).

El planeta es caótico: un cambio pequeño cambia qué placas chocan y cuándo. Para comparar con la
Tierra o calibrar, se mira la media de varias semillas, no la semilla 42 de la batería (que queda para
la no regresión). Corre el planeta entero (génesis, relieve, un año de clima) con cada semilla, a la
vez, lo analiza como la batería (`analisis/terreno.py`) y lo compara con la Tierra:

- relieve: ETOPO 2022 con la roca rebotada sin hielo, a 26 km (`config/referencias/tierra_relieve.toml`);
- tectónica: los objetivos de la adenda de balance y de la nota de fosas.

Uso:
  herramientas/planeta_semillas.py                       # semillas 42,1,2,3,4 con la config por defecto
  herramientas/planeta_semillas.py --semillas 7,8 --set tectonica.e1_km_myr=0.02
  herramientas/planeta_semillas.py --sin-compilar

Escribe cada corrida en `output/semillas/` y el resumen en `output/semillas/cierre.json`.
"""
import argparse
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
import terreno as an_t  # noqa: E402

SALIDA = 'output/semillas'


def correr(semilla, ajustes):
    nombre = f'semilla_{semilla}'
    args = [os.path.join(RAIZ, 'target', 'release', 'terreno'), 'config/terreno.toml',
            '--set', f'run.name="{nombre}"', '--set', f'run.output_path="{SALIDA}"',
            '--set', f'run.seed={semilla}', '--set', 'run.genesis_cache=""',
            '--set', 'run.dump_world=true', '--set', 'run.dump_climate=true']
    for a in ajustes:
        args += ['--set', a]
    os.makedirs(os.path.join(RAIZ, SALIDA), exist_ok=True)
    with open(os.path.join(RAIZ, SALIDA, f'{nombre}.log'), 'w') as f:
        r = subprocess.run(args, cwd=RAIZ, stdout=f, stderr=subprocess.STDOUT)
    if r.returncode != 0:
        raise RuntimeError(f'la semilla {semilla} falló (ver {SALIDA}/{nombre}.log)')
    dir_ = os.path.join(RAIZ, SALIDA)
    m = an_t.analizar(dir_, nombre)
    with open(os.path.join(dir_, f'{nombre}_tiempos.json')) as f:
        m['tiempo_s'] = json.load(f)['total_s']
    with open(os.path.join(dir_, f'{nombre}_terreno.json')) as f:
        t = json.load(f)['run']['tectonica']
    bins = t['edades_oceano']['frac_por_10_myr']
    m['tect_edad_mas_180_frac'] = sum(bins[18:])
    return m


def referencia():
    with open(os.path.join(RAIZ, 'config', 'referencias', 'tierra_relieve.toml'), 'rb') as f:
        r = tomllib.load(f)
    roca = r['relieve']['roca_rebote']['nivel3']
    return {
        'tierra_frac': roca['tierra_frac'],
        'altura_media_tierra_m': roca['altura_media_tierra_m'],
        'tierra_bajo_200_frac': roca['hipsometria_frac'][0],
        'rel_pendiente_media_tierra': r['pendientes_25km']['tierra_sobre_0']['media'],
        'rel_plataforma_km': r['plataforma']['nivel3']['anchura_media_km'],
    }


# (clave, etiqueta, objetivo en texto, función que dice si cumple).
FILAS = [
    ('tierra_frac', 'Tierra emergida (fracción)', None, None),
    ('altura_media_tierra_m', 'Altura media de la tierra (m)', None, None),
    ('tierra_bajo_200_frac', 'Tierra por debajo de 200 m (fracción)', None, None),
    ('rel_pendiente_media_tierra', 'Pendiente media de la tierra (a ±25 km)', None, None),
    ('rel_plataforma_km', 'Anchura media de la plataforma (km)', None, None),
    ('tect_bal_espesor_max_final_km', 'Corteza máxima al final (km)', '70–80', lambda x: x <= 80.0),
    ('tect_bal_espesor_max_todo_km', 'Corteza máxima de toda la deriva (km)', '70–80', lambda x: x <= 80.0),
    ('tect_edad_media_myr', 'Edad media del fondo (Myr)', '64,2', lambda x: abs(x - 64.2) <= 10.0),
    ('tect_edad_mas_180_frac', 'Fondo de más de 180 Myr (fracción)', '< 0,01', lambda x: x < 0.01),
    ('tect_f_cont_final', 'f_cont', '0,40 ± 0,03', lambda x: abs(x - 0.40) <= 0.03),
    ('tect_bal_v_cont_cambio_frac', 'Cambio del volumen continental', '±0,10', lambda x: abs(x) <= 0.10),
    ('tect_v_rms_media_cm_anio', 'Velocidad de las placas (cm/año)', '4,39', lambda x: abs(x - 4.39) <= 0.5),
    ('tect_bal_sedimento_al_mar_au', 'Sedimento al mar (AU)', '≈1,8', lambda x: abs(x - 1.8) <= 0.4),
    ('tect_bal_fosas_km', 'Longitud de fosa (km)', '51 000 ± 25 %', lambda x: abs(x / 51000.0 - 1.0) <= 0.25),
    ('tect_bal_area_subducida_km2_anio', 'Área que se hunde (km²/año)', '2,7–3,4', lambda x: 2.7 <= x <= 3.4),
    ('tiempo_s', 'Tiempo de cálculo del planeta (s)', '≤ 200', lambda x: x <= 200.0),
]


def main():
    ap = argparse.ArgumentParser(description='El planeta con varias semillas: tabla de cierre.')
    ap.add_argument('--semillas', default='42,1,2,3,4')
    ap.add_argument('--set', action='append', default=[], help='ajuste de la config (como en el binario)')
    ap.add_argument('--sin-compilar', action='store_true')
    a = ap.parse_args()
    semillas = [int(x) for x in a.semillas.split(',')]
    if not a.sin_compilar:
        subprocess.run(['cargo', 'build', '--release', '--quiet', '--bin', 'terreno'], cwd=RAIZ, check=True)
    inicio = time.monotonic()
    with ThreadPoolExecutor(max_workers=len(semillas)) as ex:
        res = list(ex.map(lambda s: correr(s, a.set), semillas))
    for m in res:
        m['tierra_bajo_200_frac'] = m['hipsometria_frac'][0]
    ref = referencia()
    resumen = {'semillas': semillas, 'ajustes': a.set, 'filas': []}
    print(f'\n{len(semillas)} semillas ({", ".join(map(str, semillas))}) en {time.monotonic() - inicio:.0f} s de pared')
    print(f'{"":<44} {"media":>10} {"desv.":>9} {"mín.":>10} {"máx.":>10}   Tierra / objetivo')
    for clave, etiqueta, objetivo, cumple in FILAS:
        v = [m[clave] for m in res]
        media = sum(v) / len(v)
        sd = math.sqrt(sum((x - media) ** 2 for x in v) / max(len(v) - 1, 1))
        tierra = ref.get(clave)
        texto = f'{tierra:.4g} (ETOPO)' if tierra is not None else (objetivo or '')
        if cumple is not None:
            texto += '  ✓' if cumple(media) else '  ✗'
        print(f'{etiqueta:<44} {media:>10.4g} {sd:>9.3g} {min(v):>10.4g} {max(v):>10.4g}   {texto}')
        resumen['filas'].append({'clave': clave, 'etiqueta': etiqueta, 'media': media, 'desv': sd,
                                 'min': min(v), 'max': max(v), 'valores': v, 'tierra': tierra,
                                 'objetivo': objetivo})
    with open(os.path.join(RAIZ, SALIDA, 'cierre.json'), 'w') as f:
        json.dump(resumen, f, indent=1, ensure_ascii=False)
    print(f'\nResumen -> {SALIDA}/cierre.json')


if __name__ == '__main__':
    main()
