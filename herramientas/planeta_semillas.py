#!/usr/bin/env python3
"""El planeta por defecto con varias semillas y la tabla de cierre de la tectónica (media ± desviación).

El planeta es caótico: un cambio pequeño cambia qué placas chocan y cuándo. Para comparar con la
Tierra o calibrar, se mira la media de varias semillas, no la semilla 42 de la batería (que queda para
la no regresión). Corre el planeta entero (génesis, relieve, un año de clima) con cada semilla, a la
vez, lo analiza como la batería (`analisis/terreno.py`) y lo compara con la Tierra:

- relieve: ETOPO 2022 con la roca rebotada sin hielo, a 26 km (`config/referencias/tierra_relieve.toml`);
- tectónica: los objetivos de la adenda de balance y de la nota de fosas (v4: la vida de los márgenes
  pasivos, Bradley 2008);
- choques, solo diagnóstico: frente a los bordes CCB de Bird (2003), medidos como el modelo
  (`config/referencias/tierra_bordes.toml`).

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
import shutil
import subprocess
import sys
import time
import tomllib
from concurrent.futures import ThreadPoolExecutor

RAIZ = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(RAIZ, 'herramientas', 'analisis'))
import terreno as an_t  # noqa: E402

SALIDA = 'output/semillas'


# Copia del binario con que corre la tanda: si se recompila mientras corre, los procesos no cargan a
# medias el binario nuevo (pasó el 2026-11-08: una semilla salió distinta en la novena cifra).
BINARIO = os.path.join(RAIZ, SALIDA, 'terreno_de_la_tanda')


def correr(semilla, ajustes):
    nombre = f'semilla_{semilla}'
    args = [BINARIO, 'config/terreno.toml',
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
    # Nota de demografía de placas §10: placas al final y fracción del planeta en las 15 mayores.
    areas = sorted((p['area_frac'] for p in t['placas_final']), reverse=True)
    m['tect_placas_final'] = len(areas)
    m['tect_15_mayores_frac'] = sum(areas[:15])
    m['tect_placas_1pc'] = sum(1 for a in areas if a >= 0.01)
    # Nota de isostasia con litosfera §8 y §10: por clase de edad tectonotérmica, la altura media de
    # la tierra y su fracción; los márgenes estirados distales por tiempo desde la rotura.
    b = t['balance']
    for i in range(len(CLASES_EDAD)):
        m[f'tect_altura_edad_{i}'] = b['altura_tierra_por_edad_final_m'][i]
        m[f'tect_tierra_edad_{i}_frac'] = b['tierra_por_edad_final_frac'][i]
    for i in range(len(CLASES_MARGEN)):
        m[f'tect_margen_prof_{i}'] = b['margen_prof_m'][i]
        m[f'tect_margen_tect_{i}'] = b['margen_tect_m'][i]
    # Hundimiento del basamento en ~100 Myr: la clase 80–120 Myr frente a la 0–20.
    m['tect_margen_hundimiento_100myr_m'] = b['margen_tect_m'][3] - b['margen_tect_m'][0]
    for i, causa in enumerate(CAUSAS_REINICIO):
        m[f'tect_reinicio_{causa}_km2_100myr'] = b['reinicio_km2_100myr'][i]
    return m


def referencia():
    with open(os.path.join(RAIZ, 'config', 'referencias', 'tierra_relieve.toml'), 'rb') as f:
        r = tomllib.load(f)
    roca = r['relieve']['roca_rebote']['nivel3']
    return {
        'tierra_frac': roca['tierra_frac'],
        'altura_media_tierra_m': roca['altura_media_tierra_m'],
        'tierra_bajo_200_frac': roca['hipsometria_frac'][0],
        'tierra_sobre_2000_frac': sum(roca['hipsometria_frac'][4:]),
        'rel_pendiente_media_tierra': r['pendientes_25km']['tierra_sobre_0']['media'],
        'rel_plataforma_km': r['plataforma']['nivel3']['anchura_media_km'],
    }


# Nota de isostasia con litosfera §10: clases de edad tectonotérmica, con la altura media de la tierra
# real en cada una y su fracción de la tierra (ETOPO sin hielo con las edades de los tipos de corteza;
# `config/referencias/tierra_isostasia_s0.toml`, sección `por_edad`), y clases del tiempo desde la
# rotura de los márgenes.
CLASES_EDAD = ['0–100', '100–500', '500–1 600', '1 600–2 500', '> 2 500']
CLASES_MARGEN = ['0–20', '20–50', '50–80', '80–120', '120–200', '> 200']
CAUSAS_REINICIO = ['rift', 'arco', 'choque', 'delaminacion']


def por_edad_tierra():
    with open(os.path.join(RAIZ, 'config', 'referencias', 'tierra_isostasia_s0.toml'), 'rb') as f:
        r = tomllib.load(f)['por_edad']['R4_sin_dinamica']
    claves = ['desde_0', 'desde_100', 'desde_500', 'desde_1600', 'desde_2500']
    return [r[k]['altura_real_m'] for k in claves], [r[k]['area_frac'] for k in claves]


ALTURA_EDAD_TIERRA, TIERRA_EDAD_FRAC = por_edad_tierra()

# (clave, etiqueta, objetivo en texto, función que dice si cumple).
FILAS = [
    ('tierra_frac', 'Tierra emergida (fracción)', None, None),
    ('altura_media_tierra_m', 'Altura media de la tierra (m)', None, None),
    ('tierra_bajo_200_frac', 'Tierra por debajo de 200 m (fracción)', None, None),
    ('tierra_sobre_2000_frac', 'Tierra por encima de 2 000 m (fracción)', None, None),
    ('rel_pendiente_media_tierra', 'Pendiente media de la tierra (a ±25 km)', None, None),
    ('rel_plataforma_km', 'Anchura media de la plataforma (km)', None, None),
    ('tect_nivel_mar_m', 'Nivel del mar al final de la deriva, respecto al dato (m)',
     '0 (la Tierra real con la regla del modelo: −150)', None),
    ('profundidad_media_oceano_m', 'Profundidad media del océano (m)', '3 667', lambda x: abs(x - 3667.0) <= 150.0),
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
    ('tect_bal_vida_media_margen_myr', 'Vida media de los márgenes pasivos que terminan (Myr)', '130–180',
     lambda x: 130.0 <= x <= 180.0),
    ('tect_bal_edad_media_margen_final_myr', 'Edad media de los márgenes pasivos al final (Myr)', '~104',
     lambda x: abs(x - 104.0) <= 26.0),
    ('tect_bal_edad_max_margen_final_myr', 'Edad máxima de los márgenes pasivos al final (Myr)', '~180',
     lambda x: x <= 225.0),
    ('tect_bal_colisiones_km', 'Choques: longitud (km)', 'CCB 23 000 ± 25 %', lambda x: abs(x / 23003.0 - 1.0) <= 0.25),
    ('tect_bal_difusas_km', 'Convergencia difusa: longitud (km)', 'OCB 17 400 ± 50 %', lambda x: abs(x / 17447.0 - 1.0) <= 0.5),
    ('tect_placas_final', 'Placas al final', 'Bird 58 en la malla (13 de ≥1 %)', None),
    ('tect_placas_1pc', 'Placas de al menos el 1 % del planeta, al final', 'Bird 13', lambda x: x <= 1.5 * 13),
    ('tect_15_mayores_frac', 'Fracción del planeta en las 15 placas mayores, al final', 'Bird 0,958 ± 0,10', lambda x: abs(x - 0.958) <= 0.10),
    ('tect_bal_convergencia_media_choques_cm_anio', 'Choques: convergencia media (cm/año)', 'CCB 1,87', None),
    ('tect_bal_duracion_media_choques_myr', 'Choques: duración media de los que acaban (Myr)', None, None),
    ('tect_bal_choques_con_losa_au', 'Choques: pérdida con la losa colgando (AU)', None, None),
    ('tect_bal_choques_sin_losa_au', 'Choques: pérdida tras la rotura (AU)', None, None),
    ('tect_bal_choques_au', 'Choques: pérdida total (AU)', '0,4–0,7', lambda x: 0.4 <= x <= 0.7),
    # Nota de rifts §10 (Pangea: 6–10 rupturas grandes en ~150 Myr; Bradley 2008; Seton et al. 2020).
    ('tect_bal_rifts_100myr', 'Rifts: rupturas por 100 Myr', '~4–7', lambda x: 4.0 <= x <= 7.0),
    ('tect_bal_rifts_sutura_frac', 'Rifts: fracción que sigue una sutura', 'apreciable', None),
    ('tect_bal_roturas_rift_100myr', 'Rifts: roturas de un lado (nace el océano) por 100 Myr', None, None),
    ('tect_bal_franja_estirada_final_km', 'Márgenes pasivos: franja estirada al final (km)', '100–300',
     lambda x: 100.0 <= x <= 300.0),
    ('tect_bal_costa_margen_final_km', 'Márgenes pasivos: corteza en la costa al final (km)', None, None),
    ('tect_bal_bloque_mayor_frac', 'Mayor bloque continental (fracción del continente, media)', None, None),
    ('tect_bal_ciclos_supercontinente', 'Ciclos de supercontinente tras el rodaje (450 Myr)',
     '~1,5 en 630 Myr (~1,1 en 450)', None),
    ('tiempo_s', 'Tiempo de cálculo del planeta (s)', '≤ 275', lambda x: x <= 275.0),
]
# Nota de isostasia con litosfera §10: la altura de la tierra por clase de edad (± 300 m de la real), la
# fracción de la tierra en cada clase (criterio blando, sin marca) y el hundimiento de los márgenes.
FILAS += [(f'tect_altura_edad_{i}', f'Altura media de la tierra con edad {c} Myr (m)',
           f'{ALTURA_EDAD_TIERRA[i]:.0f} ± 300', (lambda r: lambda x: abs(x - r) <= 300.0)(ALTURA_EDAD_TIERRA[i]))
          for i, c in enumerate(CLASES_EDAD)]
FILAS += [(f'tect_tierra_edad_{i}_frac', f'Fracción de la tierra con edad {c} Myr',
           f'{TIERRA_EDAD_FRAC[i]:.3f} (blando)', None) for i, c in enumerate(CLASES_EDAD)]
FILAS += [(f'tect_margen_prof_{i}', f'Margen distal a {c} Myr de la rotura: profundidad (m)', None, None)
          for i, c in enumerate(CLASES_MARGEN)]
FILAS += [(f'tect_margen_tect_{i}', f'Margen distal a {c} Myr: basamento sin sedimento (m)', None, None)
          for i, c in enumerate(CLASES_MARGEN)]
FILAS += [('tect_margen_hundimiento_100myr_m', 'Hundimiento del basamento del margen en ~100 Myr (m)',
           '~2 000–3 000 (McKenzie 1978)', lambda x: 2000.0 <= x <= 3000.0)]
FILAS += [(f'tect_reinicio_{c}_km2_100myr', f'Área continental reiniciada por {c} (km² por 100 Myr)', None, None)
          for c in CAUSAS_REINICIO]


def main():
    ap = argparse.ArgumentParser(description='El planeta con varias semillas: tabla de cierre.')
    ap.add_argument('--semillas', default='42,1,2,3,4')
    ap.add_argument('--set', action='append', default=[], help='ajuste de la config (como en el binario)')
    ap.add_argument('--sin-compilar', action='store_true')
    a = ap.parse_args()
    semillas = [int(x) for x in a.semillas.split(',')]
    if not a.sin_compilar:
        subprocess.run(['cargo', 'build', '--release', '--quiet', '--bin', 'terreno'], cwd=RAIZ, check=True)
    os.makedirs(os.path.join(RAIZ, SALIDA), exist_ok=True)
    shutil.copy2(os.path.join(RAIZ, 'target', 'release', 'terreno'), BINARIO)
    inicio = time.monotonic()
    with ThreadPoolExecutor(max_workers=len(semillas)) as ex:
        res = list(ex.map(lambda s: correr(s, a.set), semillas))
    for m in res:
        m['tierra_bajo_200_frac'] = m['hipsometria_frac'][0]
        m['tierra_sobre_2000_frac'] = sum(m['hipsometria_frac'][4:])
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
