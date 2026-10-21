/* WD-1: comando calendar_open de la extensión en un DOM mínimo ficticio (sin datos reales). */
const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const {webcrypto}=require('node:crypto');
const source=fs.readFileSync(__dirname+'/extension/pagina.js','utf8');
async function main(){
  let calls=[],elements=[],tableId='tablaCumplimiento';
  const args=['100','300','49','400','X-1-2026','01/03/2026','PRM Centro, ficticio','JOSÉ Ñandú Ficticio','6 Meses','3 Meses','201','1'];
  const run={diaryPairs:[['COD_Tribunal_sel','49'],['TIP_Consulta','2'],['COD_Lengueta','tdCumplimiento']],
    diaryDocument:{getElementById:id=>id===tableId?{querySelectorAll:()=>elements}:null}};
  const form={name:'InformesPpalForm',action:'https://familia.pjud.cl/SITFAWEB/InformesDAction.do',method:'post',querySelector:s=>s==='[name="COD_Lengueta"]'?{}:null};
  const context=vm.createContext({location:{origin:'https://familia.pjud.cl',href:'https://familia.pjud.cl/fixture'},
    document:{querySelector:()=>form,getElementById:()=>({})},window:{__sitfaRun:run},URL,URLSearchParams,TextEncoder,TextDecoder,Uint8Array,AbortSignal,
    crypto:webcrypto,btoa:s=>Buffer.from(s,'binary').toString('base64'),fetch:async(url,options)=>{
      calls.push({url,options});return new Response('<html>calendario ficticio</html>',{status:200,headers:{'content-type':'text/html'}});
    }});
  vm.runInContext(source,context);
  const link=text=>({getAttribute:a=>a==='onclick'?text:''});
  const call=values=>"ShowInformesProgramados("+values.map(v=>/^\d+$/.test(v)?v:"'"+v+"'").join(', ')+")";
  elements=[link(call(args))];
  const ok=await context.sitfaTask('calendar_open',{args});
  assert.equal(ok.status,200);assert.equal(ok.digest.length,64);assert.deepEqual(ok.args,args);
  assert.equal(calls.length,1);assert.equal(calls[0].options.method,'GET');assert.equal(calls[0].options.credentials,'include');
  const url=calls[0].url;
  assert.ok(url.startsWith('https://familia.pjud.cl/SITFAWEB/IrPopUpInformesAccion.do?tipo_popUp=8&ID_Ingreso=201&TIP_Programa=2&'));
  // Mismo orden que la función de la página; remisión antes que intervención; ISO-8859-1.
  assert.ok(url.includes('&GLS_Nombre=JOS%C9%20%D1and%FA%20Ficticio&REM_Informe=3%20Meses&PLZ_Intervencion=6%20Meses&ID_Ingreso=201&FLG_MejorNinez=1'));
  assert.ok(url.includes('&GLS_Centro=PRM%20Centro%2C%20ficticio&'));
  for(const change of [[10,'999'],[0,'101'],[7,'Otra persona']]){
    const other=[...args];other[change[0]]=change[1];
    assert.equal((await context.sitfaTask('calendar_open',{args:other})).__sitfaError,'CALENDAR_LINK');
  }
  const tribunal=[...args];tribunal[2]='113';
  assert.equal((await context.sitfaTask('calendar_open',{args:tribunal})).__sitfaError,'CALENDAR_IDENTITY');
  assert.equal((await context.sitfaTask('calendar_open',{args:args.slice(0,11)})).__sitfaError,'CALENDAR_CONTEXT');
  elements.push(elements[0]);
  assert.equal((await context.sitfaTask('calendar_open',{args})).__sitfaError,'CALENDAR_LINK');
  elements=[link(call(args))];tableId='tablaEspera';
  assert.equal((await context.sitfaTask('calendar_open',{args})).__sitfaError,'CALENDAR_IDENTITY');
  assert.equal(calls.length,1);
  const caps=await context.sitfaTask('capabilities',{});
  assert.equal(caps.protocolos.calendario_lectura,1);assert.equal(caps.escritura_rus,false);
  console.log('OK: calendario GET de lectura, enlace único, identidad, orden de parámetros e ISO-8859-1.');
}
main().catch(error=>{console.error(error);process.exitCode=1;});
