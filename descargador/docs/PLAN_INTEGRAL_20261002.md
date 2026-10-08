# Plan consolidado del descargador, CSMP y funciones independientes

Fecha: 2 de octubre de 2026. Uso previsto: una persona en Windows.

## 1. Decisión general

**CSMP será el punto principal de trabajo.** Desde allí se podrá iniciar la descarga, analizar los registros, revisar antecedentes, corregir observaciones y obtener el Excel final. El descargador seguirá disponible por separado y aportará las consultas y archivos originales. Las auditorías masivas y los informes de varios trabajos tendrán un módulo propio, accesible desde CSMP.

La primera ampliación debe resolver el recorrido solicitado: **Espera o Cumplimiento completo, informes por vencer del mes actual y siguiente, resoluciones firmadas de al menos 60 días, cruce de datos, reglas y Excel final con alertas**. Después se incorporarán lectura de bitácoras, análisis de historial y registro automático del texto del Excel.

Este plan consolida y sustituye, para ordenar el trabajo futuro, las propuestas anteriores. Conserva sus antecedentes y las correcciones ya documentadas. Las precisiones más recientes del usuario prevalecen sobre formulaciones anteriores.

## 2. Alcance que debe conservarse

| Función | Alcance acordado |
|---|---|
| Reglas, generación de propuestas, correos, proyectos de resolución y demás productos históricos | **Solo Laja, Mulchén y Tomé.** Seleccionar otro tribunal no habilita estos productos. |
| Descargas, cruce de resoluciones firmadas y alertas de actividad | Todos los tribunales disponibles y admitidos por el contrato de cada pantalla. |
| Consulta, exportación y análisis de bitácoras | Todos los tribunales disponibles. Observaciones de los últimos **cuatro meses calendario como máximo**, o un período menor. |
| Registro de observaciones del Excel | Todos los tribunales disponibles, con texto efectivo del Excel e ingreso correctamente identificado. No exige ejecutar las reglas históricas. |
| Informes por vencer del flujo conjunto | **Mes actual y siguiente**, calculados desde la fecha de ejecución. En diciembre, el segundo mes es enero del año siguiente. |
| Revisión de movimientos | Solo **resoluciones firmadas no invalidadas**, dentro de un período de **al menos 60 días**, ampliable. Pregrabados, despacho y devoluciones no generan tareas ni alertas. |
| Identificación del tribunal del reporte de carga | Código seleccionado y tribunal de cada fila. Se ignora el encabezado fijo de Concepción, reconocido por el usuario como error del sistema. |
| Carga de una observación | Tipo «Al Tribunal» → **CC=1**. Administrativa → CC=0 para esa categoría identificada. Tipo desconocido → pendiente de clasificación. |

Los tres períodos son distintos: informes por vencer, firmas y bitácoras. El filtro de 60 días no se aplica a la antigüedad de los ingresos de Espera o Cumplimiento. Una causa antigua puede tener firmas y observaciones recientes.

RIT y tribunal identifican una causa. Persona/RUT, centro/programa e ID real de ingreso permiten distinguir sus registros. El ID local de CSMP no sustituye al ID de RUS.

## 3. Estado de partida: conservar, integrar o desarrollar

