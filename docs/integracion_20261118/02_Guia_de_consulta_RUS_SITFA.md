# Guía de consulta de RUS/SITFA para FLOW

**Edición del 18 de noviembre de 2026 · Fuente, lectura, significado y uso en reglas.** Esta guía está pensada para quien necesite averiguar si un dato existe, dónde buscarlo, qué herramienta ya lo lee y hasta dónde se puede interpretar. Su alcance son las vistas de seguimiento RUS y las pantallas SITFA relacionadas que aparecen en los materiales del proyecto. El manual institucional reúne muchas otras materias. Para implementar los cambios acordados, usar el [plan de integración Rust y Python](01_Plan_integracion_FLOW_Rust_Python.md).

**Navegación.** Para encontrar una pantalla, empezar por la sección 1 o buscar su nombre en la 3. Para interpretar una columna, ir a la 4. Para una nomenclatura concreta, buscar su literal en la sección 9 y contrastar el efecto documentado en la 5. Las secciones 6 y 7 indican cómo realizar la lectura y qué archivos la acreditan.

## 1. Consulta rápida por pregunta

| Quiero saber… | Abrir o leer | Dato decisivo |
|---|---|---|
| ¿El tribunal dispuso el ingreso? ¿El niño ya ingresó? | Espera / Cumplimiento y la Historia de causa | Fecha de resolución y actuación de ingreso dispuesto; para el inicio efectivo, fecha de ingreso efectivo y programa. Son dos hitos. |
| ¿Cuánto lleva esperando o en intervención? | Fila del listado y sus cabeceras | Tiempo de espera o días de cumplimiento, con el corte de esa consulta. La fecha de egreso proyectado es una proyección. |
| ¿Hay otro programa vigente para este niño? | Historial Menores → Ingreso Histórico Protección | Nombre distinto del programa revisado y, en la misma fila, Fecha egreso `--` y Causal egreso `SIN INFORMACION`. |
| ¿Hay un informe por resolver? | Información de causa → Escritos | Tipo Escrito de la modalidad y Estado `Pendiente`. Para afectar una gestión de un período, atribuir ese escrito al ingreso y al informe. |
| ¿El informe ya entró a la historia? | Información de causa → Historia | Diferenciar fila tipo Escrito de fila tipo Resolución, su descripción, folio y fecha. |
| ¿Por qué el calendario lo sigue marcando pendiente? | Calendario del ingreso + Escritos + Historia | Distinguir escrito pendiente, escrito incorporado sin nomenclatura identificada, y resolución ya registrada cuya proyección requiere revisar asociación. |
| ¿Ya se hizo presente un hecho? | Bitácora del ingreso y constancias registradas disponibles | Texto y fecha del centro, programa mencionado y persona correcta. Una observación propuesta no acredita que se consignó. |
| ¿Se notificó una resolución al programa? | Historia + Notificación del detalle de causa | Acto, destinatario a la fecha y resultado registrado. Sin vínculo suficiente solo se pueden presentar candidatos. |
| ¿Una orden sigue activa? | Seguimiento de litigantes/órdenes y, cuando proceda, Historia | Estado de la orden, envío y resultado son campos distintos. Comprobar página y corte. |
| ¿El reporte RES mide el trabajo de un cargo? | Carga de Trabajo de Funcionarios | El menú 7 de la muestra y el perfil de carga 29 del Descargador Python tienen contratos distintos. |

**Qué evidencia respalda una respuesta.** Una respuesta HAR con filas permite verificar datos estructurales de esa muestra. Un encabezado sin filas acredita el esquema y la bandeja observada al corte. Un botón visible acredita una opción de interfaz. Un evento sanitizado acredita una señal o una navegación. El código acredita lectura implementada en la versión citada. El manual explica el significado funcional; la precisión del usuario fija las reglas que desea para FLOW. Estas fuentes tienen alcances distintos y deben citarse por separado.

## 2. Entidades y claves que deben conservarse

| Entidad | Identificación y alcance | Error frecuente |
|---|---|---|
| Tribunal y causa | Tribunal + identificador de causa; RIT es la referencia visible y sirve de comprobación. RUC es otra referencia de la causa cuando aparece. | Cruzar dos listados por el RIT sin considerar tribunal o atribuir todo lo que ocurre en la causa a una sola intervención. |
| Niño o litigante | Persona seleccionada. Verificar pertenencia del histórico que se está leyendo; usar el primer nombre únicamente para redactar. | Cruzar por primer nombre; suponer que dos hermanos comparten todas las actuaciones. |
| Ingreso o intervención | Identidad remota verificada del contrato: tribunal + causa + ingreso. Programa y persona aportan contexto. | Confundir dos ingresos sucesivos al mismo programa. La excepción funcional acordada es comparar solo el nombre para decidir si hay **otro programa** en el histórico del mismo niño. |
| Informe esperado | Ingreso + proyección identificable, fecha y, cuando conste, período cubierto. Una fecha de vencimiento no expresa necesariamente el mes que se informa. | Dar por recibidos todos los vencimientos de una causa porque hay un escrito posterior. |
| Actuación y escrito | Fuente, tipo de trámite, descripción/nomenclatura, referencia, folio y fecha cuando existan. «Doc.» puede ser un control de interfaz. | Deducir el contenido del documento desde el icono o equiparar escrito con resolución. |
| Observación y respuesta | Ingreso, etapa de la anotación, autoría centro/tribunal, fechas y textos independientes. | Usar una respuesta del tribunal como fecha de última observación del centro. |
| Corte y cobertura | Diferenciar fecha del hecho, de ingreso, de proyección, de lectura y fecha de corte. Guardar páginas leídas/esperadas y estados de cada fuente. | Concluir «no existe» a partir de página 1, de una tabla omitida o de una historia con ventana temporal limitada. |

