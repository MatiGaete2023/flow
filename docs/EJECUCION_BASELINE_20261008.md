# Ejecución P0 — 8 de octubre de 2026

**Destino:** [MatiGaete2023/flow](https://github.com/MatiGaete2023/flow).
**Estado:** `IMPLEMENTADA_PENDIENTE_ACEPTACION`; G0 aún no aprobado.
**Origen:** NuRus `251d5c4025749edd4767183ebd3d2baf3bb9baed`, rama
`feat/consolidacion-bitacoras-20261008`.
**Commit de importación en flow:** `bd826d524761f24204323e7ad87c287045dc40d3`.
**Commit P0:** `d74880bf7f1b3d53791e39c5245756848103bf0b`.
**PR de revisión G0:** https://github.com/MatiGaete2023/flow/pull/1.

## Resultado concreto

Se importaron los 674 archivos versionados del SHA exigido a una rama nueva
de flow, junto a los cinco adjuntos íntegros. No se cambiaron funciones de usuario,
versiones, reglas, plantillas, extensión o workflows. Se añadieron tres herramientas
P0 para verificar integridad, inventariar módulos/tests y generar/comparar productos
ficticios. La portada se adaptó a flow; el README de origen permanece íntegro en
`docs/README_NURUS_BASELINE.md`.

Flow tenía solo README y un commit inicial. Para mantener una PR revisable
contra su main sin sustituir el historial, el baseline se incorporó como snapshot
desde ese commit inicial, con procedencia por SHA y hashes de todos los archivos.
No se creó tag ni se mutó NuRus; no se fusionó a main de ninguno de los repositorios.

## P0 por tarea

| Tarea | Entregable y resultado | Límite |
|---|---|---|
| P0.01 | SHA, versiones, dos CI origen exitosos, diff main/experimental, hashes 674 archivos | CI origen no es CI de flow ni aceptación institucional |
| P0.02 | Suites completas locales, logs iniciales/finales, XML, dependencias y omisiones | Linux Python 3.12.14; no Office ni Windows local |
| P0.03 | [Inventario Descargador](INVENTARIO_DESCARGADOR_20261008.md) | Catálogo real y variantes remotas pendientes |
| P0.04 | [121 módulos y grafo](evidencia_p0/inventario/modulos.md), 562 definiciones de tests | Imports calculados/subprocess requieren inspección manual |
| P0.05 | [17 archivos golden](evidencia_p0/golden/hashes.json), comparación semántica independiente y showcase GUI ficticio | No certifica Office 2010/Outlook ni reglas contextuales nuevas |
| P0.06 | [Mapa de migración](MIGRACION_Y_MODULOS_20261008.md), configuración y sesiones legado preservadas | No se recibió configuración personal ni se migraron datos del usuario |
| P0.07 | Inspector/extensión de captura y variantes preservados fuera del nuevo flujo | No eliminar ni fusionar por nombre |

La [matriz funcional](MATRIZ_FUNCIONAL_20261008.md) registra los 95 IDs D-T01..20,
R-T01..32, INT-01..27 y F00..15 presentes en los documentos accesibles. Es una
matriz técnica inicial: las pruebas relacionadas no sustituyen el caso de
aceptación completo. La especificación 02 anunciada por el usuario aún no aparece
en los adjuntos accesibles; sus A-T01..20 quedan pendientes de incorporar.

## Ramas y CI de procedencia

`main` NuRus = `4fe0cab3c20ac7507f9503c15a18df72f7f5e657`;
experimental = `fba9ffea79ef1040a7e161270398ebeaafeaf72f`.
El baseline está 11 commits delante y 0 detrás de main; frente al experimental,
47 delante y 11 detrás. Se conserva el diff en `evidencia_p0/diff_main.txt`
y `diff_experimental.txt`; no se fusionó la rama experimental.

Se consultaron los runs [37826176379](https://github.com/MatiGaete2023/NuRus/actions/runs/37826176379)
y [37826176465](https://github.com/MatiGaete2023/NuRus/actions/runs/37826176465).
Ambos corresponden exactamente a `251d5c4`, estado `completed/success`.
Los metadatos mínimos verificables están en `evidencia_p0/ci_origen_*.json`.

## Pruebas propias ejecutadas

| Comprobación | Resultado observado |
|---|---|
| Asistente, primera corrida sin DISPLAY | 498 pasaron, 4 fallaron por falta de pantalla, 7 omitidas |
| Descargador, primera corrida sin DISPLAY | 160 pasaron, 7 fallaron por falta de pantalla; 57 subpruebas pasaron |
| Asistente con Xvfb, suite completa | **503 pasaron, 6 omitidas**, 33.13 s |
| Descargador con Xvfb, suite completa | **167 pasaron, 57 subpruebas pasaron**, 23.16 s |
| Extensión Node | Pasó: GET de lectura, identidad, modalidad, pestaña, duplicados; sin guardado RUS |
| Compileall src/tests/tools y pip check | Pasaron |
| Golden, dos directorios independientes | Paridad pasó: valores/tipos/estilos/filtros/observaciones/reglas/acciones/nóminas/Word/borradores |
| Showcase existente con GUI real en pantalla virtual | Pasó; Excel/Word/ZIP y capturas Claro/Oscuro sintéticas |
| `compatibilidad_csmp.py` | Pasó: 9 casos formato/modo; HTML/XML_2003 no se abren directamente en CSMP, OOXML sí; conversión local probada |
| `integracion_exportador_csmp.py` | Pasó: Espera/Cumplimiento, procedencia y cruce C10 mediante Excel legado; sin red/Office |

Los fallos iniciales se conservan y se explican; no se suprimió ningún test.
Las seis omisiones finales requieren GUI/contratos Windows o cmd.exe. Se
detallan por nombre en [resultados_suites.json](evidencia_p0/resultados_suites.json).
No se ejecutaron RUS autenticado, Chrome ficticio/Playwright, COM de Office,
instalador Windows ni nuevas distribuciones. `smoke_personal_gui.py` es específico
de Windows; en Linux se verificó GUI con la suite integral y el showcase.

Los golden contienen tres filas ficticias por modo, fecha de corte fija
`2026-10-08`, configuración default completa, originales preservados, nóminas,
Word Laja/Tomé donde aplica y bitácora ficticia parcial/fallida/sin vínculo.
Los correos son descripciones de borrador; no se invocó Outlook ni envío.
IDs y hashes sintéticos no son identidades remotas reales verificadas.

## Reproducción y herramientas nuevas

```powershell
python -m venv .venv-csmp
.venv-csmp/Scripts/python -m pip install ".[dev,excel-legacy,excel-native,outlook]"
.venv-csmp/Scripts/python -m pip install -r descargador/requirements.txt
.venv-csmp/Scripts/python tools/flow_baseline_inventory.py
.venv-csmp/Scripts/python tools/flow_baseline_matrix.py
.venv-csmp/Scripts/python tools/flow_baseline_golden.py --output artifacts/golden-nueva --expected docs/evidencia_p0/golden/semantic.json
.venv-csmp/Scripts/python -m pytest -q -o faulthandler_timeout=90
.venv-csmp/Scripts/python -m pytest descargador -q
node descargador/test_bitacoras_extension.cjs
```

En Linux, activar una pantalla Xvfb disponible y anteponer `DISPLAY=:<numero>`
a las suites/showcase; el Python es `.venv-csmp/bin/python`. El binario Xvfb
se extrajo localmente de un paquete Debian sin instalar como administrador;
no es dependencia productiva ni se publica en flow. Los logs son evidencia
del baseline; `dependencias.txt` documenta versiones de la venv, no un nuevo
lockfile que cambie dependencias productivas.

Las herramientas P0 no hacen llamadas institucionales. La comparación golden
conserva valores, tipos, formatos y estilos; fija timestamps ZIP únicamente
de las fixtures, evitando diferencias irrelevantes de empaquetado. No mide
fidelidad visual Office, COM ni todos los campos posibles de un XLS real.

## Decisiones y puerta de continuación

Para aprobar G0, revisar inventario, matriz, omisiones y políticas de paridad.
La cláusula del plan (§13) es: **«Una vez aprobada esa matriz, la primera
modificación de código será el validador del contrato v1, no una reescritura
de GUI.»** Esta entrega deja ese resultado concreto disponible para revisión;
no interpreta tests verdes como aprobación funcional.

El siguiente trabajo técnico está descrito en
[DECISIONES_PENDIENTES.md](../contracts/csmp_capture_v1/DECISIONES_PENDIENTES.md).
G1/P9/P10 y aceptación Windows requieren su propia evidencia; no condicionan
el trabajo offline del contrato una vez aprobada G0.