| Grupo | Estado documentado | Tratamiento en este plan |
|---|---|---|
| Auditoría de programación y edición de septiembre | Correcciones documentadas en las entregas anteriores: sincronización por identidad, conflictos por campo, recuperación, decisiones estructuradas y productos coherentes. | Conservar y comprobar regresiones. No desarrollar de nuevo esas soluciones. |
| Experiencia de usuario y edición contextual | Entrega documentada de editores más amplios, paneles de texto/datos/cambios, edición individual y masiva, deshacer, filtros persistentes, contactos y plantillas. | Reutilizar. Completar únicamente las opciones avanzadas que falten tras comprobar la base. |
| Borradores de Mulchén | Corrección documentada de reconocimiento y modalidades residenciales RTA/RTT/RVA, preparación automática/manual, tarjetas y validación previa a Outlook. | Verificar que la instalación de uso la conserve, incluidas variantes de escritura del tribunal. |
| Fechas y bordes de adjuntos | Corrección documentada en **0.4.0.dev14**: alias de vencimiento, dato adecuado por categoría, respeto de ceros y correcciones, «Sin dato» y bordes negros. | Base obligatoria para integrar mejoras. Comprobar los archivos producidos por la instalación efectiva. |
| Descargador y funciones experimentales | Existe base de consultas de Seguimiento, litigantes y calendarios; la línea experimental documenta selección múltiple, consolidación, procedencia y traspaso a CSMP. | Integrar y verificar su funcionamiento junto a la base corregida. Distinguir disponibilidad experimental de funcionamiento en la instalación final. |
| Descarga conjunta, alerta de firmas, lectura completa y registro automático de bitácoras | Especificados en los informes del 02/10/2026. No acreditados como funciones ya instaladas. | Desarrollar por las entregas de este plan. |

La evidencia anterior de pruebas de código, interfaz y paquetes no acredita por sí sola una ejecución real nueva en Excel, Outlook o RUS. La fase inicial debe identificar qué versión y lanzador se usan y validar los puntos de integración necesarios.

En la comparación de ramas examinada anteriormente, la integración experimental de CSMP partía de dev11 y carecía de cambios de dev14. Al comenzar la implementación debe comprobarse nuevamente la relación de versiones e incorporar la integración sobre una base que preserve los arreglos. La fecha de una rama no basta para elegirla.

## 4. Distribución principal de responsabilidades

| Destino | Qué incorpora | Qué resultado entrega |
|---|---|---|
| **Descargador** | Consultas, selección de tribunales/pantallas/modalidades, descargas completas, calendario mensual, reporte de carga, verificación, procedencia y recuperación de consultas. | Archivos originales comprobados y un lote identificado, utilizable por CSMP. |
| **Asistente CSMP** | Coordinación del flujo conjunto, cruce, reglas de los tres tribunales, edición, alertas del Excel, revisión de causas/bitácoras, registro y conciliación. | Trabajo revisable, propuestas, productos históricos dentro de su alcance y resultados comprobados de registro. |
| **Funciones independientes** | Auditoría masiva de bitácoras, consolidación/comparación de archivos, informes de gestión y de resoluciones revisadas, exportaciones auxiliares. | Resultados reutilizables, incluso cuando no se esté preparando una observación. |
| **Componente compartido** | Conexión con Chrome/RUS, catálogo, identidad, lectores, recibos y cobertura. | Una misma base para las tres áreas, evitando consultas y criterios duplicados. |

Una función independiente no exige una nueva aplicación con otra forma de trabajar. Puede ser una opción accesible desde CSMP y también funcionar por separado cuando resulte útil.

## 5. Elementos del descargador

| ID | Elemento | Implementación concreta | Prioridad |
|---|---|---|---|
| D01 | Selección completa y favoritos | Obtener tribunales y opciones de la pantalla actual. Permitir uno, varios o todos, favoritos y filtros. Conservar selecciones que queden ocultas al buscar. | Alta |
| D02 | Espera/Cumplimiento completo | Descargar todos los registros del alcance elegido, con modalidades y páginas pertinentes. Comprobar cantidades y no limitar por antigüedad de la causa. | Alta |
| D03 | Informes por vencer automático | Reutilizar el calendario existente. Consultar mes actual y siguiente por los mismos tribunales, comprobar cada archivo y devolver ambas fuentes. | Alta |
| D04 | Reporte de carga para firmas | Añadir el contrato específico de `MaoDAction.do`, sus campos y la acción capturada. Respetar bloques de hasta 30 días y recuperar las firmas del período de al menos 60 días con cobertura comprobada. | Alta |
| D05 | Procedencia y cobertura | Conservar tribunal, pantalla, modalidad, fechas, momento de obtención, cantidades y archivo. Distinguir vacío comprobado, parcial, fallido y no consultado. | Alta |
| D06 | Consultas secuenciales y recuperación | Consultar, descargar, verificar y guardar antes de la siguiente consulta. Reintentar únicamente consultas pendientes y no reutilizar por error un reporte anterior. | Alta |
| D07 | Pantallas y pestañas adicionales | Incorporar Egresados y otras opciones conforme se compruebe su contrato. Mostrar si permiten consulta, apertura de bitácora o registro, sin usar por defecto el contrato de Cumplimiento. | Media |
| D08 | Archivos y vínculo con CSMP | Abrir el resultado desde CSMP y devolver un lote identificable. Mantener descarga de PDF cuando haya vínculo comprobado y conservar los archivos originales. | Alta para el traspaso; media para nuevos documentos |

