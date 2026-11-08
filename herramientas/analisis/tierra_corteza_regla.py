"""La Tierra real con la regla del modelo, ahora para los continentes: relieve y grosor de la corteza.

Pregunta del responsable (2026-11-08): si se mete el grosor real de la corteza (CRUST1.0) y su
sedimento en la isostasia del modelo, con su nivel del mar, ¿sale el relieve real (ETOPO sin hielo)?
Se compara lo mismo que en la tabla de cierre: la altura media de la tierra, la tierra por debajo de
200 m y por encima de 2 000 m, la pendiente a ±25 km y la anchura de la plataforma. Se mide también
el grosor de la corteza continental del planeta frente al de CRUST1.0.

La regla del modelo (`abm_terreno::tectonica::isostasia::altura`):
- corteza continental: flotación `B = (ρ_m − ρ_c)·(h − h_nivel) + (ρ_m − ρ_s)·s`, con `h` el grosor de
  la corteza cristalina y `s` el del sedimento; al aire sube `B/ρ_m` y bajo el agua `B/(ρ_m − ρ_w)`;
- fondo oceánico: la curva de profundidad por edad más la flotación de su sedimento (como en
  `tierra_regla_modelo.py`, con las edades de Seton et al. 2020 y el sedimento de GlobSed);
- nivel del mar: el que llena las cuencas con el volumen de agua del modelo (`gel_m`).

Cómo:
- Celdas de nivel 3 (~26 km), las de la referencia del relieve (`referencia_tierra.py`).
- Fondo oceánico: las celdas con edad de Seton en al menos la mitad de su área (como
  `tierra_regla_modelo.py`). El resto, corteza continental, con la regla continental.
- CRUST1.0 (Laske et al. 2013) es una malla de 1°: en cada celda de nivel 3 se interpola
  bilinealmente entre los centros de las cuatro celdas de 1° que la rodean, usando solo las de
  corteza continental si la celda es continental (para no mezclar el fondo oceánico en la costa).
  `h` es la corteza cristalina (las tres capas) y `s` los tres sedimentos; el agua y el hielo no
  cuentan (el modelo no tiene casquetes: es la roca sin hielo con su rebote).
- Variantes:
  - `tierra`: ETOPO 2022 con la roca rebotada sin hielo (la referencia de la tabla de cierre);
  - `tierra_1grado`: lo mismo, promediado en las celdas de 1° de CRUST1.0 e interpolado igual que
    CRUST1.0, para comparar la pendiente y la plataforma a la misma resolución;
  - `regla_continentes`: los continentes con la regla del modelo y el océano real;
  - `regla`: toda la Tierra con la regla del modelo (continentes con CRUST1.0, océano con Seton y
    GlobSed);
  - `regla_densidades_crust1`: como `regla`, con la densidad de cada capa de CRUST1.0 en lugar de
    ρ_c y ρ_s (la referencia es la del modelo: `(ρ_m − ρ_c)·h_nivel`);
  - `regla_mesetas`: como `regla`, con la flotación de la corteza oceánica de más de `h_oc` (mesetas).
- Diagnósticos: la altura real y la de la regla por clases de grosor de la corteza, el residuo por tipo
  de corteza de CRUST1.0 (escudos, plataformas, fanerozoico, orógenos, zonas estiradas, márgenes) y el
  grosor de la corteza continental de CRUST1.0 frente al del planeta (volcados `*_world.json`).

Datos (en `--datos`, por defecto `output/referencias`): los de `referencia_tierra.py` y
`tierra_regla_modelo.py`, y `crust1/` (CRUST1.0 y su archivo de tipos de corteza, que se descargan de
igppweb.ucsd.edu la primera vez). Las celdas de nivel 3 de ETOPO, de Seton y de GlobSed se guardan en
caché en `output/cache/`.

Uso: python3 -I herramientas/analisis/tierra_corteza_regla.py [--planeta output/semillas/semilla_42_world.json ...]
Escribe `config/referencias/tierra_corteza.toml` (solo la Tierra) y muestra la tabla Tierra–regla–planeta.
"""
import argparse
import array
import glob
import hashlib
import inspect
import io
import json
import math
import os
import sys
import tarfile
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import referencia_tierra as rt  # noqa: E402
import tierra_regla_modelo as trm  # noqa: E402
import lluvia_observada as lo  # noqa: E402

N = rt.N3
RAIZ = rt.RAIZ
A_TIERRA_KM2 = 4.0 * math.pi * rt.R_KM ** 2

CRUST1 = 'http://igppweb.ucsd.edu/~gabi/crust1/crust1.0.tar.gz'
CRUST1_TIPOS = 'http://igppweb.ucsd.edu/~gabi/crust1/crust1.0-addon.tar.gz'
LITHO1 = 'https://igppweb.ucsd.edu/~gabi/litho1/litho1.0.tar.gz'
FICHEROS_CRUST1 = {CRUST1: ('crust1.bnds', 'crust1.rho'), CRUST1_TIPOS: ('CNtype1-1.txt', 'CNtype1_key.txt')}

# Tipos de corteza de CRUST1.0 (CNtype1_key.txt) que son fondo oceánico; el resto es continental.
TIPOS_OCEANICOS = {'A0', 'A1', 'B-', 'V1', 'Y3'}
# Grupos para el residuo por tipo (códigos de CNtype1_key.txt).
GRUPOS = [
    ('escudos y plataformas (Arcaico y Proterozoico)', {'D-', 'E-', 'F-', 'G1', 'G2', 'H1', 'H2', 'I1', 'I2'}),
    ('Fanerozoico', {'Z1', 'Z2'}),
    ('orógenos', {'O-', 'P-', 'Q-', 'R1', 'R2'}),
    ('arcos y antearcos', {'J-', 'K-', 'L1', 'L2'}),
    ('corteza estirada y rifts', {'M-', 'N-', 'X-', 'V2'}),
    ('plataformas, taludes y márgenes', {'C-', 'S-', 'T-', 'U-'}),
    ('mesetas y cuencas intermedias', {'W-', 'Y1', 'Y2'}),
    ('tipo oceánico sin edad de Seton', TIPOS_OCEANICOS),
]
LIM_H = [0.0, 10.0, 20.0, 25.0, 30.0, 32.5, 35.0, 37.5, 40.0, 42.5, 45.0, 47.5, 50.0, 55.0, 60.0, 65.0, 70.0]


def log(*a):
    print(*a, file=sys.stderr, flush=True)


# --- Datos ----------------------------------------------------------------------------------------

