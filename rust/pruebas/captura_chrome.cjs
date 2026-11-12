// Solo páginas y personas ficticias. Intercepta todas las peticiones al dominio de SITFA; no lo contacta.
const {chromium}=require('playwright');
const fs=require('node:fs/promises');
const path=require('node:path');
const os=require('node:os');
const readline=require('node:readline');
const assert=require('node:assert/strict');

let etapa="entrada";
(async()=>{
  const entrada=readline.createInterface({input:process.stdin,crlfDelay:Infinity});
  const lineas=[];const pendientes=[];
  entrada.on('line',s=>{if(pendientes.length)pendientes.shift()(s);else lineas.push(s);});
  const siguiente=()=>lineas.length?Promise.resolve(lineas.shift()):new Promise(r=>pendientes.push(r));
  const {codigo,vacia,lecturas}=JSON.parse(await siguiente());console.error('Ensayo: entrada recibida.');
  const raiz=path.resolve(__dirname,'..');
  const casos=JSON.parse(await fs.readFile(path.join(raiz,'crates/flow-captura/tests/datos/sitfa_ficticio.json'),'utf8'));
  const pestana=lecturas?'Cumplimiento':'Espera';
  const caso=casos.find(c=>c.nombre==='seguimiento/'+pestana+(vacia?'/vacia':''));
  const lector=lecturas?require('./lecturas_chrome.cjs')(caso,JSON.parse(await fs.readFile(path.join(raiz,'crates/flow-captura/tests/datos/lecturas_ficticias.json'),'utf8'))):null;
  const perfil=await fs.mkdtemp(path.join(os.tmpdir(),'flow-chrome-ficticio-'));
  let contexto;
  try {
    etapa="iniciar Chromium";
    contexto=await chromium.launchPersistentContext(perfil,{channel:'chromium',...(process.env.CHROMIUM_PATH?{executablePath:process.env.CHROMIUM_PATH}:{}),headless:true,args:[`--disable-extensions-except=${path.join(raiz,'extension')}`,`--load-extension=${path.join(raiz,'extension')}`,'--no-sandbox','--disable-dev-shm-usage','--enable-unsafe-extension-debugging'],viewport:{width:1280,height:800}});
    console.error('Ensayo: Chromium iniciado.');const pagina=await contexto.newPage();
    let actual=1,consultas=0,descargas=0;
    await contexto.route('https://familia.pjud.cl/**',async route=>{
      const req=route.request();const u=new URL(req.url());
      if(lector&&/\/(Encrypt_AjaxAccion|ConsultaDetalleAtPublicoAccion|IrPopUpInformesAccion)\.do$/.test(u.pathname))return lector.responder(u,route);
      if(u.pathname.startsWith('/sitfa/reportes/')) {
        descargas++;return route.fulfill({status:200,contentType:'application/vnd.ms-excel',body:caso[actual===2?'excel2':'excel']});
      }
      if(req.method()==='POST'){
        consultas++;const pares=new URLSearchParams(req.postData());actual=Number(pares.get('NUM_Pagina'+pestana)||1);
        return route.fulfill({status:200,contentType:'text/html; charset=utf-8',body:caso[actual===2?'html2':'html']});
      }
      // Formulario ficticio con los manejadores que la extensión necesita. No envían formularios.
      let html=caso.html.replace('name="COD_Lengueta" value=""','name="COD_Lengueta" value="td'+pestana+'"');
      html=html.replace('name="TIP_Informe"','name="TIP_Informe" id="TIP_Informe"');
      const extra=`<input type="checkbox" name="CHK_Consulta" value="on"><input name="FLG_Consulta" value="0">
        ${html.includes('name="TIP_Informe"')?'':'<select name="TIP_Informe" id="TIP_Informe"><option value="0">Todas</option></select>'}
        <input type="submit" name="irAccion" value="Buscar medida">
        <button type="button" id="tdEspera">Espera</button><button type="button" id="tdCumplimiento">Cumplimiento</button>
        <button type="button" id="tdInforme">Informes</button><button type="button" id="tdEgreso">Egresados</button>`;
      html=html.replace('</form>',extra+'</form>').replace('</body>',`<script>
        var idLengueta='td${pestana}';
        function Lengueta(id){idLengueta=id;document.querySelector('[name="COD_Lengueta"]').value=id;
          document.getElementById('TIP_Informe').innerHTML=(id==='tdInforme'?[1,2,3]:[0,4,5]).map(v=>'<option value="'+v+'">Filtro '+v+'</option>').join('');}
        function Envio(){return true;}function Seleccion(){}
      </script></body>`);
      return route.fulfill({status:200,contentType:'text/html; charset=utf-8',body:html});
    });
    etapa='abrir SITFA ficticio';
    await pagina.goto('https://familia.pjud.cl/SITFAWEB/InformesViewAccion.do?TipMenuINF=89');
    etapa='cargar extensión';
    const worker=contexto.serviceWorkers()[0]||await contexto.waitForEvent('serviceworker');
    const extensionId=new URL(worker.url()).host;
    const tabs=await worker.evaluate(()=>chrome.tabs.query({}));
    const tab=tabs.find(t=>t.url?.startsWith('https://familia.pjud.cl/'));assert.ok(tab);
    etapa='pulsar acción de la extensión';
    const cdp=await contexto.browser().newBrowserCDPSession();
    const {targetInfos}=await cdp.send('Target.getTargets',{filter:[{type:'tab'}]});
    const targetInfo=targetInfos.find(t=>t.url===pagina.url());assert.ok(targetInfo);
    const [conexion]=await Promise.all([
      contexto.waitForEvent('page'),
      cdp.send('Extensions.triggerAction',{id:extensionId,targetId:targetInfo.targetId})
    ]);
    await conexion.waitForURL(`chrome-extension://${extensionId}/conexion.html?tab=${tab.id}`);
    console.error('Ensayo: extensión cargada.');etapa='conectar puente';
    await conexion.locator('#code').fill(codigo);await conexion.locator('#connect').click();
    await siguiente();
    assert.equal(await pagina.evaluate(()=>document.querySelector('#sitfaDescargaLocal')===null),true);
    assert.equal(await pagina.locator('[name="COD_Lengueta"]').inputValue(),'td'+pestana);
    if(lector)assert.deepEqual(lector.cuentas,{bitacora:6,historia:6,calendario:6});
    assert.equal(descargas,vacia?0:2);assert.ok(consultas>=1);
    await conexion.locator('#disconnect').click({timeout:8000});
    console.log('Extensión MV3: consultas limitadas, descarga verificada y formulario restaurado.');
  } finally {if(contexto)await contexto.close();await fs.rm(perfil,{recursive:true,force:true});entrada.close();}
})().catch(error=>{console.error('Ensayo ficticio, etapa '+etapa+': '+String(error.message).replace(/[0-9a-f]{32}/g,'[oculto]').slice(0,900));process.exitCode=1;});
