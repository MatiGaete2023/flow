# Especificación — lectura de la historia de la causa

Alcance: decisiones H-a…H-h de [00_DECISIONES](00_DECISIONES.md). Todo lo marcado **[H0]** se completa con la
estructura anónima real; hasta entonces es provisional y no se programa contra ello.

## 1. Qué hace y qué no

**Hace:** para cada causa con vínculo de historia verificado en el listado del lote, abre la ventana «Información
de la Causa» con la sesión del usuario, lee la tabla de la pestaña **Historia**, conserva el HTML íntegro en
`respaldo/historia/`, y entrega los trámites con fecha dentro de la ventana de 4 meses en un Excel y, si se pide,
en el paquete del Asistente.

**No hace:** abrir/descargar/leer PDF o anexos; leer las pestañas Notificación, Diligencias, Escritos ni los botones
Litigantes, Materias, Plazos, Remisión; consultar por RIT, RUT o nombre; reutilizar historias de otras sesiones;
cambiar el Asistente, sus reglas o productos; escribir en RUS/SATURNO; usar IA.

## 2. Fuente y campos

| Campo visible | Uso | Tratamiento |
|---|---|---|
| Folio | dato | Texto literal (conservar ceros/letras/separadores). Nunca derivarlo del número de fila. |
| Doc. | **ignorado** | No se sigue el enlace. No se registra. |
| Sel | **ignorado** | Control de selección. |
| Anexo | indicador | `annex_visible = true/false` según exista icono/enlace; nunca se abre. |
| Etapa | dato | Texto literal. |
| Trámite | dato | Texto literal (Resolución, Escrito, Actuación, …). |
| Desc. Trámite | dato | Texto íntegro; sin resumir; sin truncar. |
| Referencia | dato | Texto íntegro; «Sin referencia.» se conserva tal cual. |
| Fecha Tram | dato + filtro | `dd/mm/aaaa` → ISO `aaaa-mm-dd`, `time_precision=DIA`, `time_zone=DESCONOCIDA`. Si trae hora **[H0]**, `SEGUNDO`. Fecha ilegible: fila conservada con incidencia y **dentro** del producto (no se descarta en silencio). |
| Encabezado de causa | cotejo (+ hoja Causas si P1 = Sí) | RIT, RUC, Caratulado, F. Ingreso, Proc., Forma Inicio, Est. Adm., Etapa, Estado Proc., Tribunal, marca de reserva. |

Columnas adicionales que aparezcan en H0 se inventarían en la ficha de WP-53 y se conservan en `raw_json`.

## 3. Identidad

1. Origen único de una lectura: el enlace de historia de una fila del **listado vigente** de Seguimiento
   (Espera, Cumplimiento, Informes o Egresados según el lote). Prohibido construir la consulta desde RIT/nombre.
2. Clave de lectura: `('RUS', tribunal, causa_id)`. Varias filas (ingresos) con la misma causa → **una** lectura.
   Si la misma causa aparece con datos de vínculo distintos → `IDENTIDAD_INCIERTA`, sin leer.
3. Cotejo tras leer: RIT y tribunal visibles del encabezado vs. la fila del listado (normalizados como
   `bitacora_html.normalize`). Diferencia → advertencia en la fila de la causa (no invalida si la identidad remota
   coincide), igual que `visible_identity_warnings`.
4. En productos: `scope='CAUSA'`, `ingreso_id=''`, `person_id=''`, `program_id=''`, `identity_status='CAUSA'`.
   La relación con los registros del lote se informa aparte (hoja «Vínculos»), nunca copiando la identidad de la
   fila que abrió la ventana a los trámites.

## 4. Ventana de 4 meses (H-e)

- `hasta` = fecha local de la lectura de esa causa. `desde` = `nurus.capture.temporal.earliest(hasta)` (4 meses
  naturales; misma función que la cronología WP-50).
- Se exportan los trámites con `desde <= fecha <= hasta`. Los anteriores se **cuentan** (`fuera_de_ventana`) pero no
  se exportan. Fechas posteriores a `hasta` (reloj/servidor) se exportan con advertencia.
- Optimización permitida solo si H0 confirma orden descendente estable y paginación: dejar de pedir páginas
  cuando toda una página queda antes de `desde`. Si no se confirma, leer todo y filtrar.
