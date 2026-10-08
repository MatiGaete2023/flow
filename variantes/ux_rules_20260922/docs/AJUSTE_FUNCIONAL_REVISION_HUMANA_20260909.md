# NuRus y el trabajo humano: ajuste funcional y simplificación

**Análisis iniciado:** 9 de septiembre de 2026. **Cierre documental:** 10 de septiembre de 2026.  
**Base técnica:** NuRus `afd82b80ae69375e1d2f6fc03f78c081b0bab45c`, rama `implementacion-plan-2026-09-08`.  
**Fuentes funcionales:** aclaración directa del usuario e *Informe funcional del motor de observaciones y su integración con el trabajo humano.md*.  
**Carácter:** análisis y propuesta de cambios; no se han implementado en este turno.

## 1. Conclusión principal

**La simplificación más importante es dejar de exigir una revisión terminada para entregar el Excel con el que se realiza esa revisión.**

El motor tiene una función de asistencia: calcula, detecta y propone. El funcionario revisa **uno por uno todos los registros**, consulta lo que corresponda en RUS, decide y registra allí la observación definitiva. El Excel conserva una constancia de ese trabajo —texto, fecha y clasificación administrativa— y luego alimenta correos, estadísticas e informes.

NuRus debe facilitar ese circuito. No debe convertirse en un segundo RUS ni exigir que se repita dentro de su interfaz toda la revisión ya efectuada en el sistema institucional y en Excel.

La auditoría técnica encuentra un núcleo aprovechable: reglas y textos históricos conservados, procedencia, almacenamiento y exportación sobre copia. Pero el recorrido publicado coloca la aprobación antes de la exportación y no cierra el retorno del Excel revisado. **Esta es una desalineación funcional concreta, además de los errores de lectura encontrados.** Puede corregirse conservando la arquitectura, sin reiniciar el proyecto.

## 2. Qué representa cada elemento

| Elemento | Qué representa | Qué no acredita por sí solo |
|---|---|---|
| Fuente descargada de RUS | Datos disponibles en el momento de descarga | Estado actual completo de la causa |
| Propuesta del motor | Resultado automático de reglas aplicadas a esa fuente | Que un humano revisó o registró la observación |
| Observación en RUS | Registro oficial del trabajo humano, según la definición del usuario | Que NuRus la haya leído o verificado remotamente |
| Excel revisado | Constancia operativa mantenida por el funcionario | Una sincronización automática con RUS |
| TT, CC y RES | Registro administrativo para producción, carga y resoluciones conforme a su definición institucional | Complejidad, necesidad automática de actuar o existencia de un archivo concreto |
| Correo, Word o estadística | Producto derivado de la constancia revisada y de una selección explícita | Envío de correo, firma de una resolución o gestión realizada en otro sistema |

**Regla de precedencia:** si la constancia Excel difiere de lo efectivamente registrado en RUS, el usuario debe reconciliarla con RUS. NuRus puede advertir diferencias en los archivos que conoce, pero no decidir qué quedó en un sistema al que no está conectado.

El Excel sigue siendo un insumo legítimo para productos posteriores. La necesidad de reconocer a RUS como fuente oficial no obliga a integrar su API ni a automatizar su modificación.

## 3. Cómo conversa el informe adjunto con esta definición

### Propuestas que se mantienen

- Separar propuesta automática y constancia humana final: evita sobrescribir trabajo y permite estudiar correcciones.
- Mantener datos, reglas y procedencia: NuRus ya conserva parte importante de esta estructura.
- Representar varias reglas y varias actuaciones por registro; una causa puede tener más de un NNA, programa, medida o revisión.
- Separar sugerencia, producto generado y gestión realizada externamente.
- Conservar decisiones anteriores y mostrar diferencias entre períodos.
- Usar las correcciones humanas para revisar reglas, sin cambiarlas automáticamente.
- Mantener libertad para editar el texto. El motivo de una corrección ordinaria puede ser opcional; las excepciones que requieren documentación siguen requiriéndola.

### Propuestas que requieren ajuste

