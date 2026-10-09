# Continuación — plan NURUS / CSMP en flow

**Repositorio:** https://github.com/MatiGaete2023/flow
**Rama:** `feat/ejecucion-plan-20261008`
**PR:** https://github.com/MatiGaete2023/flow/pull/1 (borrador)
**Baseline NuRus:** `251d5c4025749edd4767183ebd3d2baf3bb9baed`
**P0:** `d74880bf7f1b3d53791e39c5245756848103bf0b`
**Documento 02 incorporado:** `6c520f8e49e40c6c7d5379492e6064a55b28c0fb`
**Código compilado y verificado Windows (Descargador 2.7.5 y Asistente/conjunto):** `0795cda5c402672050e39c3806762caf99d82245`
El último checkpoint se obtiene con `git log -1`.

## ZIP bloqueado por bitacoras.json antiguo — 2.7.5

[Corrección e instrucciones](docs/CORRECCION_REFERENCIAS_ZIP_20261009.md).
El historial ocultaba referencias ausentes y la exportación seguía incluyéndolas.
Ahora se muestran como NO_DISPONIBLE; la revisión permite localizarlas o excluirlas
explícitamente antes de exportar. Conserva las fuentes válidas, las rutas recordadas
y los originales; Quitar lote del paquete aplica a todas las filas seleccionadas.
Descargador/extensión 2.7.5; seis regresiones, suite 198/59 y 36 pruebas de intercambio
aprobadas localmente; nueve pruebas finales de interfaz tras ajustar visibilidad.
Descargador Windows verificado sobre `0795cda`: CI 37979286940, 198/59, 80 lecturas
ficticias, PDF y EXE aprobados; artefacto 11639937234. Asistente/conjunto e instalador
verificados en CI 37979286900: 615/1 por Python 3.12/3.13/3.14, conjunto artefacto
11640457954. Su etiqueta histórica termina en SITFA-274 pero los binarios y JSON
de verificación corresponden a 2.7.5/SHA0795cda; las próximas ejecuciones usan
SITFA-275. Los artefactos anteriores e3a6369 son 2.7.4 y no
incluyen esta corrección. La captura del usuario
se analizó visualmente sin copiarla al repositorio; todas las pruebas son ficticias.

## Manual ilustrado solicitado por el usuario — 9 de octubre

[Manual del Descargador 2.7.4](docs/manual_descargador_2_7_4/README.md), con
PDF de 17 páginas, HTML autónomo con capturas ampliables y práctica ficticia
sin SITFA. Explica conexión, selección, hojas, lectura y recuperación de bitácoras,
Excel autónomo, combinación de lotes y el ZIP correcto para Trabajo → Importar paquete.
La práctica reproduce 800 ingresos, 4 registros de hojas y 2 bitácoras coincidentes
(798 ajenas excluidas). No incorpora adjuntos institucionales del usuario.
Generadores en `tools/manual_descargador_capturas.py` y
`tools/manual_descargador_publicar.py`; contenido editable `contenido.json`.
Entrega documental, sin cambios funcionales ni nueva compilación Windows.
Los ejecutables verificados siguen correspondiendo a `e3a6369`.

## Autorización y entrega actual

El usuario ordenó continuar hasta completar tareas y productos. Se avanzó desde
G0 con la matriz de 115 requisitos, sin pedir nueva aprobación de desarrollo.
Ver [implementación y límites](docs/IMPLEMENTACION_MVP_20261008.md).
Siete adjuntos originales y hashes conservados en `docs/plan_20261008/`.

Contrato, esquema/diccionario, fixtures, validador, exportador autónomo, importador,
UI local, cronología limitada, productos y solicitudes ya tienen implementación.
No presentar contrato como pendiente ni volver a solicitar documento 02.
No afirmar que todos los gates institucionales están cerrados.

## Regresiones del usuario — entrega 2.7.4 (9 de octubre)

[Informe y actualización desde ZIP](docs/REGRESIONES_PRUEBAS_USUARIO_20261009.md).
Código funcional `d6ec1b4`; `e3a6369` ajusta dos aserciones para rutas Windows y
HTML ficticio UTF-8. Asistente 0.5.0.dev4 y Descargador/extensión **2.7.4**.

