/* Solo lectura. No preparar, pulsar ni guardar. El HTML viaja solo al proceso local. */
'use strict';
const SITE='https://familia.pjud.cl', POST='/SITFAWEB/InformesDAction.do',OPEN='/SITFAWEB/IrPopUpInformesAccion.do';
const access={origins:[SITE+'/*']};
const button=document.getElementById('run'),status=document.getElementById('status'),input=document.getElementById('credentials');
const permission=document.getElementById('permission');
const {fail,rpc,locate,capture,selectPair,ensurePopup}=FlowLiveBrowser;
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
    if(!cmd||cmd.schema!=='LiveReadCommandV1'||cmd.session_id!==session||!['validate_read_only','validate_prepare_only'].includes(cmd.command)||cmd.write_enabled!==false)fail('ORDEN_LOCAL_INVALIDA');
    const {l,p}=await selectPair();
    const before=await locate(l,cmd.operation);
    const snap=await capture(p,cmd.operation,before.context);
    const again=await locate(l,cmd.operation);
    if(again.documentId!==before.documentId||JSON.stringify(again.context)!==JSON.stringify(before.context))fail('LISTADO_CAMBIADO');
    await ensurePopup(p,snap.documentId);
    let extra={};
    if(cmd.command==='validate_prepare_only'){
      if(!window.confirm('¿Preparar los valores aprobados desde Flow, sin guardar y sin disparar eventos? Debes cerrar la bitácora sin pulsar Nueva obs.'))fail('PREPARACION_CANCELADA');
      const prepared=await chrome.scripting.executeScript({target:{tabId:p.id,documentIds:[snap.documentId]},world:'ISOLATED',args:[cmd.operation,before.context],
        func:async(op,ctx)=>{try{const s=await RegistradorLiveAdapter.session(document,op,ctx);return s.prepare(op,{emitEvents:false});}catch(_){return null;}}});
      if(prepared.length!==1||!prepared[0].result)fail('PREPARACION_BLOQUEADA');
      extra={report:prepared[0].result};
    }
    await rpc(port,token,'ack',{schema:'LiveReadAckV1',session_id:session,command_id:cmd.command_id,status:'OK',context:before.context,html:snap.html,...extra});
    const wasPrepare=cmd.command==='validate_prepare_only';cmd=null;
    status.textContent=wasPrepare?'Valores preparados sin eventos ni guardado. Revisa la bitácora y ciérrala sin guardar.':'Lectura entregada al Asistente. Consulta allí el resultado del parser. No se ha registrado nada.';
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
