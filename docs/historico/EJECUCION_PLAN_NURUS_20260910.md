# Ejecución del plan NuRus

Fecha: 10 de septiembre de 2026. Versión: `0.3.0.dev2`.

## Resultado implementado

El flujo operativo quedó separado en cuatro actos con significado distinto:

1. el motor analiza y propone;
2. NuRus exporta una copia de trabajo sin atribuirle revisión humana;
3. la persona revisa cada registro en RUS, deja constancia en Excel y devuelve ese archivo;
4. NuRus valida, registra y congela la constancia antes de crear productos.

La observación oficial continúa siendo la registrada en RUS. El Excel y SQLite conservan la constancia, procedencia y versiones necesarias para correos, estadísticas y proyectos.

## Cambios ejecutados

### Entrada y reglas

- Informes distingue `FECHA INGRESO` de `FEC. INGRESO EFECTIVO`.
- Las filas estructurales se conservan físicamente, pero no se transforman en casos.
- La detección de cabecera reutiliza la lectura inicial.
- Una hoja de cruce incompleta no se considera cruce válido.
- Fechas negativas, ingreso sin fecha proyectada y ausencia de columna FAE generan incidencias en vez de conclusiones falsas.

### Propuesta y constancia

- La propuesta se exporta antes de cualquier aprobación.
- Se incorporan `NURUS_ID_REGISTRO`, `NURUS_PROPUESTA`, `OBSERVACION`, `FECHA_OBS`, `TT`, `CC`, `RES` y estado.
- Las filas excluidas permanecen y se resaltan.
- El retorno valida hash de evaluación, hash de origen, hoja, IDs, duplicados, fecha y observación; los ausentes se informan y permanecen pendientes.
- La importación exige responsable y confirmación expresa de revisión/registro en RUS.
- SQLite v7 guarda hash y nombre de cada Excel devuelto tanto en el lote como en cada fila incorporada, además de responsable, fecha de confirmación y campos administrativos.
- Se admiten devoluciones parciales: las filas ausentes permanecen pendientes y el lote no puede congelarse hasta incorporarlas todas.
- La excepción de Cumplimiento conserva responsable/motivo y debe existir antes de congelar cuando no hay cruce.

### Productos integrados

- Catálogo versionado de correos y matrices históricas de resolución.
- Agrupación por tribunal/programa desde registros de la constancia.
- Importación de contactos con vista previa y alias exactos.
- Nóminas adjuntas limitadas al grupo, con hash.
- Revisión final conjunta de destinatario, CC, asunto, cuerpo y adjuntos.
- Guardado de borradores Outlook sin ruta de envío; recibo `EntryID`/`StoreID` y estado incierto que impide reintento automático.
- Proyectos `.docx` no firmados, vinculados a producto y snapshot.
- Si cambia la constancia, los productos aprobados contra el snapshot anterior no pueden materializarse como borrador o Word.
- Contador de Enviados de solo lectura con período, cuenta, omitidos, errores y límite.
- Estadísticas por `FECHA_OBS`; TT/CC/RES se distribuyen por su valor original, sin inventar clasificación.

### Interfaz e instalación

- Secuencia visible: Analizar → Exportar propuestas → Cargar constancia → Preparar.
- Operaciones de Office/archivos se ejecutan fuera del hilo gráfico mediante un trabajador serial.
- Se agregaron accesos a constancia final, correos, proyectos, estadísticas, contactos y contador.
- El instalador reutiliza una `.venv` válida sin requerir `py`, acepta `NURUS_PYTHON_EXE` y puede usar `paquetes\` sin Internet.

## Límites que no deben ocultarse

- La preservación real de `.xls`, fórmulas, estilos, filtros y conexiones depende de Excel de escritorio; debe aceptarse en Excel 2010.
- Outlook clásico y la cuenta institucional no están disponibles en la CI local.
- Las matrices históricas quedaron incorporadas y editables, pero su aprobación jurídica/visual definitiva corresponde a la revisión institucional.
- El significado cerrado de cada valor TT/CC/RES debe comprobarse contra la matriz vigente. Hasta entonces el reporte no los reinterpreta.
- NuRus no comprueba remotamente el sistema RUS: conserva la declaración responsable y el archivo devuelto.

## Verificación automatizada

La suite incluye lectura, reglas, snapshots, migraciones, propuesta/constancia, exportación, productos, seguridad Outlook, contactos, Word, contador, estadísticas e instalación.

Resultado local del 10 de septiembre de 2026:

- `100 passed` tras incorporar la invalidación de productos obsoletos;
- compilación de `src/` y `tests/` sin errores;
- catálogos JSON válidos y presentes en el wheel `0.3.0.dev2`;
- búsqueda estática sin llamadas `.Send()`;
- `AGOSTO.xlsx`: Espera 95; Cumplimiento 760 (756 revisables y 4 excluidos); Informes 300, sin falsos registros estructurales;
- `RUS_CUMPLIMIENTO_20260904_114132_856292.xlsx`: 255 revisables y 7 excluidos en Cumplimiento; la ausencia de cruce queda advertida y requiere excepción;
- propuesta portable de las tres modalidades: hojas originales presentes, 30/30 fórmulas conservadas, columnas administradas incorporadas y excluidos marcados mediante formato condicional.

GitHub Actions confirmó las 100 pruebas, instalación, dependencias y compilación en Ubuntu y Windows sobre el commit `c1122c4d966eca4a80c50698dd383b914e9f9245`. La fidelidad nativa y el comportamiento real de Excel/Outlook siguen sujetos al protocolo institucional.

## Próximo cierre

Ejecutar `ACEPTACION_INSTITUCIONAL_20260910.md` en el PC objetivo y registrar evidencia. Cualquier fallo debe corregirse y repetir solo el caso y sus dependencias antes de emitir el dictamen final.
