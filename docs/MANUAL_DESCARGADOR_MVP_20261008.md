# Descargador — manual de la entrega MVP

Versión 2.7.3, con intercambio CSMP_CAPTURE 1.0.0. El SHA identifica cada
compilación. El Asistente no necesita estar abierto ni instalado.

La extensión 2.7.3 debe cargarse y recargarse junto con esta versión. Las
bitácoras admiten UTF-8 y Windows-1252/Latin-1 y conservan sus bytes originales.
Los PDF vacíos muestran la consulta solicitada, con tribunal, modalidad,
pestaña y fechas o «Sin filtro de fechas». Ese contexto se restituye localmente;
la tabla vacía se comprueba en la respuesta y no acredita un historial completo.

1. Iniciar sesión institucional manualmente y definir tribunales, modalidades,
   pestañas y período autorizado. Enlazar la extensión del mismo paquete.
2. Ejecutar consultas y revisar el control de lote: páginas esperadas/guardadas,
   vacíos comprobados, cancelaciones y fallos. Conservar los originales.
3. Para bitácoras, seleccionar ingresos con vínculos remotos comprobados y guardar
   el control de lecturas. SIN_VINCULO requiere diagnóstico; no significa vacío.
4. En Resultados elegir separadamente Consolidar XLSX, Excel bitácoras autónomo
   o Paquete CSMP v1. Reexportar un lote guardado no vuelve a consultar RUS.
5. Para incluir bitácoras en el paquete, elegir un control de lecturas ya guardado.
   Si se cancela esa selección, quedan NO_SOLICITADA. El ZIP de la interfaz no
   incluye HTML/XLS originales sensibles. Guardar en un destino nuevo.
6. Abrir la solicitud de antecedentes del Asistente para revisar ingresos y fuentes
   solicitadas. El archivo no ejecuta consultas; el operador decide el alcance.

El Excel de bitácoras contiene copia íntegra, fuentes, ingresos, lecturas,
consultas, lote y textos extensos partidos con hashes. Ausencias, errores y
cobertura parcial permanecen visibles. Abrir una copia antes de editar; los
originales y el control de captura constituyen la fuente conservada.

Calendarios no se asocian a personas por RIT/nombre. Historia/PDF por trámite
no están disponibles en este MVP. La verificación de lecturas reales requiere
el procedimiento de aceptación adjunto; las fixtures prueban la implementación.

Si un hash cambia, falta un original o la identidad no coincide, conservar
el lote y registrar el error. No corregir a mano los IDs para hacer pasar el ZIP.
Para soporte comunicar SHA, etapa, mensaje y estado de consulta; no adjuntar
NNA, cookies, tokens, HAR, HTML ni PDFs institucionales al repositorio.
