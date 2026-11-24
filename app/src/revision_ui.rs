use crate::copia_excel::{self, FilaBitacora, COLUMNAS_BITACORA};
use crate::theme::Pal;
use crate::trabajo::Trabajo;
use crate::util::{parse_date_text, Cell};
use chrono::{Months, NaiveDate};
use egui::{Context, RichText};
use egui_extras::{Column, TableBuilder};
use flow_rust::captura::revision::{Clase, Resultado, Revision};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const CLASES: [Clase; 3] = [
    Clase::ConObservacion,
    Clase::SinObservacion,
    Clase::NoVerificable,
];
pub enum Accion {
    AbrirFila(usize),
    SeleccionarEnTrabajo(Vec<usize>),
}

pub struct Dialogo {
    desde: String,
    hasta: String,
    periodo: (NaiveDate, NaiveDate),
    error: Option<String>,
    filtro: Option<Clase>,
    filas: Vec<(usize, Resultado)>,
    seleccion: HashSet<usize>,
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
            seleccion: HashSet::new(),
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
        self.seleccion.clear();
        self.error = None;
        self.salida = None;
    }

    pub fn mostrar(
        &mut self,
        ctx: &Context,
        p: &Pal,
        w: &Trabajo,
    ) -> (bool, Option<Accion>, Option<[PathBuf; 2]>) {
        let mut abierto = true;
        let mut accion = None;
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
                let filtro_anterior = self.filtro;
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
                if self.filtro != filtro_anterior {
                    self.seleccion.clear();
                }
                let visibles: Vec<_> = self
                    .filas
                    .iter()
                    .filter(|(_, r)| self.filtro.is_none_or(|clase| clase == r.clase))
                    .collect();
                let elegidas = subconjunto(&visibles, &self.seleccion);
                let habilitado = self.error.is_none() && !elegidas.is_empty();
                let exportar_solicitado = ui
                    .horizontal_wrapped(|ui| {
                        if ui
                            .add_enabled(
                                habilitado,
                                egui::Button::new(format!(
                                    "Seleccionar en Trabajo ({})",
                                    elegidas.len()
                                )),
                            )
                            .on_hover_text("Reemplaza las marcas de Trabajo con las filas elegidas")
                            .clicked()
                        {
                            accion = Some(Accion::SeleccionarEnTrabajo(
                                elegidas.iter().map(|(i, _)| *i).collect(),
                            ));
                        }
                        ui.add_enabled(habilitado, egui::Button::new("Exportar Excel y JSON…"))
                            .clicked()
                    })
                    .inner;
                if exportar_solicitado {
                    if let Some(carpeta) = rfd::FileDialog::new()
                        .set_title("Carpeta donde crear la revisión")
                        .pick_folder()
                    {
                        let nombre = chrono::Local::now()
                            .format("Revision_bitacoras_%Y-%m-%d_%H%M%S_%3f")
                            .to_string();
                        let destino = carpeta.join(nombre);
                        match exportar(&destino, w, &elegidas, self.periodo) {
                            Ok((rutas, aviso)) => {
                                productos = Some(rutas);
                                let mensaje = format!("Revisión guardada en {}", destino.display());
                                self.salida = Some(match aviso {
                                    Some(aviso) => format!("{mensaje} · {aviso}"),
                                    None => mensaje,
                                });
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
                    .column(Column::exact(37.0))
                    .column(Column::initial(126.0))
                    .column(Column::initial(105.0))
                    .column(Column::remainder().at_least(185.0))
                    .column(Column::initial(82.0))
                    .header(23.0, |mut encabezado| {
                        for titulo in ["", "Clasificación", "RIT", "Motivo", ""] {
                            encabezado.col(|ui| {
                                ui.strong(titulo);
                            });
                        }
                    })
                    .body(|cuerpo| {
                        cuerpo.rows(42.0, visibles.len(), |mut fila| {
                            let (i, resultado) = visibles[fila.index()];
                            fila.col(|ui| {
                                let mut marcada = self.seleccion.contains(i);
                                if ui
                                    .checkbox(&mut marcada, "")
                                    .on_hover_text("Seleccionar esta fila")
                                    .changed()
                                {
                                    if marcada {
                                        self.seleccion.insert(*i);
                                    } else {
                                        self.seleccion.remove(i);
                                    }
                                }
                            });
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
                                    accion = Some(Accion::AbrirFila(*i));
                                }
                            });
                        })
                    });
            });
        (abierto, accion, productos)
    }
}

fn subconjunto<'a>(
    visibles: &[&'a (usize, Resultado)],
    seleccion: &HashSet<usize>,
) -> Vec<&'a (usize, Resultado)> {
    if seleccion.is_empty() {
        return visibles.to_vec();
    }
    visibles
        .iter()
        .copied()
        .filter(|(i, _)| seleccion.contains(i))
        .collect()
}

