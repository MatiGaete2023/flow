# Índice y vigencia de documentación

Actualizado: 22 de septiembre de 2026. Rama de evolución vigente: **0.4.0.dev12**; dev11 permanece como candidata funcional congelada previa.

## Documentos vigentes para CSMP Assistant personal

- `EVOLUCION_UX_REGLAS_20260922.md`: cambios dev12 de UX y redacciones/reglas aprobadas sobre la candidata congelada dev11.

- `CONGELAMIENTO_CANDIDATA_20260922.md`: checkpoint funcional congelado, política de cambios y criterio para avanzar hacia 1.0.
- `UX_FINAL_20260922.md`: pulido final de búsqueda, filtros, contexto, atajos y visualización por excepción.
- `RES_CATEGORICO_20260922.md`: cierre funcional de `RES` como tipo explícito, validación, compatibilidad y UX de Resoluciones.
- `PANEL_CORREOS_20260921.md`: panel oscuro dev9, alcance de destinatarios, plantilla del manual y verificación.
- `LIMPIEZA_ASISTENTE_20260921.md`: auditoría dev8, código retirado, errores corregidos y verificación.
- `REVISION_USABILIDAD_20260921.md`: evidencia fechada de dev7; mejoras previas que se conservan.

- `ESTADO_CONSOLIDADO_20260920.md`: **referencia canónica de estado**; reúne evolución, observaciones del usuario, soluciones, decisiones, conflictos y próximos pasos.
- `../README.md`: instalación, uso y alcance actual.
- `IMPLEMENTACION.md`: portada del estado operativo del repositorio.
- `IMPLEMENTACION_PERSONAL.md`: comportamiento funcional vigente, incluido el alcance de correos y cierre dev9.
- `AUDITORIA_REPOSITORIO_20260916.md`: auditoría integral y limpieza de código del 16-09-2026; evidencia fechada que complementa el estado consolidado.
- `CAMBIOS_USO_20260916.md`: cambios funcionales y técnicos vigentes incorporados el 16-09-2026.
- `VERIFICACION_PERSONAL.md`: evidencia automática y límites.
- `ACEPTACION_CSMP_PERSONAL.md`: protocolo de prueba institucional.
- `MATRIZ_REGLAS_PERSONAL.json`: índice documental de reglas, sincronizado con `textos_base.json`.
- `REFERENCIA_CATALOGO_ASISTENTE.md`: referencia congelada de reglas de dominio; una modificación de regla exige actualizar código y pruebas.

## Documentos históricos

- `CAMBIOS_USO_20260914.md`: estado consolidado de la versión 0.4.0.dev5 al 14-09-2026. Se conserva como trazabilidad y no describe la release vigente.
- Los documentos fechados entre el 8 y el 13 de septiembre (`AUDITORIA_*`, `CHECKPOINT_*`, `REVISION_*`, `EJECUCION_*`, `RENDIMIENTO_*`, `README_NURUS_DEV7.md`, `INSTALACION_Y_EXCEPCION_*` y equivalentes) describen estados anteriores. Se conservan para trazabilidad y **no sustituyen** los documentos vigentes anteriores.

Referencias históricas a CI Linux, versiones dev1/dev2/dev6/dev7 anteriores, excepción obligatoria de cruce o arquitecturas previas deben interpretarse dentro de su fecha. La coincidencia del identificador “dev6” con documentación histórica no convierte aquella documentación en vigente: la release actual se identifica por el paquete 0.4.0.dev11 y el SHA registrado en `VERIFICACION_PERSONAL.md`.

La rama `revision-producto-csmp-2026-09-13` conserva una auditoría documental independiente del 13-09; sus conclusiones relevantes están incorporadas en `ESTADO_CONSOLIDADO_20260920.md` y no requieren fusionar su runtime.

La interfaz NuRus, `product_ui.py`, el benchmark histórico y las versiones duplicadas de métodos del Asistente se retiraron en dev8. El historial Git conserva sus fuentes. Se mantienen los servicios compartidos y sus regresiones; el nombre técnico `nurus` no designa un segundo producto activo.
