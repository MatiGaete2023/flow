# Descargador — manual de la entrega MVP

Versión 2.7.4, con intercambio CSMP_CAPTURE 1.0.0. El SHA identifica cada
compilación. El Asistente no necesita estar abierto ni instalado.

La extensión 2.7.4 debe cargarse y recargarse junto con esta versión. Las
bitácoras admiten UTF-8 y Windows-1252/Latin-1 y conservan sus bytes originales.
Los PDF vacíos muestran la consulta solicitada, con tribunal, modalidad,
pestaña y fechas o «Sin filtro de fechas». Ese contexto se restituye localmente;
la tabla vacía se comprueba en la respuesta y no acredita un historial completo.

1. Iniciar sesión institucional manualmente y definir tribunales, modalidades,
   pestañas y período autorizado. Enlazar la extensión del mismo paquete.
2. Ejecutar consultas y revisar el control de lote: páginas esperadas/guardadas,
   vacíos comprobados, cancelaciones y fallos. Conservar los originales.
3. **Leer bitácoras del lote** usa los vínculos comprobados de Espera,
   Cumplimiento e Informes (Por vencer, Vencidos y Recibidos). Guarda HTML,
   `bitacoras.json`, diagnóstico y **Bitacoras_RUS.xlsx** dentro de la carpeta
   del lote. SIN_VINCULO requiere diagnóstico; no significa vacío. Un error de
   consolidación no borra las lecturas y se muestra al terminar.
4. Abrir **Resultados e historial**, incluso sin Chrome. **Añadir descarga o
   lote anterior…** incorpora cada carpeta que conserve `resumen.json` y sus
   Excel originales, o `flujo.json` con sus fases. Añadir principal e informes
   descargados por separado: sus nombres aparecen en la selección del paquete.
5. **Añadir bitácoras anteriores…** permite abrir uno o varios `bitacoras.json`.
   Conservar los HTML al lado de su control: el JSON aislado no recupera textos.
   El historial recuerda las rutas entre ejecuciones y cambios de destino. Una
   descarga posterior no elimina ni reemplaza los lotes anteriores. **Excel
   bitácoras autónomo…** consolida el lote abierto o los lotes disponibles.
6. **Crear paquete para Asistente (ZIP)…** reúne las descargas añadidas y las
   bitácoras correspondientes por tribunal/causa/ingreso comprobados. Muestra
   coincidencias y exclusiones; nunca asocia solo por RIT o nombre. Guarda en un
   archivo nuevo e importa ese ZIP desde **Trabajo → Importar paquete** del
   Asistente. Las bitácoras conservan fechas y alcance originales: reutilizar
   no actualiza su contenido remoto. Sin bitácoras disponibles quedan
   NO_SOLICITADA; con lotes disponibles sin coincidencia, IDENTIDAD_INCIERTA.
7. **Quitar lote del paquete** excluye una descarga o lote de bitácoras de esta
   selección, conservando sus archivos e historial. Si dos versiones del mismo
   ingreso difieren, se elige la captura posterior con fecha inequívoca; en un
   conflicto sin orden verificable, quitar uno de los lotes. Añadirlo de nuevo
   lo reincorpora. Los hashes originales se comprueban antes de exportar. El ZIP
   contiene tablas y metadatos; no adjunta las copias HTML/XLS originales.
8. Abrir la solicitud de antecedentes del Asistente para revisar ingresos y fuentes
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
