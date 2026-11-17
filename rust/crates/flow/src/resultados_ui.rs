//! Biblioteca local de productos. Las rutas proceden de carpetas de salida conocidas, nunca de mensajes.
use eframe::egui;
use egui_extras::{Column,TableBuilder};
use std::{collections::BTreeSet,path::{Path,PathBuf},time::{Instant,Duration}};

#[derive(Default)]
pub struct Biblioteca {
    carpetas:BTreeSet<PathBuf>, archivos:Vec<PathBuf>, buscar:String, tipo:String,
    ultima:Option<Instant>, aviso:String,
}
fn explorar(carpeta:&Path, profundidad:usize, presupuesto:&mut usize, salida:&mut Vec<PathBuf>) {
    if profundidad>4 || *presupuesto==0 {return}
    let Ok(entradas)=std::fs::read_dir(carpeta)else{return};
    for entrada in entradas.flatten() {
        if *presupuesto==0 {break} *presupuesto-=1;
        let Ok(tipo)=entrada.file_type()else{continue};
        if tipo.is_symlink(){continue}
        let p=entrada.path();
        if tipo.is_dir(){explorar(&p,profundidad+1,presupuesto,salida);}
        else if tipo.is_file() && p.extension().and_then(|s|s.to_str()).is_some_and(|s|
            ["xlsx","xls","docx","eml","json","ics","zip","pdf","csv"].contains(&s.to_ascii_lowercase().as_str())) {
            salida.push(p);
        }
    }
}
impl Biblioteca {
    pub fn recordar(&mut self, carpeta:Option<&Path>) {
        if let Some(p)=carpeta {if self.carpetas.insert(p.to_path_buf()){self.ultima=None;}}
    }
    fn actualizar(&mut self) {
        self.archivos.clear();let mut presupuesto=4000;
        for carpeta in &self.carpetas {explorar(carpeta,0,&mut presupuesto,&mut self.archivos);}
        self.archivos.sort();self.archivos.dedup();self.ultima=Some(Instant::now());
        self.aviso=if presupuesto==0 {"Se alcanzó el límite de exploración; abre la carpeta para consultar todos sus archivos.".into()}else{String::new()};
    }
    pub fn mostrar(&mut self,ui:&mut egui::Ui,carpeta:Option<&Path>) {
        self.recordar(carpeta);
        if self.ultima.is_none_or(|t|t.elapsed()>Duration::from_secs(2)){self.actualizar();}
        ui.horizontal_wrapped(|ui|{
            ui.add(egui::TextEdit::singleline(&mut self.buscar).hint_text("Buscar producto…").desired_width(220.0));
            egui::ComboBox::from_id_salt("tipo_resultado").selected_text(if self.tipo.is_empty(){"Todos los tipos"}else{self.tipo.as_str()}).show_ui(ui,|ui|{
                ui.selectable_value(&mut self.tipo,String::new(),"Todos los tipos");
                for tipo in ["xlsx","xls","docx","eml","json","ics","zip","pdf","csv"] {ui.selectable_value(&mut self.tipo,tipo.into(),tipo.to_uppercase());}
            });
            if ui.button("Actualizar biblioteca").clicked(){self.actualizar();}
            if ui.button("Limpiar filtros").clicked(){self.buscar.clear();self.tipo.clear();}
        });
        ui.collapsing(format!("Carpetas de productos existentes ({})",self.carpetas.len()),|ui|{
            for p in &self.carpetas {ui.horizontal_wrapped(|ui|{
                if ui.add_enabled(p.is_dir(),egui::Button::new("Abrir carpeta")).clicked(){crate::ui::abrir_en_sistema(p);}
                ui.label(p.display().to_string());
            });}
        });
        if !self.aviso.is_empty(){ui.label(&self.aviso);}
        let buscar=self.buscar.to_lowercase();
        let visibles=self.archivos.iter().filter(|p|
            (self.tipo.is_empty()||p.extension().is_some_and(|e|e.to_string_lossy().eq_ignore_ascii_case(&self.tipo))) &&
            p.file_name().is_some_and(|n|n.to_string_lossy().to_lowercase().contains(&buscar))).collect::<Vec<_>>();
        ui.label(format!("{} productos visibles de {}",visibles.len(),self.archivos.len()));
        if self.archivos.is_empty(){ui.label("Todavía no hay archivos en las carpetas de productos de esta sesión.");}
        TableBuilder::new(ui).striped(true).column(Column::initial(260.0).resizable(true))
            .column(Column::exact(60.0)).column(Column::remainder()).column(Column::exact(65.0))
            .header(25.0,|mut row|{for titulo in ["Producto","Tipo","Ubicación","Abrir"]{row.col(|ui|{ui.strong(titulo);});}})
            .body(|body|body.rows(28.0,visibles.len(),|mut row|{
                let p=visibles[row.index()];
                row.col(|ui|{ui.add(egui::Label::new(p.file_name().unwrap_or_default().to_string_lossy()).truncate());});
                row.col(|ui|{ui.label(p.extension().unwrap_or_default().to_string_lossy().to_uppercase());});
                row.col(|ui|{ui.add(egui::Label::new(p.parent().unwrap_or(Path::new("")).display().to_string()).truncate()).on_hover_text(p.display().to_string());});
                row.col(|ui|{if ui.add_enabled(p.is_file(),egui::Button::new("Abrir")).clicked(){crate::ui::abrir_en_sistema(p);}});
            }));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn biblioteca_conserva_carpetas_previas_y_no_confunde_mensajes_con_rutas() {
        let base=tempfile::tempdir().unwrap();let a=base.path().join("primera");let b=base.path().join("segunda");
        std::fs::create_dir(&a).unwrap();std::fs::create_dir(&b).unwrap();
        std::fs::write(a.join("observaciones.xlsx"),b"ficticio").unwrap();
        std::fs::write(a.join("no-producto.tmp"),b"ficticio").unwrap();
        std::fs::write(b.join("resoluciones.docx"),b"ficticio").unwrap();
        let mut biblioteca=Biblioteca::default();biblioteca.recordar(Some(&a));biblioteca.recordar(None);biblioteca.recordar(Some(&b));
        biblioteca.actualizar();assert_eq!(biblioteca.carpetas.len(),2);assert_eq!(biblioteca.archivos.len(),2);
        let mut salida=vec![];explorar(base.path(),0,&mut 0,&mut salida);assert!(salida.is_empty());
    }
    #[cfg(unix)]
    #[test]
    fn biblioteca_no_sigue_enlaces_a_archivos_o_carpetas_ajenas() {
        let base=tempfile::tempdir().unwrap();let externa=tempfile::tempdir().unwrap();
        std::fs::write(externa.path().join("ajeno.xlsx"),b"ficticio").unwrap();
        std::os::unix::fs::symlink(externa.path(),base.path().join("enlace")).unwrap();
        std::os::unix::fs::symlink(externa.path().join("ajeno.xlsx"),base.path().join("enlace.xlsx")).unwrap();
        let mut salida=vec![];explorar(base.path(),0,&mut 100,&mut salida);assert!(salida.is_empty());
    }
}