Espera, Cumplimiento e Informes son **puertas de entrada**. Según el usuario, abrir la Historia de la misma causa desde cualquiera muestra la misma fuente y abrir la bitácora del mismo ingreso muestra la misma bitácora. Se verificó una coincidencia de tribunal, RIT e identidad de causa entre una fila de Informes y otra de Cumplimiento, con **distintos ingresos** en los enlaces de bitácora. Conviene compartir la lectura de causa en el mismo corte y mantener por separado la evidencia de cada ingreso.

## 3. Inventario de pantallas y lectura disponible

| Fuente | Reconocimiento y datos | Situación de FLOW al corte |
|---|---|---|
| Seguimiento, menú 89 | Formulario de Informes, selector de tribunal/modalidad, pestaña activa; tablas Espera, Cumplimiento, Informes y Egresados. La misma respuesta puede traer las cuatro cabeceras y datos en una sola. | Perfiles en Python completo, Lite y Rust. Verificar selección, tabla, número y total de páginas antes de consolidar. |
| Espera | RIT, tribunal, persona, modalidad/derivación, curador, tiempo de espera, fecha de resolución, revisión, duración, marcas y fechas de fichas, nacimiento, edad, oído y próximas audiencias, según variante. | Lectura de filas y reglas de Espera. En `flow.har`, 38 filas ambulatorias; en `Espera.har`, una de diagnóstico. `espera2.har` devolvió Cumplimiento. |
| Cumplimiento | ID visible según pantalla, RIT, persona, programa, fecha de resolución, ingreso efectivo, egreso proyectado, días en cumplimiento y para egresar, enlaces a informes/observaciones y fichas. | Lectura implementada. Se observan consultas residenciales y ambulatoria por causa. El nombre de la captura, el nombre de un campo de paginación heredado y la vista activa pueden diferir. |
| Informes | Tipo «Por vencer (a 25 días)», «Vencidos» o «Recibidos»; ingreso efectivo, egreso proyectado, ingreso del informe, vencimiento, días, revisión y controles. `Inf.` abre el calendario por ingreso. | El perfil consulta por tipo. Dos respuestas de `flow.har` contienen 42 próximos y 16 vencidos. «Recibidos» aparece como opción sin una consulta poblada propia en estos HAR. |
| Egresados | Pestaña y tabla de egresados. | Hay perfil de lectura. Las otras consultas exhiben su cabecera vacía; para caracterizar el esquema poblado y su causal se necesita una respuesta dedicada. |
| Programas y filtros | Carga de opciones de programa según tribunal/modalidad. El formulario admite criterios de causa, centro, plazo o fechas según pantalla. | Diferenciar opción ofrecida y filtro aplicado; exportar el alcance efectivo. Al filtrar por causa una fila devuelta no acredita un único ingreso en el tribunal. |
| Bitácora del ingreso | Acción visible `ShowObservaciones`; en la muestra, ventana de bitácora con `TablaInforme` y formulario de respuesta separado. | Descargador lee y conserva evidencia, admite lotes y reanudación. El candidato Estudio D revisa cobertura y autoría. El formulario que permite responder en RUS es una parte distinta de la tabla leída. |
| Calendario del ingreso | Acción `ShowInformesProgramados`. Encabezados de proyecciones: Fecha Proyección, Fecha Informe, Estado. Cabecera puede aportar programa, fecha de ingreso efectivo y plazos. | Lector opcional vinculado al ingreso. CAL-03 permite cotejar proyecciones con Informes/Hoja2. Hay seis aperturas correlacionadas en el ZIP; los valores de sus filas no están guardados allí. |
| Calendarios mensuales | Menú 90, medidas por vencer; menú 91, informes por vencer. Se consultan por tribunal, mes y año conforme al catálogo. | Perfiles existentes. Se usan como cruce y no se debe duplicar la persona por aparecer en el calendario. Una carga de feriados para un selector de fecha de RES no acredita apertura de estas pantallas. |
| Información de causa → Historia | Acción de la fila `ShowHistoria`; se abre el detalle de causa. Cabeceras Folio, Doc., Sel, Anexo, Etapa, Trámite, Desc. Trámite, Referencia, Fecha Tram. | Lector de Historia implementado con verificación de identidad, fecha de corte, ventana y contexto. La existencia de un PDF o anexo no acredita su lectura. |
| Información de causa → Escritos | Tabla con Doc., Tipo Escrito, Solicitante, Estado. La respuesta de muestra ya contiene la tabla. El usuario confirmó tres tipos y el estado. | El lector existente de Historia se concentra en Historia. La lectura propia de la bandeja y su integración con informes son cambios acordados. En la muestra hay cero filas, no cero escritos históricos. |
| Información de causa → Notificación | Esta.Not., Fec.Not., Tipo Not., Ente Not., RIT, RUC, Fec.Tram., Tip.Part., Nombre, Trámite, Observación. | Fuente presente en la respuesta. No se identificó en el lector actual una extracción equivalente a Historia. La atribución de un acto a un destinatario requiere evidencia adicional cuando el folio no está expuesto. |
| Información de causa → Diligencias | Folio, Doc., Resp., Est.Dil., Int., Tipo de Diligencia, Fec.Trm., Observación. | Fuente observada. Útil para revisar diligencia y respuesta si existe identidad suficiente. Su estado de envío no acredita recepción por el programa. |
| Detalle de cabecera | Litigantes, Materias, Plazos y Remisión son controles visibles del detalle. Hay también actuaciones con enlaces de documentos o anexos. | El clic sin su respuesta permite inventariar el control; las filas, estado y destinatarios exigen abrir y leer la fuente correspondiente. |
| Historial Menores → Ingreso Histórico Protección | Fuente de persona distinta de la Historia de causa. El usuario identifica Programa, Fecha egreso y Causal egreso como datos operativos. Las acciones de historial por persona y de historial de fichas son diferentes. | Fuente nueva para esta integración. Evidencia de aperturas; los HAR entregados no muestran una tabla poblada identificable con ese título y esas columnas. Capturar la tabla visible de la persona seleccionada o importarla con procedencia, sin consultas autónomas por RUT. |
| Litigantes / órdenes, menú 94 | Estado de orden, tipo, nomenclatura, resolución, medida, envío, fecha de envío, derivación y próximas audiencias, entre otros. El libro exportado puede exponer «Resultado» y títulos distintos de la pantalla. | Perfil de consultas y avisos Python OB-01 a OB-05 para órdenes activas. En el Excel no hay identidad remota de ingreso suficiente para cruzar automáticamente con bitácoras. |
| MAO / carga | Reporte por cargo, tribunal y fechas con columnas de tramitación y firma. En `res2.har` el origen es menú 7. | El perfil de carga del **Python completo** se refiere al menú 29, con otro formulario admitido/botón y verificación de esquema. Rust no enumera carga entre los perfiles de selección examinados. No confundir con columna RES del Asistente ni con proyecto Word. |
| Asistencia | Aplicación externa de registro de entrada/salida en Google Apps Script. | `asistencia.har` sirve para identificar tráfico ajeno. Excluir de cobertura de RUS. |

