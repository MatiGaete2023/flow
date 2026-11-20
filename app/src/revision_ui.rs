use crate::theme::Pal;
use crate::trabajo::Trabajo;
use crate::util::{parse_date_text, Cell};
use chrono::{Months, NaiveDate};
use egui::{Context, RichText};
use egui_extras::{Column, TableBuilder};
use flow_rust::captura::revision::{Clase, Resultado, Revision};
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const CLASES: [Clase; 3] = [
    Clase::ConObservacion,
    Clase::SinObservacion,
    Clase::NoVerificable,
];

pub struct Dialogo {
    desde: String,
    hasta: String,
    periodo: (NaiveDate, NaiveDate),
    error: Option<String>,
    filtro: Option<Clase>,
    filas: Vec<(usize, Resultado)>,
    salida: Option<String>,
}

impl Dialogo {
    pub fn nuevo(w: &Trabajo, corte: NaiveDate) -> Self {
        let desde = corte.checked_sub_months(Months::new(1)).unwrap_or(corte);
        let mut dialogo = Self {
            desde: desde.format("%d/%m/%Y").to_string(),
            hasta: corte.format("%d/%m/%Y").to_string(),
            periodo: (desde, corte),
            error: None,
            filtro: None,
            filas: Vec::new(),
            salida: None,
        };
        dialogo.calcular(w);
        dialogo
    }

    fn calcular(&mut self, w: &Trabajo) {
        let Some((desde, hasta)) = parse_date_text(&self.desde).zip(parse_date_text(&self.hasta))
        else {
            self.error = Some("Usa fechas válidas en formato dd/mm/aaaa.".into());
            return;
        };
        if desde > hasta {
            self.error = Some("La fecha Desde es posterior a Hasta.".into());
            return;
        }
        let Some(paquete) = &w.paquete else {
            return;
        };
        let Some(columna) = w.headers.iter().position(|h| h == "ID_REGISTRO_CAPTURA") else {
            return;
        };
        let registros: HashMap<_, _> = paquete
            .fuente
            .filas("registros")
            .map(|r| (r.get("record_id"), r))
            .collect();
        let revision = Revision::nueva(&paquete.fuente);
        self.filas = w
            .registros
            .iter()
            .enumerate()
            .filter(|(_, r)| !r.excluido)
            .filter_map(|(i, r)| {
                let Cell::Text(id) = r.values.get(columna)? else {
                    return None;
                };
                let resultado = revision
                    .clasificar(*registros.get(id.as_str())?, desde, hasta)
                    .ok()?;
                Some((i, resultado))
            })
            .collect();
        self.periodo = (desde, hasta);
        self.error = None;
        self.salida = None;
    }

    pub fn mostrar(
        &mut self,
        ctx: &Context,
        p: &Pal,
        w: &Trabajo,
    ) -> (bool, Option<usize>, Option<[PathBuf; 2]>) {
        let mut abierto = true;
        let mut seleccionada = None;
        let mut productos = None;
        egui::Window::new("Revisión de bitácoras")
            .open(&mut abierto)
            .default_width(755.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Desde");
                    ui.add(egui::TextEdit::singleline(&mut self.desde).desired_width(86.0));
                    ui.label("Hasta");
                    ui.add(egui::TextEdit::singleline(&mut self.hasta).desired_width(86.0));
                    if crate::theme::primary(ui, p, "Revisar período").clicked() {
                        self.calcular(w);
                    }
                });
                if let Some(e) = &self.error {
                    ui.label(RichText::new(e).color(p.amber));
                }
                let (desde, hasta) = self.periodo;
                ui.label(
                    RichText::new(format!(
                        "{} · {} a {}",
                        w.mode().label(),
                        desde.format("%d/%m/%Y"),
                        hasta.format("%d/%m/%Y"),
                    ))
                    .color(p.muted),
                );
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(
                        &mut self.filtro,
                        None,
                        format!("Todos ({})", self.filas.len()),
                    );
                    for clase in CLASES {
                        let n = self.filas.iter().filter(|(_, r)| r.clase == clase).count();
                        ui.selectable_value(
                            &mut self.filtro,
                            Some(clase),
                            format!("{} ({n})", clase.etiqueta()),
                        );
                    }
                });
                let visibles: Vec<_> = self
                    .filas
                    .iter()
                    .filter(|(_, r)| self.filtro.is_none_or(|clase| clase == r.clase))
                    .collect();
                if ui
                    .add_enabled(
                        self.error.is_none(),
                        egui::Button::new("Exportar Excel y JSON…"),
                    )
                    .clicked()
                {
                    if let Some(carpeta) = rfd::FileDialog::new()
                        .set_title("Carpeta donde crear la revisión")
                        .pick_folder()
                    {
                        let nombre = chrono::Local::now()
                            .format("Revision_bitacoras_%Y-%m-%d_%H%M%S_%3f")
                            .to_string();
                        let destino = carpeta.join(nombre);
                        match exportar(&destino, w, &visibles, self.periodo) {
                            Ok(rutas) => {
                                productos = Some(rutas);
                                self.salida =
                                    Some(format!("Revisión guardada en {}", destino.display()));
                            }
                            Err(error) => self.salida = Some(error),
                        }
                    }
                }
                if let Some(salida) = &self.salida {
                    ui.label(RichText::new(salida).color(p.muted));
                }
                TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .max_scroll_height(410.0)
                    .column(Column::initial(126.0))
                    .column(Column::initial(105.0))
                    .column(Column::remainder().at_least(185.0))
                    .column(Column::initial(82.0))
                    .header(23.0, |mut encabezado| {
                        for titulo in ["Clasificación", "RIT", "Motivo", ""] {
                            encabezado.col(|ui| {
                                ui.strong(titulo);
                            });
                        }
                    })
                    .body(|cuerpo| {
                        cuerpo.rows(42.0, visibles.len(), |mut fila| {
                            let (i, resultado) = visibles[fila.index()];
                            fila.col(|ui| {
                                ui.label(resultado.clase.etiqueta());
                            });
                            fila.col(|ui| {
                                ui.label(w.value(&w.registros[*i], "rit"));
                            });
                            fila.col(|ui| {
                                ui.label(&resultado.motivo);
                            });
                            fila.col(|ui| {
                                if ui.button("Abrir fila").clicked() {
                                    seleccionada = Some(*i);
                                }
                            });
                        })
                    });
            });
        (abierto, seleccionada, productos)
    }
}

