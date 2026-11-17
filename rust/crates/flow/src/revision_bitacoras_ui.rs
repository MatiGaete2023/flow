//! Revisión de bitácoras: operaciones locales en segundo plano, evidencia separada de propuestas.
use eframe::egui::{self};
use egui_extras::{Column, TableBuilder};
use flow_dominio::{celda::fecha_de_texto, Fecha};
use flow_motor::Config;
use flow_productos::revision::{self, Producto};
use flow_trabajo::{paquete, revision_bitacoras::Clasificacion as C, revision_informe::{self, Informe, FilaRevision}};
use std::{collections::BTreeSet, io::Read, path::PathBuf, sync::{Arc, atomic::{AtomicBool,Ordering}, mpsc::{channel,Receiver}}};

enum Evento {
    Abierto(Result<(PathBuf,Arc<Vec<u8>>,Option<(Fecha,Fecha)>),String>),
    Informe(Result<Informe,String>,bool),
    Exportado(Result<Producto,String>),
}
#[derive(Clone)]
enum Accion { Abrir(PathBuf), Analizar }

pub struct RevisionUi {
    archivo: Option<PathBuf>, datos: Option<Arc<Vec<u8>>>, informe: Option<Informe>,
    desde: String, hasta: String, corte: String, clase: C, buscar: String,
    tribunal: String, programa: String, etapa: String, estado_revision: u8,
    tribunales: Vec<String>, programas: Vec<String>, busquedas: Vec<String>,
    seleccion: BTreeSet<usize>, activa: Option<usize>, exportar_todos: bool,
    receptor: Option<Receiver<Evento>>, cancelar: Arc<AtomicBool>, mensaje: String,
    resultado: Option<PathBuf>, confirmar: Option<Accion>,
    panel:u8, distribucion:u64,
}
impl Default for RevisionUi {
    fn default() -> Self {
        Self { archivo:None,datos:None,informe:None,desde:String::new(),hasta:String::new(),
            corte:flow_tabular::libro::hoy().to_string(),clase:C::SinObservacion,buscar:String::new(),
            tribunal:String::new(),programa:String::new(),etapa:String::new(),estado_revision:0,
            tribunales:vec![],programas:vec![],busquedas:vec![],seleccion:BTreeSet::new(),activa:None,exportar_todos:false,
            receptor:None,cancelar:Arc::new(AtomicBool::new(false)),mensaje:String::new(),resultado:None,confirmar:None,panel:0,distribucion:0 }
    }
}
impl Drop for RevisionUi { fn drop(&mut self) {self.cancelar.store(true,Ordering::Relaxed);} }

