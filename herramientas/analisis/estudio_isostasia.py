"""Estudio de isostasia continental (docs/design/terreno/estudio_isostasia_continental.md).

Protocolo del estudio (§4), con datos reales y sin el planeta simulado:
1. Reproducir la isostasia de litosfera completa de Lamb et al. (2020) con sus datos (Zenodo 4031646):
   sus constantes salen de sus propias rejillas (la perturbación de grosor que anula cada anomalía entre
   la anomalía: α para la corteza, β para la litosfera), la referencia (32 km de corteza a nivel del
   mar con 100 km de litosfera) de su texto, la corteza de CRUST1.0 y la litosfera de CAM2016
   (Priestley y McKenzie), la que usaron. Se compara nuestra anomalía (observada − calculada) con la
   suya celda a celda, y su reducción a 100 km de litosfera con su tabla por continentes.
2. Evaluar las reglas R0–R4 con la Tierra real: la tabla de relieve (altura media de la tierra, por
   debajo de 200 m y por encima de 2 000 m, pendiente y plataforma, a 26 km como la tabla de cierre y a
   la misma resolución que los datos), el error por celda, por provincia y en casos con nombre.
3. Ajustar solo lo que no tiene valor medido (H_0 y, en sus rangos publicados, Δρ_ma, las densidades
   y las constantes de L(t)), con validación cruzada: se ajusta con Eurasia, África y Australia y se
   comprueba con América y la Antártida sin hielo, y al revés.
5. Repartir el error de R0 entre el manto litosférico, la densidad de la corteza y el sedimento
   (reparto de Shapley sobre las 8 combinaciones).

Las reglas (§3), para una columna con corteza h (ρ_c), sedimento s (ρ_s) y manto litosférico
h_m = L − h − s (densidad ρ_a + Δρ_ma):
  H = [(ρ_a − ρ_c)·h + (ρ_a − ρ_s)·s − Δρ_ma·h_m]/ρ_a − H_0 sobre el mar; bajo el mar, la profundidad
  por ρ_a/(ρ_a − ρ_w).
- R0: la regla del modelo (ρ_a = ρ_m = 3 300, ρ_c = 2 800, ρ_s = 2 400, Δρ_ma = 0). Tal cual
  (H_0 de `h_nivel` = 32 km) y con H_0 ajustado.
- R1: R0 con la densidad creciendo con la profundidad: ρ_u hasta la profundidad z_t y ρ_l por debajo
  (Christensen y Mooney 1995: superior 2 700–2 750, inferior 2 900–3 000, media ~2 830).
- R2: litosfera completa con L observado: CAM2016 (Lamb), TC1 (Artemieva 2006, 5°) y LITHO1.0.
- R3: como R2 con L de la edad tectonotérmica, la que el motor puede llevar: (a) la recta de Artemieva
  L = 0,04·t + 93,6 km; (b) con recuperación tras el evento, L = L_min + (L_lin − L_min)·(1 − e^(−t/τ_L)).
  La edad de cada celda sale de su tipo de corteza de CRUST1.0 (alineados con el mapa de edades de
  Artemieva y Mooney; tabla EDAD_TIPO).
- R4: R3 con la densidad de R1.

Uso: python3 -I herramientas/analisis/estudio_isostasia.py
Datos (se descargan una vez a output/referencias/): los de tierra_corteza_regla.py, CAM2016Litho de
EarthScope, TC1 a 5° de www.lithosphere.info y los conjuntos de Lamb et al. (2020) de Zenodo.
Escribe config/referencias/tierra_isostasia.toml y muestra las tablas.
"""
import argparse
import array
import io
import math
import os
import struct
import sys
import tarfile
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import referencia_tierra as rt  # noqa: E402
import tierra_regla_modelo as trm  # noqa: E402
import lluvia_observada as lo  # noqa: E402
import tierra_corteza_regla as tcr  # noqa: E402

RAIZ = rt.RAIZ
N = rt.N3
A_TIERRA_KM2 = 4.0 * math.pi * rt.R_KM ** 2
CAM2016 = 'https://data.earthscope.org/archive/seismology/products/emc/data/CAM2016/CAM2016Litho.tgz?type=file'
TC1 = 'http://www.lithosphere.info/Database/TC1-5deg.dat'
ZENODO = 'https://zenodo.org/api/records/4031646/files/{}/content'
LAMB = ('ds02.xyz', 'ds03.grd', 'ds06.grd', 'ds07.grd')
RHO_W = 1030.0

