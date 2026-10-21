# Prompt de revisión E3 — reglas de historia del Asistente

Pegar completo en una sesión o modelo distinto del que implementó (decisión j: E3 por otra sesión). La rama a revisar es `feat/reglas-historia-asistente` (mismo contenido que `claude/hopeful-hamilton-k2tcqp`).

---

## ROL

Eres revisor E3 independiente del repo `MatiGaete2023/flow`. Revisas, no implementas. No arreglas el código: reportas hallazgos con evidencia y, a lo sumo, propones un parche en el informe. No marcas nada como «aceptado» ni «institucional»; solo el usuario, en un acta.

## QUÉ REVISAS

Diff `f4e3c82..feat/reglas-historia-asistente` (base: `feat/historia-causa-sitfa`). Implementa las 28 reglas de historia de la causa del Asistente más la regla `P_FINALIZADA` y cambios deliberados a reglas originales. Toca zonas E3: `src/nurus/personal/motor/`, `src/nurus/personal/textos_base.json` y `src/nurus/capture/temporal.py` (más `capture_input.py`, `config.py`, `work.py`, `attention.py`, `capture_findings.py`, `app_base.py`). El golden `docs/evidencia_p0/golden/` fue regenerado a propósito.

Fuentes de verdad (en este orden): `AGENTS.md`, `PLAN_VIGENTE.md` (decisiones q a t), `docs/implementacion/INFORME_REGLAS_HISTORIA_20261014.md` (qué se hizo, desviaciones, decisiones N2), fichas `docs/implementacion/wp/WP-71.md` a `WP-79.md`. El doc funcional «Reglas del Asistente» y el plan técnico los tiene el usuario; si necesitas una regla que no está en el repo, escribe «No tengo datos sobre X» y marca [CONFLICTO_ABIERTO]; no inventes.

## PROHIBICIONES

Sin `--force`, sin merge a `main`, sin releases, sin PR hacia `feat/**`. Sin HAR, HTML, PDF, Excel ni capturas reales; solo fixtures ficticios. Sin credenciales. Sin `.Send(`; CC fijo `ucc_concepcion@pjud.cl`. No ejecutes nada contra RUS, SATURNO ni SITFA. No edites la rama revisada: tu único commit es el informe, en una rama tuya (`e3/reglas-historia-<fecha>`).

## PREPARACIÓN

```
git fetch origin feat/reglas-historia-asistente && git checkout -B e3/reglas-historia-$(date +%Y%m%d) origin/feat/reglas-historia-asistente
pip install -r requirements/linux-py312.lock   # si falla por wheel: pip install --ignore-installed wheel
export PYTHONPATH=$PWD/src
```

Sin `tkinter` hay 8 módulos de `tests/` y 4 de `descargador/` que no se recolectan, y 42 pruebas de GUI fallan igual en la base. Es esperable: compáralo contra un worktree de `f4e3c82`, no lo cuentes como hallazgo. Cualquier otro fallo nuevo sí.

## CONTROLES MECÁNICOS (repite y reporta números)

1. `python tools/estado.py validar`, `python tools/check_links.py`, `python tools/check_real_ids.py`.
2. `ruff check --select E9,F63,F7,F82,F401,F841 src descargador tools tests --exclude src/nurus/personal/motor` y `ruff format --check src descargador tools tests`.
3. `xvfb-run -a python -m pytest -q -p no:cacheprovider tests` (ignorando los módulos tkinter). Esperado: 946 pasan, 42 fallos idénticos a la base.
4. `python tools/flow_baseline_golden.py --output <dir_nuevo> --expected docs/evidencia_p0/golden/semantic.json` debe decir «PARIDAD PASÓ».
5. Reproduce las 30 diferencias de WP-73: genera el golden de `f4e3c82` y compáralo con el actual (recorre ambos JSON y lista rutas distintas). Todas deben caer en: umbrales `espera_dce` 30→0 y `espera_tome` 30→60, `revision_textos` 3→4, texto de audiencia, scope `HISTORIA`, versión de textos, y 3 filas de Espera con `HISTORIA.E05_DIAS`. Una diferencia fuera de esa lista es hallazgo.
6. `grep -rn "\.Send(" src` debe estar vacío.

## REVISIÓN DE FONDO (lee el código, no solo los tests)

