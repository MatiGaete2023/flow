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
    const tables=[...doc.querySelectorAll('[id="TablaInforme"]')];
    if(tables.length!==1||tables[0].tagName!=='TABLE'||!form.contains(tables[0])||!visible(tables[0])) return blocked('TABLA_NO_UNICA');
    for(const key of ['COD_Tribunal','CRR_IdCausa','ID_Ingreso','COD_Etapa','TIP_Consulta']) {
      const fields=[...form.querySelectorAll('input')].filter(n=>n.name===key);
      if(fields.length!==1||fields[0].type!=='hidden'||fields[0].form!==form) return blocked('CONTEXTO_ESTRUCTURAL_INCOMPLETO');
    }
    const controls=[...form.querySelectorAll('input,select,textarea,button')].filter(visible);
    const signature=['select','checkbox','textarea','text','button','button'];
    if(controls.length!==signature.length||controls.some((n,i)=>kind(n)!==signature[i]||n.form!==form||n.matches(':disabled')||n.closest(modal)||/^HOBS_/.test(n.name))) return blocked('FIRMA_DISTINTA');
    const [state,checkbox,editor,,create,other]=controls;
    if(state.name!=='COD_Estado_Detalle_Nuevo'||state.multiple||state.options.length!==3||editor.readOnly||checkbox.indeterminate||create.name!=='irAccion'||other.name==='irAccion') return blocked('CONTROLES_AMBIGUOS');
    const expected='enviar observación al tribunal';
    const labels=[...doc.querySelectorAll('label')].filter(n=>visible(n)&&norm(n.textContent).toLowerCase()===expected);
    if(labels.length>1||(labels.length===1&&labels[0].control&&labels[0].control!==checkbox)) return blocked('ETIQUETA_CONTRADICTORIA');
    const labelOf=n=>norm(n.tagName==='INPUT'?n.value:n.textContent).toLowerCase();
    if(labelOf(create)==='grabar'||labelOf(other)==='nueva obs') return blocked('ACCION_CONTRADICTORIA');
    // Solo opciones del selector técnico de Estado, no la opción actualmente seleccionada.
    const options=[...state.options].map(o=>({code:/^[0-9]{1,3}$/.test(o.value)?o.value:'OMITIDO',
      label:/^[\p{L}\p{M} ()/.,-]{1,60}$/u.test(norm(o.textContent))?norm(o.textContent):'OMITIDO'}));
    return {schema:'FocusedEvidenceV1',status:'CANDIDATE',write_enabled:false,
      checkbox:{...attributes(checkbox),label_associated:labels.length===1&&labels[0].control===checkbox},
      editor:{...attributes(editor),maxlength:editor.maxLength>=0?editor.maxLength:null},
      state:{...attributes(state),options},create:{...attributes(create),label_matches:labelOf(create)==='nueva obs'},
      context_fields_present:true};
  } catch (_) {return blocked('INSPECCION_NO_CONCLUYENTE');}
}

function inspectFocus() {
  if (location.origin !== 'https://familia.pjud.cl')
    return {schema:'FocusedEvidenceV1',status:'BLOCKED',reason:'ORIGEN_NO_PERMITIDO',write_enabled:false};
  return inspectFocusDocument(document);
}
inspectFocus();
