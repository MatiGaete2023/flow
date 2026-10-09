# Trazabilidad acumulativa — ejecución 2026-10-08

El usuario difirió SITFA/RUS: no hay entorno/evidencia disponibles en esta etapa.
115 requisitos conservados. P0 permanece como evidencia histórica. Los IDs Pn-Enn
son etiquetas locales de los entregables numerados/listados en el plan, no IDs nuevos
de las especificaciones. **PASÓ** identifica evidencia local acotada; la aceptación
institucional figura separadamente **NO EJECUTADO**. No se asignan porcentajes ni GO.

Fuente: `docs/plan_20261008/05_PLAN_COMPLETO_EJECUCION_NURUS_CSMP_20261008.md`.
Pruebas, productos y hashes: [evidencia](evidencia_desarrollo_20261008/README.md).

## Fases y dependencias

| Fase | Estado | Dependencia/alcance |
|---|---|---|
| P0 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Continuación autorizada; conservar auditoría y golden del SHA original. |
| P1 | BLOQUEADA_POR_EVIDENCIA | Sesión autorizada y cotejo real por modalidad/pestaña; G1 pendiente. |
| P2 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Contrato probado localmente; no certifica autenticidad de fuente institucional. |
| P3 | IMPLEMENTADA_PENDIENTE_ACEPTACION | G1 para anunciar bitácoras reales fiables; Excel 2010 pendiente. |
| P4 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Aceptación institucional de fuente primaria, identidad y cobertura. |
| P5 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Aceptación institucional de los paquetes; pruebas locales y paridad disponibles. |
| P6 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Instalaciones institucionales separadas; código local ya desacoplado. |
| P7 | EN_CURSO | Preview ficticio CT-01/02; producción NO_EVALUABLE. G1/G10 pendientes. |
| P8 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Excel 2010/Outlook 365 y aceptación humana de productos. |
| P9 | BLOQUEADA_POR_EVIDENCIA | Contrato remoto auditado de historia/PDF. Extractor NO IMPLEMENTADO. |
| P10 | BLOQUEADA_POR_EVIDENCIA | CT-03..10 NO IMPLEMENTADAS; todas CT desactivadas. Aprobación por regla y G9. |
| P11 | IMPLEMENTADA_PENDIENTE_ACEPTACION | INT reales y ampliadas NO EJECUTADAS; MVP local comprobado. |
| P12 | BLOQUEADA_POR_EVIDENCIA | P9/P10 y acta institucional; ampliación excluida del MVP. |
| P13 | IMPLEMENTADA_PENDIENTE_ACEPTACION | CI 6d3555a aprobada; verificación adicional 396a643 confirma eliminación de carpeta y conservación de datos externos del instalador publicado. Acta pendiente entregada; Windows 10 institucional/Office y promoción pendientes. |

## Todas las tareas y entregables

