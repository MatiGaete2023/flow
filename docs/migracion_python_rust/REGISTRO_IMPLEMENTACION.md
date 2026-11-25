# Registro de implementación · Flow Rust

Entradas desde el 24 de noviembre de 2026. Lo anterior (turnos 1 a 17, auditorías y entregas E-B a E-Q6) está en [historico/REGISTRO_hasta_20261124.md](historico/REGISTRO_hasta_20261124.md). El estado vigente y el siguiente paso están en [ESTADO.md](ESTADO.md); este archivo no se lee al iniciar.

## Formato

Una entrada por encargo, de diez líneas como máximo, la más reciente arriba:

```
## <fecha> · <encargo> · <commit>
- Cambió: <archivos o módulos>
- Acredita: <prueba u oráculo y su conteo>
- Diferencias declaradas: <ninguna | detalle>
- Pendiente: <ninguno | detalle>
```

No se copian salidas completas ni listas de mutaciones: la prueba afirma sus conteos y la CI del commit es la evidencia.

## 2026-11-24 · E-A5: cotejos de correos, órdenes de búsqueda y edición masiva

- Cambió: `ordenes.rs` (nuevo; lista en Trabajo), `trabajo.rs` (`aplicar_masivo`, `deshacer`), `ui.rs` (barra de edición masiva) y los oráculos `app/qa/generar_correos.py` y `app/qa/generar_ordenes.py`, agregados a la regeneración de la CI.
- Acredita: `CORREOS: 18/18` contra `prepare_drafts` (con 14 correos de Cumplimiento que difieren solo en «modulo»/«módulo», declarado); `ORDENES: 21/21` contra `avisos_orden`; `la_edicion_masiva_cambia_solo_lo_pedido_y_se_deshace`; los dos `oraculo_de_*_detecta_alteraciones`; `tools/probar_app.sh todo`: 64 y 76 pruebas, 0 fallos.
- Diferencias declaradas: las de [ESTADO.md](ESTADO.md) (F02, F08 y F12).
- CI de Windows en verde (run 45449454854). Pendiente: decisión del usuario sobre la tilde de «módulo».

## 2026-11-24 · E-T3: estadísticas y métricas

- Cambió: `estadisticas.rs` (nuevo), `config.rs` (`metricas_locales`) y `ui.rs` (tarjeta en Resultados, exportar resumen, opción en Ajustes y registro de importación, exportación y borrador); oráculo `app/qa/generar_estadisticas.py` → `app/tests/datos/estadisticas.json`, agregado a la regeneración de la CI.
- Acredita: `ESTADISTICAS: 14/14` contra `statistics.summarize`, `export_summary` y `review_metrics` (conteos, hojas, límite de 60 ciclos, archivo dañado, opción apagada); `oraculo_de_estadisticas_detecta_alteraciones` y `el_resumen_de_un_trabajo_cuenta_lo_que_la_persona_completo`; `tools/probar_app.sh todo`: 64 y 71 pruebas, 0 fallos; `inventario_escrituras` PASA (78 en 21).
- Diferencias declaradas: las de [PLAN.md](PLAN.md).
- CI de Windows en verde (run 45447605617).

## 2026-11-24 · E-T2: formularios manuales

- Cambió: `manual.rs` (nuevo), `productos.rs` y `outlook.rs` (adjuntos libres verificados por huella), `preparados.rs` (huella con adjuntos), `ui.rs` («Correo manual» y «Word manual»); el oráculo `generar_personalizacion.py` suma 11 Word y 8 bloques.
- Acredita: `MANUAL: 19/19` contra `generate_word`, `block_variables` y `render_block`; `oraculo_del_word_manual_detecta_alteraciones`; `un_correo_manual_exige_destinatario_y_conserva_la_huella_del_adjunto`, `lo_que_se_escribe_y_los_favoritos_se_recuperan` y el flujo `un_correo_manual_se_guarda_sin_excel_con_su_adjunto_y_el_cc_fijo`; `tools/probar_app.sh todo`: 64 y 68 pruebas, 0 fallos; `inventario_escrituras` PASA (76 en 20).
- Diferencias declaradas: las de [PLAN.md](PLAN.md).
- CI de Windows en verde (run 45445728806).

