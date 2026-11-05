#!/usr/bin/env python3
"""Lluvia observada de la Tierra a la escala del planeta (~26 km): el objetivo de la lluvia de
barlovento y la revisión de `c_P` (decisión del responsable, 2026-11-05).

Por qué: el ×1,36 de `referencia_tierra.py` es la regla de lluvia del modelo aplicada al relieve real,
no lluvia observada. El objetivo tiene que salir de lluvia medida, a la misma escala y con la misma
definición de barlovento que el planeta, para comparar lo mismo con lo mismo.

Qué mide, en las celdas de nivel 3 del cubo esférico (~26 km) de `referencia_tierra.py`, con su relieve
(la roca rebotada sin hielo) y la lluvia observada media de los píxeles de tierra de cada celda:
  a. la lluvia de las laderas de barlovento de las cordilleras grandes, con la misma selección que
     `referencia_tierra.pendientes` (m_oro > 1, h ≥ 300 m y cresta > 2000 m a 0–150 km a favor del
     viento del modelo);
  b. la lluvia del sotavento de las cordilleras grandes en latitudes medias y la del resto de las
     latitudes medias, con las definiciones de `terreno.py`;
  c. el factor de lluvia de barlovento observado, lo que en el modelo es m_oro: la lluvia de cada celda
     de barlovento entre la lluvia que tendría sin la montaña, estimada de dos maneras:
     i.  la de la tierra baja (h < 300 m) a barlovento, a 50–300 km contra el viento por el mismo
         paralelo (la media de los puntos cada 10 km que caen en tierra baja); las celdas sin tierra
         baja a barlovento (por ejemplo, con el mar delante) se omiten y se da el área omitida;
     ii. la media de la tierra baja (h < 300 m) de su franja de 1° de latitud;
  d. la lluvia media de la tierra, con y sin la Antártida y Groenlandia, y por franjas de 10°.
Después mide exactamente lo mismo en el volcado por celda del planeta (`run.dump_world` y
`run.dump_climate`), con la lluvia del modelo, y lo muestra al lado. El planeta no va al TOML: cambia
con cada corrida.

Datos (se descargan una vez a output/referencias/, ignorado por git; son datos no fiables: solo se
interpretan sus bytes):
- WorldClim 2.1 (Fick y Hijmans, 2017): lluvia mensual media de 1970–2000 a 10' (~18 km); la anual es la
  suma de los 12 meses. Cada píxel se parte en 4×4 trozos y cada trozo va a la celda que contiene su
  centro. Interpola pluviómetros, que escasean en las montañas: se sabe que ahí se queda corto.
- CHELSA 2.1 (Karger et al., 2017): bio12, lluvia anual de 1981–2010 a 30'' (~1 km). Modela el efecto
  del relieve (viento y orografía). No tiene reducciones internas, así que se lee 1 de cada 8 filas
  (cada 4') por peticiones parciales y se promedia de 4 en 4 columnas (2', las columnas de ETOPO),
  solo donde ETOPO dice que hay tierra. Satura en 6553,5 mm: se da el área afectada.
- El relieve, de referencia_tierra.py (ETOPO 2022 y Paxman et al., 2022), en caché en output/cache/.

Uso: python3 -I herramientas/analisis/lluvia_observada.py [--planeta output/escenarios/planeta] [--sin-chelsa]
Escribe config/referencias/tierra_lluvia.toml (solo la Tierra) y muestra la tabla Tierra–planeta. Solo usa
la biblioteca estándar de Python. Tarda ~30 s con los datos y las cachés hechos; la primera vez, unos
minutos más: el relieve de nivel 3 (~1 min en el equipo del 2026-11-05) y la lectura de CHELSA (~2 min).
"""
import argparse
import array
import concurrent.futures
import datetime
import hashlib
import inspect
import json
import math
import os
import struct
import sys
import time
import zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import referencia_tierra as rt  # noqa: E402
import terreno  # noqa: E402
import tiffmini  # noqa: E402

RAIZ = rt.RAIZ
R_KM = rt.R_KM
N3 = rt.N3
AREA_TIERRA_KM2 = 4.0 * math.pi * R_KM ** 2

WORLDCLIM = 'https://geodata.ucdavis.edu/climate/worldclim/2_1/base/wc2.1_10m_prec.zip'
CHELSA = ('https://os.zhdk.cloud.switch.ch/chelsav2/GLOBAL/climatologies/1981-2010/bio/'
          'CHELSA_bio12_1981-2010_V.2.1.tif')
SUB_WC = 4                       # cada píxel de 10' se parte en 4×4 trozos al repartirlo entre las celdas
FILAS_CH, COLS_CH = 8, 4         # CHELSA: 1 de cada 8 filas de 30'' y bloques de 4 columnas (2')
SATURADO_CH = 65535              # valor máximo de CHELSA bio12 (6553,5 mm): la lluvia real puede ser mayor
H_BAJA = 300.0                   # tierra baja: h < 300 m (el umbral de las laderas de barlovento)
D_MIN, D_MAX, D_PASO = 50.0, 300.0, 10.0    # tierra baja a barlovento: de 50 a 300 km, cada 10 km
FRANJA_REF = 1.0                 # franja de latitud de la referencia ii (grados)
FRANJAS = list(range(-90, 90, 10))           # límites sur de las franjas de 10° del resumen

