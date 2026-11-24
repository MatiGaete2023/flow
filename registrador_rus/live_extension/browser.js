/* Utilidades de navegación local; ninguna acción de escritura. */
(() => {
'use strict';
const SITE='https://familia.pjud.cl', POST='/SITFAWEB/InformesDAction.do', OPEN='/SITFAWEB/IrPopUpInformesAccion.do';
const access={origins:[SITE+'/*']};
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

async function selectPair(){
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

    return {l,p};
}
async function ensurePopup(p,documentId){
    const current=await chrome.tabs.get(p.id);
    if(current.status!=='complete'||route(current)!==route(p)||current.windowId!==p.windowId)fail('BITACORA_CAMBIADA');
    const alive=await chrome.scripting.executeScript({target:{tabId:p.id,documentIds:[documentId]},world:'ISOLATED',func:()=>true});
    if(alive.length!==1||alive[0].result!==true)fail('BITACORA_CAMBIADA');
}
globalThis.FlowLiveBrowser=Object.freeze({access,fail,route,rpc,locate,capture,selectPair,ensurePopup});
})();