## 2026-11-24 · E-T1: cuenta de Outlook y fallos al guardar

- Cambió: `outlook.rs` (nuevo; sale de `productos.rs`), `config.rs` (`cuenta_outlook`), `preparados.rs` (recibo «incierto») y `ui.rs` (cuenta en Ajustes, bloqueo de reintento y «Ya lo revisé en Outlook»).
- Acredita: `el_script_solo_guarda_y_nunca_envia`, `el_cc_fijo_y_la_cuenta_viajan_al_script`, `un_fallo_antes_de_guardar_se_distingue_de_uno_despues` (cuenta inexistente, fallo previo, fallo tras `PREPARADO`, identidad incompleta) y `un_guardado_incierto_de_outlook_bloquea_el_reintento_hasta_conciliarlo`; `tools/probar_app.sh todo`: 64 y 63 pruebas, 0 fallos; `inventario_escrituras` PASA.
- Diferencias declaradas: sin prueba con Outlook real; la firma y el cuerpo HTML de Python no se portan.
- CI de Windows en verde (run 45443830208). Pendiente: prueba del usuario con su Outlook.

## 2026-11-24 · E-A4: ajustes

- Cambió: `personalizar.rs` (nuevo), `config.rs` (alias, plantillas anteriores, archivada y categoría), `ui.rs` (alias en Contactos, editor de plantillas, importar matrices y plantilla de «Gestión particular») y `productos.rs` (una plantilla archivada no prepara correos); oráculo `app/qa/generar_personalizacion.py` → `app/tests/datos/personalizacion.json`, agregado a la regeneración de la CI.
- Acredita: `AJUSTES: 44/44` contra `save_contact`, `duplicate_template`, `archive_template`, `restore_template`, `template_validation.variables` e `import_templates`; `oraculo_de_ajustes_detecta_alteraciones`; `una_copia_de_plantilla_sirve_para_gestion_particular_y_una_archivada_no`; `tools/probar_app.sh todo`: 64 y 59 pruebas, 0 fallos; `inventario_escrituras` PASA (71 en 18).
- Diferencias declaradas: las de [PLAN.md](PLAN.md).
- CI de Windows en verde (run 45441868390), con el oráculo regenerado en Windows.

## 2026-11-24 · E-A3: recibos, vigencia y recuperación

- Cambió: `preparados.rs` (nuevo), `ui.rs` (sellado al preparar, aviso «desactualizado», bloqueo al guardar, borradores repetidos, recibos, guardado y recuperación de la sesión), `conciliar.rs` (los productos pasan a la captura nueva marcados `REVISAR_CAPTURA_ANTERIOR`), `productos.rs` y `trabajo.rs` (`Preparados` en la sesión).
- Acredita: `un_producto_desactualizado_no_se_guarda_y_uno_repetido_tampoco` (editar un registro desactualiza el producto, no se guarda, volver a prepararlo sí, el mismo contenido no se duplica, la sesión recupera productos y recibos) y la extensión del flujo de conciliación; `tools/probar_app.sh todo`: 64 y 55 pruebas, 0 fallos.
- Diferencias declaradas: huellas propias de Rust; sin estado «incierto» ante un fallo de Outlook.
- CI de Windows en verde (run 45439994977).

## 2026-11-24 · E-A2: conciliación entre capturas

