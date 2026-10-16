# Catálogo contextual — activación pendiente

[Catálogo](catalog.json) conserva CT-01..CT-10 del documento 03, con fuentes,
condiciones propuestas y límites. Ninguna genera hallazgos (`enabled: false`); falta
aceptación funcional individual y evidencia G1/G9 donde corresponda.

CT-01 y CT-02 están en `ACTIVA_SOLO_MOSTRAR` (decisión D5, WP-50): el Asistente muestra
la última observación atribuible al centro y las entradas posteriores de los últimos
cuatro meses (columna «Última obs. centro» y panel «Novedades posteriores» en Detalle).
No cambian observaciones, reglas, umbrales, `TT`, `CC` ni `RES`. Sin origen acreditado
se muestra `ORIGEN_DESCONOCIDO`. `nurus.capture.temporal.DISPLAY_RULES` debe coincidir
con este catálogo; vaciarlo es el rollback.

`nurus.capture.temporal` implementa cronología de preview para CT-01/02/06,
exclusivamente con fixtures explícitas no sensibles. Producción informa contexto
NO_EVALUABLE y permite revisar hechos capturados íntegros. CT-03..10 se conservan
como diseño desactivado; no se declara su motor sustantivo implementado.

Ningún trámite titulado «informe» acredita recepción ni subsanación, ninguna glosa
acredita efecto jurídico, ninguna marca local acredita última observación remota.
Campos como origen/coverage deben ser acreditados en el alcance aceptado.

Para completar P10: dato requerido → fuente/alcance real → condición concreta →
fragmento aprobado → acciones candidatas → incertidumbre → fixture → diferencia
baseline → referencia de aprobación → prueba Windows/institucional aplicable.
Se activa cada regla por separado, con rollback, después de esa aceptación.
