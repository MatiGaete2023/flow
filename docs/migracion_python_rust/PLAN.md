# Plan · Python → Flow Rust

Vigente desde el 24 de noviembre de 2026. Reemplaza al [plan del 20 de noviembre](historico/PLAN_20261120.md) y a los [encargos](historico/ENCARGOS_IA.md). El estado de cada capacidad está en [ESTADO.md](ESTADO.md).

## Objetivo y corte

- Flow Rust reemplaza a Flow Python cuando el ciclo mensual completo corre en `flow_rust.exe` y el usuario lo deja en un acta.
- Arquitectura (D-5): **el Descargador recolecta toda la información que Flow necesita y Flow la procesa.** El contrato entre ambos es el paquete `CSMP_CAPTURE`.
- Orden (D-7): Descargador, exportaciones de lote, Asistente. Después, Meta 4, Anonimizador y distribución.
- Cada tramo termina con una prueba corta del usuario antes de abrir el siguiente.

## Cómo se trabaja un encargo

1. Leer el módulo Python de origen en la referencia fijada y, si existe, el crate histórico de `rust/crates`.
2. Implementar en `app/` sin cambiar reglas, umbrales ni textos.
3. Acreditar con un oráculo: un generador en `app/qa/` que graba la salida de Python sobre datos ficticios, y una prueba Rust que la compara y afirma sus conteos.
4. Probar según el presupuesto de `AGENTS.md`.
5. En el mismo commit: actualizar [ESTADO.md](ESTADO.md) y agregar una entrada corta al [registro](REGISTRO_IMPLEMENTACION.md).

Estados de un encargo: `ABIERTO` → `HECHO` (con el commit y la CI en verde) → `ACEPTADO` (solo el usuario).

Qué se compara con Python:
- **Exacto:** observaciones, reglas, marcas, Excel, Word, correos y el contenido del paquete.
- **Por contenido, no byte a byte:** manifiestos y archivos internos del lote que solo lee Flow Rust.

## Tramo 0 · Base de verificación

- **E-V1 · Referencia Python única — `HECHO` en Linux; falta su CI de Windows.** Los árboles `descargador/`, `src/` y `tests/` de esta rama son los de `afc4d12`; `app/qa/REFERENCIA_PYTHON.json` fija el SHA, la CI hace checkout de ese commit y `tools/verificar_referencia_python.py` lo comprueba. Al regenerar cambiaron solo `bitacoras.json` y `paquete.json` (RIT ficticio del histórico); el contraste V2 pasa contra la referencia.
  - Revisar el diff de `descargador/flujo_paquete.py` y `src/nurus/personal/revision_bitacoras.py` entre esta rama y `flow-python` (`afc4d12`).
  - Fijar el SHA de referencia en `app/qa/REFERENCIA_PYTHON.txt`; generadores y CI usan ese checkout.
  - Regenerar `app/tests/datos/paquete.json` y el contraste V2. Criterio: los oráculos pasan contra la referencia fijada, o cada diferencia queda declarada en [ESTADO.md](ESTADO.md).
- **E-V2 · Pruebas sin variables de entorno — `HECHO` en Linux.** `prueba_entorno::requerida` da rutas por omisión del repositorio; `x11` de eframe declarado para Linux en `app/Cargo.toml`; diez pruebas dejaron de estar ignoradas.
  - Rutas por omisión relativas a `CARGO_MANIFEST_DIR` para `FLOW_CASOS`, `FLOW_FIXTURES` y `FLOW_REFERENCIA`. Quedan `#[ignore]` solo las pruebas que necesitan un ZIP generado por Python.
  - Declarar en `app/Cargo.toml` la característica `x11` de eframe para Linux. Criterio: `cargo test --manifest-path app/Cargo.toml --locked` corre los oráculos en Linux sin preparación.
- **E-V3 · CI por niveles — `HECHO`; falta ver su primer run.** `flow-rust-local.yml`: trabajo `linux` con todas las pruebas y los verificadores en cada push; `validar` (Windows) compila, prueba, contrasta V2 y E3, y regenera los oráculos solo si cambian `app/qa/**`, `app/tests/datos/**` o el árbol Python; con `workflow_dispatch` y `entrega`, el `.exe --release`, las capturas y el contraste V1. Sin `entrega`, el artefacto `Flow_Rust_Windows_exe` lleva el `.exe` de depuración.
  - Trabajo en Linux con las pruebas en cada push.
  - Windows: compilar el `.exe`. Oráculos regenerados solo si cambian `app/qa/**`, `app/tests/datos/**` o la referencia. Ejecutable de entrega, capturas y contraste V1 con `rust/`, a pedido.

