# Meta 4 · Registros y bitácoras — 2.5.1

Genera la planilla Meta 4 y sus consolidados **a partir de lo que ya descargó el Descargador SITFA**. Esta herramienta no se conecta a SITFA ni a RUS, no usa extensión de Chrome ni abre puertos: solo lee carpetas locales.

## Instalación y uso en Windows

1. Descomprime la carpeta completa. Requiere Python 3.10 o superior con tkinter (incluido por defecto en Python para Windows).
2. Ejecuta **Instalar dependencias.bat** una vez.
3. Con el **Descargador SITFA** descarga:
   - Registros: Seguimiento → **Espera** y **Cumplimiento**, modalidades **FAE** y **Residencia**. Para un Meta 4 completo conviene descargar sin filtro de fechas, sin filtros del formulario y sin filtro de vencimiento; si el lote tiene filtros, igual se procesa y se avisa.
   - Bitácoras: lectura de bitácoras del mismo alcance.
4. Ejecuta **Iniciar Meta 4.bat**.
5. En **Lotes de registros** pulsa **Agregar…** una vez por cada lote (la carpeta que contiene `resumen.json`). Puedes agregar, por ejemplo, FAE Espera, FAE Cumplimiento, Residencia Espera y Residencia Cumplimiento por separado, o un lote por modalidad con Espera y Cumplimiento: todos se combinan en **un solo Meta 4**. También sirve agregar una carpeta que contenga varios lotes. **Quitar** saca los seleccionados; **Vaciar**, todos.
   Luego elige la carpeta de **bitácoras** (con `bitacoras.json` y sus HTML). Si las bitácoras están dentro de las carpetas de registros, deja ese campo vacío.
6. Opcional: en **Tribunales** selecciona uno o varios (Ctrl + clic). Sin selección se incluyen todos. La lista muestra los tribunales consultados en los lotes agregados.
7. Revisa la fecha de corte: al elegir las bitácoras se ajusta al día en que se leyeron si es anterior a hoy. Pulsa **Generar Meta 4**.
8. Pulsa **Abrir resultados**. Cada ejecución crea una carpeta nueva. El aviso final indica lectura incompleta, consultas sin bitácoras y lotes filtrados.
9. Si corriges **Registros.xlsx** (ver «Regenerar después de revisar»), guárdalo y pulsa **Regenerar desde Registros revisado…**.

## Qué lee y qué calcula

- **Registros**: uno o varios lotes validados del Descargador (`resumen.json`, verificaciones por consulta y todas sus páginas XLS con control SHA256). Solo usa Espera y Cumplimiento de FAE y Residencia. Rechaza un lote incompleto e indica cuál. Si la misma consulta (tribunal × lista × modalidad) aparece en dos lotes, se rechaza nombrando ambos: hay que quitar uno, para no duplicar registros. Un lote con filtro de fechas, filtros del formulario o filtro de vencimiento se procesa, pero cada filtro queda informado en el aviso final, en ERRORES_Y_ADVERTENCIAS de Registros.xlsx, en la hoja Control bitácoras y en el reporte (`filtros_registros`): META 4 contiene solo los registros que entregó ese filtro.
- **Bitácoras**: uno o varios lotes `bitacoras.json` del Descargador, con sus HTML verificados por SHA256. Sirven varios lotes (por ejemplo, divididos por el límite de lecturas) y las lecturas conservadas de un lote retomado. También acepta `bitacoras_meta4.json` de Meta 4 1.0/1.1.
- Usa solo las entradas del **mes calendario móvil** terminado en la fecha de corte, ambos extremos incluidos. Ejemplos: 16/09 a 16/10; 28/02 a 31/03. Los HTML originales no se modifican y no se copian a los resultados.
- **FECHA OBS. es la fecha de la observación del centro** más reciente cuya columna Etapa sea Espera o Cumplimiento. La fecha de respuesta del tribunal no interviene. Tampoco intervienen las entradas de etapa Informes, aunque sean más recientes.
- La selección compara fecha y hora, sin depender del orden visual de las filas. En Excel se muestra la fecha.
- Si el registro pertenece a Cumplimiento y la última etapa válida es Espera, muestra una **alerta roja**, conservando su ubicación en Cumplimiento.
- Sin observación válida en ese mes: deja FECHA OBS. vacía e informa el motivo. No recupera fechas del Excel original ni de meses anteriores.
- Si dos etapas tienen exactamente la misma última fecha y hora, conserva esa fecha y pide revisión; no supone cuál fue la última.

Controles de bitácora (columna CONTROL BITÁCORA, filas amarillas):

