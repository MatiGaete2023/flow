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

## 4. Ventana «Información de la Causa» (N1, copia guardada de la ventana, Cumplimiento)

El usuario indica que la ventana es idéntica en Espera y Cumplimiento (decisión H-g); la segunda muestra de Espera
confirmó el mismo flujo de acceso. Muestra: una causa con 269 trámites.

**Documento:** `charset=windows-1252` (meta de la copia; el listado declara `iso-8859-1`; ambos se decodifican con
`decode_snapshot`). Cierra `</html>`. Formulario `AtPublicoPpalForm` → `/SITFAWEB/AtPublicoDAction.do`.

**Encabezado de causa:** `div.panel-heading > strong` («Información de la Causa» y, si aplica, «[Causa Reservada]»).
Datos en `div.panel-body > div.row > div.col-xs-4|col-xs-8`, cada uno `<label class="form-check-label">ETIQUETA:</label> VALOR`
(valor = texto que sigue a la etiqueta). Etiquetas: RIT, RUC, Caratulado, F. Ingreso, Proc., Forma Inicio, Est. Adm.,
Etapa, Estado Proc., Tribunal, Texto Requerimiento (este último con imagen de documento: se ignora).

**Pestañas:** `Lengueta('…')` ×4 (Historia, Notificación, Diligencias, Escritos). En la copia solo existe la tabla de
Historia: las otras pestañas no vienen en el HTML (fuera de alcance). Botones Litigantes/Materias/Plazos/Remisión:
`ShowPopUpCabecera(n, n)` (fuera de alcance).

**Tabla de Historia:** la **única** `<table>` de la página; `class="table table-sm table-bordered table-hover"`,
sin `id`. Ubicarla por su encabezado exacto, no por clases. `<thead>` con 9 `<th>`:
`Folio · Doc. · Sel · Anexo · Etapa · Trámite · Desc. Trámite · Referencia · Fecha Tram`. `<tbody>` con una `<tr>` por
trámite y un `<script>` entre filas (`historia[i] = new InformeHistoria(...)`): ignorar los scripts.

| Columna | Forma observada | Regla del lector |
|---|---|---|
| Folio | entero de 4–6 dígitos | texto literal |
| Doc. | `<img onclick="ShowPDF('…','…')">` o `Reservado('…')` | **ignorar** (sin PDF) |
| Sel | `<img>`; a veces `ShowPopUpLengueta(n×6)` | **ignorar** |
| Anexo | comentario HTML; a veces `<img onclick="ShowAnexos(n,n)">` | `annex_visible = true` si hay `img`; nunca abrir |
| Etapa | «Cumplimiento» (todas en la muestra) | texto literal |
| Trámite | Resolución, Escrito, Actuación | texto literal |
| Desc. Trámite | texto corto (≤ 60) | texto íntegro |
| Referencia | texto ≤ 60; «Sin referencia.» frecuente | texto íntegro, literal |
| Fecha Tram | `dd/mm/aaaa`, sin hora | ISO, `time_precision=DIA` |

**Orden y completitud:** todas las filas en una sola respuesta (269; **sin paginación**), orden por fecha
**descendente** sin violaciones. En la muestra: 31 trámites dentro de 4 meses y 237 anteriores.

**Fila final incompleta (N1, real):** la última `<tr>` trae solo 4 celdas (Folio, Doc., Sel, Anexo), sin etapa,
trámite ni fecha, aunque el HTML cierra bien. Regla: no es error de lectura; registrar advertencia
`HISTORIA_FILA_INCOMPLETA` con su folio, no exportarla como trámite (sin fecha no entra a la ventana) y contarla en la
hoja «Causas». Cualquier otra fila con un número de celdas distinto de 9 ⇒ `HISTORIA_ENCABEZADO_DISTINTO` (error).

**Tamaño:** ~690 KB para 269 trámites ⇒ el tope de 8 MB por respuesta alcanza.

**Pendiente menor (no bloquea):** cómo se ve una causa sin trámites o con acceso denegado. Mientras no haya muestra:
tabla sin filas ⇒ `VACIA_COMPROBADA` solo si el encabezado exacto está presente; sin tabla ⇒ `HISTORIA_TABLA_NO_RECONOCIDA`.
