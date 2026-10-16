# Revisión del Descargador SITFA

**Actualización:** las mejoras se implementaron en una rama experimental, con
estado por propuesta en [GUIA_EXPERIMENTAL.md](GUIA_EXPERIMENTAL.md). El cuerpo
siguiente documenta la revisión de la versión original y sus recomendaciones.

Fecha: 1 de octubre de 2026. Base revisada: `50345b7` (versión 2.2.0).

## Objetivo, funcionamiento y utilidad

Es una aplicación local para Windows con una interfaz Tkinter y una extensión de
Chrome. Automatiza consultas de lectura de SITFA usando la sesión que el usuario
ya abrió en su navegador. Permite seleccionar tribunales, modalidades, estados y
fechas; descarga secuencialmente las páginas de Seguimiento, Litigantes y los
calendarios mensuales de medidas e informes por vencer.

La extensión prepara el formulario y hace las solicitudes. `puente.py` comunica
Chrome con Python mediante un servidor HTTP limitado a `127.0.0.1`, un código
temporal y la identidad de la extensión. `perfiles.py` define y comprueba el
alcance de cada consulta; `motor.py` reconoce páginas y Excel, compara los
registros por RIT y nombre y verifica la escritura. `lotes.py` organiza consultas,
nombres y resúmenes. `evidencias.py` convierte una captura de Chrome en PDF cuando
la consulta no tiene registros. `descargador.py` reúne los controles de uso.

Su utilidad principal es ahorrar consultas y exportaciones repetitivas, conservar
archivos verificables por lote y detectar Excel antiguos, repetidos o ajenos a la
página solicitada. La captura de consultas vacías deja una evidencia local. No
modifica causas, no necesita copiar credenciales y no procesa los datos en un
servicio externo.

La separación entre interfaz, transporte, validación y planificación es adecuada
para este tamaño de proyecto. Conviene mantener las descargas secuenciales: SITFA
reutiliza el Excel temporal y el paralelismo podría mezclar resultados. También
conviene conservar la detención ante errores y la ausencia de reintentos
silenciosos.

## Hallazgos corregidos

| Hallazgo | Efecto anterior | Cambio aplicado |
|---|---|---|
| Primera página consultada dos veces | `BatchRunner` validaba el primer POST y `run_sequence` lo repetía antes del GET del Excel. Añadía carga y otra oportunidad de que cambiaran los resultados. | Se usa la primera página ya validada; se comprueba que sea la página 1 y que el enlace siga permitido. Las páginas siguientes conservan su POST y validación propios. |
| Respuestas mal formadas en `/complete` | Un identificador JSON de tipo lista o diccionario causaba una excepción por no ser una clave válida. `ok` aceptaba valores distintos de un booleano. | Se comprueban los tipos y se devuelve HTTP 400 con un mensaje controlado. |
| Finalizaciones repetidas | Una segunda respuesta podía reemplazar el resultado antes de que el hilo solicitante lo recogiera. | La búsqueda de la tarea, la escritura y su finalización se realizan bajo el mismo bloqueo; una respuesta repetida recibe HTTP 409. |
| Tareas vencidas en la cola | `/next` devolvía `idle` al encontrar una tarea vencida aunque hubiera otra válida esperando. | Se descartan las vencidas dentro de la misma petición, manteniendo la espera acotada de cuatro segundos. |
| Validación tardía del cuerpo codificado | Se decodificaba Base64 antes de comprobar el tamaño; ciertos tipos inválidos escapaban al mensaje previsto. | Se comprueban estructura, tipos y tamaño codificado antes de decodificar. Se mantiene el límite de 80 MB y la comprobación del contenido. |
| Regla del enlace Excel duplicada | Transporte y motor repetían la misma política, con riesgo de divergir en cambios futuros. | El transporte utiliza `motor.validate_export_url`, incluida la plantilla propia de cada consulta. |
| Copia intermedia de todas las celdas BIFF | El lector construía listas de objetos `Cell` y luego otra estructura con sus valores. | Lee directamente `sheet.row_values`; mantiene las celdas y registros resultantes y evita esa copia intermedia. |
| Prueba ficticia dependiente del Chrome del sistema | La integración antigua fallaba aun siguiendo la instalación de Chromium documentada. | Usa el ejecutable de pruebas instalado por Playwright; se añadió el paso de instalación que faltaba en sus instrucciones. |
| Restauración de filtros sin aviso | Al faltar un campo u opción, o fallar el enlace al terminar, el usuario podía seguir trabajando sin saber que sus filtros no se habían restaurado. | La extensión informa si la restauración fue completa y retira siempre el bloqueo. El programa muestra un aviso sin valores privados; conserva el estado de los archivos y el error original, si lo hubo. |

En una consulta no vacía de N páginas se pasa de N+1 POST a N POST; se mantienen
los N GET y todas las comparaciones de archivos. El ahorro corresponde a una
petición por consulta, no a una reducción medida del tiempo total. La segunda
consulta nativa para verificar y capturar un resultado vacío se conserva porque
cumple una función de comprobación diferente.

No se añaden dependencias de uso diario, controles nuevos, permisos de extensión,
consolidación ni cambios en el formato de los resultados.

## Validación realizada

- 77 pruebas unitarias y de interfaz aprobadas con Python 3.12 en Linux y una
  pantalla virtual para Tkinter. Incluyen los casos previos y regresiones de
  primera página, respuesta inválida, tarea vencida, finalización repetida y aviso
  de restauración sin pérdida de archivos ni del error de descarga original.
