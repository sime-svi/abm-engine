#!/usr/bin/env python3
"""Tablas de referencia de la Tierra para el relieve del planeta (diseño del relieve tectónico §10).

Calcula, a las mismas resoluciones que el planeta (niveles 0 a 3 del cubo esférico: ~208, 104, 52 y
26 km), las cifras con las que se compara el relieve, y las escribe en
`config/referencias/tierra_relieve.toml`. Solo guarda tablas resumen, con nombre, versión y fecha de
cada conjunto de datos; los datos brutos se descargan una vez a `output/referencias/` (ignorado por
git).

Conjuntos de datos:
- ETOPO 2022 (NOAA NCEI), 60 arcosegundos, versiones «superficie» y «roca» (bajo el hielo), leídas
  por OPeNDAP cada 2 minutos de arco (~3,7 km). La roca solo difiere de la superficie bajo los
  casquetes, así que se lee solo en las franjas polares (latitud < −60° y > 59°).
- Paxman, Austermann y Hollyday (2022), versión 3 (2026): roca de la Antártida y de Groenlandia con el
  rebote isostático completo sin hielo, respecto al nivel del mar actual. Se leen 1 de cada 10 filas y
  columnas (5 km) en la Antártida y 1 de cada 30 (4,5 km) en Groenlandia, con peticiones parciales.
- Seton et al. (2020), malla de edades del fondo oceánico (GTS2012, 6 minutos de arco, NetCDF-3).
- Bird (2003), modelo de placas PB2002: áreas de las 52 placas.
- Argus et al. (2011), NNR-MORVEL56: velocidad media cuadrática de la litosfera (de la publicación).

Versiones del relieve:
- `superficie`: ETOPO superficie (encima del hielo).
- `roca`: ETOPO roca (bajo el hielo, hundida por su peso).
- `roca_rebote`: ETOPO roca fuera de los casquetes y Paxman et al. dentro: roca desnuda en equilibrio
  isostático, como la del planeta. Es la referencia de la comparación (decisión del responsable,
  2026-11-05). El agua es la del océano actual.

Uso: herramientas/analisis/referencia_tierra.py [--datos output/referencias] [--salida config/referencias/tierra_relieve.toml]
Solo usa la biblioteca estándar de Python (tarda ~15 min la primera vez; después, ~10 min de cálculo).
"""
import argparse
import array
import datetime
import math
import os
import struct
import sys
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import h5mini  # noqa: E402

RAIZ = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
N0 = 48                 # celdas por arista de cara en el nivel 0 (config/terreno.toml)
NIVELES = 4             # niveles 0..3
N3 = N0 << (NIVELES - 1)
R_KM = 6371.0
D_ORO_KM = 25.0          # escala de la pendiente que ve la lluvia (config/terreno.toml)
K_ORO, M_ORO_MIN, M_ORO_MAX = 40.0, 0.2, 3.0

ETOPO = 'https://www.ngdc.noaa.gov/thredds/dodsC/global/ETOPO2022/60s/'
ETOPO_SUP = ETOPO + '60s_surface_elev_netcdf/ETOPO_2022_v1_60s_N90W180_surface.nc'
ETOPO_ROCA = ETOPO + '60s_bed_elev_netcdf/ETOPO_2022_v1_60s_N90W180_bed.nc'
PASO_ETOPO = 2           # 1 de cada 2 píxeles de 1' (2')
FILAS, COLS = 10800, 21600
OBJ = 'https://arcticdata.io/metacat/d1/mn/v2/object/'
PAXMAN_ANT = OBJ + 'urn:uuid:da941516-3797-4d0c-b077-75b342ffb583'   # BedMachineReboundedAntarcticav4.nc
PAXMAN_GRE = OBJ + 'urn:uuid:bcae0781-4a58-4269-994f-9c76e073285c'   # BedMachineReboundedGreenlandv6.nc
EDADES = 'https://www.earthbyte.org/webdav/ftp/earthbyte/agegrid/2020/Grids/age.2020.1.GTS2012.6m.grd'
BIRD = 'http://peterbird.name/oldFTP/PB2002/PB2002_plates.dig.txt'

LIM_TIERRA = [0.0, 200.0, 500.0, 1000.0, 2000.0, 3000.0, 5000.0]
LIM_MAR = [0.0, 200.0, 1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0]
CLASES_TIERRA = [0.0, 200.0, 500.0, 1000.0, 2000.0, 3000.0]
CLASES_MAR = [0.0, 200.0, 2000.0, 4000.0]


def log(*a):
    print(*a, file=sys.stderr, flush=True)


# --- Geometría del cubo esférico (la misma que crates/abm-terreno/src/geometry.rs) ---------------

def cara_y_coords(x, y, z):
    ax, ay, az = abs(x), abs(y), abs(z)
    if ax >= ay and ax >= az:
        return (0, y, z, x) if x > 0 else (2, -y, z, -x)
    if ay >= ax and ay >= az:
        return (1, -x, z, y) if y > 0 else (3, x, z, -y)
    return (4, y, -x, z) if z > 0 else (5, y, x, -z)


def localizar(x, y, z, n):
    c, pa, pb, dn = cara_y_coords(x, y, z)
    paso = (math.pi / 2) / n
    i = int(math.floor((math.atan(pa / dn) + math.pi / 4) / paso))
    j = int(math.floor((math.atan(pb / dn) + math.pi / 4) / paso))
    i = min(max(i, 0), n - 1)
    j = min(max(j, 0), n - 1)
    return (c * n + j) * n + i


