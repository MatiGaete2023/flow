# Continuación — plan NURUS / CSMP en flow

**Repositorio:** https://github.com/MatiGaete2023/flow
**Rama:** `feat/ejecucion-plan-20261008`
**PR:** https://github.com/MatiGaete2023/flow/pull/1 (borrador)
**Baseline NuRus:** `251d5c4025749edd4767183ebd3d2baf3bb9baed`
**P0:** `d74880bf7f1b3d53791e39c5245756848103bf0b`
**Documento 02 incorporado:** `6c520f8e49e40c6c7d5379492e6064a55b28c0fb`
**Código compilado y verificado Windows:** `99ed217c2fbb66d723b45f6e824622b07ae2f138`
El último checkpoint se obtiene con `git log -1`.

## Autorización y entrega actual

El usuario ordenó continuar hasta completar tareas y productos. Se avanzó desde
G0 con la matriz de 115 requisitos, sin pedir nueva aprobación de desarrollo.
Ver [implementación y límites](docs/IMPLEMENTACION_MVP_20261008.md).
Siete adjuntos originales y hashes conservados en `docs/plan_20261008/`.

Contrato, esquema/diccionario, fixtures, validador, exportador autónomo, importador,
UI local, cronología limitada, productos y solicitudes ya tienen implementación.
No presentar contrato como pendiente ni volver a solicitar documento 02.
No afirmar que todos los gates institucionales están cerrados.

## Fases

| Fase | Estado actual | Pendiente para aceptación completa |
|---|---|---|
| P0 | ENTREGADA; continuación autorizada por usuario | Evidencia histórica y golden conservados |
| P1 | DIAGNÓSTICO/TESTS LOCALES; aceptación real pendiente | Operador coteja SIN_VINCULO y cobertura con EXE/extensión mismo SHA |
| P2 | IMPLEMENTADA_LOCALMENTE | Schema/diccionario/fixtures/seguridad en `contracts/csmp_capture_v1` |
| P3 | IMPLEMENTADA_LOCALMENTE | Excel bitácoras autónomo; contraste institucional solo con G1 |
| P4 | IMPLEMENTADA_LOCALMENTE | Exportación desde lotes comprobados; no reconsulta |
| P5 | IMPLEMENTADA_LOCALMENTE | Importación validada, fecha corte, reimportación, Excel legado, GUI |
| P6 | IMPLEMENTADA_LOCALMENTE | Escritura RUS inhabilitada; sin consultas principales; recuperación local conservada |
| P7 | PARCIAL_CON_LÍMITES | Producción NO_EVALUABLE; cronología preview ficticia; requiere G1/aceptación funcional |
| P8 | IMPLEMENTADA_LOCALMENTE; aceptación Office pendiente | Editor/productos y huellas; Excel 2010/Outlook 365 reales |
| P9 | BLOQUEADA_POR_EVIDENCIA | Contrato remoto real de historia/PDF antes de desarrollar extractor |
| P10 | CATÁLOGO_DESACTIVADO; motor ampliado pendiente | Aprobación individual CT y G9 según fuente; baseline intacto |
| P11 | PRUEBAS_LOCALES; aceptación institucional pendiente | Matriz INT y límites; no confundir fixtures con RUS real |
| P12 | BLOQUEADA_POR_P9_P10 | Aceptación ampliada no sustituible por simulación |
| P13 | CI APROBADA; artefactos técnicos entregados | Aceptación Windows 10 institucional/Office y promoción diferidas |

## Descargas del código verificado

