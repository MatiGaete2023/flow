# Plan de implementación para otra IA

**Resultado esperado:** el operador puede hacer en la entrega Rust el trabajo que hoy hace en las herramientas Python, incluyendo abrir antecedentes antiguos y recuperar las decisiones humanas. La fuente de verdad es el comportamiento Python del commit fijado. El prototipo aporta referencias de interacción y una prueba diferencial acotada. La columna final de cada fila de [la matriz](MATRIZ.md) establece el resultado que se debe demostrar.

## Orden de ejecución

| Entrega | Depende de | Alcance | Producto verificable |
|---|---|---|---|
| E0 · Línea base | — | Todas | Entorno, resultados iniciales, flujos alcanzables y diferencias ya presentes |
| E1 · Interoperabilidad de bitácora | E0 | F06, F07, parte de F19 | El mismo ZIP Python se revisa en las dos aplicaciones |
| E2 · Captura, contrato y exportaciones | E1 | F17–F22, F27 | Paquete producido en Rust y abierto en Python y Rust; lote recuperable |
| E3 · Reglas y contexto | E1; cerrar casos de fuentes nuevas después de E2 | F01–F05 | Igualdad semántica documentada por regla, etapa y composición |
| E4 · Trabajo y Excel preservado | E1, E3 | F07–F10, F27 | Ida y vuelta por Excel con preservación y conflicto resuelto |
| E5 · Productos y configuración | E3, E4 | F11–F16 | Correo, nómina y Word reproducibles desde sesión y desde formulario manual |
| E6 · Meta 4 y anonimización | E2 para lotes; F25 puede probarse tras E0 | F23–F25 | Carpetas Python históricas utilizables, copias anonimizadas verificadas |
| E7 · Diseño, distribución y cierre | E1–E6 | F08, F16, F26, F28 y revisión de toda la matriz | Flujo integrado en Windows, inventario resuelto y acta de prueba del usuario cuando corresponda |

## E0. Fijar las referencias sin alterar los resultados esperados

1. Comprobar la rama con `git status -sb`, el commit Python y la existencia de `rust/crates`, `prototipos/flow-egui`, contratos y `docs/evidencia_p0/golden`. Leer el registro Python más reciente que esté en **esta base**.
2. Preparar el entorno de desarrollo indicado en [verificación](VERIFICACION.md). Ejecutar las suites por herramienta. Registrar por separado omisiones, xfail esperados, errores de entorno y diferencias del producto.
3. Inspeccionar el delta exclusivo de `rust/producto`. Para cada protección de la revisión del 17 de noviembre, localizar el comportamiento en esta base y ejecutar su prueba; una diferencia de historial no justifica fusionar ramas enteras.
4. Abrir el inventario y asociar los módulos que intervienen en el flujo elegido a los IDs de la matriz. Para funciones «por demostrar», localizar botón/CLI, productor y consumidor, persistencia y prueba. Conservar como utilidades las herramientas de desarrollo que no forman parte de la aplicación final.
5. Congelar fixtures sintéticos, corte y opciones. Comparar los catálogos de textos, alias, umbrales y matrices. El registro Python informa deriva previa del golden de configuración; documentar el campo y la versión antes de interpretar diferencias. Regenerar oráculos en un directorio de trabajo separado y revisar el diff.

## E1. Primera entrega de código: leer la evidencia V2 sin romper V1

