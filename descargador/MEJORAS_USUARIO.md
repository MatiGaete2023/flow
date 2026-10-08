# Funcionalidades e integraciones para el usuario

**Actualización:** el estado implementado de esta rama y sus límites concretos
están en [GUIA_EXPERIMENTAL.md](GUIA_EXPERIMENTAL.md). El cuerpo siguiente conserva
el análisis previo a la implementación.

Fecha: 1 de octubre de 2026. Propuestas para el Descargador SITFA 2.2.0,
complementarias a [INFORME_REVISION.md](INFORME_REVISION.md).

Estas funciones **se proponen; todavía no están implementadas**. La evaluación
se basa en el código y sus formatos de salida. La revisión posterior del
Asistente CSMP de NuRus está en [COMPARACION_NURUS.md](COMPARACION_NURUS.md):
se comprobó compatibilidad de tablas XLSX ficticias con su motor, sin APIs remotas
ni servicios externos. Los esfuerzos son relativos, no estimaciones
de días: bajo implica cambios locales acotados; medio requiere formatos y
pruebas nuevas; alto depende de contratos o sistemas externos.

## Criterio de diseño

La experiencia puede mejorar sin cambiar el objetivo: preparar consultas con
menos pasos, entender qué se descargó y trabajar con los resultados sin unir
archivos manualmente. Mantener una ventana principal y ofrecer las acciones
secundarias en «Resultados» o «Más opciones». La descarga sigue siendo secuencial
y validada; el procesamiento posterior funciona localmente, sin necesitar Chrome
ni una sesión SITFA abierta.

## Funciones priorizadas

| Prioridad / esfuerzo | Función y experiencia propuesta | Problema que resuelve | Cómo aplicarla |
|---|---|---|---|
| Alta / medio | **Consolidar lote:** botón al terminar, con un XLSX por lote y hojas separadas por tipo de listado compatible. | Abrir y unir numerosos Excel de páginas y tribunales para analizarlos. | Leer solo archivos incluidos en las verificaciones, comprobar sus hashes y generar un libro con `openpyxl`, ya instalado. Detalle abajo. |
| Alta / bajo | **Resumen legible:** tabla por consulta con tribunal, modalidad/estado, páginas, registros y resultado; botón «Ver resumen». | Los JSON actuales sirven para verificación, pero resultan incómodos para consultar lo completado, vacío o pendiente. | Construir una vista Tkinter a partir de `resumen.json` y sus verificaciones. Exportar opcionalmente `Resumen.xlsx`; no volver a consultar SITFA. |
| Alta / bajo | **Favoritos de consulta:** guardar «FAE de mis tribunales», elegirlo y revisar el alcance antes de descargar. | Repetir selecciones de tribunales y filtros en cada sesión. | Guardar IDs, pantalla, modalidades y estados en JSON local excluido de Git. Validar cada ID contra el catálogo actual; señalar opciones desaparecidas. No guardar códigos de conexión, RUT ni filtros privados del formulario. |
| Alta / bajo | **Fechas rápidas:** «Este mes», «Mes anterior» y «Últimos 30 días». | Escribir fechas repetitivas o confundirse entre rango y calendario mensual. | Calcular rangos con `datetime` y reutilizar `Dates`; en calendarios actualizar mes/año. Mostrar las fechas exactas y respetar su casilla de activación. Probar cambio de año y febrero bisiesto. |
| Alta / bajo | **Buscar tribunales:** texto de búsqueda dentro del selector, con número de seleccionados. | Encontrar un tribunal entre muchas casillas y perder de vista cuáles están marcadas. | Filtrar etiquetas ignorando tildes, sin modificar la selección oculta. Distinguir «Marcar visibles» de «Marcar todos»; Aplicar/Cancelar mantienen su comportamiento. |
| Media / bajo | **Progreso del lote:** consulta 3/12, página 2/5, registros y archivos guardados; etapa actual y aviso al finalizar. | La barra actual se reinicia por consulta y no muestra claramente el avance global. | Usar eventos existentes `query`, `total`, `progress` y `query_done`. El número de consultas es conocido; el total de páginas se descubre por consulta. Evitar un porcentaje global o tiempo restante ficticio. Aviso sonoro opcional al finalizar o detenerse. |
| Media / bajo | **Historial local y abrir archivo:** lista de lotes con fecha, alcance y estado; doble clic para abrir un resultado. | Buscar carpetas por fecha y no saber cuál lote quedó completo. | Leer resúmenes de la carpeta elegida bajo demanda, sin base de datos. Abrir solo archivos del manifiesto dentro de su carpeta; indicar cuando fueron movidos o faltan. |
| Media / medio | **Comparar dos lotes compatibles:** filas nuevas, ausentes y cambios comprobables. | Revisar manualmente qué cambió entre dos descargas. | Exigir mismo alcance estructurado y filtros comparables. Comparar multiconjuntos de filas o una clave de registro verificada por pantalla; mostrar incertidumbre cuando no existe una clave suficiente. Detalle abajo. |
| Media / medio | **Resumen de vencimientos:** agrupar por fecha y tribunal y ordenar lo más próximo. | Revisar varias planillas para localizar plazos cercanos. | Empezar con columnas de fecha identificadas y verificadas de los calendarios. Usar la fecha de descarga como referencia y mostrar fechas no interpretables. Crear hoja adicional en el consolidado, sin modificar medidas. |
| Media / medio | **Volver a ejecutar consultas pendientes:** selección preparada desde un lote incompleto. | Reconstruir manualmente el alcance tras cancelar o perder la sesión. | Añadir al resumen el plan estructurado y el resultado por consulta. Proponer las consultas no completadas en un lote nuevo, empezando cada una por su página 1 y volviendo a validarlas. No continuar sobre archivos anteriores. |
| Baja / medio | **Paquete para entrega:** ZIP local con archivos verificados y un índice. | Adjuntar muchos archivos y olvidar resultados o evidencias vacías. | Usar `zipfile`, una lista explícita del manifiesto y un nombre nuevo. Señalar si el lote es incompleto. No incluir perfiles, código de conexión ni carpetas ajenas al lote. No enviarlo automáticamente. |
| Media / medio | **Distribución más sencilla:** ejecutable Windows con extensión incluida y asistente inicial breve. | Instalar Python y dependencias resulta difícil para usuarios ocasionales. | Evaluar empaquetado con PyInstaller en Windows; resolver recursos desde la carpeta de distribución y mantener resultados/configuración fuera del ejecutable. Probar en Windows limpio sin Python y verificar actualización de la extensión. Mantener el iniciador actual como alternativa. |