**Comparación de lectores.** El Descargador completo y el Lite no tienen perfiles idénticos; la rama Rust también tiene su propio selector. Evaluar por rama y entrada de usuario, nunca por la sola presencia de un nombre de archivo. [Perfiles Python](https://github.com/MatiGaete2023/flow/blob/d2935dd8087b7467677e12d423fac512f1461190/descargador/perfiles.py) · [Perfiles Rust](https://github.com/MatiGaete2023/flow/blob/d2935dd8087b7467677e12d423fac512f1461190/rust/crates/flow-captura/src/sitfa/perfil.rs) · [Lectores Rust](https://github.com/MatiGaete2023/flow/blob/d2935dd8087b7467677e12d423fac512f1461190/rust/crates/flow-captura/src/lecturas.rs) · [Lectura Python de Historia](https://github.com/MatiGaete2023/flow/blob/d2935dd8087b7467677e12d423fac512f1461190/descargador/lectura_historia.py).

## 4. Diccionario de los datos que cambian una decisión

| Campos o términos | Interpretación operativa |
|---|---|
| Resolución de ingreso / Fec. Resolución | Día de la actuación que dispone la medida o ingreso según el contexto. Puede existir mientras el niño aún espera una plaza o inicio de intervención. |
| Ingreso efectivo / Fec.Ing.Efectivo | Fecha en que efectivamente ingresó el NNA, según el efecto registrado. Útil para inicio, plazo y proyección; no sustituirla por la fecha de la resolución. |
| Derivación, programa, modalidad, centro | La modalidad es una familia; el nombre identifica el programa concreto a efectos de la regla acordada. Dos PRM distintos pueden pertenecer a la misma modalidad. |
| Egreso proyectado / días para egresar | Proyección temporal. La fecha puede vencer sin que se haya registrado un egreso. Una prórroga puede cambiar la proyección. |
| Fecha egreso / Causal egreso | En la tabla específica del Histórico de Protección, ambos indicadores sin egreso (`--`, `SIN INFORMACION`) expresan vigencia conforme al usuario. Una fecha o causal informada indica medida no vigente. Un campo no leído sigue siendo desconocido. |
| Fecha Proyección / Fec.Vencimiento | Fechas del calendario y de la fila del listado. Compararlas con identidad y corte; una discrepancia es revisable. El día de vencimiento no prueba qué mes cubre un documento. |
| Fecha Informe / Fec.Ing.Informe / estado del calendario | Fecha registrada respecto de una proyección o de la fila de informe. El lector admite estados de su contrato y contrasta fecha/estado. Enviado y Pendiente se interpretan en el calendario, no en la tabla de notificaciones. |
| Estado = Pendiente en Escritos | El escrito figura pendiente de resolución. Clasificar como informe de avance únicamente uno de los tres tipos reconocidos. «Pendiente» en otra pestaña tiene otro significado. |
| Solicitante, Tipo Escrito y Doc. | Remitente visible, categoría de la presentación y referencia/control documental cuando exista. Un solicitante por sí solo no prueba período. No inferir el folio, fecha o programa si la vista no los expone. |
| Folio, Trámite, Desc. Trámite, Referencia | Folio local de causa; clase del acto (Escrito, Resolución, Audiencia, Actuación…); nombre o nomenclatura; texto contextual. Comparar la descripción en la columna correcta y conservar el tipo real, incluso si una nomenclatura suele aparecer como resolución. |
| Tipo Observación, Etapa, Fecha/Usuario/Observación Centro y Tribunal | Una fila puede contener dos participaciones. Separar autoría y fechas. Meta 4, en su propio alcance FAE/Residencia y Espera/Cumplimiento, usa la fecha de observación válida del centro. Evitar usar su período específico como regla general del Asistente. |
| Estado de notificación, fecha del trámite y fecha de notificación | Distinguir resolución, intento y resultado. Usar el literal del sistema y conservar Ente Not., tipo y destinatario. «Enviada» no equivale automáticamente a «Realizada». |
| Fecha de última actualización de fichas, Fec. Oído y Próxs.Auds. | Indicadores de ficha individual, ambulatoria o FAE, derecho a ser oído y próximas audiencias. Verificar fuente y tipo de celda; una celda representada por icono puede perder texto al extraerla. |
| 80 BIS, F.Amb., revisión, duración | Guardar literal, fuente, unidad y cabecera cuando estén disponibles. El manual describe selección de 80 bis en la derivación y actualización de informes RUS; una marca aislada o una fecha no autorizan crear reglas o umbrales nuevos. |
| Est. Orden / Est. Envío / Resultado | Vigencia, despacho y resultado son atributos independientes. Se observaron dos filas activas con envío «Realizada». La misma persona puede tener más de una orden. |
| RES / PC_INFO / NOMENCL | En FLOW son selecciones o tipos de producto: proyecto de pide cuenta por informe y proyecto para nomenclatura, entre otros. Generar un Word o un borrador de correo no constituye ingreso, resolución, notificación ni firma en SITFA. |

## 5. Concordancia entre nomenclaturas, manual y reglas

Las líneas citadas son de la copia aportada de **Manual sitfa.md**, decodificada como Windows-1252. La compilación repite secciones y contiene instrucciones de distintas épocas. Para reproducir la consulta: 92.735 líneas, SHA-256 `d755b78c7ed3b28760460175002ec6b0b113f3dd08e85334d34acefae22bc855`. Buscar por título si cambia la versión. Las filas que indican «HAR/código» tienen interpretación funcional limitada a la evidencia, sin atribuirles un efecto normativo adicional.

| Nomenclatura o familia | Qué representa y qué comprobar | Fundamento |
|---|---|---|
| «Ing. serv. protección especializada y otras redes»; antiguos ingresos ambulatorio, centro residencial, FAE/FAS | Disposición de ingreso. El manual agrupa las antiguas nomenclaturas y contempla regularización e interconexión. Conservar modalidad, NNA y programa. La remisión institucional a Mejor Niñez no acredita notificación realizada al programa. | Manual §1.2, líneas 16.301–16.358. En la muestra, cuatro resoluciones y una audiencia llevan la abreviatura moderna. |
| «Ingreso efectivo» | Inicio efectivo de la intervención. Al relacionarlo con un ingreso dispuesto comprobar que es la misma persona y línea. La fecha requerida por el efecto puede diferir de la fecha de tramitación. | Manual §1.3, líneas 14.839–14.852; [vocabulario del motor](https://github.com/MatiGaete2023/flow/blob/d2935dd8087b7467677e12d423fac512f1461190/src/nurus/personal/motor/vocabulario.py). |
| «Prorroga» / «Prórroga» | Extensión que puede alterar plazo de intervención y remisión. Comprobar el efecto y el nuevo calendario; una prórroga visible no resuelve por sí sola qué informe previo se cumplió. | Manual §1.4, líneas 14.918–14.973. |
| «Egreso serv. protec. especializada y otras redes» y egresos históricos por modalidad | Acción de egreso; distinguirla de «Solicitud de Egreso de niño(a)», «Preegreso» y de la fecha proyectada. Conservar literal y alcance. En una misma actuación pueden registrarse egreso, ingreso e ingreso efectivo en un orden determinado. | Manual §§1.3 y 1.5, líneas 14.853–14.857 y 14.974–15.025. Motivos de egreso, líneas 2.989–3.000. |
| Variante abreviada de egreso frente a la constante de FLOW | Nueve filas del HAR dicen «Egreso serv. protec. especializada y otras redes»; la constante Rust es «Egreso de servicio protección especializada y otras redes». La comparación por clave normalizada no expande abreviaturas. Si interesa incorporar equivalencias, probar sus efectos por separado. La regla acordada de vigencia usa los campos del histórico. | [Constantes del motor](https://github.com/MatiGaete2023/flow/blob/d2935dd8087b7467677e12d423fac512f1461190/rust/crates/flow-motor/src/hist.rs) · [Búsqueda de trámites](https://github.com/MatiGaete2023/flow/blob/d2935dd8087b7467677e12d423fac512f1461190/rust/crates/flow-dominio/src/historia.rs). |
| «Inf. Cump Programa Ambulatorio», «Inf. Cump FAE/FAS», «Inf. Cump Centro Residencial» | Tipos de escrito asociados por el usuario a intervención ambulatoria, FAE/FAS y residencia. En Escritos, junto a «Pendiente», generan la frase aprobada. En Historia, buscar resolución asociada posterior antes de decidir si procede nomenclatura. | Literales confirmados por el usuario; aparecen también en las filas de Historia y en el vocabulario Python. La bandeja de muestra está vacía. |
| «Ingreso Informe de Cumplimiento X» | Resolución con la nomenclatura de ingreso del informe. No es el escrito que presentó el programa. Si ya está atribuida al período y el calendario sigue pendiente, considerar asociación, especialmente tras una regularización. | 55 filas tipo Resolución en Historia. Manual §3.3, líneas 17.221–17.267: el mantenedor puede asociar un informe anterior a la nueva proyección cuando coinciden litigante y línea. |
| «Acompaña informe», «Informe Situación Actual», «Informe de Diagnóstico» y «Se evacua informe» | Antecedentes candidatos. Sus glosas genéricas no identifican necesariamente un informe de cumplimiento, modalidad ni período. Contrastar datos visibles antes de cambiar una gestión. | HAR de detalle y vocabulario de escritos reconocidos. |
| Orden de búsqueda, pide cuenta búsqueda, término y deja sin efecto búsqueda | Apertura, seguimiento y posibles cierres son hitos separados. Distinguir «Deja sin efecto» genérico de «Deja sin efecto búsqueda». Asociar por evidencia; jamás cerrar varias órdenes por proximidad temporal sola. | HAR, vocabulario de Historia y [reglas OB](https://github.com/MatiGaete2023/flow/blob/d2935dd8087b7467677e12d423fac512f1461190/src/nurus/personal/ordenes_busqueda.py). |
| Derivación Dirección Regional, respuesta de derivación, respuesta de continuidad, solicitud DCE y rectificación | Son pasos de derivación, respuesta o gestión diagnóstica; sus efectos específicos exigen revisar el trámite. Algunos literales están expresamente entre los ignorados por el motor. DCE tiene tratamiento de plazos propio, sin tipo de escrito DCE adicional confirmado para la regla nueva. | Manual §§1.1–1.3, entradas del índice 14.224–14.238 y texto 16.435–16.444; vocabulario y reglas de Informes. |
| Derecho a ser oído, designación de abogado y deja sin efecto designación C. Ad-Litem | Actuaciones relevantes al niño o representación. El destinatario de una notificación y el curador consignado son entidades distintas. Verificar la identidad antes de atribuir. | Manual «Derecho a ser oído», desde línea 8.347; para curador, vocabulario y reglas actuales. |
| Certif. Notificación / Correo devuelto | Pistas sobre comunicación o resultado. Cotejar acto, destinatario, canal, fecha y último resultado. El manual prevé que un cambio de litigante/programa conserve la historia del anterior. | Manual líneas 45.428–45.452; HAR de Historia y Notificación. |
| Opinión consejero técnico / Solicita Opinión Consejero Técnico | Solicitud y opinión emitida se cuentan por separado; una opinión posterior podría responder a otra solicitud. | Manual «NCL: Opinión Consejero técnico», desde línea 10.879; HAR. |
| Sin efecto de medida por interconexión / sin efecto de derivación D.R.; conciliación / dictación de sentencia | El motor considera determinadas actuaciones de historia y contexto en reglas de vigencia o cierre. Una glosa general nunca acredita por sí sola el cierre de la intervención individual. | Vocabulario y reglas históricas del código. |

**Lecturas complementarias del manual.** Líneas 3.145–3.227: columnas de ingreso efectivo, egreso proyectado, acceso al calendario «Inf.» y exportación. Líneas 3.245–3.254: significado del cálculo de días en Cumplimiento e Informes. Líneas 3.017–3.026: asociación de un escrito a resolución en la bandeja de Tramitación; la columna de juez descrita para esa bandeja no debe suponerse en la tabla Escritos del detalle. Líneas 17.243–17.267: asociación tras regularización y límite cuando el litigante fue eliminado y agregado de nuevo.

## 6. Cómo extraer y conciliar sin perder el alcance

1. **Localizar la fuente correcta.** Abrir desde el Descargador con la sesión establecida por el funcionario, o importar una fuente local admitida. El Asistente analiza el paquete. Para una revisión histórica distinguir el ZIP sanitizado del Explorador de un paquete de trabajo CSMP y del HAR con cuerpos de respuesta.
2. **Validar la vista antes de leer.** Comprobar tribunal, entidad, pestaña, filtro, tabla y encabezados. Leer campos de una misma fila. Si falta la tabla esperada, conservar incidencia y contexto; si la tabla válida está vacía, registrar vacío al corte. Verificar que las opciones seleccionadas y los resultados correspondan. No manejar un recurso estático o un HTTP 200 como si fuera un resultado funcional.
3. **Delimitar páginas y fechas.** Guardar página actual y total; recorrer las páginas previstas por el perfil y declarar parcial un recorrido incompleto. En bitácora, paginadores, marcos o indicios de corte exigen cautela antes de afirmar ausencia. En Historia guardar alcance temporal y contexto anterior permitido por el lector. Un trámite anterior al corte y fuera de la ventana es contexto, no necesariamente una novedad.
4. **Conservar literal y valor normalizado.** Comparar por cabeceras, tipos y estados. Registrar filas con metadatos de procedencia; nunca confundir un guion observado con columna ausente. Respetar acentos al redactar. Usar abreviaturas equivalentes solo si se ha verificado la equivalencia. Mantener originales y material judicial fuera de artefactos públicos; crear ejemplos de prueba ficticios.
5. **Vincular por entidad.** El paquete `CSMP_CAPTURE_V1` expone consultas, registros, bitácoras, calendarios, historia_tramites, documentos e incidencias; comprobar las tablas declaradas en el manifiesto y sus huellas. Una tabla del esquema no garantiza que su fuente se haya leído. Historia se vincula a causa, calendario y bitácora a ingreso. La fuente nueva de persona necesita procedencia propia.
6. **Conciliar en este orden.** Determinar ingreso y proyección; comprobar bandeja e historia; identificar nomenclatura; comparar estado del calendario; decidir acciones por período; finalmente agrupar por ingreso. Cuando la misma evidencia aparece en dos vistas, producir una sola observación. Si se revisan fuentes de días distintos, anotar ambas fechas antes de comparar.
7. **Conservar la decisión humana.** Diferenciar propuesta, edición del funcionario, observación registrada y producto generado. Solo el hecho consignado evita reiterar la constancia única. El Word o correo preparado no modifica el expediente. Para una afirmación de ausencia especificar qué fuente se revisó, con qué identidad, cobertura y corte.

[Contrato y validación](https://github.com/MatiGaete2023/flow/blob/d2935dd8087b7467677e12d423fac512f1461190/contracts/csmp_capture_v1/README.md) · [Diccionario de tablas del paquete](https://github.com/MatiGaete2023/flow/blob/d2935dd8087b7467677e12d423fac512f1461190/contracts/csmp_capture_v1/DICCIONARIO.md) · [Contrato histórico de lectura de causa](https://github.com/MatiGaete2023/flow/blob/d2935dd8087b7467677e12d423fac512f1461190/contracts/csmp_capture_v1/HISTORIA_VENTANA.md). Este último describía una muestra anterior con solo la tabla de Historia. La captura nueva acredita otra variante con cuatro tablas en la misma respuesta; probar ambas.

## 7. Inventario de evidencia y límites

| Material | Aporte verificable |
|---|---|
| `flow.har` | Corte del listado del 9 de octubre de 2026. Entradas 2, 22, 42 y 77: Espera, Informes por vencer, vencidos y órdenes. 16 vencimientos pertenecen a 10 ingresos; uno acumula cinco fechas. En órdenes hay 100 filas, página 1 de 5. |
| `Espera.har`, `espera2.har` | Una consulta de Espera acreditada; el segundo archivo contiene una consulta que devuelve Cumplimiento. |
| `Cumplimiento.har`, `cumplimiento2.har`, `cumplimiento3.har`, `cumplimiento6.har` | Consultas residenciales o por causa. Los archivos 2 y 3 son copias exactas. |
| `cumplimiento4.har` | Detalle del 13 de octubre. Historia con 1.489 filas, Notificación con 1.901, Diligencias con 15 y Escritos con cabecera y cero filas. Los estados de Notificación se reparten en 1.858 Realizada, 29 Pendiente, 7 Fallida, 5 Enviada y 2 No Realizada. Las 15 diligencias muestran envío por correo del sistema. |
| `Bitacora.har` | Tres aperturas con idéntico cuerpo. Una sola bitácora de muestra: 31 filas, 28 administrativas y 3 al tribunal; 23 de Cumplimiento, 6 de Informes y 2 de Espera. Estos conteos ilustran lectura; la regla de gestión por tipo de observación quedó aplazada. |
| `RES.har`, `res2.har` | HTML de Carga de Trabajo de Funcionarios y solicitud exitosa de XLS. No se conserva el contenido del XLS en el HAR; corresponde validar el libro de esa variante antes de admitirlo. |
| `cumplimiento5.har`, `rus.har`, `asistencia.har` | Los dos primeros tienen cero entradas; el último es asistencia externa. |
| ZIP `SITFA_EXPLORATION_V1_3…` | 974 eventos estructurales; 813 sin clasificación original, 88 atribuidos a Cumplimiento y 73 a Bitácoras. Hay 94 acciones de interfaz, no 974 actuaciones judiciales. Los 974 no tienen correspondencia evento a evento demostrada con los HAR. |

El nuevo análisis de los 813 separó 70 acciones explícitas, 61 eventos de ruta específica, 9 de estructura de bitácora, 90 de contexto de ventana identificable, 274 de contexto genérico, 7 de ruta compartida y 302 sin función identificable con ese método. Para contexto de ventana se exigió un único clic en los dos segundos previos en la pestaña origen. Los eventos no contienen valores de egreso, folios o fechas de los informes que permitan dar por probada una actuación judicial concreta.

**Prueba de los límites del cruce.** Los 48 escritos tipificados de Historia tienen alguna resolución con la nomenclatura de ingreso y folio posterior. Asociar a cada escrito el primer folio posterior hace converger 15 escritos en seis resoluciones. En Notificación, dos fechas de resoluciones de ingreso no dan filas coincidentes, otra da siete y otra una. En el día con una fila hay dos resoluciones. Por eso fecha, proximidad y cantidad no sustituyen la atribución.

## 8. Mantenimiento como guía de referencia

Ante una nomenclatura nueva, registrar el literal, la columna, tipo de trámite, pantalla, fuente, fecha y versión; buscar en el manual por descripción completa, abreviaturas y título de sección; verificar qué decisión toma el motor en ambos lenguajes. Si solo existe un botón, documentar «control visible». Si hay fila confirmada por el usuario, indicar «semántica confirmada por usuario». Si hay muestra poblada y prueba reproducible, marcar cada avance por separado. Mantener diferencias de edición del manual y variantes de tribunal como evidencia, sin reemplazar silenciosamente el significado anterior.

Para cada cambio futuro de regla, registrar una fila con **dato que se lee → evidencia → entidad → condición → observación → acciones afectadas → caso ambiguo → prueba**. Actualizar esta guía y los textos aprobados solo cuando cambien la evidencia o una decisión del usuario. Consultar la [rama funcional en el commit estudiado](https://github.com/MatiGaete2023/flow/tree/d2935dd8087b7467677e12d423fac512f1461190) para reproducir el estado.

## 9. Índice de las descripciones observadas en Historia

Consulta alfabética de las 1.489 filas de la causa de muestra. Hay 80 literales originales. El índice agrupa dos grafías de «Orden de búsqueda» y generaliza los dos destinos de «Exhorto a…». Se conservan 78 entradas de consulta. La frecuencia cuenta filas de Historia, no actos únicos, personas ni obligaciones pendientes. En las entradas marcadas HAR, la explicación describe la glosa y sus límites; no se ha verificado en el manual un efecto jurídico específico por cada literal.

| Desc. Trámite | Filas | Interpretación para la lectura | Base |
|---|---:|---|---|
| Acompaña documentos | 66 | Aporte, certificación, orden de certificar, copia o devolución de documentos. No permiten afirmar por sí solos qué informe quedó cumplido. | HAR |
| Acompaña informe | 80 | Presentación o actuación relativa a un informe. El objeto, la modalidad y el período requieren datos específicos. | HAR; regla tipificada más restringida |
| Audiencia no realizada | 3 | Solicitud, citación, no realización o cambio de programación según el literal. La citación por sí sola no acredita celebración. | HAR |
| Certif. Notificación | 34 | Certificación de notificación o devolución de correo. Verificar destinatario, acto y resultado en la fuente. | HAR y tabla Notificación |
| Certificado | 7 | Aporte, certificación, orden de certificar, copia o devolución de documentos. No permiten afirmar por sí solos qué informe quedó cumplido. | HAR |
| Certifíquese | 1 | Aporte, certificación, orden de certificar, copia o devolución de documentos. No permiten afirmar por sí solos qué informe quedó cumplido. | HAR |
| Cesación | 2 | Medida o cambio de estado de objeto no determinado por el rótulo. Verificar a qué medida y persona se refiere. | HAR |
| Citación a audiencia | 10 | Solicitud, citación, no realización o cambio de programación según el literal. La citación por sí sola no acredita celebración. | HAR |
| Citación Audiencia Especial | 12 | Solicitud, citación, no realización o cambio de programación según el literal. La citación por sí sola no acredita celebración. | HAR |
| Como se pide | 10 | Pronunciamiento de alcance dependiente de lo solicitado. Identificar escrito o petición objeto. | HAR |
| Comunicación entre tribunales | 9 | Comunicación, remisión, recepción o devolución interinstitucional. Comprobar destino y actuación relacionada. | HAR |
| Contesta oficio | 25 | Documento, instrucción, solicitud o contestación de oficio según el literal; identificar destino y referencia. | HAR |
| Contesta traslado | 5 | Trámite de traslado o respuesta. Asociar con la solicitud correspondiente. | HAR |
| Correo devuelto | 2 | Certificación de notificación o devolución de correo. Verificar destinatario, acto y resultado en la fuente. | HAR y tabla Notificación |
| Cúmplase | 1 | Distinguir comunicación de cumplimiento e instrucción de cumplir. Comprobar el objeto de la actuación. | HAR |
| Cumple lo ordenado | 16 | Distinguir comunicación de cumplimiento e instrucción de cumplir. Comprobar el objeto de la actuación. | HAR |
| Deja sin efecto | 7 | Medida o cambio de estado de objeto no determinado por el rótulo. Verificar a qué medida y persona se refiere. | HAR |
| Deja sin efecto búsqueda | 5 | Emisión o revocación de orden según el literal; revisar el ciclo de la orden correcta. | HAR y vocabulario |
| Deja sin efecto designación C. Ad-Litem | 3 | Cese de una designación. Contrastar la designación concreta y posibles designaciones posteriores. | HAR y vocabulario |
| Derecho a ser oido | 4 | Actuación sobre derecho a ser oído; identificar persona, fecha y alcance. | Manual, línea 8.347 |
| Derivación Dirección Reg. | 4 | Derivación a dirección regional o su respuesta. La respuesta por sí sola no prueba ingreso efectivo. | HAR y vocabulario |
| Devolucion documentos | 2 | Aporte, certificación, orden de certificar, copia o devolución de documentos. No permiten afirmar por sí solos qué informe quedó cumplido. | HAR |
| Devuelve Exhorto | 3 | Comunicación, remisión, recepción o devolución interinstitucional. Comprobar destino y actuación relacionada. | HAR |
| Digitalización | 13 | Aporte, certificación, orden de certificar, copia o devolución de documentos. No permiten afirmar por sí solos qué informe quedó cumplido. | HAR |
| Diligencias | 11 | Diligencia o documento de respuesta. Vincular con folio o referencia; no interpretar envío como cumplimiento. | HAR y tabla Diligencias |
| Doc.Respuesta Diligencia | 7 | Diligencia o documento de respuesta. Vincular con folio o referencia; no interpretar envío como cumplimiento. | HAR y tabla Diligencias |
| Egreso Programa Fae Fas/Ambulatorio | 6 | Egreso registrado mediante una nomenclatura actual o histórica. Identificar la intervención. | Manual §1.5 |
| Egreso serv. protec. especializada y otras redes | 9 | Egreso registrado mediante una nomenclatura actual o histórica. Identificar la intervención. | Manual §1.5 |
| Evacua traslado | 6 | Trámite de traslado o respuesta. Asociar con la solicitud correspondiente. | HAR |
| Exhorto a [tribunal] | 3 | Comunicación, remisión, recepción o devolución interinstitucional. Comprobar destino y actuación relacionada. | HAR |
| Ficha Ambulatoria | 3 | Registro de ficha según clase. Cotejar actualización con el campo de ficha correspondiente. | HAR |
| Ficha Individual | 17 | Registro de ficha según clase. Cotejar actualización con el campo de ficha correspondiente. | HAR |
| Ficha Niño, Niña y/o Adolescente FAE O FAS | 1 | Registro de ficha según clase. Cotejar actualización con el campo de ficha correspondiente. | HAR |
| Ha lugar | 2 | Pronunciamiento de alcance dependiente de lo solicitado. Identificar escrito o petición objeto. | HAR |
| Incidente | 1 | Materia recursiva o incidental. Revisar la actuación y decisión posterior antes de inferir efecto sobre la medida. | HAR |
| Inf. Cump Centro Residencial | 21 | Escrito de cumplimiento de la modalidad indicada. En Historia buscar su tramitación; en Escritos comprobar Estado y atribución. | HAR, usuario y vocabulario |
| Inf. Cump FAE/FAS | 12 | Escrito de cumplimiento de la modalidad indicada. En Historia buscar su tramitación; en Escritos comprobar Estado y atribución. | HAR, usuario y vocabulario |
| Inf. Cump Programa Ambulatorio | 15 | Escrito de cumplimiento de la modalidad indicada. En Historia buscar su tramitación; en Escritos comprobar Estado y atribución. | HAR, usuario y vocabulario |
| Informe de Diagnóstico | 1 | Presentación o actuación relativa a un informe. El objeto, la modalidad y el período requieren datos específicos. | HAR; regla tipificada más restringida |
| Informe Situación Actual | 65 | Presentación o actuación relativa a un informe. El objeto, la modalidad y el período requieren datos específicos. | HAR; regla tipificada más restringida |
| Ing. serv. protección especializada y otras redes | 5 | Ingreso dispuesto por la modalidad o nomenclatura de ingreso unificada. Separar del inicio efectivo. | Manual §1.2 |
| Ingreso a centro residencial | 5 | Ingreso dispuesto por la modalidad o nomenclatura de ingreso unificada. Separar del inicio efectivo. | Manual §1.2 |
| Ingreso demanda | 1 | Plazo de causa o ingreso de demanda. Mantener separado de ingreso a programa y vencimiento de informe. | HAR |
| Ingreso Efectivo | 16 | Inicio efectivo acreditado por el trámite; comprobar programa y fecha del efecto. | Manual §1.3 |
| Ingreso Informe de Cumplimiento X | 55 | Resolución tipificada de ingreso. Verificar el informe y la proyección a los que corresponde. | Manual §3.3 y regla de Historia |
| Ingreso Programa Ambulatorio | 5 | Ingreso dispuesto por la modalidad o nomenclatura de ingreso unificada. Separar del inicio efectivo. | Manual §1.2 |
| Ingreso Programa FAE/FAS | 2 | Ingreso dispuesto por la modalidad o nomenclatura de ingreso unificada. Separar del inicio efectivo. | Manual §1.2 |
| Mantención | 7 | Medida o cambio de estado de objeto no determinado por el rótulo. Verificar a qué medida y persona se refiere. | HAR |
| Medida precautoria | 28 | Medida o cambio de estado de objeto no determinado por el rótulo. Verificar a qué medida y persona se refiere. | HAR |
| Modificación | 1 | Medida o cambio de estado de objeto no determinado por el rótulo. Verificar a qué medida y persona se refiere. | HAR |
| No ha lugar | 4 | Pronunciamiento de alcance dependiente de lo solicitado. Identificar escrito o petición objeto. | HAR |
| Oficio | 1 | Documento, instrucción, solicitud o contestación de oficio según el literal; identificar destino y referencia. | HAR |
| Opinión consejero técnico | 311 | Opinión consignada. La coincidencia de fechas no basta para cerrar solicitudes. | HAR y manual, línea 10.879 |
| Orden de búsqueda | 8 | Emisión o revocación de orden según el literal; revisar el ciclo de la orden correcta. | HAR y vocabulario |
| Pídase cuenta | 2 | Petición o seguimiento. La descripción aislada no identifica documento, obligado ni período. | HAR |
| Plan de Intervención | 2 | Registro relativo al plan. Por sí solo no identifica cumplimiento de un informe proyectado. | HAR |
| Plazo Judicial Causa | 2 | Plazo de causa o ingreso de demanda. Mantener separado de ingreso a programa y vencimiento de informe. | HAR |
| Preegreso | 2 | Petición de egreso o etapa previa. Buscar un egreso efectivo posterior antes de concluir cierre. | HAR y distinción del manual §1.5 |
| Prorroga | 33 | Extensión de plazo. Revisar fechas y proyección afectadas. | Manual §1.4 |
| Rec. Reposición con Ape. subs. | 1 | Materia recursiva o incidental. Revisar la actuación y decisión posterior antes de inferir efecto sobre la medida. | HAR |
| Recep. de la Corte (fallo) | 1 | Comunicación, remisión, recepción o devolución interinstitucional. Comprobar destino y actuación relacionada. | HAR |
| Recursos | 1 | Materia recursiva o incidental. Revisar la actuación y decisión posterior antes de inferir efecto sobre la medida. | HAR |
| Remite Antecedentes | 5 | Comunicación, remisión, recepción o devolución interinstitucional. Comprobar destino y actuación relacionada. | HAR |
| Reprogramar | 2 | Solicitud, citación, no realización o cambio de programación según el literal. La citación por sí sola no acredita celebración. | HAR |
| Resp. derivación dir. regional | 4 | Derivación a dirección regional o su respuesta. La respuesta por sí sola no prueba ingreso efectivo. | HAR y vocabulario |
| Se evacua informe | 2 | Presentación o actuación relativa a un informe. El objeto, la modalidad y el período requieren datos específicos. | HAR; regla tipificada más restringida |
| Se oficie | 3 | Documento, instrucción, solicitud o contestación de oficio según el literal; identificar destino y referencia. | HAR |
| Se provea escrito pendiente | 2 | Petición o seguimiento. La descripción aislada no identifica documento, obligado ni período. | HAR |
| Se reitere solicitud | 1 | Petición o seguimiento. La descripción aislada no identifica documento, obligado ni período. | HAR |
| Sol. entrevista consejo téc. | 1 | Solicitud de opinión o entrevista. Identificar petición y respuesta por referencia. | HAR; manual, línea 10.879 para opinión |
| Solicita audiencia | 1 | Solicitud, citación, no realización o cambio de programación según el literal. La citación por sí sola no acredita celebración. | HAR |
| Solicita opinión C.T. en audiencia | 2 | Solicitud de opinión o entrevista. Identificar petición y respuesta por referencia. | HAR; manual, línea 10.879 para opinión |
| Solicita Opinión Consejero Técnico | 192 | Solicitud de opinión o entrevista. Identificar petición y respuesta por referencia. | HAR; manual, línea 10.879 para opinión |
| Solicita se oficie | 2 | Documento, instrucción, solicitud o contestación de oficio según el literal; identificar destino y referencia. | HAR |
| Solicitud de Egreso de niño(a) | 2 | Petición de egreso o etapa previa. Buscar un egreso efectivo posterior antes de concluir cierre. | HAR y distinción del manual §1.5 |
| Solicitud que se indica | 30 | Petición o seguimiento. La descripción aislada no identifica documento, obligado ni período. | HAR |
| Tengase presente | 241 | Pronunciamiento de alcance dependiente de lo solicitado. Identificar escrito o petición objeto. | HAR |
| Traslado | 2 | Trámite de traslado o respuesta. Asociar con la solicitud correspondiente. | HAR |
