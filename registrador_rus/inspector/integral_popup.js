/* Valida el esquema cerrado antes de mostrar evidencia. No transmite ni almacena. */
'use strict';
const button=document.getElementById('inspect'), output=document.getElementById('result');
const int=(min,max)=>x=>Number.isInteger(x)&&x>=min&&x<=max;
const num=int(0,65535),idx=int(-1,65535),bool=x=>typeof x==='boolean';
const one=(...values)=>x=>values.includes(x);
const tech=x=>x===null||x==='OMITIDO'||typeof x==='string'&&/^[A-Za-z_][A-Za-z_0-9-]{0,63}$/.test(x)&&!/[0-9]{3,}/.test(x);
const array=(check,max)=>x=>Array.isArray(x)&&x.length<=max&&x.every(check);
const obj=spec=>x=>x!==null&&typeof x==='object'&&!Array.isArray(x)&&Object.keys(x).length===Object.keys(spec).length&&Object.entries(spec).every(([k,v])=>Object.hasOwn(x,k)&&v(x[k]));
const attr={id:tech,name:tech};
const option=obj({code:x=>x===null||x==='OMITIDO'||typeof x==='string'&&/^[0-9]{1,3}$/.test(x),label:x=>typeof x==='string'&&/^[\p{L}\p{M} ()/.,-]{1,60}$/u.test(x),disabled:bool});
const form=obj({index:num,...attr,box_visible:bool,method_post:bool,action_matches:bool,owned_controls:num,descendant_controls:num,onsubmit:bool});
const table=obj({...attr,visible:bool,owner:idx,contains_candidate:bool,rows:num,headers:obj({columns:num,known:int(0,9),exact:bool,row:int(-1,3),order:array(int(-1,8),24)}),hidden_prefix_counts:x=>array(num,5)(x)&&x.length===5,images:num,nested_tables:num});
const context=obj({key:one('COD_Tribunal','CRR_IdCausa','ID_Ingreso','COD_Etapa','TIP_Consulta','tipo_popUp'),total:num,id_matches:num,hidden:num,owners:array(idx,8),inside_candidates:num});
const control=obj({index:num,kind:one('select','checkbox','textarea','text','radio','button'),...attr,visible:bool,owner:idx,inside_candidate:bool,in_history:bool,in_modal:bool,disabled:bool,readonly:bool,required:bool,id_count:num,name_count:num,label_associated:bool,label_nearby:bool,aria_label_matches:bool,multiple:bool,formaction:bool,formmethod:bool,onclick:bool,onchange:bool,oninput:bool,onblur:bool,maxlength_attr:bool,maxlength:x=>x===null||int(0,2147483647)(x),button_label:one(null,'NUEVA_OBS','GRABAR','CERRAR','CANCELAR','ACEPTAR','EDITAR','MODIFICAR','ELIMINAR','SIGUIENTE','ANTERIOR','OTRO'),button_type:one(null,'button','submit','image','reset'),options_count:num,options:array(option,12)});
const tableMarker=obj({tag:one('TABLE','DIV','TBODY','TR','TD','SPAN','FORM','OTHER'),id_match:bool,name_match:bool,visible:bool,owner:idx,descendant_tables:num});
const detail=obj({forms_total:num,tables_total:num,table_id_nodes:num,table_name_nodes:num,table_id_visible:num,visible_frames:num,modal_visible:num,form_candidates:array(form,8),tables:array(table,12),table_markers:array(tableMarker,12),context:array(context,6),controls:array(control,48),visible_controls:num,hidden_controls:num,history_buttons_omitted:num,pagination:obj({next:num,prev:num,buttons:num})});
const documentEntry=obj({path:array(num,4),access:one('OK','INACCESIBLE','CARGANDO','ERROR'),detail:x=>x===null||detail(x)});
const validEvidence=x=>obj({schema:one('IntegralEvidenceV1'),status:one('DIAGNOSTIC'),write_enabled:one(false),truncated:bool,documents:array(documentEntry,12)})(x)&&x.documents.length>0&&x.documents[0].path.length===0&&x.documents.every(d=>(d.access==='OK')===(d.detail!==null));
button.addEventListener('click',async()=>{
  button.disabled=true;output.value='';
  try{
    const tabs=await chrome.tabs.query({active:true,currentWindow:true});
    if(tabs.length!==1||typeof tabs[0].id!=='number'||new URL(tabs[0].url).origin!=='https://familia.pjud.cl'){output.value='ORIGEN_NO_PERMITIDO';return;}
    const r=await chrome.scripting.executeScript({target:{tabId:tabs[0].id},files:['integral.js'],world:'ISOLATED'});
    if(r.length!==1||!validEvidence(r[0].result)){output.value='INSPECCION_NO_CONCLUYENTE';return;}
    output.value=JSON.stringify(r[0].result,null,2);
  }catch(_){output.value='INSPECCION_NO_DISPONIBLE';}finally{button.disabled=false;}
});