- Todos los productos declaran: «Historia limitada a trámites entre DESDE y HASTA. No acredita ausencia de
  trámites anteriores ni posteriores a la lectura.»

## 5. Estados

Por causa (manifiesto interno `historias.json`): `PENDIENTE`, `LEIDA`, `VACIA_COMPROBADA` (solo si la ventana
muestra un vacío explícito reconocido **[H0]**), `FALLIDA`, `NO_DISPONIBLE` (acceso denegado o causa sin enlace de
historia en el listado), `IDENTIDAD_INCIERTA`, `CANCELADA`.
Mapeo a contrato (`coverage`): `LEIDA→COMPLETA` (respecto de la ventana), `VACIA_COMPROBADA→VACIA_COMPROBADA`,
`FALLIDA→FALLIDA`, `NO_DISPONIBLE→NO_DISPONIBLE`, `IDENTIDAD_INCIERTA→IDENTIDAD_INCIERTA`, `CANCELADA/PENDIENTE→CANCELADA`.
Reglas: tabla no reconocida, HTML de login, sesión vencida, encabezados distintos o página repetida ⇒ `FALLIDA`
con código; **nunca** «0 trámites».

Códigos de error (extensión → Python): `HISTORY_CONTEXT`, `HISTORY_LINK`, `HISTORY_IDENTITY`, `HISTORY_TIMEOUT`,
`HISTORY_NETWORK`, `HISTORY_RESPONSE`, `HISTORY_REDIRECT`. Códigos del parser: `HISTORIA_TABLA_NO_RECONOCIDA`,
`HISTORIA_ENCABEZADO_DISTINTO`, `HISTORIA_SESION`, `HISTORIA_ACCESO_DENEGADO`, `HISTORIA_FECHA_ILEGIBLE`,
`HISTORIA_PAGINA_REPETIDA`, `HISTORIA_IDENTIDAD_VISIBLE`.

## 6. Transporte (extensión)

Nuevo comando `history_open` en `descargador/extension/pagina.js` (y `history_page` solo si H0 muestra
paginación). Requisitos comunes, idénticos a `diary_open`:

- Pantalla `seguimiento` y listado vigente en `window.__sitfaRun.diaryDocument`; si no, `HISTORY_CONTEXT`.
- Payload estricto: `{tribunal, rit, causa}` (o lo que H0 determine), cada valor validado por regex; claves extra ⇒ error.
- Debe existir **exactamente un** enlace de historia en el listado vigente con esos valores; si no, `HISTORY_LINK`.
- URL final: origen `https://familia.pjud.cl`, ruta exacta `/SITFAWEB/ConsultaDetalleAtPublicoAccion.do`, parámetros
  exactamente `TipoPopup=3` y `a1` (regex de caracteres codificados, largo ≤ 512) **[H0]**.
- `fetch` con `credentials:"include"`, `cache:"no-store"`, timeout ≤ 60 s, tope 8 MB, sin redirecciones salvo el
  paso intermedio del escenario B (en lista blanca, mismo origen, `redirect:"manual"` y validación de `Location`).
- Respuesta: `{status, type, body(base64), params, startedAt, digest(SHA-256)}`.
- `a1` **no** se guarda en manifiestos, diagnósticos ni logs (es un token).
- `capabilities.protocolos.historia_lectura = 1`; versión 2.8.0 en `manifest.json`, `pagina.js` y `contrato_integral.py`.
- Sin permisos nuevos en `manifest.json` (mismo origen). Si H0 mostrara otro host: detener (cambio E3 + decisión).

## 7. Lectura (Python)

- **Una sola pasada.** La historia se lee dentro del mismo recorrido del listado que hace `DiaryRunner` para las
  bitácoras: misma consulta, mismas páginas, mismo `lock/unlock` y `verify_extension`. Así no se repiten consultas
  del listado a SITFA, el enlace de historia se valida contra la página vigente y un `a1` de vida corta se usa
  apenas aparece. Por cada fila: bitácora (como hoy) y, si la opción está activa y la causa es nueva en el lote,
  historia.
