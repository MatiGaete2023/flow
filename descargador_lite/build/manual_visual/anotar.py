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
    """Anota las capturas del programa (app_boxes.json, en orden) y, si están, las de Chrome (boxes.json)."""
    src, out = Path(src), Path(out)
    out.mkdir(parents=True, exist_ok=True)
    app = json.loads((src / 'app_boxes.json').read_text())
    for name, boxes in app.items():
        mark(Image.open(src / 'raw' / name), list(boxes.values())).save(out / name, optimize=True)
    if not (src / 'boxes.json').is_file():
        return  # Las capturas de Chrome no cambiaron: se conservan las anotadas en docs/img.
    web = json.loads((src / 'boxes.json').read_text())
    b = lambda k: [web[k]['x'], web[k]['y'], web[k]['width'], web[k]['height']]  # noqa: E731
    plan = {
        'r01_extensiones.png': [b('dev')],
        'r02_modo_desarrollador.png': [b('load')],
        'r03_extension_cargada.png': [b('card')],
        'r04_menu_extensiones.png': [[1150, 46, 32, 32], [1086, 212, 24, 24]],
        'r05_fijada.png': [[1116, 49, 28, 28]],
        'r06_conexion.png': [],
        'r07_codigo_pegado.png': [b('code'), b('connect')],
    }
    for name, boxes in plan.items():
        mark(Image.open(src / 'raw' / name), boxes).save(out / name, optimize=True)


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])
