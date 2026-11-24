/* Inspección focalizada de estructura, no lectura de observaciones ni escritura. */
function inspectFocusDocument(doc) {
  'use strict';
  const blocked = reason => ({schema:'FocusedEvidenceV1', status:'BLOCKED', reason, write_enabled:false});
  const norm = value => String(value ?? '').normalize('NFC').replace(/\s+/g,' ').trim();
  const visible = node => {
    if (!node || !node.isConnected || !node.getClientRects().length) return false;
    for (let n=node; n; n=n.parentElement) {
      const css = doc.defaultView.getComputedStyle(n);
      if (n.hidden || n.inert || n.getAttribute('aria-hidden')==='true' || css.display==='none' || css.visibility!=='visible' || css.opacity==='0') return false;
    }
    return true;
  };
  const kind = n => n.tagName==='SELECT'?'select':n.tagName==='TEXTAREA'?'textarea':
    n.tagName==='BUTTON'||(n.tagName==='INPUT'&&['button','submit','image'].includes(n.type))?'button':n.type;
  const technical = value => value===null||value===''?null:
    /^[A-Za-z_][A-Za-z_0-9-]{0,63}$/.test(value)&&!/[0-9]{3,}/.test(value)?value:'OMITIDO';
  const attributes = n => ({id:technical(n.getAttribute('id')),name:technical(n.getAttribute('name'))});
  try {
    const modal='dialog, [role="dialog"], [aria-modal="true"]';
    if ([...doc.querySelectorAll(modal)].some(visible)) return blocked('MODAL_VISIBLE');
    const allForms=[...doc.forms];
    const named=allForms.filter(f=>f.getAttribute('name')==='InformesPpalForm');
    const ownedVisible=f=>[...f.querySelectorAll('input,select,textarea,button')].some(n=>n.form===f&&visible(n));
    const forms=named.filter(f=>visible(f)||ownedVisible(f));
    if(named.length!==1||forms.length!==1) {
      const count=items=>Math.min(items.length,255);
      const actionMatches=f=>{try {const u=new URL(f.getAttribute('action')||'',doc.baseURI);return u.origin==='https://familia.pjud.cl'&&u.pathname==='/SITFAWEB/InformesDAction.do';}catch(_){return false;}};
      return {...blocked('FORMULARIO_NO_UNICO'),diagnostic:{
        forms:count(allForms),named:count(named),by_id:count(allForms.filter(f=>f.id==='InformesPpalForm')),
        named_box:count(named.filter(visible)),named_controls:count(named.filter(ownedVisible)),
        action_matches:count(allForms.filter(actionMatches)),tables:count([...doc.querySelectorAll('[id="TablaInforme"]')]),
        states:count([...doc.querySelectorAll('select[name="COD_Estado_Detalle_Nuevo"]')]),
        visible_frames:count([...doc.querySelectorAll('frame,iframe')].filter(visible))}};
    }
    const form=forms[0], action=new URL(form.action);
    if(form.method.toLowerCase()!=='post'||action.origin!=='https://familia.pjud.cl'||action.pathname!=='/SITFAWEB/InformesDAction.do') return blocked('ACCION_INESPERADA');
    const matches=[...doc.querySelectorAll('[id="TablaInforme"]')];
    const tables=matches.filter(n=>n.tagName==='TABLE');
    const inside=tables.filter(n=>form.contains(n));
    const tableOk=matches.length===1&&inside.length===1&&visible(inside[0]);
    const contextKeys=['COD_Tribunal','CRR_IdCausa','ID_Ingreso','COD_Etapa','TIP_Consulta'];
    const contextOk=contextKeys.filter(key=>{
      const fields=[...form.querySelectorAll('input')].filter(n=>n.name===key);
      return fields.length===1&&fields[0].type==='hidden'&&fields[0].form===form;
    }).length;
    const controls=[...form.querySelectorAll('input,select,textarea,button')].filter(visible);
    const signature=['select','checkbox','textarea','text','button','button'];
    const signatureOk=controls.length===signature.length&&controls.every((n,i)=>kind(n)===signature[i]&&n.form===form&&!n.matches(':disabled')&&!n.closest(modal)&&!/^HOBS_/.test(n.name));
    const count=items=>Math.min(items.length,255);
    const diagnostic={table_id_matches:count(matches),table_tags:count(tables),table_inside_form:count(inside),
      table_visible:count(tables.filter(visible)),table_inside_visible:count(inside.filter(visible)),
      context_fields_ok:contextOk,visible_controls:count(controls),
      selects:count(controls.filter(n=>kind(n)==='select')),checkboxes:count(controls.filter(n=>kind(n)==='checkbox')),
      editors:count(controls.filter(n=>kind(n)==='textarea')),texts:count(controls.filter(n=>kind(n)==='text')),
      buttons:count(controls.filter(n=>kind(n)==='button')),signature_ok:Number(signatureOk),
      state_options:0,control_checks_ok:0};
    let controlReason='FIRMA_DISTINTA', evidence=null;
    if(signatureOk) {
      const [state,checkbox,editor,,create,other]=controls;
      diagnostic.state_options=count([...state.options]);
      controlReason='CONTROLES_AMBIGUOS';
      if(state.name==='COD_Estado_Detalle_Nuevo'&&!state.multiple&&state.options.length===3&&!editor.readOnly&&!checkbox.indeterminate&&create.name==='irAccion'&&other.name!=='irAccion') {
        const expected='enviar observación al tribunal';
        const labels=[...doc.querySelectorAll('label')].filter(n=>visible(n)&&norm(n.textContent).toLowerCase()===expected);
        const labelOf=n=>norm(n.tagName==='INPUT'?n.value:n.textContent).toLowerCase();
        if(labels.length>1||(labels.length===1&&labels[0].control&&labels[0].control!==checkbox)) controlReason='ETIQUETA_CONTRADICTORIA';
        else if(labelOf(create)==='grabar'||labelOf(other)==='nueva obs') controlReason='ACCION_CONTRADICTORIA';
        else {
          controlReason=null;
          diagnostic.control_checks_ok=1;
          const options=[...state.options].map(o=>({code:/^[0-9]{1,3}$/.test(o.value)?o.value:'OMITIDO',
            label:/^[\p{L}\p{M} ()/.,-]{1,60}$/u.test(norm(o.textContent))?norm(o.textContent):'OMITIDO'}));
          evidence={checkbox:{...attributes(checkbox),label_associated:labels.length===1&&labels[0].control===checkbox},
            editor:{...attributes(editor),maxlength:editor.maxLength>=0?editor.maxLength:null},
            state:{...attributes(state),options},create:{...attributes(create),label_matches:labelOf(create)==='nueva obs'}};
        }
      }
    }
    // Mantener el mismo orden y rigor de bloqueo; recoger evidencia independiente en una sola inspección.
    const reason=!tableOk?'TABLA_NO_UNICA':contextOk!==5?'CONTEXTO_ESTRUCTURAL_INCOMPLETO':controlReason;
    if(reason) return {...blocked(reason),structure:diagnostic,...(evidence?{control_evidence:evidence}:{})};
    return {schema:'FocusedEvidenceV1',status:'CANDIDATE',write_enabled:false,...evidence,context_fields_present:true};
  } catch (_) {return blocked('INSPECCION_NO_CONCLUYENTE');}
}

