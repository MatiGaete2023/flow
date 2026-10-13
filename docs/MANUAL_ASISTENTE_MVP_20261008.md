# Asistente — manual de la entrega MVP

Versión 0.5.0.dev5, con intercambio CSMP_CAPTURE 1.0.0 (1.1.0 con XLSX consolidado). No requiere Chrome,
extensión ni Descargador para importar y producir archivos de un paquete local.

1. Trabajo → Importar paquete: seleccionar ZIP y fecha de corte del análisis.
   Un paquete inválido se rechaza sin sustituir el trabajo abierto. Otra captura
   archiva la sesión anterior antes de cambiarla.
2. Revisar Cobertura y fuentes. Consultas ausentes o parciales, identidades
   inciertas y hallazgos tienen límites explícitos. El contexto temporal real
   permanece NO_EVALUABLE hasta aceptación; no acredita ausencia de actuaciones.
3. Filtrar y seleccionar registros, revisar observaciones y corregir TT/CC/RES.
   Las decisiones humanas se conservan al reimportar la misma captura. Una nueva
   fecha de corte conserva decisiones, pero puede dejar productos obsoletos.
4. La copia XLSX se genera automáticamente después de importar. **Exportar copia actual** sirve para actualizarla o reintentar una exportación fallida: la importación y sesión se conservan. Una reimportación idéntica reutiliza la copia existente y conserva ediciones. Para paquetes nuevos funciona sin Excel/Office.
   Si el ZIP incluye consolidado y Excel está disponible, usa la copia nativa;
   si no, informa la fidelidad reducida de la copia portable.
   **Elegir Excel · Ctrl+O** y luego **Procesar Excel** conservan el motor y la
   exportación anteriores, con alcance SOLO_PLANILLA. **Guardar trabajo · Ctrl+S**
   conserva edición y configuración; al volver a abrir, el aviso del trabajo
   anterior ofrece **Continuar**. En Resultados, **Recuperar resultado local legado**
   incorpora una descarga conservada por versiones anteriores.
5. Correos: Preparar genera los adjuntos automáticamente. **Editar adjunto
   (opcional)** abre solo la nómina del correo seleccionado; aceptar actualiza
   ese archivo, sin obligar a revisar los demás. Los cambios de celdas, filas,
   texto y adjuntos manuales se conservan al preparar nuevamente. Revisar
   destinatarios y texto y confirmar el lote antes de Guardar en Outlook.
   Un contacto desconocido queda vacío con aviso. Outlook clásico puede guardar
   borradores; el envío es decisión humana. Sesiones anteriores con nóminas
   pendientes generan sus adjuntos al guardar. Cambios en los datos del trabajo
   requieren preparar nuevamente los productos obsoletos. Si el archivo de
   recibos de la sesión (`recibos.diario.json`) existe pero no se puede leer, el
   Asistente no guarda borradores nuevos: revisa la carpeta Borradores de Outlook
   y luego repara el acceso al archivo o muévelo fuera de la carpeta de la sesión.
6. Resoluciones: revisar plantilla y variables, elegir proyectos y guardar Word
   editable. Correo libre, Word manual y edición libre siguen disponibles.
7. Resultados: consultar productos, incidencias, hechos y fuentes. Para una
   captura nueva, elegir el alcance manualmente en el Descargador y volver a importar.
   El Asistente no inicia el Descargador ni genera solicitudes de antecedentes (D9).

OBSERVACION, FECHA_OBS, TT, CC y RES reflejan trabajo local revisado. No son
constancia de registro RUS, recepción de informes, firma o envío de correo.
Una plantilla que afirme un registro no acreditado debe corregirse antes de
guardar un nuevo borrador. El Asistente no reescribe silenciosamente esa plantilla.

Los productos preparados se marcan obsoletos si cambian fuentes, corte, reglas
o decisiones. Revisarlos y regenerarlos antes de utilizarlos. Los documentos
exportados previamente permanecen como archivos independientes.

Ante error de importación conservar ZIP y sesión. Comunicar SHA, mensaje y etapa
sin datos institucionales. Excel 2010, Outlook 365 y políticas de instalación
requieren las pruebas reales descritas en aceptación pendiente.

## Cronología de bitácoras (solo mostrar, WP-50)

Con un paquete del Descargador, Trabajo muestra la columna **Última obs. centro**
y la pestaña Detalle el panel **Novedades posteriores**. Solo muestran lo capturado
en las bitácoras del ingreso, dentro de un horizonte de 4 meses hasta la fecha de
corte. No cambian observaciones, hallazgos, TT/CC/RES ni productos.