def descargar_crust1(datos):
    """Descarga CRUST1.0 y su archivo de tipos (solo la primera vez) y saca de los tar solo los
    ficheros que se usan, por su nombre, a `datos/crust1/`. No se ejecuta nada de lo descargado."""
    dest = os.path.join(datos, 'crust1')
    os.makedirs(dest, exist_ok=True)
    for url, nombres in FICHEROS_CRUST1.items():
        if all(os.path.exists(os.path.join(dest, n)) for n in nombres):
            continue
        log(f'Descargando {url}…')
        with urllib.request.urlopen(url, timeout=120) as r:
            b = r.read()
        with tarfile.open(fileobj=io.BytesIO(b), mode='r:gz') as tf:
            por_nombre = {os.path.basename(m.name): m for m in tf.getmembers() if m.isfile()}
            for n in nombres:
                m = por_nombre.get(n)
                if m is None:
                    raise FileNotFoundError(f'{n} no está en {url}')
                with open(os.path.join(dest, n), 'wb') as f:
                    f.write(tf.extractfile(m).read())
    return dest


def leer_crust1(dest):
    """Las 64 800 celdas de 1° de CRUST1.0 (de 89,5° N a 89,5° S y de 179,5° W a 179,5° E, la
    longitud por dentro): corteza cristalina `h` y sedimento `s` (km), flotación de las capas con sus
    densidades `b_capas` (Σ (ρ_m − ρ_i)·t_i con ρ_m del modelo, en km·kg/m³, sin la referencia),
    densidad media de la corteza y del sedimento, agua, hielo y tipo de corteza."""
    def filas(n):
        with open(os.path.join(dest, n)) as f:
            return [[float(x) for x in ln.split()] for ln in f if ln.strip()]
    bnds, rho = filas('crust1.bnds'), filas('crust1.rho')
    with open(os.path.join(dest, 'CNtype1-1.txt')) as f:
        tipos = f.read().split()
    assert len(bnds) == len(rho) == len(tipos) == 64800, (len(bnds), len(rho), len(tipos))
    return bnds, rho, tipos


def columnas_crust1(bnds, rho, rho_m):
    out = {'h': [], 's': [], 'b_capas': [], 'rho_corteza': [], 'rho_sed': [], 'agua': [], 'hielo': [],
           'moho': [], 'top': []}
    for b, r in zip(bnds, rho):
        t = [b[i] - b[i + 1] for i in range(8)]   # agua, hielo, sedimentos 1–3, corteza 1–3 (km)
        h = t[5] + t[6] + t[7]
        s = t[2] + t[3] + t[4]
        bc = sum((rho_m - 1000.0 * r[i]) * t[i] for i in range(2, 8))
        out['h'].append(h)
        out['s'].append(s)
        out['b_capas'].append(bc)
        out['rho_corteza'].append(sum(1000.0 * r[i] * t[i] for i in range(5, 8)) / h if h > 0 else float('nan'))
        out['rho_sed'].append(sum(1000.0 * r[i] * t[i] for i in range(2, 5)) / s if s > 0 else float('nan'))
        out['agua'].append(t[0])
        out['hielo'].append(t[1])
        out['moho'].append(-b[8])
        out['top'].append(b[0])
    return out


def litosfera_litho1(datos, cache):
    """Base de la litosfera (LAB) y Moho de LITHO1.0 (Pasyanos et al. 2014) en los centros de las celdas
    de 1° de CRUST1.0 (el nodo más cercano de la malla icosaédrica de 40 962 nodos), en km. Se descarga
    la primera vez a `datos/litho1/` (12 MB) y se lee del tar sin extraerlo; se guarda en caché."""
    ruta_tar = os.path.join(datos, 'litho1', 'litho1.0.tar.gz')
    if not os.path.exists(ruta_tar):
        os.makedirs(os.path.dirname(ruta_tar), exist_ok=True)
        log(f'Descargando {LITHO1}…')
        with urllib.request.urlopen(LITHO1, timeout=300) as r, open(ruta_tar + '.tmp', 'wb') as f:
            f.write(r.read())
        os.replace(ruta_tar + '.tmp', ruta_tar)

    def calcular():
        log('LITHO1.0: base de la litosfera de cada nodo…')
        nodos = {}
        coords = None
        with tarfile.open(ruta_tar, 'r:gz') as tf:
            for mem in tf:
                if not mem.isfile():
                    continue
                nombre = os.path.basename(mem.name)
                if nombre == 'Icosahedron_Level7_LatLon_mod.txt':
                    coords = [[float(x) for x in ln.split()] for ln in tf.extractfile(mem).read().decode().splitlines() if ln.strip()]
                elif nombre.startswith('node') and nombre.endswith('.model'):
                    lab = moho = float('nan')
                    for ln in tf.extractfile(mem).read().decode().splitlines():
                        p = ln.split()
                        if len(p) >= 10 and p[-1] == 'LID-BOTTOM':
                            lab = float(p[0]) / 1000.0
                        elif len(p) >= 10 and p[-1] == 'LID-TOP':
                            moho = float(p[0]) / 1000.0
                    nodos[int(nombre[4:-6])] = (lab, moho)
        # Columnas: latitud geocéntrica, latitud geográfica y longitud; nodos numerados desde 1.
        xyz = []
        for i, (_, la, lo_) in enumerate(coords, 1):
            cl = math.cos(math.radians(la))
            xyz.append((cl * math.cos(math.radians(lo_)), cl * math.sin(math.radians(lo_)), math.sin(math.radians(la)), i))
        lado = 0.04
        cubos = {}
        for x, y, z, i in xyz:
            cubos.setdefault((math.floor(x / lado), math.floor(y / lado), math.floor(z / lado)), []).append((x, y, z, i))
        lab1, moho1 = [], []
        for c1 in range(64800):
            la = 89.5 - c1 // 360
            lo_ = -179.5 + c1 % 360
            cl = math.cos(math.radians(la))
            p = (cl * math.cos(math.radians(lo_)), cl * math.sin(math.radians(lo_)), math.sin(math.radians(la)))
            k0 = tuple(math.floor(v / lado) for v in p)
            mejor, dmin = None, 9.0
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    for dz in (-1, 0, 1):
                        for x, y, z, i in cubos.get((k0[0] + dx, k0[1] + dy, k0[2] + dz), ()):
                            d2 = (x - p[0]) ** 2 + (y - p[1]) ** 2 + (z - p[2]) ** 2
                            if d2 < dmin:
                                mejor, dmin = i, d2
            lab1.append(nodos[mejor][0])
            moho1.append(nodos[mejor][1])
        return [lab1, moho1]
    clave = huella(litosfera_litho1, extra=f'{os.path.getsize(ruta_tar)}')
    return en_cache(cache, 'litho1_1grado', clave, calcular, 2, 64800)