BASES = {
    0: ((0, 1, 0), (0, 0, 1), (1, 0, 0)), 1: ((-1, 0, 0), (0, 0, 1), (0, 1, 0)),
    2: ((0, -1, 0), (0, 0, 1), (-1, 0, 0)), 3: ((1, 0, 0), (0, 0, 1), (0, -1, 0)),
    4: ((0, 1, 0), (-1, 0, 0), (0, 0, 1)), 5: ((0, 1, 0), (1, 0, 0), (0, 0, -1)),
}


def centro(idx, n):
    c, r = divmod(idx, n * n)
    j, i = divmod(r, n)
    a, b, nr = BASES[c]
    paso = (math.pi / 2) / n
    tx = math.tan(-math.pi / 4 + (i + 0.5) * paso)
    ty = math.tan(-math.pi / 4 + (j + 0.5) * paso)
    v = [nr[k] + tx * a[k] + ty * b[k] for k in range(3)]
    norma = math.sqrt(sum(t * t for t in v))
    return [t / norma for t in v]


def este_norte(d):
    e = [-d[1], d[0], 0.0]
    ne = math.hypot(e[0], e[1])
    e = [1.0, 0.0, 0.0] if ne < 1e-6 else [e[0] / ne, e[1] / ne, 0.0]
    nn = [d[1] * e[2] - d[2] * e[1], d[2] * e[0] - d[0] * e[2], d[0] * e[1] - d[1] * e[0]]
    return e, nn


def desplazar(d, u, signo, delta):
    cd, sd = math.cos(delta), math.sin(delta)
    return [cd * d[k] + signo * sd * u[k] for k in range(3)]


# --- Descargas ------------------------------------------------------------------------------------

def bajar(url, destino, rango=None):
    req = urllib.request.Request(url, headers={'Range': f'bytes={rango[0]}-{rango[1]}'} if rango else {})
    for intento in range(5):
        try:
            with urllib.request.urlopen(req, timeout=300) as r:
                datos = r.read()
            if destino:
                with open(destino + '.tmp', 'wb') as f:
                    f.write(datos)
                os.replace(destino + '.tmp', destino)
            return datos
        except Exception as e:  # noqa: BLE001
            log(f'  reintento {intento + 1} de {url}: {e}')
    raise RuntimeError(f'no se pudo descargar {url}')


def etopo(url, ruta, filas_desde, filas_hasta):
    """Filas [desde, hasta) de 1' leídas cada PASO_ETOPO, como float32 big-endian en `ruta`."""
    if os.path.exists(ruta):
        return
    trozo = 600
    with open(ruta + '.tmp', 'wb') as f:
        r0 = filas_desde
        while r0 < filas_hasta:
            r1 = min(r0 + trozo, filas_hasta) - 1
            q = f'{url}.dods?z.z%5B{r0}:{PASO_ETOPO}:{r1}%5D%5B0:{PASO_ETOPO}:{COLS - 1}%5D'
            b = bajar(q, None)
            p = b.find(b'\nData:\n') + 7
            cuenta = struct.unpack('>I', b[p:p + 4])[0]
            f.write(b[p + 8:p + 8 + 4 * cuenta])
            log(f'  ETOPO {os.path.basename(ruta)}: filas {r0}–{r1}')
            r0 += trozo
    os.replace(ruta + '.tmp', ruta)


