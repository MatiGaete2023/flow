# Exportación de libro revisado desde evidencia conservada

Fecha: 9 de septiembre de 2026.  
Código: `e1f745228ce74eabca7f3a87ecd3aba62008df4e`.  
Rama: `implementacion-plan-2026-09-08`.

## Resultado

NuRus ahora crea el Excel revisado desde los bytes originales archivados y el snapshot aprobado. No vuelve a leer el archivo externo para construir la salida. Por ello, modificar, mover o eliminar el Excel que se importó no modifica la base de la exportación.

La salida conserva la extensión del archivo de origen. La hoja procesada recibe dos columnas adicionales al final: `NURUS_OBSERVACION` y `NURUS_ESTADO_REVISION`. Las filas excluidas quedan visibles y se marcan con una regla de color amarillo. Se agrega una hoja oculta `NURUS_TRAZABILIDAD` con el hash del snapshot, el hash del origen y los motivos de revisión.

Las hojas **OB** y **Medidas vencidas** se conservan en el archivo, pero NuRus no altera sus celdas ni aplica reglas sobre ellas.

## Modo de exportación

La interfaz usa el modo **nativo**, que abre una copia temporal mediante Excel de escritorio en Windows y la guarda antes de crear el destino elegido. Es el modo destinado a mantener fórmulas, estilos, anchos, filtros, validaciones, hojas, objetos y macros VBA ordinarias dentro de las capacidades de Excel.

Antes de usarlo, instala la dependencia en el entorno de NuRus:

```bat
py -3.12 -m pip install -e ".[excel-native]"
```

Esto requiere acceso a los paquetes de Python. No requiere privilegios de administrador si NuRus está instalado en un entorno virtual o en una instalación de usuario.

Existe además un modo portable interno para pruebas. Requiere confirmación explícita de fidelidad reducida y no está expuesto como opción normal de la interfaz. No usarlo para declarar conservación completa, para archivos `.xls` ni para una entrega institucional sin aceptación previa.

Los libros con conexiones externas o macros XLM se detienen antes de abrirse mediante automatización. Un libro `.xls` se admite únicamente con Excel de escritorio. La salida siempre debe conservar la misma extensión que el origen.

## Uso

1. Selecciona el Excel y ejecuta el análisis.
2. Revisa las filas, aprueba las que correspondan y excluye las restantes indicando motivo.
3. Presiona **Aprobar lote**. El snapshot queda congelado.
4. Presiona **Exportar Excel revisado** y elige un nombre de archivo nuevo.
5. Abre la copia generada en Excel y verifica las columnas de NuRus y las filas excluidas.
6. Conserva la base de NuRus y sus respaldos fuera de GitHub: contienen copias completas de los libros importados.

La salida no puede sobrescribir el archivo de origen ni un destino ya existente. Si el lote antiguo no tiene bytes archivados, NuRus pide reimportar el archivo; no reconstruye ni sustituye evidencia faltante.

## Verificación realizada

GitHub Actions ejecutó la suite sobre el código indicado:

- Linux: **77 pruebas aprobadas** en 5,28 segundos.
- Windows: **76 pruebas aprobadas y 1 omitida** en 33,94 segundos. La omitida exige Excel de escritorio real.

Evidencia: [Actions 34366711295](https://github.com/MatiGaete2023/NuRus/actions/runs/34366711295).

Las pruebas nuevas comprueban que la exportación portable de una muestra conserva las hojas, fórmulas, ocultamiento, anchos, filtros, panel congelado, validaciones y estilos evaluados; además verifica las anotaciones, el color condicional de exclusión, la trazabilidad, el uso de los bytes archivados tras eliminar el archivo externo y el bloqueo de destinos destructivos.

## Límites y aceptación pendiente

La prueba automatizada no acredita que Excel 2010 conserve todos los objetos posibles de cualquier libro institucional. Debe ejecutarse una aceptación nativa en Windows 10 con una copia de cada formato real utilizado: `.xls`, `.xlsx` y, si existe, `.xlsm`.

En cada muestra, comparar antes y después:

- hojas visibles y ocultas;
- fórmulas, valores y vínculos;
- filtros, anchos, alturas, inmovilización y validaciones;
- formatos, comentarios, imágenes, gráficos y objetos;
- macros VBA cuando proceda, sin ejecutarlas durante la exportación;
- columnas de NuRus, hoja de trazabilidad y color de las filas excluidas;
- confirmación de que OB y Medidas vencidas no cambiaron.

La excepción documentada para Cumplimiento y la aceptación institucional de Outlook siguen pendientes. La exportación no envía correos ni escribe en SATURNO.

[L-SIGUIENTE]
Implementar la excepción documentada de la hoja cruzada de Cumplimiento, con responsable, motivo y persistencia dentro del snapshot.

