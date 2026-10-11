# Reconocimiento y exportación Excel: correcciones y aceptación

Fecha: 12 de septiembre de 2026. Versión: `0.3.0.dev3`.
Base remota revisada: `028b08efa002a0a040c5b6c99e7b6e2ecf118f6a`.
Rama: `implementacion-plan-2026-09-08`.

## Resultado y alcance

Este cambio aborda las hojas genéricas de los reportes RUS, el fallo de guardado nativo y la selección repetida del Excel revisado. No modifica las reglas jurídicas, las matrices, los umbrales ni el significado de TT/CC/RES. No envía correos ni escribe en RUS o SATURNO.

La propuesta inicial de permitir todos los productos antes de la constancia no se implementó: correos de cierre y estadísticas podrían afirmar gestiones no realizadas. Se reduce la fricción recordando la copia exportada, sin eliminar la revisión humana. El Excel de propuestas continúa disponible inmediatamente. Las comunicaciones particulares mantienen su flujo independiente.

## Hallazgos, correcciones y objetivos

### P01. Hojas genéricas no reconocidas

Hecho: el lector anterior rechazaba libros de varias hojas cuando ninguna tenía el nombre de la modalidad. El archivo adjunto contiene `Hoja1` con estructura de Cumplimiento y `Hoja2` compatible con cruce de informes. No contiene una tabla de Espera.

Cambio: inspeccionar hasta 30 filas iniciales por hoja y seleccionar una tabla única compatible con la modalidad elegida. La lectura de cabeceras se comparte para evaluar todas las modalidades. Se conserva el número físico de fila. OB y Medidas vencidas quedan fuera de la selección, sin eliminarlas del libro.

No se cambia silenciosamente la modalidad. Si se elige Espera en ese archivo, el mensaje identifica las estructuras disponibles e indica cambiar la modalidad sin volver a seleccionar el archivo. Si hay dos tablas compatibles, no se desempata por cantidad de columnas opcionales. Las selecciones expresas de hoja por API siguen disponibles; el selector avanzado de hojas ambiguas en la interfaz queda pendiente.

Comprobación sobre el adjunto: Cumplimiento reconoce 262 registros en Hoja1 y 145 registros de cruce en Hoja2. SHA-256 del original antes y después: `d9e2fd6bcd569322e069fc6c2dabc68151c97d36e86a8218e97e7c6b16a1f4a1`. Son cantidades de lectura, no revisión humana ni aceptación del contenido.

### P02. Error Workbook.Save y ruta temporal inexistente

Hecho: la captura registra fallo de `Workbook.Save`. El código abría y guardaba el mismo temporal y lo eliminaba al terminar. La captura posterior muestra una referencia a un temporal ausente. La causa específica del rechazo de Excel no está reproducida en este entorno; el código genérico COM no permite atribuirla exclusivamente a permisos, bloqueo, formato o ruta.

Cambio: abrir una copia de trabajo separada y ejecutar `SaveAs` hacia otra ruta temporal, con formato explícito: XLS=56, XLSX=51 y XLSM=52. Usar prefijos cortos y no añadir las copias al historial reciente de Excel. Comprobar que existe una salida no vacía antes de entregarla. Cerrar el libro y la instancia dedicada de Excel antes de limpiar temporales. No reemplazar el original ni un destino existente.

También se corrigió el orden RGB del resaltado nativo: ahora coincide con el amarillo FFF2CC del exportador portable. No se eliminan filas excluidas.

Objetivo: un guardado separado y verificable, con errores comprensibles. El éxito de los dobles COM no acredita que el XLS concreto de Espera ya se exporte en Excel 2010. Ese archivo no fue adjuntado en esta prueba.

### P03. Selección repetida de archivos

Cambio: recordar la ruta de la copia exportada por lote en SQLite, sin modificar el snapshot ni declarar revisión. La interfaz agrega **Usar revisión guardada** y conserva **Elegir otra copia…** para devoluciones diferentes o archivos movidos.

Paso a paso:

1. Analiza el archivo en la modalidad correspondiente.
2. Exporta las propuestas a una carpeta local autorizada.
3. Revisa cada registro en RUS y deja en ese Excel la observación, FECHA_OBS y datos administrativos correspondientes.
4. Guarda y cierra Excel. Pulsa **Usar revisión guardada**.
5. Confirma el responsable y que la revisión oficial fue registrada en RUS. NuRus lee la ruta recordada sin pedir seleccionar otro archivo.
6. Si todas las filas están incorporadas, intenta congelar la constancia en esa misma acción. Si falta la excepción de Cumplimiento, documéntala y pulsa **Congelar constancia**.
7. Genera correos, proyectos y estadísticas desde la constancia disponible. No requiere una carga por cada producto.

Si el archivo fue movido o renombrado, no se adivina otra ruta: usa **Elegir otra copia…**. Si solo devolviste parte del lote, las filas restantes siguen pendientes. Guardar en Excel no acredita por sí solo revisión en RUS.

