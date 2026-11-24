# Estado de Flow Rust

Actualizado: 24 de noviembre de 2026, tras E-X1, E-X2, E-R1 y E-R2, E-S1 (con su vista), E-V1 a E-V3, E-Q7 el producto de Trabajo sin consolidado (E-W1) E-A1, E-U1, E-A2, E-A3, E-A4, E-T1, E-T2, E-T3 y E-A5. Es la única fuente del estado: se edita en el mismo commit que cierra cada encargo.

- Aplicación: `app/` (`flow_rust.exe`). Referencia funcional: rama `flow-python`.
- Rama única: `estudio/paridad-python-rust-20261119`.
- Estados: **Sí** (existe y tiene contraste con Python), **Parcial** (falta lo que se indica), **No** (ausente). Ninguno significa «aceptado»: eso solo lo declara el usuario en un acta.
- Origen: registro y encargos hasta el 24 de noviembre. En esta fecha solo se repitió la suite en Linux (`tools/probar_app.sh todo`: 61 y 43 pruebas, 0 fallos).

## Siguiente paso

1. **Ver el primer run** de «Flow Rust local · Windows» con los trabajos `linux` y `validar` (E-V1 a E-V3 de [PLAN.md](PLAN.md)).
2. **Usuario:** prueba corta del tramo 1 (ver «Pendiente del usuario»).
3. **E-U1** (por especificar con el usuario) de [PLAN.md](PLAN.md); después, el tramo 3 (Asistente).
4. **Usuario:** decidir D-10, D-11 y D-12 del [análisis del informe RUS/SITFA](ANALISIS_INFORME_RUS_SITFA_20261124.md). E-X1 (guardia de solo lectura) y E-X2 (diagnóstico de Notificación y Diligencias) no requieren decisión.

## Pendiente del usuario

- **Diagnóstico de Notificación y Diligencias (E-X2):** tras un lote con «Historia de la causa», enviar el contenido de `historia/estructura_pestanas.json` (solo estructura, sin datos). Dice si esas tablas llegan en la respuesta ya descargada.

- **Prueba corta del tramo 1** con el `.exe` de un run de «Flow Rust local · Windows» lanzado a mano con `entrega` (Actions › Run workflow): un lote Integral de Seguimiento con Chrome y la sesión SITFA propia, «Leer bitácoras del lote», «Crear paquete de este lote…», abrir el ZIP en Trabajo y guardar el Excel. Anotar lo que falle; no hace falta acta.
- Revisión visual de las capturas del run (antes E-E) y paquete H: acta de uso real.

## Capacidades

### Asistente

