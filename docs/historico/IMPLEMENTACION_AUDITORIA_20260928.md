# Implementación de la auditoría técnica y UX

Esta rama aplica las correcciones y ampliaciones del informe de revisión del commit `b5efb68665954ed28b0b6a32fced6ea5e204de00`.

## Integridad y continuidad

- `nurus.personal.sync` asocia filas por `NURUS_ID_REGISTRO` o por una identidad compuesta única. La base, la copia actual, la hoja, el encabezado, el mapeo y la posición se actualizan como una sola transacción.
- La actualización usa tres versiones por campo: última sincronización, edición local y Excel. Los cambios concurrentes levantan `SyncConflict`; la ventana de resolución permite conservar Excel, conservar la edición o combinarla. Cancelar no muta el trabajo.
- El origen analizado queda archivado por SHA-256 y no puede ser elegido como destino de exportación. Las copias de recuperación se verifican antes de reutilizarlas.
- Los generadores verifican que la copia Excel esté sincronizada antes de preparar correos, Word o estadísticas. Una selección vieja no se reutiliza silenciosamente.
- `NURUS_DECISIONES` conserva en la copia las decisiones estructuradas: tipo de resolución, gestiones incluir/omitir, exclusión, correcciones de origen y datos Word. `RES=0`, `False`, `No` y vacío no son advertencias.

## Productos y recuperación

- Cada borrador y proyecto tiene una huella de sus registros, configuración y matriz. Cambiar la decisión, el registro o la plantilla marca el producto como desactualizado antes de guardarlo.
- Las nóminas generadas normalizan metadatos ZIP y fechas para conservar bytes estables. Regenerar el mismo contenido en otra carpeta no crea un segundo borrador; modificar el adjunto sí cambia la huella.
- La sesión JSON versión 2 guarda borradores, adjuntos, proyectos, textos, decisiones, preferencias, recibos y el trabajo pendiente. Se crea una copia `.bak` y se comprueban las huellas de los libros.
- `Ctrl+S` y el guardado automático conservan la sesión local. Las entradas se bloquean mientras un hilo procesa una instantánea estable.

## Edición contextual

Trabajo incorpora un formulario por registro para `FECHA_OBS`, `TT`, `CC`, `RES`, inclusión en el lote, gestiones de correo, correcciones de origen y variables de Word. Los campos vacíos siguen significando “sin indicar”. La edición masiva se limita a campos seguros y tiene deshacer.

Correos separa Correo, Registros incluidos y Cambios; sus tarjetas distinguen preparado, editado, desactualizado, guardado y guardado incierto. Resoluciones usa una sola lista de causas con paneles Texto, Datos, Matriz y Cambios; requiere confirmar una revisión cuando se conserva un texto editado sobre una base nueva.


## Verificación

La suite ejecutada en Windows/Python 3.13 terminó con **306 pruebas correctas y 1 omitida**. Las nuevas pruebas cubren ordenación, encabezados desplazados, conciliación y conflictos, decisiones estructuradas, productos obsoletos, recuperación, campos contextuales, personalización y geometría a 1024×650 y 1180×820. El smoke de la ventana real terminó correctamente con captura adaptada por la limitación de `win32ui` del entorno de revisión.

La integración real con una instalación concreta de Excel/Outlook debe verificarse en el equipo de uso antes de distribuir un ejecutable. La rama no envía correos durante las pruebas.