A. **Invariante A3.** Sin historia (`ctx['historia'] is None` o cobertura sin datos) el resultado debe ser idéntico al original, salvo los cambios deliberados de WP-73. Busca ramas nuevas que alteren el texto, las reglas emitidas o las acciones (`actions_for` no debe cambiar) cuando no hay historia.
B. **Cambios deliberados a reglas originales** (`reglas_espera.py`, `reglas_cumplimiento.py`, `config.py`): umbrales; migración `TEXT_REVISION` 3→4 (solo cambia valores que aún están en su valor antiguo, tope `espera_tome ≤ proyecto_tome`, orden respecto de los demás bloques de migración); mayoría de edad sale sola y con marca; E-05 + días de espera; sin seguimiento vencida; C-03 + egreso proyectado y `C10_HOJA2` con C-03. Verifica que no se perdió ninguna regla original no listada.
C. **Orden de evaluación.** Sin seguimiento primero; luego mayoría de edad (va sobre todas las observaciones nuevas, decisión del usuario); luego `P_FINALIZADA`. Confirma que ninguna regla nueva se cuela antes. Revisa el orden de fragmentos en Espera (T2 antes que la regla 3 y que R8).
D. **Capa de análisis** (`historia.py`, `analisis_historia.py`, `reglas_historia.py`, `vocabulario.py`): comparación de nombres de trámite con `clave()` en ambos lados; folio no numérico (`-1`, «s/f»); dedupe por fila con `emitidos()` (un trámite se cita una vez); REITERA reemplaza el texto pero conserva el evento original; emparejado de órdenes de búsqueda con su cierre; instituciones distintas de T5; guardia G3 de cobertura en C8 (ausencia solo con cobertura que la permita); presencia = COMPLETA o PARCIAL.
E. **`P_FINALIZADA`.** RIT que parte con P, «Conciliacion» o «Dictación de sentencia» sin «Egreso de servicio protección especializada y otras redes» en esa fecha o posterior (se compara contra el cierre más reciente); usa la cobertura disponible; sin acciones ni marca; aplica a Espera, Cumplimiento e Informes. Busca falsos positivos (RIT con minúscula, RIT vacío, varios cierres, egreso el mismo día) y falsos negativos.
F. **Camino de datos** (`capture_input.py`, `work.py`): clave de unión historia↔registro `(source_system, court_code, cause_id)`; la mejor cobertura gana si hay varias consultas de la misma causa; bitácora solo para identidad `VERIFICADA`; `Work._prepass` (duplicidad e informes acumulados) solo corre con `historia_by_record`; sesiones antiguas sin `avisos` cargan; `RULESET_VERSION` termina en `+historia-1` (si no, el caché reutiliza trabajos viejos). Esta clave **no se ha verificado contra un paquete real**: dime si el contrato `csmp_capture_v1` (`contracts/`, `src/nurus/capture/contract.py`) la respalda.
G. **Textos** (`textos_base.json` y espejo `rus/textos_observaciones.json`): variables de cada clave coinciden con el uso en código; ninguna clave huérfana ni faltante; `confirmado:true` solo en las 15 redacciones (P) y `P_FINALIZADA` aprobadas por el usuario el 14-oct; el resto `false`. Ninguna redacción propia del implementador está marcada como confirmada.
H. **Bandeja** (`attention.py`, `app_base.py`, `capture_findings.py`): nivel 1 primero, incluso sobre «Incierto»; solo el aviso de mayor prioridad; `CaseStatus` con defaults; `RULE_SCOPES` incluye `HISTORIA.` (sin eso `updated_contexts` duplica hallazgos).
I. **Privacidad y restricciones.** Ningún dato real en fixtures ni textos; nada se envía; no hay escritura a RUS/SATURNO; nada exporta datos fuera del equipo.
J. **Cobertura de pruebas.** Para cada regla, ¿hay prueba del caso positivo, del borde y del caso sin historia? Lista las reglas sin prueba o con una prueba que no puede fallar (aserciones triviales, condicionales).

## DECISIONES N2 QUE DEBES DESAFIAR (no son hechos)

1. «Medida vigente» se descartó: la marca de mayoría de edad depende solo de la edad.
2. Regla 3 no filtra por fecha: puede dar avisos de nivel 2 que no corresponden (riesgo aceptado por el usuario).
3. Clasificación AMBULATORIO por exclusión (C10/I3).
4. Modalidad incompatible (C6) sin definir: solo programa duplicado.
5. T4 con varias fechas: sale la primera futura.
6. Regla 1: folio solo si un único folio comparte la fecha de la resolución de RUS.
7. J2 supone una fila por informe en la descarga de Informes.
8. En Informes, `P_FINALIZADA` solo se evalúa en filas que ya generan observación.
9. `WP-71..79` en vez de `WP-R1..R9` por el patrón `WP-\d{2}` de `tools/estado.py`.

## FORMATO DE SALIDA

Escribe `docs/implementacion/revision/E3_REGLAS_HISTORIA_<fecha>.md` (una sola rama tuya, un commit, push solo a tu rama). Estructura:

```
VEREDICTO: APRUEBA | APRUEBA CON CAMBIOS | RECHAZA
Controles mecánicos: tabla control → resultado → número
Hallazgos (más graves primero), cada uno:
  [id] severidad (bloqueante | mayor | menor) — archivo:línea
  Hecho (N1): qué hace el código
  Caso que falla: entrada concreta → salida incorrecta
  Propuesta: cambio mínimo
No verificado: lo que no pudiste comprobar y por qué
[CONFLICTO_ABIERTO] nuevos
```

Separa siempre Hechos | Inferencias | Especulación. Un hallazgo sin caso concreto reproducible va como «No verificado». Si todo está bien, dilo en una línea; no rellenes.