- Estado propio: módulo nuevo `descargador/lectura_historia.py` con `LectorHistoria` (sesión, manifiesto,
  lecturas, resultado); `DiaryRunner` solo recibe un colaborador opcional `historia=None` y lo llama por fila.
  Un fallo de historia **nunca** cambia el estado de una bitácora ni detiene el lote.
- Reintento: «Consultar y crear paquete» de nuevo con «Reutilizar bitácoras leídas hoy» (EF-04) hace casi gratis
  la parte de bitácoras; `LectorHistoria` recibe el `historias.json` anterior y solo lee causas `FALLIDA`/`PENDIENTE`.
- Pausa `PAUSA_SITFA_S = 1.0` entre lecturas reales; cancelación con el mismo evento.
- Tope `MAX_HISTORIAS_POR_LOTE = 600` (P3), independiente de `MAX_LECTURAS_POR_LOTE`.
- Valida: `digest` = SHA-256 del cuerpo; `params` devueltos = pedidos; tamaño ≤ 8 MB; `startedAt` ISO.
- Guarda `respaldo/historia/historia-<clave24>-<sha16>.html` con `m.save_exclusive`; si existe y difiere ⇒ error.
- Manifiesto `historias.json` (`version: 1`, `tipo: 'HISTORIA_LECTURA'`) con selección, consultas, causas, estados,
  conteos, ventana, versión/commit de la extensión. Escritura atómica (`write` de `lectura_bitacoras`).
- Diagnóstico anónimo `diagnostico_historia.json`: solo conteos, códigos y versión (como `anonymous_diagnostic`).
- Parser nuevo `src/nurus/historia_html.py`: `decode_snapshot` (reutilizar el de `bitacora_html`), `read_historia(
  data, esperado) -> {encabezado, filas, vacio_explicito, advertencias}`; encabezados exigidos por contrato
  **[H0]**; cualquier desvío ⇒ `ValueError` con código. Sin ejecutar scripts del HTML.

## 8. Productos

Ubicación: carpeta del lote/paquete. Nombres fijos; si existe, sufijo `_<8 hex>` como `Bitacoras_RUS.xlsx`.

### 8.1 `Historia_SITFA.xlsx` (revisión manual)

Generado con `openpyxl`, presentación de `excel_presentacion.py` (encabezado, filtros, panel fijo). Todo texto
pasa por `export_csmp_capture.literal` (nunca fórmula). Textos > 32.000 caracteres: partir como
`nurus.capture.diaries.text_parts` y señalarlo.

| Hoja | Una fila por | Columnas (en orden) |
|---|---|---|
| Trámites | trámite dentro de la ventana | Tribunal · RIT · Folio · Fecha Tram · Etapa · Trámite · Desc. Trámite · Referencia · Anexo visible (Sí/No) · Causa ID · Archivo fuente · Fila fuente · SHA-256 fuente · Leída el |
| Causas | causa intentada | Tribunal · RIT · Estado · Trámites en ventana · Trámites anteriores (conteo) · Desde · Hasta · Advertencias / error · [P1: RIT visible · RUC · Caratulado · F. Ingreso · Proc. · Forma Inicio · Est. Adm. · Etapa · Estado Proc. · Tribunal visible · Reservada] · Causa ID · Archivo fuente · SHA-256 |
| Vínculos | registro del lote que remite a una causa | Tribunal · RIT · Nombre · Programa · Pestaña · Modalidad · Ingreso ID · Causa ID · Estado de la historia de su causa |
| Control | — | Versión Descargador/extensión, commit, fecha de lectura, criterio `ventana_4_meses`, desde/hasta, totales por estado, tope alcanzado sí/no, leyendas: «Sin PDF», «Historia de la causa, no del ingreso», «Limitada a la ventana» |

Orden en «Trámites»: Tribunal, RIT, Fecha Tram descendente, Folio descendente.

### 8.2 Tabla `historia_tramites` en `Captura_CSMP.zip` (análisis posterior del Asistente)

Solo si el usuario marca «Incluir historia en el paquete». Mapeo por fila (contrato v1 + extensión aditiva de WP-53):