## Consolidación: propuesta concreta

### Qué recibiría el usuario

Después de un lote válido, pulsa **Consolidar lote**. Por ejemplo, doce Excel
de Espera y Cumplimiento de varios tribunales pasan a un `Consolidado.xlsx` con:

- **Resumen:** consulta, tribunal, modalidad, estado, archivos, registros y
  consultas sin resultados, con referencia a sus PDF.
- **Hojas de datos:** una por familia de listado y estructura compatible. Espera,
  Cumplimiento, Informes, Litigantes y calendarios no se mezclan automáticamente.
  Si dos listados de una familia tienen columnas distintas, se separan y se
  explica el motivo en Resumen.
- **Filtros y cabecera fija:** permiten buscar tribunal, modalidad, estado y
  fechas directamente en Excel. Anchos acotados para facilitar lectura.
- **Origen por fila:** identificador de consulta, archivo, página, posición de
  fila y momento del lote. Tribunal, modalidad y estado se añaden cuando aplican.
  Prefijar esas columnas para evitar colisiones con encabezados de SITFA.

Los XLS y PDF originales se conservan. El consolidado es un producto derivado;
no se presenta como una exportación original de SITFA. Una consulta vacía se
incluye en el resumen, sin fabricar una fila de persona o causa.

### Cómo implementarlo en este proyecto

1. Crear `consolidacion.py`, independiente de Chrome. Recibe una carpeta de lote,
   carga `resumen.json` y las verificaciones referenciadas y exige un lote válido
   por defecto. Comprobar que cada ruta permanezca en esa carpeta y que tamaño y
   SHA-256 coincidan. No seleccionar archivos mediante un simple `glob('*.xls')`.
2. Reutilizar `motor.read_excel`, que ya reconoce BIFF, OOXML, XML Spreadsheet y
   HTML tabular y devuelve `ExcelInfo.grid`. Añadir una extracción explícita de
   encabezados y filas de datos por formato: `records_in_grid` sirve para cotejar
   RIT/nombre, pero su `Counter` no conserva todas las columnas ni su posición.
   Probar cabeceras repetidas, pies, celdas vacías y columnas inesperadas.
