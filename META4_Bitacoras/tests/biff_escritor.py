"""Escritor BIFF8 (.xls OLE2) mínimo para pruebas: celdas de texto y número, y fecha con formato 14.

xlwt no está disponible; arma el archivo compuesto a mano, con lo justo para xlrd 2.x.
"""
import struct

ENDOFCHAIN, FREESECT, FATSECT, NOSTREAM = 0xFFFFFFFE, 0xFFFFFFFF, 0xFFFFFFFD, 0xFFFFFFFF


def rec(rc, data):
    return struct.pack('<HH', rc, len(data)) + data


def bof(dt):
    return rec(0x0809, struct.pack('<HHHHII', 0x0600, dt, 0x0DBB, 0x07CC, 0, 0x06))


def ustr16(s):
    raw = s.encode('utf-16-le')
    return struct.pack('<HB', len(s), 1) + raw


def ustr8(s):
    raw = s.encode('utf-16-le')
    return struct.pack('<BB', len(s), 1) + raw


def font(name='Arial'):
    return rec(0x0031, struct.pack('<HHHHHBBBB', 200, 0, 0x7FFF, 400, 0, 0, 0, 0, 0) + ustr8(name))


def xf(font_idx, fmt_idx, style=False):
    typ = 0xFFF5 if style else 0x0001
    return rec(0x00E0, struct.pack('<HHHBBBBIIH', font_idx, fmt_idx, typ, 0x20, 0, 0, 0, 0, 0, 0x20C0))


def workbook_stream(sheets):
    """sheets: list of (name, rows) where rows is list of lists of str|float|('date', serial)|None."""
    glob = [bof(0x0005), rec(0x0042, struct.pack('<H', 1200)), rec(0x0022, struct.pack('<H', 0))]
    glob += [font() for _ in range(5)]
    # 16 XFs (15 style + 1 cell) like Excel, plus XF16 = date (fmt 14), XF17 = general cell
    glob += [xf(0, 0, style=True) for _ in range(15)] + [xf(0, 0), xf(0, 14), xf(0, 0)]
    sheet_streams = []
    for name, rows in sheets:
        body = [bof(0x0010)]
        nrows = len(rows)
        ncols = max((len(r) for r in rows), default=0)
        body.append(rec(0x0200, struct.pack('<IIHHH', 0, nrows, 0, ncols, 0)))
        for r, row in enumerate(rows):
            for c, v in enumerate(row):
                if v is None:
                    continue
                if isinstance(v, tuple) and v[0] == 'date':
                    body.append(rec(0x0203, struct.pack('<HHHd', r, c, 16, float(v[1]))))
                elif isinstance(v, (int, float)):
                    body.append(rec(0x0203, struct.pack('<HHHd', r, c, 15, float(v))))
                else:
                    body.append(rec(0x0204, struct.pack('<HHH', r, c, 15) + ustr16(v)))
        body.append(rec(0x000A, b''))
        sheet_streams.append(b''.join(body))
    # BOUNDSHEET offsets need total globals length first
    bs_len = sum(4 + 6 + 2 + 2 * len(name) for name, _ in sheets)
    glob_len = sum(map(len, glob)) + bs_len + 4  # + EOF
    offsets, pos = [], glob_len
    for s in sheet_streams:
        offsets.append(pos)
        pos += len(s)
    bss = [rec(0x0085, struct.pack('<IBB', off, 0, 0) + ustr8(name)) for off, (name, _) in zip(offsets, sheets)]
    stream = b''.join(glob) + b''.join(bss) + rec(0x000A, b'') + b''.join(sheet_streams)
    assert len(b''.join(glob) + b''.join(bss)) + 4 == glob_len
    return stream


def cfb(stream, name='Workbook'):
    if len(stream) < 4096:
        stream = stream + b'\x00' * (4096 - len(stream))  # avoid mini-stream
    nsec = (len(stream) + 511) // 512
    data = stream + b'\x00' * (nsec * 512 - len(stream))
    entries_per_fat = 128
    # sectors: 0 = FAT, 1 = directory, 2.. = stream
    assert 2 + nsec <= entries_per_fat
    fat = [FATSECT, ENDOFCHAIN] + [3 + i for i in range(nsec - 1)] + [ENDOFCHAIN]
    fat += [FREESECT] * (entries_per_fat - len(fat))
    fat_sec = struct.pack('<128I', *fat)

    def direntry(nm, typ, child, start, size):
        if nm:
            raw = (nm + '\x00').encode('utf-16-le')
        else:
            raw = b''
        return (raw.ljust(64, b'\x00') + struct.pack('<HBB', len(raw), typ, 1)
                + struct.pack('<III', NOSTREAM, NOSTREAM, child) + b'\x00' * 16 + struct.pack('<I', 0)
                + b'\x00' * 16 + struct.pack('<IQ', start, size))

    d = direntry('Root Entry', 5, 1, ENDOFCHAIN, 0) + direntry(name, 2, NOSTREAM, 2, len(stream))
    d += direntry('', 0, NOSTREAM, 0, 0) * 2
    assert len(d) == 512
    header = (bytes.fromhex('d0cf11e0a1b11ae1') + b'\x00' * 16 + struct.pack('<HHHHH', 0x3E, 3, 0xFFFE, 9, 6)
              + b'\x00' * 6 + struct.pack('<IIIIIIIII', 0, 1, 1, 0, 4096, ENDOFCHAIN, 0, ENDOFCHAIN, 0)
              + struct.pack('<109I', 0, *([FREESECT] * 108)))
    assert len(header) == 512
    return header + fat_sec + d + data


def xls(sheets):
    return cfb(workbook_stream(sheets))
