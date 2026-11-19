# Plan vigente — solo herramientas Python (19-nov-2026)

**Rama única de trabajo: `flow-python`.** No se crean ramas nuevas. Registro de avance: [`REGISTRO.md`](REGISTRO.md).
Informe comparativo Rust → Python (base de este plan): [`INFORME_Y_PLAN_20261119.md`](INFORME_Y_PLAN_20261119.md) §1–2.
Fuente de qué se puede leer de RUS/SITFA: [`02_Guia_de_consulta_RUS_SITFA.md`](../integracion_20261118/02_Guia_de_consulta_RUS_SITFA.md)
(derivada de los HAR, el ZIP de exploración y las capturas del usuario; los originales **no** se versionan).

Decisión v (`PLAN_VIGENTE.md`): Flow Rust queda congelado; todo lo útil se porta a Python con botones u opciones.
N1 = verificado en código/documentos; N2 = inferencia con supuesto; N3 = sin medición.

---

## 1. Qué se sabe de las fuentes nuevas (respuesta a «los datos ya los entregué»)

| Fuente | Dónde está la evidencia | Qué dice (N1) | Consecuencia |
|---|---|---|---|
| **Escritos de la causa** | Guía §3 y §7 (`cumplimiento4.har`); fixture `tests/fixtures_integracion/escritos_ficticio.html` (desde la foto del usuario); REGISTRO de integración «Hallazgos» | Viene en la **misma respuesta** de «Información de la causa» que la Historia: cuatro tablas (Historia, Notificación, Diligencias, Escritos). Columnas `Doc.`, `Tipo Escrito`, `Solicitante`, `Estado`; valores en minúscula; `Doc.` es solo un icono; **sin folio, fecha ni período**. La guía advierte **otra variante** con solo la tabla Historia. | El Descargador Python **ya** extrae Escritos de la Historia guardada (`export_csmp_capture.py::_escritos`). Si no se ven, N2: (a) el Asistente no los muestra (solo los usa en reglas) o (b) esa respuesta fue la variante sin la tabla. Se resuelve con diagnóstico anónimo y una columna visible (P3a), sin pedir nada al usuario. |
| **Ingreso Histórico Protección** («Historial Menores», Mejor Niñez) | REGISTRO de integración «Hallazgos»; fixture `historial_menores_ficticio.html`; guía §3 | Ventana `PopUpHistorial.jsp?tipo_popUp=77&RUT_Litigante=…&RUT_DV=…`. Banda de título, encabezado no en la primera fila, columna **«Nombre Proyecto»**, tabla separada **«Ingreso Historico(LRPA)»** y fila de relleno. Vigente = `Fecha Egreso --` y `Causal Egreso SIN INFORMACION`. La extensión solo abre ventanas tipo 8 (calendario) y 11–13 (bitácora). Los HAR no traen una tabla poblada de esta ventana. | Lectores Python y Rust ya adaptados; falta **abrir la ventana desde el Descargador** (extensión + lote). La guía recomienda «sin consultas autónomas por RUT»: se abrirá **solo con el RUT de la fila verificada del listado**, igual que si el usuario hiciera clic (ver decisión pendiente D-H1). |
| Notificación y Diligencias | Guía §3 (`cumplimiento4.har`: 1.901 notificaciones, 15 diligencias) | Vienen en la misma respuesta de la causa; columnas conocidas. No hay reglas que las usen. | P9 opcional: leerlas y mostrarlas, sin reglas nuevas salvo decisión del usuario. |
| Litigantes, Materias, Plazos, Remisión | Guía §3 | Controles visibles; sus filas exigen abrir otra respuesta. | Fuera de alcance hasta que el usuario lo pida. |

## 2. Paquetes de trabajo (orden de ejecución)

Clase según la decisión j: **E3** = revisión independiente obligatoria (motor, textos, Outlook, contrato, extensión SITFA,
anonimizador); **V** = basta `tools/verificar_wp.py` + pruebas.

