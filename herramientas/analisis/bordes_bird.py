#!/usr/bin/env python3
"""Longitud de los bordes de placa de la Tierra medida igual que la del modelo (adenda de balance v2 §10).

Una malla de 52 km no puede dibujar una línea más fina que sus celdas. Para comparar igual con igual,
este script proyecta los bordes de Bird (2003, PB2002) sobre la malla tectónica del planeta (nivel 2
del cubo, ~52 km) y los mide con el mismo método que `tectonica/deriva.rs`:

1. Las placas se reconstruyen como regiones de la malla: se marcan las celdas que cruza cada tramo de
   Bird, se rellenan las regiones que quedan entre las líneas (4 vecinas) y cada celda marcada pasa a la
   región más frecuente entre sus vecinas.
2. En cada celda que tiene vecinas de otra placa, la longitud es `L = 2·A·|∇χ|`: el gradiente, por
   mínimos cuadrados sobre las 8 vecinas, del indicador de la placa vecina dominante. Es la medida de
   `clasificar_bordes`. Dorsales, choques y fallas transformantes cuentan la mitad en cada lado; las
   fosas, entera en el lado de la placa que se hunde, como `metricas_del_paso`.
3. El tipo se da de dos maneras:
   - **A, clases de Bird:** la clase del tramo de Bird más cercano (OSR, CRB, SUB, OCB, CCB, OTF, CTF);
   - **B, regla del modelo:** con la normal de la malla y la velocidad relativa del tramo de Bird más
     cercano, transformante si la velocidad normal es menor que `f_transformante` veces la relativa
     (0,26 en el modelo; se mide también con el umbral de Bird, 20°); si no, divergente o
     convergente. Un borde convergente es choque si los dos lados son continentales y fosa si no.
     Es oceánica la celda que tiene edad en la rejilla de Seton et al. (2020), y entre dos oceánicas se
     hunde la más vieja, como en el modelo.

Escribe `config/referencias/tierra_bordes.toml`.

Uso: python3 -I herramientas/analisis/bordes_bird.py [--datos output/referencias]
"""
import argparse
import array
import math
import os
import sys
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import referencia_tierra as rt  # noqa: E402

URL = 'http://peterbird.name/oldFTP/PB2002/PB2002_steps.dat.txt'
NIVEL = 2
R = rt.R_KM
CLASES = ('OSR', 'CRB', 'SUB', 'OCB', 'CCB', 'OTF', 'CTF')
# Clases de Bird → tipos del modelo. OCB (convergencia oceánica sin fosa desarrollada) es, para el
# modelo, una subducción entre dos placas oceánicas.
TIPO = {'OSR': 'dorsal', 'CRB': 'dorsal', 'SUB': 'fosa', 'OCB': 'fosa', 'CCB': 'choque',
        'OTF': 'transformante', 'CTF': 'transformante'}
TIPOS = ('dorsal', 'fosa', 'choque', 'transformante')
F_MODELO = 0.26                      # config/terreno.toml, f_transformante (≈15°)
F_BIRD = math.sin(math.radians(20))  # umbral que separa las clases de Bird (ver `umbral_de_bird`)
V_MIN = 0.5                          # mm/año: por debajo, transformante (0,05 cm/año en el modelo)


def dir_de(lon, lat):
    lo, la = math.radians(lon), math.radians(lat)
    return (math.cos(la) * math.cos(lo), math.cos(la) * math.sin(lo), math.sin(la))


def leer_tramos(ruta):
    """Tramos de Bird: (dir1, dir2, clase, longitud km, velocidad mm/año, az. del tramo, az. de la
    velocidad). La clase puede llevar ':' delante y '*' detrás (tramo dentro de un orógeno)."""
    tramos = []
    for linea in open(ruta):
        c = linea.split()
        if len(c) < 15:
            continue
        clase = c[-1].strip(':*')
        lon1, lat1, lon2, lat2, largo, az, v, az_v = (float(x) for x in c[2:10])
        tramos.append((dir_de(lon1, lat1), dir_de(lon2, lat2), clase, largo, v, az, az_v,
                       float(c[10]), float(c[11])))
    return tramos