La extensión y el puente existentes deben reutilizar la sesión del Chrome habitual. Las respuestas y ventanas se reconocen por sus formularios y contexto actual. El HAR es evidencia de estructura; no se ejecuta su código ni se reproducen sus credenciales.

Los calendarios y el reporte de carga se consultan una vez por tribunal y período compatible, no una vez por cada modalidad del mismo tribunal. Una ruta de exportación reutilizada exige descarga secuencial para no obtener el resultado de otra consulta.

## 6. Elementos integrados en CSMP

| ID | Elemento | Comportamiento concreto | Prioridad |
|---|---|---|---|
| C01 | Descarga y análisis en una operación | Elegir el alcance una vez, coordinar las tres fuentes y mostrar progreso por consulta. | Alta |
| C02 | Preparación de la hoja de informes | Reconocer una hoja auxiliar vigente por su esquema y cobertura o construirla desde las dos descargas mensuales. Mantener todas las fechas y la procedencia. | Alta |
| C03 | Cruce y reglas | Informes por identidad completa del ingreso; firmas por tribunal–RIT. Reutilizar C-10 y prioridades existentes. Activar productos históricos solo para los tres tribunales. | Alta |
| C04 | Alertas en el Excel final | Estado escrito, última firma, cantidad y detalle. Azul claro para firma por revisar y ámbar para revisión incompleta, sin borrar colores actuales de exclusión. | Alta |
| C05 | Ficha de revisión y accesos | Abrir causa, calendario y bitácora del ingreso verificado. Presentar identidad, fuentes, observación editable y hallazgos. | Alta |
| C06 | Calendario individual completo | Conservar todos los informes; distinguir pendiente pasado, próximo, futuro y enviado. Señalar fechas/estados contradictorios y discordancias de plazo. | Media |
| C07 | Última observación | Recuperar última entrada del centro dentro de hasta cuatro meses, texto completo, fecha, autor, tipo, etapa y respuesta. Última propia como opción. | Alta |
| C08 | Análisis de bitácora | Reiteraciones, respuestas, carga por tipo y deduplicación de extracción entre pestañas. Conservar entradas reales aunque tengan texto igual. | Alta |
| C09 | Registro del texto del Excel | Cargar directamente el Excel, localizar el ingreso, fijar texto/destino/estado por operación, guardar una vez y releer para comprobar. | Alta, después de lectura e identidad |
| C10 | Recibos y conciliación | Distinguir preparado, enviado, comprobado, incierto y pendiente de actualizar Excel. Recuperar sin duplicar y devolver la fecha efectiva a la fila correcta. | Alta |
| C11 | Bandeja de seguimiento | Reunir firmas por revisar, entradas al tribunal sin respuesta registrada, respuestas por contrastar y próximas comprobaciones. | Media |
| C12 | Edición y configuración | Conservar edición actual y completar controles de datos, selección, columnas, parámetros, contactos, plantillas, matrices y preferencias que falten. | Conservar en alta; ampliaciones en media |
| C13 | Historial y búsqueda | Buscar causa, persona, centro y gestión dentro del trabajo. Conservar propuesta, texto editado, texto consultado y texto guardado como datos distintos. | Media |
| C14 | Versiones, instalación y recuperación | Mostrar versión efectiva y estado de conexión. Conservar selecciones, filtros, cambios y operaciones pendientes al recuperar una sesión. | Alta |