# Edad tectonotérmica (Myr) de cada tipo de corteza de CRUST1.0 (CNtype1_key.txt), del punto medio de
# la edad que describe el tipo (supuesto de este estudio; los tipos siguen el mapa de edades de
# Artemieva y Mooney): Arcaico temprano 3,6–3,0 Ga, tardío 3,0–2,5; Proterozoico temprano y medio
# 2,5–1,0, tardío 1,0–0,54; plataformas con su cubierta fanerozoica (Paleozoico); Fanerozoico
# 0,54–0; orógenos mesozoicos y cenozoicos y activos (Andes, Himalaya), arcos, corteza estirada y
# rifts, recientes; márgenes y plataformas continentales, el rift mesozoico que los abrió.
EDAD_TIPO = {
    'G1': 3300.0, 'G2': 2750.0, 'F-': 3000.0,
    'H1': 1750.0, 'H2': 1750.0, 'I1': 800.0, 'I2': 800.0,
    'D-': 400.0, 'E-': 400.0, 'Z1': 300.0, 'Z2': 300.0,
    'O-': 100.0, 'P-': 70.0, 'Q-': 70.0, 'R1': 20.0, 'R2': 30.0,
    'J-': 15.0, 'K-': 15.0, 'L1': 30.0, 'L2': 20.0,
    'M-': 30.0, 'N-': 30.0, 'X-': 20.0, 'V2': 15.0, 'B-': 10.0,
    'C-': 120.0, 'S-': 120.0, 'T-': 150.0, 'U-': 150.0,
    'W-': 100.0, 'Y1': 100.0, 'Y2': 100.0,
}
# Provincias del §4 (tipos de CRUST1.0).
PROVINCIAS = [
    ('escudo arcaico', {'G1', 'G2', 'F-'}),
    ('plataforma proterozoica', {'H1', 'H2', 'I1', 'I2', 'D-', 'E-'}),
    ('cinturón fanerozoico', {'Z1', 'Z2', 'O-', 'P-', 'Q-'}),
    ('rift o zona joven', {'M-', 'N-', 'X-', 'V2', 'B-'}),
    ('orógeno activo y arco', {'R1', 'R2', 'J-', 'K-', 'L1', 'L2'}),
    ('margen pasivo', {'C-', 'S-', 'T-', 'U-', 'W-', 'Y1', 'Y2'}),
]
# Casos con nombre (centro de la celda de 1°) y su criterio (§5), si lo tiene.
CASOS = [
    ('llanura rusa', 55.5, 40.5, (None, 400.0)),
    ('Kenia', 0.5, 36.5, (1200.0, None)),
    ('Tíbet', 33.5, 88.5, (4000.0, 5500.0)),
    ('meseta del Colorado', 36.5, -110.5, (None, None)),
    ('Grandes Llanuras', 40.5, -97.5, (None, None)),
]


def log(*a):
    print(*a, file=sys.stderr, flush=True)


# --- Datos -----------------------------------------------------------------------------------------

def bajar(url, ruta):
    if os.path.exists(ruta):
        return ruta
    os.makedirs(os.path.dirname(ruta), exist_ok=True)
    log(f'Descargando {url}…')
    with urllib.request.urlopen(url, timeout=300) as r, open(ruta + '.tmp', 'wb') as f:
        f.write(r.read())
    os.replace(ruta + '.tmp', ruta)
    return ruta


def leer_cam2016(datos):
    """CAM2016Litho (Priestley y McKenzie): grosor de la litosfera térmica (km) en una malla de 2°
    (latitudes y longitudes impares). Se lee el texto del tar sin extraerlo."""
    ruta = bajar(CAM2016, os.path.join(datos, 'cam2016', 'CAM2016Litho.tgz'))
    with tarfile.open(ruta, 'r:gz') as tf:
        m = next(x for x in tf.getmembers() if x.isfile() and os.path.basename(x.name) == 'CAM2016Litho')
        texto = tf.extractfile(m).read().decode()
    rej = {}
    for ln in texto.splitlines():
        p = ln.split()
        if len(p) == 3:
            rej[(round(float(p[0])), round(float(p[1])))] = float(p[2])
    return rej


def l_cam(rej, lat, lon):
    """Bilineal en la malla de 2° de CAM2016 (nodos en grados impares)."""
    fy = (lat + 89.0) / 2.0
    fx = (lon + 179.0) / 2.0
    j0, i0 = math.floor(fy), math.floor(fx)
    a, b = fy - j0, fx - i0
    s = sw = 0.0
    for dj, di, w in ((0, 0, (1 - a) * (1 - b)), (0, 1, (1 - a) * b), (1, 0, a * (1 - b)), (1, 1, a * b)):
        jj = min(max(j0 + dj, 0), 89)
        ii = (i0 + di) % 180
        v = rej.get((-89 + 2 * jj, -179 + 2 * ii))
        if v is not None and w > 0:
            s += v * w
            sw += w
    return s / sw if sw > 0 else float('nan')


def leer_tc1(datos):
    """TC1 a 5° (Artemieva 2006): profundidad de la isoterma de 1 300 °C (km), el grosor térmico de la
    litosfera, en los nodos continentales."""
    ruta = bajar(TC1, os.path.join(datos, 'tc1', 'TC1-5deg.dat'))
    rej = {}
    with open(ruta) as f:
        cab = f.readline()
        assert 'depth-to-1300' in cab, cab
        for ln in f:
            p = ln.strip().split(',')
            if len(p) >= 10:
                rej[(int(float(p[1])), int(float(p[0])))] = float(p[9])
    return rej


def rellenar_tc1(rej, pasadas=3):
    """Rellena los nodos de 5° sin dato junto a la costa con la media de sus vecinos con dato (unas
    pocas pasadas): TC1 solo tiene nodos continentales y las celdas de la costa caerían en el hueco."""
    rej = dict(rej)
    for _ in range(pasadas):
        nuevos = {}
        for j in range(37):
            la = -90 + 5 * j
            for i in range(72):
                lo_ = -180 + 5 * i
                if (la, lo_) in rej:
                    continue
                v = [rej[(la + 5 * dj, -180 + 5 * ((i + di) % 72))]
                     for dj in (-1, 0, 1) for di in (-1, 0, 1)
                     if (dj or di) and (la + 5 * dj, -180 + 5 * ((i + di) % 72)) in rej]
                if v:
                    nuevos[(la, lo_)] = sum(v) / len(v)
        rej.update(nuevos)
    return rej