- Guardar el lote de correos excluye las comprobaciones de Excel sin borrador:
  corregido `NoneType ... to`. Adjuntos automáticos; **Editar adjunto (opcional)**
  afecta solo al seleccionado. Se mantienen 24 adjuntos al editar uno de 25,
  ediciones al preparar nuevamente y recuperación de sesiones anteriores.
- Lectura y reutilización de HTML UTF-8/Windows-1252 conserva bytes/hashes. El HAR
  confirma los tipos 11/13 de Espera/Informes; se conserva cada tipo literal,
  además del 12 de Cumplimiento. Respuestas institucionales 11/13 pendientes.
- PDF vacío muestra tribunal, modalidad, pestaña y fechas o sin filtro; contexto
  visible en píxeles de captura Chrome nativa y cambios rechazados.
- **Resultados e historial** recupera descargas/conjuntos/bitácoras anteriores
  sin Chrome. Cada lectura produce **Bitacoras_RUS.xlsx**. Se comprobó un lote
  ficticio de 800 bitácoras, versiones/duplicados, hashes y exclusiones.
- Para reunir datos: añadir carpetas de hojas y `bitacoras.json` con sus HTML,
  pulsar **Crear paquete para Asistente (ZIP)** e importarlo en Trabajo.
  Solo reutiliza por tribunal/causa/ingreso verificados; informa coincidencias y
  exclusiones y conserva la fecha original. Quitar lote del paquete no borra
  sus archivos. Reutilizar no acredita actualización remota.

Los HAR/JSON/HTML reales permanecen fuera del repositorio. Las pruebas usan datos
ficticios. Instalación ZIP: extraer la rama en carpeta nueva, ejecutar de nuevo
`Instalar_CSMP.bat`, usar sus iniciadores y cargar/recargar **su** extensión 2.7.4.
No volver a pedir los atributos onclick: el HAR ya los confirmó para Espera e
Informes. Queda contraste de las respuestas y aceptación Office/institucional.

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

- [Paquete conjunto CSMP + Descargador + extensión + manuales + acta pendiente](https://github.com/MatiGaete2023/flow/actions/runs/37942421534/artifacts/11622653641).
- [Instalador del Asistente y evidencias ficticias de verificación](https://github.com/MatiGaete2023/flow/actions/runs/37942421534/artifacts/11622223523).
- [Descargador autónomo Windows](https://github.com/MatiGaete2023/flow/actions/runs/37942421581/artifacts/11623120729).

Workflows del SHA `e3a6369`: [Asistente](https://github.com/MatiGaete2023/flow/actions/runs/37942421534)
y [Descargador](https://github.com/MatiGaete2023/flow/actions/runs/37942421581).
Ambos SUCCESS el 2026-10-09. Asistente Python 3.12/3.13/3.14:
**615 aprobadas y 1 omitida** en cada versión. Descargador:
**192 aprobadas y 59 subpruebas aprobadas**. EXE autónomos, paquete conjunto e
instalador verifican productos v1 ficticios y SHA coherente. Instalación,
reinstalación, desaparición real de carpeta y conservación de datos externos
comprobadas. Esto no acredita Outlook ni SITFA reales.

Evidencia local y CI:
[regresiones del usuario](docs/evidencia_regresiones_usuario_20261009/README.md)
y [metadatos/digests Windows](docs/evidencia_regresiones_usuario_20261009/ci_windows.json).
Linux/Xvfb: **605 aprobadas/11 omitidas** Asistente, **192/59 subpruebas**
Descargador; paridad de 17 productos tradicionales. El registro canónico en
`docs/evidencia_desarrollo_20261008/ci_windows.json` corresponde a `e3a6369`;
las entregas anteriores `99ed217` y `6d3555a` se conservan en archivos
`ci_windows_codigo99ed217.json` y `ci_windows_codigo6d3555a.json`.
[Correcciones H01–H09/M01–M03](docs/evidencia_correcciones_20261009/ci_windows.json)
conserva su evidencia histórica de `99ed217`.

Los digests ZIP los informa GitHub, no se afirma su recomputación local. Locks e
inventarios acompañan los paquetes. Los artefactos caducan según retención de
Actions. Los binarios corresponden a `e3a6369`; el checkpoint documental posterior
se obtiene con `git log -1`. La aceptación Windows 10 institucional, Office,
SITFA/RUS y promoción sigue pendiente. No fusionar a main ni publicar release
sin instrucción; continuar en esta rama y PR en borrador.

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