### El Excel final

La hoja principal conserva las columnas habituales y las observaciones editadas. Añade un grupo corto: **ACTIVIDAD RECIENTE**, **ÚLTIMA FIRMA**, **RESOLUCIONES EN EL PERÍODO** y **DETALLE**. El período revisado debe quedar visible.

El libro incluye la hoja auxiliar de **Informes por vencer** y el detalle de **Resoluciones firmadas**. Las hojas de bitácora se añaden cuando se solicita esa consulta, evitando llenar todos los libros con resultados que no se usaron. Los bordes solicitados, formatos de fecha, ceros, fórmulas y demás hojas necesarias se conservan.

Una causa con varias firmas no multiplica las filas del trabajo principal. Una causa con varios ingresos recibe una alerta de causa, sin afirmar que todas las resoluciones afectan a todos sus programas. El número de filas de firma no se presenta como un número inequívoco de resoluciones distintas si falta identidad suficiente.

### Edición desde la interfaz

Se conservarán o completarán: observación, fecha, TT/CC/RES, inclusión en el lote, gestiones de correo, correcciones locales, variables de Word, registros de nóminas y adjuntos. Cada cambio puede deshacerse y obliga a actualizar los productos dependientes.

Las opciones avanzadas incluyen presets y orden de columnas, gestión de matrices Word con prueba y respaldo, parámetros con ejemplos y restauración individual, nombres/carpetas, tamaño de texto y geometría. La fase inicial comprobará cuáles ya están disponibles para no reconstruirlas.

Los campos de identidad remota son datos de origen. Corregir localmente el nombre o una fecha no puede cambiar silenciosamente el ingreso RUS al que se apunta. TT, CC y fechas vacías siguen siendo estados válidos; no se precargan como gestiones realizadas.

## 7. Funciones que conviene mantener independientes

| ID | Función | Motivo | Relación con CSMP |
|---|---|---|---|
| F01 | Auditoría y exportación masiva de bitácoras | Permite revisar un Excel o una selección RUS sin generar propuestas ni registrar observaciones. | Usa el mismo lector, período máximo y analizador de C07–C08. Se abre desde CSMP o por separado. |
| F02 | Consolidar, convertir y comparar archivos | Trabaja con lotes, esquemas y versiones de descarga; también es útil fuera de un trabajo del asistente. | Área Resultados del descargador, con traspaso directo a CSMP. Aprovechar la base experimental disponible. |
| F03 | Informes de gestión por período | Reúne varios trabajos y autores. Distingue observaciones históricas, gestiones nuevas comprobadas y propuestas sin registrar. | Módulo Informes accesible desde CSMP. No cambia TT/CC del trabajo actual. |
| F04 | Informe de resoluciones detectadas y revisadas | Muestra cobertura, causas, firmas y revisiones, sin saturar el editor de observaciones. | Reutiliza el servicio de firmas y el historial local de revisión. |
| F05 | Exportaciones auxiliares CSV, ZIP o calendario | Sirven para distribuir, archivar o consultar resultados fuera del editor. | Mantener como opciones de resultados. Reutilizar lo experimental y verificar el significado de cada fecha. |

F01 no requiere consultar siempre el calendario ni preparar una observación. Exporta el historial completo **dentro del período elegido**, con una fila por entrada y referencias al ingreso. El resumen distingue lectura completa, vacía comprobada, parcial y fallida.

F03 separa causas, ingresos y observaciones. «Al Tribunal» conserva CC=1 aunque se repita o tenga respuesta. Las estadísticas no cuentan una importación histórica como nuevas gestiones realizadas hoy.

