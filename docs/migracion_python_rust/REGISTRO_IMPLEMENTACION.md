# Registro de implementación Python → Rust

Rama `estudio/paridad-python-rust-20261119`. Referencia Python fijada en `93a36312adbd24e2dc96b02dfac45d947193b8b1`. Punto de partida de esta sesión: `740a64b905eca207e83e8bd27f76bd5513bfaaa9`. El [plan](PLAN.md) mantiene el orden y los criterios de aceptación.

## Estado al cierre de esta sesión

| Entrega | Estado |
|---|---|
| E0 · Línea base | Confirmadas la punta de la rama y las huellas de los fuentes editados antes de intervenir. Cargo no resolvió el árbol completo por falta de dependencias locales. La comparación completa de suites y delta exclusivo de `rust/producto` sigue pendiente. |
| E1 · F06 y lectura de paquetes Python | Implementado el lector de revisión V2, con regresiones contra V1 y legado verificadas en una compilación aislada. Falta la prueba con el exportador del Descargador desde un lote y la comprobación del ejecutable con los productos de exportación. |
| E2–E7 | Pendientes. |

## Cambios de E1

- `rust/crates/flow-trabajo/src/paquete.rs` conserva la lógica de V1 y añade lectura V2. V2 valida método, estado, intervalo ISO, coherencia de captura, tipos de los campos y advertencias. Usa la tabla normalizada de bitácoras para los eventos y la consulta para acreditar cobertura. La asociación exige coincidencia de consulta e identidad RUS/tribunal/causa/ingreso, más autoría acreditada para contar una observación del centro. Ausencia requiere tabla disponible, intervalo cubierto y evidencia coherente. Varias consultas que compiten por un mismo ingreso V2 se consideran contradictorias, porque el productor Python consolida una lectura por ingreso antes de exportar.
- `periodo_revision` reconoce el intervalo V2, que la pantalla permite ajustar. La ficha de evidencia muestra el rango y alcance informado para ese formato. Los eventos V2 quedan en el informe por separado y la exportación JSON existente limita esos eventos al período seleccionado.
- `rust/crates/flow-trabajo/tests/revision_bitacoras.rs` añade dos pruebas sobre paquetes con tablas y huellas. Recrean los cuatro casos de [CASOS_PRIMERA_ENTREGA.json](CASOS_PRIMERA_ENTREGA.json) y ejercitan cambio de período, serialización del informe, advertencias, intervalos, versión desconocida, autoría, ingreso ajeno, lectura vacía, contradicción y ausencia de tabla.

## Verificación ejecutada

1. Reproduje el defecto con el lector de la base y los dos tests nuevos. Ambos fallaron con `NoVerificable` donde la entrada acreditaba `ConObservacion`.
2. Después del cambio pasaron 21 pruebas en Windows con Rust 1.100.0. La compilación aislada usó el lector ZIP, el adaptador y el clasificador del repositorio, con la parte de informe que cambia período y se serializa. Se excluyó el test dependiente de aplicar el motor completo. Incluyó 13 regresiones existentes sobre paquetes, cinco del clasificador, dos V2 y un contraste con un ZIP escrito por el contrato Python.
3. El ZIP de contraste se construyó con `nurus.capture.write_capture` y valores V2 ficticios. El clasificador Python y el lector Rust aislado coincidieron en los cuatro resultados. El ensayo no recorrió el lector HTML ni el exportador integral; la publicación del temporal de prueba usó una copia local debido al bloqueo de enlaces duros del entorno Windows.
4. `cargo test --locked --offline -p flow-trabajo --test revision_bitacoras` desde el workspace se detuvo al resolver `chrono-tz`. La recuperación en línea se detuvo por DNS de `index.crates.io`. La UI y la producción de XLSX quedaron pendientes de verificar en el ejecutable integrado.

## Siguiente acción exacta

En un entorno con las dependencias del [lock](VERIFICACION.md) instaladas:

1. Desde la raíz del checkout, ejecutar `cargo test --manifest-path rust/Cargo.toml --locked -p flow-trabajo --test revision_bitacoras` y `cargo test --manifest-path rust/Cargo.toml --locked --workspace`. El workflow `rust.yml` de partida no se activa automáticamente con pushes a esta rama.
2. Generar el ZIP con el Descargador Python desde las funciones `diaries`, `_paginar` y `_trabajo` de `descargador/test_revision_bitacoras_p2.py`, usando el lote ficticio de dos ingresos. Ejecutar también esa prueba Python. Abrir ese ZIP en el ejecutable Rust desde **Revisión de bitácoras**, fijar los períodos de [CASOS_PRIMERA_ENTREGA.json](CASOS_PRIMERA_ENTREGA.json), comprobar los cuatro resultados y exportar Excel/JSON. Verificar conteos y verificar que reabrir el mismo ZIP reproduce el resultado.
3. Con el camino de importación comprobado, empezar E2 F17 comparando perfiles y consultas del Descargador integral y Lite con `flow-captura/src/sitfa`. Mantener el diálogo de avance en este registro, con el SHA probado y los resultados reales. La interoperabilidad de los ZIP **producidos por Rust** pertenece a E2.

Los comandos, fixtures y salidas de prueba usan solo datos ficticios. La aceptación en el puesto institucional queda a cargo del usuario.


## Continuación · verificación y primeras diferencias de captura

Base de código de esta sesión: `42a86b69f12a0e7080983509333beb241fea2450`.

- **E0.** La rama ya dispara el workflow Rust. En la primera ejecución, el paso `cargo test --workspace --locked` terminó correctamente en Linux. Los demás jobs se registran por separado. La comparación de GitHub con `rust/producto` arrojó seis commits exclusivos y once archivos en el delta. Incluye diferencias en la revisión de bitácoras, la pantalla y el verificador. Es un inventario de alcance, no evidencia de que correspondan portarlos.
- **E1 / F06.** Se agregó `rust/pruebas/revision_python.py`. Reconstruye un lote ficticio con el exportador original, ejecuta el lector y el exportador Rust que usa la ventana y compara las cuatro clasificaciones con los dos productos. Se programó en GitHub Actions con el lock Python y la regresión original P2. El ejemplo Cargo pertenece a pruebas de desarrollo.
- **E2 / F17–F18.** Seguimiento, Litigantes, Informes por vencer y Medidas por vencer ya tienen perfiles y pantalla. El plan del lote ya valida el catálogo, opciones y fechas. El lote conserva resultados, permite cancelar y retomar, y admite PDF de consulta vacía. Se alineó el tope a 5.000 aperturas compartidas entre bitácoras y calendarios, y 5.000 historias aparte. La prueba del contador cubre 4.999, 5.000 y el rechazo de 5.001. La detención por cinco bitácoras consecutivas fallidas está cubierta por el ensayo de lotes preexistente.
- **E2 / F18.** Se incorporó la pausa configurable entre consultas en Avanzado, de 500 a 10.000 ms y 1.000 ms al iniciar. Se aplica a los planes de descarga y paquete. El valor queda en los planes de lote y se puede ajustar durante la sesión de la aplicación. La preferencia entre reinicios, los favoritos y la pausa manual siguen pendientes.
- **E3 / F01–F05.** En Windows se compilaron `flow-dominio` y `flow-motor` completos en un workspace de dos crates con dependencias en caché. Pasaron 6 pruebas de dominio, 5 del motor y las 4 pruebas del oráculo versionado, que reúnen 3.699 casos. Esta corrida reducida verifica el motor y su corpus. Las asociaciones multifila y los flujos de captura y escritorio requieren sus pruebas respectivas.

**Próximo cierre.** Comprobar el workflow de esta revisión, en especial «Paridad con el Descargador Python en lotes ficticios». Si falla, distinguir instalación de una diferencia de clasificación, estructura o salida. Luego continuar E2 por histórico opcional, contador compartido con esa lectura y validación con las estructuras ficticias del productor Python. Después siguen Escritos y el contrato de exportación. E4 a E7 siguen en el orden y con los criterios del plan.

## Comprobación cruzada y respaldo de Histórico