| Propuesta del informe | Ajuste necesario | Objetivo |
|---|---|---|
| Revisar solo lo cambiado cuando coinciden hashes | El hash permite comparar datos, pero **no exime de revisar cada registro**, como exige el usuario. RUS puede contener información que el Excel no refleja | Reducir búsqueda, no eliminar control humano |
| Bloquear exportación hasta resolver todas las incidencias | Distinguir propuesta para revisión de constancia lista para productos. Las incidencias deben poder salir en el primer Excel claramente marcadas | Evitar invertir el orden del trabajo |
| RES no coincide con menciones de proyectos | La discordancia es un indicio a interpretar, no prueba automática de error: texto sugerido, gestión y registro administrativo son cosas distintas | Evitar falsas alertas y conteos |
| Inferir estado final porque no cambió el texto | Texto idéntico puede estar sin revisar. Hace falta una declaración de revisión independiente | No contar propuestas como trabajo ejecutado |
| Una “tasa de aceptación automática” sobre todo lo generado | Medir aceptación **sin cambios entre revisiones confirmadas**; informar pendientes aparte | No confundir cobertura de revisión con calidad del motor |
| Agregar escalas de complejidad | Posponer; TT/CC no deben reutilizarse con ese significado | Evitar ampliar el producto antes de cerrar su flujo básico |
| Memorizar excepciones para no repetirlas | Conservar antecedentes y vigencia; mostrarlos en la revisión siguiente. No arrastrar aprobación ni suprimir controles automáticamente | Reutilizar contexto sin perpetuar decisiones obsoletas |

### Límites de la evidencia del documento

El informe adjunto declara 2.564 observaciones válidas, 1.107 combinaciones tribunal–RIT, discrepancias de RES, fechas no utilizables y 23 celdas con `#REF!`. **Esas cifras son resultados informados por el documento, no recalculados en esta auditoría.** El consolidado de junio a septiembre al que remite no está adjunto a este último mensaje; sus identificadores de citas pertenecen a otro análisis y no permiten comprobar aquí las celdas citadas.

Las cifras sirven para orientar casos de prueba y aclarar el modelo. No deben incorporarse a estadísticas de NuRus como si se hubieran recalculado desde la fuente. Los errores que sí se reprodujeron con los archivos disponibles están identificados en la auditoría técnica.

## 4. Flujo propuesto, con tres acciones principales

### A. Preparar Excel de trabajo

El usuario selecciona la fuente. NuRus identifica modalidades, aplica reglas, conserva la estructura y genera el Excel de propuestas. Puede incluir incidencias pendientes y excluidos coloreados. Indica claramente que todavía requiere revisión humana.

**No exige aprobar cada caso para obtener este archivo.** Tampoco llena TT/CC/RES como si la revisión ya se hubiera realizado.

### B. Incorporar Excel revisado

El funcionario revisa cada registro en RUS, registra allí la observación y mantiene su constancia en Excel. Después carga esa copia en NuRus. El programa identifica cambios, conserva el original y presenta solo problemas que afectan la incorporación o los productos posteriores.

No se exige reescribir el texto ni volver a pulsar “aprobar” en cada fila dentro de NuRus. Puede existir una confirmación de alcance: el usuario declara qué filas ya revisó y registró en RUS. Esa confirmación **no ejecuta** revisión por lotes en RUS ni permite omitir registros; solo evita duplicar la documentación del trabajo terminado.

### C. Preparar productos

NuRus utiliza la constancia incorporada para preparar correos, estadísticas, informes y los proyectos que correspondan. Filtra por modalidad, período, tribunal, programa y decisiones administrativas explícitas. El usuario revisa el producto resultante.

La preparación del producto y su materialización quedan diferenciadas. Se conservan la prohibición de enviar correos, la CC institucional y los controles de identidad/adjuntos. Si un proyecto necesita prepararse durante la revisión, puede hacerse como propuesta vinculada al caso, sin dar por concluida la observación en RUS ni contabilizar la gestión automáticamente.

La pantalla inicial puede limitarse a **“Preparar Excel”, “Cargar revisión” y “Preparar productos”**. Historial y configuración quedan disponibles sin ocupar el recorrido principal. Una vista por registro es una ayuda opcional de consulta/copia, no un trámite duplicado obligatorio.

## 5. Contrato mínimo de datos y del Excel

No conviene llenar la planilla de columnas nuevas. Mantener los campos operativos conocidos y guardar los detalles técnicos en metadatos o en la base local.

