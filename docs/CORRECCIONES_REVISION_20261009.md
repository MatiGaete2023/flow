# Correcciones de la revisión técnica — 9 de octubre de 2026

Se implementaron los **nueve hallazgos y las tres mejoras de ingeniería** en el código `99ed217c2fbb66d723b45f6e824622b07ae2f138`, en la [rama y PR #1](https://github.com/MatiGaete2023/flow/pull/1). Este documento complementa el [informe histórico](INFORME_REVISION_20261009.md); conserva las evidencias originales y registra cómo se verificó cada corrección. La aceptación institucional sigue diferida por instrucción del usuario.

| ID | Cambio implementado | Regresión / comprobación |
|---|---|---|
| H01 · Alta | Validación del texto original en unidades UTF-16 antes de crear celdas; rechazo explícito del exceso, sin sustitución del trabajo anterior. Servicio común `excel_cells.py` para importación y reportes. | ASCII 32.767/32.768 y caracteres fuera del plano básico; fila atómica y captura excesiva rechazada. |
| H02 · Alta | Servicio común de hallazgos tradicionales; recalculado, edición, deshacer, recuperación y sincronización Excel confirman filas y contextos conjuntamente, conservando decisiones humanas. | Reglas E05/E06 coherentes en edición, deshacer, recuperación y refresco externo; fallo sin cambios parciales. |
| H03 · Alta | Conciliación por identidad remota verificada y ámbito del registro; continuidad con otra captura/digest y remapeo de IDs/productos. Los cambios en fuentes editadas o correspondencias ambiguas exigen decisión explícita en la GUI. | Capturas equivalentes y complementarias, dos ingresos, IDs distintos, KEEP/RESET, duplicados y registros ausentes. Ventana real bajo Xvfb; sesión anterior intacta. La GUI también conserva el borrador abierto, remapea sus IDs y guarda/archiva correctamente después del redibujado incremental de 600 registros. |
| H04 · Alta | Índice de cobertura por fuente e ingreso; contrato rechaza sobredeclaración de filas/consultas/tablas. Solicitudes usan esa cobertura y no el estado global para inferir antecedentes suficientes. | Completo, parcial, fallido y vacío explícito por ingreso; contradicciones rechazadas. Sin identidad verificable se conserva incertidumbre. |
| H05 · Alta | Productor agrega estados desde todas las consultas de calendario, incluyendo intentos fallidos y consultas previstas sin archivo. | Lote parcial y consulta pendiente nunca se declaran completos/vacíos por tener cero filas. |
| H06 · Media | Conteos del acta coherentes: la partición declarada no supera observados; si se declaran todos los componentes debe cerrar. PASÓ exige cero fallidos/desconocidos y esperados=observados cuando ambos existen. | Rechazo del caso 1 observado/200.000 coincidentes y demás combinaciones imposibles; conteos opcionales conservados. |
| H07 · Media | Escritor y lector comparten límites de 4 MiB UTF-8 y 20.000 solicitudes; validación antes de crear directorio/archivo. El exceso requiere reducir o dividir la solicitud. | Límites en bytes, texto multibyte, documento ilegible y rechazo sin publicar archivo. |
| H08 · Media | Advertencias visibles del productor se transportan como incidencias; consumidor las conserva y presenta en evidencia y hoja Incidencias. | Discrepancia de identidad, ámbito de consulta, persistencia y texto literal Excel; visible aun fuera de la ventana temporal. |
| H09 · Media | Comprobación reusable del instalador del build actual: instalación, productos/SHA, reinstalación, desaparición real de toda la carpeta y datos externos intactos. Verificación histórica manual parametrizada por run/SHA. | Dos pruebas nativas distinguen retorno del lanzador de eliminación real. CI del nuevo instalador exige la postcondición; espera acotada, sin borrar la carpeta para ocultar un fallo. |
| M01 | Construcción de libros sin recalcular max_row/max_column por fila, cancelación durante esa etapa y máximo operativo de 20.000 registros de análisis. Benchmark separado de importación. | Cancelación y límite probados; medidas con 2.000/10.000/20.000 registros ficticios. |
| M02 | Lockfiles con versiones y hashes por Windows/Python 3.12–3.14 y Linux/Python 3.12; instalación sin resolución posterior e inventario de versiones/lock/SHA en los artefactos. | pip check, inventario Linux de 39 dependencias y comprobaciones CI en las tres versiones Windows. |
| M03 | Separación gradual de consultas/cobertura y conciliación; estructuras tipadas para hallazgos/identidad/cobertura y servicio compartido de hallazgos. Ruff de errores sintácticos y nombres indefinidos en esos límites. | Regresiones compartidas y análisis estático E9/F63/F7/F82; conserva motor y umbrales tradicionales. |

Las regresiones se encuentran en [test_review_corrections.py](../tests/test_review_corrections.py), [productor](../descargador/test_capture_review_corrections.py), [inventario](../tests/test_dependency_inventory.py) y [desinstalación](../tests/test_installer_removal.py). El detalle de implementación está en el commit indicado; no se modificaron los siete adjuntos originales.

## Verificación local

- Asistente: **602 aprobadas, 11 omitidas** en Linux/Xvfb. Las omisiones dependen de Windows/COM y se verifican por separado en CI cuando corresponde.
- Descargador: **171 aprobadas, 57 subpruebas aprobadas**.
- Los **17 productos de referencia mantienen paridad**; H01–H08 pasan las ocho sondas con `--verify`.
- Inventario bloqueado, `pip check`, compilación Python y Ruff aprobados. Conciliación en una ventana real comprobada bajo Xvfb con decisión KEEP y trabajo anterior intacto. La regresión GUI adicional confirma la continuidad de editores y productos tras importar 600 registros con IDs cambiados.

Evidencias: [directorio](evidencia_correcciones_20261009), [contexto y hashes](evidencia_correcciones_20261009/contexto.json), [sondas](evidencia_correcciones_20261009/sondas_verificadas.json). Para reproducir las sondas: `python tools/flow_review_probes.py --verify`; código 0 exige las ocho correcciones verificadas.

## Rendimiento y límites

| Registros | Importación con tracemalloc | Pico de memoria trazada |
|---:|---:|---:|
| 2.000 | 8,589 s | 34.254.049 bytes |
| 10.000 | 33,1178 s | 167.945.006 bytes |
| 20.000 | 72,3236 s | 337.036.718 bytes |

Mediciones Linux sintéticas de importación; no incluyen generación de todos los productos ni constituyen un plazo garantizado en Windows. El máximo operativo de análisis es 20.000 registros, incluidos los de calendario independientes; el contrato de transporte conserva su límite agregado de 200.000 filas/80 MiB. Un lote mayor requiere dividirse. Los textos que Excel no puede representar se rechazan explícitamente. [Medidas originales](evidencia_correcciones_20261009/benchmark_importacion.json).

## Entrega Windows

Código compilado: `99ed217c2fbb66d723b45f6e824622b07ae2f138`. [Asistente](https://github.com/MatiGaete2023/flow/actions/runs/37924605460) y [Descargador](https://github.com/MatiGaete2023/flow/actions/runs/37924605484) terminaron **SUCCESS**. Las tres versiones Python 3.12/3.13/3.14 aprobaron **612 pruebas y omitieron 1** cada una. Descargador: **171 aprobadas y 57 subpruebas aprobadas**. Los inventarios Windows acreditan las 45 versiones del lock correspondiente; pip check y Ruff aprobaron.

Se comprobaron los productos ficticios desde el EXE autónomo, ambos EXE del paquete conjunto y el EXE instalado. H09 queda verificado con el instalador recién construido: instalación, SHA esperado, productos, reinstalación, eliminación real de toda la carpeta y dato externo intacto. Las dos regresiones nativas de eliminación también pasaron. Entorno: GitHub Windows Server 2025; no acredita Windows 10/Office institucional.

- [Paquete conjunto](https://github.com/MatiGaete2023/flow/actions/runs/37924605460/artifacts/11614235483).
- [Instalador y JSON de verificación](https://github.com/MatiGaete2023/flow/actions/runs/37924605460/artifacts/11613692954).
- [Descargador autónomo](https://github.com/MatiGaete2023/flow/actions/runs/37924605484/artifacts/11613751543).
- [Resultados, pasos ejecutados y metadatos de artefactos](evidencia_correcciones_20261009/ci_windows.json).

La API de GitHub informa los digests ZIP; no se descargaron ni se recomputaron localmente estos artefactos. La comprobación de binarios se ejecutó en Windows CI y sus JSON acompañan los artefactos. Caducan según retención. Los checkpoints documentales posteriores no cambian el código compilado.

El primer run del Asistente (`76bae9c`) detectó IDs de pytest demasiado largos para una variable de entorno Windows: Python 3.14 aprobó 607 pruebas, omitió 1 y obtuvo 8 errores de preparación/cierre. Se conservaron los parámetros completos y se asignaron IDs cortos en `ac7fe5b`; 23 regresiones focalizadas pasaron. Antes del cierre se completó la continuidad GUI de H03 y se añadió la prueba de 600 registros en `99ed217`. Ese checkpoint es el que acredita la CI final; los runs anteriores cancelados al cambiar código no se presentan como entregas aprobadas. [Registro del fallo inicial](evidencia_correcciones_20261009/ci_76bae9c_ids_fallo.json).

## Límites de aceptación

No se consultaron SITFA/RUS ni Office/Outlook institucional. Historia/PDF continúa sin extractor remoto, CT sigue desactivado y la escritura institucional sigue bloqueada. Las pruebas sintéticas y la CI técnica no sustituyen esos gates. M03 es una separación gradual, no una reescritura completa del sistema legado. No se fusiona main ni se publica release.
