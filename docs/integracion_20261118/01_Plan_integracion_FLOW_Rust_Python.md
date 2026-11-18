# Plan de integración en FLOW · Rust y Python

**Entrega a la IA implementadora · 18 de noviembre de 2026.** Objetivo: que el trabajo habitual de Espera, Cumplimiento e Informes produzca la observación correcta, y que los borradores y proyectos incluyan únicamente los vencimientos a los que corresponden. Esta entrega es un plan. El código del producto no se ha modificado.

**Documentos de trabajo.** Usar la [guía de consulta de RUS](02_Guia_de_consulta_RUS_SITFA.md) para identificar fuentes y límites. Las decisiones funcionales se reproducen aquí para poder ejecutar el plan aunque la conversación no esté disponible.

## 1. Resultado exigido

| Condición comprobada | Observación | Efecto en productos |
|---|---|---|
| Espera o Cumplimiento. En el histórico del mismo niño hay otro programa vigente, aún no consignado | **Se hace presente que [primer nombre del niño] cuenta con ingreso efectivo al programa [nombre del programa].** Sustituir ambos marcadores, conservando el nombre de programa mostrado. Agregar junto con las demás reglas. | Ninguna gestión nueva por esta frase. Se consigna una sola vez por niño y programa en la revisión correspondiente, incluso al pasar de Espera a Cumplimiento. |
| Escrito de informe de avance pendiente, atribuible al ingreso y al informe revisado | **Pendiente de resolver, informe de avance.** La contribución de esta regla contiene solo esa frase, una vez en el texto resultante. | Para ese informe: excluir proyecto de pide cuenta y correo pidiendo cuenta. Conservar las acciones independientes. |
| Informe ya incorporado a Historia; la evidencia válida enlaza el escrito y no identifica ingreso posterior del informe | Mantener el texto vigente que remite proyecto para aplicar la nomenclatura «Ingreso Informe de Cumplimiento». | Mantener el proyecto de nomenclatura (`NOMENCL`); no sustituirlo por un pide cuenta al programa. |
| La resolución con «Ingreso Informe de Cumplimiento X» ya consta para ese informe y el calendario aún marca pendiente | Informar lo que consta y sugerir revisar su asociación a la proyección. Ejemplo: «Se hace presente que el informe de [programa] registra ingreso a folio [folio] y figura pendiente en el calendario. Se sugiere revisar su asociación con la fecha proyectada [fecha].» | No pedir nuevamente ese informe ni preparar otro proyecto para aplicar la misma nomenclatura. |
| Dos o más fechas vencidas del mismo ingreso | Mostrar el detalle por fecha. Usar el nombre del mes como período cubierto solo cuando exista respaldo. | Una gestión de pide cuenta por ingreso para las fechas que siguen correspondiendo. Mantener las destinadas a resolución o asociación fuera de esos productos. |
| No hay evidencia nueva suficiente | Mantener la decisión previa del motor y mostrar el motivo de la duda al funcionario. | Un posible documento de otra persona, otro programa u otro período no elimina gestiones. Mantener el tratamiento existente de revisión humana. |

La regla nueva para distinguir observación administrativa de planteamiento al tribunal está **aplazada**. Notificaciones, órdenes, diligencias, equivalencias generales de egreso y carga de funcionarios quedan documentadas como fuentes de consulta; su automatización no forma parte de este cambio. Sí se usa el contenido de las observaciones ya registradas para impedir la reiteración del hecho acordado.

## 2. Base y lugares de intervención

Se inspeccionó `feat/flow-estudio-d-20261117`, commit `d2935dd8087b7467677e12d423fac512f1461190`. Contiene Rust y la referencia Python. `rust/producto` apuntaba a `cfc12ee2`; `Rediseno` a `e96d99d9`; `Integral` y `CSMP-Asistente` compartían `046c3432`. La rama `main` no representa el producto integrado del corte. Trabajar sobre una base que conserve Estudio D; comparar la punta real al ejecutar. Si la distribución Python toma otra rama, identificar su flujo de publicación y preparar allí el traslado de los cambios Python probados, manteniendo una sola definición funcional. Incorporar ambos lenguajes a una rama de trabajo no acredita una entrega en ambos instaladores.