fn valor(f: &FilaRevision, nombres: &[&str]) -> String {
    f.valores.as_object().into_iter().flat_map(|m|m.iter()).find_map(|(k,v)| {
        nombres.iter().any(|n|flow_dominio::texto::normalizar_columna(n)==flow_dominio::texto::normalizar_columna(k))
            .then(||v.as_str().map(str::to_owned).unwrap_or_else(||v.to_string()))
    }).unwrap_or_default()
}
fn combo(ui: &mut egui::Ui, id: &str, valor: &mut String, opciones: &[String]) {
    egui::ComboBox::from_id_salt(id).selected_text(if valor.is_empty() {"Todos"}else{valor.as_str()}).show_ui(ui,|ui| {
        ui.selectable_value(valor,String::new(),"Todos");
        for opcion in opciones {ui.selectable_value(valor,opcion.clone(),opcion);}
    });
}
impl RevisionUi {
    pub fn demostracion(&mut self,datos:Vec<u8>,config:&Config,panel:u8)->Result<(),String>{
        let desde="2026-11-01".parse().map_err(|_|"Fecha ficticia inválida.")?;
        let hasta="2026-11-17".parse().map_err(|_|"Fecha ficticia inválida.")?;
        let mut informe=paquete::revisar_bitacoras(&datos,desde,hasta,hasta)?;
        revision_informe::aplicar_motor(&mut informe,"Paquete_FICTICIO.zip",&datos,config)?;
        self.desde=desde.to_string();self.hasta=hasta.to_string();self.corte=hasta.to_string();
        self.archivo=Some(PathBuf::from("Paquete_FICTICIO.zip"));self.datos=Some(Arc::new(datos));
        self.instalar(informe,true);self.seleccion.insert(0);self.activa=Some(0);self.panel=panel;
        self.mensaje="Paquete ficticio: cuatro estados y una propuesta obtenida del motor.".into();Ok(())
    }
    fn criterio(&self) -> Result<(Fecha,Fecha,Fecha),String> {
        let fecha = |s:&str| fecha_de_texto(s).ok_or("Usa fechas dd/mm/aaaa o aaaa-mm-dd.".to_string());
        let (a,b,c)=(fecha(&self.desde)?,fecha(&self.hasta)?,fecha(&self.corte)?);
        if a>b || b>c {return Err("El período debe terminar como máximo en el corte.".into());}
        Ok((a,b,c))
    }
    fn obsoleto(&self) -> bool {
        self.informe.as_ref().is_some_and(|i|self.criterio().ok()!=Some((i.desde,i.hasta,i.corte)))
    }
    fn tiene_ediciones(&self) -> bool {
        self.informe.as_ref().is_some_and(|i|i.motor_aplicado || i.filas.iter().any(|f|f.revisada || f.edicion.is_some()))
    }
    fn marcar_revisada(&mut self, indice: usize, revisada: bool) -> bool {
        if self.obsoleto() || self.receptor.is_some() {return false}
        let Some(fila)=self.informe.as_mut().and_then(|i|i.filas.get_mut(indice))else{return false};
        fila.revisada=revisada;true
    }
    fn solicitar(&mut self, accion: Accion, ctx: &egui::Context) {
        let reemplaza=matches!(&accion,Accion::Abrir(_)) || self.obsoleto();
        if self.tiene_ediciones() && reemplaza {self.confirmar=Some(accion);} else {self.ejecutar(accion,ctx);}
    }
    fn ejecutar(&mut self, accion: Accion, ctx: &egui::Context) {
        if self.receptor.is_some() {return}
        let (tx,rx)=channel(); let pintar=ctx.clone(); self.cancelar=Arc::new(AtomicBool::new(false));
        let cancelar=self.cancelar.clone();
        match accion {
            Accion::Abrir(path) => {
                self.mensaje="Validando paquete y cobertura…".into();
                std::thread::spawn(move|| {
                    let r=(|| {
                        let archivo=std::fs::File::open(&path).map_err(|e|e.to_string())?;
                        let mut datos=Vec::new(); archivo.take(80*1024*1024+1).read_to_end(&mut datos).map_err(|e|e.to_string())?;
                        let periodo=paquete::periodo_revision(&datos)?;
                        if cancelar.load(Ordering::Relaxed) {return Err("Apertura cancelada.".into())}
                        Ok((path,Arc::new(datos),periodo))
                    })();
                    let _=tx.send(Evento::Abierto(r));pintar.request_repaint();
                });
            }
            Accion::Analizar => {
                let (desde,hasta,corte)=match self.criterio(){Ok(v)=>v,Err(e)=>{self.mensaje=e;return}};
                let Some(datos)=self.datos.clone() else{return};
                let anterior=self.informe.clone();
                self.mensaje="Clasificando bitácoras por ingreso…".into();
                std::thread::spawn(move|| {
                    let r=if let Some(mut informe)=anterior {
                        informe.cambiar_periodo(desde,hasta,corte).map(|_|informe)
                    }else{paquete::revisar_bitacoras(&datos,desde,hasta,corte)};
                    let _=tx.send(Evento::Informe(if cancelar.load(Ordering::Relaxed){Err("Análisis cancelado.".into())}else{r},true));pintar.request_repaint();
                });
            }
        }
        self.receptor=Some(rx);
    }
    fn instalar(&mut self, informe: Informe, nueva: bool) {
        if nueva {self.seleccion.clear();self.activa=None;self.resultado=None;}
        self.busquedas=informe.filas.iter().map(|f|format!("{} {} {} {} {}",f.identidad.tribunal,f.identidad.causa,f.identidad.ingreso,f.tipo,f.valores).to_lowercase()).collect();
        self.tribunales=informe.filas.iter().map(|f|f.identidad.tribunal.clone()).collect::<BTreeSet<_>>().into_iter().collect();
        self.programas=informe.filas.iter().map(|f|valor(f,&["PROGRAMA","NOMBRE PROGRAMA"])).filter(|s|!s.is_empty()).collect::<BTreeSet<_>>().into_iter().collect();
        self.informe=Some(informe);
    }
    fn motor(&mut self, config: &Config, ctx: &egui::Context) {
        if self.obsoleto() || self.receptor.is_some() {self.mensaje="Vuelve a analizar el criterio actual antes de aplicar el motor.".into();return}
        let (Some(mut informe),Some(datos),Some(path))=(self.informe.clone(),self.datos.clone(),self.archivo.clone())else{return};
        let config=config.clone();let (tx,rx)=channel();let pintar=ctx.clone();
        self.cancelar=Arc::new(AtomicBool::new(false));let cancelar=self.cancelar.clone();
        self.mensaje="Aplicando las reglas con el contexto completo del paquete…".into();
        std::thread::spawn(move||{
            let r=revision_informe::aplicar_motor(&mut informe,&path.file_name().unwrap_or_default().to_string_lossy(),&datos,&config).map(|_|informe);
            let _=tx.send(Evento::Informe(if cancelar.load(Ordering::Relaxed){Err("Aplicación cancelada; se conserva el listado anterior.".into())}else{r},false));pintar.request_repaint();
        });self.receptor=Some(rx);
    }
    fn exportar(&mut self, ctx: &egui::Context) {
        if self.obsoleto() || self.receptor.is_some() {self.mensaje="Vuelve a analizar el criterio actual antes de exportar.".into();return}
        if !self.exportar_todos && self.seleccion.is_empty() {self.mensaje="Selecciona registros o elige Todos los clasificados.".into();return}
        let Some(informe)=self.informe.clone()else{return};
        let Some(destino)=rfd::FileDialog::new().set_title("Destino de la revisión de bitácoras").pick_folder()else{return};
        let seleccion=if self.exportar_todos{None}else{Some(self.seleccion.iter().copied().collect::<Vec<_>>())};
        let(tx,rx)=channel();let pintar=ctx.clone();
        self.cancelar=Arc::new(AtomicBool::new(false));let cancelar=self.cancelar.clone();
        self.mensaje="Preparando Excel e informe JSON…".into();
        std::thread::spawn(move||{
            let r=revision::exportar(&informe,seleccion.as_deref(),&destino,&cancelar);
            let _=tx.send(Evento::Exportado(r));pintar.request_repaint();
        });self.receptor=Some(rx);
    }
    fn recibir(&mut self) {
        let evento=match self.receptor.as_ref().map(|r|r.try_recv()) {
            Some(Ok(e))=>Some(e),
            Some(Err(std::sync::mpsc::TryRecvError::Disconnected))=>{
                self.receptor=None;self.mensaje="La operación no pudo completarse; se conserva la revisión anterior.".into();None
            }
            _=>None,
        };
        let Some(evento)=evento else{return};self.receptor=None;
        match evento {
            Evento::Abierto(Ok((path,datos,periodo))) if !self.cancelar.load(Ordering::Relaxed) => {
                self.archivo=Some(path);self.datos=Some(datos);self.informe=None;self.seleccion.clear();self.activa=None;self.resultado=None;
                if let Some((a,b))=periodo {self.desde=a.to_string();self.hasta=b.to_string();self.corte=b.to_string();self.mensaje="Paquete validado. Confirma el período y pulsa Analizar bitácoras.".into();}
                else {self.desde.clear();self.hasta.clear();self.mensaje="El paquete no acredita un intervalo. Indica las fechas; la ausencia sin cobertura quedará No verificable.".into();}
            }
            Evento::Abierto(Ok(_)) => self.mensaje="Apertura cancelada; se conserva el paquete anterior.".into(),
            Evento::Abierto(Err(e))|Evento::Informe(Err(e),_)|Evento::Exportado(Err(e))=>self.mensaje=e,
            Evento::Informe(Ok(_),_) if self.cancelar.load(Ordering::Relaxed)=>self.mensaje="Operación cancelada; se conserva el listado anterior.".into(),
            Evento::Informe(Ok(i),nueva)=>{self.instalar(i,nueva);self.mensaje="Listado actualizado. Revisa la evidencia antes de exportar.".into();}
            Evento::Exportado(Ok(p))=>{self.mensaje=format!("Revisión exportada en {}.",p.carpeta.display());self.resultado=Some(p.carpeta);}
        }
    }