def l_tc1(rej, lat, lon):
    """Bilineal en la malla de 5° de TC1 con los nodos que existan (solo hay continentales)."""
    fy = (lat + 90.0) / 5.0
    fx = (lon + 180.0) / 5.0
    j0, i0 = math.floor(fy), math.floor(fx)
    a, b = fy - j0, fx - i0
    s = sw = 0.0
    for dj, di, w in ((0, 0, (1 - a) * (1 - b)), (0, 1, (1 - a) * b), (1, 0, a * (1 - b)), (1, 1, a * b)):
        la = -90 + 5 * min(max(j0 + dj, 0), 36)
        lo_ = -180 + 5 * ((i0 + di) % 72)
        v = rej.get((la, lo_))
        if v is not None and w > 0:
            s += v * w
            sw += w
    return s / sw if sw > 0.3 else float('nan')


def leer_lamb(datos):
    """Conjuntos de Lamb et al. (2020): la tabla por continentes (ds02) y las rejillas de 1° de la
    anomalía (ds03) y de las perturbaciones de grosor de la corteza (ds06) y de la litosfera (ds07)
    que la anulan."""
    dest = os.path.join(datos, 'lamb2020')
    for n in LAMB:
        bajar(ZENODO.format(n), os.path.join(dest, n))
    tabla = []
    with open(os.path.join(dest, 'ds02.xyz')) as f:
        for ln in f:
            if ln.startswith('#') or not ln.strip():
                continue
            p = ln.rstrip('\n').split('\t')
            tabla.append({'nombre': p[0], 'area': float(p[1]), 'altura_m': float(p[2]),
                          'reducida_m': float(p[3]), 'l_km': float(p[4]), 'c_km': float(p[5])})
    rejillas = {}
    for n in ('ds03.grd', 'ds06.grd', 'ds07.grd'):
        b = open(os.path.join(dest, n), 'rb').read()
        var = rt.leer_netcdf3(b)

        def eje(nm):
            (k,), t, ini = var[nm]
            fm = 'd' if t == 6 else 'f'
            return struct.unpack(f'>{k}{fm}', b[ini:ini + (8 if fm == 'd' else 4) * k])
        xs, ys = eje('x'), eje('y')
        (ny, nx), _, ini = var['z']
        z = array.array('f', b[ini:ini + 4 * nx * ny])
        if sys.byteorder == 'little':
            z.byteswap()
        rejillas[n] = (xs, ys, z)
    return tabla, rejillas


# --- Mitades de los continentes y provincias ---------------------------------------------------------

def mitad(lat, lon):
    """'A' (Eurasia, África, Australia y el resto) o 'B' (América y la Antártida)."""
    if lat < -60.0:
        return 'B'
    if lo.antartida_o_groenlandia(lat, lon):
        return 'B'
    if -170.0 <= lon < -30.0:
        return 'B'
    return 'A'


def provincia(tipo):
    for i, (_, cods) in enumerate(PROVINCIAS):
        if tipo in cods:
            return i
    return None


# --- Reglas ----------------------------------------------------------------------------------------

def b_corteza(p, h):
    """Flotación de la corteza cristalina (km·kg/m³ respecto a la astenosfera de densidad ρ_a)."""
    ra = p['rho_a']
    if 'z_t' in p:
        return (ra - p['rho_u']) * min(h, p['z_t']) + (ra - p['rho_l']) * max(0.0, h - p['z_t'])
    return (ra - p['rho_c']) * h


def altura_regla(p, h, s, l_lit):
    """Altura (m) sobre el nivel de referencia de la regla `p` para una columna (h, s, L)."""
    ra = p['rho_a']
    b = b_corteza(p, h) + p.get('k_sed', 1.0) * (ra - p['rho_s']) * s
    drho = p.get('drho', 0.0)
    if p.get('arcaico') and 'drho_arcaico' in p:
        drho = p['drho_arcaico']
    if p.get('drho', 0.0) > 0.0:
        if not math.isfinite(l_lit):
            return float('nan')
        b -= drho * max(0.0, l_lit - h - s)
    z = 1000.0 * b / ra - p['h0']
    if z < 0.0:
        z *= ra / (ra - RHO_W)
    return z


def l_de_edad(p, t):
    """L (km) de la edad tectonotérmica t (Myr): recta de Artemieva (2006), con recuperación tras el
    evento si la regla tiene `l_min` y `tau_l`."""
    l_lin = 0.04 * t + 93.6
    if 'l_min' in p:
        return p['l_min'] + (l_lin - p['l_min']) * (1.0 - math.exp(-t / p['tau_l']))
    return l_lin


def l_de(p, c):
    """L de la celda `c` según la fuente de la regla."""
    fuente = p.get('l')
    if fuente is None:
        return float('nan')
    if fuente == 'edad':
        return l_de_edad(p, c['edad'])
    return c[fuente]


def con_edad(p, c):
    """La regla `p` en la celda `c`: marca el manto arcaico (más de 2 500 Myr) si la regla le da su
    propio contraste de densidad (variante opcional de R3: Δρ_ma según la edad)."""
    if 'drho_arcaico' in p:
        return dict(p, arcaico=c['edad'] > 2500.0)
    return p


def errores(p, celdas):
    """Error por celda (predicha − real, m) en las celdas: media, error medio absoluto y RMS,
    ponderados por área. Solo las celdas en que la regla da un número."""
    sw = s1 = sa = s2 = 0.0
    for c in celdas:
        z = altura_regla(con_edad(p, c), c['h'], c['s'], l_de(p, c))
        if not math.isfinite(z):
            continue
        e = z - c['z']
        sw += c['w']
        s1 += e * c['w']
        sa += abs(e) * c['w']
        s2 += e * e * c['w']
    if sw <= 0:
        return {'sesgo_m': float('nan'), 'error_medio_m': float('nan'), 'rms_m': float('nan'), 'area': 0.0}
    return {'sesgo_m': s1 / sw, 'error_medio_m': sa / sw, 'rms_m': math.sqrt(s2 / sw), 'area': sw}