fn exportar(
    directorio: &Path,
    w: &Trabajo,
    filas: &[&(usize, Resultado)],
    periodo: (NaiveDate, NaiveDate),
) -> Result<([PathBuf; 2], Option<String>), String> {
    let columna = w
        .headers
        .iter()
        .position(|h| h == "ID_REGISTRO_CAPTURA")
        .ok_or("El trabajo carece de identificadores de captura.")?;
    let mut lecturas = Vec::with_capacity(filas.len());
    let mut registros = Vec::with_capacity(filas.len());
    let mut totales =
        std::collections::BTreeMap::from(CLASES.map(|clase| (clase.etiqueta(), 0usize)));
    for &(indice, ref resultado) in filas.iter().copied() {
        let r = w
            .registros
            .get(indice)
            .ok_or("La selección contiene una fila que ya no está en Trabajo.")?;
        let etiqueta = resultado.clase.etiqueta();
        *totales.get_mut(etiqueta).unwrap() += 1;
        lecturas.push(FilaBitacora {
            indice,
            clasificacion: etiqueta,
            motivo: &resultado.motivo,
        });
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
    let resultado = (|| {
        let aviso = match copia_excel::base(w) {
            Ok(base) => {
                copia_excel::exportar_bitacoras(w, &base, &excel, &lecturas)?;
                None
            }
            Err(aviso) => {
                let (mut encabezados, mut datos) = crate::productos::tabla_observaciones(
                    w,
                    lecturas.iter().map(|r| &w.registros[r.indice]),
                );
                encabezados.extend(COLUMNAS_BITACORA.map(str::to_owned));
                for (fila, lectura) in datos.iter_mut().zip(&lecturas) {
                    fila.extend([lectura.clasificacion.into(), lectura.motivo.into()]);
                }
                crate::productos::escribir_xlsx(&[(w.mode().label(), encabezados, datos)], &excel)?;
                Some(aviso)
            }
        };
        std::fs::write(&informe, json).map_err(|err| err.to_string())?;
        Ok::<_, String>(aviso)
    })();
    match resultado {
        Ok(aviso) => Ok(([excel, informe], aviso)),
        Err(error) => {
            let _ = std::fs::remove_file(&excel);
            let _ = std::fs::remove_file(&informe);
            let _ = std::fs::remove_dir(directorio);
            Err(format!("No se pudo exportar la revisión: {error}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn usa_el_mismo_subconjunto_para_exportar_y_transferir() {
        let resultado = |clase| Resultado {
            clase,
            motivo: "Motivo ficticio".into(),
        };
        let filas = [
            (0, resultado(Clase::SinObservacion)),
            (1, resultado(Clase::NoVerificable)),
            (2, resultado(Clase::SinObservacion)),
        ];
        let visibles = [&filas[0], &filas[2]];
        let indices = |s: &HashSet<usize>| {
            subconjunto(&visibles, s)
                .iter()
                .map(|(i, _)| *i)
                .collect::<Vec<_>>()
        };
        assert_eq!(indices(&HashSet::new()), [0, 2]);
        assert_eq!(indices(&HashSet::from([1, 2])), [2]);
        assert!(indices(&HashSet::from([1])).is_empty());
    }

    #[test]
    #[ignore = "requiere FLOW_P2_PAQUETE"]
    fn exporta_mismo_lote_de_python() {
        let paquete = crate::prueba_entorno::requerida("FLOW_P2_PAQUETE");
        let destino =
            PathBuf::from(std::env::var_os("FLOW_P2_SALIDA").expect("carpeta del contraste P2"));
        let corte = NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();
        let w = Trabajo::abrir_archivo(PathBuf::from(paquete), None, corte, &Config::default())
            .unwrap();
        let mut dialogo = Dialogo::nuevo(&w, corte);
        dialogo.desde = "08/09/2026".into();
        dialogo.hasta = "08/10/2026".into();
        dialogo.calcular(&w);
        assert!(dialogo.error.is_none());
        assert_eq!(dialogo.filas.len(), 2);
        let filas: Vec<_> = dialogo.filas.iter().collect();
        exportar(&destino, &w, &filas, dialogo.periodo).unwrap();
        crate::productos::excel_observaciones(&w, destino.parent().unwrap()).unwrap();
    }

    #[test]
    fn exporta_resultados_y_preserva_el_destino_existente() {
        let raiz = crate::prueba_entorno::requerida("FLOW_REFERENCIA");
        let corte = NaiveDate::from_ymd_opt(2026, 11, 20).unwrap();
        let mut w = Trabajo::abrir_archivo(
            PathBuf::from(raiz).join("contracts/csmp_capture_v1/fixtures/completa.zip"),
            None,
            corte,
            &Config::default(),
        )
        .unwrap();
        let dialogo = Dialogo::nuevo(&w, corte);
        let fila = dialogo.filas.last().expect("fila de revisión");
        let registro = &mut w.registros[fila.0];
        registro.editada = Some("Observación ficticia editada por la persona revisora.".into());
        registro.revisada = true;
        registro.res = "PC_IE".into();
        let filas = [fila];
        let carpeta = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test_out");
        std::fs::create_dir_all(&carpeta).unwrap();
        let destino = carpeta.join(format!(
            "revision_{}",
            chrono::Utc::now().timestamp_millis()
        ));
        let ([xlsx, informe], aviso) = exportar(&destino, &w, &filas, dialogo.periodo).unwrap();
        assert!(aviso.is_some());
        let hojas = crate::lectura::read_sheets(&std::fs::read(&xlsx).unwrap()).unwrap();
        let general = crate::productos::excel_observaciones(&w, &destino).unwrap();
        let general = crate::lectura::read_sheets(&std::fs::read(general).unwrap()).unwrap();
        assert_eq!(hojas[0].rows.len(), filas.len() + 1);
        assert_eq!(hojas[0].name, general[0].name);
        let comunes = general[0].rows[0].len();
        assert_eq!(&hojas[0].rows[0][..comunes], general[0].rows[0].as_slice());
        assert_eq!(
            &hojas[0].rows[1][..comunes],
            general[0].rows[fila.0 + 1].as_slice()
        );
        assert_eq!(
            crate::lectura::headers_of(&hojas[0].rows[0][comunes..]),
            COLUMNAS_BITACORA
        );
        assert_eq!(hojas[0].rows[1][comunes].display(), "No verificable");
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