| Campo / información | Tratamiento recomendado |
|---|---|
| RIT, tribunal, NNA, programa, modalidad | Conservar su contexto completo. No deduplicar por RIT solamente |
| OBSERVACION | Campo operativo editable. En una salida de propuesta aún no es constancia final; en una copia revisada representa lo que el humano declara haber registrado |
| Propuesta original | Conservarla en el lote/metadatos, vinculada por ID; no volver a calcularla para sustituir una edición humana |
| FECHA_OBS | Fecha efectiva de la observación humana; no la fecha de generación del motor. Preservar las fechas existentes |
| Fecha de análisis | Guardar por separado, preferentemente como metadato del lote |
| TT / CC / RES | Conservar valores originales y definición administrativa. No asignar por reglas del motor ni convertir `*` o vacío en cero silenciosamente |
| Estado de revisión | Diferenciar propuesta pendiente, revisión declarada, revisión con incidencias y registro pospuesto. Puede mantenerse como una columna de estado compacta |
| Registro en RUS | Declaración del usuario, con fecha/alcance; no etiquetarla “verificado en RUS” sin integración y evidencia |
| Identidad técnica | ID por registro, vínculo a fuente/hoja/fila/hash; metadatos protegidos frente a cambios accidentales, no tratados como prueba de autenticidad |
| Productos derivados | Tipo, selección de registros, versión de constancia, plantilla y recibo/hash del producto |

Hay dos conceptos llamados CC: **la columna administrativa de carga del Excel** y **la copia de destinatarios del correo**. Deben permanecer separados en nombres internos, estadísticas y pantalla. La obligación de copiar a UCC no depende del valor administrativo de CC.

La columna RES no debe reemplazarse por un ciclo de estados técnicos. Si se necesita saber si un Word fue generado o revisado, guardar ese estado aparte. La interpretación exacta de valores especiales de TT/CC/RES debe quedar en un diccionario de datos; no se inventa en una migración.

## 6. Cambios concretos, objetivo y paso a paso

### F01. Separar exportación de propuesta y exportación de constancia · prioridad inmediata

**Estado actual:** `export_preserved_workbook` exige `db.get_snapshot`, que corresponde a un lote aprobado. La GUI exige aprobar antes de exportar.

**Objetivo:** obtener el insumo necesario para empezar la revisión.

1. Crear una ruta de exportación desde la evaluación inicial inmutable, con su fuente y reglas.
2. Conservar casos pendientes, advertencias, excluidos y estructura; distinguirlos visualmente y con texto.
3. Mantener otra ruta para constancia incorporada/productos desde un snapshot revisado.
4. No asignar revisión, fecha efectiva ni producción administrativa al generar la propuesta.
5. Permitir exportación diagnóstica de lo legible si una parte es incierta, sin anunciar una modalidad analizada cuando no lo fue.

**Éxito:** se obtiene Excel para trabajar sin aprobar casos; no se generan estadísticas de gestión ni comunicaciones automáticamente.

### F02. Resolver importación real y filas estructurales · prioridad inmediata

**Estado comprobado:** Informes de AGOSTO falla por alias de ingreso. Los totales de Espera/Cumplimiento quedan como casos bloqueados. La captura de 103 bloqueados requiere el `.xls` exacto para cerrar su diagnóstico.

**Objetivo:** no hacer que el usuario repare manualmente cada archivo o cada fila por un error del lector.

1. Separar los dos conceptos de fecha de ingreso y validar solo campos necesarios.
2. Detectar cabeceras con una lectura de la región inicial; ofrecer elección cuando haya ambigüedad.
3. Distinguir totales/fórmulas de casos incompletos, conservando ambos en el libro.
4. Agrupar errores de esquema en un diagnóstico único.
5. Registrar el mapa confirmado con coordenadas de origen.

**Éxito:** AGOSTO se procesa en las tres modalidades y sus sumas se conservan sin transformarse en casos.

### F03. Importar de vuelta la revisión humana · prioridad inmediata

**Objetivo:** que el trabajo realizado en RUS y Excel sea la entrada real de los productos posteriores.

1. Incorporar un ID técnico de revisión por fila y conservar el vínculo con la propuesta inicial.
2. Leer la copia devuelta sin recalcular ni reemplazar OBSERVACION.
3. Mostrar cantidad de filas sin cambios, modificadas, añadidas, ausentes y con identidad dudosa.
4. Comparar también FECHA_OBS y TT/CC/RES; conservar representación original y valores interpretados por separado.
5. Confirmar el alcance revisado/registrado en RUS y guardar una nueva versión de constancia.
6. Permitir incorporación parcial: pendientes permanecen identificados y no alimentan productos que requieran cierre.

**Riesgos y solución:** ordenar filas cambia coordenadas; usar ID y comprobar identidad. Una copia puede perder metadatos; ofrecer conciliación por múltiples campos, nunca solo RIT. Varias medidas de la misma persona pueden coincidir; exigir resolución explícita de esa ambigüedad. Un archivo ausente de una fila no acredita que el caso haya desaparecido de RUS.

**Éxito:** editar/reordenar Excel y volver a cargarlo conserva propuesta, edición y procedencia sin duplicación ni sobrescritura.

### F04. Simplificar confirmaciones y motivos · prioridad alta

