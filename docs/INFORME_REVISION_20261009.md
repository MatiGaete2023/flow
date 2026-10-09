# Informe de revisión técnica — NURUS / CSMP en flow

**Fecha:** 9 de octubre de 2026. **Código revisado:** `45757c7d4acc47e96566528519c828fa6ba696d6`, rama `feat/ejecucion-plan-20261008`, [PR #1](https://github.com/MatiGaete2023/flow/pull/1).

La revisión encontró **ocho problemas reproducidos con datos sintéticos y una brecha en la verificación de futuras entregas**. Cinco tienen prioridad alta por afectar fidelidad, decisiones humanas o cobertura; cuatro tienen prioridad media. Se proponen además tres mejoras de ingeniería. Las correcciones funcionales quedan pendientes: este trabajo entrega diagnóstico, evidencia y criterios para verificarlas.

Las pruebas actuales del área examinada pasan, pero no detectan estos casos. Esto limita lo que puede concluirse de las anteriores comprobaciones satisfactorias del MVP. Se recomienda corregir los hallazgos de prioridad alta antes de promover una versión institucional.

## Alcance y método

Se revisaron el contrato de intercambio, productor del Descargador, importador del Asistente, cronología, revisión y recalculado de filas, productos y solicitudes, persistencia, actas y workflows de Windows. Se contrastaron con los documentos 02/04/05, la implementación y `CONTINUAR.md`. Se inspeccionaron también controles de fórmulas y parsers. No es una auditoría exhaustiva de todas las variantes históricas del repositorio.

- Ocho sondas ejecutaron funciones reales con datos exclusivamente ficticios; sus productos temporales se eliminaron al terminar.
- Regresión focalizada: **68 pruebas aprobadas** del Asistente; **19 pruebas y 2 subpruebas aprobadas** del Descargador. No se volvió a ejecutar toda la suite ni a compilar Windows en esta revisión.
- Se inspeccionaron los workflows y se confirmó que la ejecución anterior `37880150415` terminó correctamente. Esa evidencia no cubre los nuevos casos reproducidos aquí.
- No se accedió a SITFA/RUS, Outlook ni Office institucional. No se modificaron reglas, código productivo ni adjuntos originales.

Evidencia: [resultados de las sondas](evidencia_revision_20261009/sondas.json), [tests Asistente](evidencia_revision_20261009/tests_asistente.txt), [tests Descargador](evidencia_revision_20261009/tests_descargador.txt), [contexto de ejecución](evidencia_revision_20261009/contexto.json). Herramienta reproducible: [flow_review_probes.py](../tools/flow_review_probes.py).

Se observaron controles útiles que deben conservarse: el lector ZIP verifica hashes y límites sin extraer archivos; los libros generados fuerzan texto en las celdas externas; las CT permanecen desactivadas en producción; `registration.submit` rechaza escritura institucional. Las sondas se centraron en los límites de esos controles y en la coherencia del flujo.

## Hallazgos priorizados

| ID | Prioridad | Hallazgo | Comprobación |
|---|---|---|---|
| H01 | Alta | Texto largo se trunca antes de validar su longitud | Reproducido: 33.000 → 32.767 caracteres |
| H02 | Alta | Recalcular una fila deja hallazgos de la evaluación anterior | Reproducido: regla actual E06, hallazgo guardado E05 |
| H03 | Alta | Captura equivalente con otro digest no conserva decisiones en el trabajo nuevo | Reproducido con mismos registros y fuentes |
| H04 | Alta | Se acepta cobertura contradictoria y se omiten solicitudes necesarias | Reproducido: fila/consulta parcial, tabla completa |
| H05 | Alta | Calendario parcial se exporta como tabla completa | Reproducido desde un lote incompleto |
| H06 | Media | Acta acepta conteos imposibles en un resultado PASÓ | Reproducido: 1 observado, 200.000 coincidentes |
| H07 | Media | Se genera una solicitud que el propio lector rechaza por tamaño | Reproducido: 4.908.375 bytes |
| H08 | Media | Advertencias de identidad visible se pierden al exportar bitácoras | Reproducido con nombre visible discrepante |
| H09 | Media | La comprobación completa de desinstalación no protege nuevas entregas | Inspección de los dos workflows |

### H01 — Validar texto antes de entregarlo a openpyxl

**Ubicación:** [capture_input.py, línea 74](https://github.com/MatiGaete2023/flow/blob/45757c7d4acc47e96566528519c828fa6ba696d6/src/nurus/personal/capture_input.py#L74).

**Evidencia e impacto.** `sheet.append(...)` ocurre antes del control de longitud. openpyxl recorta cadenas a 32.767 caracteres al asignarlas a una celda. Una captura válida con un nombre ficticio de 33.000 caracteres se importa sin error, pero el libro base contiene 32.767. El JSON fuente conserva el texto completo: el problema es la divergencia silenciosa del libro derivado, no la destrucción del ZIP original. Incumple la invariancia de texto sin truncamiento oculto del documento 02, §7.

**Cómo corregirlo.** Comprobar cada valor original antes de crear/asignar la celda, contando unidades UTF-16. Para campos que no puedan representarse íntegramente, rechazar la conversión con un mensaje claro o crear una hoja de fragmentos y una referencia explícita, siguiendo la solución de bitácoras. Centralizar esta política para todos los libros nuevos.

**Prueba de aceptación.** Casos de 32.767/32.768 caracteres ASCII y caracteres fuera del plano básico. Un valor excesivo debe rechazarse antes de cambiar el trabajo, o reconstruirse exactamente desde los fragmentos. Nunca debe aceptarse una diferencia silenciosa entre fuente y salida.

### H02 — Actualizar los hallazgos al editar y recalcular

**Ubicación:** [record_edits.py, línea 189](https://github.com/MatiGaete2023/flow/blob/45757c7d4acc47e96566528519c828fa6ba696d6/src/nurus/personal/record_edits.py#L189); construcción inicial en [capture_input.py, línea 118](https://github.com/MatiGaete2023/flow/blob/45757c7d4acc47e96566528519c828fa6ba696d6/src/nurus/personal/capture_input.py#L118).

**Evidencia e impacto.** Cambiar `espera` de 40 a 0 mediante `record_edits.apply` cambia `row.rules` de `ESPERA.E05_PROYECTO_Y_CORREO` a `ESPERA.E06_SIN_RESOLUCION`. Sin embargo, `capture_context[row.id]['findings']` conserva E05. La pantalla de evidencia usa esos hallazgos y puede explicar una regla que ya no produjo la observación vigente. Los hechos y acciones guardados también pueden quedar obsoletos.

**Cómo corregirlo.** Extraer un servicio para regenerar los hallazgos tradicionales de una fila. Usarlo después de recalcular, sincronizar cambios externos y deshacer ediciones, preservando los hallazgos contextuales y las decisiones humanas. Construir y confirmar conjuntamente fila y contexto para evitar actualizaciones parciales.

**Prueba de aceptación.** Editar, deshacer, guardar/recuperar y refrescar una edición Excel. Las reglas, hechos y acciones de los hallazgos deben corresponder a la evaluación vigente, sin duplicados ni borrado de observaciones humanas.

### H03 — Conservar decisiones ante capturas complementarias compatibles

**Ubicación:** [capture_input.py, línea 104](https://github.com/MatiGaete2023/flow/blob/45757c7d4acc47e96566528519c828fa6ba696d6/src/nurus/personal/capture_input.py#L104), [app_base.py, línea 352](https://github.com/MatiGaete2023/flow/blob/45757c7d4acc47e96566528519c828fa6ba696d6/src/nurus/personal/app_base.py#L352). Requisito explícito: documento 04, §idempotencia, línea 385.

**Evidencia e impacto.** Cambiar únicamente `downloader.commit` genera otro digest ZIP. Manteniendo `capture_id`, IDs de registros y todas las fuentes iguales, el nuevo trabajo pierde la observación revisada y la exclusión del primer registro. La GUI archiva la sesión anterior en `sesion/anteriores`, por lo que no se afirma pérdida irreversible; sí se pierde continuidad en el trabajo activo y se puede volver a incluir una fila excluida. Una captura complementaria real también cambia el digest.

**Cómo corregirlo.** Separar identidad de bytes del paquete e identidad de trabajo. Conciliar registros por identidad remota comprobada y discriminadores de modalidad/tipo/procedencia, detectar conflictos y trasladar decisiones solo cuando exista correspondencia inequívoca. Los IDs actuales del productor incorporan hash de fuente y posición: no bastan como clave estable entre capturas. Presentar conflictos para revisión, mantener el archivo previo e invalidar productos dependientes de fuentes cambiadas.

**Prueba de aceptación.** Reexportación equivalente, captura con nueva bitácora, cambio de orden de lotes, dos ingresos de la misma causa y datos incompatibles. Conservar decisiones en los casos equivalentes; pedir conciliación en los ambiguos y marcar productos anteriores obsoletos.

### H04 — Comprobar coherencia de cobertura y solicitar por ingreso

**Ubicación:** [contract.py, línea 165](https://github.com/MatiGaete2023/flow/blob/45757c7d4acc47e96566528519c828fa6ba696d6/src/nurus/capture/contract.py#L165) y línea 208; [capture_products.py, línea 9](https://github.com/MatiGaete2023/flow/blob/45757c7d4acc47e96566528519c828fa6ba696d6/src/nurus/personal/capture_products.py#L9).

**Evidencia e impacto.** El validador acepta una consulta con una de dos páginas, filas de bitácora `PARCIAL` y estado agregado `bitacoras=COMPLETA`. La generación de solicitudes consulta solo `table_states` y produce **cero solicitudes de bitácora para ese ingreso parcial**. El estado global puede ocultar faltantes o, si es parcial, solicitar otra vez fuentes suficientes de otros ingresos. El contexto de producción continúa NO_EVALUABLE; el defecto confirmado afecta cobertura y planificación de antecedentes.

**Cómo corregirlo.** Definir la relación entre consulta, tabla e identidad, incluidos resultados vacíos comprobados. Validar que filas y agregados no afirmen mayor cobertura que sus consultas. Calcular solicitudes por fuente e ingreso usando ese alcance; usar el agregado solo como resumen. Conservar los distintos estados de ausencia, fallo y vacío.

**Prueba de aceptación.** Paquete mixto con un ingreso completo, uno parcial, uno fallido y otro vacío comprobado. Rechazar contradicciones y solicitar únicamente los antecedentes insuficientes, sin confundir falta de filas con vacío comprobado.

### H05 — Agregar correctamente la cobertura de calendarios

**Ubicación:** [export_csmp_capture.py, línea 85](https://github.com/MatiGaete2023/flow/blob/45757c7d4acc47e96566528519c828fa6ba696d6/descargador/export_csmp_capture.py#L85).

**Evidencia e impacto.** Desde un lote de calendario con una página disponible de dos, el productor exporta consulta y filas `PARCIAL`, pero declara `calendarios=COMPLETA`. La condición que conserva parcialidad está restringida a `name=='bitacoras'`; cualquier calendario con filas termina como completo. Este caso muestra que H04 puede originarse en el productor del propio proyecto, sin editar manualmente el ZIP.

**Cómo corregirlo.** Agregar los estados de todas las consultas de calendario y de su plan pendiente, incluyendo consultas sin filas. Reutilizar una función de cobertura común y registrar los motivos de parcialidad en incidencias.

**Prueba de aceptación.** Calendario completo, vacío comprobado, incompleto con filas y lote con consulta de calendario planificada pero no ejecutada. El resumen nunca debe declarar COMPLETA mientras falten páginas o consultas de su alcance.

### H06 — Validar relaciones entre los conteos de las actas

**Ubicación:** [acceptance.py, línea 59](https://github.com/MatiGaete2023/flow/blob/45757c7d4acc47e96566528519c828fa6ba696d6/src/nurus/capture/acceptance.py#L59) y línea 76.

**Evidencia e impacto.** `validate_acta` acepta `PASÓ` con `expected=1`, `observed=1` y `matched=200000`. Valida los límites de cada número y que PASÓ no tenga fallos, pero no sus relaciones. El acta no aprueba automáticamente un gate; aun así, su comprobación de coherencia permite cifras imposibles.

**Cómo corregirlo.** Documentar la unidad y relación de los conteos. Exigir como mínimo que `matched` no supere `observed` cuando ambos estén presentes. Si matched/failed/unknown forman una partición, verificar su suma; si representan métricas distintas, separarlas expresamente. Definir cuándo una cobertura insuficiente es incompatible con PASÓ y permitir actas sin conteos donde no correspondan.

**Prueba de aceptación.** Rechazar coincidencias superiores a observados, sumas incompatibles y resultados incoherentes con sus métricas. Mantener casos válidos con conteos opcionales y evidencia sintética separada de la institucional.

### H07 — Compartir el límite de tamaño entre escritor y lector de solicitudes

**Ubicación:** [requests.py, línea 27](https://github.com/MatiGaete2023/flow/blob/45757c7d4acc47e96566528519c828fa6ba696d6/src/nurus/capture/requests.py#L27) y línea 33.

**Evidencia e impacto.** Una solicitud válida de 2.200 elementos, cada uno con un motivo de 2.000 caracteres permitidos, se escribe correctamente con **4.908.375 bytes**. `read_request` rechaza ese mismo archivo por superar 4 MiB. Se entrega al usuario un producto que el Descargador no puede consumir.

**Cómo corregirlo.** Serializar y comprobar el tamaño UTF-8 antes de publicar. Compartir constantes y límites entre validación, escritura y lectura; decidir si se permite dividir una solicitud grande en partes con identificación de origen. El mensaje debe explicar cómo reducir el alcance.

**Prueba de aceptación.** Todo documento que `write_request` publique debe ser legible por `read_request`. Probar el umbral exacto, caracteres multibyte y exceso; el exceso no debe dejar archivo final ni sobrescribir uno existente.

### H08 — Transportar las advertencias de bitácora hasta el Asistente

**Ubicación:** [diaries.py, línea 27](https://github.com/MatiGaete2023/flow/blob/45757c7d4acc47e96566528519c828fa6ba696d6/src/nurus/capture/diaries.py#L27), [export_csmp_capture.py, línea 73](https://github.com/MatiGaete2023/flow/blob/45757c7d4acc47e96566528519c828fa6ba696d6/descargador/export_csmp_capture.py#L73), [capture_input.py, línea 101](https://github.com/MatiGaete2023/flow/blob/45757c7d4acc47e96566528519c828fa6ba696d6/src/nurus/personal/capture_input.py#L101).

**Evidencia e impacto.** El lector detecta «nombre visible de bitácora distinto al listado». El exportador no transporta `snapshot['warnings']`, deja vacío el error de consulta y no crea una incidencia de esa discrepancia. La bitácora llega con identidad VERIFICADA y el operador del Asistente deja de ver la advertencia. Las incidencias `IDENTIDAD_INCIERTA` observadas en la sonda pertenecen al listado independiente y no sustituyen esta advertencia.

Además, una segunda sonda introduce una incidencia explícita en `incidencias.csv`: su mensaje no aparece en el trabajo guardado ni en la vista de evidencia. El importador valida esa tabla, pero no conserva sus filas. Transportar las advertencias solo hasta el ZIP no basta.

**Cómo corregirlo.** Convertir cada advertencia en incidencia estructurada con consulta e identidad afectadas; conservar la tabla en `Work` y exponer incidencias generales y por registro. Mantener la identidad remota fuerte cuando esté comprobada, distinguiéndola de discrepancias de nombre/RIT visible; no cambiarla automáticamente por comparación de nombres. Conservar asimismo advertencias de respuesta/fecha incongruentes.

**Prueba de aceptación.** Inyectar una discrepancia visible y una advertencia de respuesta: ambas deben sobrevivir lectura, exportación, importación, pantalla de evidencia y exportación de control, sin atribuirlas a otro ingreso.

### H09 — Integrar la comprobación completa del instalador en cada build

**Ubicación:** [verify-delivered-installer.yml, línea 4](https://github.com/MatiGaete2023/flow/blob/45757c7d4acc47e96566528519c828fa6ba696d6/.github/workflows/verify-delivered-installer.yml#L4), líneas 29/36; [tests.yml, línea 228](https://github.com/MatiGaete2023/flow/blob/45757c7d4acc47e96566528519c828fa6ba696d6/.github/workflows/tests.yml#L228).

**Evidencia e impacto.** La comprobación adicional espera la eliminación real de la carpeta, pero fija el run `37878632095` y SHA `6d3555a`, y solo se activa automáticamente al cambiar su propio YAML. El workflow principal sigue verificando retorno del lanzador y dato externo sin esperar la desaparición de la carpeta. Una nueva versión puede pasar el principal sin esa postcondición. Además, la comprobación histórica dejará de poder descargarse al caducar el artefacto.

**Cómo corregirlo.** Extraer la prueba de instalación/reinstalación/desinstalación a un script reutilizable y ejecutarla sobre el instalador que acaba de producir cada build. Parametrizar run y SHA cuando se quiera verificar una entrega histórica. Conservar el plazo de eliminación, datos externos, logs y comprobación del SHA del binario.

**Prueba de aceptación.** Un cambio de instalador debe ejecutar la comprobación del nuevo SHA. Simular un lanzador que retorna pero deja la carpeta: CI debe fallar. La construcción normal no debe depender de un artefacto anterior con retención limitada. Este hallazgo no invalida la comprobación ya aprobada de `6d3555a`.

## Mejoras adicionales

| ID | Prioridad | Evidencia y efecto | Mejora propuesta y forma de comprobarla |
|---|---|---|---|
| M01 | Media | En `capture_input.py`, cada fila usa `sheet.max_row` y un acceso de fila que consulta `max_column`. En openpyxl instalado ambas propiedades recorren las celdas acumuladas; esa construcción tiene trabajo cuadrático. El bucle tampoco comprueba cancelación. Los benchmarks guardados llegan a 2.000 registros, frente al máximo contractual de 200.000 filas agregadas. | Usar índices conocidos de fila/columna o celdas preparadas, evitar recalcular dimensiones en cada iteración y añadir checkpoints periódicos. Medir tiempo/memoria con lotes mayores, fijar un límite operativo basado en esos resultados y comprobar cancelación sin modificar el trabajo previo. No se extrapolan tiempos de producción desde el benchmark existente. |
| M02 | Media | `pyproject.toml` admite rangos amplios y CI instala `pyinstaller` sin versión fija. Los SHA identifican fuente, pero reconstruirla posteriormente puede resolver otras dependencias. El Descargador fija varias versiones, aunque no todas; no hay lockfile completo de construcción. | Conservar un conjunto probado de restricciones/lock y sus hashes por versión Python/plataforma, registrar dependencias en cada artefacto y actualizarlo mediante CI. Verificar que una reconstrucción use el mismo inventario, sin prometer igualdad binaria si existen timestamps u otros factores variables. |
| M03 | Baja | El contrato y su validador contienen muchas condiciones compactas en una línea; GUI, reglas y productos mantienen varias representaciones de fila/contexto. H02 y H04 muestran el costo de mantener invariantes en lugares distintos. | Refactorizar gradualmente validación de consulta, fila y agregado; crear estructuras tipadas y un servicio único de actualización de hallazgos. Añadir análisis estático al código nuevo y pruebas de invariantes, evitando una reescritura de reglas tradicionales. Criterio: mismas salidas golden y errores precisos en cada frontera del contrato. |

## Omisiones conocidas que siguen diferidas

Estas dependencias estaban documentadas antes de la revisión; no se presentan como regresiones descubiertas ni se declaran resueltas por datos ficticios.

| Pendiente | Cómo completarlo cuando exista evidencia/entorno |
|---|---|
| Historia SITFA y PDF por trámite (P9) | Obtener contrato remoto observado con autorización; especificar identidad causa/ingreso, paginación y estados; implementar primero historia tabular y después PDF por selección; probar identidad, límites y fidelidad. |
| Reglas contextuales ampliadas y aceptación (P7/P10/P12) | Convertir cada regla pendiente en casos con fuentes suficientes/insuficientes, contradicciones y empates; desarrollar el motor y aprobar individualmente la regla antes de activarla. Mantener NO_EVALUABLE donde falte evidencia. |
| Excel 2010, Outlook 365 y Windows institucional | Ejecutar las actas preparadas sobre el artefacto identificado: abrir/revisar libros, comprobar plantillas y borradores, migración y recuperación. La CI Windows existente no sustituye estas pruebas. El usuario difirió el acceso institucional. |

## Orden de corrección recomendado

1. **Fidelidad y coherencia:** H01, H02, H04 y H05. Añadir primero los casos negativos que hoy faltan y corregir conjuntamente productor/consumidor para cobertura.
2. **Continuidad del trabajo:** H03, con conciliación explícita y pruebas de conflictos; H08 para mantener advertencias visibles.
3. **Entrega y evidencia:** H06, H07 y H09; después M01/M02. Cada corrección debe conservar la paridad y registrar su propio SHA y pruebas.
4. **Mantenimiento gradual:** M03 y, cuando el usuario disponga de entorno, las aceptaciones institucionales diferidas.

Reproducción de las sondas, desde la raíz del repositorio y con sus dependencias de desarrollo instaladas:

```sh
python tools/flow_review_probes.py
```

La herramienta imprime observaciones JSON y termina con código 0 cuando pudo ejecutarlas; **no significa que los defectos estén corregidos**. Las futuras pruebas de regresión deben exigir el comportamiento esperado descrito en cada hallazgo. Los enlaces de código fijan el SHA revisado para que la evidencia siga siendo interpretable tras las correcciones.
