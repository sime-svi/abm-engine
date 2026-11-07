"""La Tierra real con la regla del modelo: nivel del mar y profundidad media del océano.

Pregunta: con las edades reales del fondo (Seton et al. 2020), el grosor real del sedimento oceánico
(GlobSed, Straume et al. 2019) y el volumen de agua del modelo (`gel_m` de `config/terreno.toml`, el
del océano actual), ¿qué nivel del mar y qué profundidad media da la regla del modelo, es decir, la
curva de profundidad por edad y la isostasia del sedimento de `abm_terreno::tectonica::isostasia`?
En la Tierra, nivel 0 y profundidad media 3 667 m (la roca sin hielo con su rebote, a 26 km;
`config/referencias/tierra_relieve.toml`).

Cómo:
- Celdas de nivel 3 (~26 km), como la referencia del relieve (`referencia_tierra.py`).
- Fondo oceánico: las celdas con edad de Seton en al menos la mitad de su área. Su altura con la regla
  del modelo es `−d(edad) + S·(ρ_m − ρ_s)/(ρ_m − ρ_w)`, con `d` la curva del modelo (Parsons y
  Sclater con su aplanamiento exponencial, parámetros de la config) y `S` el grosor de GlobSed.
- El resto (continentes, plataformas y márgenes sin edad): la roca real con su rebote.
- El nivel del mar es el que llena las cuencas con el volumen de agua, como `isostasia::nivel_del_mar`.
- Además, la profundidad del zócalo (el fondo descargado de su sedimento) frente a la edad, por
  clases de 10 Myr, para ver si deja de crecer pasados 80–100 Myr. Se compara con la curva del modelo,
  con GDH1 (Stein y Stein 1992) y con la raíz de la edad sin límite, y se ajusta una curva de placa
  (la raíz del modelo hasta `t_x` y `d_∞ − A·e^(−t/τ)` después) a la mediana y a la media del zócalo
  real, con su nivel del mar.
- Los escalones de edad entre celdas vecinas a la escala de la deriva (nivel 2), con las condiciones
  del nacimiento espontáneo en zonas de fractura de la nota de fosas v5.

Datos (en `--datos`, por defecto `output/referencias`): los de `referencia_tierra.py` (ETOPO 2022,
Paxman et al., Seton et al.) y `globsed/globsed-v3.asc`: GlobSed v3 en ASCII de ArcGIS, de la copia
de CaltechDATA (doi:10.22002/k4070-ngc79, CC0) de lo que publicaba NOAA NCEI hasta mayo de 2025.

Uso: python3 -I herramientas/analisis/tierra_regla_modelo.py [--datos output/referencias]
Escribe `config/referencias/tierra_nivel_mar.toml`.
"""
import argparse
import array
import math
import os
import struct
import sys
import tomllib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import referencia_tierra as rt  # noqa: E402

N = rt.N3


def cfg_modelo():
    with open(os.path.join(rt.RAIZ, 'config', 'terreno.toml'), 'rb') as f:
        c = tomllib.load(f)['tectonica']
    return c


def prof_modelo(c, t):
    """`isostasia::profundidad_por_edad` (m, positiva)."""
    t = max(t, 0.0)
    if t <= c['t_x_myr']:
        return c['d_r_m'] + c['c_hs'] * math.sqrt(t)
    return c['d_inf_m'] - c['a_pl_m'] * math.exp(-t / c['tau_pl_myr'])


def prof_gdh1(t):
    """GDH1 (Stein y Stein 1992), m."""
    t = max(t, 0.0)
    return 2600.0 + 365.0 * math.sqrt(t) if t < 20.0 else 5651.0 - 2473.0 * math.exp(-0.0278 * t)


def prof_raiz(c, t):
    """La raíz de la edad sin límite, con los parámetros del modelo (m)."""
    return c['d_r_m'] + c['c_hs'] * math.sqrt(max(t, 0.0))