**Objetivo:** no duplicar la revisión individual que el humano ya realizó.

1. Renombrar “aprobación de lote” como confirmación de la constancia incorporada cuando corresponda.
2. Permitir confirmación conjunta del alcance efectivamente revisado; mostrar pendientes por separado.
3. Mantener el motivo ordinario opcional o mediante categorías simples. Conservar las dos versiones del texto aunque no haya motivo.
4. Exigir responsable/motivo en la excepción de Cumplimiento y en resoluciones de conflictos materiales que lo requieran.
5. No interpretar “texto idéntico” ni TT como única prueba de revisión registrada en RUS.

**Éxito:** el usuario no tiene que repetir 500 aprobaciones en NuRus para documentar 500 revisiones hechas externamente, pero NuRus tampoco afirma haber revisado esas 500 filas por sí mismo.

### F05. Validar por etapa y por producto · prioridad alta

**Objetivo:** señalar problemas sin bloquear el insumo que permite solucionarlos.

1. En propuesta: errores de regla producen incidencia, conservando fragmentos válidos y fuente. No confundir falta de datos con “sin observaciones”.
2. En constancia: advertir fecha ausente/inválida, texto vacío con gestión declarada, fórmulas erróneas y clasificaciones no reconocidas.
3. En estadísticas: excluir del período fechado lo que no tenga fecha válida e informar ese grupo aparte; no eliminarlo del archivo.
4. En comunicaciones/Word: bloquear solo las condiciones materiales de su selección —identidad, destinatarios según política, causal, plantilla y adjuntos—, no filas ajenas al producto.
5. En cruce: registrar si C-10 se evaluó y documentar su excepción. Una hoja llamada Hoja2 con esquema inválido no equivale a cruce válido.

**Éxito:** se puede entregar propuesta con advertencias, pero nunca llamarla constancia completa ni usar datos incompletos como producción válida.

### F06. Vincular productos con la constancia humana, no con frases del motor · prioridad alta

**Objetivo:** generar insumos posteriores acordes con lo que el funcionario decidió.

1. Conservar `rule_ids` como explicación de la propuesta, no como mandato final inalterable.
2. Añadir selección explícita de actuaciones finales cuando sea necesaria; no extraerla únicamente buscando “se remite” en el texto.
3. Preparar correos por modalidad/grupo desde registros revisados seleccionados y constancia vigente.
4. Conservar separados RES administrativo, proyecto sugerido, Word generado y gestión externa declarada.
5. Si cambia la constancia, marcar productos pendientes de actualización; no sobrescribir documentos o borradores ya revisados.

**Éxito:** una propuesta descartada por el humano no genera automáticamente correo o proyecto. Un borrador guardado queda como borrador; no como enviado.

### F07. Conservar continuidad sin omitir la revisión de todos los registros · prioridad alta

**Objetivo:** reutilizar antecedentes y evitar perder correcciones.

1. Distinguir identidad de negocio, identidad física de la fuente y evento de revisión.
2. Calcular hash de archivo y huella de datos relevantes como conceptos separados.
3. Mostrar propuesta actual, constancia anterior y cambios; conservar las dos versiones.
4. Mostrar excepciones previas con vigencia/contexto; pedir revisión cuando corresponda.
5. No trasladar FECHA_OBS, TT/CC/RES ni un estado revisado al nuevo período automáticamente.

**Éxito:** el funcionario dispone del antecedente, pero cada registro del ciclo actual sigue sujeto a revisión. Mismo hash no significa que RUS no haya cambiado.

### F08. Estadísticas con semántica administrativa · después del retorno revisado

**Objetivo:** contar trabajo real sin inventar complejidad o ejecución.

1. Definir diccionario de TT/CC/RES y sus valores especiales a partir de las matrices/instrucciones vigentes.
2. Separar filas importadas, propuestas, revisiones declaradas, TT administrativo, carga, resoluciones administrativas y productos efectivamente generados.
3. Filtrar por FECHA_OBS, no por la fecha del motor o la última importación.
4. No deduplicar observaciones válidas solo porque comparten tribunal/RIT.
5. Informar registros fuera de período o fecha pendiente como grupos separados.
6. Medir aceptación sin cambios entre propuestas cuya revisión está confirmada; publicar su denominador y pendientes. Clasificar correcciones de redacción aparte de errores de regla cuando haya evidencia suficiente.

**Éxito:** totales reconciliables con Excel revisado y sin equiparar CC a complejidad ni RES a archivos Word. El consolidado citado en el informe será necesario para comprobar sus cifras exactas.

### F09. Mantener edición de contenidos sin convertirla en programación · después del flujo básico

