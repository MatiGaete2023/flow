const byId=id=>document.getElementById(id);
let running=false,connecting=false,endpoint=null,key=null,frameId=null,busy=false;
const tabId=Number(new URLSearchParams(location.search).get("tab"));
const messages={
  DIARY_CONTEXT:"No se conserva el listado actual para abrir bitácoras. Vuelve a consultar el lote sin navegar en RUS durante la lectura.",
  DIARY_LINK:"El enlace de la bitácora está ausente o repetido en el listado actual. No se abrió una causa por suposición.",
  DIARY_IDENTITY:"La identidad solicitada no coincide con el tribunal, modalidad e ingreso del listado actual.",
  DIARY_NETWORK:"La consulta de bitácora falló al contactar con RUS o recibió una redirección. Comprueba la sesión y reintenta el lote.",
  DIARY_TIMEOUT:"RUS no respondió a tiempo al abrir la bitácora. Reintenta las lecturas fallidas.",
  DIARY_RESPONSE:"RUS no entregó una respuesta válida de bitácora. Revisa la sesión y reintenta las lecturas fallidas.",
  CONNECTION_CODE:"El descargador rechazó el código. Copia el código actual de la ventana que está abierta; cambia al reiniciar o desconectar.",
  EXTENSION_ORIGIN:"El descargador rechazó el origen de la extensión. Actualiza el descargador y recarga la extensión en chrome://extensions.",
  EXTENSION_CHANGED:"Este descargador está vinculado a otra copia de la extensión. Pulsa Desconectar en el descargador y usa el nuevo código.",
  LOCAL_UNREACHABLE:"No se pudo contactar con el descargador de este equipo. Comprueba que siga abierto y pega su código actual.",
  LOCAL_TIMEOUT:"El descargador local no respondió a tiempo. Comprueba que siga abierto y vuelve a conectar.",
  LOCAL_PROTOCOL:"El descargador y la extensión no completaron el enlace. Actualiza ambos y vuelve a conectar.",
  SITE_ACCESS:"El enlace local funciona, pero Chrome no permite acceder a la pestaña SITFA. Comprueba que siga abierta y que la extensión tenga acceso a familia.pjud.cl. Abre de nuevo la conexión desde esa pestaña.",
  FOLLOWUP_REQUIRED:"El enlace local funciona, pero no se encuentra el formulario de Seguimiento. Entra a Seguimiento en la pestaña original de SITFA y vuelve a pulsar Cargar opciones en el descargador.",
  MULTIPLE_FORMS:"Se encontró más de un formulario de Seguimiento. Abre la conexión desde una pestaña con un solo formulario.",
  SITFA_TASK:"SITFA no respondió como se esperaba. Revisa la sesión y la pantalla de consulta elegida.",
  SCREEN_OPEN:"No se pudo abrir la pantalla elegida en SITFA. Revisa la sesión, entra a una pantalla de consulta y vuelve a Cargar opciones.",
  EVIDENCE_CAPTURE:"No se pudo capturar la consulta vacía. Mantén SITFA abierto en una ventana visible, pulsa de nuevo el icono de la extensión desde SITFA y vuelve a conectar. El lote conserva los archivos guardados.",
  EVIDENCE_OPEN:"No se pudo cargar la vista SITFA para el PDF de la consulta vacía. Revisa que la sesión siga abierta y actualiza la extensión y el descargador juntos. Los archivos anteriores se conservan.",
};
function status(message){byId("status").textContent=message;}
function failure(code){return Object.assign(new Error(code),{code});}
function safeCode(error,fallback){return Object.hasOwn(messages,error?.code)?error.code:fallback;}
async function bridge(path,body) {
  let response;
  try {
    response=await fetch(endpoint+path,{method:body===undefined?"GET":"POST",cache:"no-store",
      headers:{"X-Sitfa-Code":key,"X-Sitfa-Extension":chrome.runtime.id,
        ...(body===undefined?{}:{"Content-Type":"application/json"})},
      ...(body===undefined?{}:{body:JSON.stringify(body)}),signal:AbortSignal.timeout(70000)});
  } catch (error) {throw failure(error?.name==="TimeoutError"?"LOCAL_TIMEOUT":"LOCAL_UNREACHABLE");}
  let data;
  try {data=await response.json();} catch (_) {throw failure("LOCAL_PROTOCOL");}
  if (!response.ok) throw failure(["CONNECTION_CODE","EXTENSION_ORIGIN","EXTENSION_CHANGED"].includes(data?.error)?data.error:"LOCAL_PROTOCOL");
  return data;
}
async function inject(command,payload={}) {
  let results;
  try {
    results=await chrome.scripting.executeScript({target:frameId===null||command==="catalog"?{tabId,allFrames:true}:{tabId,frameIds:[frameId]},
      world:"MAIN",func:sitfaTask,args:[command,payload]});
  } catch (_) {throw failure("SITE_ACCESS");}
  const candidates=results.filter(r=>r.result!==null&&r.result!==undefined);
  if (!candidates.length) throw failure(command==="catalog"?"FOLLOWUP_REQUIRED":"SITFA_TASK");
  if (candidates.length!==1) throw failure("MULTIPLE_FORMS");
  if(candidates[0].result?.__sitfaError) throw failure(Object.hasOwn(messages,candidates[0].result.__sitfaError)?candidates[0].result.__sitfaError:"SITFA_TASK");
  frameId=candidates[0].frameId;return candidates[0].result;
}
async function execute(command,payload={}) {
  if(command==="evidence_capture") {
    let changed=false;
    const activated=info=>{if(info.windowId===windowId&&info.tabId!==tabId)changed=true;};
    let windowId;
    try {
      const tab=await chrome.tabs.get(tabId);windowId=tab.windowId;
      if(new URL(tab.url).origin!=="https://familia.pjud.cl" || tab.discarded)throw new Error("pestaña");
      const win=await chrome.windows.get(windowId);
      if(win.state==="minimized")await chrome.windows.update(windowId,{state:"normal"});
      await chrome.tabs.update(tabId,{active:true});
      chrome.tabs.onActivated.addListener(activated);
      await inject("evidence_show",payload);
      // Dar tiempo al compositor, también cuando la pestaña estaba en segundo plano.
      await new Promise(resolve=>setTimeout(resolve,600));
      const active=await chrome.tabs.query({windowId,active:true});
      if(changed || active.length!==1 || active[0].id!==tabId)throw new Error("pestaña");
      const png=await chrome.tabs.captureVisibleTab(windowId,{format:"png"});
      await inject("evidence_show",payload);
      if(changed || !(await chrome.tabs.get(tabId)).active || !png.startsWith("data:image/png;base64,"))throw new Error("pestaña");
      return {png:png.slice("data:image/png;base64,".length),digest:payload.digest};
    } catch (_) {throw failure("EVIDENCE_CAPTURE");}
    finally {chrome.tabs.onActivated.removeListener(activated);try {await inject("evidence_close");}catch(_) {}}
  }
  if(command!=="catalog" || !payload.screen) return inject(command,payload);
  if(!["seguimiento","litigantes","calendario_informes","calendario_medidas","carga"].includes(payload.screen)) throw failure("SCREEN_OPEN");
  const current=await inject("catalog");
  if(current.pantalla===payload.screen) return current;
  status("Abriendo la pantalla de consulta elegida en SITFA…");
  await inject("navigate",{screen:payload.screen});frameId=null;
  const until=Date.now()+15000;
  while(Date.now()<until) {
    await new Promise(resolve=>setTimeout(resolve,250));
    try {
      const catalog=await inject("catalog");
      if(catalog.pantalla===payload.screen) {
        status("Conectado. Opciones de la pantalla elegida cargadas.");return catalog;
      }
    } catch (_) {}
  }
  throw failure("SCREEN_OPEN");
}
async function disconnect() {
  running=false;
  if (frameId!==null) {try {await execute("unlock");} catch (_) {}}
  key=null;endpoint=null;frameId=null;connecting=false;
  byId("connect").disabled=false;byId("disconnect").disabled=true;byId("code").disabled=false;
}
byId("connect").addEventListener("click",async()=>{
  if (running || busy || connecting) return;
  if (!Number.isInteger(tabId) || tabId<=0) {status("Abre esta conexión pulsando el icono de la extensión desde la pestaña SITFA.");return;}
  const match=/^(\d{1,5})-([0-9a-f]{32})$/.exec(byId("code").value.trim());
  if (!match || Number(match[1])<1 || Number(match[1])>65535) {status("Copia el código completo desde el descargador.");return;}
  endpoint="http://127.0.0.1:"+match[1];key=match[2];frameId=null;
  connecting=true;byId("connect").disabled=true;byId("code").disabled=true;
  status("Comprobando el enlace con el descargador local…");
  try {
    // Validar el enlace local no depende de que Seguimiento ya esté abierto.
    await bridge("/connect",{});
    running=true;connecting=false;byId("code").value="";
    byId("connect").disabled=true;byId("disconnect").disabled=false;
    status("Conectado. Vuelve al descargador y pulsa Cargar opciones. Mantén abierta esta pestaña.");
    while (running) {
      const task=await bridge("/next");
      if (!running) break;
      if (task.idle) continue;
      busy=true;byId("disconnect").disabled=true;
      let result;
      try {result={id:task.id,ok:true,result:await execute(task.command,task.payload)};}
      catch (error) {
        const code=safeCode(error,"SITFA_TASK");
        result={id:task.id,ok:false,error:code};status(messages[code]);
      }
      await bridge("/complete",result);
      busy=false;byId("disconnect").disabled=false;
    }
  } catch (error) {
    status(messages[safeCode(error,"LOCAL_PROTOCOL")]);
  } finally {busy=false;await disconnect();}
});
byId("disconnect").addEventListener("click",()=>{
  if (busy || connecting) return;
  // Cambia la bandera; /next finaliza su espera acotada antes de limpiar el contexto.
  running=false;status("Desconectado. Para volver a conectar, copia el código actual del descargador.");
});
