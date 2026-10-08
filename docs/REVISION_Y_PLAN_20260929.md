# Revisión de la última versión corregida y plan de implementación

Fecha: 29-09-2026. Base remota verificada: `2fe07fbd26dcdad348b756b25a7cbe8ae9d3717e`,
rama `codex/auditoria-ux-20260925-final`. Uso previsto: una persona, Windows,
Excel/Word y borradores de Outlook. Se revisaron interfaz, filtros, generadores,
ediciones, conciliación, recuperación, contactos, configuración, instalación,
documentación y CI. Esta revisión parte del arreglo RTA/RTT/RVA, que se conserva.

## Evidencia inicial

Al iniciar la revisión, el árbol local estaba limpio y correspondía a los cambios publicados. Las pruebas
de geometría que fallaron el 29-09 no fallan en todas las ejecuciones: dos pruebas
aisladas pasan actualmente. Sí se reprodujo una reducción del editor de correo
de 150 a **66 píxeles** al abrir Filtros a 1024×650. También se reprodujeron un
error al mostrar una dirección editable inválida, una plantilla duplicada que
cambia de destinatario programa a tribunal, una nómina de medidas que toma un
valor de espera en lugar del egreso y una huella que no detecta otra causa nueva
del mismo tribunal. Las demás incidencias se verifican contra las rutas concretas
de código indicadas abajo y tendrán regresiones antes de considerarse cerradas.

## Hallazgos y soluciones, en orden de implementación

### 1. Filtros de correo reducen demasiado el editor — prioridad alta

**Dónde:** `personal/mail_view.py`, `build_mail_page`.
**Problema:** los filtros se insertan sobre el editor y consumen el mismo alto.
En una ventana pequeña el cuerpo queda reducido a pocas líneas. Al obtener cero
borradores, la interfaz tampoco explica si el resultado se debe al alcance,
modalidad, exclusiones o reglas.

**Pasos:** mover los filtros a una pestaña propia del panel de correo; conservar
el acceso «Filtros / opciones»; mostrar un resumen compacto de filtros activos;
dar un mensaje específico al no obtener resultados, sin ampliar automáticamente
la selección ni retirar decisiones de omisión. Guardar y preparar siguen siendo
acciones distintas. **Verificación:** ventanas a 1024×650 y 1180×820, navegación
entre filtros y redacción, botones accesibles y explicación de un resultado vacío.

### 2. Un borrador con datos editables inválidos impide mostrar la lista — alta

**Dónde:** `mail_view.DraftCards.set_drafts` invoca `draft_fingerprint` para obtener
el estado. Ese cálculo valida direcciones y lee adjuntos.
**Problema:** una dirección todavía incompleta o un archivo inaccesible levanta
una excepción al reconstruir las tarjetas; dificulta volver al editor y corregirla.

**Pasos:** separar el estado visual del acto de guardar; capturar errores de
validación/lectura por tarjeta y mostrar «Revisar destinatarios / adjuntos»;
mantener validación estricta al guardar. **Verificación:** la tarjeta con correo
inválido sigue seleccionable, otra válida se muestra y guardar la inválida falla
antes de invocar Outlook.

### 3. Duplicar una plantilla cambia su comportamiento — alta

**Dónde:** `personalization.duplicate_template` y `outputs.prepare_drafts`.
**Problema:** una copia obtiene una clave `particular_*`; el motor deduce tipo,
destino, elegibilidad y formato de nómina a partir de la clave. Una copia de
«Lista de espera · programa» pasa a dirigirse al tribunal y pierde la regla de espera.

**Pasos:** guardar una categoría operativa explícita en las nuevas copias; las
plantillas existentes mantienen la semántica de sus claves conocidas; una copia
de una copia conserva esa categoría. Aplicar la categoría a destino automático,
decisiones, elegibilidad y columnas del adjunto, conservando la clave propia para
texto y archivo. Ofrecer la categoría en Configuración para corregir copias antiguas
sin adivinar su intención. «Preparar todos» conserva sus tipos automáticos canónicos;
las copias se eligen explícitamente. **Verificación:** duplicar un correo de programa
conserva destinatario y filas elegibles y duplicar un informativo conserva tribunal.

### 4. Fechas y adjuntos ignoran algunas correcciones — alta

**Dónde:** `outputs._table`, `prepare_drafts`, `word_values`.
**Problema:** Cumplimiento usa `egreso_proy`, pero la nómina de medidas busca
`vencimiento` o `espera`. La fecha de tarjeta y la fecha de resolución Word se leen
de la celda original, aunque hayan sido corregidas desde la interfaz.

