# Punto de continuación — plan NURUS / CSMP

**Repositorio:** https://github.com/MatiGaete2023/flow.
**Rama:** `feat/ejecucion-plan-20261008`.
**Origen NuRus:** `251d5c4025749edd4767183ebd3d2baf3bb9baed`.
**Importación inicial en flow:** `bd826d524761f24204323e7ad87c287045dc40d3`.
El commit exacto de la entrega se obtiene con `git log -1`; este archivo viaja
en el mismo commit y no se refiere a un SHA futuro inventado.

## Qué quedó hecho

- Baseline completo y cinco adjuntos originales copiados con hashes.
- P0: inventario de pantallas, 121 módulos/imports, configuración/sesiones,
  inspector, 95 requisitos identificados y evidencia de regresión.
- Asistente: 503 pruebas pasaron, 6 omitidas Windows. Descargador: 167 pruebas
  y 57 subpruebas pasaron. Extensión Node, compileall y pip check pasaron.
- Herramientas nuevas: `flow_baseline_inventory.py`, `flow_baseline_matrix.py`
  y `flow_baseline_golden.py`, todas en `tools/`.
- Golden ficticios y showcase; paridad en una segunda generación independiente.
- Propuestas técnicas P2 preparadas; **no hay contrato v1 implementado**.

## Estado por fase

| Fase | Estado | Evidencia / dependencia | Próxima acción |
|---|---|---|---|
| P0 | IMPLEMENTADA_PENDIENTE_ACEPTACION | `docs/EJECUCION_BASELINE_20261008.md`, matriz y fixtures | Revisar matriz y cerrar G0; incorporar especificación 02 |
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
2. Incorporar `02_ESPECIFICACION_TECNICA_ASISTENTE_CSMP.md` cuando sea accesible.
   El usuario informó que la subió; esta ejecución aún no ve el archivo. No
   inventar sus A-T01..20. Conservar los adjuntos anteriores sin edición.
3. Comprobar si existe respuesta de aprobación G0/contrato en la conversación
   o revisión de PR. No pedir aprobación de nuevo si ya fue concedida.
4. Primera modificación funcional: validador `CSMP_CAPTURE_V1`. Revisión técnica
   de decisiones: [P2](contracts/csmp_capture_v1/DECISIONES_PENDIENTES.md).
5. Antes de integrar P4/P5, mantener paridad golden y rechazo de ZIP malicioso;
   corte vertical Espera + bitácora ficticia → ZIP → importación → reglas → Excel.

No se hicieron consultas institucionales ni se alteraron reglas/GUI/capturas.
El ZIP del Descargador actual es legado; no anunciarlo como v1. No fusionar a
main ni publicar releases por pasar pruebas. Todo avance nuevo se guarda en flow.
