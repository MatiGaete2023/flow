# Flow en Rust — plan de ejecución y estado

**Si eres otra IA o persona que retoma: lee este archivo entero y sigue desde el primer paso sin marcar.**
Actualiza la tabla de estado y haz commit al terminar cada paso.

## 0. Cómo retomar

```bash
git fetch origin rust/producto && git checkout rust/producto
```

```bash
cd rust && cargo test --workspace
```

- Rama única de trabajo: `rust/producto`. No tocar `Rediseno` (herramientas Python) ni `main`.
- Plan general y decisiones: `docs/proyecto_rust/PLAN_PROYECTO_RUST_20261112.md`.
- Informe previo: `docs/migracion_rust/INFORME_MIGRACION_RUST_20261112.md`.
- El código Python del repo es la **especificación y el oráculo**. No se modifica.
- Antes de cada commit: `python3 tools/check_real_ids.py --stage && python3 tools/check_contactos.py --stage`.
- Commits pequeños, mensaje en español, push a `origin rust/producto`.

## 1. Reglas que no se negocian (decisión del usuario, 12-nov-2026)

1. Sin ingreso de datos en RUS ni SATURNO. No existe módulo de escritura.
2. Sin envío de correos. Outlook: solo guardar borrador. El identificador `Send` no puede aparecer en
   `crates/flow-outlook` (lo controla `crates/flow-outlook/tests/sin_envio.rs`).
3. Todo correo con CC `ucc_concepcion@pjud.cl` (constante `CC_FIJO`, no configurable).
4. Sin datos reales en repo, CI ni conversaciones: solo ficticios. Si alguien ofrece un archivo real, pedir el
   extracto anónimo (`descargador/extraer_estructura_*.py`, `tools/anonimizar_excel.py`).
5. Sin credenciales, cookies ni tokens en código, registros o fixtures.
6. Los textos de observaciones se copian literalmente de `src/nurus/personal/textos_base.json`. No se redactan.
7. Reglas y umbrales: los vigentes. Cambiar una regla es decisión del usuario.
8. No marcar nada como «aceptado»: solo el usuario.

## 2. Arquitectura

```
rust/
  Cargo.toml              workspace
  EJECUCION.md            este archivo
  datos/                  textos.json, umbrales.json (copiados del Python)
  oraculo/                scripts Python que generan casos esperados con el motor Python
  casos/                  casos JSON generados (ficticios) que las pruebas de Rust comparan
  crates/
    flow-dominio          tipos (Celda, Registro, Etapa, Tribunal, Tramite…), fechas y texto en español
    flow-motor            reglas → observación, eventos, marcas, incidencias, acciones. Sin E/S ni reloj
    flow-tabular          lectura de Excel RUS (.xls BIFF/HTML/XML 2003, .xlsx), cabeceras y mapeo de columnas
    flow-productos        Excel de observaciones y nóminas, Word de proyectos, cuerpo de correos
    flow-outlook          borradores por COM (solo Windows); en otros sistemas, simulación para pruebas
    flow-trabajo          tabla analizada + observación final editable; guardar/cargar y retención
    flow-captura          puente local con la extensión de Chrome, lotes (fase posterior)
    flow-anonimo          anonimizador (fase posterior)
    flow-meta4            Meta 4 (fase posterior)
    flow                  ejecutable: subcomandos y ventana (egui)
  extension/              extensión Chrome MV3 (fase de captura)
```

Decisiones técnicas tomadas (cambiar solo con motivo escrito aquí):

- **Interfaz: egui/eframe.** Rust puro, un `.exe`, compila en Linux para probar. Motivo: velocidad de
  implementación y cero dependencias de sistema.
- **Motor por traducción fiel de la lógica, con tipos.** El orden de evaluación de `reglas_*.py` se conserva porque
  el texto final depende de él. Lo «rediseñado» es la entrada tipada (`Celda`, ausente ≠ ilegible), sin estado
  global: el contexto es un struct que se pasa.
- **Oráculo.** `oraculo/generar_casos.py` arma registros ficticios al azar (semilla fija), ejecuta el motor Python y
  guarda entrada y salida esperada en `casos/*.json`. `cargo test -p flow-motor` exige igualdad exacta. Las
  diferencias aceptadas se anotan en la sección 5.
- **Fechas en texto:** se aceptan `aaaa-mm-dd`, `aaaa-mm-dd hh:mm:ss`, `dd/mm/aaaa`, `dd-mm-aaaa` y serial de Excel.
  El Python además acepta formatos libres vía pandas; en Rust eso es «ilegible» e incidencia. Diferencia deliberada.

## 3. Estado

Marcar `[x]` al terminar. Un paso está terminado cuando su verificación pasa.

