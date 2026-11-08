#!/usr/bin/env python3
"""Validación del modo Tierra (diseno_modo_tierra.md §10; modo-tierra-diseno.md §3).

Corre el modo Tierra un año, con el volcado por celda, y lo compara con:
- ETOPO 2022 a la misma resolución: `config/referencias/tierra_relieve.toml`, superficie, nivel 3;
- CHELSA V2.1: una estimación independiente de la media sobre la tierra de la malla, con una muestra
  de los píxeles de cada GeoTIFF (1 de cada 40 filas y 1 de cada 8 columnas);
- CRU TS 4.09: la variabilidad por regiones, las rachas de años secos y el Sahel de 1984. Son las
  pruebas `--ignored` de `crates/abm-terreno/tests/tierra.rs`, que esta herramienta corre;
- HydroSHEDS (HydroRIVERS v1.0): las 20 mayores cuencas, su área y su desembocadura; HydroLAKES: el
  área de los lagos; las cuencas cerradas que rebosan; el caudal que llega al mar (Dai y Trenberth
  2002) y el del Amazonas y el Congo (adenda 1 §1);
- MODIS MOD17A3HGF v6.1 (ICDC, 0,5°, media de 2001–2023): la producción vegetal, su mapa, la del
  bosque boreal y la de los desiertos (adenda 1 §2);
- el RGI 7.0: los glaciares por región (adenda 1 §3).

Escribe `output/tierra_validacion/validacion.json` y la tabla en la terminal.

Uso:
    python3 -I herramientas/tierra/validar.py [--sin-correr] [--sin-pruebas] [--sin-chelsa]
Las fuentes se bajan antes con `herramientas/tierra/descargar.sh`. Solo usa la biblioteca estándar.
"""
import argparse
import array
import json
import math
import os
import struct
import subprocess
import sys
import tomllib
import zipfile

RAIZ = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.join(RAIZ, 'herramientas', 'analisis'))
import referencia_tierra as rt  # noqa: E402  (geometría del cubo esférico)
import tiffmini  # noqa: E402

SALIDA = os.path.join(RAIZ, 'output', 'tierra_validacion')
REF = os.path.join(RAIZ, 'output', 'referencias')
N0, NIVELES = 48, 4
DIAS = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
C_MATERIA_SECA = 0.47   # fracción de carbono de la materia seca vegetal (IPCC 2006, valor por defecto)


def log(*a):
    print(*a, file=sys.stderr, flush=True)


def correr():
    """El modo Tierra un año, con el volcado por celda y el diagnóstico ampliado del relieve."""
    subprocess.run(['cargo', 'build', '--release', '--quiet', '--bin', 'terreno'], cwd=RAIZ, check=True)
    subprocess.run([os.path.join(RAIZ, 'target', 'release', 'terreno'), 'config/terreno.toml',
                    '--set', 'mundo.modo=tierra', '--set', 'run.name=tierra',
                    '--set', 'run.output_path=output/tierra_validacion', '--set', 'run.dump_world=true',
                    '--set', 'run.dump_climate=true', '--set', 'run.diagnostico_relieve=true'],
                   cwd=RAIZ, check=True)


def pruebas():
    """Las pruebas con las fuentes: variabilidad por regiones y el Sahel de 1984."""
    subprocess.run(['cargo', 'test', '--release', '-p', 'abm-terreno', '--test', 'tierra', '--',
                    '--ignored', '--nocapture'], cwd=RAIZ, check=True)


class Malla:
    """Las celdas del volcado y un índice (nivel, celda de la tabla del nivel) → posición."""

    def __init__(self, celdas):
        self.c = celdas
        self.indice = {}
        for q, c in enumerate(celdas):
            lv = c['level']
            la, lo = math.radians(c['lat_deg']), math.radians(c['lon_deg'])
            k = rt.localizar(math.cos(la) * math.cos(lo), math.cos(la) * math.sin(lo), math.sin(la), N0 << lv)
            self.indice[(lv, k)] = q

    def celda(self, lat_g, lon_g):
        la, lo = math.radians(lat_g), math.radians(lon_g)
        x, y, z = math.cos(la) * math.cos(lo), math.cos(la) * math.sin(lo), math.sin(la)
        for lv in range(NIVELES):
            q = self.indice.get((lv, rt.localizar(x, y, z, N0 << lv)))
            if q is not None:
                return q
        return -1