**Pasos:** elegir la fecha según categoría y modo; medidas de Cumplimiento usan
egreso proyectado; informes usan vencimiento; aplicar `value` a todos los datos
operativos corregidos. **Verificación:** egreso y fecha de resolución corregidos
aparecen en nómina, tarjeta y Word, preservando ceros y texto literal en Excel.

### 5. La huella de un correo no cubre todo su grupo — alta

**Dónde:** `personal/product_state.py`.
**Problema:** un informativo agrupado por tribunal se vigila como si fuera un
proyecto por tribunal/RIT. Otra causa de ese tribunal no lo marca obsoleto. Los
nombres abreviados y completos del mismo tribunal tampoco comparten grupo.

**Pasos:** distinguir correo de proyecto al calcular dependencias; usar los mismos
grupos y la misma identidad de tribunal que cada generador; considerar la categoría
de plantilla y el destino real del borrador. **Verificación:** añadir o cambiar otra
causa del mismo tribunal invalida el correo; una causa de otro tribunal no lo hace;
las resoluciones siguen agrupadas por tribunal/RIT.

### 6. Regenerar adjuntos recupera archivos retirados parcialmente — media

**Dónde:** `session.merge_draft_edits`.
**Problema:** solo se conserva el retiro cuando se eliminaron todos los adjuntos
automáticos. Si se quitó uno de varios, reaparece al preparar de nuevo.

**Pasos:** asociar adjuntos automáticos por su nombre estable, conservar cada
retiro individual y las adiciones manuales; usar los nuevos bytes para los que
continúan incluidos. **Verificación:** tres adjuntos, uno retirado y uno añadido;
regeneración conserva exactamente esa decisión y actualiza las rutas automáticas.

### 7. Recuperación incompleta de los filtros — media

**Dónde:** `personal/session.py`.
**Problema:** se recuperan destino y tipo, pero no modalidades, tribunales,
selección manual, búsqueda/filtro de resoluciones ni separadores. La sesión puede
continuar con un alcance distinto al elegido antes de cerrar.

**Pasos:** persistir estas preferencias por IDs; restaurar solo valores válidos
para la configuración/trabajo actual; sesiones anteriores usan los valores
predeterminados. Actualizar el resumen visible después de recuperar.
**Verificación:** guardar/reabrir reproduce filtros y selección; sesión antigua
sin campos nuevos sigue abriendo.

### 8. Historial muestra la fecha de hoy para toda actividad — media

**Dónde:** recibos de `outputs`, `resolutions`, `app._export_current` y
`app_base._update_local_activity`.
**Problema:** el historial inventa una fecha al mostrar recibos antiguos; borradores
carecen de asunto, registros y fecha. «Actividad local» no aclara que se limita al
trabajo activo.

**Pasos:** registrar fecha/hora y metadatos al crear los recibos; conservarlos al
pasar de guardando a incierto o creado; mostrar «Fecha no registrada» para recibos
antiguos; identificar el panel como actividad del trabajo actual. **Verificación:**
reabrir al día siguiente conserva las fechas; un guardado incierto conserva asunto
y registros; un recibo antiguo no adquiere una fecha ficticia.

### 9. Fallar al guardar la sesión permite cerrar y perder cambios — alta

**Dónde:** `app_base._save_session`, `_close`, `_autosave`, `_poll`.
**Problema:** `_save_session` absorbe `OSError` y `_close` destruye la ventana de
todas formas. Otros errores durante el guardado pueden interrumpir el sondeo de
operaciones y dejar la interfaz sin recibir resultados.

**Pasos:** propagar el error de persistencia; mantener la ventana abierta al fallar
el cierre; presentar el error y reprogramar autoguardado/sondeo incluso si falla
la recuperación. **Verificación:** disco/carpeta no escribible simulado impide el
cierre, conserva cambios y permite seguir procesando eventos.

### 10. Conciliación recalcula con el mapeo anterior — alta

**Dónde:** `sync.refresh`, antes de asignar `work.mapping=data['mapping']`.
**Problema:** si Excel cambia un encabezado por otro alias válido y modifica un
dato operativo, `recalculate` ve el mapeo viejo contra las nuevas celdas; produce
observaciones con datos ausentes.

**Pasos:** recalcular contra una instantánea con el nuevo mapeo y encabezado;
mantener la confirmación completa al final y sin mutaciones ante conflictos.
**Verificación:** cambiar encabezado de espera por su alias y días produce la
misma propuesta que analizar el archivo nuevo; conflictos no alteran el trabajo.

### 11. Un contacto nuevo puede colisionar con un alias ajeno — media

**Dónde:** `personalization.save_contact`.
**Problema:** se comprueban contactos y los alias entrantes, pero no si el nombre
nuevo ya es alias de otro contacto. La resolución del destinatario puede seguir
apuntando al contacto anterior y ocultar el nuevo.

