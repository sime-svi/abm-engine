#!/usr/bin/env python3
"""Medidas de una corrida del planeta (binario `terreno`).

Lee `<dir>/<nombre>_terreno.json` y, si existe, el volcado por celda `<nombre>_world.json` (corrida
con `run.dump_world = true` y `run.dump_climate = true`). Las medidas de la salida JSON, como los
biomas, el diagnóstico del planeta y `c_P`, están siempre. Las que necesitan el volcado, como la
sombra pluviométrica, las laderas de barlovento de pie a cresta o los percentiles de `w` y `m_oro`,
solo con él. Lo usan la batería de escenarios (`herramientas/escenarios.py`), la lluvia observada
(`lluvia_observada.py`, que mide la Tierra con las mismas funciones) y quien quiera comparar variantes
del planeta.

Uso: herramientas/analisis/terreno.py DIR NOMBRE [NOMBRE...]
     (la primera corrida es la referencia de los Δ; el relieve de la sombra sale de ella)
"""
import itertools
import json
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import referencia_tierra as rt  # noqa: E402  (geometría del cubo esférico)

R_KM = 6371.0
BIOMAS = ['oceano', 'hielo', 'desierto', 'tundra', 'taiga', 'pradera', 'sabana', 'bosque_templado', 'selva']


def cargar(directorio, nombre, con_celdas=True):
    with open(f'{directorio}/{nombre}_terreno.json') as f:
        doc = json.load(f)
    celdas = None
    if con_celdas:
        try:
            with open(f'{directorio}/{nombre}_world.json') as f:
                celdas = json.load(f)
        except FileNotFoundError:
            pass
    return doc, celdas


def percentiles(vals, pesos, qs):
    orden = sorted(zip(vals, pesos))
    tot = sum(pesos)
    out, acc, i = [], 0.0, 0
    for q in qs:
        while i < len(orden) - 1 and acc + orden[i][1] < q * tot:
            acc += orden[i][1]
            i += 1
        out.append(orden[i][0])
    return out


def resumen_salida(doc):
    """Medidas de la salida JSON (sin volcado)."""
    t = doc['ticks']
    ultimo = t[-1]
    area = sum(b['area_m2'] for b in ultimo['biome_area'])
    r = {
        'biomas_pct': {b['biome']: 100.0 * b['area_m2'] / area for b in ultimo['biome_area']},
        'lagos': ultimo['lake_cells'],
        'rios_km': ultimo['river_length_km'],
        'temp_media': ultimo['temp_mean'],
        'residuo_agua_max': max(abs(x['water_residual_rel']) for x in t),
        'c_p': doc['run']['lluvia']['c_p'],
        'desvio_lluvia': doc['run']['lluvia']['desvio'],
        'lluvia_total_m3': sum(x['precip_total_m3'] for x in t),
    }
    diag = dict(doc.get('diagnostico') or {})
    rel = diag.pop('relieve', None)
    r.update(diag)
    # Relieve tectónico (esquema v6; v7 añade el balance de la corteza): historia de la génesis y
    # diagnóstico ampliado del relieve.
    t = doc['run'].get('tectonica')
    if t:
        r.update({
            'tect_f_cont_final': t['f_cont_final'],
            'tect_nivel_mar_m': t['nivel_mar_m'],
            'tect_nivel_mar_tras_prerrodaje_m': t['nivel_mar_tras_prerrodaje_m'],
            'tect_v_rms_media_cm_anio': t['v_rms_media_cm_anio'],
            'tect_forsyth_uyeda_fosa': t['forsyth_uyeda_fosa'],
            'tect_forsyth_uyeda_continente': t['forsyth_uyeda_continente'],
            'tect_edad_media_myr': t['edades_oceano']['media_myr'],
            'tect_edad_mediana_myr': t['edades_oceano']['mediana_myr'],
            'tect_edad_max_myr': t['edades_oceano']['max_myr'],
            'tect_edad_mas_200_frac': t['edades_oceano']['frac_por_10_myr'][-1],
            'tect_placas_final': len(t['placas_final']),
            'tect_placa_mayor_frac': t['placas_final'][0]['area_frac'] if t['placas_final'] else 0.0,
            'tect_rifts': sum(1 for e in t['eventos'] if e['tipo'] == 'rift'),
            'tect_suturas': sum(1 for e in t['eventos'] if e['tipo'] == 'sutura'),
            'tect_remuestreo_rel': t['cuentas']['remuestreo_km3'] / t['cuentas']['volumen_inicial_km3'],
        })
        # Balance de la corteza continental (adenda de balance, esquema v7): medias sin el rodaje.
        if 'balance' in t:
            r.update({f'tect_bal_{k}': v for k, v in t['balance'].items()})
            r['tect_error_balance_max_rel'] = t['cuentas']['error_balance_max_rel']
            # Nota de bordes §10: fondo consumido en bordes divergentes y transformantes (en la
            # Tierra, cero) y corteza continental solapada en ellos, como fracción del total.
            a = t['cuentas'].get('area_subducida_por_borde_km2')
            v = t['cuentas'].get('choque_solapado_por_borde_km3')
            if a and sum(a) > 0:
                r['tect_consumo_no_convergente_frac'] = (a[1] + a[4]) / sum(a)
            if v and sum(v) > 0:
                r['tect_solape_no_convergente_frac'] = (v[1] + v[4]) / sum(v)
    if rel:
        pend = {int(p['sobre_m']): p for p in rel['pendientes_25km']}
        r.update({
            'rel_plataforma_km': rel['plataforma_anchura_km'],
            'rel_hack_h': rel['hack_h'],
            'rel_pendiente_media_tierra': pend[0]['media'],
            'rel_pendiente_p50_sobre_2000': pend[2000]['p50_p75_p90_p95_p99'][0],
            'rel_oceano_0_200_frac': rel['hipsometria_oceano_frac'][0],
            'rel_tierra_en_depresion_frac': rel['tierra_en_depresion_frac'],
        })
    return r


