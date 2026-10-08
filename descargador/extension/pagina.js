/* Esta funcion se ejecuta en el formulario. No usa document.cookie ni almacenes. */
async function sitfaTask(command, payload) {
  let diaryPhase='CONTEXTO';
  try {
  if (location.origin !== "https://familia.pjud.cl") return null;
  const form = document.querySelector('form[name="InformesPpalForm"],form[name="MaoPpalForm"]');
  if (!form) return null;
  const carga=form.name==="MaoPpalForm";
  const endpoint = "https://familia.pjud.cl/SITFAWEB/"+(carga?"MaoDAction.do":"InformesDAction.do");
  if (new URL(form.action,location.href).href!==endpoint || form.method.toLowerCase()!=="post") return null;
  const screen=carga&&form.querySelector('[name="COD_TribunalDist"]')?"carga":form.querySelector('[name="COD_Lengueta"]')&&document.getElementById("tablaEspera")?"seguimiento":
    form.querySelector('[name="COD_EstBusqueda"]')?"litigantes":
    form.querySelector('#ExcelInformesPorVencer1,[name="ExcelInformesPorVencer1"]')?"calendario_informes":
    form.querySelector('#ExcelMedidasPorVencer1,[name="ExcelMedidasPorVencer1"]')?"calendario_medidas":null;
  if (!screen) return null;
  if(command==='capabilities') return {tipo:'CSMP_RUS_CAPACIDADES',version:'2.7.2',
    protocolos:{bitacoras_lectura:1,descarga_conjunta:1,pdf_seleccion:1},escritura_rus:false};
  const actions={seguimiento:"Buscar medida",litigantes:"Consulta Informe",calendario_informes:"Buscar Inf.",calendario_medidas:"Buscar",carga:"Aud.Carga Func.-"};
  const menus={seguimiento:"89",litigantes:"94",calendario_informes:"91",calendario_medidas:"90",carga:"29"};
  const fields={seguimiento:`COD_Centro GLS_TribunalOrigen FLG_Consulta COD_Lengueta
    COD_Tribunal_sel TIP_Consulta COD_CentroResidencial COD_PlazoIntervencion TIP_Informe
    COD_TiempoEspera TIP_Causa ROL_Causa ERA_Causa RUT_Consulta RUT_DvConsulta
    COD_TipLitigante CHK_Consulta FEC_Inicio FEC_Fin irAccion NUM_PaginaEspera
    NUM_TotalEspera NUM_PaginaCumplimiento NUM_TotalCumplimiento NUM_PaginaInforme
    NUM_TotalInforme NUM_PaginaEgreso NUM_TotalEgreso`,
    litigantes:`COD_Centro GLS_TribunalOrigen FLG_Consulta COD_Tribunal_sel TIP_Consulta TIP_Causa ROL_Causa ERA_Causa RUT_Consulta RUT_DvConsulta CHK_Consulta FEC_Inicio FEC_Fin irAccion COD_Medida COD_EstBusqueda NUM_PaginaInforme NUM_Total`,
    calendario_informes:`FLG_Busqueda COD_TribunalActual GLS_TribunalActual busquedaPorTribunal TIP_Consulta COD_Tribunal_sel FEC_Inicio COD_Mes_sel COD_Anio_Sel CHK_Contrae irAccion`,
    calendario_medidas:`FLG_Busqueda COD_TribunalActual GLS_TribunalActual busquedaPorTribunal TIP_Consulta COD_Tribunal_sel FEC_Inicio COD_Mes_sel COD_Anio_Sel CHK_Contrae irAccion`,
    carga:`GLS_Tribunal COD_Rus COD_TribunalDist FEC_Desde FEC_Hasta irAccion`};
  const allowed = new Set(fields[screen].split(/\s+/));
  const validPairs = pairs => Array.isArray(pairs) && pairs.length<=100 &&
    pairs.every(p=>Array.isArray(p)&&p.length===2&&allowed.has(p[0])&&typeof p[1]==="string") &&
    new Set(pairs.map(([k])=>k)).size===pairs.length && new Map(pairs).get("irAccion")===actions[screen];
  const evidenceBytes = doc => {
    const copy=doc.documentElement.cloneNode(true);
    const live=doc.querySelectorAll("input,select,textarea"),clones=copy.querySelectorAll("input,select,textarea");
    live.forEach((e,i)=>{
      const c=clones[i];if(e.tagName==="SELECT")Array.from(e.options).forEach((o,j)=>c.options[j].toggleAttribute("selected",o.selected));
      else if(e.tagName==="TEXTAREA")c.textContent=e.value;
      else {c.setAttribute("value",e.value);if(["checkbox","radio"].includes(e.type))c.toggleAttribute("checked",e.checked);}
    });
    return new TextEncoder().encode(copy.outerHTML);
  };
  const sha256 = async data => Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256",data))).map(x=>x.toString(16).padStart(2,"0")).join("");
  const restoreEvidenceSelection = (doc,win,pairs) => {
    const evidenceForm=doc.querySelector('form[name="InformesPpalForm"],form[name="MaoPpalForm"]');
    if(!evidenceForm)throw new Error("estructura");
    const submitted=new Map(pairs);
    const localTabMap={Espera:"tdEspera", Cumplimiento:"tdCumplimiento", Informes:"tdInforme", Egresados:"tdEgreso"};
    const defaultSelectText=name=>({
      COD_Tribunal_sel:"Consulta reconstruida",
      COD_TribunalDist:"Consulta reconstruida",
      TIP_Consulta:"Consulta reconstruida",
      TIP_Informe:"Consulta reconstruida",
      COD_EstBusqueda:"Consulta reconstruida",
      COD_Mes_sel:"Consulta reconstruida",
      COD_Anio_Sel:"Consulta reconstruida",
      COD_CentroResidencial:"Consulta reconstruida",
      COD_PlazoIntervencion:"Consulta reconstruida",
      COD_TiempoEspera:"Consulta reconstruida",
      TIP_Causa:"Consulta reconstruida",
      COD_TipLitigante:"Consulta reconstruida"
    }[name]||`Seleccionado (${valuePreview(name)})`);
    const valuePreview=name=>submitted.get(name)||"";
    const controls=name=>Array.from(evidenceForm.elements).filter(e=>e.name===name);
    const ensureSelectOption=(e,name,value)=>{
      if(Array.from(e.options).some(o=>o.value===value))return;
      const option=doc.createElement("option");
      option.value=value;
      option.textContent=defaultSelectText(name);
      option.setAttribute("data-sitfa-restored","1");
      e.append(option);
    };
    const ensureDateEnabled=name=>{for(const e of controls(name)){e.disabled=false;e.readOnly=false;}};
    const apply=(name,value,notify=false)=>{
      // La paginación pertenece a la respuesta del servidor, no al aspecto del PDF.
      if(/^NUM_(Pagina|Total)/.test(name))return false;
      const list=controls(name);if(!list.length)return false;
      let applied=false;
      for(const e of list) {
        if(e.type==="radio"||e.type==="checkbox") {
          e.checked=e.value===value;applied=applied||e.checked;
          if(e.checked)e.setAttribute("checked","checked"); else e.removeAttribute("checked");
          continue;
        }
        if(e.tagName==="SELECT") {
          ensureSelectOption(e,name,value);
          e.value=value;applied=true;
          Array.from(e.options).forEach(o=>o.toggleAttribute("selected",o.value===value));
          if(notify)e.dispatchEvent(new win.Event("change",{bubbles:true}));
          continue;
        }
        e.disabled=false;
        e.value=value;
        e.setAttribute("value",value);
        applied=true;
      }
      return applied;
    };
    const activateTab=tab=>{
      if(!tab||!Object.values(localTabMap).includes(tab))return;
      try {
        if(typeof win.Lengueta==="function")win.Lengueta(tab);
        else doc.getElementById(tab)?.click();
      } catch(_) {}
      apply("COD_Lengueta",tab,false);
      // Refuerza el estado visual de pestañas en la evidencia.
      for(const id of Object.values(localTabMap)) {
        const item=doc.getElementById(id);if(!item)continue;
        item.setAttribute("data-sitfa-tab",id===tab?"active":"inactive");
        if(id===tab){item.style.outline="2px solid #1b4f9c";item.style.outlineOffset="-2px";}
      }
    };
    // La respuesta "sin registros" de SITFA puede volver a los valores por defecto.
    // La evidencia se corrige solo dentro del iframe usado para el PDF; la página
    // original del usuario no se modifica.
    if(screen==="seguimiento")activateTab(submitted.get("COD_Lengueta"));
    // Orden importante: tribunal/modalidad pueden reconstruir otros selectores.
    for(const name of ["COD_Tribunal_sel","COD_TribunalDist"])if(submitted.has(name))apply(name,submitted.get(name),true);
    if(submitted.has("TIP_Consulta"))apply("TIP_Consulta",submitted.get("TIP_Consulta"),true);
    if(submitted.has("TIP_Informe"))apply("TIP_Informe",submitted.get("TIP_Informe"),true);
    for(const [name,value] of pairs)if(!["COD_Tribunal_sel","COD_TribunalDist","TIP_Consulta","TIP_Informe"].includes(name))apply(name,value,false);
    if(screen==="seguimiento"||screen==="litigantes") {
      const checked=submitted.get("FLG_Consulta")==="1"||submitted.has("CHK_Consulta");
      const check=controls("CHK_Consulta")[0];if(check){check.checked=checked; if(checked)check.setAttribute("checked","checked"); else check.removeAttribute("checked");}
      try {if(typeof win.Seleccion==="function")win.Seleccion();} catch(_) {}
      ensureDateEnabled("FEC_Inicio");ensureDateEnabled("FEC_Fin");
      // Seleccion() puede reponer las fechas de hoy; prevalece la consulta enviada.
      for(const name of ["FEC_Inicio","FEC_Fin","FLG_Consulta"])if(submitted.has(name))apply(name,submitted.get(name),false);
      if(check){check.checked=checked; if(checked)check.setAttribute("checked","checked"); else check.removeAttribute("checked");}
      if(submitted.has("COD_Lengueta"))activateTab(submitted.get("COD_Lengueta"));
    } else if(screen.startsWith("calendario_")) {
      try {if(typeof win.cambiacheck==="function")win.cambiacheck();} catch(_) {}
      for(const [name,value] of pairs)apply(name,value,false);
      ensureDateEnabled("FEC_Inicio");
      ensureDateEnabled("FEC_Fin");
    }
    // Marca el formulario de evidencia reconstruido y fuerza un último repaso.
    evidenceForm.setAttribute("data-sitfa-evidence-restored","1");
    for(const [name,value] of pairs)apply(name,value,false);
  };
  const evidenceClose = () => {
    const run=window.__sitfaRun;
    if(run?.evidence) {
      run.evidence.remove();
      for(const item of run.evidenceScroll||[])try{item.window.scrollTo(item.x,item.y);}catch(_){}
      delete run.evidence;delete run.evidenceFrame;delete run.evidenceDigest;delete run.evidenceDocument;delete run.evidenceScroll;
    }
    const cover=document.getElementById("sitfaDescargaLocal");if(cover)cover.style.visibility="visible";
  };
  if(command==="evidence_close") {evidenceClose();return {ok:true};}
  if(command==="evidence_open") {
    if(!window.__sitfaRun || !validPairs(payload.pairs)) throw new Error("estructura");
    evidenceClose();
    const panel=document.createElement("div");panel.id="sitfaEvidenciaLocal";
    panel.style.cssText="position:fixed;inset:0;z-index:2147483646;background:white;display:flex;flex-direction:column";
    const frame=document.createElement("iframe");frame.name="sitfaEvidenciaConsulta";
    frame.style.cssText="width:100%;flex:1;min-height:0;border:0;background:white";
    panel.append(frame);document.body.append(panel);
    const blocker=document.createElement("div");blocker.style.cssText="position:absolute;inset:0;z-index:1";panel.append(blocker);
    window.__sitfaRun.evidence=panel;window.__sitfaRun.evidenceFrame=frame;
    try {
      await new Promise((resolve,reject)=>{
        const timer=setTimeout(()=>reject(new Error("tiempo")),60000);
        frame.addEventListener("load",()=>{
          try {
            if(frame.contentWindow.location.href==="about:blank")return;
            if(frame.contentWindow.location.href!==endpoint)throw new Error("sesion");
            clearTimeout(timer);resolve();
          } catch(error) {clearTimeout(timer);reject(error);}
        });
        // POST de consulta nativo: Chrome conserva la autenticación y renderiza SITFA.
        const request=document.createElement("form");request.method="post";request.action=endpoint;request.target=frame.name;
        request.style.display="none";
        for(const [name,value] of payload.pairs) {
          const e=document.createElement("input");e.type="hidden";e.name=name;e.value=value;request.append(e);
        }
        document.body.append(request);
        try {HTMLFormElement.prototype.submit.call(request);} finally {request.remove();}
      });
      // Refleja los valores actuales de los controles sin modificar la página.
      const doc=frame.contentDocument;
      restoreEvidenceSelection(doc,frame.contentWindow,payload.pairs);
      const data=evidenceBytes(doc),digest=await sha256(data);
      if(data.length>80*1024*1024)throw new Error("estructura");
      window.__sitfaRun.evidenceDigest=digest;
      window.__sitfaRun.evidenceDocument=doc;
      const chunks=[];for(let i=0;i<data.length;i+=32768)chunks.push(String.fromCharCode(...data.subarray(i,i+32768)));
      return {status:200,type:"text/html; charset=utf-8",body:btoa(chunks.join("")),digest};
    } catch(error) {evidenceClose();throw error;}
  }
  if(command==="evidence_show") {
    const run=window.__sitfaRun;
    if(!run?.evidenceFrame || payload.digest!==run.evidenceDigest || !/^[a-f0-9]{64}$/.test(payload.digest))throw new Error("estructura");
    const doc=run.evidenceFrame.contentDocument;
    if(run.evidenceFrame.contentWindow.location.href!==endpoint || doc.querySelector('input[type="password"]'))throw new Error("sesion");
    // Se conserva el DOM validado: la captura no acepta una navegación posterior.
    if(run.evidenceDocument!==doc || await sha256(evidenceBytes(doc))!==payload.digest)throw new Error("cambio");
    if(!run.evidenceScroll) {
      run.evidenceScroll=[];
      let w=window;
      while(w!==w.top) {
        const element=w.frameElement;if(!element)break;
        const parent=w.parent;run.evidenceScroll.push({window:parent,x:parent.scrollX,y:parent.scrollY});
        element.scrollIntoView({block:"start",inline:"start",behavior:"instant"});w=parent;
      }
    }
    if(run.evidenceFrame.clientWidth<400 || run.evidenceFrame.clientHeight<200)throw new Error("pantalla");
    document.getElementById("sitfaDescargaLocal").style.visibility="hidden";
    return {ok:true};
  }
  if (command==="navigate") {
    if (!Object.hasOwn(menus,payload.screen) || window.__sitfaRun) throw new Error("estructura");
    const target="https://familia.pjud.cl/SITFAWEB/"+(payload.screen==="carga"?"MAOViewAccion.do?TipMenuMAO=":"InformesViewAccion.do?TipMenuINF=")+menus[payload.screen];
    setTimeout(()=>location.assign(target),100);return {navigating:true};
  }
  const field = name => form.querySelector('[name="'+name+'"]');
  const tabMap = {Espera:"tdEspera", Cumplimiento:"tdCumplimiento", Informes:"tdInforme", Egresados:"tdEgreso"};
  const activeTab = () => typeof idLengueta === "string" ? idLengueta : field("COD_Lengueta")?.value||"";
  const changeTab = id => {
    if (!document.getElementById(id)) throw new Error("estructura");
    if (activeTab() !== id) {
      if (typeof Lengueta === "function") Lengueta(id);
      else document.getElementById(id).click();
    }
  };
  const options = name => Object.fromEntries(Array.from(field(name)?.options || [])
    .filter(o => /^[1-9]\d{0,7}$/.test(o.value) || (name==="COD_Tribunal_sel" && screen.startsWith("calendario_") && o.value==="-1"))
    .map(o=>[o.value,o.textContent.trim().slice(0,160)]));
  const select = (name,value) => {
    const e=field(name);
    if (!e) throw new Error("estructura");
    if (e.tagName === "SELECT" && !Array.from(e.options).some(o=>o.value === value))
      throw new Error("opcion");
    if (e.value !== value) {e.value=value;e.dispatchEvent(new Event("change",{bubbles:true}));}
  };
  const snapshot = () => Array.from(form.elements).filter(e=>e.name && allowed.has(e.name))
    .map(e=>({name:e.name,value:e.value,checked:e.checked,disabled:e.disabled,type:e.type}));
  const restoreField = item => {
    const e=item.type==="radio"?Array.from(form.elements).find(e=>e.name===item.name&&e.type==="radio"&&e.value===item.value):field(item.name);if (!e) return;
    if (e.tagName === "SELECT" && !Array.from(e.options).some(o=>o.value===item.value))
      throw new Error("filtro");
    e.value=item.value;if (["checkbox","radio"].includes(e.type)) e.checked=item.checked;
    return e;
  };
  if (command === "catalog") {
    if(screen==="carga") return {pantalla:screen,tribunales:options("COD_TribunalDist"),modalidades:{},informes:{},medidas:{},pestanas:[],estados:{},meses:{},anios:{},
      capacidades:{consulta:true,firmas_cobertura_completa:false},seleccion:{desde:field("FEC_Desde")?.value,hasta:field("FEC_Hasta")?.value}};
    if (screen!=="seguimiento") {
      let tribunales=options("COD_Tribunal_sel");
      // La opción global no se mezcla con tribunales individuales en un lote.
      if (Object.keys(tribunales).some(k=>k!=="-1")) delete tribunales["-1"];
      return {pantalla:screen,tribunales,modalidades:{},informes:{},medidas:{},pestanas:[],
        estados:options("COD_EstBusqueda"),meses:options("COD_Mes_sel"),anios:options("COD_Anio_Sel"),
        seleccion:{estado:field("COD_EstBusqueda")?.value,mes:field("COD_Mes_sel")?.value,anio:field("COD_Anio_Sel")?.value}};
    }
    const before=activeTab(), reportBefore=field("TIP_Informe")?.value;
    let reports,measures;
    try {changeTab("tdCumplimiento");measures=options("TIP_Informe");changeTab("tdInforme");reports=options("TIP_Informe");}
    finally {
      if (before) changeTab(before);
      if (field("TIP_Informe") && Array.from(field("TIP_Informe").options).some(o=>o.value===reportBefore))
        field("TIP_Informe").value=reportBefore;
    }
    return {pantalla:screen,tribunales:options("COD_Tribunal_sel"), modalidades:options("TIP_Consulta"), informes:reports,medidas:measures,
      pestanas:Object.keys(tabMap).filter(k=>document.getElementById(tabMap[k])),
      seleccion:{tribunal:field("COD_Tribunal_sel").value, modalidad:field("TIP_Consulta").value,
        pestaña:before,informe:reportBefore,desde:field("FEC_Inicio")?.value,hasta:field("FEC_Fin")?.value}};
  }
  if (command === "lock") {
    if (window.__sitfaRun) throw new Error("ocupado");
    window.__sitfaRun={fields:snapshot(),tab:activeTab()};
    const cover=document.createElement("div");cover.id="sitfaDescargaLocal";
    cover.textContent="Descargando el lote. Usa Cancelar en la herramienta para detenerlo.";
    cover.style.cssText="position:fixed;inset:0;z-index:2147483647;background:#f3f6fa;padding:48px;font:22px sans-serif;color:#20324d";
    document.body.appendChild(cover);
    window.__sitfaRun.blockKey=e=>{e.preventDefault();e.stopImmediatePropagation();};
    window.addEventListener("keydown",window.__sitfaRun.blockKey,true);
    return {ok:true};
  }
  if (command === "unlock") {
    evidenceClose();
    const original=window.__sitfaRun;
    let restored=true;
    try {
      if (original) {
        try {if (screen==="seguimiento" && original.tab) changeTab(original.tab);} catch (_) {restored=false;}
        for (const item of original.fields) {
          try {const e=restoreField(item);if(e)e.disabled=item.disabled;else restored=false;}
          catch (_) {restored=false;}
        }
      }
    } finally {
      document.getElementById("sitfaDescargaLocal")?.remove();
      if (original) window.removeEventListener("keydown",original.blockKey,true);
      delete window.__sitfaRun;
    }
    return {ok:true,restored};
  }
  if (!window.__sitfaRun) throw new Error("ocupado");
  if (command === "prepare") {
    const s=payload.selection;
    if ((s.screen||"seguimiento")!==screen || (!/^[1-9]\d*$/.test(s.tribunal) && !(screen.startsWith("calendario_")&&s.tribunal==="-1")))
      throw new Error("opcion");
    if(screen==="carga") {
      select("COD_TribunalDist",s.tribunal);
      const day=value=>{
        if(typeof value!=="string"||!/^\d{2}\/\d{2}\/\d{4}$/.test(value))throw new Error("filtro");
        const [d,m,y]=value.split("/").map(Number),date=new Date(Date.UTC(y,m-1,d));
        if(date.getUTCFullYear()!==y||date.getUTCMonth()!==m-1||date.getUTCDate()!==d)throw new Error("filtro");return date;
      };
      const difference=(day(s.end)-day(s.start))/86400000;
      if(difference<0||difference>=30)throw new Error("filtro");
      field("FEC_Desde").value=s.start;field("FEC_Hasta").value=s.end;
      const button=form.querySelector('input[type="submit"][name="irAccion"][value="Aud.Carga Func.-"]');
      if(!button)throw new Error("estructura");
      const pairs=Array.from(new FormData(form,button).entries());
      if(!pairs.some(([k])=>k==="irAccion"))pairs.push(["irAccion",actions[screen]]);
      if(!validPairs(pairs))throw new Error("estructura");
      return {pairs};
    }
    select("COD_Tribunal_sel",s.tribunal);
    if(screen==="seguimiento") {
      if (!tabMap[s.tab] || !["1","2","3","4"].includes(s.modality)) throw new Error("opcion");
      changeTab(tabMap[s.tab]);select("TIP_Consulta",s.modality);
    } else if(screen==="litigantes") select("COD_EstBusqueda",s.state);
    else {
      select("COD_Mes_sel",s.month);select("COD_Anio_Sel",s.year);
      const radios=Array.from(form.elements).filter(e=>e.name==="TIP_Consulta"&&e.type==="radio");
      if (!radios.some(e=>e.value==="1")) throw new Error("estructura");
      for (const e of radios) e.checked=e.value==="1";
    }
    if (screen==="seguimiento" && s.tab === "Informes") select("TIP_Informe",s.report);
    else if (s.tab === "Cumplimiento" && field("TIP_Informe")) {
      select("TIP_Informe",s.report || "0");
    }
    // Primero todas las selecciones basicas; sus manejadores pueden reiniciar filtros.
    const filters=new Set(["COD_CentroResidencial","COD_PlazoIntervencion","COD_TiempoEspera","TIP_Causa",
      "ROL_Causa","ERA_Causa","RUT_Consulta","RUT_DvConsulta","COD_TipLitigante","COD_Medida"]);
    if (payload.keep_filters && !screen.startsWith("calendario_")) {
      for (const item of window.__sitfaRun.fields) if (filters.has(item.name)) restoreField(item);
    } else if (!screen.startsWith("calendario_")) {
      for (const name of filters) {
        const e=field(name);if (!e) continue;
        if (e.tagName === "SELECT") {
          const reset=Array.from(e.options).find(o=>["-1","0",""].includes(o.value));
          if (!reset) throw new Error("filtro");e.value=reset.value;
        } else e.value=["RUT_Consulta","RUT_DvConsulta","ROL_Causa"].includes(name) ? "" : "0";
      }
    }
    const check=field("CHK_Consulta"),from=field("FEC_Inicio"),to=field("FEC_Fin");
    if (!screen.startsWith("calendario_")) {
      if (!check || !from || !to || !field("FLG_Consulta")) throw new Error("estructura");
      check.checked=Boolean(payload.dates);
      if (typeof Seleccion === "function") Seleccion();
      if (payload.dates) {from.disabled=false;to.disabled=false;from.value=payload.dates.start;to.value=payload.dates.end;}
    }
    for (const e of form.elements) if (/^NUM_(Pagina|Total)/.test(e.name)) e.value="";
    const oldAlert=window.alert;let rejected=false;
    try {
      window.alert=()=>{rejected=true;};
      if(screen.startsWith("calendario_")) {
        if(typeof cambiacheck!=="function") throw new Error("estructura");
        cambiacheck();
      } else {
        if(typeof Envio!=="function") throw new Error("estructura");
        if(Envio()===false) rejected=true;
      }
      if(rejected) throw new Error("filtro");
    }
    finally {window.alert=oldAlert;}
    if (screen==="seguimiento") field("COD_Lengueta").value=tabMap[s.tab];
    if (screen==="litigantes") field("TIP_Consulta").value="2";
    if (!screen.startsWith("calendario_")) field("FLG_Consulta").value=payload.dates ? "1" : "0";
    const button=form.querySelector('input[type="submit"][name="irAccion"][value="'+actions[screen]+'"]');
    if (!button || new URL(form.action,location.href).href!==endpoint || form.method.toLowerCase()!=="post")
      throw new Error("estructura");
    const pairs=Array.from(new FormData(form,button).entries());
    if (!pairs.some(([k])=>k==="irAccion")) pairs.push(["irAccion",actions[screen]]);
    if (pairs.some(([k,v])=>!allowed.has(k) || typeof v!=="string") ||
        new Set(pairs.map(([k])=>k)).size!==pairs.length) throw new Error("estructura");
    return {pairs};
  }
  let url,init;
  let diaryParams=null,startedAt=new Date().toISOString();
  if(command==="diary_open") {
    const run=window.__sitfaRun,expected=payload.params;
    if(screen!=="seguimiento" || !run?.diaryDocument || !expected || typeof expected!=="object")throw new Error("DIARY_CONTEXT");
    diaryPhase='IDENTIDAD';
    const names=["tipo_popUp","CRR_IdCausa","COD_Tribunal","TIP_Consulta","ID_Ingreso","COD_Etapa","FLG_MejorNinez"];
    if(Object.keys(expected).length!==names.length || names.some(k=>typeof expected[k]!=="string"||!/^\d{1,20}$/.test(expected[k])) ||
       expected.tipo_popUp!=="12" || !["1","2","3","4"].includes(expected.TIP_Consulta))throw new Error("estructura");
    const selected=new Map(run.diaryPairs);
    if(expected.COD_Tribunal!==selected.get("COD_Tribunal_sel")||expected.TIP_Consulta!==selected.get("TIP_Consulta"))throw new Error("estructura");
    const tableIds={tdEspera:"tablaEspera",tdCumplimiento:"tablaCumplimiento",tdInforme:"tablaInforme",tdEgreso:"tablaEgresados"};
    const table=run.diaryDocument.getElementById(tableIds[selected.get("COD_Lengueta")]);
    if(!table)throw new Error("estructura");
    const tuple=[expected.CRR_IdCausa,expected.COD_Tribunal,expected.ID_Ingreso,"12",expected.COD_Etapa,expected.FLG_MejorNinez];
    let matches=0;
    for(const el of table.querySelectorAll("[onclick],[href]"))for(const attr of ["onclick","href"]) {
      for(const match of (el.getAttribute(attr)||"").matchAll(/\bShowObservaciones\s*\(([^()]*)\)/g)) {
        const args=match[1].split(",").map(s=>{
          const m=/^(?:(\d{1,20})|'(\d{1,20})'|"(\d{1,20})")$/.exec(s.trim());return m?(m[1]||m[2]||m[3]):null;
        });
        if(args.length===6 && args.every((v,i)=>v===tuple[i]))matches++;
      }
    }
    if(matches!==1)throw new Error("DIARY_LINK");
    diaryParams=expected;
    url="https://familia.pjud.cl/SITFAWEB/IrPopUpInformesAccion.do?"+new URLSearchParams(names.map(k=>[k,expected[k]]));
    init={method:"GET"};
  } else
  if (command === "search") {
    const pairs=payload.pairs;
    if (!validPairs(pairs))
      throw new Error("estructura");
    url=endpoint;init={method:"POST",body:new URLSearchParams(pairs)};
    delete window.__sitfaRun.diaryDocument;delete window.__sitfaRun.diaryPairs;
  } else if (command === "download") {
    const u=new URL(payload.url);
    if (u.origin!=="https://familia.pjud.cl" || u.search || u.hash || !/^\/sitfa\/reportes\/[A-Za-z0-9_-]+\.xls$/.test(u.pathname))
      throw new Error("estructura");
    url=u.href;init={method:"GET"};
  } else throw new Error("estructura");
  diaryPhase='RED';
  const response=await fetch(url,{...init,credentials:"include",cache:"no-store",redirect:"error",
    signal:AbortSignal.timeout(Math.min(payload.timeout||60000,60000))});
  diaryPhase='RESPUESTA';
  const reader=response.body?.getReader();
  if(!reader)throw new Error("estructura");
  const received=[];let length=0;
  try {
    while(true) {
      const {done,value}=await reader.read();if(done)break;
      length+=value.byteLength;
      if(length>80*1024*1024){await reader.cancel();throw new Error("estructura");}
      received.push(value);
    }
  } finally {reader.releaseLock();}
  const bytes=new Uint8Array(length);let offset=0;
  for(const value of received){bytes.set(value,offset);offset+=value.length;}
  if (response.status!==200 || !bytes.length || bytes.length>80*1024*1024) throw new Error("sesion");
  const chunks=[];
  for (let i=0;i<bytes.length;i+=32768) chunks.push(String.fromCharCode(...bytes.subarray(i,i+32768)));
  if(command==="search" && screen==="seguimiento") {
    const type=response.headers.get("content-type")||"";
    if(!type.toLowerCase().includes("html"))throw new Error("estructura");
    let html;
    try {html=new TextDecoder("utf-8",{fatal:true}).decode(bytes);}
    catch (_) {html=new TextDecoder("windows-1252").decode(bytes);}
    window.__sitfaRun.diaryDocument=new DOMParser().parseFromString(html,"text/html");
    window.__sitfaRun.diaryPairs=payload.pairs;
  }
  return {status:response.status,type:response.headers.get("content-type")||"",body:btoa(chunks.join("")),
    ...(diaryParams?{params:diaryParams,startedAt,digest:await sha256(bytes)}:{})};
  } catch (error) {
    if(command==='diary_open') {
      const explicit=['DIARY_CONTEXT','DIARY_LINK'].includes(error?.message)?error.message:null;
      const code=explicit || (diaryPhase==='RED'?(error?.name==='TimeoutError'?'DIARY_TIMEOUT':'DIARY_NETWORK'):
        diaryPhase==='IDENTIDAD'?'DIARY_IDENTITY':'DIARY_RESPONSE');
      return {__sitfaError:code};
    }
    return {__sitfaError:command==="evidence_open"?"EVIDENCE_OPEN":"SITFA_TASK"};
  }
}
