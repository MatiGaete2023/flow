"""Video tutorial (MP4, sin audio, con subtítulos) de la instalación de la extensión.

Uso: video.py <carpeta_capturas> <docs/img> <salida.mp4>
"""

import json
import subprocess
import sys
from pathlib import Path

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
    """Puntero hacia el objetivo, clic y acercamiento suave. target = (x, y, w, h) en la captura."""
    img = Image.open(src).convert('RGB')
    base, s, (ox, oy) = fit(img)
    canvas = Image.new('RGB', (W, H - BAR), (243, 246, 250))
    canvas.paste(base, (ox, oy))
    n = int(seconds * FPS)
    if target:
        tx, ty, tw, th = target
        cx, cy = ox + (tx + tw / 2) * s, oy + (ty + th / 2) * s
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
    d.text((80, 150), 'SITFA LITE 1.0.0 · SOLO DESCARGAS RUS', font=font(BOLD, 20), fill=BLUE)
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


def main(src, img, out):
    src, img = Path(src), Path(img)
    web = json.loads((src / 'boxes.json').read_text())
    app = json.loads((src / 'app_boxes.json').read_text())
    b = lambda k: (web[k]['x'], web[k]['y'], web[k]['width'], web[k]['height'])  # noqa: E731
    a = lambda shot, k: tuple(app[shot][k])  # noqa: E731
    total = 9
    parts = [
        card('Cómo instalar la extensión de Chrome', ['Una sola vez · 2 minutos · sin administrador',
                                                      'Antes: instala SITFA Lite con Instalar_SITFA_Lite_1.0.0.exe']),
        scene(img / 'a0_inicio.png', a('a0_inicio.png', 'instalar'),
              'Abre SITFA Lite y pulsa «Instalar extensión…». Copia la ruta de la carpeta y abre chrome://extensions.', 1, total),
        scene(img / 'r01_extensiones.png', b('dev'), 'En chrome://extensions activa «Modo Desarrollador» (arriba a la derecha).', 2, total),
        scene(img / 'r02_modo_desarrollador.png', b('load'), 'Pulsa «Cargar descomprimida».', 3, total),
        card('Paso 4 · Elige la carpeta extension', ['En la ventana de carpetas pega la ruta con Ctrl+V',
                                                     'en la barra de dirección y pulsa «Seleccionar carpeta»:'], 6.0,
             mono=r'C:\Users\<tu usuario>\AppData\Local\Programs\SITFA_Lite\extension'),
        scene(img / 'r03_extension_cargada.png', b('card'),
              'Aparece la tarjeta «SITFA Lite · Descargas RUS 1.0.0», activada. No muevas la carpeta extension.', 5, total, zoom=1.5),
        scene(img / 'r04_menu_extensiones.png', (1086, 212, 24, 24),
              'Pulsa el rompecabezas de Chrome y luego el alfiler junto a SITFA Lite para fijar el ícono.', 6, total),
        scene(img / 'r05_fijada.png', (1116, 49, 28, 28), 'El ícono azul queda fijo en la barra.', 7, total, seconds=4.5),
        scene(img / 'a0_inicio.png', a('a0_inicio.png', 'copiar'),
              'Abre SITFA en Chrome, inicia sesión y en SITFA Lite pulsa «Copiar código».', 8, total),
        scene(img / 'r07_codigo_pegado.png', b('connect'),
              'Con la pestaña de SITFA activa pulsa el ícono SITFA Lite, pega el código y pulsa «Conectar».', 9, total),
        scene(img / 'a1_conectado.png', a('a1_conectado.png', 'estado'),
              'Listo: SITFA Lite muestra «● Conectado». Sigue con el Manual de uso.', 0, total, seconds=5.0),
        card('Extensión instalada', ['Mantén abiertas la pestaña de SITFA y la de conexión mientras descargas.',
                                     'Manual de uso: menú Inicio → SITFA Lite → Manual de uso.'], 4.0),
    ]
    encode(parts, out)


