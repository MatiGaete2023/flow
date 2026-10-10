# Ajustes de uso — 14 de septiembre de 2026

Versión consolidada: **0.4.0.dev5**. Objetivo: preparar Excel, Word y borradores con menos pasos, manteniendo revisión humana y sin escribir en RUS/SATURNO ni enviar correos.

## Cambios consolidados

- Solo Windows; CI en Python 3.12, 3.13 y 3.14.
- Observaciones de gestiones ejecutadas restauradas a “Se remite…” / “Se remite proyecto…”. La matriz documental de reglas se sincroniza con `textos_base.json` y existe una regresión que impide divergencias futuras.
- FAS se clasifica como Familia de acogida.
- La planilla revisada respeta el modo elegido cuando contiene columnas de otros modos.
- Cumplimiento sin cruce utilizable deja advertencia y no evalúa C-10; ya no bloquea ni muestra diálogo de excepción en CSMP Assistant personal.
- Correos: selección de modalidades, editor ampliado, carga de planilla modificada/externa, guardado de todo el lote y adjuntos automáticos con nombre del programa. Se agrega **Preparar TODOS los correos necesarios**, que incluye el informativo general de la pestaña y los correos específicos derivados de reglas en una sola preparación.
- Correos: los formatos generales de Espera, Cumplimiento, Informes, medidas sin vigencia/por vencer e informes por vencer se ajustan al Manual de Funciones CSMP 2025. La migración de configuración repone plantillas incorporadas en versiones posteriores sin sobrescribir textos personalizados por el usuario.
- Correos: las fechas provenientes de celdas Excel se muestran en los adjuntos como `dd/mm/aaaa`, sin agregar `00:00:00`, y los valores numéricos cero dejan de convertirse accidentalmente en celdas vacías.
- Correos Outlook: cuando se usa `Display()` para incorporar la firma institucional, el Inspector se cierra después de guardar y comprobar el borrador, evitando dejar una ventana abierta por cada elemento del lote. No se incorpora ninguna llamada de envío.
- Las planillas exportadas conservan `NURUS_REGLAS` en columna técnica oculta; al reabrirlas se reconstruyen acciones/correos. Para copias anteriores se usa `NURUS_TRAZABILIDAD` como respaldo cuando es posible.
- Resoluciones: tipo manual independiente, agrupación tribunal + RIT + tipo, una resolución por grupo y un único Word con salto de página entre proyectos. Las fechas generadas se expresan completamente en palabras.
- Resoluciones: la columna humana `RES` de una planilla revisada prevalece sobre las sugerencias del motor; si se usa, solo los registros marcados afirmativamente entran al lote automático. Para varios NNA en un mismo RIT, cada nombre queda inmediatamente asociado a su propia cédula en la redacción principal.
- Resoluciones: cuando una causa contiene más de un tipo explícito en `RES`, el orden se obtiene de la planilla y no del orden interno de una estructura no determinista.
- Matrices: seis bases disponibles (Laja y Mulchén). Cinco cuerpos revisados se verifican por SHA-256; PC_INFO Laja se incorporó desde Word antiguo. Tomé sigue pendiente por falta de fuente.
- La revisión de matrices repone correctamente un archivo eliminado aun cuando la revisión ya estaba marcada como aplicada.
- Recursos de matrices divididos en cinco payloads pequeños independientes para evitar corrupción/truncamiento del archivo monolítico anterior.
- `.gitignore` cubre ahora `.venv-csmp`, `dist`, `build` y `*.egg-info` para evitar incorporar entornos o productos locales al repositorio.

## Verificación

La CI comprueba instalación aislada, wheel, dependencias, compilación, suite, cinco parches y seis matrices. Python 3.12 agrega smoke GUI y distribución Windows. El run exacto de cada release se identifica por el SHA del HEAD; no se usa un número de ejecución fijo como fuente única porque quedaría obsoleto al actualizar documentación.

## Límites y conflicto abierto

No hay matrices de Tomé. La CI no acredita comportamiento real de Excel 2010/Outlook clásico ni calidad jurídica de una resolución concreta; esas verificaciones siguen siendo humanas/institucionales.

[CONFLICTO_ABIERTO] El Manual de Funciones CSMP 2025 indica, para DCE en Informes, pedir cuenta de informes pendientes de entrega de más de 40 días. El motor vigente clasifica como `I01_VENCIDO_DCE` cualquier informe DCE cuya fecha de vencimiento ya pasó. No se modifica automáticamente esta regla en esta revisión porque puede existir una decisión operativa posterior al manual y debe resolverse contra el perfil Asistente vigente antes de alterar producción.

[G-ESTADO] Código y documentación alineados para 0.4.0.dev5, con correcciones de migración, salidas, Outlook y determinismo incorporadas.
[L-SIGUIENTE] Ejecutar aceptación Office institucional con copias autorizadas y resolver el conflicto de umbral DCE contra la instrucción operativa vigente.
