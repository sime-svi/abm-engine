#!/usr/bin/env python3
"""Diagnóstico del supercontinente con varias semillas (pedido del responsable el 2026-11-08).

Lee las corridas del planeta de `herramientas/planeta_semillas.py` (`output/semillas/semilla_*_terreno.json`)
y, con las fotos que la deriva toma cada 25 Myr (`fotos_supercontinente`) y las métricas de cada Myr,
responde a cinco preguntas, sin cambiar nada del modelo:

1. ¿Qué fracción del continente está en el bloque mayor? (la mayor masa conexa, por encima de los bordes
   de placa, y el mayor bloque de una sola placa);
2. ¿Se acercan o se alejan los bloques continentales? (distancia media entre las masas de al menos el
   1 % del continente, por pares pesados por el producto de sus áreas, y su ritmo con las velocidades
   de cada foto);
3. Para cada océano abierto por un rift (rodeado de los márgenes pasivos de sus dos lados): su edad,
   cuándo se vuelve activo uno de sus márgenes y si llega a cerrarse (sus dos márgenes vuelven a
   tocarse);
4. ¿Qué fracción de la longitud de margen pasivo que termina pasa a ser activa, por invasión (fosa o
   convergencia difusa) o por choque?
5. ¿Qué fuerzas mueven las placas que llevan continentes, y van los continentes hacia las fosas?

Uso:
  herramientas/analisis/supercontinente.py                    # semillas 42,1,2,3,4 de output/semillas
  herramientas/analisis/supercontinente.py --dir output/semillas --semillas 42,1 --salida x.json

Escribe el resumen en JSON (por defecto `output/semillas/supercontinente.json`) y una tabla en
Markdown por la salida estándar.
"""
import argparse
import json
import math
import os

RAIZ = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# Un océano está abierto cuando la mediana de la distancia entre sus dos márgenes pasa de ABIERTO_KM, y se
# cierra cuando, después, vuelve a bajar de CERRADO_KM (la mayor parte de los dos márgenes se junta otra
# vez). La distancia mínima no sirve: en las puntas del rift los márgenes conjugados siguen juntos.
ABIERTO_KM = 1000.0
CERRADO_KM = 300.0
# Un margen deja de ser pasivo cuando al menos esta fracción de sus celdas tiene un borde convergente
# cerca.
ACTIVO_FRAC = 0.25
DESTINOS = ['sigue pasivo', 'activo por invasión (fosa o difusa cerca)', 'activo por choque',
            'ya no es margen, sin borde convergente cerca', 'su muestra ya no está']
FUERZAS = ['tirón de la losa', 'empuje (dorsales y resto)', 'choques', 'arrastre basal']


def leer(directorio, semilla):
    with open(os.path.join(directorio, f'semilla_{semilla}_terreno.json')) as f:
        return json.load(f)['run']['tectonica']


def media(v):
    v = [x for x in v if x is not None]
    return sum(v) / len(v) if v else float('nan')


def desv(v):
    v = [x for x in v if x is not None]
    if len(v) < 2:
        return 0.0
    m = media(v)
    return math.sqrt(sum((x - m) ** 2 for x in v) / (len(v) - 1))


def oceanos(t, lado_km):
    """Historia de cada océano de rift a partir de las fotos."""
    por_rift = {}
    for f in t['fotos_supercontinente']:
        for o in f['oceanos']:
            por_rift.setdefault(o['rift'], []).append((f['myr'], o))
    fin = t['fotos_supercontinente'][-1]['myr'] if t['fotos_supercontinente'] else 0.0
    res = []
    for rid, serie in sorted(por_rift.items()):
        rotura = serie[0][1]['rotura_myr']
        ancho_max = max(o['ancho_km'] for _, o in serie)
        abierto = None
        cierre = None
        activo = None
        for myr, o in serie:
            if abierto is None and o['ancho_km'] >= ABIERTO_KM:
                abierto = myr
            if abierto is not None and cierre is None and o['ancho_km'] <= CERRADO_KM:
                cierre = myr
            if activo is None and max(o['activo_frac']) >= ACTIVO_FRAC:
                activo = myr
        ultimo = serie[-1][0]
        # Un océano que deja de verse antes del final: alguno de sus márgenes desapareció.
        desaparece = ultimo < fin and cierre is None
        res.append({
            'rift': rid, 'rotura_myr': rotura, 'ancho_max_km': ancho_max,
            'ancho_final_km': serie[-1][1]['ancho_km'],
            'abierto': abierto is not None,
            'cerrado': cierre is not None,
            'edad_al_cerrar_myr': (cierre - rotura) if cierre is not None else None,
            'edad_al_activarse_myr': (activo - rotura) if activo is not None else None,
            'desaparece': desaparece,
            'edad_final_myr': (ultimo - rotura),
        })
    return res