| # | Paquete | Herramienta · botón u opción | Clase | Estado |
|---|---|---|---|---|
| **P1** | Botón «Marcar como consignada» (frase «otro programa vigente») en el detalle de Trabajo, con deshacer; persiste y el recálculo la respeta (`Work.consignar` ya existe) | Asistente · Trabajo · detalle | V | **Hecho** (ver REGISTRO) |
| **P2** | **Revisión de bitácoras = separar registros sin observación.** (a) Descargador: registrar en cada bitácora la evidencia de alcance (tabla reconocida, vacía comprobada, posible recorte), port de `rust/crates/flow-captura/src/lecturas.rs::cobertura_bitacora` y `REVISION_BITACORAS_ESQUEMA.md`. (b) Asistente: pantalla «Revisión de bitácoras…» que clasifica Con observación / Sin observación / No verificable, filtra, marca Revisada y exporta Excel + JSON; el motor corre sobre todo el paquete antes de separar | Descargador (automático) · Asistente: botón «Revisión de bitácoras…» | V | **Hecho** (ver REGISTRO) |
| **P3a** | Escritos visibles: columna/indicador «Escritos» en Trabajo (pendientes de la causa y estado de la fuente: leída, vacía, sin tabla, fallida); `diagnostico_historia.json` cuenta por causa si la respuesta traía la tabla Escritos y cuántas filas (anónimo) | Asistente · Trabajo; Descargador · diagnóstico | V | **Hecho** |
| **P3b** | Escritos en la variante sin tabla: abrir la pestaña Escritos desde la ventana de la causa | Descargador + extensión | **E3** | **Hecho en lo posible** (detección + extracto anónimo automático) |
| **P3c** | **Histórico automático**: opción «Leer Ingreso Histórico (Mejor Niñez)» en Seguimiento; la extensión abre `PopUpHistorial` tipo 77 con el RUT de la fila verificada; se valida RUT de la ventana = RUT de la fila; lector existente; se incluye en el paquete (`historico_proteccion`) | Descargador · opciones de lectura | **E3** | **Hecho; prueba real pendiente** (el contenido viene del iframe `IrPopUpPublicoAccion.do`) · E3 pendiente |
| **P8** | Diagnóstico del paquete: historial anónimo en `respaldo/proceso.json` (fases, decisión por lote, error literal, resumen) y aviso final de bitácoras faltantes por tribunal | Descargador | V | **Hecho** |
| **P4** | Resultados con búsqueda y carpetas previas; Meta 4 «Elegir plantilla Excel…» (el código ya acepta plantilla); paridad de importación de contactos | Asistente · Resultados; Meta 4 | V | **Hecho** |
| **P5** | Diseño útil de Estudio D en Tk: conteos por etapa, confirmación antes de recalcular, lista/ficha en Correos y Resoluciones, ayuda contextual | Asistente | V | **Hecho en parte**: confirmación antes de reprocesar y conteos del resumen; F1 ya abría el manual. **No hecho:** lista/ficha en Correos y Resoluciones (rediseño visual que exige Tk real; queda para cuando haya prueba en Windows) |
| **P9** | Notificación y Diligencias: lectura y vista, sin reglas | Descargador · Asistente | V (E3 si se agregan reglas) | **Postergado por el usuario** (19-nov) |
| **P6** | Borradores `.eml` cuando no hay Outlook (CC fijo, sin envío) | Asistente · Correos | **E3** | **Postergado por el usuario** (19-nov) |
| **P7** | Consola `observar` / `producir` del Asistente | `python -m nurus.personal …` | V | **Postergado por el usuario** (19-nov) |

**Excluido:** captura dentro del Asistente (AGENTS.md: «El Asistente no navega SITFA ni inicia el Descargador»).

## 3. Decisiones pendientes del usuario

- **D-H1 (P3c): RESUELTA «Sí» (usuario, 19-nov-2026, «Sí a D-H1»).** El Descargador abre el Histórico de cada niño del
  lote con el RUT de su fila verificada del listado, sin clic por niño; casilla «Leer Ingreso Histórico» desactivada por
  defecto.
- **P6 y P7:** si se quieren.

## 3b. Cambios al plan durante la ejecución (19-nov-2026)

- **P3a** confirmado en uso real (Escritos llegan en la respuesta de la causa).
- **P3c** cambió su forma técnica: `PopUpHistorial.jsp` es un envoltorio con un iframe a `/SITFAWEB/IrPopUpPublicoAccion.do`;
  el contenido se lee de ese marco y la pertenencia se decide por el «Rut :» visible. La prueba real queda **pendiente**
  (usuario); la revisión E3 de la extensión también.
- **Tope de aperturas por lote 2000 → 5000** (decisión x) y suspensión del histórico tras 5 fallas seguidas.
- **P9, P6 y P7 postergados** por el usuario («sigue con los demás pasos pendientes, menos P9, 6 y 7»).
- Orden vigente: P8 → P5; luego pruebas del usuario (P1–P4, P3c) y E3 de P3c.

## 4. Criterio de terminado (cada paquete)

Pruebas nuevas que fallan sin el cambio; suite Python sin fallos nuevos (en el contenedor sin Tk, igual a
`docs/integracion_20261118/base_python_fallos.txt`); `ruff` sin avisos nuevos en los archivos tocados; botón u opción
visible y descrito en el manual de la herramienta; `REGISTRO.md` actualizado; commit y push a `flow-python`.
Nada se marca «aceptado»: la puerta sigue siendo el acta del usuario.