### Fase 1 — Base
- [x] 1.1 Workspace, `flow-dominio` (Celda, fechas, texto), datos copiados.
- [x] 1.2 CI `.github/workflows/rust.yml`: test Linux + compilación Windows que publica el `.exe` como artefacto.

### Fase 2 — Motor
- [x] 2.1 `flow-motor`: textos, composición, comunes, Espera, Cumplimiento, Informes, Historia, acciones.
- [x] 2.2 Oráculo: `oraculo/generar_casos.py` y casos en `casos/`.
- [x] 2.3 Prueba diferencial en verde (`cargo test -p flow-motor`).

### Fase 3 — Lectura
- [x] 3.1 `flow-tabular`: detección por contenido, BIFF/OOXML (calamine), HTML y XML 2003 (propio).
- [x] 3.2 Fila de cabecera, mapeo de columnas (`src/nurus/rus/columns.py`), hoja 2 de cruce, pre-pase
      (duplicidad, acumulados).
- [x] 3.3 `flow observar ARCHIVO --etapa E --corte F` imprime JSON; comparado con Python sobre fixtures ficticios.

### Fase 4 — Productos
- [x] 4.1 Excel de salida con observaciones (libro nuevo `.xlsx`, columnas originales + OBSERVACIÓN + avisos).
- [x] 4.2 Correos: destinatarios por programa (contactos locales), asunto, cuerpo, tabla; CC fijo.
- [x] 4.3 `flow-outlook`: `guardar_borrador` por COM; prueba `sin_envio`.
- [x] 4.4 Word: proyectos PC_IE, PC_INFO, NOMENCL desde plantillas `.docx`.
- [x] 4.5 `flow-almacen`: guardar/cargar trabajo, retención 60 días / 10 sesiones.

### Fase 5 — Interfaz
- [x] 5.1 Ventana: Trabajo (abrir Excel, tabla, editar observación), Correos, Resoluciones, Resultados.
- [x] 5.2 Prueba de humo con `xvfb-run` y captura.

### Fase 5b — Paquete del Descargador (agregada)
- [x] 5b.1 `flow-trabajo::paquete`: importa el ZIP `CSMP_CAPTURE` (huellas verificadas), historia por causa, bitácora
      por ingreso y próximo informe del calendario. Oráculo `oraculo/generar_paquete.py` contra `import_capture`.
- [ ] 5b.2 Avisos de bitácora CT-05, CT-07 (con texto `COMUN.REITERA_OBSERVACION`) y CT-11
      (`src/nurus/personal/bitacora_avisos.py`; necesita la cronología de `capture/temporal.py` y una razón de
      similitud como `difflib.SequenceMatcher`).
- [ ] 5b.3 Avisos N-03/N-04 (`contexto_excel.marcar_paquete`), OB-01..05 (`ordenes_busqueda.py`), cotejo y
      proyecciones del calendario por ingreso (`calendario_ingreso.py`), registros EGRESADOS (hoy se omiten).

### Fase 6 — Anonimizador
- [ ] 6.1 `flow-anonimo` + `flow anonimizar` (xlsx/xls/csv/json/txt), verificación «falla cerrado».

### Fase 7 — Captura SITFA

Se porta primero el alcance de **SITFA Lite** (descargar los XLS); bitácoras e historia quedan para 7.4. Mientras,
el Descargador Python sigue sirviendo y su ZIP se importa con 5b.1.

Cómo funciona hoy (leído en `descargador_lite/`):

1. El programa abre un servidor HTTP en `127.0.0.1:<puerto libre>` y muestra el código `<puerto>-<32 hex>`.
2. El usuario pulsa el icono de la extensión en la pestaña SITFA; se abre `conexion.html`, pega el código.
3. La extensión hace `POST /connect` y luego un ciclo `GET /next` (espera larga de 4 s; responde `{"idle":true}` o
   una tarea `{id, command, payload}`) y `POST /complete` con `{id, ok, result | error, motivo?}`.
   Cabeceras obligatorias: `X-Sitfa-Code` (los 32 hex, comparación en tiempo constante) y `X-Sitfa-Extension`
   (id de la extensión, `[a-p]{32}`); `Origin` `chrome-extension://<id>` si viene. El primer origen queda fijado.
   CORS: `Access-Control-Allow-Origin` solo a ese origen, `Allow-Private-Network: true`, `Cache-Control: no-store`.
   Límite de cuerpo 112 MiB. Sin registro de peticiones. Código y tareas solo en RAM.
