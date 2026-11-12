# Flow en Rust — plan de ejecución y estado

**Si eres otra IA o persona que retoma: lee este archivo entero y sigue desde el foco activo indicado abajo.**
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

### Punto de continuación (12-nov-2026)

- Revisión de las tres ramas: `informe/migracion-rust` (`86fba39`) es antecesora de `plan/proyecto-rust`
  (`ef97158`), y ambas están contenidas en `rust/producto`. No hay trabajo divergente que integrar.
- Último avance antes de retomar: `aa455a2`, puente con Chrome (7.1).
- **Foco activo: 8.1, Meta 4.** Fase 6.1 terminada: ZIP/CSMP_CAPTURE, carpetas y sesión común, informe,
  consola y ventana con progreso/cancelación. Continuar con `META4_Bitacoras/scripts/` y `LEEME.md`.
  Siguen exportación ZIP nativa de Captura y 8.2 (manual, guía, instalador y cierre). El producto completo sigue abierto.
- Instrucción actual del usuario: continuar sobre la misma parte y sin ramas nuevas. Trabajar directamente en
  `rust/producto`; la convención antigua de una rama por fase del plan general queda sustituida para este trabajo.
- Objetivo ampliado vigente: terminar todos los pasos y pruebas y dejar un producto listo para usar. Tras 7.4,
  cerrar 5b.2/5b.3, 6.1, 8.1 y 8.2; no confundir un paso terminado con la entrega final del producto.

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
    flow-captura          puente local, validación, lotes, checkpoints, PDF y reanudación
    flow-anonimo          anonimizador (fase posterior)
    flow-meta4            Meta 4 (fase posterior)
    flow                  ejecutable: subcomandos y ventana (egui)
  extension/              extensión Chrome MV3 de Flow (solo consultas)
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
- [x] 5b.2 Avisos de bitácora CT-05, CT-07 (con texto `COMUN.REITERA_OBSERVACION`) y CT-11
      (`src/nurus/personal/bitacora_avisos.py`; necesita la cronología de `capture/temporal.py` y una razón de
      similitud como `difflib.SequenceMatcher`).
- [x] 5b.3 Avisos N-03/N-04 (`contexto_excel.marcar_paquete`), OB-01..05 (`ordenes_busqueda.py`), cotejo y
      proyecciones del calendario por ingreso (`calendario_ingreso.py`), registros EGRESADOS (hoy se omiten).

### Fase 6 — Anonimizador
- [x] 6.1 `flow-anonimo` + `flow anonimizar`, modos y verificación «falla cerrado» del Anonimizador Python.
  - [x] 6.1a Núcleo local: texto, JSON/JSONL anidado, CSV/TSV y Excel BIFF/OOXML/XML 2003/HTML con salida XLSX;
        seudónimos, fechas, clasificación de columnas y revisión antes de publicar sin sobrescribir.
  - [x] 6.1b.1 HTML independiente, ICS y DOCX; dimensiones/celdas declaradas de XLSX antes de calamine y tamaño
        de identidades antes de construir sus índices. Pruebas de documentos y entradas adversas.
  - [x] 6.1b.2 ZIP genéricos y CSMP_CAPTURE, carpetas y sesión común entre archivos; rehacer manifiestos y
        huellas, relaciones y metadatos; exclusión/rechazo e informe. Validar las referencias entre todas las
        tablas: `parent_event_id` debe usar el mismo seudónimo que `event_id`, además de las identidades remotas
        presentes en `filters_json`, JSON normalizado y consolidado. Nunca copiar miembros sin tratarlos.
  - [x] 6.1c Subcomando, ventana con trabajo en hilo, avance/cancelación e instrucciones de revisión manual.
        Los formatos aún no portados deben seguir rechazándose; no copiar miembros sin tratarlos.

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
- [x] 7.2 `flow-captura::sitfa`: validación de respuestas y paginación (puntos 5). Fuente: `motor.py`,
      `perfiles.py`, `filtros_respuesta.py`. Probar con HTML del SITFA ficticio
      (`descargador_lite/tests/sitfa_ficticio.py` lo genera; muestras en `tests/datos/sitfa_ficticio.json`).
      Incluye cotejo XLS/HTML con multiplicidad, rechazo de páginas/filas repetidas y verificación del mes del
      calendario. Conectada al lote y a la ventana en 7.3.