def regresion(filas, pesos):
    """Mínimos cuadrados ponderados: `filas` = [(y, x1, …, xn)]; devuelve [b0, b1, …, bn] y r²."""
    n = len(filas[0])
    ata = [[0.0] * n for _ in range(n)]
    aty = [0.0] * n
    for f, w in zip(filas, pesos):
        x = (1.0,) + tuple(f[1:])
        for i in range(n):
            aty[i] += w * x[i] * f[0]
            for j in range(n):
                ata[i][j] += w * x[i] * x[j]
    # Eliminación de Gauss con pivote parcial.
    a = [ata[i] + [aty[i]] for i in range(n)]
    for col in range(n):
        piv = max(range(col, n), key=lambda r: abs(a[r][col]))
        a[col], a[piv] = a[piv], a[col]
        for r in range(n):
            if r != col:
                f = a[r][col] / a[col][col]
                for cc in range(col, n + 1):
                    a[r][cc] -= f * a[col][cc]
    b = [a[i][n] / a[i][i] for i in range(n)]
    sw = sum(pesos)
    my = sum(f[0] * w for f, w in zip(filas, pesos)) / sw
    ss_tot = sum((f[0] - my) ** 2 * w for f, w in zip(filas, pesos))
    ss_res = sum((f[0] - (b[0] + sum(bi * xi for bi, xi in zip(b[1:], f[1:])))) ** 2 * w for f, w in zip(filas, pesos))
    return b, 1.0 - ss_res / ss_tot


def huella(*objs, extra=''):
    h = hashlib.sha256()
    for o in objs:
        h.update(inspect.getsource(o).encode())
    h.update(extra.encode())
    return h.hexdigest()[:16]


def en_cache(cache, nombre, clave, calcular, n_arrays, m):
    """`n_arrays` listas de `m` reales calculadas por `calcular()` (o leídas de la caché)."""
    ruta = os.path.join(cache, f'{nombre}_{clave}.bin')
    if os.path.exists(ruta):
        out = []
        with open(ruta, 'rb') as f:
            if f.read(8) != b'ABMCAP01':
                raise ValueError(f'caché con otro formato: {ruta}')
            for _ in range(n_arrays):
                a = array.array('d')
                a.fromfile(f, m)
                out.append(list(a))
        log(f'{nombre}: leído de la caché {os.path.relpath(ruta, RAIZ)}')
        return out
    out = calcular()
    os.makedirs(cache, exist_ok=True)
    with open(ruta + '.tmp', 'wb') as f:
        f.write(b'ABMCAP01')
        for v in out:
            array.array('d', v).tofile(f)
    os.replace(ruta + '.tmp', ruta)
    log(f'{nombre}: guardado en la caché {os.path.relpath(ruta, RAIZ)}')
    return out


# --- Geometría ------------------------------------------------------------------------------------

def lat_lon_centros(n):
    lat, lon = [], []
    for k in range(6 * n * n):
        d = rt.centro(k, n)
        lat.append(math.degrees(math.asin(max(-1.0, min(1.0, d[2])))))
        lon.append(math.degrees(math.atan2(d[1], d[0])))
    return lat, lon


def celda_1grado(lat, lon):
    i = min(max(int(math.floor(90.0 - lat)), 0), 179)
    j = int(math.floor(lon + 180.0)) % 360
    return i * 360 + j


def vecinas_bilineal(lat, lon):
    """Los cuatro centros de 1° que rodean el punto, con sus pesos bilineales."""
    fi = 89.5 - lat
    fj = lon + 179.5
    i0 = math.floor(fi)
    j0 = math.floor(fj)
    a, b = fi - i0, fj - j0
    out = []
    for di, dj, w in ((0, 0, (1 - a) * (1 - b)), (0, 1, (1 - a) * b), (1, 0, a * (1 - b)), (1, 1, a * b)):
        i = min(max(i0 + di, 0), 179)
        j = (j0 + dj) % 360
        out.append((i * 360 + j, w))
    return out


def interpolar(campos, tipos, lat, lon, es_cont):
    """Campos de CRUST1.0 interpolados en las celdas de nivel 3 (ver la cabecera)."""
    m = len(lat)
    out = {c: [0.0] * m for c in campos}
    tipo = [''] * m
    for k in range(m):
        vs = vecinas_bilineal(lat[k], lon[k])
        tipo[k] = tipos[celda_1grado(lat[k], lon[k])]
        if es_cont[k]:
            v2 = [(i, w) for i, w in vs if tipos[i] not in TIPOS_OCEANICOS]
            if sum(w for _, w in v2) > 1e-9:
                vs = v2
        for c, v in campos.items():
            sw = swv = 0.0
            for i, w in vs:
                x = v[i]
                if math.isfinite(x):
                    sw += w
                    swv += w * x
            out[c][k] = swv / sw if sw > 0 else float('nan')
    return out, tipo


def a_1grado_y_vuelta(z, w, lat, lon):
    """El relieve real promediado en las celdas de 1° de CRUST1.0 y de vuelta a nivel 3 con la misma
    interpolación bilineal (sin distinguir tipos): la Tierra a la resolución de CRUST1.0."""
    sw = [0.0] * 64800
    szw = [0.0] * 64800
    for k in range(len(z)):
        c = celda_1grado(lat[k], lon[k])
        sw[c] += w[k]
        szw[c] += z[k] * w[k]
    z1 = [szw[c] / sw[c] if sw[c] > 0 else float('nan') for c in range(64800)]
    out = []
    for k in range(len(z)):
        s = sv = 0.0
        for i, wi in vecinas_bilineal(lat[k], lon[k]):
            if math.isfinite(z1[i]):
                s += wi
                sv += wi * z1[i]
        out.append(sv / s if s > 0 else z[k])
    return out


# --- La regla del modelo --------------------------------------------------------------------------

def con_flotacion(c, z0, b):
    """`isostasia::con_flotacion` (m)."""
    agua, aire = c['rho_m'] - c['rho_w'], c['rho_m']
    if b >= 0.0:
        if z0 >= 0.0:
            return z0 + 1000.0 * b / aire
        z = z0 + 1000.0 * b / agua
        if z <= 0.0:
            return z
        return 1000.0 * (b - (-z0 / 1000.0) * agua) / aire
    if z0 <= 0.0:
        return z0 + 1000.0 * b / agua
    z = z0 + 1000.0 * b / aire
    if z >= 0.0:
        return z
    return 1000.0 * (b + (z0 / 1000.0) * aire) / agua


def b_continental(c, h, s):
    return (c['rho_m'] - c['rho_c']) * (h - c['h_nivel_km']) + (c['rho_m'] - c['rho_s']) * s


# --- Medidas --------------------------------------------------------------------------------------

def celdas_de(z, w, zeta):
    cel = rt.Celdas(N)
    cel.w = array.array('d', w)
    cel.zw = array.array('d', ((zk - zeta) * wk for zk, wk in zip(z, w)))
    cel.z2w = array.array('d', ((zk - zeta) ** 2 * wk for zk, wk in zip(z, w)))
    return cel


def medir(z, w, zeta):
    """Las filas de relieve de la tabla de cierre, con la definición de la referencia (nivel 3)."""
    cel = celdas_de(z, w, zeta)
    est = rt.estadisticas(cel)
    filas = rt.factor_orografico(cel)
    a = sum(f[5] for f in filas)
    pend = sum(f[6] * f[5] for f in filas) / a if a > 0 else float('nan')
    plat = rt.plataforma(cel)
    return {
        'nivel_mar_m': zeta,
        'tierra_frac': est['tierra_frac'],
        'altura_media_tierra_m': est['altura_media_tierra_m'],
        'altura_mediana_tierra_m': est['altura_mediana_tierra_m'],
        'tierra_bajo_200_frac': est['hipsometria_frac'][0],
        'tierra_sobre_2000_frac': sum(est['hipsometria_frac'][4:]),
        'hipsometria_frac': est['hipsometria_frac'],
        'pendiente_media_tierra': pend,
        'plataforma_km': plat['anchura_media_km'],
        'profundidad_media_oceano_m': est['profundidad_media_oceano_m'],
    }