## Tramo 1 · Descargador

- **E-Q5d · Excel del lote y XLSX consolidado — `HECHO` en Linux (`c734f84`); falta ver su CI de Windows.**
  - `descargador/excel.rs` y `descargador/resultados.rs`, con el oráculo `app/qa/generar_excel.py`.
  - `include_originals` no se porta: ninguna ruta de Python lo activa.
- **E-Q7 · Avance por fila de la lectura — `HECHO` en Linux.** `bitacoras::ejecutar_con_avance` emite `query`, `diary_progress` y `warning` como `DiaryRunner`; la sección «Bitácoras y paquete» muestra las filas revisadas, el estado y los avisos.
- **E-U1 · Intercambio entre herramientas (F27) — `HECHO` con la opción B (D-9), CI de Windows en verde (run 45432865102).** «Abrir en Trabajo» junto al paquete recién creado en el Descargador abre el ZIP en Trabajo (guarda antes la sesión pendiente). No hay archivos de solicitud ni de respuesta: el contrato es el paquete (D-5), y el Asistente sigue sin navegar SITFA ni iniciar el Descargador. `capture/temporal.py` no era parte de F27: su cronología ya está portada.
- **Prueba del usuario:** la de [ESTADO.md](ESTADO.md).

## Tramo 2 · Exportaciones de lote

- **E-S1 · Exportaciones y comparador — `HECHO` en Linux; falta su CI de Windows.**
  - `descargador/resultados.rs`: `export_book`, `export_csv`, `export_ics`, `export_zip` y `compare_lots`, con el oráculo `app/qa/generar_resultados.py`.
  - Vista: sección «Resultados del lote» (`descargador_resultados_ui.rs`) en el Descargador; se abre sola con el lote recién descargado. Pruebas `ui_resultados_abre_un_lote_y_crea_sus_productos` y `resultados_cruza_compara_y_rechaza_como_python`.

- **E-W1 · Producto de Trabajo sin consolidado — `HECHO` (CI de Windows en verde, run 45420239558).** Pedido del usuario tras probar un paquete de Cumplimiento (24-nov): el Excel salía de una hoja con 28 columnas, sin identificadores SITFA ni columnas de revisión.
  - Trabajo conserva todas las columnas de `normalized_json` (`captura_trabajo::hoja`).
  - `producto_excel.rs`: libro nuevo con `NURUS_LISTAS`, `NURUS_TRAZABILIDAD`, `Resumen`, `SITFA_ARCHIVOS`, la hoja de datos y `Vencimientos`; revisión en el orden del 20-nov y `NURUS_CLAVE_INGRESO` (tribunal-causa-ingreso, estable entre capturas; el `record_id` cambia en cada descarga).
  - El paquete deja `CONSOLIDADO_OMITIDO` cuando se pidió el consolidado y no se pudo; Trabajo explica por qué no hay libro base.
  - «Avisos del paquete» agrupa las incidencias repetidas en Trabajo. El paquete conserva una fila por registro porque Flow Python las lee así.
  - Revisión de bitácoras: filtros «Sin bitácora» y «Con advertencia de lectura».
  - Pendiente: un lote conjunto de Cumplimiento e Informes sigue sin generar consolidado (decisión del usuario: queda fuera).

## Tramo 3 · Asistente

En este orden, salvo que el usuario indique otro:

- **E-A1 · Reimportación y conciliación desde el Excel revisado (F10) — `HECHO` (CI de Windows en verde, run 45429755148).** `sync.py`. `sincronizar.rs` y el botón «Actualizar desde Excel» de Trabajo; oráculo `app/qa/generar_sync.py` (12 casos). Diferencias: una observación vaciada en Excel no se importa; las columnas de origen editadas en Excel solo se avisan, no se recalculan; `NURUS_DECISIONES` no se combina.
- **E-A2 · Conciliación entre capturas (F07) — `HECHO` (CI de Windows en verde, run 45437385372).** `capture_reconciliation.py`. `conciliar.rs` y el diálogo «Conciliar capturas»; oráculo `app/qa/generar_conciliacion.py` (12 casos). Diferencias: concilia solo la etapa abierta; no hay decisiones de correo, anulaciones ni variables Word que trasladar; con los mismos bytes se continúa el trabajo anterior como en Python.
- **E-A3 · Recibos, vigencia y recuperación de sesiones (F09) — `HECHO` (CI de Windows en verde, run 45439994977).** `product_state.py`, `activity.py` y `session.py`. `preparados.rs`: huella de dependencia, huella de borrador, recibos y productos preparados guardados en la sesión. Prueba de comportamiento `un_producto_desactualizado_no_se_guarda_y_uno_repetido_tampoco`; sin oráculo de huellas porque no se compara el hash. Pendiente: estado «incierto» de un borrador de Outlook tras un fallo.
- **E-A4 · Ajustes (F11) — `HECHO` (CI de Windows en verde, run 45441868390).** `personalizar.rs` y sus pantallas: alias en Contactos, editor de plantillas en Ajustes (selector de variables, duplicar, archivar, restaurar) e «Importar matrices Word (ZIP)». Oráculo `app/qa/generar_personalizacion.py` (44 casos). Diferencias: las matrices quedan como `TRIBUNAL_TIPO.docx` en la carpeta de plantillas de Flow Rust (Python: `TRIBUNAL/TIPO.docx`); un Word dañado se rechaza con un mensaje propio; `install_bundled_templates` (revisión empaquetada) no se porta porque Flow Rust lleva sus matrices dentro del ejecutable.
- **E-T1 · Cuenta Outlook (F13) — `HECHO` (CI de Windows en verde, run 45443830208).** `outlook.rs`: cuenta elegible en Ajustes (dirección o nombre exactos y únicos), solo `Save()`, CC fijo, estado «incierto» tras un fallo posterior a guardar y botón «Ya lo revisé en Outlook». Pruebas de la salida simulada de PowerShell (antes y después de guardar); sin prueba con Outlook real.
- **E-T2 · Formularios manuales (F15) — `HECHO` (CI de Windows en verde, run 45445728806).** `manual.rs`, «Correo manual» en Correos y «Word manual» en Resoluciones; oráculo en `generar_personalizacion.py` (11 Word y 8 bloques; `manual.py` se carga con `tkinter` simulado). Diferencias: sin `improve_pagination`; el Word libre no usa los estilos de Word (título centrado en negrita); los adjuntos libres también viajan en el `.eml` y en Outlook.
- **E-T3 · Estadísticas y métricas (F16) — `HECHO` (CI de Windows en verde, run 45447605617).** `estadisticas.rs`: «Estadísticas de constancias» y «Exportar resumen (Excel)» en Resultados; métricas locales opcionales en Ajustes. Oráculo `app/qa/generar_estadisticas.py` (7 estadísticas y 7 métricas). Diferencias: las fechas y TT/CC se leen como texto (`FECHA_OBS`, `TT`, `CC` de Trabajo); un número de serie de Excel en `FECHA_OBS` no se interpreta como fecha.
- **E-A5 · Cotejos pendientes (F02, F08, F12) — `HECHO` (CI de Windows en verde, run 45449454854).** Correos: `generar_correos.py` (18 casos, 1 diferencia declarada). Órdenes de búsqueda: `ordenes.rs` y `generar_ordenes.py` (21 casos). Edición masiva y deshacer: `Trabajo::aplicar_masivo` y `deshacer` con su barra en Trabajo. Egresados quedó en E-A2. Pendiente: identidad multifila (sin brecha identificada) y las anulaciones de columnas de origen.
- **E-A6 · Importación de contactos (F11) — `HECHO` (commit `6bc52f3`, CI Windows verde, run 45503587810).** `personalizar::importar_contactos_archivo`, `importar_catastro` (`outputs.import_contacts`) y `parse_contactos` (`config.parse_contacts`); el botón «Importar» de Contactos las usa y avisa los conflictos. Oráculo `app/qa/generar_catastro.py` (24 catastros y 12 textos; el `.xlsx` se escribe sin fechas para que se regenere igual) y prueba `importacion_de_contactos_igual_a_python` (36 · 5 errores · 4 conflictos), más `oraculo_de_catastro_detecta_alteraciones`. Antes pisaba contactos sin avisar, aceptaba direcciones inválidas y no tenía pruebas. Diferencias: en CSV y HTML las filas «Tribunal: X» actualizan los destinatarios del tribunal; la rama JSON se prueba por comportamiento (en Python es código de la ventana).
- **E-A7 · Recálculo al editar columnas de origen en la copia (F10) — `HECHO` (commit `35f3a70`, CI Windows verde, run 45625544370).** `sincronizar::actualizar` recibe la configuración, toma de Excel los valores de origen cambiados, vuelve a analizar el trabajo (`Trabajo::analizar`) y repone lo que ese análisis no conserva (campos de Excel, identificador de copia y línea base); `Resumen.recalculadas` se muestra en Trabajo. Oráculo `generar_sync.py` ampliado a 17 casos (propuesta y reglas por fila; 5 con `T ESPERA` editado, uno con conflicto) y `oraculo_de_sync_detecta_alteraciones` también altera una regla recalculada. Diferencias: en un paquete se amplía en E-A9; se reanalizan todas las filas; el nombre del programa va en tipo título (R6). `NURUS_DECISIONES` sigue sin combinarse hasta que exista F08 (anulaciones y gestiones de correo).
- **E-A8 · Gestión de correo por fila (F08) y `NURUS_DECISIONES` (F10) — `HECHO` (commit `43a5f7d`, CI Windows verde, run 45634055296).** `Registro.gestiones_correo` (`include`/`omit` por tipo de correo) con su efecto en `productos::preparar_borradores`, selector «Gestión de correo de esta fila» en Trabajo, sesión, conciliación entre capturas y `NURUS_DECISIONES` (escritura y combinación en `sincronizar`, con línea base `sync_correo`). Oráculos: `generar_correos.py` ampliado a 24 casos (seis con `record_edits.apply`) y `generar_sync.py` a 25 (ocho de gestión: remota, local, mezcla con otros campos, conflicto, igual en ambos, en filas distintas y dos celdas inválidas); pruebas permanentes que alteran lo esperado en ambos; más pruebas de comportamiento (conflicto resuelto con cualquiera de los dos valores, sesión). Diferencias: solo se combina la gestión de correo de esa celda; la exclusión manual, las anulaciones de columnas de origen y las variables de Word por fila siguen sin existir (F08 parcial).
- **E-A9 · Recálculo al editar columnas de origen en un trabajo de paquete (F10) — `HECHO` (commit `7bdd696`, CI Windows verde, run 45639115067).** `sincronizar::actualizar` ya no se limita a archivos de RUS: escribe los valores de Excel en la hoja del paquete y vuelve a analizar con el contexto de la captura. Oráculo `app/qa/generar_sync_paquete.py` (6 casos: Python arma el paquete desde el lote de listados con tres columnas de origen, lo abre con `import_capture`, exporta la copia y refresca) y pruebas `actualizar_desde_excel_en_un_paquete_igual_a_python` y `oraculo_de_sync_en_paquete_detecta_alteraciones`. En el contraste, reglas y propuestas coinciden en los 6 casos (`C05` a `C04` a `C09`).
- **E-P1 · Proyectos Word: elegir el formato al crearlos (F14) — `ABIERTO`.** Decisión del usuario del 25-nov: la persona elige entre un `.docx` por proyecto (como hoy) y uno por tribunal. Los proyectos se separan con un índice al inicio y un salto de página entre ellos; falta su oráculo (Python usa `docxcompose` en `resolutions.py`).
- **E-I2 · Actualización en el lugar y extensión de carpeta fija (F26) — `HECHO` (commit `5f21612`, CI Windows verde, run 45644045890).** Pedido del usuario (25-nov): que una corrección no obligue a reinstalar todo, ni siquiera la extensión de Chrome. Decisiones del usuario: la fuente es una carpeta local de actualizaciones (sin red ni credenciales) y Flow avisa y aplica con un botón. `actualizacion.rs` (`buscar`, `aplicar`, `volver`, `reiniciar`; huella SHA-256 contra el `SHA256.txt` de la CI; restaura si algo falla) con aviso al abrir, sección «Actualizaciones» en Ajustes y la regla permanente en `AGENTS.md`. `extension::instalar_en_el_lugar`: carpeta fija por edición, escribe solo lo que cambió, quita sobrantes y devuelve `Nueva`, `Actualizada` o `SinCambios` (la interfaz dice qué hacer en cada caso). Pruebas: `la_actualizacion_reemplaza_el_programa_y_deja_el_anterior`, `una_actualizacion_igual_incompleta_o_ajena_no_se_aplica` y `la_extension_se_actualiza_en_el_lugar_sin_cambiar_de_carpeta`. Corrección en el camino: la huella del programa no se calcula en cada cuadro (congelaba la ventana). Limitaciones: el aplicador no se probó con Windows real ni con un `.exe` en `Program Files` (sin permiso se avisa); la primera actualización es manual.
- **E-U2 · Correcciones de la primera prueba del usuario con el `.exe` (Descargador y Meta 4) — `HECHO` en Linux; falta su CI de Windows y la confirmación del usuario.** (1) Las vistas Descargador y Meta 4 se dibujan dentro de un desplazamiento vertical general: antes, al abrir «Planificar lote», lo que quedaba bajo el borde de la ventana no se alcanzaba. (2) «Validar perfil» explica en pantalla que es una consulta única y que varios tribunales, pestañas o modalidades se eligen en «Planificar lote». (3) `cargar_lote` distingue una carpeta de conjunto (`flujo.json`), un lote de bitácoras (`bitacoras.json`) y una carpeta cualquiera, y dice cuál elegir; prueba `una_carpeta_sin_resumen_dice_que_es_y_que_elegir`. Pendiente: abrir conjuntos de Flow Python en Resultados (Python resuelve `flujo.json` a sus fases) si el lote antiguo del usuario resulta ser de ese tipo; el desplazamiento no tiene prueba automática (la interfaz la valida el usuario).
- **E-A10 · Anulaciones de columnas de origen por fila (F08) — `ABIERTO`.** Editar a mano un dato de origen de una fila en Trabajo y recalcular esa fila y las de su causa si cambia una duplicidad o un acumulado (`record_edits.apply(overrides=…)`, `_prepass_peers`), con su escritura en `NURUS_DECISIONES` y su combinación al sincronizar. Oráculo contra Python.
- **E-I1 · Instalador (F26) — `ABIERTO`.** Instalación por usuario con acceso en el menú, sobre el `.exe` que la CI ya entrega con su huella; debe respetar la carpeta de actualizaciones de E-I2. Faltan decidir la firma y dónde se aloja; se decide tras la prueba corta del `.exe` (ver [CONTINUACION.md](CONTINUACION.md)).
- **E-D10 y E-D11 · `REVERTIDOS` el 25-nov por decisión del usuario (commit `f9df427`, CI Windows verde, run 45637998137).** Se habían implementado y pasado la CI de Windows (run 45609310864, commit `a8a0338`): ausentes con y sin cobertura (D-10) y R11 sin redactar con varias órdenes en una causa compartida (D-11). El usuario eligió volver a Python en ambos: Rust rotula «egresadas» toda fila ausente y redacta R11 como Python. Los riesgos R-D y R-A del [análisis](ANALISIS_INFORME_RUS_SITFA_20261124.md) quedan como riesgos heredados conocidos.
- **Corrección del oráculo de correos (E-A5).** `correos_iguales_a_python` dependía del día (Python y Rust ponían la fecha de hoy en el asunto): `generar_correos.py` fija `date.today()` en 2099-01-01 (campo `hoy` del JSON) y Rust usa `util::hoy()`, fijable en pruebas.

