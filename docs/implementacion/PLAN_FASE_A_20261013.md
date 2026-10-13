# Plan de la Fase A: revisión por excepción y bandeja de atención

13 de octubre de 2026 · base `feat/ejecucion-plan-20261008` en `54a976c` · encargo del usuario: «Planifica la
implementación de esta fase para que otra IA la ejecute sin problemas».
Origen: la propuesta del usuario (bandeja, revisión por excepción, cierre visible por caso y medición) y el análisis
de la sesión del 13 de octubre.

## 1. Objetivo y límites

**Objetivo:** que el funcionario revise solo lo que cambió desde la captura anterior y vea, por caso, qué falta y
cuál es la próxima acción, sin buscar entre pestañas.

**Paquetes, en este orden** (cada uno se integra por separado):

| WP | Ficha | Qué entrega | Riesgo (decisión j) |
| --- | --- | --- | --- |
| WP-95 | [wp/WP-95.md](wp/WP-95.md) | Comparación de capturas sucesivas y filtro «Solo cambios» | No riesgoso; sin E3 |
| WP-96 | [wp/WP-96.md](wp/WP-96.md) | Estado por caso, motivo y próxima acción; bandeja «Qué requiere mi atención» | No riesgoso: solo lee recibos; sin E3 |
| WP-97 | [wp/WP-97.md](wp/WP-97.md) | Métrica local opcional (conteos, nunca datos) para decidir la Fase B | No riesgoso; sin E3 |

**No negociable** (AGENTS.md y PLAN_VIGENTE.md):
- `src/nurus/personal/motor/` y `textos_base.json` intactos; la paridad golden de 17 productos sigue PASA.
- **Los productos exportados no cambian**: ni el Excel, ni el Word, ni las nóminas, ni los borradores. Toda la
  información nueva vive en la sesión (`trabajo.json`) y en la interfaz.
- Sin escritura en Outlook, RUS ni SATURNO. WP-96 solo **lee** `work.receipts`.
- No se fusionan «Preparar tipo» y «Preparar todos», y el ZIP de entrega, el resumen, el ICS y la recuperación
  legada se quedan donde están (decisión k).
- Sin datos reales en pruebas ni documentos; solo fixtures (`nurus.capture.fixtures.fixture`,
  `tests/test_personal_flow.exported`).
- Sin `--force`, sin `main` y sin releases.

**Fuera de la Fase A** (no implementar):
- retomar «Consultar y crear paquete»;
- índice del paquete;
- vista por tribunal o programa;
- versiones de Excel y Word;
- hoja operativa dentro del Excel (riesgo para «Actualizar desde Excel» y para golden);
- hoja de apoyo para RUS.

## 2. Qué existe y se reutiliza (verificado en `54a976c`)

| Necesidad | Ya existe | Dónde |
| --- | --- | --- |
| Emparejar filas entre capturas por identidad | `reconcile()`: `_key`, `matches`, `changed` por fila y `id_map` | `src/nurus/personal/capture_reconciliation.py:91` |
| Conservar ediciones humanas al reimportar | `EDIT_FIELDS` copiados al candidato | idem, `:9` y bloque `for field in EDIT_FIELDS` |
| Reimportar los mismos bytes con la misma configuración | Ruta de caché que devuelve `deepcopy(previous)` | `src/nurus/personal/capture_input.py:44-60` |
| Producto obsoleto tras un cambio de datos | `stale()` y `require_fresh()` | `src/nurus/personal/product_state.py:98-125` |
| Estado de correo y Word por fila | `_mail_state`, `_word_done` y `product_indicators` | `src/nurus/personal/ux_support.py:36-98` |
| Detalle de caso en Trabajo | `case_detail_lines` | `ux_support.py:101` |
| Filtro de la tabla de Trabajo | `work_filter` (valores) y `_apply_work_filter` (tags) | `work_view.py:196-206`; `app_base.py:455` |
| Filas, tags y valores de la tabla | `_show_work` | `app.py:371-470` |
| Columnas de Trabajo y vista guardada | `WORK_COLUMNS` y `columnas_conocidas` | `work_view.py:12-14,222-230`; `work_tools.py:170` |
| Fecha de captura y origen por fila | `work.capture_sources[row.id]` (`captured_at`, `query_id`, `source_system`) | `capture_input.py` |
| Vencimiento por correo | `due_value(work, row, kind)` | `outputs.py:349` |
| Conteos de «Siguiente paso» | `next_step.counts` | `next_step.py:20` |

## 3. Disciplina de ejecución (igual que WP-90 a WP-94)

1. Rama `wp/NN-tema` desde `origin/feat/ejecucion-plan-20261008`. Al empezar, la ficha y `estado/WP-NN.json` pasan
   a `TOMADO`.
2. Mientras se trabaja, solo pruebas focales: `PYTHONPATH=<ruta absoluta>/src`, y `xvfb-run -a` para la interfaz.
3. Commit limpio y luego una medición:
   `python tools/verificar_wp.py --linea-base --salida docs/implementacion/verificacion/WP-NN.json`.
4. `python tools/estado.py agregar && python tools/estado.py continuacion && python tools/estado.py validar &&
   python tools/check_links.py && python tools/check_real_ids.py`.
5. Mensajes de commit: «WP-NN: …», con una línea «Verificación: …» y los trailers del proyecto. Subir el commit de
   código sin `[skip ci]`; los commits de documentación van con `[skip ci]`.
6. Integrar por fast-forward en `feat`. Luego, estado `INTEGRADO` con `integrado_en` y la verificación.

## 4. Criterios de aceptación de la fase

- Reimportar el mismo paquete marca 0 cambios. Un paquete con 1 fila nueva, 1 egreso, 1 dato fuente distinto y
  1 resultado de reglas distinto se clasifica exactamente así.
- «Solo cambios» muestra únicamente esas filas, y las ediciones humanas se conservan.
- Cada fila tiene un estado de caso de la tabla de WP-96, y la bandeja ordena según la regla de WP-96.
- `verificar_wp.py` da PASA, sin bajas de pruebas, con golden intacto y productos exportados idénticos.
- Rendimiento con 3000 filas ficticias (`tools/benchmark_ef02.py` como base): la comparación tarda menos de 0,5 s y la
  bandeja menos de 1 s, sin releer adjuntos por fila.
- La métrica de WP-97 no contiene nombres, RUT, RIT ni textos, solo conteos y fechas.

## 5. Prompt para la IA ejecutora

```text
Eres programador del repositorio MatiGaete2023/flow (rama feat/ejecucion-plan-20261008). Español de Chile, directo.
Lee AGENTS.md, PLAN_VIGENTE.md, CONTINUAR.md y docs/implementacion/PLAN_FASE_A_20261013.md; después la ficha
del WP que tomes (WP-95, luego WP-96, luego WP-97). Implementa EXACTAMENTE lo que dice la ficha: archivos,
funciones, contratos de datos y pruebas. Si algo de la ficha no calza con el código, no improvises: detente,
documenta la diferencia en la ficha y continúa solo con lo que no dependa de ella. No toques motor/ ni textos_base.json,
no cambies productos exportados, no escribas en Outlook/RUS/SATURNO, no fusiones los botones de preparación,
sin datos reales, sin --force ni main. Sigue la disciplina de la sección 3 del plan y termina con un informe de
300 palabras como máximo: commits, cifras de verificar_wp, desviaciones y pendientes.
```
