// Ventanas ficticias para probar los enlaces del listado mediante la extensión real.
const assert=require('node:assert/strict');
const escapar=s=>s.replaceAll('&','&amp;').replaceAll('"','&quot;').replaceAll('<','&lt;');
module.exports=function preparar(caso,ventanas){
  const personas=new Map();const cuentas={bitacora:0,historia:0,calendario:0};
  for(const campo of ['html','html2','excel','excel2']){
    caso[campo]=caso[campo].replaceAll('<td>Nombre</td>','<td>RUT</td><td>Nombre</td><td>Derivación</td><td>Acciones</td>')
      .replace(/<tr><td>(X-(\d+)-2099)<\/td><td>([^<]+)<\/td><\/tr>/g,(_,rit,n,nombre)=>{
        const causa=String(1000+Number(n)),ingreso=String(2000+Number(n)),programa='PRM Centro Ficticio';
        personas.set(causa,{rit,ingreso,nombre,programa});
        const llamar=(f,args)=>`<a onclick="${escapar(f+'('+args.map(x=>JSON.stringify(x)).join(',')+')')}">${f}</a>`;
        const enlaces=llamar('ShowHistoria',['901',rit,causa])+llamar('ShowObservaciones',[causa,'901',ingreso,'12','3','0'])+
          llamar('ShowInformesProgramados',[causa,'300','901','400',rit,'01/03/2026',programa,nombre,'6 Meses','3 Meses',ingreso,'1']);
        return `<tr><td>${rit}</td><td>11111111-0</td><td>${nombre}</td><td>${programa}</td><td>${enlaces}</td></tr>`;
      });
  }
  const plantilla=t=>ventanas.find(v=>v.tipo===t).html;
  const campo=(html,k,v)=>html.replace(new RegExp('(name="'+k+'" value=")[^"]*"'),'$1'+escapar(v)+'"');
  return {cuentas,async responder(u,route){
    if(u.pathname.endsWith('Encrypt_AjaxAccion.do')){
      const causa=u.searchParams.get('crr_idcausaUrl');assert.ok(personas.has(causa));
      return route.fulfill({json:{crr_causa:'FICTICIOCAUSA'+causa,tip_causa:'FICTICIOTIPO',rol_causa:'FICTICIOROL',era_causa:'FICTICIOERA',cod_tribunal:'FICTICIOTRIBUNAL'}});
    }
    if(u.pathname.endsWith('ConsultaDetalleAtPublicoAccion.do')){
      const persona=personas.get(u.searchParams.get('a1').replace('FICTICIOCAUSA',''));assert.ok(persona);cuentas.historia++;
      const html=plantilla('historia').replaceAll('C-99999-2099',persona.rit).replace('Juzgado Canario de Prueba','901');
      return route.fulfill({contentType:'text/html; charset=utf-8',body:html});
    }
    if(u.pathname.endsWith('IrPopUpInformesAccion.do')){
      const persona=personas.get(u.searchParams.get('CRR_IdCausa'));assert.ok(persona);
      const tipo=u.searchParams.get('tipo_popUp')==='8'?'calendario':'bitacora';cuentas[tipo]++;
      let html=plantilla(tipo);
      for(const [k,v] of Object.entries({COD_Tribunal:'901',CRR_IdCausa:u.searchParams.get('CRR_IdCausa'),ID_Ingreso:persona.ingreso,
        RIT_Causa:persona.rit,GLS_Nombre:persona.nombre,GLS_Centro:persona.programa}))html=campo(html,k,v);
      return route.fulfill({contentType:'text/html; charset=utf-8',body:html});
    }
    return false;
  }};
};
