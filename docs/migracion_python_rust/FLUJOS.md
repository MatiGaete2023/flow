# Recorrido operativo · de la entrada al producto

Estos recorridos describen el comportamiento de referencia que otra IA debe reproducir. Las diferencias por implementación están en la matriz. Para seguir cada recorrido en código, comienza por la función indicada; las pruebas deben entrar por la misma ruta que usa el operador.

## 1. Obtener un lote desde Chrome · F17–F19

1. El usuario abre el Descargador y conecta la extensión de la misma entrega desde su propia pestaña. El escritorio comprueba versión y protocolos. El código de conexión es efímero. El modo Lite permite concentrarse en las consultas habituales.
2. Se elige pantalla, tribunales, pestañas/modalidades y fechas o mes/año según el perfil. El plan conserva la selección efectiva. Una consulta de Cumplimiento y una de Informes del mismo tribunal siguen siendo consultas diferentes.
3. Por cada consulta el proceso valida la selección y las respuestas, comprueba el archivo obtenido y registra su procedencia y huella. Un resultado vacío reconocido y una consulta que falló tienen estados distintos. Cuando se solicita PDF de consulta vacía, se conserva la selección correspondiente.
4. El lote guarda sus originales y progreso. Una cancelación detiene lo pendiente; retomar verifica el material previo antes de reutilizarlo. Aplicar los límites y las pausas del flujo específico, y distinguir fallas consecutivas de bitácora de la suspensión de históricos.
5. Con la opción de lecturas activada, asociar cada enlace al ingreso o causa que corresponda. Bitácora y calendario tienen alcance por ingreso. Historia y escritos de la causa pueden involucrar varios ingresos; la distribución al registro exige evidencia. Una página parcial o una identidad no verificable queda registrada con su motivo.
6. Presentar al usuario consultas obtenidas, pendientes y fallidas por tribunal. El historial de proceso conserva estados, códigos y conteos anónimos. La revisión offline de los lotes debe funcionar después de cerrar Chrome.

**Seguir en código:** `descargador/descargador.py` → `flujo_paquete.run` → `lotes.py`, `lectura_bitacoras.py`, `lectura_historia.py` → `resultados.py`. En Rust, `flow/src/captura_ui.rs` → `flow-captura` y `flow-trabajo/src/flujo.rs`.

## 2. Histórico, escritos y evidencia suficiente · F06, F20, F21

1. El Histórico por lote requiere activación expresa en la interfaz. Partir de una fila de listado verificada; validar titular y procedencia siguiendo el contrato del original. Compartir una lectura física de la misma persona conserva la asociación de cada consulta y tribunal.
2. Separar apertura, lectura reconocida y asociación. Una identidad contradictoria, un respaldo dañado o una lectura pendiente dejan el histórico incierto o fallido. El proceso parcial conserva las demás fuentes válidas.
3. Historia puede traer la tabla de escritos en su propia respuesta. Registrar los escritos visibles y su estado. Si la variante visible depende de otra carga y falta evidencia estructural, conservar diagnóstico anónimo y estado pendiente. La confirmación de esa variante requiere estructura ficticia equivalente o extracto anónimo.
4. Para bitácora, evaluar período solicitado, método de lectura, autor, etapa, identidad, consulta y cobertura. La existencia de una observación del centro puede acreditarse con evidencia positiva. Acreditar ausencia requiere cobertura suficiente de todo el intervalo.
5. Clasificar la fila según la regla de la versión y permitir revisión humana. Un paquete antiguo sin acreditación suficiente sigue siendo legible. La exportación de la revisión conserva conteos y clasificación en Excel y JSON.

**Ejemplo sintético:** mismo ingreso con tabla reconocida, intervalo completo y cero observaciones del centro; repetir con exactamente las mismas filas y sin respaldo de cobertura. El primer caso puede quedar «Sin observación». El segundo debe quedar «No verificable». Esta pareja revela el defecto aunque ambos ZIP abran sin error.

## 3. Formar un paquete y entregarlo al Asistente · F07, F22, F27

