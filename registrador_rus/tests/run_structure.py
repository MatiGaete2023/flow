"""Prueba focalizada, DOM reconstruido con la firma de los ZIP; nunca RUS."""
import os
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = '''<form name="InformesPpalForm" method="post">
<table id="TablaInforme"><tr><td>Histórico ficticio</td></tr></table>
<select name="COD_Estado_Detalle_Nuevo"><option>a</option><option>b</option><option>c</option></select>
<span>Enviar observación al tribunal</span><input type="checkbox" id="demo-check">
<textarea></textarea><input type="text" readonly><button name="irAccion" type="submit">Nueva obs</button><button type="button">Cerrar</button>
<div hidden><input type="checkbox"><textarea name="GLS_Observacion" maxlength="2000"></textarea><button name="irAccion">Grabar</button></div>
<input type="hidden" name="COD_Tribunal"><input type="hidden" name="CRR_IdCausa"><input type="hidden" name="ID_Ingreso"><input type="hidden" name="COD_Etapa"><input type="hidden" name="TIP_Consulta">
</form>'''


def main():
    js = (ROOT/'extension'/'structure.js').read_text(encoding='utf-8')
    with sync_playwright() as p:
        options = {'executable_path': os.environ['REGISTRADOR_CHROMIUM']} if os.getenv('REGISTRADOR_CHROMIUM') else {}
        browser = p.chromium.launch(headless=True, timeout=10000, **options)
        try:
            page = browser.new_page(); page.route('**/*', lambda route: route.abort())
            count = 0
            def check(change='', error=None, **expected):
                nonlocal count
                page.set_content(FIXTURE); page.add_script_tag(content=js)
                page.evaluate('''() => {window.events=0; for(const e of ['submit','input','change','click']) document.addEventListener(e,()=>window.events++,true);}''')
                if change: page.evaluate(change)
                result = page.evaluate('''() => {try {return RegistradorStructuralCandidate.inspect(document)} catch(e){return {error:e.message}}}''')
                if error: assert result == {'error':error}, result
                else:
                    assert result['matches'] and not result['write_enabled'], result
                    for k,v in expected.items(): assert result[k] == v, result
                assert page.evaluate('window.events') == 0
                count += 1
            check(destination_label_associated=False, create_label_matches=True, editor_maxlength=None)
            check("document.querySelector('span').outerHTML='<label for=demo-check>Enviar observación al tribunal</label>'", destination_label_associated=True)
            check("document.querySelector('textarea').maxLength=2000", editor_maxlength=2000)
            check("document.querySelector('form').append(document.createElement('textarea'))", 'FIRMA_DISTINTA')
            check("document.querySelector('div[hidden]').hidden=false", 'FIRMA_DISTINTA')
            check("document.querySelector('textarea').readOnly=true", 'CONTROLES_AMBIGUOS')
            check("document.querySelector('input[name=ID_Ingreso]').remove()", 'CONTEXTO_ESTRUCTURAL_INCOMPLETO')
            check("document.querySelector('select').remove(0)", 'ESTADO_DISTINTO')
            check("document.querySelector('button[name=irAccion]').textContent='Grabar'", 'ACCION_CONTRADICTORIA')
            check("document.querySelector('button[name=irAccion]').name='otro'", 'CONTROLES_AMBIGUOS')
            check("document.querySelector('table').id='otra'", 'TABLA_NO_UNICA')
            check("document.querySelector('form').method='get'", 'METODO_INESPERADO')
            print(f'PASS {count} structural checks')
        finally: browser.close()

if __name__=='__main__': main()