    pub fn mostrar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, config: &Config) {
        self.recibir();let ocupada=self.receptor.is_some();let obsoleto=self.obsoleto();
        let compacto=ui.available_width()<1000.0||ui.available_height()<480.0;
        if !compacto {
            ui.heading("Revisión de bitácoras");
            ui.label("Sin observación del centro en el período verificable; las reglas generan propuestas para revisión humana.");
        }
        ui.horizontal_wrapped(|ui|{
            if compacto {for (p,n) in [(0,"Lista"),(1,"Evidencia"),(2,"Propuesta"),(3,"Configurar")]{ui.selectable_value(&mut self.panel,p,n);}}
            if ui.button("Restaurar distribución").clicked(){self.distribucion=self.distribucion.wrapping_add(1);self.panel=0;}
        });
        let panel=self.panel;
        if !compacto||self.datos.is_none(){self.criterios(ui,ctx,ocupada);}
        if let Some(p)=&self.archivo {ui.small(if compacto{p.file_name().unwrap_or_default().to_string_lossy().into_owned()}else{p.display().to_string()}).on_hover_text(p.display().to_string());}
        if obsoleto {ui.colored_label(ui.visuals().warn_fg_color,"⚠ Cambió el criterio: vuelve a analizar para aplicar reglas o exportar.");}
        if ocupada {ui.horizontal(|ui|{ui.spinner();if ui.button("Cancelar").clicked(){self.cancelar.store(true,Ordering::Relaxed);self.mensaje="Cancelando al terminar la operación actual…".into();}});}
        if compacto{ui.small(&self.mensaje);}else{ui.label(&self.mensaje);}
        if let Some(path)=&self.resultado {if ui.button("Abrir resultados").clicked(){crate::ui::abrir_en_sistema(path);}}
        let Some(informe)=self.informe.as_ref()else{self.confirmacion(ctx);return};
        let conteos=informe.conteos();let n=conteos[0];let aplicado=informe.motor_aplicado;
        let clases=[(C::SinObservacion,conteos[0]),(C::ConObservacion,conteos[1]),(C::NoVerificable,conteos[2]),(C::FueraDeAlcance,conteos[3])];
        ui.horizontal_wrapped(|ui|{
            if compacto {
                ui.label(format!("{} registros",informe.filas.len()));
                egui::ComboBox::from_id_salt("clasificacion_compacta").selected_text(revision::etiqueta(self.clase)).show_ui(ui,|ui|{
                    for (clase,cantidad) in clases{ui.selectable_value(&mut self.clase,clase,format!("{} ({cantidad})",revision::etiqueta(clase)));}
                });
            }else{
                ui.label(format!("{} registros lógicos · {} filas fuente · {} duplicados",informe.filas.len(),informe.filas_fuente,informe.duplicados_registros));
                for (clase,cantidad) in clases{ui.selectable_value(&mut self.clase,clase,format!("{} ({cantidad})",revision::etiqueta(clase)));}
            }
        });
        if !compacto{self.acciones(ui,ctx,config,ocupada,obsoleto,n,aplicado);}
        if !compacto{self.filtros(ui);}
        if compacto&&panel==3 {
            egui::ScrollArea::vertical().id_salt("revision_configuracion").show(ui,|ui|{
                self.criterios(ui,ctx,ocupada);
                ui.separator();self.acciones(ui,ctx,config,ocupada,obsoleto,n,aplicado);
                ui.separator();self.filtros(ui);
            });
            self.confirmacion(ctx);return;
        }
        let informe=self.informe.as_mut().unwrap();
        let buscar=self.buscar.to_lowercase();
        let visibles=informe.filas.iter().enumerate().filter(|(i,f)|f.revision.clasificacion==self.clase &&
            (self.tribunal.is_empty()||f.identidad.tribunal==self.tribunal)&&(self.etapa.is_empty()||f.tipo==self.etapa)&&
            (self.programa.is_empty()||valor(f,&["PROGRAMA","NOMBRE PROGRAMA"])==self.programa)&&
            (self.estado_revision==0||(self.estado_revision==2)==f.revisada)&&self.busquedas[*i].contains(&buscar)).map(|(i,_)|i).collect::<Vec<_>>();
        ui.horizontal_wrapped(|ui|{
            ui.label(if compacto{format!("{} visibles · {} seleccionados",visibles.len(),self.seleccion.len())}else{format!("{} visibles · {} seleccionados (también pueden estar ocultos)",visibles.len(),self.seleccion.len())});
            if ui.button("Siguiente pendiente").clicked(){
                self.activa=visibles.iter().copied().filter(|&i|!informe.filas[i].revisada).find(|i|self.activa.is_none_or(|a|*i>a))
                    .or_else(||visibles.iter().copied().find(|&i|!informe.filas[i].revisada));
            }
            if ui.button("Quitar selección").clicked(){self.seleccion.clear();}
        });
        if self.activa.is_none(){self.activa=visibles.first().copied();}
        let mut revision_cambio=None;
        let mut editor=|ui:&mut egui::Ui|{
            if let Some(f)=self.activa.and_then(|i|informe.filas.get_mut(i)){
                egui::ScrollArea::vertical().id_salt("revision_editor_scroll").show(ui,|ui|{
                    ui.strong("Propuesta y revisión");ui.small(format!("{} · ingreso {}",valor(f,&["RIT"]),f.identidad.ingreso));
                    let mut revisada=f.revisada;
                    if ui.add_enabled(!ocupada && !obsoleto,egui::Checkbox::new(&mut revisada,"Revisada")).changed(){
                        if let Some(i)=self.activa{revision_cambio=Some((i,revisada));}
                    }
                    if let Some(p)=&f.propuesta {
                        if p.excluido {ui.label("El motor excluye este registro.");}
                        if p.observacion.is_empty(){ui.label("El motor no produjo texto de observación.");}
                        let mut texto=f.edicion.clone().unwrap_or_else(||p.observacion.clone());
                        if ui.add_enabled(!ocupada && !obsoleto,egui::TextEdit::multiline(&mut texto).desired_width(f32::INFINITY).desired_rows(if compacto{3}else{4})).changed(){f.edicion=Some(texto);}
                        ui.horizontal(|ui|{
                            if ui.add_enabled(f.edicion.is_some()&&!ocupada&&!obsoleto,egui::Button::new("Restaurar propuesta")).clicked(){f.edicion=None;}
                            if ui.button("Copiar observación").clicked(){ui.ctx().copy_text(f.edicion.as_ref().unwrap_or(&p.observacion).clone());}
                        });
                        ui.collapsing("Propuesta original, reglas y avisos",|ui|{ui.label(&p.observacion);ui.label(p.reglas.join(" · "));for a in &p.avisos{ui.label(a);}});
                    }else{ui.label("Sin propuesta aplicada. Solo los registros con ausencia verificada pueden recibirla.");}

                });
            }else{ui.label("Elige un registro de la tabla.");}
        };
        if compacto {if panel==2{editor(ui);}}else{
            egui::TopBottomPanel::bottom(egui::Id::new(("revision_editor",self.distribucion))).resizable(true).default_height(190.0).min_height(100.0).show_inside(ui,&mut editor);
        }
        let mut evidencia=|ui:&mut egui::Ui|{
            egui::ScrollArea::vertical().id_salt("revision_evidencia_scroll").show(ui,|ui|{
                ui.strong("Evidencia del ingreso");
                if let Some(f)=self.activa.and_then(|i|informe.filas.get(i)){
                    if !visibles.contains(&self.activa.unwrap()){ui.label("El registro activo está oculto por los filtros.");}
                    ui.label(format!("{} · {} · causa {} · ingreso {}",f.identidad.sistema,f.identidad.tribunal,f.identidad.causa,f.identidad.ingreso));
                    ui.label(revision::etiqueta(f.revision.clasificacion));ui.label(&f.revision.motivo);
                    ui.label(format!("{} eventos elegibles · última fecha {}",f.revision.eventos_elegibles,f.revision.ultima_fecha.map(|d|d.to_string()).unwrap_or_else(||"Sin fecha".into())));
                    ui.label(format!("Período {} a {} · corte {}",informe.desde,informe.hasta,informe.corte));
                    for p in &f.procedencia {ui.small(format!("{} · consulta {} · {} / {} / fila {} · {}",p.registro_id,p.consulta_id,p.archivo,p.tabla,p.fila,p.asociacion));}
                    for (i,l) in f.lecturas.iter().enumerate() {
                        ui.collapsing(format!("Lectura {} · {}",i+1,l["coverage"].as_str().unwrap_or("Desconocida")),|ui|{
                            ui.label(format!("Capturada: {}",l["captured_at"].as_str().unwrap_or("Sin fecha acreditada")));
                            let evidencia=&l["evidencia"];
                            if evidencia["version"]!=1 {ui.label("Este paquete no conserva la evidencia original necesaria para acreditar una ausencia.");return}
                            ui.label(format!("Intervalo de lectura: {} a {}",evidencia["desde"].as_str().unwrap_or("desconocido"),evidencia["hasta"].as_str().unwrap_or("desconocido")));
                            let entradas=evidencia["entries"].as_array();
                            if entradas.is_some_and(Vec::is_empty){ui.label("Lectura sin entradas capturadas. La clasificación depende de su cobertura verificada.");}
                            for (n,e) in entradas.into_iter().flatten().take(200).enumerate(){
                                ui.collapsing(format!("Entrada {} · {} · {}",n+1,e["fecha"].as_str().unwrap_or("Fecha ilegible"),e["autor"].as_str().unwrap_or("Autor desconocido")),|ui|{
                                    ui.label(format!("{} · {}",e["tipo"].as_str().unwrap_or("Tipo desconocido"),e["etapa"].as_str().unwrap_or("Etapa desconocida")));
                                    ui.label(match e["origen"].as_str(){Some("centro")=>"Autoría del centro acreditada por la lectura",Some("tribunal"|"programa")=>"Entrada de otro actor",_=>"Autoría desconocida"});
                                    ui.label(e["texto"].as_str().unwrap_or("Texto ilegible"));
                                });
                            }
                            if entradas.is_some_and(|e|e.len()>200){ui.label("Se muestran las primeras 200 entradas. El informe JSON conserva la evidencia completa.");}
                        });
                    }
                    ui.collapsing("Datos de origen",|ui|{for (campo,v) in f.valores.as_object().into_iter().flat_map(|m|m.iter()){ui.label(format!("{campo}: {}",v.as_str().map(str::to_owned).unwrap_or_else(||v.to_string())));}});
                }
            });
        };
        if compacto {if panel==1{evidencia(ui);}}else{
            egui::SidePanel::right(egui::Id::new(("revision_evidencia",self.distribucion))).resizable(true).default_width(350.0).min_width(200.0).show_inside(ui,&mut evidencia);
        }
        if !compacto||panel==0 {
        egui::CentralPanel::default().show_inside(ui,|ui|{
            TableBuilder::new(ui).striped(true).sense(egui::Sense::click())
                .column(Column::exact(26.0)).column(Column::initial(100.0).resizable(true)).column(Column::initial(160.0).resizable(true))
                .column(Column::initial(140.0).resizable(true)).column(Column::remainder())
                .header(24.0,|mut row|{for h in ["☐","RIT / causa","Nombre","Programa","Control"]{row.col(|ui|{ui.strong(h);});}})
                .body(|body|body.rows(28.0,visibles.len(),|mut row|{
                    let i=visibles[row.index()];let f=&informe.filas[i];row.set_selected(self.activa==Some(i));
                    row.col(|ui|{
                        let mut seleccionada=self.seleccion.contains(&i);
                        if ui.checkbox(&mut seleccionada,"").on_hover_text("Seleccionar para exportación; no marca Revisada.").changed(){
                            if seleccionada{self.seleccion.insert(i);}else{self.seleccion.remove(&i);}
                        }
                    });
                    let rit=valor(f,&["RIT"]);let rit=if rit.is_empty(){f.identidad.causa.clone()}else{rit};
                    for texto in [rit,valor(f,&["NOMBRE","NOMBRE NNA","NOMBRE MENOR"]),valor(f,&["PROGRAMA","NOMBRE PROGRAMA"]),f.revision.motivo.clone()] {
                        row.col(|ui|{ui.add(egui::Label::new(texto).truncate().selectable(false));});
                    }
                    if row.response().clicked(){self.activa=Some(i);}
                }));
        });
        }
        if let Some((i,revisada))=revision_cambio{self.marcar_revisada(i,revisada);}
        self.confirmacion(ctx);
    }
    fn criterios(&mut self,ui:&mut egui::Ui,ctx:&egui::Context,ocupada:bool){
        ui.add_enabled_ui(!ocupada,|ui|{
            ui.horizontal_wrapped(|ui|{
                if ui.button("Abrir paquete…").clicked(){if let Some(path)=rfd::FileDialog::new().add_filter("Paquete CSMP",&["zip"]).pick_file(){self.solicitar(Accion::Abrir(path),ctx);}}
                for (nombre,valor) in [("Desde",&mut self.desde),("Hasta",&mut self.hasta),("Corte",&mut self.corte)] {
                    ui.label(nombre);ui.add(egui::TextEdit::singleline(valor).desired_width(100.0));
                }
                if ui.add_enabled(self.datos.is_some(),egui::Button::new("Analizar bitácoras")).clicked(){self.solicitar(Accion::Analizar,ctx);}
            });
        });
    }

    fn acciones(&mut self,ui:&mut egui::Ui,ctx:&egui::Context,config:&Config,ocupada:bool,obsoleto:bool,n:usize,aplicado:bool){
        ui.add_enabled_ui(!ocupada && !obsoleto,|ui|{
            ui.horizontal_wrapped(|ui|{
                if ui.add_enabled(n>0 && !aplicado,egui::Button::new(format!("Aplicar motor a {n} registros sin observación"))).on_hover_text("Conserva el conjunto completo para duplicidades y acumulados. No crea correos ni resoluciones.").clicked(){self.motor(config,ctx);}
                ui.selectable_value(&mut self.exportar_todos,false,format!("Seleccionados ({})",self.seleccion.len()));
                ui.selectable_value(&mut self.exportar_todos,true,"Todos los clasificados");
                if ui.add_enabled(self.exportar_todos || !self.seleccion.is_empty(),egui::Button::new("Exportar listado…")).clicked(){self.exportar(ctx);}
            });
        });
    }

    fn filtros(&mut self,ui:&mut egui::Ui){
        ui.horizontal_wrapped(|ui|{
            ui.label("Tribunal");combo(ui,"revision_tribunal",&mut self.tribunal,&self.tribunales);
            ui.label("Programa");combo(ui,"revision_programa",&mut self.programa,&self.programas);
            ui.label("Etapa");combo(ui,"revision_etapa",&mut self.etapa,&["ESPERA".into(),"CUMPLIMIENTO".into(),"INFORMES".into(),"EGRESADOS".into()]);
            egui::ComboBox::from_id_salt("revision_revisada").selected_text(["Toda revisión","Pendientes","Revisadas"][self.estado_revision as usize]).show_ui(ui,|ui|{
                for (v,n) in [(0,"Toda revisión"),(1,"Pendientes"),(2,"Revisadas")] {ui.selectable_value(&mut self.estado_revision,v,n);}
            });
            ui.add(egui::TextEdit::singleline(&mut self.buscar).hint_text("Buscar causa, nombre o programa…").desired_width(220.0));
            if ui.button("Limpiar filtros").clicked(){self.buscar.clear();self.tribunal.clear();self.programa.clear();self.etapa.clear();self.estado_revision=0;}
        });
    }
    fn confirmacion(&mut self, ctx: &egui::Context) {
        if self.confirmar.is_none(){return}let mut ejecutar=None;
        egui::Window::new("Descartar revisión actual").collapsible(false).resizable(false).show(ctx,|ui|{
            ui.label("Cambiar el paquete o período descartará las propuestas, ediciones y marcas de esta revisión.");
            ui.horizontal(|ui|{
                if ui.button("Conservar revisión").clicked(){self.confirmar=None;}
                if ui.button("Descartar y continuar").clicked(){ejecutar=self.confirmar.take();}
            });
        });
        if let Some(a)=ejecutar{self.ejecutar(a,ctx);}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ui() -> RevisionUi {
        let d:Fecha="2026-11-17".parse().unwrap();
        let mut ui=RevisionUi::default();
        ui.desde=d.to_string();ui.hasta=d.to_string();ui.corte=d.to_string();
        ui.informe=Some(Informe {version:1,contrato:"1.0.0".into(),huella_paquete:"a".repeat(64),
            desde:d,hasta:d,corte:d,filas_fuente:0,duplicados_registros:0,filas:vec![],motor_aplicado:false,version_reglas:None});
        ui
    }
    #[test]
    fn criterio_obsoleto_bloquea_motor_y_exportacion_antes_de_efectos() {
        let mut ui=ui();assert!(!ui.obsoleto());
        ui.desde="2026-11-16".into();assert!(ui.obsoleto());
        let ctx=egui::Context::default();
        ui.motor(&Config::default(),&ctx);assert!(ui.receptor.is_none());
        ui.exportar(&ctx);assert!(ui.receptor.is_none());assert!(ui.resultado.is_none());
        assert!(ui.mensaje.contains("criterio"));
    }
    #[test]
    fn cancelacion_tardia_y_worker_fallido_conservan_informe_anterior() {
        let mut ui=ui();let anterior=ui.informe.clone().unwrap();
        let(tx,rx)=channel();ui.receptor=Some(rx);
        let mut nueva=anterior.clone();nueva.contrato="no-debe-instalarse".into();
        tx.send(Evento::Informe(Ok(nueva),true)).unwrap();
        ui.cancelar.store(true,Ordering::Relaxed);ui.recibir();
        assert_eq!(ui.informe.as_ref().unwrap().contrato,anterior.contrato);
        let(tx,rx)=channel();ui.receptor=Some(rx);drop(tx);ui.recibir();
        assert!(ui.receptor.is_none());assert_eq!(ui.informe.as_ref().unwrap().contrato,anterior.contrato);
    }
    #[test]
    fn se_revisan_todas_las_categorias_aunque_no_tengan_propuesta() {
        use flow_trabajo::revision_bitacoras::{Evidencia,clasificar};
        use flow_trabajo::revision_informe::Identidad;
        let mut ui=ui();let d=ui.informe.as_ref().unwrap().corte;
        let evidencia=Evidencia {admitido:true,identidad_verificada:true,contradictoria:false,cobertura_completa:Some((d,d)),eventos:Some(vec![])};
        ui.informe.as_mut().unwrap().filas.push(FilaRevision {
            identidad:Identidad{sistema:"RUS".into(),tribunal:"901".into(),causa:"100".into(),ingreso:"201".into()},
            tipo:"ESPERA".into(),valores:serde_json::json!({}),procedencia:vec![],lecturas:vec![],
            revision:clasificar(&evidencia,d,d).unwrap(),evidencia,propuesta:None,edicion:None,revisada:false,
        });
        for c in [C::SinObservacion,C::ConObservacion,C::NoVerificable,C::FueraDeAlcance] {
            ui.informe.as_mut().unwrap().filas[0].revision.clasificacion=c;
            assert!(ui.marcar_revisada(0,false));assert!(ui.marcar_revisada(0,true));
            assert!(ui.informe.as_ref().unwrap().filas[0].revisada);
        }
        ui.desde="2026-11-16".into();
        assert!(!ui.marcar_revisada(0,false));
        assert!(ui.informe.as_ref().unwrap().filas[0].revisada);
    }
}