def umbral_de_bird(tramos):
    """Oblicuidad (ángulo entre la velocidad relativa y el borde) que separa las transformantes de
    Bird del resto: percentil 95 de las transformantes y percentil 5 de las demás, por longitud."""
    def pct(lista, q):
        lista.sort()
        tot = sum(L for _, L in lista)
        acc = 0.0
        for a, L in lista:
            acc += L
            if acc >= q * tot:
                return a
        return float('nan')
    tr, otras = [], []
    for t in tramos:
        a = math.degrees(math.atan2(abs(t[7]), abs(t[8])))
        (tr if t[2] in ('OTF', 'CTF') else otras).append((a, t[3]))
    return pct(tr, 0.95), pct(otras, 0.05)


def area_celda(idx, n):
    """Área (km²) de la celda equiangular `idx`: ángulo sólido del rectángulo gnomónico."""
    j, i = divmod(idx % (n * n), n)
    paso = (math.pi / 2) / n

    def f(x, y):
        return math.atan(x * y / math.sqrt(1 + x * x + y * y))
    x0, x1 = math.tan(-math.pi / 4 + i * paso), math.tan(-math.pi / 4 + (i + 1) * paso)
    y0, y1 = math.tan(-math.pi / 4 + j * paso), math.tan(-math.pi / 4 + (j + 1) * paso)
    return (f(x1, y1) - f(x0, y1) - f(x1, y0) + f(x0, y0)) * R * R


def vecinas_orto(n):
    """Las 4 vecinas de lado de cada celda (cruzando aristas de cara por geometría)."""
    paso = (math.pi / 2) / n
    out = []
    for idx in range(6 * n * n):
        c, r = divmod(idx, n * n)
        j, i = divmod(r, n)
        a, b, nr = rt.BASES[c]
        vs = []
        for di, dj in ((0, -1), (-1, 0), (1, 0), (0, 1)):
            ii, jj = i + di, j + dj
            if 0 <= ii < n and 0 <= jj < n:
                vs.append((c * n + jj) * n + ii)
                continue
            fi = -1e-3 if ii < 0 else (n + 1e-3 if ii >= n else ii + 0.5)
            fj = -1e-3 if jj < 0 else (n + 1e-3 if jj >= n else jj + 0.5)
            tx = math.tan(-math.pi / 4 + fi * paso)
            ty = math.tan(-math.pi / 4 + fj * paso)
            v = [nr[k] + tx * a[k] + ty * b[k] for k in range(3)]
            vs.append(rt.localizar(v[0], v[1], v[2], n))
        out.append(vs)
    return out


