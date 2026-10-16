# Guía de implementación — Historia de la causa en el Descargador SITFA

**Para:** una IA o un desarrollador que **no** participó en el diseño. Este documento es autosuficiente: si algo
no está aquí, **detente y pregunta**; no lo inventes.
**Rama:** `feat/historia-causa-sitfa` (repositorio `MatiGaete2023/flow`). **Fecha:** 13 de octubre de 2026.
**Responsable funcional:** el usuario titular (usuario único). Tú programas; **no decides reglas ni alcance**.

---

## 0. Cómo usar este documento

1. Lee las secciones 1 a 6 una vez (contexto, objetivo, prohibiciones, materia prima, producto, criterios).
2. Ejecuta la sección 9 **en orden**, un paso a la vez. Cada paso dice: archivos, qué hacer, código de referencia,
   prueba y criterio de salida. No empieces un paso sin cerrar el anterior.
3. Si un paso no calza con el código real (línea movida, función renombrada): busca con `grep -n`, adapta
   **solo** nombres o líneas, nunca el comportamiento. Si el comportamiento no calza, detente y anótalo.
4. Antes de cada `git push`: `tools/ci_local.sh <pruebas del paso>`. Al final de cada paquete:
   `tools/ci_local.sh` completo y `python tools/verificar_wp.py`.
5. No leas otros planes del repositorio salvo que esta guía los cite. Ahorra tiempo y tokens.

Convención de certeza: **[HECHO]** verificado en código o en la estructura real · **[SUPUESTO]** inferencia a
comprobar en la prueba real · **[DECISIÓN]** decidido por el usuario.

---

## 1. Contexto mínimo

**Flow** son dos aplicaciones locales de escritorio (Python 3.12, Tkinter, Windows) para un funcionario judicial:

- **Descargador SITFA** (`descargador/`): con la sesión del usuario en Chrome, consulta SITFA (sistema web de
  tribunales de familia, `https://familia.pjud.cl`), guarda originales, lee **bitácoras** y arma un paquete ZIP.
- **Asistente CSMP** (`src/nurus/`): importa ese ZIP, aplica reglas y prepara documentos. **No se toca aquí.**

Cómo el Descargador habla con SITFA **[HECHO]**:

```
Descargador (Python) ──HTTP 127.0.0.1──► puente.py ──cola de tareas──► conexion.js (pestaña de la extensión)
        ▲                                                                   │ chrome.scripting.executeScript
        └──────────── resultado (base64 + SHA-256) ◄── pagina.js: sitfaTask(command, payload) ◄┘ (en la pestaña SITFA,
                                                                   world "MAIN", fetch con la sesión del usuario)
```

- `descargador/extension/pagina.js` → función `sitfaTask(command, payload)`: única que hace `fetch` a SITFA.
  Cada comando valida todo y solo permite rutas exactas.
- `descargador/extension/conexion.js` → inyecta `sitfaTask`, traduce códigos de error a mensajes.
- `descargador/puente.py` → `Bridge.call(command, payload)`: **lista blanca de comandos** y mensajes de error.
- `descargador/lectura_bitacoras.py` → `DiaryRunner`: recorre el listado de Seguimiento página a página y, por
  cada fila, abre su bitácora con el comando `diary_open`. **Es el modelo a seguir.**
- `descargador/contrato_integral.py` → `PROTOCOLS` y `VERSION`: la app exige que la extensión declare
  exactamente los mismos protocolos y versión.

---

## 2. Objetivo y alcance

**Objetivo [DECISIÓN o]:** que el Descargador lea, además de las bitácoras, la **tabla de la pestaña Historia**
de la ventana «Información de la Causa» de cada causa del lote, **solo los trámites de los últimos 4 meses**, y
la entregue:
1. en un Excel para revisión manual (`Historia_SITFA.xlsx`), y
2. en el paquete `Captura_CSMP.zip` (tabla `historia_tramites`) para que el Asistente la analice **en otro paso
   futuro** (este trabajo no cambia el Asistente).

**Dentro del alcance:** Folio, Etapa, Trámite, Desc. Trámite, Referencia, Fecha Tram, presencia de anexo
(sí/no) y el encabezado de la causa (RIT, RUC, Caratulado, F. Ingreso, Proc., Forma Inicio, Est. Adm., Etapa,
Estado Proc., Tribunal, marca de reserva).

**Fuera del alcance:** PDF y anexos (no se abren ni descargan), pestañas Notificación/Diligencias/Escritos,
botones Litigantes/Materias/Plazos/Remisión, historial por RUT (`ShowHistoriaRut`, `ShowHistorialFichas`),
cambios en el Asistente, reglas CT-03..10, visor dentro del Descargador, reutilizar historias entre sesiones.

**Uso:** personal, en Windows, 100 % local [DECISIÓN].

---

## 3. Prohibiciones (romper una = trabajo rechazado)

**Absolutas del proyecto** (`AGENTS.md`, `PLAN_VIGENTE.md`):
1. Sin escritura en RUS/SATURNO. Solo lectura.
2. Sin credenciales, cookies ni tokens en código, logs, fixtures, archivos de salida o conversaciones.
3. **Sin datos reales** (HAR, HTML, PDF, Excel, capturas) en el repositorio, la CI ni conversaciones con agentes.
   Solo fixtures ficticios. Si alguien te ofrece un archivo real, pide la estructura anónima
   (`descargador/extraer_estructura_historia.py`).
4. No tocar `src/nurus/personal/motor/`, `textos_base.json`, Outlook ni el Asistente.
5. Sin `--force`, sin merge a `main`, sin releases. No marcar nada como «aceptado»: solo el usuario, en un acta.

**Propias de esta función:**
6. No abrir, descargar ni seguir enlaces de documentos (`ShowPDF`, `Reservado`, `ShowAnexos`, `ShowPDFO`).
7. No consultar por RIT, RUT ni nombre: la historia solo se abre desde el enlace `ShowHistoria` presente en el
   listado vigente.
8. Los valores cifrados (`a1`…`e5`) **nunca salen de la extensión**: no van a Python, manifiestos, Excel, ZIP ni logs.
9. Un error, una tabla no reconocida o una sesión vencida **nunca** se informa como «sin trámites».
10. Un trámite es de la **causa**, no del niño/ingreso: `scope='CAUSA'`, `ingreso_id=''` en el paquete.
11. Con la opción apagada, el Descargador debe comportarse **exactamente** como hoy (mismos archivos y ZIP).
12. No ejecutar código de SITFA (no llamar `window.ShowHistoria`); replicar sus dos peticiones.
13. No añadir permisos ni hosts al `manifest.json` de la extensión.

