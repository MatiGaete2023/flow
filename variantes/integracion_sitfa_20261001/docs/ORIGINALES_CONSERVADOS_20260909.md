# Conservación de libros originales — NuRus
Fecha: 9 de septiembre de 2026.
Código: `67a23b7cf289b2e4adefa3b80e53049964ef03d0`.
Rama: `implementacion-plan-2026-09-08`.

## Resultado
El flujo WorkController.analyze conserva los bytes originales del Excel en SQLite, junto con el registro del lote. El lector procesa esos mismos bytes en memoria; su SHA-256 identifica la copia conservada. No se guardan los Excel de usuarios en GitHub.

La tabla workbook_sources conserva una sola copia por hash. Una nueva importación del mismo libro crea otro lote, pero reutiliza el contenido idéntico. El API get_original_workbook permite recuperar y verificar los bytes incluso si la ruta externa desaparece. Puede utilizar el hash de un snapshot histórico para obtener la referencia de origen congelada.

## Fallos abordados
1. **Pérdida de la copia temporal.** Antes, el lector eliminaba la copia al terminar y solo persistían valores de filas. Ahora transmite los bytes capturados al almacenamiento.
2. **Divergencia entre lectura y archivo externo.** Cambiar la ruta externa durante el análisis no cambia lo que se procesa o guarda. La prueba sustituye el archivo antes de abrir el lector de pandas y comprueba que el lote y los bytes siguen correspondiendo a la captura inicial.
3. **Persistencia parcial.** Un fallo al insertar una fila revierte el lote y su nueva copia de origen en la misma transacción.
4. **Contenido inconsistente.** Se comprueba SHA-256 al guardar y hash/tamaño al recuperar. UPDATE y DELETE ordinarios sobre los originales se rechazan mediante triggers.
5. **Migración sin recuperación.** La versión 4 crea un respaldo SQLite previo a migrar una base anterior; no degrada versiones posteriores desconocidas.
6. **Reconstrucción falsa de datos antiguos.** Cuando faltan bytes conservados, el API informa que se debe reimportar. No vuelve a leer automáticamente la ruta externa ni presume que corresponde al contenido histórico.

## Verificación
73 pruebas aprobadas en Linux (3,16 s) y 73 en Windows (60,37 s), sobre 67a23b7. Evidencia: [Actions 34360067190](https://github.com/MatiGaete2023/NuRus/actions/runs/34360067190).

Pruebas nuevas: recuperación tras modificación, eliminación y reapertura; sustitución externa durante lectura; deduplicación; rechazo de hash incorrecto; rollback de lote y copia; protección frente a UPDATE/DELETE; detección de corrupción; tratamiento explícito de lotes antiguos sin bytes; respaldo v3→v4; hash del lector y omisión del contenido binario en repr.

## Actualización y uso
1. Cerrar las otras instancias que usen la base antes de actualizar la aplicación.
2. Actualizar desde la rama de trabajo indicada. Este cambio no añade dependencias ni requiere un servicio externo para procesar libros.
3. Abrir la aplicación y comprobar el respaldo local con sufijo `.pre-v4-<identificador>.bak` si había una base anterior.
4. Importar el Excel mediante el flujo normal y revisar el lote. La captura se conserva automáticamente al registrar el resultado.
5. Mantener base y respaldos fuera de GitHub: ahora contienen también los libros completos y sus datos.
6. Si se informa que un lote antiguo no tiene bytes originales, reimportar el archivo acreditado. Si su contenido cambió, se tratará de un origen distinto; no atribuirle el hash anterior.
7. Ante corrupción o errores de migración, conservar los archivos y recuperar una copia de respaldo con la aplicación cerrada. No alterar hashes ni user_version manualmente.

## Límites y siguiente fase
- Se ha probado la conservación exacta de libros XLSX sintéticos, incluida una hoja OB con fórmula y ancho. La igualdad byte a byte acredita la captura de esos archivos, no la fidelidad de una futura exportación modificada.
- El lector admite XLS/XLSX/XLSM según sus dependencias existentes; este cambio no acredita por sí solo exportación fiel en esos tres formatos ni ejecución en Excel 2010.
- La base crecerá según el tamaño de los libros distintos. La captura usa memoria proporcional al archivo; queda sujeto al límite de tamaño configurado en el lector. No se ha medido carga con los archivos institucionales.
- Los triggers y el hash detectan o impiden operaciones ordinarias, pero no constituyen una garantía frente a modificaciones administrativas coordinadas del esquema y de la evidencia.
- La API de persistencia conserva la posibilidad de guardar evaluaciones sintéticas sin bytes; el flujo normal de importación sí los entrega. El exportador fiel debe exigir que existan antes de producir una salida.
- No hay todavía una opción de interfaz para recuperar el libro archivado. Tampoco se ha sustituido el exportador tabular.
- Continúan pendientes la exportación fiel con excluidos coloreados, excepción documentada de Cumplimiento, matrices acreditadas y aceptación institucional.

[L-SIGUIENTE]
Implementar el exportador sobre una copia de los bytes conservados, con observaciones y colores por fila física, y pruebas de preservación de las partes originales. Diferenciar explícitamente el backend nativo de cualquier alternativa con fidelidad reducida.

