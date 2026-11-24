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
  console.log('PASS 14 popup contract cases');
})().catch(error=>{console.error(error);process.exitCode=1;});
