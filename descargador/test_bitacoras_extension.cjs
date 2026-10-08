/* Prueba del comando real de la extensión en un DOM mínimo ficticio. */
const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const {webcrypto}=require('node:crypto');
const source=fs.readFileSync(__dirname+'/extension/pagina.js','utf8');
async function main(){
  let calls=[],elements=[],tableId='tablaCumplimiento';
  const params={tipo_popUp:'12',CRR_IdCausa:'100',COD_Tribunal:'49',TIP_Consulta:'2',ID_Ingreso:'201',COD_Etapa:'3',FLG_MejorNinez:'0'};
  const run={diaryPairs:[['COD_Tribunal_sel','49'],['TIP_Consulta','2'],['COD_Lengueta','tdCumplimiento']],
    diaryDocument:{getElementById:id=>id===tableId?{querySelectorAll:()=>elements}:null}};
  const form={name:'InformesPpalForm',action:'https://familia.pjud.cl/SITFAWEB/InformesDAction.do',method:'post',querySelector:s=>s==='[name="COD_Lengueta"]'?{}:null};
  const context=vm.createContext({location:{origin:'https://familia.pjud.cl',href:'https://familia.pjud.cl/fixture'},
    document:{querySelector:()=>form,getElementById:()=>({})},window:{__sitfaRun:run},URL,URLSearchParams,TextEncoder,TextDecoder,Uint8Array,AbortSignal,
    crypto:webcrypto,btoa:s=>Buffer.from(s,'binary').toString('base64'),fetch:async(url,options)=>{
      calls.push({url,options});return new Response('<html>captura ficticia</html>',{status:200,headers:{'content-type':'text/html'}});
    }});
  vm.runInContext(source,context);
  function link(text){return {getAttribute:a=>a==='onclick'?text:''};}
  elements=[link("ShowObservaciones('100','49','201','12','3','0')")];
  const ok=await context.sitfaTask('diary_open',{params});
  assert.equal(ok.status,200);assert.equal(ok.digest.length,64);assert.equal(calls.length,1);
  assert.equal(calls[0].options.method,'GET');assert.equal(calls[0].options.credentials,'include');
  assert.equal(new URL(calls[0].url).searchParams.get('TIP_Consulta'),'2');
  for(const change of [{ID_Ingreso:'999'},{COD_Tribunal:'113'},{TIP_Consulta:'1'},{tipo_popUp:'13'},{evil:'extra'}]){
    const out=await context.sitfaTask('diary_open',{params:{...params,...change}});
    assert.equal(out.__sitfaError,change.ID_Ingreso?'DIARY_LINK':'DIARY_IDENTITY');assert.equal(calls.length,1);
  }
  elements.push(elements[0]);assert.equal((await context.sitfaTask('diary_open',{params})).__sitfaError,'DIARY_LINK');
  assert.equal(calls.length,1);elements=[elements[0]];
  tableId='tablaEspera';assert.equal((await context.sitfaTask('diary_open',{params})).__sitfaError,'DIARY_IDENTITY');
  assert.equal(calls.length,1);
  console.log('OK: GET de lectura, identidad, modalidad, pestaña y duplicados; ningún guardado RUS.');
}
main().catch(error=>{console.error(error);process.exitCode=1;});
