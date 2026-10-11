# Correos alineados al Creador CSMP v2.1 — 14 de septiembre de 2026

Referencia funcional: `Creador de Correos.py` aportado por el usuario.

La pestaña **Correos** del CSMP Assistant mantiene el motor integrado de NuRus, pero adopta el flujo de uso del creador independiente:

1. selección del tipo de correo;
2. selección explícita de uno, varios o todos los tribunales;
3. selección de una, varias o todas las modalidades, con botones Todos/Ninguna;
4. período editable;
5. borradores preparados y correo completamente editable antes de guardar en Outlook.

Se conserva la redacción semántica del alcance: `la modalidad ...`, `las modalidades ...` y `todas las modalidades`, incluyendo el uso de `e` antes de palabras iniciadas con sonido i.

Las plantillas que declaran `usa_modalidades: false` ya no exigen seleccionar una modalidad. La selección de tribunales filtra los registros del trabajo antes de preparar los borradores. La selección manual de registros de la pestaña Trabajo sigue disponible y se cruza con los tribunales elegidos.

Se agregan controles para vista previa, adjuntar archivos al borrador actual, adjuntar los mismos archivos a todo el lote y quitar los adjuntos del borrador actual.

Se mantienen las capacidades propias de NuRus que superan al creador independiente: detección automática de comunicaciones específicas según reglas, generación automática de adjuntos por programa, recuperación desde planilla modificada/externa, prevención de duplicados mediante recibos de trabajo y guardado masivo sin envío.

Invariantes: solo Windows/Outlook de escritorio; solo se guardan borradores; nunca se llama a `Send()`; no se escribe en RUS ni SATURNO; UCC Concepción sigue incorporada como copia institucional.
