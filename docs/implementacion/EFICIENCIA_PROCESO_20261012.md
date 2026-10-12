# Eficiencia del proceso de agentes (12 de octubre de 2026)

> **Propuesta histórica.** La numeración WP-87/WP-88 que aparece aquí fue sustituida; el alcance real de cada WP
> está en [PLAN_EFICIENCIA_UX_20261012.md](PLAN_EFICIENCIA_UX_20261012.md) y en `estado/*.json`.

Respuesta a «¿hay una forma más eficiente que la que estás usando?». **Sí.** Más de la mitad del costo (tiempo,
tokens y minutos de Actions) se va en repetir trabajo, no en el cambio en sí.

Niveles: **N1** = medido en esta sesión; **N2** = inferencia con el supuesto indicado; **N3** = especulación.

## 1. Dónde se pierde hoy

| # | Desperdicio | Evidencia | Nivel |
| --- | --- | --- | --- |
| P1 | Las suites se corren 3 veces por ciclo: pytest manual, `verificar_wp.py` y `--linea-base` | Consolidación del 12 de octubre: asistente ~125 s y descargador ~30 s por corrida, en 3 corridas ≈ 8 min en vez de ≈ 2,5 | N1 |
| P2 | Corridas manuales que fallan por falta de pantalla y se repiten | El descargador falló sin `xvfb`; `verificar_wp.py` ya usa `xvfb-run` solo (línea 50), así que la corrida manual sobraba | N1 |
| P3 | Un mismo dato (cifras de pruebas, SHA, estado del acta) se escribe en 5–6 archivos | `estado/*.json` (43), `wp/*.md` (40), `ESTADO.json`, `CIERRE.md`, `CONTINUAR.md`, `PLAN_VIGENTE.md` y `global.json`. La revisión integral encontró 14 desajustes (D1–D14), casi todos por esta causa | N1 |
| P4 | Una rama, un PR, una revisión E3 en otra sesión y una espera de CI por cada WP | 45 ramas que hubo que consolidar; revisión E3 de WP-86 con 11 commits propios | N1 |
| P5 | Auditorías en abanico (5–6 subagentes leyendo todo el repositorio) | WP-80 y la revisión integral: 6 y 5 auditorías completas, más su reverificación | N1 |
| P6 | Monitoreo de PR/CI con despertares periódicos | Desactivado por pedido del usuario el 12 de octubre | N1 |
| P7 | CI en Windows en cada push o PR (PyInstaller dos veces) | Presupuesto de Actions de 5 USD; `tests.yml` tiene 7 referencias a Windows | N1 (costo exacto: sin datos) |
| P8 | Clon aparte en el scratchpad en vez de trabajar en el clon de la sesión | Se necesitó por la purga de historial; hoy el remoto ya está limpio | N2 (supuesto: no quedan etiquetas antiguas en el clon de la sesión) |

## 2. Qué cambiar (por impacto/costo)

1. **Una sola medición por ciclo (P1, P2).** Commit, después `python tools/verificar_wp.py` y nada más. Mientras se
   trabaja, solo `pytest <archivo tocado>`.
   - Cambio de herramienta: que `--linea-base` reutilice la medición de la corrida PASA del mismo árbol (por
     ejemplo, caché `.verificacion/<sha>.json`) en vez de volver a correr.
   - Ahorro: ≈ 5 min por ciclo (N2, supuesto: mismas duraciones).
2. **Fuente única de estado (P3).** Las cifras y el SHA vigentes viven solo en `linea_base.json` y `estado/*.json`.
   - `CIERRE.md` y `CONTINUAR.md` se generan con `tools/estado.py` (como ya pasa con `ESTADO.json`) o citan el
     archivo sin copiar números.
   - Elimina la clase de defecto D2, D4 y D6 de la revisión integral (N2).
3. **Integración por ola, no por WP (P4).** Una rama por ola, commits `WP-XX:` dentro de ella y un solo PR por ola.
   - E3 por muestreo: obligatoria solo si el diff toca `motor/`, `textos_base.json`, Outlook, el contrato de captura
     o el borrado de datos; para el resto basta `verificar_wp.py` PASA.
   - Requiere una decisión del usuario, porque cambia la regla E3 del plan.
4. **Auditorías acotadas al diff (P5).** `git diff <base>..HEAD --stat` decide qué revisar.
   - Un solo revisor para diffs de menos de 500 líneas; abanico solo para la revisión de cierre.
   - Ahorro estimado de tokens del 50–70 % en revisiones (N2, supuesto: diffs típicos de WP de 200–800 líneas).
5. **CI barata (P7).**
   - En pushes a `feat`: solo Linux (pytest, ruff, `verificar_wp.py`).
   - Windows y PyInstaller solo en `workflow_dispatch`, en el PR a `main` o en una etiqueta de entrega.
   - Seguir usando `[skip ci]` en commits solo de documentación.
6. **Entorno listo al iniciar (P8).** Un hook `SessionStart` que cree `.venv` e instale dependencias.
   - Se trabaja directamente en la rama designada del clon de la sesión, sin clon aparte.
7. **Sin monitoreo pasivo (P6).** Revisar la CI una sola vez al final, o solo cuando el usuario lo pida.

## 3. Lo que no conviene recortar

- `verificar_wp.py` completo antes de cada push a `feat`: detectó regresiones reales (WP-84 caché, migración F7).
- La paridad golden y `check_real_ids.py`: son las restricciones absolutas (motor intacto y sin datos reales).
- Las actas firmadas por el usuario: no son automatizables por definición.

## 4. Propuesta concreta para el próximo ciclo

| Paso | Acción | Quién |
| --- | --- | --- |
| 1 | Decidir si E3 pasa a ser por muestreo (punto 3) | Usuario |
| 2 | WP-87: caché de medición en `verificar_wp.py`; `CIERRE.md` y `CONTINUAR.md` generados; CI Windows solo bajo demanda | Agente |
| 3 | Hook `SessionStart` con `.venv` | Agente |

N3: con los puntos 1–3 aplicados, un ciclo WP típico bajaría de ~4 commits, 3 corridas de suites y 2 sesiones a
~2 commits, 1 corrida y 1 sesión. No hay medición previa que lo respalde.