---

## 4. Materia prima (lo que ya existe y debes usar)

### 4.1 Cómo se abre la historia en SITFA [HECHO, estructura anónima del 13-oct]

En la columna **RIT** de cada fila de los listados `tablaEspera` y `tablaCumplimiento` hay un enlace:

```html
<a href="#" onclick="ShowHistoria(49,'C-123-2025',4567890)">C-123-2025</a>   <!-- números sin comillas -->
```

Argumentos: `(tribunal, RIT, idCausa)`. La función de SITFA hace **dos peticiones al mismo origen**:

1. `POST https://familia.pjud.cl/SITFAWEB/Encrypt_AjaxAccion.do?opcion=1&TIP_Causa=<C>&ROL_Causa=<123>&ERA_Causa=<2025>&crr_idcausaUrl=<idCausa>&cod_tribunalUrl=<tribunal>`
   (parámetros en la URL, sin cuerpo; el RIT se parte por `-`). Respuesta 200, `content-type: text/html`,
   ~230 bytes, **cuerpo JSON** con claves `tip_causa`, `rol_causa`, `era_causa`, `cod_tribunal`, `crr_causa`
   (valores cifrados, texto).
2. `GET https://familia.pjud.cl/SITFAWEB/ConsultaDetalleAtPublicoAccion.do?TipoPopup=3&a1=<crr_causa>&b2=<tip_causa>&c3=<rol_causa>&d4=<era_causa>&e5=<cod_tribunal>&B4=1&FLG_Encripta=1`
   (valores concatenados **tal como llegan**, sin volver a codificar [SUPUESTO: vienen ya codificados para URL]).

### 4.2 La ventana «Información de la Causa» [HECHO]

- HTML `windows-1252`, ~690 KB con 269 trámites. **Toda la historia en una sola respuesta, sin paginación**,
  ordenada por fecha **descendente**.
- **Una sola `<table>`** en la página, sin `id`, `class="table table-sm table-bordered table-hover"`; `<thead>` con
  9 `<th>`: `Folio · Doc. · Sel · Anexo · Etapa · Trámite · Desc. Trámite · Referencia · Fecha Tram`.
- `<tbody>`: una `<tr>` por trámite y un `<script>` entre filas (ignorarlo).
- **Última fila incompleta real:** solo 4 celdas (Folio, Doc., Sel, Anexo). No es error.
- Encabezado de causa: `<label class="form-check-label">RIT:</label> VALOR` (valor = texto que sigue a la etiqueta).
  Marca «[Causa Reservada]» dentro de `div.panel-heading`.
- Detalle completo: `contracts/csmp_capture_v1/HISTORIA_VENTANA.md`.

### 4.3 Código ya entregado en esta rama (no reescribir)

| Pieza | Archivo | Estado |
|---|---|---|
| Lector de la ventana | `src/nurus/historia_html.py` → `read_historia(data)`, `separar_ventana(filas, hasta)`, `HEADERS`, `mensaje(codigo)` | **HECHO** + `tests/test_historia_html.py` (14 pruebas) |
| Vínculo del listado | `descargador/vinculos.py` → `extract_historia(table, tribunal_code)` | **HECHO** + 2 pruebas en `descargador/test_vinculos.py` |
| Extractor anónimo | `descargador/extraer_estructura_historia.py` | HECHO (diagnóstico; su salida incluye `lector_historia`: cómo lee el lector real una copia, solo conteos y códigos — úsalo si SITFA cambia) |
| Fixtures ficticios fieles | `descargador/fixtures/historia_ficticia/ventana_historia.html`, `listado_seguimiento.html` | HECHO |
| CI local | `tools/ci_local.sh` | HECHO |

Qué devuelve `read_historia(bytes)`:

```python
{'encabezado': {'RIT': '…', 'RUC': '…', 'Caratulado': '…', 'F. Ingreso': '…', 'Proc.': '…', 'Forma Inicio': '…',
                'Est. Adm.': '…', 'Etapa': '…', 'Estado Proc.': '…', 'Tribunal': '…'},
 'reservada': True,
 'filas': [{'fila': 1, 'folio': '9120', 'etapa': 'Cumplimiento', 'tramite': 'Resolución',
            'descripcion': '…', 'referencia': 'Sin referencia.', 'fecha_texto': '13/10/2099',
            'fecha_iso': '2099-10-13', 'anexo': False}, ...],
 'vacio_explicito': False,                     # True solo si la tabla exacta existe y no tiene filas
 'advertencias': [{'codigo': 'HISTORIA_FILA_INCOMPLETA', 'fila': 4, 'folio': '9001'},
                  {'codigo': 'HISTORIA_FECHA_ILEGIBLE', 'fila': 2, 'folio': '…'},
                  {'codigo': 'HISTORIA_ENCABEZADO_REPETIDO', 'fila': 1, 'folio': ''}],
 'sha256': '…64 hex…', 'source': '', 'captured_at': ''}
# Errores: ValueError('HISTORIA_TABLA_NO_RECONOCIDA' | 'HISTORIA_ENCABEZADO_DISTINTO' | 'HISTORIA_SESION'
#                     | 'HISTORIA_RESPUESTA_INCOMPLETA')   # este último: HTML sin </html> (respuesta cortada)
```

`separar_ventana(filas, hasta_date)` → `(dentro, anteriores, posteriores, sin_fecha)`; usa
`nurus.capture.temporal.earliest` (4 meses naturales).

`extract_historia(table, '49')` → `[{'rit','nombre','rut','programa','tribunal_codigo','causa_id','ingreso_id','identidad'}]`,
una entrada por fila con **un** enlace `ShowHistoria` concordante (tribunal y RIT). `ingreso_id` solo si la fila
trae `ShowObservaciones` de la misma causa y tribunal (si no, `''`). `identidad` es `'VERIFICADA'` o
`'INCIERTA'` (el listado asocia un RIT a dos causas o una causa a dos RIT): **las INCIERTA no se leen** y su causa
queda `IDENTIDAD_INCIERTA`. Varias filas pueden tener la misma causa: **deduplica tú** por `('RUS', tribunal, causa_id)`.