1. Tomar el constructor del test `descargador/test_revision_bitacoras_p2.py`. Generar el ZIP sintético con dos ingresos desde el **exportador original**, una ventana completa y otra con paginador. Conservar los cuatro resultados de [CASOS_PRIMERA_ENTREGA.json](CASOS_PRIMERA_ENTREGA.json).
2. Añadir una prueba del **importador de paquetes** Rust junto a `rust/crates/flow-trabajo/tests/revision_bitacoras.rs`. Usar el ZIP y las fechas del JSON. Demostrar la diferencia «sin observación → no verificable» para la ventana completa con evidencia V2. Esto obliga a probar el lector real y su enlace a la clasificación.
3. En `flow-trabajo/src/paquete.rs` distinguir los contratos del productor. Para V1 mantener validación de eventos embebidos, rango, autoría, huella y contradicciones. Para V2 leer `filters_json`, la consulta y la tabla de bitácoras. Reutilizar la representación común después de validar el formato de origen.
4. Replicar `P/revision_bitacoras.py::datos_de_registro`: tipos estrictos, identidad compuesta, método reconocido, cero advertencias, alcance compatible, fechas consistentes y corte contenido. Asociar eventos a su ingreso/consulta. Un campo ausente, versión desconocida o discrepancia de identidad deja la ausencia sin acreditar. No convertir el número 2 en 1 ni suponer que V2 trae `entries`.
5. Repetir pruebas con el legado, copias contradictorias, evidencia parcial y dos personas de una causa. Pasar la misma sesión por clasificación, pantalla, exportación Excel/JSON y restauración. Capturar el conteo por categoría, no solo «paquete abierto».
6. Cierre de E1: las cuatro filas de aceptación coinciden, la suite V1 sigue pasando y el usuario puede abrir en Rust el ZIP Python sin recapturar información que ya venía acreditada. No modificar los umbrales del motor en este cambio.

## E2. Capturar y producir con interoperabilidad en ambos sentidos

1. Construir una tabla de los perfiles integral/Lite, selección y pantallas contra `flow-captura/src/sitfa` y `captura_ui.rs`. Reutilizar componentes comunes. Verificar lotes, pausas, favoritas, filtros, PDF vacío, pendientes, importación desde archivo y comparación offline mediante pruebas de `descargador/` y `descargador_lite/tests/`.
2. Incorporar al Descargador Rust el histórico **opcional**, desactivado por defecto. Seguir `contrato_integral.py`, `test_extension_historico.py` y el lector Python actual. Resolver primero en página ficticia la identidad por fila, la lectura reconocida, la incoherencia, el respaldo local y el rechazo de respuesta inesperada. Escritorio y extensión deben negociar la misma capacidad.
3. Alinear el tope de aperturas y los contadores por flujo con Python (5.000 en Python frente a 2.000 en la constante Rust localizada). Modelar la suspensión tras fallos consecutivos sin perder los Excel ni confundir unidades o tipos de apertura. Probar en memoria los puntos 4/5 fallos y 4.999/5.000/5.001 candidatos, sin consultas reales.
4. Reproducir para escritos las variantes tabla presente, vacío verificado, pestaña sin tabla y lecturas parciales divergentes. Reutilizar los diagnósticos anónimos y pruebas Python. Cuando falte evidencia de una variante externa, mantenerla pendiente y continuar con los demás casos.
5. En `flow-trabajo/src/exportar.rs`, contrastar las tablas y el manifiesto con `contracts/csmp_capture_v1` y `descargador/export_csmp_capture.py`. Incluir consolidado opcional y distingos de compatibilidad, e identificar cómo el consumidor Python reconocerá el alcance V2 que se emita. Cubrir registros, historia, escritos, calendario, incidentes y sus asociaciones.
6. Portar las salidas adicionales del lote desde `resultados.py`. Comparar CSV, Excel, ZIP e ICS semánticamente. Validar registros, zonas de fecha, nombres de hoja, tipos de celda, hashes y procedencia. Usar los tests `test_resultados_ics_semantics.py`, `test_consolidado_paquete.py`, `test_retomar_paquete.py` y `test_retencion_lotes.py`.
7. Entregar cuatro recorridos con el mismo lote ficticio: Python→Python como control, Python→Rust, Rust→Python y Rust→Rust. Fallo de lectura, reintento y cobertura parcial deben ser visibles y coherentes en producto y diagnóstico. La integración de Chrome real se reserva al entorno institucional autorizado.