def tierra(c):
    return c['surface'] != 'oceano'


# --- 1. Relieve ------------------------------------------------------------------------------------

def relieve(doc):
    ref = tomllib.load(open(os.path.join(RAIZ, 'config', 'referencias', 'tierra_relieve.toml'), 'rb'))
    r3 = ref['relieve']['superficie']['nivel3']
    dg = doc['diagnostico']
    t = dg['tierra']
    filas = [
        ('Fracción de tierra', dg['tierra_frac'], r3['tierra_frac'], 0.01),
        ('Altura media de la tierra (m)', dg['altura_media_tierra_m'], r3['altura_media_tierra_m'], 0.01),
    ]
    for k, (m, e) in enumerate(zip(dg['hipsometria_frac'], r3['hipsometria_frac'])):
        filas.append((f'Hipsometría, franja {k}', m, e, None))
    return {
        'filas': filas,
        'hipsometria_max_dif': max(abs(m - e) for m, e in zip(dg['hipsometria_frac'], r3['hipsometria_frac'])),
        'conservacion_niveles_rel': t['conservacion_niveles_rel'],
        'conservacion_malla_rel': t['conservacion_malla_rel'],
        'hielo_frac': t['hielo_frac'],
        'hielo_mkm2': t['hielo_frac'] * 4 * math.pi * 6371.0 ** 2 / 1e6,
    }


# --- 2. Clima: CHELSA a la misma resolución ---------------------------------------------------------

def chelsa(malla):
    """Media de la temperatura anual y de la lluvia anual sobre la tierra de la malla, con una muestra
    de los píxeles de CHELSA (cada píxel pesa cos φ)."""
    dirs = os.path.join(REF, 'chelsa_mensual')
    rutas = [os.path.join(dirs, f'CHELSA_{v}_{m:02d}_1981-2010_V.2.1.tif') for v in ('tas', 'pr')
             for m in range(1, 13)]
    with open(rutas[0], 'rb') as f:
        t0 = tiffmini.GeoTiff(tiffmini.leer_memoria(f.read()))
    filas = list(range(20, t0.alto, 40))
    cols = list(range(4, t0.ancho, 8))
    log(f'CHELSA: {len(filas)} filas × {len(cols)} columnas por archivo; celdas de la muestra…')
    celda, peso = [], []
    for r in filas:
        lat = t0.lat0 - (r + 0.5) * t0.paso_lat
        w = math.cos(math.radians(lat))
        for c in cols:
            q = malla.celda(lat, t0.lon0 + (c + 0.5) * t0.paso_lon)
            celda.append(q)
            peso.append(w if q >= 0 and tierra(malla.c[q]) else 0.0)
    n = len(celda)
    t_anual = array.array('d', bytes(8 * n))
    p_anual = array.array('d', bytes(8 * n))
    valido = bytearray(b'\x01' * n)
    for k, ruta in enumerate(rutas):
        with open(ruta, 'rb') as f:
            g = tiffmini.GeoTiff(tiffmini.leer_memoria(f.read()))
        es_t = k < 12
        i = 0
        for r in filas:
            fila = g.fila(r)
            for c in cols:
                x = fila[c]
                if x == 65535 or (es_t and x == 0):
                    valido[i] = 0
                elif es_t:
                    t_anual[i] += (x * g.escala + g.desplazamiento) * DIAS[k] / 365.0
                else:
                    p_anual[i] += x * g.escala + g.desplazamiento
                i += 1
        log(f'  {os.path.basename(ruta)}')
    st = sp = sw = 0.0
    for i in range(n):
        if peso[i] > 0 and valido[i]:
            st += t_anual[i] * peso[i]
            sp += p_anual[i] * peso[i]
            sw += peso[i]
    return {'temp_tierra_c': st / sw, 'lluvia_tierra_mm': sp / sw, 'muestras_tierra': sum(1 for x in peso if x > 0)}


# --- 4. Ríos: HydroRIVERS -----------------------------------------------------------------------------

def dbf_campos(b):
    n_reg, cab, tam = struct.unpack('<IHH', b[4:12])
    campos, p, desde = {}, 32, 1
    while b[p] != 0x0D:
        nombre = b[p:p + 11].split(b'\0')[0].decode()
        ancho = b[p + 16]
        campos[nombre] = (desde, ancho)
        desde += ancho
        p += 32
    return n_reg, cab, tam, campos


