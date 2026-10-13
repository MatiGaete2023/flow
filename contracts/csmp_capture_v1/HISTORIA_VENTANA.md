# Ventana de historia de causa SITFA — contrato observado (H0)

Fuente: `estructura_historia.json` generado el 13 de octubre de 2026 con `descargador/extraer_estructura_historia.py`
sobre dos HAR del listado (Cumplimiento y Espera) del usuario, más los rasgos visibles de la ventana descritos por
el usuario. Sin valores de causas. **N1** = observado en la estructura; **N2** = inferencia; **PENDIENTE** = falta muestra.

## 1. Acceso — escenario B (N1)

Enlace en la columna **RIT** de las filas de `tablaCumplimiento` y `tablaEspera` (también presente en Espera):
`ShowHistoria(<tribunal:num>, '<RIT>', <idCausa:num>)` (aridad 3, tipos `num,str,num`, sin comillas en los números).

La función de la página (`InformesDAction.do`, script en línea) hace:

1. `rit.split('-')` → `tip_causa`, `rol_causa`, `era_causa`.
2. `$.ajax({type: 'POST', async: false, cache: false, dataType: 'json'})` a
   `/SITFAWEB/Encrypt_AjaxAccion.do?opcion=1&TIP_Causa=<tip>&ROL_Causa=<rol>&ERA_Causa=<era>&crr_idcausaUrl=<idCausa>&cod_tribunalUrl=<tribunal>`
   (parámetros en la URL; sin cuerpo). Respuesta 200, `content-type: text/html`, ~230 bytes, JSON con claves
   `tip_causa`, `rol_causa`, `era_causa`, `cod_tribunal`, `crr_causa` (valores cifrados). Error: `alert('Error al Encriptar')`.
3. `window.open('/SITFAWEB/ConsultaDetalleAtPublicoAccion.do?TipoPopup=3&a1=' + crr_causa + '&b2=' + tip_causa +
   '&c3=' + rol_causa + '&d4=' + era_causa + '&e5=' + cod_tribunal + '&B4=1&FLG_Encripta=1', 'plantillaVentana1', …)`.
   Los valores se concatenan **tal como llegan** (N2: el servidor los entrega ya codificados para URL; la captura
   visual muestra `a1` con `%2B`/`%3D`).

Consecuencias para `history_open` (WP-53 §53.3):
- Dos peticiones por causa, ambas `https://familia.pjud.cl`, rutas exactas anteriores; sin ejecutar JS de SITFA.
- Entradas solo desde el enlace validado del listado vigente: tribunal (`^\d{1,4}$`), RIT (`^[A-Z]{1,3}-\d{1,7}-\d{4}$`),
  idCausa (`^\d{1,20}$`).
- Paso 1: `fetch(urlEncrypt, {method:'POST', credentials:'include', cache:'no-store', redirect:'error'})`;
  cuerpo ≤ 4 KB; `JSON.parse`; exactamente esas 5 claves, cada valor string `^[A-Za-z0-9%+/=._-]{1,512}$`.
- Paso 2: URL armada con concatenación idéntica a la página; `GET`, `redirect:'error'`; tope 8 MB; `html`.
- Los valores cifrados **no salen de la extensión** (no van a Python, manifiestos ni diagnósticos).
- Cualquier diferencia ⇒ `HISTORY_LINK`/`HISTORY_RESPONSE`, nunca «sin trámites».

## 2. Fuera de alcance (N1, mismas filas)

`ShowHistoriaRut(rut)` y `ShowHistorialFichas(rut)` (columna Nombre) abren
`/SITFAWEB/jsp/Tramitacion/PopUp/PopUpHistorial.jsp?tipo_popUp=…&RUT_Litigante=…&RUT_DV=…`: historial **por RUT**, no la
historia de la causa. No se usan (prohibido consultar por RUT).

## 3. Listado (N1, para `extract_historia`)

- Tablas: `tablaEspera`, `tablaCumplimiento`, `tablaInforme`, `tablaEgresados`; encabezados con `RIT`, `Tribunal`,
  `RUT`/`RUT (->RCEI)`, `Nombre`, `Tipo`, `Derivación` (otras columnas no reconocidas por la lista del extractor).
- En Cumplimiento la primera columna no es RIT (columna adicional antes de RIT).
- `ShowObservaciones` aridad 6, tipos `num,num,str,num,num,str` (columna Obs.).
- Muestras pequeñas (3 filas Cumplimiento, 1 Espera): suficiente para el mecanismo, no para variantes.

## 4. Ventana «Información de la Causa» — PENDIENTE

No hay HAR de la ventana emergente. Rasgos visibles (descripción del usuario, N1 visual): pestañas Historia,
Notificación, Diligencias, Escritos; botones Litigantes, Materias, Plazos, Remisión; tabla con columnas
`Folio · Doc. · Sel · Anexo · Etapa · Trámite · Desc. Trámite · Referencia · Fecha Tram`; orden por fecha descendente.
Falta, con una captura de la ventana (guía H0, paso 2.3): id/clase de la tabla, si las pestañas son locales o piden
otra URL, paginación/«ver más», vacío explícito, charset y forma de las celdas.
