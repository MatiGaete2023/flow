# Plan · Python → Flow Rust

Vigente desde el 24 de noviembre de 2026. Reemplaza al [plan del 20 de noviembre](historico/PLAN_20261120.md) y a los [encargos](historico/ENCARGOS_IA.md). El estado de cada capacidad está en [ESTADO.md](ESTADO.md).

## Objetivo y corte

- Flow Rust reemplaza a Flow Python cuando el ciclo mensual completo corre en `flow_rust.exe` y el usuario lo deja en un acta.
- Arquitectura (D-5): **el Descargador recolecta toda la información que Flow necesita y Flow la procesa.** El contrato entre ambos es el paquete `CSMP_CAPTURE`.
- Orden (D-7): Descargador, exportaciones de lote, Asistente. Después, Meta 4, Anonimizador y distribución.
- Cada tramo termina con una prueba corta del usuario antes de abrir el siguiente.

## Cómo se trabaja un encargo

1. Leer el módulo Python de origen en la referencia fijada y, si existe, el crate histórico de `rust/crates`.
2. Implementar en `app/` sin cambiar reglas, umbrales ni textos.
3. Acreditar con un oráculo: un generador en `app/qa/` que graba la salida de Python sobre datos ficticios, y una prueba Rust que la compara y afirma sus conteos.
4. Probar según el presupuesto de `AGENTS.md`.
5. En el mismo commit: actualizar [ESTADO.md](ESTADO.md) y agregar una entrada corta al [registro](REGISTRO_IMPLEMENTACION.md).

Estados de un encargo: `ABIERTO` → `HECHO` (con el commit y la CI en verde) → `ACEPTADO` (solo el usuario).

Qué se compara con Python:
- **Exacto:** observaciones, reglas, marcas, Excel, Word, correos y el contenido del paquete.
- **Por contenido, no byte a byte:** manifiestos y archivos internos del lote que solo lee Flow Rust.

## Tramo 0 · Base de verificación

- **E-V1 · Referencia Python única — `ABIERTO`.**
  - Revisar el diff de `descargador/flujo_paquete.py` y `src/nurus/personal/revision_bitacoras.py` entre esta rama y `flow-python` (`afc4d12`).
  - Fijar el SHA de referencia en `app/qa/REFERENCIA_PYTHON.txt`; generadores y CI usan ese checkout.
  - Regenerar `app/tests/datos/paquete.json` y el contraste V2. Criterio: los oráculos pasan contra la referencia fijada, o cada diferencia queda declarada en [ESTADO.md](ESTADO.md).
- **E-V2 · Pruebas sin variables de entorno — `ABIERTO`.**
  - Rutas por omisión relativas a `CARGO_MANIFEST_DIR` para `FLOW_CASOS`, `FLOW_FIXTURES` y `FLOW_REFERENCIA`. Quedan `#[ignore]` solo las pruebas que necesitan un ZIP generado por Python.
  - Declarar en `app/Cargo.toml` la característica `x11` de eframe para Linux. Criterio: `cargo test --manifest-path app/Cargo.toml --locked` corre los oráculos en Linux sin preparación.
- **E-V3 · CI por niveles — `ABIERTO`.**
  - Trabajo en Linux con las pruebas en cada push.
  - Windows: compilar el `.exe`. Oráculos regenerados solo si cambian `app/qa/**`, `app/tests/datos/**` o la referencia. Ejecutable de entrega, capturas y contraste V1 con `rust/`, a pedido.

## Tramo 1 · Descargador

- **E-Q5d · Excel del lote y XLSX consolidado — `HECHO` en Linux (`c734f84`); falta ver su CI de Windows.**
  - `descargador/excel.rs` y `descargador/resultados.rs`, con el oráculo `app/qa/generar_excel.py`.
  - Por revisar: si `include_originals` quedó cubierto.
- **E-Q7 · Avance por fila de la lectura — `ABIERTO`.** Hoy la interfaz muestra un indicador de actividad y un resumen al terminar.
- **E-U1 · Intercambio entre herramientas (F27) — `ABIERTO`, por especificar.**
  - Python: `download_link.py`, `download_transfer.py` y `capture/temporal.py`.
  - Restricción vigente: el Asistente no navega SITFA ni inicia el Descargador. Antes de implementar, proponer al usuario cómo se expresa en un solo ejecutable (devolución del paquete a Trabajo, procedencia, cancelación y comparación temporal).
- **Prueba del usuario:** la de [ESTADO.md](ESTADO.md).

## Tramo 2 · Exportaciones de lote

