# RES categórico y cierre de Resoluciones — 22 de septiembre de 2026

Introducido en **0.4.0.dev10** y vigente en **0.4.0.dev11**.

## Decisión

La columna humana `RES` deja de recomendar números para las copias nuevas y pasa a expresar directamente el tipo de proyecto:

- vacío: no corresponde proyecto;
- `PC_IE`: pide cuenta por ingreso efectivo;
- `PC_INFO`: pide cuenta por informe;
- `NOMENCL`: proyecto de nomenclatura.

La copia Excel generada agrega una lista desplegable compatible con Excel 2010. Al volver a incorporar una copia que contiene la columna `RES`, esa columna es autoritativa incluso si todas sus celdas están vacías: vacío significa expresamente que no corresponde proyecto. La decisión humana de `RES` tiene prioridad sobre la inferencia desde observación o regla.

## Compatibilidad

Las planillas antiguas no se invalidan. Marcas como `1`, `X`, `sí` o equivalentes siguen indicando que existe proyecto; el Asistente puede inferir su tipo con la lógica histórica. `0` sigue significando no marcado.

Un texto no reconocido, por ejemplo `PC_INOF`, no se corrige ni aproxima: la fila queda con advertencia **RES no reconocido** y no obtiene un tipo inventado.

## Interfaz

La columna Origen de Resoluciones diferencia:
- `Definido en RES`;
- `RES antiguo · tipo inferido`;
- `Sugerencia automática revisable`;
- `Ajustado manualmente en Resoluciones` cuando el usuario cambia el selector de la ventana.

El selector manual permanece como corrección final y afecta únicamente las filas seleccionadas.

## Criterio de diseño

Este cambio reduce inferencia y mantiene el flujo personal: Motor → revisión Excel → RES explícito → proyecto Word. No agrega un diseñador de reglas ni un nuevo paso obligatorio.

## Compatibilidad Excel nativa

La validación nativa no consulta `Application.International`, porque pywin32 puede exponer esa propiedad de forma no invocable en algunas instalaciones de Excel. La lista se alimenta mediante una hoja técnica oculta y el nombre de libro `NURUS_RES_TIPOS`, evitando depender del separador regional y manteniendo compatibilidad con Excel 2010.
