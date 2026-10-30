"""Anota capturas (recuadros y números) para manuales y video. Uso: anotar.py <carpeta_capturas> <salida_img>."""

import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

RED = (220, 38, 38)
FONT = '/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf'


def font(size):
    try:
        return ImageFont.truetype(FONT, size)
    except OSError:
        return ImageFont.load_default()


def mark(image, boxes, pad=6):
    im = image.convert('RGB')
    d = ImageDraw.Draw(im)
    for n, (x, y, w, h) in enumerate(boxes, 1):
        d.rounded_rectangle([x - pad, y - pad, x + w + pad, y + h + pad], radius=8, outline=RED, width=4)
        r = 15
        cx, cy = max(r + 2, x - pad), max(r + 2, y - pad)
        d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=RED)
        d.text((cx, cy), str(n), font=font(18), fill='white', anchor='mm')
    return im


def main(src, out):
    src, out = Path(src), Path(out)
    out.mkdir(parents=True, exist_ok=True)
    web = json.loads((src / 'boxes.json').read_text())
    app = json.loads((src / 'app_boxes.json').read_text())
    b = lambda k: [web[k]['x'], web[k]['y'], web[k]['width'], web[k]['height']]  # noqa: E731
    def a(shot, *keys):
        # Los controles dentro del área desplazable se recortan a lo visible.
        cx, cy, cw, ch = app[shot]['_canvas']
        result = []
        for k in keys:
            x, y, w, h = app[shot][k]
            if k not in ('descargar', 'lista', 'copiar', 'abrir', 'instalar', 'estado', 'radios', 'cargar'):
                top, bottom = max(y, cy + 4), min(y + h, cy + ch - 8)
                y, h = top, max(0, bottom - top)
            result.append([x, y, w, h])
        return result
    plan = {
        'r01_extensiones.png': [b('dev')],
        'r02_modo_desarrollador.png': [b('load')],
        'r03_extension_cargada.png': [b('card')],
        'r04_menu_extensiones.png': [[1150, 46, 32, 32], [1086, 212, 24, 24]],
        'r05_fijada.png': [[1116, 49, 28, 28]],
        'r06_conexion.png': [],
        'r07_codigo_pegado.png': [b('code'), b('connect')],
        'a0_inicio.png': a('a0_inicio.png', 'abrir', 'instalar', 'copiar'),
        'a1_conectado.png': a('a1_conectado.png', 'estado', 'radios', 'cargar'),
        'a2_opciones_seguimiento.png': a('a2_opciones_seguimiento.png', 'tribunales', 'pestanas', 'modalidades'),
        'a3_informes_medidas_fechas.png': a('a3_informes_medidas_fechas.png', 'informes', 'medidas', 'fechas'),
        'a4_descarga_terminada.png': a('a4_descarga_terminada.png', 'descargar', 'lista'),
        'a5_busqueda_lit.png': a('a5_busqueda_lit.png', 'tribunales', 'estados'),
        'a6_medidas_por_vencer.png': a('a6_medidas_por_vencer.png', 'meses', 'anio'),
    }
    for name, boxes in plan.items():
        mark(Image.open(src / 'raw' / name), boxes).save(out / name, optimize=True)


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])
