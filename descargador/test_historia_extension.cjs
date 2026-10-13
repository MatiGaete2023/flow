/* Prueba del comando history_open en un DOM mínimo ficticio (sin datos reales). */
const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const {webcrypto}=require('node:crypto');
const source=fs.readFileSync(__dirname+'/extension/pagina.js','utf8');
const CIFRADO={crr_causa:'AAA%2BBBB',tip_causa:'CCC',rol_causa:'DDD',era_causa:'EEE',cod_tribunal:'FFF'};
async function main(){
  assert.ok(!source.replace(/\/\*[\s\S]*?\*\//g,'').includes('document.cookie'));
  let calls=[],step1=null,step2={body:'<html>ventana ficticia</html>',type:'text/html'},cancelled=false;
  const run={diaryPairs:[['COD_Tribunal_sel','49'],['TIP_Consulta','2'],['COD_Lengueta','tdCumplimiento']],diaryDocument:null};
  const link=t=>({getAttribute:a=>a==='onclick'?t:''});
  let elements=[link("ShowHistoria(49,'X-1-2026',100)")];
  run.diaryDocument={getElementById:id=>id==='tablaCumplimiento'?{querySelectorAll:()=>elements}:null};
  const form={name:'InformesPpalForm',action:'https://familia.pjud.cl/SITFAWEB/InformesDAction.do',method:'post',querySelector:s=>s==='[name="COD_Lengueta"]'?{}:null};
  const context=vm.createContext({location:{origin:'https://familia.pjud.cl',href:'https://familia.pjud.cl/fixture'},
    document:{querySelector:()=>form,getElementById:()=>({})},window:{__sitfaRun:run},URL,URLSearchParams,TextEncoder,TextDecoder,Uint8Array,AbortSignal,
    Headers,ReadableStream,crypto:webcrypto,btoa:s=>Buffer.from(s,'binary').toString('base64'),fetch:async(url,options)=>{
      calls.push({url,options});
      if(url.includes('Encrypt_AjaxAccion.do')){const r=step1===null?JSON.stringify(CIFRADO):step1;
        return new Response(r.body??r,{status:r.status??200,headers:{'content-type':'text/html'}});}
      if(step2.streaming)return {status:200,headers:new Headers({'content-type':step2.type}),body:new ReadableStream({pull(controller){controller.enqueue(new Uint8Array(1024*1024));},cancel(){cancelled=true;}})};
      return new Response(step2.body,{status:200,headers:{'content-type':step2.type}});
    }});
  vm.runInContext(source,context);
  const params={tribunal:'49',rit:'X-1-2026',causa:'100'};
  const ok=await context.sitfaTask('history_open',{params});
  assert.equal(ok.status,200);assert.equal(ok.digest.length,64);assert.deepEqual(ok.params,params);
  assert.equal(calls.length,2);
  assert.equal(calls[0].url,'https://familia.pjud.cl/SITFAWEB/Encrypt_AjaxAccion.do?opcion=1&TIP_Causa=X&ROL_Causa=1&ERA_Causa=2026&crr_idcausaUrl=100&cod_tribunalUrl=49');
  assert.equal(calls[0].options.method,'POST');assert.equal(calls[0].options.credentials,'include');
  assert.equal(calls[1].url,'https://familia.pjud.cl/SITFAWEB/ConsultaDetalleAtPublicoAccion.do?TipoPopup=3&a1=AAA%2BBBB&b2=CCC&c3=DDD&d4=EEE&e5=FFF&B4=1&FLG_Encripta=1');
  assert.equal(calls[1].options.method,'GET');assert.equal(calls[1].options.credentials,'include');
  const dump=JSON.stringify(ok);for(const v of Object.values(CIFRADO))assert.ok(!dump.includes(v));
  // identidad / enlace: error y ningún fetch
  const casos=[[{rit:'X-2-2026'},'HISTORY_LINK'],[{causa:'101'},'HISTORY_LINK'],[{tribunal:'113'},'HISTORY_IDENTITY'],
    [{extra:'1'},'HISTORY_IDENTITY'],[{rit:'x-1-2026'},'HISTORY_IDENTITY'],[{rit:'X-1-26'},'HISTORY_IDENTITY']];
  for(const [cambio,codigo] of casos){
    calls=[];const out=await context.sitfaTask('history_open',{params:{...params,...cambio}});
    assert.equal(out.__sitfaError,codigo,JSON.stringify(cambio));assert.equal(calls.length,0);
  }
  elements=[];calls=[];
  assert.equal((await context.sitfaTask('history_open',{params})).__sitfaError,'HISTORY_LINK');assert.equal(calls.length,0);
  elements=[link("ShowHistoria(49,'X-1-2026',100)")];
  // respuesta intermedia inválida: 1 fetch
  const malas=['no es json',JSON.stringify({...CIFRADO,crr_causa:undefined}),JSON.stringify({...CIFRADO,rol_causa:'a&b'}),
    JSON.stringify({...CIFRADO,extra:'valor ficticio'}),JSON.stringify(CIFRADO)+' '.repeat(5000),{status:500,body:JSON.stringify(CIFRADO)},'[]'];
  for(const mala of malas){
    calls=[];step1=mala;
    assert.equal((await context.sitfaTask('history_open',{params})).__sitfaError,'HISTORY_RESPONSE');assert.equal(calls.length,1);
  }
  step1=null;
  elements=[link("ShowHistoria(49,'X-1-2026',100)"),link("ShowHistoria(49,'X-1-2026',100)")];
  assert.equal((await context.sitfaTask('history_open',{params})).status,200);
  const action="ShowHistoria(49,'X-1-2026',100)";
  for(const text of [`"${action}"`,`\`${action}\``,`/* ${action} */`,`// ${action}`,`if(x) /${action}/.test(x)`]) {
    elements=[link(text)];calls=[];
    assert.equal((await context.sitfaTask('history_open',{params})).__sitfaError,'HISTORY_LINK',text);
    assert.equal(calls.length,0);
  }
  elements=[link(action)];
  for(const body of ['<html><input type="hidden" value="AAA%2BBBB"></html>',
                     '<html><input value="AAA+BBB"></html>',
                     '<html><input value="&#65;&#65;&#65;%2BBBB"></html>']) {
    step2={body,type:'text/html'};
    const out=await context.sitfaTask('history_open',{params});
    assert.equal(out.__sitfaError,'HISTORY_RESPONSE');assert.equal(out.body,undefined);
  }
  step2={body:'contenido ficticio',type:'application/json'};
  assert.equal((await context.sitfaTask('history_open',{params})).__sitfaError,'HISTORY_RESPONSE');
  step2={streaming:true,type:'text/html'};cancelled=false;
  assert.equal((await context.sitfaTask('history_open',{params})).__sitfaError,'HISTORY_RESPONSE');assert.ok(cancelled);
  console.log('OK: history_open: 2 peticiones exactas, identidad, enlace y respuestas inválidas; sin valores cifrados en el resultado.');
}
main().catch(error=>{console.error(error);process.exitCode=1;});
