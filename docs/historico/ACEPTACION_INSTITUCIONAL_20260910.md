# Protocolo de aceptación institucional NuRus

Versión bajo prueba: `0.3.0.dev6`. No utilizar datos reales en GitHub.

Para esta actualización ejecutar además los casos de [reconocimiento, SaveAs y revisión guardada](CORRECCIONES_EXCEL_20260912.md). Los resultados simulados no acreditan Excel/Outlook institucionales.

## Registro obligatorio por caso

Completar: ID; commit/paquete; Windows/Office/Python; archivo y SHA-256; fecha de corte; resultado esperado; resultado obtenido; evidencia; responsable; estado; incidencia asociada.

## Casos

| ID | Acción | Criterio de aceptación |
|---|---|---|
| A01 | Instalar sin launcher `py`, con Python por usuario y con `.venv` previa | Abre la versión indicada y conserva SQLite; los errores explican qué falta |
| A02 | Analizar AGOSTO en Espera, Cumplimiento e Informes | Reconoce hoja/cabecera; las sumas no son casos; conserva fila y hash |
| A03 | Analizar el `.xls` del incidente | No produce el falso bloqueo masivo; el mapa de columnas es correcto |
| A04 | Probar cruce válido, ausente, incompleto, ambiguo y vacío | Solo la ausencia válida admite excepción documentada |
| A05 | Exportar propuestas de las tres modalidades | Se crea antes de revisión; no asigna fecha ni afirma registro en RUS |
| A06 | Abrir propuestas en Excel 2010 | Sin reparación; conserva hojas, fórmulas, estilos, anchos, filtros, OB y Medidas vencidas; excluidos visibles y coloreados |
| A07 | Reordenar filas, editar observación/TT/CC/RES, completar fecha y reimportar | Identidad por ID; detecta duplicados, archivo ajeno y fechas inválidas |
| A07b | Reimportar solo una parte del lote y luego completar el resto | Cada fila conserva archivo/hash; las ausentes quedan pendientes y no se puede congelar antes de completar el lote |
| A08 | Cumplimiento sin cruce: cargar constancia, documentar excepción y congelar | Responsable/motivo/hash quedan en snapshot; no congela sin excepción |
| A09 | Preparar cada familia de correo | Grupo y adjunto correctos; CC institucional una vez; contenido coincide con plantilla revisada |
| A10 | Guardar borrador en Outlook clásico y simular resultado incierto | Nunca envía; usa cuenta/carpeta elegidas; guarda recibo o exige conciliación sin duplicar |
| A11 | Preparar cada matriz Word autorizada | Datos de una sola fila; tribunal correcto; sin variables; archivo no firmado y no sobrescrito |
| A12 | Contar un intervalo conocido de Enviados y exportar | Límites inclusivos; cuenta comparable; omitidos/errores/truncamiento visibles |
| A13 | Ejecutar 100/1.000/10.000 filas y cerrar/reabrir | Interfaz responde; lote y constancia se recuperan; tiempos quedan registrados |

| A14 | Guardar constancia y volver a exportar el libro final | OBSERVACION, FECHA_OBS, TT, CC y RES coinciden con la revisión congelada |
| A15 | Cerrar NuRus y usar Retomar lote; mover la entrada original | Recupera el mismo ID, la copia de trabajo y el snapshot sin volver a analizar |
| A16 | Dos hojas de cruce válidas; misma identidad con fechas futuras distintas | Selección explícita de hoja; conflicto por fila con referencias; no inventa C-10 |
| A17 | Intercambiar solo los IDs de dos registros en Excel | Reimportación rechazada por identidad distinta |

## Cierre

No declarar aceptación si existe un P0, si Excel solicita reparar el libro, si falta una fila, si aparece un envío, si un borrador puede duplicarse tras resultado incierto o si el producto no coincide con el contenido aprobado. Registrar el conflicto y su evidencia; corregir; repetir el caso afectado y sus dependencias.