Los informes pueden generarse cuando el usuario los solicite. El plan no crea recordatorios ni ejecuciones programadas por el mero hecho de incluir informes periódicos.

## 8. Base compartida que necesitan las tres áreas

| ID | Base | Responsabilidad |
|---|---|---|
| B01 | Catálogo y capacidades | Códigos actuales de tribunal, pantallas, pestañas, modalidades y acciones disponibles. Alcance separado por función. |
| B02 | Identidad y vínculos | Causa, persona, centro/programa, ingreso real, etapa y relación con la fila local, independiente de orden o posición. |
| B03 | Conexión y ventanas | Sesión del navegador, identificación de pestaña/ventana y comprobación de formulario e identidad al abrir cada detalle. |
| B04 | Fuentes y cobertura | Fecha de corte, origen, momento de consulta, páginas recuperadas, precisión temporal y estados de resultado. |
| B05 | Operaciones y recuperación | Intención, envío, comprobación, recibo y devolución a Excel. Consulta antes de repetir operaciones inciertas. |

Los lectores de calendario y bitácora no se duplican entre CSMP y la utilidad autónoma. La descarga y el registro tienen acciones diferentes, aunque compartan conexión e identidad.

## 9. Reglas de comparación y presentación

### Firmas y cobertura de 60 días

Una fila con firma válida y no invalidada, dentro del período, genera una coincidencia positiva. Se compara además con la última revisión del centro cuando esa fecha exista. Una firma ya revisada puede seguir siendo reciente, pero deja de estar pendiente.

Falta confirmar qué fecha filtra el reporte de carga. La muestra del 01/09/2026 incluye firmas hasta el 16/09/2026. Si la consulta selecciona por fecha de origen, bloques recientes pueden omitir firmas recientes de trámites antiguos. La cobertura requiere comprobar ese criterio y completar la búsqueda mediante historial o consultas de origen pertinentes.

Hasta entonces, ausencia de filas significa **«Sin coincidencias en los informes consultados»**, no «La causa no tuvo cambios». Una consulta fallida o incompleta nunca se convierte en cero firmas confirmado.

### Informes y reglas existentes

El calendario mensual contiene vencimientos programados. Su fecha por sí sola no acredita que el informe siga pendiente o que no se haya enviado. El calendario individual o la fuente de estado correspondiente permite comprobarlo.

El cruce de Cumplimiento conserva el criterio aprobado de C-10. La consolidación de dos meses debe mantener un orden de fuente definido y conservar las fechas. Mostrar el próximo pendiente y el pendiente pasado es útil, pero no autoriza sustituir la regla histórica por otra selección de vencimiento.

En Espera no se aplica C-10 automáticamente por la presencia de la hoja auxiliar. Una glosa «Prórroga», «Egreso» o «Archivar» no modifica por sí sola fechas, estados o propuestas: requiere comprobar la resolución y su ingreso pertinente.

### Bitácoras y registro

Se consulta la última observación del centro dentro de hasta cuatro meses, de cualquier autor salvo filtro elegido. No se rellena con una observación anterior al período cuando no hay entradas recientes. Fechas empatadas o ilegibles requieren conservar la ambigüedad.

Una entrada administrativa no se clasifica como pendiente de respuesta del tribunal por carecer de respuesta. Las entradas remitidas sin respuesta, respondidas y no comprobables son estados distintos. La similitud de textos señala posibles reiteraciones; no elimina entradas.

Antes de registrar el Excel se comprueba si aparecieron observaciones nuevas o resoluciones firmadas pertinentes. El texto definitivo permanece editable. Tras guardar se vuelve a leer la bitácora y solo entonces se devuelve la fecha efectiva a `FECHA_OBS`.

Si RUS guardó y Excel falló, queda «Registrada; pendiente de actualizar Excel». Se reintenta esa devolución, sin reenviar la observación. Si el guardado quedó incierto, se consulta antes de cualquier nuevo envío.

