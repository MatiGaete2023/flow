# Cierre de la implementación (WP-70) — borrador de métricas

12 de octubre de 2026 · punta medida: `feat/ejecucion-plan-20261008` en `5a8f68e` (consolidación de WP-86, su E3 y la
[revisión integral](REVISION_INTEGRAL_20261011.md); rama `consolidacion/20261012`). `verificar_wp.py` PASA en `5a8f68e`.
**WP-70 no está cerrado:** faltan las actas del usuario y D2.

## Métricas de término (PLAN_IMPLEMENTACION.md, «Criterio de término y métricas»)

| Métrica | Inicio (`2765b47`) | Ahora | Meta | Cumple |
| --- | --- | --- | --- | --- |
| Identidades reales (`check_real_ids.py` y `--historia`) | 18 | 0 en el árbol; 0 en el historial; etiquetas antiguas ya borradas del remoto | 0 | Sí; refs/pull y cachés pendientes de Soporte GitHub (WP-17) |
| Correos `@pjud.cl` distintos del CC fijo | 14 | 0 (solo `ucc_concepcion@pjud.cl` y el dominio en validaciones) | 0 | Sí |
| Copias de Excel en la sesión tras 3 cargas | 3 | 2 (`tests/test_retencion_sesion.py`, WP-13) | ≤ 2 | Sí |
| Lotes del Descargador con plazo de retención | No | Sí: 60 días y 10 sesiones (D4, WP-13/WP-14) | Sí | Sí |
| Módulos no alcanzables | 12 | 0 (`modulos_no_alcanzables.py`: 100 módulos, 94 desde la app, 6 solo herramientas/Descargador) | 0 | Sí |
| Archivos versionados | 950 | 776 en `5a8f68e` | ~600 | Parcial |
| Enlaces Markdown rotos | 26 | 0 | 0 | Sí |
| Casos R-T con prueba ejecutada | 2 de 32 | 32 de 32 (WP-60; 6 en xfail estricto por conflicto con el documento 03) | 32 de 32 | Sí, con 6 conflictos abiertos |
| Acciones para llevar un lote al Asistente | ~10 | ~4: «Crear paquete de este lote» → guardar → «Último» → fecha de corte (WP-43, WP-44) | ~4 | Sí |
| Paridad de 17 productos golden | PASA | PASA | PASA | Sí |
| Restricciones absolutas | Sin infracciones | Sin infracciones | Sin infracciones | Sí |
| Actas de uso real firmadas | 0 | 0 | ≥ 2 | **No** (WP-02 y WP-70, del usuario) |

Línea base vigente (`linea_base.json`, regenerada sobre `5a8f68e`): asistente 858 aprobadas, 11 omitidas y 6 xfail;
descargador 240 aprobadas y 62 subpruebas.

## Para cerrar WP-70

1. **Acta de uso real (WP-02, D2):** plantilla `docs/aceptacion/ACTA_USO_REAL_20261012.json` para `5a8f68e`
   (40 casos D-T/A-T, `NO_EJECUTADO`, tipo de evidencia vacío, decisión h). El usuario prueba con datos reales en su PC autorizado, elige el tipo y
   firma esa acta (o una regenerada si instala otro SHA). El agente no la redacta ni la presume.
2. **D2:** listar los casos D-T y A-T que pasaron.
3. **Acta de 10 ingresos (WP-50):** la cronología está visible en `feat` desde `8c4036d` (decisión g); su aceptación
   espera ese contraste.
4. **Soporte GitHub (WP-17):** retirar `refs/pull` y cachés. Las etiquetas `pre-WP-12` y
   `pre-implementacion-20261009` ya no existen en el remoto.
5. **Acta final y D7:** con el acta firmada, sacar el PR #1 de borrador y fusionarlo a `main`, sin `--force` y sin
   release.
