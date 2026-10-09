# Continuación — plan NURUS / CSMP en flow

**Repositorio:** https://github.com/MatiGaete2023/flow
**Rama:** `feat/ejecucion-plan-20261008`
**PR:** https://github.com/MatiGaete2023/flow/pull/1 (borrador)
**Baseline NuRus:** `251d5c4025749edd4767183ebd3d2baf3bb9baed`
**P0:** `d74880bf7f1b3d53791e39c5245756848103bf0b`
**Documento 02 incorporado:** `6c520f8e49e40c6c7d5379492e6064a55b28c0fb`
**Código compilado y verificado Windows:** `6d3555a14fe4b6966f5091dc6a7db51a1e54e521`
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

- [Paquete conjunto CSMP + Descargador + extensión + manuales + acta pendiente](https://github.com/MatiGaete2023/flow/actions/runs/37878632095/artifacts/11593867357).
- [Instalador del Asistente y evidencias ficticias de verificación](https://github.com/MatiGaete2023/flow/actions/runs/37878632095/artifacts/11594002109).
- [Descargador autónomo Windows](https://github.com/MatiGaete2023/flow/actions/runs/37878632105/artifacts/11594120109).

Ambos workflows del SHA `6d3555a` finalizaron SUCCESS el 2026-10-09.
Asistente Python 3.12/3.13/3.14 aprobados (3.12: 584 aprobadas, 1 omitida).
Descargador: 169 aprobadas y 57 subtests. El EXE autónomo, el paquete conjunto y
el instalador comprobaron el flujo de productos v1 con datos ficticios. Se verificó
SHA coherente entre ambos EXE y conservación de datos al reinstalar/desinstalar.

La verificación adicional [396a643, SUCCESS](https://github.com/MatiGaete2023/flow/actions/runs/37880150341)
reutilizó ese instalador: instalación, productos desde el EXE instalado,
reinstalación, desaparición de la carpeta completa y conservación de un dato
ficticio externo. Espera la eliminación real hasta 60 s, porque el retorno del
lanzador de Inno puede anteceder a su proceso temporal. No cambia el binario
`6d3555a`. [Logs y JSON de entrega](https://github.com/MatiGaete2023/flow/actions/runs/37880150341/artifacts/11593803911);
resumen en `ci_windows_396a643_instalador.json`.

Metadatos y digests ZIP informados por GitHub: `docs/evidencia_desarrollo_20261008/ci_windows.json`.
La descarga local de los ZIP devolvió HTTP 403; no se afirma recomputación local
de sus hashes. Las comprobaciones de binarios fueron ejecutadas en CI. El
paquete conjunto conserva sus JSON de verificación y SHA de ambos EXE.
El registro anterior está en `ci_windows_codigo01e013d.json`; el fallo CP1252 y
la corrección se conservan en `ci_windows_1e50f95_fallo.json`.

Los artefactos Actions caducan según la retención del repositorio. Un checkpoint
posterior de documentación no cambia el código distribuido: estos binarios
corresponden a `6d3555a`, no al SHA de ese checkpoint.

## Comandos reproducibles

```sh
python -m pip install ".[dev,excel-legacy]"
python -m pytest -q
python -m pytest descargador -q
python tools/flow_capture_cli.py validate contracts/csmp_capture_v1/fixtures/completa.zip
python tools/flow_capture_pipeline.py /ruta/nueva/prueba --rows 100 500 2000
python tools/flow_baseline_golden.py --output /ruta/nueva/golden --expected docs/evidencia_p0/golden/semantic.json
```

En Linux, las pruebas GUI necesitan DISPLAY/pantalla virtual. Los tests Windows
omitidos no acreditan COM ni aceptación institucional. El inventario P0 sigue
contrastando archivos contra SHA original: detecta diferencias funcionales ahora
esperadas; conservar ese resultado como historial, no cambiar los hashes originales.

## Al retomar

**Cierre 2026-10-09:** CI del SHA `6d3555a` aprobada en ambos workflows. Los límites
por proceso, aislamiento GUI Windows y diagnóstico del instalador están verificados.
El run adicional `37880150341` confirma la eliminación real de la aplicación y
datos externos intactos usando el instalador publicado de ese mismo código.
Los párrafos siguientes conservan el historial de incidencias ya resueltas.


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
2. Usar los artefactos aprobados del código `6d3555a` y la verificación adicional
   del instalador `396a643`. Los checkpoints documentales no requieren recompilar.
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
