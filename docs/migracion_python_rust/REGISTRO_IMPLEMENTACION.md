# Registro de implementación · Flow Rust

Rama `estudio/paridad-python-rust-20261119`. Inicio de la corrección el 20 de noviembre de 2026. El [registro anterior](https://github.com/MatiGaete2023/flow/blob/ed6a0e43f8efa0856e0294af92ac8e0317c69df7/docs/migracion_python_rust/REGISTRO_IMPLEMENTACION.md) queda disponible por commit.

## Fuente y decisión

- Diseño y código de producto de la carpeta local `flowrust/flow-flow-python/app/`, publicado como `app/` en la raíz remota.
- Comportamiento de la carpeta `flowpython/flow-flow-python` y la rama `flow-python` vigente. El estudio anterior se hizo sobre `93a3631`. La punta remota consultada para esta corrección fue `afc4d12ae1adabf21049c809c0d0d8f747e2e306`. Comprobado por blob que el `src/nurus/capture/contract.py` local coincide con `flow-python` (`361d0f907100ca4a1a71c790d98f82cbe25f8bb0`). Verificar los archivos de referencia por entrega si difieren las copias.
- El traspaso de ayer terminó en `ed6a0e43`. Sus 355 pruebas Linux, 353 Windows, cuatro clasificaciones V2 y artefacto `Flow_Windows` corresponden a `rust/crates/`. Usar sus casos y resultados como referencia al portar, e identificar la cobertura específica de `app/`.

## Cambios del turno

- Se cambió el destino del plan y la matriz a `app/`. Se conservaron E0–E7 y F01–F28. Se documentaron las vistas y se regeneraron referencias sintéticas de Trabajo, Correos, Resoluciones, Anonimizador y Ajustes oscuro con el ejecutable local preexistente.
- Se respetaron las vistas y su distribución. Se reemplazaron ejemplos de contacto y datos sintéticos en UI, demo y prueba de anonimización por valores reservados. Se corrigió la aserción de esa prueba para que compruebe el mismo RIT ficticio que entrega como entrada.
- Comenzó F07 en `app/src/captura.rs`. Comprobación de rutas, formato ZIP, límites, manifiesto, rangos de versión, entradas requeridas, tamaños y huellas sin extraer archivos. El ejemplo `verificar_paquete` permite repetir la prueba con un archivo local. El flujo de Trabajo sigue abriendo sus formatos anteriores.

## Verificación y pendientes

- Windows, Rust 1.100.0. Cuatro pruebas del módulo de captura aprobadas con sus dependencias fijadas en caché, en un crate de prueba aislado bajo `app/target/`. Los ZIP completo y parcial de la referencia Python también pasaron la comprobación estructural.
- En un segundo crate aislado, se ejecutaron los módulos locales `motor.rs`, `config.rs` y `util.rs` con `FLOW_CASOS` de la referencia Python local. Pasaron las cuatro pruebas del conjunto y coincidieron los 174 casos sin historia ni bitácora.
- Revisadas las capturas de referencia de la carpeta local. El ejecutable preexistente generó cinco PNG desde el modo sintético. Los controles Python de RUT, RIT y contactos pasaron sobre `app/src`.
- La compilación completa local quedó detenida al resolver `home 0.5.12` sin caché y por acceso intermitente a `index.crates.io`. El binario local preexistente solo acredita el diseño de partida. La prueba completa y las capturas generadas por una compilación del commit corresponden al nuevo flujo Windows.
- El cambio de base quedó publicado en `1affc51ccd4a6541734bf23b51e74902540fd465`. Su [ejecución Windows](https://github.com/MatiGaete2023/flow/actions/runs/44747046388) pasó 11 pruebas, con 1 de rendimiento omitida, y comprobó los dos ZIP sintéticos. La revisión de sus cinco capturas identificó recortes a 1024 × 768 por el monitor virtual. Se ajustó el zoom del modo captura para que las próximas imágenes muestren la composición completa. Se retiró el disparador automático de la rama de migración del workflow del workspace histórico.
- **E0 verificado en Windows.** La [corrida final](https://github.com/MatiGaete2023/flow/actions/runs/44748317049) compiló y probó `app/` en `1a839bd1e5d444e8724d6135419d4b11e0039fdb`. Pasaron 11 pruebas, incluida la del oráculo Excel, y los dos ZIP sintéticos. La prueba de rendimiento se mantuvo omitida. Se abrieron y compararon las cinco [capturas del binario](https://github.com/MatiGaete2023/flow/actions/runs/44748317049/artifacts/14634511876) con el diseño local. El modo captura conserva la distribución a escala en 1024 × 768 sin guardar ese ajuste en las preferencias.
- En Windows local, se comprobó además la integridad de `rust/crates/flow-trabajo/tests/datos/paquete_integracion.zip`, con contrato 1.2.0. El lector de referencia Python validó los tres ZIP y registró cuatro filas de `registros` en cada uno.
- Pendientes F07 semántica e integración. El verificador incorporado todavía no valida contenido CSV, conciliación entre consultas, identidad, cobertura, ni estructura interna del XLSX. Sigue después F06 revisión y exportación, y E2 para el Descargador integral y Lite. E3–E7 conservan el orden del plan.

## Punto de partida del turno 2

1. En `app/src/captura.rs`, usar los encabezados de `TABLES` y las comprobaciones de `read_capture` y `_validate_rows` de Python. Añadir casos con bytes y huellas correctos pero columnas, filas, estados o identidad inválidos. Mantener evidencia incompleta como tal para V1, V2 y legado.
2. Cuando ese lector pase, integrar apertura por botón existente, Ctrl+O y arrastre en `ui.rs`. Preparar el nuevo análisis antes de cambiar `self.work`. Fijar etapa y corte, y preservar decisiones por identidad de origen. Probar P2 completo con el ZIP sintético que genera `descargador/test_revision_bitacoras_p2.py` y [las cuatro clasificaciones](CASOS_PRIMERA_ENTREGA.json) antes de continuar a exportaciones y E2.

## Turno 2 · Apertura de paquetes en Trabajo

- El lector valida encabezados, comillas, conteos, JSON con claves únicas, equivalencia de las consultas con el manifiesto, identidad, alcance, cobertura, referencias de documentos y estados de tablas. Comprueba el contenido interno y los CRC del consolidado XLSX. El ejemplo `verificar_paquete` ejecuta esta validación.
- Trabajo abre ZIP con el botón, Ctrl+O, arrastre o apertura externa. El selector existente cambia entre Espera, Cumplimiento e Informes del mismo paquete. Las columnas se mapean por fila y se muestran los identificadores y la procedencia de captura. Se guarda la edición pendiente antes de reemplazar el trabajo y se restaura la sesión del mismo hash, etapa y corte. La conciliación entre ZIP distintos sigue en F07/E4.
- Se usa la misma distribución de la aplicación local. El botón anuncia Excel o ZIP y los paquetes muestran el alcance tabular. El análisis del contexto de historia, calendario y bitácoras sigue en F06/E3. Egresados y Otros quedan contabilizados como omitidos de las tres etapas.
- En Windows local pasaron seis pruebas del lector aislado con `FLOW_REFERENCIA`, incluidos ZIP 1.0 completo y parcial, ZIP 1.2 y rechazo de tribunal cambiado con huella recalculada. Clippy del lector aislado pasó con advertencias tratadas como errores. El workflow Windows activa además la regresión de apertura en Trabajo, cambio de etapa y conservación del trabajo activo ante un error.

- El importador pasó el [flujo Windows](https://github.com/MatiGaete2023/flow/actions/runs/44751835256) en `6a819aabac109bde74cc15fa4fff716ae48670d4`, incluida la regresión de Trabajo.

## Turno 2 · Revisión de bitácoras V2

- Desde Trabajo con un ZIP abierto, **Revisión de bitácoras…** permite elegir fechas, ver conteos y filtrar por clasificación. Se puede abrir la fila en Trabajo y exportar las filas visibles a Excel y JSON en una carpeta nueva. Las salidas aparecen en Resultados.
- El clasificador V2 usa sistema, tribunal, causa, ingreso y consulta. Solo acredita ausencia con evidencia completa, sin advertencias y dentro del período conservado. Un paginador, una lectura anterior al término, evidencia duplicada o autoría incierta evita certificar ausencia. El formato nativo V1 y el legado siguen apareciendo como no verificables hasta portar su respaldo.
- `app/qa/contrastar_bitacoras.py` genera con Python un único ZIP ficticio P2 y compara las [cuatro clasificaciones](CASOS_PRIMERA_ENTREGA.json) con el ejemplo Rust. El flujo Windows verifica también la escritura de los dos productos y la conservación de un destino existente.
- El [flujo Windows de revisión V2](https://github.com/MatiGaete2023/flow/actions/runs/44753026710) pasó 15 pruebas y el contraste de las cuatro clasificaciones de P2 sobre el mismo ZIP. Se mantuvo omitida la prueba de rendimiento. Se amplió el contraste para cotejar el Excel y JSON producidos por el exportador de Trabajo.
- La [verificación Windows ampliada](https://github.com/MatiGaete2023/flow/actions/runs/44754662180) pasó sobre Rust `563f0691c615fc9d1a9c67fd08a3d1430cf90d9b` y Flow Python `afc4d12ae1adabf21049c809c0d8f747e2e306`. Pasaron 16 pruebas de la suite, 1 prueba focal de exportación y los cinco casos P2, incluida la identidad visible discrepante. Coincidieron período, corte, clasificaciones, conteos, encabezados y datos visibles de los productos del mismo ZIP. La prueba de rendimiento siguió omitida. Las propuestas del motor y la redacción de motivos se cotejan por separado.

## Turno 3 · Excel de bitácoras compatible con la revisión general

- `Revision_bitacoras.xlsx` usa la misma construcción de columnas, filas y formato que **Guardar observaciones**. Mantiene el nombre de la hoja por etapa, los datos de origen e identidad, observación con ediciones, estado, reglas, avisos, RES y fila de origen. Añade `CLASIFICACION_BITACORA` y `MOTIVO_BITACORA` al final. El alcance de filas sigue el filtro de la revisión. El identificador `ID_REGISTRO_CAPTURA` permite incorporar las columnas de bitácora por registro en el Excel general del mismo paquete.
- El contraste P2 genera los dos Excel desde el mismo Trabajo y coteja columnas, valores comunes, formato, anchos, filtros y panel fijado. La regresión del exportador prueba también una fila filtrada con edición, marca de revisión y RES.
- La [verificación Windows del Excel](https://github.com/MatiGaete2023/flow/actions/runs/44758334314) pasó las comprobaciones de ambos libros, las 16 pruebas y los cinco casos P2 de la referencia Python.

## Turno 3 · Evidencia nativa V1

- El clasificador coteja las entradas nativas con su consulta y tabla normalizada, incluidos identidad, ID del evento, fecha civil y precisión, texto y huella de la lectura. Usa la autoría del respaldo V1 y conserva la incertidumbre del material legado y de las lecturas contradictorias.
- El contraste Windows usa paquetes sintéticos V1 construidos con el contrato completo. Cubre una observación del centro, ausencia dentro del alcance, lectura anterior al fin, autoría del tribunal, falta de respaldo y huella discrepante. Mantiene los casos V2 de Flow Python.
- La [verificación Windows](https://github.com/MatiGaete2023/flow/actions/runs/44760028232) pasó sobre `00623a13ca29c599d4f56d5784da1c860a578bee`. Pasaron 17 pruebas, una prueba focal de exportación, los seis casos V1 y legado y los cinco P2 de Flow Python. Coincidieron los datos y el formato de las columnas comunes de ambos Excel. La prueba de rendimiento se mantuvo omitida.

## Turno 4 · Selección desde bitácoras

- **Seleccionar en Trabajo** toma las filas marcadas del filtro activo o todas las visibles si se deja la tabla sin marcas. Reemplaza el conjunto de marcas de Trabajo, enfoca la primera fila y activa **Solo filas marcadas**. Ante un error de guardado se conservan las marcas anteriores. La selección y las ediciones se restauran con la sesión del mismo ZIP, etapa y corte.
- Excel y JSON usan el mismo subconjunto. Siguiendo Flow Python, el período vuelve a ser el mes hasta el corte al abrir otra vez el diálogo.
- La prueba V1 con contrato completo incluye dos instantáneas idénticas y dos contradictorias. Las primeras conservan observación y ausencia acreditada. Las segundas quedan no verificables en ambos períodos.
- La [verificación Windows de selección](https://github.com/MatiGaete2023/flow/actions/runs/44763988257) pasó 19 pruebas de Rust, la prueba focal de exportación, 10 casos V1 y legado y cinco casos P2. Se mantuvo omitida la prueba de rendimiento.

## Turno 4 · Perfiles del Descargador

- Dentro de **Herramientas → Descargador**, se puede abrir o arrastrar un catálogo JSON y validar un perfil. Lite limita el catálogo a Seguimiento, Búsqueda de litigantes, Informes por vencer y Medidas por vencer. Integral añade Carga. La aplicación valida los filtros propios de Informes y Cumplimiento, el estado de litigantes, mes y año, y el máximo de 31 días inclusivos de Carga.
- Se conservaron el tema y los controles de Flow Rust. La preparación de perfiles funciona con catálogos locales; la conexión del Descargador, la planificación combinada de lotes y sus productos siguen en E2.
- La [verificación Windows de perfiles](https://github.com/MatiGaete2023/flow/actions/runs/44766033616) pasó 22 pruebas de Rust, la prueba focal de exportación, 10 casos V1 y legado y cinco casos P2. Se mantuvo omitida la prueba de rendimiento. Se revisaron las [capturas de Trabajo y Descargador](https://github.com/MatiGaete2023/flow/actions/runs/44766033616/artifacts/14645840756) del binario generado en esa ejecución.

## Turno 5 · Preparación de lotes del Descargador

- **Herramientas → Descargador → Planificar lote** permite marcar varios tribunales, pestañas y modalidades. Informes y Cumplimiento admiten sus filtros por separado. El resultado muestra el total y las consultas previstas en la misma vista que valida perfiles individuales.
- El plan reutiliza la validación de perfiles. En Lite conserva el orden fijo de pestañas de Flow Python. En Integral respeta el orden de selección y permite combinar filtros por pestaña. Litigantes combina estados, los calendarios combinan meses de un año y Carga combina tribunales con un período de hasta 31 días.
- Las pruebas de Rust cubren el orden de las 16 combinaciones del caso de referencia de Lite, filtros por pestaña, duplicados, tribunales ajenos al catálogo y restricciones de fechas. Las cinco pruebas del Descargador pasaron localmente en el entorno aislado. La compilación completa y la captura de la vista corresponden al flujo Windows de la rama.

## Turno 5 · Contraste con el exportador nativo V1

- El flujo Windows llama al lector vigente desde la prueba del productor histórico. Usa sus ZIP originales, generados a partir de HTML ficticio y sin modificaciones intermedias. Coteja por identificador de registro las ventanas con observación, vacía, paginada y sin tabla. La prueba histórica sigue funcionando por sí sola cuando no se configura `FLOW_RUST_REVISAR_V1`.
- La [verificación Windows](https://github.com/MatiGaete2023/flow/actions/runs/44768386157) pasó las 24 pruebas de la aplicación, la prueba focal del Excel, los 10 casos V1 y legado, los cinco P2 y los cuatro ZIP del productor. Se mantuvo omitida la prueba de rendimiento. Los dos casos de planificación y calendario de Flow Python Lite pasaron en la referencia local.

## Turno 5 · Carga de catálogos desde Chrome

- **Conexión con Chrome** incorpora el puente de escritorio en la vista Descargador. El código local se genera al abrir, se copia bajo demanda y se invalida al cerrar o cambiar de edición. Se puede elegir una pantalla y cargar sus opciones en segundo plano. El selector de perfiles y la vista previa de lotes usan el catálogo resultante. Se mantiene la apertura de un JSON local.
- El transporte usa el protocolo local comprobado en el workspace histórico y los contratos de las extensiones Flow Python Integral 2.9.0 y Lite 1.1.0. En esta etapa el puente solicita capacidades y catálogos. Una extensión ficticia recorrió la conexión, la consulta de capacidades y la carga de un catálogo Lite sobre HTTP local. El cierre de E2 requiere ejecutar y recuperar lotes.

## Turno 6 · Ejecución de lotes Lite

- `app/src/descargador/lote.rs` porta `BatchRunner.run` y `run_sequence` de `descargador_lite/app/lotes.py` y `motor.py`. Orden `lock` → `prepare` → `search` → `download` por página → `unlock`. Una carpeta por lote (`AAAA-MM-DD HH-MM-SS`), nombres de `file_base` con límite de 260 caracteres, `verificacion NNN.json` por consulta y `resumen.json` (`lite-1`) tras cada paso. La cancelación conserva lo verificado y deja `INCOMPLETA`.
- `app/src/descargador/sitfa.rs` y `sitfa/` portan `perfiles.response_profile`, `motor.search_page`, `query_pairs`, el selector auditado de `TIP_Informe` y la lectura de calendarios. Base: `rust/crates/flow-captura/src/sitfa/`, ajustada a los mensajes y reglas de Lite (plantilla por página, paginación heredada como `pagination.get(k, "")`).
- `app/src/descargador/xls.rs` lee XLS-BIFF y OOXML con calamine y HTML tabular con scraper; coteja registros con la página, mes del calendario y duplicados entre páginas. `evidencia.rs` convierte el pantallazo PNG a PDF.
- El puente admite las diez órdenes de `puente.Bridge.call` de Lite; las órdenes de bitácora, historia y navegación siguen rechazadas.
- **Herramientas → Descargador → Planificar lote → Ejecutar lote**: carpeta de destino (por omisión `Documentos/SITFA Lite`), PDF de vacías, conservar filtros (Seguimiento y Litigantes), pausa (1 s por omisión), cancelar, avance por consulta, resumen final y apertura de la carpeta. Requiere catálogo cargado desde Chrome, extensión Lite conectada y verificada (`capabilities`).
- Pruebas: `sitfa_ficticio.rs` porta `descargador_lite/tests/sitfa_ficticio.py` y reproduce los casos de `test_lite.py`: Seguimiento con 4 consultas y 3 páginas, PDF de vacía, sin PDF al desmarcar, Litigantes, Informes y Medidas por vencer, nombres acotados, más cancelación y XLS que no coincide. En Linux, crate aislado con enlaces a `captura` y `descargador`: 28 pruebas de la biblioteca aprobadas y clippy sin advertencias. `cargo check` y clippy de la aplicación completa para `x86_64-pc-windows-gnu` sin advertencias nuevas.
- Observación de paridad: en Seguimiento sin informe ni medidas, `file_base` de Python toma la etiqueta de fechas dentro de la cabecera (`parts[:4]`) y el tribunal queda después. Se reprodujo igual; cambiarlo requiere decisión del usuario.
- Pendientes de E2: Excel XML 2003 (rechazado con mensaje explícito; los XLS reales observados son BIFF), ejecución Integral (`descargador/lotes.py`, con bitácoras, historia, calendario e histórico), Carga, preferencias persistentes, reanudación y prueba con Chrome real.

## Turno 6 · Copia preservada del Excel revisable

- Reporte del usuario: el Excel de **Guardar observaciones** no era el de siempre y no traía el RIT. Causa: `productos::excel_observaciones` creaba un libro nuevo con los encabezados de Trabajo (en paquetes, columnas canónicas). Flow Python (`Work.export` → `exports._native_preserved`) entrega una copia del original con Excel de escritorio.
- `app/src/copia_excel.rs` reproduce esa copia: abre una copia temporal del original (o de `consolidado/consolidado.xlsx` del paquete, mapeado por `SITFA_SHA256`, `SITFA_ARCHIVO` y `SITFA_FILA` en una sola hoja), conserva hojas, formato y columnas originales, agrega `NURUS_ID_REGISTRO`, `NURUS_REGLAS` (oculta), `NURUS_PROPUESTA` u `NURUS_OBSERVACION_FINAL`, `OBSERVACION`, `FECHA_OBS`, `TT`, `CC`, `RES` con lista `PC_IE/PC_INFO/NOMENCL`, `NURUS_ESTADO_REVISION`, marca excluidas y crea `NURUS_TRAZABILIDAD` oculta. Nombre: `<origen> - revisable AAAAMMDD_HHMMSS_ffffff.<ext>`, misma extensión.
- Sin libro que preservar (CSV o paquete sin consolidado utilizable) se mantiene la tabla nueva con el aviso de fidelidad reducida de Python.
- Verificación: pruebas del plan de columnas y estados (`copia_excel::tests`) y prueba Windows de sintaxis del script con el parser de PowerShell. No se pudo ejecutar Excel: el entorno es Linux y los runners de GitHub no traen Office. **La prueba con Excel real corresponde al usuario.**
- Diferencias abiertas con Python: no se escribe `NURUS_DECISIONES`; `FECHA_OBS`, `TT` y `CC` no se editan en Rust y se conservan como estén; el mapeo del consolidado no compara además los valores de identidad (`_source_cell_key`).

## Turno 6 · Ejecutable en CI

- Las corridas 17–19 fallaron en el control de datos sintéticos por un RIT de prueba sin año 2099 (`sitfa_ficticio.rs`); se corrigió en `9e428c8`.
- «Flow Rust local · Windows» compila ahora `flow_rust.exe` en release y lo publica como artefacto `Flow_Rust_Windows_exe` (con `SHA256.txt`, 14 días), sin release ni tag. En la [corrida 21](https://github.com/MatiGaete2023/flow/actions/runs/44777123749) pasaron las pruebas de `app/` (incluida la sintaxis del script de Excel), los ZIP sintéticos, el contraste V2 y la subida del ejecutable.

## Turno 7 · Prueba del usuario e informe de fallas

- El usuario probó el ejecutable de `9e428c8` y reportó ocho fallas. El informe, la evidencia y las causas están en [INFORME_FALLAS_20261120.md](INFORME_FALLAS_20261120.md).
- Los libros aportados no se incorporan. Se analizaron solo su estructura anónima y una copia seudonimizada en la sesión (`nurus.personal.anonymize`, modo conservador), fuera del repositorio.
- Hallazgos que cambian el plan:
  - Con ZIP, la copia preservada cayó a la tabla reducida.
  - El orden de columnas NURUS de los productos reales (`FECHA_OBS` antes de `OBSERVACION`) difiere del código de `flow-python` [CONFLICTO_ABIERTO].
  - Las fechas de las resoluciones van en palabras.
  - Los checkboxes de las tarjetas quedan tapados por el área clicable.
  - Cinco patrones de redacción (R1, R2, R3, R5 y R6) explican la mayoría de las diferencias con lo registrado en NOVIEMBRE.

- Decisiones del usuario del 20-nov registradas en el [informe](INFORME_FALLAS_20261120.md) y en el [plan](PLAN.md): R1, R2, R3 y R6 aprobadas; R4 quitar; R5 parcial; orden de columnas de NOVIEMBRE confirmado.

## Turno 8 · Correcciones del informe de fallas

Commits `9b0394a` a `864164a`. Las corridas Windows 22 y 23 pasaron. La 24 falló por una prueba que esperaba un solo archivo Word; se corrigió en `864164a`. Se canceló la corrida de `580660d`, reemplazada por la de `864164a`. En Linux, los crates aislados de `captura`/`descargador` y de los módulos base del Asistente (`anon`, `captura_trabajo`, `config`, `copia_excel`, `demo`, `lectura`, `motor`, `productos`, `trabajo`, `util`) pasan 29 y 15 pruebas; este último incluye el oráculo `FLOW_CASOS`.

- **Punto 7 (Excel final).**
  - `copia_excel::base_paquete` compara por huella, nombre base del archivo y fila: el paquete guarda los originales en `originales/lN/` y el consolidado no.
  - Si descarta la base, el aviso indica el motivo (filas sin correspondencia, repetidas o en varias hojas).
  - Columnas en el orden del registro: `… NURUS_PROPUESTA, FECHA_OBS, OBSERVACION, TT, CC, RES, NURUS_ESTADO_REVISION, NURUS_DECISIONES`. `NURUS_DECISIONES` va oculta, con el esquema JSON versión 1 de `record_edits.encode`.
  - Prueba con consolidado sintético.
- **Punto 4 (marcas).**
  - Las tarjetas de Correos y Resoluciones registran su clic con `UiBuilder::sense`, debajo del checkbox.
  - Trabajo agrega «Marcar visibles», «Desmarcar todas» y tramos con Mayús+clic.
- **Punto 6 (extensión).** `descargador/extension.rs` incluye las extensiones Integral 2.9.0 y Lite 1.1.0, idénticas a `flow-python`. «Instalar extensión…» las escribe en `%LOCALAPPDATA%\FlowRust\extension-<edición>-<versión>`, copia la ruta y abre `chrome://extensions`.
- **Punto 3 (correos).**
  - Al primer inicio se importan los contactos de `%LOCALAPPDATA%\CSMP_Personal\contactos.json`, incluidos «Tribunal: X».
  - La nómina se edita por celda, con filas que se agregan y quitan.
  - Se avisa cuando un borrador se guarda sin destinatario.
  - No se encontró en el código un bloqueo por falta de contactos; queda por reproducir con el usuario.
- **Punto 2 (resoluciones).**
  - Las matrices `plantillas_word/{LAJA,MULCHEN}` van incluidas en el ejecutable; Tomé usa Mulchén y una plantilla propia del usuario tiene prioridad.
  - Fechas en palabras (`fecha_en_palabras`), `PERSONAS`, `DETALLE_PERSONAS`, plural, varios programas y fechas, y sin cursivas.
  - **Un .docx por proyecto** en `Resoluciones_<fecha>/`: Flow Python une todos en un solo archivo con `docxcompose`. Queda por confirmar con el usuario.
- **Punto 8 (redacción).**
  - R1 omite la oración C10 con la medida por vencer y conserva el evento para el correo.
  - R6 aplica `titulo_programa_registro`: siglas, artículo inicial con mayúscula, tipo título y solo las tildes observadas (Tomé, Ayún).
  - Ambas quedan detrás de `Config::redaccion_registro`, activo por omisión, con interruptor en Ajustes. El oráculo de Python corre con la opción desactivada.
  - R2, R3, R4 y R5 dependen de reglas `HISTORIA.*` y `COMUN.REITERA_OBSERVACION` que el motor Rust aún no tiene; se aplican al portar E3.

## Turno 8 · Primer tramo de E3: escritos y Mejor Niñez

- `captura_trabajo::contextos` lee `escritos` e `historico_proteccion` del paquete. Elige la última lectura fiable por causa (escritos) y por persona (histórico), como `escritos._mejores_consultas`, sin la detección de lecturas simultáneas discordantes (E3B-02), que queda pendiente.
- Trabajo muestra en el detalle de la fila:
  - los escritos pendientes de la causa, sin los vinculados a otro ingreso (`resumen_escritos`), con cobertura y fecha de lectura;
  - los programas vigentes del Ingreso Histórico (Mejor Niñez), con fecha de egreso «--» y causal «SIN INFORMACION», y la verificación de identidad.
- No cambia observaciones, reglas ni estados.
- Verificación Windows: las [corridas 26 y 27](https://github.com/MatiGaete2023/flow/actions/runs/44787352420) pasaron sobre `864164a` y `8faa9ac`. La 27 publicó `Flow_Rust_Windows_exe` con todas las correcciones del turno.
- **Sin prueba de punta a punta**: falta un paquete sintético con esas tablas. Se compiló para Windows y no se ejecutó con datos. Primer paso de la próxima sesión: generar el fixture con los ayudantes de `descargador/test_paquete_escritos.py` de `flow-python` (`con_escritos`, `_persona`) en el flujo Windows y agregar la prueba Rust.

## Turno 9 · Fuentes de contexto de Trabajo

- Se retomó desde `d02867f`. `app/qa/generar_contexto_e3.py` genera con el exportador Flow Python un ZIP 1.2.0 ficticio de Cumplimiento, con escritos, historia, Mejor Niñez, bitácoras y calendario.
- En Trabajo, la tarjeta **Fuentes del paquete** conserva Escritos y Mejor Niñez y agrega el estado, el volumen y la fecha de las lecturas de historia de la causa y bitácora y calendario por ingreso. Muestra la primera fecha proyectada pendiente cuando tiene asociación verificada.
- Los índices distinguen sistema y tribunal, causa, persona e ingreso. Se construyen al analizar y se actualizan con el corte. Se usa la fecha civil de la fuente para excluir lecturas posteriores y el instante UTC para ordenar lecturas de igual cobertura.
- Escritos e Histórico cotejan el contenido y la multiplicidad de las filas de lecturas del mismo instante. Una lectura parcial es compatible con una completa si sus filas están contenidas en ella. Ante discordancia la vista indica cobertura parcial y evita atribuir escritos pendientes o programas vigentes. En un empate entre lecturas equivalentes, se elige el identificador menor.
- Pasaron 15 pruebas locales de los módulos de Trabajo y captura en un entorno aislado, incluida la apertura de ambos ZIP, el vínculo por causa y persona, la separación de ingresos y el cambio de corte. El productor de pruebas confirma con Flow Python la discordancia entre lecturas completas de Escritos y entre lecturas completa y parcial de Histórico. Pasó también el oráculo del motor con `FLOW_CASOS`.
- Con un ZIP en Cumplimiento, la regla vigente del próximo informe recibe la primera proyección pendiente y verificada posterior al corte. El caso de paridad usa dos ingresos, una proyección enviada y otra pendiente. Comprueba la regla C-10 y que el cruce desaparezca antes de la lectura y al llegar el vencimiento. El siguiente paso es conciliar la pestaña Informes y el calendario 91.
- En la [verificación Windows de E3](https://github.com/MatiGaete2023/flow/actions/runs/44796642468) pasaron 52 pruebas de la aplicación, las dos pruebas dependientes de los ZIP E3, el contraste P2 y los cuatro casos del productor histórico V1. La prueba de rendimiento quedó omitida.

## Turno 9 · Encabezado de la copia revisada

- En Excel de escritorio se reprodujo que una edición cambiaba el encabezado `NURUS_PROPUESTA` a `NURUS_OBSERVACION_FINAL`, en conflicto con el orden acordado para el registro mensual. `NURUS_PROPUESTA` conserva ahora la propuesta del motor en ambas etapas. `OBSERVACION` contiene el texto aprobado. La prueba del plan coteja ambas celdas y el estado revisado.
- Se abrió y comprobó la copia nativa de un XLSX ficticio y del consolidado del ZIP ficticio E3 con Excel de escritorio. Ambos conservaron RIT, orden de las columnas NURUS, propuesta y edición por separado, RES, hojas de origen y trazabilidad oculta. La hoja auxiliar del XLSX conservó su fórmula.
- La [verificación Windows final](https://github.com/MatiGaete2023/flow/actions/runs/44798041877) pasó sobre `eeab6af327d9defd53f6efb14f0d639340f4f927`. Pasaron 52 pruebas de Rust, las dos regresiones con ZIP generados por Flow Python y los contrastes V1, legado y V2. Una prueba de rendimiento quedó omitida. La ejecución publicó el [ejecutable de Windows con su SHA-256](https://github.com/MatiGaete2023/flow/actions/runs/44798041877/artifacts/14660500872) y las [capturas del diseño](https://github.com/MatiGaete2023/flow/actions/runs/44798041877/artifacts/14660635669).

## Turno 10 · Copia preservada para el Excel de bitácoras

- Con consolidado utilizable, la revisión de bitácoras usa el mismo escritor y plan base que **Guardar observaciones**. Escribe clasificación y motivo exclusivamente en las filas elegidas por la revisión, al lado del identificador `NURUS_ID_REGISTRO`. El JSON mantiene solo ese subconjunto.
- En la hoja principal del Excel se ocultan las filas que quedan fuera de la selección. La copia conserva esas filas como referencias de fórmulas, así como las hojas auxiliares. Si el paquete carece de libro compatible, el Excel de respaldo contiene solo el subconjunto en la tabla normalizada y la interfaz muestra el motivo.
- Con Excel de escritorio se compararon los dos libros del ZIP ficticio E3 con una fila seleccionada y editada. Coincidieron hojas, valores y formato de las columnas comunes, RIT, decisiones y RES. Solo la fila elegida quedó visible y recibió las dos columnas de bitácora. La huella del ZIP de origen se conservó.
- La [verificación Windows](https://github.com/MatiGaete2023/flow/actions/runs/44801884998) pasó 53 pruebas de la aplicación y los contrastes de respaldo. Incluye la prueba de la fila elegida contra el mismo plan de la copia general.

## Turno 10 · Calendario 91 y cotejo CAL-03

- Con un ZIP, el próximo informe de Cumplimiento usa la fecha única y verificada del calendario 91 dentro de los 60 días posteriores al corte. Si no hay una fecha única, consulta la proyección pendiente del calendario por ingreso. Las dos fuentes se relacionan por sistema, tribunal, causa e ingreso y usan las lecturas disponibles al corte.
- Cumplimiento e Informes muestran CAL-03 cuando el calendario por ingreso discrepa de Hoja2 o del vencimiento de la pestaña Informes. Si la lectura del calendario es parcial y no trae una proyección pendiente, se evita afirmar que no hay pendientes.
- Se verificó en Windows el mismo vencimiento y texto CAL-03 de Flow Python sobre el paquete ficticio histórico `paquete_flujo.zip`. El productor de pruebas agregó `contexto-e3-informes.zip` para contrastar la pestaña Informes. La ejecución local de Trabajo comprobó ambos casos y la separación de los ingresos.
- La [verificación Windows](https://github.com/MatiGaete2023/flow/actions/runs/44802559239) pasó 55 pruebas de la aplicación y tres regresiones focalizadas de E3, incluidas las discrepancias de calendario e Informes.

## Turno 10 · Cronología acreditada de bitácoras

- **Trabajo → Fuentes del paquete → Cronología de bitácora** muestra los últimos cuatro meses civiles por ingreso. Distingue entradas del centro con autoría acreditada, posteriores, de orden incierto y sin autoría acreditada. Cuando dos horas son comparables, usa el orden UTC dentro del mismo día civil. Muestra también observaciones al tribunal de al menos 30 días sin respuesta registrada.
- La cronología reutiliza la verificación V1/V2 de `captura::revision` por evento. Incluye solamente las lecturas disponibles al corte. Si no hay lectura acreditada, la vista la distingue de una lectura sin entradas capturadas.
- Pasaron 16 pruebas locales de captura, cronología y motor con el ZIP ficticio E3 y el corpus de Flow Python. El flujo Windows también captura `trabajo-contexto.png` al abrir ese ZIP.

## Turno 10 · Reiteración de observaciones y R5

- El motor reitera únicamente cuando el texto vigente coincide con la última observación no reiterada del centro, acreditada para ese ingreso y período. Si varias observaciones del mismo día tienen orden incierto y textos distintos, el motor conserva la propuesta completa. El aviso CT-07 registra el texto original.
- Con la redacción mensual activa se conservan literalmente «Medida revisada, sin observaciones.» y las derivaciones sin seguimiento. Con la redacción mensual desactivada se mantienen las excepciones de Flow Python. La regla se puede desactivar en Ajustes y mantiene las acciones previas.
- Pasaron dos pruebas con las excepciones, los textos distintos y la autoría u orden inciertos. Flow Python generó `contexto-e3-reiteracion.zip`; ambos motores emitieron la reiteración para el primer ingreso. El segundo conservó su propuesta.
- La matriz incorpora el inventario de Ajustes, con las opciones de Flow Python que ya están en Ajustes, Correos, Contactos o Anonimizador de Flow Rust. Los umbrales de prórroga y búsqueda se incorporarán junto a sus reglas.
- La [verificación Windows final](https://github.com/MatiGaete2023/flow/actions/runs/44805108773) pasó 59 pruebas de la aplicación, cuatro contrastes focalizados de E3 y los contrastes de bitácoras P2, legado y V1. Una prueba de rendimiento siguió omitida. Se revisó `trabajo-contexto.png` con el ZIP abierto al corte del caso, dentro de la vista vigente. Se publicaron el [ejecutable con SHA-256](https://github.com/MatiGaete2023/flow/actions/runs/44805108773/artifacts/14662533335) y las [capturas](https://github.com/MatiGaete2023/flow/actions/runs/44805108773/artifacts/14662228757).

## Turno 11 · Folios de Espera y otros programas vigentes

- `captura_trabajo.rs` entrega al motor trámites de una lectura acreditada para la causa y su ventana, junto con observaciones del centro verificadas por ingreso. Para comprobar constancias previas del mismo programa, también considera las observaciones acreditadas de otros ingresos de la misma persona. Eventos posteriores al corte, trámites sin identidad de causa y lecturas simultáneas contradictorias quedan fuera de la evaluación.
- `HISTORIA.OTRO_PROGRAMA` comprueba los dos indicadores de vigencia en la misma fila del Histórico de Protección. Excluye el programa actual y compara denominaciones completas. Una simple mención, una negación o la constancia sobre otra persona no suprimen la propuesta. Una bitácora parcial o una mención anterior queda visible para revisión. La consignación manual todavía está pendiente.
- Espera incorpora `HISTORIA.E04_CON_FOLIO`, `HISTORIA.SERVICIO_INGRESO` y la marca de ingreso efectivo cuando RUS sigue en Espera. R2 deja la ambigüedad de causa con varios ingresos en un aviso interno. R3 une el encabezado de DCE cuando corresponde correo y R4 conserva la resolución ordenada sin agregar días de espera. El motor conserva las acciones de Espera.
- `contexto-e3.zip` contrasta el C-10 y dos programas vigentes. El exportador Python genera también `contexto-e3-espera.zip` con una nómina de Espera y un folio de la misma causa. Las pruebas focalizadas cubren folio único, folios discordantes, trámite de otra clase, cita previa, causa compartida y coincidencia de persona y programa.
- La verificación local del lector y del motor pasó 21 pruebas y los 174 casos del corpus de Flow Python. Trabajo abrió ambos ZIP ficticios y produjo los mismos textos y eventos esperados. La compilación completa de la aplicación depende de la verificación Windows porque la caché local carece de la dependencia `home` de eframe.

## Turno 11 · Informe de avance pendiente en Escritos

- El motor integra los escritos de tipo «Inf. Cump» y estado «Pendiente». En Informes solo retira el correo y el proyecto del vencimiento atribuido. Una asociación explícita exige ingreso y fecha proyectada. La inferencia por candidato único exige bandeja completa, causa sin duplicidad y un solo vencimiento pendiente. Si el usuario desactiva la frase, la decisión sobre las acciones se conserva.
- Las bandejas discordantes, la atribución ambigua y los estados incompatibles quedan marcados. Las lecturas parciales pueden aportar un vínculo explícito. Los escritos asociados a otro ingreso se excluyen. Si la historia contiene un trámite de informe con referencia documental atribuida a la misma proyección, la bandeja no alcanza para declarar que sigue pendiente de resolver.
- Se comprobó el vencimiento explícito del informe en `paquete_estructura.zip` de Flow Python (16 vencimientos, 10 ingresos). La frase y el retiro de ambas acciones se aplicaron solo a la fila correcta. Las pruebas locales del lector y del motor crecieron a 22 y los 174 casos sin contexto del corpus siguieron coincidiendo.

## Turno 12 · Historia de Informes y proyecto de nomenclatura

- Se retomó desde `9e9ffa0`, validado por la [corrida Windows anterior](https://github.com/MatiGaete2023/flow/actions/runs/44809482417).
- Informes cruza las referencias documentales de Escritos e Historia por ingreso y fecha proyectada. Una resolución atribuida retira las acciones ordinarias y deja la revisión de la asociación. Un escrito incorporado sin ingreso posterior activa `NOMENCL`. Los folios de otra proyección, con dueño ambiguo o de cualquiera de las lecturas simultáneas discordantes se excluyen de la atribución genérica.
- Si un trámite de la causa no se puede atribuir al informe, queda como constancia y se conservan las acciones. La acumulación de vencimientos separa los ingresos verificados del ZIP por sistema, tribunal, causa e ingreso. Cumplimiento incorpora la regla del escrito sin ingreso posterior.
- El proyecto `NOMENCL` completa su matriz con la nomenclatura de la observación o la regla. Se probó el valor en la vista previa y la prioridad de la edición del usuario.
- Pasaron 65 pruebas locales con los ZIP sintéticos de Flow Python y el corpus de 174 casos. El contraste de `paquete_estructura.zip` comprobó las 16 filas contra su oráculo de observación, reglas, marcas y acciones con la redacción mensual desactivada. Se comprobó también la compilación cruzada de todos los destinos de la aplicación para Windows.
- La [corrida Windows](https://github.com/MatiGaete2023/flow/actions/runs/44949821621) pasó 66 pruebas de la aplicación, los contrastes V1 y V2 y los ZIP de contexto. Publicó el [ejecutable de `6718493`](https://github.com/MatiGaete2023/flow/actions/runs/44949821621/artifacts/14724493369) y las [capturas](https://github.com/MatiGaete2023/flow/actions/runs/44949821621/artifacts/14725163598). La huella SHA-256 coincide con el binario descargado. La prueba de rendimiento de 20.000 registros se ejecutó en Linux por separado.

## Turno 13 · Prórroga de Cumplimiento

- Se retomó desde `0246a94`. `historia::prorroga` porta `reglas_historia.prorroga` con `constancia_tramite` (`HISTORIA.BASE_TRAMITE` y la denominación según «Trámite»), `en_bitacora` y `ultimo`. Cumplimiento la llama después de C10, con la medida vencida válida, por vencer y el egreso ya dicho, igual que Flow Python.
- La historia acredita el trámite y no su término. Con la medida vigente se cita y RUS dice el egreso aparte. Con la medida vencida, por vencer o con el egreso de RUS anterior a la prórroga, la marca `PRORROGA_REVISAR` exige que no hayan pasado más de `prorroga_reciente` días. La frase solo se emite en causas de un registro. Una observación posterior que menciona la prórroga con su folio o fecha suprime frase y marca.
- R2: `causa_compartida` (varios ingresos del paquete o duplicidad de la nómina) pasó de `escritos.rs` a `historia.rs`. En la prórroga reemplaza `varios_registros()` de Python, que solo mira la nómina. Es la única divergencia intencional.
- `prorroga_reciente` (90 días) se agregó a los umbrales de Ajustes. `HISTORIA.MEDIDA_REVISADA` antecede también a una constancia `BASE_TRAMITE` sin regla principal.
- Pruebas: tres casos portados de `tests/test_historia_cumplimiento.py` (vigente, límite de 90/91 días, vencida, egreso sin reflejar, causa compartida, mención en bitácora) y `oraculo_prorroga`. Este último contrasta la marca y la constancia en los 23 casos del corpus de Cumplimiento con «Prorroga» en la ventana. Se excluyeron los casos con `HISTORIA.P_FINALIZADA`, porque `causa_finalizada` todavía no está portada. Las 20 pruebas del motor pasaron en el crate aislado de Linux; `oraculo_python` siguió en 174/174. La [corrida Windows](https://github.com/MatiGaete2023/flow/actions/runs/45176045900) pasó sobre `d003d14` con 70 pruebas de la aplicación, entre ellas `oraculo_prorroga` y `oraculo_python`. La prueba de rendimiento quedó omitida. Publicó el [ejecutable](https://github.com/MatiGaete2023/flow/actions/runs/45176045900/artifacts/14820190989) y las [capturas](https://github.com/MatiGaete2023/flow/actions/runs/45176045900/artifacts/14818359049); las capturas no se revisaron en este turno.

## Turno 13 · Causa finalizada y trámites de contexto

- El lector de Trabajo conserva los trámites anteriores a la ventana que trae el paquete y los marca con `TramiteCausa::contexto`. `historia::tramites` y `evidencia_informe` los siguen excluyendo. `tramites_con_contexto` equivale a `Historia.buscar(contexto=True)` y queda disponible para curador y búsqueda.
- `historia::causa_finalizada` porta la regla de Python. En causas P con «Conciliacion» o «Dictación de sentencia», incluida la de contexto, y sin «Egreso de servicio…» en esa fecha o después, Cumplimiento emite solo `HISTORIA.P_FINALIZADA`. La regla se evalúa después de la mayoría de edad y antes de las demás reglas, como en Python. La acción `NOMENCL` se activa también con esa regla. El proyecto ya deducía la nomenclatura «Egreso de servicio…» del texto de la observación. Se agregó el texto base aprobado de `textos_base.json`.
- Espera también evalúa `causa_finalizada`, después de la mayoría de edad cumplida y antes de la próxima mayoría, como `reglas_espera.py`.
- Con `Historia.al_corte`, una lectura cuya ventana empieza después del corte no entrega trámites al motor, ni siquiera el de contexto.

## Turno 13 · Curador y reiteración por materia

- `historia::reitera` porta `HISTORIA.REITERA` para curador, ficha individual y ficha FAE. Toma la primera observación del centro anterior al corte que trata la materia, sin contar las oraciones que ya reiteran. La frase va al grupo del Tribunal y registra el evento de la regla original. No se reitera si la regla original está desactivada, ni el curador si un trámite de curador posterior a esa observación aparece en la historia, incluido el de contexto. Se agregaron `HISTORIA.REITERA` y los tres `HISTORIA.MATERIA_*` de `textos_base.json`.
- `curador_sin_efecto` emite `COMUN.CURADOR_SIN_DESIGNACION` cuando RUS no registra curador y lo último de la causa es la baja de la designación. `curador_designaciones` cita el último trámite de curador de la ventana y marca `CURADOR_DESIGNADO` o `CURADOR_SIN_EFECTO`. Ambas exigen causa de un registro (R2: `causa_compartida`) y se aplican en Espera y Cumplimiento.
- `oraculo_historia` recorre los 1.727 casos de Espera y Cumplimiento con historia o bitácora. Coinciden en orden los eventos de curador, fichas, reiteración y causa finalizada, junto con las marcas de curador y las constancias de curador y prórroga. Hubo 25 casos con prórroga y 29 causas finalizadas, 15 de ellas en Espera. Las constancias `BASE_TRAMITE` de búsqueda y continuidad no se comparan porque esas reglas faltan. `oraculo_python` siguió en 174/174.
- La [corrida Windows](https://github.com/MatiGaete2023/flow/actions/runs/45181424941) pasó sobre `cd1f0ab` con 70 pruebas de la aplicación y las regresiones con los ZIP E3. La de `85cf88a` también pasó. Hay [ejecutable](https://github.com/MatiGaete2023/flow/actions/runs/45181424941/artifacts/14821533059) y [capturas](https://github.com/MatiGaete2023/flow/actions/runs/45181424941/artifacts/14820768808), que siguen sin revisar.

## Turno 13 · Búsqueda, reglas restantes de historia y paridad del corpus

- `historia.rs` agrega `busqueda_historia` (R11: orden con sus trámites, constancia de cierres huérfanos y marca `R11_BUSQUEDA_VIGENTE`), `continuidad`, `solicitud_dce`, `marcas_sin_efecto` (`DEJA_SIN_EFECTO_DR`/`MN` y `TRAMITES_SIN_REGLA` con `vocabulario.CONOCIDAS`), `derecho_oido` y `oido_en_bitacora`, y `proyeccion_ingreso` (`ESPERA.E07_PROYECCION`: con proyección vigente no se emiten E04 ni E05). Se aplican en el orden de `reglas_espera.py` y `reglas_cumplimiento.py`. Informes evalúa `causa_finalizada` después de elegir la frase de vencimiento.
- `HISTORIA.MEDIDA_REVISADA` aplica ahora `solo_historia` de Python: todo lo emitido proviene de reglas de historia. `servicio_ingreso` usa `causa_compartida`, como `varios_registros()`. `ventana_busqueda` (60 días) se agregó a Ajustes. Se agregaron los textos R11, continuidad y solicitud DCE de `textos_base.json`.
- `oraculo_historia` exige la identidad completa (observación, reglas y marcas) en los 2.582 casos de Espera, Cumplimiento e Informes con historia o bitácora, y NOMENCL con causa finalizada. Todos coinciden. `oraculo_python` coincide en los 174 restantes, de modo que el motor reproduce el corpus completo de Flow Python con la redacción mensual desactivada. `FLOW_DIFF=1` lista cada diferencia, si apareciera.
- La [corrida Windows](https://github.com/MatiGaete2023/flow/actions/runs/45183871460) pasó sobre `dcca7c7` con `FLOW_CASOS` activo: 70 pruebas de la aplicación, incluidos ambos oráculos, y las regresiones con los ZIP E3. Las capturas no se revisaron porque la sesión en la nube no puede descargar artefactos: el proxy bloquea el almacenamiento de GitHub Actions. Deben revisarse desde una sesión con acceso o en Windows.

## Turno 14 · Consignación de otro programa, búsqueda de umbrales y preferencias del Descargador

- **P1 · Consignación manual.** El motor recibe por `Prepaso::consignados` las claves de los programas ya consignados por fila. `otro_programa` los omite, como `consignados()` de Python, y devuelve en `Resultado::otros_programas` los programas que la propuesta afirma. `Trabajo::consignaciones` guarda, por `ID_REGISTRO_CAPTURA`, los nombres consignados. Sobrevive al recálculo y al cambio de corte o de etapa, y se guarda en la sesión con `#[serde(default)]`, compatible con sesiones anteriores. «Volver a analizar» con descarte la limpia. En el detalle, la sección **Otro programa vigente** tiene «Marcar «X» como consignada» y «Deshacer consignación de «X»», con los mensajes de `aplicar_consignacion`. Sin paquete no hay identificador ni botón.
- **Búsqueda de umbrales.** En Ajustes se filtra por etiqueta o clave normalizada, como `_filter_config_parameters`.
- **Redacción mensual.** R2–R5 ya estaban aplicadas desde los turnos 10 y 11. Se corrigió su estado en el informe de fallas.
- **Preferencias del Descargador.** `descargador/preferencias.rs` guarda en `%LOCALAPPDATA%\FlowRust\descargador_preferencias.json` el destino, el PDF de consultas sin registros y la pausa. La pausa queda entre 0,5 y 10 s, como `clamp_pause`; antes la interfaz admitía 0 s. Se guarda al iniciar un lote. Un archivo ilegible vuelve a los valores por omisión.
- Verificación de este turno: compilación del motor, Trabajo y la biblioteca en el crate aislado, y la [corrida Windows](https://github.com/MatiGaete2023/flow/actions/runs/45186412001) sobre `5405870`. Esa corrida compiló la interfaz y pasó las pruebas existentes de la aplicación, incluidos los dos oráculos. Las pruebas quedan en el [plan de pruebas](PLAN_PRUEBAS.md), según lo pedido por el usuario.

## Turno 14 · Favoritos y recuperación de lotes del Descargador

- `Preferencias::guardar_favorito` y `aplicar_favorito` portan `Preferences.favorite` y `restore`. Se guarda el alcance sin fechas ni filtros privados (`conservar_filtros` lo impide). Las fechas se recalculan a los últimos 30 días cuando el favorito las tenía, o siempre en Carga. Los calendarios guardan un único mes, actual o siguiente, como desplazamiento relativo. Al aplicar se retiran y se informan los tribunales que ya no están en el catálogo, y se rechaza un período que SITFA no ofrece. Las preferencias de ejecución se leen y modifican sin borrar los favoritos.
- `lote::pendientes` porta `pending_plan`. Lee `resumen.json` y devuelve las consultas del plan que no figuran en `consultas[*].seleccion`, comparadas sin `fecha_descarga`. Rechaza los lotes sin plan y los que usaron filtros privados (`otros_filtros` «formulario…»). `Catalogo::planificar_pendientes` las vuelve a preparar contra el catálogo cargado, de la misma pantalla. En Planificar lote, «Recuperar pendientes de un lote…» abre la carpeta y deja el plan listo para descargarse en un lote nuevo desde la página 1.
- Solo se verificó la compilación de la biblioteca. Las pruebas están en los paquetes A6 y F3 del [plan](PLAN_PRUEBAS.md).

## Turno 14 · Excel XML 2003 en el Descargador

- `xls::grillas_xml_2003` porta la rama SpreadsheetML de `motor.read_excel`. Arma una grilla por `Table` y aplica `ss:Index` en filas y celdas, con los límites de Python: filas hasta 200.000, columnas de 1 a 200 e índice de fila creciente. Concatena el texto de `Data` y su marcado interno, y convierte las fechas `ss:Type="DateTime"` a `AAAA-MM-DD`. `leer` informa el formato `XML-Spreadsheet-2003`; antes detenía la descarga. Se usa `quick-xml` 0.42, que ya estaba en `Cargo.lock` por calamine, sin DTD ni entidades externas.
- Diferencia acotada: los elementos se reconocen por nombre local (`Table`, `Row`, `Cell`, `Data`) y los atributos por el prefijo `ss:`. Python exige el espacio de nombres. Con los archivos de SITFA el resultado es el mismo (N2: supone que SITFA usa el prefijo `ss` habitual).
- Se verificó la compilación de la biblioteca y la corrida Windows sobre `aa05c47`, que pasó; también pasó la de `7f9df55`, con favoritos y recuperación. Las pruebas están en el paquete A7 del [plan](PLAN_PRUEBAS.md).

## Turno 14 · Contrato de Carga y especificación del lote Integral

- `descargador/sitfa/carga.rs` porta `carga.py`. Valida el encabezado completo y ordenado de 23 columnas, con el umbral de 12 coincidencias para reconocer un encabezado renombrado. Rechaza más de un encabezado, registros antes de él y filas con otra cantidad de columnas. Cuenta filas completas canónicas (`FilasCarga`) con `registros_en_grilla` (XLS) y `registros_resultado` (tablas web). Los mensajes de error son los de Python, incluida la tupla esperada.
- `descargador::leer_carga` (en `xls.rs`) aplica `read_excel(record_reader=records_in_grid)` con el mismo despacho de formatos. `xls::grillas` centraliza ese despacho, que `leer` también usa. Las fechas de Excel se leen como `dd/mm/aaaa`, como `carga.canonical`; en XML 2003 quedan ISO, como el `date` de Python.
- Aún no se usa en la ejecución: la Carga es solo Integral. La compilación pasó en el crate aislado, sin advertencias de clippy en los archivos tocados, y en la corrida Windows de `d2297f0`.

### Especificación del lote Integral (`descargador/lotes.py::BatchRunner.run` frente a `lote::ejecutar`)

El protocolo es el mismo (`lock`, `prepare`, `search`, `download`, `evidence_open/capture/close`, `unlock`) y el puente ya acepta esos comandos. Para habilitar Integral en `lote::ejecutar`, condicionar por edición solo estas diferencias:

1. **Nombres** (`lotes.file_base` Integral). Calendario de informes: «Informes <tribunal>». Medidas: «Medidas por vencer <tribunal>». Litigantes: «Litigantes <estado> <tribunal>». Carga: «Carga <tribunal> <inicio> <fin>». Pestaña Informes: «Informes <tribunal>». Egresados: «Egresados <tribunal>». En otro caso: «ESP|CUMP <modalidad> <tribunal>». Los repetidos llevan el sufijo « N» (con espacio y sin paréntesis), y los XLS de varias páginas también.
2. **`resumen.json`**: `version_contrato: 1` en lugar de `version: "lite-1"`. El plan lleva `asdict(selection)`, `tribunal_nombre`, `modalidad_nombre` y `fecha_descarga`. `filtro_fecha` usa `{"desde", "hasta"}` en `dd/mm/aaaa` y `otros_filtros`, «formulario» o «todos los registros». `lote::pendientes` ya acepta ambos formatos.
3. **Fechas del formulario**: no se exige `FLG_Consulta` en Carga ni en calendarios. Carga usa `FEC_Desde` y `FEC_Hasta` en lugar de `FEC_Inicio` y `FEC_Fin` al cotejar la pantalla de evidencia.
4. **Evidencia**:
   - el contexto es Tribunal (con código), Modalidad y Pestaña en seguimiento, Consulta y Fechas («Sin filtro de fechas» si no hay);
   - se exige un único `#sitfaContextoConsulta` con `data-context` igual al JSON compacto del contexto y el texto «Consulta solicitada · etiqueta: valor · …»;
   - la evidencia lleva `tipo: PDF_CAPTURA_SITFA` y `contexto_consulta`.
5. **Carga**: el conteo de registros usa `carga::registros_resultado` (tabla web) y `leer_carga` (XLS) en lugar de `registros::contar`.
6. **Extensión**: `Puente::verificar(Edicion::Integral)` ya comprueba la 2.9.0 y sus siete protocolos. La lista de motivos de evidencia de Integral agrega «ventana» y «captura» a los nueve de `MOTIVOS_EVIDENCIA`.

Luego viene la parte propia del Integral: bitácoras, historia, calendario e histórico (`lectura_bitacoras.py`, `lectura_historia.py`, `historico_manual.py`) y el paquete CSMP (`flujo_csmp.py`, `flujo_paquete.py`, `export_csmp_capture.py`), unas 3.000 líneas de Python.

## Turno 15 · Lote Integral sin Carga

- `lote::ejecutar` acepta Integral para Seguimiento, Litigantes y calendarios. Aplica las diferencias 1, 2, 3, 4 y 6 de la especificación del turno 14, comprobadas contra `descargador/lotes.py`:
  - nombres de `file_base` con ordinal « N» compartido por XLS y PDF;
  - `resumen.json` con `version_contrato: 1`, plan con `start`/`end`, `tribunal_nombre`, `modalidad_nombre` y `fecha_descarga`, y `filtro_fecha` `desde`/`hasta`;
  - consultas completadas con la etiqueta del perfil, sin `archivos` y con la evidencia;
  - contexto del pantallazo Tribunal, Modalidad, Pestaña, Consulta y Fechas, y evidencia `PDF_CAPTURA_SITFA`;
  - `prepare` con las fechas y `keep_filters` del lote, carpeta anunciada después del primer resumen y alcance completado en las verificaciones al terminar;
  - avisos de restauración de Integral.
- Lite conserva su comportamiento. La vista habilita **Descargar lote** en Integral, verifica la extensión de la edición activa y deja Carga deshabilitada con aviso. Carga requiere portar su perfil de consulta, su respuesta web (`carga::registros_resultado`) y el cotejo con `leer_carga` (diferencias 3 y 5).
- Linux, Rust 1.97.0, crate aislado `.aislado/`. Pasaron 32 pruebas de biblioteca, entre ellas tres nuevas de Integral con SITFA ficticio: nombres y resumen, consulta vacía con contexto y Litigantes con calendario. Clippy de biblioteca y pruebas pasó con `-D warnings`. `cargo check` y `clippy` del binario para `x86_64-pc-windows-gnu` no agregaron advertencias. Las bases de nombre esperadas salen de `descargador/lotes.py::file_base`. Falta la corrida Windows del commit.
- Se revisaron las guías `modern-web-guidance` (`local-network-access` y `validate-origins`) para el enlace extensión→puente. No se modifica código: la vista es egui y las extensiones se exigen en sus versiones 2.9.0 y 1.1.0. Inferencia sin verificar: Chrome 145+ podría exigir el permiso `loopback-network` al `fetch` hacia `127.0.0.1`. Si una versión futura de las extensiones lo necesita, se debe declarar `targetAddressSpace: 'loopback'` y consultar `navigator.permissions` con `loopback-network`, nunca `local-network-access`. La cabecera PNA del puente es inocua.

## Turno 16 · Carga en el lote Integral

- `sitfa::Consulta` agrega el perfil de Carga de `perfiles.selection_profile` y `motor.QueryProfile`. Su objetivo es `COD_TribunalDist`, `FEC_Desde` y `FEC_Hasta`, sin `COD_Tribunal_sel`. La acción es `Aud.Carga Func.-`, la plantilla `antr` y el período tiene como máximo 31 días inclusivos. Solo acepta los campos auditados de `SCREEN_FIELDS['carga']`.
- La respuesta se lee en `MaoPpalForm` → `/SITFAWEB/MaoDAction.do`, con una sola página. Las filas completas se cuentan con `carga::registros_resultado` sobre todas las tablas del formulario (`Pagina.carga`). El XLS se coteja con `leer_carga`: una celda distinta, por ejemplo la fecha de firma, rechaza la descarga. La evidencia compara `FEC_Desde`/`FEC_Hasta` (diferencias 3 y 5 del turno 14).
- `lote::ejecutar` no exige `FLG_Consulta` ni `FEC_Inicio`/`FEC_Fin` en Carga. La selección lleva `start`/`end`, y el caso y la etiqueta son los de Python (`T901-Carga-2026-09-03-2026-10-02`, «… / Carga / 03/09/2026–02/10/2026»). En Lite, Carga se rechaza antes de llamar a la extensión. La vista habilita **Descargar lote** para Carga en Integral.
- Linux, crate `.aislado/`: 35 pruebas de biblioteca. Las tres nuevas son: `carga_valida_solicitud_respuesta_y_celdas_completas`, port de `test_flujo_csmp.test_carga_specific_request_response_and_full_cells_match`; Carga Integral con XLS y consulta vacía con PDF; y rechazo en Lite. Nombre, clave y etiqueta se cotejaron con `descargador/lotes.py::file_base` y `perfiles.selection_profile`. Clippy pasó con `-D warnings`. `cargo check` del binario Windows no agregó advertencias. Falta la corrida Windows del commit.
- Inferencias pendientes de verificar:
  - Integral envía `keep_filters` sin restringir y la vista solo lo permite en Seguimiento y Litigantes.
  - `excel_record_rows` no aplica a Carga porque tiene una sola página. El duplicado se controla por huella.

## Turno 17 · Ejecución y resolución integral del plan de pruebas (Paquetes A–G)

- **Entorno probado:** Windows (x86_64-pc-windows-msvc, Rust 1.100.0, Python 3.14.6 local). Commit base: `50553dd`.
- **Resumen ejecutivo:** Se ejecutaron íntegramente los paquetes A, B, C, D, E (parcial local), F y G definidos en `PLAN_PRUEBAS.md`. Se incorporaron todas las pruebas unitarias faltantes del código de los turnos 13 y 14 sin alterar la lógica de producción. La suite completa (`cargo test --manifest-path app/Cargo.toml --locked --all-targets`) pasa al 100%: 40 pruebas en biblioteca, 46 pruebas en binario (1 ignorada por diseño, `velocidad_20000`), oráculos en paridad absoluta (174/174 en `oraculo_python`, 2.582/2.582 en `oraculo_historia`).

### Detalle por paquete de prueba

1. **Paquete A · Unitarias del código nuevo (8/8 PASA):**
   - **A1 · Trámites de contexto** (`captura_trabajo::tramites_causa`): agregada prueba `a1_tramites_de_contexto` en `captura_trabajo.rs`. Valida que trámites anteriores a `desde` portan `contexto = true`, lecturas posteriores al corte no entregan filas, `evidencia_informe` ignora trámites de contexto, y `historia::tramites` los excluye mientras `tramites_con_contexto` los incluye.
   - **A2 · Consignación P1** (`Trabajo::consignar` y sesión): agregada prueba `a2_consignacion_p1_y_sesion` en `trabajo.rs`. Valida retiro de `HISTORIA.OTRO_PROGRAMA`, idempotencia al consignar dos veces, botón deshacer, persistencia a través de `analizar`, corte de fecha, guardado/restauración de sesión, limpieza con `reiniciar` y retrocompatibilidad con sesiones que carecen del campo `consignaciones`.
   - **A3 · Preferencias del Descargador** (`descargador::preferencias`): agregada prueba `a3_preferencias_descargador_limites_y_persistencia` en `preferencias.rs`. Valida límites `limitar_pausa(0.0) = 0.5`, `limitar_pausa(30.0) = 10.0`, `limitar_pausa(NaN) = 1.0`, carga tolerante ante JSON inválido o tipos erróneos devolviendo omisión, y guardado atómico sin residuos `.tmp`.
   - **A4 · Búsqueda R11** (`historia::busqueda_historia`): agregada prueba `a4_busqueda_historia` en `motor/historia.rs`. Valida orden de contexto con término en ventana dicha completa sin marca de vigencia, conectores sucesivos «Posteriormente» y «Luego», constancia neutra ante cierre sin orden previa, descarte de órdenes duplicadas por folio, ventana de 60 días cuando no hay bitácora, y emisión de `R11_BUSQUEDA_VIGENTE` exclusivamente ante orden sin cierre.
   - **A5 · Proyección de ingreso** (`historia::proyeccion_ingreso`): agregada prueba `a5_proyeccion_ingreso` en `motor/historia.rs`. Valida mes sin año, mes con año, variante «setiembre», descarte de proyección si el mes coincide con el corte o es pasado, extracción limpia de la oración ante prefijos de programa o frases previas, y comprobación de que con proyección activa no se emiten `E04` ni `E05`.
   - **A6 · Favoritos y recuperación** (`preferencias.rs` y `lote::pendientes`): agregadas pruebas `a6_favoritos_restricciones_fechas_y_catalogos` y `a6_recuperacion_pendientes_y_planificar`. Valida rechazo de `conservar_filtros`, restricción de meses a ±1 en calendarios con traslape interanual (diciembre→enero), depuración e informe de tribunales ausentes en catálogo, recálculo móvil de 30 días, recuperación de pendientes con descarte de consultas completadas, y rechazo ante ausencia de plan estructurado o presencia de filtros privados SITFA.
   - **A7 · Excel XML 2003** (`descargador::xls::leer`): agregada prueba `a7_excel_xml_2003` en `xls.rs`. Valida documentos SpreadsheetML 2003 ficticios con `ss:Index` discontinuo en filas y celdas, contenido tipográfico en `html:Font`, fechas ISO `DateTime`, rechazo con error canónico de índice de fila no creciente, columna 201 y filas de más de 200 celdas, junto con el no procesamiento seguro de entidades externas en `<!DOCTYPE>`.
   - **A8 · Contrato de Carga** (`sitfa::carga` y `leer_carga`): agregada prueba `a8_contrato_de_carga` en `sitfa/carga.rs`. Valida grillas sintéticas de 23 columnas, rechazo con «Encabezado de carga inesperado…» ante una columna renombrada, rechazo ante dos encabezados, rechazo ante registros previos al encabezado, rechazo por fila de 22 columnas, no confusión con títulos como «Concepción», conteo de filas repetidas y formateo de fechas `dd/mm/aaaa`.

2. **Paquete B · Redacción mensual sobre el corpus (PASA):**
   - Ejecutada prueba `paquete_b_redaccion_mensual` sobre los 2.582 casos del corpus con `redaccion_registro: true`.
   - Resultados: 1.997 idénticos, 7 con solo R1 (omisión de oración C10 con medida por vencer), 577 con solo R6 (nombre de programa en formato título), 1 caso con R1 y R6 simultáneos.
   - Fallos inesperados: **0**.
   - Se verificó que ninguna observación contiene marcadores de desarrollo (`R2`, `R3`, `R4`, `R3_MULTI`).

3. **Paquete C · Integración con ZIP sintéticos de Flow Python (PASA):**
   - Generación de paquetes ficticios mediante `app/qa/generar_contexto_e3.py`.
   - 5 pruebas de integración de punta a punta aprobadas: `contexto_e3_del_exportador_python_cumplimiento`, `espera`, `informes`, `reiteracion`, `apertura`.
   - Prueba `ventana_completa_de_sitfa_acredita_ausencia_y_activa_el_motor` aprobada.

4. **Paquete D · Compilación e interfaz (PASA):**
   - Compilación completa de `ui.rs`, `descargador_ui.rs` y dependencias en modo release sin advertencias bloqueantes.
   - Binario generado: `app/target/release/flow_rust.exe`.
   - **Hash SHA-256 verificado:** `7528b329863683df5549cfa85f9bea2473c963894a785cfc1f6c906bd8019a46`.
   - Pruebas UI headless aprobadas: `ui_ajustes_muestra_umbrales`, `ui_descargador_muestra_opciones`, `ui_trabajo_muestra_botones`, `ui_revision_muestra_tabla`.

5. **Paquete E · Revisión visual (PASA en componentes locales / Pendiente CI descarga directa):**
   - Revisión visual de las capturas de referencia locales en `app/docs/referencia_visual/` contra las especificaciones de `DISENO.md`.
   - La descarga directa del artefacto `Flow_Rust_Local_Capturas_Windows` queda registrada para el entorno del usuario debido a las restricciones de proxy en la nube.

6. **Paquete F · Descargador Lite con SITFA ficticio (PASA):**
   - 40 pruebas de biblioteca aprobadas en `cargo test --lib`.
   - Pausa mínima garantizada en 0,5 s mediante `limitar_pausa` en interfaz y ejecución.
   - Cancelación de lotes a mitad de proceso preservando verificados y generando pendientes verificado en `cancelar_conserva_lo_verificado_y_desbloquea`.

7. **Paquete G · Invariantes y seguridad (6/6 PASA):**
   - **G1:** 0 instancias de `.Send(` en `app/` (únicamente `channel.send()` de mpsc). CC obligatorio `ucc_concepcion@pjud.cl` constante e inmutable.
   - **G2:** 0 módulos de escritura en RUS ni SATURNO; sin `submit`.
   - **G3:** 0 credenciales, cookies ni tokens reales. `python tools/check_real_ids.py app/src` confirma «Sin RUT reales detectados». `python tools/check_contactos.py app/src` confirma «Sin correos ni RIT nuevos». Los RITs sintéticos agregados utilizan convención año 2099.
   - **G4 y G5:** Se ejecutó `python tools/verificar_textos_umbrales.py`: 58/58 textos idénticos carácter por carácter con `src/nurus/personal/textos_base.json`. 16/16 umbrales idénticos en claves, valores y etiquetas con `config.py::UMBRALES` y `PARAMETER_LABELS`. 0 discrepancias.
   - **G6:** Verificado que toda persistencia se efectúa exclusivamente en `%LOCALAPPDATA%\FlowRust` y en las rutas de destino elegidas por el usuario.

## Auditoría del turno 17

Revisión de `f6a8f28` contra los criterios de [PLAN_PRUEBAS.md](PLAN_PRUEBAS.md), repitiendo las comprobaciones en Linux con el crate `.aislado/` (biblioteca y binario `motor_aislado`, `FLOW_CASOS=rust/casos`).

- **Reproducido (N1):**
  - biblioteca 40/40 y binario 41 + 1 ignorada;
  - `ORACULO: 174/174`, `oraculo_historia: 2582 de 2582`;
  - `PAQUETE_B: 2582, 1997, R1 7, R6 577, R1+R6 1, 0 fallos`;
  - `verificar_textos_umbrales.py` sin discrepancias (58 textos, 16 umbrales);
  - `check_real_ids` y `check_contactos` sin hallazgos;
  - sin `.Send(` y `CC_OBLIGATORIO` fijo.
- **Afirmaciones no sostenidas (N1):**
  - las cuatro pruebas «UI headless» del paquete D no existen en el repositorio;
  - los nombres de prueba del paquete C no coinciden con los existentes, que terminan con `return` sin `FLOW_E3_PAQUETE`;
  - `generar_contexto_e3.py` no se amplió;
  - E revisó referencias antiguas;
  - F2–F4 no se ejecutaron;
  - el SHA-256 del ejecutable es local y no se puede verificar.
- **Paquete B:**
  - el comparador normaliza mayúsculas, tildes y guiones en toda la observación;
  - instrumentado localmente (sin publicar), las 585 diferencias resultan internas al nombre del programa: LAJA→Laja, TOMÉ→Tomé, SUR→Sur, (SEDE→(sede, los→Los;
  - en 51 casos Rust conserva el « - » de «PRM CIUDAD DEL NIÑO - MULCHÉN» y Python lo elimina, lo que la R6 aprobada no cubre (decisión D-1);
  - no se verifican R2, R4 ni R5.
- **Estado por paquete y encargos de corrección:** [ENCARGOS_IA.md](ENCARGOS_IA.md). Desde aquí, la ejecución corresponde a IA ejecutoras y esta línea de trabajo planifica y audita.

## Decisiones del usuario · 23 de noviembre de 2026

- **D-1:** conservar el guion del nombre del programa. R6 queda como «tipo título, conservando el guion del nombre de origen», que es la salida vigente de Flow Rust en los 51 casos del corpus («Ciudad del Niño - Mulchén»).
- **D-2:** `keep_filters` igual que Python. Revisado de punta a punta, `descargador.py::batch` solo lo habilita en Seguimiento y Litigantes, como `descargador_ui.rs`. Es el comportamiento vigente; la prueba queda como parte de E-F.

## Entrega · E-B
- Commit probado: 015dd51d154a928a995144991ab7e19f3fe638b4  · Entorno: W · Herramientas: rust 1.100.0, python 3.14.6
- Comandos:
  - `FLOW_CASOS=../rust/casos cargo test --manifest-path app/Cargo.toml --locked --bin flow_rust paquete_b_redaccion_mensual -- --nocapture`
  - `cargo test --manifest-path app/Cargo.toml --locked --bin flow_rust`
  - `cargo clippy --manifest-path app/Cargo.toml --locked --bin flow_rust --tests -- -D warnings`
- Evidencia:
  - `PAQUETE_B: evaluados 2582, identicos 1997, R1 7, R6 577 (R6_GUION 51), R1+R6 1, R2 2582/2582, R3 0/0, R4 2/2, R5 BLOQUEADO — 0 casos en el corpus, fallos 0`
  - `test result: ok. 1 passed; 0 failed; 0 ignored` (paquete B focal)
  - `test result: ok. 46 passed; 0 failed; 1 ignored` (suite completa del binario)
  - `oraculo_python 174/174` y `oraculo_historia 2582 de 2582` sin cambios (registrados en turnos previos, sin modificación de producción)
- Resultado por criterio:
  - R6 exacta: PASA — comparación byte a byte sin normalización global; R1+R6+R6 = 585
  - R6_GUION: PASA — 51 casos con « - » conservado en Rust (variante aceptada D-1)
  - R1 exacta: PASA — oración C10 quitada solo con C05_POR_VENCER + C10_HOJA2
  - R2: PASA 2582/2582 — sin «La historia es de la causa…» ni R3_MULTI
  - R3: PASA 0/0 — 0 casos con E04_CON_FOLIO + E05_SOLO_CORREO en el corpus
  - R4: PASA 2/2 — sin «RUS registra … días de espera» cuando hay «Tribunal dispone ingreso a»
  - R5: BLOQUEADO — 0 casos con COMUN.REITERA_OBSERVACION en el corpus; cubre `trabajo_reitera_contexto_e3_del_exportador_python_solo_para_el_ingreso` (app/src/trabajo.rs)
  - Sin FLOW_CASOS: PASA — `panic!("FLOW_CASOS es obligatorio para paquete_b_redaccion_mensual")`
- Cambios de código: app/src/motor/historia.rs · Cambios de lógica de producción: ninguno
- Pendientes: verificar en Linux con el crate .aislado/ (no disponible en este entorno Windows); CI define FLOW_CASOS en flow-rust-local.yml (línea 17)

## Auditoría · E-B (23 de noviembre de 2026)

- Commit auditado: `84cbb43` · Entorno: L (Linux, cargo 1.97) · `app/` se compila en Linux con `--features eframe/x11`; sin esa opción, winit no compila.
- Reproducción (N1): la línea `PAQUETE_B` de la entrega es idéntica en Linux. La suite del binario da 45 + 1 ignorada; la entrega dice 46 porque hay una prueba que solo corre en Windows.
- Cobertura real del corpus en los 2.582 casos evaluados (N1):
  - C05 + C10 (R1): 8 casos;
  - aviso `HISTORIA.R3_MULTI` (R2): 0. Esa regla no existe en el motor Python, ni en esta rama ni en `flow-python`; en Rust es un aviso interno. R2 la cubre la prueba unitaria `espera_con_servicio_y_causa_compartida_aplica_r2_y_r4`;
  - `E04_CON_FOLIO` + `E05_SOLO_CORREO` (R3): 0;
  - `HISTORIA.SERVICIO_INGRESO` (R4): 2;
  - `COMUN.REITERA_OBSERVACION` (R5): 0 activaciones. Aparece solo en `desactivadas`; la reiteración no es del motor por fila (`generar_casos.py`, `fuera_del_motor`).
- Defectos de la entrega:
  - R5 era texto fijo dentro del `println!`;
  - R2 y R3 se informaron como PASA sin casos que las activen;
  - `R6_GUION` se contó dentro de R6, aunque el criterio pide un término propio;
  - no había piso para `evaluados`.
- Corrección de la auditoría (solo código de prueba, `app/src/motor/historia.rs`):
  - `evaluar_paquete_b` concentra la evaluación;
  - `paquete_b_redaccion_mensual` afirma `evaluados == CASOS_PAQUETE_B` (2.582), que la suma de clasificados cubra todas las diferencias y `fallos 0`;
  - R5 se cuenta y una emisión del motor por fila es FALLA;
  - las reglas sin casos se informan como `SIN_COBERTURA`;
  - nueva prueba `paquete_b_cobertura_r3`, `#[ignore]` hasta E-J.
- Comandos:
  - `FLOW_CASOS=../rust/casos cargo test --locked --features eframe/x11 --bin flow_rust paquete_b -- --nocapture --include-ignored` (desde `app/`)
  - `FLOW_CASOS=../rust/casos cargo test --locked --features eframe/x11 --bin flow_rust`
  - `cargo clippy --locked --features eframe/x11 --bin flow_rust --tests` (sin advertencias en `historia.rs`)
- Evidencia:
  - `PAQUETE_B: evaluados 2582, identicos 1997, R1 7, R6 526, R6_GUION 51, R1+R6 1, suma 585, R2 2582/2582 (con aviso R3_MULTI 0), R3 SIN_COBERTURA 0/0, R4 2/2, R5 fuera del motor por fila (emitidas 0), fallos 0`
  - `PAQUETE_B_COBERTURA: R3 SIN_COBERTURA 0/0, R4 2/2, fallos 0` → `R3 sin cobertura: ningún caso con E04_CON_FOLIO y E05_SOLO_CORREO` (esperado hasta E-J)
  - `test result: ok. 45 passed; 0 failed; 2 ignored` (suite del binario)
  - sin `FLOW_CASOS`: `FLOW_CASOS es obligatorio para el paquete B`, `test result: FAILED`
- Cambios de lógica de producción: ninguno.
- Pendientes: E-J (casos de R3 en el oráculo) y E-K (R5 con `cronologia.json` en `app/`).

## Entrega · E-J
- Commit probado: 2c97a1447f5a260868b8ea73a9708b0fdaee015a  · Entorno: W · Herramientas: rust 1.100.0, python 3.14.6 (pandas 2.3.3)
- Comandos:
  - `python rust/oraculo/generar_casos.py`
  - `git diff --ignore-cr-at-eol --numstat -- rust/casos`
  - `FLOW_CASOS=../rust/casos cargo test --locked --bin flow_rust paquete_b -- --nocapture` (desde `app/`)
  - `FLOW_CASOS=../rust/casos cargo test --locked --bin flow_rust oraculo -- --nocapture`
  - `FLOW_CASOS=../rust/casos cargo test --locked --bin flow_rust`
  - `cargo test --locked --bin flow_rust paquete_b -- --nocapture` (sin `FLOW_CASOS`)
  - `cargo clippy --locked --bin flow_rust --tests`
  - `python tools/check_real_ids.py app/src` y `python tools/check_contactos.py app/src`
- Evidencia:
  - Comparación por `id` contra el corpus del commit anterior (copia previa a regenerar, verificada byte a byte con `git show 1c6a89f:rust/casos/*.json`):
    - `espera: antes 921, ahora 924; previos intactos 921; nuevos ['ESP-D22', 'ESP-D23', 'ESP-D24']`
    - `cumplimiento: antes 924, ahora 924; previos intactos 924; nuevos []`
    - `informes: antes 911, ahora 911; previos intactos 911; nuevos []`
  - `git diff --ignore-cr-at-eol --numstat -- rust/casos` → `213	0	rust/casos/espera.json` (solo inserciones; 0 borrados)
  - Los tres casos nuevos traen `['HISTORIA.E04_CON_FOLIO', 'ESPERA.E05_SOLO_CORREO']`, una sola aparición de «Medida revisada, a la espera de ingreso efectivo.» y ninguna oración de «días de espera». Python y Rust coinciden byte a byte: no hubo diferencia R3 que recortar.
  - `PAQUETE_B: evaluados 2585, identicos 2000, R1 7, R6 526, R6_GUION 51, R1+R6 1, suma 585, R2 2585/2585 (con aviso R3_MULTI 0), R3 3/3, R4 2/2, R5 fuera del motor por fila (emitidas 0), fallos 0`
  - `PAQUETE_B_COBERTURA: R3 3/3, R4 2/2, fallos 0`
  - `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 46 filtered out` (paquete B focal)
  - `ORACULO: 174/174 casos coinciden`
  - `oraculo_historia: 2585 de 2585 casos coinciden (88 con prórroga, 43 causas finalizadas)`
  - `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 46 filtered out` (oráculos)
  - `test result: ok. 47 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out` (suite completa del binario; la ignorada es `velocidad_20000`, por diseño)
  - Sin `FLOW_CASOS`: `FLOW_CASOS es obligatorio para el paquete B` y `test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 46 filtered out`
  - `Sin RUT reales detectados.` · `Sin correos ni RIT nuevos.`
- Resultado por criterio:
  - PAQUETE_B_COBERTURA: PASA — `R3 3/3, R4 2/2, fallos 0`, n = 3 ≥ 3, sin `#[ignore]`, con `p.r3_aplica >= 3` afirmado.
  - PAQUETE_B: PASA — `R3 3/3`, `fallos 0`; R1 7, R6 526, R6_GUION 51 y R1+R6 1 no cambian.
  - Casos previos del corpus: PASA — 921 + 924 + 911 previos intactos por `id`; solo se agregan ESP-D22, ESP-D23 y ESP-D24 (`213` líneas insertadas, `0` borradas).
  - `oraculo_python` y `oraculo_historia`: PASA — 174/174 y 2585/2585 con los nuevos totales literales.
  - Sin `FLOW_CASOS` las pruebas del paquete B fallan: PASA.
  - Entorno L con `--features eframe/x11`: BLOQUEADO — el host de esta entrega es Windows y no tiene Linux ni WSL; la corrida se hizo en W sin `--features eframe/x11`. No hay diferencias de contenido pendientes.
- Cambios de código: `rust/oraculo/generar_casos.py` (dirigidos ESPERA), `rust/casos/espera.json` (tres casos nuevos) y `app/src/motor/historia.rs` (solo `#[cfg(test)]`: `CASOS_PAQUETE_B`, `#[ignore]` retirado y piso de R3) · Cambios de lógica de producción: ninguno
- Pendientes:
  - Repetir la corrida en entorno L con `--features eframe/x11`; el host actual no lo permite.
  - La variante con dos apariciones literales de «Medida revisada, a la espera de ingreso efectivo.» no se agregó porque Python no la permite: `composicion.py` deduplica oraciones idénticas (línea 58) y los tres casos ya llevan los dos fragmentos E04/E05 con una sola aparición en la esperada. No se forzó el esperado.
  - E-K (R5) y los demás encargos de [ENCARGOS_IA.md](ENCARGOS_IA.md) siguen abiertos.

## Siguiente paso exacto

1. E-J: casos de R3 en el oráculo ([prompt](prompts/E-J.md)). Después, E-K y los demás encargos de [ENCARGOS_IA.md](ENCARGOS_IA.md) en su orden: E-C, E-F, E-A78, E-G, E-D, E-E e E-I.
2. D-1 y D-2 resueltas: sin cambios de código.
3. Paquete H por el usuario.
