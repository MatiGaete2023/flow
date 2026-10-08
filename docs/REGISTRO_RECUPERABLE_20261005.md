# Diario recuperable y devolución de fechas a Excel

Este avance de dev16 desarrolla C09–C10/B05. No incluye todavía el adaptador real de RUS: las pruebas de envío utilizan un adaptador ficticio, sin consultar ni modificar el sistema judicial.

## Operaciones

`nurus.personal.registration` conserva en SQLite la intención exacta, el ingreso real, usuario, tipo, estado, destino, etapa, modalidad y procedencia del Excel. El texto proviene de la columna efectiva de un Excel cargado como registro externo. Una propuesta del motor, un cambio local sin exportar o un archivo modificado después de leerlo no sustituyen ese texto.

Antes de guardar, el adaptador debe proporcionar una lectura completa y reciente de la bitácora y releer los campos efectivos del formulario. El diario persiste `ENVIANDO` antes del POST. Una respuesta HTTP o un mensaje de éxito no bastan: se necesita una entrada nueva, con ID, texto, autor, tipo, estado y destino coincidentes. La consulta tiene que pertenecer al ingreso y contexto preparados.

| Estado | Significado |
|---|---|
| PREPARADA | Intención conservada; no enviada. |
| ENVIANDO | El intento quedó persistido antes de guardar. Un corte en este punto exige consulta. |
| INCIERTA | Envío sin comprobación por lectura. |
| NO_HALLADA | La lectura no identifica una entrada nueva coincidente. No provoca reenvío automático. |
| AMBIGUA | Varias entradas coinciden; requiere revisión. |
| REVISAR_PREVIAS | Hay varias entradas idénticas del día antes de intentar guardar; no se envía. |
| PENDIENTE_EXCEL | Entrada nueva releída y comprobada; falta devolver su fecha. |
| COMPROBADA | Registro comprobado y copia de Excel releída. |

Reabrir una operación incierta solamente consulta: nunca repite el guardado. Entradas antiguas con el mismo texto no prueban un registro nuevo. La misma intención del día se reutiliza aunque cambie el orden del Excel; otro intento del mismo ingreso queda bloqueado mientras exista uno sin resolver.

Una entrada idéntica del día, del mismo autor y con tipo/estado/destino coincidentes, se reutiliza sin enviar. Su fecha puede devolverse al Excel, pero el informe de gestión la separa de las observaciones nuevas del asistente. Si varias entradas previas coinciden, se requiere revisión y no se registra otra. Una entrada de un día anterior no impide por sí sola una revisión nueva.

El contrato pendiente debe obtener IDs de entrada, identidad del usuario, campos efectivos, paginación/cobertura y fecha del servidor. Si faltan, la aplicación no puede afirmar que registró una observación. El criterio CC sigue siendo tipo «Al Tribunal» → 1 y «Administrativa» → 0; no se deduce del contenido del texto.

## Devolución a Excel

`nurus.personal.registration_excel.reconcile_excel` vuelve a leer el Excel actual y localiza la fila por causa, tribunal, ingreso, persona, centro y RIT. No utiliza la posición antigua. Escribe solamente FECHA_OBS y CC en una copia nueva y conserva el texto, TT, RES y demás datos. Fecha o CC manuales contradictorios, texto posterior distinto, ingresos duplicados, fórmulas y celdas combinadas de gestión exigen revisar el conflicto.

La fecha procede de la entrada guardada y releída; no de la fecha de importación ni de una aprobación local. Después de generar la copia, se releen identidad, texto, fecha y CC antes de confirmar conjuntamente las devoluciones en el diario. Un Excel bloqueado deja los recibos pendientes: volver a intentar esta devolución no llama al guardado de RUS.

La vía habitual requiere Excel de escritorio en Windows. La vía portable para XLSX/XLSM requiere aceptar fidelidad reducida explícitamente. El origen no se sobrescribe y un destino existente no se reemplaza.

## Acceso

Resultados incorpora «Informe de operaciones de registro» y «Recuperar devolución de fechas a Excel». Leen `registro_observaciones.sqlite` de la carpeta de configuración del prototipo y no envían observaciones. Los recibos se incorporan a los informes de gestión únicamente cuando coinciden con ingresos reales del trabajo.

También puede ejecutarse la conciliación por separado:

```text
python -m nurus.personal.registration_excel --diario registro_observaciones.sqlite --excel registro.xlsx --salida registro_con_fechas.xlsx --modo CUMPLIMIENTO --hoja Cumplimiento
```

## Evidencia y trabajo pendiente

El código dev16 aprobado contiene persistencia, recuperación de cortes, campos/contextos distintos, ambigüedad, reordenación de filas, conflictos, archivo bloqueado y devolución conjunta. Incluye el texto actual del archivo, protección de fórmulas y acceso a Resultados en 1024×650.

La devolución específica aprobó su prueba nativa en instancias aisladas de Excel para XLS, XLSX y XLSM: ingreso correcto tras reordenar filas, original intacto, fecha/CC efectivos y conservación de texto, TT, RES, fórmulas, estilos y otras hojas. El adaptador de registro fue ficticio: esto no acredita lectura ni escritura reales de RUS. El escritor conserva XLSM sin proyecto VBA y cierra los archivos sin advertencias.

El código del paquete dev16 aprobó 443 pruebas y una omisión por condición de plataforma en Windows con Python 3.12, 3.13 y 3.14: https://github.com/MatiGaete2023/NuRus/actions/runs/37348807233. El ejecutable dev16 se comprobó también después de extraer su ZIP. Los paquetes anteriores se conservan como checkpoints; estas pruebas no constituyen aceptación real de RUS.