def ajustar_placa(c, tabla, clave):
    """Curva de placa ajustada a la Tierra: la raíz del modelo hasta `t_x` (`d_r + c_hs·√t`, que ya
    coincide con el zócalo real joven) y `d_∞ − A·e^(−t/τ)` después, continua en `t_x`. Ajusta `d_∞`
    y `τ` por mínimos cuadrados ponderados por área a la columna `clave` de la tabla del zócalo por
    edad (clases de 20 a 180 Myr). Devuelve (d_∞, A, τ, rms)."""
    tx = c['t_x_myr']
    d_tx = c['d_r_m'] + c['c_hs'] * math.sqrt(tx)
    filas = [x for x in tabla if tx <= x['desde_myr'] < 180.0]
    mejor = None
    for d_inf in range(4800, 6601, 10):
        for t10 in range(50, 1001, 5):
            tau = t10 / 10.0
            a = (d_inf - d_tx) * math.exp(tx / tau)
            err = sum(x['area_frac'] * (d_inf - a * math.exp(-(x['desde_myr'] + 5.0) / tau) - x[clave]) ** 2
                      for x in filas)
            if mejor is None or err < mejor[0]:
                mejor = (err, float(d_inf), a, tau)
    w = sum(x['area_frac'] for x in filas)
    return mejor[1], mejor[2], mejor[3], math.sqrt(mejor[0] / w)


def acumular_seton(ruta, n=N):
    """Edad media (Myr) y fracción del área con edad de cada celda del cubo de `n` por arista."""
    b = open(ruta, 'rb').read()
    var = rt.leer_netcdf3(b)
    nombre_z = 'z' if 'z' in var else [k for k in var if len(var[k][0]) == 2][0]
    (ny, nx), t, inicio = var[nombre_z]
    fmt = {5: 'f', 6: 'd'}[t]
    es = 4 if fmt == 'f' else 8

    def eje(nombres, nd):
        for nm in nombres:
            if nm in var:
                _, tt, ini = var[nm]
                f = 'd' if tt == 6 else 'f'
                return struct.unpack(f'>{nd}{f}', b[ini:ini + (8 if f == 'd' else 4) * nd])
        raise KeyError(nombres)
    lats, lons = eje(('lat', 'y'), ny), eje(('lon', 'x'), nx)
    datos = array.array(fmt, b[inicio:inicio + es * nx * ny])
    if sys.byteorder == 'little':
        datos.byteswap()
    m = 6 * n * n
    sw, saw, stot = array.array('d', bytes(8 * m)), array.array('d', bytes(8 * m)), array.array('d', bytes(8 * m))
    cl = [math.cos(math.radians(x)) for x in lons]
    sl = [math.sin(math.radians(x)) for x in lons]
    for r in range(ny):
        la = math.radians(lats[r])
        cla, sla = math.cos(la), math.sin(la)
        w = cla * (0.5 if r in (0, ny - 1) else 1.0)
        if w <= 0:
            continue
        fila = r * nx
        for c in range(nx - 1):  # la última columna repite la primera
            k = rt.localizar(cla * cl[c], cla * sl[c], sla, n)
            stot[k] += w
            a = datos[fila + c]
            if math.isfinite(a):
                sw[k] += w
                saw[k] += a * w
    edad = [saw[k] / sw[k] if sw[k] > 0 else float('nan') for k in range(m)]
    frac = [sw[k] / stot[k] if stot[k] > 0 else 0.0 for k in range(m)]
    return edad, frac


def vecinas_de_lado(n):
    """Pares de celdas vecinas por un lado (4 vecinas) del cubo de `n`, cada par una vez."""
    paso = (math.pi / 2) / n
    pares = set()
    for idx in range(6 * n * n):
        c, r = divmod(idx, n * n)
        j, i = divmod(r, n)
        a, b, nr = rt.BASES[c]
        for di, dj in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            ii, jj = i + di, j + dj
            if 0 <= ii < n and 0 <= jj < n:
                v = (c * n + jj) * n + ii
            else:
                fi = -1e-3 if ii < 0 else (n + 1e-3 if ii >= n else ii + 0.5)
                fj = -1e-3 if jj < 0 else (n + 1e-3 if jj >= n else jj + 0.5)
                tx = math.tan(-math.pi / 4 + fi * paso)
                ty = math.tan(-math.pi / 4 + fj * paso)
                x = [nr[k] + tx * a[k] + ty * b[k] for k in range(3)]
                v = rt.localizar(x[0], x[1], x[2], n)
            if v != idx:
                pares.add((min(idx, v), max(idx, v)))
    return sorted(pares)


