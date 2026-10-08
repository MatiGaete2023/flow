# NuRus: cierre de aprobación y protección del origen
Fecha de revisión: 9 de septiembre de 2026.
Rama: `implementacion-plan-2026-09-08`.

## Resultado y evidencia
La aprobación del lote y la creación del snapshot se ejecutan en una única transacción. Los productos y las exportaciones tabulares consultan los registros congelados. Las vistas previas antiguas no pueden registrarse como productos de una nueva aprobación.

Código: `bef83f7298267fce3215b37ad1668e932cbaa530`.
Verificación: [Actions 34287905754](https://github.com/MatiGaete2023/NuRus/actions/runs/34287905754): 58 pruebas aprobadas en Linux y 58 en Windows.

Se detectó y corrigió además una exportación potencialmente destructiva: `overwrite=True` permitía elegir el propio libro de entrada. La comprobación nueva rechaza rutas equivalentes y archivos con la misma identidad antes de crear la salida.
Código: `1b931961192d0cb7dba2b7fc484f76b6fc1e406f`.
Verificación: [Actions 34357265914](https://github.com/MatiGaete2023/NuRus/actions/runs/34357265914): 62 pruebas aprobadas en Linux (4,59 s) y 62 en Windows (20,95 s). Ambos trabajos terminaron correctamente.

## Hallazgos y cambios aplicados
| Problema | Consecuencia anterior | Solución y comprobación |
|---|---|---|
| Aprobación en dos transacciones | Filas aprobadas sin snapshot si fallaba el segundo paso | Una transacción con bloqueo de escritura; fallos inyectados en snapshot y lote revierten ambos y las decisiones |
| Historial basado en filas editables | Una revisión posterior podía cambiar los datos consultados para generar productos | Payload histórico completo, hash canónico y lectura desde approved_snapshots |
| Vista previa antigua | Podía asociarse a la aprobación vigente aunque sus datos fueran de otra revisión | Product conserva source_snapshot_hash; persistencia rechaza discrepancias |
| Reaprobación sin cambios | Riesgo de generar estados históricos redundantes | Hash estable, una sola copia y fecha de aprobación conservada |
| Migración sin respaldo | Actualización sin copia recuperable de la base anterior | Respaldo SQLite antes de migrar a versión 3 y rechazo de versiones posteriores desconocidas |
| Destino igual al origen | Sobrescritura del libro original al confirmar overwrite | Resolución de rutas y samefile; pruebas para ruta directa, relativa y enlace duro |
| Base antigua con hash sin payload | Imposibilidad de acreditar qué contenido había sido aprobado | Bloqueo explícito hasta nueva revisión y aprobación; no se inventa el histórico |

Los triggers impiden UPDATE y DELETE ordinarios sobre la tabla de snapshots; el hash verifica el payload al leerlo. Esto no es una garantía frente a alguien con capacidad administrativa para alterar el esquema o sustituir toda la base.

## Incorporación y operación paso a paso
1. Utilizar la rama indicada y conservar una copia independiente de los datos antes de actualizar la instalación.
2. Cerrar otras instancias de NuRus que utilicen la misma base durante la actualización.
3. Al abrir una base anterior a v3, comprobar que se creó el archivo de respaldo con sufijo `.pre-v3-<identificador>.bak`. El respaldo contiene datos privados y debe permanecer fuera de GitHub.
4. Importar y revisar un lote de prueba. Resolver los bloqueos de cada fila antes de aprobar.
5. Aprobar el lote. La operación guarda decisiones, contenido congelado y hash juntos; cualquier fallo transaccional debe dejar el lote sin una aprobación parcial.
6. Preparar una vista previa desde esa aprobación. Si se revisan datos y se vuelve a aprobar, preparar nuevamente el producto; no reutilizar la vista previa anterior.
7. Exportar a una ruta distinta del origen. La autorización de sobrescritura solo habilita reemplazar otro destino; nunca el archivo fuente identificado por el lote.
8. Ante una base antigua sin payload histórico, revisar el estado actual y aprobar de nuevo expresamente. Esa nueva aprobación no acredita el contenido de una aprobación pasada.

## Problemas operativos y respuesta
- **Base bloqueada:** cerrar la instancia que mantiene la escritura y reintentar la acción después de resolver el bloqueo. No eliminar la base ni sus archivos auxiliares.
- **Migración o respaldo fallidos:** detener el uso de esa instalación, conservar el error y los archivos. Para volver al respaldo, cerrar todas las instancias y realizar la restauración en una copia; no reemplazar una base abierta.
- **Versión de base posterior:** utilizar la aplicación compatible con esa versión; no bajar manualmente user_version.
- **Vista previa desactualizada:** abrir el lote aprobado vigente y generar una nueva.
- **Destino de origen rechazado:** elegir un archivo nuevo en una carpeta de salida.
- **Ruta de origen ausente:** reimportar el archivo con la versión vigente; la exportación no omite silenciosamente la comprobación.
- **Snapshot histórico inexistente:** revisar y aprobar otra vez; conservar los productos anteriores como históricos sin atribuirles un payload recuperado.

## Pendientes materiales para completar NuRus
1. **Conservar el libro original en bytes.** Capturar los bytes durante la importación, calcular su SHA-256, almacenarlos de forma inmutable fuera del repositorio y referenciarlos desde el lote. Validar que lectura, hash y copia correspondan a la misma entrada. Comprobar que retirar o modificar el archivo externo no cambia los bytes conservados.
2. **Exportación Excel de fidelidad completa.** Partir de esa copia conservada, aplicar solo las intervenciones autorizadas y entregar Espera, Cumplimiento e Informes. Retener excluidos con colores y no procesar OB ni Medidas vencidas. Comprobar hojas, fórmulas, estilos, anchos, filtros y objetos mediante Excel institucional. El exportador tabular actual no satisface este requisito.
3. **Excepción de hoja cruzada.** Incorporar aprobación explícita con responsable, motivo y condición excepcional en el snapshot, con pruebas de rechazo sin justificación y de persistencia de la excepción. No tratarla como una aprobación silenciosa.
4. **Plantillas, adjuntos y proyectos definitivos.** Contrastar el catálogo con matrices y manual; conservar versiones y permitir edición fuera del código. Faltan fuentes acreditadas para Tomé y pide-cuenta de informes de Laja: no completar sus textos por analogía.
5. **Aceptación institucional.** Probar interfaz, Excel 2010, Word y Outlook clásico en Windows 10, con copias y borradores. Verificar CC obligatoria, adjuntos y ausencia de envío; probar resultados inciertos y contador de solo lectura.

## Revisión final prevista
Cada fase debe incorporar pruebas del fallo corregido y de sus dependencias. Antes de declarar compatibilidad productiva:
- ejecutar la suite del commit exacto y conservar su enlace;
- comparar salidas con referencias originales y con la salida representativa de RUS Engine;
- comprobar conservación integral del libro y exclusiones coloreadas;
- acreditar la aceptación nativa de Excel y Outlook;
- resolver las matrices faltantes y cualquier diferencia de resultados.

[A-LIMITE]
Las pruebas de CI utilizan datos sintéticos. Windows en GitHub Actions no equivale a aceptación en Windows 10 con Excel 2010 y Outlook institucional. No se generaron productos de causas ni se enviaron mensajes. El código local anterior inaccesible no se considera recuperado ni publicado.

[L-SIGUIENTE]
Inspeccionar la captura del archivo en WorkController.analyze y el modelo del lote para implementar conservación de bytes originales vinculados al hash.