- Cambió: `conciliar.rs` (nuevo), `ui.rs` (abrir un paquete con otro abierto, diálogo «Conciliar capturas», resumen «Frente a la captura anterior» y etiqueta por fila), `Trabajo.cambios` y el oráculo `app/qa/generar_conciliacion.py` → `app/tests/datos/conciliacion.json`, agregado a la regeneración de la CI.
- Acredita: `CONCILIACION: 12/12` contra `capture_reconciliation.reconcile` (mismo paquete, identidad perdida, fuente cambiada con y sin edición, conservar, reiniciar, respuesta ajena, faltantes); `oraculo_de_conciliacion_detecta_alteraciones`; flujo `un_paquete_nuevo_conserva_decisiones_y_pide_respuesta_ante_un_conflicto`; `tools/probar_app.sh todo`: 64 y 54 pruebas, 0 fallos.
- Diferencias declaradas: las de [PLAN.md](PLAN.md).
- CI de Windows en verde (run 45437385372), con el oráculo regenerado en Windows.

## 2026-11-24 · E-U1: Abrir en Trabajo (D-9)

- Cambió: `descargador_ui.rs` (botón «Abrir en Trabajo» bajo el paquete creado y `paquete_para_trabajo`) y `ui.rs` (abre el paquete y pasa a Trabajo).
- Acredita: la extensión de `ui_descargador_bitacoras_y_paquete` (el clic deja la ruta del paquete, que se recoge una sola vez); `tools/probar_app.sh todo`: 64 y 51 pruebas, 0 fallos.
- Diferencias declaradas: sin solicitudes, respuestas ni recuperación como Python (decisión del usuario, opción B).
- CI de Windows en verde (run 45432865102).

## 2026-11-24 · E-A1: Actualizar desde Excel

- Cambió: `sincronizar.rs` (nuevo), `Registro` (`campos`, `id_excel`, `sync_base`, `sync_local`, guardados en la sesión), `producto_excel.rs` (escribe `FECHA_OBS`, `TT` y `CC`), `ui.rs` (botón, diálogo de conflictos y memoria de la última copia) y el oráculo `app/qa/generar_sync.py` → `app/tests/datos/sync.json`.
- Acredita: `SYNC: 12/12` contra `nurus.personal.sync.refresh` (remota, local, conflicto y sus resoluciones, misma edición, RES, sin columna técnica, identidad cambiada, faltan registros); `oraculo_de_sync_detecta_alteraciones`; `la_copia_guardada_se_importa_sin_cambios`; flujo `guardar_la_copia_y_actualizar_desde_excel_trae_la_revision`; `tools/probar_app.sh todo`: 64 y 51 pruebas, 0 fallos.
- Diferencias declaradas: las de [PLAN.md](PLAN.md) (observación vaciada, origen editado, `NURUS_DECISIONES`).
- CI de Windows en verde (run 45429755148), con el oráculo regenerado en Windows. La CI regenera solo los oráculos que cambian (a78 no coincide byte a byte en Windows desde antes). Pendiente: copia preservada de Excel de escritorio sin prueba con Excel real.

## 2026-11-24 · E-W1: producto de Trabajo sin consolidado

- Cambió: `producto_excel.rs` (nuevo), `captura_trabajo.rs` (columnas completas y avisos agrupados), `copia_excel.rs` (motivo sin consolidado), `descargador/paquete.rs` (`CONSOLIDADO_OMITIDO`), `revision_ui.rs` (filtros de atención) y `ui.rs` (avisos del paquete).
- Acredita: `paquete_sin_consolidado_exporta_la_estructura_de_flow_python` (columnas SITFA, orden de revisión, clave de ingreso, hojas y trazabilidad), la extensión de `paquete_con_consolidado` y `la_revision_destaca_las_filas_sin_bitacora_y_con_advertencia`; `tools/probar_app.sh todo`: 64 y 47 pruebas, 0 fallos; `inventario_escrituras` PASA.
- Diferencias declaradas: libro nuevo en lugar de copia preservada; `FECHA_OBS`, `TT` y `CC` salen vacías hasta que Trabajo las modele; avisos agrupados solo en pantalla.
- CI de Windows en verde (run 45420239558). Pendiente: revisión del usuario con su paquete.

## 2026-11-24 · E-S1: vista de Resultados del Descargador

