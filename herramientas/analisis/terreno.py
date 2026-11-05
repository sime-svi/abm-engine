#!/usr/bin/env python3
"""Medidas de una corrida del planeta (binario `terreno`).

Lee `<dir>/<nombre>_terreno.json` y, si existe, el volcado por celda `<nombre>_world.json` (corrida
con `run.dump_world = true` y `run.dump_climate = true`). Las medidas de la salida JSON, como los
biomas, el diagnóstico del planeta y `c_P`, están siempre. Las que necesitan el volcado, como la
sombra pluviométrica o los percentiles de `w` y `m_oro`, solo con él. Lo usan la batería de
escenarios (`herramientas/escenarios.py`) y quien quiera comparar variantes del planeta.

Uso: herramientas/analisis/terreno.py DIR NOMBRE [NOMBRE...]
     (la primera corrida es la referencia de los Δ; el relieve de la sombra sale de ella)
"""
import json
import math
import sys

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
    r.update(doc.get('diagnostico') or {})
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


def resumen_celdas(celdas, rel):
    """Medidas del volcado por celda: lluvia, `w`, `m_oro`, sombra pluviométrica."""
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
    # Laderas de barlovento de las cordilleras grandes: suben con el viento (m_oro > 1), h ≥ 300 m y
    # cresta > 2000 m a 0–150 km a favor del viento.
    lad = [c for c in tierra if c['m_oro'] > 1.0 and c['h'] >= 300.0 and _hay_cresta(c, rel, 0.0, 150.0, +1)]
    al = sum(c['area_km2'] for c in lad)
    r['barlovento_m_oro'] = sum(c['m_oro'] * c['area_km2'] for c in lad) / al if al else float('nan')
    r['barlovento_lluvia_mm'] = sum(c['lluvia_anual_mm'] * c['area_km2'] for c in lad) / al if al else float('nan')
    # Sotavento en latitudes medias: tierra < 2000 m con cresta > 2000 m a 200–500 km a barlovento.
    banda = [c for c in tierra if 30.0 <= abs(c['lat_deg']) < 60.0 and c['h'] < 2000.0]
    marca = [_hay_cresta(c, rel, 200.0, 500.0, -1) for c in banda]
    dentro = [c for c, m in zip(banda, marca) if m]
    fuera = [c for c, m in zip(banda, marca) if not m]

    def zona(cs):
        ar = sum(c['area_km2'] for c in cs)
        if ar == 0:
            return {'area_km2': 0.0, 'lluvia_mm': float('nan'), 'desierto_pct': float('nan')}
        des = sum(c['area_km2'] for c in cs if c['biome'] == 'desierto')
        return {'area_km2': ar, 'lluvia_mm': sum(c['lluvia_anual_mm'] * c['area_km2'] for c in cs) / ar,
                'desierto_pct': 100.0 * des / ar}
    r['sotavento'] = zona(dentro)
    r['resto_latitudes_medias'] = zona(fuera)
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
