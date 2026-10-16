# Meta 4 · Registros y bitácoras — 1.1

Herramienta independiente para descargar los registros necesarios de SITFA y generar la planilla Meta 4 y sus consolidados. Basada en el generador Meta 4 v2.4 suministrado y en los componentes de lectura del Descargador SITFA 2.7.6 del entorno.

## Instalación y uso en Windows

1. Descomprime la carpeta completa. Requiere Python 3.10 o superior con tkinter (incluido por defecto en Python para Windows).
2. Ejecuta **Instalar dependencias.bat** una vez.
3. En Chrome, abre `chrome://extensions`, activa Modo desarrollador y carga la carpeta **descargador/extension**. Si ya tienes esa misma extensión 2.7.6, puedes utilizarla; evita dejar dos copias activas.
4. Ejecuta **Iniciar Meta 4.bat**.
5. En tu Chrome, inicia sesión en SITFA y entra a **Seguimiento**. Pulsa el icono de la extensión y conecta usando el código de la ventana de Meta 4. Mantén abierta su pestaña de conexión.
6. Pulsa **Cargar tribunales de SITFA** y selecciona uno o varios con Ctrl + clic (o **Seleccionar todos**).
7. Revisa la fecha de corte y la carpeta de destino. Pulsa **Descargar y generar Meta 4**. Mantén la pestaña de SITFA sin navegar mientras trabaja.
8. Pulsa **Abrir resultados**. Cada ejecución crea una carpeta nueva. Si la lectura de bitácoras no quedó COMPLETA, el aviso final lo indica.

No necesitas iniciar simultáneamente la aplicación del Descargador anterior: esta herramienta reutiliza su transporte y su extensión.

## Qué descarga y qué calcula

- Consulta Espera y Cumplimiento para FAE y Residencia, sin limitar la antigüedad de los ingresos. Un ingreso antiguo también necesita aparecer en Meta 4.
- Incluye **todas las páginas verificadas** de cada consulta.
- No consulta Informes, Egresados ni la historia de la causa.
- Lee las bitácoras por el enlace del ingreso. Conserva en JSON solo las entradas del **mes calendario móvil** terminado en la fecha de corte, ambos extremos incluidos. Ejemplos: 16/09 a 16/10; 28/02 a 31/03. De cada entrada guarda solo identificador, fecha, etapa, tipo y texto del centro: no guarda la respuesta del tribunal ni el usuario autor.
- **FECHA OBS. es la fecha de la observación del centro** más reciente cuya columna Etapa sea Espera o Cumplimiento. La fecha de respuesta del tribunal no interviene. Tampoco intervienen las entradas de etapa Informes, aunque sean más recientes.
- La selección compara fecha y hora, sin depender del orden visual de las filas. En Excel se muestra la fecha.
- Si el registro pertenece a Cumplimiento y la última etapa válida es Espera, muestra una **alerta roja**, conservando su ubicación en Cumplimiento.
- Sin observación válida en ese mes: deja FECHA OBS. vacía e informa el motivo. No recupera fechas del Excel original ni de meses anteriores.
- Si dos etapas tienen exactamente la misma última fecha y hora, conserva esa fecha y pide revisión; no supone cuál fue la última.

**Límite de SITFA:** el endpoint disponible devuelve la ventana de bitácora sin parámetro de fechas. La respuesta se valida y filtra en memoria; no se guarda HTML ni se conservan entradas anteriores al mes. Por ello no se promete que el servidor transfiera únicamente ese mes.

## Resultados

- **META 4 - actualizado AAAA-MM.xlsx**: las cuatro hojas tradicionales (FAE y Residencia, Espera y Cumplimiento), fechas actualizadas, filas rojas para la alerta y comentarios que explican el motivo. Amarillo indica revisión de bitácora. Incluye una hoja de explicación del período y colores.
- **Registros.xlsx**: fuentes de Meta 4, FECHA OBS., etapa de última observación, control y alerta, inventario y matriz de cobertura.
- **Registros consolidados.xlsx**: Espera, Cumplimiento, Duplicados y Alertas y revisión. Incluye FECHA OBS. y la alerta escrita. Duplicados conserva la comparación por RUT y nombre del consolidador suministrado.
- **Bitacoras_del_mes.xlsx**: observaciones del período con fecha y hora del centro, etapa, texto e identificadores del ingreso; incluye control de lecturas.
- **Reporte META4.json**: período, totales, número de alertas y de registros que requieren revisión, conteo por control de bitácora, estado de la lectura de bitácoras (COMPLETA, INCOMPLETA, CANCELADA) e incidencias de las fuentes.
- En la carpeta de descarga se conservan los Excel originales, sus controles de integridad y **bitacoras_meta4.json**, con las entradas del mes y el estado de cada lectura.