## 10. Plan de trabajo por entregas

Las entregas se ordenan por dependencias, sin inventar fechas de terminación antes de comprobar los contratos pendientes. Cada una debe incluir resultado utilizable, evidencia de pruebas y versión identificable.

### Entrega 0. Base corregida y alcance común

**Trabajo:** identificar la instalación efectiva, conservar dev14 y correcciones anteriores, integrar las funciones experimentales compatibles y definir B01–B04. Separar explícitamente el alcance histórico de tres tribunales del alcance universal de lectura y registro.

**Entregable:** base Windows única con catálogo, identidad, capacidades y versiones coherentes.

**Aceptación:** regresiones de Mulchén automático/manual, RTA/RTT/RVA, borradores visibles, destinatarios, adjuntos, fechas, ceros, bordes, edición y recuperación. Seleccionar un cuarto tribunal no activa productos históricos.

**Dependencias:** ninguna entrega nueva. La captura de contratos faltantes puede iniciarse durante esta fase sin impedir integrar lo que ya está comprobado.

### Entrega 1. Descarga conjunta comprobada

**Trabajo:** D01–D06 y D08 para el traspaso, C01–C02. Descargar principal completa y los dos calendarios mensuales. Añadir el reporte de carga y validar su criterio temporal, límites y cobertura de firmas antiguas.

**Entregable:** lote de fuentes identificado, listo para el análisis, sin unión manual de archivos.

**Aceptación:** todas las consultas del alcance tienen un resultado explícito; diciembre incluye enero del siguiente año; causas antiguas permanecen; una descarga anterior no sustituye a la actual; los fallos se reintentan por consulta.

**Dependencias:** entrega 0. Si la cobertura de firmas requiere investigación adicional, se puede entregar principal y calendarios, pero el lote identifica la actividad pendiente y no se presenta como revisión completa.

### Entrega 2. Cruce, reglas y Excel con alertas

**Trabajo:** C03–C04. Construir la hoja auxiliar, cruzar firmas, preservar las reglas y generar el producto final. Incorporar detalle de firmas y persistencia del estado de revisión.

**Entregable:** flujo «Descargar y analizar» que produce la planilla habitual con alertas escritas y color.

**Aceptación:** mismo RIT en tribunales distintos no se mezcla; informes respetan persona/centro; varias firmas no duplican registros principales; solo firmas pertinentes al período generan alertas; sin firma no hay alerta; colores no borran exclusiones ni bordes.

**Dependencias:** fuentes de entrega 1 y cobertura informada. El Excel indica cualquier cruce incompleto.

### Entrega 3. Revisión de causas, calendario y bitácoras

**Trabajo:** C05–C08, componentes de lectura y F01. Capturar y comprobar los contratos de las ventanas, textos completos, paginación, respuestas y variantes de etapa. Guardar revisión local por resolución e ingreso.

**Entregable:** ficha de revisión de CSMP y utilidad de consulta/exportación de bitácoras, ambas con los mismos resultados y criterios.

**Aceptación:** período máximo de cuatro meses, última entrada correcta, autor visible, tipo y CC coherentes, respuestas separadas, textos completos y misma entrada recuperada desde dos pestañas contada una vez. Una lectura parcial no afirma ausencia de reiteraciones o respuestas.

**Dependencias:** B01–B04 y contratos reales de lectura. El lector de bitácoras puede avanzar sobre esa base aunque una ampliación particular del reporte de carga siga pendiente.

### Entrega 4. Registro automático y devolución a Excel

**Trabajo:** C09–C10 y B05. Verificar el contrato de nueva observación, destino y estado por etapa. Ejecutar primero un caso elegido y después el lote seleccionado, con recibos y conciliación.

**Entregable:** registro secuencial del texto efectivo del Excel para cualquier tribunal admitido, con fecha real y resultado individual.