- Cambió: `app/src/descargador_resultados_ui.rs` (nuevo) y su sección en `descargador_ui.rs`, equivalente a `ventana_resultados.py`: carpeta del lote, «Permitir resultados INCOMPLETOS», tabla de consultas con su compatibilidad CSMP, y los botones Consolidar XLSX, Libro CSMP, Añadir informes al cruce, Comparar con otro lote, CSV, Calendario ICS, ZIP y Abrir carpeta. Cada producto recarga y verifica el lote en un hilo.
- Acredita: `ui_resultados_abre_un_lote_y_crea_sus_productos` (CSV de la vista igual al de Python, rechazos de Libro CSMP e ICS, lote alterado rechazado) y `resultados_cruza_compara_y_rechaza_como_python`; `tools/probar_app.sh todo` en Linux: 64 y 45 pruebas, 0 fallos; `clippy --lib -D warnings` limpio.
- Diferencias declaradas: sin doble clic para abrir el primer archivo ni «Preparar pendientes» (lo cubre «Recuperar pendientes de un lote…»); «Libro CSMP» no lanza CSMP aparte.
- Pendiente: CI de Windows; revisión visual del usuario.

## 2026-11-24 · E-R0

- Cambió: `app/Cargo.toml` y `app/Cargo.lock` (dependencias por ruta `flow-meta4` y `flow-anonimo`); la CI de Windows corre sus pruebas.
- Acredita: `tools/probar_app.sh todo` sin cambios (64 y 43); `cargo test -p flow-meta4` y `-p flow-anonimo` con `serde_json/preserve_order`: 29 y 38 pruebas, 0 fallos.
- Diferencias declaradas: dos versiones de `rust_xlsxwriter` y de `quick-xml` en el ejecutable; `calamine` con `dates` unificado (sin efecto en la suite).
- Pendiente: compilación y pruebas en Windows (CI).

## 2026-11-24 · E-Q7

- Cambió: `descargador/bitacoras.rs` (`ejecutar_con_avance`, eventos `query`, `diary_progress` y `warning`) y `descargador_ui.rs` (filas revisadas, estado y avisos).
- Acredita: `lote_de_bitacoras_igual_a_python` afirma una fila de avance por registro en los 16 escenarios (`AVANCE`); `ui_descargador_bitacoras_y_paquete` comprueba las etiquetas.
- Pendiente: ver la lectura real con la extensión (usuario).

## 2026-11-24 · E-V3

- Cambió: `.github/workflows/flow-rust-local.yml` (trabajo Linux, detección de cambios, entrega a pedido) y `motor/historia.rs` (el paquete B usa `prueba_entorno::requerida`).
- Acredita: localmente, lo que corre el trabajo Linux: `cargo test --locked --all-targets` (64 y 43, 0 fallos) y los cinco verificadores en PASA.
- Pendiente: primer run en GitHub (no verificable desde esta sesión).

## 2026-11-24 · E-V1 y E-V2

- Cambió: `descargador/`, `src/` y `tests/` igualados a `flow-python` `afc4d12` (9 archivos); `app/qa/REFERENCIA_PYTHON.json`, `tools/verificar_referencia_python.py` y la CI (checkout del SHA y verificación); `bitacoras.json` y `paquete.json` regenerados; `prueba_entorno.rs` con rutas por omisión; `x11` para Linux en `app/Cargo.toml`; `contrastar_bitacoras.py` pide `--include-ignored`.
- Acredita: `tools/probar_app.sh todo`; contraste V2 local contra `afc4d12` (10 casos V1 y 5 P2); `verificar_referencia_python.py` en PASA.
- Diferencias declaradas: ninguna nueva. Rust ya seguía la revisión de bitácoras de `afc4d12`.
- Pendiente: CI de Windows; E-V3.

## 2026-11-24 · E-S1 · `d56c1ed` y el commit de cierre

