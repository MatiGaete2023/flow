# Correcciones de las pruebas del usuario — 9 de octubre de 2026

Entrega: Asistente CSMP 0.5.0.dev4, Descargador y extensión **2.7.4**.
Desarrollo en `feat/ejecucion-plan-20261008`, PR 1 en borrador. Estas correcciones
continúan H01–H09/M01–M03; no sustituyen el informe anterior ni la aceptación
institucional pendiente.

Se analizaron localmente los JSON, las capturas y el HAR entregados como
evidencia de fallos. El HAR se usó exclusivamente para verificar la estructura
de los vínculos y no se incorporó como fixture. El usuario confirmó que instaló el ZIP de la rama. Los JSON son datos
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

## 4. SIN_VINCULO en Espera e Informes

**Causa confirmada por el HAR.** La acción `ShowObservaciones` usa seis
argumentos numéricos y conserva el tipo de ventana de cada pantalla: 11 en
Espera, 12 en Cumplimiento y 13 en Informes. El extractor y la extensión
solo admitían 12. Los lotes recibidos de Espera e Informes enumeraban sus
registros, pero descartaban los vínculos y nunca abrían sus bitácoras.

**Corrección.** Se conserva el tipo literal del vínculo comprobado. Python,
GET de Chrome y parser comparten el mismo tipo; permanecen las comprobaciones
de tribunal/causa/ingreso/etapa/modalidad y unicidad de la acción. No se ejecuta
el JavaScript recibido ni se inventa un identificador. El HAR también contiene
la pantalla de órdenes con tipo 10; no se presenta como bitácora validada de
Seguimiento ni se amplía su lectura sin comprobar su contrato de respuesta.

**Prueba.** Extensión instalada y servidor ficticio, 80 lecturas: dos tribunales,
cuatro modalidades, dos páginas, Espera/Cumplimiento y las tres opciones de
Informes, con UTF-8/Windows-1252. Node rechaza tipos ajenos y valores que no
corresponden al vínculo. Las respuestas reales de ventanas 11/13 no están en
el HAR: su contraste final en SITFA queda pendiente, aunque sus enlaces sí
están confirmados. Archivos: `descargador/vinculos.py`, `lectura_bitacoras.py`,
`extension/pagina.js` y `verificar_bitacoras_extension.py`.

## 5. Recuperar, consolidar y reutilizar bitácoras anteriores

**Omisión.** Resultados dependía de la última carpeta de descarga y buscaba
únicamente `resumen.json`. Una carpeta de bitácoras contiene `bitacoras.json`;
una nueva descarga cambiaba la última carpeta y la interfaz no ofrecía importar
el lote anterior. El lector autónomo existía, pero el flujo no mostraba cómo
reunir hojas separadas, bitácoras y paquete del Asistente.

**Corrección.** El historial conserva referencias locales entre ejecuciones y
cambios de destino. Resultados abre descargas, conjuntos y bitácoras sin Chrome.
Permite añadir carpetas anteriores y controles con sus HTML, excluir lotes de
la selección y crear un ZIP con todas las descargas añadidas. Las bitácoras se
filtran por tribunal/causa/ingreso verificados en las hojas; nunca por nombre/RIT.
El resumen informa coincidencias y exclusiones, conserva la fecha de captura y
no atribuye actualización remota. Se reúnen copias iguales sin duplicar entradas;
entre contenidos distintos se elige la captura posterior con zona horaria y
fecha inequívoca. Un conflicto sin orden verificable pide seleccionar un lote.
Una copia fallida nunca sustituye una válida. Cada original conserva su hash.

Al terminar una lectura se genera `Bitacoras_RUS.xlsx` junto al control y HTML.
El botón **Excel bitácoras autónomo…** también genera el consolidado offline de
lotes anteriores; incluye textos íntegros, lecturas fallidas y alcance. No requiere
el Asistente. El generador evita recorrer repetidamente toda la hoja al escribir
cada fila, para que un lote grande no tenga un coste cuadrático.

**Prueba.** Lote ficticio de 800 bitácoras: Excel con 800 ingresos y textos,
originales intactos y ZIP de dos hojas con dos coincidencias y 798 exclusiones.
Casos sin identidad, versiones repetidas/posteriores/en conflicto, rutas,
hashes alterados y recuperación de fases. Ventana real de Resultados importa
hojas y bitácoras anteriores, produce Excel/ZIP y excluye un lote sin borrarlo,
con Chrome desconectado. La instalación desde ZIP usa su parser local antes
de otra versión instalada. Archivos: `descargador/test_reutilizacion_local.py`
y `descargador/test_experiencia.py`.