class Edades:
    """Edad del fondo oceánico (Seton et al. 2020, rejilla de 6') en el nodo más cercano."""

    def __init__(self, ruta):
        b = open(ruta, 'rb').read()
        var = rt.leer_netcdf3(b)
        nz = 'z' if 'z' in var else [k for k in var if len(var[k][0]) == 2][0]
        (self.ny, self.nx), t, inicio = var[nz]
        fmt = {5: 'f', 6: 'd'}[t]
        es = 4 if fmt == 'f' else 8
        self.z = array.array(fmt, b[inicio:inicio + es * self.nx * self.ny])
        if sys.byteorder == 'little':
            self.z.byteswap()

        def eje(nombres, nd):
            for nm in nombres:
                if nm in var:
                    _, tv, i0 = var[nm]
                    f2 = {5: 'f', 6: 'd'}[tv]
                    a = array.array(f2, b[i0:i0 + (4 if f2 == 'f' else 8) * nd])
                    if sys.byteorder == 'little':
                        a.byteswap()
                    return a[0], a[-1]
            raise KeyError(nombres)
        self.lat0, self.lat1 = eje(('lat', 'y'), self.ny)
        self.lon0, self.lon1 = eje(('lon', 'x'), self.nx)

    def edad(self, d):
        lat = math.degrees(math.asin(max(-1.0, min(1.0, d[2]))))
        lon = math.degrees(math.atan2(d[1], d[0]))
        r = round((lat - self.lat0) / (self.lat1 - self.lat0) * (self.ny - 1))
        c = round(((lon - self.lon0) % 360.0) / (self.lon1 - self.lon0) * (self.nx - 1))
        r, c = min(max(r, 0), self.ny - 1), min(max(c, 0), self.nx - 1)
        a = self.z[r * self.nx + c]
        return a if math.isfinite(a) else None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--datos', default=os.path.join(rt.RAIZ, 'output', 'referencias'))
    ap.add_argument('--salida', default=os.path.join(rt.RAIZ, 'config', 'referencias', 'tierra_bordes.toml'))
    ap.add_argument('--placas', type=int, default=15, help='placas de la Tierra reducida (las del modelo)')
    a = ap.parse_args()
    ruta = os.path.join(a.datos, 'bird2003_PB2002_steps.dat.txt')
    if not os.path.exists(ruta):
        urllib.request.urlretrieve(URL, ruta)
    tramos = leer_tramos(ruta)
    vect = {c: 0.0 for c in CLASES}
    for t in tramos:
        vect[t[2]] += t[3]
    u_tr, u_otras = umbral_de_bird(tramos)
    rt.log(f'{len(tramos)} tramos; oblicuidad: p95 de las transformantes {u_tr:.1f}°, p5 del resto {u_otras:.1f}°')
    edades = Edades(os.path.join(a.datos, 'seton2020_edades_6m.grd'))
    n = rt.N0 << NIVEL
    num = 6 * n * n
    # 1) Celdas que cruza cada tramo (muestreo cada ~5 km).
    borde = [False] * num
    for d1, d2, _, largo, *_ in tramos:
        m = max(2, int(largo / 5.0) + 1)
        for i in range(m + 1):
            f = i / m
            v = [d1[k] * (1 - f) + d2[k] * f for k in range(3)]
            nv = math.sqrt(sum(x * x for x in v))
            borde[rt.localizar(v[0] / nv, v[1] / nv, v[2] / nv, n)] = True
    rt.log('vecinas ...')
    vec = rt.vecinas_cubo(n)
    orto = vecinas_orto(n)
    # 2) Placas = regiones conexas (4 vecinas) de celdas que no son borde; las de borde pasan a la
    #    región más frecuente entre sus 8 vecinas (desempate por número de región).
    etiqueta = [-1] * num
    nreg = 0
    for s in range(num):
        if borde[s] or etiqueta[s] >= 0:
            continue
        pila = [s]
        etiqueta[s] = nreg
        while pila:
            k = pila.pop()
            for v in orto[k]:
                if not borde[v] and etiqueta[v] < 0:
                    etiqueta[v] = nreg
                    pila.append(v)
        nreg += 1
    for _ in range(6):
        nuevas = []
        for s in range(num):
            if etiqueta[s] >= 0:
                continue
            cuenta = {}
            for v in vec[s]:
                if etiqueta[v] >= 0:
                    cuenta[etiqueta[v]] = cuenta.get(etiqueta[v], 0) + 1
            if cuenta:
                nuevas.append((s, max(sorted(cuenta), key=lambda x: cuenta[x])))
        for s, e in nuevas:
            etiqueta[s] = e
    sin_placa = sum(1 for e in etiqueta if e < 0)
    # Búsqueda del tramo más cercano: rejilla gruesa (nivel 0) por el punto medio de cada tramo.
    medios = {}
    for t, tr in enumerate(tramos):
        v = [(tr[0][k] + tr[1][k]) / 2 for k in range(3)]
        nv = math.sqrt(sum(x * x for x in v))
        medios.setdefault(rt.localizar(v[0] / nv, v[1] / nv, v[2] / nv, rt.N0), []).append(t)
    vec0 = rt.vecinas_cubo(rt.N0)
    dirs = [rt.centro(k, n) for k in range(num)]
    areas = [area_celda(k, n) for k in range(num)]
    print(f'Placas reconstruidas en la malla: {nreg} (Bird: 52); celdas sin placa: {sin_placa}')
    print(f'Oblicuidad que separa las transformantes de Bird: p95 de las transformantes {u_tr:.1f}°, '
          f'p5 del resto {u_otras:.1f}°')
    ctx = (tramos, vec, dirs, areas, medios, vec0, edades, n)
    todas = medir(etiqueta, ctx)
    informe(todas, vect, f'todas las placas ({nreg})')
    # Una Tierra con tantas placas como el modelo (~15): las pequeñas se funden, de menor a mayor,
    # con la vecina con la que comparten más borde.
    grandes = fundir_pequenas(etiqueta, vec, areas, a.placas)
    quince = medir(grandes, ctx)
    informe(quince, vect, f'las {a.placas} placas mayores')
    secciones = []
    for nombre, r in (('todas', todas), (f'mayores_{a.placas}', quince)):
        secciones += [
            (f'bordes.{nombre}.malla_clases_km', {c: r['a'][c] for c in CLASES} | {'total': sum(r['a'].values())}),
            (f'bordes.{nombre}.malla_por_tipo_km', {t: sum(r['a'][c] for c in CLASES if TIPO[c] == t) for t in TIPOS}),
            (f'bordes.{nombre}.regla_modelo_15_km', dict(r['b']['modelo']) | {'total': sum(r['b']['modelo'].values())}),
            (f'bordes.{nombre}.regla_bird_20_km', dict(r['b']['bird']) | {'total': sum(r['b']['bird'].values())}),
            (f'bordes.{nombre}.malla', {'placas': r['placas'], 'ambiguas_frac': r['ambiguas']}),
        ]
    secciones = [('bordes.vectorial_km', {c: vect[c] for c in CLASES} | {'total': sum(vect.values())}),
                 ('bordes.oblicuidad_grados', {'p95_transformantes': u_tr, 'p5_resto': u_otras})] + secciones
    cab = ('# Bordes de placa de la Tierra medidos como los del modelo (adenda de balance v2 §10).\n'
           '# Generado por herramientas/analisis/bordes_bird.py a partir de Bird (2003, PB2002_steps) y\n'
           '# Seton et al. (2020) para el tipo de corteza. Malla tectónica de nivel 2 (~52 km).\n'
           '# Longitudes en km: dorsales, choques y transformantes, la mitad de cada lado; fosas, el lado\n'
           '# que se hunde. "malla_clases": clase de Bird del tramo más cercano. "regla_modelo_15": tipo\n'
           '# con la regla de clasificar_bordes (f_transformante 0,26); "regla_bird_20", con 20°.\n'
           f'# "mayores_{a.placas}": las placas pequeñas fundidas con su vecina hasta quedar {a.placas}.\n')
    rt.escribir_toml(a.salida, secciones, cab)
    rt.log(f'escrito {a.salida}')