- Cambió: `descargador/resultados.rs` (CSV, ICS, ZIP de entrega y comparador), `excel.rs` (hoja con formato de cuerpo), oráculo `app/qa/generar_resultados.py` → `app/tests/datos/resultados.json`, agregado a la CI.
- Acredita: `PRODUCTOS: 13/13`; pruebas permanentes `oraculo_de_productos_detecta_alteraciones` y `comparador_de_excel_detecta_alteraciones`; `tools/probar_app.sh todo`.
- Diferencias declaradas: sin `include_originals` (ninguna ruta de Python lo activa).
- Pendiente: CI de Windows; ofrecer los productos en la vista de Resultados.

## 2026-11-24 · D-8: retiro de la documentación Python de esta rama

- Cambió: se retiraron 656 archivos de `docs/` y, de la raíz, `CONTINUAR.md`, `PLAN_VIGENTE.md` y `RESUMEN_PROYECTO_NURUS_CSMP_20261009.md`. `README.md` reescrito. `docs/` pasó de 36 MB a 1,2 MB.
- Se conservan: `docs/migracion_python_rust/`, `docs/aceptacion/`, `docs/evidencia_p0/golden/`, `docs/implementacion/linea_base.json` y `docs/implementacion/verificacion/FLOW_D.json` (los usa `rust/`).
- Acredita: `tools/probar_app.sh todo` sin fallos; `check_real_ids`, `check_contactos`, `verificar_textos_umbrales` e `inventario_escrituras` en PASA.
- Diferencias declaradas: 18 archivos de pruebas y herramientas Python citan documentos retirados; esas pruebas se ejecutan en `flow-python`. No se ejecutó la suite Python en esta rama.
- Pendiente: flujos de CI de otras ramas, `rust/*.md`, `prototipos/` y el árbol Python (tras E-V1) siguen sin tocar.

## 2026-11-24 · E-Q5d y parte de E-S1 · `c734f84` (otra sesión)

- Cambió: `descargador/excel.rs` y `descargador/resultados.rs` (nuevos), `paquete.rs`, `xls.rs`, `descargador_ui.rs` y el oráculo `app/qa/generar_excel.py`.
- Acredita: `EXCEL_BITACORAS: 3/3` y `LIBROS: 14/14`. Repetido tras la fusión con la consolidación: `tools/probar_app.sh todo`, `61 passed` y `43 passed`, 0 fallos.
- Pendiente: CI de Windows de ese commit; de E-S1 faltan CSV, ICS, ZIP y el comparador. Esa sesión no dejó entrada de registro; esta la escribió la sesión de consolidación a partir del mensaje del commit.

## 2026-11-24 · Consolidación de la documentación y decisiones D-5 a D-7

- Cambió: `ESTADO.md` (nuevo), `PLAN.md` y `VERIFICACION.md` (reescritos), este registro (rotado), `AGENTS.md`, `MIGRACION_RUST.md`, `CONTINUAR.md` y `tools/probar_app.sh` (nuevo). Catorce documentos pasaron a `historico/`. En `app/`, solo el mensaje de `prueba_entorno.rs`, que ahora remite a `VERIFICACION.md`.
- Acredita: `tools/probar_app.sh todo` en Linux, `58 passed` y `43 passed; 10 filtered out`, 0 fallos (antes de fusionar `c734f84`).
- Decisiones del usuario: D-5 (el Descargador recolecta y Flow procesa), D-6 (F27 sigue en el alcance) y D-7 (orden Descargador, exportaciones, Asistente). Detalle en [PLAN.md](PLAN.md).
- Pendiente: D-8; E-V1 a E-V3. No se eliminó ningún archivo ni se cambió la CI.

## 2026-11-24 · Revisión del plan y de la documentación · `751634b`

- Cambió: [REVISION_20261124.md](REVISION_20261124.md). Sin cambios de código.
- Hallazgos: los oráculos de `app/qa` usan el Python de esta rama, que difiere de `flow-python` (`afc4d12`) en `descargador/flujo_paquete.py` y `src/nurus/personal/revision_bitacoras.py`; la matriz tenía capacidades pendientes sin encargo; la suite corre en Linux con `--features eframe/x11`, sin `.aislado`.

## 2026-11-24 · E-Q5 y E-Q6 · `61b307c` a `09448d4`

