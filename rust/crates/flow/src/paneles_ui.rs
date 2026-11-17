//! Selectores de lista con ficha; en ventanas pequeñas utilizan un único panel.
use eframe::egui;
pub fn configuracion(ui:&mut egui::Ui,compacto:bool,titulo:&str,contenido:impl FnOnce(&mut egui::Ui)){
    if compacto {ui.collapsing(titulo,contenido);}else{contenido(ui);}
}
pub fn selector(ui:&mut egui::Ui,id:&str,titulos:&[String],seleccion:&mut Option<usize>)->Option<usize>{
    if seleccion.is_none_or(|i|i>=titulos.len()){*seleccion=if titulos.is_empty(){None}else{Some(0)};}
    if ui.available_width()<950.0 {
        egui::ComboBox::from_id_salt(id).width(ui.available_width().min(600.0))
            .selected_text(seleccion.and_then(|i|titulos.get(i)).map(String::as_str).unwrap_or("Sin elementos"))
            .show_ui(ui,|ui|{for (i,titulo) in titulos.iter().enumerate(){ui.selectable_value(seleccion,Some(i),titulo);}});
    }else{
        egui::SidePanel::left(egui::Id::new(id)).resizable(true).default_width(300.0).min_width(190.0).show_inside(ui,|ui|{
            ui.strong(format!("{} elementos",titulos.len()));
            egui::ScrollArea::vertical().id_salt((id,"lista")).show(ui,|ui|{
                for (i,titulo) in titulos.iter().enumerate(){ui.selectable_value(seleccion,Some(i),titulo);}
            });
        });
    }
    *seleccion
}