**Aceptación:** localizar el ingreso correcto, fijar destino en cada operación, respetar el límite real del formulario, guardar una vez y releer. Corte tras enviar, sesión vencida o Excel bloqueado no provocan duplicados. Cambiar el orden de la planilla no cambia la fila de devolución.

**Dependencias:** entrega 3 y contrato de escritura comprobado. Tener un borrador, una aprobación local o una fecha previa no acredita un registro en RUS.

### Entrega 5. Seguimiento, informes y ajustes avanzados

**Trabajo:** C11–C13, D07 y F02–F05. Completar la bandeja, comparación de lotes, reportes y opciones de configuración aún ausentes. Incorporar nuevas pantallas mediante contratos propios.

**Entregable:** seguimiento del trabajo y análisis de varios lotes sin ampliar innecesariamente el editor diario.

**Aceptación:** indicadores distinguen gestiones nuevas de historial consultado; comparación conserva multiplicidad; opciones avanzadas tienen prueba, restauración o deshacer; preferencias persisten; pantallas adicionales muestran su capacidad real.

**Dependencias:** servicios y datos comprobados de las entregas anteriores. Las mejoras independientes que ya tengan base experimental pueden integrarse antes si no alteran el flujo prioritario.

## 11. Pesquisas necesarias para implementar con precisión

| Pesquisa | Qué debe resolver | Entrega que depende de ella |
|---|---|---|
| Versiones y ramas de integración | Qué código debe conservarse y qué ejecutable usa el usuario. | 0 |
| Criterio temporal de carga | Recuperación de firmas recientes de trámites antiguos y forma de contar el máximo de 30 días. | 1–2 para cobertura completa |
| Contratos de bitácora y detalle | Identidad, textos completos, IDs, paginación, respuestas y apertura de documentos. | 3 |
| Contrato de nueva observación | Acción, campos, destino, estado, codificación, límite y comprobación del guardado. | 4 |
| Variantes por pestaña y modalidad | Qué cambia en Espera, Cumplimiento, Informes, Egresados y otras pantallas. | 3–5 según la capacidad |
| Calendarización y resolución vigente | Relación de informes, prórrogas, duración, persona e ingreso. | Revisión factual de 3 y enriquecimiento de propuestas de los tres tribunales |

El video demuestra la operación manual, pero los HAR anteriores no contienen todas las respuestas de las ventanas ni el guardado de la bitácora. La investigación debe incluir esas ventanas. No se debe confundir una prueba con datos ficticios con una comprobación real en RUS.

## 12. Interfaz prevista

Mantener el diseño visual de CSMP y aprovechar los editores actuales. La navegación principal puede ofrecer **Asistente**, **Bitácoras**, **Registro** y **Resultados**, conservando dentro del Asistente el acceso a Trabajo, Correos, Resoluciones y Configuración.

Arriba se muestra función activa, tribunales, pestañas/modalidades y período. Los filtros detallados quedan plegados y los favoritos reducen pasos. La fila seleccionada abre una ficha con editor amplio, fuentes y acciones necesarias. Calendario e historial completo no añaden columnas permanentes interminables.

Las acciones habituales serán «Descargar y analizar», «Consultar bitácoras», «Registrar observaciones del Excel» y «Exportar copia actual». Cada una presenta su alcance efectivo y resultado. Los casos válidos avanzan por lote; las incidencias se separan para revisión, evitando confirmaciones repetidas por cada fila.

Conservar prueba de interfaz Windows en ventanas de 1024×650 y 1180×820, acciones accesibles, filtros sin ocultar el editor y persistencia de separadores. La vista de borradores sigue diferenciando preparado, editado, necesita actualizar, guardado en Outlook e incierto. Guardar un borrador no se presenta como enviar un correo.

## 13. Criterio global de terminación