Se mantiene el alcance de tribunales del generador v2.4 y las modalidades FAE/Residencia. La herramienta indica los archivos que no puede clasificar; no genera una planilla parcial si un reporte de entrada no se puede leer. Una lectura de bitácora fallida sí permite producir los registros, con fecha vacía y control visible para revisión. Revisar también la matriz para tribunales no consultados.

## Usar una descarga existente

Pulsa **Usar carpeta ya descargada…** y elige la carpeta de **un solo lote**, que contenga:

- Los Excel originales y `resumen.json` con sus archivos de verificación.
- La subcarpeta de bitácoras con `bitacoras.json` y sus HTML (descargador anterior), o `bitacoras_meta4.json` (esta herramienta).

Si los Excel y las bitácoras están en carpetas hermanas, selecciona su carpeta común, siempre que incluya un solo lote de registros. Los originales no se modifican. También admite una carpeta de reportes Excel independientes con sus bitácoras; en ese caso mantiene la selección de versiones por combinación del generador original. Para asegurar todas las páginas, utiliza los controles del descargador.

**El ZIP para el Asistente no sustituye esta carpeta:** su tabla de eventos no conserva la columna Etapa de la observación, necesaria para excluir Informes. Usa los originales y las bitácoras del lote o vuelve a descargar con esta herramienta.

Para asociar una fila exige tribunal, RIT, persona (RUT o nombre si falta RUT) y programa. Usa además los identificadores remotos de causa e ingreso cuando el control de descarga los aporta inequívocamente. Si hay varios ingresos compatibles o una discrepancia de identidad, deja la fecha vacía y pide revisión.

Las bitácoras guardadas por esta herramienta solo sirven para un período contenido en el período conservado. Para otra fecha de corte, vuelve a descargarlas. El máximo del transporte es 600 lecturas por ejecución; para una carga mayor, procesa menos tribunales por lote. No hay recuperación automática de una descarga incompleta desde esta ventana.

## Línea de comandos y pruebas

Desde la carpeta de la herramienta, con las dependencias instaladas:

```text
python scripts/meta4_bitacoras.py --folder "C:\Lote" --output-dir "C:\Resultados\Meta4-nuevo" --cutoff 2026-10-16
```

La carpeta de salida debe ser nueva; se rechaza antes de procesar si ya existe. Los archivos se escriben en una carpeta temporal hermana y solo se publican si la verificación final pasa: un error no deja una planilla parcial. `--month AAAA-MM` cambia el mes de emisión de la planilla; `--cutoff` controla por separado el período de bitácoras. `--template archivo.xlsx` permite una plantilla propia; si no existe, es un error (antes se ignoraba en silencio). La fecha de corte no puede estar en el futuro. La interfaz usa la fecha de corte como fecha de emisión. La fecha de corte filtra las observaciones; no reconstruye el estado histórico de los registros de SITFA, que son los vigentes al descargar.

```text
python -m pip install pytest
python -m pytest tests -q
```

Las pruebas usan exclusivamente identidades y datos ficticios. Se comprobó el procesamiento local y el transporte con respuestas simuladas. No se realizó una descarga con una sesión real de SITFA ni se ejecutaron los BAT en Windows en este entorno Linux.

## Cambios 1.1

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

Sin cambios: transporte, extensión y validaciones del descargador 2.7.6 (SHA256 en ORIGEN_COMPONENTES.txt), pausas de 1 s entre consultas y lecturas en SITFA, límite de 600 lecturas, criterios de FECHA OBS. y alerta.
