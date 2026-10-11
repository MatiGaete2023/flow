# Historial de continuación anterior

Archivado el 9 de octubre de 2026 desde `2765b47`.

# Continuación — plan NURUS / CSMP en flow

**Repositorio:** https://github.com/MatiGaete2023/flow
**Rama:** `feat/ejecucion-plan-20261008`
**PR:** https://github.com/MatiGaete2023/flow/pull/1 (borrador)
**Baseline NuRus:** `251d5c4025749edd4767183ebd3d2baf3bb9baed`
**P0:** `d74880bf7f1b3d53791e39c5245756848103bf0b`
**Documento 02 incorporado:** `6c520f8e49e40c6c7d5379492e6064a55b28c0fb`
**Código compilado y verificado Windows (Descargador 2.7.5 y Asistente/conjunto):** `0795cda5c402672050e39c3806762caf99d82245`
El último checkpoint se obtiene con `git log -1`.

[Resumen completo del proyecto](../RESUMEN_PROYECTO_NURUS_CSMP_20261009.md): fundamentos,
documentos, planificación, hallazgos, correcciones, implementación, pruebas y pasos pendientes.

## ZIP bloqueado por bitacoras.json antiguo — 2.7.5

[Corrección e instrucciones](CORRECCION_REFERENCIAS_ZIP_20261009.md).
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

[Manual del Descargador 2.7.4](manual_descargador_2_7_4/README.md), con
PDF de 17 páginas, HTML autónomo con capturas ampliables y práctica ficticia
sin SITFA. Explica conexión, selección, hojas, lectura y recuperación de bitácoras,
Excel autónomo, combinación de lotes y el ZIP correcto para Trabajo → Importar paquete.
La práctica reproduce 800 ingresos, 4 registros de hojas y 2 bitácoras coincidentes
(798 ajenas excluidas). No incorpora adjuntos institucionales del usuario.
Generadores en `tools/manual_descargador_capturas.py` y
`tools/manual_descargador_publicar.py`; contenido editable `contenido.json`.
Ese checkpoint fue documental, sin cambios funcionales ni nueva compilación Windows:
sus ejecutables eran `e3a6369` (2.7.4). Posteriormente se compiló y verificó
`0795cda` (2.7.5); el manual conserva sus imágenes y se complementa con la guía del ZIP.

## Autorización y entrega actual

El usuario ordenó continuar hasta completar tareas y productos. Se avanzó desde
G0 con la matriz de 115 requisitos, sin pedir nueva aprobación de desarrollo.
Ver [implementación y límites](IMPLEMENTACION_MVP_20261008.md).
Siete adjuntos originales y hashes conservados en `docs/plan_20261008/`.

Contrato, esquema/diccionario, fixtures, validador, exportador autónomo, importador,
UI local, cronología limitada, productos y solicitudes ya tienen implementación.
No presentar contrato como pendiente ni volver a solicitar documento 02.
No afirmar que todos los gates institucionales están cerrados.

## Regresiones del usuario — entrega 2.7.4 (9 de octubre)

[Informe y actualización desde ZIP](REGRESIONES_PRUEBAS_USUARIO_20261009.md).
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
ficticios. Instrucciones históricas 2.7.4 (para actualizar ahora, usar 2.7.5):
extraer la rama en carpeta nueva, ejecutar de nuevo
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

