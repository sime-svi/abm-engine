"""Comprobación previa de la nota de isostasia con litosfera (nota_isostasia_litosfera.md §0) y los datos
de la Tierra que necesita su implementación.

1. §0: repetir el ajuste de R4 del estudio de isostasia continental sin las celdas donde la propia regla
   de Lamb et al. (2020) deja más de 300 m de residuo (la topografía dinámica que la regla no tiene).
   El ajuste es por descenso por coordenadas, dentro de los rangos publicados de cada parámetro, desde
   los valores del estudio. Se mira si los parámetros salen del borde de su rango.
2. §11.1: el sedimento. R4 no le da flotación (k_s = 0). Se mide también la variante en que el
   sedimento de CRUST1.0 cuenta como parte de la corteza (con su densidad de capa), que es lo que es en
   el motor: su corteza continental no separa el sedimento viejo, y su `sed` es el que deja la erosión.
3. §11.3: la altura real de la tierra y la de R4 por clase de edad tectonotérmica (0–100, 100–500,
   500–1 600, 1 600–2 500 y más de 2 500 Myr).
4. §6.5: la distribución de la corteza continental por edad tectonotérmica para el estado inicial: la
   de hoy (área de la tierra por tipo de corteza de CRUST1.0, con la edad de cada tipo repartida en su
   rango), solo con las edades de más de 500 Myr, restándoles 500. Al lado, la que sale de invertir el
   grosor térmico de TC1 con la recta de Artemieva (z = 0,04·t + 93,6 km).

Uso: python3 -I herramientas/analisis/nota_isostasia_s0.py
Escribe config/referencias/tierra_isostasia_s0.toml.
"""
import argparse
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import referencia_tierra as rt  # noqa: E402
import tierra_regla_modelo as trm  # noqa: E402
import lluvia_observada as lo  # noqa: E402
import tierra_corteza_regla as tcr  # noqa: E402
import estudio_isostasia as ei  # noqa: E402

RAIZ = rt.RAIZ
N = rt.N3

# Rangos publicados (estudio §4) y valores de R4 del estudio.
RANGOS = {
    'rho_u': [2700.0, 2710.0, 2720.0, 2730.0, 2740.0, 2750.0],
    'rho_l': [2900.0, 2920.0, 2940.0, 2960.0, 2980.0, 3000.0],
    'z_t': [15.0, 17.5, 20.0, 22.5, 25.0],
    'drho': [20.0, 25.0, 30.0, 35.0, 40.0],
    'drho_arcaico': [0.0, 5.0, 10.0, 15.0, 20.0],
    'l_min': [40.0, 50.0, 60.0, 70.0, 80.0],
    'tau_l': [20.0, 50.0, 100.0, 150.0, 200.0],
}
R4_ESTUDIO = {'rho_a': 3255.0, 'rho_s': 2400.0, 'k_sed': 0.0, 'l': 'edad', 'rho_u': 2750.0, 'rho_l': 3000.0,
              'z_t': 15.0, 'drho': 20.0, 'drho_arcaico': 10.0, 'l_min': 40.0, 'tau_l': 100.0}
CLASES_EDAD = [0.0, 100.0, 500.0, 1600.0, 2500.0]
# Rango de edad (Myr) de cada tipo de corteza de más de 500 Myr (CNtype1_key.txt y el mapa de edades
# de Artemieva y Mooney): su edad se reparte uniformemente en él.
RANGO_TIPO = {'G1': (3000.0, 3600.0), 'G2': (2500.0, 3000.0), 'F-': (2500.0, 3600.0),
              'H1': (1000.0, 2500.0), 'H2': (1000.0, 2500.0), 'I1': (541.0, 1000.0), 'I2': (541.0, 1000.0)}


def log(*a):
    print(*a, file=sys.stderr, flush=True)


def descenso(base, celdas, rangos, max_vueltas=8):
    """Descenso por coordenadas: cada parámetro, por turno, al mejor valor de su rejilla (con H_0
    ajustado), hasta que ninguno cambia."""
    p = ei.ajustar_h0(dict(base), celdas)
    mejor = ei.errores(p, celdas)['rms_m']
    for _ in range(max_vueltas):
        cambio = False
        for k, vals in rangos.items():
            for v in vals:
                if v == p.get(k):
                    continue
                q = ei.ajustar_h0(dict(p, **{k: v}), celdas)
                e = ei.errores(q, celdas)['rms_m']
                if e < mejor - 1e-9:
                    p, mejor, cambio = q, e, True
        if not cambio:
            break
    return p


def en_borde(p, rangos):
    return {k: (p[k] == vals[0] or p[k] == vals[-1]) for k, vals in rangos.items() if k in p}