- **F01 Lectura de hojas — Sí.** `lectura.rs`.
- **F02 Trabajo y cruces — Parcial.** Tres etapas, Hoja 2, calendario 91 y CAL-03. Egresados (E-A2) y avisos de órdenes de búsqueda OB-01 a OB-05 (E-A5, 21 casos contra Python; se listan en Trabajo para los paquetes con Litigantes). Falta la identidad multifila: no hay en Python un comportamiento distinto del ya cotejado en el corpus multifila de E3.
- **F03 Reglas de Excel — Sí.** `oraculo_python`, 174 casos.
- **F04 Contexto de causa — Sí.** `oraculo_historia`, 2.585 casos; paquete C con ZIP de Flow Python.
- **F05 Otro programa, reiteración y Escritos — Sí.** Cronología con 181 casos, R5 parcial y consignación con deshacer.
- **F06 Revisión de bitácoras — Sí.** V1 y V2 contrastadas en la CI.
- **F07 Importación de paquetes — Sí.** Conciliación entre capturas (E-A2): las decisiones pasan a las filas con la misma identidad remota, los conflictos se resuelven en pantalla y Trabajo muestra qué filas son nuevas, tienen fuente o reglas cambiadas y cuáles egresaron. Contrastado con Python en 12 casos. Trabajo concilia la etapa abierta; las decisiones de otras etapas del paquete anterior no se trasladan.
- **F08 Edición y revisión — Parcial.** Edición masiva de las filas marcadas (revisar y RES), también las ocultas por el filtro, y «Deshacer» (E-A5); se prueba su comportamiento, sin oráculo. Faltan las anulaciones de columnas de origen y las decisiones de correo por fila.
- **F09 Sesión y vigencia — Sí, sin oráculo de huellas.** Los correos y proyectos preparados se sellan con la huella de lo que leen (registros, plantillas, contactos, firma, captura); si eso cambia quedan «desactualizados» y no se guardan. Cada borrador guardado deja un recibo y uno idéntico no se crea dos veces. La sesión recupera los productos preparados y sus recibos; al conciliar capturas pasan marcados para revisar. Las huellas son propias de Rust (no coinciden byte a byte con Python); su comportamiento se prueba de punta a punta. Sin estado «incierto» tras un fallo de Outlook.
- **F10 Excel preservado — Parcial.** «Actualizar desde Excel» (E-A1) trae la revisión de la copia (observación, `FECHA_OBS`, `TT`, `CC`, `RES`) con asociación por identidad y conflictos que se resuelven en pantalla, contrastado con Python en 12 casos. Falta recalcular cuando se editan columnas de origen en Excel y combinar `NURUS_DECISIONES`. Sin libro base (paquete sin consolidado), el producto sale como libro nuevo con la estructura de `FAE_RES` (hojas de servicio, todas las columnas del registro, las 10 columnas de revisión en el orden del usuario y `NURUS_CLAVE_INGRESO`); la copia preservada sigue exigiendo Windows y Excel de escritorio.
- **F11 Configuración y contactos — Parcial.** 16 umbrales y textos con verificador. Alias de contactos con las mismas colisiones que Python, editor de plantillas de correo con selector de variables, duplicar, archivar y restaurar, e importación de matrices Word desde un ZIP (E-A4), contrastados con Python en 44 casos. Falta el importador de matrices de contactos y paquetes de plantillas con revisión empaquetada (`install_bundled_templates`, no aplica a Rust). Una copia de plantilla sirve para «Gestión particular»; el correo automático por programa sigue usando las plantillas base.
- **F12 Correos y nómina — Sí.** Agrupación, destinatarios, CC, firma y alcance contrastados con `prepare_drafts` de Python en 18 casos (E-A5). Diferencia declarada: la plantilla «Cumplimiento» de Python dice «modulo» sin tilde y Flow Rust «módulo».
- **F13 Borradores Outlook — Parcial.** EML y Outlook con cuenta elegible (E-T1): solo `Save()`, CC fijo, y un fallo antes de guardar se distingue de uno posterior (queda «incierto» y no se reintenta hasta revisar Borradores). Contrastado por pruebas de la salida de PowerShell; sin prueba con Outlook real.
- **F14 Proyectos Word — Sí.** Matrices de Laja y Mulchén. Por confirmar con el usuario: un `.docx` por proyecto.
- **F15 Formularios manuales — Sí.** Correo manual con adjuntos (huella de lo revisado, CC fijo, sin Excel) y Word libre o con matriz `{{CAMPO}}`, bloques favoritos con variables y recuperación de lo escrito (E-T2); 19 casos contrastados con Python. Sin la mejora de paginación de Python.
- **F16 Resultados y estadísticas — Sí.** Vista y reporte; estadísticas de constancias (propuestas, constancias con y sin carga, datos incompletos, borradores y proyectos creados) con su Excel, y métricas locales opcionales de cada ciclo (E-T3), contrastadas con Python en 14 casos. Las métricas no guardan datos de casos.

### Descargador

- **F17 Integral y Lite — Sí** sobre SITFA ficticio. Sin prueba con Chrome real.
- **F18 Gestión de lotes — Sí.** Pausa, cancelación, pendientes, favoritos y preferencias.
- **F19 Bitácora, historia y calendario — Sí** sobre SITFA ficticio (E-Q1 a E-Q4, 16 escenarios). Avance por fila en la interfaz (E-Q7).
- **F20 Escritos — Sí** dentro de la lectura de historia.
- **F21 Histórico de Protección — Sí**, con suspensión por fallas.
- **F22 Exportaciones de lote — Sí** en Linux. Paquete `CSMP_CAPTURE` desde el lote (5 escenarios), `Bitacoras_RUS.xlsx` e `Historia_SITFA.xlsx` (`EXCEL_BITACORAS` 3/3), libro consolidado y «Para CSMP» (`LIBROS` 14/14), y CSV, ICS, ZIP de entrega y comparador (`PRODUCTOS` 13/13). Sección «Resultados del lote» en el Descargador con consolidado, Libro CSMP, cruce, comparación, CSV, ICS y ZIP (sin el doble clic para abrir el primer archivo ni «Preparar pendientes», que ya cubre «Recuperar pendientes de un lote…»). Falta la CI de Windows.
- **F27 Intercambio entre herramientas — Sí, opción B (D-9).** «Abrir en Trabajo» tras crear el paquete. Sin solicitudes ni recuperación entre herramientas.