- **E-S1 · Exportaciones y comparador — `ABIERTO`; `export_book` ya está (`c734f84`).**
  - Faltan de `descargador/resultados.py`: `export_csv`, `export_ics`, `export_zip` y `compare_lots`. Base en `rust/crates/flow-trabajo/src/exportar.rs`.
  - Oráculo nuevo en `app/qa/` sobre lotes ficticios.

## Tramo 3 · Asistente

En este orden, salvo que el usuario indique otro:

- **E-A1 · Reimportación y conciliación desde el Excel revisado (F10).** `sync.py` y `services/exports.py`.
- **E-A2 · Conciliación entre capturas (F07).** `capture_reconciliation.py`.
- **E-A3 · Recibos, vigencia y recuperación de sesiones (F09).** `product_state.py` y `session.py`.
- **E-A4 · Ajustes (F11).** Alias de contactos, selector de variables y validación por regla, plantillas de correo persistentes, e importador de matrices y de paquetes (`template_package.py`).
- **E-T1 · Cuenta Outlook (F13).** Solo borradores (`mail.Save()`) y CC fijo. Incluye simular fallos antes y después de guardar.
- **E-T2 · Formularios manuales (F15).** `manual.py`.
- **E-T3 · Estadísticas y métricas (F16).** `statistics.py` y `review_metrics.py`.
- **E-A5 · Cotejos pendientes (F02, F08, F12).** Egresados, OB e identidad multifila; edición masiva y deshacer; agrupación y firma de correos. Un caso de oráculo por comportamiento; no requiere interfaz nueva.

## Tramo 4 · Meta 4, Anonimizador y distribución

- **E-R0 · Prueba de reutilización — `ABIERTO`, una hora.** Agregar `flow-meta4` y `flow-anonimo` a `app/` como dependencias por ruta, compilar para Windows y correr sus 29 y 38 pruebas. Si funciona, E-R1, E-R2 y E-T4 se reducen al adaptador y a la vista; si no, se portan.
- **E-R1 · Meta 4: cálculo (F23).** Oráculos `rust/oraculo/generar_calculo_meta4.py` (450 casos) y `generar_tribunales_meta4.py`. Adaptador para carpetas de lote con el formato de Python.
- **E-R2 · Meta 4: plantilla, regeneración y vista (F24).** Cinco productos contrastados con Python y la vista en la navegación de `ui.rs`.
- **E-T4 · Anonimizador (F25).** ZIP, paquetes de captura y verificación posterior de XLSX. Oráculo `generar_anonimo.py`.
- **E-T5 · Distribución, ayuda y autodiagnóstico (F26, F28).** `portable/` y `package_check.py`.
- **Paquete H:** acta del usuario.

## Decisiones del usuario vigentes

- **Producto Excel (20-nov).** Columnas NURUS al final del consolidado o del original, en el orden `NURUS_ID_REGISTRO, NURUS_REGLAS, NURUS_PROPUESTA, FECHA_OBS, OBSERVACION, TT, CC, RES, NURUS_ESTADO_REVISION, NURUS_DECISIONES`. Prima sobre el orden de `flow-python`.
- **Redacción (20-nov).** Se aplica en Flow Rust; cambiar Flow Python requiere instrucción expresa.
  - **R1:** con la medida por vencer, sin la oración del próximo informe.
  - **R2:** la imposibilidad de atribuir un trámite a un ingreso queda como aviso interno.
  - **R3:** DCE con un solo encabezado y sin días.
  - **R4:** sin días de espera cuando el tribunal dispuso el ingreso.
  - **R5:** texto vigente en lugar de reiterar, solo para «Medida revisada, sin observaciones.» y para la derivación sin seguimiento.
  - **R6:** nombre del programa en tipo título.
  - R7 y R8: sin cambio.
- **D-1 (23-nov):** R6 conserva el guion del nombre de origen («Ciudad del Niño - Mulchén»).
- **D-2 (23-nov):** `keep_filters` solo en Seguimiento y Litigantes, como Python.
- **D-3 (23-nov):** se conserva el mensaje de Rust ante un XML 2003 con entidad externa.
- **D-4 (23-nov):** integrar todas las herramientas de Flow Python.
- **D-5 (24-nov):** «El Descargador recolecta toda la información que Flow necesita para analizar. Descargador recolecta info, Flow la procesa.» Interpretación aplicada en este plan, por confirmar: el contrato es el paquete, y los archivos internos del lote no necesitan ser idénticos a los de Python.
- **D-6 (24-nov):** F27 se mantiene en el alcance.
- **D-7 (24-nov):** orden Descargador, exportaciones de lote, Asistente.
- **D-8:** pendiente.

El detalle de las decisiones del 20 de noviembre está en el [informe de fallas](historico/INFORME_FALLAS_20261120.md#decisiones-del-usuario--20-nov-2026).
