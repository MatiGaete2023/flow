//! Tema compartido Estudio D. No modifica reglas ni estado persistido.
use eframe::egui::{self, Color32, FontId, Stroke, TextStyle};
pub const ERROR:Color32=Color32::from_rgb(255,158,166);
pub const AVISO:Color32=Color32::from_rgb(255,214,128);

pub fn aplicar(ctx: &egui::Context) {
    // El estilo elegido debe persistir después del primer evento de tema de Windows.
    ctx.set_theme(egui::Theme::Dark);
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
    estilo.visuals.selection.stroke = Stroke::new(1.5_f32, lavanda);
    estilo.visuals.widgets.inactive.bg_fill = superficie;
    estilo.visuals.widgets.hovered.bg_stroke = Stroke::new(1.5_f32, lavanda);
    estilo.visuals.widgets.active.bg_stroke = Stroke::new(2.0_f32, menta);
    estilo.visuals.warn_fg_color = AVISO;
    estilo.visuals.error_fg_color = ERROR;
    estilo.spacing.item_spacing = egui::vec2(8.0, 8.0);
    estilo.spacing.interact_size.y = 28.0;
    estilo.text_styles.insert(TextStyle::Body, FontId::proportional(15.0));
    estilo.text_styles.insert(TextStyle::Button, FontId::proportional(15.0));
    ctx.set_style_of(egui::Theme::Dark, estilo);
}

#[cfg(test)]
mod tests {
    fn luminancia(rgb:[u8;3])->f64{
        let c=rgb.map(|v|{let s=v as f64/255.0;if s<=0.04045{s/12.92}else{((s+0.055)/1.055).powf(2.4)}});
        0.2126*c[0]+0.7152*c[1]+0.0722*c[2]
    }
    #[test]
    fn texto_y_estados_superan_contraste_minimo_en_superficies_d(){
        for fondo in [[32,36,58],[42,48,72],[74,65,99]]{
            for texto in [[242,244,250],[202,183,239],[197,245,222],[255,158,166],[255,214,128]]{
                let contraste=(luminancia(texto)+0.05)/(luminancia(fondo)+0.05);
                assert!(contraste>=4.5,"Contraste {contraste} insuficiente para {texto:?}/{fondo:?}");
            }
        }
    }
}

