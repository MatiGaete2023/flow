//! Tema compartido Estudio D. No modifica reglas ni estado persistido.
use eframe::egui::{self, Color32, FontId, Stroke, TextStyle};

pub fn aplicar(ctx: &egui::Context) {
    let mut estilo = (*ctx.style()).clone();
    let fondo = Color32::from_rgb(32, 36, 58);
    let superficie = Color32::from_rgb(42, 48, 72);
    let lavanda = Color32::from_rgb(202, 183, 239);
    let menta = Color32::from_rgb(197, 245, 222);
    let texto = Color32::from_rgb(242, 244, 250);
    estilo.visuals = egui::Visuals::dark();
    estilo.visuals.override_text_color = Some(texto);
    estilo.visuals.panel_fill = fondo;
    estilo.visuals.window_fill = superficie;
    estilo.visuals.extreme_bg_color = fondo;
    estilo.visuals.faint_bg_color = superficie;
    estilo.visuals.selection.bg_fill = Color32::from_rgb(74, 65, 99);
    estilo.visuals.selection.stroke = Stroke::new(1.5, lavanda);
    estilo.visuals.widgets.inactive.bg_fill = superficie;
    estilo.visuals.widgets.hovered.bg_stroke = Stroke::new(1.5, lavanda);
    estilo.visuals.widgets.active.bg_stroke = Stroke::new(2.0, menta);
    estilo.visuals.warn_fg_color = Color32::from_rgb(255, 214, 128);
    estilo.visuals.error_fg_color = Color32::from_rgb(255, 158, 166);
    estilo.spacing.item_spacing = egui::vec2(8.0, 8.0);
    estilo.spacing.interact_size.y = 28.0;
    estilo.text_styles.insert(TextStyle::Body, FontId::proportional(15.0));
    estilo.text_styles.insert(TextStyle::Button, FontId::proportional(15.0));
    ctx.set_style(estilo);
}