def escalones_de_edad(edad, frac, n, a_min, da_fz):
    """Longitud (km) de los escalones de edad entre celdas vecinas con edad (zonas de fractura,
    transformantes y lo que separe dos fondos de edad distinta), por tamaño del escalón y con las
    condiciones del nacimiento espontáneo de la nota de fosas v5 (lado viejo de más de `a_min`,
    contraste de más de la mitad de la edad vieja, escalón de más de `da_fz`)."""
    lado = (math.pi / 2) * rt.R_KM / n
    umbrales = [5.0, 10.0, 20.0, 40.0, 70.0]
    largo = [0.0] * len(umbrales)
    v5 = v5_sin_contraste = v5_a100 = total = 0.0
    for u, v in vecinas_de_lado(n):
        if frac[u] < 0.5 or frac[v] < 0.5 or not (math.isfinite(edad[u]) and math.isfinite(edad[v])):
            continue
        total += lado
        vieja, joven = max(edad[u], edad[v]), min(edad[u], edad[v])
        d = vieja - joven
        for i, x in enumerate(umbrales):
            if d > x:
                largo[i] += lado
        if d > da_fz and vieja > a_min:
            v5_sin_contraste += lado
            if d > 0.5 * vieja:
                v5 += lado
        if d > da_fz and vieja > 100.0 and d > 0.5 * vieja:
            v5_a100 += lado
    return {
        'nivel_lado_km': lado, 'vecinas_con_edad_km': total,
        'escalon_mas_de_myr': umbrales, 'largo_km': largo,
        'regla_v5_km': v5, 'regla_v5_sin_contraste_km': v5_sin_contraste,
        'regla_v5_con_a_min_100_km': v5_a100,
    }


def acumular_globsed(ruta):
    """Grosor medio del sedimento (m) de cada celda de nivel 3 (NaN si no hay dato)."""
    m = 6 * N * N
    sw, ssw = array.array('d', bytes(8 * m)), array.array('d', bytes(8 * m))
    with open(ruta) as f:
        cab = {}
        for _ in range(6):
            k, v = f.readline().split()
            cab[k.lower()] = float(v)
        nx, ny = int(cab['ncols']), int(cab['nrows'])
        x0 = cab['xllcorner'] + 0.5 * cab['cellsize']
        y0 = cab['yllcorner'] + 0.5 * cab['cellsize']
        cs, nodata = cab['cellsize'], cab['nodata_value']
        lons = [x0 + c * cs for c in range(nx)]
        cl = [math.cos(math.radians(x)) for x in lons]
        sl = [math.sin(math.radians(x)) for x in lons]
        for r in range(ny):
            vals = f.readline().split()
            lat = y0 + (ny - 1 - r) * cs   # la primera fila es la del norte
            la = math.radians(lat)
            cla, sla = math.cos(la), math.sin(la)
            w = cla * (0.5 if r in (0, ny - 1) else 1.0)
            if w <= 0:
                continue
            for c in range(nx - 1):
                try:
                    v = float(vals[c])
                except ValueError:   # «1.#QNAN»: el NaN de Windows con que GlobSed marca el hueco
                    continue
                if v == nodata or not math.isfinite(v):
                    continue
                k = rt.localizar(cla * cl[c], cla * sl[c], sla, N)
                sw[k] += w
                ssw[k] += v * w
    return [ssw[k] / sw[k] if sw[k] > 0 else float('nan') for k in range(m)]


def nivel_del_mar(z, area, vol_km3):
    """Como `isostasia::nivel_del_mar`: Σ max(0, ζ − z)·A = V, por bisección (m)."""
    lo, hi = -12000.0, 12000.0
    for _ in range(200):
        mid = 0.5 * (lo + hi)
        v = sum(max(0.0, (mid - zk) / 1000.0) * a for zk, a in zip(z, area))
        if v < vol_km3:
            lo = mid
        else:
            hi = mid
        if hi - lo < 1e-4:
            break
    return 0.5 * (lo + hi)


def prof_media(z, area, zeta):
    num = sum((zeta - zk) * a for zk, a in zip(z, area) if zk < zeta)
    den = sum(a for zk, a in zip(z, area) if zk < zeta)
    return num / den, den


def mediana(pares):
    pares = sorted(pares)
    tot = sum(w for _, w in pares)
    acc = 0.0
    for v, w in pares:
        acc += w
        if acc >= 0.5 * tot:
            return v
    return float('nan')