- Cambió: `descargador/paquete.rs`, `descargador_ui.rs` y `descargador/preferencias.rs`.
- Acredita: `PAQUETE: 5/5` contra `export_capture` y `package_current_lot`; `ui_descargador_bitacoras_y_paquete`.
- Diferencias declaradas: las de [ESTADO.md](ESTADO.md).
- Pendiente: E-Q5d y E-Q7.

## 2026-11-24 · E-T4 · Anonimizador con `flow-anonimo`

- Cambió: `app/src/anon.rs` (motor propio reemplazado por `flow_anonimo::lote::ejecutar`), filtro de archivos en `ui.rs`, `tools/inventario_escrituras.py`.
- Acredita: `anon::tests` (texto, ZIP y paquete de captura sin identidades y con el original intacto; captura alterada sin copias) y las 38 pruebas del crate.
- Diferencia declarada: el informe no cuenta reemplazos por tipo.
- CI Windows verde (run 45457412572). Pendiente: E-R1 y E-R2 (adaptador de lotes).

## 2026-11-24 · E-T5 · Autodiagnóstico y ayuda

- Cambió: `app/src/autodiagnostico.rs`, `ayuda.rs`, `main.rs` (`--self-test`), Ajustes en `ui.rs`, CI y `tools/inventario_escrituras.py`.
- Acredita: `autodiagnostico::tests` (7 comprobaciones pasan, el informe no se sobrescribe, código 2 si existe) y `ayuda::tests` (una sección por vista y reglas fijas).
- Diferencia declarada: sin `portable/` ni instalador; el equivalente de `package_check.py` cubre lo que existe en Rust.
- CI Windows verde (run 45458302666).

## 2026-11-24 · Diagnóstico anónimo de calendarios e históricos

- Cambió: `app/src/descargador/bitacoras.rs` escribe además `diagnostico_calendarios.json` (estados de calendario e histórico y códigos `CAL_*`, sin texto libre). Extensión de Rust: `diagnostico_bitacoras.json` queda igual al de Flow Python, porque `descargador/` es la referencia congelada (`verificar_referencia_python.py`).
- Motivo: el diagnóstico del lote del 24-nov (114 bitácoras, 92 historias, 114 históricos, todo LEIDA) no mostraba calendarios, y de ahí o del calendario de informes por mes salen las lecturas parciales que reporta el usuario.
- Acredita: `diagnostico_cuenta_calendarios_e_historicos_sin_texto_libre`.
- Pendiente: un diagnóstico nuevo del usuario con este archivo; luego E-R1.
- CI: los runs 45461517036 (verificador de la referencia congelada) y 45462587676 fallaron; este último por el detector de oráculos, que comparó contra el push anterior (donde `descargador/` había cambiado y se restauró) y regeneró «todos», incluido `a78`, que no es idéntico byte a byte desde antes. Sin cambios de código en la corrección.
- CI: con «todos» ya no se regenera `a78` (ZIP en base64 no reproducible entre ejecutores); se contrasta cuando cambian su generador o sus datos.
- CI Windows verde (run 45463811153, commit `8cd013b`).

## 2026-11-24 · Análisis del informe técnico RUS/SITFA del usuario

- Documento: `ANALISIS_INFORME_RUS_SITFA_20261124.md`, sobre `a47b1e3`. Sin cambios de código.
- Conflictos: el informe parte de otra rama (`9707e72`, ausente en el clon), da a Rust por congelado (lo contradicen D-4 y D-5) y propone un registrador que `AGENTS.md` prohíbe.
- Riesgos heredados de Python, con paridad exacta:
  - `conciliar.rs:344` rotula «egresadas» a las filas ausentes sin mirar la cobertura (`comparar_lotes` sí la mira);
  - R11 cierra la orden por cercanía y no aplica R2 (`motor/historia.rs:442`);
  - C10 se suprime con cualquier nomenclatura posterior (`escritos.rs:120`).
- Propuestas: decisiones D-10 a D-12, más E-X1 (guardia de solo lectura) y E-X2 (diagnóstico de Notificación y Diligencias).

