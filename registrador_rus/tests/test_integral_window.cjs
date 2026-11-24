'use strict';
const assert=require('node:assert/strict'), fs=require('node:fs'), path=require('node:path'), vm=require('node:vm');
const source=fs.readFileSync(path.join(__dirname,'../inspector/integral_popup.js'),'utf8');
const SITE='https://familia.pjud.cl', OPEN=SITE+'/SITFAWEB/IrPopUpInformesAccion.do', POST=SITE+'/SITFAWEB/InformesDAction.do';
const empty={forms_total:0,tables_total:0,table_id_nodes:0,table_name_nodes:0,table_id_visible:0,visible_frames:0,modal_visible:0,form_candidates:[],tables:[],table_markers:[],context:[],controls:[],visible_controls:0,hidden_controls:0,history_buttons_omitted:0,pagination:{next:0,prev:0,buttons:0}};
const evidence={schema:'IntegralEvidenceV1',status:'DIAGNOSTIC',write_enabled:false,truncated:false,documents:[{path:[],access:'OK',detail:{...empty,table_id_nodes:1}}]};
const tab=(id,url,windowId=4,status='complete')=>({id,url,windowId,status});
async function exercise({tabs=[tab(7,OPEN+'?NO_EXPORTAR=123456'),tab(8,POST,5)],types={4:'popup',5:'normal'},allow=true,current=null,result=evidence,fail=false,remove=true}={}){
 let handler,request=0,query=0,inject=[],removed=0;
 const button={disabled:false,addEventListener:(name,fn)=>{assert.equal(name,'click');handler=fn;}};
 const output={value:''},status={textContent:''};
 const chrome={permissions:{request:async x=>{request++;assert.equal(x.origins.join(','),SITE+'/*');return allow;},remove:async()=>{removed++;return remove;},contains:async()=>!remove},
  tabs:{query:async q=>{query++;assert.equal(q.url.join('|'),OPEN+'*|'+POST+'*');return tabs;},get:async id=>current||tabs.find(t=>t.id===id)},
  windows:{get:async id=>({type:types[id]})},scripting:{executeScript:async args=>{inject.push(args.target.tabId);assert.equal(args.files[0],'integral.js');assert.equal(args.world,'ISOLATED');if(fail)throw Error('NO_EXPORTAR');return [{result}];}}};
 vm.runInNewContext(source,{document:{getElementById:id=>id==='inspect'?button:id==='result'?output:status},URL,chrome});
 await handler();assert.equal(button.disabled,false);assert.equal(request,1);assert.equal(removed,allow?1:0);assert(!output.value.includes('NO_EXPORTAR'));assert(!output.value.includes(SITE));return {output:output.value,query,inject,status:status.textContent};
}
(async()=>{
 let r=await exercise();assert.deepEqual(JSON.parse(r.output),evidence);assert.deepEqual(r.inject,[7]);assert.equal(r.status,'Acceso opcional retirado.');
 r=await exercise({allow:false});assert.equal(r.output,'PERMISO_OPCIONAL_DENEGADO');assert.equal(r.query,0);
 r=await exercise({tabs:[tab(8,POST,5)]});assert.equal(r.output,'BITACORA_NO_ENCONTRADA');assert.equal(r.inject.length,0);
 r=await exercise({tabs:[tab(8,POST)]});assert.deepEqual(r.inject,[8]);assert.deepEqual(JSON.parse(r.output),evidence);
 r=await exercise({tabs:[tab(7,OPEN),tab(8,POST)]});assert.equal(r.output,'BITACORAS_MULTIPLES: 2');assert.equal(r.inject.length,0);
 r=await exercise({tabs:[tab(7,OPEN+'extra'),tab(8,'https://example.test/SITFAWEB/IrPopUpInformesAccion.do')]});assert.equal(r.output,'BITACORA_NO_ENCONTRADA');
 r=await exercise({tabs:[tab(7,OPEN,4,'loading')]});assert.equal(r.output,'BITACORA_CARGANDO');
 r=await exercise({current:tab(7,POST)});assert.equal(r.output,'BITACORA_CAMBIADA');assert.equal(r.inject.length,0);
 r=await exercise({tabs:[tab(8,POST)],result:{...evidence,documents:[{path:[],access:'OK',detail:empty}]}});assert.equal(r.output,'VENTANA_POST_NO_BITACORA');
 r=await exercise({result:{...evidence,html:'NO_EXPORTAR'}});assert.equal(r.output,'INSPECCION_NO_CONCLUYENTE');
 r=await exercise({fail:true});assert.equal(r.output,'INSPECCION_NO_DISPONIBLE');
 r=await exercise({remove:false});assert(r.status.includes('sigue activo'));
 console.log('PASS 12 window selection cases');
})().catch(e=>{console.error(e);process.exitCode=1;});