def analizar(t, lado_km):
    fotos = t['fotos_supercontinente']
    met = [m for m in t['metricas'] if not m['rodaje']]
    r = {}
    # 1–2. Series cada 25 Myr.
    r['serie'] = [{k: f[k] for k in ('myr', 'masa_mayor_frac', 'bloque_mayor_frac', 'masas', 'dist_media_km',
                                     'dist_ritmo_km_myr', 'pares_acercandose_frac', 'hacia_fosa_frac',
                                     'v_hacia_fosa_cm_anio', 'fosa_propia_hundiendose_frac',
                                     'fosa_propia_cabalgando_frac')} for f in fotos]
    tras = [f for f in fotos if f['myr'] > met[0]['myr'] - 1.0] if met else fotos
    r['masa_mayor_media'] = media([f['masa_mayor_frac'] for f in tras])
    r['masa_mayor_max'] = max((f['masa_mayor_frac'] for f in tras), default=0.0)
    r['bloque_mayor_media'] = media([f['bloque_mayor_frac'] for f in tras])
    r['dist_media_km'] = media([f['dist_media_km'] for f in tras])
    r['dist_ritmo_km_myr'] = media([f['dist_ritmo_km_myr'] for f in tras])
    r['fotos_acercandose_frac'] = media([1.0 if f['dist_ritmo_km_myr'] < 0 else 0.0 for f in tras])
    r['pares_acercandose_frac'] = media([f['pares_acercandose_frac'] for f in tras])
    # Cambio real de la distancia entre fotos consecutivas (km por 25 Myr), frente al ritmo.
    cambios = [b['dist_media_km'] - a['dist_media_km'] for a, b in zip(tras, tras[1:])]
    r['dist_cambio_km_25myr'] = media(cambios)
    # 3. Océanos.
    oc = oceanos(t, lado_km)
    abiertos = [o for o in oc if o['abierto']]
    r['oceanos'] = oc
    r['oceanos_rotos'] = len(oc)
    r['oceanos_abiertos'] = len(abiertos)
    r['oceanos_cerrados'] = sum(o['cerrado'] for o in abiertos)
    r['oceanos_activados'] = sum(o['edad_al_activarse_myr'] is not None for o in abiertos)
    r['oceanos_desaparecen'] = sum(o['desaparece'] for o in abiertos)
    r['edad_al_cerrar_myr'] = media([o['edad_al_cerrar_myr'] for o in abiertos])
    r['edad_al_activarse_myr'] = media([o['edad_al_activarse_myr'] for o in abiertos])
    r['edad_final_abiertos_myr'] = media([o['edad_final_myr'] for o in abiertos
                                          if not o['cerrado'] and not o['desaparece']])
    r['ancho_max_km'] = media([o['ancho_max_km'] for o in abiertos])
    # 4. El margen pasivo de cada foto, 25 Myr después (entre fotos, sin el vaivén de la costa).
    con = [f['margen_tras_25myr_frac'] for f in tras if sum(f.get('margen_tras_25myr_frac', [0] * 5)) > 0]
    for i, c in enumerate(DESTINOS):
        r[f'margen_25_{i}'] = media([x[i] for x in con])
    r['margen_pasivo_medio_km'] = media([m['margen_pasivo_km'] for m in met])
    # Los márgenes como entidades que nacen en cada rift: cómo terminan y su vida.
    ms = t.get('margenes_rift', [])
    fin = [m for m in ms if m['fin_myr'] is not None]
    r['margenes_rift'] = len(ms)
    r['margenes_rift_terminados'] = len(fin)
    for c in ('invasion', 'choque', 'desaparece'):
        r[f'margen_rift_{c}_frac'] = sum(m['causa'] == c for m in fin) / len(fin) if fin else 0.0
    r['vida_margen_rift_myr'] = media([m['fin_myr'] - m['rotura_myr'] for m in fin])
    # 5. Fuerzas de las placas con continente y continentes hacia las fosas.
    pot = [0.0] * 4
    pot_pos = [0.0] * 4
    n_placas, con_tiron, v_cont = 0, 0, []
    hacia_placa = []
    for f in tras:
        for p in f['placas']:
            n_placas += 1
            for i in range(4):
                pot[i] += p['potencias'][i]
                pot_pos[i] += max(p['potencias'][i], 0.0)
            if p['potencias'][0] > 0.0:
                con_tiron += 1
            v_cont.append(p['v_cm_anio'])
            hacia_placa.append(p['v_hacia_fosa_cm_anio'])
    motor = pot_pos[0] + pot_pos[1]
    r['potencia_tiron_frac'] = pot_pos[0] / motor if motor > 0 else 0.0
    r['potencia_empuje_frac'] = pot_pos[1] / motor if motor > 0 else 0.0
    freno = -(pot[2] + pot[3])
    r['freno_choques_frac'] = -pot[2] / freno if freno > 0 else 0.0
    r['freno_arrastre_frac'] = -pot[3] / freno if freno > 0 else 0.0
    r['placas_cont_con_tiron_frac'] = con_tiron / n_placas if n_placas else 0.0
    r['placas_cont_por_foto'] = n_placas / len(tras) if tras else 0.0
    r['v_placas_cont_cm_anio'] = media(v_cont)
    r['hacia_fosa_frac'] = media([f['hacia_fosa_frac'] for f in tras])
    r['v_hacia_fosa_cm_anio'] = media([f['v_hacia_fosa_cm_anio'] for f in tras])
    r['fosa_propia_hundiendose_frac'] = media([f['fosa_propia_hundiendose_frac'] for f in tras])
    r['fosa_propia_cabalgando_frac'] = media([f['fosa_propia_cabalgando_frac'] for f in tras])
    r['v_placas_cont_hacia_fosa_cm_anio'] = media(hacia_placa)
    r['v_rms_media_cm_anio'] = t['v_rms_media_cm_anio']
    return r