def paxman(url, ruta, paso):
    """Rejilla rebotada respecto al nivel del mar actual, 1 de cada `paso` filas y columnas."""
    if os.path.exists(ruta):
        return
    fetch = h5mini.open_remote(url)
    sb = fetch(0, 64)
    raiz = h5mini.u(sb, 36, 8)
    enlaces = h5mini.links(h5mini.parse_ohdr(None, raiz, fetch))
    info = {k: h5mini.dataset_info(h5mini.parse_ohdr(None, a, fetch)) for k, a in enlaces.items()}
    var = info['bed_rebounded_modern_sl']
    ny, nx = var['dims']
    _, base, _ = var['layout']

    def coord(nombre):
        _, a, tam = info[nombre]['layout']
        return array.array('f', fetch(a, tam))
    xs, ys = coord('x'), coord('y')
    filas = list(range(0, ny, paso))
    with open(ruta + '.tmp', 'wb') as f:
        f.write(struct.pack('<III', len(filas), (nx + paso - 1) // paso, paso))
        array.array('f', xs[::paso]).tofile(f)
        array.array('f', [ys[r] for r in filas]).tofile(f)
        for k, r in enumerate(filas):
            fila = array.array('f', fetch(base + 4 * nx * r, 4 * nx))
            array.array('f', fila[::paso]).tofile(f)
            if k % 200 == 0:
                log(f'  Paxman {os.path.basename(ruta)}: fila {k}/{len(filas)}')
    os.replace(ruta + '.tmp', ruta)


# --- Proyección estereográfica polar inversa (Snyder 1987, elipsoide WGS84) ------------------------

A_WGS, E_WGS = 6378137.0, 0.0818191908426215


def estereo_inversa(x, y, lat_ts, lon0, sur):
    e = E_WGS
    phc = math.radians(abs(lat_ts))
    tc = math.tan(math.pi / 4 - phc / 2) / ((1 - e * math.sin(phc)) / (1 + e * math.sin(phc))) ** (e / 2)
    mc = math.cos(phc) / math.sqrt(1 - (e * math.sin(phc)) ** 2)
    if sur:
        x, y = -x, -y
    rho = math.hypot(x, y)
    t = rho * tc / (A_WGS * mc)
    phi = math.pi / 2 - 2 * math.atan(t)
    for _ in range(6):
        s = e * math.sin(phi)
        phi = math.pi / 2 - 2 * math.atan(t * ((1 - s) / (1 + s)) ** (e / 2))
    lam = math.radians(lon0) + math.atan2(x, -y)
    if sur:
        phi, lam = -phi, -lam + 2 * math.radians(lon0)
    m = math.cos(phi) / math.sqrt(1 - (e * math.sin(phi)) ** 2)
    k = rho / (A_WGS * m) if rho > 0 else 1.0
    return phi, lam, k


# --- Acumulación en celdas de nivel 3 -------------------------------------------------------------

class Celdas:
    def __init__(self, n):
        self.n = n
        m = 6 * n * n
        self.w = array.array('d', bytes(8 * m))
        self.zw = array.array('d', bytes(8 * m))
        self.z2w = array.array('d', bytes(8 * m))

    def media(self, k):
        return self.zw[k] / self.w[k] if self.w[k] > 0 else float('nan')

    def var(self, k):
        if self.w[k] <= 0:
            return 0.0
        m = self.zw[k] / self.w[k]
        return max(self.z2w[k] / self.w[k] - m * m, 0.0)


def acumular_etopo(sup, roca, n):
    """Tres versiones a la vez: superficie, roca y (de momento) roca; Paxman sustituye después."""
    out = {v: Celdas(n) for v in ('superficie', 'roca', 'roca_rebote')}
    nc = COLS // PASO_ETOPO
    lons = [math.radians(-180.0 + (c * PASO_ETOPO + 0.5) / 60.0) for c in range(nc)]
    cl = [math.cos(t) for t in lons]
    sl = [math.sin(t) for t in lons]
    filas_sup = len(sup) // nc
    fila_roca0, filas_roca = roca
    vs, vr, vb = out['superficie'], out['roca'], out['roca_rebote']
    for r in range(filas_sup):
        i1 = r * PASO_ETOPO
        lat = math.radians(-90.0 + (i1 + 0.5) / 60.0)
        cp, sp = math.cos(lat), math.sin(lat)
        w = cp
        fila = sup[r * nc:(r + 1) * nc]
        rr = r - fila_roca0
        fila_r = filas_roca[rr] if 0 <= rr < len(filas_roca) and filas_roca[rr] is not None else None
        ws, zs, z2s = vs.w, vs.zw, vs.z2w
        wr, zr, z2r = vr.w, vr.zw, vr.z2w
        wb, zb, z2b = vb.w, vb.zw, vb.z2w
        for c in range(nc):
            k = localizar(cp * cl[c], cp * sl[c], sp, n)
            z = fila[c]
            ws[k] += w
            zs[k] += z * w
            z2s[k] += z * z * w
            zz = fila_r[c] if fila_r is not None else z
            wr[k] += w
            zr[k] += zz * w
            z2r[k] += zz * zz * w
            wb[k] += w
            zb[k] += zz * w
            z2b[k] += zz * zz * w
        if r % 400 == 0:
            log(f'  binning ETOPO: fila {r}/{filas_sup}')
    return out


def leer_paxman(ruta):
    with open(ruta, 'rb') as f:
        nf, nx, paso = struct.unpack('<III', f.read(12))
        xs = array.array('f')
        xs.fromfile(f, nx)
        ys = array.array('f')
        ys.fromfile(f, nf)
        z = array.array('f')
        z.fromfile(f, nf * nx)
    return nf, nx, xs, ys, z


def sustituir_paxman(cel, ruta, lat_ts, lon0, sur, n):
    nf, nx, xs, ys, z = leer_paxman(ruta)
    w = Celdas(n)
    for r in range(nf):
        y = ys[r]
        for c in range(nx):
            v = z[r * nx + c]
            if not math.isfinite(v) or v < -20000 or v > 20000:
                continue
            phi, lam, k = estereo_inversa(xs[c], y, lat_ts, lon0, sur)
            cp = math.cos(phi)
            kk = localizar(cp * math.cos(lam), cp * math.sin(lam), math.sin(phi), n)
            peso = 1.0 / (k * k)
            w.w[kk] += peso
            w.zw[kk] += v * peso
            w.z2w[kk] += v * v * peso
    cambiadas = 0
    for kk in range(len(w.w)):
        if w.w[kk] > 0:
            # Se conserva el peso (área) de ETOPO y se toma la media y la varianza de Paxman.
            m = w.zw[kk] / w.w[kk]
            var = max(w.z2w[kk] / w.w[kk] - m * m, 0.0)
            a = cel.w[kk]
            cel.zw[kk] = m * a
            cel.z2w[kk] = (var + m * m) * a
            cambiadas += 1
    log(f'  Paxman {os.path.basename(ruta)}: {cambiadas} celdas de nivel 3 sustituidas')
    return cambiadas


def subir_nivel(cel):
    """Celdas del nivel siguiente hacia arriba (madre = (c, i/2, j/2)): sumas exactas."""
    n = cel.n // 2
    out = Celdas(n)
    for k in range(len(cel.w)):
        c, r = divmod(k, cel.n * cel.n)
        j, i = divmod(r, cel.n)
        m = (c * n + j // 2) * n + i // 2
        out.w[m] += cel.w[k]
        out.zw[m] += cel.zw[k]
        out.z2w[m] += cel.z2w[k]
    return out


# --- Estadísticas ---------------------------------------------------------------------------------

def mediana_ponderada(pares):
    pares.sort()
    tot = sum(a for _, a in pares)
    acc = 0.0
    for h, a in pares:
        acc += a
        if acc >= 0.5 * tot:
            return h
    return float('nan')


def franja(h, lim):
    k = 0
    for i, l in enumerate(lim):
        if h >= l:
            k = i
    return k


def estadisticas(cel):
    m = len(cel.w)
    a_tot = sum(cel.w)
    tierra, mar = [], []
    for k in range(m):
        if cel.w[k] <= 0:
            continue
        h = cel.zw[k] / cel.w[k]
        (tierra if h >= 0.0 else mar).append((h, cel.w[k]))
    a_t = sum(a for _, a in tierra)
    a_m = sum(a for _, a in mar)
    hip_t = [0.0] * len(LIM_TIERRA)
    for h, a in tierra:
        hip_t[franja(h, LIM_TIERRA)] += a
    hip_m = [0.0] * len(LIM_MAR)
    for h, a in mar:
        hip_m[franja(-h, LIM_MAR)] += a
    hist = [0.0] * 40          # todo el planeta, franjas de 500 m de −11 000 a +9 000 m
    for h, a in tierra + mar:
        b = int(math.floor((h + 11000.0) / 500.0))
        hist[min(max(b, 0), 39)] += a
    return {
        'tierra_frac': a_t / a_tot,
        'altura_media_tierra_m': sum(h * a for h, a in tierra) / a_t,
        'altura_mediana_tierra_m': mediana_ponderada(list(tierra)),
        'hipsometria_frac': [x / a_t for x in hip_t],
        'profundidad_media_oceano_m': -sum(h * a for h, a in mar) / a_m,
        'hipsometria_oceano_frac': [x / a_m for x in hip_m],
        'histograma_planeta_frac': [x / a_tot for x in hist],
    }


def relieve_por_escala(madres, hijas, sub=None):
    """Desviación típica de las alturas dentro de cada celda madre (de sus 4 hijas, o de los
    píxeles si `sub` es None y `hijas` es el nivel 3), por clase de la madre: el espectro del
    relieve real por escala y tipo (decisión 6 del responsable)."""
    clases = {('tierra', l): [0.0, 0.0] for l in CLASES_TIERRA}
    clases.update({('mar', l): [0.0, 0.0] for l in CLASES_MAR})
    for k in range(len(madres.w)):
        if madres.w[k] <= 0:
            continue
        hm = madres.zw[k] / madres.w[k]
        if hijas is None:
            var = madres.var(k)
        else:
            c, r = divmod(k, madres.n * madres.n)
            j, i = divmod(r, madres.n)
            n2 = hijas.n
            sw = szw = sz2w = 0.0
            for dj in (0, 1):
                for di in (0, 1):
                    h = (c * n2 + 2 * j + dj) * n2 + 2 * i + di
                    if hijas.w[h] > 0:
                        zh = hijas.zw[h] / hijas.w[h]
                        sw += hijas.w[h]
                        szw += zh * hijas.w[h]
                        sz2w += zh * zh * hijas.w[h]
            if sw <= 0:
                continue
            var = max(sz2w / sw - (szw / sw) ** 2, 0.0)
        if hm >= 0:
            clave = ('tierra', max(l for l in CLASES_TIERRA if hm >= l))
        else:
            clave = ('mar', max(l for l in CLASES_MAR if -hm >= l))
        clases[clave][0] += var * madres.w[k]
        clases[clave][1] += madres.w[k]
    return {f'{t}_{int(l)}': (math.sqrt(v / a) if a > 0 else float('nan'), a)
            for (t, l), (v, a) in clases.items()}


def factor_orografico(cel):
    """Celdas de tierra de nivel 3 con su pendiente a 25 km como en el planeta (diferencias centrales
    a ±d_oro; el aire ve el mar plano) y su factor orográfico con k_oro = 40 y el viento del modelo
    (alisios y vientos polares del este, oestes en 30–60°): lista de (k, lat, lon, h, m_oro, área,
    |∇h|)."""
    n = cel.n
    delta = D_ORO_KM / R_KM
    alturas = [cel.media(k) for k in range(len(cel.w))]

    def h_en(p):
        return alturas[localizar(p[0], p[1], p[2], n)]
    filas = []
    for k in range(len(cel.w)):
        h = alturas[k]
        if not (h >= 0.0):
            continue
        d = centro(k, n)
        e, nn = este_norte(d)
        he, ho = h_en(desplazar(d, e, 1, delta)), h_en(desplazar(d, e, -1, delta))
        hn, hs = h_en(desplazar(d, nn, 1, delta)), h_en(desplazar(d, nn, -1, delta))
        den = 2 * D_ORO_KM * 1000.0
        ge = (he - ho) / den
        gn = (hn - hs) / den
        ge_aire = (max(he, 0.0) - max(ho, 0.0)) / den
        s = math.hypot(ge, gn)
        lat = math.degrees(math.asin(max(-1.0, min(1.0, d[2]))))
        viento = 1.0 if 30.0 <= abs(lat) < 60.0 else -1.0
        m_oro = min(max(1.0 + K_ORO * ge_aire * viento, M_ORO_MIN), M_ORO_MAX)
        lon = math.degrees(math.atan2(d[1], d[0]))
        filas.append((k, lat, lon, h, m_oro, cel.w[k], s))
    return filas


def laderas_barlovento(filas):
    """Las filas de `factor_orografico` que son laderas de barlovento de las cordilleras grandes, con
    la misma definición que herramientas/analisis/terreno.py: suben con el viento (m_oro > 1),
    h ≥ 300 m y cresta > 2000 m a 0–150 km a favor del viento (casillas de 0,25°)."""
    rel = {}
    for k, lat, lon, h, _, _, _ in filas:
        kk = (math.floor(lat / 0.25), math.floor(lon / 0.25))
        rel[kk] = max(rel.get(kk, -1e9), h)
    lad = []
    for fila in filas:
        k, lat, lon, h, m_oro, a, _ = fila
        if m_oro <= 1.0 or h < 300.0:
            continue
        coslat = max(math.cos(math.radians(lat)), 1e-3)
        hacia_el_este = 1.0 if 30.0 <= abs(lat) < 60.0 else -1.0
        kl = math.floor(lat / 0.25)
        dd, hay = 0.0, False
        while dd <= 150.0 and not hay:
            lo = lon + hacia_el_este * math.degrees(dd / (R_KM * coslat))
            lo = (lo + 180.0) % 360.0 - 180.0
            ko = math.floor(lo / 0.25)
            hay = any(rel.get((kl + dk, ko), -1e9) > 2000.0 for dk in (-1, 0, 1))
            dd += 10.0
        if hay:
            lad.append(fila)
    return lad


def pendientes(cel):
    """Pendiente a 25 km como en el planeta en la tierra por encima de 0/500/1000/2000 m, y factor
    orográfico medio de las laderas de barlovento de las cordilleras grandes (`factor_orografico`)."""
    filas = factor_orografico(cel)
    por_clase = {l: [] for l in (0.0, 500.0, 1000.0, 2000.0)}
    for _, _, _, h, _, a, s in filas:
        for l in por_clase:
            if h >= l:
                por_clase[l].append((s, a))
    out = {}
    for l, v in por_clase.items():
        v.sort()
        tot = sum(a for _, a in v)
        qs = []
        for q in (0.5, 0.75, 0.9, 0.95, 0.99):
            acc = 0.0
            for s, a in v:
                acc += a
                if acc >= q * tot:
                    qs.append(s)
                    break
        out[f'tierra_sobre_{int(l)}'] = {'media': sum(s * a for s, a in v) / tot, 'percentiles': qs}
    lad_a = lad_m = 0.0
    for _, _, _, _, m_oro, a, _ in laderas_barlovento(filas):
        lad_a += a
        lad_m += m_oro * a
    out['barlovento_m_oro'] = lad_m / lad_a if lad_a else float('nan')
    return out


def plataforma(cel):
    """Anchura media de la plataforma continental: área del mar de menos de 200 m de profundidad
    entre la longitud de costa (aristas tierra-mar de las celdas de nivel 3)."""
    n = cel.n
    alt = [cel.media(k) for k in range(len(cel.w))]
    area_plat = sum(cel.w[k] for k in range(len(alt)) if -200.0 <= alt[k] < 0.0)
    costa_km = 0.0
    lado = (math.pi / 2) * R_KM / n
    for k in range(len(alt)):
        if not alt[k] >= 0.0:
            continue
        c, r = divmod(k, n * n)
        j, i = divmod(r, n)
        for di, dj in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            i2, j2 = i + di, j + dj
            if 0 <= i2 < n and 0 <= j2 < n:
                k2 = (c * n + j2) * n + i2
                if alt[k2] < 0.0:
                    costa_km += lado
    a_tot = sum(cel.w)
    area_km2 = area_plat / a_tot * 4 * math.pi * R_KM ** 2
    return {'area_plataforma_km2': area_km2, 'costa_km': costa_km,
            'anchura_media_km': area_km2 / costa_km if costa_km else float('nan')}


def vecinas_cubo(n):
    """Vecinas (8) de cada celda del cubo de `n` por arista, cruzando aristas de cara por geometría
    (como `geometry::neighbor_addr`)."""
    paso = (math.pi / 2) / n
    out = []
    for idx in range(6 * n * n):
        c, r = divmod(idx, n * n)
        j, i = divmod(r, n)
        a, b, nr = BASES[c]
        vs = []
        for dj in (-1, 0, 1):
            for di in (-1, 0, 1):
                if di == 0 and dj == 0:
                    continue
                ii, jj = i + di, j + dj
                if 0 <= ii < n and 0 <= jj < n:
                    vs.append((c * n + jj) * n + ii)
                    continue
                if not (0 <= ii < n) and not (0 <= jj < n):
                    continue
                fi = -1e-3 if ii < 0 else (n + 1e-3 if ii >= n else ii + 0.5)
                fj = -1e-3 if jj < 0 else (n + 1e-3 if jj >= n else jj + 0.5)
                tx = math.tan(-math.pi / 4 + fi * paso)
                ty = math.tan(-math.pi / 4 + fj * paso)
                v = [nr[k] + tx * a[k] + ty * b[k] for k in range(3)]
                vs.append(localizar(v[0], v[1], v[2], n))
        out.append(vs)
    return out


def depresiones(cel):
    """Tierra en cuencas cerradas a la resolución de la celda: priority-flood desde el océano (como
    `hydro.rs`). Devuelve la fracción de la tierra en depresión y el nº de celdas."""
    import heapq
    n = cel.n
    h = [cel.media(k) for k in range(len(cel.w))]
    vec = vecinas_cubo(n)
    lleno = list(h)
    visto = [x < 0.0 for x in h]
    heap = [(h[k], k) for k in range(len(h)) if visto[k]]
    heapq.heapify(heap)
    while heap:
        z, k = heapq.heappop(heap)
        for v in vec[k]:
            if not visto[v]:
                visto[v] = True
                lleno[v] = max(h[v], z)
                heapq.heappush(heap, (lleno[v], v))
    a_t = sum(cel.w[k] for k in range(len(h)) if h[k] >= 0.0)
    dep = [k for k in range(len(h)) if h[k] >= 0.0 and h[k] < lleno[k]]
    return {'celdas_en_depresion': len(dep), 'tierra_en_depresion_frac': sum(cel.w[k] for k in dep) / a_t,
            'celdas_tierra': sum(1 for x in h if x >= 0.0)}


# --- Edades del fondo oceánico (NetCDF-3) ---------------------------------------------------------

def leer_netcdf3(b):
    assert b[:3] == b'CDF'
    v = b[3]
    p = 4

    def i32():
        nonlocal p
        x = struct.unpack('>i', b[p:p + 4])[0]
        p += 4
        return x

    def off():
        nonlocal p
        if v == 2:
            x = struct.unpack('>q', b[p:p + 8])[0]
            p += 8
            return x
        return i32()

    def nombre():
        nonlocal p
        ln = i32()
        s = b[p:p + ln].decode()
        p += (ln + 3) // 4 * 4
        return s
    tam = {1: 1, 2: 1, 3: 2, 4: 4, 5: 4, 6: 8}

    def atributos():
        nonlocal p
        tag = i32()
        nat = i32()
        out = {}
        for _ in range(nat):
            nm = nombre()
            t = i32()
            ne = i32()
            ln = ne * tam[t]
            out[nm] = (t, b[p:p + ln])
            p += (ln + 3) // 4 * 4
        return out
    i32()  # numrecs
    tag = i32()
    ndims = i32()
    dims = []
    for _ in range(ndims):
        dims.append((nombre(), i32()))
    atributos()
    tag = i32()
    nvars = i32()
    variables = {}
    for _ in range(nvars):
        nm = nombre()
        nd = i32()
        ids = [i32() for _ in range(nd)]
        atributos()
        t = i32()
        i32()
        inicio = off()
        variables[nm] = ([dims[i][1] for i in ids], t, inicio)
    return variables


def edades(ruta):
    b = open(ruta, 'rb').read()
    var = leer_netcdf3(b)
    nombre_z = 'z' if 'z' in var else [k for k in var if len(var[k][0]) == 2][0]
    (ny, nx), t, inicio = var[nombre_z]
    fmt = {5: 'f', 6: 'd'}[t]
    es = 4 if fmt == 'f' else 8
    lat_dims, _, lat0 = var['lat'] if 'lat' in var else var['y']
    lats = struct.unpack(f'>{ny}d', b[lat0:lat0 + 8 * ny]) if var.get('lat', var.get('y'))[1] == 6 else \
        struct.unpack(f'>{ny}f', b[lat0:lat0 + 4 * ny])
    datos = array.array(fmt, b[inicio:inicio + es * nx * ny])
    if sys.byteorder == 'little':
        datos.byteswap()
    bins = [0.0] * 21   # de 10 en 10 Myr hasta 200, y > 200
    pares = []
    a_tot = 0.0
    for r in range(ny):
        w = math.cos(math.radians(lats[r]))
        if r in (0, ny - 1):
            w *= 0.5  # filas del polo (rejilla de nodos)
        for c in range(nx - 1):  # la última columna repite la primera (−180 = +180)
            a = datos[r * nx + c]
            if not math.isfinite(a):
                continue
            bins[min(int(a // 10), 20)] += w
            pares.append((a, w))
            a_tot += w
    pares.sort()
    acc, qs = 0.0, {}
    objetivo = [(0.5, 'mediana'), (0.9, 'p90'), (0.99, 'p99'), (0.999, 'p999')]
    i = 0
    for a, w in pares:
        acc += w
        while i < len(objetivo) and acc >= objetivo[i][0] * a_tot:
            qs[objetivo[i][1]] = a
            i += 1
    w_planeta = sum(math.cos(math.radians(lats[r])) * (0.5 if r in (0, ny - 1) else 1.0)
                    for r in range(ny)) * (nx - 1)
    return {
        'fraccion_del_planeta': a_tot / w_planeta,
        'edad_media_myr': sum(a * w for a, w in pares) / a_tot,
        'edad_mediana_myr': qs['mediana'], 'p90_myr': qs['p90'], 'p99_myr': qs['p99'],
        'p999_myr': qs['p999'], 'edad_maxima_myr': pares[-1][0],
        'area_frac_por_10_myr': [x / a_tot for x in bins],
    }


# --- Placas de Bird (2003) ------------------------------------------------------------------------

def area_poligono(pts):
    """Área (sr) de un polígono esférico por la suma de triángulos desde el primer vértice."""
    def v(lo, la):
        lo, la = math.radians(lo), math.radians(la)
        return (math.cos(la) * math.cos(lo), math.cos(la) * math.sin(lo), math.sin(la))
    vs = [v(lo, la) for lo, la in pts]
    if vs[0] == vs[-1]:
        vs = vs[:-1]
    a = vs[0]
    tot = 0.0
    for k in range(1, len(vs) - 1):
        b_, c_ = vs[k], vs[k + 1]
        num = a[0] * (b_[1] * c_[2] - b_[2] * c_[1]) - a[1] * (b_[0] * c_[2] - b_[2] * c_[0]) + \
            a[2] * (b_[0] * c_[1] - b_[1] * c_[0])
        den = 1 + sum(a[i] * b_[i] for i in range(3)) + sum(b_[i] * c_[i] for i in range(3)) + \
            sum(c_[i] * a[i] for i in range(3))
        tot += 2 * math.atan2(num, den)
    tot = abs(tot)
    return min(tot, 4 * math.pi - tot)


def placas_bird(ruta):
    areas, nombre, pts = {}, None, []
    for linea in open(ruta):
        linea = linea.strip()
        if not linea:
            continue
        if linea.startswith('***'):
            areas[nombre] = area_poligono(pts)
            nombre, pts = None, []
        elif nombre is None:
            nombre = linea
        else:
            lo, la = (float(x) for x in linea.split(','))
            pts.append((lo, la))
    fr = sorted((a / (4 * math.pi) for a in areas.values()), reverse=True)
    return {'n_placas': len(fr), 'fracciones_ordenadas': fr,
            'n_mayores_1pct': sum(1 for x in fr if x >= 0.01),
            'suma_7_mayores': sum(fr[:7]), 'suma_frac': sum(fr)}


# --- Salida TOML ----------------------------------------------------------------------------------

def toml_valor(x):
    if isinstance(x, bool):
        return 'true' if x else 'false'
    if isinstance(x, (int,)):
        return str(x)
    if isinstance(x, float):
        return 'nan' if math.isnan(x) else f'{x:.6g}'
    if isinstance(x, str):
        return '"' + x.replace('"', "'") + '"'
    if isinstance(x, (list, tuple)):
        return '[' + ', '.join(toml_valor(v) for v in x) + ']'
    raise TypeError(type(x))


def escribir_toml(ruta, secciones, cabecera):
    with open(ruta, 'w') as f:
        f.write(cabecera)
        for nombre, datos in secciones:
            f.write(f'\n[{nombre}]\n')
            for k, v in datos.items():
                f.write(f'{k} = {toml_valor(v)}\n')


F_SUR, F_NORTE = 30 * 60, 149 * 60   # latitud −60° y +59° en filas de 1' (franjas de la roca)


def descargar(datos):
    """Descarga a `datos` los conjuntos que falten (solo la primera vez)."""
    os.makedirs(datos, exist_ok=True)
    d = lambda n: os.path.join(datos, n)  # noqa: E731
    etopo(ETOPO_SUP, d('etopo2022_superficie_2m.f32be'), 0, FILAS)
    # La roca solo difiere bajo los casquetes: franjas polares.
    etopo(ETOPO_ROCA, d('etopo2022_roca_sur_2m.f32be'), 0, F_SUR)
    etopo(ETOPO_ROCA, d('etopo2022_roca_norte_2m.f32be'), F_NORTE, FILAS)
    paxman(PAXMAN_ANT, d('paxman2022v3_antartida_5km.bin'), 10)
    paxman(PAXMAN_GRE, d('paxman2022v3_groenlandia_4p5km.bin'), 30)
    if not os.path.exists(d('seton2020_edades_6m.grd')):
        bajar(EDADES, d('seton2020_edades_6m.grd'))
    if not os.path.exists(d('bird2003_PB2002_plates.dig.txt')):
        bajar(BIRD, d('bird2003_PB2002_plates.dig.txt'))


def relieve_nivel3(datos):
    """Las tres versiones del relieve en celdas de nivel 3 ({'superficie', 'roca', 'roca_rebote'}) y
    el nº de celdas que Paxman et al. sustituyen en la Antártida y en Groenlandia."""
    d = lambda n: os.path.join(datos, n)  # noqa: E731
    log('ETOPO a celdas de nivel 3…')
    nc = COLS // PASO_ETOPO

    def leer(ruta):
        a = array.array('f')
        with open(ruta, 'rb') as f:
            a.frombytes(f.read())
        if sys.byteorder == 'little':
            a.byteswap()
        return a
    sup = leer(d('etopo2022_superficie_2m.f32be'))
    roca_s, roca_n = leer(d('etopo2022_roca_sur_2m.f32be')), leer(d('etopo2022_roca_norte_2m.f32be'))
    filas_sup = len(sup) // nc
    filas_roca = [None] * filas_sup
    for r in range(len(roca_s) // nc):
        filas_roca[r] = roca_s[r * nc:(r + 1) * nc]
    r0n = F_NORTE // PASO_ETOPO
    for r in range(len(roca_n) // nc):
        filas_roca[r0n + r] = roca_n[r * nc:(r + 1) * nc]
    cel3 = acumular_etopo(sup, (0, filas_roca), N3)
    del sup, roca_s, roca_n, filas_roca
    log('Paxman et al. sobre la Antártida y Groenlandia…')
    n_ant = sustituir_paxman(cel3['roca_rebote'], d('paxman2022v3_antartida_5km.bin'), -71.0, 0.0, True, N3)
    n_gre = sustituir_paxman(cel3['roca_rebote'], d('paxman2022v3_groenlandia_4p5km.bin'), 70.0, -45.0, False, N3)
    return cel3, n_ant, n_gre


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--datos', default=os.path.join(RAIZ, 'output', 'referencias'))
    ap.add_argument('--salida', default=os.path.join(RAIZ, 'config', 'referencias', 'tierra_relieve.toml'))
    args = ap.parse_args()
    d = lambda n: os.path.join(args.datos, n)  # noqa: E731

    log('Descargas (solo la primera vez)…')
    descargar(args.datos)
    cel3, n_ant, n_gre = relieve_nivel3(args.datos)

    secciones = []
    for version, c3 in cel3.items():
        niveles = [None] * NIVELES
        niveles[NIVELES - 1] = c3
        for l in range(NIVELES - 2, -1, -1):
            niveles[l] = subir_nivel(niveles[l + 1])
        for l in range(NIVELES):
            lado = (math.pi / 2) * R_KM / (N0 << l)
            est = estadisticas(niveles[l])
            est = {'lado_km': lado, **est}
            secciones.append((f'relieve.{version}.nivel{l}', est))
            log(f'  {version} nivel {l}: tierra {100 * est["tierra_frac"]:.1f} %, media '
                f'{est["altura_media_tierra_m"]:.0f} m, mediana {est["altura_mediana_tierra_m"]:.0f} m, '
                f'océano {est["profundidad_media_oceano_m"]:.0f} m')
        if version == 'roca_rebote':
            log('Relieve por escala, pendientes y plataforma (roca_rebote)…')
            for l in range(NIVELES - 1):
                rel = relieve_por_escala(niveles[l], niveles[l + 1])
                secciones.append((f'espectro.nivel{l}_a_{l + 1}', {
                    'lado_madre_km': (math.pi / 2) * R_KM / (N0 << l),
                    **{f'{k}_sigma_m': v[0] for k, v in rel.items()},
                    **{f'{k}_area_frac': v[1] / sum(niveles[l].w) for k, v in rel.items()}}))
            rel = relieve_por_escala(c3, None)
            secciones.append(('espectro.nivel3_a_pixel', {
                'lado_madre_km': (math.pi / 2) * R_KM / N3, 'pixel_km': 1.852 * PASO_ETOPO,
                **{f'{k}_sigma_m': v[0] for k, v in rel.items()}}))
            pen = pendientes(c3)
            for k, v in pen.items():
                if isinstance(v, dict):
                    secciones.append((f'pendientes_25km.{k}', {'media': v['media'], 'p50_p75_p90_p95_p99': v['percentiles']}))
                else:
                    secciones.append(('pendientes_25km.barlovento', {'m_oro_medio_k_oro_40': v}))
            secciones.append(('plataforma.nivel3', plataforma(c3)))
            log('Cuencas cerradas (niveles 2 y 3)…')
            for l in (2, 3):
                secciones.append((f'depresiones.nivel{l}', depresiones(niveles[l])))

    log('Edades del fondo oceánico…')
    secciones.append(('fondo_oceanico.edades', edades(d('seton2020_edades_6m.grd'))))
    log('Placas de Bird (2003)…')
    secciones.append(('placas.tamano_bird2003', placas_bird(d('bird2003_PB2002_plates.dig.txt'))))
    secciones.append(('placas.velocidad_argus2011', {
        'rms_litosfera_nnr_cm_anio': 4.39,
        'nota': 'NNR-MORVEL56: RMS de toda la litosfera respecto al marco sin rotación neta, 43,9 mm/año'}))

    hoy = datetime.date.today().isoformat()
    cab = f'''# Tablas de referencia de la Tierra para el relieve del planeta (diseño del relieve tectónico §10).
# Generado por herramientas/analisis/referencia_tierra.py el {hoy}. No editar a mano: se regenera.
#
# Conjuntos de datos:
# - ETOPO 2022 v1 (NOAA NCEI, doi:10.25921/fd45-gt74), 60", superficie y roca, muestreado cada 2'.
#   La roca se lee solo en latitudes < −60° y > 59° (bajo los casquetes); fuera es igual a la superficie.
# - Paxman, Austermann y Hollyday (2022), Sci. Rep. 12, 11399; rejillas v3 (2026), Arctic Data Center
#   doi:10.18739/A22Z12R8C: roca rebotada sin hielo respecto al nivel del mar actual (BedMachine
#   Antarctica v4 y Greenland v6), muestreada cada 5 km (Antártida, {n_ant} celdas de nivel 3) y cada
#   4,5 km (Groenlandia, {n_gre} celdas).
# - Seton et al. (2020), G-cubed, doi:10.1029/2020GC009214: age.2020.1.GTS2012.6m.grd.
# - Bird (2003), G-cubed 4(3), doi:10.1029/2001GC000252: PB2002_plates.dig.txt.
# - Argus, Gordon y DeMets (2011), G-cubed, doi:10.1029/2011GC003751 (NNR-MORVEL56).
#
# Versiones del relieve: superficie (encima del hielo), roca (bajo el hielo) y roca_rebote (la
# referencia: roca desnuda en equilibrio isostático, con el agua del océano actual).
# Alturas en metros sobre el nivel del mar actual; las celdas son las del cubo esférico del planeta
# (n0 = {N0}, niveles 0–3) con la media de los píxeles de cada celda. Tierra = celdas con media ≥ 0.
#
# hipsometria_frac: fracción de la tierra en las franjas {LIM_TIERRA} m (la última sin techo).
# hipsometria_oceano_frac: fracción del océano por profundidad, franjas {LIM_MAR} m.
# histograma_planeta_frac: todo el planeta en franjas de 500 m, de −11 000 a +9 000 m.
# espectro.*: desviación típica de la altura dentro de cada celda madre (de sus 4 hijas, o de sus
#   píxeles en nivel3_a_pixel), por clase de la madre: tierra_L = altura media ≥ L m; mar_L =
#   profundidad ≥ L m. Es la amplitud del relieve real por escala y tipo, que el ruido fino continúa.
# pendientes_25km: |∇h| a ±25 km (como el planeta) en la tierra por encima de 0/500/1000/2000 m, y el
#   factor orográfico medio de barlovento de las cordilleras grandes con k_oro = 40 y el viento del
#   modelo (misma definición que herramientas/analisis/terreno.py).
'''
    os.makedirs(os.path.dirname(args.salida), exist_ok=True)
    escribir_toml(args.salida, secciones, cab)
    log(f'Escrito {args.salida}')


if __name__ == '__main__':
    main()
