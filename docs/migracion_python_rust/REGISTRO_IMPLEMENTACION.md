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

## Siguiente paso exacto

1. Revisar la corrida Windows de `864164a` y descargar `Flow_Rust_Windows_exe` para la prueba del usuario: Excel desde ZIP, marcas, «Instalar extensión…», contactos importados, nómina editable, Word con matriz y redacción R1/R6.
2. E3 (punto 1), en este orden:
   1. Exponer en `captura.rs` las tablas `historia_tramites`, `bitacoras`, `calendario_ingreso`, `escritos` e `historico_proteccion`.
   2. Construir el índice por identidad en `captura_trabajo.rs`.
   3. Portar `chronology`, `captured_rule_context`, `escritos_para` e `historico_para` de `capture_input.py`, y las reglas `HISTORIA.*` y `COMUN.REITERA_OBSERVACION` del motor de Flow Python.
   4. Aplicar entonces R2, R3, R4 (quitar días con tribunal que dispuso el ingreso) y R5 parcial, con casos ficticios por regla.
3. Punto 5: inventario de opciones de Flow Python contra `ui.rs` en una tabla de la matriz. Agregar `prorroga_reciente` y `ventana_busqueda` con E3.
4. Pendientes del Descargador (E2): lotes Integral, Carga, XML 2003, preferencias y reanudación.