def fundir_pequenas(etiqueta, vec, areas, objetivo):
    """Funde la región más pequeña con la vecina con la que comparte más pares de celdas, hasta que
    queden `objetivo` regiones. Devuelve las etiquetas nuevas."""
    et = list(etiqueta)
    while True:
        area = {}
        for k, e in enumerate(et):
            area[e] = area.get(e, 0.0) + areas[k]
        if len(area) <= objetivo:
            return et
        menor = min(sorted(area), key=lambda e: area[e])
        comparte = {}
        for k, e in enumerate(et):
            if e != menor:
                continue
            for v in vec[k]:
                if et[v] != menor:
                    comparte[et[v]] = comparte.get(et[v], 0) + 1
        destino = max(sorted(comparte), key=lambda e: comparte[e])
        et = [destino if e == menor else e for e in et]


def medir(etiqueta, ctx):
    """Longitud de los bordes por clase de Bird (A) y por tipo con la regla del modelo (B)."""
    tramos, vec, dirs, areas, medios, vec0, edades, n = ctx
    num = len(etiqueta)
    largo_a = {c: 0.0 for c in CLASES}
    largo_b = {f: {t: 0.0 for t in TIPOS} for f in ('modelo', 'bird')}
    cruce = {c: {t: 0.0 for t in TIPOS} for c in CLASES}   # clase de Bird → tipo con la regla del modelo
    ambiguas = total_b = 0.0
    for k in range(num):
        p = etiqueta[k]
        if p < 0:
            continue
        otras = [etiqueta[v] for v in vec[k] if etiqueta[v] != p and etiqueta[v] >= 0]
        if not otras:
            continue
        q = max(sorted(set(otras)), key=otras.count)
        x = dirs[k]
        e, nn = rt.este_norte(x)
        sxx = sxy = syy = bx = by = 0.0
        for v in vec[k]:
            dv = [dirs[v][i] - x[i] for i in range(3)]
            de = sum(dv[i] * e[i] for i in range(3)) * R
            dn = sum(dv[i] * nn[i] for i in range(3)) * R
            chi = 1.0 if etiqueta[v] == q else 0.0
            sxx += de * de
            sxy += de * dn
            syy += dn * dn
            bx += de * chi
            by += dn * chi
        det = sxx * syy - sxy * sxy
        if abs(det) < 1e-12:
            continue
        ge, gn = (syy * bx - sxy * by) / det, (sxx * by - sxy * bx) / det
        gmod = math.hypot(ge, gn)
        if gmod <= 0.0:
            continue
        largo = 2 * areas[k] * gmod
        ne_, nn_ = ge / gmod, gn / gmod          # normal de p hacia q, en (este, norte)
        c0 = rt.localizar(x[0], x[1], x[2], rt.N0)
        cand = medios.get(c0, []) + [t for v in vec0[c0] for t in medios.get(v, [])]
        if not cand:
            continue
        t = max(cand, key=lambda t: sum(x[i] * (tramos[t][0][i] + tramos[t][1][i]) for i in range(3)))
        _, _, clase, _, vmm, az, az_v, _, _ = tramos[t]
        # A) Clase de Bird: mitad en cada lado.
        largo_a[clase] += 0.5 * largo
        # B) Regla del modelo. Normal de Bird que da su componente divergente: (−cos az, sin az).
        sa, ca = math.sin(math.radians(az)), math.cos(math.radians(az))
        ve, vn = vmm * math.sin(math.radians(az_v)), vmm * math.cos(math.radians(az_v))
        alin = ne_ * (-ca) + nn_ * sa
        s = 1.0 if alin >= 0 else -1.0
        div = s * (ve * ne_ + vn * nn_)            # > 0: se separan
        total_b += largo
        if abs(alin) < 0.3:
            ambiguas += largo
        # Celda de q más alineada con la normal (como `k_q` en el modelo).
        nrm3 = [ne_ * e[i] + nn_ * nn[i] for i in range(3)]
        kq = max((v for v in vec[k] if etiqueta[v] == q),
                 key=lambda v: (sum((dirs[v][i] - x[i]) * nrm3[i] for i in range(3)), -v))
        ep, eq = edades.edad(x), edades.edad(dirs[kq])
        for umbral, f in ((F_MODELO, 'modelo'), (F_BIRD, 'bird')):
            if vmm < V_MIN or abs(div) < umbral * vmm:
                tipo, cuenta = 'transformante', 0.5 * largo
            elif div > 0:
                tipo, cuenta = 'dorsal', 0.5 * largo
            elif ep is None and eq is None:
                tipo, cuenta = 'choque', 0.5 * largo
            else:
                # Se hunde la oceánica; entre dos oceánicas, la más vieja.
                hunde_p = eq is None or (ep is not None and ep > eq)
                tipo, cuenta = 'fosa', (largo if hunde_p else 0.0)
            largo_b[f][tipo] += cuenta
            if f == 'modelo':
                cruce[clase][tipo] += 0.5 * largo
    return {'a': largo_a, 'b': largo_b, 'cruce': cruce, 'ambiguas': ambiguas / total_b,
            'placas': len(set(etiqueta))}


