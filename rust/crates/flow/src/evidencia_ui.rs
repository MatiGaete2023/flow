//! Capturas de la interfaz real con datos ficticios y almacenamiento aislado.
use eframe::egui;
use std::{path::PathBuf,io::Write};

pub struct Config {pub pantalla:String,pub salida:PathBuf,pub ancho:f32,pub alto:f32,pub escala:f32}
impl Config {
    pub fn argumentos(args:&[String])->Result<Self,String>{
        if args.len()!=6{return Err("Uso: --demo-ui PANTALLA CARPETA ANCHO ALTO ESCALA".into())}
        let pantallas=["trabajo","correos","resoluciones","resultados","contactos","captura","anonimizador","meta4","revision","ayuda","recalculo"];
        if !pantallas.contains(&args[1].as_str()){return Err("Pantalla de demostración desconocida.".into())}
        let numero=|i:usize|args[i].parse::<f32>().map_err(|_|"Tamaño o escala inválidos.".to_string());
        let (ancho,alto,escala)=(numero(3)?,numero(4)?,numero(5)?);
        if !ancho.is_finite()||!alto.is_finite()||!escala.is_finite()||!(320.0..=4096.0).contains(&ancho)||!(240.0..=2160.0).contains(&alto)||!(0.5..=3.0).contains(&escala){return Err("Tamaño o escala fuera de rango.".into())}
        let salida=PathBuf::from(&args[2]);
        if !salida.is_dir(){return Err("La carpeta de capturas debe existir.".into())}
        Ok(Self{pantalla:args[1].clone(),salida,ancho,alto,escala})
    }
}
pub struct Captura {
    pub config:Config,
    pub temporal:tempfile::TempDir,
    cuadros:u32,pedida:bool,
}
impl Captura {
    pub fn nueva(config:Config)->Result<Self,String>{
        Ok(Self{config,temporal:tempfile::tempdir().map_err(|e|e.to_string())?,cuadros:0,pedida:false})
    }
    pub fn actualizar(&mut self,ctx:&egui::Context){
        self.cuadros+=1;
        let imagen=ctx.input(|i|i.events.iter().find_map(|e|match e{egui::Event::Screenshot{image,..}=>Some(image.clone()),_=>None}));
        if let Some(imagen)=imagen {
            let nombre=format!("{}-{}x{}-{}",self.config.pantalla,self.config.ancho as u32,self.config.alto as u32,(self.config.escala*100.0)as u32);
            let guardar=(||->Result<(),String>{
                let ruta=self.config.salida.join(format!("{nombre}.ppm"));
                let mut f=std::fs::OpenOptions::new().create_new(true).write(true).open(&ruta).map_err(|e|e.to_string())?;
                write!(f,"P6\n{} {}\n255\n",imagen.width(),imagen.height()).map_err(|e|e.to_string())?;
                for p in &imagen.pixels {let [r,g,b,_]=p.to_array();f.write_all(&[r,g,b]).map_err(|e|e.to_string())?;}
                let rect=ctx.input(|i|i.screen_rect);
                let metadatos=serde_json::json!({"pantalla":self.config.pantalla,"ancho_solicitado":self.config.ancho,"alto_solicitado":self.config.alto,
                    "escala":self.config.escala,"pixeles":[imagen.width(),imagen.height()],"tamano_efectivo":[rect.width(),rect.height()],
                    "sintetico":true,"captura_real_de_egui":true});
                std::fs::write(self.config.salida.join(format!("{nombre}.json")),serde_json::to_vec_pretty(&metadatos).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
                Ok(())
            })();
            if let Err(e)=guardar {eprintln!("Fallo de captura sintética: {e}");}
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }else if self.cuadros>=12 && !self.pedida {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));self.pedida=true;
        }else if self.cuadros>300 {
            eprintln!("No se recibió la captura de la interfaz.");ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(30));
    }
}