**Pasos:** validar el nombre contra alias ajenos; permitir conservar los alias
propios al renombrar; rechazar el cambio con un mensaje concreto y sin mutar la
configuración. **Verificación:** colisión rechazada y renombre con alias propios válido.

### 12. Documentación, versión y elementos redundantes — media

**Dónde:** README, portadas documentales, versión de paquete, workflow y
`_draft_originals`, además de argumentos/importaciones sin consumidores.
**Problema:** las portadas mezclan dev11, dev12 y estados del 22-09; la rama de
trabajo no dispara CI por push; el diccionario de originales por identidad de objeto
duplica `Draft.original` y crece en cada preparación. `confirmed_scope` no se usa.

**Pasos:** publicar dev13 con instrucciones y estado actuales; apuntar el índice
a los documentos vigentes y marcar checkpoints antiguos como históricos; incluir
la rama de trabajo en CI; quitar el diccionario duplicado, el argumento sin uso
y las importaciones comprobadas como innecesarias. Se mantienen archivos históricos
útiles y componentes compartidos con consumidores/pruebas; no se borran por antigüedad.
**Verificación:** una versión común en paquete/metadatos/distribución, instalación
fuera del árbol fuente y enlaces documentales correctos; compilación y suite completa.

## Secuencia y criterio de cierre

1. Escribir regresiones de los errores y conservar esta evidencia inicial.
2. Implementar primero integridad de fechas, plantillas, dependencia y conciliación.
3. Implementar recuperación, recibos, guardado y validación de contactos.
4. Simplificar paneles de correo y estado de las tarjetas; probar ventanas reales.
5. Actualizar documentación/versión/CI y retirar duplicación confirmada.
6. Ejecutar regresiones enfocadas, suite completa, compilación y prueba del paquete
   instalado; inspeccionar fallos antes de publicar.
7. Registrar por cada punto el resultado, evidencia y limitaciones en este mismo
   informe. Publicar el código revisable y la versión descargable.

No se considera terminado un punto por tener código escrito: debe cumplir su
criterio de verificación. Las pruebas usan datos ficticios y adaptadores de Outlook
simulados; no acreditan un envío ni una nueva aceptación institucional de Office.

## Registro de implementación

Implementación final: **0.4.0.dev13**, sobre la base indicada al inicio. Los doce
puntos están implementados. Esta tabla enlaza cada cierre con su evidencia;
los procedimientos y criterios anteriores se conservan para futuras regresiones.

| Punto | Resultado implementado | Evidencia de cierre |
|---|---|---|
| 1 | Filtros en pestaña propia, resumen compacto y mensajes para selección vacía, exclusiones, modalidades y gestiones. | `test_editors_and_actions_fit_without_detail_overlap`: ventanas Windows de 1024×650 y 1180×820; editor de al menos 120 px tras volver de Filtros. `test_empty_preparation_explains_modality_and_exclusion`. |
| 2 | Las tarjetas siguen visibles con datos inválidos; validación estricta antes de guardar. | `test_invalid_editable_draft_does_not_hide_cards_or_editor`; `test_invalid_editable_draft_is_rejected_before_outlook`: destinatario inválido y archivo ausente no invocan el adaptador. |
| 3 | Categoría operativa preservada al duplicar; editable en Configuración para copias antiguas. | Regresiones de copia de copia de programa, columnas y omisión; copia de informativo mantiene tribunal. Prueba del paquete instalado: elegir una copia antigua, cambiar Comportamiento a programa y guardar conserva `categoria`. |
| 4 | Correcciones de egreso y fecha de resolución llegan a los productos. | `test_corrected_measure_due_and_resolution_date_are_used_in_products`: nómina y tarjeta usan 15/10/2026 corregido; Word usa dos de septiembre de dos mil veintiséis. |
| 5 | Huella coherente con el grupo del generador y destino real. | `test_mail_dependencies_cover_whole_canonical_court_group`: otra causa de Mulchén invalida el informativo; Laja no; pruebas existentes de resoluciones preservadas. |
| 6 | Retiro individual de adjuntos y adiciones manuales sobreviven regeneración. | `test_partial_attachment_removal_and_manual_addition_survive_regeneration`: se conservan A/C y el archivo manual; B retirado no reaparece. |
| 7 | Persistencia de modalidades, tribunales, selección manual, filas, búsqueda/filtro y separadores. | `test_full_session_and_contextual_fields_recover`, con dos ventanas reales. La regresión de tarjetas incluye recuperar una sesión antigua sin preferencias nuevas. |
| 8 | Fecha/hora real, asunto, registros y estado de los recibos; actividad del trabajo activo. | `test_receipt_metadata_and_activity_date_survive_session_reload`; `test_uncertain_outlook_save_preserves_activity_metadata`; recibos antiguos muestran Fecha no registrada. |
| 9 | Fallar la persistencia impide cerrar; el sondeo vuelve a programarse. | Regresiones de cierre sin permiso, propagación de `PermissionError` y continuidad de `_poll`; datos de recuperación siguen en memoria. |
| 10 | Conciliación recalcula con los encabezados actuales antes de confirmar cambios. | `test_sync_recalculates_against_new_valid_header_mapping`: `T ESPERA` pasa a `DIAS_ESPERA` y 45 días a 5; propuesta recalculada correctamente. Regresiones existentes de conflictos sin mutación pasan. |
| 11 | Colisión de un nombre con alias ajeno rechazada antes de modificar contactos. | `test_contact_name_cannot_hide_another_contact_alias`: configuración original intacta; renombre sobre alias propio permitido. |
| 12 | Versión común dev13, portadas actuales, antecedentes identificados, CI por push a codex, eliminación de duplicación/importaciones/argumento sin uso. | Contratos de versión y documentos, revisión del índice Git, `diff --check`, compilación y wheel instalado fuera del árbol fuente; recursos 5 parches y 6 plantillas Word presentes. |