def relieve_por_casillas(celdas, paso=0.25):
    """Altura máxima de la tierra en casillas de `paso` grados (para buscar cordilleras)."""
    rel = {}
    for c in celdas:
        if c['surface'] != 'oceano':
            k = (math.floor(c['lat_deg'] / paso), math.floor(c['lon_deg'] / paso))
            rel[k] = max(rel.get(k, -1e9), c['h'])
    return rel


def _hay_cresta(c, rel, dmin, dmax, sentido, paso=0.25, umbral=2000.0):
    """¿Hay relieve > umbral entre dmin y dmax km a lo largo del paralelo? `sentido` = −1: hacia
    barlovento (de donde viene el viento); +1: a favor del viento. Vientos del oeste en 30–60° y del
    este en el resto, como el clima del modelo."""
    lat = c['lat_deg']
    coslat = max(math.cos(math.radians(lat)), 1e-3)
    hacia_el_este = 1.0 if 30.0 <= abs(lat) < 60.0 else -1.0
    kl = math.floor(lat / paso)
    d = dmin
    while d <= dmax:
        lon = c['lon_deg'] + sentido * hacia_el_este * math.degrees(d / (R_KM * coslat))
        lon = (lon + 180.0) % 360.0 - 180.0
        ko = math.floor(lon / paso)
        if any(rel.get((kl + dk, ko), -1e9) > umbral for dk in (-1, 0, 1)):
            return True
        d += 10.0
    return False


def laderas_barlovento(tierra, rel):
    """Laderas de barlovento de las cordilleras grandes: suben con el viento (m_oro > 1), h ≥ 300 m y
    cresta > 2000 m a 0–150 km a favor del viento."""
    return [c for c in tierra if c['m_oro'] > 1.0 and c['h'] >= 300.0 and _hay_cresta(c, rel, 0.0, 150.0, +1)]


def sotavento_y_resto(tierra, rel):
    """Tierra de latitudes medias (30–60°) por debajo de 2000 m, partida en sotavento (cresta > 2000 m
    a 200–500 km a barlovento) y resto."""
    banda = [c for c in tierra if 30.0 <= abs(c['lat_deg']) < 60.0 and c['h'] < 2000.0]
    marca = [_hay_cresta(c, rel, 200.0, 500.0, -1) for c in banda]
    return [c for c, m in zip(banda, marca) if m], [c for c, m in zip(banda, marca) if not m]