- GitHub Actions ejecutó correctamente el workspace Rust y el paso completo «Paridad con el Descargador Python en lotes ficticios» en [44647521936](https://github.com/MatiGaete2023/flow/actions/runs/44647521936), sobre `4242086851aa6147671e2c84c77b9a87baced00b`. Ese paso instala el lock, ejecuta P2 y Lite, y confirma los cuatro casos en el ZIP del productor Python, el JSON Rust y el XLSX Rust. La prueba usa el lector y exportador de la ventana. La comprobación visual y la entrega Windows se siguen en los jobs específicos.
- **E2 / F19 y F22.** El control de Histórico manual ahora distingue un manifiesto inexistente de uno corrupto, publica el control mediante temporal y verifica cada respaldo por nombre, tamaño y huella. Si una captura es incoherente o perdió su respaldo, el ZIP conserva las demás y registra `HISTORICO_RESPALDO_INVALIDO`. Se agregó una prueba que exporta dos lecturas ficticias, una alterada, y luego comprueba que un control corrupto se conserva íntegro. Validación pendiente en CI para este cambio.
- **Continuidad.** Primero comprobar la ejecución CI de este commit. Luego completar Histórico automático con prueba de pertenencia por RUT rotulado del titular (sin inferir por nombre ni por la URL en la captura automática) y suspensión independiente de las bitácoras tras cinco fallas. El arreglo de persistencia anterior se verifica a través del ZIP, sin consultar SITFA. Siguen pendientes las demás etapas recogidas en el plan.

## Protecciones revisadas de rust/producto

Cotejo dirigido con la auditoría del 17-nov y las puntas `ca0a317` y `e14b000`. Se incorporaron dos correcciones funcionales con sus regresiones. El cotejo de una fila normalizada con eventos embebidos se limita a consultas V1. Por ello un paquete legado coherente queda como «No verificable» por falta de evidencia, sin diagnosticar una contradicción inexistente. La ruta V2 conserva su comprobación independiente. Repetir el análisis con las mismas fechas mantiene selección, registro activo y propuestas. Los ajustes cosméticos y el verificador Python del delta quedan para el cotejo de E0/E7. Validación de estos dos cambios en la nueva ejecución CI.

## Estado para retomar

La validación del paquete Python y sus salidas Excel/JSON está cerrada por prueba automatizada. En [la ejecución del histórico](https://github.com/MatiGaete2023/flow/actions/runs/44648179157) pasaron 353 pruebas del workspace, incluida la regresión con un respaldo alterado. En [la ejecución de la auditoría](https://github.com/MatiGaete2023/flow/actions/runs/44648737252), sobre `a4a60ff7aa1a0e9772680707be81128cad6e0016`, terminaron correctamente el workspace Rust completo y el contraste con Python, incluidas las pruebas de Lite. La ejecución completó Linux y Windows: 355 pruebas del workspace en Linux y 353 en Windows, todas aprobadas. Pasaron las regresiones del respaldo alterado, la revisión legada y la selección al repetir el análisis. Se generaron capturas sintéticas y el [candidato Flow_Windows](https://github.com/MatiGaete2023/flow/actions/runs/44648737252/artifacts/14590307572) para ese commit de código. La aprobación institucional queda pendiente.

| Entrega | Ya comprobado | Trabajo técnico pendiente |
|---|---|---|
| E0 | CI con lock en Linux, comparación dirigida del delta de `rust/producto`. | Completar control `verificar_wp.py` y comparación dirigida de visualización. La CI sintética de Linux y Windows terminó correctamente. |
| E1 | ZIP creado por el exportador Python y cuatro clasificaciones iguales en Python, JSON Rust y XLSX Rust. Regresiones V1, V2 y legado. | Completar aceptación en el puesto. La verificación sintética de Windows ya pasó. |
| E2 | Perfiles presentes, 15 pruebas Lite, tope en 5.000, pausa configurable y respaldo Histórico por entrada. | Histórico automático, Escritos divergentes, favoritos, pausa manual, persistencia de preferencias, exportaciones y contrato Rust→Python. |
| E3 | Corpus versionado de 3.699 casos verdes y suite completa Rust en Linux. | Cotejos fuera del corpus y recorridos multifila de las fuentes nuevas cuando E2 esté listo. |
| E4–E7 | Regresiones existentes de productos, Outlook simulado, Meta 4, anonimización y arranques dentro de las suites de la base. | Preservación y vuelta Excel; paridad de productos; importación de lotes históricos para Meta 4; integración y entrega. Las pruebas existentes todavía no acreditan esos hitos. |

**Siguiente cambio de código.** Completar la captura opcional del Histórico en E2. Leer `descargador/lectura_bitacoras.py::_leer_historico`, `descargador/extension/pagina.js` para `historico_open` y `descargador/test_extension_historico.py`. Conectar primero la negociación de capacidad y el lector de la tabla con su RUT rotulado. Reutilizar el respaldo ya corregido y probar reuso por persona, pertenencia incierta, quinta falla y recuperación con un lote ficticio. Después comparar Escritos y la salida Rust→Python. La prueba con SITFA real y la recepción institucional se reservan al puesto autorizado.
