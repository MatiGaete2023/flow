# CSMP Assistant personal — Windows

Versión **0.4.0.dev12** en rama de evolución UX/reglas, actualizada el 22 de septiembre de 2026. Parte desde la candidata funcional congelada **0.4.0.dev11** y aplica únicamente cambios expresamente aprobados. La referencia de implementación y límites está en [`docs/EVOLUCION_UX_REGLAS_20260922.md`](docs/EVOLUCION_UX_REGLAS_20260922.md). Dev11 permanece como checkpoint funcional previo hasta que dev12 complete CI y prueba real.

## Estado de congelamiento

La prueba real del 22-09 confirmó procesamiento/modificación de Excel, RES categórico, generación de resoluciones, creación/edición de borradores y parámetros. Ver [`docs/CONGELAMIENTO_CANDIDATA_20260922.md`](docs/CONGELAMIENTO_CANDIDATA_20260922.md). La próxima fase es usar esta candidata durante uno o dos ciclos normales y registrar solo incidencias reales; aún no se genera `.exe` ni se promueve a 1.0.

## Cambios de esta actualización

El [pulido final dev11](docs/UX_FINAL_20260922.md) agrega búsqueda y filtros en Trabajo/Resoluciones, descripciones legibles de tipos de resolución, color por origen, barra de contexto, acceso a carpeta de salida y atajos mínimos. La visualización prioriza excepciones y mantiene los registros ocultos por filtro fuera de cualquier eliminación o cambio. No se agregan contadores laterales ni empaquetado `.exe`.

La [actualización dev10, vigente en dev11](docs/RES_CATEGORICO_20260922.md) convierte `RES` en una instrucción explícita: las copias nuevas ofrecen un desplegable `PC_IE / PC_INFO / NOMENCL`; vacío significa que no corresponde proyecto. Un valor desconocido se advierte y nunca se adivina. Las marcas antiguas (`1`, `X`, etc.) siguen siendo legibles para compatibilidad. Resoluciones muestra `Definido en RES` cuando el tipo proviene de esa columna y conserva el cambio manual como corrección final.

La [actualización dev9](docs/PANEL_CORREOS_20260921.md) incorpora CustomTkinter en modo oscuro, navegación lateral, tarjetas de programas y adjuntos individuales. **Solo programas / Solo tribunales / Ambos** define los destinatarios al preparar; el tribunal de las causas es un filtro opcional independiente. Se recupera el texto del manual de Informes por vencer mediante migración que conserva ediciones personales.

La [auditoría y limpieza dev8](docs/LIMPIEZA_ASISTENTE_20260921.md) retira la interfaz NuRus y las implementaciones duplicadas del Asistente. Corregimos pérdida de ajustes de resoluciones al actualizar/exportar, productos anteriores visibles después de cambiar de copia, selección de hoja arrastrada entre modos y desactivaciones de reglas perdidas al guardar umbrales. No se agregan aprobaciones ni recargas. La [revisión dev7](docs/REVISION_USABILIDAD_20260921.md) conserva la evidencia de las mejoras previas.

## Instalación y actualización

1. Descarga el ZIP de `main` y extrae su contenido en una carpeta nueva para no arrastrar archivos retirados. La rama `csmp-personal-2026-09-13` se conserva como historial de integración del producto personal.
2. Ejecuta `Instalar_CSMP.bat` y luego `Abrir_CSMP.bat`.
3. Se admite Python 3.12, 3.13 o 3.14. El entorno queda en `.venv-csmp` dentro de la carpeta de la aplicación. No requiere permisos de administrador ni Node. El instalador incorpora `customtkinter` 5.2.x dentro de ese entorno aislado.
4. Excel de escritorio y Outlook clásico son necesarios para la ruta completa de uso institucional. La instalación normal puede descargar dependencias Python; si existe `paquetes/`, el instalador usa ese repositorio local.

La configuración del usuario se guarda en `LOCALAPPDATA/CSMP_Personal`. La actualización conserva la configuración y las matrices personalizadas. Las migraciones de textos y matrices dejan respaldo cuando corresponde.

## Flujo de trabajo

En Trabajo selecciona Espera, Cumplimiento o Informes, carga el Excel y pulsa **PROCESAR**. La copia generada se comparte con Correos y Resoluciones. Las observaciones de gestiones ejecutadas usan redacción apta para registrar posteriormente en RUS; que el motor genere ese texto no acredita por sí solo que la gestión haya sido realizada.

Puedes editar la observación directamente en la ventana. Al exportar se captura automáticamente el contenido visible aunque no hayas pulsado antes **Aplicar edición**. Si existe revisión humana, las filas modificadas quedan `REVISADO`; las no modificadas quedan `PENDIENTE`. Se exportan `OBSERVACION`, `FECHA_OBS`, `TT`, `CC` y `RES` sin borrar valores existentes que no fueron editados. La neutralización de fórmulas de Excel se conserva.

También puedes cargar una planilla modificada/externa. Se buscan encabezados en todas las hojas hasta la fila 60. Si falta un cruce utilizable en Cumplimiento, C-10 no se evalúa, se deja advertencia y el proceso continúa sin pedir una excepción ni bloquear la exportación.