# --- Laderas de barlovento de pie a cresta (regla del responsable, 2026-11-05) ----------------------
# Una celda es ladera de barlovento si sube de cara al viento del modelo (del oeste en 30–60° de
# latitud y del este en el resto) desde el pie hasta la cresta:
#   1. el terreno sube a favor del viento en la celda (m_oro > 1) y h ≥ 300 m, como hasta ahora;
#   2. contra el viento, hasta el pie, todo es más bajo que la celda: fuera las mesetas y los sotaventos;
#   3. la cresta a la que sube (lo más alto a favor del viento antes de que el terreno empiece a bajar más
#      de TOL_LADERA_M) pasa de 2000 m: es una cordillera grande.
# Se camina por el paralelo cada PASO_LADERA_KM sobre las celdas de nivel 3: contra el viento hasta
# ALCANCE_LADERA_KM o hasta el mar, que cuenta como llano a 0 m (el aire ve el mar plano), y a favor
# del viento para buscar la cresta. El pie es lo más bajo de ese camino contra el viento, donde empieza
# la subida: el primer punto que ya está a menos de TOL_LADERA_M de esa altura. Si ese punto es el mar
# (antes de la costa no hay tierra a menos de TOL_LADERA_M del nivel del mar), el pie está en el mar.
# La referencia es la lluvia del llano del pie: el pie y hasta dos muestras más contra el viento, solo
# de tierra y en el llano.
PASO_LADERA_KM = 10.0
ALCANCE_LADERA_KM = 500.0
TOL_LADERA_M = 100.0
H_LADERA_M = 300.0
CRESTA_LADERA_M = 2000.0
N_REF_LADERA = 3
LIM_ALTURA_LADERA = [300.0, 1000.0, 2000.0]     # franjas de altura de las laderas (la última sin techo)


def mapa_nivel3(celdas):
    """Para cada celda de nivel 3 del cubo esférico, la celda del volcado que la contiene. El volcado
    mezcla niveles de detalle: una celda de nivel l cubre 4^(3−l) celdas de nivel 3."""
    n3 = rt.N3
    dueno = [None] * (6 * n3 * n3)
    for c in celdas:
        n = rt.N0 << c['level']
        la, lo = math.radians(c['lat_deg']), math.radians(c['lon_deg'])
        idx = rt.localizar(math.cos(la) * math.cos(lo), math.cos(la) * math.sin(lo), math.sin(la), n)
        cara, resto = divmod(idx, n * n)
        j, i = divmod(resto, n)
        f = n3 // n
        for dj in range(f):
            base = (cara * n3 + j * f + dj) * n3 + i * f
            dueno[base:base + f] = [c] * f
    return dueno


def _es_mar(v):
    return v is None or v['surface'] == 'oceano'


def _camino(c, dueno, sentido):
    """Las celdas cada PASO_LADERA_KM por el paralelo de `c`, contra el viento del modelo (sentido −1)
    o a favor (+1), hasta ALCANCE_LADERA_KM, sin la propia `c`. `dueno` da, para cada celda de nivel 3,
    la celda que la contiene (None donde no hay ninguna: el mar sin datos)."""
    la = math.radians(c['lat_deg'])
    cp, sp = math.cos(la), math.sin(la)
    hacia_el_este = 1.0 if 30.0 <= abs(c['lat_deg']) < 60.0 else -1.0
    paso = sentido * hacia_el_este * math.degrees(PASO_LADERA_KM / (R_KM * max(cp, 1e-3)))
    n3, loc = rt.N3, rt.localizar
    for k in range(1, int(ALCANCE_LADERA_KM / PASO_LADERA_KM) + 1):
        lo = math.radians(c['lon_deg'] + k * paso)
        v = dueno[loc(cp * math.cos(lo), cp * math.sin(lo), sp, n3)]
        if v is not c:
            yield v


def ladera_pie_cresta(c, dueno):
    """Si `c` es una ladera de barlovento de pie a cresta, su pie: {'pie_h' (0 en el mar), 'en_mar',
    'ref' (celdas de tierra del llano del pie), 'ref_mar' (celdas de mar justo delante, si el pie está
    en el mar), 'cresta_m'}. Si no lo es, None."""
    hc = c['h']
    if c['m_oro'] <= 1.0 or hc < H_LADERA_M:
        return None
    tramo = []                  # (celda, altura) del camino contra el viento; el mar, a 0 m
    camino = _camino(c, dueno, -1)
    for v in camino:
        mar = _es_mar(v)
        tramo.append((v, 0.0 if mar else v['h']))
        if mar:
            break
    if not tramo:
        return None
    m = min(h for _, h in tramo)
    i = next(i for i, (_, h) in enumerate(tramo) if h <= m + TOL_LADERA_M)
    if m >= hc or any(h >= hc for _, h in tramo[:i]):
        return None             # tierra tan alta como la celda antes del pie: meseta o sotavento
    pie, h_pie = tramo[i]
    en_mar = _es_mar(pie)
    ref = [v for v, h in tramo[i:i + N_REF_LADERA] if not _es_mar(v) and h <= m + TOL_LADERA_M]
    ref_mar = []
    if en_mar:
        ref_mar = [v for v in [pie] + list(itertools.islice(camino, N_REF_LADERA - 1))
                   if v is not None and _es_mar(v)]
    cresta = hc
    for v in _camino(c, dueno, +1):
        h = 0.0 if _es_mar(v) else v['h']
        if h < cresta - TOL_LADERA_M:
            break
        cresta = max(cresta, h)
    if cresta <= CRESTA_LADERA_M:
        return None
    return {'pie_h': h_pie, 'en_mar': en_mar, 'ref': ref, 'ref_mar': ref_mar, 'cresta_m': cresta}