FILAS = [
    ('masa_mayor_media', 'Continente en la mayor masa conexa (media)', '{:.3f}'),
    ('masa_mayor_max', 'Continente en la mayor masa conexa (máximo)', '{:.3f}'),
    ('bloque_mayor_media', 'Continente en el mayor bloque de una placa (media)', '{:.3f}'),
    ('dist_media_km', 'Distancia media entre masas de ≥ 1 % (km)', '{:.0f}'),
    ('dist_ritmo_km_myr', 'Ritmo de la distancia con las velocidades de cada foto (km/Myr)', '{:+.2f}'),
    ('dist_cambio_km_25myr', 'Cambio real de la distancia entre fotos (km por 25 Myr)', '{:+.0f}'),
    ('fotos_acercandose_frac', 'Fotos en que las masas se acercan (fracción)', '{:.2f}'),
    ('pares_acercandose_frac', 'Pares de masas que se acercan (fracción, por área)', '{:.2f}'),
    ('oceanos_rotos', 'Océanos de rift (rifts rotos con los dos márgenes)', '{:.1f}'),
    ('oceanos_abiertos', f'… que llegan a abrirse más de {ABIERTO_KM:.0f} km', '{:.1f}'),
    ('ancho_max_km', '… anchura máxima media (km)', '{:.0f}'),
    ('oceanos_activados', f'… con un margen activo (≥ {ACTIVO_FRAC:.0%} de sus celdas)', '{:.1f}'),
    ('edad_al_activarse_myr', '… edad del océano al activarse un margen (Myr)', '{:.0f}'),
    ('oceanos_cerrados', '… que se cierran (sus márgenes vuelven a tocarse)', '{:.1f}'),
    ('edad_al_cerrar_myr', '… edad al cerrarse (Myr)', '{:.0f}'),
    ('oceanos_desaparecen', '… cuyo margen desaparece sin cerrarse', '{:.1f}'),
    ('edad_final_abiertos_myr', '… edad al final de los que siguen abiertos (Myr)', '{:.0f}'),
    ('margen_pasivo_medio_km', 'Margen pasivo medio (km)', '{:.0f}'),
    ('margenes_rift', 'Márgenes pasivos nacidos en rifts (entidades)', '{:.1f}'),
    ('margenes_rift_terminados', '… que terminan', '{:.1f}'),
    ('margen_rift_invasion_frac', '… por invasión (fracción de los que terminan)', '{:.2f}'),
    ('margen_rift_choque_frac', '… por un choque', '{:.2f}'),
    ('margen_rift_desaparece_frac', '… porque desaparecen', '{:.2f}'),
    ('vida_margen_rift_myr', '… vida media de los que terminan (Myr)', '{:.0f}'),
] + [(f'margen_25_{i}', f'Margen pasivo de una foto, 25 Myr después: {c} (fracción)', '{:.3f}')
     for i, c in enumerate(DESTINOS)] + [
    ('placas_cont_por_foto', 'Placas con ≥ 5 % del continente (por foto)', '{:.1f}'),
    ('v_placas_cont_cm_anio', 'Velocidad de esas placas (cm/año)', '{:.2f}'),
    ('v_rms_media_cm_anio', 'Velocidad media de todas las placas (cm/año)', '{:.2f}'),
    ('placas_cont_con_tiron_frac', 'Placas con continente que tienen losa propia (fracción)', '{:.2f}'),
    ('potencia_tiron_frac', 'Su motor: tirón de la losa (fracción de la potencia)', '{:.2f}'),
    ('potencia_empuje_frac', 'Su motor: empuje (fracción)', '{:.2f}'),
    ('freno_arrastre_frac', 'Su freno: arrastre basal (fracción)', '{:.2f}'),
    ('freno_choques_frac', 'Su freno: choques (fracción)', '{:.2f}'),
    ('hacia_fosa_frac', 'Continente que se acerca a la fosa activa más cercana (fracción)', '{:.2f}'),
    ('v_hacia_fosa_cm_anio', 'Velocidad media del continente hacia esa fosa (cm/año)', '{:+.2f}'),
    ('fosa_propia_hundiendose_frac', 'Continente cuya fosa más cercana es una donde se hunde su placa', '{:.2f}'),
    ('fosa_propia_cabalgando_frac', 'Continente cuya fosa más cercana está bajo su placa (cabalga)', '{:.2f}'),
]