## Tramo 4 · Meta 4, Anonimizador y distribución

- **E-R0 · Prueba de reutilización — `HECHO` en Linux; Windows en la CI.** `flow-meta4` y `flow-anonimo` son dependencias por ruta de `app/`; `app/` compila y pasa su suite, y sus 29 y 38 pruebas pasan con `preserve_order` de serde_json (la característica que `app/` unifica). Duplica `rust_xlsxwriter` (0.102/0.103) y `quick-xml` (0.41/0.42). Resultado: E-R1, E-R2 y E-T4 se reducen al adaptador y a la vista. Agregar `flow-meta4` y `flow-anonimo` a `app/` como dependencias por ruta, compilar para Windows y correr sus 29 y 38 pruebas. Si funciona, E-R1, E-R2 y E-T4 se reducen al adaptador y a la vista; si no, se portan.
- **E-R1 · Meta 4: cálculo (F23) — `HECHO` (R1a, R1b y R1c; commit `4313c54`, CI Windows verde, run 45487699648). Alcance A elegido por el usuario el 2026-11-24.** (R1a) `descargador/meta4.rs` lee el formato del Descargador y porta `indexar_bitacoras`/`buscar`/`control_identidad`/`_lote`/`_leer_bitacoras` tal cual. (R1b) Oráculo `app/qa/generar_meta4.py` (22 casos sobre el lote ficticio del Descargador con columna TRIBUNAL y programa de residencia): `meta4_bitacoras.generar` con `_escribir` capturado frente a `meta4::calcular`, fila a fila, más los libros y el reporte de 5 casos; prueba `meta4_del_descargador_igual_a_python` (conteos 22 · 7 · 30) y `oraculo_de_meta4_detecta_alteraciones`. Hallazgos: los mensajes de error de lote de `load_lot` difieren de los de `_lote` (portados); los productos del crate difieren de los de Python en 9 hojas auxiliares (fijadas, D-13). (R1c) El caso `calendario_fallido_con_bitacora_leida` acredita que una bitácora leída vale aunque falle su calendario. Corrección de Windows: `leer_bitacoras` canonicaliza las carpetas (`copia_local` compara con la ruta canónica; con un nombre corto `RUNNER~1` o un enlace toda bitácora salía «NO VERIFICABLE»), con la prueba `meta4_lee_bitacoras_elegidas_por_una_ruta_no_canonica`.
- **E-R2 · Meta 4: plantilla, regeneración y vista (F24) — `HECHO` (commit `4313c54`, CI Windows verde, run 45487699648).** `meta4_ui.rs` y «Meta 4» en la navegación: genera y regenera con los productos de `flow-meta4`, plantilla propia opcional, carpeta de salida nueva; prueba de vista `la_vista_genera_y_regenera_desde_un_lote_del_descargador`.
- **E-T4 · Anonimizador (F25) — `HECHO` (commit `5a669a0`, CI Windows verde, run 45457412572).** `anon::ejecutar` usa `flow_anonimo::lote::ejecutar`. Sin oráculo propio: la paridad con Python vive en `rust/crates/flow-anonimo/tests/paridad.rs`; `app/` prueba texto, ZIP y paquete sin identidades, original intacto y captura alterada sin copias.
- **E-T5 · Distribución, ayuda y autodiagnóstico (F26, F28) — `HECHO` (commit `a95b404`, CI Windows verde, run 45458302666).** `autodiagnostico.rs` (equivalente de `package_check.py` para lo que existe en Rust), `ayuda.rs`, `--self-test` y su paso en la CI. Fuera de alcance: el paquete `portable/` del Anonimizador de Python (otro producto) y el instalador.
- **E-X1 · Guardia de solo lectura — `HECHO` (CI Windows verde, run 45487699648).** `tools/verificar_solo_lectura.py` falla si `app/src`, la extensión del Descargador o Lite mencionan los controles de creación o edición de observaciones de RUS (`COD_Estado_Detalle_Nuevo`, `id_Obs_centro`, `GLS_ObsCentro_Nueva`, `chkBitacora`, `Txt_Cont2`…) o un `.Send(` de Outlook. Su `--autoprueba` se corre en la CI junto al verificador (origen: análisis del informe RUS/SITFA).
- **E-X2 · Diagnóstico de las pestañas de la causa — `HECHO` (CI Windows verde, run 45487699648).** `historia::pestanas_causa` y `resumen_pestanas` registran en `historia/estructura_pestanas.json`, sin valores, si la respuesta de la causa que ya se descarga trae tablas de Historia, Notificación y Diligencias, con sus encabezados y filas. No cambia `historias.json`. El resultado decide si vale un lector de Notificación y Diligencias.
- **Paquete H:** acta del usuario.

