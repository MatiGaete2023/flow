# Evidencia de H01–H09 y M01–M03

Todos los datos son sintéticos. [Informe de correcciones](../CORRECCIONES_REVISION_20261009.md).

- `tests_asistente.txt/xml`: regresión Linux/Xvfb del código final 99ed217, 602 aprobadas/11 omitidas.
- `tests_descargador.txt/xml`: 171 aprobadas y 57 subpruebas aprobadas.
- `tests_ids_windows.txt/xml` e `ids_pytest.txt`: 23 regresiones aprobadas tras acotar IDs pytest; cuatro casos UTF-16 conservados en ac7fe5b.
- `sondas_verificadas.json`: H01–H08 pasan comprobaciones explícitas con `--verify`.
- `golden.txt`: paridad de los 17 productos tradicionales.
- `conciliacion_gui.txt`: ventana real con diferencias de fuentes y opción KEEP bajo Xvfb, sin alterar el trabajo anterior.
- `benchmark_importacion.json`: importación con tracemalloc, 2.000/10.000/20.000 registros ficticios. No mide producción institucional ni todos los productos.
- `dependencias_linux.json`: inventario real contra el lock Linux, 39 dependencias, SHA del lock y del checkpoint.
- `ci_76bae9c_ids_fallo.json`: fallo Windows de preparación/cierre debido a IDs de pruebas demasiado largos; extractos acotados, sin omitir ni acortar los parámetros de prueba.
- `ci_windows.json`: CI del código final, tests por versión, postcondiciones del instalador y metadatos/digests de artefactos de GitHub.
- `adjuntos_verificados.json`: los siete originales coinciden con sus hashes P0.
- `contexto.json`: entorno, comandos y hashes de estas evidencias. La CI Windows se registra por separado con su SHA y artefactos.

Los TXT/XML se conservan tal como los emitieron las herramientas. La regresión final del Asistente incluye la continuidad de productos/editores abiertos y guardado tras redibujado de 600 registros. Los tests del Descargador y el golden se conservan desde 76bae9c: el cambio posterior afecta la GUI del Asistente y los IDs pytest, sin modificar sus módulos. Los siete adjuntos originales se comprueban con `tools/flow_plan_progress.py`; no se cambian sus hashes. Las omisiones locales de Windows/COM no equivalen a aceptación institucional. Office/SITFA/RUS quedan diferidos por decisión del usuario.