### Otras herramientas

- **F23 y F24 Meta 4 — Parcial (E-R1 y E-R2, `HECHO` en Linux; falta su CI de Windows).** `descargador/meta4.rs` lee el formato del Descargador (`resumen.json` con `plan`/`consultas`, `verificacion NNN.json` con `vinculos_ingreso`, `bitacoras.json` `BITACORAS_LECTURA` v1 con HTML verificados por SHA-256), porta `_lote`, `_fuentes`, `_leer_bitacoras`, `buscar` y `enriquecer` de `meta4_bitacoras.py` y entrega el `Meta4` de `flow-meta4`, cuyos productos (planilla, Registros, consolidados, bitácoras del mes, reporte, plantilla propia y regeneración) se reutilizan. Vista «Meta 4» en la navegación: lotes, bitácoras, corte, mes, tribunales, plantilla, generar y regenerar. Sin oráculo contra `meta4_bitacoras.generar` sobre un lote del Descargador (R1b): hoy solo pruebas de humo con el lote ficticio de Flow Python pasado a Residencia. Diferencias con Python: no acepta `bitacoras_meta4.json` (Meta 4 1.0/1.1) ni carpetas de reportes Excel sueltos.
- **F25 Anonimizador — Implementado (E-T4, `HECHO`: CI Windows verde).** `app/src/anon.rs` delega en `flow-anonimo`: tablas, texto, JSON, ICS, Word, ZIP y paquetes de captura (contrato, huellas y referencias se reconstruyen), con verificación de cada copia antes de publicarla y carpeta `Anonimizados_*` nueva. Paridad con Python en las 38 pruebas del crate (`tests/paridad.rs`); en `app/`, tres pruebas (`anon::tests`). Diferencia: el informe ya no cuenta reemplazos por tipo; entrega avisos y la nota de revisión manual.
- **F26 Distribución — Parcial (E-T5, `HECHO`: CI Windows verde).** El `.exe` de la CI ahora corre `--self-test` (autodiagnóstico sintético, informe nuevo, código 0/1/2) antes de entregarse, junto con su SHA-256 y commit. Sin instalador ni paquete portable propio; sin firma Authenticode.
- **F28 Utilidades y ayuda — Parcial (E-T5, `HECHO`: CI Windows verde).** Ajustes trae la ayuda por vista y las reglas fijas, y el botón «Ejecutar autodiagnóstico» (matrices Word, CC fijo, flujo de las tres etapas con producto Excel y Anonimizador). Sin manuales aparte.

## Diferencias declaradas frente a Python

- R1 a R6 y el orden de columnas NURUS: decisiones del usuario (ver [PLAN.md](PLAN.md)).
- D-3: mensaje propio de Rust ante un XML 2003 con entidad externa.
- Correos: «módulo» con tilde en la plantilla de Cumplimiento (Python: «modulo»). Decisión del usuario pendiente.
- Carga en XLS BIFF: Python escribe `5.0` y Rust `5`. En HTML y XLSX coinciden.
- Historia: Rust la lee siempre que se elige «Historia de la causa»; Python solo al retomar.
- Lecturas: el puente de Rust no trae el `code` de la extensión, y las fallas quedan con el código genérico.
- Paquete: sin `include_originals` (ninguna ruta de Python lo activa).

## Documentos

- Lectura al iniciar: `AGENTS.md`, este archivo y [PLAN.md](PLAN.md).
- Según necesidad: [VERIFICACION.md](VERIFICACION.md), [DISENO.md](DISENO.md), [REGISTRO_IMPLEMENTACION.md](REGISTRO_IMPLEMENTACION.md), [REVISION_20261124.md](REVISION_20261124.md) y [ANALISIS_INFORME_RUS_SITFA_20261124.md](ANALISIS_INFORME_RUS_SITFA_20261124.md).
- La documentación del proyecto Python se retiró de esta rama (D-8) y sigue en `flow-python`. Quedan `docs/aceptacion/`, `docs/evidencia_p0/golden/` y dos archivos de `docs/implementacion/` que usa `rust/`.
- [historico/](historico/) guarda el registro, los encargos, la matriz y los informes anteriores al 24 de noviembre. No se lee al iniciar.
