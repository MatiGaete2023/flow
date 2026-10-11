# WP-84 · G-5 y limpieza restante del historial

11 de octubre de 2026. Base feat `286c28f`. Paquete en preparación; E3 pendiente.

## Hallazgos y corrección

1. Consulta con 100 borradores y un adjunto compartido: 100 lecturas antes. Compartir
   únicamente hashes de adjuntos durante la consulta síncrona; descartar al finalizar.
2. Tarjetas: leían bytes incluso sin recibos. Omitir huellas en ese caso conservando
   comprobación de existencia y adjuntos obligatorios.
3. Guardar conserva draft_fingerprint fresco, separado de la caché de presentación;
   una edición con igual tamaño/mtime se detecta en la siguiente consulta y al guardar.
4. Dos etiquetas antiguas eliminadas por API; referencias de PR/cachés requieren Soporte GitHub.

## Verificación

Nueve regresiones focales PASA, incluidas dos ejecuciones GUI reales de tarjetas (con/sin recibos).
Resultados: 100 → 1 lecturas por consulta; segunda consulta vuelve a leer. Contexto aislado del
hilo trabajador; errores limpian el contexto; duplicados siguen bloqueados al guardar.
La primera ejecución focal detectó un error de invocación en la prueba GUI, corregido sin cambiar
código de producción ni omitir casos. Verificación completa en curso. Sin pruebas retiradas ni nuevas omisiones; sin modificar motor, textos, golden,
CC, Outlook Save-only ni línea base. #36–38 tienen CI aprobada y ensayo conjunto separado;
no se integran ni se aprueban sin E3 independiente.

## Continuación

E3 de #36–38 y WP-84; integrar aprobados con nueva medición/línea base tras cada integración.
Soporte GitHub: confirmar retiro de referencias y cachés antiguas. WP-02/WP-50/WP-70
siguen sujetos a las pruebas reales y al acta del usuario.