## Correos

Las modalidades se seleccionan con casillas: Residencial, Ambulatorio, Familia de acogida y DCE. FAS se clasifica como Familia de acogida. Los campos Para, CC, asunto, cuerpo y adjuntos son editables antes de guardar.

**Preparar todos** respeta el alcance elegido: **Solo programas** genera las gestiones a programas sin añadir el informativo al tribunal; **Solo tribunales** prepara sus comunicaciones; **Ambos** conserva el conjunto anterior. No seleccionar un tribunal en los filtros incluye todas las causas. Las tarjetas muestran programa, tribunal y vencimiento si están disponibles en los datos; seleccionar una tarjeta carga el editor. **Guardar todos** puede guardar un lote ya preparado o, si todavía no existe vista previa, preparar y guardar el conjunto necesario en una sola acción. No existe envío automático. Si no se conoce el destinatario, Para queda vacío. Siempre se incorpora la copia institucional configurada.

La identidad de un borrador depende de destinatarios efectivos, CC, asunto, cuerpo y contenido de los adjuntos. La ruta temporal de una nómina no forma parte de esa identidad: volver a preparar el mismo correo en otra carpeta UUID no habilita un duplicado, pero una edición real sí genera una identidad distinta.

Las copias nuevas incorporan una columna técnica oculta `NURUS_REGLAS`; al cargar una planilla modificada se recuperan esas reglas y, con ellas, los correos específicos que correspondan. Las copias anteriores intentan recuperar la misma información desde `NURUS_TRAZABILIDAD`.

## Resoluciones

Los proyectos se agrupan por **tribunal + RIT + tipo** y la lista muestra una sola fila por ese grupo. En las copias nuevas, `RES` tiene un desplegable con `PC_IE`, `PC_INFO` y `NOMENCL`; dejarlo vacío significa que no corresponde proyecto. El tipo explícito en `RES` prevalece. Las marcas antiguas `1/0/X` se siguen leyendo, pero su tipo debe inferirse como compatibilidad. Un texto RES desconocido genera una advertencia visible y no se corrige automáticamente.

Una mención a informe/diagnóstico discrimina `PC_INFO`; ingreso efectivo/fecha estimada de ingreso discrimina `PC_IE`; nomenclatura discrimina `NOMENCL`. Si `RES` no fue utilizado, una observación por sí sola no crea un proyecto: debe existir una acción de resolución del motor. Para cambiar el tipo manualmente debes seleccionar de forma explícita la fila o filas concretas; no seleccionar nada ya no equivale a modificar todos los proyectos.

Si hay varias personas en el mismo RIT y tipo, se genera un solo proyecto y cada NNA queda individualizado junto a su propia cédula. **Generar UN Word** produce un único archivo y cada proyecto comienza en página nueva. Los datos ausentes se marcan como `[COMPLETAR ...]`. Las fechas insertadas por el generador se escriben íntegramente en palabras.

### Matrices vigentes

Hay seis matrices base: `LAJA/NOMENCL`, `LAJA/PC_IE`, `LAJA/PC_INFO`, `MULCHEN/NOMENCL`, `MULCHEN/PC_IE` y `MULCHEN/PC_INFO`. Cinco cuerpos DOCX provienen de la revisión del paquete entregado el 14-09-2026 y se verifican por SHA-256; `LAJA/PC_INFO` fue convertida desde el Word antiguo entregado. No hay matrices de Tomé: la aplicación no inventa ni reutiliza una matriz de otro tribunal.

## Arquitectura y mantenimiento

Solo existe una interfaz operativa: `nurus.personal.app.App`. `app_base` contiene controles compartidos, sin versiones alternativas de los métodos del flujo. `nurus.app`, su acceso por consola y los BAT antiguos redirigen al Asistente; ya no contienen la interfaz NuRus ni los diálogos de aprobación/materialización. Se retira también el benchmark del flujo histórico. El historial Git conserva esas fuentes.

El paquete Python sigue llamándose `nurus` para mantener instalaciones y archivos compatibles. Los componentes compartidos de lectura, exportación, Office, modelos y persistencia permanecen, con sus pruebas; esta limpieza no elimina datos ni migra sesiones. Las columnas técnicas de las planillas conservan sus nombres para poder releer las copias existentes.

## Estado de verificación

La CI ejecuta instalación, wheel, recursos, dependencias, compilación y pruebas en Windows con Python 3.12, 3.13 y 3.14. Python 3.12 ejecuta además la ventana real y construye `CSMP-Windows-dev11`. La evidencia exacta del commit y de la ejecución está en [VERIFICACION_PERSONAL.md](docs/VERIFICACION_PERSONAL.md); no se atribuyen los resultados anteriores a cambios nuevos.

La validación automática no reemplaza la prueba final con Excel/Outlook institucionales. Consulta primero `docs/ESTADO_CONSOLIDADO_20260920.md` y `docs/INDICE_DOCUMENTACION.md`; la implementación, verificación, auditorías y documentos históricos quedan enlazados desde ese índice.