## E3. Motor y composición de las observaciones

1. Enumerar cada regla y umbral en las tres familias más historia, composición y acciones. Vincular el código Python a `flow-motor`, y el contexto que recibe a `flow-trabajo`. Revisar también cómo la interfaz elige etapa y hoja.
2. Correr los tres archivos de oráculo y el de integración del workspace. Distinguir casos individuales de combinaciones multifila. Comparar reglas ordenadas, texto, acciones, exclusión y marcas, incluyendo nivel y contenido cuando el contrato lo traiga.
3. Probar fronteras (día anterior, exacto y siguiente), fecha de corte, 29 de febrero, egreso vencido/hoy, márgenes de DCE y concurrencia. Ejemplo: en Informes, el margen Python es `-10 <= días < 0` para DCE; verificar -11/-10/-1/0 con y sin evidencia posterior atribuible. Contrastar el caso con dos informes vencidos y evidencia ambigua.
4. Probar otro programa vigente y consignación/deshacer, historia finalizada, reiteración, curador, OB, escritos y calendario. Asegurar que una constancia no solicite dos veces lo mismo y que el agrupamiento mantenga la acción. Copiar los textos del catálogo versionado y mantener como pendientes las redacciones aún no confirmadas por el titular.
5. Para cada diferencia, adjuntar un caso mínimo, corregir en dominio/motor y reproducir desde archivo o ZIP. Conservar los 174 casos del prototipo como control de su alcance; la validación del workspace incluye todo el corpus.

## E4. Preservar el trabajo humano y los libros

1. Partir de `tests/test_preserved_workbook_export.py`, `test_personal_flow.py`, `test_copia_desde_consolidado.py` y `test_exportacion_segura.py`. Crear un libro ficticio con hoja auxiliar, fórmula, formatos, encabezado desplazado y columnas de revisión existentes. Guardar su copia y huella antes del cambio.
2. Separar explícitamente la exportación de un informe nuevo de la copia preservada. Seleccionar la estrategia que mantenga hojas, contenido y validaciones admitidas por el Python del formato correspondiente. Un `Workbook::new()` solo acredita la ruta de informe nuevo.
3. Reproducir la identidad y las reglas de `sync.associate`, registrar la foto base y combinar con `refresh`. Cubrir dos ingresos del mismo RIT, reordenamiento, archivo ajeno, encabezado ambiguo y conflicto de edición. Preparar todo en memoria y publicar solo al confirmar.
4. Conectar la misma transacción a GUI y persistencia. Agregar recuperación de captura/sesión y recalcular vigencia de productos según dependencias. Ensayar cancelación con Excel abierto, producto bloqueado y reintento.
5. Verificar continuidad con sesiones/carpetas Python existentes mediante copias. Si los formatos internos difieren, usar importación de solo lectura con versión, conversión a carpeta nueva y diagnóstico de campos omitidos. No migrar en el lugar la única copia del operador.

## E5. Cerrar productos, ajustes y modos manuales

1. Comparar plantillas, contactos, modalidades, firmas, nóminas, adjuntos y agrupación real contra `outputs.py`, `resolutions.py` y los golden. Verificar contacto inexistente, dirección repetida y CC fijo. Importar y revertir ajustes desde una configuración ficticia antigua.
2. Usar COM nativo y recibos de `flow-outlook`. Simular éxito, error antes de guardar, estado incierto y reintento. Verificar con `tests/test_outlook_safety.py`, `test_outlook_signature_regression_20260914.py` y `test_ef03_diario_recibos.py`. La prueba real usa un borrador revisable con autorización explícita.
3. Para Word comparar paquete OOXML y apertura en Word: estilos, variables resueltas, relación de headers/footers, orden y separación de proyectos. Reusar matrices originales. Cubrir `test_matrix_revision_20260914.py` y `test_nomenclatura_proyecto_20261116.py`.
4. Implementar o demostrar los editores manuales sin planilla, previsualización, biblioteca, estadísticas, preflight, diagnóstico y entrega seleccionada. Transportar el estado de lo revisado e invalidar solo los productos afectados.
5. Aceptación técnica: mismo lote genera los productos equivalentes, con el mismo alcance, y dos ejecuciones no sobrescriben archivos existentes. Si solo difieren metadatos de serialización explicar y comparar las partes semánticas.

