/* Solo lectura. No preparar, pulsar ni guardar. El HTML viaja solo al proceso local. */
'use strict';
const SITE='https://familia.pjud.cl', POST='/SITFAWEB/InformesDAction.do',OPEN='/SITFAWEB/IrPopUpInformesAccion.do';
const access={origins:[SITE+'/*']};
const button=document.getElementById('run'),status=document.getElementById('status'),input=document.getElementById('credentials');
const permission=document.getElementById('permission');
const fail=code=>{throw new Error(code);};
function route(tab){
  try{const u=new URL(tab.url);return u.origin===SITE?u.pathname:null;}catch(_){return null;}
}
async function rpc(port,token,path,body){
  const r=await fetch('http://127.0.0.1:'+port+'/v1/'+path,{method:'POST',headers:{'Content-Type':'application/json','Authorization':'Bearer '+token},
    body:JSON.stringify(body),cache:'no-store',credentials:'omit',redirect:'error',referrerPolicy:'no-referrer',signal:AbortSignal.timeout(20000)});
  if(!r.ok)fail('PUENTE_LOCAL_RECHAZADO');
  return r.json();
}
async function locate(tab,op){
  const injected=await chrome.scripting.executeScript({target:{tabId:tab.id,allFrames:true},files:['prepare.js','observed_form.js','live_adapter.js'],world:'ISOLATED'});
  const hits=[];
  for(const frame of injected){
    if(typeof frame.documentId!=='string')fail('DOCUMENTO_NO_IDENTIFICADO');
    const result=await chrome.scripting.executeScript({target:{tabId:tab.id,documentIds:[frame.documentId]},world:'ISOLATED',args:[op],
      func:op=>{try{return {context:RegistradorLiveAdapter.locate(document,op).context};}catch(_){return null;}}});
    if(result.length===1&&result[0].result?.context)hits.push({documentId:frame.documentId,context:result[0].result.context});
  }
  if(hits.length!==1)fail('INGRESO_LISTADO_NO_UNICO');
  return hits[0];
}
async function capture(tab,op,context){
  const inject=await chrome.scripting.executeScript({target:{tabId:tab.id},files:['prepare.js','observed_form.js','live_adapter.js'],world:'ISOLATED'});
  if(inject.length!==1||typeof inject[0].documentId!=='string')fail('DOCUMENTO_NO_IDENTIFICADO');
  const rows=await chrome.scripting.executeScript({target:{tabId:tab.id,documentIds:[inject[0].documentId]},world:'ISOLATED',args:[op,context],
    func:(op,ctx)=>{try{return {html:RegistradorLiveAdapter.capture(document,op,ctx)};}catch(_){return null;}}});
  if(rows.length!==1||typeof rows[0].result?.html!=='string')fail('BITACORA_NO_VERIFICADA');
  return {documentId:inject[0].documentId,html:rows[0].result.html};
}
button.addEventListener('click',async()=>{
  button.disabled=true;permission.textContent='';
  const parts=input.value.trim().split(/\s*\|\s*/);input.value='';
  let granted=false,cmd=null,port=null,session=null,token=null;
  try{
    if(parts.length!==3||!/^[0-9]{1,5}$/.test(parts[0])||Number(parts[0])<1||Number(parts[0])>65535||
      !/^[a-f0-9]{32}$/.test(parts[1])||!/^[A-Za-z0-9_-]{43}$/.test(parts[2]))fail('CREDENCIALES_LOCALES_INVALIDAS');
    [port,session,token]=parts;
    granted=await chrome.permissions.request(access);
    if(!granted)fail('PERMISO_DENEGADO');
    const response=await rpc(port,token,'poll',{schema:'BridgePollV1',session_id:session});
    cmd=response.command;
    if(!cmd||cmd.schema!=='LiveReadCommandV1'||cmd.session_id!==session||cmd.command!=='validate_read_only'||cmd.write_enabled!==false)fail('ORDEN_LOCAL_INVALIDA');
    const tabs=await chrome.tabs.query({url:[SITE+'/*']});
    const listing=[],popups=[];
    for(const tab of tabs){
      if(!Number.isInteger(tab.id)||tab.status!=='complete')continue;
      const w=await chrome.windows.get(tab.windowId);
      if(w.type==='normal')listing.push(tab);
      else if(w.type==='popup'&&[OPEN,POST].includes(route(tab)))popups.push(tab);
    }
    if(listing.length!==1||popups.length!==1)fail('ABRIR_UN_LISTADO_Y_UNA_BITACORA');
    const l=listing[0],p=popups[0];
    if(Number.isInteger(p.openerTabId)&&p.openerTabId!==l.id)fail('VENTANA_DE_OTRO_LISTADO');
    const before=await locate(l,cmd.operation);
    const snap=await capture(p,cmd.operation,before.context);
    const again=await locate(l,cmd.operation);
    if(again.documentId!==before.documentId||JSON.stringify(again.context)!==JSON.stringify(before.context))fail('LISTADO_CAMBIADO');
    const current=await chrome.tabs.get(p.id);
    if(current.status!=='complete'||route(current)!==route(p)||current.windowId!==p.windowId)fail('BITACORA_CAMBIADA');
    const alive=await chrome.scripting.executeScript({target:{tabId:p.id,documentIds:[snap.documentId]},world:'ISOLATED',
      func:()=>true});
    if(alive.length!==1||alive[0].result!==true)fail('BITACORA_CAMBIADA');
    await rpc(port,token,'ack',{schema:'LiveReadAckV1',session_id:session,command_id:cmd.command_id,status:'OK',context:before.context,html:snap.html});
    cmd=null;status.textContent='Lectura entregada al Asistente. Consulta allí el resultado del parser. No se ha registrado nada.';
  }catch(e){
    // No mostrar errores del navegador, URL ni contenido de expedientes.
    const known=['CREDENCIALES_LOCALES_INVALIDAS','PERMISO_DENEGADO','PUENTE_LOCAL_RECHAZADO','ORDEN_LOCAL_INVALIDA','ABRIR_UN_LISTADO_Y_UNA_BITACORA',
      'VENTANA_DE_OTRO_LISTADO','INGRESO_LISTADO_NO_UNICO','BITACORA_NO_VERIFICADA','LISTADO_CAMBIADO','BITACORA_CAMBIADA','DOCUMENTO_NO_IDENTIFICADO'];
    status.textContent=known.includes(e.message)?e.message:'VALIDACION_BLOQUEADA';
    if(cmd?.command_id)try{await rpc(port,token,'ack',{schema:'LiveReadAckV1',session_id:session,command_id:cmd.command_id,status:'BLOCKED'});}catch(_){}
  }finally{
    if(granted)try{await chrome.permissions.remove(access);permission.textContent=await chrome.permissions.contains(access)?
      'Permiso opcional aún activo; revisa chrome://extensions/.':'Permiso opcional retirado.';}catch(_){permission.textContent='No se confirmó el retiro; revisa chrome://extensions/.';}
    parts.fill('');port=session=token=cmd=null;button.disabled=false;
  }
});
