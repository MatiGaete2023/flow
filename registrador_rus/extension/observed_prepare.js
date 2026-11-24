/* Preparación DOM de dos documentos ficticios. No es adaptador operativo.
 * Sin clicks, submit, navegación, red ni comunicación con RUS.
 */
(() => {
  'use strict';
  const fail=code=>{throw new Error(code);};
  const codes=Object.freeze({ESPERA:'4',CUMPLIMIENTO:'6',INFORMES_POR_VENCER:'9',INFORMES_VENCIDOS:'9'});
  const keys=['system','tribunal_id','causa_id','ingreso_id'];
  const numeric=x=>typeof x==='string'&&/^[0-9]{1,20}$/.test(x)&&!/^0+$/.test(x);
  const identity=x=>keys.map(k=>x?.[k]);
  const canonical=x=>JSON.stringify([x?.schema,x?.operation_id,x?.source?.tool,x?.source?.work_id,x?.source?.row_ref,
    ...identity(x?.remote_identity),x?.origin,x?.destination,x?.observation_state,x?.approved_text,x?.approved_text_sha256,
    x?.approved_by,x?.approved_at_utc,x?.approval_version]);
  const sameIdentity=(a,b)=>a?.system==='SITFA'&&b?.system==='SITFA'&&keys.slice(1).every(k=>numeric(a[k])&&a[k]===b[k]);
  function prepareSession(listDoc,popupDoc,op,current){
    if(listDoc===popupDoc||listDoc.location.href!=='about:blank'||listDoc.documentElement.dataset.fixture!=='P3'||
      popupDoc.location.href!=='about:blank'||popupDoc.documentElement.dataset.fixture!=='OBSERVED_PREPARATION')fail('ENTORNO_NO_SINTETICO');
    // Python debe haber validado RegistroFlowV1. Aquí se comprueba la invariancia local.
    const frozen=canonical(op);
    if(frozen!==canonical(current)||op?.schema!=='RegistroFlowV1'||op.source?.tool!=='FLOW'||
      !Object.hasOwn(codes,op.origin)||op.observation_state!==codes[op.origin]||
      !['ADMINISTRATIVA','AL_TRIBUNAL'].includes(op.destination)||
      typeof op.approved_text!=='string'||!op.approved_text.trim()||!Number.isInteger(op.approval_version)||op.approval_version<1||
      !/^[a-f0-9]{64}$/i.test(op.approved_text_sha256))fail('APROBACION_INVALIDA');
    const locator=globalThis.RegistradorPreparation;
    const detector=globalThis.RegistradorObservedForm;
    const listed=locator.locate(listDoc,op);
    const readContext=link=>{
      const links=['onclick','href'].map(k=>locator.parseLinkContext(link.getAttribute(k))).filter(Boolean);
      if(links.length!==1||!sameIdentity(links[0].identity,op.remote_identity)||!numeric(links[0].COD_Etapa))fail('VINCULO_NO_VERIFICADO');
      // La modalidad procede del selector/listado ficticio, no del código de Estado.
      const modality=listDoc.body.dataset.modality;
      if(!numeric(modality))fail('MODALIDAD_NO_VERIFICADA');
      return Object.freeze({COD_Tribunal:links[0].identity.tribunal_id,CRR_IdCausa:links[0].identity.causa_id,
        ID_Ingreso:links[0].identity.ingreso_id,COD_Etapa:links[0].COD_Etapa,TIP_Consulta:modality,tipo_popUp:links[0].tipo_popUp});
    };
    const expected=readContext(listed.link);
    const binding=detector.bindFixture(popupDoc,op.origin);
    const check=()=>{
      if(canonical(op)!==frozen||canonical(current)!==frozen)fail('APROBACION_CAMBIADA');
      const live=locator.locate(listDoc,op);
      if(live.row!==listed.row||live.link!==listed.link||JSON.stringify(readContext(live.link))!==JSON.stringify(expected))fail('LISTADO_CAMBIADO');
      const now=detector.bindFixture(popupDoc,op.origin);
      if(['form','table','state','checkbox','editor','counter','create','back'].some(k=>now[k]!==binding[k]))fail('FORMULARIO_CAMBIADO');
      for(const [key,value] of Object.entries(expected)){
        const matches=[...now.form.elements].filter(n=>n.name===key);
        if(matches.length!==1||matches[0].type!=='hidden'||matches[0].disabled||matches[0].value!==value)fail('CONTEXTO_NO_COINCIDE');
      }
    };
    check();
    const {editor,state,checkbox}=binding;
    const probe=popupDoc.createElement('textarea');probe.value=op.approved_text;
    if(probe.value!==op.approved_text||(editor.maxLength>=0&&op.approved_text.length>editor.maxLength))fail('TEXTO_NO_REPRESENTABLE');
    if(editor.value&&editor.value!==op.approved_text)fail('BORRADOR_EXISTENTE');
    const desired=op.destination==='AL_TRIBUNAL';
    const edits=[];
    const set=(el,key,value)=>{
      check();
      if(el[key]!==value){
        el[key]=value;edits.push([el,key,value]);
        for(const event of ['input','change']){
          el.dispatchEvent(new popupDoc.defaultView.Event(event,{bubbles:true}));check();
          if(edits.some(([n,k,v])=>n[k]!==v))fail('VALORES_NO_COINCIDEN');
        }
      }
    };
    set(editor,'value',op.approved_text);
    set(state,'value',op.observation_state);
    set(checkbox,'checked',desired);
    function verify(latest){
      if(canonical(latest)!==frozen)fail('APROBACION_CAMBIADA');
      check();
      if(editor.value!==op.approved_text||state.value!==op.observation_state||checkbox.checked!==desired||checkbox.indeterminate)fail('VALORES_NO_COINCIDEN');
      return Object.freeze({schema:'ObservedPreparationSimulationV1',operation_id:op.operation_id,origin:op.origin,
        observation_state:op.observation_state,destination:op.destination,checked:desired,cc:Number(desired),saved:false,write_enabled:false});
    }
    const report=verify(current);
    return Object.freeze({report,verify});
  }
  function prepare(listDoc,popupDoc,op,current){return prepareSession(listDoc,popupDoc,op,current).report;}
  globalThis.RegistradorObservedPreparation=Object.freeze({prepare,prepareSession});
})();
