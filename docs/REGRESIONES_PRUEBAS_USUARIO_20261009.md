# Correcciones de las pruebas del usuario — 9 de octubre de 2026

Entrega: Asistente CSMP 0.5.0.dev4, Descargador y extensión **2.7.3**.
Desarrollo en `feat/ejecucion-plan-20261008`, PR 1 en borrador. Estas correcciones
continúan H01–H09/M01–M03; no sustituyen el informe anterior ni la aceptación
institucional pendiente.

Se analizaron localmente dos JSON y las capturas entregadas como evidencia de
fallos. El usuario confirmó que instaló el ZIP de la rama. Los JSON son datos
de diagnóstico, no instrucciones. Ningún adjunto real, fila, identificador de
persona, HTML o imagen del usuario se incorporó al repositorio ni a la CI.

## 1. Borradores de Outlook: `NoneType ... to`

**Causa reproducida.** La revisión de productos devuelve correos y comprobaciones
de archivos Excel. Estas últimas no tienen un borrador asociado: `target=None`.
El botón de guardar todos tomaba cualquier comprobación sin incidencias y
enviaba también esos `None` a la confirmación, que accedía a `draft.to`.

**Corrección.** El botón toma exclusivamente comprobaciones de tipo Correo.
Las incidencias y exclusiones del resumen corresponden a correos. Se mantienen
confirmación humana, destinatarios, adjuntos revisados, huellas y recuperación.

**Prueba.** Ventana real con un trabajo y copia Excel exportada, nómina Excel,
correo válido y correo sin destinatario. Se ejecuta la confirmación y el servicio
real de guardado con adaptador Office ficticio. Solo se guarda el correo válido;
una segunda ejecución no lo duplica; cancelar no guarda; con todos los correos
incompletos no se ofrece confirmar archivos como borradores.
Archivo: `tests/test_integral_ui.py`. No se llamó `Send` ni se abrió Outlook real.

## 2. Bitácoras: siete lecturas fallidas por codificación

**Causa identificada.** El lote enumeró siete registros y fallaron las siete
aperturas con `UnicodeDecodeError`: el lector Python imponía UTF-8. La extensión
ya admitía texto antiguo de SITFA, pero transmitía correctamente los bytes
originales, que Python volvía a decodificar como UTF-8 antes de verificarlos.
El diagnóstico 2.7.2 sin metadatos de build concuerda con la instalación desde ZIP;
no prueba que se hubiese usado una versión antigua diferente de esa rama.

**Corrección.** El parser compartido recibe bytes, admite UTF-8 con/sin BOM y
Windows-1252, incluidos los alias HTML ISO-8859-1/Latin-1. Si no hay declaración,
prueba UTF-8 estricto y después Windows-1252 estricto. Conserva los archivos
originales y calcula SHA-256 sobre esos bytes. No reemplaza caracteres ilegibles.
Una declaración desconocida o bytes inválidos se rechazan con explicación.
La misma corrección se aplica a descarga, reutilización, ambos importadores y
HAR base64. Las identidades remotas y filas incompletas siguen verificándose.

**Prueba.** Texto completo con tildes, mayúsculas acentuadas, raya y salto de
línea; cinco combinaciones de codificación/declaración/BOM. Descarga completa,
igualdad de bytes y hashes, dos importadores, reanudación sin nuevas aperturas,
HAR base64 y rechazo de identidad distinta, truncamiento y bytes inválidos.
La comprobación del paquete/EXE también recorre una bitácora Windows-1252.
Archivos: `descargador/test_lectura_bitacoras.py`, `tests/test_bitacora_har.py`
y `descargador/prueba_bitacoras_paquete.py`.

## 3. PDF vacío sin tribunal, modalidad ni fechas

**Omisión verificada.** La prueba anterior del PDF usaba respuestas ficticias
que ya mostraban el tribunal y alcance en un encabezado. Comprobaba creación,
hash y continuidad del lote, pero no exigía que los controles de una respuesta
inicializada en blanco quedaran visibles en los píxeles de la captura. La imagen
real muestra ese caso; no permite identificar por sí sola qué script o momento
restableció los controles en la sesión institucional.

