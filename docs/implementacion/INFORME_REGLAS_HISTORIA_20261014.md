# Informe de implementación — reglas de historia del Asistente (14 de octubre de 2026)

Rama de trabajo: `claude/hopeful-hamilton-k2tcqp` (base `feat/historia-causa-sitfa` @ f4e3c82). Plan: «Flow — Plan de implementación técnica» (WP-R0..R9) y doc «Reglas del Asistente». Nada está aceptado; falta revisión E3 y aprobación del usuario.

## 1. Hechos (N1) — qué se hizo, por WP

| WP plan | ID estado | Commit | Contenido |
| --- | --- | --- | --- |
| R0 | — | — | Línea base: `estado.py validar` PASA; 49 pruebas focales pasan (+6 xfail); golden PARIDAD PASÓ |
| R1 | WP-71 | e6d35a0 | `motor/historia.py`; contexto (historia, bitácora, marcar); `capture_input` arma historia por causa y bitácora por ingreso; `Work.analyze` las inyecta; `Row.avisos`; `RULESET_VERSION` +historia-1 |
| R2 | WP-72 | 2ad22c1 | Scope `HISTORIA` (33 textos con P_FINALIZADA, `confirmado:false`), `_meta.version` v9.2.0, espejo RUS |
| R3 | WP-73 | 6025bb6, fae7032 | Umbrales `espera_dce=0`, `espera_tome=60`; `TEXT_REVISION=4` con migración; «Audiencia decretada»; mayoría de edad sola + marca nivel 3 (por edad); E-05 + días; sin seguimiento vencida; C-03 ampliado; golden regenerado (30 diferencias explicadas) |
| R4 | WP-74 | 2f86824 | `vocabulario.py`, `analisis_historia.py`, `reglas_historia.py` |
| R5 | WP-75 | fd1adff | Espera: reglas 1, 3, 5, 6, 7, 8, E9, E10, R9, R11, C3, T2, T3, T5; `Work._prepass` |
| R6 | WP-76 | 3f291ad | Cumplimiento: C1, C2, C-03 con folio, C4, C5, C6, C8, C9, C10 |
| R7 | WP-77 | a574a45 | Informes: I1, I2, I3, J2, I-01 contradicha |
| R8 | WP-78 | ab71972 | Bandeja: marcas de prioridad y columna Estado |
| Extra | WP-74..77 | b7069cc, 6824d40, 35a9fdb | Marca de mayoría de edad solo por edad; regla nueva `P_FINALIZADA` (ver 1b); golden regenerado (2 diferencias, solo config) |
| R9 | WP-79 | (commit de cierre) | Fichas, estado, `PLAN_VIGENTE.md`, nota en `REFERENCIA_CATALOGO_ASISTENTE.md`, este informe |

## 1b. Regla agregada el 14-oct por el usuario (`HISTORIA.P_FINALIZADA`)

- **Condición**: RIT que parte con P (sin distinguir mayúscula) y la historia trae «Conciliacion» o «Dictación de sentencia», sin «Egreso de servicio protección especializada y otras redes» en esa fecha o posterior (se compara contra el cierre más reciente, por fecha).
- **Observación**: solo «La causa se encuentra finalizada, se deja constancia que se remite proyecto de resolución que aplica nomenclatura «Egreso de servicio protección especializada y otras redes» a fin de regularizar informáticamente la causa.» Sin acciones ni marca. Aplica a Espera, Cumplimiento e Informes.
- **Decisiones N2 mías (corregibles)**: (a) precede a la mayoría de edad y a las demás reglas, salvo «derivación sin seguimiento», porque dijiste «la observación debe ser»; (b) la ausencia del egreso exige cobertura COMPLETA de la historia (guardia G3; con PARCIAL no se afirma ausencia); (c) en Informes solo se evalúa en filas que ya generan observación; (d) escribí «informáticamente» con tilde y comillas «» como en C10 (tu texto traía comillas “” y sin tilde); (e) texto `confirmado:false`.
- **[CONFLICTO_ABIERTO]**: no tengo el nombre exacto, en Desc. Trámite, del trámite de egreso; se usa el que indicaste. Si la columna trae otra forma (p. ej. abreviada como «Ing.Serv. …»), la regla no verá el egreso y disparará de más.

## 2. Cómo se hizo

- R1–R3: escritos a mano siguiendo el plan; archivos compartidos entre WP (`reglas_espera`, `reglas_cumplimiento`, `work`, `textos_base`) se editaron solo con los cambios del WP.
- R4–R8: se tomó el código del parche de referencia **archivo por archivo**, tras comparar cada diff contra el plan y el doc de reglas; no se aplicó entero. Las pruebas de cada WP se copiaron del mismo parche, salvo `test_cambios_originales.py`, que se escribió nuevo (el de referencia no cubría mayoría de edad, sin seguimiento, C-03 ni la migración con tope).
- Verificación por WP: pytest focal, `ruff check` (alcance CI) y `ruff format --check`. Suite completa y golden en R3, R5 y R9.
- Un commit por WP (R3 en dos: código y golden). Push tras cada WP a la rama de trabajo.

