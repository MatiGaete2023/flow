# Continuación — plan NURUS / CSMP en flow

**Repositorio:** https://github.com/MatiGaete2023/flow
**Rama:** `feat/ejecucion-plan-20261008`
**PR:** https://github.com/MatiGaete2023/flow/pull/1 (borrador)
**Baseline NuRus:** `251d5c4025749edd4767183ebd3d2baf3bb9baed`
**P0:** `d74880bf7f1b3d53791e39c5245756848103bf0b`
**Documento 02 incorporado:** `6c520f8e49e40c6c7d5379492e6064a55b28c0fb`
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
| P13 | PREPARACIÓN/CI | Artefactos Windows y aceptación sin admin/Office; release/merge sin autorizar |

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

1. Leer implementación, trazabilidad y evidencia en `docs/evidencia_desarrollo_20261008`.
2. Comprobar resultados de workflows Windows del último SHA de la PR; atender
   fallos de empaquetado sin eliminar tests. Conservar mismo SHA para EXE/extensión.
3. Leer [aceptación pendiente](docs/ACEPTACION_INSTITUCIONAL_PENDIENTE.md).
   La pregunta sobre entorno Windows y ubicación de evidencia quedó planteada
   al usuario; usar su respuesta si existe, sin pedir credenciales ni publicar datos.
4. P9 no admite extractor especulativo. P10 CT-01..10 siguen desactivadas: el
   catálogo en `contracts/csmp_context_v1/` distingue implementación de preview
   y reglas aún pendientes. No activarlas por pasar fixtures.
5. Conservar ediciones, contactos, adjuntos retirados y productos obsoletos. No
   usar marcas locales/SQLite como última observación institucional.

La simulación antigua de escritura reside solo en `tests/test_registration.py`.
La API de producción `registration.submit` siempre rechaza. `rus_recorded` nunca
se marca por revisión local en Work. No hay envío de correos automático.
No hacer merge ni publicar releases sin instrucción correspondiente.