- [x] 7.3 Lote, extensión y ventana: copiar `descargador_lite/extension/` a `rust/extension/` cambiando nombre y
      textos (y `tipo`/`version` de `capabilities`, que el programa verifica); pestaña «Captura» con código de
      conexión, opciones y avance; lo descargado pasa directo a Trabajo. Probar de punta a punta con Playwright +
      Chromium (están en este contenedor) contra el SITFA ficticio.
- [x] 7.4 Bitácoras, historia de la causa y calendario (`descargador/lectura_bitacoras.py`,
      `lectura_historia.py`, `extension/pagina.js` del Descargador integral, `src/nurus/bitacora_html.py`,
      `historia_html.py`, `calendario_html.py`). Ritmo y límite de 600 lecturas por lote (decisión m).
  - [x] 7.4a Lectores locales de las tres ventanas, identidad de bitácora/calendario y ventana de 1 a 4 meses;
        oráculo de ventanas ficticias contra Python y pruebas negativas.
  - [x] 7.4b Extracción y cotejo de los enlaces del listado actual, adaptación de los comandos de lectura
        `diary_open`, `history_open`, `calendar_open` y negociación de sus protocolos; sin persistir valores cifrados.
  - [x] 7.4c Integración con lotes, checkpoints, reutilización de lecturas del día, tope de 600, controles de período
        y paso de los contextos verificados a Trabajo; pruebas de extremo a extremo de las lecturas.

Prohibido en toda la fase: leer cookies o contraseñas, enviar formularios que no sean las consultas listadas,
guardar el código de conexión, y cualquier escritura en RUS.

### Fase 8 — Meta 4 y cierre
- [x] 8.1 `flow-meta4`: cálculo, cinco productos, consola y pestaña (sobre lotes de Flow).
- [x] 8.2 Manual (`LEEME.md`) y guía de pruebas (`GUIA_PRUEBAS_WINDOWS.md`, 34 pruebas). Sin instalador: un
      `flow.exe` y la carpeta `extension`.
- [ ] 8.3 Meta 4: «Regenerar desde Registros revisado» y plantilla propia (`meta4_bitacoras.py::regenerar`,
      `generate_meta4.py::load_template`); leer también lotes del Descargador Python si el usuario lo pide.
- [ ] 8.4 Prueba del usuario en Windows real y su resultado. Nada está aceptado hasta entonces.

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
- 2026-11-12: retomada la captura desde `aa455a2`. Paso 7.2: perfiles de las cuatro pantallas y cuatro pestañas,
  parámetros de consulta en lista cerrada, formulario/destino/filtros, selector dinámico auditado (sin ejecutar
  JavaScript), paginación de hasta 500 páginas, exportación limitada al sitio y cotejo de registros del XLS.
  El calendario exige fechas del mes/año consultados; las descargas repetidas se detectan por huella y por filas
  completas, conservando las distintas medidas de una misma persona.
  Oráculo reproducible: `python3 rust/oraculo/generar_sitfa.py` (14 respuestas ficticias, cotejadas con Python,
  RIT del año reservado 2099). Regresiones en `crates/flow-captura/tests/sitfa.rs` (17 pruebas nuevas).
  Verificación: suite anterior de `aa455a2` reproducida desde un archivo temporal del commit: 36 pruebas;
  suite completa actual: 53 pruebas, sin fallos. Clippy de `flow-captura` y comprobación del workspace para
  `x86_64-pc-windows-gnu`; controles de identidades y contactos sobre los cambios. No se ha probado contra SITFA
  ni en Windows real; la prueba nativa Windows corresponde a la CI y el uso real sigue pendiente del usuario.
  Siguiente: 7.3; usar `Perfil::desde_catalogo`, `validar_respuesta`, `siguiente_consulta` y
  `VerificadorDescargas` antes de guardar o importar cada descarga. La negociación de capacidades
  (`escritura_rus:false` y versiones/protocolos exactos), el lote, los checkpoints, la extensión y la interfaz
  aún corresponden a 7.3. El estado de `VerificadorDescargas` debe durar toda una consulta y reiniciarse al cambiarla.