### 4.4 Código existente que reutilizas (modelos)

| Para | Mira | Líneas aprox. |
|---|---|---|
| Comando de extensión | `pagina.js` bloque `if(command==="diary_open")` y el `catch` final | 393–471 |
| Mensajes de error | `conexion.js` objeto `messages`; `puente.py` `call()` (lista blanca y `messages`) | 4–20; 184–240 |
| Recorrido del listado | `lectura_bitacoras.py` `DiaryRunner.enumerar_listado` | 461–515 |
| Lectura y guardado | `lectura_bitacoras.py` `_abrir_bitacora`, `_cerrar`, `_resultado`; `write()` | 669–747; 21 |
| Tabla del listado | `lectura_bitacoras.py` `listing_rows` (cómo ubicar la tabla con `profile`) | 46–70 |
| Decodificar respuesta | `lotes.py` `ChromeTransport.body(result)` | 128 |
| Guardar sin pisar | `motor.py` `save_exclusive(path, data)` | 662 |
| Excel | `src/nurus/capture/diaries.py` `export_diaries`, `text_parts`; `descargador/excel_presentacion.py` | 327, 310 |
| Texto seguro en Excel | `descargador/export_csmp_capture.py` `literal(value)` | 26 |
| Paquete | `descargador/export_csmp_capture.py` `export_capture` (consultas, incidencias, `table_states`) | 58–325 |
| Contrato | `src/nurus/capture/contract.py` `TABLES`, `csv_bytes` | 55–120, 265 |
| Retención | `descargador/preferencias.py` `_package_controls`, `old_batches` | 150–205 |
| Flujo «Consultar y crear paquete» | `descargador/flujo_paquete.py` `run` | 10–70 |

---

## 5. Producto final esperado

### 5.1 Archivos de código (nuevos o cambiados)

| Archivo | Cambio |
|---|---|
| `descargador/extension/pagina.js` | comando `history_open`; versión 2.8.0 en `capabilities` |
| `descargador/extension/manifest.json` | solo `version` y `version_name` → 2.8.0 |
| `descargador/extension/conexion.js` | mensajes `HISTORY_*` |
| `descargador/puente.py` | `'history_open'` en la lista blanca; mensajes `HISTORY_*` |
| `descargador/contrato_integral.py` | `PROTOCOLS['historia_lectura'] = 1`, `VERSION = '2.8.0'` |
| `descargador/lectura_bitacoras.py` | gancho opcional `historia` en `DiaryRunner` (3 líneas) |
| `descargador/lectura_historia.py` (nuevo) | `LectorHistoria` |
| `src/nurus/capture/historias.py` (nuevo) | `export_historias` → `Historia_SITFA.xlsx` y filas del contrato |
| `src/nurus/capture/contract.py` | columnas aditivas de `historia_tramites` |
| `descargador/export_csmp_capture.py`, `descargador/descargador.py` | historia en el paquete |
| `descargador/preferencias.py` | retención + 2 preferencias |
| `descargador/flujo_paquete.py`, `descargador/descargador.py` | casillas y fase de historia |
| `descargador/SITFA_Descargador.spec` | `hiddenimports` |
| Pruebas nuevas | `descargador/test_historia_extension.cjs`, `descargador/test_lectura_historia.py`, `tests/test_historia_export.py`, casos en pruebas existentes |

### 5.2 Productos para el usuario (en la carpeta del paquete)

**A. `Historia_SITFA.xlsx`** (revisión manual). Texto siempre con `literal()` (nunca fórmula). Filtros y
encabezado fijo (`excel_presentacion.configure_sheet`).

| Hoja | Una fila por | Columnas en este orden |
|---|---|---|
| Trámites | trámite dentro de la ventana | Tribunal · RIT · Folio · Fecha Tram · Etapa · Trámite · Desc. Trámite · Referencia · Anexo visible (Sí/No) · Causa ID · Archivo fuente · Fila fuente · SHA-256 fuente · Leída el |
| Causas | causa intentada | Tribunal · RIT · Estado · Trámites en ventana · Trámites anteriores · Filas incompletas · Desde · Hasta · Advertencias / error · RIT visible · RUC · Caratulado · F. Ingreso · Proc. · Forma Inicio · Est. Adm. · Etapa · Estado Proc. · Tribunal visible · Reservada (Sí/No) · Causa ID · Archivo fuente · SHA-256 |
| Vínculos | fila del listado que remite a una causa | Tribunal · RIT · Nombre · RUT · Programa · Pestaña · Modalidad · Ingreso ID · Causa ID · Estado de la historia |
| Control | — (pares Campo/Valor) | Versión Descargador, Versión extensión, Commit, Fecha de lectura, Criterio (`ventana_4_meses`), Desde, Hasta, Causas por estado, Tope alcanzado (Sí/No) y 3 leyendas: «Sin PDF: no se abrieron documentos.» · «La historia es de la causa, no de un ingreso.» · «Solo trámites entre DESDE y HASTA; no acredita ausencia de trámites fuera de ese rango.» |

Orden en «Trámites»: Tribunal, RIT, Fecha Tram descendente, Folio descendente.
Para textos que superan 30.000 unidades UTF-16, conservar la primera parte en la columna original y añadir
columnas «<campo> (parte 2)», «<campo> (parte 3)», etc. Concatenarlas reconstruye el texto íntegro sin crear
trámites duplicados ni una quinta hoja. Todas las partes siguen siendo texto literal.

**B. `historia_tramites` dentro de `Captura_CSMP.zip`** (solo si la casilla «Incluir historia en el paquete»
está marcada). Una fila por trámite dentro de la ventana; ver mapeo exacto en el paso 53.6.

**C. Respaldo** en `respaldo/historia/`: `historia-<clave24>-<sha16>.html` (HTML íntegro, nunca en el ZIP),
`historias.json` (control) y `diagnostico_historia.json` (solo conteos y códigos).

### 5.3 Interfaz

Casilla «Leer también la historia de la causa (últimos 4 meses)» (desmarcada por defecto, recordada) y casilla
«Incluir historia en el paquete» (marcada por defecto, visible solo si la anterior está marcada). Hasta el acta, el
texto termina en «(prueba)». Sin botones nuevos.

### 5.4 Documentos