**Uso.** Resultados e historial → Añadir descarga o lote anterior (principal e
informes por separado) → Añadir bitácoras anteriores → Crear paquete para
Asistente (ZIP). Importar el ZIP desde Trabajo → Importar paquete. Conservar
siempre el JSON con sus HTML y las carpetas de hojas con su control original.
El [manual del Descargador](MANUAL_DESCARGADOR_MVP_20261008.md) detalla el circuito.

## 6. Edición de adjuntos opcional

**Cambio pedido por el usuario.** Preparar correos generaba nóminas diferidas y
exigía aceptar el editor de cada borrador. Se retira ese paso obligatorio:
Preparar genera los adjuntos y **Editar adjunto (opcional)** permite modificar
solo la nómina del correo seleccionado. Las sesiones anteriores pendientes
materializan sus adjuntos al guardar, tanto en lote como individualmente.
Cambios del trabajo siguen marcando productos obsoletos; se mantiene la
confirmación de destinatarios, texto y adjuntos antes de guardar en Outlook.

Además se corrigió una colisión de variables en la preservación de ediciones:
el índice de correos anteriores se sustituía por una fila de nómina, por lo
que las ediciones posteriores al primer correo podían perderse al preparar
nuevamente. Se mantienen los adjuntos manuales y retirados sin retener copias
automáticas obsoletas.

**Prueba.** Ventana real con 25 correos: 25 adjuntos automáticos, edición y
regeneración del décimo, otros 24 archivos/rutas/hashes intactos, confirmación
y guardado de todos con adaptador ficticio, repetición sin duplicados,
repreparación que conserva la celda/texto/adjunto manual y recuperación de
sesiones diferidas por lote y correo individual. Ningún `Send` ni Outlook real.
Archivo: `tests/test_integral_ui.py`.

## Verificación

Verificación local: **605 pruebas aprobadas y 11 omitidas** del Asistente;
**192 pruebas y 59 subpruebas aprobadas** del Descargador. Se conserva paridad
de **17 productos tradicionales**. Las pruebas de ventanas, Node y navegador
con extensión real usan fixtures; no se consultó SITFA ni Outlook real.
Los resultados finales, SHA y compilación Windows se registran en
`CONTINUAR.md` y `docs/evidencia_regresiones_usuario_20261009/`.
El contraste en Outlook/SITFA institucional sigue pendiente. Los datos reales
se conservaron fuera del repositorio; ningún HAR/JSON/HTML institucional forma
parte de la evidencia publicada.

## Actualizar la instalación desde ZIP

1. Cerrar Asistente y Descargador y conservar las sesiones, configuración y lotes
   anteriores. Descargar nuevamente el ZIP de **la rama**
   `feat/ejecucion-plan-20261008` y extraerlo en una carpeta nueva.
2. Ejecutar `Instalar_CSMP.bat` dentro de esa carpeta y después `Abrir_CSMP.bat`.
   Instalar el código vuelve a incorporar las correcciones al entorno Python;
   abrir el acceso directo de la carpeta antigua conserva el código antiguo.
3. Iniciar `descargador/Iniciar.cmd` de la carpeta nueva. En Chrome, desactivar
   copias anteriores de la extensión, cargar `descargador/extension` de esta
   misma entrega y pulsar recargar. Debe indicar **2.7.4**.
4. Cerrar la pestaña de conexión anterior y abrir una nueva desde SITFA; copiar
   el código actual del Descargador. Una extensión de otra versión se rechaza al comprobar
   capacidades, antes de leer bitácoras.
5. Preparar correos genera adjuntos; editar únicamente el que se quiera
   cambiar. Guardar borradores después de revisar el resumen y confirmar. Para el lote fallido,
   usar **Reintentar lecturas fallidas** con su carpeta original: se conservan
   las copias verificadas y se reintentan las aperturas fallidas. Comprobar el
   texto completo y el Excel de bitácoras generado.
6. Repetir una consulta conocida sin registros, con/sin fechas según corresponda,
   y abrir el PDF para comprobar tribunal, modalidad, pestaña y período.

Como alternativa, usar Asistente, Descargador y extensión del paquete conjunto
del mismo SHA indicado en `CONTINUAR.md`. No hace falta mezclar archivos de otras
entregas ni modificar los manifiestos de lotes para que pasen la verificación.