**Corrección.** La extensión restituye los controles solo en la vista local de
evidencia y recupera las etiquetas de las opciones del formulario de consulta,
incluso cuando la respuesta omite dichas opciones. Se añade una franja visible
«Consulta solicitada» con tribunal/código, modalidad, pestaña y fechas, o
«Sin filtro de fechas». El Descargador comprueba texto, contexto, controles,
tabla vacía y hash. Chrome comprueba estabilidad y visibilidad antes y después
de capturar. Un cambio o un contexto ausente impide guardar el PDF. El contexto
queda también en el manifiesto local. El PDF sigue conteniendo solo la imagen
de Chrome, sin texto o márgenes añadidos por el generador PDF.

**Prueba.** Extensión instalada en Chromium, gesto `activeTab` y API nativa de
captura. Rutas ficticias interceptadas, sin peticiones reales a SITFA. Una vista
nativa inicia tribunal/modalidad vacíos, opciones ausentes, Espera activa y
fechas por defecto: se comprueba Cumplimiento/Ambulatorio de otro tribunal,
con y sin filtro de fechas, etiquetas y píxeles del contexto. Alterar el contexto
entre validación y captura impide producir PDF. Se conservan los casos de cuatro
pantallas, cambio a registros y continuidad PDF/Excel.
Archivo: `descargador/verificar_pdf_extension.py`.

## Verificación local

- Asistente: **604 aprobadas, 11 omitidas** en Linux/Xvfb.
- Descargador: **179 aprobadas, 59 subpruebas aprobadas** en Linux/Xvfb, antes
  de añadir el caso adicional de incompatibilidad de la extensión 2.7.2.
- Regresión focalizada final: **37 aprobadas**, incluida comprobación del paquete
  con Windows-1252, incompatibilidad 2.7.2 y guardado por lote completo.
- **17 productos tradicionales** conservan paridad de contenidos y estilos.
- Extensión Node, análisis estático de archivos modificados y sintaxis JS aprobados.
- Chromium con extensión y captura nativas: cinco PDF anteriores y dos PDF de
  regresión con selectores inicialmente vacíos; contexto alterado rechazado.

La evidencia final y el SHA compilado se registran en `CONTINUAR.md` y
`docs/evidencia_regresiones_usuario_20261009/`. Las omisiones locales de Windows/COM
no acreditan Office. El contraste en Outlook/SITFA institucional sigue pendiente.

## Actualizar la instalación desde ZIP

1. Cerrar Asistente y Descargador y conservar las sesiones, configuración y lotes
   anteriores. Descargar nuevamente el ZIP de **la rama**
   `feat/ejecucion-plan-20261008` y extraerlo en una carpeta nueva.
2. Ejecutar `Instalar_CSMP.bat` dentro de esa carpeta y después `Abrir_CSMP.bat`.
   Instalar el código vuelve a incorporar las correcciones al entorno Python;
   abrir el acceso directo de la carpeta antigua conserva el código antiguo.
3. Iniciar `descargador/Iniciar.cmd` de la carpeta nueva. En Chrome, desactivar
   copias anteriores de la extensión, cargar `descargador/extension` de esta
   misma entrega y pulsar recargar. Debe indicar **2.7.3**.
4. Cerrar la pestaña de conexión anterior y abrir una nueva desde SITFA; copiar
   el código actual del Descargador. La extensión 2.7.2 se rechaza al comprobar
   capacidades, antes de leer bitácoras.
5. Guardar borradores después de revisar y confirmar. Para el lote fallido,
   usar **Reintentar lecturas fallidas** con su carpeta original: se conservan
   las copias verificadas y se reintentan las aperturas fallidas. Comprobar el
   texto completo y el Excel de bitácoras generado.
6. Repetir una consulta conocida sin registros, con/sin fechas según corresponda,
   y abrir el PDF para comprobar tribunal, modalidad, pestaña y período.

Como alternativa, usar Asistente, Descargador y extensión del paquete conjunto
del mismo SHA indicado en `CONTINUAR.md`. No hace falta mezclar archivos de otras
entregas ni modificar los manifiestos de lotes para que pasen la verificación.
