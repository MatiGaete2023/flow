/* Diagnóstico estructural integral; solo lectura, sin autorización de operaciones. */
function inspectIntegralTree(root, origin) {
  'use strict';
  const LIMIT={documents:12,depth:4,forms:8,tables:12,controls:48,options:12};
  const report={schema:'IntegralEvidenceV1',status:'DIAGNOSTIC',write_enabled:false,truncated:false,documents:[]};
  const contexts=['COD_Tribunal','CRR_IdCausa','ID_Ingreso','COD_Etapa','TIP_Consulta','tipo_popUp'];
  const prefixes=['HOBS_Centro_','HOBS_Tribunal_','HCOD_Estado_','HFUN_Tribunal_','HFLG_Envio_'];
  const headers=['tipo observacion','etapa','fecha centro','usuario centro','observacion centro','fecha tribunal','usuario tribunal','observacion tribunal','accion'];
  const norm=v=>String(v??'').slice(0,256).normalize('NFKD').replace(/\p{M}/gu,'').replace(/[-().]+/g,' ').replace(/\s+/g,' ').trim().toLowerCase();
  const expected='enviar observacion al tribunal';
  const count=a=>Math.min(a.length,65535);
  const tech=v=>v===null||v===''?null:/^[A-Za-z_][A-Za-z_0-9-]{0,63}$/.test(v)&&!/[0-9]{3,}/.test(v)?v:'OMITIDO';
  const attrs=n=>({id:tech(n.getAttribute('id')),name:tech(n.getAttribute('name'))});
  const visible=n=>{
    if(!n||!n.isConnected||!n.getClientRects().length)return false;
    for(let x=n;x;x=x.parentElement){const c=n.ownerDocument.defaultView.getComputedStyle(x);if(x.hidden||x.inert||x.getAttribute('aria-hidden')==='true'||c.display==='none'||c.visibility!=='visible'||c.opacity==='0')return false;}
    return true;
  };
  const kind=n=>n.tagName==='SELECT'?'select':n.tagName==='TEXTAREA'?'textarea':n.tagName==='BUTTON'?'button':n.tagName==='INPUT'&&['button','submit','image','reset'].includes(n.type)?'button':n.tagName==='INPUT'&&['checkbox','radio','text','hidden'].includes(n.type)?n.type:'other';
  const modal='dialog, [role="dialog"], [aria-modal="true"]';
  const labelToken=n=>{const t=norm(n.tagName==='INPUT'?n.value:n.textContent);return {'nueva obs':'NUEVA_OBS','nueva observacion':'NUEVA_OBS','grabar':'GRABAR','cerrar':'CERRAR','cancelar':'CANCELAR','aceptar':'ACEPTAR','editar':'EDITAR','modificar':'MODIFICAR','eliminar':'ELIMINAR','siguiente':'SIGUIENTE','anterior':'ANTERIOR'}[t]||'OTRO';};
  const actionMatches=(f,doc)=>{try{const u=new URL(f.getAttribute('action')||'',doc.baseURI);return u.origin==='https://familia.pjud.cl'&&u.pathname==='/SITFAWEB/InformesDAction.do';}catch(_){return false;}};
  const headerInfo=t=>{let best={columns:0,known:0,exact:false,row:-1,order:[]};for(let i=0;i<Math.min(t.rows.length,4);i++){const cells=[...t.rows[i].cells],labels=cells.map(c=>norm(c.textContent));const h={columns:count(cells),known:headers.filter(v=>labels.includes(v)).length,exact:labels.length===headers.length&&headers.every((v,j)=>labels[j]===v),row:i,order:limit(labels.map(v=>headers.indexOf(v)),24)};if(best.row===-1||h.exact||h.known>best.known)best=h;if(h.exact)break;}return best;};
  const limit=(a,n)=>{if(a.length>n)report.truncated=true;return a.slice(0,n);};
  const seen=new Set();
  function visit(doc,path){
    if(report.documents.length>=LIMIT.documents){report.truncated=true;return;}
    if(seen.has(doc))return;seen.add(doc);
    const item={path,access:'OK',detail:null};report.documents.push(item);
    try{
      const forms=[...doc.forms], all=[...doc.querySelectorAll('input,select,textarea,button')], tables=[...doc.querySelectorAll('table')];
      const frames=[...doc.querySelectorAll('frame,iframe')].filter(visible);
      const index=f=>f===null?-1:forms.indexOf(f);
      const tableInfo=tables.map(t=>({t,h:headerInfo(t)}));
      const histories=tableInfo.filter(x=>x.t.id==='TablaInforme'||x.t.getAttribute('name')==='TablaInforme'||!!x.t.closest('[id="TablaInforme"],[name="TablaInforme"]')||x.h.known>=3);
      const markers=all.filter(n=>contexts.includes(n.name)||n.name==='COD_Estado_Detalle_Nuevo'||n.id==='COD_Estado_Detalle_Nuevo'||n.name==='irAccion');
      const candidates=forms.filter(f=>f.getAttribute('name')==='InformesPpalForm'||f.id==='InformesPpalForm'||actionMatches(f,doc)||markers.some(n=>n.form===f||f.contains(n))||histories.some(x=>f.contains(x.t)));
      const relevant=candidates.length>0||markers.length>0||histories.length>0;
      const d={forms_total:count(forms),tables_total:count(tables),table_id_nodes:count([...doc.querySelectorAll('[id="TablaInforme"]')]),table_name_nodes:count([...doc.querySelectorAll('[name="TablaInforme"]')]),table_id_visible:count([...doc.querySelectorAll('[id="TablaInforme"]')].filter(visible)),visible_frames:count(frames),modal_visible:count([...doc.querySelectorAll(modal)].filter(visible)),
        form_candidates:[],tables:[],table_markers:[],context:[],controls:[],visible_controls:0,hidden_controls:0,history_buttons_omitted:0,pagination:{next:0,prev:0,buttons:0}};
      item.detail=d;
      for(const n of limit([...doc.querySelectorAll('[id="TablaInforme"],[name="TablaInforme"]')],LIMIT.tables))d.table_markers.push({tag:['TABLE','DIV','TBODY','TR','TD','SPAN','FORM'].includes(n.tagName)?n.tagName:'OTHER',id_match:n.id==='TablaInforme',name_match:n.getAttribute('name')==='TablaInforme',visible:visible(n),owner:index(n.closest('form')),descendant_tables:count([...n.querySelectorAll('table')])});
      for(const f of limit(candidates,LIMIT.forms))d.form_candidates.push({index:index(f),...attrs(f),box_visible:visible(f),method_post:f.method.toLowerCase()==='post',action_matches:actionMatches(f,doc),owned_controls:count(all.filter(n=>n.form===f)),descendant_controls:count(all.filter(n=>f.contains(n))),onsubmit:f.hasAttribute('onsubmit')});
      for(const {t,h} of limit(histories,LIMIT.tables)){
        const fields=[...t.querySelectorAll('input')];
        d.tables.push({...attrs(t),visible:visible(t),owner:index(t.closest('form')),contains_candidate:candidates.some(f=>t.contains(f)),rows:Math.min(t.rows.length,65535),headers:h,hidden_prefix_counts:prefixes.map(p=>count(fields.filter(n=>n.name.startsWith(p)&&n.type==='hidden'))),images:count([...t.querySelectorAll('img')]),nested_tables:count([...t.querySelectorAll('table')])});
      }
      if(relevant){
        for(const key of contexts){const hits=all.filter(n=>n.name===key);d.context.push({key,total:count(hits),id_matches:count(all.filter(n=>n.id===key)),hidden:count(hits.filter(n=>kind(n)==='hidden')),owners:limit(hits.map(n=>index(n.form)),LIMIT.forms),inside_candidates:count(hits.filter(n=>candidates.some(f=>f.contains(n))))});}
        const scope=all.filter(n=>kind(n)!=='hidden'&&!prefixes.some(p=>n.name.startsWith(p))&&['select','checkbox','textarea','text','radio','button'].includes(kind(n))&&
          (candidates.includes(n.form)||candidates.some(f=>f.contains(n))||markers.includes(n)||!n.form));
        const historyButton=n=>kind(n)==='button'&&n.name!=='irAccion'&&histories.some(x=>x.h.known>=3&&x.t.contains(n))&&!['NUEVA_OBS','GRABAR','CERRAR'].includes(labelToken(n));
        d.history_buttons_omitted=count(scope.filter(historyButton));
        const entry=scope.filter(n=>!historyButton(n));
        const vis=entry.filter(visible),hid=entry.filter(n=>!visible(n)&&['select','checkbox','textarea','button'].includes(kind(n)));
        d.visible_controls=count(vis);d.hidden_controls=count(hid);
        const labels=[...doc.querySelectorAll('label')].filter(n=>norm(n.textContent)===expected);
        for(const n of limit([...vis,...hid],LIMIT.controls)){
          const k=kind(n),id=n.getAttribute('id'),name=n.getAttribute('name');
          const exactLabel=labels.some(l=>l.control===n);
          const cell=n.closest('td,th'),cellLabel=cell&&[cell.previousElementSibling,cell.nextElementSibling].some(x=>x&&norm(x.textContent)===expected);
          const sibling=[n.previousElementSibling,n.nextElementSibling].some(x=>x&&['LABEL','SPAN','TD','B','FONT'].includes(x.tagName)&&norm(x.textContent)===expected);
          const nearby=[...(n.parentElement?.childNodes||[])].some(x=>x.nodeType===3&&norm(x.nodeValue)===expected);
          const c={index:all.indexOf(n),kind:k,...attrs(n),visible:visible(n),owner:index(n.form),inside_candidate:candidates.some(f=>f.contains(n)),in_history:histories.some(x=>x.t.contains(n)),in_modal:!!n.closest(modal),disabled:n.matches(':disabled'),readonly:!!n.readOnly,required:!!n.required,id_count:id?count([...doc.querySelectorAll('[id]')].filter(x=>x.getAttribute('id')===id)):0,name_count:name?count(all.filter(x=>x.getAttribute('name')===name)):0,label_associated:exactLabel,label_nearby:sibling||nearby||!!cellLabel,aria_label_matches:norm(n.getAttribute('aria-label'))===expected,multiple:!!n.multiple,formaction:n.hasAttribute('formaction'),formmethod:n.hasAttribute('formmethod'),onclick:n.hasAttribute('onclick'),onchange:n.hasAttribute('onchange'),oninput:n.hasAttribute('oninput'),onblur:n.hasAttribute('onblur'),maxlength_attr:n.hasAttribute('maxlength'),maxlength:(k==='textarea'||k==='text')&&n.maxLength>=0?n.maxLength:null,button_label:k==='button'?labelToken(n):null,button_type:k==='button'?n.type:null,options_count:k==='select'?count([...n.options]):0,options:[]};
          if(k==='select'&&(n.name==='COD_Estado_Detalle_Nuevo'||n.id==='COD_Estado_Detalle_Nuevo'))c.options=limit([...n.options],LIMIT.options).map(o=>({code:/^[0-9]{1,3}$/.test(o.value)?o.value:o.value===''?null:'OMITIDO',label:/^[\p{L}\p{M} ()/.,-]{1,60}$/u.test(o.textContent.trim())?o.textContent.trim():'OMITIDO',disabled:o.disabled}));
          d.controls.push(c);
        }
        d.pagination={next:count([...doc.querySelectorAll('[rel="next"]')]),prev:count([...doc.querySelectorAll('[rel="prev"]')]),buttons:count(vis.filter(n=>kind(n)==='button'&&['SIGUIENTE','ANTERIOR'].includes(labelToken(n))))};
      }
      for(let i=0;i<frames.length;i++){
        if(path.length>=LIMIT.depth||report.documents.length>=LIMIT.documents){report.truncated=true;break;}
        const childPath=[...path,i];let child=null;
        try{child=frames[i].contentDocument;if(!child||child.defaultView.location.origin!==origin)child=null;}catch(_){child=null;}
        if(!child)report.documents.push({path:childPath,access:'INACCESIBLE',detail:null});
        else if(child.readyState==='loading')report.documents.push({path:childPath,access:'CARGANDO',detail:null});
        else visit(child,childPath);
      }
    }catch(_){item.access='ERROR';item.detail=null;}
  }
  visit(root,[]);return report;
}
function inspectIntegral(){
  if(location.origin!=='https://familia.pjud.cl')return {schema:'IntegralEvidenceV1',status:'BLOCKED',reason:'ORIGEN_NO_PERMITIDO',write_enabled:false};
  return inspectIntegralTree(document,'https://familia.pjud.cl');
}
inspectIntegral();