def pct(pares, qs):
    pares = sorted(pares)
    tot = sum(a for _, a in pares)
    out = []
    for q in qs:
        acc = 0.0
        for v, a in pares:
            acc += a
            if acc >= q * tot:
                out.append(v)
                break
        else:
            out.append(float('nan'))
    return out


QS = (0.1, 0.25, 0.5, 0.75, 0.9)
UMBRALES_H = (40.0, 45.0, 50.0, 60.0, 70.0)


def distribucion(pares_h, pares_s):
    """Media, percentiles y fracciones de área por encima de cada umbral del grosor de la corteza
    (`pares_h`: (h, área)) y media del sedimento (`pares_s`)."""
    a = sum(x[1] for x in pares_h)
    if a <= 0:
        return {}
    return {
        'area_frac_planeta': a,
        'h_media_km': sum(h * w for h, w in pares_h) / a,
        'h_p10_p25_p50_p75_p90_km': pct(pares_h, QS),
        'h_mas_de_km': list(UMBRALES_H),
        'h_mas_de_frac': [sum(w for h, w in pares_h if h > u) / a for u in UMBRALES_H],
        's_media_km': sum(s * w for s, w in pares_s) / sum(w for _, w in pares_s),
        'h_mas_s_media_km': sum(h * w for h, w in pares_h) / a + sum(s * w for s, w in pares_s) / sum(w for _, w in pares_s),
    }


def corteza_planeta(ruta):
    """Grosor de la corteza continental del planeta al final, del volcado por celda (`run.dump_world`):
    toda la continental y la que está en tierra."""
    with open(ruta) as f:
        celdas = json.load(f)
    a_tot = sum(c['area_km2'] for c in celdas)
    todo_h, todo_s, tierra_h, tierra_s, mar_h, mar_s = [], [], [], [], [], []
    alt = []
    for c in celdas:
        t = c.get('tectonica')
        if not t or not t['continental']:
            continue
        a = c['area_km2'] / a_tot
        h, s = t['espesor_km'], t['sedimento_km']
        todo_h.append((h, a))
        todo_s.append((s, a))
        if c['surface'] != 'oceano':
            tierra_h.append((h, a))
            tierra_s.append((s, a))
            alt.append((h, c['h'], a))
        else:
            mar_h.append((h, a))
            mar_s.append((s, a))
    return {'continental': distribucion(todo_h, todo_s), 'tierra': distribucion(tierra_h, tierra_s),
            'bajo_el_mar': distribucion(mar_h, mar_s), 'altura_por_h': altura_por_h(alt)}


def altura_por_h(trios):
    """(h, z, área) → por clases de `h` (LIM_H): área y altura media."""
    clases = [[0.0, 0.0] for _ in LIM_H]
    for h, z, a in trios:
        i = max(i for i, l in enumerate(LIM_H) if h >= l) if h >= 0 else 0
        clases[i][0] += a
        clases[i][1] += z * a
    tot = sum(x[0] for x in clases)
    return {'desde_km': LIM_H, 'area_frac': [x[0] / tot if tot else 0.0 for x in clases],
            'altura_media_m': [x[1] / x[0] if x[0] > 0 else float('nan') for x in clases]}


