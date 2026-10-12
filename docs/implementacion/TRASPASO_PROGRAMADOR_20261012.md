# Traspaso al siguiente programador

12 de octubre de 2026. Repositorio: [MatiGaete2023/flow](https://github.com/MatiGaete2023/flow). Rama de trabajo consolidada: `feat/ejecucion-plan-20261008`.

Este informe describe el tramo WP-89 y contextualiza lo heredado. Referencia al comenzar este traspaso: `42df402`; añadir este documento no cambia el código comprobado. Los estados actuales se consultan en los JSON, no se deducen de notas históricas que dicen «pendiente».

## 1. Dónde retomé el trabajo

Retomé en `2bc277f`, después de la integración del PR #42/WP-88. El usuario informó que había actualizado la facturación de Actions y pidió continuar con un plan de CI más eficiente.

| Trabajo heredado | Situación al retomarlo |
|---|---|
| WP-87 / PR #41 | Tema compartido y adaptadores de apariencia integrados. |
| WP-88 / PR #42 | Autoexportación Excel tras importar, consulta→paquete, preferencia XLSX común, retiro del campo firmas, intermedios en `respaldo/` y lanzador con dos modos integrados. También cuatro etapas, siguiente acción, progreso y controles visibles. |
| Validación local WP-88 | PASA; 873 pruebas Asistente y 246 Descargador. |
| Windows/instalador WP-88 | Pendientes: Actions no iniciaba jobs por facturación. |
| EF-01 | Propuesta de CI eficiente, generación de continuación y reutilización segura de mediciones aún sin completar. |

Las consolidaciones previas, G-5 y la purga de ramas pertenecen al trabajo anterior. Su antecedente está en [CONTINUAR](../../CONTINUAR.md). Persistía la limpieza externa de referencias/cachés de GitHub mediante Soporte.

## 2. Qué hice, cómo y por qué

Trabajé en `wp/89-ci-eficiente`, desde la rama consolidada, y lo integré mediante [PR #43](https://github.com/MatiGaete2023/flow/pull/43), merge `321413e`. El código integrado coincide con el candidato local medido `dc45582`; después solo se actualizaron documentos y estados.

| Cambio | Implementación | Razón |
|---|---|---|
| CI automática Linux | Nuevo `.github/workflows/ci-rapida.yml`: privacidad, estado y enlaces antes de instalar. Cambios de código añaden locks con hashes, ambas suites, Node, lint/formato y Xvfb. | Detectar problemas baratos antes de gastar en Windows. |
| Windows mínimo | `tests.yml` y `descargador-integral.yml`: Python 3.12 en PR de código hacia `feat/**`, con filtros de rutas. | Mantener control nativo durante desarrollo. |
| Entrega completa bajo demanda | Eliminé los triggers push Windows. Matriz 3.12–3.14, navegador y empaquetado se ejecutan por `workflow_dispatch` sobre el candidato vigente. | Evitar construir una entrega por cada integración o cambio documental. |
| Construcción única | `tests.yml` llama al `tools/build_integral.ps1` existente. Artefactos individuales e instalador usan los EXE de `dist/integral/`. El workflow Descargador conserva sus pruebas y navegador, sin otro PyInstaller. | Había compilaciones duplicadas de los mismos ejecutables. El script de construcción no se modificó. |
| Límites de recursos | Timeouts por job, caché pip; integral/instalador pasan de 90 a 30 días de retención, diagnósticos siguen en 14. | Limitar jobs colgados y almacenamiento acumulado. |
| Plan y contratos | Actualicé E4, plan vigente, ola de eficiencia, fichas/estado y pruebas de workflows. | Alinear instrucciones y ejecución real; conservar los nodeids de pruebas. |

La CI permitió confirmar que la facturación funciona: los jobs iniciaron y ejecutaron pasos. E3 independiente no aplica a este diff según la decisión j; se hizo revisión propia, sin atribuirla a otra raíz.

### Corrección de producto encontrada por Windows

La primera prueba Windows detectó un editor de correos de **110px** en una ventana de **1024×650**, frente al mínimo existente de **120px**. El aviso persistente repetía el estado en una segunda fila.

Corregí `src/nurus/personal/app_base.py`, función `_show_result_notice`: conserva el mensaje, pero oculta su fila cuando está vacío o ya aparece en el estado. Durante otra operación, un resultado anterior distinto sigue visible. Añadí cobertura en `tests/test_flujo_ux_integrado.py`; la prueba original de geometría en `tests/test_audit_gui_20260928.py` conserva su mínimo.

Cancelé la ejecución completa del candidato defectuoso. Desde entonces el plan exige esperar Windows mínimo aprobado antes de construir la entrega. Linux también detectó un enlace roto al renombrar el informe histórico; se corrigió antes de la medición final. El primer informe se conserva, y una medición interrumpida no se presenta como aprobada.

## 3. Qué está comprobado

| Evidencia | Resultado / procedencia |
|---|---|
| [Verificación local final](verificacion/WP-89.json) y [línea base](linea_base.json) | PASA sobre `dc45582c5cc655d3b0cc4db404404e8c5129abec`, limpio al inicio/final. Asistente: 874 aprobadas, 11 omitidas, 6 xfail; Descargador: 246 aprobadas y 62 subpruebas. Son cifras Linux. |
| Restricciones y regresión | Golden 17 intacto; sin bajas de pruebas, identidades detectadas, infracciones, errores de lint ni enlaces rotos. |
| [Windows mínimo corregido](https://github.com/MatiGaete2023/flow/actions/runs/38258410423) | PASA; incluye la geometría que había fallado. |
| [Windows y entrega completa](https://github.com/MatiGaete2023/flow/actions/runs/38259239456) | PASA en `c896b0f`: Python 3.12/3.13/3.14, ambos EXE y contratos, instalación/reinstalación/desinstalación. Código equivalente al medido localmente. |
| [Descargador y navegador](https://github.com/MatiGaete2023/flow/actions/runs/38257631987) | PASA en `280a785`. Se conservó esa evidencia porque Descargador, tema común, contratos, dependencias y workflow específico no cambiaron después. |
| [Linux tras integración](https://github.com/MatiGaete2023/flow/actions/runs/38261053010) | PASA en `321413e`; la integración no inició Windows. El PR permanente feat→main omitió el duplicado Linux. |

El run Windows `38259239456` acumuló **25,50 minutos de duración de sus tres jobs**; no es el total de todas las ejecuciones de esta tarea. No equivale a dinero facturado ni demuestra un porcentaje de ahorro. La descarga local de diagnósticos fue denegada por la red (403): se comprobó el resultado de los pasos y sus guardas, sin inspeccionar los JSON descargados. Los artefactos están en el run de entrega.

**Esto acredita validación automática con datos sintéticos.** Excel/Outlook/SITFA reales, usabilidad a escala Windows 150 % y aceptación del usuario siguen pendientes. No hubo merge a `main` ni release.

## 4. Qué leer y dónde revisar el código

Leer en este orden:

1. [AGENTS.md](../../AGENTS.md) y [PLAN_VIGENTE.md](../../PLAN_VIGENTE.md): restricciones y decisiones actuales.
2. [ESTADO.json](ESTADO.json): vista agregada. Fuentes editables: `estado/global.json` y `estado/WP-XX.json`; regenerar con `python tools/estado.py agregar`, validar con `validar`.
3. [PLAN_IMPLEMENTACION.md](PLAN_IMPLEMENTACION.md): reglas, E3 por riesgo y E4 revisada; después, la ficha del WP elegido.
4. [Plan de eficiencia/UX](PLAN_EFICIENCIA_UX_20261012.md): diferencia entre fases implementadas y propuestas pendientes.
5. [Ficha WP-89](wp/WP-89.md), [estado WP-89](estado/WP-89.json), informe y línea base: historial de incidencias y evidencias actuales.

Para revisar productos: `src/nurus/ui_theme.py`, `personal/next_step.py`, `personal/app_base.py`; `descargador/descargador.py`, `flujo_paquete.py`, `preferencias.py`, `ventana_resultados.py`; `tools/csmp_launcher.py`, `build_integral.ps1`, `verify_installer.ps1` e `installer/CSMP.iss`. Los archivos `personal/` están bajo `src/nurus/`.

Revisar también los manuales MVP en `docs/MANUAL_ASISTENTE_MVP_20261008.md` y `docs/MANUAL_DESCARGADOR_MVP_20261008.md`. Los informes `EFICIENCIA_*_20261012.md` son propuestas históricas; contrastarlos con el plan de la ola y los estados antes de implementar otra vez algo ya integrado.

## 5. Qué falta y cómo continuar

| Orden | Trabajo / responsable | Criterio para avanzar |
|---|---|---|
| 1 | Programador: resto EF-01 | Crear ficha nueva desde feat; generar continuación desde estado. Diseñar caché de mediciones con commit, árbol limpio, entorno/locks, herramientas, plataforma, alcance e inventario. SHA solo no basta; conservar controles que fallan ante evidencia ausente. |
| 2 | Programador: EF-02 | Benchmark sintético de edición/exportación antes de optimizar; conservar paridad y dependencias de cálculo. |
| 3 | Usuario + programador: WP-02 y WP-50 | Pruebas reales Excel/Outlook/SITFA, diez ingresos de cronología y resultados del acta. WP-50 ya tiene código integrado y visible: su bloqueo es de aceptación. |
| 4 | Titular de cuenta/Soporte GitHub: WP-17 | Resolver refs de PR y cachés externas; dejar evidencia del saneamiento. No repetir force ni importar referencias anteriores a la purga. |
| 5 | Optimización posterior: EF-03/04 | Outlook/retención requieren E3; preservar persistencia antes/después de cada efecto y `uncertain`. Reutilización SITFA requiere evidencia del puesto, identidad/alcance/fecha/hashes y pausas intactas. |
| 6 | Usuario + Orquestador: WP-70 | WP-02, WP-17, WP-50 y WP-63 satisfechos y acta final; recién entonces tramitar PR #1 hacia main y publicación autorizada. |

La plantilla `docs/aceptacion/ACTA_USO_REAL_20261012.json` está ligada a `5a8f68e`. Para validar la entrega actual, usar el acta generada dentro del integral de `c896b0f` o generar una nueva para el artefacto elegido; conservar la anterior. No cambiar su SHA a mano ni trasladar resultados a otra versión. Solo el usuario completa resultados de uso real; nunca subir datos personales.

## 6. Comandos y disciplina de trabajo

Partir de un clon de la rama saneada:

```sh
git clone --no-tags --single-branch --branch feat/ejecucion-plan-20261008 https://github.com/MatiGaete2023/flow.git flow
cd flow
git status --short
python tools/estado.py validar
python tools/check_real_ids.py
python tools/check_links.py
```

Para desarrollar en Linux: Python 3.12, venv y `requirements/linux-py312.lock` con `pip install --require-hashes`; instalar el proyecto con `--no-deps --no-build-isolation`. En Windows usar el lock de su versión. [Instrucciones de dependencias](../../requirements/README.md). Las pruebas GUI Linux necesitan Xvfb. Usar `PYTHONPATH` absoluto al `src` del worktree: el entorno editable de otra copia puede cargar código viejo.

Pruebas focales mientras se modifica; antes de medir, verificar imports, geometría, enlaces y diff. Congelar el candidato con commit limpio. Para un WP nuevo, una medición completa también actualiza la línea base:

```sh
python tools/verificar_wp.py --linea-base --salida docs/implementacion/verificacion/WP-XX.json
```

Sustituir WP-XX por la ficha real; no editar código o documentos durante la medición. No enlazar un informe inexistente antes de generarlo. Repetir el conjunto completo solo por cambios, fallos o una preocupación pendiente.

Tras Windows mínimo aprobado, construir una entrega por ola sobre la rama candidata:

```sh
gh workflow run tests.yml --ref feat/ejecucion-plan-20261008
# Ejecutar también cuando cambia el alcance respecto de una evidencia válida:
gh workflow run descargador-integral.yml --ref feat/ejecucion-plan-20261008
```

Si falla facturación, confirmar la causa antes de reintentar. No hacer vigilancia periódica ni repetir builds por informes/documentación. Conservar el SHA de cada artefacto y revisar expiración de 30 días antes de preparar aceptación.

## 7. Límites que debe preservar el siguiente programador

Motor y `textos_base.json` intactos; Outlook `Save()` exclusivamente y CC fijo `ucc_concepcion@pjud.cl`; ninguna escritura RUS/SATURNO ni datos reales en repo/CI. El Asistente no navega SITFA ni inicia Descargador: la elección está en el lanzador anterior a crear App. Retención local: 60 días y hasta 10 sesiones anteriores.

Conservar separados «Preparar tipo» y «Preparar todos»; mantener ubicación de ZIP de entrega, resumen, ICS y recuperación legado. No borrar/omitir pruebas para esconder fallos, ni convertir seis xfail estrictos en aceptación de reglas. E3 independiente solo en los alcances riesgosos enumerados por decisión j. Ninguna prueba automática sustituye el acta ni autoriza main/release.
