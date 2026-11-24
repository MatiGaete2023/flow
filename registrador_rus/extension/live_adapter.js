/* Adaptador DOM RUS real, candidato P8, sin carga automática en extensiones.
 * API de listado, lectura, preparación y creación única. El controlador local
 * DEBE acreditar autorización, vigencia, custodia, baseline y reserva durable.
 * La creación permanece bloqueada por constante de compilación hasta aceptación.
 * No POST directo, eval, credenciales, edición ni borrado.
 */
(() => {
  'use strict';
  const SITE='https://familia.pjud.cl';
  const POST='/SITFAWEB/InformesDAction.do', OPEN='/SITFAWEB/IrPopUpInformesAccion.do';
  const WRITE_ACCEPTED=false;
  const VIEWS=Object.freeze({
    ESPERA:Object.freeze(['tablaEspera','tdEspera','4',null,null]),
    CUMPLIMIENTO:Object.freeze(['tablaCumplimiento','tdCumplimiento','6',null,null]),
    INFORMES_POR_VENCER:Object.freeze(['tablaInforme','tdInforme','9','1','Por vencer (a 25 días)']),
    INFORMES_VENCIDOS:Object.freeze(['tablaInforme','tdInforme','9','2','Vencidos'])
  });
  const TABLES=['tablaEspera','tablaCumplimiento','tablaInforme','tablaEgresados'];
  const keys=['COD_Tribunal','CRR_IdCausa','ID_Ingreso','COD_Etapa','TIP_Consulta','tipo_popUp'];
  const fail=code=>{throw new Error(code);};
  const one=(items,code)=>{if(items.length!==1)fail(code);return items[0];};
  const digit=s=>typeof s==='string'&&/^[0-9]{1,20}$/.test(s)&&!/^0+$/.test(s);
  const decimal=s=>typeof s==='string'&&/^[0-9]{1,20}$/.test(s);
  const hash=s=>typeof s==='string'&&/^[0-9a-f]{64}$/.test(s);
  const norm=s=>String(s??'').normalize('NFC').replace(/\s+/g,' ').trim();
  const visible=n=>{
    if(!n?.isConnected||!n.getClientRects().length)return false;
    for(let p=n;p;p=p.parentElement){
      const s=p.ownerDocument.defaultView.getComputedStyle(p);
      if(p.hidden||p.inert||p.getAttribute('aria-hidden')==='true'||s.display==='none'||s.visibility!=='visible'||s.opacity==='0')return false;
    }
    return true;
  };
  const same=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
  function approval(op){
    const id=op?.remote_identity, src=op?.source;
    if(op?.schema!=='RegistroFlowV1'||src?.tool!=='FLOW'||!hash(op.operation_id)||
      id?.system!=='SITFA'||![id.tribunal_id,id.causa_id,id.ingreso_id].every(digit)||
      !Object.hasOwn(VIEWS,op.origin)||op.observation_state!==VIEWS[op.origin][2]||
      !['ADMINISTRATIVA','AL_TRIBUNAL'].includes(op.destination)||
      typeof op.approved_text!=='string'||!op.approved_text.trim()||!hash(op.approved_text_sha256)||
      !Number.isSafeInteger(op.approval_version)||op.approval_version<1||
      ![src.work_id,src.row_ref,op.approved_by,op.approved_at_utc].every(s=>typeof s==='string'&&s.length))fail('APROBACION_INVALIDA');
    return JSON.stringify([op.schema,op.operation_id,src.tool,src.work_id,src.row_ref,id.system,id.tribunal_id,id.causa_id,id.ingreso_id,
      op.origin,op.destination,op.observation_state,op.approved_text,op.approved_text_sha256,op.approved_by,op.approved_at_utc,op.approval_version]);
  }
  async function verifyText(op){
    if(!globalThis.crypto?.subtle)fail('WEB_CRYPTO_NO_DISPONIBLE');
    const bytes=new TextEncoder().encode(op.approved_text);
    const digest=[...new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))].map(b=>b.toString(16).padStart(2,'0')).join('');
    if(digest!==op.approved_text_sha256)fail('HUELLA_TEXTO_DISTINTA');
  }
  function exactField(form,name,tag){
    const n=one([...form.elements].filter(n=>n.name===name),'CAMPO_LISTADO_NO_UNICO');
    if(n.form!==form||!form.contains(n)||n.tagName!==tag||n.disabled)fail('CAMPO_LISTADO_DISTINTO');
    return n;
  }
  function locateCore(doc,op){
    approval(op);
    const [tableId,tab,,report,label]=VIEWS[op.origin];
    const form=one([...doc.forms].filter(f=>f.name==='InformesPpalForm'),'LISTADO_FORMULARIO_NO_UNICO');
    const action=new URL(form.action);
    if(form.method.toLowerCase()!=='post'||action.origin!==SITE||action.pathname!==POST)fail('LISTADO_ACCION_DISTINTA');
    if(exactField(form,'COD_Tribunal_sel','SELECT').value!==op.remote_identity.tribunal_id)fail('TRIBUNAL_DISTINTO');
    const modality=exactField(form,'TIP_Consulta','SELECT').value;
    if(!digit(modality))fail('MODALIDAD_NO_VERIFICADA');
    // COD_Lengueta puede estar vacío en la respuesta; la tabla activa visible debe ser única.
    const tabField=exactField(form,'COD_Lengueta','INPUT');
    if(tabField.type!=='hidden'||!['',tab].includes(tabField.value))fail('PESTANA_DISTINTA');
    const tables=[...form.querySelectorAll('table[id]')].filter(t=>TABLES.includes(t.id)&&visible(t));
    const table=one(tables,'LISTADO_NO_UNICO');
    if(table.id!==tableId||doc.querySelectorAll('[id="'+tableId+'"]').length!==1)fail('PESTANA_DISTINTA');
    if(report!==null){
      const s=exactField(form,'TIP_Informe','SELECT');
      const o=one([...s.options].filter(o=>o.value===report),'FILTRO_INFORMES_NO_ACREDITADO');
      if(s.value!==report||!o.selected||o.disabled||norm(o.textContent)!==label)fail('FILTRO_INFORMES_DISTINTO');
    }
    const hits=[];
    for(const row of [...table.rows].filter(visible)){
      const elements=[...row.querySelectorAll('[onclick],[href]')].filter(visible);
      const calls=[];
      let malformed=false;
      for(const el of elements)for(const attr of ['onclick','href']){
        const raw=el.getAttribute(attr)||'';
        if(!/\bShowObservaciones\s*\(/.test(raw))continue;
        const parsed=RegistradorPreparation.parseLinkContext(raw);
        if(!parsed){malformed=true;continue;}
        calls.push({el,parsed});
      }
      const matching=calls.filter(c=>{
        const i=c.parsed.identity, e=op.remote_identity;
        return i.tribunal_id===e.tribunal_id&&i.causa_id===e.causa_id&&i.ingreso_id===e.ingreso_id;
      });
      if(matching.length&&(malformed||calls.length!==1))fail('FILA_AMBIGUA');
      if(matching.length)hits.push({row,link:matching[0].el,parsed:matching[0].parsed});
    }
    const hit=one(hits,'INGRESO_NO_UNICO');
    const p=hit.parsed;
    if(!decimal(p.COD_Etapa)||!['11','12','13'].includes(p.tipo_popUp))fail('VINCULO_NO_VERIFICADO');
    return Object.freeze({...hit,form,table,context:Object.freeze({
      COD_Tribunal:p.identity.tribunal_id,CRR_IdCausa:p.identity.causa_id,ID_Ingreso:p.identity.ingreso_id,
      COD_Etapa:p.COD_Etapa,TIP_Consulta:modality,tipo_popUp:p.tipo_popUp
    })});
  }
  function liveList(doc,op){
    const u=new URL(doc.location.href);
    if(u.origin!==SITE||u.pathname!==POST)fail('RUTA_NO_LISTADO');
    return locateCore(doc,op);
  }
  function verifyContext(binding,context,op){
    if(!context||Object.keys(context).length!==keys.length||!keys.every(k=>Object.hasOwn(context,k)&&decimal(context[k])))fail('CONTEXTO_INCOMPLETO');
    const id=op.remote_identity;
    if(context.COD_Tribunal!==id.tribunal_id||context.CRR_IdCausa!==id.causa_id||context.ID_Ingreso!==id.ingreso_id||
      !digit(context.TIP_Consulta)||!['11','12','13'].includes(context.tipo_popUp))fail('CONTEXTO_NO_COINCIDE');
    for(const k of keys){
      const field=one([...binding.form.elements].filter(n=>n.name===k),'CONTEXTO_NO_UNICO');
      if(field.type!=='hidden'||field.disabled||field.value!==context[k])fail('CONTEXTO_NO_COINCIDE');
    }
  }
  function captureCore(doc,op,context,binder){
    approval(op);
    const b=binder(doc,op.origin);
    verifyContext(b,context,op);
    // HTML solo para lector LOCAL, nunca para diagnóstico exportable.
    // No ejecutar scripts ni cargar documentos. La cobertura la decide read_snapshot.
    const html='<!DOCTYPE html>\n'+doc.documentElement.outerHTML;
    if(new TextEncoder().encode(html).length>8*1024*1024)fail('LECTURA_EXCEDE_LIMITE');
    return html;
  }
  async function sessionCore(doc,op,context,binder){
    const frozen=approval(op);
    const ctx=Object.freeze({...context});
    await verifyText(op);
    if(approval(op)!==frozen)fail('APROBACION_CAMBIADA');
    const b=binder(doc,op.origin);
    const check=latest=>{
      if(approval(op)!==frozen||approval(latest)!==frozen)fail('APROBACION_CAMBIADA');
      const now=binder(doc,op.origin);
      if(['form','table','state','checkbox','editor','counter','create','back'].some(k=>now[k]!==b[k]))fail('FORMULARIO_CAMBIADO');
      verifyContext(now,ctx,op);
    };
    check(op);
    const {editor,state,checkbox}=b;
    const desired=op.destination==='AL_TRIBUNAL';
    const probe=doc.createElement('textarea');probe.value=op.approved_text;
    if(probe.value!==op.approved_text||(editor.maxLength>=0&&op.approved_text.length>editor.maxLength))fail('TEXTO_NO_REPRESENTABLE');
    if(editor.value&&editor.value!==op.approved_text)fail('BORRADOR_EXISTENTE');
    let prepared=false,used=false;
    function verify(latest){
      check(latest);
      if(!prepared||editor.value!==op.approved_text||state.value!==op.observation_state||checkbox.checked!==desired||checkbox.indeterminate)
        fail('VALORES_NO_COINCIDEN');
      return Object.freeze({operation_id:op.operation_id,origin:op.origin,state:op.observation_state,cc:Number(desired),saved:false});
    }
    function prepare(latest){
      if(prepared||used)fail('SESION_YA_PREPARADA');
      check(latest);
      if(editor.value&&editor.value!==op.approved_text)fail('BORRADOR_EXISTENTE');
      const changes=[];
      for(const [el,key,value] of [[editor,'value',op.approved_text],[state,'value',op.observation_state],[checkbox,'checked',desired]]){
        check(latest);
        if(el[key]===value)continue;
        el[key]=value;
        changes.push([el,key,value]);
        for(const event of ['input','change']){
          el.dispatchEvent(new doc.defaultView.Event(event,{bubbles:true}));
          check(latest);
          if(changes.some(([n,k,v])=>n[k]!==v))fail('VALORES_NO_COINCIDEN');
        }
      }
      prepared=true;
      return verify(latest);
    }
    function createOnce(latest,reservation){
      // Nunca habilitar por config, mensaje, booleano de usuario o datos del DOM.
      if(!WRITE_ACCEPTED)fail('ESCRITURA_REAL_NO_HABILITADA');
      if(used||!prepared||!reservation||reservation.operation_id!==op.operation_id||
        reservation.approved_text_sha256!==op.approved_text_sha256||reservation.approval_version!==op.approval_version||
        reservation.state!=='ENVIANDO'||!hash(reservation.reservation_id))fail('RESERVA_NO_ACREDITADA');
      verify(latest);
      used=true;
      // Única acción de creación normal; resultado permanece incierto hasta relectura.
      b.create.click();
      return Object.freeze({operation_id:op.operation_id,state:'INCIERTA'});
    }
    return Object.freeze({prepare,verify,createOnce});
  }
  function assertFixture(doc){
    if(doc.location.href!=='about:blank'||doc.documentElement.dataset.fixture!=='LIVE_ADAPTER_TEST')fail('ENTORNO_NO_SINTETICO');
  }
  function bindFixture(doc,origin){
    assertFixture(doc);
    // El detector de formulario usa su propio marcador; los tests lo configuran explícitamente.
    return RegistradorObservedForm.bindFixture(doc,origin);
  }
  // Los tests invocan las mismas funciones de núcleo con guardas ficticias;
  // no se sustituyen los selectores, ni se navega al dominio institucional.
  const fixture=Object.freeze({
    locate(doc,op){assertFixture(doc);return locateCore(doc,op);},
    session(doc,op,ctx){if(doc.location.href!=='about:blank'||doc.documentElement.dataset.fixture!=='OBSERVED_PREPARATION')fail('ENTORNO_NO_SINTETICO');
      return sessionCore(doc,op,ctx,RegistradorObservedForm.bindFixture);},
    capture(doc,op,ctx){if(doc.location.href!=='about:blank'||doc.documentElement.dataset.fixture!=='OBSERVED_PREPARATION')fail('ENTORNO_NO_SINTETICO');
      return captureCore(doc,op,ctx,RegistradorObservedForm.bindFixture);}
  });
  globalThis.RegistradorLiveAdapter=Object.freeze({
    locate:liveList,
    capture:(doc,op,ctx)=>captureCore(doc,op,ctx,RegistradorObservedForm.bindReal),
    session:(doc,op,ctx)=>sessionCore(doc,op,ctx,RegistradorObservedForm.bindReal),
    fixture
  });
})();
