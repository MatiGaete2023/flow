"""Prueba DOM con Chromium y formulario ficticio. No utiliza una sesión institucional."""
import json
import os
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[1]
SOURCE = (ROOT / 'tools/rus_capture_extension/capture.js').read_text(encoding='utf-8')
FIXTURE = '''<!doctype html><meta charset="utf-8"><title>Formulario ficticio RUS</title>
<form id="registro" method="post" action="/fixture-save?csrf=SECRETO_URL">
<input type="hidden" name="COD_Ingreso" value="20">
<input type="hidden" name="csrf" value="SECRETO_CSRF">
<input type="hidden" name="desconocido" value="SECRETO_OCULTO">
<input type="password" name="password" value="SECRETO_PASSWORD">
<input name="session" value="SECRETO_SESION">
<label for="texto">Observación</label><textarea name="texto" id="texto" maxlength="3000">Texto 😀\nÍntegro.</textarea>
<select name="tipo"><option value="1">Administrativa</option><option value="2" selected>Al Tribunal</option></select>
<input type="checkbox" name="tribunal" checked><button type="submit">Guardar</button></form>
<table id="entradas"><tr id="entry-31"><td>31</td><td>Texto completo 😀</td></tr></table>
<a href="/fixture-page?token=SECRETO_LINK">Siguiente</a>
<script>window.submits=0; document.forms[0].addEventListener('submit',e=>{e.preventDefault();window.submits++});</script>'''


def main():
    with sync_playwright() as p:
        browser = p.chromium.launch(executable_path=os.environ.get('RUS_TEST_CHROMIUM') or None, headless=True)
        try:
            page = browser.new_page()
            requests = []
            page.on('request', lambda request: requests.append(request.url))
            page.route('https://familia.pjud.cl/**', lambda route: route.fulfill(body=FIXTURE, content_type='text/html'))
            page.goto('https://familia.pjud.cl/fixture?session=SECRETO_URL')
            page.locator('#texto').fill('Texto actual editado 😀\nSegunda línea.')
            page.evaluate(SOURCE)
            before = page.locator('body').inner_html()
            count = len(requests)
            capture = page.evaluate('captureRusDocument()')
            assert page.locator('body').inner_html() == before and page.evaluate('window.submits') == 0
            assert len(requests) == count, 'La captura no debe iniciar solicitudes'
            encoded = json.dumps(capture, ensure_ascii=False)
            assert 'SECRETO_' not in encoded
            controls = {c['name']: c for c in capture['forms'][0]['controls']}
            assert controls['COD_Ingreso']['value'] == '20'
            assert controls['texto']['value'] == 'Texto actual editado 😀\nSegunda línea.'
            assert controls['texto']['max_length'] == '3000'
            assert controls['tipo']['options'][1]['selected'] is True
            assert controls['tribunal']['checked'] is True
            assert capture['tables'][0]['rows'][0]['cells'][1] == 'Texto completo 😀'
            assert capture['capabilities']['complete_entries'] is False
            page.locator('#texto').evaluate('(e)=>e.removeAttribute("maxlength")')
            assert page.evaluate('captureRusDocument()')['forms'][0]['controls'][5]['max_length'] is None
            page.route('https://otro.example/**', lambda route: route.fulfill(body=FIXTURE))
            page.goto('https://otro.example/fixture')
            page.evaluate(SOURCE)
            assert page.evaluate('captureRusDocument()')['skipped'] is True
            print('Chromium: texto vivo, UTF-16, opciones, destino, identidad, omisiones sensibles y ausencia de escritura comprobados en sitio ficticio.')
        finally:
            browser.close()


if __name__ == '__main__':
    main()