fn exportar(
    directorio: &Path,
    w: &Trabajo,
    filas: &[&(usize, Resultado)],
    periodo: (NaiveDate, NaiveDate),
) -> Result<[PathBuf; 2], String> {
    let titulo = "Revisión de bitácoras";
    let encabezados = [
        "Clasificación",
        "Motivo",
        "Tribunal",
        "RIT",
        "Nombre",
        "Programa",
        "Observación propuesta",
    ]
    .map(str::to_owned)
    .to_vec();
    let columna = w
        .headers
        .iter()
        .position(|h| h == "ID_REGISTRO_CAPTURA")
        .ok_or("El trabajo carece de identificadores de captura.")?;
    let mut datos = Vec::with_capacity(filas.len());
    let mut registros = Vec::with_capacity(filas.len());
    let mut totales =
        std::collections::BTreeMap::from(CLASES.map(|clase| (clase.etiqueta(), 0usize)));
    for (indice, resultado) in filas.iter().copied() {
        let r = &w.registros[*indice];
        let etiqueta = resultado.clase.etiqueta();
        *totales.get_mut(etiqueta).unwrap() += 1;
        datos.push(vec![
            etiqueta.into(),
            resultado.motivo.clone(),
            w.value(r, "tribunal"),
            w.value(r, "rit"),
            w.value(r, "nombre"),
            w.value(r, "programa"),
            r.observacion().into(),
        ]);
        registros.push(json!({
            "id": r.values[columna].display(), "clasificacion": resultado.clase, "motivo": resultado.motivo,
        }));
    }
    let (desde, hasta) = periodo;
    let json = serde_json::to_vec_pretty(&json!({
        "tipo": "REVISION_BITACORAS", "version": 1, "periodo": {"desde": desde, "hasta": hasta},
        "corte": w.corte, "resumen": totales, "registros": registros,
    }))
    .map_err(|e| e.to_string())?;
    let excel = directorio.join("Revision_bitacoras.xlsx");
    let informe = directorio.join("Revision_bitacoras.json");
    std::fs::create_dir(directorio)
        .map_err(|e| format!("No se pudo crear la carpeta de la revisión: {e}"))?;
    let resultado = crate::productos::escribir_xlsx(&[(titulo, encabezados, datos)], &excel)
        .and_then(|_| std::fs::write(&informe, json).map_err(|e| e.to_string()));
    if let Err(error) = resultado {
        let _ = std::fs::remove_file(&excel);
        let _ = std::fs::remove_file(&informe);
        let _ = std::fs::remove_dir(directorio);
        return Err(format!("No se pudo exportar la revisión: {error}"));
    }
    Ok([excel, informe])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn exporta_resultados_y_preserva_el_destino_existente() {
        let Some(raiz) = std::env::var_os("FLOW_REFERENCIA") else {
            return;
        };
        let corte = NaiveDate::from_ymd_opt(2026, 11, 20).unwrap();
        let w = Trabajo::abrir_archivo(
            PathBuf::from(raiz).join("contracts/csmp_capture_v1/fixtures/completa.zip"),
            None,
            corte,
            &Config::default(),
        )
        .unwrap();
        let dialogo = Dialogo::nuevo(&w, corte);
        let filas: Vec<_> = dialogo.filas.iter().collect();
        assert!(!filas.is_empty());
        let carpeta = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test_out");
        std::fs::create_dir_all(&carpeta).unwrap();
        let destino = carpeta.join(format!(
            "revision_{}",
            chrono::Utc::now().timestamp_millis()
        ));
        let [xlsx, informe] = exportar(&destino, &w, &filas, dialogo.periodo).unwrap();
        let hojas = crate::lectura::read_sheets(&std::fs::read(&xlsx).unwrap()).unwrap();
        assert_eq!(hojas[0].rows.len(), filas.len() + 1);
        assert_eq!(hojas[0].rows[0][0].display(), "Clasificación");
        let bytes = std::fs::read(&informe).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            json["resumen"]["No verificable"].as_u64(),
            Some(filas.len() as u64)
        );
        assert!(exportar(&destino, &w, &filas, dialogo.periodo).is_err());
        assert_eq!(std::fs::read(&informe).unwrap(), bytes);
    }
}
