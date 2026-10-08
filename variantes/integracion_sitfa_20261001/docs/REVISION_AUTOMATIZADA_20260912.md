# NuRus — revisión y mejoras automatizables

Fecha: 12 de septiembre de 2026. Versión de entrega: `0.3.0.dev5`. Base revisada: commit `0cb0fa0e10d94b8faf16108fe29832dc8f5a29ae`, rama `implementacion-plan-2026-09-08`.

## Resultado y alcance

Se revisaron lectura, cruces, identidad y cierre de constancias, recuperación de trabajo, exportaciones, estadísticas, adjuntos, empaquetado e instalación. Se corrigieron los defectos reproducibles descritos abajo y se incorporaron pruebas de regresión. Esta entrega conserva el flujo: propuesta del motor → revisión individual y registro humano en RUS → constancia Excel → productos derivados. No intenta sustituir la revisión ni duplicar el sistema oficial.

La versión anterior ya contenía correcciones para hojas RUS genéricas, guardado nativo mediante SaveAs y reutilización de la copia revisada. Esta revisión comprueba sus dependencias y resuelve defectos adicionales; no da por superada la prueba institucional de esas rutas.

## Hallazgos, acciones y objetivos

### P01 — Confirmaciones antiguas después de editar

**Hecho:** editar o restaurar una fila podía conservar `rus_recorded`, fecha y procedencia de una devolución anterior. Esa evidencia ya no acreditaba el contenido modificado.

**Objetivo:** impedir que una edición posterior se presente como confirmada en RUS.

**Aplicado:** `Database.set_record_decision()` y `restore_record()` eliminan confirmación, fecha y referencia de importación de la fila afectada; invalidan el snapshot vigente. Una nueva importación confirmada debe volver a acreditar la fila. Los valores administrativos permanecen como datos, sin convertirlos en evidencia.

**Comprobación:** `tests/test_review_import.py` cubre editar/restaurar y una segunda devolución parcial que no puede reciclar la confirmación anterior.

### P02 — Cierre de una devolución parcial mediante la API general

**Hecho:** el parámetro opcional de aprobación permitía omitir la exigencia de constancia aunque el lote ya tuviera una importación parcial.

**Objetivo:** evitar productos que aparenten revisión completa con filas sin constancia.

**Aplicado:** una importación existente obliga a comprobar la revisión por fila al congelar, aunque quien llame a la API omita ese parámetro. No se eliminó la capacidad de exportar propuestas antes de la revisión.

**Comprobación:** prueba de rechazo del cierre por defecto de un lote parcialmente importado; pruebas de cierre completo y snapshot existentes.

### P03 — IDs válidos asociados a la persona equivocada

**Hecho:** validar solamente el ID permitía intercambiar IDs entre dos filas y atribuir observaciones a otra identidad.

**Objetivo:** conservar identidad y procedencia al ordenar o devolver Excel.

**Aplicado:** además del ID se comparan las columnas de identidad disponibles en el mapeo original: RIT, RUT, nombre, tribunal y programa. Se rechazan ausencia, ambigüedad o cambio de identidad; se admiten equivalencias numéricas integrales de Excel. Se validan estados admitidos y devoluciones sin actualizaciones útiles.

**Uso:** ordenar filas completas sigue permitido. Cambiar los identificadores originales requiere corregir la fuente y analizar un nuevo lote; no reasignar IDs manualmente.

**Comprobación:** intercambio de IDs, archivos ajenos, estados no admitidos y constancias parciales en `tests/test_review_import.py`.

### P04 — Exportación final incompleta

**Hecho:** la exportación final podía mantener la propuesta/los campos originales sin reflejar todos los campos de la constancia aprobada.

**Objetivo:** entregar un Excel final que coincida con el insumo efectivo de los productos.

**Aplicado:** ambos exportadores incorporan `OBSERVACION`, `FECHA_OBS`, `TT`, `CC` y `RES` del snapshot para filas aprobadas y confirmadas. Reutilizan encabezados administrados existentes. Se conservan los datos de origen y la trazabilidad; los excluidos permanecen visibles con color.

**Límite:** esta función no fusiona cualquier edición arbitraria de otras hojas o columnas de la devolución. El archivo revisado sigue siendo la constancia de esas ediciones; el final se reconstruye sobre el original archivado con los campos administrados. No se afirma fidelidad total de objetos Office sin la prueba nativa.

**Comprobación:** exportación/reapertura con openpyxl y contratos COM simulados, en `test_review_import.py` y `test_preserved_workbook_export.py`.

### P05 — Cruce elegido por nombre y encabezado incorrecto

**Hecho:** preferir `Hoja2` ocultaba otra hoja válida; utilizar la fila de encabezado principal para el cruce ignoraba desplazamientos diferentes.

**Objetivo:** seleccionar evidencia de cruce inequívoca sin exigir renombrar el archivo.

