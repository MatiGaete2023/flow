# Verificación, reproducción y límites

## Ejecutado para este informe

| Comprobación | Resultado observado |
|---|---|
| Procedencia del ZIP y copia usada | El comentario del ZIP coincide con `flow-python@93a36312adbd24e2dc96b02dfac45d947193b8b1`. Cero diferencias de bytes entre los miembros y los archivos extraídos usados. El árbol Git reconstruido, `e8ed777982255e9a613731b0f4306c5c66986586`, coincide con el que declara GitHub. |
| Catálogo de fuentes | Se analizaron por AST los 444 archivos `.py` de ese árbol. El TSV incluye módulos de aplicación, soporte, utilidades, documentación, suites y oráculos. La asignación temática es orientativa; el número de archivos no mide avance funcional. |
| Catálogos | Se contrastaron las claves y valores: 59/59 textos y 16/16 umbrales coinciden entre Python y los archivos de datos del workspace. El prototipo contiene 37 de los 59 textos, todos idénticos en esas claves, y 14 de los 16 umbrales. Ver [detalle por clave](CATALOGOS.md). |
| Prototipo local, Windows | Cargo/rustc 1.100.0. En la suite predeterminada pasaron siete pruebas y quedó una ignorada. Sin la variable FLOW_CASOS el test del oráculo retorna temprano. Ejecución dirigida con la variable configurada: 174/174. |
| Copia del prototipo publicada para estudio | Se compiló y ejecutó la suite de la copia con sus ejemplos normalizados y FLOW_CASOS activo: siete pruebas pasaron, una ignorada, 174/174 coincidencias; cinco advertencias del compilador. El recorrido sintético verifica lectura, observaciones, EML, DOCX en las etapas cubiertas y anonimización tabular. |
| Extensiones Python y Rust | Node 24.18.0. Pasaron los tres scripts de bitácora, calendario e historia del árbol Python y los tres equivalentes del workspace Rust. Usan respuestas y páginas ficticias. |
| Suite completa del workspace Rust | Se intentó `cargo test --workspace --locked --offline`. La resolución se detuvo antes de compilar porque faltaba `chrono-tz` en la caché. Los resultados históricos citados en otros documentos corresponden a sus propios entornos. |
| Suite Python de aplicación | Python local 3.14.6 carecía de pytest, pandas, openpyxl y CustomTkinter. Se verificaron las fuentes y los catálogos sin instalar dependencias. El registro histórico de esta base conserva una ejecución previa Linux/Tk y sus omisiones. |
| Pendientes | Prueba real de SITFA, integración Outlook/Word/Excel de escritorio, visual a distintas escalas, compatibilidad con Excel 2010 y medición comparativa de rendimiento. La prueba `velocidad_20000` está ignorada por defecto. |

## Repetir desde la raíz de esta rama (PowerShell)

Los pasos de instalación corresponden al **equipo de desarrollo**. Para compilar Rust hace falta Cargo, el toolchain Windows y las dependencias del lock disponibles en caché, vendor o registro habilitado. El operador recibe la distribución ya construida. Para Python usar 3.12, 3.13 o 3.14 con el lock Windows que corresponda. La instalación de dependencias puede requerir un índice autorizado. Los scripts de extensión que se indican abajo usan Node. Las pruebas de escritorio real necesitan las aplicaciones y permisos del equipo.

### Prototipo y workspace

```powershell
$env:FLOW_CASOS = (Resolve-Path rust/casos).Path
cargo test --manifest-path prototipos/flow-egui/Cargo.toml --locked -- --nocapture
cargo test --manifest-path rust/Cargo.toml --workspace --locked
cargo test --manifest-path rust/Cargo.toml -p flow-trabajo --test revision_bitacoras --locked
cargo test --manifest-path rust/Cargo.toml -p flow-motor --locked
```

