#!/usr/bin/env python3
"""Medidas de una corrida de la biología de los agentes (binario `agentes`).

Lee `<dir>/<nombre>_agentes.json` y resume cada especie en un tramo de años: población frente a
`N_max`, densidad, fecundidad, esperanza de vida, hambre y mortalidad por sexo. Lo usan la batería de
escenarios (`herramientas/escenarios.py`) y quien quiera consultar una corrida.

Uso: herramientas/analisis/agentes.py DIR NOMBRE [DESDE_AÑO [HASTA_AÑO]]
     (p. ej. `herramientas/analisis/agentes.py output agentes 40`)
"""
import json
import math
import sys


def cargar(directorio, nombre):
    with open(f'{directorio}/{nombre}_agentes.json') as f:
        return json.load(f)


def _media(xs):
    xs = [x for x in xs if x is not None]
    return sum(xs) / len(xs) if xs else float('nan')


def indice_especie(doc, especie):
    for k, e in enumerate(doc['run']['especies']):
        if e['nombre'] == especie:
            return k
    raise KeyError(f"no hay especie '{especie}' en la corrida")


def serie_poblacion(doc, especie):
    """Población media de cada año."""
    k = indice_especie(doc, especie)
    return [a['especies'][k]['poblacion_media'] for a in doc['anios']]


def resumen(doc, especie, desde=0, hasta=None):
    """Medias de la especie en los años [desde, hasta)."""
    k = indice_especie(doc, especie)
    e = doc['run']['especies'][k]
    xs = [a['especies'][k] for a in doc['anios'][desde:hasta]]
    area = doc['run']['area_total_km2']
    pob = [x['poblacion_media'] for x in xs]
    m = _media(pob)
    muertes = ['muertes_inanicion', 'muertes_intrinseca', 'muertes_edad_max', 'muertes_orfandad',
               'muertes_destete']
    total_muertes = sum(sum(x.get(c) or 0 for c in muertes) for x in xs)
    n_ult = sum(x.get('n_ultimo_parto') or 0 for x in xs)
    ult = (sum((x.get('edad_ultimo_parto_anios') or 0.0) * (x.get('n_ultimo_parto') or 0) for x in xs)
           / n_ult) if n_ult else float('nan')
    grupos = len(xs[0].get('fecundidad_grupos') or []) if xs else 0
    fec = [_media([x['fecundidad_grupos'][g] for x in xs]) for g in range(grupos)]
    # Fecundidad relativa a la del grupo 20–24 años (índice 4 con grupos de 5 años).
    fec_rel = [fec[g] / fec[4] for g in range(4, 10)] if grupos >= 10 and fec[4] > 0 else []
    return {
        'n_max': e['n_max'],
        'n_rel': m / e['n_max'] if e['n_max'] > 0 else float('nan'),
        'cv': math.sqrt(_media([(p - m) ** 2 for p in pob])) / m if m > 0 else float('nan'),
        'densidad_km2': m / area,
        'tgf': _media([x['tgf'] for x in xs]),
        'e0_anios': _media([x['e0_anios'] for x in xs]),
        'ultimo_parto_anios': ult,
        'n_ultimo_parto': n_ult,
        'fecundidad_rel': fec_rel,
        'hambre_frac': sum(x['muertes_inanicion'] for x in xs) / total_muertes if total_muertes else 0.0,
        'mortalidad_hembras': [_media([x['mortalidad_hembras'][g] for x in xs]) for g in range(4)],
        'mortalidad_machos': [_media([x['mortalidad_machos'][g] for x in xs]) for g in range(4)],
        'extinta_anio': next((a['anio'] for a in doc['anios'] if a['especies'][k]['poblacion_media'] == 0), None),
    }


def hembras_adultas(doc, especie, anio):
    """Fracción de hembras entre los adultos (edad ≥ a_rep) en la pirámide del año."""
    k = indice_especie(doc, especie)
    e = doc['run']['especies'][k]
    clase = math.ceil(e['a_rep_dias'] / e['delta_a_dias'])
    a = doc['anios'][anio]['especies'][k]
    h, m = sum(a['piramide_hembras'][clase:]), sum(a['piramide_machos'][clase:])
    return h / (h + m) if h + m else float('nan')


def residuo_energia(doc):
    """Peor residuo del balance de energía por registro (kcal)."""
    return max(abs(t['balance']['residuo']) for t in doc['ticks'])


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(2)
    d, nombre = sys.argv[1], sys.argv[2]
    desde = int(sys.argv[3]) if len(sys.argv) > 3 else 0
    hasta = int(sys.argv[4]) if len(sys.argv) > 4 else None
    doc = cargar(d, nombre)
    print(f"{nombre}: {len(doc['anios'])} años, {doc['run']['area_total_km2']:.0f} km², "
          f"residuo de energía {residuo_energia(doc):.1e} kcal; medias de los años {desde}–{hasta or len(doc['anios'])}")
    for e in doc['run']['especies']:
        r = resumen(doc, e['nombre'], desde, hasta)
        print(f"  {e['nombre']}: N/N_max {r['n_rel']:.2f} (cv {r['cv']:.2f}), densidad {r['densidad_km2']:.3f}/km², "
              f"TGF {r['tgf']:.2f}, e0 {r['e0_anios']:.1f} años, último parto {r['ultimo_parto_anios']:.1f} años "
              f"(n {r['n_ultimo_parto']}), hambre {100 * r['hambre_frac']:.0f} % de las muertes, extinta {r['extinta_anio']}")
        if r['fecundidad_rel']:
            print('    fecundidad relativa 20–24 … 45–49: ' + ' / '.join(f'{x:.2f}' for x in r['fecundidad_rel']))
        print('    mortalidad (1/año) crías/jóvenes/adultos/viejos: hembras '
              + ' / '.join(f'{x:.3f}' for x in r['mortalidad_hembras']) + ' · machos '
              + ' / '.join(f'{x:.3f}' for x in r['mortalidad_machos']))


if __name__ == '__main__':
    main()