**Objetivo:** que el usuario pueda mantener correos y modelos finales con control de versiones.

1. Conservar textos y matrices con versión y fuente.
2. Permitir editar contenido, asunto, firma y políticas de adjuntos mediante formularios acotados.
3. Separar esa edición de cambios de reglas/umbrales; estos requieren decisión funcional y pruebas.
4. Validar variables antes de publicar la plantilla.
5. Vincular el producto a la versión exacta utilizada.

**Éxito:** cambiar una plantilla no cambia archivos históricos ni exige modificar Python. No es necesario incorporar IA o aprendizaje automático para lograrlo.

### F10. Medir velocidad del proceso humano y cerrar aceptación · fase final

**Objetivo:** reducir tiempo total de trabajo, no únicamente segundos del motor.

1. Eliminar las 32 lecturas observadas en la prueba de cabecera desplazada y mover exportación fuera del hilo gráfico.
2. Medir tiempo de preparación, incorporación de revisión, resolución de incidencias y elaboración de productos.
3. Probar con un ciclo real autorizado: fuente → propuesta → revisión individual en RUS → Excel revisado → incorporación → producto.
4. Comprobar en Excel 2010 preservación de hojas/fórmulas/estilos y en Outlook clásico borradores, CC y adjuntos.
5. Documentar resultados reales, limitaciones y pendientes. No declarar que RUS fue verificado por NuRus si solo se cuenta con declaración humana.

**Éxito:** el recorrido completo cumple el trabajo descrito por el usuario sin duplicar tareas ni enviar correos.

## 7. Qué simplificar ahora y qué dejar fuera

**Simplificar ahora:** quitar la aprobación previa del Excel de propuestas; agrupar incidencias de esquema; no exigir explicación de cada edición ordinaria; reutilizar los campos conocidos; hacer opcionales los detalles de reglas y trazabilidad; permitir retorno del Excel como vía principal de incorporación del trabajo humano.

**Conservar:** todos los registros revisables, excluidos coloreados, reglas aprobadas, copia íntegra, versiones, hashes, historia de cambios, CC institucional y revisión de productos finales.

**No hacer:** duplicar RUS, sincronizarlo sin autorización específica, recalcular la constancia humana sobre una nueva propuesta, clasificar automáticamente carga/RES, omitir revisiones por hash, generar estadísticas desde propuestas ni incorporar un puntaje de complejidad antes de estabilizar el flujo.

La revisión por componentes sugerida en el informe puede ser una ayuda posterior. No debe convertirse en otra pantalla obligatoria que retrase la tarea principal.

## 8. Orden de implementación recomendado

1. **Desbloquear el insumo:** H01/H02 de auditoría y F01/F02 — lectura correcta y Excel previo a aprobación.
2. **Cerrar la vuelta:** F03/F04 — importar constancia y confirmar alcance sin repetir la revisión.
3. **Proteger significado y contenido:** F05/F07, correcciones de fechas, cruce, ficha FAE, fórmulas y archivos parciales.
4. **Completar productos:** F06/F08/F09 — usar constancia revisada, integrar módulos candidatos y estadísticas definidas.
5. **Aceptar el circuito real:** F10, junto con instalación y pruebas institucionales de la auditoría.

No es necesario construir todo el modelo de eventos y analítica antes de poder entregar el primer Excel útil. La separación entre propuesta, constancia y producto sí debe estar definida desde el principio para no perpetuar el error de orden actual.

## 9. [G-ESTADO] Actualización material

**Confirmado por el usuario:** RUS conserva la observación oficial; todos los registros se revisan individualmente; Excel conserva constancia y alimenta productos posteriores.  
**Cambio respecto de la auditoría inicial:** la exportación de propuesta no requiere revisión aprobada; los controles finales se aplican a la constancia y a cada producto.  
**Comprobado en código:** la ruta publicada exige snapshot aprobado para exportar; no incorpora el retorno completo del Excel revisado.  
**Pendiente:** F01–F10 en el orden indicado, junto con los defectos técnicos demostrados.  
**Limitaciones:** cifras del nuevo informe no recalculadas; RUS no consultado; aceptación con Office institucional no ejecutada.  
**Próxima acción:** conservar el orden técnico de la auditoría: corregir primero la ambigüedad de Informes y verificarla con la estructura real, implementando inmediatamente después la exportación de propuesta separada de la constancia aprobada.

El [informe de auditoría técnica](AUDITORIA_INTEGRAL_NURUS_20260909.md) contiene las reproducciones, fuentes, errores específicos y pruebas de aceptación que complementan este ajuste funcional.