1. Desde el Descargador, seleccionar los lotes verificados y el tipo de entrega. Validar archivos y consulta antes de consolidar. Para el libro CSMP comprobar compatibilidad de encabezados, modos, tribunales y filtros; un lote parcial restringe esa salida.
2. Construir registros, consultas, incidencias y tablas opcionales del contrato. Cada fila debe conservar su clave de origen y relación con evidencia. Si hay un único ingreso compatible, asociar el calendario según el original. Con candidatos múltiples conservar la incertidumbre.
3. Construir el ZIP y su manifiesto. El XLSX consolidado es opcional en la salida Python y se agrega solo si el lote lo permite. Si no corresponde, explicar el motivo y conservar la entrega válida que permite el original.
4. Abrir en Trabajo mediante archivo, último paquete o devolución local según el flujo. Verificar rutas de miembros, tamaño, hashes, versión y capacidades requeridas antes de sustituir la sesión.
5. Combinar entradas por identidad contrastada. Si existen ediciones previas y la captura cambió, presentar conflictos y aplicar las decisiones como una transacción. Una cancelación deja intacto el trabajo anterior.

## 4. Analizar Espera, Cumplimiento e Informes · F01–F05

1. Desde Excel, detectar formato por contenido, hoja, encabezado, columnas y etapa. Desde un ZIP, reconstruir registros y relaciones. Fijar fecha de corte. Conservar fila física, valor original y tribunal.
2. Evaluar sobre el conjunto antes de filtrar la vista. El prepaso permite detectar duplicidades y acumulaciones. Mantener separados causa, ingreso, persona y programa. Las filas de informes o egresados no se convierten por proximidad en ingresos de intervención.
3. En **Espera**, evaluar derivaciones excluidas, edad, fecha de resolución y tiempo de espera según el tribunal y el programa. En **Cumplimiento**, revisar ingreso, vigencia, egreso, oído, curador y fichas aplicables. En **Informes**, calcular vencimiento y tratar los márgenes que correspondan a DCE. Leer días y comparadores de la configuración fijada: las guías históricas contienen valores de otras versiones.
4. Agregar solo el contexto disponible para la persona/ingreso. Historia puede aportar actuaciones de ingreso, búsqueda, curador, prórroga y escritos. Bitácora permite cotejar la última observación o reiteración; calendario aporta proyecciones. Separar avisos informativos de hechos que alteran la observación o las acciones.
5. Componer el texto desde el catálogo Python por grupos —constancia, acción del Centro, gestión del Tribunal y cierre— con deduplicación y puntuación. Conservar las reglas disparadas y las acciones aunque el texto final se edite. Comparar con el resultado Python como una unidad.
6. Mantener propuesta original, observación que quedará en la salida, incidencia y marca humana. Al consignar otro programa se recalcula la constancia correspondiente y se invalidan productos dependientes. Comprobar también deshacer, cambio de programa y recuperación de sesión.

**Ruta:** `P/work.py::Work.analyze` → prepaso → `P/motor/reglas_*.py` → `composicion.py` → `work.actions_for`. Destino: `flow-tabular`, `flow-trabajo`, `flow-motor`. En el prototipo el recorrido equivalente empieza en `trabajo.rs::Trabajo::abrir`.

## 5. Revisar, exportar y volver desde Excel · F08–F10

1. Filtrar por tribunal, estado o incidencia, abrir la ficha y editar los campos habilitados. Guardar la decisión asociada a su registro. Un cambio de filtro no altera la selección lógica del usuario.
2. Exportar la copia revisable reutilizando columnas que ya existen. Conservar observaciones y RES manuales conforme al original. Respetar hojas auxiliares, trazabilidad, TT, CC, FECHA_OBS y validaciones según el modo de exportación Python.
3. Una vez revisada la copia en Excel, leerla en un estado temporal. Cotejar identidades aunque el usuario haya reordenado las filas; no usar el número de fila como única clave.
4. Comparar base exportada, valor local y valor recibido para OBSERVACION, FECHA_OBS, TT, CC y RES. Ante dos modificaciones del mismo campo, pedir elección. Al cancelar no cambian las demás filas.
5. Aplicar todo el conjunto, actualizar la huella y volver a evaluar la vigencia de correos y resoluciones. Reabrir la sesión debe conservar las ediciones confirmadas y distinguir productos antiguos.

## 6. Correos, nóminas y Outlook · F11–F13, F15