**Aplicado:** detección independiente de encabezado y mapeo de cada candidata. Dos candidatas válidas generan ambigüedad; una selección explícita la resuelve. La interfaz incluye **Hojas y encabezado…** con nombres reales. La ausencia válida mantiene su excepción documentada.

**Comprobación:** múltiples candidatas, Hoja2 inválida y encabezados desplazados en `test_cross_sheet_exception.py`; inspección de nombres sin modificar el original en `test_reader_sheet_selection.py`.

### P06 — Vencimiento dependiente del orden de las filas

**Hecho:** el índice de cruce sobrescribía una coincidencia anterior con la última fila. Dos vencimientos futuros distintos podían producir diferente C-10 al ordenar Excel.

**Objetivo:** impedir una conclusión arbitraria del motor.

**Aplicado:** las fechas futuras contradictorias para la misma clave producen `CROSS_RECORD_CONFLICT`, referencias a las filas relacionadas y revisión pendiente. No se genera C-10 desde esa coincidencia. Los duplicados con la misma fecha no crean un conflicto falso. Se incrementa la versión del motor a `0.2.1` para identificar este cambio de evaluación.

**Comprobación:** mismas fechas en orden inverso producen conflicto; duplicados equivalentes siguen permitiendo C-10. La elección de la fecha correcta permanece a cargo de la revisión humana de origen.

### P07 — Repetición del análisis para continuar otro día

**Objetivo:** aprovechar datos ya procesados y reducir pasos de alimentación del sistema.

**Aplicado:** **Retomar lote…** lista trabajos RUS y recupera el mismo ID, filas, original archivado, ruta recordada y snapshot. Comprueba la integridad del original/snapshot; no ejecuta las reglas otra vez. Rechaza cambios de lote durante una operación activa.

**Comprobación:** recuperación con archivo de entrada ausente y mismo lote en `test_saved_review_ui.py`; interacción gráfica real pendiente del PC institucional.

### P08 — Archivos parciales o sobrescritos

**Objetivo:** un fallo de guardado no debe dejar un producto aparentemente completo ni sustituir trabajo ajeno.

**Aplicado:** `services/file_output.py` genera primero en un temporal y abre el destino en modo exclusivo. Limpia el destino parcial si lo creó esta operación y la copia falla. Se utiliza en nóminas, estadísticas, contador, Word y exportación tabular nueva.

**Comprobación:** fallo del escritor, destino creado concurrentemente y fallo de copia en `test_file_output.py`. Este control no garantiza invisibilidad durante la copia final ni recuperación ante corte eléctrico abrupto; se debe verificar el resultado si el proceso termina inesperadamente.

### P09 — Texto interpretado como fórmula y recuentos equívocos

**Objetivo:** datos externos de nuevas salidas no deben ejecutar fórmulas ni alterar el significado de estadísticas.

**Aplicado:** protección de cadenas que comienzan con `=`, `+`, `-` o `@` en nóminas, categorías administrativas y metadatos del contador. Las fórmulas originales del libro conservado no se transforman. Las estadísticas distinguen falta de confirmación en RUS de fecha inválida y mantienen categorías TT/CC/RES sin interpretarlas.

**Comprobación:** reapertura de salidas y tipo texto de las celdas; suma reconciliable de categorías en `test_statistics.py` y `test_file_output.py`.

### P10 — Adjunto distinto después de aprobarlo

**Hecho:** comprobar un archivo y adjuntarlo más tarde desde la misma ruta dejaba una ventana para que sus bytes cambiaran.

**Objetivo:** que el borrador use exactamente el adjunto revisado.

**Aplicado:** cada adjunto se copia a un directorio temporal privado mientras se calcula su hash; se compara con el aprobado y Outlook recibe esa copia, conservando su nombre. Se mantiene hasta terminar Save y se elimina después. El guardado incierto conserva su tratamiento de conciliación; no se reintenta automáticamente.

**Comprobación:** sustitución del origen después de copiar y rechazo por hash en `test_outlook_safety.py`. No se envió ningún correo ni se creó un borrador institucional durante estas pruebas.

### P11 — Catálogo y hash leídos en momentos distintos

**Objetivo:** acreditar exactamente qué textos y reglas se evaluaron.

**Aplicado:** `load_catalog_snapshot()` obtiene JSON y hash desde una sola lectura. La evaluación conserva ese hash; una modificación concurrente del archivo no cambia retrospectivamente la evidencia.

**Comprobación:** prueba que modifica el catálogo después de leerlo y verifica los bytes efectivamente evaluados en `test_catalog_integrity.py`.

### P12 — Instalación editable y documentación desactualizada

**Objetivo:** distribuir una versión identificable, evitar dependencia accidental del árbol de desarrollo y alinear la guía con el programa.