- `REVISAR: bitácora leída antes de la fecha de corte`: lo escrito entre la lectura y el corte no está en la bitácora guardada. Usa como corte el día de lectura o vuelve a leer.
- `LECTURA NO COMPLETADA`: el Descargador no logró leerla (fallida, pendiente o sin enlace). Si otro lote tiene una lectura verificada del mismo ingreso, se usa esa.
- `BITÁCORA NO VERIFICABLE`: el HTML cambió después de la descarga, falta o pertenece a otro ingreso. Bloquea la asociación aunque exista otra lectura.
- `VERIFICADA EN EL PERÍODO`: bitácora asociada y leída sin problemas, con al menos una observación del centro de etapa Espera o Cumplimiento en el período. FECHA OBS. es la más reciente.
- `SIN OBSERVACIÓN DE ESPERA/CUMPLIMIENTO EN EL MES`: bitácora asociada y leída, pero sin observación del centro de etapa Espera o Cumplimiento en el período (no hay, son anteriores o son de Informes). FECHA OBS. vacía.
- `REVISAR IDENTIDAD VISIBLE`: el **RIT** que muestra la bitácora es distinto al del listado. Bloquea: FECHA OBS. vacía.
- `REVISAR NOMBRE VISIBLE (fecha tomada de la bitácora)`: el nombre que muestra la bitácora es distinto al del listado, aunque coinciden tribunal, causa, ingreso, RIT, RUT y programa. **La fecha se usa** y la fila queda amarilla. No se marca si solo cambia el orden de las palabras, las tildes o falta un nombre o apellido.
- `IDENTIDAD AMBIGUA`, `SIN BITÁCORA ASOCIADA`: varios ingresos compatibles o ninguna bitácora para la fila.
- Si los lotes de bitácoras declaran sus consultas, el reporte lista las consultas con registros que **ningún lote de bitácoras enumeró**.

## Regenerar después de revisar

Registros.xlsx es la hoja de trabajo. En las hojas ESPERA y CUMPLIMIENTO puedes:

- Escribir o corregir **FECHA OBS.** (DD-MM-AAAA) y **ETAPA ÚLTIMA OBS.** (Espera o Cumplimiento).
- Escribir **REVISADO** (o un texto que empiece así) en **CONTROL BITÁCORA** para quitar el amarillo.
- Borrar filas que no corresponden.

Guarda el archivo y pulsa **Regenerar desde Registros revisado…** (o `--regenerar Registros.xlsx`). Se crea una carpeta nueva `… regenerado` con META 4, Registros.xlsx, Registros consolidados.xlsx, Reporte META4.json y una copia de Bitacoras_del_mes.xlsx. La ALERTA roja se recalcula con la etapa. Período, tribunales, lotes y avisos se toman del Reporte META4.json de la misma carpeta; si falta, hay que indicar la fecha de corte (`--cutoff`). La carpeta original no se modifica. Una FECHA OBS. ilegible detiene el proceso indicando hoja y fila.

## Resultados

- **META 4 - actualizado AAAA-MM.xlsx**: las cuatro hojas tradicionales (FAE y Residencia, Espera y Cumplimiento), fechas actualizadas, filas rojas para la alerta y comentarios que explican el motivo. Amarillo indica revisión de bitácora. Incluye la hoja Control bitácoras con período, estado de lectura y consultas sin bitácoras.
- **Registros.xlsx**: fuentes de Meta 4, FECHA OBS., etapa de última observación, control y alerta, inventario, matriz de cobertura y errores/advertencias.
- **Registros consolidados.xlsx**: **Resumen por tribunal**, Espera, Cumplimiento, Duplicados y Alertas y revisión. El resumen cuenta por tribunal los registros de cada lista (FAE/Residencia × Espera/Cumplimiento), alertas, filas por revisar, filas sin FECHA OBS. y consultas sin bitácoras, con una fila TOTAL. Las filas cuyo tribunal no se reconoce aparecen como SIN TRIBUNAL RECONOCIDO: quedan fuera de META 4. Duplicados conserva la comparación por RUT y nombre del consolidador suministrado.
- **Bitacoras_del_mes.xlsx**: observaciones del período con fecha y hora del centro, etapa, texto e identificadores del ingreso; incluye control de lecturas.
- **Reporte META4.json**: período, tribunales incluidos, resumen por tribunal, totales, alertas, registros que requieren revisión, conteo por control, estado de lectura, consultas sin bitácoras, avisos de filtros e incidencias de las fuentes.

**Filtro de tribunales.** Con un filtro, META 4 contiene solo los bloques de esos tribunales y su nombre termina en `- parcial`; la hoja Control bitácoras y el reporte indican los tribunales incluidos; la matriz de cobertura marca el resto como NO INCLUIDO y calcula la cobertura solo sobre los elegidos; Bitacoras_del_mes.xlsx se limita a ellos. Un nombre no reconocido o un tribunal sin registros en el lote es un error.

