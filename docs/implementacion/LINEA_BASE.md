# Línea base de la implementación — 9 de octubre de 2026

Medida en `2765b47`, etiqueta `pre-implementacion-20261009`, con Python 3.12.14,
Tk 9.0.4, dependencias fijadas y Xvfb. Comando: `python tools/verificar_wp.py --linea-base`.
La medición inicial permanece aquí; `linea_base.json` se actualiza tras cada integración.

| Control | Resultado |
| --- | --- |
| Asistente | 605 aprobadas, 11 omitidas, 0 fallos |
| Descargador | 198 aprobadas, 59 subpruebas, 0 fallos |
| Identidades detectadas | 18 (6 en pruebas, 12 en variantes) |
| Paridad de 17 productos golden | PASA |
| Restricciones absolutas | Sin infracciones |
| Ruff E9/F63/F7/F82 | 0 |
| Módulos no alcanzables | 12 |
| Enlaces rotos | 26 |

Los ocho controles reproducen la línea base del plan. Esto no acredita uso real.