3. Agrupar por pantalla, tipo de listado y encabezados completos compatibles;
   conservar orden, columnas y multiplicidad. Cotejar el número de filas de
   salida con los registros de cada archivo verificado. Una estructura ambigua
   detiene la consolidación sin alterar los originales.
4. Ampliar las verificaciones con metadatos estructurados de la selección:
   pantalla, ID/etiqueta de tribunal, modalidad, pestaña, estado, mes/año y fechas.
   Hoy contienen `caso` y `consulta`, pero no todo ese alcance estructurado;
   no conviene reconstruirlo interpretando nombres de archivo o etiquetas.
   Los lotes antiguos mantienen su etiqueta y origen disponibles, indicando lo
   que no se puede reconstruir de manera fiable.
5. Escribir con `openpyxl` por filas, en un temporal, y publicar en una ruta
   nueva sin sobrescribir. Guardar un manifiesto del derivado con fuentes,
   hashes, filas y versión de transformación. El fallo del consolidado no cambia
   la validación de la descarga ni borra XLS/PDF.
6. Añadir un botón en Resultados y ejecutar el trabajo fuera del hilo Tkinter.
   Habilitarlo también al abrir un lote del historial. Confirmar la escritura y
   ofrecer «Abrir consolidado»; no es necesario consultar de nuevo SITFA.

### Reglas que evitan resultados engañosos

- **No deduplicar por RIT y nombre.** Una causa puede tener varias medidas o
  aparecer en distintas consultas. Preservar todas las filas y su origen;
  ofrecer una vista agrupada opcional con recuento, sin borrar registros.
- **Registros no equivale a personas únicas.** El resumen usa «filas/registros»;
  no anuncia un total de personas sin un identificador adecuado.
- **Conservar identificadores como texto.** RUT, RIT, roles y valores con ceros
  iniciales no deben convertirse automáticamente a números. Mantener fechas
  originales; convertir solo columnas y formatos comprobados.
- **Contenido literal.** Escribir texto recibido como texto de celda, evitando
  que un valor iniciado con `=` se convierta en una fórmula. No copiar macros,
  enlaces externos ejecutables ni fórmulas de archivos originales.
- **Límites de Excel y memoria.** Definir un máximo de filas/celdas por hoja,
  dividir cuando sea necesario y reflejarlo en Resumen; XLSX admite 1.048.576
  filas por hoja, incluida la cabecera. No materializar todo el lote a la vez.
- **Lotes incompletos:** una consolidación parcial sería una opción posterior,
  explícita, con portada «INCOMPLETO» y consultas pendientes identificadas. Nunca
  confundir un resultado parcial con todo el alcance solicitado.

## Integraciones útiles, de menor a mayor complejidad

| Integración | Valor para el usuario | Implementación propuesta | Condición o límite |
|---|---|---|---|
| **Excel / Power Query** | Actualizar una tabla de trabajo con nuevos lotes sin copiar y pegar. | Ofrecer consolidado y una exportación estable por familia; documentar cómo importar desde una carpeta. Incluir ID de lote y procedencia. | No mezclar todos los lotes históricos por defecto: repetir descargas acumularía las mismas filas. Elegir último lote válido por alcance o construir una vista histórica explícita. |
| **Power BI Desktop** | Ver recuentos por tribunal, modalidad y período en un tablero local. | Empezar con XLSX/CSV de esquema versionado y una guía de importación; una plantilla de tablero puede añadirse después. | No requiere API ni cuenta para preparar archivos. Publicar en Power BI Service es una acción distinta que transmite datos; no se incluye automáticamente. |
| **CSV compatible con herramientas de oficina** | Reutilizar resultados en aplicaciones que no reconocen los XLS de SITFA. | Exportar una tabla por esquema, UTF-8 con BOM y separador documentado; usar `csv`, conservar texto y escapar delimitadores/saltos. | Para abrir en Excel, tratar fórmulas peligrosas y advertir sobre conversión de identificadores. CSV pierde tipos y varias hojas; XLSX es la opción principal. |
| **Calendario mediante ICS** | Revisar recordatorios de vencimiento en un calendario elegido por el usuario. | Crear un archivo `.ics` local desde fechas verificadas, con eventos de día completo cuando no hay hora, UID estable y descripciones mínimas. Exportar previa selección. | La fecha exportada no se recalcula como plazo jurídico. Muchos calendarios importan duplicados; documentar actualizaciones y bajas. No incluir nombres ni RUT por defecto. |
| **Carpeta sincronizada, OneDrive o unidad compartida** | Consultar resultados desde otro equipo mediante una carpeta de trabajo ya configurada. | Aprovechar «Cambiar carpeta» y documentar destino; para consolidación/ZIP, generar primero localmente y copiar al completar, verificando la copia. | La sincronización depende del cliente externo y no se puede garantizar desde la aplicación. El usuario elige el destino y el acceso a los datos. |
| **CSMP Assistant / NuRus** | Continuar con observaciones, revisión, borradores y proyectos sin convertir/unir planillas a mano. | Exportar XLSX real por modo y procedencia; comprobar columnas y cruce de informes. La sonda de [COMPARACION_NURUS.md](COMPARACION_NURUS.md) demuestra compatibilidad con tablas ficticias. | Falta implementar el exportador y validar descargas representativas. No equivale a escribir en RUS/SATURNO ni a enviar correos. RIT/nombre no basta para identificar una medida. |
| **Otros sistemas de gestión** | Evitar cargas manuales adicionales. | Obtener un formato de importación documentado o API autorizada y definir correspondencias con ejemplos ficticios. | Sin evaluación concreta de otros sistemas; no asumir compatibilidad por la integración con NuRus. |

