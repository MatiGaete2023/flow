// Prueba del contrato de salida y del flujo popup. API Chrome simulada, no instalación MV3.
'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const source = fs.readFileSync(path.join(__dirname,'../inspector/popup.js'),'utf8');
const valid = {
  schema:'FocusedEvidenceV1',status:'CANDIDATE',write_enabled:false,
  checkbox:{id:'envio',name:'envio',label_associated:true},
  editor:{id:null,name:'obs',maxlength:2000},
  state:{id:'estado',name:'COD_Estado_Detalle_Nuevo',options:[
    {code:'1',label:'Estado ficticio'},{code:'2',label:'Otro estado'},{code:'3',label:'Pendiente'}]},
  create:{id:null,name:'irAccion',label_matches:true},context_fields_present:true
};
async function exercise({tabs=[{id:7,url:'https://familia.pjud.cl/'}],results=[{result:valid}],throws=false}={}) {
  let handler, calls=0;
  const button={disabled:false,addEventListener:(event,fn)=>{assert.equal(event,'click');handler=fn;}};
  const output={value:'inicial'};
  const chrome={
    tabs:{query:async()=>tabs},
    scripting:{executeScript:async args=>{
      calls++;
      assert.equal(args.target.tabId,7);
      assert.equal(args.world,'ISOLATED');
      assert.equal(args.files[0],'inspect.js');
      if(throws) throw Error('NO_EXPORTAR_ERROR');
      return results;
    }}
  };
  vm.runInNewContext(source,{document:{getElementById:id=>id==='inspect'?button:output},chrome,URL});
  await handler();
  assert.equal(button.disabled,false);
  assert(!output.value.includes('NO_EXPORTAR'));
  return {output:output.value,calls};
}
(async()=>{
  let r=await exercise();assert.deepEqual(JSON.parse(r.output),valid);assert.equal(r.calls,1);
  r=await exercise({tabs:[{id:7,url:'https://example.test/'}]});assert.equal(r.calls,0);assert.equal(r.output,'ORIGEN_NO_PERMITIDO');
  const blocked={schema:'FocusedEvidenceV1',status:'BLOCKED',reason:'FIRMA_DISTINTA',write_enabled:false};
  r=await exercise({results:[{result:blocked}]});assert.deepEqual(JSON.parse(r.output),blocked);
  for(const result of [
    {...valid,html:'NO_EXPORTAR'},
    {...valid,editor:{...valid.editor,text:'NO_EXPORTAR'}},
    {...valid,state:{...valid.state,selected:'2'}},
    {...valid,checkbox:{...valid.checkbox,checked:true}},
    {...valid,write_enabled:true},
    {...valid,editor:{...valid.editor,id:'ingreso123456'}},
    {...blocked,reason:'NO_EXPORTAR'},
    {...valid,state:{...valid.state,options:[{code:'1',label:'NO_EXPORTAR_123'},...valid.state.options.slice(1)]}},
    null
  ]){
    r=await exercise({results:[{result}]});assert.equal(r.output,'INSPECCION_NO_CONCLUYENTE');
  }
  r=await exercise({results:[]});assert.equal(r.output,'INSPECCION_NO_CONCLUYENTE');
  r=await exercise({throws:true});assert.equal(r.output,'INSPECCION_NO_DISPONIBLE');

  const diagnostic={...blocked,reason:'FORMULARIO_NO_UNICO',diagnostic:{
    forms:1,named:0,by_id:1,named_box:0,named_controls:0,action_matches:1,tables:1,states:1,visible_frames:0}};
  r=await exercise({results:[{result:diagnostic}]});assert.deepEqual(JSON.parse(r.output),diagnostic);
  for(const d of [{...diagnostic.diagnostic,text:'NO_EXPORTAR'}, {...diagnostic.diagnostic,named:-1}, {...diagnostic.diagnostic,forms:256}, {...diagnostic.diagnostic,states:'1'}]){
    r=await exercise({results:[{result:{...diagnostic,diagnostic:d}}]});assert.equal(r.output,'INSPECCION_NO_CONCLUYENTE');
  }
  r=await exercise({results:[{result:{...valid,frame_depth:1}}]});assert.deepEqual(JSON.parse(r.output),{...valid,frame_depth:1});
  r=await exercise({results:[{result:{...diagnostic,frame_depth:2}}]});assert.deepEqual(JSON.parse(r.output),{...diagnostic,frame_depth:2});
  for(const reason of ['MARCOS_AMBIGUOS','PROFUNDIDAD_MARCOS_EXCEDIDA','MARCO_NO_ACCESIBLE','MARCO_CARGANDO']){
    const result={...blocked,reason,frame_depth:1};
    r=await exercise({results:[{result}]});assert.deepEqual(JSON.parse(r.output),result);
  }
  for(const frame_depth of [0,5,-1,'1',1.5]){
    r=await exercise({results:[{result:{...valid,frame_depth}}]});assert.equal(r.output,'INSPECCION_NO_CONCLUYENTE');
  }
  const structure={table_id_matches:0,table_tags:0,table_inside_form:0,table_visible:0,table_inside_visible:0,context_fields_ok:5,visible_controls:6,selects:1,checkboxes:1,editors:1,texts:1,buttons:2,signature_ok:1,state_options:3,control_checks_ok:1};
  const control_evidence={checkbox:valid.checkbox,editor:valid.editor,state:valid.state,create:valid.create};
  const partial={...blocked,reason:'TABLA_NO_UNICA',frame_depth:1,structure,control_evidence};
  r=await exercise({results:[{result:partial}]});assert.deepEqual(JSON.parse(r.output),partial);
  for(const wrong of [
    {...partial,structure:{...structure,text:'NO_EXPORTAR'}},
    {...partial,structure:{...structure,context_fields_ok:6}},
    {...partial,structure:{...structure,signature_ok:2}},
    {...partial,structure:{...structure,table_tags:-1}},
    {...partial,control_evidence:{...control_evidence,editor:{...valid.editor,text:'NO_EXPORTAR'}}},
    {...partial,reason:'FORMULARIO_NO_UNICO'},
    {...partial,structure:{...structure,control_checks_ok:0}}
  ]){
    r=await exercise({results:[{result:wrong}]});assert.equal(r.output,'INSPECCION_NO_CONCLUYENTE');
  }
  console.log('PASS 38 popup contract cases');
})().catch(error=>{console.error(error);process.exitCode=1;});
