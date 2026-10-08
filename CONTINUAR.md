# Punto de continuación — plan NURUS / CSMP

**Repositorio:** https://github.com/MatiGaete2023/flow.
**Rama:** `feat/ejecucion-plan-20261008`.
**PR de continuación:** https://github.com/MatiGaete2023/flow/pull/1 (borrador, sin merge).
**Commit P0:** `d74880bf7f1b3d53791e39c5245756848103bf0b`.
**Origen NuRus:** `251d5c4025749edd4767183ebd3d2baf3bb9baed`.
**Importación inicial en flow:** `bd826d524761f24204323e7ad87c287045dc40d3`.
El último checkpoint se obtiene con `git log -1`; el commit P0 anterior contiene
las herramientas, matriz y evidencia. Los checkpoints documentales posteriores
no cambian el comportamiento de las aplicaciones.

## Qué quedó hecho

- Baseline completo y siete adjuntos originales copiados con hashes; las cuatro
  especificaciones 01–04 ya están disponibles.
- P0: inventario de pantallas, 121 módulos/imports, configuración/sesiones,
  inspector, **115 requisitos identificados**, incluidos A-T01..A-T20, y evidencia
  de regresión. Las copias idénticas de adjuntos no duplican requisitos.
- Asistente: 503 pruebas pasaron, 6 omitidas Windows. Descargador: 167 pruebas
  y 57 subpruebas pasaron. Extensión Node, compileall y pip check pasaron.
- Herramientas nuevas: `flow_baseline_inventory.py`, `flow_baseline_matrix.py`
  y `flow_baseline_golden.py`, todas en `tools/`.
- Golden ficticios y showcase; paridad en una segunda generación independiente.
- Propuestas técnicas P2 preparadas; **no hay contrato v1 implementado**.
- Documento 02 analizado contra `0.5.0.dev4`: ver
  [análisis y diferencias](docs/ANALISIS_ESPECIFICACION_02_20261008.md).
  Se incorporaron `NO_SOLICITADA`, importación atómica, modelo por ingreso y
  separación de decisiones locales, productos y hechos institucionales.
- Verificación al incorporar 02: 192 pruebas pasaron, 1 omitida Windows,
  cuatro rechazos del generador comprobados y paridad de 17 artefactos.
  Evidencia enlazada en el análisis del documento.

## Estado por fase

| Fase | Estado | Evidencia / dependencia | Próxima acción |
|---|---|---|---|
| P0 | IMPLEMENTADA_PENDIENTE_ACEPTACION | `docs/EJECUCION_BASELINE_20261008.md`, matriz de 115 requisitos y fixtures; 02 incorporado | Revisar matriz completa y cerrar G0 |
| P1 | BLOQUEADA_POR_EVIDENCIA | SIN_VINCULO y cobertura reales sin contraste nuevo | Operador autorizado prueba mismo EXE/extensión; no pedir credenciales |
| P2 | PENDIENTE | Propuesta de decisiones; requiere G0 | Schema, diccionario y validador con fixture completa/parcial |
| P3 | PENDIENTE | Excel bitácora aún en Asistente | Extraer exportador común tras G0/G2 |
| P4 | PENDIENTE | No existe exportador CSMP_CAPTURE_V1 | ZIP desde fuentes verificadas, temporal/atómico y sin reconsulta |
| P5 | PENDIENTE | No existe importador ZIP v1 | Validar antes de cambiar sesión; conservar Excel legado |
| P6 | PENDIENTE | GUI aún lanza Descargador/bitácoras | Desacoplar tras G3/G5 conservando recuperación |
| P7 | PENDIENTE | No hay temporalidad contextual aceptada | Evidencia por ingreso; sin G1 limitar a fixtures/NO_EVALUABLE |
| P8 | PENDIENTE | Editor/productos legado conservados | Integrar bandeja y decisiones tras G5/G6 |
| P9 | BLOQUEADA_POR_EVIDENCIA | Historia/PDF sin contrato remoto | Obtener muestras institucionales locales autorizadas |
| P10 | BLOQUEADA_POR_EVIDENCIA | Reglas nuevas sin aceptación funcional | Catálogo/casos y aprobación por regla; no cambiar baseline |
| P11 | PENDIENTE | Sin nuevo MVP completo | Integración/aceptación tras gates anteriores |
| P12 | BLOQUEADA_POR_EVIDENCIA | Fuentes/reglas ampliadas pendientes | Solo después de P9/P10 |
| P13 | PENDIENTE | No se publicó nueva distribución | Windows, aceptación y autorización de release |

## Primera tarea al retomar

1. Leer [documentos rectores](docs/plan_20261008/README.md),
   [baseline](docs/EJECUCION_BASELINE_20261008.md) y
   [matriz](docs/MATRIZ_FUNCIONAL_20261008.md).
2. Leer el [análisis del documento 02](docs/ANALISIS_ESPECIFICACION_02_20261008.md)
   y sus A-T01..20 ya incorporados. No volver a pedir el archivo. `review_store`
   y `rus_activity` no existen en el árbol activo; no reintroducirlos. La API
   genérica `registration.submit(adapter)` sigue heredada y debe aislarse en P6;
   las marcas recuperadas de sesiones no son última observación institucional.
3. Comprobar si existe respuesta de aprobación G0/contrato en la conversación
   o revisión de PR. No pedir aprobación de nuevo si ya fue concedida.
4. Primera modificación funcional: validador `CSMP_CAPTURE_V1`. Revisión técnica
   de decisiones: [P2](contracts/csmp_capture_v1/DECISIONES_PENDIENTES.md).
5. Antes de integrar P4/P5, mantener paridad golden y rechazo de ZIP malicioso;
   corte vertical Espera + bitácora ficticia → ZIP → importación → reglas → Excel.

No se hicieron consultas institucionales ni se alteraron reglas/GUI/capturas.
El ZIP del Descargador actual es legado; no anunciarlo como v1. No fusionar a
main ni publicar releases por pasar pruebas. Todo avance nuevo se guarda en flow.
