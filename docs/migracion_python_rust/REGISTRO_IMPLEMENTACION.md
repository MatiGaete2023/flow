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

## Siguiente paso exacto

1. Completar selección masiva desde la revisión y persistencia del período si corresponde a la referencia. Ampliar la prueba de V1 contra un ZIP generado por el productor histórico y trasladar los casos de combinaciones de instantáneas de `rust/crates/flow-trabajo/tests/revision_bitacoras.rs` al contrato completo.
2. Continuar E2 por perfiles del Descargador integral y Lite. Comparar `descargador/perfiles.py` y `descargador_lite/app/perfiles.py` con `rust/crates/flow-captura/src/sitfa/perfil.rs`. Usar `test_solo_cuatro_consultas_rus` y `test_plan_exige_filtros_de_cada_pestana` como primeras regresiones. Incorporar el flujo a los componentes gráficos de `app/`. E3 incorpora historia y calendario al motor de Trabajo; E4 aborda la conciliación entre capturas y la copia del consolidado.
