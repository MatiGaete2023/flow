"""Capturas de Chrome para los manuales de SITFA Lite, tomadas en Windows (como las ve el usuario).

    python tools/manuales/capturas_chrome_win.py <carpeta extension> <salida>

Escribe <salida>/raw/r01…r07.png (ventana completa de Chromium, en español) y <salida>/boxes.json con los
recuadros relativos a la imagen. Solo páginas locales (chrome://extensions y la pestaña de conexión de la extensión con
un código ficticio): no abre SITFA ni usa datos reales. Lo corre el workflow manuales-windows.yml.
"""

import ctypes
import json
from pathlib import Path
import sys
import tempfile
import time

from PIL import ImageGrab
from playwright.sync_api import sync_playwright

CODIGO = '51234-0123456789abcdef0123456789abcdef'
ARGS = [
    '--window-position=40,40',
    '--window-size=1280,760',
    '--lang=es-419',
    '--no-first-run',
    '--no-default-browser-check',
    '--hide-crash-restore-bubble',
]


class Ventana:
    """Captura la ventana de Chromium y traduce recuadros de la página a coordenadas de la imagen."""

    def __init__(self, page, salida):
        self.page, self.salida, self.boxes = page, Path(salida), {}
        (self.salida / 'raw').mkdir(parents=True, exist_ok=True)

    def medidas(self):
        sx, sy, ow, oh, iw, ih = self.page.evaluate(
            '[screenX, screenY, outerWidth, outerHeight, innerWidth, innerHeight]'
        )
        borde = max(0, (ow - iw) // 2)  # bordes invisibles de Windows a izquierda, derecha y abajo
        return sx + borde, sy, ow - 2 * borde, oh - borde, oh - ih - borde

    def foto(self, nombre):
        time.sleep(0.8)
        x, y, w, h, _ = self.medidas()
        ImageGrab.grab(bbox=(x, y, x + w, y + h), all_screens=True).save(self.salida / 'raw' / nombre)

    def caja(self, selector, clave):
        b = self.page.locator(selector).first.bounding_box()
        top = self.medidas()[4]
        self.boxes[clave] = {'x': b['x'], 'y': b['y'] + top, 'width': b['width'], 'height': b['height']}

    def clic_pantalla(self, x, y):
        """Clic real en la barra de Chrome (fuera de la página): rompecabezas y alfiler."""
        user32 = ctypes.windll.user32
        user32.SetCursorPos(int(x), int(y))
        user32.mouse_event(2, 0, 0, 0, 0)
        user32.mouse_event(4, 0, 0, 0, 0)


def main(extension, salida):
    extension, salida = Path(extension).resolve(), Path(salida)
    with sync_playwright() as pw:
        perfil = tempfile.mkdtemp()
        ctx = pw.chromium.launch_persistent_context(perfil, headless=False, no_viewport=True, args=ARGS)
        page = ctx.pages[0]
        v = Ventana(page, salida)
        page.goto('chrome://extensions/')
        page.wait_for_timeout(1500)
        v.caja('#devMode', 'dev')
        v.foto('r01_extensiones.png')
        page.locator('#devMode').click()
        page.wait_for_timeout(900)
        v.caja('#loadUnpacked', 'load')
        v.foto('r02_modo_desarrollador.png')
        ctx.close()

        perfil = tempfile.mkdtemp()
        args = [*ARGS, f'--disable-extensions-except={extension}', f'--load-extension={extension}']
        ctx = pw.chromium.launch_persistent_context(perfil, headless=False, no_viewport=True, args=args)
        page = ctx.pages[0]
        v.page = page
        page.goto('chrome://extensions/')
        page.wait_for_timeout(1500)
        if not page.locator('#loadUnpacked').is_visible():
            page.locator('#devMode').click()
            page.wait_for_timeout(800)
        v.caja('extensions-item', 'card')
        v.foto('r03_extension_cargada.png')
        # Rompecabezas (a la derecha de la barra) y alfiler del menú de extensiones.
        x, y, w, _, top = v.medidas()
        puzzle = (w - 114, top - 25)
        v.clic_pantalla(x + puzzle[0], y + puzzle[1])
        page.wait_for_timeout(1200)
        pin = (puzzle[0] - 80, puzzle[1] + 150)
        v.boxes['r04'] = [[puzzle[0] - 16, puzzle[1] - 16, 32, 32], [pin[0] - 12, pin[1] - 12, 24, 24]]
        v.foto('r04_menu_extensiones.png')
        v.clic_pantalla(x + pin[0], y + pin[1])
        page.wait_for_timeout(800)
        page.keyboard.press('Escape')
        page.wait_for_timeout(600)
        v.boxes['r05'] = [[puzzle[0] - 64, puzzle[1] - 14, 28, 28]]
        v.foto('r05_fijada.png')
        worker = ctx.service_workers[0] if ctx.service_workers else ctx.wait_for_event('serviceworker')
        ident = worker.url.split('/')[2]
        page.goto(f'chrome-extension://{ident}/conexion.html?tab=1')
        page.wait_for_timeout(900)
        v.foto('r06_conexion.png')
        page.fill('#code', CODIGO)
        page.wait_for_timeout(300)
        v.caja('#code', 'code')
        v.caja('#connect', 'connect')
        v.foto('r07_codigo_pegado.png')
        ctx.close()
    (salida / 'boxes.json').write_text(json.dumps(v.boxes, indent=1), encoding='utf-8')


if __name__ == '__main__':
    main(*sys.argv[1:3])
