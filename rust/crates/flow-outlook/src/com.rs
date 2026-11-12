//! Automatización COM de Outlook por nombre (IDispatch). Solo crea el elemento, lo llena y lo guarda.

use crate::Resultado;
use flow_productos::correos::{direcciones, Borrador};
use windows::core::{Interface, GUID, HSTRING, PCWSTR};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::System::Com::{
    CLSIDFromProgID, CoCreateInstance, CoInitializeEx, CoUninitialize, IDispatch, CLSCTX_LOCAL_SERVER,
    COINIT_APARTMENTTHREADED, DISPATCH_FLAGS, DISPATCH_METHOD, DISPATCH_PROPERTYGET, DISPATCH_PROPERTYPUT, DISPPARAMS,
    EXCEPINFO,
};
use windows::Win32::System::Ole::DISPID_PROPERTYPUT;

const LOCALE: u32 = 0x0400;
/// olMailItem.
const ELEMENTO_CORREO: i32 = 0;

fn error(etapa: &str, e: windows::core::Error) -> String {
    format!("Outlook no pudo {etapa}: {}", e.message())
}

unsafe fn invocar(objeto: &IDispatch, nombre: &str, modo: DISPATCH_FLAGS, mut args: Vec<VARIANT>) -> windows::core::Result<VARIANT> {
    let ancho = HSTRING::from(nombre);
    let nombres = [PCWSTR(ancho.as_ptr())];
    let mut id = 0i32;
    objeto.GetIDsOfNames(&GUID::zeroed(), nombres.as_ptr(), 1, LOCALE, &mut id)?;
    args.reverse();
    let mut nombrado = DISPID_PROPERTYPUT;
    let es_asignacion = modo == DISPATCH_PROPERTYPUT;
    let parametros = DISPPARAMS {
        rgvarg: args.as_mut_ptr(),
        cArgs: args.len() as u32,
        rgdispidNamedArgs: if es_asignacion { &mut nombrado } else { std::ptr::null_mut() },
        cNamedArgs: u32::from(es_asignacion),
    };
    let mut resultado = VARIANT::default();
    let mut excepcion = EXCEPINFO::default();
    objeto.Invoke(id, &GUID::zeroed(), LOCALE, modo, &parametros, Some(&mut resultado), Some(&mut excepcion), None)?;
    Ok(resultado)
}

unsafe fn asignar(objeto: &IDispatch, propiedad: &str, valor: &str) -> windows::core::Result<()> {
    invocar(objeto, propiedad, DISPATCH_PROPERTYPUT, vec![VARIANT::from(valor)]).map(|_| ())
}

unsafe fn objeto(valor: &VARIANT) -> windows::core::Result<IDispatch> {
    IDispatch::try_from(valor)
}

unsafe fn aplicacion() -> windows::core::Result<IDispatch> {
    let clase = CLSIDFromProgID(&HSTRING::from("Outlook.Application"))?;
    CoCreateInstance(&clase, None, CLSCTX_LOCAL_SERVER)
}

struct Com;

impl Com {
    fn iniciar() -> Com {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
        Com
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        unsafe { CoUninitialize() }
    }
}

pub fn disponible() -> bool {
    let _com = Com::iniciar();
    unsafe { CLSIDFromProgID(&HSTRING::from("Outlook.Application")).is_ok() }
}

pub fn guardar(borrador: &Borrador) -> Result<Resultado, String> {
    let _com = Com::iniciar();
    unsafe {
        let outlook = aplicacion().map_err(|e| error("abrirse", e))?;
        let creado = invocar(&outlook, "CreateItem", DISPATCH_METHOD, vec![VARIANT::from(ELEMENTO_CORREO)])
            .map_err(|e| error("crear el borrador", e))?;
        let correo = objeto(&creado).map_err(|e| error("crear el borrador", e))?;
        asignar(&correo, "To", &direcciones(&borrador.para).join("; ")).map_err(|e| error("poner el destinatario", e))?;
        asignar(&correo, "CC", &direcciones(&borrador.cc).join("; ")).map_err(|e| error("poner el CC", e))?;
        asignar(&correo, "Subject", &borrador.asunto).map_err(|e| error("poner el asunto", e))?;
        asignar(&correo, "Body", &borrador.cuerpo.replace('\n', "\r\n")).map_err(|e| error("poner el texto", e))?;
        if !borrador.adjuntos.is_empty() {
            let lista = invocar(&correo, "Attachments", DISPATCH_PROPERTYGET, Vec::new())
                .and_then(|v| objeto(&v))
                .map_err(|e| error("abrir los adjuntos", e))?;
            for ruta in &borrador.adjuntos {
                let texto = ruta.to_string_lossy();
                invocar(&lista, "Add", DISPATCH_METHOD, vec![VARIANT::from(texto.as_ref())])
                    .map_err(|e| error("adjuntar la nómina", e))?;
            }
        }
        // Único efecto externo permitido: guardar el borrador.
        match invocar(&correo, "Save", DISPATCH_METHOD, Vec::new()) {
            Ok(_) => Ok(Resultado::Guardado),
            Err(e) => Ok(Resultado::Incierto(error("confirmar el guardado", e))),
        }
    }
}

#[allow(dead_code)]
fn _interfaz_usada() -> GUID {
    IDispatch::IID
}