def main():
    ap = argparse.ArgumentParser(description='La Tierra real con la regla del modelo (nivel del mar).')
    ap.add_argument('--datos', default=os.path.join(rt.RAIZ, 'output', 'referencias'))
    ap.add_argument('--salida', default=os.path.join(rt.RAIZ, 'config', 'referencias', 'tierra_nivel_mar.toml'))
    a = ap.parse_args()
    c = cfg_modelo()
    k_sed = (c['rho_m'] - c['rho_s']) / (c['rho_m'] - c['rho_w'])
    cel3, _, _ = rt.relieve_nivel3(a.datos)
    rr = cel3['roca_rebote']
    m = 6 * N * N
    rt.log('Edades de Seton a celdas de nivel 3…')
    edad, frac = acumular_seton(os.path.join(a.datos, 'seton2020_edades_6m.grd'))
    rt.log('Sedimento de GlobSed a celdas de nivel 3…')
    sed = acumular_globsed(os.path.join(a.datos, 'globsed', 'globsed-v3.asc'))
    a_tot_km2 = 4.0 * math.pi * rt.R_KM ** 2
    w_tot = sum(rr.w)
    idx = [k for k in range(m) if rr.w[k] > 0]
    area = [rr.w[k] / w_tot * a_tot_km2 for k in idx]
    z_real = [rr.media(k) for k in idx]
    oc = [frac[k] >= 0.5 and math.isfinite(edad[k]) for k in idx]
    s_m = [sed[k] if math.isfinite(sed[k]) else 0.0 for k in idx]
    vol = c['gel_m'] / 1000.0 * a_tot_km2

    def superficie(curva, k_s=k_sed, hasta=math.inf):
        """El fondo con edad con la regla (la curva `curva` y el sedimento con el factor `k_s`); el
        de más de `hasta` Myr y el resto del planeta, como en la Tierra."""
        return [(-curva(edad[k]) + k_s * s) if o and edad[k] <= hasta else zr
                for k, o, s, zr in zip(idx, oc, s_m, z_real)]

    res = {}
    # La Tierra tal cual (la referencia del relieve) y con la regla del modelo.
    pm0, a_mar0 = prof_media(z_real, area, 0.0)
    zeta_r = nivel_del_mar(z_real, area, vol)
    pm_r, _ = prof_media(z_real, area, zeta_r)
    res['tierra'] = {'volumen_km3': vol, 'profundidad_media_a_nivel_0_m': pm0, 'area_mar_frac': a_mar0 / a_tot_km2,
                     'nivel_mar_con_el_volumen_m': zeta_r, 'profundidad_media_m': pm_r,
                     'fondo_con_edad_frac': sum(a for a, o in zip(area, oc) if o) / a_tot_km2}
    k_2000 = (c['rho_m'] - 2000.0) / (c['rho_m'] - c['rho_w'])
    mod = lambda t: prof_modelo(c, t)  # noqa: E731
    variantes = [
        ('modelo', mod, k_sed, math.inf),
        ('modelo_sin_sedimento', mod, 0.0, math.inf),
        # Sensibilidad: sedimento pelágico poco compactado (ρ_s ≈ 2 000 kg/m³ en su primer km).
        ('modelo_rho_s_2000', mod, k_2000, math.inf),
        # Qué parte viene del fondo joven: la regla solo hasta 80 Myr, el resto como en la Tierra.
        ('modelo_solo_hasta_80_myr', mod, k_sed, 80.0),
        ('gdh1', prof_gdh1, k_sed, math.inf),
        ('raiz_sin_limite', lambda t: prof_raiz(c, t), k_sed, math.inf),
    ]
    for nombre, curva, k_s, hasta in variantes:
        z = superficie(curva, k_s, hasta)
        zeta = nivel_del_mar(z, area, vol)
        pm, a_m = prof_media(z, area, zeta)
        # Profundidad media del fondo con edad (respecto al nivel del mar de cada caso).
        po = sum((zeta - zk) * ar for zk, ar, o in zip(z, area, oc) if o) / sum(ar for ar, o in zip(area, oc) if o)
        res[nombre] = {'nivel_mar_m': zeta, 'profundidad_media_m': pm, 'area_mar_frac': a_m / a_tot_km2,
                       'profundidad_media_fondo_con_edad_m': po}
    # Fondo con edad en la Tierra (respecto al nivel 0), observado.
    res['tierra']['profundidad_media_fondo_con_edad_m'] = (
        -sum(zr * ar for zr, ar, o in zip(z_real, area, oc) if o) / sum(ar for ar, o in zip(area, oc) if o))
    res['tierra']['sedimento_medio_fondo_con_edad_m'] = (
        sum(s * ar for s, ar, o in zip(s_m, area, oc) if o) / sum(ar for ar, o in zip(area, oc) if o))
    # Profundidad del zócalo frente a la edad.
    lim = list(range(0, 200, 10))
    clases = {i: [] for i in range(len(lim) + 1)}
    for k, o, s, zr, ar in zip(idx, oc, s_m, z_real, area):
        if not o:
            continue
        i = min(int(edad[k] // 10), len(lim))
        clases[i].append((-zr + k_sed * s, -zr, ar, s))
    tabla = []
    for i in range(len(lim) + 1):
        t0 = lim[i] if i < len(lim) else 200
        tm = t0 + 5.0 if i < len(lim) else 210.0
        v = clases[i]
        if not v:
            continue
        tabla.append({
            'desde_myr': float(t0), 'area_frac': sum(x[2] for x in v) / a_tot_km2,
            'zocalo_mediana_m': mediana([(x[0], x[2]) for x in v]),
            'zocalo_media_m': sum(x[0] * x[2] for x in v) / sum(x[2] for x in v),
            'fondo_mediana_m': mediana([(x[1], x[2]) for x in v]),
            'sedimento_media_m': sum(x[3] * x[2] for x in v) / sum(x[2] for x in v),
            'modelo_m': prof_modelo(c, tm), 'gdh1_m': prof_gdh1(tm), 'raiz_m': prof_raiz(c, tm),
        })
    res['zocalo_por_edad'] = {
        'desde_myr': [x['desde_myr'] for x in tabla],
        'area_frac': [x['area_frac'] for x in tabla],
        'zocalo_mediana_m': [x['zocalo_mediana_m'] for x in tabla],
        'zocalo_media_m': [x['zocalo_media_m'] for x in tabla],
        'fondo_mediana_m': [x['fondo_mediana_m'] for x in tabla],
        'sedimento_media_m': [x['sedimento_media_m'] for x in tabla],
        'modelo_m': [x['modelo_m'] for x in tabla],
        'gdh1_m': [x['gdh1_m'] for x in tabla],
        'raiz_sin_limite_m': [x['raiz_m'] for x in tabla],
    }
    # Curvas de placa ajustadas al zócalo real (a su mediana y a su media), y su nivel del mar.
    for clave, nombre in (('zocalo_mediana_m', 'placa_ajustada_mediana'), ('zocalo_media_m', 'placa_ajustada_media')):
        d_inf, a_pl, tau, rms = ajustar_placa(c, tabla, clave)

        def curva(t, d_inf=d_inf, a_pl=a_pl, tau=tau):
            t = max(t, 0.0)
            if t <= c['t_x_myr']:
                return c['d_r_m'] + c['c_hs'] * math.sqrt(t)
            return d_inf - a_pl * math.exp(-t / tau)
        z = superficie(curva, k_sed, math.inf)
        zeta = nivel_del_mar(z, area, vol)
        pm, a_m = prof_media(z, area, zeta)
        po = sum((zeta - zk) * ar for zk, ar, o in zip(z, area, oc) if o) / sum(ar for ar, o in zip(area, oc) if o)
        res[nombre] = {'d_inf_m': d_inf, 'a_pl_m': a_pl, 'tau_pl_myr': tau, 'rms_ajuste_m': rms,
                       'nivel_mar_m': zeta, 'profundidad_media_m': pm, 'area_mar_frac': a_m / a_tot_km2,
                       'profundidad_media_fondo_con_edad_m': po}
        variantes.append((nombre, curva, k_sed, math.inf))
    print(f"\nVolumen de agua del modelo: {vol:.4g} km³ (gel_m = {c['gel_m']:.0f} m)")
    t = res['tierra']
    print(f"Tierra (roca sin hielo con rebote): a nivel 0, profundidad media {t['profundidad_media_a_nivel_0_m']:.0f} m "
          f"y mar {100 * t['area_mar_frac']:.1f} %; con el volumen del modelo, nivel {t['nivel_mar_con_el_volumen_m']:+.0f} m "
          f"y profundidad media {t['profundidad_media_m']:.0f} m; fondo con edad {100 * t['fondo_con_edad_frac']:.1f} % del "
          f"planeta, a {t['profundidad_media_fondo_con_edad_m']:.0f} m de media con {t['sedimento_medio_fondo_con_edad_m']:.0f} m de sedimento")
    for nombre, *_ in variantes:
        r = res[nombre]
        print(f"{nombre:<22} nivel del mar {r['nivel_mar_m']:+7.0f} m; profundidad media {r['profundidad_media_m']:5.0f} m "
              f"(mar {100 * r['area_mar_frac']:.1f} %); fondo con edad a {r['profundidad_media_fondo_con_edad_m']:.0f} m")
    for nombre in ('placa_ajustada_mediana', 'placa_ajustada_media'):
        r = res[nombre]
        print(f"  {nombre}: d_∞ = {r['d_inf_m']:.0f} m, A = {r['a_pl_m']:.0f} m, τ = {r['tau_pl_myr']:.1f} Myr "
              f"(error rms {r['rms_ajuste_m']:.0f} m)")
    print('\nZócalo (fondo descargado del sedimento) por edad (m): mediana y media; fondo y sedimento:')
    print(f"{'edad':>8} {'área %':>7} {'zóc.med':>7} {'zóc.mm':>7} {'fondo':>7} {'sedim.':>7} {'modelo':>7} {'GDH1':>7} {'raíz':>7}")
    for x in tabla:
        print(f"{x['desde_myr']:>6.0f}+ {100 * x['area_frac']:>7.2f} {x['zocalo_mediana_m']:>7.0f} {x['zocalo_media_m']:>7.0f} "
              f"{x['fondo_mediana_m']:>7.0f} {x['sedimento_media_m']:>7.0f} "
              f"{x['modelo_m']:>7.0f} {x['gdh1_m']:>7.0f} {x['raiz_m']:>7.0f}")
    # Escalones de edad a la escala de la deriva (nivel 2, ~52 km: `nivel_malla`).
    n2 = rt.N0 << 2
    rt.log('Escalones de edad de Seton a celdas de nivel 2…')
    e2, f2 = acumular_seton(os.path.join(a.datos, 'seton2020_edades_6m.grd'), n2)
    esc = escalones_de_edad(e2, f2, n2, c['a_min_myr'], c.get('da_fz_myr', 10.0))
    print(f"\nEscalones de edad entre vecinas (nivel 2, lado {esc['nivel_lado_km']:.0f} km), de "
          f"{esc['vecinas_con_edad_km']:.3g} km de lados con edad a los dos lados:")
    for x, l in zip(esc['escalon_mas_de_myr'], esc['largo_km']):
        print(f"  escalón de más de {x:>4.0f} Myr: {l:>9.0f} km")
    print(f"  regla de la nota v5 (viejo > {c['a_min_myr']:.0f} Myr, contraste > ½·viejo, escalón > Δa_fz): "
          f"{esc['regla_v5_km']:.0f} km; sin el contraste: {esc['regla_v5_sin_contraste_km']:.0f} km; "
          f"con a_min = 100: {esc['regla_v5_con_a_min_100_km']:.0f} km")
    secciones = [(f'nivel_mar.{k}', v) for k, v in res.items()]
    secciones.append(('zonas_fractura', esc))
    cab = ('# La Tierra real con la regla del modelo (nivel del mar y profundidad media del océano).\n'
           '# Generado por herramientas/analisis/tierra_regla_modelo.py: edades de Seton et al. (2020),\n'
           '# sedimento de GlobSed (Straume et al. 2019; copia de CaltechDATA doi:10.22002/k4070-ngc79) y la\n'
           '# roca sin hielo con su rebote de ETOPO 2022 y Paxman et al. (2022), en celdas de nivel 3 (~26 km).\n'
           '# "tierra": la roca real; "modelo": el fondo con edad con la curva de profundidad y la isostasia\n'
           '# del sedimento del modelo (config/terreno.toml); "gdh1" y "raiz_sin_limite": otras curvas.\n')
    rt.escribir_toml(a.salida, secciones, cab)
    rt.log(f'escrito {a.salida}')


if __name__ == '__main__':
    main()
