//! Flow (Rust): Asistente CSMP y Anonimizador.
//! No escribe en RUS ni en SATURNO; no envía correos: solo deja borradores con CC fijo.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod anon;
mod config;
mod demo;
mod lectura;
mod motor;
mod productos;
mod theme;
mod trabajo;
mod ui;
mod util;

use std::path::PathBuf;

struct FlowApp {
    app: ui::App,
}

impl eframe::App for FlowApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.app.frame(ctx);
        // Modo captura: dibujar ~120 cuadros, pedir la imagen, guardarla y salir.
        let mut salir = false;
        if let Some(s) = &mut self.app.screenshot {
            s.frames += 1;
            ctx.request_repaint();
            if s.frames >= 120 && !s.pedido {
                s.pedido = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            let imagen = ctx.input(|i| {
                i.raw.events.iter().find_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let Some(img) = imagen {
                let [w, h] = img.size;
                let mut buf = image::RgbaImage::new(w as u32, h as u32);
                for (i, px) in img.pixels.iter().enumerate() {
                    let (x, y) = ((i % w) as u32, (i / w) as u32);
                    buf.put_pixel(x, y, image::Rgba(px.to_srgba_unmultiplied()));
                }
                if let Some(dir) = s.path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                if let Err(e) = buf.save(&s.path) {
                    eprintln!("No se pudo guardar la captura: {e}");
                }
                salir = true;
            }
            if s.frames > 900 {
                eprintln!("La captura no llegó a tiempo.");
                salir = true;
            }
        }
        if salir {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let screenshot = args.iter().position(|a| a == "--screenshot").and_then(|i| args.get(i + 1)).map(PathBuf::from);
    let vista = args.iter().position(|a| a == "--vista").and_then(|i| args.get(i + 1)).cloned();
    let abrir = args
        .iter()
        .enumerate()
        .skip(1)
        .find(|(i, a)| !a.starts_with("--") && !matches!(args.get(i - 1).map(|s| s.as_str()), Some("--screenshot") | Some("--vista")))
        .map(|(_, a)| PathBuf::from(a));

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Flow — Asistente CSMP")
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([1100.0, 680.0]);
    if screenshot.is_some() {
        viewport = viewport.with_active(false);
        #[cfg(target_os = "windows")]
        {
            viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
    }
    let mut options = eframe::NativeOptions { viewport, ..Default::default() };
    if screenshot.is_some() {
        options.persist_window = false;
        #[cfg(target_os = "macos")]
        {
            options.event_loop_builder = Some(Box::new(|b| {
                use winit::platform::macos::EventLoopBuilderExtMacOS;
                b.with_activate_ignoring_other_apps(false);
            }));
        }
    }
    eframe::run_native(
        "Flow",
        options,
        Box::new(move |_cc| {
            let mut app = ui::App::new(screenshot);
            if let Some(p) = abrir {
                app.abrir_externo(p);
            }
            if let Some(v) = &vista {
                for parte in v.split(',') {
                    app.vista_inicial(parte);
                }
            }
            Ok(Box::new(FlowApp { app }))
        }),
    )
}
