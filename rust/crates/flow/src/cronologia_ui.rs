use eframe::egui;
use flow_motor::Config;
use flow_trabajo::{cronologia, FilaTrabajo};
use serde_json::Value;

fn texto<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap_or("")
}
fn fecha(s: &str) -> String {
    s.get(..10).and_then(|s| s.parse::<chrono::NaiveDate>().ok()).map(|d| d.format("%d/%m/%Y").to_string()).unwrap_or_default()
}
fn entradas(ui: &mut egui::Ui, display: &Value, clave: &str, titulo: &str) {
    let Some(eventos) = display[clave].as_array().filter(|e| !e.is_empty()) else {
        return;
    };
    ui.label(egui::RichText::new(titulo).strong());
    for e in eventos {
        let cuando = texto(e, "event_at");
        let hora = cuando.get(11..16).unwrap_or("sin hora");
        ui.group(|ui| {
            ui.label(format!(
                "{} {} · {} · {}",
                fecha(cuando),
                hora,
                texto(e, "author"),
                if e["origin"] == "CENTRO" { "Centro" } else { "Origen no acreditado del centro" }
            ));
            ui.label(texto(e, "text"));
            if !texto(e, "response").trim().is_empty() {
                ui.label(format!("Respuesta del tribunal ({}): {}", fecha(texto(e, "response_at")), texto(e, "response")));
            }
        });
    }
}

pub fn mostrar(ui: &mut egui::Ui, fila: &FilaTrabajo, config: &Config, corte: chrono::NaiveDate) {
    let Some(display) = &fila.cronologia else {
        return;
    };
    let estado = match texto(display, "status") {
        "ACREDITADA" => "observaciones del centro capturadas",
        "NO_SOLICITADA" => "no solicitada",
        "NO_LEIDA" => "no leída; retoma el lote",
        "ORIGEN_DESCONOCIDO" => "origen del centro no acreditado",
        _ => "sin entradas capturadas en el período",
    };
    egui::CollapsingHeader::new(format!("Bitácora: {estado}")).default_open(true).show(ui, |ui| {
        ui.label(format!(
            "Período {} a {}. Solo muestra lo capturado; no acredita historial completo.",
            fecha(texto(display, "period_from")),
            fecha(texto(display, "period_to"))
        ));
        entradas(ui, display, "last_center", "Última observación atribuible al centro");
        entradas(ui, display, "later", "Entradas posteriores");
        entradas(ui, display, "uncertain", "Mismo día, orden incierto");
        entradas(ui, display, "unattributed", "Entradas sin origen acreditado del centro");
        for (regla, aviso) in cronologia::avisos(display, &fila.observacion, &fila.propuesta.reglas, config, corte) {
            ui.colored_label(egui::Color32::from_rgb(170, 110, 0), format!("{regla}: {aviso}"));
        }
        for (k, titulo) in
            [("outside_horizon", "anteriores al período"), ("after_cutoff", "posteriores al corte"), ("undated", "sin fecha")]
        {
            if let Some(n) = display[k].as_u64().filter(|n| *n > 0) {
                ui.label(format!("{n} entrada(s) {titulo}; no se consideran en la cronología."));
            }
        }
    });
}