Sección en `docs/MANUAL_DESCARGADOR_MVP_20261008.md`, entrada 2.8.0 en `descargador/CHANGELOG.md`, guía de prueba
`docs/aceptacion/GUIA_PRUEBAS_HISTORIA_<fecha>.md` y plantilla de acta (sección 11).

---

## 6. Criterios de aceptación (objetivos medibles)

1. Con la opción **apagada**: mismos archivos y mismo ZIP que antes (prueba automática).
2. Con la opción **encendida** sobre fixtures: una lectura por causa; cada trámite de la ventana aparece una vez
   con sus 6 campos; trámites anteriores solo contados; fila incompleta advertida.
3. Ningún caso de error produce «0 trámites»: login, tabla distinta, JSON inválido, red, tiempo agotado.
4. Ningún archivo de salida contiene `a1`…`e5`, cookies ni rutas de documentos.
5. El Asistente actual importa el ZIP con historia y produce **el mismo trabajo** que sin historia.
6. `tools/ci_local.sh` y `python tools/verificar_wp.py` → PASA. Revisión E3 → APROBADA.
7. Prueba real del usuario H-01…H-12 → PASA y acta firmada.

---

## 7. Arquitectura de la solución

```
DiaryRunner.enumerar_listado (sin cambios de comportamiento)
  por cada página del listado:
     ├─ por cada fila: leer_bitacora(...)                      (como hoy)
     └─ si historia activa: LectorHistoria.pagina(...)         (NUEVO, 1 llamada por página)
            ├─ extract_historia(tabla_de_la_página, tribunal)  → filas con causa
            ├─ deduplicar por ('RUS', tribunal, causa_id); registrar vínculos fila→causa
            └─ por cada causa nueva (máx. 600 por lote, pausa 1 s):
                 bridge.call('history_open', {'params': {tribunal, rit, causa}})
                    └─ pagina.js: valida enlace en listado vigente → POST Encrypt → GET ventana → base64 + SHA-256
                 valida digest/params/tamaño → read_historia → guarda HTML → separar_ventana → estado
LectorHistoria.cerrar() → historias.json + diagnostico_historia.json + Historia_SITFA.xlsx
package_current_lot(..., historia=historias.json) → historia_tramites en Captura_CSMP.zip
```

¿Por qué en la misma pasada? El enlace solo se valida contra la **página del listado que está cargada en la
extensión** (`window.__sitfaRun.diaryDocument`, se reemplaza en cada búsqueda). Leer la historia después de cada
página evita repetir consultas a SITFA.

---

## 8. Reglas de calidad para cada paso

- Prueba primero: escribe la prueba con fixtures ficticios, verla fallar, luego el código.
- Un commit por paso: `WP-53: <qué cambia>`; cuerpo con «Verificación: <resultado>».
- Conservar las aserciones existentes; se permite corregir fixtures de identidad discordante para probar una respuesta válida y añadir regresiones negativas explícitas.
- Formato: `ruff format` en los archivos tocados (la CI lo exige).
- Datos de prueba: solo canarios (`NOMBRECANARIO`, `C-99999-2099`, fechas 2099). RUT ficticio permitido:
  `11.111.111-1`. Corre `python tools/check_real_ids.py` antes de cada push.

---

## 9. Paso a paso

### Paso 0 · Preparar (una vez)

```sh
git fetch origin feat/historia-causa-sitfa && git checkout feat/historia-causa-sitfa
pip install uv && uv python install 3.12
$(uv python find 3.12) -m venv .venv && . .venv/bin/activate
pip install --require-hashes -r requirements/linux-py312.lock
pip install --no-deps --no-build-isolation -e . && pip install ruff
tools/ci_local.sh tests/test_historia_html.py descargador/test_vinculos.py   # debe decir «CI local: PASA»
```
Necesitas también Node (para `*.cjs`) y `xvfb-run` en Linux para pruebas de interfaz.
Marca el paquete como tomado: en `docs/implementacion/estado/WP-53.json` pon `"estado": "TOMADO"`, `"ejecutor"`,
`"tomado"`; luego `python tools/estado.py agregar && python tools/estado.py continuacion && python tools/estado.py validar`.

---

### WP-53 · Núcleo (riesgoso: una revisión E3 al final)

#### 53.1 Lector de la ventana — **HECHO**
`src/nurus/historia_html.py` y `tests/test_historia_html.py`. No lo cambies salvo error demostrado con una prueba.

#### 53.2 Vínculo del listado — **HECHO**
`extract_historia` en `descargador/vinculos.py`. El analizador léxico compartido por `calls()` también protege `extract`; conservar su comportamiento válido.

#### 53.3 Comando `history_open` en la extensión

**Archivos:** `pagina.js`, `conexion.js`, `manifest.json`, `puente.py`, `contrato_integral.py`, prueba nueva
`descargador/test_historia_extension.cjs`.

1. El bloque `history_open` y los auxiliares locales de `sitfaTask` en
   `descargador/extension/pagina.js` son la implementación de referencia. Mantener estas fronteras:
   - Parámetros exactos `tribunal`, `rit`, `causa`; tribunal del listado vigente.
   - Reconocer llamadas reales mediante análisis léxico: excluir strings, backticks, comentarios y regex.
     Una misma causa puede tener varios enlaces idénticos por sus ingresos.
   - Las dos peticiones mantienen las rutas exactas, `credentials:'include'`, `redirect:'error'` y timeout.
   - La respuesta intermedia se lee en streaming con tope de **4096 bytes** y exige exactamente las cinco
     claves `crr_causa`, `tip_causa`, `rol_causa`, `era_causa`, `cod_tribunal`, todas strings válidas.
   - La ventana se lee en streaming con tope de **8 MiB**; cancelar el lector al sobrepasarlo. Exigir HTTP 200,
     contenido no vacío y `text/html`. Python conserva una segunda validación.
   - Rechazar HTML que refleje cualquiera de los cinco valores cifrados, incluidos valores ocultos y variantes
     URL/entidades HTML, **antes** de devolver bytes a Python. Nunca devolver la URL intermedia ni el JSON cifrado.
   - Devolver solo `status`, `type`, `body`, `params`, `startedAt`, `digest`; errores con códigos cerrados.

2. En el `catch` final de `sitfaTask`, junto al caso `diary_open`, añade:

```js
    if(command==='history_open') {
      const explicit=['HISTORY_CONTEXT','HISTORY_LINK','HISTORY_IDENTITY','HISTORY_RESPONSE'].includes(error?.message)?error.message:null;
      return {__sitfaError: explicit || (diaryPhase==='RED'?(error?.name==='TimeoutError'?'HISTORY_TIMEOUT':'HISTORY_NETWORK'):
        diaryPhase==='IDENTIDAD'?'HISTORY_IDENTITY':'HISTORY_RESPONSE')};
    }
```
3. `capabilities`: `version:'2.8.0'` y `protocolos:{bitacoras_lectura:1,descarga_conjunta:1,pdf_seleccion:1,historia_lectura:1}`.
4. `manifest.json`: `"version": "2.8.0"`, `"version_name": "2.8.0 integral · bitácoras e historia"`. **Nada más.**
5. `conexion.js` → objeto `messages`: añade
   `HISTORY_CONTEXT` «No se conserva el listado actual para abrir la historia. Vuelve a consultar sin navegar en SITFA.»,
   `HISTORY_LINK` «El enlace de historia no está en el listado actual. No se abrió una causa por suposición.»,
   `HISTORY_IDENTITY` «La causa pedida no coincide con el tribunal del listado actual.»,
   `HISTORY_NETWORK` «Falló el contacto con SITFA al abrir la historia. Comprueba la sesión.»,
   `HISTORY_TIMEOUT` «SITFA no respondió a tiempo al abrir la historia.»,
   `HISTORY_RESPONSE` «SITFA no entregó una historia válida. Revisa la sesión.».
6. `puente.py` → `Bridge.call`: añade `'history_open'` a la tupla de comandos permitidos y los mismos 6 códigos
   al diccionario `messages` (texto equivalente).
7. `contrato_integral.py`: `PROTOCOLS = {'bitacoras_lectura': 1, 'descarga_conjunta': 1, 'pdf_seleccion': 1, 'historia_lectura': 1}`,
   `VERSION = '2.8.0'`; ajusta el texto de error de `verify_extension` para nombrar historia.
8. Busca y actualiza pruebas que fijan versión/protocolos: `grep -rn "2\.7\.6\|PROTOCOLS\|pdf_seleccion" descargador tests tools`.
9. Prueba `descargador/test_historia_extension.cjs` (copia el arnés de `descargador/test_bitacoras_extension.cjs`:
   `vm.createContext`, `fetch` falso que registra llamadas). Casos mínimos:
   - Válido: listado con `ShowHistoria(49,'X-1-2026',100)`, `COD_Tribunal_sel=49` ⇒ **2** `fetch` en orden:
     POST a `.../Encrypt_AjaxAccion.do?opcion=1&TIP_Causa=X&ROL_Causa=1&ERA_Causa=2026&crr_idcausaUrl=100&cod_tribunalUrl=49`,
     luego GET a `.../ConsultaDetalleAtPublicoAccion.do?TipoPopup=3&a1=…&B4=1&FLG_Encripta=1`; ambos con
     `credentials:'include'`; resultado con `digest` de 64 hex y `params` iguales a los pedidos; **sin** los valores cifrados.
   - Sin enlace en listado / RIT distinto / tribunal distinto / clave extra / RIT mal formado ⇒ error y **0** `fetch`.
   - JSON inválido, clave faltante o adicional, valor con `&`, cuerpo > 4096 o estado ≠ 200 en el paso 1 ⇒ `HISTORY_RESPONSE` y **1** `fetch`.
   - Llamadas dentro de literales/comentarios/regex ⇒ cero peticiones; HTML con cifrado reflejado o tipo no HTML ⇒
     `HISTORY_RESPONSE` sin `body`; respuesta >8 MiB ⇒ cancelación del streaming.
   - El código fuente no contiene `document.cookie`.
10. Verifica: `node descargador/test_bitacoras_extension.cjs && node descargador/test_historia_extension.cjs` y
    `tools/ci_local.sh descargador/test_poc.py descargador/test_flujos.py` (o las pruebas que tocaste).

11. Agrega `node descargador/test_historia_extension.cjs` a `.github/workflows/ci-rapida.yml` (paso «Código y
    ambas suites sintéticas», junto a `test_bitacoras_extension.cjs`) y a `tools/ci_local.sh` (misma posición).
    Un archivo de prueba que la CI no ejecuta no protege nada.

**Salida:** ambas pruebas `.cjs` pasan localmente y en la CI; ninguna otra prueba falla.

#### 53.4 Lectura en una pasada — `LectorHistoria`

**Archivos:** `descargador/lectura_historia.py` (nuevo), `descargador/lectura_bitacoras.py` (gancho),
`descargador/test_lectura_historia.py` (nuevo).

1. Gancho en `lectura_bitacoras.py`: todas las llamadas opcionales pasan por
   `DiaryRunner._historia('abrir'|'pagina'|'cerrar', ...)`. Aislar también fallos de disco y del aviso de error;
   ninguna frontera de historia debe abortar ni marcar fallida una consulta de bitácoras.
2. `descargador/lectura_historia.py` es la implementación de referencia:
   - Inicializar un manifiesto nuevo por consulta. `anterior` se admite solo como reintento explícito del lote
     elegido; nunca buscar historias recientes del destino. Activar únicamente causas presentes en el listado
     de ese reintento y cuyo HTML conserva su SHA-256; FALLIDA/PENDIENTE se releen.
   - Mantener índices RIT → causas y causa → RIT durante **todo el lote**, separados por tribunal. Ante conflicto
     entre páginas, marcar todas las causas y vínculos afectados `IDENTIDAD_INCIERTA` y excluir sus trámites.
   - Cotejar RIT y tribunal visibles contra el listado/catálogo antes de guardar el HTML.
   - Tope 600 y pausa solo tras lectura real; cancelación y límite de bitácoras dejan pendientes sin leer.
   - Persistencia protegida en apertura, página y cierre. Un fallo deja historia `INCOMPLETA`; las bitácoras
     continúan. Diagnóstico con códigos cerrados, nunca `str(exc)` ni rutas de usuario.
   - Registrar `HISTORIA_FECHA_POSTERIOR` al conservar filas posteriores al día de lectura.
   - Cerrar con diagnóstico anónimo y Excel. Un fallo de diagnóstico, Excel o persistencia final deja el
     resultado de historia incompleto y el paquete puede conservar las bitácoras.