| ID | Línea del plan | Estado | Tarea |
|---|---|---|---|
| P0.01 | 185 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Fijar SHA, versiones y estado real de CI; registrar divergencia con ramas históricas |
| P0.02 | 186 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Ejecutar suite actual y registrar número de pruebas, fallas/skips, dependencias y artefactos |
| P0.03 | 187 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Inventario de entradas, filtros, pestañas, modalidades, salidas y clases de error |
| P0.04 | 188 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Inventario Asistente y grafo de imports; identificar dependencias de utilidades y componentes heredados |
| P0.05 | 189 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Capturar resultados baseline con **fixtures ficticias**: Espera, Cumplimiento, Informes, Excel, Word, nóminas y borradores; anotar config |
| P0.06 | 190 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Verificar configuración y compatibilidad de versiones sin perder cambios del usuario |
| P0.07 | 194 | IMPLEMENTADA_PENDIENTE_ACEPTACION | decisión de alcance: |
| P1.01 | 202 | BLOQUEADA_POR_EVIDENCIA | Comprobar que EXE, extensión y metadatos pertenecen al mismo SHA y anuncian versiones/capacidades coherentes |
| P1.02 | 203 | BLOQUEADA_POR_EVIDENCIA | Reproducir los casos conocidos `SIN_VINCULO` mediante fixture y **contraste institucional local autorizado** |
| P1.03 | 204 | BLOQUEADA_POR_EVIDENCIA | Ajustar vínculo `ShowObservaciones`/`ShowHistoria` sin fabricar causa/ingreso por RIT; comprobar diferencias visuales tolerables |
| P1.04 | 205 | BLOQUEADA_POR_EVIDENCIA | Validar paginación, fechas, respuestas, fuentes y reintentos; comprobar exportación de textos largos |
| P1.05 | 206 | BLOQUEADA_POR_EVIDENCIA | Ejecutar batería institucional por modalidades y pestañas permitidas; documentar alcance real |
| P2-E01 | 216 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Especificación `contracts/csmp_capture_v1/README.md` con semántica, versiones, compatibilidad, claves de consulta/fila/ingreso, validación de esquema y reglas de contenido opcional. |
| P2-E02 | 217 | IMPLEMENTADA_PENDIENTE_ACEPTACION | `manifest.schema.json` (JSON Schema) y diccionario **por tabla** con tipos, nulos, cardinalidad, valores permitidos, claves ajenas, reglas de normalización y ejemplos ficticios. |
| P2-E03 | 218 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Definir exportación ZIP con `manifest.json`, `data/consultas.csv`, `registros.csv`, `bitacoras.csv`, `historia_tramites.csv`, `documentos.csv`, `incidencias.csv`, y carpeta de originales **solo cuando su inclusión esté autorizada**. Las fuentes opcionales pueden omitirse; su omisión debe estar declarada. |
| P2-E04 | 219 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Estados separados **por consulta, por archivo, por registro y por evento**, por ejemplo `COMPLETA`, `VACIA_COMPROBADA`, `PARCIAL`, `FALLIDA`, `NO_CONSULTADA`; no mezclar ausencia de filas, fallo y cobertura incompleta. |
| P2-E05 | 220 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Canonicalización de fecha y zona **sin inventar zona horaria** para eventos que carezcan de ella. Si faltan hora o fecha, dejar incertidumbre explícita. RIT/RUT/IDs remotos como texto. |
| P2-E06 | 221 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Trazabilidad original por `source_system`, `query_id`, tribunal, ingreso, modalidad, origen, archivo, hoja, fila, hash y fecha de captura; **no** confiar en RIT+nombre como clave única. |
| P2-E07 | 222 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Formato de textos extensos fuera del límite de celda Excel, con sidecar/split documentado, evitando truncamiento silencioso y preservando integridad. |
| P2-E08 | 223 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Contrato de solicitud opcional de nuevas consultas desde el Asistente hacia el Descargador: **archivo declarativo**, nunca instrucciones ejecutables, navegación directa ni llamadas a escritura. |
| P2-E09 | 224 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Validación contra ZIP slip (`../`), symlinks, rutas absolutas, colisiones, archivos falsos, expansión desproporcionada, hash incorrecto, CSV fórmulas, encoding, registros duplicados y IDs cruzados. Establecer límites cuantificados tras medir fixtures y la memoria disponible. |
| P2-E10 | 225 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Compatibilidad `1.x`: campos opcionales aditivos; cualquier cambio semántico incompatible requiere nueva versión mayor y migrador probado. |
| P3.01 | 237 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Mantener intacta la captura individual por pantalla, modalidad, estado, fechas y paginación |
| P3.02 | 238 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Mantener consolidado XLSX, CSV, ICS, ZIP, comparador de lotes, favoritos, controles y evidencias de consultas vacías |
| P3.03 | 239 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Hacer disponible `Bitacoras_RUS.xlsx` desde la ventana del Descargador, con hojas de lecturas, consultas, copia íntegra y errores |
| P3.04 | 240 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Compartir parser de bitácoras con Asistente sin importar paquetes exclusivos de UI de CSMP; mantener `nurus.bitacora_html` reutilizable |
| P3.05 | 241 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Separar selección de exportaciones: «Excel habitual», «Excel bitácoras» y «Paquete CSMP» |
| P3.06 | 242 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Desactivar dependencia del menú de «configurar CSMP» como requisito de trabajo; preservar compatibilidad de intercambio legado en fase transitoria |
| P3.07 | 243 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Conservar controles de pausa, cancelación, resultado parcial y recuperación de consultas |
| P4-E01 | 251 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Obtener inventario de consultas y original/hashes; conservar fuente primaria y mapeos estables. |
| P4-E02 | 252 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Transformar solo datos con lector validado; preservar originales junto a normalizados y registrar los problemas por fila. |
| P4-E03 | 253 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Producir tablas con claves de captura/consulta/ingreso; permitir fuentes no consultadas sin archivos falsos. |
| P4-E04 | 254 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Guardar primero en carpeta temporal, validar y publicar con escritura atómica; no sobrescribir paquete previo. |
| P4-E05 | 255 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Si una consulta está parcialmente fallida, etiquetar explícitamente la cobertura; no declarar total y no deducir desapariciones. |
| P4-E06 | 256 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Permitir abrir/reexportar un lote previo sin volver a acceder a RUS; hash y metadatos sin mutaciones silenciosas. |
| P5.01 | 266 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Validar esquema, hash, tamaño, rutas ZIP y tipos antes de alterar la sesión actual |
| P5.02 | 267 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Importar fuentes disponibles por tabla y registrar consultas no realizadas/errores |
| P5.03 | 268 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Modelo de identidad por causa + ingreso + persona/programa cuando corresponda; relación por ID remoto verificado |
| P5.04 | 269 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Normalizar eventos de bitácora y calendario, con metadatos de fuente y cobertura |
| P5.05 | 270 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Mantener importación de Excel legado y misma evaluación/observación tradicional con golden files |
| P5.06 | 271 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Reimportar misma captura y recuperar sesión/ediciones de forma explícita |
| P5.07 | 272 | IMPLEMENTADA_PENDIENTE_ACEPTACION | UI «Importar paquete» y reporte legible de integridad/cobertura antes de aceptar el trabajo |
| P6-E01 | 280 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Modificar `src/nurus/personal/results_view.py`: la lectura/descarga directa de bitácoras será **opción del Descargador**, no botón operativo de CSMP. Los Excel de bitácoras se producen allí; CSMP conserva **importación/análisis**. |
| P6-E02 | 281 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Desacoplar `src/nurus/personal/bitacora_link.py`, `download_link.py`, `download_transfer.py` y `integral_contract.py` del camino principal. No eliminarlos antes de comprobar grafo de imports, recuperación de solicitudes antiguas y rutas de UI heredadas. |
| P6-E03 | 282 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Evitar que `app_base.py` lance ejecutables externos para iniciar trabajo nuevo; mantener `Abrir Excel legado` y `Importar paquete`. |
| P6-E04 | 283 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Analizar otras utilidades duplicadas: exportaciones de fuentes en Descargador; propuestas, Word, Outlook y editor en Asistente; herramientas independientes se conservan fuera del flujo o se integran por un adaptador explícito. |
| P6-E05 | 284 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Distinguir una sesión de trabajo recuperable de un historial de revisiones institucionales. Retirar/dejar inactiva **solo** la persistencia de marcas locales como autoridad temporal; conservar edición, borradores, sesiones y comprobantes técnicos. |
| P7-E01 | 292 | EN_CURSO | Discriminar observaciones del centro, observaciones de otros actores, respuesta del tribunal, trámite SITFA, firma y calendario; no confundir fuentes. |
| P7-E02 | 293 | EN_CURSO | Determinar `ultima_observacion_centro` **solo** entre entradas con identidad remota y origen suficientemente acreditado; fecha/hora sin inventar y respetando cobertura consultada. |
| P7-E03 | 294 | EN_CURSO | Comparar eventos posteriores por fecha; ante empate sin hora, orden desconocido o campo ambiguo, mostrar incertidumbre y solicitar revisión focalizada. |
| P7-E04 | 295 | EN_CURSO | Mostrar estado de cobertura y hecho literal que sustenta cada hallazgo. No equivaler registro previo a trabajo recién ejecutado. |
| P7-E05 | 296 | EN_CURSO | Aplicar reglas tradicionales exactamente como baseline mientras las nuevas capas de contexto actúan solo como señales auxiliares controladas. |
| P7-E06 | 297 | EN_CURSO | Clasificar causas en `EXHAUSTIVA`, `FOCALIZADA`, `ORDINARIA`, `SIN_INCIDENCIA_DETECTADA` o `NO_EVALUABLE`, con razones visibles; los criterios nuevos se aprobarán en P10. |
| P8-E01 | 305 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Preservar páginas Trabajo, Correos, Resoluciones, Resultados y Configuración, editor de observaciones, filtros, selección múltiple, guardado y recuperación. |
| P8-E02 | 306 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Una fila de bandeja por registro/ingreso, con múltiples hallazgos relacionados; todas las incidencias deben poder consultarse sin generar duplicaciones erróneas en Excel. |
| P8-E03 | 307 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Mantener/reconciliar `OBSERVACION`, `FECHA_OBS`, `TT`, `CC`, `RES` y decisiones humanas. Ningún cambio de reglas puede reemplazar texto editado sin consentimiento. |
| P8-E04 | 308 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Producir Excel revisable y adjuntos de informes por vencer/pendientes **desde datos fuente válidos**, incluso si no se formula una observación nueva; agrupar según programa/tribunal/período. |
| P8-E05 | 309 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Mantener `outputs.py`, `resolutions.py`, `manual_products.py` y `adapters/outlook.py` como productores de Word y borradores sin envío; resolver destinatarios solo desde contactos comprobados, dejar desconocidos vacíos y generar error legible. |
| P8-E06 | 310 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Invalidar/regenerar productos preparados cuando cambie su fuente o decisión humana, sin afectar documentos ya exportados ajenos. |
| P8-E07 | 311 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Mantener funciones auxiliares de edición libre y productos manuales si aún se utilizan; no eliminar porque no formen parte del flujo principal. |
| P9-E01 | 319 | BLOQUEADA_POR_EVIDENCIA | Capturar localmente y bajo autorización ejemplos de ventana «Información de la causa» e historia SITFA: tabla real, fecha, descripción, referencia, identificador remoto, parámetros, paginación y estados. |
| P9-E02 | 320 | BLOQUEADA_POR_EVIDENCIA | Identificar nivel de identidad: **causa** vs **ingreso/persona**; impedir atribución al NNA equivocado si el trámite solo identifica la causa. |
| P9-E03 | 321 | BLOQUEADA_POR_EVIDENCIA | Desarrollar lectura secuencial `history_open`/equivalente únicamente sobre endpoint y parámetros auditados, sin permisos extra innecesarios. |
| P9-E04 | 322 | BLOQUEADA_POR_EVIDENCIA | Exportar `Historia_SITFA.xlsx` de modo autónomo y `historia_tramites.csv` para el paquete, preservando texto y referencias. |
| P9-E05 | 323 | BLOQUEADA_POR_EVIDENCIA | Registrar vínculos de documentos y estado; habilitar PDF **solo por selección explícita**, control de origen/identidad/tipo/tamaño/hash y política institucional. Un PDF no disponible **no** invalida la historia tabular comprobada. |
| P9-E06 | 324 | BLOQUEADA_POR_EVIDENCIA | Incorporar fuentes SITFA al importador del Asistente y mostrar contraste sin cambiar automáticamente resultados de reglas. |
| P10-E01 | 332 | BLOQUEADA_POR_EVIDENCIA | Crear matriz `regla → dato requerido → fuente → cobertura → condición → fragmento/acción → prioridad → incertidumbre`. |
| P10-E02 | 333 | BLOQUEADA_POR_EVIDENCIA | Mantener el catálogo vigente y parámetros de `config.py`/personalizaciones; congelar golden files antes/después y registrar diferencias exactas. |
| P10-E03 | 334 | BLOQUEADA_POR_EVIDENCIA | Introducir `CT-xx` como **desactivadas** por defecto, con versión, fixtures ficticios, explicación, consentimiento para activar y rollback por regla. |
| P10-E04 | 335 | BLOQUEADA_POR_EVIDENCIA | Evitar que una glosa de bitácora «informe enviado» pruebe por sí sola recepción, trámite correcto o subsanación; clasificar `PENDIENTE_VERIFICACION`. |
| P10-E05 | 336 | BLOQUEADA_POR_EVIDENCIA | No automatizar con/sin carga ni `TT/CC/RES` por simple coincidencia textual. No extender reglas jurídicas específicas de Laja/Mulchén/Tomé a otros tribunales sin catálogo nuevo. |
| P10-E06 | 337 | BLOQUEADA_POR_EVIDENCIA | Validar estados `NO_EVALUABLE` cuando falta fuente o identidad, y `SIN_INCIDENCIA_DETECTADA` como resultado **acotado**, no certificado de regularidad integral. |
| P13-E01 | 359 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Construir artefactos con metadatos de versión y SHA, incluyendo ejecutable Descargador, extensión 2.7.2 o nueva versión coherente y ejecutable Asistente; **preferir paquete conjunto para distribución**, aunque operen autónomamente. |
| P13-E02 | 360 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Verificar arranque desde carpeta nueva sin admin, configuración local migrable, preservación de datos al reinstalar y capacidad de volver a la versión anterior. |
| P13-E03 | 361 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Confirmar en Windows 10 el análisis/exportación sin Office; luego con Excel 2010 y Outlook 365 cuando corresponda. No presumir disponibilidad de COM en CI. |
| P13-E04 | 362 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Entregar manuales separados, diagramas, notas de versión, criterios de soporte, riesgos residuales, lista de `NO_EJECUTADO` y limitaciones reales. |
| P13-E05 | 363 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Realizar aceptación explícita del usuario antes de fusionar ramas, publicar release/instalador final o sustituir la versión en uso. |
| P11-ACEPTACION | 341 | IMPLEMENTADA_PENDIENTE_ACEPTACION | Aceptación integral según gate G11 |
| P12-ACEPTACION | 349 | BLOQUEADA_POR_EVIDENCIA | Aceptación integral según gate G12 |