def ajustar_h0(p, celdas):
    """H_0 por mínimos cuadrados en la tierra (sobre el mar la regla es lineal en H_0): el residuo
    medio, iterado dos veces por las celdas que la regla deja bajo el mar."""
    q = dict(p, h0=0.0)
    for _ in range(3):
        sw = s1 = 0.0
        for c in celdas:
            z = altura_regla(con_edad(q, c), c['h'], c['s'], l_de(q, c))
            if not math.isfinite(z):
                continue
            sw += c['w']
            s1 += (z - c['z']) * c['w']
        if sw <= 0:
            return q
        q['h0'] += s1 / sw
    return q


def rejilla(rangos):
    """Producto de los valores de cada parámetro libre (dict nombre → lista)."""
    claves = list(rangos)
    combos = [{}]
    for k in claves:
        combos = [dict(c, **{k: v}) for c in combos for v in rangos[k]]
    return combos


def ajustar(base, rangos, celdas):
    """El mejor juego de parámetros de la rejilla `rangos` (con H_0 ajustado en cada uno) por el RMS
    en `celdas`. Una regla con `h0` propio y sin rangos no se ajusta."""
    if 'h0' in base and not rangos:
        return dict(base)
    mejor = None
    for combo in rejilla(rangos):
        p = ajustar_h0(dict(base, **combo), celdas)
        e = errores(p, celdas)['rms_m']
        if math.isfinite(e) and (mejor is None or e < mejor[0]):
            mejor = (e, p)
    return mejor[1]