3. Pruebas `descargador/test_lectura_historia.py` — usa un `Bridge` falso que responde a `history_open` con el
   fixture `ventana_historia.html` en base64:
   `{'status': 200, 'type': 'text/html; charset=windows-1252', 'body': b64, 'digest': sha, 'params': params, 'startedAt': '2099-10-13T12:00:00Z'}`
   (`status` y `type` son obligatorios: `ChromeTransport.body` los exige). El RIT del fixture es `C-99999-2099`:
   el listado falso debe usar ese RIT, o la lectura fallará con `HISTORIA_IDENTIDAD_VISIBLE` (eso también se prueba).
   Casos: dos filas misma causa ⇒ 1 llamada; fila sin enlace ⇒ no se llama y la causa no aparece; respuesta de
   login ⇒ `FALLIDA` con `HISTORIA_SESION`; digest distinto ⇒ `FALLIDA`; tope (pon el tope en 1 con monkeypatch)
   ⇒ resto `PENDIENTE` y `tope_alcanzado`; cancelar ⇒ no más llamadas; `anterior` ⇒ solo releer `FALLIDA`;
   con fecha de lectura 2099-10-13 ⇒ `en_ventana=3`, `anteriores=0`, `incompletas=1`; ningún archivo de la
   carpeta contiene `a1=`; las bitácoras del mismo lote son idénticas con y sin historia (usa el arnés de
   `descargador/test_lectura_bitacoras.py`); RIT visible distinto al pedido ⇒ `FALLIDA` con
   `HISTORIA_IDENTIDAD_VISIBLE` y **sin** archivo guardado; respuesta sin `</html>` ⇒ `FALLIDA` con
   `HISTORIA_RESPUESTA_INCOMPLETA`; fila con `identidad='INCIERTA'` ⇒ causa `IDENTIDAD_INCIERTA` sin llamada;
   un `PocError` dentro de `pagina()` no cambia el estado de la consulta de bitácoras.

**Salida:** `tools/ci_local.sh descargador/test_lectura_historia.py descargador/test_lectura_bitacoras.py` PASA.

#### 53.5 Excel — `src/nurus/capture/historias.py`

1. `export_historias(manifest_path, destination) -> Path`:
   - Lee `historias.json`; para cada causa `LEIDA`/`VACIA_COMPROBADA` relee el HTML, **verifica SHA-256** (si no
     coincide: `ValueError`), aplica `read_historia` y `separar_ventana(filas, fecha_de_lectura)`.
   - Escribe las 4 hojas **exactas** de 5.2-A con `openpyxl`, en un archivo temporal y luego `replace`
     (destino existente ⇒ `ValueError`).
   - Todo texto con `literal()` de `descargador/export_csmp_capture.py` (impórtala o muévela a
     `src/nurus/capture/` y reexpórtala; no la reimplementes distinta). Textos > 30.000 con `text_parts`.
   - Fechas en la hoja como texto `dd/mm/aaaa` (literal visible) — no conviertas a fecha de Excel.
2. `LectorHistoria.cerrar()` llama a `export_historias(self.carpeta / MANIFEST, carpeta_paquete / 'Historia_SITFA.xlsx')`
   (si existe, sufijo `_<8 hex>` como `Bitacoras_RUS.xlsx`).
3. Pruebas `tests/test_historia_export.py`: hojas y columnas exactas; orden; «Sin referencia.» intacto; `=…`
   queda como texto; causa `FALLIDA` aparece en «Causas» y no en «Trámites»; HTML alterado ⇒ error; Control con
   las 3 leyendas.

#### 53.6 Paquete para el Asistente

1. `contract.py`:
   - Debajo de `TABLES`: `EXTENSIONS = {'historia_tramites': ('folio', 'stage', 'annex_visible', 'raw_json')}`.
   - En `csv_bytes(table, rows)`: columnas = `TABLES[table] + tuple(c for c in EXTENSIONS.get(table, ()) if any(c in r for r in rows))`;
     usa esas columnas en `fieldnames`, en la comprobación de columnas no definidas y en `writerow`.
   - En la validación de filas (`_validate_rows`), si la fila trae `annex_visible`, debe ser `'true'` o `'false'`.
   - **No** cambies `VERSION`, `VERSION_CONSOLIDATED` ni `required_features`. El lector ya acepta columnas
     extra al final (`fields[:len(TABLES[table])]`).
   - Documenta en `contracts/csmp_capture_v1/DICCIONARIO.md` (sección `historia_tramites`, «Extensión aditiva»).
2. `export_capture(..., historia=None)`: si `historia` (ruta a `historias.json`), por cada causa crea una consulta
   y filas:

| Columna | Valor |
|---|---|
| `query_id` | `hist-<n>` (n = 1, 2, … por causa) |
| `source_system` | `'RUS'` |
| `court_code` / `court_name` | tribunal y su nombre del catálogo |
| `rit_original` | RIT del listado |
| `cause_id` | `causa_id` |
| `ingreso_id`, `person_id`, `program_id` | `''` |
| `modality_raw`, `modality_code`, `tab` | de la primera fila vinculada |
| `captured_at` | `leida` |
| `original_file` | `respaldo/historia/<archivo>` |
| `original_sheet` | `''` |
| `original_row` | `str(fila)` (posición en la tabla, ≥ 1) |
| `source_sha256` | SHA-256 del HTML |
| `coverage` | `'COMPLETA'` |
| `event_id` | `f'hist-{n}-{fila}'` |
| `event_at` | `fecha_iso` |
| `time_precision`, `time_zone` | `'DIA'`, `'DESCONOCIDA'` |
| `scope` | `'CAUSA'` |
| `event_type` | `tramite` |
| `description` | `descripcion` |
| `reference` | `referencia` |
| `status`, `document_id` | `''` |
| `identity_status` | `'CAUSA'` |
| `folio` | `folio` |
| `stage` | `etapa` |
| `annex_visible` | `'true'` / `'false'` |
| `raw_json` | `json.dumps({'Folio':…,'Etapa':…,'Trámite':…,'Desc. Trámite':…,'Referencia':…,'Fecha Tram':…}, ensure_ascii=False)` |

   Solo filas **dentro de la ventana** (incluye posteriores y sin fecha con su advertencia como incidencia).
   Consulta: `screen='historia_tramites'`, filtros `{'tribunal': …, 'tab': 'HISTORIA', 'capture_tables': ['historia_tramites'],
   'historia_causa': {'source_system':'RUS','court_code':…,'cause_id':…}, 'ventana': {'criterio':'ventana_4_meses','desde':…,'hasta':…}}`.
   **No uses `remote_identity`** (exige `ingreso_id`). Cobertura de la consulta: `LEIDA→COMPLETA`,
   `VACIA_COMPROBADA→VACIA_COMPROBADA`, `FALLIDA→FALLIDA` (+ incidencia `HISTORIA_FALLIDA`),
   `IDENTIDAD_INCIERTA→IDENTIDAD_INCIERTA` (+ incidencia), `PENDIENTE/CANCELADA→CANCELADA`. Crear además consultas/incidencias `HISTORIA_ENUMERACION_FALLIDA` cuando falle una página o el
   lote no termine de enumerarse, incluso sin causas leídas. `table_states['historia_tramites']` = `aggregate` de todas esas coberturas (función
   `aggregate` de `src/nurus/capture/coverage.py`). Sin `historia` ⇒ salida idéntica a la de hoy.