4. Comandos permitidos (lista cerrada): `capabilities`, `catalog`, `lock`, `unlock`, `prepare`, `search`,
   `download`, `evidence_open`, `evidence_capture`, `evidence_close`.
   - `capabilities` → `{tipo, version, protocolos, escritura_rus:false}`; el programa exige versión y protocolos
     iguales y **rechaza si `escritura_rus` no es `false`**.
   - `catalog {screen?}` → tribunales, modalidades, informes, medidas, pestañas, estados, meses, años, selección.
   - `lock` / `unlock`: cubre la página y restaura el formulario.
   - `prepare {selection, dates, keep_filters}` → `{pairs}` (el formulario tal como lo enviaría SITFA).
   - `search {pairs, timeout}` → `{status, type, body(base64 del HTML)}`.
   - `download {url, timeout}` → igual, con el XLS. Solo `https://familia.pjud.cl/sitfa/reportes/<nombre>.xls`.
5. El programa valida cada respuesta antes de guardar (`descargador_lite/app/motor.py`): no es página de login;
   un solo formulario `InformesPpalForm` con el destino esperado; mismo tribunal, modalidad y pestaña; sin campos
   `csrf|token|nonce`; paginación `1 ≤ actual ≤ total ≤ 500`; la URL de exportación cumple el patrón; y los
   registros (RIT + nombre) del XLS coinciden con los de las páginas HTML. Consulta vacía → PDF de evidencia.
6. Lote (`lotes.py`): por cada selección (tribunal × pestaña × modalidad): `prepare` → `search` página 1 →
   resto de páginas → `download` → verificación → guardar con nombre seguro, `resumen.json` y `verificacion.json`.
   Pausa entre consultas, cancelación, y nunca sobrescribe.

Pasos:

- [x] 7.1 `flow-captura::puente`: servidor HTTP con el protocolo del punto 3 y `llamar(comando, payload, plazo)`.
      Pruebas con un cliente HTTP en el mismo test: código malo → 403; origen distinto → 403; tarea → resultado;
      plazo vencido → `TASK_EXPIRED`. Fuente: `descargador_lite/app/puente.py`.
- [ ] 7.2 `flow-captura::sitfa`: validación de respuestas y paginación (puntos 5). Fuente: `motor.py`,
      `perfiles.py`, `filtros_respuesta.py`. Probar con HTML del SITFA ficticio
      (`descargador_lite/tests/sitfa_ficticio.py` lo genera; guardar muestras en `tests/datos/`).
- [ ] 7.3 Lote, extensión y ventana: copiar `descargador_lite/extension/` a `rust/extension/` cambiando nombre y
      textos (y `tipo`/`version` de `capabilities`, que el programa verifica); pestaña «Captura» con código de
      conexión, opciones y avance; lo descargado pasa directo a Trabajo. Probar de punta a punta con Playwright +
      Chromium (están en este contenedor) contra el SITFA ficticio.
- [ ] 7.4 Bitácoras, historia de la causa y calendario (`descargador/lectura_bitacoras.py`,
      `lectura_historia.py`, `extension/pagina.js` del Descargador integral, `src/nurus/bitacora_html.py`,
      `historia_html.py`, `calendario_html.py`). Ritmo y límite de 600 lecturas por lote (decisión m).

Prohibido en toda la fase: leer cookies o contraseñas, enviar formularios que no sean las consultas listadas,
guardar el código de conexión, y cualquier escritura en RUS.

### Fase 8 — Meta 4 y cierre
- [ ] 8.1 `flow-meta4`.
- [ ] 8.2 Manual de uso, guía de pruebas del usuario, instalador.

## 4. Guía por paso (fuentes en el Python)

| Paso | Leer | Notas |
| --- | --- | --- |
| 2.1 | `src/nurus/personal/motor/*.py`, `personal/work.py` (`analyze`, `_prepass`, `actions_for`), `personal/contexto_excel.py` | El orden de los fragmentos importa |
| 3.1 | `descargador/motor.py` (`read_excel`), `src/nurus/rus/reader.py` | SITFA guarda HTML y XML con extensión `.xls` |
| 3.2 | `src/nurus/rus/columns.py`, `rus/util.py` | Columna ambigua = error, no adivinar |
| 4.1 | `src/nurus/services/exports.py`, `personal/excel_presentation.py` | Textos que empiezan con `=` se escriben como texto |
| 4.2 | `src/nurus/personal/outputs.py` (`prepare_drafts`), `personal/correos_base.json`, `personal/config.py` | |
| 4.3 | `src/nurus/adapters/outlook.py` | Solo `Save()` |
| 4.4 | `src/nurus/personal/resolutions.py`, `outputs.py` (`fill_docx`, `word_values`), `plantillas_word/` | |
| 4.5 | `src/nurus/personal/session.py`, `work.py` (`save`, `load`) | |
| 6.1 | `anonimizador/anonimizador_csmp/` | |
| 7.x | `descargador/puente.py`, `motor.py`, `lotes.py`, `perfiles.py`, `lectura_bitacoras.py`, `extension/` | |
| 8.1 | `META4_Bitacoras/scripts/` y `LEEME.md` | |