| Capa | Python | Rust |
|---|---|---|
| Captura y lectura | `descargador/extension/pagina.js`, `descargador/lectura_historia.py`, `descargador/lectura_bitacoras.py`, `src/nurus/historia_html.py`, `src/nurus/calendario_html.py` | `rust/extension/pagina.js`, `flow-captura/src/lecturas.rs`, `recoleccion.rs`, `vinculos.rs` |
| Paquete, alcance y cobertura | `descargador/export_csmp_capture.py`, `src/nurus/capture/contract.py`, `coverage.py`, `src/nurus/personal/capture_input.py` | `flow-trabajo/src/paquete.rs`, `captura.rs`, `lib.rs`; exportación del productor de captura |
| Reglas y composición | `src/nurus/personal/motor/reglas_espera.py`, `reglas_cumplimiento.py`, `reglas_informes.py`, `reglas_historia.py`, `vocabulario.py`, `personal/calendario_ingreso.py` | `flow-motor/src/reglas.rs`, `hist.rs`, `lib.rs`, `ctx.rs` y contexto de trabajo en `flow-trabajo` |
| Textos y acciones | `src/nurus/personal/textos_base.json`, `personal/work.py::actions_for`, `personal/resolutions.py` y preparación de correos | `rust/oraculo/generar_datos.py` genera `rust/datos/textos.json`; `flow-motor::acciones`; `flow-productos/src/correos.rs`, `word.rs` |
| Validación | `tests/test_historia_informes.py`, `test_reglas_informes.py`, `test_nomenclatura_proyecto_20261116.py`, pruebas del Descargador y paquetes ficticios | `flow-motor/tests/oraculo.rs`, pruebas de `flow-trabajo`, `flow-productos`, `rust/pruebas/` y flujo de CI de Rust |