| Columna | Valor |
|---|---|
| `capture_id`, `query_id` | del exportador; una consulta por causa: `hist-<n>` |
| `source_system` | `RUS` |
| `court_code`, `court_name`, `rit_original` | del listado (tribunal y RIT de la fila de origen) |
| `cause_id` | `causa_id` remoto |
| `ingreso_id`, `person_id`, `program_id` | vacío |
| `modality_raw`, `modality_code`, `tab` | contexto de la consulta que encontró la causa |
| `captured_at` | `startedAt` de la lectura |
| `original_file` | `respaldo/historia/historia-…html` (relativo; el HTML **no** va en el ZIP) |
| `original_sheet` | vacío |
| `original_row` | posición 1-based de la fila en la tabla de historia |
| `source_sha256` | SHA-256 del HTML |
| `coverage` | `COMPLETA` |
| `event_id` | `hist-<n>-<fila>` |
| `event_at` | ISO `aaaa-mm-dd` |
| `time_precision` / `time_zone` | `DIA` / `DESCONOCIDA` |
| `scope` | `CAUSA` |
| `event_type` | Trámite |
| `description` | Desc. Trámite |
| `reference` | Referencia |
| `status` | vacío |
| `document_id` | vacío |
| `identity_status` | `CAUSA` |
| **extensión** `folio` | Folio literal |
| **extensión** `stage` | Etapa literal |
| **extensión** `annex_visible` | `true`/`false` |
| **extensión** `raw_json` | objeto JSON con todas las celdas visibles por etiqueta (sin Doc./Sel) |

Consulta (`consultas`) por causa: `screen='historia_tramites'`, filtros
`{'tribunal', 'tab': 'HISTORIA', 'capture_tables': ['historia_tramites'], 'historia_causa': {'source_system','court_code','cause_id'}, 'ventana': {'criterio':'ventana_4_meses','desde','hasta'}}`;
**sin** `remote_identity` (exige `ingreso_id`). Cobertura de la consulta = estado de la causa (§5).
`table_states['historia_tramites']` = `aggregate` de las consultas de historia (`COMPLETA`, `PARCIAL`, …);
`NO_SOLICITADA` si el usuario no pidió historia. `documentos` sigue `NO_SOLICITADA`. Incidencias por causa fallida.

Compatibilidad exigida: el Asistente actual importa el paquete sin error y **sin cambio** en registros, reglas,
productos ni paridad golden; los paquetes sin historia siguen idénticos.

### 8.3 Respaldo y control

`respaldo/historia/` (HTML íntegros), `historias.json`, `diagnostico_historia.json`. Incluidos en la retención de
60 días / 10 sesiones (D4) y en `archivo_local.py`/`ventana_resultados.py` como fuente del lote. Nunca en el ZIP.

## 9. Interfaz (Descargador)

- Casilla «Leer también la historia de la causa (últimos 4 meses)» junto a «Reutilizar bitácoras leídas hoy»;
  desmarcada por defecto; preferencia persistente en `preferencias.py`.
- Casilla «Incluir historia en el paquete» (visible solo si la anterior está marcada).
- Al marcarla, aviso único: «Se abrirá una historia por causa (no por registro), hasta 600 por lote. Aumenta el
  tiempo de la consulta.»
- Progreso dentro del existente: «Historia: N causas leídas · F fallidas». Pausa/Cancelar existentes.
- Sin botones nuevos en Avanzado: el reintento es volver a «Consultar y crear paquete» (ver §7).
- Fin: resumen con conteos por estado y botón «Abrir Historia_SITFA.xlsx».
- Si la extensión no declara `historia_lectura`, las casillas quedan deshabilitadas con el texto «Actualiza la
  extensión a 2.8.0 para leer la historia». Hasta el acta, la opción lleva la marca «(prueba)».

## 10. Límites conocidos

- Cobertura limitada a la ventana y a las causas con enlace de historia en el listado.
- Una causa reservada puede no mostrarse → `NO_DISPONIBLE`.
- El Asistente no mostrará la historia hasta un WP posterior (fuera de alcance); por ingreso verá historia como
  «no consultada».
- Rendimiento estimado (N2): ≈ 1 s de pausa + respuesta por causa. 300 causas ≈ 6–10 min adicionales.
- La descripción puede venir abreviada por SITFA; no se completa leyendo documentos.
