# Asistente — manual de la entrega MVP

Versión 0.5.0.dev4, con intercambio CSMP_CAPTURE 1.0.0. No requiere Chrome,
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
4. Exportar copia actual a XLSX. Para paquetes nuevos funciona sin Excel/Office.
   Abrir Excel legado conserva el motor y la exportación anteriores, con alcance
   SOLO_PLANILLA. Guardar/recuperar sesión conserva edición y configuración.
5. Correos: revisar destinatarios, texto y nómina. Retirar adjuntos o cambiar
   contactos según el caso. Un contacto desconocido queda vacío con aviso.
   Outlook clásico puede guardar un borrador; el envío es decisión humana.
6. Resoluciones: revisar plantilla y variables, elegir proyectos y guardar Word
   editable. Correo libre, Word manual y edición libre siguen disponibles.
7. Resultados: consultar detalle de hechos/fuentes y exportar una solicitud de
   antecedentes. Abrirla manualmente en Descargador para una nueva captura.

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