def main():
    ap = argparse.ArgumentParser(description='Diagnóstico del supercontinente con varias semillas.')
    ap.add_argument('--dir', default=os.path.join(RAIZ, 'output', 'semillas'))
    ap.add_argument('--semillas', default='42,1,2,3,4')
    ap.add_argument('--salida', default=None)
    a = ap.parse_args()
    semillas = [int(x) for x in a.semillas.split(',')]
    res = {}
    for s in semillas:
        t = leer(a.dir, s)
        res[s] = analizar(t, t['lado_km'])
    salida = a.salida or os.path.join(a.dir, 'supercontinente.json')
    with open(salida, 'w') as f:
        json.dump({str(k): v for k, v in res.items()}, f, indent=1, ensure_ascii=False)
    print(f'| Magnitud | media | desv. | {" | ".join(str(s) for s in semillas)} |')
    print('|---|---|---|' + '---|' * len(semillas))
    for clave, etiqueta, fmt in FILAS:
        v = [res[s][clave] for s in semillas]
        print(f'| {etiqueta} | {fmt.format(media(v))} | {desv(v):.3g} | '
              + ' | '.join(fmt.format(x) for x in v) + ' |')
    print()
    print('Serie cada 25 Myr (media de las semillas): Myr, masa mayor, distancia (km), ritmo (km/Myr), '
          'continente hacia la fosa')
    n = min(len(res[s]['serie']) for s in semillas)
    for i in range(n):
        fs = [res[s]['serie'][i] for s in semillas]
        print(f'  {fs[0]["myr"]:7.0f}  {media([f["masa_mayor_frac"] for f in fs]):.3f}  '
              f'{media([f["dist_media_km"] for f in fs]):6.0f}  {media([f["dist_ritmo_km_myr"] for f in fs]):+6.2f}  '
              f'{media([f["hacia_fosa_frac"] for f in fs]):.2f}')
    print(f'\nResumen -> {salida}')


if __name__ == '__main__':
    main()