# Groenlandia: polígono (lon, lat) por el centro de los estrechos que la separan de las islas de Baffin
# y Ellesmere (estrechos de Davis y Nares, bahía de Baffin) y de Islandia (estrecho de Dinamarca).
GROENLANDIA = [(-44.0, 58.5), (-52.0, 60.0), (-57.5, 64.5), (-58.0, 67.0), (-62.0, 70.5), (-66.0, 74.0),
               (-75.5, 77.0), (-73.8, 78.3), (-70.5, 79.5), (-66.5, 80.8), (-63.0, 81.6), (-59.0, 82.3),
               (-57.0, 83.2), (-40.0, 84.2), (-20.0, 84.0), (-8.0, 82.0), (-8.0, 78.0), (-14.0, 73.0),
               (-18.0, 70.0), (-28.5, 66.5), (-33.0, 64.0), (-40.0, 60.5)]


def log(*a):
    print(*a, file=sys.stderr, flush=True)


def en_poligono(lon, lat, pol):
    dentro = False
    for (x1, y1), (x2, y2) in zip(pol, pol[1:] + pol[:1]):
        if (y1 > lat) != (y2 > lat) and lon < x1 + (lat - y1) * (x2 - x1) / (y2 - y1):
            dentro = not dentro
    return dentro


def antartida_o_groenlandia(lat, lon):
    """La Antártida es la tierra al sur de 60° S; Groenlandia, la de dentro de su polígono."""
    return lat < -60.0 or (lat > 58.0 and en_poligono(lon, lat, GROENLANDIA))


# --- El relieve (referencia_tierra.py), en caché ----------------------------------------------------

def relieve_roca_rebote(datos, cache):
    """Celdas de nivel 3 de la roca rebotada sin hielo (`roca_rebote`), las mismas que usa
    `referencia_tierra.pendientes`. La primera vez se calculan con referencia_tierra.py y se guardan
    en `cache`; la clave es la huella del código que las calcula y el tamaño de los datos."""
    ficheros = ['etopo2022_superficie_2m.f32be', 'etopo2022_roca_sur_2m.f32be', 'etopo2022_roca_norte_2m.f32be',
                'paxman2022v3_antartida_5km.bin', 'paxman2022v3_groenlandia_4p5km.bin']
    if not all(os.path.exists(os.path.join(datos, f)) for f in ficheros):
        log('Faltan los datos del relieve: se descargan con referencia_tierra.py…')
        rt.descargar(datos)
    h = hashlib.sha256()
    for f in (rt.cara_y_coords, rt.localizar, rt.Celdas, rt.acumular_etopo, rt.leer_paxman, rt.estereo_inversa,
              rt.sustituir_paxman, rt.relieve_nivel3):
        h.update(inspect.getsource(f).encode())
    h.update(repr((rt.N3, rt.PASO_ETOPO, rt.COLS, rt.F_SUR, rt.F_NORTE)).encode())
    for f in ficheros:
        h.update(f'{f}:{os.path.getsize(os.path.join(datos, f))}'.encode())
    ruta = os.path.join(cache, f'tierra_roca_rebote_n3_{h.hexdigest()[:16]}.bin')
    cel = rt.Celdas(N3)
    m = len(cel.w)
    if os.path.exists(ruta):
        with open(ruta, 'rb') as f:
            if f.read(8) != b'ABMREL01' or struct.unpack('<I', f.read(4))[0] != N3:
                raise ValueError(f'caché del relieve con otro formato: {ruta}')
            cel.w = array.array('d')
            cel.w.fromfile(f, m)
            cel.zw = array.array('d')
            cel.zw.fromfile(f, m)
        log(f'Relieve de nivel 3 leído de la caché {os.path.relpath(ruta, RAIZ)}')
        return cel
    cel3, _, _ = rt.relieve_nivel3(datos)
    cel = cel3['roca_rebote']
    os.makedirs(cache, exist_ok=True)
    with open(ruta + '.tmp', 'wb') as f:
        f.write(b'ABMREL01' + struct.pack('<I', N3))
        cel.w.tofile(f)
        cel.zw.tofile(f)
    os.replace(ruta + '.tmp', ruta)
    log(f'Relieve de nivel 3 guardado en la caché {os.path.relpath(ruta, RAIZ)}')
    return cel


# --- WorldClim 2.1 ---------------------------------------------------------------------------------

def worldclim_celdas(directorio):
    """Lluvia anual de WorldClim 2.1 (suma de los 12 meses; sin dato si falta alguno) en las celdas de
    nivel 3: (suma de pesos, suma de lluvia × peso) por celda, con el peso ∝ área de cada trozo."""
    zip_ruta = os.path.join(directorio, 'wc2.1_10m_prec.zip')
    if not os.path.exists(zip_ruta):
        log('Descarga de WorldClim 2.1 (10\', ~7 MB)…')
        os.makedirs(directorio, exist_ok=True)
        rt.bajar(WORLDCLIM, zip_ruta)
    anual, ref = None, None
    with zipfile.ZipFile(zip_ruta) as z:
        for mes in range(1, 13):
            nombre = f'wc2.1_10m_prec_{mes:02d}.tif'
            if z.getinfo(nombre).file_size > 50_000_000:
                raise ValueError(f'{nombre}: tamaño inesperado')
            t = tiffmini.GeoTiff(tiffmini.leer_memoria(z.read(nombre)))
            geo = (t.ancho, t.alto, t.lon0, t.lat0, t.paso_lon, t.paso_lat, t.escala, t.desplazamiento)
            if ref is None:
                ref = t
                if geo[:2] != (2160, 1080) or t.nodata is None:
                    raise ValueError(f'WorldClim con otra rejilla: {geo}')
                anual = [[0.0] * t.ancho for _ in range(t.alto)]
            elif geo != (ref.ancho, ref.alto, ref.lon0, ref.lat0, ref.paso_lon, ref.paso_lat, ref.escala,
                         ref.desplazamiento):
                raise ValueError(f'{nombre}: rejilla distinta de la de enero')
            nd, esc, des = t.nodata, t.escala, t.desplazamiento
            for r in range(t.alto):
                fila = t.fila(r)
                anual[r] = [a + (v * esc + des) if (a is not None and v != nd) else None
                            for a, v in zip(anual[r], fila)]
    t = ref
    m = 6 * N3 * N3
    w = array.array('d', bytes(8 * m))
    pw = array.array('d', bytes(8 * m))
    cl, sl = [], []
    for c in range(t.ancho):
        for s in range(SUB_WC):
            lo = math.radians(t.lon0 + (c + (s + 0.5) / SUB_WC) * t.paso_lon)
            cl.append(math.cos(lo))
            sl.append(math.sin(lo))
    loc = rt.localizar
    for r, fila in enumerate(anual):
        cols = [(c, v) for c, v in enumerate(fila) if v is not None]
        for s in range(SUB_WC):
            lat = math.radians(t.lat0 - (r + (s + 0.5) / SUB_WC) * t.paso_lat)
            cp, sp = math.cos(lat), math.sin(lat)
            for c, v in cols:
                for q in range(c * SUB_WC, (c + 1) * SUB_WC):
                    k = loc(cp * cl[q], cp * sl[q], sp, N3)
                    w[k] += cp
                    pw[k] += v * cp
    return w, pw


