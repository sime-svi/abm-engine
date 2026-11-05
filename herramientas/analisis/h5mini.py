"""Lector mínimo de HDF5 (superbloque v2/v3, cabeceras de objeto v2, enlaces compactos, datos contiguos)."""
import struct, sys

def u(b, p, n):
    return int.from_bytes(b[p:p+n], 'little')

def parse_ohdr(b, addr, fetch):
    """Devuelve la lista de mensajes (tipo, cuerpo) de la cabecera de objeto v2 en `addr`."""
    msgs = []
    blocks = [(addr, None)]
    while blocks:
        a, ln = blocks.pop(0)
        if ln is None:
            hb = fetch(a, 4096)
            assert hb[:4] == b'OHDR', ('no OHDR', a, hb[:4])
            ver, flags = hb[4], hb[5]
            p = 6
            if flags & 0x20: p += 16
            if flags & 0x10: p += 4
            nsz = 1 << (flags & 3)
            size0 = u(hb, p, nsz); p += nsz
            hb = fetch(a, p + size0 + 4)
            end = p + size0
        else:
            hb = fetch(a, ln)
            assert hb[:4] == b'OCHK', ('no OCHK', a)
            flags_ = flags
            p = 4
            end = ln - 4
        while p + 4 <= end:
            t = hb[p]; sz = u(hb, p+1, 2); fl = hb[p+3]; p += 4
            if flags & 0x04: p += 2
            body = hb[p:p+sz]; p += sz
            if t == 0x10:  # continuación
                ca = u(body, 0, 8); cl = u(body, 8, 8)
                blocks.append((ca, cl))
            msgs.append((t, body))
    return msgs

def links(msgs):
    out = {}
    for t, body in msgs:
        if t != 0x06: continue
        ver, fl = body[0], body[1]; p = 2
        ltype = 0
        if fl & 0x08: ltype = body[p]; p += 1
        if fl & 0x04: p += 8
        if fl & 0x10: p += 1
        nl = 1 << (fl & 3)
        ln = u(body, p, nl); p += nl
        name = body[p:p+ln].decode(); p += ln
        if ltype == 0:
            out[name] = u(body, p, 8)
    return out

def dataset_info(msgs):
    info = {}
    for t, body in msgs:
        if t == 0x01:  # dataspace
            ver = body[0]; nd = body[1]; fl = body[2]
            p = 4 if ver == 2 else 8
            info['dims'] = [u(body, p + 8*i, 8) for i in range(nd)]
        elif t == 0x03:  # datatype
            cls = body[0] & 0x0F; size = u(body, 4, 4)
            info['dtype'] = (cls, size, body[1])
        elif t == 0x08:  # layout
            ver, lc = body[0], body[1]
            if lc == 1:
                info['layout'] = ('contiguous', u(body, 2, 8), u(body, 10, 8))
            else:
                info['layout'] = ('class', lc)
        elif t == 0x0B:  # filter pipeline
            info['filters'] = True
    return info

def open_remote(url):
    import urllib.request
    cache = {}
    def fetch(a, n):
        req = urllib.request.Request(url, headers={'Range': f'bytes={a}-{a+n-1}'})
        with urllib.request.urlopen(req, timeout=60) as r:
            return r.read()
    return fetch

if __name__ == '__main__':
    url = sys.argv[1]
    fetch = open_remote(url)
    sb = fetch(0, 64)
    assert sb[:8] == b'\x89HDF\r\n\x1a\n'
    ver = sb[8]
    root = u(sb, 36, 8) if ver in (2, 3) else None
    print('superbloque v', ver, 'raíz', root)
    rm = parse_ohdr(None, root, fetch)
    ls = links(rm)
    for name, a in ls.items():
        di = dataset_info(parse_ohdr(None, a, fetch))
        print(name, a, di)