def main():
    ap = argparse.ArgumentParser(description='§0 de la nota de isostasia con litosfera.')
    ap.add_argument('--datos', default=os.path.join(RAIZ, 'output', 'referencias'))
    ap.add_argument('--cache', default=os.path.join(RAIZ, 'output', 'cache'))
    ap.add_argument('--salida', default=os.path.join(RAIZ, 'config', 'referencias', 'tierra_isostasia_s0.toml'))
    a = ap.parse_args()
    m = 6 * N * N
    cel = lo.relieve_roca_rebote(a.datos, a.cache)
    w3 = list(cel.w)
    z3 = [cel.zw[k] / cel.w[k] if cel.w[k] > 0 else float('nan') for k in range(m)]
    ruta_seton = os.path.join(a.datos, 'seton2020_edades_6m.grd')
    edad3, frac3 = tcr.en_cache(a.cache, 'seton_n3', tcr.huella(trm.acumular_seton, rt.localizar, rt.leer_netcdf3,
                                                               extra=f'{N}:{os.path.getsize(ruta_seton)}'),
                                lambda: trm.acumular_seton(ruta_seton), 2, m)
    oc3 = [frac3[k] >= 0.5 and math.isfinite(edad3[k]) for k in range(m)]
    bnds, rho, tipos = tcr.leer_crust1(tcr.descargar_crust1(a.datos))
    col = tcr.columnas_crust1(bnds, rho, 3300.0)
    cam = ei.leer_cam2016(a.datos)
    tc1 = ei.leer_tc1(a.datos)
    lat3, lon3 = tcr.lat_lon_centros(N)
    sw1 = [0.0] * 64800
    sz1 = [0.0] * 64800
    so1 = [0.0] * 64800
    for k in range(m):
        c1 = tcr.celda_1grado(lat3[k], lon3[k])
        sw1[c1] += w3[k]
        sz1[c1] += z3[k] * w3[k]
        so1[c1] += w3[k] * (1.0 if oc3[k] else 0.0)
    tierra = []
    for c1 in range(64800):
        if sw1[c1] <= 0 or so1[c1] / sw1[c1] >= 0.5:
            continue
        z = sz1[c1] / sw1[c1]
        if z < 0.0:
            continue
        la, lo_ = 89.5 - c1 // 360, -179.5 + c1 % 360
        t = tipos[c1]
        tierra.append({'i': c1, 'lat': la, 'lon': lo_, 'w': math.cos(math.radians(la)), 'z': z, 'tipo': t,
                       'h': col['h'][c1], 's': col['s'][c1], 'cam': ei.l_cam(cam, la, lo_),
                       'edad': ei.EDAD_TIPO.get(t, 300.0), 'mitad': ei.mitad(la, lo_)})
    a_tierra = sum(c['w'] for c in tierra)

    # La regla de Lamb tal cual (α 7,8 y β 93: estudio §3) y su residuo.
    alfa, beta, rho_a = 7.8, 93.0, 3255.0
    rho_c_lamb = rho_a + rho_a / beta - rho_a / alfa
    lamb = {'rho_a': rho_a, 'rho_c': rho_c_lamb, 'rho_s': rho_c_lamb, 'drho': rho_a / beta, 'l': 'cam',
            'h0': 1000.0 * (32.0 / alfa - 100.0 / beta)}
    sin_dinamica = []
    for c in tierra:
        zl = ei.altura_regla(lamb, c['h'], c['s'], c['cam'])
        if math.isfinite(zl) and abs(c['z'] - zl) <= 300.0:
            sin_dinamica.append(c)
    f_sin = sum(c['w'] for c in sin_dinamica) / a_tierra
    log(f'Celdas con el residuo de Lamb ≤ 300 m: {len(sin_dinamica)} de {len(tierra)} ({100 * f_sin:.0f} % de la tierra)')

    # 1. R4 con todas las celdas (el estudio) y sin las de más de 300 m de residuo de Lamb.
    log('R4 con todas las celdas (descenso por coordenadas)…')
    r4_todo = descenso(R4_ESTUDIO, tierra, RANGOS)
    log('R4 sin la topografía dinámica…')
    r4_s0 = descenso(R4_ESTUDIO, sin_dinamica, RANGOS)
    # Validación cruzada de R4 §0 entre mitades (con las celdas sin dinámica).
    cv = {}
    for aj, pr in (('A', 'B'), ('B', 'A')):
        p = descenso(r4_s0, [c for c in sin_dinamica if c['mitad'] == aj], RANGOS, max_vueltas=3)
        cv[f'ajuste_{aj}'] = ei.errores(p, [c for c in sin_dinamica if c['mitad'] == aj])
        cv[f'prueba_{pr}'] = ei.errores(p, [c for c in tierra if c['mitad'] == pr])

    # 2. El sedimento como parte de la corteza (lo que es en el motor).
    def como_corteza(c):
        return dict(c, h=c['h'] + c['s'], s=0.0)
    tierra_cc = [como_corteza(c) for c in tierra]
    sin_cc = [como_corteza(c) for c in sin_dinamica]
    log('R4 con el sedimento como corteza…')
    r4_cc = descenso(r4_s0, sin_cc, RANGOS)

    def resumen(p, celdas_todas, celdas_ajuste):
        return {'todo': ei.errores(p, celdas_todas), 'ajuste': ei.errores(p, celdas_ajuste),
                'parametros': {k: v for k, v in p.items() if k != 'l'}, 'en_borde': en_borde(p, RANGOS)}
    res = {'R4_estudio_todo': resumen(r4_todo, tierra, tierra),
           'R4_sin_dinamica': resumen(r4_s0, tierra, sin_dinamica),
           'R4_sedimento_como_corteza': resumen(r4_cc, tierra_cc, sin_cc)}

    # 3. Altura real y de R4 por clase de edad tectonotérmica.
    def por_clase(p, celdas):
        out = []
        for i, t0 in enumerate(CLASES_EDAD):
            t1 = CLASES_EDAD[i + 1] if i + 1 < len(CLASES_EDAD) else 1e9
            cs = [c for c in celdas if t0 <= c['edad'] < t1]
            aa = sum(c['w'] for c in cs)
            if aa <= 0:
                out.append({'desde_myr': t0, 'area_frac': 0.0})
                continue
            zr = sum(c['z'] * c['w'] for c in cs) / aa
            zp = sum(ei.altura_regla(ei.con_edad(p, c), c['h'], c['s'], ei.l_de(p, c)) * c['w'] for c in cs) / aa
            out.append({'desde_myr': t0, 'area_frac': aa / a_tierra, 'altura_real_m': zr, 'altura_regla_m': zp,
                        'h_media_km': sum(c['h'] * c['w'] for c in cs) / aa,
                        's_media_km': sum(c['s'] * c['w'] for c in cs) / aa})
        return out
    clases = {'R4_sin_dinamica': por_clase(r4_s0, tierra), 'R4_estudio': por_clase(r4_todo, tierra)}

    # 4. Distribución de edades para el estado inicial (área de la tierra de más de 500 Myr, −500).
    bordes = [0.0, 100.0, 200.0, 300.0, 400.0, 500.0, 750.0, 1000.0, 1250.0, 1500.0, 1750.0, 2000.0, 2250.0,
              2500.0, 2750.0, 3000.0, 3250.0]
    hist = [0.0] * (len(bordes) - 1)
    for c in tierra:
        r = RANGO_TIPO.get(c['tipo'])
        if r is None:
            continue
        # La celda reparte su área uniformemente en el rango del tipo (menos 500 Myr).
        t0, t1 = r[0] - 500.0, r[1] - 500.0
        for i in range(len(hist)):
            lo_b, hi_b = bordes[i], bordes[i + 1]
            sol = max(0.0, min(hi_b, t1) - max(lo_b, t0))
            hist[i] += c['w'] * sol / (t1 - t0)
    tot = sum(hist)
    dist_tipos = [x / tot for x in hist]
    viejo = tot / a_tierra
    # Comprobación: la de invertir el grosor térmico de TC1 (z = 0,04·t + 93,6).
    hist_tc1 = [0.0] * (len(bordes) - 1)
    for c in tierra:
        z = ei.l_tc1(tc1, c['lat'], c['lon'])
        if not math.isfinite(z):
            continue
        t = (z - 93.6) / 0.04 - 500.0
        if t <= 0.0:
            continue
        i = max(j for j in range(len(hist_tc1)) if bordes[j] <= t) if t < bordes[-1] else len(hist_tc1) - 1
        hist_tc1[i] += c['w']
    tot_tc1 = sum(hist_tc1)
    dist_tc1 = [x / tot_tc1 for x in hist_tc1]

    # 5. Relieve a 26 km (océano real, agua del modelo) de las tres versiones de R4.
    log('Relieve a 26 km…')
    c_mod = trm.cfg_modelo()
    cont3 = [not x for x in oc3]
    ci, tipo3 = tcr.interpolar({'h': col['h'], 's': col['s']}, tipos, lat3, lon3, cont3)
    vol = c_mod['gel_m'] / 1000.0 * ei.A_TIERRA_KM2
    w_tot = sum(w3)
    area_km2 = [x / w_tot * ei.A_TIERRA_KM2 for x in w3]

    def relieve(p, como_corteza=False):
        z = []
        for k in range(m):
            if not cont3[k]:
                z.append(z3[k])
                continue
            c = {'edad': ei.EDAD_TIPO.get(tipo3[k], 300.0)}
            h, s_ = ci['h'][k], ci['s'][k]
            if como_corteza:
                h, s_ = h + s_, 0.0
            zk = ei.altura_regla(ei.con_edad(p, c), h, s_, ei.l_de(p, c))
            z.append(zk if math.isfinite(zk) else z3[k])
        zeta = trm.nivel_del_mar(z, area_km2, vol)
        return tcr.medir(z, w3, zeta)
    relieves = {'tierra': tcr.medir(z3, w3, 0.0), 'R4_estudio': relieve(r4_todo), 'R4_sin_dinamica': relieve(r4_s0),
                'R4_sedimento_como_corteza': relieve(r4_cc, True)}

    # --- Salida ---
    print(f'§0: celdas de tierra con el residuo de la regla de Lamb ≤ 300 m: {100 * f_sin:.0f} % de la tierra '
          f'({len(sin_dinamica)} de {len(tierra)} celdas de 1°).')
    for nombre, r in res.items():
        e1, e2 = r['todo'], r['ajuste']
        borde = [k for k, v in r['en_borde'].items() if v]
        print(f"  {nombre:<26} error medio {e1['error_medio_m']:.0f} m (RMS {e1['rms_m']:.0f}) en toda la tierra; "
              f"{e2['error_medio_m']:.0f} (RMS {e2['rms_m']:.0f}) en las celdas del ajuste; en el borde: {borde}")
        print(f"     {r['parametros']}")
    print(f"  validación cruzada de R4 sin dinámica (prueba en toda la otra mitad): B {cv['prueba_B']['error_medio_m']:.0f} m, "
          f"A {cv['prueba_A']['error_medio_m']:.0f} m")
    print('§11.3, altura media de la tierra por clase de edad (real / R4 sin dinámica / R4 del estudio):')
    for x, y in zip(clases['R4_sin_dinamica'], clases['R4_estudio']):
        if x['area_frac'] > 0:
            print(f"  desde {x['desde_myr']:5.0f} Myr: {100 * x['area_frac']:5.1f} % de la tierra; real {x['altura_real_m']:5.0f} m; "
                  f"R4 {x['altura_regla_m']:5.0f}; R4 del estudio {y['altura_regla_m']:5.0f} (h {x['h_media_km']:.1f} km, s {x['s_media_km']:.1f})")
    print(f'§6.5, edades iniciales (la tierra de más de 500 Myr, el {100 * viejo:.0f} % de la de hoy, menos 500 Myr):')
    for i in range(len(hist)):
        print(f'  {bordes[i]:5.0f}–{bordes[i + 1]:5.0f} Myr: tipos {100 * dist_tipos[i]:5.1f} %, TC1 invertido {100 * dist_tc1[i]:5.1f} %')

    print('Relieve a 26 km con el océano real:')
    for clave in ('nivel_mar_m', 'altura_media_tierra_m', 'tierra_bajo_200_frac', 'tierra_sobre_2000_frac',
                  'pendiente_media_tierra', 'plataforma_km'):
        print(f'  {clave:<26}' + ''.join(f'{relieves[n][clave]:>14.4g}' for n in relieves) + '   ' + ' / '.join(relieves))
    secciones = [('s0', {'celdas_ajuste_frac_tierra': f_sin, 'residuo_lamb_max_m': 300.0})]
    for nombre, r in relieves.items():
        secciones.append((f'relieve.{nombre}', r))
    for nombre, r in res.items():
        secciones.append((f'{nombre}.todo', r['todo']))
        secciones.append((f'{nombre}.ajuste', r['ajuste']))
        secciones.append((f'{nombre}.parametros', r['parametros']))
        secciones.append((f'{nombre}.en_borde', r['en_borde']))
    secciones.append(('R4_sin_dinamica.validacion_cruzada', {f'{k}_error_medio_m': v['error_medio_m'] for k, v in cv.items()}))
    for nombre, filas in clases.items():
        for x in filas:
            secciones.append((f'por_edad.{nombre}.desde_{int(x["desde_myr"])}', x))
    secciones.append(('edades_iniciales', {'bordes_myr': bordes, 'frac_tipos': dist_tipos, 'frac_tc1_invertido': dist_tc1,
                                           'tierra_de_mas_de_500_frac': viejo}))
    cab = ('# Comprobación del §0 de la nota de isostasia con litosfera, y datos de la Tierra para implementarla.\n'
           '# Generado por herramientas/analisis/nota_isostasia_s0.py (los datos del estudio de isostasia).\n')
    rt.escribir_toml(a.salida, secciones, cab)
    log(f'escrito {a.salida}')


if __name__ == '__main__':
    main()