## 5. Diferencias aceptadas con el Python

- **Fechas en texto libre.** Python acepta cualquier formato que entienda pandas; Rust solo los cuatro listados en
  la sección 2. Lo demás es «ilegible» y genera incidencia.
- **Filas de totales.** Python toma como registro cualquier fila con un dato en una columna reconocida; Rust exige
  dos. Evita que «Total: 2» bajo RIT se analice como causa.
- **Etapa deducida.** Rust deduce la etapa por el nombre de la hoja, por ser la única compatible o por el nombre
  del archivo; Python la pide siempre.
- **Hoja de cruce.** Rust la detecta sola si hay exactamente una hoja con las seis columnas; no hay selector.
- **Época 1904 de Excel:** no soportada (RUS no la usa).

## 6. Bitácora de avance

- 2026-11-12: creado el plan. Motor Python leído completo.
- 2026-11-12: `flow-dominio` y `flow-motor` completos. Oráculo con 2.713 casos ficticios (900 al azar + dirigidos por
  etapa): Rust responde idéntico a Python en observación, reglas, acciones, avisos y marcas. Cubre 58 de 61 textos;
  los otros 3 no los usa el motor (`COMUN.REITERA_OBSERVACION` es CT-07 y se aplica fuera del motor: revisar
  `src/nurus/personal/bitacora_avisos.py` al hacer el paso 3.3; `C09_BASE_BREVE` y `HISTORIA.VARIOS` no se alcanzan).
  Se comprobó que la prueba detecta una mutación de texto (13 casos fallan al cambiar «Asimismo»).
- 2026-11-12: `flow-tabular` (BIFF, OOXML, XML 2003, HTML; encabezado desplazado; cruce), `flow-trabajo`
  (pre-pase + análisis de la tabla) y ejecutable `flow observar`. Prueba de punta a punta con los libros ficticios de
  `docs/evidencia_p0/golden`: mismas observaciones, reglas, acciones y avisos que `semantic.json`. CI en
  `.github/workflows/rust.yml` (no se ha visto correr todavía: revisar el primer resultado en GitHub Actions).
  Pendiente de 3.3: CT-07 (`COMUN.REITERA_OBSERVACION`), que necesita bitácora y se hace con la captura (fase 7).
- 2026-11-12: `flow-productos` (Excel de observaciones, nóminas, borradores con CC fijo y salida `.eml` con
  `X-Unsent: 1`, Word con un proyecto por página) y `flow-outlook` (COM por IDispatch: crear, llenar, adjuntar y
  guardar; verificado solo con `cargo check --target x86_64-pc-windows-gnu`, **nunca ejecutado contra Outlook real**:
  es lo primero que debe probar el usuario en Windows). Comando `flow producir`. Word validado con python-docx y el
  `.eml` con el módulo `email` de Python.
  Decisiones: NOMENCLATURA se deduce de la regla (C10_ESCRITO o P_FINALIZADA); Tomé no trae plantillas Word (el
  usuario las deja en su carpeta de datos como `TOME_PC_IE.docx`); un documento Word por tribunal y tipo.
  No portado del Python (decidir con el usuario si hace falta): firma de Outlook preservada, elección de cuenta,
  cuerpo HTML, edición libre del texto del proyecto, parches de matriz, diario de recibos.
- 2026-11-12: almacén local (`flow-trabajo::almacen`, retención 60 días / 10 sesiones) y ventana egui
  (`crates/flow/src/ui.rs`): Trabajo (tabla, filtros, edición, revisada), Correos (modalidades, borradores editables,
  guardar en Outlook o `.eml`, aprende contactos), Resoluciones (completar faltantes, generar Word), Resultados y
  Contactos. Sin argumentos abre la ventana; con comando, consola. Probada en Linux con `xvfb-run` y clics de
  `xdotool` (capturas con datos ficticios en `rust/docs/capturas/`). `cargo check --target x86_64-pc-windows-gnu`
  pasa para todo el workspace. **No probado en Windows real.**
  Para probar la ventana en Linux hacen falta `libxkbcommon-x11-0`, `libegl1` y `libgl1-mesa-dri`.
- 2026-11-12: CI corregida (el `.gitignore` raíz excluía `.docx`/`.xls`/`.zip`; `rust/.gitignore` los readmite solo
  en carpetas de plantillas y datos de prueba). `flow.exe` compilado en Linux con mingw (10,9 MB, solo DLL del
  sistema) y ejecutado bajo Wine: consola y ventana funcionan con datos ficticios. Entregado al usuario.
- 2026-11-12: importación del paquete ZIP del Descargador, con varias etapas por archivo (botones «Ver ETAPA»).
  Mismas observaciones, reglas, acciones y marcas que el Asistente en el paquete ficticio con historia.