Las rutas Rust que empiezan por nombre de crate son relativas a `rust/crates/`. Antes de editar, seguir las llamadas desde el punto de entrada real. [Reglas y acciones Rust](https://github.com/MatiGaete2023/flow/blob/d2935dd8087b7467677e12d423fac512f1461190/rust/crates/flow-motor/src/lib.rs) · [Acciones Python](https://github.com/MatiGaete2023/flow/blob/d2935dd8087b7467677e12d423fac512f1461190/src/nurus/personal/work.py) · [Contrato de paquetes](https://github.com/MatiGaete2023/flow/blob/d2935dd8087b7467677e12d423fac512f1461190/contracts/csmp_capture_v1/README.md).

## 3. Secuencia de implementación

### Paso 1. Fijar la base y una comparación reproducible

Leer `AGENTS.md`, el plan vigente y el estado del trabajo del repositorio. Anotar SHA, versiones del contrato y resultado inicial de las suites de los componentes afectados. Revisar qué rama produce cada entrega. Conservar un paquete anterior al cambio y una salida de referencia de cada motor. Resultado: lista de fallos preexistentes y un conjunto fijo de casos con fecha de corte explícita. No actualizar resultados esperados para hacer pasar una prueba sin explicar el cambio funcional.

### Paso 2. Definir el dato que viaja de la captura al motor

Diseñar un cambio aditivo en el contrato compartido y probar primero las versiones que aceptan los validadores existentes. Modelar (a) filas de Escritos de la causa, (b) filas de Histórico de Protección de la persona, (c) asociación del informe con ingreso y fecha proyectada, (d) evidencia de que el hecho del otro programa fue consignado. Mantener identidad, fuente, fecha real de lectura, cobertura, literal original, referencia de fila y fundamento de atribución. Separar dato capturado de decisión derivada.

Cada lector debe representar por separado «fuente no solicitada», «leída sin filas», «con filas», «parcial», «fallida» e «identidad incierta», usando los estados equivalentes del contrato. No representar todo como lista vacía. Validar con un ZIP actual, un ZIP legado sin los campos opcionales y un ZIP nuevo con una fuente parcial. Si se propone un archivo opcional nuevo, comprobar que manifest, esquema, importadores y exportadores lo aceptan. Si lectores antiguos lo rechazan, explicitar compatibilidad de lectura hacia atrás y versionar la extensión; no prometer compatibilidad bidireccional sin probarla.

### Paso 3. Incorporar Escritos en el detalle común de causa

Reutilizar la lectura existente de Historia y su respuesta HTML. Localizar por encabezados «Doc.», «Tipo Escrito», «Solicitante», «Estado». Leer valores **de la misma fila**, independientemente del orden de columnas. Una tabla con cabecera válida y cero filas es una bandeja vacía al instante capturado. Una pestaña presente sin cuerpo legible es una fuente no verificada. En la muestra ya recibida vienen las cuatro tablas en el detalle; tolerar otras versiones que solo carguen la pestaña al abrirla.

Reconocer literalmente los tres tipos del apartado 4 y el estado «Pendiente», conservando el texto original. Separar **detectar un informe pendiente en la causa** de **asignarlo al ingreso y período**. Registrar referencia documental si está presente como metadato visible; no hace falta abrir el documento. Pasar la evidencia a Trabajo; generar texto y acciones en el motor, nunca en la extensión. Probar la lectura aislada y a través de exportación, importación y apertura de un paquete.

### Paso 4. Incorporar «Ingreso Histórico Protección» de la persona seleccionada

El lector actual de Historia está acotado a causa y excluye la consulta por RUT. La nueva fuente se puede integrar leyendo la tabla visible **abierta manualmente en el flujo de trabajo autorizado**, con un control de captura explícito en el Descargador, o importando la tabla de esa vista con contexto verificado. Definir y ensayar la asociación de la vista abierta con la persona de la fila seleccionada. No convertir el clic histórico en una consulta por RUT en segundo plano. Si falta prueba de pertenencia o el contenido está en un marco aún no accesible, devolver «no verificable» y permitir continuar con Escritos y los demás insumos. Mantener la separación Asistente–Descargador.

Dentro del histórico del mismo niño, el único criterio para **distinguir otro programa** es el nombre. Comparar la denominación completa; normalizar espacios, mayúsculas y Unicode para presentación equivalente, conservando el nombre visible para redactar. No agrupar programas distintos porque ambos sean PRM o AFT. El identificador de la persona sirve para leer el histórico correcto; no exigir RIT, fecha ni identificador de otra intervención para reconocer el otro programa. Excluir el programa actual. Para cada nombre restante, aplicar el par de campos de egreso en la misma fila. Si un programa tiene una fila antigua egresada y otra vigente, usar la vigente; colapsar las filas vigentes repetidas del mismo nombre. Un dato ilegible no acredita vigencia.

### Paso 5. Resolver «solo una vez» y obtener el primer nombre

Buscar constancias registradas del mismo niño y del mismo otro programa en la evidencia disponible, incluso si la etapa cambió. Reutilizar el procesamiento de bitácora y las identidades existentes. Reconocer la frase exacta y variaciones inequívocas; una mera mención del nombre, una negación, una propuesta descartada o un texto referido a otro niño no bastan. Evitar coincidencias por subcadena entre programas con nombres parecidos.

Una marca local debe significar «consignado» solo si refiere una observación efectivamente registrada o confirmada por el funcionario. Mostrar en revisión una posible mención previa si la redacción es ambigua. Si la bitácora está truncada y no hay un registro fiable, hacer visible ese límite antes de sugerir otra vez la frase. Guardar y recuperar la evidencia de consignación en forma idempotente; no marcarla al generar o exportar una propuesta. Obtener el primer nombre del campo de nombres o del formateador ya verificado de FLOW. Probar nombres completos en orden distinto, tildes y dos personas con igual primer nombre; si solo hay apellidos o el formato es indeterminado, pedir revisión de esa fila.

### Paso 6. Conservar cada vencimiento y resolver sus fuentes

Extender el agrupador actual, que devuelve número de vencimientos distintos y fecha más antigua, para conservar el conjunto de fechas y referencias de filas. Resolver por tribunal, causa, ingreso y fecha proyectada cuando la identidad está verificada. Mantener las asociaciones del mismo informe entre pestañas. Con Excel legado sin ID remoto, preservar la comparación histórica; el grupo aproximado no autoriza cancelar la gestión de otro ingreso.

Por cada fecha, reunir calendario, bandeja e historia al mismo corte. Una referencia documental o una asociación explícita permite enlazar evidencias. Con un solo candidato, aplicar las condiciones de atribución ya existentes y distinguirlas de un vínculo explícito; varias personas, ingresos, períodos o lecturas contradictorias requieren marcar la ambigüedad. Para un escrito que cubra varios períodos, asignar solo los que estén respaldados. No derivar el mes informado del mes del vencimiento. En fechas diferentes comparar primero qué se sabe al corte: una vista anterior de Escritos no debe prevalecer sobre la incorporación posterior acreditada en Historia. Con dos estados incompatibles sin secuencia fiable, conservar revisión.

### Paso 7. Integrar con el motor existente y componer la observación

Añadir la constancia del otro programa en Espera y Cumplimiento a los fragmentos que ya aplicaban. Mantener las exclusiones y el tratamiento de las reglas tradicionales. Ejecutar la detección de informe pendiente en todos los contextos a los que corresponda, sin depender de que el funcionario haya comenzado en la pestaña Informes. Resolver por informe y componer después; si coinciden el detector de Escritos y el cotejo del calendario, emitir una sola vez la frase exacta por observación resultante. La regla de Escritos es un estado que se vuelve a evaluar al corte. La constancia de otro programa sí es histórica y no se reitera una vez consignada.

Mantener I-01/I-02, la regla DCE con margen de 10 días cuando corresponda, C10/I3, CAL-03 y el guardado de ediciones humanas. Si una evidencia nueva cambia el resultado, registrar por qué y con qué fuente. La situación «escrito ya en Historia sin ingreso posterior» conserva la rama que prepara nomenclatura. La situación «ya existe nomenclatura y el calendario sigue pendiente» lleva a revisión de asociación. No tratar «Acompaña informe» o «Téngase presente» como equivalentes automáticos de una entrega de cumplimiento.

### Paso 8. Propagar la decisión a correo, Word y Excel

En Python, `actions_for` interpreta eventos `INFORMES.I01_*` como `programa_vencido` y `PC_INFO`; `HISTORIA.C10_ESCRITO` activa `NOMENCL`. En Rust verificar los equivalentes de `acciones`. Elegir una rama explícita que no vuelva a activar el pide cuenta por el prefijo del identificador de regla. Comprobar las opciones del usuario, las marcas humanas de RES y el flujo manual de revisión; los productos obsoletos deben volver a prepararse bajo el mecanismo vigente. Separar el cálculo automático de las decisiones manuales registradas.

Por ingreso, preparar una sola gestión de pide cuenta con las fechas aún exigibles. Respetar la agrupación habitual de destinatarios del correo y conservar el detalle por ingreso en la nómina. Impedir que una fila excluida reaparezca en el Excel adjunto, en el texto del Word o al exportar todos los seleccionados. Una marca `NOMENCL` de otro período puede coexistir con `PC_INFO`; no aplastar ambas decisiones en un único campo. Mostrar al funcionario el motivo y los períodos si el formato de salida admite una sola resolución por fila. Regenerar los textos Rust desde la fuente Python y revisar el diff.

### Paso 9. Ejecutar la matriz y comprobar la paridad

Crear un único conjunto ficticio con los mismos datos, corte y expectativas para ambos lenguajes. Comparar literal por literal: observación final, referencias, estado de cobertura, lista de acciones, fechas en nómina, clase y contenido de los proyectos. Incluir la ruta normal del Descargador hasta el Asistente, selección parcial de filas, abrir/guardar/reabrir, importar el mismo paquete dos veces y recalcular después de una edición humana.

| Prueba de aceptación | Resultado que debe comprobar la IA |
|---|---|
| AFT/PRM/PDE/PDC/PIE, FAE/FAS y residencial con tipo correcto y Pendiente | Mismo texto exacto en todos; sin pide cuenta por ese informe. Tipo correcto en una fila y Pendiente en otra no activa la regla. Un tipo distinto o estado distinto tampoco. |
| Un ingreso con tres fechas: pendiente en Escritos, incorporado sin nomenclatura, vencido exigible | La primera produce la frase y ninguna gestión de pide cuenta; la segunda, proyecto de nomenclatura; solo la tercera aparece en correo y proyecto de pide cuenta. |
| Cinco fechas, entrega acreditada para dos y fila de listado duplicada | Resolver las dos acreditadas, conservar tres fechas exigibles si las demás condiciones se cumplen, una gestión y cero fechas duplicadas. |
| Nomenclatura presente y calendario pendiente tras regularización | Revisión de asociación. No preparar nuevamente nomenclatura ni requerimiento. |
| Dos programas o NNA en una causa, o dos ingresos sucesivos al mismo programa | Reutilizar Historia de causa; conservar bitácora/calendario de cada ingreso. Una entrega ambigua no cancela productos. |
| Programa actual A, otro B vigente repetido y C con egreso informado | Frase exacta para B una sola vez. C no se incluye. Probar además fecha sola, causal sola, programa actual repetido y dos otros vigentes de nombres diferentes. |
| Ya observado en Espera y revisión en Cumplimiento; borrador descartado; historia parcial | El registro verdadero evita repetición; el descartado no. Evidencia parcial se distingue de ausencia comprobada. |
| Revisión desde Excel antiguo o paquete sin Escritos/Histórico; fuente vacía acreditada; lectura fallida | En el legado se preservan propuestas previas; vacío, no disponible y fallo aparecen como estados diferentes. |
| Cambio de corte, selección oculta, reintento tras cancelación, producto ya editado | No mezclar cortes, duplicar constancias ni sobrescribir productos o decisiones humanas. |
| Caso real de estructura reproducido ficticiamente: 16 vencimientos, 10 ingresos; un grupo con cinco fechas | Ambos motores producen el mismo inventario y justifican cada inclusión/exclusión. Para validación funcional usar datos ficticios con verdad conocida. |

### Paso 10. Verificar y entregar

Desde la raíz con dependencias de desarrollo de la rama, ejecutar `python -m pytest -q -o faulthandler_timeout=90` y `python tools/verificar_wp.py`. Desde `rust/`, ejecutar `cargo test --workspace --locked` y, para el candidato, `cargo build --release --locked -p flow`. Correr las pruebas del Descargador desde su configuración de CI y los escenarios de extensión en Chromium ficticio definidos en el workflow Rust; añadir cobertura equivalente para las dos fuentes nuevas. Comprobar paquete Python → lector Rust y paquete Rust → lector Python, así como los lotes antiguos.

Entregar el diff, las reglas y formatos cambiados, una tabla Python/Rust de los escenarios, registros de pruebas, excepciones frente a la línea base y una guía breve para verificar en la interfaz. Para reversión, conservar paquetes originales, probar apertura por la versión anterior cuando esté soportada y separar el cambio aditivo de captura del de decisiones. La aceptación institucional de Windows/SITFA se registra por separado. Publicación o integración al tronco son pasos posteriores al encargo de implementación. [CI Python](https://github.com/MatiGaete2023/flow/blob/d2935dd8087b7467677e12d423fac512f1461190/.github/workflows/tests.yml) · [CI Rust](https://github.com/MatiGaete2023/flow/blob/d2935dd8087b7467677e12d423fac512f1461190/.github/workflows/rust.yml).

## 4. Literales de entrada para las pruebas

| Fuente | Condición en una sola fila |
|---|---|
| Escritos, intervención ambulatoria | Tipo Escrito = `Inf. Cump Programa Ambulatorio`; Estado = `Pendiente`. |
| Escritos, FAE/FAS | Tipo Escrito = `Inf. Cump FAE/FAS`; Estado = `Pendiente`. |
| Escritos, residencia | Tipo Escrito = `Inf. Cump Centro Residencial`; Estado = `Pendiente`. |
| Ingreso Histórico Protección, mismo niño y otro nombre de programa | Fecha egreso = `--` y Causal egreso = `SIN INFORMACION`. Una fecha o una causal informada indica medida no vigente. |
| Historia, resolución de ingreso de informe | Descripción `Ingreso Informe de Cumplimiento X`. Normalizar comparación y conservar original, folio, tipo de actuación y fecha. |

La lista de programas ambulatorios aportada por el usuario es ilustrativa. El tipo del escrito determina la familia; si existen dos programas de la misma familia, resolver la atribución antes de excluir un período.
