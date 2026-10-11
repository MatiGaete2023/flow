# Cierre de la implementación (WP-70) — borrador de métricas

11 de octubre de 2026 · punta medida: `feat/ejecucion-plan-20261008` en `51b2477` (historial purgado). **WP-70 no está
cerrado:** faltan el paquete compilado en CI, el acta de uso real del usuario y D2.

## Métricas de término (PLAN_IMPLEMENTACION.md, «Criterio de término y métricas»)

| Métrica | Inicio (`2765b47`) | Ahora | Meta | Cumple |
| --- | --- | --- | --- | --- |
| Identidades reales (`check_real_ids.py` y `--historia`) | 18 | 0 en el árbol; 0 en el historial de las ramas | 0 | Sí, salvo 2 etiquetas viejas que debe borrar el usuario |
| Correos `@pjud.cl` distintos del CC fijo | 14 | 0 (solo `ucc_concepcion@pjud.cl` y el dominio en validaciones) | 0 | Sí |
| Copias de Excel en la sesión tras 3 cargas | 3 | 2 (`tests/test_retencion_sesion.py`, WP-13) | ≤ 2 | Sí |
| Lotes del Descargador con plazo de retención | No | Sí: 60 días y 10 sesiones (D4, WP-13/WP-14) | Sí | Sí |
| Módulos no alcanzables | 12 | 0 | 0 | Sí |
| Archivos versionados | 950 | 736 | ~600 | Parcial |
| Enlaces Markdown rotos | 26 | 0 | 0 | Sí |
| Casos R-T con prueba ejecutada | 2 de 32 | 32 de 32 (WP-60; 6 en xfail estricto por conflicto con el documento 03) | 32 de 32 | Sí, con 6 conflictos abiertos |
| Acciones para llevar un lote al Asistente | ~10 | ~4: «Crear paquete de este lote» → guardar → «Último» → fecha de corte (WP-43, WP-44) | ~4 | Sí |
| Paridad de 17 productos golden | PASA | PASA | PASA | Sí |
| Restricciones absolutas | Sin infracciones | Sin infracciones | Sin infracciones | Sí |
| Actas de uso real firmadas | 0 | 0 | ≥ 2 | **No** (WP-02 y WP-70, del usuario) |

Suites en la punta: asistente 592 aprobadas, 11 omitidas y 6 xfail; descargador 231 aprobadas y 62 subpruebas
(`verificar_wp.py`: PASA). Medido en local: la CI quedó sin runners desde la madrugada del 11 de octubre.

## Para cerrar WP-70

1. **CI:** cuando haya runners, ejecutar el workflow del paquete conjunto (instalador y Descargador) sobre `feat`. El
   usuario recibe los artefactos.
2. **Uso real:** el usuario prueba con datos reales en su PC autorizado y firma el acta
   `docs/aceptacion/ACTA_USO_REAL_<fecha>.json`. Solo él la crea; el agente no la redacta ni la presume.
3. **D2 y WP-02:** listar los casos D-T y A-T que pasaron. WP-50 depende de eso y del acta de 10 ingresos (D5).
4. **D7:** con el acta final, sacar el PR #1 de borrador y fusionarlo a `main`, sin `--force` y sin release.