# --- Programa -------------------------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser(description='La Tierra real con la regla del modelo: los continentes.')
    ap.add_argument('--datos', default=os.path.join(RAIZ, 'output', 'referencias'))
    ap.add_argument('--cache', default=os.path.join(RAIZ, 'output', 'cache'))
    ap.add_argument('--salida', default=os.path.join(RAIZ, 'config', 'referencias', 'tierra_corteza.toml'))
    ap.add_argument('--planeta', nargs='*', default=None,
                    help='volcados *_world.json del planeta (por defecto, output/semillas/semilla_*_world.json)')
    a = ap.parse_args()
    c = trm.cfg_modelo()
    k_sed = (c['rho_m'] - c['rho_s']) / (c['rho_m'] - c['rho_w'])
    m = 6 * N * N

    # ETOPO (roca rebotada sin hielo), Seton y GlobSed en celdas de nivel 3.
    cel = lo.relieve_roca_rebote(a.datos, a.cache)
    w = list(cel.w)
    z_real = [cel.zw[k] / cel.w[k] if cel.w[k] > 0 else float('nan') for k in range(m)]
    ruta_seton = os.path.join(a.datos, 'seton2020_edades_6m.grd')
    ruta_gs = os.path.join(a.datos, 'globsed', 'globsed-v3.asc')
    edad, frac = en_cache(a.cache, 'seton_n3', huella(trm.acumular_seton, rt.localizar, rt.leer_netcdf3,
                                                      extra=f'{N}:{os.path.getsize(ruta_seton)}'),
                          lambda: trm.acumular_seton(ruta_seton), 2, m)
    (sed_oc,) = en_cache(a.cache, 'globsed_n3', huella(trm.acumular_globsed, rt.localizar,
                                                       extra=f'{N}:{os.path.getsize(ruta_gs)}'),
                         lambda: [trm.acumular_globsed(ruta_gs)], 1, m)
    oc = [frac[k] >= 0.5 and math.isfinite(edad[k]) for k in range(m)]
    es_cont = [not x for x in oc]

    # CRUST1.0 en celdas de nivel 3.
    dest = descargar_crust1(a.datos)
    bnds, rho, tipos = leer_crust1(dest)
    col = columnas_crust1(bnds, rho, c['rho_m'])
    lab1, moho1 = litosfera_litho1(a.datos, a.cache)
    log('Centros de las celdas de nivel 3…')
    lat, lon = lat_lon_centros(N)
    log('CRUST1.0 y LITHO1.0 a celdas de nivel 3…')
    campos = {k: col[k] for k in ('h', 's', 'b_capas', 'rho_corteza', 'rho_sed', 'hielo')}
    campos['lab'] = lab1
    campos['moho_litho1'] = moho1
    ci, tipo = interpolar(campos, tipos, lat, lon, es_cont)
    h, s = ci['h'], ci['s']
    l_lm = [ci['lab'][k] - ci['moho_litho1'][k] for k in range(m)]   # manto litosférico (km)
    vol = c['gel_m'] / 1000.0 * A_TIERRA_KM2
    w_tot = sum(w)
    wn = [x / w_tot for x in w]   # fracción del planeta de cada celda
    area_km2 = [x * A_TIERRA_KM2 for x in wn]

    def nivel(z):
        return trm.nivel_del_mar(z, area_km2, vol)

    z_oc_regla = [con_flotacion(c, -trm.prof_modelo(c, edad[k]),
                                (c['rho_m'] - c['rho_s']) * (sed_oc[k] / 1000.0 if math.isfinite(sed_oc[k]) else 0.0))
                  if oc[k] else float('nan') for k in range(m)]
    z_cont_regla = [con_flotacion(c, 0.0, b_continental(c, h[k], s[k])) if es_cont[k] else float('nan')
                    for k in range(m)]
    b_ref = (c['rho_m'] - c['rho_c']) * c['h_nivel_km']
    z_cont_dens = [con_flotacion(c, 0.0, ci['b_capas'][k] - b_ref) if es_cont[k] else float('nan')
                   for k in range(m)]
    z_oc_mesetas = [con_flotacion(c, -trm.prof_modelo(c, edad[k]),
                                  (c['rho_m'] - c['rho_oc']) * max(0.0, h[k] - c['h_oc_km'])
                                  + (c['rho_m'] - c['rho_s']) * (sed_oc[k] / 1000.0 if math.isfinite(sed_oc[k]) else 0.0))
                    if oc[k] else float('nan') for k in range(m)]

    variantes = {
        'tierra': z_real,
        'tierra_1grado': a_1grado_y_vuelta(z_real, w, lat, lon),
        'regla_continentes': [z_cont_regla[k] if es_cont[k] else z_real[k] for k in range(m)],
        'regla': [z_cont_regla[k] if es_cont[k] else z_oc_regla[k] for k in range(m)],
        'regla_densidades_crust1': [z_cont_dens[k] if es_cont[k] else z_oc_regla[k] for k in range(m)],
        'regla_mesetas': [z_cont_regla[k] if es_cont[k] else z_oc_mesetas[k] for k in range(m)],
    }
    # Diagnóstico: cuánto pesa el sedimento (sin él, o contado como corteza) y si basta con mover el
    # cero de la regla (`h_nivel` que da la altura media real de la tierra, por bisección).
    variantes['regla_sin_sedimento'] = [con_flotacion(c, 0.0, b_continental(c, h[k], 0.0)) if es_cont[k]
                                        else z_oc_regla[k] for k in range(m)]
    variantes['regla_sedimento_como_corteza'] = [con_flotacion(c, 0.0, b_continental(c, h[k] + s[k], 0.0))
                                                 if es_cont[k] else z_oc_regla[k] for k in range(m)]

    def con_h_nivel(hn):
        cc = dict(c, h_nivel_km=hn)
        return [con_flotacion(cc, 0.0, b_continental(cc, h[k], s[k])) if es_cont[k] else z_oc_regla[k]
                for k in range(m)]

    def media_tierra(z):
        zeta = nivel(z)
        num = den = 0.0
        for zk, wk in zip(z, wn):
            if zk >= zeta:
                num += (zk - zeta) * wk
                den += wk
        return num / den
    lo_h, hi_h = c['h_nivel_km'], c['h_nivel_km'] + 15.0
    for _ in range(14):
        mid = 0.5 * (lo_h + hi_h)
        if media_tierra(con_h_nivel(mid)) > 671.0:
            lo_h = mid
        else:
            hi_h = mid
    h_nivel_media = 0.5 * (lo_h + hi_h)
    variantes['regla_cero_movido'] = con_h_nivel(h_nivel_media)
    res = {}
    for nombre, z in variantes.items():
        log(f'Midiendo {nombre}…')
        zeta = 0.0 if nombre.startswith('tierra') else nivel(z)
        res[nombre] = medir(z, w, zeta)
    res['regla_cero_movido']['h_nivel_km'] = h_nivel_media

    # La corteza continental de CRUST1.0: la de las celdas sin edad (la clase continental de la regla)
    # y la que está en tierra en la Tierra real.
    pares = lambda sel, v: [(v[k], wn[k]) for k in range(m) if sel(k) and math.isfinite(v[k])]  # noqa: E731
    cort = {
        'continental': distribucion(pares(lambda k: es_cont[k], h), pares(lambda k: es_cont[k], s)),
        'tierra': distribucion(pares(lambda k: es_cont[k] and z_real[k] >= 0, h),
                               pares(lambda k: es_cont[k] and z_real[k] >= 0, s)),
        'bajo_el_mar': distribucion(pares(lambda k: es_cont[k] and z_real[k] < 0, h),
                                    pares(lambda k: es_cont[k] and z_real[k] < 0, s)),
        'bajo_el_mar_tipo_continental': distribucion(
            pares(lambda k: es_cont[k] and z_real[k] < 0 and tipo[k] not in TIPOS_OCEANICOS, h),
            pares(lambda k: es_cont[k] and z_real[k] < 0 and tipo[k] not in TIPOS_OCEANICOS, s)),
    }
    # Densidad media de la corteza cristalina y del sedimento de CRUST1.0 en la tierra.
    tierra_k = [k for k in range(m) if es_cont[k] and z_real[k] >= 0]
    a_t = sum(wn[k] for k in tierra_k)
    rho_c_media = sum(ci['rho_corteza'][k] * wn[k] for k in tierra_k if math.isfinite(ci['rho_corteza'][k])) / a_t
    rho_s_media = (sum(ci['rho_sed'][k] * s[k] * wn[k] for k in tierra_k if math.isfinite(ci['rho_sed'][k]))
                   / sum(s[k] * wn[k] for k in tierra_k if math.isfinite(ci['rho_sed'][k])))

    # Altura real y de la regla por clases de grosor de la corteza (tierra y plataforma continental).
    zeta_r = res['regla']['nivel_mar_m']
    sel = [k for k in range(m) if es_cont[k] and z_real[k] >= -200.0]
    rel_h = {'desde_km': LIM_H}
    for etiqueta, z, z0 in (('real', z_real, 0.0), ('regla', variantes['regla'], zeta_r),
                            ('regla_densidades_crust1', variantes['regla_densidades_crust1'],
                             res['regla_densidades_crust1']['nivel_mar_m'])):
        t = altura_por_h([(h[k], z[k] - z0, wn[k]) for k in sel])
        rel_h['area_frac'] = t['area_frac']
        rel_h[f'altura_media_{etiqueta}_m'] = t['altura_media_m']
    # Grosor de la corteza a nivel del mar en la Tierra (celdas continentales de −100 a +100 m).
    costa = [(h[k], wn[k]) for k in range(m) if es_cont[k] and -100.0 <= z_real[k] <= 100.0]
    h_costa = pct(costa, QS)
    # Recta de la altura real frente a h en la tierra (mínimos cuadrados ponderados por área).
    sw = sum(wn[k] for k in tierra_k)
    mx = sum(h[k] * wn[k] for k in tierra_k) / sw
    my = sum(z_real[k] * wn[k] for k in tierra_k) / sw
    sxx = sum((h[k] - mx) ** 2 * wn[k] for k in tierra_k)
    sxy = sum((h[k] - mx) * (z_real[k] - my) * wn[k] for k in tierra_k)
    pend_real = sxy / sxx
    syy = sum((z_real[k] - my) ** 2 * wn[k] for k in tierra_k)
    r2 = sxy * sxy / (sxx * syy)

    # Residuo (regla − real) por tipo de corteza, en la tierra real.
    por_grupo = {}
    for nombre, codigos in GRUPOS:
        ks = [k for k in tierra_k if tipo[k] in codigos]
        aa = sum(wn[k] for k in ks)
        if aa <= 0:
            continue
        zr = sum(z_real[k] * wn[k] for k in ks) / aa
        zg = sum((variantes['regla'][k] - zeta_r) * wn[k] for k in ks) / aa
        zd = sum((variantes['regla_densidades_crust1'][k] - res['regla_densidades_crust1']['nivel_mar_m']) * wn[k]
                 for k in ks) / aa
        hm = sum(h[k] * wn[k] for k in ks) / aa
        sm = sum(s[k] * wn[k] for k in ks) / aa
        rc = [ci['rho_corteza'][k] for k in ks if math.isfinite(ci['rho_corteza'][k])]
        por_grupo[nombre] = {'area_tierra_frac': aa / a_t, 'h_media_km': hm, 's_media_km': sm,
                             'rho_corteza_media': sum(rc) / len(rc) if rc else float('nan'),
                             'altura_real_m': zr, 'altura_regla_m': zg, 'altura_regla_densidades_m': zd,
                             'sobre_2000_real_frac': sum(wn[k] for k in ks if z_real[k] >= 2000.0) / aa,
                             'sobre_2000_regla_frac': sum(wn[k] for k in ks if variantes['regla'][k] - zeta_r >= 2000.0) / aa}

    # La plataforma y los márgenes: a qué altura pone la regla la plataforma real (mar de menos de 200 m
    # sobre corteza continental) y la corteza continental sumergida de tipo continental.
    z_r = variantes['regla']
    plat_k = [k for k in range(m) if es_cont[k] and -200.0 <= z_real[k] < 0.0]
    a_pl = sum(wn[k] for k in plat_k)
    sum_k = [k for k in range(m) if es_cont[k] and z_real[k] < 0.0 and tipo[k] not in TIPOS_OCEANICOS]
    a_su = sum(wn[k] for k in sum_k)
    plataforma_regla = {
        'area_plataforma_real_frac_planeta': a_pl,
        'con_la_regla_tierra_frac': sum(wn[k] for k in plat_k if z_r[k] - zeta_r >= 0.0) / a_pl,
        'con_la_regla_plataforma_frac': sum(wn[k] for k in plat_k if -200.0 <= z_r[k] - zeta_r < 0.0) / a_pl,
        'con_la_regla_mas_honda_frac': sum(wn[k] for k in plat_k if z_r[k] - zeta_r < -200.0) / a_pl,
        'plataforma_real_h_media_km': sum(h[k] * wn[k] for k in plat_k) / a_pl,
        'plataforma_real_s_media_km': sum(s[k] * wn[k] for k in plat_k) / a_pl,
        'plataforma_real_altura_regla_media_m': sum((z_r[k] - zeta_r) * wn[k] for k in plat_k) / a_pl,
        'sumergida_continental_altura_real_media_m': sum(z_real[k] * wn[k] for k in sum_k) / a_su,
        'sumergida_continental_altura_regla_media_m': sum((z_r[k] - zeta_r) * wn[k] for k in sum_k) / a_su,
    }

    # El residuo (real − regla) frente al manto litosférico de LITHO1.0 y frente al sedimento, y las
    # rectas de la altura real con h, s y el manto litosférico (tierra continental).
    resid = {k: z_real[k] - (variantes['regla'][k] - zeta_r) for k in tierra_k}
    lim_l = [0.0, 50.0, 100.0, 150.0, 200.0, 250.0]
    lim_s = [0.0, 0.5, 1.0, 2.0, 4.0, 8.0]

    def por_clases(valor, lims):
        cl = [[0.0, 0.0, 0.0, 0.0, 0.0] for _ in lims]   # área, residuo, h, z real, L
        for k in tierra_k:
            v = valor(k)
            if not math.isfinite(v):
                continue
            i = max(i for i, x in enumerate(lims) if v >= x) if v >= lims[0] else 0
            cl[i][0] += wn[k]
            cl[i][1] += resid[k] * wn[k]
            cl[i][2] += h[k] * wn[k]
            cl[i][3] += z_real[k] * wn[k]
            cl[i][4] += l_lm[k] * wn[k]
        return {'desde': lims, 'area_tierra_frac': [x[0] / a_t for x in cl],
                'residuo_m': [x[1] / x[0] if x[0] > 0 else float('nan') for x in cl],
                'h_media_km': [x[2] / x[0] if x[0] > 0 else float('nan') for x in cl],
                'altura_real_m': [x[3] / x[0] if x[0] > 0 else float('nan') for x in cl],
                'manto_litosferico_km': [x[4] / x[0] if x[0] > 0 else float('nan') for x in cl]}
    res_lit = por_clases(lambda k: l_lm[k], lim_l)
    res_sed = por_clases(lambda k: s[k], lim_s)
    ks_r = [k for k in tierra_k if math.isfinite(l_lm[k])]
    b_hs, r2_hs = regresion([(z_real[k], h[k], s[k]) for k in ks_r], [wn[k] for k in ks_r])
    b_hsl, r2_hsl = regresion([(z_real[k], h[k], s[k], l_lm[k]) for k in ks_r], [wn[k] for k in ks_r])
    lit_media = sum(l_lm[k] * wn[k] for k in ks_r) / sum(wn[k] for k in ks_r)
    # La misma recta con un término por grupo de tipo de corteza (el primero, escudos y plataformas,
    # es la referencia): si el sedimento sigue sin levantar la tierra dentro de cada grupo.
    nombres_g = [n for n, _ in GRUPOS]
    grupo_de = {}
    for i, (_, cods) in enumerate(GRUPOS):
        for cd in cods:
            grupo_de[cd] = i
    filas_g = []
    for k in ks_r:
        g = grupo_de.get(tipo[k], 0)
        filas_g.append((z_real[k], h[k], s[k]) + tuple(1.0 if g == i else 0.0 for i in range(1, len(GRUPOS))))
    b_g, r2_g = regresion(filas_g, [wn[k] for k in ks_r])

    # Lo más que puede arreglar una corrección por el estado térmico de la litosfera, si se toma de los
    # tipos de corteza (la edad termotectónica): la regla más el residuo medio de su grupo en la tierra.
    # Y la recta real por grupo (h, s y grupo), con el océano real: lo más que da una regla lineal.
    desvio_g = [0.0] * len(GRUPOS)
    for i, (_, cods) in enumerate(GRUPOS):
        ks = [k for k in tierra_k if tipo[k] in cods]
        aa = sum(wn[k] for k in ks)
        if aa > 0:
            desvio_g[i] = sum(resid[k] * wn[k] for k in ks) / aa
    extra = {
        'regla_mas_desvio_por_tipo': [variantes['regla'][k] + desvio_g[grupo_de.get(tipo[k], 0)] if es_cont[k]
                                      else z_oc_regla[k] for k in range(m)],
        'recta_real_por_tipo': [b_g[0] + b_g[1] * h[k] + b_g[2] * s[k]
                                + (b_g[2 + grupo_de.get(tipo[k], 0)] if grupo_de.get(tipo[k], 0) > 0 else 0.0)
                                if es_cont[k] else z_real[k] for k in range(m)],
    }
    for nombre, z in extra.items():
        log(f'Midiendo {nombre}…')
        variantes[nombre] = z
        res[nombre] = medir(z, w, nivel(z))

    # El planeta: grosor de su corteza continental al final.
    rutas = a.planeta if a.planeta is not None else sorted(glob.glob(os.path.join(RAIZ, 'output', 'semillas', 'semilla_*_world.json')))
    planeta = {}
    for ruta in rutas:
        log(f'Planeta: {os.path.relpath(ruta, RAIZ)}…')
        planeta[os.path.basename(ruta).replace('_world.json', '')] = corteza_planeta(ruta)

    # --- Salida ---
    claves = [('nivel_mar_m', 'Nivel del mar respecto al dato (m)', '{:+.0f}'),
              ('tierra_frac', 'Tierra emergida (fracción)', '{:.3f}'),
              ('altura_media_tierra_m', 'Altura media de la tierra (m)', '{:.0f}'),
              ('altura_mediana_tierra_m', 'Altura mediana de la tierra (m)', '{:.0f}'),
              ('tierra_bajo_200_frac', 'Tierra por debajo de 200 m', '{:.3f}'),
              ('tierra_sobre_2000_frac', 'Tierra por encima de 2 000 m', '{:.3f}'),
              ('pendiente_media_tierra', 'Pendiente media de la tierra (±25 km)', '{:.4f}'),
              ('plataforma_km', 'Anchura media de la plataforma (km)', '{:.1f}'),
              ('profundidad_media_oceano_m', 'Profundidad media del océano (m)', '{:.0f}')]
    nombres = list(variantes)
    print('\nRelieve, nivel 3 (~26 km):')
    print(f'{"":<40}' + ''.join(f'{n[:15]:>16}' for n in nombres))
    for clave, etq, fmt in claves:
        print(f'{etq:<40}' + ''.join(f'{fmt.format(res[n][clave]):>16}' for n in nombres))
    print(f'\nCorteza continental de CRUST1.0 (celdas sin edad de Seton): densidad media de la corteza en tierra '
          f'{rho_c_media:.0f} kg/m³ (modelo {c["rho_c"]:.0f}); del sedimento {rho_s_media:.0f} (modelo {c["rho_s"]:.0f})')
    for zona, d in cort.items():
        print(f'  {zona:<12} {100 * d["area_frac_planeta"]:5.1f} % del planeta; h media {d["h_media_km"]:.1f} km '
              f'(p10–p90 {", ".join(f"{x:.1f}" for x in d["h_p10_p25_p50_p75_p90_km"])}); '
              f'más de {", ".join(f"{u:.0f}" for u in UMBRALES_H)} km: '
              f'{", ".join(f"{100 * x:.1f}" for x in d["h_mas_de_frac"])} %; sedimento {d["s_media_km"]:.2f} km')
    print(f'  grosor de la corteza a nivel del mar (tierra continental de −100 a +100 m): p10–p90 '
          f'{", ".join(f"{x:.1f}" for x in h_costa)} km (modelo: h_nivel = {c["h_nivel_km"]:.1f})')
    print(f'  recta de la altura real frente a h en la tierra: {pend_real:.0f} m por km (r² {r2:.2f}); '
          f'la regla: {1000 * (c["rho_m"] - c["rho_c"]) / c["rho_m"]:.0f} m por km')
    print('\nAltura media por grosor de la corteza (tierra y plataforma continental de la Tierra real):')
    print(f'{"h (km)":>8} {"área %":>7} {"real":>7} {"regla":>7} {"dens.":>7}')
    for i, l in enumerate(LIM_H):
        if rel_h['area_frac'][i] < 1e-4:
            continue
        print(f'{l:>8.1f} {100 * rel_h["area_frac"][i]:>7.2f} {rel_h["altura_media_real_m"][i]:>7.0f} '
              f'{rel_h["altura_media_regla_m"][i]:>7.0f} {rel_h["altura_media_regla_densidades_crust1_m"][i]:>7.0f}')
    print('\nPor tipo de corteza de CRUST1.0 (tierra real):')
    for nombre, g in por_grupo.items():
        print(f'  {nombre:<46} {100 * g["area_tierra_frac"]:5.1f} % de la tierra; h {g["h_media_km"]:.1f} km, '
              f's {g["s_media_km"]:.1f} km, ρ_c {g["rho_corteza_media"]:.0f}; altura real {g["altura_real_m"]:.0f} m, '
              f'regla {g["altura_regla_m"]:.0f}, con densidades {g["altura_regla_densidades_m"]:.0f}; '
              f'>2 km: real {100 * g["sobre_2000_real_frac"]:.1f} %, regla {100 * g["sobre_2000_regla_frac"]:.1f} %')
    pr = plataforma_regla
    print(f'\nLa plataforma real ({100 * pr["area_plataforma_real_frac_planeta"]:.1f} % del planeta; h {pr["plataforma_real_h_media_km"]:.1f} km, '
          f's {pr["plataforma_real_s_media_km"]:.1f} km) con la regla: tierra {100 * pr["con_la_regla_tierra_frac"]:.0f} %, '
          f'plataforma {100 * pr["con_la_regla_plataforma_frac"]:.0f} %, más honda {100 * pr["con_la_regla_mas_honda_frac"]:.0f} % '
          f'(altura media {pr["plataforma_real_altura_regla_media_m"]:.0f} m); corteza continental sumergida: real '
          f'{pr["sumergida_continental_altura_real_media_m"]:.0f} m, regla {pr["sumergida_continental_altura_regla_media_m"]:.0f} m')
    print('\nResiduo (real − regla) en la tierra frente al manto litosférico de LITHO1.0 (LAB − Moho):')
    for i, l in enumerate(lim_l):
        if res_lit['area_tierra_frac'][i] > 0:
            print(f'  desde {l:>5.0f} km: {100 * res_lit["area_tierra_frac"][i]:5.1f} % de la tierra, residuo '
                  f'{res_lit["residuo_m"][i]:6.0f} m (h {res_lit["h_media_km"][i]:.1f} km, altura real {res_lit["altura_real_m"][i]:.0f} m)')
    print('Residuo frente al sedimento:')
    for i, l in enumerate(lim_s):
        if res_sed['area_tierra_frac'][i] > 0:
            print(f'  desde {l:>4.1f} km: {100 * res_sed["area_tierra_frac"][i]:5.1f} % de la tierra, residuo '
                  f'{res_sed["residuo_m"][i]:6.0f} m (h {res_sed["h_media_km"][i]:.1f} km, manto litosférico '
                  f'{res_sed["manto_litosferico_km"][i]:.0f} km)')
    print(f'Recta de la altura real (m) en la tierra: {b_hs[0]:.0f} + {b_hs[1]:.0f}·h + {b_hs[2]:.0f}·s (r² {r2_hs:.2f}); '
          f'con el manto litosférico L: {b_hsl[0]:.0f} + {b_hsl[1]:.0f}·h + {b_hsl[2]:.0f}·s {b_hsl[3]:+.1f}·L (r² {r2_hsl:.2f}); '
          f'L medio {lit_media:.0f} km. La regla: {1000 * (c["rho_m"] - c["rho_c"]) / c["rho_m"]:.0f}·h y '
          f'{1000 * (c["rho_m"] - c["rho_s"]) / c["rho_m"]:.0f}·s')
    print(f'Con un término por grupo de corteza (r² {r2_g:.2f}): {b_g[0]:.0f} + {b_g[1]:.0f}·h + {b_g[2]:.0f}·s; '
          + '; '.join(f'{nombres_g[i]} {b_g[2 + i]:+.0f}' for i in range(1, len(GRUPOS))))
    print(f'Mover el cero de la regla (h_nivel, por bisección hacia la altura media real): {h_nivel_media:.1f} km '
          f'(modelo {c["h_nivel_km"]:.1f}); con el agua del modelo, la tierra se hunde: ver la columna regla_cero_movido')
    if planeta:
        print('\nCorteza continental del planeta al final (volcado por celda):')
        for nombre, p in planeta.items():
            for zona in ('continental', 'tierra', 'bajo_el_mar'):
                d = p[zona]
                if not d:
                    continue
                print(f'  {nombre:<12} {zona:<12} {100 * d["area_frac_planeta"]:5.1f} %; h media {d["h_media_km"]:.1f} km '
                      f'(p10–p90 {", ".join(f"{x:.1f}" for x in d["h_p10_p25_p50_p75_p90_km"])}); más de '
                      f'{", ".join(f"{u:.0f}" for u in UMBRALES_H)} km: {", ".join(f"{100 * x:.1f}" for x in d["h_mas_de_frac"])} %; '
                      f'sedimento {d["s_media_km"]:.2f} km')
        def media_sd(vals):
            mu = sum(vals) / len(vals)
            sd = math.sqrt(sum((v - mu) ** 2 for v in vals) / max(len(vals) - 1, 1))
            return mu, sd
        print('\n  Media de las semillas ± desviación, frente a CRUST1.0:')
        for zona in ('continental', 'tierra', 'bajo_el_mar'):
            ds = [p[zona] for p in planeta.values() if p[zona]]
            if not ds:
                continue
            ref = cort[zona]
            filas_r = [('área (fracción del planeta)', [d['area_frac_planeta'] for d in ds], ref['area_frac_planeta'], '{:.3f}'),
                       ('h media (km)', [d['h_media_km'] for d in ds], ref['h_media_km'], '{:.1f}')]
            for i, q in enumerate(QS):
                filas_r.append((f'h p{int(100 * q)} (km)', [d['h_p10_p25_p50_p75_p90_km'][i] for d in ds],
                                ref['h_p10_p25_p50_p75_p90_km'][i], '{:.1f}'))
            for i, u in enumerate(UMBRALES_H):
                filas_r.append((f'h > {u:.0f} km (fracción)', [d['h_mas_de_frac'][i] for d in ds], ref['h_mas_de_frac'][i], '{:.3f}'))
            filas_r.append(('sedimento medio (km)', [d['s_media_km'] for d in ds], ref['s_media_km'], '{:.2f}'))
            print(f'  {zona}:')
            for etq, vals, r, fmt in filas_r:
                mu, sd = media_sd(vals)
                print(f'    {etq:<28} planeta {fmt.format(mu)} ± {fmt.format(sd)}   CRUST1.0 {fmt.format(r)}')
        print('\n  Altura media de la tierra del planeta por grosor de la corteza (media de las semillas):')
        for i, l in enumerate(LIM_H):
            v = [p['altura_por_h']['altura_media_m'][i] for p in planeta.values()
                 if p['altura_por_h']['area_frac'][i] > 1e-4]
            ar = [p['altura_por_h']['area_frac'][i] for p in planeta.values()]
            if v:
                print(f'  {l:>8.1f} km: {100 * sum(ar) / len(ar):6.2f} % de la tierra, {sum(v) / len(v):6.0f} m')

    secciones = [(f'relieve.{n}', r) for n, r in res.items()]
    secciones += [(f'corteza_crust1.{zona}', d) for zona, d in cort.items()]
    secciones.append(('corteza_crust1.densidades', {'rho_corteza_tierra_kg_m3': rho_c_media,
                                                    'rho_sedimento_tierra_kg_m3': rho_s_media}))
    secciones.append(('corteza_crust1.nivel_del_mar', {'h_tierra_de_menos100_a_100m_p10_p25_p50_p75_p90_km': h_costa,
                                                       'recta_altura_real_m_por_km': pend_real,
                                                       'recta_r2': r2}))
    secciones.append(('altura_por_grosor', rel_h))
    secciones.append(('plataforma_con_la_regla', plataforma_regla))
    secciones.append(('residuo.por_manto_litosferico_litho1', res_lit))
    secciones.append(('residuo.por_sedimento', res_sed))
    secciones.append(('residuo.rectas', {'altura_h_s_m': b_hs, 'r2_h_s': r2_hs, 'altura_h_s_l_m': b_hsl,
                                         'r2_h_s_l': r2_hsl, 'manto_litosferico_medio_km': lit_media,
                                         'altura_h_s_grupos_m': b_g, 'r2_h_s_grupos': r2_g,
                                         'grupos': nombres_g}))
    secciones += [(f'por_tipo."{n}"', g) for n, g in por_grupo.items()]
    cab = ('# La Tierra real con la regla del modelo, para los continentes: relieve y grosor de la corteza.\n'
           '# Generado por herramientas/analisis/tierra_corteza_regla.py: corteza y sedimento de CRUST1.0\n'
           '# (Laske et al. 2013, igppweb.ucsd.edu/~gabi/crust1.html), edades de Seton et al. (2020),\n'
           '# sedimento oceánico de GlobSed (Straume et al. 2019) y la roca sin hielo con su rebote de ETOPO 2022\n'
           '# y Paxman et al. (2022), en celdas de nivel 3 (~26 km), con la regla de config/terreno.toml.\n'
           '# "tierra": la roca real; "tierra_1grado": la misma a la resolución de CRUST1.0; "regla": toda la\n'
           '# Tierra con la isostasia y el nivel del mar del modelo; "regla_continentes": solo los continentes.\n')
    rt.escribir_toml(a.salida, secciones, cab)
    log(f'escrito {a.salida}')


if __name__ == '__main__':
    main()