## 2026-11-24 · E-R1 y E-R2 · Meta 4 desde los lotes del Descargador

- Cambió: `app/src/descargador/meta4.rs` (lector del formato del Descargador: `calcular`, `fecha_lectura`, `tribunales_disponibles`), `app/src/meta4_ui.rs` y la vista «Meta 4» en `ui.rs`, `ayuda.rs` y `main.rs`; `paquete.rs` expone `leer_registro`, `Clave` y `Fuente::original`; `flow-dominio` por ruta en `app/Cargo.toml`.
- Acredita: `meta4::tests` (lote y bitácoras de Flow Python pasados a Residencia: registros, identidad visible, lecturas, inventario, publicación de los cinco productos, rechazo de lote duplicado, tribunal ajeno, lote incompleto y bitácoras ausentes) y `meta4_ui::pruebas` (genera y regenera). `tools/probar_app.sh todo`: 68 y 84 pruebas, 0 fallos; clippy de la biblioteca limpio.
- Diferencias declaradas: no lee `bitacoras_meta4.json` ni carpetas de reportes Excel sueltos; sin `PERIODO_INCOMPLETO` (Python no lo tiene).
- Pendiente: oráculo R1b (`meta4_bitacoras.generar` sobre un lote del Descargador, fila a fila) y CI de Windows.

## 2026-11-24 · E-X1 y E-X2 · Guardia de solo lectura y diagnóstico de pestañas de la causa

- Cambió: `tools/verificar_solo_lectura.py` (con `--autoprueba`) y su paso en la CI; `descargador/historia.rs` (`pestanas_causa`, `resumen_pestanas`, `estructura_pestanas.json` al cerrar); `tools/inventario_escrituras.py` (+2 escrituras).
- Acredita: `--autoprueba` (cada control prohibido y `.Send(` se detectan; los módulos de prueba se ignoran), `pruebas_pestanas` (tablas, filas y encabezados sin valores; vacía frente a ausente) y los oráculos de bitácoras e historia sin cambios. `probar_app.sh todo`: 69 y 84 pruebas, 0 fallos; clippy limpio.
- Origen: análisis del informe técnico RUS/SITFA del usuario.
- Pendiente: el usuario envía `estructura_pestanas.json` de un lote real; de ahí sale la decisión sobre un lector de Notificación y Diligencias.

## 2026-11-24 · E-R1b · Oráculo de Meta 4 y corrección de Windows

- Causa del rojo de los runs 45470928947 y 45471386495: `meta4::tests` devolvía «BITÁCORA NO VERIFICABLE» en Windows; `copia_local` compara la carpeta con la ruta canónica y `leer_bitacoras` no la canonicalizaba (nombre corto de 8.3). Corrección: `paquete::rutas_bitacoras` y su prueba con un enlace simbólico (falla sin la corrección).
- Cambió: `app/qa/generar_meta4.py` (22 casos, `meta4.json`, entrada nueva en el workflow), `meta4.rs` (mensajes de lote y de tribunales como `_lote`, `pruebas_oraculo`).
- Acredita: `meta4_del_descargador_igual_a_python` (22 casos, 7 errores iguales, 30 filas, 5 casos con libros y reporte) y `oraculo_de_meta4_detecta_alteraciones`. `probar_app.sh todo`: 72 y 84 pruebas, 0 fallos; clippy limpio.
- Diferencias: 9 hojas auxiliares de los productos del crate (fijadas en `HOJAS_DECLARADAS`); `requieren_revision` suma alertas.
- CI: run 45487699648 verde (`linux` y `validar`, `meta4.json` regenerado en Windows). Pendiente: decisión D-13.

## 2026-11-24 · E-A6 · Importación de contactos

