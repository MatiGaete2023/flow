# P0.03 — Inventario del Descargador

Código inspeccionado: NuRus `251d5c4025749edd4767183ebd3d2baf3bb9baed`.
Versión del Descargador/extensión: `2.7.2`. Los nombres de pantallas se toman
de `descargador/perfiles.py`; las opciones concretas provienen del catálogo
de la sesión, no de valores inventados por el exportador.

| Pantalla | Pestañas/unidad | Selección y filtros | Productos actuales | Pruebas de partida |
|---|---|---|---|---|
| Seguimiento | Espera, Cumplimiento, Informes, Egresados | Tribunal, modalidad, tipo de informe/filtro de cumplimiento disponible, páginas | Original por página, verificación de consulta, consolidado y CSV; modo CSMP para las tablas compatibles | `test_poc.py`, `test_fase2.py`, `test_lotes.py`, `test_periodos_xls.py` |
| Informes por vencer | Calendario mensual | Tribunal, mes y año del catálogo; sin multiplicación por modalidad | Original, XLSX/CSV y calendario ICS cuando hay fechas válidas | `test_flujos.py`, `test_filtros_respuesta.py`, `test_resultados_ics_semantics.py` |
| Medidas por vencer | Calendario mensual | Tribunal, mes y año | Original, derivados y calendario ICS cuando corresponde | `test_flujos.py`, `test_filtros_respuesta.py`, `test_resultados.py` |
| Litigantes/órdenes | Consulta y páginas | Tribunal, estado disponible | Original y derivados verificables; no confundir con ingreso en seguimiento | `test_flujos.py`, `test_fase3.py` |
| Carga/resoluciones firmadas | Intervalo de consulta | Tribunal, fechas válidas, hasta 30 días inclusivos | Tabla de firmas y cobertura parcial; no prueba historia completa por fecha de firma | `test_carga.py`, `test_flujo_csmp.py` |
| Bitácoras | Ingreso con vínculo real | Tribunal+causa+ingreso, etapa, modalidad, pestaña; GET de lectura autorizado | HTML local íntegro, `bitacoras.json`, diagnóstico categórico | `test_lectura_bitacoras.py`, `test_vinculos.py`, prueba Node de extensión |
| Historia SITFA | Vínculo `ShowHistoria` reconocido | Granularidad causa/ingreso todavía no acreditada | No hay extractor general aceptado | P9: bloqueado por capturas/contrato reales |
| Informe general | No identificado | Falta pantalla/catálogo real | No anunciar como disponible | Conflicto C05/CA-01 |

`descargador/resultados.py` conserva exportación XLSX, CSV, ICS, ZIP de lote
y comparación de lotes. **Ese ZIP no es `CSMP_CAPTURE_V1`.** La ventana de
resultados y los favoritos/preferencias se conservan; no se cambiaron sus nombres
ni el comportamiento de pausa, cancelación y recuperación.

## Estados y errores que deben conservarse

- Consultas: `VALIDADA`, `SIN_RESULTADOS`, `INCOMPLETA` en el contrato legado.
  Necesitan un adaptador explícito a los estados por fuente del futuro contrato.
- Bitácoras: `LEIDA`, `FALLIDA`, `SIN_VINCULO`; consultas pendientes/no consultadas
  permanecen distinguibles de una tabla vacía comprobada.
- Se rechazan sesión vencida, pantalla/filtros diferentes, página ajena/repetida,
  cambios en totales, enlace de exportación no permitido, hash o tamaño alterado,
  rutas externas al lote y cabeceras ambiguas.
- Diferencias visuales con IDs remotos iguales generan advertencia; IDs de otro
  ingreso bloquean la asociación. No completar IDs con RIT, nombre o ceros.

## Límites comprobados en el código

`resultados.MAX_ROWS=200000`, `MAX_JSON=4 MiB`; archivos de origen hasta 80 MiB
y HTML de bitácora hasta 8 MiB donde se valida su reutilización. El intervalo
de análisis de bitácoras llega a cuatro meses y no certifica historia remota total.

El Excel de bitácoras sigue en `personal.bitacora_lote.export_lote`/
`personal.bitacoras`; hacerlo autónomo requiere P3. El parser común
`nurus.bitacora_html` y `descargador.csmp_shared` se conservarán, con pruebas
de empaquetado que no requieran la GUI del Asistente.
