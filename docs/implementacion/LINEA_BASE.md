# Línea base de la implementación — 9 de octubre de 2026

Medida con `python tools/verificar_wp.py --linea-base` sobre `2765b47` (etiqueta `pre-implementacion-20261009`),
Linux, Python 3.12.13 con Tk, `requirements/linux-py312.lock`, Xvfb. JSON completo: `linea_base.json`
(se regenera tras cada integración; este documento conserva la medición inicial).

| Control | Valor inicial | Esperado por el plan |
| --- | --- | --- |
| Suite Asistente | 605 aprobadas, 11 omitidas, 0 fallos | 605/11 |
| Suite Descargador | 198 aprobadas, 59 subpruebas, 0 fallos | 198/59 |
| `check_real_ids.py` | 18 hallazgos (6 en `tests/`, 12 en `variantes/`) | 18 |
| Paridad 17 productos golden | PASA | PASA |
| Restricciones absolutas | sin infracciones | sin infracciones |
| Ruff E9/F63/F7/F82 en `src/` | 0 | 0 |
| Módulos no alcanzables | 12 | 12 |
| Enlaces Markdown rotos | 26 | 26 |

Resultado: la línea base coincide con la del plan en los 8 controles.
