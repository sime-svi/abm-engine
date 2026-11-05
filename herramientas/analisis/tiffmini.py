"""Lector mínimo de GeoTIFF (sin librerías): la primera imagen de una banda, guardada en tiras, sin
comprimir o con Deflate, con o sin predictor horizontal. Es lo que usan WorldClim 2.1 y CHELSA 2.1;
cualquier otra cosa (teselas, LZW, varias bandas…) da un error claro en vez de leer mal.

Lee un archivo que ya está en memoria o uno remoto por peticiones parciales (cabecera HTTP `Range`),
fila a fila, sin bajarlo entero. Los archivos son datos no fiables: solo se interpretan sus bytes.

Uso: herramientas/analisis/tiffmini.py ARCHIVO.tif | URL   (muestra la cabecera)
"""
import array
import itertools
import re
import struct
import sys
import urllib.request
import zlib

# Tipos de los campos de las etiquetas: formato de struct y tamaño en bytes.
TIPOS = {1: ('B', 1), 2: ('c', 1), 3: ('H', 2), 4: ('I', 4), 5: ('I', 8), 6: ('b', 1), 7: ('B', 1),
         8: ('h', 2), 9: ('i', 4), 10: ('i', 8), 11: ('f', 4), 12: ('d', 8), 16: ('Q', 8), 17: ('q', 8),
         18: ('Q', 8)}
# Formato de cada píxel según (SampleFormat, BitsPerSample): letra de `array`.
LETRAS = {(1, 8): 'B', (1, 16): 'H', (1, 32): 'I', (2, 8): 'b', (2, 16): 'h', (2, 32): 'i',
          (3, 32): 'f', (3, 64): 'd'}


def leer_memoria(datos):
    """Función de lectura sobre unos bytes ya cargados."""
    return lambda desde, n: datos[desde:desde + n]


def leer_remoto(url, intentos=5, espera=60):
    """Función de lectura por peticiones parciales a `url`."""
    def leer(desde, n):
        req = urllib.request.Request(url, headers={'Range': f'bytes={desde}-{desde + n - 1}'})
        for k in range(intentos):
            try:
                with urllib.request.urlopen(req, timeout=espera) as r:
                    b = r.read()
                if len(b) != n:
                    raise IOError(f'{len(b)} bytes en vez de {n} (¿el servidor ignora Range?)')
                return b
            except Exception as e:  # noqa: BLE001
                if k == intentos - 1:
                    raise
                print(f'  reintento {k + 1} de {url} [{desde}+{n}]: {e}', file=sys.stderr, flush=True)
    return leer


