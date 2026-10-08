# NuRus 0.3.0.dev6 — rendimiento y pendientes

Fecha: 13 de septiembre de 2026. Base: `b6657f57032b7888e0ba81aa17bec044185fe785` (dev5). Rama de integración: `implementacion-plan-2026-09-08`. Motor: 0.2.1; esquema SQLite: 8, sin migración nueva.

## Resultado de este tramo

Se ejecutaron mejoras técnicas de rendimiento y selección de productos, con pruebas automáticas. Los pendientes institucionales de la versión anterior se mantienen: no se transforman en aprobaciones por haber corrido pruebas simuladas.

El objetivo sigue siendo producir propuestas para la revisión individual humana en RUS y, desde su constancia, preparar productos. No se envían correos, no se escribe en RUS/SATURNO y no se modifican umbrales ni textos institucionales. Se conservan origen, hoja, fila y hash; los excluidos permanecen visibles.

## Cambios ejecutados

| ID | Hallazgo y objetivo | Cambio aplicado | Evidencia |
|---|---|---|---|
| R01 | La exportación portable consultaba `sheet.max_row` dentro del bucle. openpyxl recorre las celdas al calcularlo, multiplicando el trabajo al aumentar las filas. | Obtener el límite original una sola vez antes de anotar; conservar la validación de cada fila. | Benchmark antes/después y pruebas de exportación preservada. |
| R02 | El lector copiaba todo el DataFrame y creaba una Series por registro. | Usar tuplas sin copiar el DataFrame; conservar su índice físico y nombres de columnas normalizados. | `test_reader_performance_contract.py`: filas intercaladas vacías, tipos, fórmulas e inmutabilidad del DataFrame. |
| R03 | El exportador nativo escribía cada valor y formato por celda mediante COM. | Agrupar actualizaciones por columna y tramos contiguos, con máximo 500 filas por bloque. Para OBSERVACION de propuestas, leer y completar solo celdas vacías. | `test_native_bulk.py`: 10.000 valores en 20 escrituras, texto de 3.000 caracteres íntegro, filas no seleccionadas y fórmulas intactas. |
| R04 | Una condición de color por excluido aumentaba las llamadas COM y el límite de color no incluía siempre las últimas columnas administradas. | Una regla condicional sobre el tramo, usando el estado de cada fila; extender hasta la última columna original o administrada. | Contratos COM existentes, prueba de color y revisión del cálculo de columnas. |
| R05 | Una lista vacía de selección se trataba como selección de todo el lote. | `None` conserva el significado de todas las filas elegibles; `[]` produce cero correos. | `test_explicit_empty_selection_does_not_prepare_entire_batch`. |
| R06 | La nómina buscaba cada ID en una secuencia y podía aceptar un snapshot técnico con filas sin constancia. | Crear un conjunto para la búsqueda y exigir `rus_recorded` en cada registro del adjunto. | Pruebas de grupo limitado y rechazo de adjunto sin confirmación, antes de crear archivos. |
| R07 | Faltaba medición reproducible para lotes grandes. | Añadir `tools/benchmark_pipeline.py` y ejecutarlo en CI Windows/Linux; adjuntar JSON de resultados al wheel. | Verifica conteo, hash, fila física, fórmula en OB, exportación y recuperación de SQLite con datos sintéticos. |

Las pruebas que usan mocks representan contratos del código. No acreditan rendimiento, conservación completa de objetos Office ni interacción visual en Excel 2010.

## Medición local

Mismo entorno Linux/Python 3.12 y generador sintético. Cada tamaño se ejecutó una vez por versión para el flujo completo. La conversión interna del lector usa la mediana de cinco repeticiones. Se excluye del tiempo total la creación de la muestra y la verificación posterior; el total suma lectura, evaluación, persistencia, exportación portable y recuperación.

| Registros | Exportación dev5 | Exportación dev6 | Flujo dev5 | Flujo dev6 |
|---:|---:|---:|---:|---:|
| 100 | 0,030 s | 0,029 s | 0,068 s | 0,063 s |
| 1.000 | 0,397 s | 0,203 s | 0,599 s | 0,377 s |
| 10.000 | 22,094 s | 2,522 s | 24,096 s | 4,207 s |

Para 10.000 filas, la conversión interna del lector pasó de 0,381 a 0,114 segundos. La mejora mayor del flujo medido proviene de eliminar el cálculo repetido del tamaño de la hoja. Son mediciones de esta muestra, no una garantía temporal para cualquier libro o PC. No representan el tiempo de revisión humana ni de Outlook.

Datos exactos y entorno: [benchmark_20260913.json](benchmark_20260913.json). La medición nativa disponible es el número de operaciones del adaptador simulado; el tiempo de Excel real está pendiente.

## Cómo verificar y actualizar

1. Descargar la rama de integración y ejecutar `Instalar_NuRus.bat` para instalar dev6 en el entorno aislado existente. La carpeta local de datos y el esquema SQLite permanecen iguales. No requiere Node, servicios nuevos ni permisos administrativos adicionales.
2. Ejecutar `python -m pytest -q` dentro del entorno de desarrollo con las dependencias de pruebas. Resultado local: 148 aprobadas y una omitida por ser exclusiva de Windows.
3. Ejecutar `python tools/benchmark_pipeline.py --output medicion.json` desde la raíz. Elegir un nombre inexistente: el comando no sobrescribe el informe. Usa datos ficticios y elimina sus libros temporales.
4. Consultar GitHub Actions del commit para comprobar compilación, dependencias, instalación aislada, pruebas y benchmark en Windows/Linux. El artefacto del trabajo incluye wheel y JSON de medición.
5. En el PC institucional, procesar una copia autorizada y verificar propuestas/final con Excel 2010. Comprobar especialmente observaciones de más de 255 caracteres, fórmulas intermedias, campos administrativos y excluidos coloreados. NuRus continúa usando Excel nativo en Windows para la ruta de fidelidad completa; el benchmark portable no la sustituye.
6. Si surge una diferencia, conservar versión, hash del archivo, modalidad y mensaje de error. Reproducir el caso, corregir su causa y repetir su prueba y dependencias antes de aprobar la actualización.

## Pendientes que no pueden cerrarse desde este entorno

- Aceptación de conservación del libro con Excel 2010, guardado real de borradores en Outlook clásico e interacción gráfica en el equipo institucional.
- Ratificación humana de matrices, destinatarios/adjuntos y significado institucional de TT/CC/RES.
- Archivo exacto de Espera del incidente, si el error persiste; el `.xls` aportado anteriormente es de Cumplimiento.
- Confirmación por registro de lo ingresado en RUS y autorización documentada de las excepciones de cruce que correspondan.

[G-ESTADO] Las correcciones técnicas de este tramo y las comprobaciones locales están ejecutadas. El commit y su ejecución de CI identifican la publicación verificada. No se declara aceptación institucional.

[L-SIGUIENTE] Ejecutar el caso de exportación nativa con Excel 2010 sobre una copia autorizada y registrar el resultado; no volver a alimentar el sistema para cada producto del mismo lote.
