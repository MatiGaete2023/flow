# Ejecución del plan · CSMP y Descargador para Windows

Fecha: 6 de octubre de 2026. Versiones: **CSMP 0.5.0.dev1** y **Descargador 2.5.0**.

Se implementó el plan sobre las fuentes publicadas disponibles, en ramas separadas llamadas `feat/windows-trabajo-actual-20261006`. La entrega mantiene el trabajo actual y su recuperación, con cinco páginas en CSMP: Trabajo, Correos, Resoluciones, Resultados y Configuración.

## Cambios retirados

Se eliminaron la pestaña de historial, la consulta y exportación retrospectiva de Enviados de Outlook, los informes de varias sesiones, la revisión acumulada de firmas, los exportadores de auditoría/reiteraciones de bitácoras y la comparación entre lotes/planillas. El flujo conjunto del Descargador dejó de pedir reportes de firmas y ahora reúne únicamente la descarga principal y los informes del mes actual y siguiente.

Se retiraron también los paneles visuales de comparación y las referencias a otros sistemas en la documentación de los productos. El soporte y los workflows de construcción son para Windows.

Se conservaron la copia original, la sesión actual, las ediciones humanas, el cruce actual Cumplimiento/Informes y los recibos críticos que evitan duplicar efectos externos. Los contratos de lectura de archivos antiguos siguen siendo utilizables para recuperar un trabajo interrumpido; no existe una pantalla para acumular o comparar esos trabajos.

## Funciones implementadas

| Función | Resultado concreto |
|---|---|
| Nómina editable previa | Preparar correos en la interfaz ya no crea de inmediato los archivos de nómina. Revisar nómina permite editar celdas e incluir/excluir filas; aceptar genera los XLSX. Los cambios afectan al adjunto y no alteran el original. |
| Repreparación de nómina | Las modificaciones explícitas y exclusiones se conservan por ID del registro cuando se vuelve a preparar el mismo producto; se exige una nueva revisión. |
| Correo libre | Editor independiente de Excel, con destinatarios, cuerpo, adjuntos y guardado en Outlook como borrador. Conserva la edición actual y los recibos críticos de guardado. |
| Word manual | Genera texto libre o completa una matriz DOCX. Valida campos requeridos antes de escribir y conserva estilos de la matriz. |
| Bloques favoritos | Guarda bloques de texto sin adjuntos ni recibos. Las variables permitidas se completan antes de insertarlos. |
| Pendientes actuales | Avisos por fila, observaciones propuestas por revisar, contactos faltantes, nóminas pendientes, productos desactualizados y productos preparados que aún requieren guardado/generación. Un doble clic lleva al editor correspondiente. |
| Resumen previo a Outlook | Muestra destinatarios efectivos, CC, texto, filas aceptadas de nómina y archivos con tamaño antes de confirmar la creación de borradores. Se retiró la ruta que preparaba y guardaba todo sin revisión intermedia. |
| Contactos | Búsqueda por nombre o dirección. Las coincidencias se muestran para elección explícita; no se escoge automáticamente entre contactos ambiguos. |
| Calidad del archivo | Identifica campos obligatorios vacíos, números/fechas ilegibles e identidades repetidas. La revisión no elimina automáticamente filas repetidas. |
| Explicación de sugerencias | Presenta reglas activadas, acciones, datos utilizados, umbrales configurados y avisos del registro actual. |
| Preferencias de vista | Conserva columnas elegidas y selecciones lógicas cuando una búsqueda oculta filas o se actualiza la tabla. |
| Fechas actuales | Presenta por separado vencimiento de informe y egreso proyectado. El ICS del Descargador también distingue ambos significados. |
| Diagnóstico | Exporta versión, rutas, dependencias, matrices y registro de aplicaciones Office disponible en el equipo Windows. No incorpora contenido de planillas ni credenciales. |
| Entrega ZIP | Incluye únicamente los archivos seleccionados; admite nombres repetidos sin colisiones y conserva los originales. |
| Perfiles del Descargador | Recalcula mes actual/siguiente, incluido cambio de año, y últimos 30 días para consultas con intervalo. Rechaza períodos que no estén disponibles en el catálogo del portal. |
| Pausa y recuperación | La pausa espera en los límites de control del trabajador; cancelar libera una pausa y conserva el avance comprobado. La recuperación por fase pide las unidades pendientes. |