def laderas_pie_cresta(tierra, dueno):
    """Las laderas de barlovento de pie a cresta de `tierra`: lista de (celda, pie)."""
    out = []
    for c in tierra:
        p = ladera_pie_cresta(c, dueno)
        if p is not None:
            out.append((c, p))
    return out


def cociente_laderas(laderas, clave, con_mar=False):
    """Lluvia (`clave`; None sin dato) de cada ladera de pie a cresta entre la del llano de su pie. Sin
    referencia de tierra (el pie en el mar), la ladera se omite, salvo con `con_mar`, que toma la lluvia
    del mar justo delante. Da el área de las laderas, la fracción con el pie en el mar, la omitida (de
    las que tienen dato), el cociente de las medias por área, la mediana y la media de los cocientes de
    cada celda, las dos lluvias medias, la altura media del pie y el cociente por franjas de altura."""
    nan = float('nan')
    a_tot = a_mar = a_dato = 0.0
    filas = []                  # (cociente, área, lluvia, referencia, h de la ladera, h del pie)
    for c, p in laderas:
        a = c['area_km2']
        a_tot += a
        if p['en_mar']:
            a_mar += a
        if c.get(clave) is None:
            continue
        a_dato += a
        ref = [v[clave] for v in p['ref'] if v.get(clave) is not None]
        if not ref and con_mar:
            ref = [v[clave] for v in p['ref_mar'] if v.get(clave) is not None]
        if not ref or sum(ref) <= 0.0:
            continue
        pr = sum(ref) / len(ref)
        filas.append((c[clave] / pr, a, c[clave], pr, c['h'], p['pie_h']))
    out = {'area_km2': a_tot, 'pie_en_mar_frac': a_mar / a_tot if a_tot else nan,
           'omitida_frac': nan, 'cociente_de_medias': nan, 'mediana': nan, 'media': nan,
           'lluvia_mm': nan, 'referencia_mm': nan, 'pie_h_medio_m': nan,
           'por_altura': {'area_frac': [], 'cociente_de_medias': []}}
    sa = sum(f[1] for f in filas)
    if sa:
        sp = sum(f[2] * f[1] for f in filas)
        sr = sum(f[3] * f[1] for f in filas)
        out.update({'omitida_frac': max(0.0, 1.0 - sa / a_dato), 'cociente_de_medias': sp / sr,
                    'mediana': rt.mediana_ponderada([(f[0], f[1]) for f in filas]),
                    'media': sum(f[0] * f[1] for f in filas) / sa, 'lluvia_mm': sp / sa, 'referencia_mm': sr / sa,
                    'pie_h_medio_m': sum(f[5] * f[1] for f in filas) / sa})
    for lo, hi in zip(LIM_ALTURA_LADERA, LIM_ALTURA_LADERA[1:] + [math.inf]):
        fs = [f for f in filas if lo <= f[4] < hi]
        a_franja = sum(c['area_km2'] for c, _ in laderas if lo <= c['h'] < hi)
        sr = sum(f[3] * f[1] for f in fs)
        out['por_altura']['area_frac'].append(a_franja / a_tot if a_tot else nan)
        out['por_altura']['cociente_de_medias'].append(sum(f[2] * f[1] for f in fs) / sr if sr else nan)
    return out