- Cambió: `personalizar.rs` (`importar_catastro`, `parse_contactos`, `importar_contactos_archivo`), `ui.rs` (botón «Importar» de Contactos), `app/qa/generar_catastro.py` y `catastro.json` (en el workflow).
- Acredita: `importacion_de_contactos_igual_a_python` (36 casos, 5 errores, 4 conflictos), `oraculo_de_catastro_detecta_alteraciones` y dos pruebas de comportamiento (JSON con tribunales y conflictos; CSV releído). `probar_app.sh todo`: 72 y 88 pruebas, 0 fallos; clippy de la biblioteca limpio.
- Corrige: la importación anterior pisaba contactos, aceptaba direcciones inválidas y no informaba conflictos.
- Diferencias: filas «Tribunal: X» de CSV/HTML actualizan el tribunal (extensión de Rust).
- CI: el run 45502222110 pasó las pruebas y falló al regenerar `catastro.json`: `ZipInfo.create_system` vale 0 en Windows y 3 en Linux, y cambiaba los bytes del `.xlsx`. Fijado a 3 en el generador.
- CI: run 45503587810 verde (`linux` y `validar`, `catastro.json` regenerado igual en Windows).

## 2026-11-25 · D-10 y D-11 (decisiones delegadas) y oráculo de correos

- Decisiones del asistente por delegación del usuario («Decide tú»): D-10 y D-11 aprobadas, D-12 diferida, D-13 conservar (ver `PLAN.md`).
- Cambió: `conciliar.rs` y `ui.rs` (ausentes con/sin cobertura), `motor/historia.rs` (R11 no se redacta con varias órdenes en causa compartida), `util.rs`/`productos.rs` (`hoy()`), `generar_correos.py` y `correos.json` (fecha fija).
- Acredita: `ausente_no_implica_egreso_sin_la_misma_consulta_completa`, `d11_varias_ordenes_en_causa_compartida_no_se_atribuyen` y los oráculos de historia con `d11 == 3` declarados. `probar_app.sh todo`: 72 y 90 pruebas, 0 fallos; clippy limpio.
- Corrige: `correos_iguales_a_python` fallaba al cambiar el día (el oráculo guardaba la fecha de generación).
- CI: run 45609310864 verde (`linux` y `validar`, con el contraste E3 con el ZIP de Python y `correos.json` regenerado en Windows).

## 2026-11-25 · E-A7 · Recálculo al editar columnas de origen en la copia

- Cambió: `sincronizar.rs` (`actualizar` con `Config`, `Resumen.recalculadas`), `ui.rs` (aviso de filas recalculadas), `generar_sync.py` y `sync.json` (17 casos, con propuesta y reglas de Python).
- Acredita: `actualizar_desde_excel_igual_a_python` (17 casos: reglas iguales y propuesta igual salvo R6) y `oraculo_de_sync_detecta_alteraciones`. `probar_app.sh todo`: 72 y 90 pruebas, 0 fallos; clippy limpio.
- Diferencias: solo archivos de RUS (un paquete avisa); se reanalizan todas las filas; `NURUS_DECISIONES` no se combina (sin F08 no hay estado que combinar).
- CI: run 45625544370 verde (`linux` y `validar`, `sync.json` regenerado igual en Windows).

## 2026-11-25 · E-A8 · Gestión de correo por fila y NURUS_DECISIONES

- Cambió: `trabajo.rs` (`gestiones_correo`, `sync_correo`, sesión), `productos.rs` (borradores), `conciliar.rs`, `copia_excel.rs`, `sincronizar.rs` (`leer_decisiones`, combinación) y el selector de la fila en `ui.rs`; `generar_correos.py` y `generar_sync.py` con sus JSON.
- Acredita: `correos_iguales_a_python` (24 casos), `actualizar_desde_excel_igual_a_python` (25 casos), las dos pruebas permanentes que alteran lo esperado y dos de comportamiento. `probar_app.sh todo`: 72 y 92 pruebas, 0 fallos; clippy limpio.
- Diferencias: solo la gestión de correo de `NURUS_DECISIONES` se combina (Python combina todo el estado de la fila); sin exclusión manual, anulaciones de origen ni variables de Word por fila.
- Pendiente: CI de Windows verde.