1. Elegir modalidad, tribunales y alcance de filas. Determinar acciones desde Trabajo y el tipo de gestión. Resolver contacto por programa/tribunal y preparar asunto, texto, firma y CC fijo. Si falta dirección, permitir completarla.
2. Formar la nómina asociada, revisar filas incluidas y adjuntos. Conservar cambios humanos compatibles al preparar de nuevo. Un adjunto obligatorio ausente bloquea ese producto.
3. Antes de guardar, contrastar huella de las dependencias: filas, plantilla, contacto y decisiones pertinentes. Registrar la intención y consultar el recibo. Preparar el objeto COM, agregar los adjuntos y guardar exclusivamente como borrador.
4. Confirmar el resultado en el recibo. Si la operación pudo haberse realizado, presentar «incierto» y permitir que el usuario lo resuelva al revisar Borradores. Un reintento no debe crear un duplicado. El EML que ya ofrece Rust es una salida distinta y debe quedar identificado como tal.
5. En modo manual, el formulario recibe datos aportados por el usuario y utiliza los mismos controles de plantilla/adjuntos. La prueba comienza con Trabajo vacío.

## 7. Proyectos Word y entrega · F14–F16

1. Tomar las decisiones RES y agrupar por tribunal, RIT y tipo de proyecto. Incorporar a las personas que correspondan a la misma causa. Resolver tipo, matriz y nomenclatura con las prioridades del original.
2. Mostrar valores editables, duración, texto y campos pendientes. Si el origen contiene una observación humana, contrastar en prueba el valor que recibe la matriz. Una muestra de texto en pantalla no prueba el resultado DOCX.
3. Renderizar las matrices originales y componer el documento del lote conservando estilos, saltos, partes y relaciones. Aplicar los controles de completitud de la operación escogida. Verificar también el modo manual y la vista previa con Word donde corresponda.
4. En Resultados, listar los productos generados y previos, filtrar, abrir la ubicación, revisar incidencias y seleccionar qué entregar. Si se crea ZIP, reabrir manifiesto e índice y cotejar los bytes. Registrar productos generados sin contabilizar gestiones externas por el solo hecho de generarlos.

## 8. Meta 4 · F23–F24

1. Elegir carpetas de lote, tribunales y fecha de corte. Validar los respaldos y su identidad. Separar Residencia/FAE y Espera/Cumplimiento.
2. Por ingreso, seleccionar la última observación del centro cuya etapa sea Espera o Cumplimiento en el mes y hasta el corte. Comparar fecha y hora. Una observación de Informes posterior no sustituye a la válida.
3. Conservar el control de identidad, empate o lectura incompleta. Con registro actual en Cumplimiento y última etapa válida en Espera, conservar su ubicación y marcar la alerta. Aplicar exactamente el tratamiento diferenciado de divergencia de RIT y de nombre visible.
4. Producir planilla Meta 4, Registros, consolidados, bitácoras del mes y reporte según el origen. Al escoger plantilla, validar las cuatro hojas y su estructura antes de publicar.
5. Tras editar o quitar filas en Registros, regenerar en otra carpeta. Recalcular alerta y preservar los cambios humanos, corte y origen. Comprobar el resultado con lotes Python ya existentes y lotes Rust nuevos.

## 9. Anonimizar antes de compartir · F25

1. Elegir archivos/carpetas, destino, modo y opciones de fechas o semilla. Preparar un mapa coherente para todo el lote sin registrar la semilla. Trabajar sobre copias.
2. Detectar formato por contenido y procesar tablas, texto, JSON anidado, ICS, DOCX o ZIP con el adaptador específico; el modo conservador requiere revisión del texto libre. Separar miembros no admitidos y rechazo total según la opción.
3. Neutralizar identidades, nombres de archivos, propiedades, enlaces y fórmulas según formato. En un ZIP CSMP reconstruir relaciones, tamaños y huellas. Conservar coherencia de la misma identidad ficticia entre archivos.
4. Inspeccionar la copia antes de publicarla; ante rechazo imprimir categoría y conservar íntegros los originales. El informe debe permitir identificar qué se creó y qué requiere revisión sin exponer los valores sustituidos.

## 10. Instalación, configuración y retorno · F11, F26, F28

1. En desarrollo, construir con Cargo y lock; Python y Node intervienen en pruebas, oráculos y extensión. Localizar los datos que el operador necesita en runtime, incluidas matrices, configuración de usuario y extensión correspondiente.
2. En una cuenta Windows estándar, abrir el modo solicitado desde su ejecutable. Verificar operación local con un archivo ficticio; después probar integraciones de escritorio en el entorno autorizado.
3. Importar desde copia de la configuración anterior con previsualización de contactos, alias, textos y plantillas. La sesión y los originales siguen disponibles para volver a Python.
4. Probar recuperación tras cierre, reemplazo de versión y archivo bloqueado por Office. Registrar pasos reales, versiones y resultado. Dejar al usuario la aceptación en el PC institucional.