- [Paquete conjunto actual 2.7.5](https://github.com/MatiGaete2023/flow/actions/runs/37979286900/artifacts/11640457954).
- [Instalador actual del Asistente](https://github.com/MatiGaete2023/flow/actions/runs/37979286900/artifacts/11640877263).
- [Descargador autónomo Windows 2.7.5](https://github.com/MatiGaete2023/flow/actions/runs/37979286940/artifacts/11639937234).

Código `0795cda5c402672050e39c3806762caf99d82245`, verificado en CI 37979286900
y 37979286940. Asistente: 615 aprobadas/1 omitida por Python 3.12–3.14;
Descargador: 198 aprobadas/59 subpruebas. EXE, conjunto e instalador comprobados.
El nombre del conjunto conserva SITFA-274, con versión interna 2.7.5/SHA0795cda.
Detalle en el [resumen](../RESUMEN_PROYECTO_NURUS_CSMP_20261009.md) y la
[corrección 2.7.5](CORRECCION_REFERENCIAS_ZIP_20261009.md).

### Descargas históricas 2.7.4 — reemplazadas por 2.7.5

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
[regresiones del usuario](evidencia_regresiones_usuario_20261009/README.md)
y [metadatos/digests Windows](evidencia_regresiones_usuario_20261009/ci_windows.json).
Linux/Xvfb: **605 aprobadas/11 omitidas** Asistente, **192/59 subpruebas**
Descargador; paridad de 17 productos tradicionales. El registro canónico en
`docs/evidencia_desarrollo_20261008/ci_windows.json` corresponde a `e3a6369`;
las entregas anteriores `99ed217` y `6d3555a` se conservan en archivos
`ci_windows_codigo99ed217.json` y `ci_windows_codigo6d3555a.json`.
[Correcciones H01–H09/M01–M03](evidencia_correcciones_20261009/ci_windows.json)
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
[informe de correcciones](CORRECCIONES_REVISION_20261009.md) y
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
2. Usar los artefactos aprobados del código `0795cda` (2.7.5) y su verificación del
   instalador en el mismo build. Los checkpoints documentales no requieren recompilar.
   Si se modifica código, verificar su nueva CI sin eliminar tests y conservar
   mismo SHA para ambos EXE/extensión.
3. Leer [aceptación pendiente](ACEPTACION_INSTITUCIONAL_PENDIENTE.md).
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


## Traspasos de implementación anteriores a la revisión del 11 de octubre de 2026

Referencia histórica; el estado vigente está en [CONTINUAR.md](../CONTINUAR.md).
Los pasos pendientes de estos traspasos pueden estar completados.

# Continuar

**Estado:** recuperación del plan del 9 de octubre de 2026 desde `2765b47`.
**Rama base:** `feat/ejecucion-plan-20261008`. Cada WP usa una rama propia;
solo el Orquestador integra después de verificación, revisión y auditoría.

## Estado al cierre del 11 de octubre de 2026 (léelo primero)

**El historial se reescribió (WP-17).** Todo clon, worktree o rama local anterior al 11 de octubre de 2026 tiene el
historial viejo con datos reales: vuelve a clonar y nunca hagas push desde una copia antigua.

- **Integrado en `feat`** (excepción a E3 autorizada por el usuario): #27, Codex #4, #8 y #11, #12–#32, más WP-61
  (formato). `ESTADO.json` es una vista generada desde `docs/implementacion/estado/` (`tools/estado.py`, PASA). La
  línea base se regeneró: asistente 592/11/6 xfail, descargador 231/62, identidades 0, enlaces rotos 0.
- **Decisiones:** D1 (purga hecha), D3–D9 respondidas, D2 «Ninguna todavía».
- **Pendientes del usuario:**
  1. Borrar las etiquetas `pre-WP-12` y `pre-implementacion-20261009` (conservan el historial viejo).
  2. Pedir a GitHub la eliminación de datos sensibles en las referencias de PR.
  3. ~~Borrar los artefactos de Actions previos~~: hecho (607 borrados por la API, 11 de octubre de 2026).
  4. Billing: GitHub Pro y presupuesto de Actions de 5 USD (11 de octubre de 2026); CI verde en `14f679d`.
- **Lo que falta del plan:**
  - WP-02 (D2: casos probados en uso real).
  - WP-50: implementado en `wp/50-cronologia-solo-mostrar` (PR #33 borrador, CI verde, `verificar_wp` PASA). Se integra
    tras el acta de 10 ingresos del usuario y WP-02.
  - WP-70: cierre. Paquete compilado en CI (`14f679d`, run 38136720831); esperar el acta final y, con D7, sacar el PR #1
    de borrador y fusionarlo a `main`, sin release.
- **Conflictos respondidos** (11 de octubre de 2026): `cc_obligatorio` se conserva; motor/ queda intacto (4 F401
  excluidos en CI; los 6 R-T de WP-60 siguen xfail estricto hasta una decisión por regla).

## Traspaso del 10 de octubre de 2026 (sesión claude-sesion-012yjVgL)

**Léelo en este orden:** `PLAN_VIGENTE.md` → `AGENTS.md` → este traspaso → `docs/implementacion/REPORTE_REVISION_20261010.md`
(hallazgos, pasos y enmiendas) → la ficha `docs/implementacion/wp/WP-XX.md` del PR que toques.

### Qué se hizo y cómo

Todo el trabajo ejecutable del plan está hecho y abierto como PR borrador. Cada WP tiene su rama `wp/<ID>-<slug>`, su ficha
en `docs/implementacion/wp/` y su verificación en `docs/implementacion/verificacion/`. Esta sesión **no editó**
`ESTADO.json` en las ramas de WP (propuesta E1); el estado propuesto de cada uno está en su ficha.

| PR | WP / contenido | Base |
| --- | --- | --- |
| #12–#25 | WP-20, 21, 22, 23, 30, 31, 32, 35, 24, 34, 16, 40, 41, 42 | pila: cada una sobre la anterior, la primera sobre `feat` |
| #26 | WP-33 ritmo y checkpoint de lectura | `feat` |
| #27 | plan: reporte, `tools/estado.py`, verificador sin holgura, enmiendas E1–E6 aprobadas, D5–D9 registradas | `feat` |
| #28 | WP-43 paquete del lote actual | `feat` |
| #29 | WP-62 dividir `DiaryRunner.run` | #26 |
| #30 | WP-44 importar último paquete | #25 |
| #31 | WP-45 retirar solicitud de antecedentes (D9) | #30 |
| #32 | WP-51 consolidado en el ZIP (D6) | `feat`, sobre #28 |

De Codex: #4 (WP-14, aprobado y conforme), #8 (WP-15), #11 (WP-60). No los modifiqué.

### Decisiones del usuario (respuesta literal en `ESTADO.json`, rama #27)

- D3 retirar inspector y diario RUS · D4 60 días y 10 sesiones · D8 sin fallos · D6 «Sí, solo el consolidado» · D9 «Retirar
  botón» · D5 «Sí, horizonte de 4 meses» · D7 «Sí, tras el acta final».
- D1 (11 de octubre de 2026): «Reescribir historial». WP-17 se ejecuta solo después de integrar WP-10, WP-12 y WP-20, y
  el push forzado necesita además la autorización escrita del usuario en ese momento; NuRus está fuera del alcance de esta sesión.
- **Pendiente:** D2 (lista de casos probados en uso real; el usuario no marcó ninguno).
- **Revisión:** el usuario pidió que esta sesión revise con subagentes; por E3 es apoyo, no aprobación.

### Qué falta (en este orden)

1. **CI caída.** Desde las 03:24 UTC del 10 de octubre Actions no asigna runners (los jobs fallan en 1–2 s, `runner_id` 0,
   sin logs). Causa probable, no verificada: facturación o minutos. El usuario debe revisar *Settings › Billing › Actions*.
   Hasta entonces la única verificación es la local. Si la CI vuelve y falla con logs, corregir en la rama del PR y
   propagar hacia arriba en la pila con merges (nunca rebase ni `--force`).
2. **Integración (solo el Orquestador, que no es el Ejecutor de esos WP).** Orden: #27 primero; luego `python tools/estado.py
   dividir`; reemplazar `TODAS_LAS_DEMAS` en WP-61 y WP-70 por listas, y marcar como `archivo` WP-33→14, WP-43→33 y
   WP-44→43; `agregar` y `validar`; después #12–#25, #26, #28–#32. Regenerar `linea_base.json` tras cada integración.
   Revisor y auditor deben ser de otra raíz de agente (E3): nadie ha revisado los PR de esta sesión.
3. **Conflictos de texto previstos:** `work.py` entre WP-32 (#18) y WP-51 (#32); `ESTADO.json` entre las ramas que lo
   editan y #27.
4. **WP-02** necesita D2. **WP-50** depende de WP-02 y exige el acta de 10 ingresos contrastados por el usuario.
   **WP-17** necesita D1. **WP-61** (formato) va al final y excluye `src/nurus/personal/motor/`. **WP-70** es el cierre.
5. **Fuera de ficha, a decidir por el Orquestador:** un botón o casilla en `descargador/descargador.py` y
   `ventana_resultados.py` para pasar `consolidated=True` (WP-51 no tiene entrada en la GUI); el manual 2.7.4 aún menciona
   el botón retirado en WP-45.
6. **Sin probar:** la salida nativa con Excel de WP-51 (solo Windows, PC autorizado). La ruta portable sí tiene pruebas.

### Integración de prueba (11 de octubre de 2026)

CI: tras la caída se relanzó cada PR una vez. #12–#30 quedaron en verde; #29 tenía un error real (WP-62 importaba `nurus`
antes de `parser()`), corregido en `c65f9fb`.

Rama `integracion/prueba-20261011` (no es `feat`; nadie la integra): `feat` + #27 + Codex #4, #8 y #11 + cima de la pila
(`wp/WP-45`, que trae #12–#25, #30 y #31) + `wp/WP-62` (#26 y #29) + `wp/WP-51` (#28 y #32). Resultado:

- Conflictos textuales solo en `descargador/descargador.py` (ritmo de WP-33 y comando `package` de WP-43: se conservan
  ambos) y `src/nurus/personal/work.py` (base consolidada de WP-51 dentro del `try` de WP-32, que borra la copia si falla).
- Un choque semántico: WP-34 quitó `from pathlib import Path` de `descargador/export_csmp_capture.py` (F401) y WP-51 lo usa.
  Se restituye al integrar.
- `verificar_wp.py --pruebas-eliminadas-asistente 148`: PASA (Asistente 569/11 y 6 xfail de WP-60; Descargador 227/62;
  identidades 0; módulos no alcanzables 0; enlaces rotos 0). Los 4 F401 del motor quedan excluidos del lint, como en la CI.
- El Orquestador puede usar esa rama como guía de resolución.

### Revisión con subagentes (11 de octubre de 2026; por E3 es apoyo, no aprobación)

Cinco revisores aislados (anonimizador, retiros, interfaz, Descargador y contrato, CI y herramientas). Corregido, cada
arreglo con prueba que falla sin él, y propagado hacia arriba en la pila con merges:

- **WP-16:** nueve filtraciones del anonimizador (RUT con guion largo o espacios o sin dígito, teléfonos 3-3-3, enteros
  de 7 a 9 dígitos, encabezados `CUR. AD LITEM`/`R.U.N.`/`TEL.`/`N° RIT`/`RUN NNA`, teléfonos en columnas del motor,
  curador institucional con abogado, RIT con barra). `_scan` revisa además números, RIT y teléfonos.
- **WP-40:** Ctrl+K borraba texto del campo antes de abrir la paleta; los atajos actuaban con el diálogo de fecha abierto.
- **WP-42:** la paleta mostraba «Revisar y guardar borradores» disponible con el botón deshabilitado.
- **WP-33:** la pausa no separaba una consulta de la siguiente. **WP-62:** al retomar, el manifiesto perdía lo ya
  leído hasta el cierre.
- **WP-51:** el consolidado admitía vínculos externos, conexiones y macros declaradas en `[Content_Types].xml`; el mapeo
  podía fallar en vez de caer a portable.
- **WP-31:** `--sincronizar-extension` ignoraba `--descargador`. **WP-35:** sin `requirements/` se borraba el `.venv` en
  cada arranque.
- **#27:** `verificar_wp.py` daba PASA cuando `check_real_ids`, ruff o pytest no pudieron medir; `--lista-eliminadas`
  aceptaba nodos inventados; `estado.py` sin fuentes daba traceback y trataba como error un conflicto abierto.

**No corregido, decide el usuario o el Orquestador:**

- `exports._annotation_values` deja un apóstrofo visible en `NURUS_PROPUESTA`/`NURUS_OBSERVACION_FINAL` de la copia
  portable; la ficha de WP-32 dice «sin apóstrofo», pero una prueba de la línea base
  (`test_preserved_export_uses_archived_bytes_and_marks_excluded`) exige `'=texto de prueba`. Es una decisión de diseño
  (defensa contra fórmulas).
- `tools/flow_baseline_matrix.py` y `flow_plan_progress.py` reescriben `docs/` aun con `--help` y devuelven enlaces a
  rutas retiradas: no ejecutarlos hasta corregirlos.
- Restos sin uso tras WP-21 (`operational_templates.json`, `resolution_templates.json`, clases `Issue`/`Evaluation` en
  `rus/models.py`), manual 2.7.4 con el botón de WP-45, `os.startfile` no vigilado por `verificar_wp.py`.
- Sin casilla en la GUI para `consolidated=True` (WP-51); búsqueda del último paquete en el hilo de la interfaz (WP-44).

### Conflictos abiertos que decide el usuario (detalle en el reporte, §4)

`cc_obligatorio` contra la prueba golden · 4 avisos F401 en el motor · la ficha de WP-23 contra el retiro de
`download_link.start` · fórmulas de OBSERVACIÓN copiadas como texto · 6 casos R-T de WP-60 · lock de entorno para la
instalación sin red.

### Cómo verificar y trabajar

```sh
python -m venv .venv && . .venv/bin/activate     # o usar el existente
python -m pip install --require-hashes -r requirements/linux-py312.lock
python -m pip install --no-deps --no-build-isolation -e .
xvfb-run -a python tools/verificar_wp.py --salida docs/implementacion/verificacion/WP-XX.json
```

- Desde WP-23 hay que agregar `--pruebas-eliminadas-asistente 148` (pruebas retiradas acumuladas) en las ramas de la pila.
  En #27 la verificación sin holgura usa `--lista-eliminadas` con nodeids.
- Las ramas de la pila se verifican con la cabeza de la pila (`wp/WP-42-…`): 512/11 Asistente y 198 Descargador. #26: 625/11
  y 203/59. #28: 625/11 y 200/59. #29: 625/11 y 203/59. #30: 514/11. #31: 515/11 y 198/59. #32: 637/11 y 204/59. #27: 630/11.
- Pruebas de interfaz en Linux: `xvfb-run`, con stubs de `pywinauto`, `win32gui` y `window_capture` para
  `tools/smoke_personal_gui.py`; la prueba de layout corre `_layout_probe` a 1024×650 y 1180×820 (el editor debe medir ≥ 120 px).
- Un worktree por WP evita pisar ramas: `git worktree add -b wp/<ID>-<slug> <ruta> origin/<base>`.
- El proxy de esta sesión no puede subir etiquetas (`pre-<ID>` las crea el Orquestador).

### Reglas que no se negocian (resumen; el texto vigente está en `AGENTS.md`)

Sin escritura automática en RUS ni SATURNO · Outlook solo borradores, CC fijo `ucc_concepcion@pjud.cl` · sin credenciales ni
datos reales (solo estructura anónima) · reglas, umbrales y textos tradicionales intactos · el Asistente no navega SITFA ni
inicia el Descargador · nada «aceptado» o «institucional» salvo acta del usuario · sin `--force`, sin fusionar a `main` ni
releases sin autorización escrita · cada WP toca solo los archivos de su ficha.

## Traspaso de la sesión claude-sesion-012yjVgL (10 de octubre de 2026)

**Qué se hizo:**
- Ejecuté como Ejecutor una pila de PR en borrador. Cada PR apunta a la rama del anterior porque todos editan
  `ESTADO.json`.
- Orden de integración (lo hace solo el Orquestador, con squash):

  | PR | WP | Rama o tema |
  | --- | --- | --- |
  | #12 | WP-20 | `ccr-4116db41-nm8g0r` |
  | #13 | WP-21 | — |
  | #14 | WP-22 | — |
  | #15 | WP-23 | — |
  | #16 | WP-30 | — |
  | #17 | WP-31 | — |
  | #18 | WP-32 | — |
  | #19 | WP-35 | — |
  | #20 | WP-24 | — |
  | #21 | WP-34 | CI |
  | #22 | WP-16 | anonimizador |
  | #23 | WP-40 | — |
  | #24 | WP-41 | — |
  | #25 | WP-42 | — |

**Cómo:**
- Una rama `wp/<ID>-<slug>` por WP.
- `python tools/verificar_wp.py --pruebas-eliminadas-asistente 143`: PASA en cada WP. Las 143 pruebas retiradas
  corresponden a WP-21, WP-22 y WP-23.
- Ficha en `docs/implementacion/wp/WP-XX.md` y verificación en `docs/implementacion/verificacion/WP-XX.json`.
- Todos los WP quedan en `EN_VERIFICACION`: no revisé ni audité mis propios WP.

**Para qué:** ejecutar las olas 2 a 4 del plan sin tocar el motor ni los textos tradicionales.

**Revisión del 10 de octubre (misma sesión).**
- Cinco revisores independientes encontraron errores; todos quedaron corregidos en sus ramas.
- Nuevos PR independientes o declarados:

  | PR | Contenido | Base |
  | --- | --- | --- |
  | #26 | WP-33 | `feat` |
  | #27 | plan, reporte y herramientas | `feat` |
  | #28 | WP-43 | `feat` |
  | #29 | WP-62 | sobre #26 |
  | #30 | WP-44 | sobre #25 |

- Hallazgos, pasos y enmiendas E1–E6:
  [REPORTE_REVISION_20261010.md](implementacion/REPORTE_REVISION_20261010.md) (rama
  `plan/revision-20261010`).
- Desde las 03:24 UTC, Actions no asigna runners: revisar *Settings › Billing › Actions*.

**Falta:**
- Revisión, auditoría e integración de #12 a #25, en ese orden.
- WP-33, WP-43, WP-44 y WP-62: esperan WP-14 (Codex, PR #4).
- Decisiones del usuario: WP-45 (D9), WP-50 (D5), WP-51 (D6), WP-02 (D2), WP-17 (D1) y WP-70 (D7).
- WP-60 y WP-15: Codex.
- WP-61: al final.
- `conflictos_abiertos` de ESTADO: `cc_obligatorio` frente a golden y 4 F401 en el motor.
- Probar el anonimizador con un `.xls` real en el PC autorizado.

## Enlaces

- [Plan vigente](../PLAN_VIGENTE.md) · [Instrucciones](../AGENTS.md)
- [Plan de implementación](implementacion/PLAN_IMPLEMENTACION.md)
- [Recuperación](implementacion/wp/WP-01.md)
- [Historial anterior](HISTORIAL.md)

## Comandos

```sh
python -m pip install --require-hashes -r requirements/linux-py312.lock
python -m pip install --no-deps --no-build-isolation -e .
python tools/verificar_wp.py
```

## Migración de contactos antes de actualizar

Copia tu `contactos_base.json` de la instalación anterior a
`%LOCALAPPDATA%\CSMP_Personal\contactos.json` antes de reemplazarla.
Si no existe esa copia, el primer inicio abre «Importar contactos» para elegir
el JSON anterior o el catastro Excel; mientras tanto se usa el ejemplo ficticio.

## Control de identidades

Activa el control antes de cada commit con:

```sh
git config core.hooksPath tools/hooks
```

El hook revisa el contenido del índice, incluidos los XML de Excel y Word;
no lee las modificaciones que aún no agregaste. Ambos workflows ejecutan el
control antes de instalar dependencias o producir artefactos, sobre todo el
árbol: WP-20 retiró `variantes/` y las rutas personales (recuperables desde `pre-WP-20`).