## 3. Resultados de verificación (N1)

- Suite del Asistente (excluye los 8 módulos que no recolectan sin `tkinter`): base 42 fallos / 843 pasan; final 42 fallos / 944 pasan. **Los 42 fallos son los mismos nodeids** (GUI/tkinter), comparados con un worktree de f4e3c82.
- Golden: PARIDAD PASÓ tras regenerar (R3). Diferencias de R3 = 30, todas en `config` (umbrales, revisión, audiencia, scope HISTORIA, versión) y 3 filas de Espera con `HISTORIA.E05_DIAS`.
- `ruff check` (alcance CI) y `ruff format --check`: limpios. `check_real_ids`, `check_links`, `estado.py validar`: PASAN.
- `verificar_wp.py`: PASAN identidades (0), golden, restricciones absolutas, ruff, módulos y enlaces; FALLAN las suites del Asistente (8 módulos) y del Descargador (4 módulos) **solo por `No module named 'tkinter'`** (no instalable aquí: `apt` sin candidato), y por consecuencia el inventario de pruebas. Pendiente repetir en un entorno con tkinter (Windows).

## 4. Desviaciones del plan (con motivo)

1. **Rama de trabajo**: el plan manda `feat/ejecucion-plan-20261008`, pero esa rama no contiene `capture/historias.py` (R0 dice detenerse). La sesión exige desarrollar en `claude/hopeful-hamilton-k2tcqp`; se basó en `feat/historia-causa-sitfa`, que sí lo contiene, y no se tocó ninguna rama `feat/**`.
2. **Golden en R2**: el plan esperaba paridad; el golden serializa la configuración, así que el scope `HISTORIA` y la versión lo cambian. Se regeneró en R3.
3. **`capture_findings.RULE_SCOPES`** se adelantó de R8 a R3: al emitirse `HISTORIA.E05_DIAS` fallaban 4 pruebas sin él.
4. **Identificadores de estado**: `tools/estado.py` exige `WP-\d{2}`; las fichas usan WP-71..WP-79 en vez de WP-R1..R9 (mapeo en cada ficha).
5. **`sincronizar_rus.py` no existe** en el repo; el espejo `rus/textos_observaciones.json` se mantuvo con el parche de referencia y las pruebas de igualdad pasan.
6. Se aplicó primero el parche entero en una copia de trabajo para inspeccionarlo y se revirtió (`git apply -R`) antes de implementar por WP.

## 5. Falta por hacer (requiere al usuario)

- **Revisión E3** por otra sesión o modelo (el diff toca `motor/`, `textos_base.json` y `capture/`). No se sustituye con revisión propia.
- **Aprobar redacciones (P)**: `R3_MULTI`, `MATERIA_CURADOR/SIN_FICHA/SIN_FAE/VENCIDA`, `C1_RUS`, `C3_CON_FOLIO`, `EGRESO_PROYECTADO`, `C4_FICHA_RUS/SIN_RUS/FOLIO`, `C8_NO_USO`, `I1_VENCIDO/POR_VENCER`, `T2_MN`. Todas `confirmado:false`.
- Corregido el 14-oct por indicación del usuario: la marca de mayoría de edad (nivel 3, `REVISAR_MAYORIA`) depende solo de la edad del NNA, sin relación con la vigencia de la medida; sale siempre que sale `MAYORIA_EDAD` (Espera y Cumplimiento).
- **Verificar la clave de unión historia↔registro** `(source_system, court_code, cause_id)` con un paquete real (extracto anónimo).
- Repetir `verificar_wp.py` y la suite completa en un entorno con tkinter; confirmar la CI Windows.
- Decidir si el trabajo pasa a `feat/ejecucion-plan-20261008` (decisión n) y cuándo.

## 6. [CONFLICTO_ABIERTO] heredados (no resueltos aquí)

C6 modalidad incompatible (solo programa duplicado); T4 con varias fechas (primera futura); nombre exacto de la resolución de ingreso (regla 1: folio solo si es único para esa fecha); clasificación AMBULATORIO por exclusión (C10/I3).

## 7. Inferencias / especulación

- N2: las 6 decisiones de WP-R4 y el orden de fragmentos de WP-R5 se tomaron del parche de referencia y del plan; las pruebas pasan, pero no hay paquete real que las contraste.
- N3: la regla 3 puede dar avisos de nivel 2 que no corresponden (sin filtro de fecha), riesgo aceptado en el doc de reglas.