3. `descargador.py` → `package_current_lot(sheets_folder, diary_manifest, target, consolidated=False, historia=None)`
   pasa `historia` a `export_capture_with_note`/`export_capture`.
4. Pruebas: ida y vuelta (`nurus.capture.read_capture`) con historia ficticia; filas `scope=CAUSA` sin
   `ingreso_id`; **el Asistente** (`nurus.personal.capture_input.import_capture`) importa con y sin historia y
   el trabajo resultante es igual; paquete sin historia igual al actual;
   `python tools/flow_capture_cli.py validate contracts/csmp_capture_v1/fixtures/completa.zip` (y `parcial.zip`) PASA.

#### 53.7 Retención y empaquetado Windows

1. `preferencias.py`: añade `'historias.json'` a las dos tuplas `('resumen.json', 'bitacoras.json', 'flujo.json')`
   (en `_package_controls` el patrón es `backup.glob('*/' + name)`: la historia vive en `respaldo/historia/`, por
   lo que queda cubierta). Rechazar enlaces simbólicos en todos los componentes de la ruta y revalidar cada
   candidato inmediatamente antes de archivar o borrar. Prueba nueva en `descargador/test_retencion_lotes.py`: un paquete con solo
   `historias.json` antiguo es elegible; reciente, no.
2. `SITFA_Descargador.spec`: `hiddenimports` += `'nurus.historia_html'`, `'nurus.capture.temporal'`, `'nurus.capture.historias'`.

#### Cierre de WP-53

```sh
tools/ci_local.sh
python tools/verificar_wp.py --salida docs/implementacion/verificacion/WP-53.json
```
Ambos PASA. Pide la revisión E3 (sección 12). Con APROBADA, actualiza `docs/implementacion/estado/WP-53.json`
(`estado`, `verificacion`, `revision`) y regenera con `tools/estado.py agregar|continuacion|validar`.

---

### WP-54 · Interfaz, flujo, entrega y acta (no riesgoso)

1. `preferencias.py`: defaults `'leer_historia': False`, `'historia_en_paquete': True` (junto a `'reutilizar_bitacoras'`).
2. `descargador.py`: junto a la casilla «Reutilizar bitácoras leídas hoy» (busca ese texto), añade las 2 casillas de
   5.3, ligadas a esas preferencias. Al marcar la primera, aviso único: «Se abrirá una historia por causa (no por
   registro), hasta 600 por lote. La consulta tardará más.» Deshabilítalas con el texto «Actualiza la extensión a
   2.8.0 para leer la historia» si `capabilities` no trae `historia_lectura`. Muestra en el progreso existente
   los eventos `history_progress` («Historia: N leídas · F fallidas»).
3. `flujo_paquete.py` → `run(..., historia=False, historia_en_paquete=True)`: si `historia`, crea
   `LectorHistoria(bridge, relay, cancel, pause=pause)` (sin reutilización entre consultas nuevas)
   y pásalo a `DiaryRunner(..., historia=lector)`. El botón existente «Reintentar lecturas fallidas…»
   puede pasar `anterior` del lote elegido explícitamente; la casilla de reutilización de bitácoras no autoriza
   reutilizar historias de otro lote. Pasa `respaldo/historia/historias.json` a `package_current_lot`
   si `historia_en_paquete`. `partial` debe ser `True` si la historia no quedó `COMPLETA`. Un fallo de historia
   **nunca** impide crear el paquete.
4. Pruebas en `descargador/test_flujo_paquete.py`: apagada ⇒ paquete idéntico; encendida ⇒ `Historia_SITFA.xlsx` +
   tabla en ZIP; historia fallida ⇒ paquete creado y `partial=True`. Prueba de UI de las casillas (patrón de las
   pruebas Tk existentes; `xvfb-run`).
5. Documentos: sección «Historia de la causa» en `docs/MANUAL_DESCARGADOR_MVP_20261008.md` (qué lee, qué no, 4
   meses, sin PDF, cómo recargar la extensión 2.8.0) y entrada 2.8.0 en `descargador/CHANGELOG.md`.
   Copia la sección 11 a `docs/aceptacion/GUIA_PRUEBAS_HISTORIA_<fecha>.md`.
6. `tools/ci_local.sh` y `verificar_wp.py` PASA; push.
7. **Windows, una sola vez** (cuando el usuario lo pida): un PR `feat/historia-causa-sitfa → feat/ejecucion-plan-20261008`
   y un `workflow_dispatch` de «CSMP Assistant Windows» para construir el instalador. El usuario prueba ese
   artefacto y firma el acta. No integres sin su autorización escrita.

---

## 10. Pruebas: lista consolidada y comandos

| Archivo | Qué prueba |
|---|---|
| `tests/test_historia_html.py` | lector (HECHO) |
| `descargador/test_vinculos.py` | vínculo (HECHO) |
| `descargador/test_historia_extension.cjs` | comando de extensión |
| `descargador/test_lectura_historia.py` | lector en una pasada |
| `tests/test_historia_export.py` | Excel |
| `tests/test_capture_contract.py`, `tests/test_capture_input.py` | contrato aditivo y Asistente sin cambios |
| `descargador/test_retencion_lotes.py` | retención |
| `descargador/test_flujo_paquete.py` | flujo apagado/encendido/fallido |