- [Paquete conjunto CSMP + Descargador + extensión + manuales + acta pendiente](https://github.com/MatiGaete2023/flow/actions/runs/37924605460/artifacts/11614235483).
- [Instalador del Asistente y evidencias ficticias de verificación](https://github.com/MatiGaete2023/flow/actions/runs/37924605460/artifacts/11613692954).
- [Descargador autónomo Windows](https://github.com/MatiGaete2023/flow/actions/runs/37924605484/artifacts/11613751543).

Ambos workflows del SHA `99ed217` terminaron SUCCESS el 2026-10-09.
Asistente Python 3.12/3.13/3.14: **612 aprobadas y 1 omitida** en cada versión.
Descargador: **171 aprobadas y 57 subpruebas aprobadas**. El EXE autónomo,
paquete conjunto e instalador comprobaron productos v1 con datos ficticios y SHA
coherente entre ambos EXE. Cada build ahora verifica instalación, reinstalación,
productos desde el EXE instalado, desaparición real de toda la carpeta y datos
externos intactos. La comprobación histórica manual se parametriza por run/SHA.

Metadatos y digests ZIP de GitHub, pasos y extractos ejecutados:
[CI de las correcciones](docs/evidencia_correcciones_20261009/ci_windows.json).
El registro canónico se actualizó en `docs/evidencia_desarrollo_20261008/ci_windows.json`;
la entrega anterior `6d3555a` se conserva en `ci_windows_codigo6d3555a.json`,
con su verificación histórica adicional `396a643`. No se afirma recomputación
local de los hashes ZIP: los binarios se comprueban en CI y los metadatos los
informa la API de GitHub. Los inventarios de dependencias y locks acompañan
los artefactos; son conjuntos de versiones probadas, no una promesa de identidad
binaria frente a timestamps u otros factores.

Los artefactos Actions caducan según la retención del repositorio. Los binarios
corresponden al código `99ed217`; el checkpoint documental posterior se obtiene
con `git log -1` y no cambia el código distribuido. La aceptación Windows 10
institucional/Office/SITFA/RUS sigue diferida por instrucción del usuario.

## Comandos reproducibles

```sh
python -m pip install --require-hashes -r requirements/linux-py312.lock
python -m pip install --no-deps --no-build-isolation .
python -m pytest -q
python -m pytest descargador -q
python tools/flow_review_probes.py --verify
python tools/flow_dependency_inventory.py --lock requirements/linux-py312.lock --output /ruta/nueva/dependencias.json
python tools/flow_capture_cli.py validate contracts/csmp_capture_v1/fixtures/completa.zip
python tools/flow_capture_pipeline.py /ruta/nueva/prueba --rows 100 500 2000
python tools/flow_baseline_golden.py --output /ruta/nueva/golden --expected docs/evidencia_p0/golden/semantic.json
```

En Linux, las pruebas GUI necesitan DISPLAY/pantalla virtual. Los tests Windows
omitidos no acreditan COM ni aceptación institucional. El inventario P0 sigue
contrastando archivos contra SHA original: detecta diferencias funcionales ahora
esperadas; conservar ese resultado como historial, no cambiar los hashes originales.

## Al retomar

**Revisión técnica corregida:** H01–H09 y M01–M03 están implementados. Leer
[informe de correcciones](docs/CORRECCIONES_REVISION_20261009.md) y
`docs/evidencia_correcciones_20261009/`. Se conservan el informe y las sondas
anteriores en `docs/evidencia_revision_20261009/` como diagnóstico histórico.
`python tools/flow_review_probes.py --verify` exige las ocho correcciones
reproducibles; H09 se comprueba con pruebas nativas y el instalador de cada build.

La conciliación preserva decisiones con capturas compatibles y exige decisión
humana ante cambios/ambigüedad. La cobertura se calcula por fuente/ingreso y se
mantienen las incidencias visibles. Los límites son 20.000 registros de análisis,
texto Excel en 32.767 unidades UTF-16 y solicitud de 4 MiB UTF-8/20.000 entradas.
El contrato de transporte conserva 200.000 filas/80 MiB. Excesos se rechazan antes
de sustituir el trabajo o publicar la solicitud; dividir el lote/alcance.

**Cierre de correcciones 2026-10-09:** H01–H09/M01–M03 implementados y verificados
con datos ficticios. Linux: 602 aprobadas/11 omitidas del Asistente, 171 y 57
subpruebas del Descargador; 17 productos conservan paridad. Windows del código
`99ed217` y el instalador actual aprobados. Los párrafos siguientes mantienen
el historial de las entregas anteriores y sus incidencias resueltas.

### Continuación de verificación y actas

Se amplió `--verificar-paquete` del Asistente: además del contrato comprueba
cuatro modalidades/tres modos, edición y lectura Excel externa, recuperación,
reimportación con exclusiones humanas, productos obsoletos, rechazo de corrupción,
Word/nóminas/solicitudes y descriptores de borrador sin Outlook. La prueba corre
dentro del EXE autónomo e instalador; CI exige `capture_pipeline_verified`.
Ambos EXE comprueban contrato vacío/completo/parcial con fixtures compartidas.

Reproducción local: `python tools/flow_verify_capture_products.py /ruta/nueva`.
`tools/flow_acceptance.py init/validate` crea actas estructuradas, separa evidencia
sintética/institucional y conserva resultados `NO_EJECUTADO` al preparar plantillas.
El paquete conjunto incluye una plantilla del SHA compilado. Consultar comandos,
campos y límites en el manual de aceptación pendiente. Validar no aprueba gates.

Regresión de esta continuación: 576 aprobadas/6 omitidas Asistente, 169 aprobadas
y 57 subtests Descargador; 17 productos golden conservan paridad. Logs con prefijo
`flow-continuacion-` en la evidencia. CI Windows del SHA corregido `6d3555a` aprobada; enlaces y metadatos actualizados
arriba. El checkpoint documental posterior no cambia los binarios.

El run Windows `37857365015` del SHA `1e50f95` detectó CP1252 en stderr del
validador de actas (580 pasaron, una falló, una omitida). Se corrige stdout/stderr
a UTF-8 y la prueba reproduce explícitamente CP1252, incluso en Linux. Conservar
el fallo como historial; no omitir esa prueba. Descargador `37857364791` SUCCESS.

Continuación 2026-10-09: `9cd9f50` corrigió UTF-8 (24 pruebas focalizadas).
Windows superó 581 pruebas/1 omitida, EXE autónomo y paquete conjunto; Descargador
`37875982173` SUCCESS. El run Asistente `37875982153` se quedó en la fase de
instalador; se solicitó cancelación para recuperar logs. No declarar esa fase
aprobada. Se añaden plazos por proceso, etiquetas de etapas y logs separados
con `tools/windows_process_check.ps1`; sus pruebas Windows comprueban código de
salida y terminación de un proceso ficticio colgado. Mantener el fallo y el
registro de cancelación, y cerrar la nueva ejecución antes de cambiar descargas.

`32daba5` superó 584 tests Windows/1 omitido del Asistente, incluidas las tres
pruebas nativas de procesos; el EXE autónomo también pasó. La suite Descargador
detectó el mismo problema Tcl en otras dos pruebas GUI. Se aíslan las seis
pruebas restantes mediante `descargador/gui_test_isolation.py`, conservando
aserciones y sin reintentos/omisiones. Seis casos pasaron en Linux/Xvfb forzando
la ruta de subprocess; esa prueba no sustituye la nueva CI Windows. Evidencia en
`ci_windows_32daba5_tcl.json` y `flow-gui-aislada.txt/xml`.

### Trabajo institucional diferido

1. Leer implementación, trazabilidad y evidencia en `docs/evidencia_desarrollo_20261008`.
2. Usar los artefactos aprobados del código `99ed217` y su verificación del
   instalador en el mismo build. Los checkpoints documentales no requieren recompilar.
   Si se modifica código, verificar su nueva CI sin eliminar tests y conservar
   mismo SHA para ambos EXE/extensión.
3. Leer [aceptación pendiente](docs/ACEPTACION_INSTITUCIONAL_PENDIENTE.md).
   El usuario confirmó que no hay entorno/evidencia disponibles y que SITFA/RUS
   se usarán después. La aceptación real queda diferida por esa decisión;
   no insistir en obtener acceso ahora ni pedir credenciales o publicar datos.
4. P9 no admite extractor especulativo. P10 CT-01..10 siguen desactivadas: el
   catálogo en `contracts/csmp_context_v1/` distingue implementación de preview
   y reglas aún pendientes. No activarlas por pasar fixtures.
5. Conservar ediciones, contactos, adjuntos retirados y productos obsoletos. No
   usar marcas locales/SQLite como última observación institucional.

La simulación antigua de escritura reside solo en `tests/test_registration.py`.
La API de producción `registration.submit` siempre rechaza. `rus_recorded` nunca
se marca por revisión local en Work. No hay envío de correos automático.
No hacer merge ni publicar releases sin instrucción correspondiente.