- 2026-11-12: 7.3 completo: pestaña Captura con conexión, carga de opciones por pantalla, selección múltiple,
  fechas, filtros, descarga, cancelación y Retomar. Las llamadas a Chrome corren en un hilo; la interfaz sigue
  respondiendo. Negociación de versión/protocolos y `escritura_rus:false` obligatoria antes de consultar.
  Cada página se coteja antes de guardarse sin sobrescribir; `resumen.json` conserva el avance. Al reanudar se
  verifican las huellas, se cotejan las páginas parciales con la respuesta actual y se reutilizan las válidas.
  PDF de consulta vacía desde PNG, sin imágenes intermedias; se acredita la selección visible y su huella.
  Los Excel de intervención pasan a Trabajo con la etapa de la consulta, agrupando páginas; se comprueba de
  nuevo la huella antes de analizar los mismos bytes. Litigantes y calendarios quedan como archivos de consulta.
  Pruebas nuevas: 6 del lote y 2 del pase a Trabajo; más dos recorridos con el puente Rust y la extensión MV3
  real en Chromium contra páginas ficticias (dos páginas XLS y consulta vacía con PDF). La prueba pulsa la acción
  de la extensión mediante CDP `Extensions.triggerAction` sobre el target de tipo `tab`, sin ampliar permisos:
  así Chrome concede `activeTab` igual que al pulsar el icono. No se contacta con SITFA real.
  Reproducir: `cd rust/pruebas && npm ci && npx playwright install chromium`, luego desde `rust/`:
  `cargo run --locked -p flow-captura --example probar_captura` y el mismo comando con `-- --vacia`.
  CI Linux ejecuta ambos; el artefacto Windows incluye la extensión. El commit anterior, `f8f78bf`, pasó pruebas
  y compilación Windows nativas en [run 43252413001](https://github.com/MatiGaete2023/flow/actions/runs/43252413001).
  El uso real en SITFA y Outlook sigue sin ejecutarse. Próximo paso de desarrollo: 7.4.
- 2026-11-12: 7.4a: `flow-captura::lecturas` porta los lectores de `bitacora_html.py`, `historia_html.py` y
  `calendario_html.py`. La bitácora vincula los textos completos con el ID de observación, coteja la identidad
  remota y conserva respuesta, fecha, estado y metadatos de envío. Mantiene cobertura PARCIAL: una ventana no
  acredita por sí sola todo el historial remoto. Historia lee solo la tabla visible, conserva fechas ilegibles
  con advertencia y detecta posible paginación; nunca sigue enlaces de documentos. El calendario coteja tribunal,
  causa e ingreso y exige coherencia entre ENVIADO/PENDIENTE y fecha del informe. Límite local de 8 MiB por ventana;
  UTF-8/Windows-1252 y alias auditados, con error ante bytes inválidos. Una respuesta cortada o de acceso no se
  convierte en una lista vacía. Ventana de meses naturales (1–4) con fechas futuras y sin fecha separadas.
  Oráculo reproducible: `python3 rust/oraculo/generar_lecturas.py` (8 ventanas ficticias; mismas salidas que Python).
  Siete pruebas nuevas, suite completa 68 PASA, Clippy y comprobación cruzada Windows PASA. El paso 7.4 sigue
  abierto hasta integrar protocolo/lote/UI: continuar en 7.4b desde `descargador/vinculos.py` y los tres comandos
  de `descargador/extension/pagina.js`. CI nativa y Chromium del avance 7.3: run 43259160734 PASA.
- 2026-11-12: 7.4b: `flow-captura::vinculos` conserva todas las filas del listado, extrae solo argumentos
  literales de acciones directas y coteja causa, ingreso, tribunal, RIT y calendario. Nunca obtiene identidad de
  comentarios, cadenas, plantillas, regex ni expresiones ejecutables. Las ambigüedades se marcan; no se deducen IDs.
  `abrir_bitacora`, `abrir_historia` y `abrir_calendario` exigen capacidades de solo lectura, parámetros idénticos,
  huella HTML y fecha de captura. Historia coteja además el RIT y tribunal visibles. Extensión 0.2.0 con protocolos
  de lectura explícitos; los valores cifrados de la apertura de Historia quedan en Chrome y una reflexión de esos
  valores en el HTML se rechaza antes de devolverlo. Se conservan cuatro pantallas de consulta (sin agregar Carga).
  Oráculo: `python3 rust/oraculo/generar_vinculos.py` (9 listados ficticios); 6 pruebas Rust nuevas. Suite completa:
  74 PASA. Pruebas JS de bitácora, historia y calendario, Clippy, compilación cruzada Windows y dos recorridos MV3
  (descarga y PDF vacío) PASA. Los comandos quedan disponibles, pero la fase sigue abierta hasta 7.4c: invocarlos
  dentro de cada página vigente, con ritmo y checkpoints, y pasar solamente contextos verificados al análisis.

- 2026-11-12: 7.4c: lecturas durante cada página vigente del lote; selección independiente en Captura y período
  de 1–4 meses naturales. Pausa cancelable antes de cada lectura; límite compartido de 600 bitácoras/calendarios
  y otro de 600 historias por corrida, como en Python. Guarda JSON estructurado con SHA-256 y vínculo, sin HTML
  ni valores cifrados. Reutiliza lecturas verificadas del mismo día entre los diez lotes hermanos más recientes;
  bitácora/historia requieren el mismo período. Los fallos parciales conservan Excel y lecturas y dejan la consulta
  pendiente; Retomar coteja páginas y reintenta lo que falta. Trabajo verifica huellas, vincula por los cuatro
  campos completos dentro de su consulta y reúne páginas antes del pre-pase; historia, bitácora y próximo informe
  llegan al motor vigente. No atribuye contextos de otra persona por compartir RIT o RUT.
  Pruebas nuevas: límite de lecturas, cuatro de persistencia/reutilización/reanudación y dos de importación,
  identidad y archivos alterados. Suite completa: 81 PASA. Recorrido MV3 real con SITFA ficticio: dos Excel y
  18 lecturas verificadas (seis de cada tipo), filtros restaurados; nuevo escenario `probar_captura -- --lecturas`
  incluido en CI. Clippy sin advertencias y comprobación cruzada Windows PASA; CI nativa del paso anterior
  `1b7950b` PASA (run 43263570103). Continúa en 5b.2; este paso no implementa todavía los avisos CT/N/OB ni Meta 4/anonimizador.

- 2026-11-12: CI de `66c0cf2`: Linux y los tres recorridos MV3 PASA (run 43268712430). Windows detectó una
  colisión de carpetas temporales entre pruebas paralelas: `SystemTime` tiene menor precisión allí. Las tres
  fábricas nuevas de carpetas de prueba incorporan un contador atómico, además de proceso y reloj. No se altera
  el flujo del producto ni se evita la ejecución paralela de pruebas.

- 2026-11-12: 5b.2: `flow-trabajo::cronologia` porta la vista CT-01/02 de cuatro meses naturales y los avisos
  CT-05, CT-07 y CT-11. Respeta origen acreditado, cobertura, corte y empates con hora/zona inciertas; distingue
  no solicitada, no leída, origen desconocido y sin entradas capturadas. La similitud reproduce
  `difflib.SequenceMatcher(autojunk=False)`, sin ignorar caracteres repetidos. CT-07 aplica literalmente
  `COMUN.REITERA_OBSERVACION`, conserva el prefijo y las acciones y guarda el texto previo en el aviso; se puede
  desactivar con la misma regla. Integrado tanto en ZIP CSMP_CAPTURE como en capturas nativas, con identidad
  verificada. La cronología persiste en las sesiones; el detalle muestra observaciones, respuestas, posteriores,
  empates y avisos. Los avisos CT no generan acciones nuevas.
  Oráculo reproducible: `python3 rust/oraculo/generar_cronologia.py`: 164 cronologías (incluidos los límites
  29/30 días y similitud 0,91/0,92), 256 comparaciones y un ZIP ficticio cotejado con Python. Cuatro pruebas nuevas
  entre diferencial e integración; suite completa 85 PASA, Clippy y comprobación cruzada Windows PASA. CI del
  arreglo `0d2fdff`: Linux/MV3 y pruebas Windows PASA; compilación release aún en curso al escribir esta nota.
  Siguiente: 5b.3, sin considerar terminados anonimización, Meta 4 ni entrega.

- 2026-11-12: 5b.3: controles por identidad remota verificada N-03/N-04 y Q-03; órdenes de búsqueda activas
  OB-01..05 conservando columnas ausentes frente a vacías; calendario proyectado por ingreso y cotejo CAL-03.
  Hoja2 conserva la prioridad; el calendario del ingreso aporta la próxima PENDIENTE solo si Hoja2 no aporta
  fecha (sin aplicar a esa proyección el límite de 60 días del listado calendario 91), igual que Python.
  Las proyecciones se conservan y muestran en el detalle; no alteran la fecha original de Informes ni duplican
  observaciones. En lotes nativos el calendario 91 se cruza por selección de tribunal e identidad completa y
  avisa si no trae las columnas necesarias. Solo una fecha inequívoca aporta contexto.
  EGRESADOS y Búsqueda Lit se conservan como trabajos auxiliares con fuente explícita, columnas originales y
  avisos. No ejecutan el motor tradicional ni generan acciones. Internamente mantienen la etapa de lectura
  ESPERA, como el importador Python, y `fuente`/`nombre()` identifican su tipo real en la ventana, Excel y sesión;
  no se modificó el enum ni las reglas del motor. Las sesiones de auxiliares ya no pueden sobrescribir las de
  Espera del mismo ZIP. La apertura local detecta Búsqueda Lit por su columna de estado y Egresados por nombre.
  Oráculo: `python3 rust/oraculo/generar_controles.py`: 94 órdenes, 100 calendarios y un ZIP ficticio con cinco
  registros y cuatro tipos de fuente, cotejados con Python. Cinco pruebas nuevas entre diferencial, importación
  nativa y consulta local; suite completa 90 PASA. Clippy y comprobación cruzada Windows PASA. La CI completa del
  avance anterior `c2dc05b`, incluidas las pruebas y el ejecutable nativo Windows, PASA (run 43270528975).
  Siguiente: 6.1; no considerar terminados Anonimizador, Meta 4 ni la entrega del producto.

- 2026-11-12: 6.1a: nuevo `flow-anonimo`, aún sin integrar al ejecutable. Dos pasadas, modo Estricto (omite texto
  libre) y Conservador (reemplaza patrones/nombres y advierte revisión), seudónimos de sesión y semilla HMAC en RAM.
  JSON anidado y JSONL, CSV/TSV con delimitador y neutralización de fórmulas, TXT/MD/LOG, XLS BIFF/OOXML/XML 2003/HTML
  reconstruidos como XLSX solo con valores. Preserva fechas y horas, desplazamiento y generalización de nacimiento.
  RUT ficticios con DV inválido; RIT del año reservado 2099, conservando su letra porque el motor distingue causas
  P. Evita RIT ya presentes. Revisión final de nombres e identificadores antes de devolver una salida publicable;
  nombres y rutas de archivos nuevos independientes del original, publicación exclusiva con temporal/enlace físico.
  Código/señales de credenciales etiquetadas se omiten también en Conservador. Ninguna red ni modificación de
  originales. Archivos/entradas ZIP con rutas inseguras, cifrado, enlaces, expansión excesiva o DTD/ENTITY se rechazan.
  Límites actuales: 80 MiB por archivo, dos millones de celdas/nodos; identidades no latinas o demasiado cortas
  fallan cerrado y deben mantenerse así hasta implementar y probar su tratamiento. El Conservador no acredita
  anonimización absoluta: falta su presentación al usuario y la revisión manual documentada.
  `python3 rust/oraculo/generar_anonimo.py` reproduce vocabulario y 187 clasificaciones iguales a Python.
  Doce pruebas nuevas cubren relaciones, nombres entre archivos, RUT/DV, RIT/telefonía, JSON anidado, fechas/horas,
  las tres variantes XLS, conservación de columnas útiles, publicación sin sobrescritura y entradas rechazadas.
  Suite completa: 102 PASA; Clippy del crate y comprobación cruzada Windows PASA. CI del avance 5b.3 `8abdaf0`
  completa en Linux/MV3/Windows: run 43273832868 PASA. La fase 6.1 sigue abierta: completar 6.1b y 6.1c.

- 2026-11-12: 6.1b.1: HTML independiente reconstruido solo como párrafos/tablas, sin atributos, scripts, imágenes
  ni vínculos. ICS conserva fechas, horas, zonas IANA y recurrencia con gramática cerrada; cambia UID, trata los
  textos y excluye alarmas, asistentes, organizadores y propiedades no admitidas. No conserva parámetros libres.
  Word reúne párrafos entre runs y lee también encabezados, pies, notas y comentarios; reconstruye un DOCX mínimo
  de texto, sin formato, relaciones ni metadatos del original. Rechaza imágenes, objetos y macros incrustados.
  Antes de abrir XLSX, revisa dimensiones, coordenadas y conteos XML para evitar reservas desmesuradas. Identidades
  de más de 512 bytes o 64 palabras fallan cerrado; HTML con profundidad >128 o más de 200.000 elementos se rechaza.
  Cinco pruebas nuevas: HTML sin contenido activo, ICS con zona/hora/pliegues, Word con metadatos y texto partido,
  coordenadas/conteos Excel adversos y estructuras/identidades excesivas. Suite completa 107 PASA; Clippy y
  comprobación cruzada Windows PASA. CI anterior `c937fbd`: Linux/MV3 y Windows PASA (run 43283361127).
  Continúa en 6.1b.2; no están portados todavía ZIP genérico, CSMP_CAPTURE, carpetas, consola ni ventana.

- 2026-11-12: 6.1b.2/6.1c: ZIP genéricos y CSMP_CAPTURE reconstruidos sin copiar archivos sin tratar,
  huellas nuevas y claves foráneas coherentes (incluido evento padre). Tres paquetes sintéticos pasan el
  contrato estricto Python; prueba adicional de documento/evento. Carpetas en dos pasadas con sesión común,
  nombres neutros, límites, exclusión/rechazo, informe sin identidades ni semilla, cancelación y limpieza de
  salidas propias. Consola `anonimizar` y pestaña con hilo de trabajo; semilla solo en RAM. Preserva coordenadas
  de celdas Excel. Suite completa: 114 pruebas, más una prueba focal de coordenadas Excel (115 en total); comprobación Windows cruzada y clippy focal del anonimizador.
  CI del avance anterior `d9faef6`: Linux y Windows aprobados (43287539795). META4 y cierre siguen pendientes.

- 2026-11-12: 8.1 Meta 4. Crate `flow-meta4` sobre los lotes de captura de Flow (`resumen.json`, XLS verificados,
  vínculos y lecturas de bitácora): período de mes calendario móvil, última observación del centro de etapa
  Espera/Cumplimiento (Informes no participa), empate → revisar, alerta roja, controles (verificada, sin observación,
  lectura no completada, no verificable, leída antes del corte, identidad/nombre visible, ambigua, sin bitácora),
  filtro de tribunales con `- parcial`, consulta repetida entre lotes rechazada, publicación atómica en carpeta
  nueva. Salidas: planilla de cuatro hojas con bloques por tribunal y hoja de control, Registros, consolidado con
  resumen por tribunal/duplicados/alertas, bitácoras del mes y reporte JSON sin datos de personas.
  Diferencias deliberadas con el Python: no acepta el alias «LA» en una celda; no lee lotes del Descargador Python,
  PDF de consulta vacía ni carpetas de reportes sueltos; sin regenerar ni plantilla (8.3). Paridad **por reglas**
  (mismos ejemplos del LEEME de Meta 4), no por comparación de archivos. Suite completa: 39 conjuntos PASA.
- 2026-11-12: prueba del usuario en el Descargador Python: «Calendario de informes no leído: fecha ilegible en
  Fecha Informe (forma - - -)». Causa: SITFA escribe la fecha vacía con guiones separados y el lector solo aceptaba
  `-`, `--` y `---`. Corregido en `src/nurus/calendario_html.py` (rama `Rediseno`, commit 38fa03d) y en
  `flow-captura::lecturas::fecha_calendario`, con prueba en ambos.
  El usuario también informa que los paquetes grandes no leen todas las bitácoras. Causa: tope de 600 lecturas por
  corrida, en el que cada calendario cuenta igual que una bitácora (decisión del 19-oct, con prueba propia
  `test_calendario_cuenta_en_el_maximo_de_lecturas`); con calendario activo alcanza para 300 ingresos y hay que
  «Retomar». **No se cambió**: es decisión del usuario (subir el tope, contar el calendario aparte o seguir retomando).