La tarea de implementación se considera completa cuando el usuario puede iniciar el flujo conjunto, recibir el Excel con reglas y alertas, consultar bitácoras y registrar las observaciones seleccionadas sin preparar manualmente los cruces, manteniendo el alcance de tribunales acordado.

Debe poder comprobarse por cada resultado qué fuentes se leyeron, qué quedó pendiente, qué texto se registró y cuándo. La recuperación conserva tanto ediciones personales como operaciones comprobadas. Las funciones autónomas utilizan los mismos datos y criterios.

Cada entrega necesita pruebas significativas de identidad, períodos, fuentes vacías/fallidas, multiplicidad, formato y recuperación, junto a las regresiones afectadas. La validación de lectura real precede al registro real. La entrega Windows final debe identificar su versión y conservar la configuración y sesiones compatibles.

## 14. Antecedentes y trazabilidad

| Documento | Aporte a este plan |
|---|---|
| [Auditoría inicial](C:/Users/cmgaete/Documents/Codex/2026-09-25/github-plugin-github-openai-curated-remote/outputs/Informe_NuRus_2026-09-25.md) | Errores de estado/sincronización, diseño y ampliación de lo editable. |
| [Implementación de la auditoría](C:/Users/cmgaete/Documents/Codex/2026-09-25/github-plugin-github-openai-curated-remote/outputs/Implementacion_NuRus_2026-09-28.md) | Base ya corregida y reorganización de interfaz. |
| [Revisión y cierre dev13](C:/Users/cmgaete/Documents/Codex/2026-09-25/github-plugin-github-openai-curated-remote/outputs/Revision_NuRus_2026-09-29.md) | Correcciones adicionales, Mulchén, productos, recuperación y pruebas documentadas. |
| [Corrección de adjuntos dev14](C:/Users/cmgaete/Documents/Codex/2026-09-25/github-plugin-github-openai-curated-remote/outputs/Correccion_adjuntos_NuRus_2026-09-30.md) | Fechas, ceros, bordes y dependencias de adjuntos. |
| [Análisis del recorrido SITFA](C:/Users/cmgaete/Documents/Codex/2026-09-25/github-plugin-github-openai-curated-remote/outputs/Analisis_flujo_SITFA_2026-10-02.md) | Video/HAR, aperturas, calendario, pesquisas y contratos pendientes. |
| [Exportación autónoma de bitácoras](C:/Users/cmgaete/Documents/Codex/2026-09-25/github-plugin-github-openai-curated-remote/outputs/Ampliacion_bitacoras_Excel_2026-10-02.md) | Libro de historial, resumen e incidencias, y comparación entre consultas. |
| [Bitácoras y registro desde Excel](C:/Users/cmgaete/Documents/Codex/2026-09-25/github-plugin-github-openai-curated-remote/outputs/Propuesta_CSMP_RUS_y_bitacoras_2026-10-02.md) | Última entrada, cuatro meses, carga por tipo, registro y recibos. |
| [Alcance por tribunal](C:/Users/cmgaete/Documents/Codex/2026-09-25/github-plugin-github-openai-curated-remote/outputs/Plan_CSMP_todos_los_tribunales_2026-10-02.md) | Universalidad de lectura/registro y conservación del asistente de tres tribunales. |
| [Resoluciones firmadas](C:/Users/cmgaete/Documents/Codex/2026-09-25/github-plugin-github-openai-curated-remote/outputs/Analisis_carga_tribunal_y_cruce_CSMP_2026-10-02.md) | Reporte de carga, error fijo del encabezado y límite de cobertura temporal. |
| [Flujo conjunto de 60 días](C:/Users/cmgaete/Documents/Codex/2026-09-25/github-plugin-github-openai-curated-remote/outputs/Especificacion_flujo_integrado_CSMP_60_dias_2026-10-02.md) | Tres fuentes, dos meses de informes, alertas y producto final. |

«NuRus» en los títulos de antecedentes identifica el repositorio y los documentos históricos. El producto final de este plan es CSMP.