Comandos: `tools/ci_local.sh <archivos>` mientras desarrollas; `tools/ci_local.sh` y
`python tools/verificar_wp.py` al cerrar. En Linux sin pantalla, `ci_local.sh` usa `xvfb-run` si existe.

---

## 11. Prueba real (la hace el usuario) y acta

Alcance pequeño: un tribunal, Cumplimiento y Espera, período habitual. Anotar solo conteos y PASA/FALLA.

| Código | Operación | Esperado |
|---|---|---|
| H-01 | Recargar la extensión 2.8.0 y abrir el Descargador | Casilla de historia habilitada |
| H-02 | Casilla desmarcada; «Consultar y crear paquete» | Igual que antes; sin `Historia_SITFA.xlsx` |
| H-03 | Marcar historia; iniciar | Aviso «una historia por causa, hasta 600» |
| H-04 | Dejar terminar | Progreso de historia; resumen con conteos |
| H-05 | Abrir `Historia_SITFA.xlsx` | Hojas Trámites, Causas, Vínculos, Control |
| H-06 | 5 causas (2 de Espera): comparar con la ventana SITFA | Mismos folios, fechas, trámites, descripciones y referencias de los últimos 4 meses |
| H-07 | En esas causas, «Trámites anteriores» | Coincide con lo que la ventana muestra antes de «Desde» |
| H-08 | Causa con varios ingresos del lote | Una fila en Causas, varias en Vínculos, trámites sin repetir |
| H-09 | Cancelar; usar «Reintentar lecturas fallidas…» y elegir el mismo lote; luego iniciar una consulta nueva | El reintento conserva historias verificadas del lote elegido; la consulta nueva vuelve a leer todas sus causas |
| H-10 | Revisar Descargas y carpeta del lote | Cero PDF de trámites |
| H-11 | Importar el ZIP en el Asistente | Importa; mismo trabajo que sin historia |
| H-12 | Cerrar sesión SITFA a mitad (opcional) | Restantes FALLIDA con motivo; nada «vacío» |

Acta `docs/aceptacion/ACTA_HISTORIA_<fecha>.json`:

```json
{"tipo": "ACTA_HISTORIA_CAUSA", "fecha": "", "version_descargador": "2.8.0", "commit": "",
 "alcance": {"tribunales": 0, "modalidades": [], "registros": 0, "causas": 0},
 "resultados": {"H-01": "", "H-02": "", "H-03": "", "H-04": "", "H-05": "", "H-06": "", "H-07": "", "H-08": "", "H-09": "", "H-10": "", "H-11": "", "H-12": ""},
 "conteos": {"LEIDA": 0, "FALLIDA": 0, "PENDIENTE": 0, "tramites_en_ventana": 0},
 "observaciones": "", "firmado_por": "", "decision": ""}
```

---

## 12. Git, CI y revisión E3

- Trabaja **directo** en `feat/historia-causa-sitfa` (sin PR por paso). Cada push corre «CI rápida Linux»
  (~3 min). Commits solo de documentos con `[skip ci]`.
- Antes de cada push: `tools/ci_local.sh <pruebas>`. Un push rojo cuesta una corrida: no empujes a ciegas.
- **E3** (obligatoria al cerrar WP-53, hecha por otra sesión o persona): revisar el diff completo con este foco:
  extensión solo pide las 2 rutas exactas y no lee cookies; ningún error queda como vacío; filas CAUSA sin
  `ingreso_id`; valores cifrados nunca persisten; paquete sin historia idéntico; Asistente sin cambios de
  trabajo; retención incluye `historias.json`. Respuesta: APROBADA o CAMBIOS (`archivo:línea → problema`).

---

## 13. Problemas frecuentes

| Síntoma | Causa probable | Qué hacer |
|---|---|---|
| `Accion no permitida.` | Falta `'history_open'` en la lista blanca de `puente.py` | Paso 53.3.6 |
| La app rechaza la extensión | `PROTOCOLS` o `VERSION` distintos entre `pagina.js` y `contrato_integral.py` | Igualar ambos (2.8.0, `historia_lectura:1`) |
| Todo `HISTORY_LINK` | Se llamó después de cambiar de página del listado | Llamar `LectorHistoria.pagina` antes de pedir la página siguiente |
| `HISTORIA_SESION` | Sesión SITFA vencida | Correcto: queda FALLIDA; el usuario reinicia sesión y reintenta |
| `HISTORIA_ENCABEZADO_DISTINTO` en real | SITFA cambió la tabla | No adaptar a ciegas: pedir estructura anónima nueva al usuario |
| `ruff format` falla en CI | Archivo sin formatear | `ruff format <archivo>` y repetir `ci_local.sh` |
| `check_real_ids` falla | Un RUT con dígito verificador válido en un fixture | Usar `11.111.111-1` o canarios no válidos |

---

## 14. Glosario

- **SITFA / RUS:** sistema web del Poder Judicial; aquí solo se lee.
- **Listado:** resultado de Seguimiento (pestañas Espera, Cumplimiento, Informes, Egresados).
- **Causa / ingreso:** una causa (RIT) puede tener varios ingresos (niños/programas). La historia es de la causa.
- **Bitácora:** observaciones por ingreso (ya implementado). **Historia:** trámites de la causa (esta función).
- **Ventana de 4 meses:** desde `earliest(fecha_de_lectura)` hasta la fecha de lectura.
- **E3:** revisión independiente obligatoria para cambios riesgosos (extensión, contrato, retención).
- **Canario:** valor ficticio único usado para comprobar que nada real se filtra.

## 15. Lista final de comprobación

- [ ] Opción apagada ⇒ comportamiento idéntico (prueba).
- [ ] 2 peticiones exactas por causa; sin cookies; sin valores cifrados en archivos.
- [ ] Una lectura por causa; tope 600; pausa 1 s; cancelar y reintentar funcionan.
- [ ] `Historia_SITFA.xlsx` con 4 hojas exactas; texto literal; leyendas.
- [ ] `historia_tramites` en ZIP con `scope=CAUSA`; Asistente sin cambios; fixtures de contrato válidos.
- [ ] Retención y `hiddenimports` actualizados.
- [ ] `tools/ci_local.sh` y `verificar_wp.py` PASA; E3 APROBADA; estado WP actualizado.
- [ ] Manual, CHANGELOG y guía de prueba; acta firmada por el usuario.