## E6. Mantener el historial Meta 4 y la anonimización especializada

1. Adaptar las carpetas del Descargador Python al modelo de entrada de `flow-meta4`. Validar huellas, relación ingreso/bitácora, filtros, fechas y origen. Usar el mismo cálculo para el lote nativo y el adaptado.
2. Ejecutar `META4_Bitacoras/tests/test_meta4_bitacoras.py`, `test_generate_meta4.py` y las pruebas Rust de cálculo, plantilla y regeneración. Comparar libro estándar, plantilla propia, registro eliminado, empate y etapa anterior. Verificar regla de color y fecha vacía según motivo.
3. Pasar las mismas entradas sintéticas por `anonimizador/anonimizador_csmp` y `flow-anonimo`. Usar `test_archive.py`, `test_fuga.py`, `test_capture.py`, `rust/pruebas/anonimo_contrato.py` y el oráculo dedicado. Para identidades aleatorias comparar consistencia y neutralización. Para semilla estable comprobar reproducibilidad en el ámbito prometido por Python.
4. En publicación abortada no dejar un producto utilizable a medias. Revisar el archivo producido además del código de salida. Comprobar operación por carpeta y por CLI, y que el Asistente puede leer el paquete anónimo con sus nuevas huellas.

## E7. Integrar el diseño y entregar en Windows

1. Medir apertura → revisión → correo/Word → resultados. Adoptar del prototipo solo los elementos que reduzcan pasos sin duplicar estado. Verificar teclado, selección al filtrar, scroll, zoom 100/150/200 %, mínimo de ventana y retorno desde un diálogo cancelado.
2. Validar desde los nombres de ejecutables separados y desde el lanzador general. Las acciones del Asistente operan sobre archivos locales. Las lecturas de SITFA se efectúan en el Descargador. Comprobar la separación mediante un test del modo de arranque.
3. Resolver F28 y los módulos del inventario: registrar si son runtime, recurso distribuido, utilidad de soporte o prueba de referencia. Conservar generadores y scripts de desarrollo en Python cuando sirvan como oráculo independiente.
4. Banco de rendimiento propuesto: mismo equipo, mismo archivo sintético con 100/1.500/20.000 filas, mismo corte y compilación optimizada. Repetir al menos cinco veces por carga y separar primera apertura, análisis, edición y escritura. Informar mediana, peor muestra, memoria máxima, tamaño de distribución y versión. La comparación incluye el tiempo de lectura y se presenta separada de las pruebas con servicios externos.
5. Probar el paquete portable sin privilegios elevados. Registrar Windows, versión de Excel/Word/Outlook, política de Chrome y extensión. Verificar en Excel 2010 si se exige compatibilidad con ese puesto. Conservar versión previa para retorno. El usuario decide la aceptación institucional y cualquier publicación de versión.

## Registro que debe dejar cada entrega

Guardar ID funcional y de entrega, commit de partida, archivos tocados, fixture/corte, comando, estado anterior y posterior, discrepancias, pruebas pendientes y siguiente acción concreta. Estados recomendados: pendiente, presente por inspección, diferencia reproducida, corregido con prueba, verificado en Windows y aceptado por el usuario. Una falla de infraestructura se registra aparte. No reemplazar un golden para silenciar una diferencia. Si una decisión cambia un texto oficial, umbral o alcance institucional, dejar el punto y la evidencia requerida al usuario y avanzar en las filas independientes.