## Decisiones del usuario vigentes

- **Producto Excel (20-nov).** Columnas NURUS al final del consolidado o del original, en el orden `NURUS_ID_REGISTRO, NURUS_REGLAS, NURUS_PROPUESTA, FECHA_OBS, OBSERVACION, TT, CC, RES, NURUS_ESTADO_REVISION, NURUS_DECISIONES`. Prima sobre el orden de `flow-python`.
- **Redacción (20-nov).** Se aplica en Flow Rust; cambiar Flow Python requiere instrucción expresa.
  - **R1:** con la medida por vencer, sin la oración del próximo informe.
  - **R2:** la imposibilidad de atribuir un trámite a un ingreso queda como aviso interno.
  - **R3:** DCE con un solo encabezado y sin días.
  - **R4:** sin días de espera cuando el tribunal dispuso el ingreso.
  - **R5:** texto vigente en lugar de reiterar, solo para «Medida revisada, sin observaciones.» y para la derivación sin seguimiento.
  - **R6:** nombre del programa en tipo título.
  - R7 y R8: sin cambio.
- **D-1 (23-nov):** R6 conserva el guion del nombre de origen («Ciudad del Niño - Mulchén»).
- **D-2 (23-nov):** `keep_filters` solo en Seguimiento y Litigantes, como Python.
- **D-3 (23-nov):** se conserva el mensaje de Rust ante un XML 2003 con entidad externa.
- **D-4 (23-nov):** integrar todas las herramientas de Flow Python.
- **D-5 (24-nov):** «El Descargador recolecta toda la información que Flow necesita para analizar. Descargador recolecta info, Flow la procesa.» Interpretación aplicada en este plan, confirmada por el usuario el 25-nov: el contrato es el paquete, y los archivos internos del lote no necesitan ser idénticos a los de Python.
- **D-6 (24-nov):** F27 se mantiene en el alcance.
- **D-7 (24-nov):** orden Descargador, exportaciones de lote, Asistente.
- **D-9 (24-nov):** F27 se resuelve con la opción B: un botón «Abrir en Trabajo» tras «Crear paquete», sin solicitudes ni recuperación entre herramientas. Si se necesitara recuperar una descarga a medias, se retoma como encargo nuevo.
- **Decisiones del 25-nov (respuestas del usuario).**
  - **«módulo» con tilde:** se conserva la plantilla de Rust; queda como diferencia declarada frente a Python («modulo»).
  - **D-10 y D-11: revertir las dos.** Rust vuelve a la conducta de Python (egresadas; R11 sin condición de causa compartida).
  - **D-12: se mantiene diferida** (C10 por asociación). Se reabre solo con un caso real sin propuesta `NOMENCL`.
  - **D-13: conservar** las 9 hojas auxiliares de Meta 4 del crate, fijadas en `HOJAS_DECLARADAS`.
  - **D-5: confirmada** (el contrato es el paquete).
  - **F14: la persona elige** el formato al crear los proyectos Word (un `.docx` por proyecto o uno por tribunal): encargo E-P1.
  - **M7 (deriva de esquema): no por ahora.**
  - **Tercera tanda (25-nov, 10 preguntas):**
    - **E-P1:** un `.docx` por tribunal lleva un índice al inicio y un salto de página entre proyectos.
    - **Meta 4:** solo lotes del Descargador; no se aceptan `bitacoras_meta4.json` ni reportes Excel sueltos.
    - **F08 restante:** solo las anulaciones de columnas de origen por fila (encargo E-A10). Quedan fuera las variables de Word por fila y la exclusión manual.
    - **Entrega (F26):** instalador (encargo E-I1). Falta decidir firma y dónde se aloja.
    - **Anonimizador:** sin conteo de reemplazos por tipo. **Word manual:** sin la mejora de paginación de Python.
    - **M6 (Notificación y Diligencias): descartado.** E-X2 queda como diagnóstico.
    - **Prioridad:** seguir con el Asistente (E-A10), luego E-P1 y E-I1.
    - **Acta:** un mes real completo, en paralelo con Python.
    - **Proceso:** el asistente decide y sigue, registra las decisiones delegadas como revertibles y pregunta en tandas.
  - **Actualización sin reinstalar (25-nov):** fuente = carpeta local de actualizaciones; Flow avisa y aplica con un botón (E-I2). Regla permanente en `AGENTS.md`.
  - **Orden del Asistente:** primero el recálculo para paquetes (E-A9); después, si el usuario lo pide, las anulaciones de origen por fila, las variables de Word por fila y la exclusión manual.
- **D-8 (24-nov):** se retira de esta rama la documentación del proyecto Python; sigue en `flow-python`. El árbol de código Python se conserva hasta cerrar E-V1.

El detalle de las decisiones del 20 de noviembre está en el [informe de fallas](historico/INFORME_FALLAS_20261120.md#decisiones-del-usuario--20-nov-2026).