def hydrorivers(ruta_zip, n_rios=20):
    """Las `n_rios` desembocaduras (tramos sin tramo aguas abajo) de mayor cuenca: (lat, lon, km²,
    endorreica) del final del tramo."""
    z = zipfile.ZipFile(ruta_zip)
    nombres = {os.path.splitext(os.path.basename(i.filename))[1].lower(): i.filename for i in z.infolist()}
    log('HydroRIVERS: tabla de atributos…')
    b = z.read(nombres['.dbf'])
    n_reg, cab, tam, campos = dbf_campos(b)
    (d_nd, a_nd), (d_up, a_up), (d_en, a_en) = campos['NEXT_DOWN'], campos['UPLAND_SKM'], campos['ENDORHEIC']
    salidas = []
    for k in range(n_reg):
        p = cab + k * tam
        if int(b[p + d_nd:p + d_nd + a_nd]) == 0:
            salidas.append((float(b[p + d_up:p + d_up + a_up]), k, int(b[p + d_en:p + d_en + a_en])))
    del b
    salidas.sort(reverse=True)
    elegidas = salidas[:n_rios]
    log('HydroRIVERS: geometría de las desembocaduras…')
    shx = z.read(nombres['.shx'])
    shp = z.open(nombres['.shp'])
    out = []
    for area, k, endo in sorted(elegidas, key=lambda x: x[1]):
        off = struct.unpack('>I', shx[100 + 8 * k:104 + 8 * k])[0] * 2
        shp.seek(off)
        rec = shp.read(8 + 44)
        n_partes, n_puntos = struct.unpack('<II', rec[8 + 36:8 + 44])
        datos = shp.read(4 * n_partes + 16 * n_puntos)
        x, y = struct.unpack('<dd', datos[4 * n_partes + 16 * (n_puntos - 1):])
        out.append({'lat': y, 'lon': x, 'area_km2': area, 'endorreica': bool(endo)})
    out.sort(key=lambda r: -r['area_km2'])
    return out


def distancia_km(la1, lo1, la2, lo2):
    a, b = math.radians(la1), math.radians(la2)
    d = math.sin((b - a) / 2) ** 2 + math.cos(a) * math.cos(b) * math.sin(math.radians(lo2 - lo1) / 2) ** 2
    return 2 * 6371.0 * math.asin(min(1.0, math.sqrt(d)))


def rios(malla, reales, radio_km=150.0):
    """Para cada río real: la celda de tierra del modelo con más área de drenaje a menos de `radio_km`
    de su desembocadura (si el río del modelo llega ahí, es su cuenca), con su caudal."""
    filas = []
    for r in reales:
        mejor, caudal = 0.0, 0.0
        for c in malla.c:
            if not tierra(c) or abs(c['lat_deg'] - r['lat']) > 2.5:
                continue
            if distancia_km(c['lat_deg'], c['lon_deg'], r['lat'], r['lon']) <= radio_km \
                    and c.get('drenaje_km2', 0.0) > mejor:
                mejor, caudal = c['drenaje_km2'], c.get('caudal_km3_anio', 0.0)
        filas.append({**r, 'modelo_km2': mejor, 'cociente': mejor / r['area_km2'], 'caudal_km3_anio': caudal})
    return filas


# Desembocaduras de referencia para el caudal (adenda 1 §1: del orden de 6 000 y 1 300 km³/año).
CAUDALES = {'Amazonas': (-0.5, -50.0, 6000.0), 'Congo': (-6.0, 12.4, 1300.0)}


def caudal_cerca(malla, lat, lon, radio_km=200.0):
    """El mayor caudal (km³/año) del modelo a menos de `radio_km` de un punto."""
    m = 0.0
    for c in malla.c:
        if tierra(c) and abs(c['lat_deg'] - lat) < 3 and distancia_km(c['lat_deg'], c['lon_deg'], lat, lon) <= radio_km:
            m = max(m, c.get('caudal_km3_anio', 0.0))
    return m


# Cajas de las regiones de glaciares (adenda 1 §3) y del bosque boreal.
GLACIARES = {'Patagonia': (-56, -45, -76, -70), 'Karakórum': (34, 37.5, 73, 78), 'Alaska': (56, 66, -156, -132),
             'Alpes': (44, 48.5, 5, 15), 'Svalbard': (76, 81, 10, 34), 'Ártico canadiense': (74, 84, -100, -72),
             'Islandia': (63, 67, -25, -13), 'Himalaya central': (27, 31, 80, 89)}