La ruta persiste en la base; esto no agrega por sí solo un selector para reabrir lotes de sesiones anteriores. La recuperación de lotes desde la interfaz y la edición de constancia dentro de NuRus son mejoras pendientes, no funciones acreditadas por esta entrega.

### P04. Diferencia entre bytes leídos y hash confirmado

Hecho: la importación leía el archivo nuevamente después de validarlo. Si cambiaba entre lecturas, el hash registrado podía no describir los valores procesados.

Cambio: leer una copia en memoria, procesar esos mismos bytes y registrar su hash. La interfaz pasa el hash presentado durante la confirmación; si el archivo cambia durante ese intervalo, exige validar nuevamente. El lector XLS cierra su recurso y reutiliza una sola apertura para datos y trazabilidad.

### P05. Análisis anterior visible tras un fallo nuevo

Cambio: invalidar el lote activo al iniciar un nuevo análisis. Así, un fallo no deja habilitado un lote anterior como si correspondiera al archivo que se intentó procesar. El mensaje de una exportación asíncrona solo actualiza el estado si sigue seleccionado el lote correspondiente.

## Actualización en el PC institucional

No requiere permisos de administrador nuevos ni desactivar controles. Mantiene Python 3.12 y las dependencias existentes.

1. Cierra NuRus. Respalda la carpeta `%LOCALAPPDATA%\NuRus` por un medio institucional autorizado; contiene información de trabajo y no debe subirse a GitHub.
2. Descarga la rama indicada, no una copia antigua de `main`. Extrae en una carpeta nueva, estable y de ruta corta autorizada por la institución.
3. Ejecuta `Instalar_NuRus.bat` y luego `Abrir_NuRus.bat`.
4. Desde la carpeta instalada puedes comprobar la versión con `.venv\Scripts\python.exe -c "import nurus; print(nurus.__version__)"`. Debe mostrar `0.3.0.dev3`.
5. La base pasa a esquema 8. Antes de migrar una base anterior se crea un respaldo `.pre-v8-…bak`. No abras la base migrada con una versión anterior; para revertir, utiliza una copia del respaldo con el programa anterior, con NuRus cerrado.

## Verificación técnica realizada

- Suite local: 117 pruebas aprobadas, incluida regresión de selección ambigua, cabecera desplazada, conservación de origen y trazabilidad.
- Pruebas COM simuladas: SaveAs correcto para XLS/XLSX/XLSM, fallo de guardado, cierre y limpieza de temporales, color de excluidos. No son pruebas de Office real.
- Pruebas de interfaz sin pantalla: reutilización de ruta sin selector, cancelación y copia inexistente.
- Pruebas de importación: archivo modificado durante confirmación y registro de los mismos bytes procesados.
- Migración con respaldo, persistencia de ruta y mantenimiento de exigencia de constancia humana.
- Compilación de fuentes y pruebas sin errores. Búsqueda estática sin llamadas `.Send()` en `src`.

## Aceptación pendiente en Excel 2010 y Outlook clásico

| Caso | Pasos | Resultado exigido |
|---|---|---|
| I01 | Abrir el adjunto en Cumplimiento | Hoja1 principal y Hoja2 de cruce, sin error de selección |
| I02 | Elegir Espera en el adjunto de Cumplimiento | Mensaje explicativo; no cambiar reglas ni usar el lote anterior |
| I03 | Analizar y exportar el XLS real de Espera de las capturas | Salida existente, no vacía, abre sin reparación y sin mensaje del temporal |
| I04 | Comparar origen y salida XLS/XLSX/XLSM | Hojas, fórmulas, estilos, anchos y filtros conservados; excluidos amarillos y presentes; original intacto |
| I05 | Editar la copia exportada, guardar y usar revisión guardada | No abre selector; valida la constancia y permite productos tras confirmación |
| I06 | Modificar el Excel mientras está abierta la confirmación | Rechaza la versión cambiada y pide validar nuevamente |
| I07 | Mover la copia o impedir el guardado por un control institucional | Explica el problema; no declara éxito ni desactiva controles |
| I08 | Preparar y aprobar correo desde constancia; guardar en Outlook clásico | Solo un borrador, CC obligatoria, adjuntos y texto correctos; nunca envío |

Registrar versión, Windows/Office/Python, responsable, fecha, archivo/hash, resultado y evidencia. Si Excel pide reparar el libro o cambia su estructura, detener uso productivo y conservar origen/salida para diagnóstico. No convertir un bloqueo institucional en una excepción automática.

## Checkpoint

[G-ESTADO]
Objetivo: corregir reconocimiento/exportación y reducir la carga repetida conservando la revisión humana.
Confirmado: P01–P05 implementados y pruebas locales descritas arriba.
Limitaciones: no hay Excel 2010 ni Outlook institucional en este entorno; falta el XLS exacto de Espera de la captura.
Pendiente: I01–I08; selector avanzado de hojas ambiguas y recuperación de lotes desde interfaz fuera de este parche.

[L-SIGUIENTE]
Instalar `0.3.0.dev3` y ejecutar I03 con el XLS de Espera que produjo el error de Save.
