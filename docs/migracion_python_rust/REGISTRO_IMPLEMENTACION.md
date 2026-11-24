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
- Pendiente: CI de Windows; E-R1 y E-R2 (adaptador de lotes); E-T5.