La carpeta de salida debe ser nueva; se rechaza antes de procesar si ya existe. Los archivos se escriben en una carpeta temporal hermana y solo se publican si la verificación final pasa: un error no deja una planilla parcial. No genera una planilla si un reporte de entrada no se puede leer.

Para asociar una fila exige tribunal, RIT, persona (RUT o nombre si falta RUT) y programa. Usa además los identificadores remotos de causa e ingreso cuando el control de descarga los aporta inequívocamente. Si hay varios ingresos compatibles o una discrepancia de identidad, deja la fecha vacía y pide revisión.

**El ZIP para el Asistente no sustituye estas carpetas:** su tabla de eventos no conserva la columna Etapa de la observación, necesaria para excluir Informes.

También admite una carpeta de reportes Excel independientes (sin `resumen.json`); en ese caso mantiene la selección de versiones por combinación del generador v2.4. Ese modo acepta una sola carpeta y no se mezcla con lotes del Descargador.

## Línea de comandos y pruebas

```text
python scripts/meta4_bitacoras.py --folder "C:\Descargas\FAE Espera" --folder "C:\Descargas\FAE Cumplimiento" --folder "C:\Descargas\RES Espera" --folder "C:\Descargas\RES Cumplimiento" --diaries "C:\Descargas\Bitacoras 2026-10-16" --output-dir "C:\Resultados\Meta4-nuevo" --cutoff 2026-10-16
```

Regenerar desde un Registros.xlsx revisado:

```text
python scripts/meta4_bitacoras.py --regenerar "C:\Resultados\Meta4-nuevo\Registros.xlsx" --output-dir "C:\Resultados\Meta4-revisado"
```

`--folder` es repetible: cada lote se agrega al mismo Meta 4 (también sirve una carpeta que contenga varios lotes). `--diaries` es opcional y repetible si las bitácoras están dentro de `--folder`. `--tribunal NOMBRE` (repetible) limita la generación a esos tribunales, por ejemplo `--tribunal Laja --tribunal Tomé`. `--month AAAA-MM` cambia el mes de emisión de la planilla; `--cutoff` controla por separado el período de bitácoras y no puede estar en el futuro. `--template archivo.xlsx` permite una plantilla propia; si no existe, es un error. La fecha de corte filtra las observaciones; no reconstruye el estado histórico de los registros de SITFA, que son los vigentes al descargar.

```text
python -m pip install -r requirements-dev.txt
python -m pytest tests -q
```

Las pruebas usan exclusivamente identidades y datos ficticios.

## Cambios 2.5.1

- **Rutas largas en Windows.** Una carpeta de destino profunda (por ejemplo, dentro de otra carpeta de resultados) superaba el límite de 260 caracteres de Windows y fallaba con `[WinError 3]`. Ahora la ruta se comprueba antes de procesar y el mensaje indica el largo máximo del destino (199 caracteres). Los nombres de carpeta son más cortos (sin microsegundos) y los temporales también.
- Si la ruta final de los archivos supera 218 caracteres, el aviso final advierte que Excel podría no abrirlos.
- Recomendación: deja la herramienta y los resultados en una carpeta corta, por ejemplo `D:\Meta4`.

## Cambios 2.5

- **Nombre visible distinto ya no borra la fecha.** Antes cualquier aviso de identidad del Descargador dejaba FECHA OBS. vacía, aunque la bitácora tuviera observaciones. Ahora solo un RIT visible distinto bloquea; un nombre distinto usa la fecha y deja la fila amarilla con `REVISAR NOMBRE VISIBLE`. Orden de palabras, tildes o un nombre de menos no generan aviso.
- **Regenerar desde Registros.xlsx revisado** (botón y `--regenerar`).
- `REVISADO` en CONTROL BITÁCORA quita el amarillo al regenerar.
- El reporte guarda `mes_emision`.

## Cambios 2.4

- **Varios lotes de registros en un solo Meta 4.** La ventana tiene una lista de lotes (Agregar…, Quitar, Vaciar) y la línea de comandos acepta `--folder` repetido. Una carpeta con varios lotes dentro también se combina; antes se rechazaba.
- Cada lote se valida por separado (estado, páginas, SHA256) y el error indica qué lote falla. Los avisos de filtros terminan con `[lote]`.
- La misma consulta en dos lotes, o la misma combinación tribunal × modalidad × lista en dos lotes, se rechaza antes de escribir, nombrando ambos lotes.
- Un lote elegido dos veces, o una carpeta que contiene a otro lote ya elegido, se lee una sola vez.
- La hoja Control bitácoras agrega la fila **Lotes de registros** (cantidad y nombres) y el reporte JSON el campo `lotes_registros`.
- Las bitácoras se buscan en todas las carpetas de registros si no se indica una carpeta propia.

## Cambios 2.3 (auditoría de código y pruebas)

Correcciones:

- **Tildes en XLS HTML sin charset:** un archivo UTF-8 se leía como ISO-8859-1 ("CaÃ±ete"). Los tribunales con tilde no se reconocían y sus filas quedaban fuera de META 4 (en un lote ficticio, 144 de 612). Ahora se lee como UTF-8 o, si no lo es, como Windows-1252.
- **Excel con dimensión desactualizada:** un .xlsx cuya dimensión declarada era menor que su contenido perdía filas sin aviso. Ahora se leen todas.
- **Filas de otro tribunal dentro de un archivo:** desplazaban al reporte propio de ese tribunal y se perdían sus filas. Ahora se suman al reporte propio sin repetir filas y se informa.
- **Fila con TRIBUNAL no reconocible:** quedaba en Registros pero fuera de META 4 sin aviso. Ahora se informa en ERRORES_Y_ADVERTENCIAS y en el aviso final.
- **Caracteres de control en una celda** abortaban la generación; ahora se reemplazan por un espacio.
- Una bitácora del formato anterior con parámetros nulos detenía todo; ahora queda como NO VERIFICABLE.

Código retirado, con prueba de que el producto no lo usaba:

- Línea de comandos propia de generate_meta4 (selector de carpeta, feriados, reporte TXT, verificación de duplicados del v2.4), con `assets/feriados_adicionales.txt` y la dependencia `holidays`.
- `descargador/motor.py` (transporte a SITFA): se reemplaza por `scripts/lector_excel.py`, que contiene solo la lectura de Excel. La equivalencia se comprobó con 244 entradas: 238 resultados idénticos y 6 diferencias que son exactamente las dos correcciones anteriores.
- Funciones sin uso del lector de bitácoras y ramas inalcanzables (formatos de fecha que nunca llegan, archivos de imagen).

Pruebas: las que pasaban por la línea de comandos retirada se reescribieron contra el camino del producto; se agregaron pruebas del lector de Excel (incluido .xls binario, antes siempre omitido), de la ventana y de cada corrección.

## Cambios 2.2

- Los lotes de registros con filtros (fechas, formulario, vencimiento) ya no se rechazan: se procesan y cada filtro se informa.

## Cambios 2.1

- Hoja **Resumen por tribunal** en Registros consolidados.xlsx y en el reporte JSON.
- **Filtro de tribunales** en la ventana (lista del lote) y en la línea de comandos (`--tribunal`); la salida filtrada se marca como parcial.

## Cambios 2.0

- Eliminada la descarga directa: puente local, extensión de Chrome, transporte, lectura de bitácoras en vivo y captura PDF. La herramienta solo procesa lotes del Descargador. Dependencias retiradas: reportlab, Pillow, tzdata.
- Registros y bitácoras pueden estar en carpetas separadas; se aceptan varios lotes de bitácoras y las lecturas conservadas de lotes retomados.
- Nuevos controles: lote de registros con filtros (rechazo hasta 2.1; aviso desde 2.2), lectura anterior a la fecha de corte, consultas sin bitácoras, lotes de bitácoras con filtros.
- Una lectura fallida en un lote ya no anula la lectura verificada del mismo ingreso en otro lote; una bitácora alterada sigue bloqueando.
- La fecha de corte propuesta es el día de lectura de las bitácoras.

## Cambios 1.1 (resumen)

Correcciones:

- Un texto de SITFA que empieza con `=` (nombre, tipo, curador, observación) se perdía: quedaba como fórmula sin valor en el XLSX intermedio y en Registros/META 4. Ahora se conserva como texto en todos los libros.
- Las páginas de una misma consulta aparecían como **DUPLICADO** en la matriz de cobertura. Ahora cuentan como un solo origen con el total de registros.
- La plantilla indicada con `--template` que no existía se reemplazaba en silencio por la interna.
- Una carpeta de salida existente se detectaba recién al final del proceso, y una verificación fallida dejaba archivos parciales.
- La marca roja/amarilla de META 4 depende de que las filas escritas y las filas en memoria estén alineadas; ahora se comprueba RIT por RIT y, si no coinciden, no se publica la planilla.
- La interfaz mostraba errores técnicos en inglés para fechas inválidas y no advertía una lectura de bitácoras incompleta.

Rendimiento (lote ficticio de 612 registros, 68 consultas, 1.811 entradas de bitácora, 1.379 dentro del período): **20,7 s → 1,7 s** en la generación local, con resultados idénticos celda a celda.

- Asociación fila–bitácora por índice (tribunal, RIT) en lugar de comparar cada fila con todas las bitácoras.
- Clasificación de tribunal y normalización de texto memoizadas; expresiones precompiladas.
- Las páginas validadas por el descargador se leen directo a memoria, sin escribir y releer un XLSX temporal.
- Registros.xlsx se escribe una sola vez con sus columnas de control; estilos de celda reutilizados.