### Corrección de Mulchén incluida

Se conserva y verifica la corrección de identificación de tribunal y modalidades
residenciales. El fallo específico ocurrió **antes de Outlook**: RTA/RTT/RVA no
eran reconocidos al filtrar Residencial. La selección manual también aplicaba ese
filtro. Ver [diagnóstico y regresión](correccion_residencial_20260929.md).

La interfaz Windows de esta versión muestra la tarjeta, destinatarios y asunto
en preparación automática y manual. El paquete instalado se probó con cinco
variantes ficticias del tribunal, incluidos diéresis, acento descompuesto y orden
distinto de palabras, y programas RTA/RTT/RVA: un borrador de cinco registros en
automático y uno de un registro seleccionado en manual. Guardar llega al adaptador
simulado y registra el recibo. Esto no acredita un guardado real en Office.

### Verificación y límites

- Suite completa Windows / Python 3.13: **362 aprobadas, 1 omitida** en 46,02 s.
  La omisión prueba rechazo del backend nativo fuera de Windows. Dos regresiones
  adicionales de validación antes de Outlook pasan en la ejecución enfocada:
  **20 aprobadas** incluyendo contratos de versión/documentación.
- `compileall` de fuentes, pruebas y herramientas: correcto; `git diff --check`:
  correcto. No se eliminan documentos históricos útiles ni consumidores compartidos.
- Wheel `nurus-0.4.0.dev13-py3-none-any.whl` construido y instalado con pip en una
  carpeta aislada; proceso nuevo importa desde esa instalación, sin `src` ni
  `tests` en su ruta. Verificación de recursos y flujo de interfaz completada.
- Para esta revisión se usaron dependencias de prueba en el workspace. El sandbox
  Windows aplica ACL especiales a directorios temporales 0700; el arnés de revisión
  adapta esos permisos solo en sus procesos. No cambia el código de la aplicación
  ni las políticas de Windows. El build de pip en subproceso presentó esa restricción;
  el build con el backend estándar en el arnés y la instalación terminaron correctamente.
- Los adaptadores de Office se simularon. No se enviaron correos, no se crearon
  borradores reales y no se modificaron los archivos personales del usuario.
- Las huellas nuevas pueden marcar productos preparados por versiones anteriores
  como pendientes de actualización. Volver a preparar conserva las ediciones
  mediante la recuperación de borradores y actualiza las dependencias.
- CI de tres versiones Windows queda configurado para la rama de trabajo. Los
  resultados locales anteriores no se presentan como resultados del nuevo run remoto.

### Instalar y comprobar

1. Cerrar el asistente abierto.
2. Descomprimir `CSMP_Assistant_personal_0.4.0-dev13.zip`, o descargar la rama
   `codex/auditoria-ux-20260925-final`.
3. Ejecutar `Instalar_CSMP.bat` desde la carpeta descargada. El instalador copia
   el paquete al entorno virtual: bajar fuentes sin reinstalar no actualiza la app.
4. Ejecutar `Abrir_CSMP.bat` en esa misma carpeta; comprobar `0.4.0.dev13` en el título.
5. En Correos → Filtros elegir tribunal Mulchén, destino Tribunales y Residencial.
   Preparar el informativo del modo de la hoja; para selección manual, seleccionar
   antes las filas en Trabajo y activar la opción manual.
6. Revisar la tarjeta, destinatarios, asunto, texto y adjuntos; guardar el borrador
   en Outlook. Si el resultado es vacío, el nuevo mensaje identifica el filtro o
   la falta de gestiones y permite corregirlo sin ampliar el lote automáticamente.

La configuración y la sesión permanecen en `CSMP_Personal` de la carpeta de usuario.