- Integración con la extensión realmente cargada en Chrome for Testing:
  conexión y errores específicos, catálogo, bloqueo y lote de tres páginas;
  filtro desaparecido con aviso de restauración parcial y retirada del bloqueo.
- Integración de cuatro pantallas: cinco lotes, nueve consultas y 19 páginas
  Excel ficticias; navegación, tribunales excluidos, fechas, filtros restaurados
  y orden POST/Excel comprobados.
- Integración de captura: cinco PDF ficticios, continuidad PDF/Excel en un lote
  y rechazo de una consulta que deja de estar vacía. Captura con las API reales
  y el permiso `activeTab` obtenido al ejecutar la acción de la extensión.
- Comparación del lector BIFF anterior y nuevo con un libro generado de datos
  ficticios: mismo formato reconocido, mismas celdas y mismos registros.
- Comprobación de sintaxis y revisión del diff.

Los informes de las integraciones se generan en `pruebas_locales/`, carpeta
excluida de Git. Se mantienen los informes históricos de la versión original.
Playwright y xlwt se utilizaron solo para pruebas; no se incorporan a
`requirements.txt`.

Todas las rutas SITFA usadas en las integraciones se interceptaron y respondieron
con datos ficticios. No se accedió al perfil ni a la sesión del usuario. Estas
pruebas verifican el funcionamiento local y los contratos simulados; no certifican
la estructura actual de cada pantalla del SITFA real. Tampoco se ejecutó
`Iniciar.cmd` en Windows durante esta revisión.

## Mejoras recomendadas, aún no implementadas

| Prioridad / esfuerzo | Mejora | Cómo aplicarla conservando la simplicidad | Problema que resolvería |
|---|---|---|---|
| Alta / bajo | Pruebas automáticas en GitHub | Añadir un workflow de GitHub Actions en Windows con Python 3.10 y una versión reciente; instalar `requirements.txt` y ejecutar `unittest discover`. Ejecutar integraciones ficticias de Chrome en un trabajo aparte cuando cambien extensión o transporte. Usar datos ficticios y no subir descargas como artefactos. | Detectar regresiones antes de entregar una actualización y comprobar la plataforma de uso y la versión mínima declarada. |
| Alta / bajo | Comprobación de requisitos al iniciar | En `Iniciar.cmd`, comprobar Python >= 3.10 y disponibilidad de Tkinter antes de abrir `pythonw.exe`; mostrar un mensaje concreto en la misma consola si falta algo. | Evitar que una instalación antigua abra la aplicación sin mostrar por qué falla. |
| Alta / bajo | Instalación ligada a las dependencias | Sustituir el marcador fijo `dependencias_2_2_0` por una huella de `requirements.txt`; actualizarla únicamente tras una instalación correcta. Mantener el entorno privado existente. | Evitar dependencias desactualizadas si cambia el archivo de requisitos sin cambiar el marcador del iniciador. |
| Media / medio | Límites durante la recepción y lectura | Leer la respuesta de Chrome por bloques y cortar al superar 80 MB; en Excel OOXML comprobar tamaños descomprimidos y establecer límites razonables de filas y celdas antes de construir todas las listas. Probar con ficheros ficticios grandes. | Los límites actuales se comprueban después de materializar la respuesta y no acotan por completo la expansión de un XLSX comprimido. |
| Media / bajo | Diagnóstico local mínimo | Mostrar un código de etapa ante fallos de carpeta, conexión, lectura o captura. Si se guarda un diagnóstico, registrar únicamente etapa, fecha y código controlado, sin URL remota, cuerpos, filtros privados ni excepciones crudas. | Facilitar soporte sin necesitar capturas privadas o repetir todo el lote para localizar el fallo. |
| Media / medio | Fixtures de cambios de SITFA | Añadir formularios ficticios con opciones ausentes, encabezados cambiados, fechas y filtros dependientes. Mantener el rechazo de estructuras desconocidas y revisar cada nuevo contrato antes de admitirlo. | Reducir roturas cuando SITFA modifica formularios y evitar interpretar silenciosamente una consulta diferente. |
| Baja / bajo | Selección de tribunales reutilizable | Guardar solo los identificadores elegidos en una configuración local excluida de Git; cruzarlos con el catálogo actual al abrir y descartar los que ya no existan. Mantener el selector por casillas. | Ahorrar una selección repetitiva entre sesiones sin guardar filtros de personas ni ampliar la interfaz. |

La siguiente entrega debería priorizar las tres primeras recomendaciones.
No se recomienda añadir descargas paralelas, un servidor
remoto, una base de datos o reanudación automática: aumentarían la complejidad y
las posibilidades de mezclar datos, con poco beneficio para el objetivo actual.
Una reanudación futura exigiría volver a validar todas las páginas y el alcance
de la consulta; no bastaría con continuar desde el último número guardado.

## Funcionalidades e integraciones para el usuario

La ampliación [MEJORAS_USUARIO.md](MEJORAS_USUARIO.md) evalúa consolidación XLSX
opcional, resumen legible, favoritos, fechas rápidas, búsqueda de tribunales,
historial, comparación de lotes, vencimientos y exportaciones para Excel,
Power Query, Power BI, calendarios y sistemas de gestión. Incluye prioridades,
implementación, límites y criterios de aceptación. Son propuestas pendientes;
la consolidación se plantea después de validar la descarga y conserva los
archivos originales y todas sus filas.

La comparación con el Asistente CSMP del repositorio NuRus y una propuesta de
integración comprobada con tablas ficticias están en
[COMPARACION_NURUS.md](COMPARACION_NURUS.md). El exportador productivo queda
pendiente; la sonda no cambia las funciones de uso diario de ninguna aplicación.