def resumen_celdas(celdas, rel):
    """Medidas del volcado por celda: lluvia, `w`, `m_oro`, sombra pluviométrica y laderas de
    barlovento de pie a cresta."""
    tierra = [c for c in celdas if c['surface'] != 'oceano']
    a = [c['area_km2'] for c in tierra]
    at = sum(a)
    w = [c['w_hum'] for c in tierra]
    mo = [c['m_oro'] for c in tierra]
    r = {
        'lluvia_tierra_mm': sum(c['lluvia_anual_mm'] * c['area_km2'] for c in tierra) / at,
        'w_percentiles': percentiles(w, a, [0.01, 0.1, 0.5, 0.9, 0.99]),
        'tierra_w_bajo_pct': 100.0 * sum(x for x, ww in zip(a, w) if ww < 0.5) / at,
        'm_oro_percentiles': percentiles(mo, a, [0.01, 0.1, 0.5, 0.9, 0.99]),
    }
    lad = laderas_barlovento(tierra, rel)
    al = sum(c['area_km2'] for c in lad)
    r['barlovento_m_oro'] = sum(c['m_oro'] * c['area_km2'] for c in lad) / al if al else float('nan')
    r['barlovento_lluvia_mm'] = sum(c['lluvia_anual_mm'] * c['area_km2'] for c in lad) / al if al else float('nan')
    dentro, fuera = sotavento_y_resto(tierra, rel)

    def zona(cs):
        ar = sum(c['area_km2'] for c in cs)
        if ar == 0:
            return {'area_km2': 0.0, 'lluvia_mm': float('nan'), 'desierto_pct': float('nan')}
        des = sum(c['area_km2'] for c in cs if c['biome'] == 'desierto')
        return {'area_km2': ar, 'lluvia_mm': sum(c['lluvia_anual_mm'] * c['area_km2'] for c in cs) / ar,
                'desierto_pct': 100.0 * des / ar}
    r['sotavento'] = zona(dentro)
    r['resto_latitudes_medias'] = zona(fuera)
    # Laderas de barlovento de pie a cresta, con la lluvia del llano del pie como referencia (solo de
    # tierra, como en la Tierra observada: las de pie en el mar se omiten). Necesita el nivel de cada celda.
    if all('level' in c for c in celdas):
        q = cociente_laderas(laderas_pie_cresta(tierra, mapa_nivel3(celdas)), 'lluvia_anual_mm')
        r['barlovento_ladera_ratio'] = q['cociente_de_medias']
        r['barlovento_ladera_area_km2'] = q['area_km2']
    return r


def analizar(directorio, nombre, rel=None):
    doc, celdas = cargar(directorio, nombre)
    r = resumen_salida(doc)
    if celdas is not None:
        r.update(resumen_celdas(celdas, rel if rel is not None else relieve_por_casillas(celdas)))
    return r


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(2)
    d, nombres = sys.argv[1], sys.argv[2:]
    _, celdas0 = cargar(d, nombres[0])
    rel = relieve_por_casillas(celdas0) if celdas0 is not None else None
    rs = [(n, analizar(d, n, rel)) for n in nombres]
    print(f"{'corrida':<18} {'c_P':>6} {'desvío':>7} {'lagos':>6} {'ríos km':>8}  biomas (% del planeta: "
          + ', '.join(b[:8] for b in BIOMAS) + ')')
    for n, r in rs:
        print(f"{n:<18} {r['c_p']:>6.3f} {100 * r['desvio_lluvia']:>+6.1f}% {r['lagos']:>6} {r['rios_km']:>8.0f}  "
              + ' '.join(f"{r['biomas_pct'].get(b, 0.0):.2f}" for b in BIOMAS))
    if any('tierra_frac' in r for _, r in rs):
        print('\nDiagnóstico (salida JSON): tierra · altura media / mediana de la tierra · % de la tierra por franjas · '
              'océano · lluvia tierra / mar · w<0,5 · PPN Miami')
        for n, r in rs:
            if 'tierra_frac' not in r:
                continue
            franjas = ' / '.join(f'{100 * x:.1f}' for x in r['hipsometria_frac'])
            print(f"{n:<18} {100 * r['tierra_frac']:.1f} % · {r['altura_media_tierra_m']:.0f} / "
                  f"{r['altura_mediana_tierra_m']:.0f} m · [{franjas}] (límites {r['hipsometria_limites_m']} m) · "
                  f"{r['profundidad_media_oceano_m']:.0f} m · {r['lluvia_tierra_mm']:.0f} / {r['lluvia_oceano_mm']:.0f} mm · "
                  f"{100 * r['tierra_w_bajo_frac']:.1f} % · {r['ppn_miami_tierra']:.0f} g/m²/año")
    if any('sotavento' in r for _, r in rs):
        print('\nSombra pluviométrica (volcado): laderas de barlovento (m_oro, lluvia) · sotavento de latitudes medias '
              '(lluvia, desierto) · resto de latitudes medias')
        for n, r in rs:
            if 'sotavento' not in r:
                continue
            s, f = r['sotavento'], r['resto_latitudes_medias']
            print(f"{n:<18} m_oro {r['barlovento_m_oro']:.2f}, {r['barlovento_lluvia_mm']:.0f} mm · "
                  f"{s['lluvia_mm']:.0f} mm, desierto {s['desierto_pct']:.1f} % · {f['lluvia_mm']:.0f} mm, "
                  f"desierto {f['desierto_pct']:.1f} % · w<0,5 {r['tierra_w_bajo_pct']:.1f} % · "
                  f"w p10/p50 {r['w_percentiles'][1]:.2f}/{r['w_percentiles'][2]:.2f}")


if __name__ == '__main__':
    main()
