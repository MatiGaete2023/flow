"""P3c: comando `historico_open` de la extensión del Descargador en un DOM mínimo ficticio (Node). Sin datos reales."""

import shutil
import subprocess
from pathlib import Path

import pytest

HARNESS = r"""
const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const {webcrypto}=require('node:crypto');
const source=fs.readFileSync(process.argv[2],'utf8');
async function main(){
  let calls=[],tipo='text/html',envoltorio='<iframe src="/SITFAWEB/IrPopUpPublicoAccion.do"></iframe>';
  const run={diaryPairs:[['COD_Tribunal_sel','49'],['TIP_Consulta','2'],['COD_Lengueta','tdCumplimiento']],diaryDocument:null};
  const link=(t,rut)=>({getAttribute:a=>a==='onclick'?t:'',
    closest:()=>rut?{cells:[{textContent:rut}],closest:()=>table}:null});
  const table={querySelectorAll:()=>elements};
  const template=()=>({set innerHTML(h){
    this.content={querySelectorAll:()=>[...h.matchAll(/<(iframe|frame)[^>]*src="([^"]*)"/gi)]
      .map(m=>({localName:m[1],getAttribute:()=>m[2].replace(/&amp;/g,'&')}))};
  }});
  let elements=[link("ShowHistoriaRut('11.111.111-1')")];
  run.diaryDocument={getElementById:id=>id==='tablaCumplimiento'?table:null};
  const form={name:'InformesPpalForm',action:'https://familia.pjud.cl/SITFAWEB/InformesDAction.do',method:'post',querySelector:s=>s==='[name="COD_Lengueta"]'?{}:null};
  const context=vm.createContext({location:{origin:'https://familia.pjud.cl',href:'https://familia.pjud.cl/fixture'},
    document:{querySelector:()=>form,getElementById:()=>({}),createElement:template},window:{__sitfaRun:run},URL,URLSearchParams,TextEncoder,TextDecoder,
    Uint8Array,AbortSignal,Headers,ReadableStream,crypto:webcrypto,btoa:s=>Buffer.from(s,'binary').toString('base64'),
    DOMParser:class{parseFromString(h){return {querySelectorAll:()=>[...h.matchAll(/<i?frame[^>]*src="([^"]*)"/gi)]
      .map(m=>({getAttribute:()=>m[1].replace(/&amp;/g,'&')}))};}},
    fetch:async(url,options)=>{calls.push({url,options});
      const body=url.includes('PopUpHistorial.jsp')?envoltorio:'<html>Historial Menores · contenido</html>';
      return new Response(body,{status:200,headers:{'content-type':tipo}});}});
  vm.runInContext(source,context);
  const caps=await context.sitfaTask('capabilities',{});
  assert.equal(caps.protocolos.historico_lectura,2);
  const params={tribunal:'49',rut:'111111111'};
  const ok=await context.sitfaTask('historico_open',{params});
  assert.equal(ok.status,200);assert.equal(ok.digest.length,64);assert.deepEqual(ok.params,params);
  assert.equal(calls.length,2);
  assert.equal(calls[0].url,'https://familia.pjud.cl/SITFAWEB/jsp/Tramitacion/PopUp/PopUpHistorial.jsp?tipo_popUp=77&RUT_Litigante=11111111&RUT_DV=1');
  assert.equal(calls[0].options.method,'GET');assert.equal(calls[0].options.credentials,'include');
  // El RUT debe venir de un enlace ShowHistoriaRut del listado vigente; si no, ningún fetch.
  const casos=[[{rut:'222222222'},'HISTORICO_LINK'],[{tribunal:'113'},'HISTORICO_IDENTITY'],[{rut:'11.111.111-1'},'HISTORICO_IDENTITY'],
    [{extra:'1'},'HISTORICO_IDENTITY']];
  for(const [cambio,codigo] of casos){
    calls=[];const out=await context.sitfaTask('historico_open',{params:{...params,...cambio}});
    assert.equal(out.__sitfaError,codigo,JSON.stringify(cambio));assert.equal(calls.length,0);
  }
  elements=[link("ShowHistorialFichas('11.111.111-1')")];calls=[];
  assert.equal((await context.sitfaTask('historico_open',{params})).__sitfaError,'HISTORICO_LINK');assert.equal(calls.length,0);
  elements=[link("ShowHistoriaRut(11111111)")];calls=[];
  assert.equal((await context.sitfaTask('historico_open',{params})).__sitfaError,'HISTORICO_LINK');
  assert.equal(calls.length,0);
  elements=[link("ShowHistoriaRut(11111111)", "11.111.111-1")];
  assert.equal((await context.sitfaTask('historico_open',{params})).status,200);
  // Envoltorio real: un marco a IrPopUpPublicoAccion.do; se sigue SOLO ese marco del mismo sitio.
  envoltorio='<html><iframe src="/SITFAWEB/IrPopUpPublicoAccion.do?tipo_popUp=77&amp;RUT_Litigante=11111111&amp;RUT_DV=1"></iframe></html>';
  calls=[];const marco=await context.sitfaTask('historico_open',{params});
  assert.equal(marco.status,200);assert.equal(calls.length,2);
  assert.equal(calls[1].url,'https://familia.pjud.cl/SITFAWEB/IrPopUpPublicoAccion.do?tipo_popUp=77&RUT_Litigante=11111111&RUT_DV=1');
  assert.equal(calls[1].options.credentials,'include');
  assert.ok(Buffer.from(marco.body,'base64').toString().includes('contenido'));
  for(const malo of ['', '<frame src="/SITFAWEB/IrPopUpPublicoAccion.do">', '<iframe src="https://otro.sitio/SITFAWEB/IrPopUpPublicoAccion.do"></iframe>',
                     '<iframe src="/SITFAWEB/OtraAccion.do"></iframe>',
                     '<iframe src="/SITFAWEB/IrPopUpPublicoAccion.do"></iframe><iframe src="/SITFAWEB/IrPopUpPublicoAccion.do"></iframe>']){
    envoltorio='<html>'+malo+'</html>';calls=[];
    assert.equal((await context.sitfaTask('historico_open',{params})).__sitfaError,'HISTORICO_RESPONSE',malo);
    assert.equal(calls.length,1);
  }
  envoltorio='<html>Historial Menores</html>';
  tipo='application/pdf';elements=[link("ShowHistoriaRut('11111111-1')")];
  assert.equal((await context.sitfaTask('historico_open',{params})).__sitfaError,'HISTORICO_RESPONSE');
  console.log('ok');
}
main().catch(e=>{console.error(e);process.exit(1);});
"""


@pytest.mark.skipif(shutil.which('node') is None, reason='Node no está instalado')
def test_p3c_historico_open_valida_el_rut_contra_el_listado(tmp_path):
    script = tmp_path / 'historico.cjs'
    script.write_text(HARNESS, encoding='utf-8')
    pagina = Path(__file__).parent / 'extension' / 'pagina.js'
    result = subprocess.run(['node', str(script), str(pagina)], capture_output=True, text=True, timeout=60, check=False)
    assert result.returncode == 0 and 'ok' in result.stdout, result.stderr