# --- Programa ----------------------------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser(description='Estudio de isostasia continental (Lamb et al. 2020 y R0–R4).')
    ap.add_argument('--datos', default=os.path.join(RAIZ, 'output', 'referencias'))
    ap.add_argument('--cache', default=os.path.join(RAIZ, 'output', 'cache'))
    ap.add_argument('--salida', default=os.path.join(RAIZ, 'config', 'referencias', 'tierra_isostasia.toml'))
    a = ap.parse_args()
    c_mod = trm.cfg_modelo()
    m = 6 * N * N

    # ETOPO, Seton y GlobSed en nivel 3; CRUST1.0, LITHO1.0, CAM2016 y TC1.
    cel = lo.relieve_roca_rebote(a.datos, a.cache)
    w3 = list(cel.w)
    z3 = [cel.zw[k] / cel.w[k] if cel.w[k] > 0 else float('nan') for k in range(m)]
    ruta_seton = os.path.join(a.datos, 'seton2020_edades_6m.grd')
    ruta_gs = os.path.join(a.datos, 'globsed', 'globsed-v3.asc')
    edad3, frac3 = tcr.en_cache(a.cache, 'seton_n3', tcr.huella(trm.acumular_seton, rt.localizar, rt.leer_netcdf3,
                                                               extra=f'{N}:{os.path.getsize(ruta_seton)}'),
                                lambda: trm.acumular_seton(ruta_seton), 2, m)
    (sed3,) = tcr.en_cache(a.cache, 'globsed_n3', tcr.huella(trm.acumular_globsed, rt.localizar,
                                                             extra=f'{N}:{os.path.getsize(ruta_gs)}'),
                           lambda: [trm.acumular_globsed(ruta_gs)], 1, m)
    oc3 = [frac3[k] >= 0.5 and math.isfinite(edad3[k]) for k in range(m)]
    cont3 = [not x for x in oc3]
    bnds, rho, tipos = tcr.leer_crust1(tcr.descargar_crust1(a.datos))
    col = tcr.columnas_crust1(bnds, rho, 3300.0)
    lab1, moho1 = tcr.litosfera_litho1(a.datos, a.cache)
    cam = leer_cam2016(a.datos)
    tc1 = rellenar_tc1(leer_tc1(a.datos))
    tabla_lamb, rej_lamb = leer_lamb(a.datos)
    log('Centros de nivel 3…')
    lat3, lon3 = tcr.lat_lon_centros(N)

    # Celdas de 1°: ETOPO y fracción oceánica medias de las de nivel 3 que caen dentro.
    sw1 = [0.0] * 64800
    sz1 = [0.0] * 64800
    so1 = [0.0] * 64800
    for k in range(m):
        c1 = tcr.celda_1grado(lat3[k], lon3[k])
        sw1[c1] += w3[k]
        sz1[c1] += z3[k] * w3[k]
        so1[c1] += w3[k] * (1.0 if oc3[k] else 0.0)
    celdas1 = []
    for c1 in range(64800):
        if sw1[c1] <= 0:
            continue
        la = 89.5 - c1 // 360
        lo_ = -179.5 + c1 % 360
        z = sz1[c1] / sw1[c1]
        t = tipos[c1]
        celdas1.append({
            'i': c1, 'lat': la, 'lon': lo_, 'w': math.cos(math.radians(la)), 'z': z,
            'oc': so1[c1] / sw1[c1] >= 0.5, 'tipo': t, 'h': col['h'][c1], 's': col['s'][c1],
            'b_capas': col['b_capas'][c1], 'cam': l_cam(cam, la, lo_), 'tc1': l_tc1(tc1, la, lo_),
            'litho': lab1[c1], 'edad': EDAD_TIPO.get(t, 300.0), 'mitad': mitad(la, lo_),
            'prov': provincia(t),
        })
    tierra1 = [c for c in celdas1 if not c['oc'] and c['z'] >= 0.0]
    log(f'{len(tierra1)} celdas de tierra continental de 1°')

    # --- 1. Reproducir a Lamb et al. (2020) ---
    xs, ys, anom = rej_lamb['ds03.grd']
    _, _, dc = rej_lamb['ds06.grd']
    _, _, dl = rej_lamb['ds07.grd']
    cocientes_a, cocientes_b = [], []
    for i in range(len(anom)):
        if math.isfinite(anom[i]) and abs(anom[i]) > 100.0 and math.isfinite(dc[i]) and math.isfinite(dl[i]):
            cocientes_a.append(round(dc[i] / anom[i] * 1000.0, 2))
            cocientes_b.append(round(-dl[i] / anom[i] * 1000.0, 1))

    def moda(v):
        cuenta = {}
        for x in v:
            cuenta[x] = cuenta.get(x, 0) + 1
        x, n = max(cuenta.items(), key=lambda t: (t[1], -abs(t[0])))
        return x, n / len(v)
    alfa, f_alfa = moda(cocientes_a)
    beta, f_beta = moda(cocientes_b)
    beta = abs(beta)
    # Lamb: H = (C − 32)/α − (L − 100)/β (km) con 32 km de corteza a nivel del mar y 100 km de litosfera.
    # Nuestra anomalía en sus nodos (grados enteros): ETOPO medio en la caja de 1° centrada en el nodo.
    swn, szn = {}, {}
    for k in range(m):
        nodo = (round(lat3[k]), round(lon3[k]))
        swn[nodo] = swn.get(nodo, 0.0) + w3[k]
        szn[nodo] = szn.get(nodo, 0.0) + z3[k] * w3[k]
    son = {}
    for k in range(m):
        if oc3[k]:
            nodo = (round(lat3[k]), round(lon3[k]))
            son[nodo] = son.get(nodo, 0.0) + w3[k]
    pares = []
    pares_tierra = []
    for j, y in enumerate(ys):
        for i, x in enumerate(xs):
            v = anom[j * len(xs) + i]
            if not math.isfinite(v):
                continue
            nodo = (round(y), round(x))
            if nodo not in swn:
                continue
            h_obs = szn[nodo] / swn[nodo]
            tierra_cont = h_obs >= 0.0 and son.get(nodo, 0.0) / swn[nodo] < 0.5
            l_ = l_cam(cam, y, x)
            if not math.isfinite(l_):
                continue
            # Su corteza es la total sin agua (CRUST1.0 «crsthk»): cristalina más sedimento.
            c_tot = 0.0
            for c1, wb in tcr.vecinas_bilineal(y, x):
                c_tot += (col['h'][c1] + col['s'][c1]) * wb
            h_calc = 1000.0 * ((c_tot - 32.0) / alfa - (l_ - 100.0) / beta)
            pares.append((h_obs - h_calc, v, math.cos(math.radians(y))))
            if tierra_cont:
                pares_tierra.append(pares[-1])

    def comparar(pp):
        sw = sum(p[2] for p in pp)
        mx = sum(p[0] * p[2] for p in pp) / sw
        my = sum(p[1] * p[2] for p in pp) / sw
        sxx = sum((p[0] - mx) ** 2 * p[2] for p in pp)
        syy = sum((p[1] - my) ** 2 * p[2] for p in pp)
        sxy = sum((p[0] - mx) * (p[1] - my) * p[2] for p in pp)
        return {'celdas': len(pp), 'correlacion': sxy / math.sqrt(sxx * syy),
                'anomalia_nuestra_media_m': mx, 'anomalia_lamb_media_m': my,
                'anomalia_nuestra_desv_m': math.sqrt(sxx / sw), 'anomalia_lamb_desv_m': math.sqrt(syy / sw),
                'diferencia_rms_m': math.sqrt(sum((p[0] - p[1]) ** 2 * p[2] for p in pp) / sw)}
    repro = {'alfa': alfa, 'alfa_frac_celdas': f_alfa, 'beta': beta, 'beta_frac_celdas': f_beta,
             **{f'tierra_{k}': v for k, v in comparar(pares_tierra).items()},
             **{f'todo_{k}': v for k, v in comparar(pares).items()}}
    # Su tabla por continentes: la altura reducida a 100 km de litosfera, y la que da su regla.
    repro_tabla = []
    for t in tabla_lamb:
        reducida = t['altura_m'] + 1000.0 * (t['l_km'] - 100.0) / beta
        calculada = 1000.0 * ((t['c_km'] - 32.0) / alfa - (t['l_km'] - 100.0) / beta)
        repro_tabla.append({**t, 'reducida_nuestra_m': reducida, 'calculada_m': calculada})

    # --- 2 y 3. Reglas y validación cruzada ---
    r0_tal_cual = {'rho_a': 3300.0, 'rho_c': 2800.0, 'rho_s': 2400.0,
                   'h0': 1000.0 * (3300.0 - 2800.0) * c_mod['h_nivel_km'] / 3300.0}
    rho_a_lit = 3255.0   # Lamb: (ρ_m − ρ_a)/ρ_a = 1/β con Δρ_ma = 35
    # R2 con las constantes de Lamb (α, β y la referencia de 32 km a nivel del mar con 100 km de
    # litosfera; el sedimento cuenta como corteza): ρ_m − ρ_c = ρ_a/α, ρ_m − ρ_a = ρ_a/β.
    rho_c_lamb = rho_a_lit + rho_a_lit / beta - rho_a_lit / alfa
    r2_lamb = {'rho_a': rho_a_lit, 'rho_c': rho_c_lamb, 'rho_s': rho_c_lamb, 'drho': rho_a_lit / beta,
               'l': 'cam', 'h0': 1000.0 * (32.0 / alfa - 100.0 / beta)}
    reglas = {
        'R0': ({'rho_a': 3300.0, 'rho_c': 2800.0, 'rho_s': 2400.0}, {}),
        'R2_lamb': (dict(r2_lamb), {}),
        'R1': ({'rho_a': 3300.0, 'rho_s': 2400.0},
               {'rho_u': [2700.0, 2725.0, 2750.0], 'rho_l': [2900.0, 2950.0, 3000.0], 'z_t': [15.0, 20.0, 25.0]}),
        'R2_cam2016': ({'rho_a': rho_a_lit, 'rho_s': 2400.0, 'l': 'cam'},
                       {'rho_c': [2800.0, 2830.0, 2860.0, 2890.0], 'drho': [20.0, 25.0, 30.0, 35.0, 40.0]}),
        'R2_tc1': ({'rho_a': rho_a_lit, 'rho_s': 2400.0, 'l': 'tc1'},
                   {'rho_c': [2800.0, 2830.0, 2860.0, 2890.0], 'drho': [20.0, 25.0, 30.0, 35.0, 40.0]}),
        'R2_litho1': ({'rho_a': rho_a_lit, 'rho_s': 2400.0, 'l': 'litho'},
                      {'rho_c': [2800.0, 2830.0, 2860.0, 2890.0], 'drho': [20.0, 25.0, 30.0, 35.0, 40.0]}),
        'R3a': ({'rho_a': rho_a_lit, 'rho_s': 2400.0, 'l': 'edad'},
                {'rho_c': [2800.0, 2830.0, 2860.0, 2890.0], 'drho': [20.0, 25.0, 30.0, 35.0, 40.0]}),
        'R3b': ({'rho_a': rho_a_lit, 'rho_s': 2400.0, 'l': 'edad'},
                {'rho_c': [2800.0, 2830.0, 2860.0, 2890.0], 'drho': [20.0, 30.0, 40.0],
                 'l_min': [40.0, 60.0, 80.0], 'tau_l': [20.0, 50.0, 100.0, 200.0]}),
        # El sedimento rellena el hueco que deja la litosfera al hundirse y no levanta la tierra
        # (McKenzie 1978; §1 del estudio).
        'R3b_relleno': ({'rho_a': rho_a_lit, 'rho_s': 2400.0, 'l': 'edad', 'k_sed': 0.0},
                        {'rho_c': [2800.0, 2830.0, 2860.0, 2890.0], 'drho': [20.0, 30.0, 40.0],
                         'l_min': [40.0, 60.0, 80.0], 'tau_l': [20.0, 50.0, 100.0, 200.0]}),
        # Variante opcional de R3 (§3): Δρ_ma según la edad. El manto arcaico, el más empobrecido,
        # con su propio contraste (Poudjom Djomani et al. 2001: el manto litosférico es más ligero
        # cuanto más viejo).
        'R3c': ({'rho_a': rho_a_lit, 'rho_s': 2400.0, 'l': 'edad', 'k_sed': 0.0},
                {'rho_c': [2830.0, 2860.0, 2890.0], 'drho': [20.0, 30.0, 40.0],
                 'drho_arcaico': [0.0, 5.0, 10.0, 15.0, 20.0],
                 'l_min': [40.0, 60.0], 'tau_l': [50.0, 100.0]}),
    }
    # Validación cruzada: ajustar en una mitad y comprobar en la otra.
    mitades = {'A': [c for c in tierra1 if c['mitad'] == 'A'], 'B': [c for c in tierra1 if c['mitad'] == 'B']}
    cv = {}
    finales = {}
    for nombre, (base, rangos) in reglas.items():
        log(f'Ajustando {nombre}…')
        fila = {}
        for ajuste, prueba in (('A', 'B'), ('B', 'A')):
            p = ajustar(base, rangos, mitades[ajuste])
            fila[f'ajuste_{ajuste}'] = errores(p, mitades[ajuste])
            fila[f'prueba_{prueba}'] = errores(p, mitades[prueba])
            fila[f'parametros_{ajuste}'] = p
        p = ajustar(base, rangos, tierra1)
        finales[nombre] = p
        fila['todo'] = errores(p, tierra1)
        cv[nombre] = fila
    # R4: R3 (la mejor de a y b en la prueba) con la densidad de R1, solo si R1 aporta.
    def prueba_media(f):
        return 0.5 * (f['prueba_B']['error_medio_m'] + f['prueba_A']['error_medio_m'])
    r3 = min(('R3a', 'R3b', 'R3b_relleno', 'R3c'), key=lambda n: prueba_media(cv[n]))
    r1_aporta = prueba_media(cv['R1']) < 0.97 * prueba_media(cv['R0'])
    if r1_aporta:
        base = {k: v for k, v in finales[r3].items()
                if k not in ('rho_c', 'h0', 'drho', 'drho_arcaico', 'l_min', 'tau_l')}
        rangos_r3 = {k: v for k, v in reglas[r3][1].items() if k != 'rho_c'}
        rangos = {'rho_u': [2700.0, 2750.0], 'rho_l': [2900.0, 3000.0], 'z_t': [15.0, 20.0, 25.0], **rangos_r3}
        log(f'Ajustando R4 (sobre {r3})…')
        fila = {}
        for ajuste, prueba in (('A', 'B'), ('B', 'A')):
            p = ajustar(base, rangos, mitades[ajuste])
            fila[f'ajuste_{ajuste}'] = errores(p, mitades[ajuste])
            fila[f'prueba_{prueba}'] = errores(p, mitades[prueba])
            fila[f'parametros_{ajuste}'] = p
        p = ajustar(base, rangos, tierra1)
        finales['R4'] = p
        fila['todo'] = errores(p, tierra1)
        cv['R4'] = fila

    # Errores por provincia y casos con nombre (parámetros ajustados con todos los continentes).
    por_prov = {}
    for nombre, p in finales.items():
        filas = []
        for i, (pn, _) in enumerate(PROVINCIAS):
            cs = [c for c in tierra1 if c['prov'] == i]
            e = errores(p, cs)
            zr = sum(c['z'] * c['w'] for c in cs) / sum(c['w'] for c in cs)
            filas.append({'provincia': pn, 'area_frac': sum(c['w'] for c in cs) / sum(c['w'] for c in tierra1),
                          'altura_real_m': zr, **e})
        por_prov[nombre] = filas
    por_c1 = {c['i']: c for c in celdas1}
    casos = {}
    for nombre_caso, la, lo_, _ in CASOS:
        c = por_c1.get(tcr.celda_1grado(la, lo_))
        casos[nombre_caso] = {'real_m': c['z'], 'tipo': c['tipo'], 'h_km': c['h'], 's_km': c['s'],
                              'l_cam_km': c['cam'], 'l_tc1_km': c['tc1'], 'edad_myr': c['edad']}
        casos[nombre_caso]['R0_tal_cual'] = altura_regla(r0_tal_cual, c['h'], c['s'], float('nan'))
        for nombre, p in finales.items():
            casos[nombre_caso][nombre] = altura_regla(con_edad(p, c), c['h'], c['s'], l_de(p, c))

    # --- 5. Reparto del error de R0 (Shapley sobre litosfera, densidad de la corteza y sedimento) ---
    def regla_combinada(lit, dens, sed):
        base = {'rho_a': rho_a_lit if lit else 3300.0, 'rho_s': 2400.0, 'k_sed': 1.0 if sed else 0.0}
        rangos = {}
        if dens:
            rangos.update({'rho_u': [2700.0, 2725.0, 2750.0], 'rho_l': [2900.0, 2950.0, 3000.0], 'z_t': [15.0, 20.0, 25.0]})
        else:
            base['rho_c'] = 2800.0
        if lit:
            base['l'] = 'cam'
            rangos['drho'] = [20.0, 25.0, 30.0, 35.0, 40.0]
        p = ajustar(base, rangos, tierra1)
        return errores(p, tierra1)
    combos = {}
    for lit in (False, True):
        for dens in (False, True):
            for sed in (True, False):
                log(f'Reparto: litosfera {lit}, densidad {dens}, sedimento que levanta {sed}…')
                combos[(lit, dens, sed)] = regla_combinada(lit, dens, sed)

    def valor(conj, clave):
        # conj: arreglos aplicados ('lit', 'dens', 'sed'); 'sed' aplicado = el sedimento no levanta.
        return combos[('lit' in conj, 'dens' in conj, 'sed' not in conj)][clave]
    factores = ['lit', 'dens', 'sed']
    reparto = {}
    import itertools
    for clave in ('rms_m', 'error_medio_m'):
        total = valor(set(), clave) - valor(set(factores), clave)
        aporte = {}
        for f in factores:
            otros = [g for g in factores if g != f]
            s = 0.0
            for r in range(len(otros) + 1):
                for sub in itertools.combinations(otros, r):
                    peso = math.factorial(r) * math.factorial(len(factores) - r - 1) / math.factorial(len(factores))
                    s += peso * (valor(set(sub), clave) - valor(set(sub) | {f}, clave))
            aporte[f] = s
        reparto[clave] = {'R0': valor(set(), clave), 'con_todo': valor(set(factores), clave),
                          'reduccion_total': total, **{f'por_{f}': v for f, v in aporte.items()}}

    # --- Tabla de relieve a 26 km (nivel 3) con el océano real ---
    log('Relieve a 26 km…')
    campos = {k: col[k] for k in ('h', 's')}
    campos['lab'] = lab1
    ci, tipo3 = tcr.interpolar(campos, tipos, lat3, lon3, cont3)
    cam3 = [l_cam(cam, lat3[k], lon3[k]) if cont3[k] else float('nan') for k in range(m)]
    tc13 = [l_tc1(tc1, lat3[k], lon3[k]) if cont3[k] else float('nan') for k in range(m)]
    vol = c_mod['gel_m'] / 1000.0 * A_TIERRA_KM2
    w_tot = sum(w3)
    area_km2 = [x / w_tot * A_TIERRA_KM2 for x in w3]

    def relieve(p):
        z = []
        for k in range(m):
            if not cont3[k]:
                z.append(z3[k])
                continue
            c = {'cam': cam3[k], 'tc1': tc13[k], 'litho': ci['lab'][k], 'edad': EDAD_TIPO.get(tipo3[k], 300.0)}
            zk = altura_regla(con_edad(p, c), ci['h'][k], ci['s'][k], l_de(p, c))
            z.append(zk if math.isfinite(zk) else z3[k])
        zeta = trm.nivel_del_mar(z, area_km2, vol)
        return tcr.medir(z, w3, zeta)
    relieves = {'tierra': tcr.medir(z3, w3, 0.0),
                'tierra_1grado': tcr.medir(tcr.a_1grado_y_vuelta(z3, w3, lat3, lon3), w3, 0.0),
                'R0_tal_cual': relieve(r0_tal_cual)}
    for nombre, p in finales.items():
        log(f'  {nombre}…')
        relieves[nombre] = relieve(p)

    # --- Salida ---
    print(f"\n1. Reproducción de Lamb et al. (2020): α = {alfa} (en el {100 * f_alfa:.0f} % de las celdas), "
          f"β = {beta} ({100 * f_beta:.0f} %).")
    for z, etq in (('tierra', 'tierra continental'), ('todo', 'todos los nodos, también el océano')):
        print(f"   {etq}, {repro[z + '_celdas']} nodos: correlación de nuestra anomalía con la suya "
              f"{repro[z + '_correlacion']:.3f}; media {repro[z + '_anomalia_nuestra_media_m']:.0f} m (suya "
              f"{repro[z + '_anomalia_lamb_media_m']:.0f}); desviación {repro[z + '_anomalia_nuestra_desv_m']:.0f} m (suya "
              f"{repro[z + '_anomalia_lamb_desv_m']:.0f}); diferencia RMS {repro[z + '_diferencia_rms_m']:.0f} m")
    print('   Su tabla por continentes: altura, reducida (suya / nuestra), calculada con su regla:')
    for t in repro_tabla:
        print(f"   {t['nombre']:<16} {t['altura_m']:7.0f} m; reducida {t['reducida_m']:7.0f} / {t['reducida_nuestra_m']:7.0f}; "
              f"calculada {t['calculada_m']:7.0f} (C {t['c_km']:.1f} km, L {t['l_km']:.0f} km)")
    print('\n2–3. Error por celda de tierra (1°), ajuste y prueba cruzada (m: sesgo / error medio / RMS):')
    for nombre, f in cv.items():
        def e(x):
            return f"{x['sesgo_m']:+5.0f} / {x['error_medio_m']:4.0f} / {x['rms_m']:4.0f}"
        print(f"  {nombre:<11} ajuste A {e(f['ajuste_A'])} → prueba B {e(f['prueba_B'])} | ajuste B {e(f['ajuste_B'])} "
              f"→ prueba A {e(f['prueba_A'])} | todo {e(f['todo'])}")
        q = {k: v for k, v in finales[nombre].items() if k not in ('l',)}
        print(f"              parámetros (todo): {q}")
    claves = [('nivel_mar_m', 'Nivel del mar (m)', '{:+.0f}'), ('tierra_frac', 'Tierra emergida', '{:.3f}'),
              ('altura_media_tierra_m', 'Altura media de la tierra (m)', '{:.0f}'),
              ('tierra_bajo_200_frac', 'Tierra < 200 m', '{:.3f}'), ('tierra_sobre_2000_frac', 'Tierra > 2 000 m', '{:.3f}'),
              ('pendiente_media_tierra', 'Pendiente (±25 km)', '{:.4f}'), ('plataforma_km', 'Plataforma (km)', '{:.1f}')]
    print('\nRelieve a 26 km con el océano real:')
    nombres = list(relieves)
    print(f'{"":<30}' + ''.join(f'{n[:11]:>12}' for n in nombres))
    for clave, etq, fmt in claves:
        print(f'{etq:<30}' + ''.join(f'{fmt.format(relieves[n][clave]):>12}' for n in nombres))
    print('\nError medio por provincia (m) y altura real:')
    for i, (pn, _) in enumerate(PROVINCIAS):
        print(f"  {pn:<26} {100 * por_prov['R0'][i]['area_frac']:4.1f} %, real {por_prov['R0'][i]['altura_real_m']:5.0f} m: "
              + ', '.join(f"{n} {por_prov[n][i]['sesgo_m']:+5.0f}/{por_prov[n][i]['error_medio_m']:4.0f}" for n in finales))
    print('\nCasos con nombre (m):')
    for nombre_caso, d in casos.items():
        print(f"  {nombre_caso:<22} real {d['real_m']:5.0f} ({d['tipo']}, h {d['h_km']:.1f}, s {d['s_km']:.1f}, "
              f"L cam {d['l_cam_km']:.0f}, tc1 {d['l_tc1_km']:.0f}, edad {d['edad_myr']:.0f}): R0 tal cual "
              f"{d['R0_tal_cual']:.0f}; " + ', '.join(f"{n} {d[n]:.0f}" for n in finales))
    print('\n5. Reparto del error de R0 (Shapley):')
    for clave, r in reparto.items():
        print(f"  {clave}: R0 {r['R0']:.0f} → con todo {r['con_todo']:.0f}; manto litosférico {r['por_lit']:.0f}, "
              f"densidad de la corteza {r['por_dens']:.0f}, sedimento {r['por_sed']:.0f}")

    secciones = [('reproduccion_lamb', repro)]
    for t in repro_tabla:
        secciones.append((f'reproduccion_lamb.continentes."{t["nombre"]}"', {k: v for k, v in t.items() if k != 'nombre'}))
    for nombre, f in cv.items():
        for k, v in f.items():
            if k.startswith('parametros'):
                secciones.append((f'reglas.{nombre}.{k}', {kk: vv for kk, vv in v.items()}))
            else:
                secciones.append((f'reglas.{nombre}.{k}', v))
    for nombre, r in relieves.items():
        secciones.append((f'relieve.{nombre}', r))
    for nombre, filas in por_prov.items():
        for fila in filas:
            secciones.append((f'provincias.{nombre}."{fila["provincia"]}"', {k: v for k, v in fila.items() if k != 'provincia'}))
    for nombre_caso, d in casos.items():
        secciones.append((f'casos."{nombre_caso}"', d))
    for clave, r in reparto.items():
        secciones.append((f'reparto_error_R0.{clave}', r))
    secciones.append(('edad_por_tipo', dict(EDAD_TIPO)))
    cab = ('# Estudio de isostasia continental (docs/design/terreno/estudio_isostasia_continental.md).\n'
           '# Generado por herramientas/analisis/estudio_isostasia.py: CRUST1.0, LITHO1.0, CAM2016 (Priestley y\n'
           '# McKenzie), TC1 (Artemieva 2006, 5°), Lamb et al. (2020, Zenodo 4031646), ETOPO 2022 sin hielo,\n'
           '# Seton et al. (2020) y GlobSed. Errores en m (predicha − real) en la tierra continental a 1°.\n')
    rt.escribir_toml(a.salida, secciones, cab)
    log(f'escrito {a.salida}')


if __name__ == '__main__':
    main()