## Eficiencia y mantenimiento

El análisis de la interfaz reutiliza el libro ya leído para comprobar compatibilidad y evaluar reglas. La importación usa un conjunto de identificadores para detectar duplicados; la selección de resoluciones y sus orígenes utiliza índices por registro/causa para reducir recorridos repetidos.

La búsqueda de Trabajo espera 180 ms antes de recorrer la tabla. El guardado calcula la huella del estado completo y omite escrituras si no hubo cambios; los cambios de productos, preferencias, ediciones o recibos siguen provocando un guardado. Se mantiene la verificación de integridad de las copias archivadas.

Las funciones nuevas se separaron en módulos de nóminas, productos manuales, herramientas del trabajo actual y selección. Los workflows admiten las ramas `feat/**` y mantienen construcción y comprobación de paquetes en Windows.

No se afirma una reducción porcentual de tiempo: no se realizó una medición con una carga institucional representativa.

## Verificación local

- CSMP: **448 pruebas aprobadas, 6 omitidas** por condiciones externas específicas.
- Descargador: **111 pruebas aprobadas y 38 subpruebas aprobadas**.
- Comprobaciones adicionales de perfiles/pausa e interfaz del Descargador tras ajustar la fecha local del portal.
- Regresión de las cinco páginas, filtros, contexto, tarjetas, adjuntos y botones a 1024 × 650 con pantalla virtual. La captura nativa de Windows se verifica en el workflow.
- Pruebas específicas de recibo durable antes del guardado en Outlook, recuperación manual sin Excel, datos de nómina realmente exportados, texto de fórmula conservado como texto, DOCX con estilos, selección oculta, cambio de año y cancelación durante pausa.
- Compilación Python y comprobación del paquete fuente del Descargador satisfactorias.

Las llamadas de Office de las pruebas usan adaptadores controlados y datos ficticios. Estas pruebas no demuestran aún funcionamiento contra una instalación institucional real de Excel, Outlook o RUS.

## Estado de cada tarea del plan

| ID | Estado | Observación |
|---|---|---|
| A01 | Parcial por fuente externa | Consolidación de dev17/2.4.0 publicada; dev19 y 2.4.4 locales no están disponibles. |
| A02–A06 | Implementadas | Retiro de funciones y documentación Windows. |
| B01–B03 | Implementadas | Nóminas, productos manuales y preferencias/selecciones. |
| C01–C03 | Implementadas | Calidad, reutilización de lectura/índices y guardado sin escrituras innecesarias. |
| N01, N03, N04 | Implementadas | Perfiles, pendientes actuales y resumen previo. |
| N07–N10 | Implementadas | Bloques, contactos, pausa y explicación con datos de reglas. |
| N12–N14 | Implementadas | Diagnóstico, ZIP seleccionado y fechas con significado separado. |
| R01 | Pendiente externo opcional | Falta el formulario y una sesión RUS operables para implementar y demostrar el guardado real. |

## Límites y siguientes pasos

La fuente CSMP disponible parte de `b95f0db360059749727213b9ce2d24c690681acd` y la del Descargador de `226b9b1845073532cb2c7e1b9596ef11db3dc1e2`. Las versiones locales mencionadas en la memoria no se recuperaron exactamente; se reconstruyeron las funciones necesarias sobre esas bases.

Para R01, conviene una herramienta separada **Puente RUS** que consuma la copia actual revisada, valide identidad de ingreso, guarde una sola vez, compruebe la respuesta y devuelva fecha/CC a una copia. El núcleo de recibos y readback ya existente se conserva; no se habilitó un botón que simule un registro real. Falta acceso al formulario y una sesión válida para terminar esa integración.

La aceptación final debe cubrir en el equipo Windows de uso: apertura de XLS/XLSX/XLSM, exportación preservada con Excel, un borrador con Outlook clásico y firma, matrices reales de Word y descarga con sesión SITFA vigente. El build automatizado verifica distribución y arranque; no sustituye estas pruebas institucionales.

Otras mejoras recomendadas para una siguiente iteración: asistente de primer inicio que compruebe Office y rutas; editor masivo de nóminas con pegado desde Excel; estimación de consultas restantes durante la descarga; y medidas de rendimiento con libros ficticios de tamaño representativo. No se añadieron como funciones terminadas en esta entrega.
