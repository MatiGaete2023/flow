/* Candidato piloto: recibe órdenes solo de Flow local, no auto-poll.
 * La creación real sigue bloqueada por WRITE_ACCEPTED en live_adapter.js.
 */
'use strict';
const {access,fail,rpc,locate,capture,selectPair,ensurePopup}=FlowLiveBrowser;
const input=document.getElementById('credentials'),run=document.getElementById('run'),connect=document.getElementById('connect'),
  disconnect=document.getElementById('disconnect'),status=document.getElementById('status'),permission=document.getElementById('permission');
let host=null,prepared=null,creationConsumed=false,busy=false;
const same=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
function credentials(value){
  const v=value.trim().split(/\s*\|\s*/);
  if(v.length!==3||!/^[0-9]{1,5}$/.test(v[0])||Number(v[0])<1||Number(v[0])>65535||
     !/^[a-f0-9]{32}$/.test(v[1])||!/^[A-Za-z0-9_-]{43}$/.test(v[2]))fail('CREDENCIALES_LOCALES_INVALIDAS');
  return {port:v[0],session:v[1],token:v[2]};
}
function lease(cmd){
  if(!Number.isSafeInteger(cmd.expires_at_ms)||Date.now()>cmd.expires_at_ms)fail('ORDEN_EXPIRADA');
}
async function closeSession(){
  host=null;prepared=null;creationConsumed=false;input.value='';run.disabled=true;disconnect.disabled=true;connect.disabled=false;
  try{
    await chrome.permissions.remove(access);
    permission.textContent=await chrome.permissions.contains(access)?'Permiso opcional sigue activo; revisar chrome://extensions/.':'Permiso opcional retirado.';
  }catch(_){permission.textContent='No se confirmó retiro de permiso.';}
}
connect.addEventListener('click',async()=>{
  if(busy||host)return;
  busy=true;connect.disabled=true;
  try{
    const c=credentials(input.value);input.value='';
    if(!await chrome.permissions.request(access))fail('PERMISO_DENEGADO');
    host=c;run.disabled=false;disconnect.disabled=false;status.textContent='Sesión local conectada. Espera una orden de Flow.';
  }catch(_){await closeSession();status.textContent='Conexión bloqueada.';}
  finally{busy=false;}
});
disconnect.addEventListener('click',async()=>{if(!busy){await closeSession();status.textContent='Sesión cerrada. Si hubo intento, recuperar solo por lectura.';}});
async function invoke(p,docId,func,args){
  const r=await chrome.scripting.executeScript({target:{tabId:p.id,documentIds:[docId]},world:'ISOLATED',func,args});
  if(r.length!==1||!r[0].result||r[0].result.blocked)fail('OPERACION_DOM_BLOQUEADA');
  return r[0].result;
}
async function execute(cmd){
  lease(cmd);
  const {l,p}=await selectPair();
  const listed=await locate(l,cmd.operation);
  lease(cmd);
  if(cmd.command==='readback'){
    const snap=await capture(p,cmd.operation,listed.context);
    const again=await locate(l,cmd.operation);
    if(again.documentId!==listed.documentId||!same(again.context,listed.context))fail('LISTADO_CAMBIADO');
    await ensurePopup(p,snap.documentId);
    return {context:listed.context,html:snap.html};
  }
  if(cmd.command==='prepare'){
    if(prepared||creationConsumed)fail('SESION_YA_PREPARADA');
    const r=await chrome.scripting.executeScript({target:{tabId:p.id},files:['prepare.js','observed_form.js','live_adapter.js'],world:'ISOLATED'});
    if(r.length!==1||typeof r[0].documentId!=='string')fail('DOCUMENTO_NO_IDENTIFICADO');
    const documentId=r[0].documentId;
    const result=await invoke(p,documentId,async (op,ctx,session,expiry)=>{
      try{
        if(Date.now()>expiry)throw Error('ORDEN_EXPIRADA');
        const s=await RegistradorLiveAdapter.session(document,op,ctx);
        if(Date.now()>expiry)throw Error('ORDEN_EXPIRADA');
        const report=s.prepare(op);
        globalThis.FlowPilotSession=Object.freeze({session,operation_id:op.operation_id,s});
        return {report};
      }catch(_){return {blocked:true};}
    },[cmd.operation,listed.context,cmd.session_id,cmd.expires_at_ms]);
    await ensurePopup(p,documentId);
    const again=await locate(l,cmd.operation);
    if(again.documentId!==listed.documentId||!same(again.context,listed.context))fail('LISTADO_CAMBIADO');
    prepared={listTab:l.id,listDocument:listed.documentId,popupTab:p.id,popupDocument:documentId,
              context:listed.context,operation_id:cmd.operation.operation_id};
    return {report:result.report,context:listed.context};
  }
  if(!prepared||prepared.operation_id!==cmd.operation.operation_id||prepared.listTab!==l.id||
    prepared.listDocument!==listed.documentId||prepared.popupTab!==p.id||!same(prepared.context,listed.context))fail('SESION_PREPARADA_CAMBIADA');
  await ensurePopup(p,prepared.popupDocument);
  lease(cmd);
  if(cmd.command==='verify'){
    const result=await invoke(p,prepared.popupDocument,(op,ctx,session,expiry)=>{
      try{
        if(Date.now()>expiry||globalThis.FlowPilotSession?.session!==session||FlowPilotSession.operation_id!==op.operation_id)throw Error('SESION_DISTINTA');
        const report=FlowPilotSession.s.verify(op);
        return {report,html:RegistradorLiveAdapter.capture(document,op,ctx)};
      }catch(_){return {blocked:true};}
    },[cmd.operation,listed.context,cmd.session_id,cmd.expires_at_ms]);
    return {report:result.report,context:listed.context,html:result.html};
  }
  if(cmd.command==='create_once'){
    if(creationConsumed)fail('CREACION_YA_CONSUMIDA');
    // Consumir antes de invocar. Una navegación o excepción nunca habilita repetición.
    creationConsumed=true;
    await invoke(p,prepared.popupDocument,(op,session,reservation,expiry)=>{
      try{
        if(Date.now()>expiry||globalThis.FlowPilotSession?.session!==session||FlowPilotSession.operation_id!==op.operation_id)throw Error('SESION_DISTINTA');
        return FlowPilotSession.s.createOnce(op,reservation);
      }catch(_){return {blocked:true};}
    },[cmd.operation,cmd.session_id,cmd.reservation,cmd.expires_at_ms]);
    return {};
  }
  fail('ORDEN_PROHIBIDA');
}
run.addEventListener('click',async()=>{
  if(busy||!host)return;
  busy=true;run.disabled=true;disconnect.disabled=true;
  const {port,session,token}=host;
  let cmd=null;
  try{
    cmd=(await rpc(port,token,'poll',{schema:'BridgePollV1',session_id:session})).command;
    if(!cmd){status.textContent='Sin orden pendiente. Revisa Flow.';return;}
    if(cmd.schema!=='PilotCommandV1'||cmd.session_id!==session||
      !['prepare','verify','create_once','readback'].includes(cmd.command))fail('ORDEN_PROHIBIDA');
    const body=await execute(cmd);
    await rpc(port,token,'ack',{schema:'PilotAckV1',session_id:session,command_id:cmd.command_id,status:'OK',...body});
    status.textContent=cmd.command==='create_once'?'Intento consumido. Solo relectura; no repetir.':'Orden completada. Revisa el resultado en Flow.';
    cmd=null;
  }catch(_){
    status.textContent='Orden bloqueada o respuesta perdida. Revisa Flow; no repetir creación.';
    if(cmd?.command_id)try{await rpc(port,token,'ack',{schema:'PilotAckV1',session_id:session,command_id:cmd.command_id,status:'BLOCKED'});}catch(_){}
  }finally{busy=false;run.disabled=!host;disconnect.disabled=!host;}
});