- La columna da la fecha de la última entrada atribuible al centro y cuántas hay
  después. Vacía si el trabajo no viene de un paquete.
- **ORIGEN_DESCONOCIDO**: hay entradas, pero ninguna con origen del centro acreditado.
- **Sin entradas capturadas**: se pidieron bitácoras y la del ingreso no trae entradas;
  no acredita ausencia de actuaciones.
- **Sin entradas en 4 meses · N anteriores**: la bitácora sí tiene entradas, pero todas
  son anteriores al horizonte; el panel las cuenta y no las muestra.
- **Bitácoras no solicitadas**: el paquete no pidió bitácoras.
- **Bitácora no leída**: el paquete pidió bitácoras, pero la de este ingreso no se leyó
  (límite de 300 lecturas, falla o lote incompleto). Usa «Retomar» en el Descargador y
  vuelve a crear el paquete.
- El panel da texto íntegro, fecha, origen y autor, y cuenta aparte las entradas
  sin fecha y las posteriores a la fecha de corte.

La aceptación espera el contraste de 10 ingresos reales; la vista no es una regla.

## Aviso de fidelidad reducida

Si el paquete trae XLSX consolidado pero no sirve como base (por ejemplo, una hoja
por modalidad o un registro repetido), la copia sale portable y el mensaje de
«Excel generado» lo avisa. El consolidado queda intacto dentro del paquete.

## Correcciones de revisión y capturas complementarias

La importación comprueba longitud UTF-16 antes de generar celdas: rechaza texto
que Excel no puede conservar íntegro. El paquete y el trabajo previo permanecen
intactos. El límite operativo es 20.000 registros por trabajo, incluidos calendarios
independientes; exportar por lotes si se supera.

Una captura complementaria conserva ediciones/exclusiones cuando hay una
correspondencia única de identidad remota y tipo/modalidad/persona/programa.
Sin identidad verificada solo se recupera la misma fila física íntegra; no se
asocia por nombre o RIT. Si cambian los datos o hay ambigüedad, la ventana de
conciliación muestra diferencias y pide conservar decisiones o usar la nueva
evaluación. En correspondencias ambiguas/no presentes se permite reset explícito,
no traslado automático. Cancelar conserva todo el trabajo anterior.

La sesión anterior se archiva y los productos preparados quedan obsoletos al
cambiar de captura. Las decisiones no acreditan registros RUS. Editar, deshacer,
refrescar Excel y recuperar sesión mantienen los hallazgos alineados con la regla
vigente. Fuentes/cobertura muestran las incidencias generales y del ingreso; el
Excel de evidencia incluye una hoja Incidencias.

Para un vacío por ingreso se requiere una consulta con alcance explícito; un
estado vacío de toda la tabla no se aplica automáticamente a cada persona.

## Siguiente acción y acceso común

La barra muestra Importar → Revisar → Correos → Resoluciones, con estado visible. **Siguiente acción** abre la pantalla o enfoca la preparación pendiente; no guarda Outlook ni genera Word sin tu acción. **Detalle** conserva los conteos de productos. También disponible en Ctrl+K. «Preparado» no acredita envío, firma ni registro. Los avisos finales permanecen visibles hasta ocultarlos o terminar otra acción.

En Windows, el acceso **Flow CSMP** o **Abrir_Flow.cmd** permite elegir Asistente o Descargador antes de abrir la app. El Asistente por sí solo sigue funcionando sin Descargador; los accesos directos anteriores conservan su función.

## Revisar solo lo que cambió (WP-95)

Al importar una captura teniendo una sesión anterior, Trabajo agrega la columna **Cambio** y el filtro **Solo cambios**.
«Nueva» es una fila que no existía; «Fuente» cambia un dato de la captura; «Reglas» cambia la evaluación por fecha de corte.
Las ediciones humanas se conservan. El resumen de importación indica también las filas egresadas; **Ver egresados** las muestra
en modo de solo lectura.

## Qué requiere mi atención (WP-96)

La bandeja ordena los casos que requieren una decisión: incierto, obsoleto, con aviso, por preparar o preparado. «Borrador guardado»
significa que el producto está en Outlook, pero el envío y la firma siguen siendo acciones humanas. Doble clic lleva al caso y a la
pestaña de Correo o Resoluciones correspondiente. La bandeja solo lee la sesión.

## Métricas locales opcionales (WP-97)

Configuración › Avanzado › Métricas locales permite guardar únicamente conteos y horas en `metricas.json`. Está apagado por defecto,
conserva como máximo 60 ciclos y no incluye nombres, RUT, RIT, textos ni rutas. **Copiar resumen** prepara el documento para entregarlo
al programador.