def encode(parts, out):
    ff = subprocess.Popen(
        ['ffmpeg', '-y', '-loglevel', 'error', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-s', f'{W}x{H}', '-r', str(FPS),
         '-i', '-', '-c:v', 'libx264', '-preset', 'medium', '-crf', '23', '-pix_fmt', 'yuv420p', '-movflags', '+faststart',
         str(out)],
        stdin=subprocess.PIPE,
    )
    for part in parts:
        for frame in part:
            ff.stdin.write(frame.tobytes())
    ff.stdin.close()
    if ff.wait() != 0:
        raise SystemExit('ffmpeg falló')


def complete(src, img, out):
    """Video completo: instalar el programa, cargar la extensión, conectar y usar las 4 consultas."""
    src, img = Path(src), Path(img)
    web = json.loads((src / 'boxes.json').read_text())
    app = json.loads((src / 'app_boxes.json').read_text())
    b = lambda k: (web[k]['x'], web[k]['y'], web[k]['width'], web[k]['height'])  # noqa: E731
    a = lambda shot, k: tuple(app[shot][k])  # noqa: E731
    parts = [
        card('SITFA Lite · Instalación y uso', [
            'Descarga los XLS que produce RUS, con todas sus páginas:',
            '• Seguimiento · Informe seguimiento   • Seguimiento · Búsqueda litigantes',
            '• Informes por vencer   • Medidas por vencer',
            'Pantallazo PDF cuando una consulta no tiene registros.'], 6.0),
        # Parte 1: instalar el programa (ventanas estándar de Windows; sin capturas)
        card('Parte 1 · Instalar el programa', [
            '1. Doble clic en Instalar_SITFA_Lite_1.0.0.exe.',
            '2. Si aparece «Windows protegió su PC»: Más información → Ejecutar de todas formas.',
            '3. Siguiente → deja la carpeta propuesta → Instalar (no pide administrador).',
            '4. Al terminar deja marcadas «Ver el video…» y «Abrir SITFA Lite».'], 9.0,
            mono=r'C:\Users\<tu usuario>\AppData\Local\Programs\SITFA_Lite'),
        card('Parte 2 · Cargar la extensión de Chrome', ['Una sola vez. La extensión consulta SITFA con tu propia sesión;',
                                                         'el programa no ve tu contraseña ni tus cookies.'], 4.0),
        scene(img / 'a0_inicio.png', a('a0_inicio.png', 'instalar'),
              'En SITFA Lite pulsa «Instalar extensión…»: copia la ruta de la carpeta y abre chrome://extensions.', 1, 7),
        scene(img / 'r01_extensiones.png', b('dev'), 'Activa «Modo Desarrollador» (arriba a la derecha).', 2, 7),
        scene(img / 'r02_modo_desarrollador.png', b('load'), 'Pulsa «Cargar descomprimida».', 3, 7),
        card('Paso 4 · Elige la carpeta extension', ['En la ventana de carpetas pega la ruta con Ctrl+V',
                                                     'en la barra de dirección y pulsa «Seleccionar carpeta»:'], 6.0,
             mono=r'C:\Users\<tu usuario>\AppData\Local\Programs\SITFA_Lite\extension'),
        scene(img / 'r03_extension_cargada.png', b('card'),
              'Aparece la tarjeta «SITFA Lite · Descargas RUS 1.0.0», activada. No muevas la carpeta extension.', 5, 7, zoom=1.5),
        scene(img / 'r04_menu_extensiones.png', (1086, 212, 24, 24),
              'Rompecabezas de Chrome → alfiler junto a SITFA Lite para fijar el ícono.', 6, 7),
        scene(img / 'r05_fijada.png', (1116, 49, 28, 28), 'El ícono azul queda fijo en la barra.', 7, 7, seconds=4.5),
        card('Parte 3 · Conectar', ['Cada vez que abras SITFA Lite.'], 3.0),
        scene(img / 'a0_inicio.png', a('a0_inicio.png', 'abrir'),
              'Pulsa «Abrir SITFA en Chrome» e inicia sesión como siempre.', 1, 4),
        scene(img / 'a0_inicio.png', a('a0_inicio.png', 'copiar'), 'Pulsa «Copiar código».', 2, 4, seconds=4.5),
        scene(img / 'r07_codigo_pegado.png', b('connect'),
              'En la pestaña de SITFA pulsa el ícono SITFA Lite, pega el código y pulsa «Conectar». Deja esa pestaña abierta.', 3, 4),
        scene(img / 'a1_conectado.png', a('a1_conectado.png', 'estado'), 'SITFA Lite muestra «● Conectado».', 4, 4, seconds=4.5),
        card('Parte 4 · Descargar', ['Elegir consulta → Cargar opciones → marcar filtros → Descargar.'], 3.5),
        scene(img / 'a1_conectado.png', a('a1_conectado.png', 'radios'),
              'Marca una de las 4 consultas.', 1, 8, seconds=4.5, zoom=1.4),
        scene(img / 'a1_conectado.png', a('a1_conectado.png', 'cargar'),
              'Pulsa «Cargar opciones»: SITFA abre esa pantalla y el programa trae sus tribunales y filtros.', 2, 8),
        scene(img / 'a2_opciones_seguimiento.png', a('a2_opciones_seguimiento.png', 'tribunales'),
              'Seguimiento: marca tribunales (clic). «Buscar» filtra la lista; «Todos/Ninguno» marca lo visible.', 3, 8, zoom=1.3),
        scene(img / 'a2_opciones_seguimiento.png', a('a2_opciones_seguimiento.png', 'pestanas'),
              'Pestañas: Espera, Cumplimiento, Informes, Egresados. Modalidades: Residencia, Ambulatorio, FAE, DCE.', 4, 8),
        scene(img / 'a3_informes_medidas_fechas.png', a('a3_informes_medidas_fechas.png', 'informes'),
              'Pestaña Informes: elige el tipo de informe. Pestaña Cumplimiento: elige el filtro de medidas.', 5, 8),
        scene(img / 'a3_informes_medidas_fechas.png', a('a3_informes_medidas_fechas.png', 'fechas'),
              'Filtro de fecha opcional (Desde/Hasta o atajos). «Conservar los otros filtros» usa lo preparado en SITFA.', 6, 8, zoom=1.5),
        scene(img / 'a4_descarga_terminada.png', a('a4_descarga_terminada.png', 'descargar'),
              'Pulsa «Descargar». No uses la pestaña de SITFA mientras dice «Descargando el lote».', 7, 8),
        scene(img / 'a4_descarga_terminada.png', a('a4_descarga_terminada.png', 'lista'),
              'Cada archivo aparece en la lista con sus registros. Doble clic lo abre; «Abrir carpeta del lote» abre la carpeta.', 8, 8, zoom=1.4),
        scene(img / 'a5_busqueda_lit.png', a('a5_busqueda_lit.png', 'estados'),
              'Búsqueda litigantes: tribunales + uno o varios estados (+ fechas opcionales).', 0, 0, zoom=1.5),
        scene(img / 'a6_medidas_por_vencer.png', a('a6_medidas_por_vencer.png', 'meses'),
              'Informes / Medidas por vencer: tribunales + mes(es) + año. No usan rango de fechas.', 0, 0, zoom=1.5),
        card('Nombres de los archivos', [
            'Subcarpeta por lote con fecha y hora, en Documentos\\SITFA Lite.',
            'Seguimiento - Cumplimiento - Ambulatorio - Vencidos - Jdo Familia X',
            '      - 01-10-2026 al 30-10-2026 - p2 de 3.xls',
            'Busqueda Lit - Pendientes - Jdo Familia X.xls',
            'Medidas por vencer - Octubre 2026 - Jdo Familia X.xls',
            'Sin registros → … - SIN REGISTROS.pdf (pantallazo de SITFA)'], 10.0),
        card('Consejos', [
            '• Deja Chrome visible (no minimizado): el pantallazo PDF lo necesita.',
            '• Cancelar detiene al terminar la página en curso; lo guardado se conserva.',
            '• Si dice «Sesión expirada» o «Sin conexión»: entra de nuevo a SITFA,',
            '   pulsa el ícono SITFA Lite y pega el código actual.',
            '• Manuales PDF: menú Inicio → SITFA Lite.'], 9.0),
    ]
    encode(parts, out)


if __name__ == '__main__':
    if sys.argv[1] == '--completo':
        complete(*sys.argv[2:5])
    else:
        main(*sys.argv[1:4])
