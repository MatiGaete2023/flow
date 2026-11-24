'use strict';
const button=document.getElementById('inspect');
const output=document.getElementById('result');
// El límite de salida debe coincidir exactamente con el contrato del inspector.
const exact = (value, keys) => value !== null && typeof value === 'object' &&
  !Array.isArray(value) && Object.keys(value).sort().join('|') === keys.split(' ').sort().join('|');
const technical = value => value === null || value === 'OMITIDO' ||
  (typeof value === 'string' && /^[A-Za-z_][A-Za-z_0-9-]{0,63}$/.test(value) && !/[0-9]{3,}/.test(value));
const boolean = value => typeof value === 'boolean';
const attrs = value => technical(value.id) && technical(value.name);
const reasons = new Set(['ORIGEN_NO_PERMITIDO','MODAL_VISIBLE','FORMULARIO_NO_UNICO',
  'ACCION_INESPERADA','TABLA_NO_UNICA','CONTEXTO_ESTRUCTURAL_INCOMPLETO','FIRMA_DISTINTA',
  'CONTROLES_AMBIGUOS','ETIQUETA_CONTRADICTORIA','ACCION_CONTRADICTORIA','INSPECCION_NO_CONCLUYENTE',
  'MARCOS_AMBIGUOS','PROFUNDIDAD_MARCOS_EXCEDIDA','MARCO_NO_ACCESIBLE','MARCO_CARGANDO']);
function validEvidence(value) {
  if (!value || value.schema !== 'FocusedEvidenceV1' || value.write_enabled !== false) return false;
  if (Object.hasOwn(value,'frame_depth')) {
    if (!Number.isInteger(value.frame_depth)||value.frame_depth<1||value.frame_depth>4) return false;
    const {frame_depth,...body}=value;
    return validEvidence(body);
  }
  if (value.status === 'BLOCKED') {
    if (value.reason === 'FORMULARIO_NO_UNICO' && Object.hasOwn(value,'diagnostic')) {
      return exact(value,'schema status reason write_enabled diagnostic') &&
        exact(value.diagnostic,'forms named by_id named_box named_controls action_matches tables states visible_frames') &&
        Object.values(value.diagnostic).every(n=>Number.isInteger(n)&&n>=0&&n<=255);
    }
    return exact(value,'schema status reason write_enabled') && reasons.has(value.reason);
  }
  if (value.status !== 'CANDIDATE' ||
      !exact(value,'schema status write_enabled checkbox editor state create context_fields_present') ||
      value.context_fields_present !== true) return false;
  const {checkbox,editor,state,create} = value;
  return exact(checkbox,'id name label_associated') && attrs(checkbox) && boolean(checkbox.label_associated) &&
    exact(editor,'id name maxlength') && attrs(editor) &&
    (editor.maxlength === null || (Number.isInteger(editor.maxlength) && editor.maxlength >= 0 && editor.maxlength <= 2147483647)) &&
    exact(state,'id name options') && attrs(state) && Array.isArray(state.options) && state.options.length === 3 &&
    state.options.every(o => exact(o,'code label') &&
      typeof o.code === 'string' && (o.code === 'OMITIDO' || /^[0-9]{1,3}$/.test(o.code)) &&
      typeof o.label === 'string' && (o.label === 'OMITIDO' || /^[\p{L}\p{M} ()/.,-]{1,60}$/u.test(o.label))) &&
    exact(create,'id name label_matches') && attrs(create) && boolean(create.label_matches);
}
button.addEventListener('click',async()=>{
  button.disabled=true;
  output.value='';
  try {
    const tabs=await chrome.tabs.query({active:true,currentWindow:true});
    if(tabs.length!==1||typeof tabs[0].id!=='number'||new URL(tabs[0].url).origin!=='https://familia.pjud.cl') {
      output.value='ORIGEN_NO_PERMITIDO';
      return;
    }
    const results=await chrome.scripting.executeScript({target:{tabId:tabs[0].id},files:['inspect.js'],world:'ISOLATED'});
    if(results.length!==1||!validEvidence(results[0].result)) {
      output.value='INSPECCION_NO_CONCLUYENTE';
      return;
    }
    output.value=JSON.stringify(results[0].result,null,2);
  } catch (_) {output.value='INSPECCION_NO_DISPONIBLE';}
  finally {button.disabled=false;}
});
