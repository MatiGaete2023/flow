"""P3: Chromium ficticio; todas las solicitudes interceptadas, sin RUS."""
import os
from pathlib import Path
from hashlib import sha256
from playwright.sync_api import sync_playwright
from registrador_rus.app.contracts import RegistroFlowV1

ROOT = Path(__file__).resolve().parents[1]
HTML = '''<!doctype html><html data-fixture="P3"><body data-menu="89" data-origin="ESPERA">
<table data-current="true"><tbody>
<tr><td><a href="javascript:ShowObservaciones(101,202,303,11,1,0)">Otro ingreso</a></td></tr>
<tr><td><a href="javascript:ShowObservaciones(101,202,304,11,1,0)">Ingreso exacto</a></td></tr>
</tbody></table>
<form name="InformesPpalForm" data-system="SITFA" data-tribunal="202" data-causa="101" data-ingreso="304">
<textarea maxlength="2000"></textarea><label for="check">Enviar observación al tribunal</label><input id="check" type="checkbox" checked>
<select name="COD_Estado_Detalle_Nuevo"><option value="demo">Estado demo</option><option value="otro">Otro</option></select>
<button type="submit">Nueva obs</button></form></body></html>'''
TEXT = ' Constancia ficticia.\nUnicode: á y 😀. '
OP = dict(schema='RegistroFlowV1', operation_id='demo', source=dict(tool='FLOW', work_id='demo', row_ref='demo'),
          remote_identity=dict(system='SITFA', tribunal_id='202', causa_id='101', ingreso_id='304'),
          origin='ESPERA', destination='ADMINISTRATIVA', observation_state='otro', approved_text=TEXT,
          approved_text_sha256=sha256(TEXT.encode()).hexdigest(), approved_by='demo',
          approved_at_utc='2026-11-24T12:00:00Z', approval_version=1)

RegistroFlowV1.from_mapping(OP)
with sync_playwright() as p:
    options = {'executable_path': os.environ['REGISTRADOR_CHROMIUM']} if os.getenv('REGISTRADOR_CHROMIUM') else {}
    browser = p.chromium.launch(headless=True, timeout=10000, **options)
    try:
        page = browser.new_page()
        page.route('**/*', lambda route: route.abort())
        page.set_content(HTML, timeout=10000)
        for path in ('observer.js', 'prepare.js'):
            page.add_script_tag(content=(ROOT / 'extension' / path).read_text(encoding='utf-8'))
        result = page.evaluate('''async (op) => {
          const api = RegistradorPreparation, form = document.forms[0], editor = form.querySelector('textarea'),
            checkbox = form.querySelector('input'), state = form.querySelector('select'), table = document.querySelector('table');
          let count = 0, submits = 0, changes = 0;
          document.addEventListener('submit', e => { e.preventDefault(); submits++; });
          checkbox.addEventListener('change', () => changes++);
          const assert = (ok, code) => { if (!ok) throw Error(code); count++; };
          const reject = async (fn, code) => { let got; try { await fn(); } catch(e) { got = e.message; } assert(got === code, `${code}: ${got}`); };
          for (const origin of ['ESPERA','CUMPLIMIENTO','INFORMES_POR_VENCER','INFORMES_VENCIDOS']) {
            document.body.dataset.origin = origin;
            for (const destination of ['ADMINISTRATIVA','AL_TRIBUNAL']) {
              const a = {...op, origin, destination}, current = structuredClone(a), listed = api.locate(document, a);
              assert(listed.identity.ingreso_id === '304' && listed.row.textContent.includes('exacto'), 'IDENTIDAD');
              const receipt = await api.prepare(document, a, current, listed);
              assert(!receipt.saved && editor.value === a.approved_text && state.value === 'otro' && checkbox.checked === (destination === 'AL_TRIBUNAL'), 'PREPARAR');
              const before = changes; await api.prepare(document, a, current, listed); assert(changes === before, 'IDEMPOTENCIA');
            }
          }
          document.body.dataset.origin = op.origin;
          const listed = api.locate(document, op), current = structuredClone(op);
          document.body.dataset.menu = '91'; await reject(() => api.locate(document, op), 'ORIGEN_INCORRECTO'); document.body.dataset.menu = '89';
          const dup = listed.row.cloneNode(true); table.tBodies[0].append(dup); await reject(() => api.locate(document, op), 'INGRESO_NO_UNICO'); dup.remove();
          assert(api.parseLink('ShowObservaciones(101,202,304,11,1,0); other()') === null && api.parseLink('ShowObservaciones(101,202,304,99,1,0)') === null, 'PARSER');
          await reject(() => api.prepare(document, op, {...current, approval_version:2}, listed), 'APROBACION_INVALIDA');
          const bad = {...op, approved_text:'Otro texto'}; await reject(() => api.prepare(document,bad,current,listed), 'APROBACION_INVALIDA');
          form.dataset.ingreso = '303'; await reject(() => api.prepare(document,op,current,listed), 'FORMULARIO_CAMBIADO'); form.dataset.ingreso = '304';
          editor.maxLength = 2; await reject(() => api.prepare(document,op,current,listed), 'TEXTO_NO_REPRESENTABLE'); editor.maxLength = 2000;
          editor.value = 'Borrador distinto'; await reject(() => api.prepare(document,op,current,listed), 'BORRADOR_EXISTENTE'); editor.value = op.approved_text;
          state.options[1].disabled = true; await reject(() => api.prepare(document,op,current,listed), 'ESTADO_NO_DISPONIBLE'); state.options[1].disabled = false;
          checkbox.indeterminate = true; await reject(() => api.prepare(document,op,current,listed), 'VALORES_NO_COINCIDEN'); checkbox.indeterminate = false;
          assert(submits === 0, 'ESCRITURA');
          return count;
        }''', OP)
        assert result == 35, result
        print(f'PASS {result}')
    finally:
        browser.close()