Google Sheets, correo y mensajería pueden recibir exportaciones manualmente,
pero no aportan suficiente ventaja inicial para añadir autenticación, permisos y
envío automático al descargador. Las primeras integraciones pueden resolverse
con archivos locales reutilizables.

## Comparación entre lotes: límites y diseño

Antes de comparar, comprobar pantalla, tribunales, modalidades, estados y rango
de fechas. Un lote de Espera no es directamente comparable a otro de
Cumplimiento. Si se conservaron filtros privados preparados en SITFA, el resumen
actual solo indica «formulario»: no permite asegurar que ambos lotes consultaron
la misma persona, causa o centro. En ese caso no declarar comparación automática
de alcance equivalente.

Para una primera versión, comparar filas completas como multiconjuntos dentro
de cada consulta compatible. Informar «fila presente solo en el lote nuevo» o
«fila ausente en el nuevo», conservando multiplicidad. Ausente no significa
egresada, resuelta o eliminada. Un cambio de columna puede aparecer como una
fila ausente y otra nueva; no llamarlo «modificación de la misma medida» hasta
disponer de una clave de registro comprobada en los formatos de origen.

Generar `Comparacion.xlsx` con Resumen, Nuevas y Ausentes y referencias a ambos
lotes. No modificar sus archivos y no publicar los resultados en servicios.
Probar filas repetidas, alcance diferente, lotes vacíos, cambios de encabezados
y filtros privados no comparables.

## Orden recomendado de entregas

1. **Menos pasos:** búsqueda de tribunales, favoritos, fechas rápidas y destino
   recordado. Criterio: preparar una consulta repetida sin volver a marcar cada
   tribunal, revisando siempre su alcance actual.
2. **Resultados aprovechables:** resumen legible, abrir archivos y consolidación
   XLSX opcional. Criterio: cada fila puede rastrearse hasta su archivo original
   y los recuentos coinciden con todas las verificaciones del lote.
3. **Trabajo recurrente:** historial, preparar pendientes y exportación estable
   para Power Query/Power BI. Criterio: no acumular descargas repetidas como si
   fueran registros distintos del último estado consultado.
4. **Seguimiento:** comparación entre lotes y vencimientos/ICS, una vez
   comprobados los identificadores y las columnas de fecha necesarias.
5. **Continuidad con CSMP:** el contrato de NuRus ya se evaluó en
   [COMPARACION_NURUS.md](COMPARACION_NURUS.md); su exportación puede incorporarse
   en la entrega 2, antes de conectores externos. Distribución Windows y otros
   sistemas quedan como etapas posteriores según su contrato específico.

La mejor primera ampliación funcional es **resumen legible + consolidación
opcional**: aprovecha datos que ya se validan y dependencias ya instaladas,
ahorra trabajo manual y mantiene la descarga original sencilla. Favoritos y
búsqueda de tribunales son los complementos de menor esfuerzo.