# --- CHELSA 2.1 ------------------------------------------------------------------------------------

def chelsa_muestras(directorio):
    """Filas muestreadas de CHELSA bio12: media de cada bloque de COLS_CH columnas (en décimas de mm) y
    nº de píxeles saturados del bloque. Se bajan una vez por peticiones parciales y se guardan."""
    ruta = os.path.join(directorio, f'CHELSA_bio12_1981-2010_V.2.1_filas{FILAS_CH}_cols{COLS_CH}.bin')
    if os.path.exists(ruta):
        with open(ruta, 'rb') as f:
            if f.read(8) != b'ABMCHL01':
                raise ValueError(f'muestras de CHELSA con otro formato: {ruta}')
            nf, nb, alto, ancho = struct.unpack('<IIII', f.read(16))
            lon0, lat0, plon, plat, esc = struct.unpack('<5d', f.read(40))
            filas = array.array('I')
            filas.fromfile(f, nf)
            medias = array.array('H')
            medias.fromfile(f, nf * nb)
            sat = array.array('B')
            sat.fromfile(f, nf * nb)
        return {'filas': filas, 'nb': nb, 'medias': medias, 'sat': sat, 'lon0': lon0, 'lat0': lat0,
                'paso_lon': plon, 'paso_lat': plat, 'escala': esc}
    log('Lectura de CHELSA 2.1 bio12 por peticiones parciales (1 de cada 8 filas, ~20 MB)…')
    t = tiffmini.GeoTiff(tiffmini.leer_remoto(CHELSA))
    if (t.ancho, t.alto, t.letra) != (43200, 20880, 'H') or t.ancho % COLS_CH:
        raise ValueError(f'CHELSA con otra rejilla: {t.ancho} × {t.alto}, {t.letra}')
    filas = array.array('I', range(FILAS_CH // 2, t.alto, FILAS_CH))
    nb = t.ancho // COLS_CH
    medias = array.array('H', bytes(2 * nb * len(filas)))
    sat = array.array('B', bytes(nb * len(filas)))

    def tira(r):
        return t.leer(*t.rango_tira(r))
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as ex:
        for i, b in enumerate(ex.map(tira, filas)):
            f = t.fila(filas[i], b)
            base = i * nb
            sumas = [sum(f[j:j + COLS_CH]) for j in range(0, t.ancho, COLS_CH)]
            medias[base:base + nb] = array.array('H', ((s + COLS_CH // 2) // COLS_CH for s in sumas))
            if f.count(SATURADO_CH):
                for j in range(nb):
                    sat[base + j] = f[j * COLS_CH:(j + 1) * COLS_CH].count(SATURADO_CH)
            if i % 300 == 0:
                log(f'  CHELSA: fila {i}/{len(filas)}')
    os.makedirs(directorio, exist_ok=True)
    with open(ruta + '.tmp', 'wb') as f:
        f.write(b'ABMCHL01' + struct.pack('<IIII', len(filas), nb, t.alto, t.ancho))
        f.write(struct.pack('<5d', t.lon0, t.lat0, t.paso_lon, t.paso_lat, t.escala))
        filas.tofile(f)
        medias.tofile(f)
        sat.tofile(f)
    os.replace(ruta + '.tmp', ruta)
    return chelsa_muestras(directorio)


def chelsa_celdas(directorio, datos):
    """CHELSA en las celdas de nivel 3, solo con los bloques que ETOPO (superficie, cada 2') da por
    tierra: (pesos, lluvia × peso, píxeles saturados × peso, píxeles × peso) por celda."""
    mu = chelsa_muestras(directorio)
    nb = mu['nb']
    nc = rt.COLS // rt.PASO_ETOPO
    if nb != nc:
        raise ValueError('los bloques de CHELSA no coinciden con las columnas de ETOPO')
    m = 6 * N3 * N3
    w, pw, sw, nw = (array.array('d', bytes(8 * m)) for _ in range(4))
    cl, sl = [], []
    for j in range(nb):
        lo = math.radians(mu['lon0'] + (j + 0.5) * COLS_CH * mu['paso_lon'])
        cl.append(math.cos(lo))
        sl.append(math.sin(lo))
    loc = rt.localizar
    esc = mu['escala']
    with open(os.path.join(datos, 'etopo2022_superficie_2m.f32be'), 'rb') as fe:
        filas_etopo = os.path.getsize(fe.name) // (4 * nc)
        for i, r in enumerate(mu['filas']):
            lat = mu['lat0'] - (r + 0.5) * mu['paso_lat']
            # Fila de ETOPO cada 2' más cercana (fila e: latitud −90 + (2e + 0,5)/60).
            e = min(max(int(round(((lat + 90.0) * 60.0 - 0.5) / rt.PASO_ETOPO)), 0), filas_etopo - 1)
            fe.seek(4 * nc * e)
            sup = array.array('f')
            sup.frombytes(fe.read(4 * nc))
            if sys.byteorder == 'little':
                sup.byteswap()
            cp, sp = math.cos(math.radians(lat)), math.sin(math.radians(lat))
            base = i * nb
            for j in range(nb):
                if not sup[j] >= 0.0:
                    continue
                k = loc(cp * cl[j], cp * sl[j], sp, N3)
                w[k] += cp
                pw[k] += mu['medias'][base + j] * esc * cp
                sw[k] += mu['sat'][base + j] * cp
                nw[k] += COLS_CH * cp
    return w, pw, sw, nw


# --- Los dos mundos --------------------------------------------------------------------------------

class Mundo:
    """Las celdas de tierra de un mundo con lo que piden las medidas: `tierra` (dicts con lat_deg,
    lon_deg, h, area_km2, m_oro y una clave por conjunto de lluvia, None sin dato), `dueno` (para cada
    celda de nivel 3, la celda de tierra que la contiene, o None) y `barlovento`."""

    def __init__(self, nombre, tierra, dueno, barlovento):
        self.nombre, self.tierra, self.dueno, self.barlovento = nombre, tierra, dueno, barlovento
        self.rel = terreno.relieve_por_casillas(tierra)


def mundo_tierra(cel, lluvias):
    """La Tierra: celdas de nivel 3 con h ≥ 0 en la roca rebotada, y las laderas de barlovento de
    `referencia_tierra.laderas_barlovento` (la selección de `pendientes`)."""
    filas = rt.factor_orografico(cel)
    km2 = AREA_TIERRA_KM2 / sum(cel.w)
    tierra, dueno = [], [None] * len(cel.w)
    for k, lat, lon, h, m_oro, a, _ in filas:
        c = {'k': k, 'lat_deg': lat, 'lon_deg': lon, 'h': h, 'm_oro': m_oro, 'area_km2': a * km2,
             'surface': 'tierra', 'hielo': antartida_o_groenlandia(lat, lon)}
        for clave, v in lluvias.items():
            c[clave] = v[k]
        tierra.append(c)
        dueno[k] = c
    lad = [dueno[f[0]] for f in rt.laderas_barlovento(filas)]
    m = Mundo('Tierra', tierra, dueno, lad)
    # Comprobación: la selección de terreno.py sobre las mismas celdas es la misma.
    otra = terreno.laderas_barlovento(tierra, m.rel)
    if [c['k'] for c in otra] != [c['k'] for c in lad]:
        raise RuntimeError('referencia_tierra.py y terreno.py no eligen las mismas laderas de barlovento')
    return m


def mundo_planeta(prefijo):
    """El planeta: las celdas de tierra de su volcado (todas las que no son océano, como terreno.py) y
    sus laderas de barlovento con `terreno.laderas_barlovento`. El volcado mezcla niveles de detalle
    (LOD): cada celda cubre 4^(3−nivel) celdas de nivel 3."""
    with open(prefijo + '_world.json') as f:
        celdas = json.load(f)
    dueno = [None] * (6 * N3 * N3)
    tierra = []
    for c in celdas:
        if c['surface'] == 'oceano':
            continue
        n = rt.N0 << c['level']
        la, lo = math.radians(c['lat_deg']), math.radians(c['lon_deg'])
        idx = rt.localizar(math.cos(la) * math.cos(lo), math.cos(la) * math.sin(lo), math.sin(la), n)
        cara, resto = divmod(idx, n * n)
        j, i = divmod(resto, n)
        f = 1 << (3 - c['level'])
        for dj in range(f):
            for di in range(f):
                dueno[(cara * N3 + j * f + dj) * N3 + i * f + di] = c
        c['modelo'] = c['lluvia_anual_mm']
        c['hielo'] = False
        tierra.append(c)
    m = Mundo('planeta', tierra, dueno, None)
    m.barlovento = terreno.laderas_barlovento(tierra, m.rel)
    with open(prefijo + '_terreno.json') as f:
        m.c_p = json.load(f)['run']['lluvia']['c_p']
    return m


# --- Medidas (las mismas en los dos mundos) --------------------------------------------------------

def media(cs, clave, campo=None):
    """Media ponderada por área de `campo` (por defecto, la lluvia `clave`) en las celdas con dato de
    lluvia; y el área con dato y el área total."""
    a_tot = sa = sv = 0.0
    for c in cs:
        a = c['area_km2']
        a_tot += a
        if c[clave] is None:
            continue
        sa += a
        sv += (c[clave] if campo is None else c[campo]) * a
    return (sv / sa if sa else float('nan')), sa, a_tot


def referencia_arriba(m, clave):
    """Referencia i: lluvia media de los puntos a 50–300 km contra el viento del modelo, por el mismo
    paralelo y cada 10 km, que caen en tierra baja con dato. None si no hay ninguno."""
    def ref(c):
        lat = c['lat_deg']
        coslat = max(math.cos(math.radians(lat)), 1e-3)
        hacia_el_este = 1.0 if 30.0 <= abs(lat) < 60.0 else -1.0
        cp, sp = math.cos(math.radians(lat)), math.sin(math.radians(lat))
        vals = []
        d = D_MIN
        while d <= D_MAX:
            lo = math.radians(c['lon_deg'] - hacia_el_este * math.degrees(d / (R_KM * coslat)))
            v = m.dueno[rt.localizar(cp * math.cos(lo), cp * math.sin(lo), sp, N3)]
            if v is not None and v['h'] < H_BAJA and v[clave] is not None:
                vals.append(v[clave])
            d += D_PASO
        return sum(vals) / len(vals) if vals else None
    return ref


def referencia_franja(m, clave):
    """Referencia ii: lluvia media (por área) de la tierra baja con dato de la franja de 1° de latitud."""
    acc = {}
    for c in m.tierra:
        if c['h'] < H_BAJA and c[clave] is not None:
            b = math.floor(c['lat_deg'] / FRANJA_REF)
            s = acc.setdefault(b, [0.0, 0.0])
            s[0] += c[clave] * c['area_km2']
            s[1] += c['area_km2']
    medias = {b: s[0] / s[1] for b, s in acc.items()}
    return lambda c: medias.get(math.floor(c['lat_deg'] / FRANJA_REF))


def factor(lad, clave, ref):
    """Factor de barlovento observado: lluvia de cada celda entre su referencia sin montaña `ref(c)`.
    Se omiten las celdas sin referencia; `m_oro` es la media de la regla del modelo en las mismas."""
    a_dato = sa = sp = sr = sm = 0.0
    pares = []
    for c in lad:
        p = c[clave]
        if p is None:
            continue
        a = c['area_km2']
        a_dato += a
        r = ref(c)
        if r is None or r <= 0.0:
            continue
        pares.append((p / r, a))
        sa += a
        sp += p * a
        sr += r * a
        sm += c['m_oro'] * a
    if not sa:
        nan = float('nan')
        return {'cociente_de_medias': nan, 'mediana': nan, 'media': nan, 'lluvia_mm': nan,
                'referencia_mm': nan, 'm_oro': nan, 'omitida_frac': 1.0}
    return {'cociente_de_medias': sp / sr, 'mediana': rt.mediana_ponderada(pares),
            'media': sum(f * a for f, a in pares) / sa, 'lluvia_mm': sp / sa, 'referencia_mm': sr / sa,
            'm_oro': sm / sa, 'omitida_frac': 1.0 - sa / a_dato}


LIM_ALTURA = [300.0, 1000.0, 2000.0, 3000.0]     # franjas de altura del barlovento (m; la última sin techo)


def barlovento(m, lad, clave):
    """Lluvia y factor observado de las laderas de barlovento `lad`, también por franjas de altura."""
    p, a_dato, a_tot = media(lad, clave)
    ref_f = referencia_franja(m, clave)
    out = {'area_km2': a_tot, 'con_dato_frac': a_dato / a_tot if a_tot else float('nan'), 'lluvia_mm': p,
           'm_oro': media(lad, clave, 'm_oro')[0],
           'factor_arriba': factor(lad, clave, referencia_arriba(m, clave)),
           'factor_franja': factor(lad, clave, ref_f)}
    franjas = [[c for c in lad if lo <= c['h'] < hi] for lo, hi in zip(LIM_ALTURA, LIM_ALTURA[1:] + [math.inf])]
    out['por_altura'] = {'area_frac': [sum(c['area_km2'] for c in cs) / a_tot for cs in franjas],
                         'lluvia_mm': [media(cs, clave)[0] for cs in franjas],
                         'factor_franja': [factor(cs, clave, ref_f)['cociente_de_medias'] for cs in franjas]}
    if clave == 'chelsa':
        sat = sum(c['chelsa_sat'] * c['area_km2'] for c in lad if c['chelsa'] is not None)
        out['saturada_frac'] = sat / a_dato if a_dato else float('nan')
    return out


def sombra(m, clave):
    """Sotavento de las cordilleras grandes en latitudes medias y resto, con terreno.py."""
    dentro, fuera = terreno.sotavento_y_resto(m.tierra, m.rel)
    ps, a_s, _ = media(dentro, clave)
    pr, a_r, _ = media(fuera, clave)
    return {'sotavento_lluvia_mm': ps, 'sotavento_area_km2': a_s, 'resto_lluvia_mm': pr, 'resto_area_km2': a_r,
            'sotavento_entre_resto': ps / pr}


def tierra_media(m, clave):
    """Lluvia media de la tierra, con y sin la Antártida y Groenlandia, y por franjas de 10°."""
    p, a_dato, a_tot = media(m.tierra, clave)
    p_sin, _, _ = media([c for c in m.tierra if not c['hielo']], clave)
    franjas = {b: [] for b in FRANJAS}
    for c in m.tierra:
        franjas[min(max(int(math.floor(c['lat_deg'] / 10.0)) * 10, -90), 80)].append(c)
    return {'lluvia_mm': p, 'lluvia_sin_antartida_ni_groenlandia_mm': p_sin, 'con_dato_frac': a_dato / a_tot,
            'franjas_area_frac': [sum(c['area_km2'] for c in franjas[b]) / a_tot for b in FRANJAS],
            'franjas_lluvia_mm': [media(franjas[b], clave)[0] for b in FRANJAS],
            'franjas_lluvia_sin_antartida_ni_groenlandia_mm':
                [media([c for c in franjas[b] if not c['hielo']], clave)[0] for b in FRANJAS]}


def medir(m, clave):
    r = {'tierra': tierra_media(m, clave), 'barlovento': barlovento(m, m.barlovento, clave),
         'sombra': sombra(m, clave)}
    if any(c['hielo'] for c in m.barlovento):
        r['barlovento_sin_antartida_ni_groenlandia'] = barlovento(m, [c for c in m.barlovento if not c['hielo']], clave)
    return r


# --- Salida ----------------------------------------------------------------------------------------

def mostrar(res, plan):
    """Tabla de la Tierra (cada conjunto) frente al planeta (lluvia del modelo)."""
    cols = [(f'Tierra {k}', v) for k, v in res.items()] + ([('planeta', plan)] if plan else [])

    def fila(nombre, f=None, fmt='{:.0f}'):
        vals = []
        for _, r in cols if f else []:
            try:
                x = f(r)
                vals.append('—' if x is None or (isinstance(x, float) and math.isnan(x)) else fmt.format(x))
            except KeyError:
                vals.append('—')
        print(f'{nombre:<42}' + ''.join(f'{v:>19}' for v in vals))
    print(f'{"":<42}' + ''.join(f'{n:>19}' for n, _ in cols))
    fila('tierra: lluvia media (mm)', lambda r: r['tierra']['lluvia_mm'])
    fila('  sin la Antártida ni Groenlandia', lambda r: r['tierra']['lluvia_sin_antartida_ni_groenlandia_mm'])
    fila('barlovento: área (millones de km²)', lambda r: r['barlovento']['area_km2'] / 1e6, '{:.2f}')
    fila('  lluvia (mm)', lambda r: r['barlovento']['lluvia_mm'])
    fila('  m_oro de la regla del modelo', lambda r: r['barlovento']['m_oro'], '{:.3f}')
    fila('  en píxeles saturados de CHELSA', lambda r: r['barlovento']['saturada_frac'], '{:.3f}')
    for ref, nombre in (('factor_arriba', 'factor i: tierra baja a barlovento'),
                        ('factor_franja', 'factor ii: tierra baja de la franja de 1°')):
        fila(nombre)
        fila('  cociente de medias', lambda r: r['barlovento'][ref]['cociente_de_medias'], '{:.3f}')
        fila('  mediana de los cocientes', lambda r: r['barlovento'][ref]['mediana'], '{:.3f}')
        fila('  media de los cocientes', lambda r: r['barlovento'][ref]['media'], '{:.3f}')
        fila('  barlovento / referencia (mm)', lambda r: f"{r['barlovento'][ref]['lluvia_mm']:.0f} / "
             f"{r['barlovento'][ref]['referencia_mm']:.0f}", '{}')
        fila('  m_oro de la regla en esas celdas', lambda r: r['barlovento'][ref]['m_oro'], '{:.3f}')
        fila('  área omitida (sin referencia)', lambda r: r['barlovento'][ref]['omitida_frac'], '{:.3f}')
    fila('barlovento por altura: área / mm / factor ii')
    lims = LIM_ALTURA + [None]
    for i in range(len(LIM_ALTURA)):
        tramo = f'{lims[i]:.0f}–{lims[i + 1]:.0f} m' if lims[i + 1] else f'≥ {lims[i]:.0f} m'
        fila(f'  {tramo}', lambda r: f"{r['barlovento']['por_altura']['area_frac'][i]:.2f} / "
             f"{r['barlovento']['por_altura']['lluvia_mm'][i]:.0f} / "
             f"{r['barlovento']['por_altura']['factor_franja'][i]:.2f}", '{}')
    fila('barlovento sin Antártida ni Groenlandia')
    fila('  lluvia (mm)', lambda r: r['barlovento_sin_antartida_ni_groenlandia']['lluvia_mm'])
    fila('  factor i / ii (cociente de medias)',
         lambda r: f"{r['barlovento_sin_antartida_ni_groenlandia']['factor_arriba']['cociente_de_medias']:.3f} / "
         f"{r['barlovento_sin_antartida_ni_groenlandia']['factor_franja']['cociente_de_medias']:.3f}", '{}')
    fila('sotavento de latitudes medias (mm)', lambda r: r['sombra']['sotavento_lluvia_mm'])
    fila('resto de las latitudes medias (mm)', lambda r: r['sombra']['resto_lluvia_mm'])
    fila('sotavento / resto', lambda r: r['sombra']['sotavento_entre_resto'], '{:.3f}')
    fila('tierra por franjas de 10°: fracción · mm')
    for i, b in enumerate(FRANJAS):
        fila(f'  {b:+d}° a {b + 10:+d}°', lambda r: f"{r['tierra']['franjas_area_frac'][i]:.3f} · "
             f"{r['tierra']['franjas_lluvia_mm'][i]:.0f}", '{}')
    if plan:
        # Indicativo: supone que la lluvia solo depende de la latitud (no de lo grande que es cada continente).
        fp = plan['tierra']['franjas_area_frac']
        for k, r in res.items():
            pt = r['tierra']['franjas_lluvia_mm']
            x = sum(f * p for f, p in zip(fp, pt) if f > 0 and not math.isnan(p)) / \
                sum(f for f, p in zip(fp, pt) if f > 0 and not math.isnan(p))
            print(f'Lluvia de la Tierra ({k}) franja a franja, con la tierra repartida por latitud como en el '
                  f'planeta: {x:.0f} mm (planeta: {plan["tierra"]["lluvia_mm"]:.0f} mm con c_P = {plan["c_p"]:.3f})')


def escribir(ruta, res, extra):
    """config/referencias/tierra_lluvia.toml: solo la Tierra, con una nota la primera vez que sale
    cada clave."""
    notas = {
        'lluvia_media_mm': 'Lluvia anual media de la tierra (celdas con h ≥ 0 en la roca rebotada), por área.',
        'lluvia_media_sin_antartida_ni_groenlandia_mm': 'Lo mismo sin la Antártida ni Groenlandia.',
        'con_dato_frac': 'Fracción del área con dato del conjunto.',
        'lluvia_mm': 'Lluvia anual media (mm) por área.',
        'lluvia_sin_antartida_ni_groenlandia_mm': 'Lo mismo sin la Antártida ni Groenlandia.',
        'area_km2': 'Área de las laderas de barlovento.',
        'm_oro_regla': 'Media de m_oro con la regla del modelo (k_oro = 40) en el relieve real: no es lluvia '
                       'observada.',
        'cociente_de_medias': 'Lluvia media del barlovento entre la media de su referencia (la cifra robusta).',
        'mediana': 'Mediana (por área) del cociente de cada celda.',
        'media': 'Media (por área) del cociente de cada celda: la homóloga de la media de m_oro, sensible a las '
                 'referencias muy secas.',
        'referencia_mm': 'Lluvia media de la referencia en esas celdas.',
        'omitida_frac': 'Fracción del barlovento con dato sin referencia (omitida).',
        'limites_m': 'Límite inferior de cada franja de altura (la última sin techo).',
        'area_frac': 'Fracción del área en cada franja.',
        'factor_franja': 'Factor ii (cociente de medias) en cada franja de altura.',
        'saturada_frac': 'Fracción del barlovento en píxeles de CHELSA saturados (≥ 6553,5 mm): la lluvia real '
                         'es mayor.',
        'sotavento_lluvia_mm': 'Tierra de 30–60° bajo 2000 m con cresta > 2000 m a 200–500 km a barlovento.',
        'sotavento_area_km2': 'Su área.',
        'resto_lluvia_mm': 'El resto de esa tierra de latitudes medias.',
        'resto_area_km2': 'Su área.',
        'sotavento_entre_resto': 'Cociente de las dos lluvias.',
    }
    vistas = set()

    def seccion(datos):
        """Filas (clave, valor[, nota]): la nota propia o la de `notas`, solo la primera vez."""
        out = []
        for d in datos:
            k, v = d[0], d[1]
            nota = d[2] if len(d) > 2 else notas.get(k)
            out.append((k, v, None if (k, nota) in vistas else nota))
            vistas.add((k, nota))
        return out
    secciones = [('franjas_10', seccion([
        ('lat_sur', FRANJAS, 'Límite sur de cada franja de 10° de latitud.'),
        ('area_frac', next(iter(res.values()))['tierra']['franjas_area_frac'],
         'Fracción de la tierra de la Tierra en cada franja.')]))]
    for nombre, r in res.items():
        t = r['tierra']
        secciones.append((f'{nombre}.tierra', seccion([
            ('lluvia_media_mm', t['lluvia_mm']),
            ('lluvia_media_sin_antartida_ni_groenlandia_mm', t['lluvia_sin_antartida_ni_groenlandia_mm']),
            ('con_dato_frac', t['con_dato_frac'])])))
        secciones.append((f'{nombre}.tierra.franjas_10', seccion([
            ('lluvia_mm', t['franjas_lluvia_mm'], 'Lluvia anual media (mm) de la tierra de cada franja de 10°.'),
            ('lluvia_sin_antartida_ni_groenlandia_mm', t['franjas_lluvia_sin_antartida_ni_groenlandia_mm'])])))
        for bl in ('barlovento', 'barlovento_sin_antartida_ni_groenlandia'):
            b = r[bl]
            datos = [('area_km2', b['area_km2']), ('lluvia_mm', b['lluvia_mm']), ('m_oro_regla', b['m_oro'])]
            if 'saturada_frac' in b:
                datos.append(('saturada_frac', b['saturada_frac']))
            secciones.append((f'{nombre}.{bl}', seccion(datos)))
            for ref in ('factor_arriba', 'factor_franja'):
                f = b[ref]
                secciones.append((f'{nombre}.{bl}.{ref}', seccion([
                    ('cociente_de_medias', f['cociente_de_medias']), ('mediana', f['mediana']), ('media', f['media']),
                    ('lluvia_mm', f['lluvia_mm'], 'Lluvia media del barlovento en las celdas con referencia.'),
                    ('referencia_mm', f['referencia_mm']),
                    ('m_oro_regla', f['m_oro'], 'Media de m_oro con la regla del modelo en esas celdas.'),
                    ('omitida_frac', f['omitida_frac'])])))
            if bl == 'barlovento':
                pa = b['por_altura']
                secciones.append((f'{nombre}.barlovento.por_altura', seccion([
                    ('limites_m', LIM_ALTURA),
                    ('area_frac', pa['area_frac'], 'Fracción del barlovento en cada franja de altura.'),
                    ('lluvia_mm', pa['lluvia_mm'], 'Lluvia anual media (mm) de cada franja de altura.'),
                    ('factor_franja', pa['factor_franja'])])))
        s = r['sombra']
        claves = ('sotavento_lluvia_mm', 'sotavento_area_km2', 'resto_lluvia_mm', 'resto_area_km2',
                  'sotavento_entre_resto')
        secciones.append((f'{nombre}.sombra', seccion([(k, s[k]) for k in claves])))
    hoy = datetime.date.today().isoformat()
    chelsa = 'chelsa' in res
    cab = f'''# Lluvia observada de la Tierra a la escala del planeta (~26 km): el objetivo de la lluvia de barlovento
# y la revisión de c_P (decisión del responsable, 2026-11-05).
# Generado por herramientas/analisis/lluvia_observada.py el {hoy}. No editar a mano: se regenera.
#
# Conjuntos de datos:
# - WorldClim 2.1 (Fick y Hijmans, 2017, Int. J. Climatol. 37, 4302–4315, doi:10.1002/joc.5086), versión
#   de enero de 2020: lluvia mensual media de 1970–2000 a 10' (wc2.1_10m_prec.zip), sumada en el año.
#   Interpola pluviómetros, que escasean en las montañas: ahí se queda corto.
# - CHELSA 2.1 (Karger et al., 2017, Sci. Data 4, 170122, doi:10.1038/sdata.2017.122; datos
#   doi:10.16904/envidat.228): bio12, lluvia anual de 1981–2010 a 30''. Modela el viento y el relieve.
#   Leída 1 de cada {FILAS_CH} filas (cada 4') en bloques de {COLS_CH} columnas (2'), solo sobre la tierra de
#   ETOPO (superficie). Satura en 6553,5 mm.{"" if chelsa else " (No incluida en esta corrida.)"}
# - El relieve de config/referencias/tierra_relieve.toml: roca rebotada sin hielo (ETOPO 2022 y Paxman
#   et al., 2022).
#
# Celdas: las de nivel 3 del cubo esférico del planeta (n0 = {rt.N0}, ~26 km), con la media por área de los
#   píxeles de tierra con dato de cada una (WorldClim: cada píxel partido en {SUB_WC}×{SUB_WC} trozos).
#   Tierra = celdas con h ≥ 0 en la roca rebotada ({extra["tierra_km2"] / 1e6:.1f} millones de km²).
# Barlovento: las laderas de barlovento de las cordilleras grandes de referencia_tierra.pendientes y
#   terreno.py: m_oro > 1 (k_oro = 40, viento del oeste en 30–60° de latitud y del este en el resto),
#   h ≥ 300 m y cresta > 2000 m a 0–150 km a favor del viento. En las mesetas altas (Tíbet, Altiplano,
#   Irán) la cresta siempre está cerca y entran casi enteras: el barlovento llueve menos que la tierra baja.
# Factor de barlovento observado (lo que en el modelo es m_oro): lluvia de cada celda de barlovento entre
#   una lluvia de referencia sin montaña:
#   - factor_arriba: media de los puntos de tierra baja (h < {H_BAJA:.0f} m) a {D_MIN:.0f}–{D_MAX:.0f} km contra el
#     viento del modelo, por el mismo paralelo y cada {D_PASO:.0f} km; sin tierra baja delante (p. ej., el mar),
#     se omite;
#   - factor_franja: media de la tierra baja (h < {H_BAJA:.0f} m) de su franja de {FRANJA_REF:.0f}° de latitud.
# Sombra: tierra de 30–60° bajo 2000 m con cresta > 2000 m a 200–500 km a barlovento (sotavento) o sin ella
#   (resto), como terreno.py.
# Antártida = tierra al sur de 60° S; Groenlandia = tierra dentro de un polígono por los estrechos que la
#   separan de Baffin, Ellesmere e Islandia. Su barlovento se elige por la roca bajo el hielo, que no es
#   el relieve que ve la lluvia: por eso se da también sin ellas.
'''
    os.makedirs(os.path.dirname(ruta), exist_ok=True)
    with open(ruta, 'w') as f:
        f.write(cab)
        for nombre, filas in secciones:
            f.write(f'\n[{nombre}]\n')
            for k, v, nota in filas:
                if nota:
                    f.write(f'# {nota}\n')
                f.write(f'{k} = {rt.toml_valor(v)}\n')


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--datos', default=os.path.join(RAIZ, 'output', 'referencias'))
    ap.add_argument('--cache', default=os.path.join(RAIZ, 'output', 'cache'))
    ap.add_argument('--planeta', default=os.path.join(RAIZ, 'output', 'escenarios', 'planeta'),
                    help='prefijo del volcado del planeta: <prefijo>_world.json y <prefijo>_terreno.json')
    ap.add_argument('--sin-chelsa', action='store_true', help='solo WorldClim')
    ap.add_argument('--salida', default=os.path.join(RAIZ, 'config', 'referencias', 'tierra_lluvia.toml'))
    args = ap.parse_args()
    t0 = time.time()
    cel = relieve_roca_rebote(args.datos, args.cache)
    log('WorldClim 2.1 a celdas de nivel 3…')
    w, pw = worldclim_celdas(os.path.join(args.datos, 'worldclim'))
    lluvias = {'worldclim': [pw[k] / w[k] if w[k] > 0 else None for k in range(len(w))]}
    if not args.sin_chelsa:
        log('CHELSA 2.1 a celdas de nivel 3…')
        w, pw, sw, nw = chelsa_celdas(os.path.join(args.datos, 'chelsa'), args.datos)
        lluvias['chelsa'] = [pw[k] / w[k] if w[k] > 0 else None for k in range(len(w))]
        lluvias['chelsa_sat'] = [sw[k] / nw[k] if nw[k] > 0 else 0.0 for k in range(len(w))]
    log('Medidas en la Tierra…')
    tierra = mundo_tierra(cel, lluvias)
    res = {k: medir(tierra, k) for k in ('worldclim', 'chelsa') if k in lluvias}
    plan = None
    if os.path.exists(args.planeta + '_world.json'):
        log(f'Medidas en el planeta ({os.path.relpath(args.planeta, RAIZ)}_world.json)…')
        planeta = mundo_planeta(args.planeta)
        plan = medir(planeta, 'modelo')
        plan['c_p'] = planeta.c_p
    else:
        log(f'No hay volcado del planeta en {args.planeta}_world.json: solo la Tierra '
            '(herramientas/escenarios.py --solo planeta lo genera).')
    mostrar(res, plan)
    escribir(args.salida, res, {'tierra_km2': sum(c['area_km2'] for c in tierra.tierra)})
    log(f'Escrito {os.path.relpath(args.salida, RAIZ)} ({time.time() - t0:.0f} s)')


if __name__ == '__main__':
    main()