def informe(r, vect, titulo):
    largo_a, largo_b, cruce = r['a'], r['b'], r['cruce']
    print(f'\n=== {titulo} ===')
    print(f'{"clase de Bird":<14}{"vectorial":>11}{"en la malla":>13}{"factor":>8}')
    for c in CLASES:
        print(f'{c:<14}{vect[c]:>11.0f}{largo_a[c]:>13.0f}{largo_a[c] / vect[c]:>8.2f}')
    tv, ta = sum(vect.values()), sum(largo_a.values())
    print(f'{"total":<14}{tv:>11.0f}{ta:>13.0f}{ta / tv:>8.2f}')
    a_tipo = {t: sum(largo_a[c] for c in CLASES if TIPO[c] == t) for t in TIPOS}
    v_tipo = {t: sum(vect[c] for c in CLASES if TIPO[c] == t) for t in TIPOS}
    print(f'\n{"tipo":<14}{"vectorial":>11}{"A: clases":>11}{"B: regla 15°":>14}{"B: regla 20°":>14}')
    for t in TIPOS:
        print(f'{t:<14}{v_tipo[t]:>11.0f}{a_tipo[t]:>11.0f}{largo_b["modelo"][t]:>14.0f}{largo_b["bird"][t]:>14.0f}')
    print(f'{"total":<14}{tv:>11.0f}{ta:>11.0f}{sum(largo_b["modelo"].values()):>14.0f}'
          f'{sum(largo_b["bird"].values()):>14.0f}')
    print(f'\nCon la regla del modelo (15°), cada clase de Bird pasa a (km, mitad por lado):')
    print(f'{"":<6}' + ''.join(f'{t:>15}' for t in TIPOS))
    for c in CLASES:
        print(f'{c:<6}' + ''.join(f'{cruce[c][t]:>15.0f}' for t in TIPOS))
    print(f'Celdas de borde con la normal de la malla casi paralela al tramo (ambiguas): '
          f'{r["ambiguas"]:.1%} de la longitud')


if __name__ == '__main__':
    main()