## Matriz acumulativa de requisitos

| ID | Estado | Prueba local acotada | Institucional | Límite |
|---|---|---|---|---|
| D-T01 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Fixtures paginación pasan; aceptación institucional por variante pendiente. |
| D-T02 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Catálogo validado en fixtures; falta contraste real de combinaciones. |
| D-T03 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Vacío y evidencia probados localmente; navegador/institución pendiente. |
| D-T04 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; No repetir aceptación de filtros privados sin sesión autorizada. |
| D-T05 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Fechas y períodos fixtures; contraste por año real pendiente. |
| D-T06 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Cobertura temporal de firma no acreditada; probar cada pantalla. |
| D-T07 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Recuperación técnica local; no certifica selección institucional. |
| D-T08 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Validación de fixtures; mantener rechazo ante contrato inesperado. |
| D-T09 | BLOQUEADA_POR_EVIDENCIA | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Tests de identidad pasan; cotejo RUS y fallas conocidas siguen pendientes. |
| D-T10 | BLOQUEADA_POR_EVIDENCIA | NO EJECUTADO | NO EJECUTADO | Baseline preservado; G1 requiere sesión legítima y muestra por modalidad/pestaña. |
| D-T11 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Excel bitácoras autónomo, texto íntegro y consultas locales. G1/Excel 2010 pendientes. |
| D-T12 | BLOQUEADA_POR_EVIDENCIA | NO EJECUTADO | NO EJECUTADO | Ampliación excluida: extractor historia/PDF y reglas CT correspondientes no implementados/aceptados. |
| D-T13 | BLOQUEADA_POR_EVIDENCIA | NO EJECUTADO | NO EJECUTADO | Ampliación excluida: extractor historia/PDF y reglas CT correspondientes no implementados/aceptados. |
| D-T14 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Exportaciones locales con original intacto; Office real pendiente. |
| D-T15 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | ZIP completo/parcial desde originales comprobados; fuente institucional no certificada por fixtures. |
| D-T16 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | ZIP completo/parcial desde originales comprobados; fuente institucional no certificada por fixtures. |
| D-T17 | BLOQUEADA_POR_EVIDENCIA | NO EJECUTADO | NO EJECUTADO | Baseline preservado; No hay Excel 2010 en este Linux; CI Windows no prueba Office 2010. |
| D-T18 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Solo lectura en rutas revisadas y fixtures; nunca subir material real. |
| D-T19 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Baseline preservado; cualquier adaptación exige nueva regresión. |
| D-T20 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | CI 6d3555a SUCCESS: productos v1 desde EXE autónomo, paquete SHA coherente e instalador; 396a643 verifica eliminación real y datos externos intactos. Actas separan evidencia ficticia/institucional. Windows 10 institucional/Office pendientes. |
| R-T01 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Baseline sin cambio; contextual pendiente. |
| R-T02 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Baseline sin cambio; contextual pendiente. |
| R-T03 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Baseline sin cambio; contextual pendiente. |
| R-T04 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Baseline sin cambio; contextual pendiente. |
| R-T05 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Baseline sin cambio; contextual pendiente. |
| R-T06 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Baseline sin cambio; contextual pendiente. |
| R-T07 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Fallback E-06 conservado; NO_EVALUABLE contextual falta. |
| R-T08 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Baseline sin cambio; contextual pendiente. |
| R-T09 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Baseline sin cambio; contextual pendiente. |
| R-T10 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Baseline sin cambio; contextual pendiente. |
| R-T11 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Work advierte contradicción; clasificación agregada todavía falta. |
| R-T12 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Baseline sin cambio; contextual pendiente. |
| R-T13 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Conflicto C08: falta ficha puede confundirse con falta de columna; no corregir regla unilateralmente. |
| R-T14 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Baseline sin cambio; contextual pendiente. |
| R-T15 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Baseline sin cambio; contextual pendiente. |
| R-T16 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Baseline sin cambio; contextual pendiente. |
| R-T17 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Preview ficticio acotado a cuatro meses y mismo ingreso; interpretación real NO_EVALUABLE. |
| R-T18 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Preview ficticio acotado a cuatro meses y mismo ingreso; interpretación real NO_EVALUABLE. |
| R-T19 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Preview ficticio acotado a cuatro meses y mismo ingreso; interpretación real NO_EVALUABLE. |
| R-T20 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Preview ficticio acotado a cuatro meses y mismo ingreso; interpretación real NO_EVALUABLE. |
| R-T21 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Preview ficticio acotado a cuatro meses y mismo ingreso; interpretación real NO_EVALUABLE. |
| R-T22 | BLOQUEADA_POR_EVIDENCIA | NO EJECUTADO | NO EJECUTADO | Ampliación excluida: extractor historia/PDF y reglas CT correspondientes no implementados/aceptados. |
| R-T23 | BLOQUEADA_POR_EVIDENCIA | NO EJECUTADO | NO EJECUTADO | Ampliación excluida: extractor historia/PDF y reglas CT correspondientes no implementados/aceptados. |
| R-T24 | BLOQUEADA_POR_EVIDENCIA | NO EJECUTADO | NO EJECUTADO | Ampliación excluida: extractor historia/PDF y reglas CT correspondientes no implementados/aceptados. |
| R-T25 | BLOQUEADA_POR_EVIDENCIA | NO EJECUTADO | NO EJECUTADO | Ampliación excluida: extractor historia/PDF y reglas CT correspondientes no implementados/aceptados. |
| R-T26 | BLOQUEADA_POR_EVIDENCIA | NO EJECUTADO | NO EJECUTADO | Ampliación excluida: extractor historia/PDF y reglas CT correspondientes no implementados/aceptados. |
| R-T27 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Incidencias tradicionales existen; falta modelo de hallazgo contextual por regla. |
| R-T28 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Tribunales sin regla configurada advierten; no extrapolar catálogo jurídico. |
| R-T29 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Reanálisis/reimportación conserva decisiones; fuente/regla/corte invalida productos preparados. |
| R-T30 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Nóminas baseline; pendientes pruebas de enlace remoto del nuevo contrato. |
| R-T31 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Contactos desconocidos vacíos, borrador sin Send y advertencia de registro no acreditado. Outlook real pendiente. |
| R-T32 | BLOQUEADA_POR_EVIDENCIA | NO EJECUTADO | NO EJECUTADO | Ampliación excluida: extractor historia/PDF y reglas CT correspondientes no implementados/aceptados. |
| INT-01 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Excel autónomo legado probado; alcance real pendiente. |
| INT-02 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | ZIP completo/parcial desde originales comprobados; fuente institucional no certificada por fixtures. |
| INT-03 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Calendarios legados locales; falta normalización v1. |
| INT-04 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Filtros y cobertura fixtures; institución pendiente. |
| INT-05 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Pantallas legadas; aceptación por variante pendiente. |
| INT-06 | IMPLEMENTADA_PENDIENTE_ACEPTACION | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Recuperación legado funciona en fixtures. |
| INT-07 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Vacío con evidencia legado; contraste real pendiente. |
| INT-08 | BLOQUEADA_POR_EVIDENCIA | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Identidad fixtures; cotejo real pendiente. |
| INT-09 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Cuatro modalidades, hash/identidad, edición y legado locales; golden sin divergencia. Office real pendiente. |
| INT-10 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Excel bitácoras autónomo, texto íntegro y consultas locales. G1/Excel 2010 pendientes. |
| INT-11 | BLOQUEADA_POR_EVIDENCIA | NO EJECUTADO | NO EJECUTADO | Ampliación excluida: extractor historia/PDF y reglas CT correspondientes no implementados/aceptados. |
| INT-12 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Fuentes ausentes e inciertas explícitas; contexto de producción NO_EVALUABLE. |
| INT-13 | BLOQUEADA_POR_EVIDENCIA | NO EJECUTADO | NO EJECUTADO | Ampliación excluida: extractor historia/PDF y reglas CT correspondientes no implementados/aceptados. |
| INT-14 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Preview ficticio acotado a cuatro meses y mismo ingreso; interpretación real NO_EVALUABLE. |
| INT-15 | EN_CURSO | PASÓ | NO EJECUTADO | Hallazgos por ingreso y UI local; benchmark 100/500/2000. No certifica productividad institucional. |
| INT-16 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Reanálisis/reimportación conserva decisiones; fuente/regla/corte invalida productos preparados. |
| INT-17 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Calendario independiente y métricas de nómina desde fuente, aun sin nueva observación; recepción no inferida. |
| INT-18 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Word baseline y dobles Outlook; nueva entrega institucional pendiente. |
| INT-19 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Contactos desconocidos vacíos, borrador sin Send y advertencia de registro no acreditado. Outlook real pendiente. |
| INT-20 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Cuatro modalidades, hash/identidad, edición y legado locales; golden sin divergencia. Office real pendiente. |
| INT-21 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Cuatro modalidades, hash/identidad, edición y legado locales; golden sin divergencia. Office real pendiente. |
| INT-22 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Cuatro modalidades, hash/identidad, edición y legado locales; golden sin divergencia. Office real pendiente. |
| INT-23 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Reanálisis/reimportación conserva decisiones; fuente/regla/corte invalida productos preparados. |
| INT-24 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Sin lanzador remoto principal; submit rechaza antes de tocar adaptador. Marcas locales no persistidas como autoridad. |
| INT-25 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Excel bitácoras autónomo, texto íntegro y consultas locales. G1/Excel 2010 pendientes. |
| INT-26 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Sin lanzador remoto principal; submit rechaza antes de tocar adaptador. Marcas locales no persistidas como autoridad. |
| INT-27 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Paridad de 17 productos baseline y regresión local. No mezclar ramas históricas ni presumir aceptación Office. |
| F00 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Autorización y sesión son responsabilidad humana; prueba real pendiente. |
| F01 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Plan congelado/catálogo legado; variantes reales pendientes. |
| F02 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Lectura por pantalla conserva cobertura; no prueba todas las variantes reales. |
| F03 | BLOQUEADA_POR_EVIDENCIA | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Cotejo de bitácoras y cobertura requiere G1. |
| F04 | BLOQUEADA_POR_EVIDENCIA | NO EJECUTADO | NO EJECUTADO | Ampliación excluida: extractor historia/PDF y reglas CT correspondientes no implementados/aceptados. |
| F05 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Fuentes ausentes e inciertas explícitas; contexto de producción NO_EVALUABLE. |
| F06 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Excel bitácoras autónomo, texto íntegro y consultas locales. G1/Excel 2010 pendientes. |
| F07 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | ZIP completo/parcial desde originales comprobados; fuente institucional no certificada por fixtures. |
| F08 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Cuatro modalidades, hash/identidad, edición y legado locales; golden sin divergencia. Office real pendiente. |
| F09 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Cuatro modalidades, hash/identidad, edición y legado locales; golden sin divergencia. Office real pendiente. |
| F10 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Preview ficticio acotado a cuatro meses y mismo ingreso; interpretación real NO_EVALUABLE. |
| F11 | EN_CURSO | PASÓ | NO EJECUTADO | Hallazgos por ingreso y UI local; benchmark 100/500/2000. No certifica productividad institucional. |
| F12 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Reanálisis/reimportación conserva decisiones; fuente/regla/corte invalida productos preparados. |
| F13 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Calendario independiente y métricas de nómina desde fuente, aun sin nueva observación; recepción no inferida. |
| F14 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Actuación oficial humana; no automatizar registro ni envío. |
| F15 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Solicitud declarativa local; no ejecuta consultas. |
| A-T01 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Cuatro modalidades, hash/identidad, edición y legado locales; golden sin divergencia. Office real pendiente. |
| A-T02 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Fuentes ausentes e inciertas explícitas; contexto de producción NO_EVALUABLE. |
| A-T03 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Cuatro modalidades, hash/identidad, edición y legado locales; golden sin divergencia. Office real pendiente. |
| A-T04 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Cuatro modalidades, hash/identidad, edición y legado locales; golden sin divergencia. Office real pendiente. |
| A-T05 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Cuatro modalidades, hash/identidad, edición y legado locales; golden sin divergencia. Office real pendiente. |
| A-T06 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Cuatro modalidades, hash/identidad, edición y legado locales; golden sin divergencia. Office real pendiente. |
| A-T07 | EN_CURSO | PASÓ | NO EJECUTADO | Hallazgos por ingreso y UI local; benchmark 100/500/2000. No certifica productividad institucional. |
| A-T08 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Cuatro modalidades, hash/identidad, edición y legado locales; golden sin divergencia. Office real pendiente. |
| A-T09 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Reanálisis/reimportación conserva decisiones; fuente/regla/corte invalida productos preparados. |
| A-T10 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Calendario independiente y métricas de nómina desde fuente, aun sin nueva observación; recepción no inferida. |
| A-T11 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Contactos desconocidos vacíos, borrador sin Send y advertencia de registro no acreditado. Outlook real pendiente. |
| A-T12 | BLOQUEADA_POR_EVIDENCIA | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Dobles de Outlook verifican Save sin Send y adjuntos; falta Outlook 365 institucional, contactos autorizados y recepción humana. |
| A-T13 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Contactos desconocidos vacíos, borrador sin Send y advertencia de registro no acreditado. Outlook real pendiente. |
| A-T14 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Cuatro modalidades, hash/identidad, edición y legado locales; golden sin divergencia. Office real pendiente. |
| A-T15 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Sin lanzador remoto principal; submit rechaza antes de tocar adaptador. Marcas locales no persistidas como autoridad. |
| A-T16 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Sin lanzador remoto principal; submit rechaza antes de tocar adaptador. Marcas locales no persistidas como autoridad. |
| A-T17 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Cuatro modalidades, hash/identidad, edición y legado locales; golden sin divergencia. Office real pendiente. |
| A-T18 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | CI 6d3555a SUCCESS: productos v1 desde EXE autónomo, paquete SHA coherente e instalador; 396a643 verifica eliminación real y datos externos intactos. Actas separan evidencia ficticia/institucional. Windows 10 institucional/Office pendientes. |
| A-T19 | IMPLEMENTADA_PENDIENTE_ACEPTACION | PASÓ | NO EJECUTADO | Sin lanzador remoto principal; submit rechaza antes de tocar adaptador. Marcas locales no persistidas como autoridad. |
| A-T20 | EN_CURSO | NO EJECUTADO | NO EJECUTADO | Baseline preservado; Snapshot exacto 251d5c4 preserva 11 commits sobre main; golden/UI pasan, pero no hay aceptación funcional de todas las diferencias históricas. |

CSV/JSON adjuntos contienen requisito, aceptación, rutas de código/tests y línea
de la especificación. NO EJECUTADO también se usa cuando los tests son parciales:
su existencia o que la suite pase no demuestra el escenario integral completo.