Comprobar que la primera salida imprime `ORACULO: 174/174 casos coinciden` para la foto estudiada. El test aislado de velocidad se ejecuta con `cargo test --release --manifest-path prototipos/flow-egui/Cargo.toml --locked velocidad_20000 -- --ignored --nocapture`. Ese número por sí solo no permite comparar implementaciones.

### Referencia Python y extensiones

Ejemplo para una cuenta de desarrollo con Python 3.12. Invocar directamente el Python de la carpeta evita depender de la activación de scripts PowerShell.

```powershell
py -3.12 -m venv .venv-paridad
$py = (Resolve-Path .venv-paridad/Scripts/python.exe).Path
& $py -m pip install --require-hashes -r requirements/windows-py312.lock
& $py -m pip install --no-deps --no-build-isolation .
& $py -m pytest -q tests
& $py -m pytest -q descargador
& $py -m pytest -q META4_Bitacoras/tests
& $py -m pytest -q anonimizador/tests
& $py -m pytest -q descargador_lite/tests
```

Ejecutar conjuntos por separado, revisar los README de cada programa para sus dependencias y conservar el código de salida de cada comando. Para concentrarse en E1:

```powershell
& $py -m pytest -q tests/test_revision_bitacoras.py descargador/test_revision_bitacoras_p2.py
node descargador/test_bitacoras_extension.cjs
node descargador/test_calendario_extension.cjs
node descargador/test_historia_extension.cjs
node rust/pruebas/bitacoras_extension.cjs
node rust/pruebas/calendario_extension.cjs
node rust/pruebas/historia_extension.cjs
```

## Qué comparar por producto

| Producto | Comparación que decide | Variación que debe explicarse |
|---|---|---|
| Motor | Identidad, etapa, entrada normalizada, corte, códigos ordenados, textos exactos, acciones, avisos y marcas por fila | Igual número de filas puede ocultar asociaciones cruzadas. |
| XLSX revisado | Hojas, tipos, campos de revisión, fórmulas y sus referencias, validaciones, estilos relevantes, trazabilidad y datos no editados | Metadatos y compresión del contenedor pueden cambiar el hash; para el original conservado el hash sí debe permanecer idéntico. |
| ZIP CSMP | Manifiesto, versión, tablas, conteos, huellas verificables, procedencia, cobertura e incertidumbre por ingreso | Una lectura exitosa del archivo no demuestra que el consumidor comprendió el metadato V2. |
| Word | Cantidad y orden de proyectos, matriz, texto, campos pendientes, saltos, estilos, partes y relaciones; apertura real | Un DOCX válido como ZIP puede abrir con reparación o perder componentes. |
| Correo | Destinatarios, asunto, CC, cuerpo, firma, adjuntos y recibo en reintento | Mantener separados preparado, guardado, incierto y EML. |
| Meta 4 | Fecha usada, control de bitácora, alerta, hoja, estilos y totales por ingreso; regeneración en destino nuevo | La fecha más reciente de una etapa ajena no es la fecha válida del cálculo. |
| Anonimización | Rutas de salida nuevas, pseudónimos coherentes, neutralización, rechazo previo a publicar y relectura del paquete | Identificadores aleatorios y nombres neutros varían según la opción de semilla. |

Para el primer lote de E1, [el archivo de casos](CASOS_PRIMERA_ENTREGA.json) identifica el constructor existente y los cuatro resultados por fecha. Para regresión global usar `rust/oraculo/`, `rust/casos/`, `docs/evidencia_p0/golden/`, `tests/fixtures_integracion/` y los tests de cada aplicación. Los datos deben ser ficticios.

## Salida de una verificación

Registrar versión y commit, opciones, entrada ficticia, corte, etapa, resultado esperado de Python, observado en Rust y condición exacta que explica la diferencia. Conservar resultados fallidos. Si la salida proviene de un test histórico, identificar su commit y entorno. No declarar aceptación institucional a partir de una prueba sintética.