class GeoTiff:
    """Primera imagen (IFD) de un GeoTIFF. `leer(desde, n)` devuelve `n` bytes desde `desde`."""

    def __init__(self, leer):
        self.leer = leer
        cab = leer(0, 16)
        if cab[:2] not in (b'II', b'MM'):
            raise ValueError('no es un TIFF')
        self.e = '<' if cab[:2] == b'II' else '>'
        version = struct.unpack(self.e + 'H', cab[2:4])[0]
        if version == 42:
            self.grande, ifd = False, struct.unpack(self.e + 'I', cab[4:8])[0]
        elif version == 43:
            self.grande, ifd = True, struct.unpack(self.e + 'Q', cab[8:16])[0]
        else:
            raise ValueError(f'versión de TIFF desconocida: {version}')
        self.etiquetas = self._ifd(ifd)
        t = self.etiquetas
        self.ancho, self.alto = t[256][0], t[257][0]
        if t.get(277, (1,))[0] != 1:
            raise ValueError('solo una banda (SamplesPerPixel = 1)')
        if 322 in t or 324 in t:
            raise ValueError('imagen en teselas: no soportada (solo tiras)')
        self.compresion = t.get(259, (1,))[0]
        if self.compresion not in (1, 8, 32946):
            raise ValueError(f'compresión {self.compresion} no soportada (solo ninguna o Deflate)')
        self.predictor = t.get(317, (1,))[0]
        if self.predictor not in (1, 2):
            raise ValueError(f'predictor {self.predictor} no soportado (solo 1 o 2)')
        bits = t[258][0]
        formato = t.get(339, (1,))[0]
        if (formato, bits) not in LETRAS:
            raise ValueError(f'píxel de {bits} bits y formato {formato} no soportado')
        self.letra = LETRAS[(formato, bits)]
        self.bytes_pixel = bits // 8
        self.filas_por_tira = min(t.get(278, (self.alto,))[0], self.alto)
        self.tiras, self.cuentas = t[273], t[279]
        # Georreferencia: tamaño del píxel (33550) y punto de amarre (33922); la clave 1025
        # (GTRasterTypeGeoKey) dice si el amarre es la esquina (1, «área») o el centro del píxel (2).
        sx, sy = t[33550][0], t[33550][1]
        i0, j0, _, x0, y0, _ = t[33922][:6]
        claves = t.get(34735, ())
        tipo_raster = 1
        for k in range(4, len(claves), 4):
            if claves[k] == 1025 and claves[k + 1] == 0:
                tipo_raster = claves[k + 3]
        desfase = 0.5 if tipo_raster == 2 else 0.0
        self.paso_lon, self.paso_lat = sx, sy
        self.lon0 = x0 - (i0 + desfase) * sx   # borde oeste de la columna 0
        self.lat0 = y0 + (j0 + desfase) * sy   # borde norte de la fila 0
        nodata = t.get(42113)
        self.nodata = float(nodata) if nodata not in (None, '') else None
        meta = t.get(42112, '')
        self.escala = self._meta(meta, 'scale', 1.0)
        self.desplazamiento = self._meta(meta, 'offset', 0.0)
        self._cache = (None, None)

    @staticmethod
    def _meta(texto, papel, defecto):
        m = re.search(r'<Item[^>]*role="' + papel + r'"[^>]*>\s*([-+0-9.eE]+)\s*</Item>', texto)
        return float(m.group(1)) if m else defecto

    def _ifd(self, desde):
        e = self.e
        if self.grande:
            n = struct.unpack(e + 'Q', self.leer(desde, 8))[0]
            tam, p0 = 20, desde + 8
        else:
            n = struct.unpack(e + 'H', self.leer(desde, 2))[0]
            tam, p0 = 12, desde + 2
        b = self.leer(p0, n * tam)
        out = {}
        for k in range(n):
            q = k * tam
            if self.grande:
                tag, tipo, cuenta = struct.unpack(e + 'HHQ', b[q:q + 12])
                campo, cabe = b[q + 12:q + 20], 8
            else:
                tag, tipo, cuenta = struct.unpack(e + 'HHI', b[q:q + 8])
                campo, cabe = b[q + 8:q + 12], 4
            if tipo not in TIPOS:
                continue
            letra, tam_tipo = TIPOS[tipo]
            nb = tam_tipo * cuenta
            if nb <= cabe:
                datos = campo[:nb]
            else:
                datos = self.leer(struct.unpack(e + ('Q' if self.grande else 'I'), campo)[0], nb)
            if tipo == 2:
                out[tag] = datos.split(b'\0')[0].decode('latin-1')
            else:
                nv = cuenta * (2 if tipo in (5, 10) else 1)
                out[tag] = struct.unpack(f'{e}{nv}{letra}', datos)
        return out

    def rango_tira(self, r):
        """(desde, n) de los bytes de la tira que contiene la fila `r`."""
        s = r // self.filas_por_tira
        return self.tiras[s], self.cuentas[s]

    def fila(self, r, comprimida=None):
        """Valores crudos de la fila `r` (sin escala; los nodata tal cual). `comprimida`, si se da,
        son los bytes de su tira ya descargados (para bajar varias en paralelo)."""
        s = r // self.filas_por_tira
        if self._cache[0] != s:
            b = comprimida if comprimida is not None else self.leer(*self.rango_tira(r))
            if self.compresion != 1:
                b = zlib.decompress(b)
            n_filas = min(self.filas_por_tira, self.alto - s * self.filas_por_tira)
            v = array.array(self.letra)
            v.frombytes(b[:n_filas * self.ancho * self.bytes_pixel])
            if (sys.byteorder == 'little') != (self.e == '<'):
                v.byteswap()
            self._cache = (s, v)
        v = self._cache[1]
        k = (r - s * self.filas_por_tira) * self.ancho
        f = v[k:k + self.ancho]
        if self.predictor == 2:
            # Diferencias horizontales: cada valor es la suma acumulada de la fila, módulo 2^bits.
            mascara = (1 << (8 * self.bytes_pixel)) - 1
            if self.letra.isupper():
                f = array.array(self.letra, (x & mascara for x in itertools.accumulate(f)))
            else:
                medio = 1 << (8 * self.bytes_pixel - 1)
                f = array.array(self.letra, (((x & mascara) ^ medio) - medio for x in itertools.accumulate(f)))
        return f

    def centro(self, c, r):
        """(lon, lat) del centro del píxel de la columna `c` y la fila `r`, en grados."""
        return self.lon0 + (c + 0.5) * self.paso_lon, self.lat0 - (r + 0.5) * self.paso_lat


def main():
    if len(sys.argv) != 2:
        print(__doc__)
        sys.exit(2)
    ruta = sys.argv[1]
    if ruta.startswith(('http://', 'https://')):
        t = GeoTiff(leer_remoto(ruta))
    else:
        with open(ruta, 'rb') as f:
            t = GeoTiff(leer_memoria(f.read()))
    print(f'{t.ancho} × {t.alto} píxeles, tipo {t.letra}, compresión {t.compresion}, predictor {t.predictor}, '
          f'{t.filas_por_tira} filas por tira')
    print(f'oeste {t.lon0}, norte {t.lat0}, paso {t.paso_lon} × {t.paso_lat} grados; nodata {t.nodata}; '
          f'escala {t.escala}, desplazamiento {t.desplazamiento}')


if __name__ == '__main__':
    main()
