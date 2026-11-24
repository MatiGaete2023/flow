"""Integra el consumidor DOM con puente real localhost mediante transporte de prueba.
No carga MV3: política del Chromium de desarrollo; crypto y fetch se simulan únicamente
como adaptadores de entorno, pero el SHA-256 y el HTTP se ejecutan realmente en Python.
"""
from dataclasses import replace
from hashlib import sha256
import http.client
import json
import os
from pathlib import Path
from playwright.sync_api import sync_playwright

from registrador_rus.app.bridge import LocalBridge
from registrador_rus.app.simulation import demo_operations

ROOT = Path(__file__).resolve().parents[1]
ORIGIN = 'chrome-extension://' + 'a'*32


def main():
    bridge = LocalBridge(ORIGIN)
    try:
        def post(url, options):
            assert url in [f'http://{bridge.host}/v1/poll', f'http://{bridge.host}/v1/ack']
            conn = http.client.HTTPConnection(bridge.host, timeout=3)
            try:
                conn.request('POST', '/v1/' + url.rsplit('/',1)[-1],
                             body=options['body'], headers=options['headers'] | {'Origin':ORIGIN})
                response = conn.getresponse(); body = response.read()
                return {'ok':response.status==200,'data':json.loads(body) if body else None}
            finally: conn.close()

        with sync_playwright() as p:
            options = {'executable_path': os.environ['REGISTRADOR_CHROMIUM']} if os.getenv('REGISTRADOR_CHROMIUM') else {}
            browser = p.chromium.launch(headless=True, timeout=10000, **options)
            try:
                page = browser.new_page(); page.route('**/*', lambda route: route.abort())
                page.expose_function('fixturePost', post)
                page.expose_function('fixtureDigest', lambda octets:list(sha256(bytes(octets)).digest()))
                html = (ROOT/'extension/demo.html').read_text(encoding='utf-8')
                html = html.split('<script src="observer.js">')[0] + '</body></html>'
                page.set_content(html,timeout=10000)
                page.evaluate('''() => {
                  window.fetch = async (url, options) => { const r = await fixturePost(url,options); return {ok:r.ok,json:async()=>r.data}; };
                  Object.defineProperty(crypto,'subtle',{value:{digest:async (algorithm,bytes)=>{
                    if(algorithm!=='SHA-256') throw Error('algoritmo');
                    return Uint8Array.from(await fixtureDigest(Array.from(bytes))).buffer;
                  }}});
                }''')
                for name in ('observer.js','prepare.js','demo.js'):
                    page.add_script_tag(content=(ROOT/'extension'/name).read_text(encoding='utf-8'))
                page.locator('#port').fill(str(bridge.server.server_port))
                page.locator('#session').fill(bridge.mailbox.session_id)
                page.locator('#token').fill(bridge.token)
                count=0
                for op in demo_operations():
                    command_id=bridge.mailbox.submit('prepare',frozen=op,current=op)
                    page.locator('#poll').click(); page.wait_for_function("!document.querySelector('#poll').disabled")
                    status=page.locator('#status').inner_text()
                    assert status == 'Preparación ficticia recibida. No se guardó.',status
                    frame=page.frame_locator('#fixture')
                    assert frame.locator('textarea').input_value()==op.approved_text
                    assert frame.locator('input[type=checkbox]').is_checked()==op.tribunal_checked
                    assert frame.locator('button').is_disabled()
                    assert bridge.mailbox.take_result()==dict(command_id=command_id,status='OK',simulation_only=True)
                    count+=1
                op=demo_operations()[0]; invalid=replace(op,approved_text='otro',approved_text_sha256=sha256(b'otro').hexdigest())
                bridge.mailbox.submit('prepare',frozen=invalid,current=invalid)
                page.locator('#poll').click(); page.wait_for_function("!document.querySelector('#poll').disabled")
                assert bridge.mailbox.take_result()['status']=='BLOCKED'
                assert page.frame_locator('#fixture').locator('textarea').count()==0
                print(f'PASS {count} preparaciones y rechazo de operación ajena (transporte de prueba)')
            finally: browser.close()
    finally: bridge.close()


if __name__=='__main__': main()