**Aplicado:** instalación local no editable; expansión retardada deshabilitada en los BAT; CI instala el paquete, verifica importación aislada y recursos, construye wheel y lo publica como artefacto. README y estado operativo describen la recuperación de lotes y dejan de presentar funcionalidades existentes como futuras. Se conserva la detección de Python corregida previamente.

**Comprobación:** contrato de BAT, construcción de wheel y prueba aislada del paquete. Ejecutar `Instalar_NuRus.bat` nuevamente tras descargar una actualización: cambiar solamente los `.py` ya no actualiza la copia instalada.

### P13 — Exceso de llamadas COM de trazabilidad

**Objetivo:** reducir el costo de comunicación con Excel al exportar.

**Aplicado:** los datos de la hoja técnica se entregan mediante una matriz a un rango, en lugar de escribir cada celda por COM. Se conserva formato texto y procedencia.

**Evidencia:** inspección y pruebas del adaptador simulado. No se cuantifica una mejora de segundos o porcentaje: requiere medir el equipo institucional.

## Verificación prevista y reproducible

1. Ejecutar `python -m pytest -q`: el conjunto contiene pruebas de lectores, reglas, snapshots, importaciones parciales, productos, contactos, salida preservada, seguridad Outlook, instalación y regresiones nuevas.
2. Ejecutar `python -m compileall -q src tests` y `python -m pip check` en el entorno instalado.
3. Construir con `python -m pip wheel --no-deps --wheel-dir dist .` e instalar el paquete en entorno aislado. Verificar versión, JSON de catálogo y plantillas, y creación de SQLite desde un proceso `python -I`.
4. Publicar en la rama de integración sin sobrescribir cambios remotos. Comparar hashes de los archivos publicados y el commit revisado.
5. Después de publicar, comprobar la ejecución Windows/Linux de GitHub Actions y revisar otra vez las rutas modificadas. Si aparece una regresión, corregirla y repetir su prueba y dependencias.

Los resultados concretos de la ejecución se registran en la entrega y en GitHub Actions para el commit. Las simulaciones no equivalen a aceptación Office. La ausencia de fallos en estas pruebas no demuestra ausencia absoluta de errores.

## Segunda revisión, posterior a la publicación

La primera publicación de estas mejoras fue `391b5fef52dbb19cd92323b17472bfce461b6248` (dev4). Se contrastaron los hashes Git de los 31 archivos modificados: coincidieron con la copia local probada. La revisión posterior encontró y corrigió dos casos adicionales; se identifica esta corrección como `0.3.0.dev5` para distinguir los paquetes.

- **P14 — Cambio de modalidad:** una selección explícita de hoja podía seguir activa al pasar de Espera a Cumplimiento. El evento ahora restablece selección automática y encabezado y elimina el análisis visible anterior. Prueba: `test_mode_change_discards_previous_modes_explicit_sheet`.
- **P15 — Memoria de la devolución:** al invalidar una fila, eliminar también el hash de importación del lote permitía que la aprobación general olvidara que se encontraba en un flujo de constancia. Se conserva en el lote la referencia de la última devolución, se invalida la confirmación y se exige nuevamente evidencia válida de la fila. Se comprueba el rechazo antes de recibir otra devolución, tanto al editar como al restaurar.

**Resultados locales:** 142 pruebas aprobadas; una prueba de `cmd.exe` se omite en Linux y debe ejecutarse en el trabajo Windows de CI. Compilación y construcción/instalación aislada del paquete comprobadas. El resultado final de CI se identifica por el commit y enlace de la entrega; incluye `pip check`, recursos instalados y wheel. No se publicó información de los registros institucionales.

## Actualización y próximos pasos humanos

1. Cerrar NuRus y conservar respaldo de su carpeta de datos local. Descargar la rama actualizada en una carpeta estable.
2. Ejecutar `Instalar_NuRus.bat`, después `Abrir_NuRus.bat`. Confirmar versión `0.3.0.dev5`; la base sigue en su ubicación local y no cambia de esquema.
3. Retomar un lote o analizar una copia. Si la selección es ambigua, indicar hoja/encabezado. Exportar propuestas sin exigir una revisión previa.
4. Revisar individualmente en RUS; completar y guardar Excel. Usar **Usar revisión guardada**, confirmar responsable y generar los productos sin recargar para cada uno.
5. Ejecutar los casos del [protocolo institucional](ACEPTACION_INSTITUCIONAL_20260910.md), especialmente A01, A04–A10 y A14–A17. Registrar errores con versión, archivo autorizado/hash y mensaje completo.
6. Ratificar plantillas y diccionario administrativo con la fuente institucional. No cambiar reglas jurídicas ni interpretar códigos automáticamente para superar una prueba.

[A-LIMITE] No se dispone de una sesión del PC institucional ni del archivo exacto de Espera de la captura. No se declara aceptación en Excel 2010/Outlook clásico. Esta es la frontera pendiente entre comprobación automática y uso institucional.