// Seguir solo una cadena inequívoca de marcos visibles del mismo origen.
// No navega, no abre marcos, no amplía permisos ni lee atributos de sus URLs.
function inspectFocusTree(root, allowedOrigin) {
  'use strict';
  const blocked = reason => ({schema:'FocusedEvidenceV1',status:'BLOCKED',reason,write_enabled:false});
  let doc=root, depth=0;
  const annotate = result => depth===0?result:{...result,frame_depth:depth};
  function frameVisible(frame) {
    if (!frame.isConnected || !frame.getClientRects().length) return false;
    for (let n=frame;n;n=n.parentElement) {
      const css=doc.defaultView.getComputedStyle(n);
      if (n.hidden || n.inert || n.getAttribute('aria-hidden')==='true' || css.display==='none' || css.visibility!=='visible' || css.opacity==='0') return false;
    }
    return true;
  }
  try {
    while (true) {
      const result=inspectFocusDocument(doc);
      const d=result.diagnostic;
      // Solo descender si el documento es un contenedor sin ningún marcador del formulario.
      if (result.reason!=='FORMULARIO_NO_UNICO'||!d||d.forms||d.named||d.by_id||d.tables||d.states||!d.visible_frames) return annotate(result);
      const frames=[...doc.querySelectorAll('frame,iframe')].filter(frameVisible);
      if (frames.length!==1) return annotate(blocked('MARCOS_AMBIGUOS'));
      if (depth>=4) return annotate(blocked('PROFUNDIDAD_MARCOS_EXCEDIDA'));
      let next;
      try {
        next=frames[0].contentDocument;
        if (!next||!next.defaultView||next.defaultView.location.origin!==allowedOrigin) return annotate(blocked('MARCO_NO_ACCESIBLE'));
      } catch (_) {return annotate(blocked('MARCO_NO_ACCESIBLE'));}
      if (next.readyState==='loading') return annotate(blocked('MARCO_CARGANDO'));
      doc=next;depth++;
    }
  } catch (_) {return annotate(blocked('INSPECCION_NO_CONCLUYENTE'));}
}

function inspectFocus() {
  if (location.origin !== 'https://familia.pjud.cl')
    return {schema:'FocusedEvidenceV1',status:'BLOCKED',reason:'ORIGEN_NO_PERMITIDO',write_enabled:false};
  return inspectFocusTree(document,'https://familia.pjud.cl');
}
inspectFocus();
