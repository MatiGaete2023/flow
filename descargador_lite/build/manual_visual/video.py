"""Escenas y tarjetas (cuadros PIL) para los videos narrados: las usan narrado.py y tools/manuales/videos.py."""

from PIL import Image, ImageDraw, ImageFont

W, H, FPS = 1280, 720, 25
BAR = 112
BLUE, DARK, RED = (25, 95, 168), (32, 50, 77), (220, 38, 38)
BOLD = '/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf'
REG = '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'
MONO = '/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf'


def font(path, size):
    return ImageFont.truetype(path, size)


def ease(t):
    t = max(0.0, min(1.0, t))
    return t * t * (3 - 2 * t)


def wrap(draw, text, fnt, width):
    lines, line = [], ''
    for word in text.split():
        test = (line + ' ' + word).strip()
        if draw.textlength(test, font=fnt) <= width:
            line = test
        else:
            lines.append(line)
            line = word
    return lines + [line]


def caption(frame, step, total, text):
    d = ImageDraw.Draw(frame)
    d.rectangle([0, H - BAR, W, H], fill=DARK)
    if step:
        d.ellipse([28, H - BAR + 26, 88, H - BAR + 86], fill=BLUE)
        d.text((58, H - BAR + 56), str(step), font=font(BOLD, 28), fill='white', anchor='mm')
        d.text((W - 24, H - 18), f'Paso {step} de {total}', font=font(REG, 15), fill=(160, 180, 205), anchor='rs')
    f = font(BOLD, 25)
    lines = wrap(d, text, f, W - 160)
    y = H - BAR / 2 - (len(lines) * 32) / 2
    for line in lines:
        d.text((110, y), line, font=f, fill='white')
        y += 32


def cursor(frame, x, y, click=0.0):
    d = ImageDraw.Draw(frame)
    if click > 0:
        r = 10 + 30 * click
        d.ellipse([x - r, y - r, x + r, y + r], outline=(*RED, 255), width=4)
    pts = [(0, 0), (0, 26), (7, 20), (12, 31), (17, 29), (12, 18), (21, 18)]
    pts = [(x + px, y + py) for px, py in pts]
    d.polygon(pts, fill='black')
    inner = [(x + px * 0.8 + 1.5, y + py * 0.8 + 3) for px, py in [(0, 0), (0, 24), (7, 18), (12, 28), (16, 26), (11, 16), (19, 16)]]
    d.polygon(inner, fill='white')


def fit(img):
    """Escala la captura al área sobre la barra de subtítulos; devuelve imagen, escala y desplazamiento."""
    area_w, area_h = W, H - BAR
    s = min(area_w / img.width, area_h / img.height)
    im = img.resize((round(img.width * s), round(img.height * s)), Image.LANCZOS)
    return im, s, ((area_w - im.width) // 2, (area_h - im.height) // 2)


def scene(src, target, text, step, total, seconds=6.0, zoom=1.7):
    """Puntero hacia el objetivo, clic, recuadro y acercamiento suave. target = (x, y, w, h) en la captura.

    Usar capturas sin marcas (raw): el único recuadro es el del paso. El acercamiento se limita para que el
    recuadro completo quede a la vista y nunca supera 1,4.
    """
    img = Image.open(src).convert('RGB')
    base, s, (ox, oy) = fit(img)
    canvas = Image.new('RGB', (W, H - BAR), (243, 246, 250))
    canvas.paste(base, (ox, oy))
    n = int(seconds * FPS)
    if target:
        tx, ty, tw, th = target
        cx, cy = ox + (tx + tw / 2) * s, oy + (ty + th / 2) * s
        x0, y0, x1, y1 = ox + tx * s - 6, oy + ty * s - 6, ox + (tx + tw) * s + 6, oy + (ty + th) * s + 6
        ImageDraw.Draw(canvas).rounded_rectangle([x0, y0, x1, y1], radius=8, outline=RED, width=4)
        zoom = max(1.0, min(zoom, 1.4, W * 0.85 / (x1 - x0), (H - BAR) * 0.85 / (y1 - y0)))
    else:
        cx, cy = W / 2, (H - BAR) / 2
    start = (W * 0.55, (H - BAR) * 0.75)
    for i in range(n):
        t = i / FPS
        z = 1 + (zoom - 1) * ease((t - 2.2) / 1.6) if target else 1.0
        vw, vh = W / z, (H - BAR) / z
        left = min(max(cx - vw / 2, 0), W - vw)
        top = min(max(cy - vh / 2, 0), H - BAR - vh)
        view = canvas.crop((round(left), round(top), round(left + vw), round(top + vh))).resize((W, H - BAR), Image.LANCZOS)
        frame = Image.new('RGB', (W, H))
        frame.paste(view, (0, 0))
        if target:
            m = ease((t - 0.3) / 1.3)
            px, py = start[0] + (cx - start[0]) * m, start[1] + (cy - start[1]) * m
            sx, sy = (px - left) * z, (py - top) * z
            click = ease((t - 1.7) / 0.5) if 1.7 <= t <= 2.4 else 0
            cursor(frame, sx, sy, click)
        caption(frame, step, total, text)
        yield frame


def card(title, lines, seconds=4.0, mono=None):
    frame = Image.new('RGB', (W, H), (243, 246, 250))
    d = ImageDraw.Draw(frame)
    d.rectangle([0, 0, 14, H], fill=BLUE)
    d.text((80, 150), 'SITFA LITE 1.1.0 · DESCARGAS RUS', font=font(BOLD, 20), fill=BLUE)
    y = 200
    for line in wrap(d, title, font(BOLD, 46), W - 160):
        d.text((80, y), line, font=font(BOLD, 46), fill=DARK)
        y += 60
    y += 20
    for text in lines:
        for line in wrap(d, text, font(REG, 26), W - 160):
            d.text((80, y), line, font=font(REG, 26), fill=DARK)
            y += 40
    if mono:
        d.rounded_rectangle([70, y + 10, W - 70, y + 80], radius=10, fill=(226, 234, 244))
        d.text((90, y + 45), mono, font=font(MONO, 22), fill=DARK, anchor='lm')
    for _ in range(int(seconds * FPS)):
        yield frame