def glaciares(malla, ruta_rgi):
    import csv
    rgi = {k: 0.0 for k in GLACIARES}
    total = 0.0
    with open(ruta_rgi) as f:
        for r in csv.DictReader(f):
            a = float(r['area_km2'])
            total += a
            la, lo = float(r['cenlat']), float(r['cenlon'])
            for k, (a0, a1, o0, o1) in GLACIARES.items():
                if a0 <= la < a1 and o0 <= lo < o1:
                    rgi[k] += a
    mod = {k: 0.0 for k in GLACIARES}
    for c in malla.c:
        h = c.get('hielo', 0.0)
        for k, (a0, a1, o0, o1) in GLACIARES.items():
            if h > 0 and a0 <= c['lat_deg'] < a1 and o0 <= c['lon_deg'] < o1:
                mod[k] += h * c['area_km2']
    return {'rgi_total_km2': total, 'regiones': {k: {'rgi_km2': rgi[k], 'malla_km2': mod[k]} for k in GLACIARES}}


# --- 5. Vegetación: MODIS ------------------------------------------------------------------------------

def netcdf3_var(b, nombre):
    """(forma, tipo, inicio, atributos) de una variable de un NetCDF-3 en memoria."""
    p = 8

    def i32():
        nonlocal p
        x = struct.unpack('>i', b[p:p + 4])[0]
        p += 4
        return x

    def nom():
        nonlocal p
        n = i32()
        s = b[p:p + n].decode()
        p += (n + 3) // 4 * 4
        return s
    tam = {1: 1, 2: 1, 3: 2, 4: 4, 5: 4, 6: 8}
    fmt = {1: 'b', 3: 'h', 4: 'i', 5: 'f', 6: 'd'}

    def atts():
        nonlocal p
        i32()
        out = {}
        for _ in range(i32()):
            k = nom()
            t = i32()
            m = i32()
            raw = b[p:p + m * tam[t]]
            p += (m * tam[t] + 3) // 4 * 4
            out[k] = raw.decode('latin-1') if t == 2 else struct.unpack('>' + fmt[t] * m, raw)
        return out
    i32()                       # etiqueta de la lista de dimensiones (o 0 si no hay)
    n_dims = i32()
    dims = [(nom(), i32()) for _ in range(n_dims)]
    atts()
    i32()
    for _ in range(i32()):
        nm = nom()
        ids = [i32() for _ in range(i32())]
        a = atts()
        t = i32()
        i32()
        ini = i32()
        if nm == nombre:
            return [dims[i][1] for i in ids], t, ini, a
    raise KeyError(nombre)


PIXEL_MODIS_KM2 = 0.46331271653 ** 2   # píxel de la rejilla sinusoidal de 500 m de MODIS


def modis():
    """Media de 2001–2023 de la NPP de MODIS (kg C m⁻² año⁻¹) en la rejilla de 0,5°, el área media de
    sus píxeles válidos de 500 m (km²), y la latitud y la longitud de sus filas y columnas."""
    z = zipfile.ZipFile(os.path.join(REF, 'modis_npp', 'modis_npp_05.zip'))
    suma = cuenta = area = None
    lat = lon = None
    for info in sorted(z.infolist(), key=lambda i: i.filename):
        b = z.read(info)
        forma, t, ini, a = netcdf3_var(b, 'npp')
        n = forma[-1] * forma[-2]
        v = array.array('f', b[ini:ini + 4 * n])
        v.byteswap()
        _, _, ini_v, _ = netcdf3_var(b, 'numberofvalidvalues')
        nv = array.array('i', b[ini_v:ini_v + 4 * n])
        nv.byteswap()
        relleno = a.get('_FillValue', (None,))[0]
        if suma is None:
            suma, cuenta, area = [0.0] * n, [0] * n, [0.0] * n
            fl, tl, il, _ = netcdf3_var(b, 'lat')
            lat = array.array('f', b[il:il + 4 * fl[0]])
            lat.byteswap()
            fo, to, io, _ = netcdf3_var(b, 'lon')
            lon = array.array('f', b[io:io + 4 * fo[0]])
            lon.byteswap()
            unidades = a.get('units', '')
        for k, x in enumerate(v):
            if math.isfinite(x) and x != relleno and 0.0 <= x < 10.0:  # kg C m⁻² año⁻¹
                suma[k] += x
                cuenta[k] += 1
                area[k] += nv[k] * PIXEL_MODIS_KM2
    media = [s / c if c >= 12 else float('nan') for s, c in zip(suma, cuenta)]
    area = [s / c if c >= 12 else 0.0 for s, c in zip(area, cuenta)]
    return media, area, list(lat), list(lon), unidades


