/* Contrato de estructura basado en cuatro inspecciones, 24-11-2026.
 * Exclusivamente lectura de DOM. No rellena, registra, navega ni autoriza.
 * Solo se usa en pruebas ficticias; no se carga en extensiones operativas.
 */
(() => {
  'use strict';
  const fail=code=>{throw new Error(code);};
  const norm=x=>String(x??'').normalize('NFC').replace(/\s+/g,' ').trim().toLowerCase();
  const modal='dialog,[role="dialog"],[aria-modal="true"]';
  const visible=n=>{
    if(!n||!n.isConnected||!n.getClientRects().length)return false;
    for(let p=n;p;p=p.parentElement){
      const s=p.ownerDocument.defaultView.getComputedStyle(p);
      if(p.hidden||p.inert||p.getAttribute('aria-hidden')==='true'||s.display==='none'||s.visibility!=='visible'||s.opacity==='0')return false;
    }
    return true;
  };
  const one=(nodes,reason)=>{if(nodes.length!==1)fail(reason);return nodes[0];};
  const required=Object.freeze({ESPERA:Object.freeze(['4','Gestión Previa']),CUMPLIMIENTO:Object.freeze(['6','Realizada']),INFORMES_POR_VENCER:Object.freeze(['9','Realizada']),INFORMES_VENCIDOS:Object.freeze(['9','Realizada'])});
  const headers=['tipo observación','etapa','fecha centro','usuario centro','observación centro','fecha tribunal','usuario tribunal','observación tribunal','acción'];
  function binding(doc,origin){
    if(typeof origin!=='string'||!Object.hasOwn(required,origin))fail('ORIGEN_INVALIDO');
    const [code,label]=required[origin];
    const form=one([...doc.forms].filter(f=>f.getAttribute('name')==='InformesPpalForm'),'FORMULARIO_NO_UNICO');
    if(doc.forms.length!==1||[...doc.querySelectorAll(modal)].some(visible))fail('CONTEXTO_AMBIGUO');
    const action=new URL(form.action);
    if(form.method.toLowerCase()!=='post'||action.origin!=='https://familia.pjud.cl'||action.pathname!=='/SITFAWEB/InformesDAction.do')fail('ACCION_INESPERADA');
    const all=[...doc.querySelectorAll('input,select,textarea,button')];
    const usable=n=>n.form===form&&form.contains(n)&&visible(n)&&!n.matches(':disabled')&&!n.closest(modal);
    for(const name of ['COD_Tribunal','CRR_IdCausa','ID_Ingreso','COD_Etapa','TIP_Consulta','tipo_popUp']){
      const n=one(all.filter(n=>n.name===name),'CONTEXTO_NO_UNICO');
      if(n.tagName!=='INPUT'||n.type!=='hidden'||n.form!==form||!form.contains(n))fail('CONTEXTO_ESTRUCTURAL_INCOMPLETO');
      // Nunca leer los valores de contexto en este detector.
    }
    const table=one([...doc.querySelectorAll('[id="TablaInforme"]')],'TABLA_NO_UNICA');
    if(table.tagName!=='TABLE'||!form.contains(table)||!visible(table))fail('TABLA_DISTINTA');
    const row=table.rows[0];
    if(!row||row.cells.length!==headers.length||[...row.cells].some((c,i)=>norm(c.textContent)!==headers[i]))fail('ENCABEZADOS_DISTINTOS');
    const named=(name,tag,type,id)=>{
      const n=one(all.filter(n=>n.name===name),'CONTROL_NO_UNICO');
      if(!usable(n)||n.tagName!==tag||(type&&n.type!==type)||(id&&n.id!==id))fail('CONTROL_DISTINTO');
      if(id&&[...doc.querySelectorAll('[id]')].filter(e=>e.id===id).length!==1)fail('ID_NO_UNICO');
      return n;
    };
    const state=named('COD_Estado_Detalle_Nuevo','SELECT',null,'COD_Estado_Detalle_Nuevo');
    const checkbox=named('chkBitacora','INPUT','checkbox');
    const editor=named('id_Obs_centro','TEXTAREA',null,'GLS_ObsCentro_Nueva');
    const counter=named('Txt_Cont2','INPUT','text','Txt_Cont2');
    const back=one(all.filter(n=>n.name==='volv'),'CONTROL_NO_UNICO');
    if(!usable(back)||!['INPUT','BUTTON'].includes(back.tagName)||back.type!=='button')fail('CONTROL_DISTINTO');
    const create=one(all.filter(n=>n.name==='irAccion'&&visible(n)),'CREACION_NO_UNICA');
    if(!usable(create)||!['INPUT','BUTTON'].includes(create.tagName)||create.type!=='submit'||
      norm(create.tagName==='INPUT'?create.value:create.textContent)!=='nueva obs'||
      create.hasAttribute('formaction')||create.hasAttribute('formmethod'))fail('CREACION_DISTINTA');
    // El mismo nombre irAccion existe en la edición oculta; jamás elegirlo por nombre solo.
    if(editor.readOnly||!counter.readOnly||state.multiple||checkbox.indeterminate)fail('CONTROL_DISTINTO');
    const expected='enviar observación al tribunal';
    const labels=[...doc.querySelectorAll('label')].filter(n=>visible(n)&&norm(n.textContent)===expected);
    if(labels.some(n=>n.control&&n.control!==checkbox))fail('ETIQUETA_CONTRADICTORIA');
    const nearby=[checkbox.previousElementSibling,checkbox.nextElementSibling].some(n=>n&&norm(n.textContent)===expected)||
      [...(checkbox.parentElement?.childNodes||[])].some(n=>n.nodeType===3&&norm(n.nodeValue)===expected)||
      [checkbox.closest('td,th')?.previousElementSibling,checkbox.closest('td,th')?.nextElementSibling].some(n=>n&&norm(n.textContent)===expected);
    if(!labels.some(n=>n.control===checkbox)&&!nearby&&norm(checkbox.getAttribute('aria-label'))!==expected)fail('ETIQUETA_NO_CONFIRMADA');
    const options=[...state.options].filter(o=>o.value===code);
    if(options.length!==1||options[0].disabled||options[0].closest('optgroup[disabled]')||norm(options[0].textContent)!==norm(label))fail('ESTADO_OBLIGATORIO_NO_DISPONIBLE');
    // Otros controles de entrada visibles deben revisarse, no se ignoran.
    const entry=all.filter(n=>usable(n)&&['INPUT','SELECT','TEXTAREA','BUTTON'].includes(n.tagName)&&n.type!=='hidden'&&!table.contains(n));
    if(entry.length!==6||[state,checkbox,editor,counter,create,back].some(n=>!entry.includes(n)))fail('FIRMA_DISTINTA');
    const report=Object.freeze({schema:'ObservedFormBindingV1',evidence:'CUATRO_VISTAS_OBSERVADAS_PRUEBA_FICTICIA',
      origin,states:Object.freeze([code]),context_fields:6,visible_entry_controls:6,table_headers:9,
      editor_maxlength:editor.maxLength>=0?editor.maxLength:null,write_enabled:false});
    return Object.freeze({report,form,table,state,checkbox,editor,counter,create,back});
  }
  function inspect(doc,origin){return binding(doc,origin).report;}
  function bindFixture(doc,origin){
    if(doc.location.href!=='about:blank'||doc.documentElement.dataset.fixture!=='OBSERVED_PREPARATION')fail('ENTORNO_NO_SINTETICO');
    return binding(doc,origin);
  }
  globalThis.RegistradorObservedForm=Object.freeze({inspect,bindFixture});
})();