def miami(t, p):
    f_t = 3000.0 / (1.0 + math.exp(1.315 - 0.119 * t))
    f_p = 3000.0 * (1.0 - math.exp(-0.000664 * p))
    return max(0.0, min(f_t, f_p))


def pearson(x, y):
    n = len(x)
    mx, my = sum(x) / n, sum(y) / n
    sxy = sum((a - mx) * (b - my) for a, b in zip(x, y))
    sxx = sum((a - mx) ** 2 for a in x)
    syy = sum((b - my) ** 2 for b in y)
    return sxy / math.sqrt(sxx * syy)


def vegetacion(malla):
    npp, area_valida, lat, lon, unidades = modis()
    nlat, nlon = len(lat), len(lon)
    total_modis = total_modis_valido = 0.0
    pares_veg, pares_miami = [], []
    for i in range(nlat):
        a_km2 = (math.radians(0.5) * 6371.0) ** 2 * math.cos(math.radians(lat[i]))
        for j in range(nlon):
            x = npp[i * nlon + j]
            if not math.isfinite(x):
                continue
            total_modis += x * a_km2 * 1e6 / 1e12        # kg C → Pg C, con la celda entera
            total_modis_valido += x * area_valida[i * nlon + j] * 1e6 / 1e12   # con sus píxeles válidos
            q = malla.celda(lat[i], lon[j])
            if q < 0:
                continue
            c = malla.c[q]
            if not tierra(c) or c.get('hielo', 0.0) >= 0.5:
                continue
            pares_veg.append((c['veg'], x))
            pares_miami.append((miami(c['temp_anual_c'], c['lluvia_anual_mm']), x))
    total_miami = 0.0
    for c in malla.c:
        if tierra(c) and c.get('hielo', 0.0) < 0.5:
            total_miami += miami(c['temp_anual_c'], c['lluvia_anual_mm']) * c['area_km2'] * 1e6 * C_MATERIA_SECA / 1e15
    # Bosque boreal (50–70° N) y desiertos (lluvia de la climatología por debajo de 250 mm): producción
    # del modelo (la vegetación del modo Tierra, en g de materia seca, a carbono) frente a MODIS, en las
    # mismas celdas de 0,5°.
    boreal = [0.0, 0.0]
    desierto = [0.0, 0.0, 0]
    for i in range(nlat):
        for j in range(nlon):
            x = npp[i * nlon + j]
            if not math.isfinite(x):
                continue
            q = malla.celda(lat[i], lon[j])
            if q < 0:
                continue
            c = malla.c[q]
            if not tierra(c) or c.get('hielo', 0.0) >= 0.5:
                continue
            a = area_valida[i * nlon + j]
            if 50.0 <= lat[i] < 70.0:
                boreal[0] += c['veg'] * C_MATERIA_SECA / 1000.0 * a
                boreal[1] += x * a
            if c['lluvia_anual_mm'] < 250.0:
                desierto[0] += c['veg'] * C_MATERIA_SECA
                desierto[1] += x * 1000.0
                desierto[2] += 1
    return {
        'modis_total_pgc': total_modis_valido,
        'modis_total_celda_entera_pgc': total_modis,
        'modis_unidades': unidades,
        'miami_climatologia_total_pgc': total_miami,
        'r_veg_motor_modis': pearson([a for a, _ in pares_veg], [b for _, b in pares_veg]),
        'r_miami_modis': pearson([a for a, _ in pares_miami], [b for _, b in pares_miami]),
        'boreal_modelo_frente_a_modis': boreal[0] / boreal[1] if boreal[1] > 0 else float('nan'),
        'desiertos_modelo_gc_m2': desierto[0] / max(desierto[2], 1),
        'desiertos_modis_gc_m2': desierto[1] / max(desierto[2], 1),
        'celdas': len(pares_veg),
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--sin-correr', action='store_true', help='usa la corrida que ya está en output/tierra_validacion')
    ap.add_argument('--sin-pruebas', action='store_true', help='no corre las pruebas con las fuentes (usa sus archivos)')
    ap.add_argument('--sin-chelsa', action='store_true', help='sin la estimación independiente de CHELSA (tarda ~10 min)')
    args = ap.parse_args()
    os.makedirs(SALIDA, exist_ok=True)
    if not args.sin_correr:
        correr()
    if not args.sin_pruebas:
        pruebas()
    doc = json.load(open(os.path.join(SALIDA, 'tierra_terreno.json')))
    malla = Malla(json.load(open(os.path.join(SALIDA, 'tierra_world.json'))))
    out = {'relieve': relieve(doc)}
    t = doc['diagnostico']['tierra']
    out['clima_motor'] = {'temp_tierra_c': t['temp_tierra_climatologia_c'],
                          'lluvia_tierra_mm': t['lluvia_tierra_climatologia_mm']}
    previo = os.path.join(SALIDA, 'validacion.json')
    if not args.sin_chelsa:
        out['clima_chelsa'] = chelsa(malla)
    elif os.path.exists(previo) and 'clima_chelsa' in json.load(open(previo)):
        out['clima_chelsa'] = json.load(open(previo))['clima_chelsa']   # la de la última vez
    out['variabilidad'] = json.load(open(os.path.join(SALIDA, 'variabilidad_l10.json')))['regiones']
    for r in out['variabilidad'].values():
        r.pop('serie_cru_mm', None)
        r.pop('serie_malla_por_anio_mm', None)
    out['sahel_1984'] = json.load(open(os.path.join(SALIDA, 'sahel_1984.json')))
    hr = os.path.join(REF, 'hydrosheds', 'HydroRIVERS_v10_shp.zip')
    if os.path.exists(hr) and zipfile.is_zipfile(hr):
        out['rios'] = rios(malla, hydrorivers(hr))
    else:
        out['rios'] = None
        log('Sin HydroRIVERS: la validación de los ríos queda pendiente.')
    out['cuencas_modelo'] = t['cuencas'][:30]
    out['hidrologia'] = {k: t[k] for k in ('lagos_km2', 'lagos_fraccion_km2', 'lagos_hydrolakes_km2', 'al_mar_km3_anio',
                                           'a_cerradas_km3_anio', 'cerradas_que_rebosan_km2',
                                           'abiertas_que_acaban_cerradas_km2')}
    out['hidrologia']['caudales'] = {k: {'modelo_km3_anio': caudal_cerca(malla, la, lo), 'referencia_km3_anio': ref}
                                     for k, (la, lo, ref) in CAUDALES.items()}
    out['vegetacion'] = vegetacion(malla)
    out['vegetacion']['produccion_modelo_pgc'] = t['produccion_pgc']
    out['glaciares'] = glaciares(malla, os.path.join(REF, 'rgi7', 'RGI2000-v7.0-G-global-attributes.csv'))
    out['glaciares']['hielo_total_mkm2'] = out['relieve']['hielo_mkm2']
    with open(os.path.join(SALIDA, 'validacion.json'), 'w') as f:
        json.dump(out, f, indent=1, ensure_ascii=False)

    # Tabla.
    rel = out['relieve']
    print('== Relieve (ETOPO 2022 superficie, nivel 3)')
    for nombre, m, e, tol in rel['filas']:
        ok = '' if tol is None else ('✓' if abs(m / e - 1) <= tol else '✗')
        print(f'  {nombre:<34} {m:10.4f} {e:10.4f} {ok}')
    print(f"  hipsometría: diferencia máxima {rel['hipsometria_max_dif']:.4f}")
    print(f"  conservación: entre niveles {rel['conservacion_niveles_rel']:.1e}, malla {rel['conservacion_malla_rel']:.1e}")
    print(f"  hielo: {rel['hielo_mkm2']:.2f} Mkm²")
    cm = out['clima_motor']
    print('== Clima sobre la tierra (motor frente a CHELSA a la misma resolución)')
    if 'clima_chelsa' in out:
        ch = out['clima_chelsa']
        print(f"  temperatura {cm['temp_tierra_c']:.3f} °C frente a {ch['temp_tierra_c']:.3f}")
        print(f"  lluvia {cm['lluvia_tierra_mm']:.1f} mm frente a {ch['lluvia_tierra_mm']:.1f} "
              f"({cm['lluvia_tierra_mm'] / ch['lluvia_tierra_mm'] - 1:+.2%})")
    print('== Variabilidad (años con estaciones): CRU TS / modelo; rachas de ≥2 años secos y del quinto seco (p de Poisson)')
    for k, r in out['variabilidad'].items():
        c, m = r['cru'], r['modelo_remuestreado']
        print(f"  {k:<22} CV {c['cv']:.3f}/{m['cv']:.3f} ({m['cv'] / c['cv'] - 1:+.0%})  "
              f"ac1 {c['ac1']:+.3f}/{m['ac1']:+.3f}  rachas {c['rachas_secas']}/{r['rachas_secas_esperadas_en_el_periodo']:.2f} "
              f"(p {r['rachas_secas_p_poisson']:.2f})  quinto {r['rachas_quinto_seco_cru']}/{r['rachas_quinto_seco_esperadas']:.2f} "
              f"(p {r['rachas_quinto_seco_p_poisson']:.2f})")
    s = out['sahel_1984']
    print(f"== Sahel 1984: malla {s['malla_1984_mm']:.0f} mm ({s['malla_1984_mm'] / s['malla_climatologia_mm']:.3f}), "
          f"CRU TS {s['cru_1984_mm']:.0f} mm ({s['cru_1984_mm'] / s['cru_1981_2010_mm']:.3f})")
    if out['rios']:
        print('== Ríos (HydroRIVERS): cuenca real, del modelo cerca de la desembocadura')
        for r in out['rios']:
            print(f"  ({r['lat']:6.2f}, {r['lon']:7.2f}) {r['area_km2']:10.0f} {r['modelo_km2']:10.0f} {r['cociente']:5.2f}"
                  f" {'✓' if abs(r['cociente'] - 1) <= 0.2 else '✗'}{' (endorreica)' if r['endorreica'] else ''}")
    h = out['hidrologia']
    print(f"== Lagos: {h['lagos_fraccion_km2'] / 1e6:.2f} Mkm² en la malla frente a {h['lagos_hydrolakes_km2'] / 1e6:.2f} de HydroLAKES "
          f"({h['lagos_fraccion_km2'] / h['lagos_hydrolakes_km2'] - 1:+.1%}); celdas de lago {h['lagos_km2'] / 1e6:.2f} Mkm²")
    print(f"== Cuencas cerradas que rebosan: {h['cerradas_que_rebosan_km2']:.0f} km²; abiertas que acaban cerradas: "
          f"{h['abiertas_que_acaban_cerradas_km2']:.0f} km²")
    print(f"== Caudal al mar: {h['al_mar_km3_anio']:.0f} km³/año (Dai y Trenberth: 37 000–40 000); a cuencas cerradas, "
          f"{h['a_cerradas_km3_anio']:.0f}; " + ', '.join(f"{k} {x['modelo_km3_anio']:.0f} (~{x['referencia_km3_anio']:.0f})"
                                                       for k, x in h['caudales'].items()))
    g = out['glaciares']
    print(f"== Glaciares: RGI {g['rgi_total_km2'] / 1e6:.3f} Mkm²; hielo total {g['hielo_total_mkm2']:.2f} Mkm²; " +
          ', '.join(f"{k} {x['malla_km2'] / max(x['rgi_km2'], 1) - 1:+.0%}" for k, x in g['regiones'].items()))
    v = out['vegetacion']
    print(f"== Vegetación: modelo {v['produccion_modelo_pgc']:.1f} PgC/año; MODIS {v['modis_total_pgc']:.1f} (con la celda de 0,5° entera, "
          f"{v['modis_total_celda_entera_pgc']:.1f}); boreal, modelo/MODIS {v['boreal_modelo_frente_a_modis']:.2f}; "
          f"desiertos, {v['desiertos_modelo_gc_m2']:.0f} frente a {v['desiertos_modis_gc_m2']:.0f} gC/m²/año; "
          f"r(veg del motor, MODIS) = {v['r_veg_motor_modis']:.3f}; r(Miami, MODIS) = {v['r_miami_modis']:.3f}")
    print(f'Resultados -> {os.path.join(SALIDA, "validacion.json")}')


if __name__ == '__main__':
    main()
