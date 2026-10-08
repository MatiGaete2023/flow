# Auditoría integral de NuRus: fidelidad funcional, errores y aptitud operativa

**Auditoría y corte de pruebas:** 9 de septiembre de 2026. **Cierre documental:** 10 de septiembre de 2026.  
**Repositorio:** `MatiGaete2023/NuRus`.  
**Rama examinada:** `implementacion-plan-2026-09-08`.  
**Código auditado:** `afd82b80ae69375e1d2f6fc03f78c081b0bab45c` — versión declarada `0.2.0.dev1`.  
**Alcance de esta entrega:** diagnóstico, comprobaciones reproducibles y plan de corrección. No se modificó el comportamiento del programa durante esta auditoría.

**Aclaración funcional incorporada:** la observación oficial reside en RUS y el humano revisa todos los registros. El Excel de propuestas es un insumo previo a esa revisión; el Excel revisado es su constancia y la entrada para productos derivados. Por tanto, la exigencia de snapshot aprobado corresponde a esos productos posteriores, no a la primera exportación. El [informe de ajuste funcional](AJUSTE_FUNCIONAL_REVISION_HUMANA_20260909.md) desarrolla esta distinción y precisa el orden de implementación de esta auditoría.

## 1. Dictamen

**NuRus conserva el núcleo del objetivo, pero la versión publicada examinada todavía no está en condiciones de reemplazar íntegramente las herramientas anteriores en la operación institucional.**

No hay evidencia de que sea necesario desechar el proyecto ni reescribir todas las reglas. Los 25 textos de observaciones comparados coinciden con el catálogo del motor histórico. Con el mismo día de evaluación y el mismo cruce de datos, coinciden 95 observaciones de Espera y 760 de Cumplimiento de `AGOSTO.xlsx`, además de 262 del archivo de Cumplimiento aportado. Las diferencias restantes en esas comparaciones corresponden a filas de totales tratadas incorrectamente como registros; no acreditan una divergencia general de las reglas.

Sin embargo, existen errores reproducidos que impiden importar o aprobar archivos reales, validaciones que pueden aceptar resultados incompletos y funciones históricas que no están conectadas al flujo publicado. **Tener reglas que redactan “se remite correo” no equivale a haber preparado el correo; guardar texto en SQLite no equivale a entregar un Word o un borrador en Outlook.**

La arquitectura local —lector, motor, revisión, snapshot, productos y adaptadores— sigue siendo adecuada. La desviación se encuentra principalmente en la ejecución del plan: se ha avanzado más en controles y persistencia que en cerrar los productos finales y su uso cotidiano. Debe priorizarse el recorrido completo de archivos reales antes de agregar controles o pantallas opcionales.

**Uso actualmente defendible:** evaluación técnica y pruebas supervisadas de los módulos cubiertos.  
**Uso todavía no acreditado:** producción institucional completa, equivalencia integral con el RUS Engine instalado por el usuario y fidelidad nativa comprobada con Excel 2010/Outlook clásico.

## 2. Objetivo, invariantes y fuentes de autoridad

### [G-OBJETIVO]

Unificar el procesamiento de Espera, Cumplimiento e Informes, la preparación de comunicaciones/proyectos y el contador de correos en una herramienta local sencilla, editable sin cambiar código para los contenidos operativos, que entregue productos verificables y conserve la procedencia de los datos.

### [G-REGLAS] Invariantes vigentes

- No enviar correos: preparar y guardar borradores con revisión humana.
- No escribir directamente en SATURNO.
- Conservar archivo, hoja, fila y hash de origen. Exportar propuestas antes de la revisión; preparar productos posteriores desde la constancia revisada, sin sustituir la observación oficial de RUS.
- Conservar la estructura original de Excel, incluidas hojas, fórmulas, estilos, anchos y filtros. Las filas excluidas permanecen y se distinguen con colores.
- OB y Medidas vencidas no se procesan como las tres modalidades principales; conservarlas en la copia no significa analizarlas.
- Permitir Cumplimiento sin cruce mediante excepción documentada. La autorización general no reemplaza el registro de cada excepción.
- Incorporar `ucc_concepcion@pjud.cl` en copia en cada correo.
- Mantener el objetivo Windows 10, Python 3.12, Excel 2010 y Outlook clásico; someterlo a aceptación real.
- Conservar cambios ajenos y usar el repositorio indicado como referencia de integración.

### Fuentes examinadas y límites de autoridad

| Fuente | Uso en la auditoría | Límite |
|---|---|---|
| Código de NuRus en el commit indicado | Estado publicado verificable: lector, reglas, GUI, productos, exportación, almacenamiento, Outlook, instaladores y pruebas | No representa automáticamente lo instalado en el PC del usuario |
| `Asistente (2).zip`, especialmente `motor/`, `comunicaciones/`, `resoluciones/` y especificaciones | Referencia histórica aportada para el motor y sus productos | README y módulos muestran etiquetas de versión distintas; no hay identificación inequívoca de un ejecutable separado llamado RUS Engine |
| `Catalogo_Reglas_CSMP_v2_20260714.md`, dentro del ZIP | Especificación funcional consolidada; registra decisiones expresas y cambios al manual | Su declaración de aprobación es evidencia documental; no sustituye un acta institucional que no se haya aportado |
| `CSMP_2025_Fusionado(2).md` | Manual y plan: comunicaciones, periodicidad, identificación, adjuntos, cargas y límites de proyectos | Es el documento entregado, no una comprobación externa de su vigencia normativa |
| Creador de Correos y Contador de Correos originales | Funciones que la integración debe conservar o reemplazar explícitamente | No se ejecutó su conexión real a Outlook |
| `AGOSTO.xlsx` | Prueba de estructura y lectura con datos reales aportados | No se publican datos personales en este informe |
| `RUS_CUMPLIMIENTO_20260904_114132_856292.xlsx` | Salida representativa y estructura histórica | No se aportó el insumo exacto que la produjo ni su contexto completo de ejecución |
| Captura de `cc46_cga_amblistcump (82).xls` | Síntoma real: 103 filas bloqueadas y columnas obligatorias no reconocidas | El `.xls` exacto no está disponible; la captura no demuestra por sí sola dónde está su cabecera |
| Copia local `nurus-implementation` | Trabajo recuperable: comunicaciones, Word, contador y otros módulos | No está incorporada íntegramente a la rama auditada; sus pruebas son una suite distinta |

Los hashes de archivos y el entorno están en [fuentes_y_entorno.json](auditoria_20260909/fuentes_y_entorno.json). El código del repositorio debe consultarse en el commit auditado, no en una rama que puede seguir cambiando.

## 3. Qué se comprobó

### 3.1 Pruebas automatizadas y experimentos

| Verificación | Resultado | Interpretación correcta |
|---|---|---|
| Suite de la rama publicada, Python 3.12.14/Linux | **81 aprobadas, 0 fallidas** | Los casos existentes pasan; la suite no detecta todos los defectos que siguen abajo |
| GitHub Actions del mismo commit, ejecución `34372680288` | **success** | CI declara éxito; su configuración usa `windows-latest` y Ubuntu, no acredita Excel 2010 |
| Suite de la copia local candidata | **54 aprobadas, 4 omitidas** | Hay trabajo reutilizable; no prueba que pueda copiarse sobre la rama sin regresiones |
| Cabecera sintética desplazada a fila 14 | Detectada; primera fila de datos 15 | Comprueba ese ejemplo, no el `.xls` de la captura |
| Hoja cruzada `Hoja2` con columnas inválidas | Lote aprobado sin excepción; C-10 no evaluada | Defecto confirmado de control del cruce |
| Observación literal `=texto de prueba` exportada | Protegida en hoja visible; fórmula en `NURUS_TRAZABILIDAD!E9` | Defecto confirmado de tratamiento del texto nuevo |
| Fallo simulado en copia final de exportación | Queda archivo incompleto de 10 bytes | Defecto confirmado de publicación de archivos |
| Excel 2010 y Outlook institucionales | **No ejecutado** | Sigue siendo una limitación material |

El error inicial del entorno de auditoría —caché de bytecode incompleta en dependencias locales— se resolvió ejecutando con un prefijo de caché nuevo. No fue un fallo del código NuRus y no se confundió con los resultados de la suite.

### 3.2 Lectura de los Excel aportados

Evaluación fijada al **09-09-2026**. `reviewed` significa evaluado sin incidencia bloqueante, no aprobación humana ni producto creado.

| Archivo / modalidad | Filas reconocidas como registros | Estado devuelto | Resultado relevante |
|---|---:|---|---|
| AGOSTO / Espera | 96 | 95 reviewed, 1 blocked | El bloqueo corresponde a la fila de sumas 98 |
| AGOSTO / Cumplimiento | 761 | 756 reviewed, 4 excluded, 1 blocked | El bloqueo corresponde a la fila de sumas 763 |
| AGOSTO / Informes | No completa la lectura | Error de columna ambigua | Confunde dos fechas distintas para el campo `ingreso` |
| RUS_CUMPLIMIENTO / Cumplimiento | 263 | 255 reviewed, 7 excluded, 1 blocked | Fila de sumas 265 bloqueada y advertencia de ausencia de cruce |

Las tres filas de sumas contienen únicamente fórmulas `SUM` en X e Y. No son casos con identidad incompleta. Deben conservarse como estructura del libro y quedar fuera del conjunto de casos sometidos a aprobación.

### 3.3 Comparación directa con el motor histórico

Se ejecutaron las funciones históricas de cálculo y las nuevas reglas sobre los mismos datos, fijando el reloj histórico al mismo día. Para Cumplimiento de AGOSTO se suministró expresamente Informes como cruce a la comparación histórica: esto mide las reglas con igual entrada, **no** certifica el selector automático antiguo.

| Datos comparados | Observaciones idénticas | Diferencias detectadas |
|---|---:|---|
| AGOSTO / Espera | 95 | 1 fila de totales |
| AGOSTO / Cumplimiento | 760 | 1 fila de totales |
| RUS_CUMPLIMIENTO / Cumplimiento | 262 | 1 fila de totales |
| AGOSTO / Informes | Sin comparación nueva completa | El cálculo histórico recorre 302 filas del DataFrame; el lector nuevo aborta |

La comparación histórica de bajo nivel incluye una fila vacía adicional en cada una de las tres primeras tablas. NuRus omite esa fila vacía. No debe contarse como pérdida de un caso ni utilizarse para calcular un porcentaje engañoso de equivalencia.

Los **25 textos de observación** presentes en el catálogo nuevo son idénticos a sus equivalentes históricos. En cambio, el catálogo nuevo no conserva los mismos metadatos de confirmación y notas del catálogo histórico.

Se examinó la salida representativa como archivo de entrada para esta comparación. **No** se afirma haber reproducido su generación original del 4 de septiembre ni haber certificado equivalencia binaria de archivos.

Evidencia reproducible: [probes.py](auditoria_20260909/probes.py), [evidencia.json](auditoria_20260909/evidencia.json), [comparar_historico.py](auditoria_20260909/comparar_historico.py) y [paridad_observaciones.json](auditoria_20260909/paridad_observaciones.json).

## 4. Comparación funcional: objetivo, legado y producto actual

| Función | Referencia histórica / objetivo | Rama publicada examinada | Evaluación |
|---|---|---|---|
| Reglas Espera, Cumplimiento e Informes | Catálogo consolidado y funciones del motor | Reglas portadas y textos coincidentes | Conservar; corregir casos límite y entrada |
| Reconocer planillas reales | Lectura de tablas originales | Cabeceras desplazadas soportadas parcialmente; Informes real falla; totales bloquean | Bloqueador operativo |
| Mantener procedencia | Nuevo requisito explícito | Copia estable, SHA-256, hoja, fila, cruce y snapshot | Mejora que debe conservarse |
| Excel final con formato original | Nuevo requisito más exigente que el exportador histórico | Exportador nativo sobre copia; portable con advertencia | Aceptación nativa pendiente; contrato de columnas incompleto |
| FECHA_OBS, OBSERVACION, TT, CC, RES | Catálogo §8; TT/CC/RES manuales | Se agregan campos NURUS; no se reconstruye todo ese contrato | Falta compatibilidad del producto final |
| Correos por modalidad y destinatario | Creador original con seis familias; comunicaciones históricas agrupadas | Dos plantillas iniciales genéricas entre correo y resolución; preparación de texto | Integración funcional incompleta |
| Guardar borradores Outlook | Reemplazo controlado de herramienta histórica | Adaptador aislado y probado con dobles; GUI no lo llama | No equivale a correo entregado |
| Adjuntos, tablas y firma | Manual y matrices particulares | Adaptador publicado asigna `Body`; no incorpora adjuntos | Incompleto para comunicaciones que los requieren |
| Proyectos Word | Generador histórico y matrices por tribunal | Texto guardado en base; no exportador Word en la rama | Producto obligatorio pendiente |
| Editar contenidos sin código | Catálogo/plantillas históricas y solicitud del usuario | Versiones de plantillas editables; tipos y políticas no administrables completamente | Parcial |
| Contactos | Catastro, importación y resolución explícita | Servicio de importación; pestaña publicada de consulta | Falta conexión al flujo |
| Contador de enviados | Script original de lectura y Excel | Ausente de la rama; existe candidato local | Integración pendiente |
| Experiencia unificada | Elegir, revisar incidencias y obtener productos | Flujo base claro, pero salidas desconectadas y opciones de importación ausentes | Simplificar al completar funciones |

El motor histórico tampoco era perfecto: su exportador reconstruía tablas y descartaba elementos originales; incluía comportamientos que ahora se reproducen como defectos de validación; las matrices de resolución no acreditan cobertura completa para todos los tribunales. La migración debe conservar lo correcto y corregir lo defectuoso, no copiarlo indiscriminadamente.

## 5. Manual CSMP frente al catálogo consolidado

### 5.1 Diferencias deliberadas que no son regresiones de NuRus

El catálogo del 14 de julio se presenta como especificación definitiva y documenta decisiones posteriores al manual. Sus §§3, 5, 8, 11 y 12 permiten distinguir lo deliberado de una omisión:

- DCE: umbral de 30 días para correo, sin proyecto. El catálogo reconoce expresamente que la indicación del manual de comunicar sin importar días no se aplica de esa manera en la práctica descrita por el usuario.
- Espera no DCE: Laja/Mulchén desde 30 días; Tomé correo desde 30 y proyecto desde 60.
- Próxima mayoría de edad: ventana de 1 a 60 días.
- Cumplimiento próximo a vencer: 0 a 45 días; Informes por vencer: 0 a 30.
- C-06 y los hitos mensuales internos fueron eliminados expresamente. C-10 toma el dato del cruce.
- No agregar ficha ambulatoria ni cálculo automático de carga/sin carga. TT/CC/RES se completan manualmente.
- Con duplicados del cruce se conserva la última coincidencia según la regla histórica aprobada.

**Acción:** conservar estas decisiones para la migración. Relacionarlas con su fuente y, para la aceptación institucional, con la instrucción o acta aplicable. No volver a implementar C-06 ni alterar umbrales basándose únicamente en recuerdos o en una versión anterior del manual.

### 5.2 [CONFLICTO_ABIERTO] Aspectos que requieren delimitación operativa

| Asunto | Evidencia y efecto | Resolución propuesta |
|---|---|---|
| Mayoría de edad | El catálogo permite observar “se sugiere egresar”; el manual, en el apartado de proyectos/egreso de mayores de edad, exige la resolución judicial correspondiente para preparar ese proyecto | Mantener la observación aprobada. Exigir fundamento judicial para el proyecto; edad por sí sola no habilita una resolución |
| No seguimiento y medidas vencidas | E-01 corta la evaluación; el manual distingue el aviso al tribunal sobre vencimiento de ciertas derivaciones no seguidas | Separar el aviso/manual de medidas vencidas del motor de las tres modalidades; dejar su activación y evidencia definidas, sin reabrir E-01 automáticamente |
| “Se remite” en la observación | Texto histórico aprobado; NuRus puede producirlo antes de que se cree o gestione la comunicación | Mantener texto y registrar estado real de gestión por separado. No registrar envío ni actividad consumada basándose en ese texto |
| Hoja de cruce siempre disponible | Era un supuesto del catálogo, modificado expresamente por el usuario al permitir excepción | La decisión reciente prevalece; registrar ausencia, responsable, motivo, alcance C-10 no evaluado y snapshot |
| Observación en todos los registros | Manual pide observaciones; Informes fuera de la ventana de 30 días devuelve texto vacío en ambos motores | Mostrar “sin regla aplicable en este corte” como estado de evaluación, sin inventar una gestión. Definir presentación final con el responsable funcional |

El manual aportado exige identificación de NNA y programa, CC institucional, comunicaciones generales y adjuntos en supuestos específicos. Véanse sus apartados de comunicaciones, alrededor de las líneas 467–489 del Markdown, y el de proyectos/egreso, alrededor de la línea 603. Estas obligaciones se traducen en los controles y productos de las acciones siguientes; no basta con que una plantilla mencione que se adjunta algo.

## 6. Hallazgos y acciones detalladas

Prioridades: **P0** bloquea la entrega correcta de un producto requerido o permite aprobar información materialmente incompleta; **P1** debe resolverse antes de la aceptación integral; **P2** mejora sostenibilidad o rendimiento y se ejecuta después de asegurar corrección. No son puntuaciones de vulnerabilidad.

### H01 — Ambigüedad falsa en Informes y selección insuficiente de entrada · P0

**Evidencia:** `rus/columns.py:MODE_COLUMNS`, `reader.py:_resolve_header_row/read_workbook`. `AGOSTO.xlsx` falla por las columnas `FECHA INGRESO` y `FEC. INGRESO EFECTIVO`. El cálculo actual de Informes no necesita `ingreso` para I-01/I-02. El mensaje propone selección explícita, pero la GUI no ofrece un mapeador de columnas.

**Causa y efecto:** el mapeador aplica validación global sobre alias de un campo opcional y semánticamente ambiguo. Una columna ajena al cálculo bloquea toda la modalidad. El lector también puede pedir selección de hoja sin que la pantalla la permita.

**Objetivo:** importar archivos reales sin confundir conceptos ni obligar a editar el Excel.

**Pasos:**

1. Separar fecha de ingreso de causa y fecha de ingreso efectivo; conservar ambas columnas originales.
2. Validar los campos consumidos por cada regla/producto. Una ambigüedad no utilizada no debe detener el cálculo principal.
3. Agregar selección avanzada de hoja, fila de cabecera y mapeo, visible solo cuando la detección no sea inequívoca.
4. Persistir el mapeo confirmado y mostrar una pequeña previsualización con identidad y procedencia antes del análisis.
5. Si faltan columnas estructurales, detener la importación con un diagnóstico único. No crear centenares de incidencias idénticas por fila.

**Verificación:** AGOSTO/Informes debe completar lectura, mantener ambas fechas y evaluar contra la referencia histórica. Probar encabezados repetidos, dos tablas candidatas, selección manual y archivos sin tabla. El `.xls` exacto de la captura sigue siendo necesario para cerrar ese incidente particular.

### H02 — Filas de sumas interpretadas como casos · P0

**Evidencia:** `reader.py:_records` solo descarta filas completamente vacías. Las fórmulas `X98/Y98`, `X763/Y763` y `X265/Y265` de los archivos aportados generan registros bloqueados por `REQUIRED_DATA_MISSING`.

**Objetivo:** conservar totales y fórmulas sin exigir que se aprueben como personas.

**Pasos:**

1. Delimitar la región de datos utilizando estructura, encabezados, identidad y fórmulas, no solo valores calculados.
2. Clasificar filas como caso, estructura del libro o fila dudosa. Una fila con fórmulas de suma exclusivamente fuera de los campos de caso es estructura.
3. Mantener esas filas intactas en el libro; no asignarles observación de una persona ni incluirlas en el contador de casos.
4. Mantener revisión humana para filas parcialmente identificadas: no descartarlas silenciosamente por carecer de RIT.
5. Mostrar contadores de casos y excluidos; los elementos estructurales pueden quedar en el diagnóstico avanzado.

**Verificación:** los tres totales aportados no bloquean aprobación; sus fórmulas son idénticas en la salida. Casos incompletos reales siguen visibles. No se usa “excluir todo lo bloqueado” como solución.

### H03 — Cruce inválido permite aprobación sin excepción · P0

**Evidencia reproducida:** una `Hoja2` con una columna `nota` produce `CROSS_MAPPING_MISSING`, se omite C-10 y el lote se aprueba sin excepción. `database.py:approve_batch` comprueba `CROSS_SHEET_NOT_SELECTED`, pero no ese estado. `reader.py` prioriza el nombre Hoja2 sin exigir un esquema completo. Un cruce vacío también necesita distinguirse de uno inválido.

**Objetivo:** que toda evaluación parcial de Cumplimiento tenga un estado inequívoco y auditable.

**Pasos:**

1. Sustituir el control por prefijos de mensajes por un estado estructurado: válido, válido sin coincidencias, ausente, incompleto o ambiguo.
2. Validar esquema aunque la hoja no contenga registros; detectar cabecera del cruce de manera independiente.
3. Para ausencia, mantener la excepción ya autorizada. Para cruce incompleto, pedir corrección; si el responsable decide continuar sin cruce, registrar esa decisión y su alcance expresamente.
4. No resolver dos cruces posibles mediante una excepción silenciosa: seleccionar el correcto.
5. Vincular estado, hoja, cabecera, motivo y responsable al snapshot y a su exportación.

**Verificación:** la sonda de cruce inválido deja de aprobar sin control. Probar hoja válida vacía, sin coincidencias, incompleta, ausente, ambigua y con encabezado desplazado. La excepción de cruce no debe levantar errores ajenos a C-10.

### H04 — Validación no distingue esquema, identidad e incidencias de regla · P1

**Evidencia:** `service.py:_precheck/evaluate_batch`. Faltantes obligatorios anulan la observación completa; cualquier incidencia bloquea la fila. El catálogo §1.1/G-05 permite omitir una regla incompleta conservando los demás fragmentos. Un tribunal desconocido debe impedir sus reglas dependientes, no inventar un tribunal. Tampoco existe una exigencia transversal suficiente de identidad para todo producto.

**Objetivo:** evitar tanto la aprobación de casos anónimos como la revisión manual innecesaria de información válida.

**Pasos:**

1. Definir un único contrato por modalidad y requisitos por regla; eliminar las listas obligatorias duplicadas del lector y servicio.
2. Separar error de archivo, dato esencial de caso, advertencia de regla y ausencia legítima.
3. Conservar fragmentos calculados válidamente; señalar cuál no se evaluó y por qué.
4. Exigir identidad suficiente antes de producir comunicaciones o proyectos; permitir lectura diagnóstica sin falsificar datos.
5. Registrar decisiones de corrección/revisión y mantener la invalidación del snapshot al cambiar contenido.

**Verificación:** casos con una fecha faltante conservan reglas independientes; casos sin identificación no llegan a productos nominales. Un tribunal no reconocido nunca se reemplaza por Laja, Mulchén o Tomé por defecto.

### H05 — Resultados “sin observaciones” o vencimiento incoherente · P0

**Evidencia reproducida:** con `dias_egresar=10` y egreso proyectado ausente, `evaluate_compliance` produce C-09 “sin observaciones”, sin incidencia. Con días de cumplimiento negativos y fecha de egreso futura produce “vencida desde” una fecha futura, también sin incidencia. La lógica problemática ya existe en el código histórico.

**Objetivo:** impedir conclusiones positivas basadas en datos insuficientes o contradictorios.

**Pasos:**

1. Validar presencia y formato de egreso proyectado cuando C-04/C-05 necesita ese dato.
2. Añadir comprobación entre signo de días, fechas y fecha de corte, considerando que una planilla puede haberse descargado días antes.
3. Ante contradicción, conservar valores originales y marcar conflicto; no recalcular ni sustituir datos administrativos sin dejar evidencia.
4. Impedir el cierre C-09 cuando falta información necesaria para descartar una incidencia.
5. Añadir incidencias explícitas para fechas relevantes inválidas; mantener formatos y épocas Excel definidos.

**Verificación:** las dos sondas dejan de figurar como revisión limpia. Probar 0/1/30/45/46 días, fechas ausentes, texto inválido, futuro y fecha de corte. El catálogo consideraba imposible el segundo caso: esta reproducción justifica reabrir ese supuesto técnico, no cambiar silenciosamente el plazo de negocio.

### H06 — Confundir columna de ficha FAE ausente con ficha inexistente · P1

**Evidencia:** `rules.py:evaluate_compliance`, C-08. La condición no exige `fae_column`; una planilla sin esa columna afirma que la persona no tiene ficha. El catálogo C-08 exige que la columna exista. Es un defecto heredado, reproducido con un programa FAE sintético.

**Objetivo:** no afirmar una ausencia documental que la fuente no permite comprobar.

**Pasos:** comprobar existencia de la columna; distinguir columna ausente de celda vacía y de fecha inválida; generar la observación solo en el supuesto aprobado; informar “no evaluable” en los otros casos; conservar las reglas complementarias válidas.

**Verificación:** FAE/FAS con y sin columna, con fecha, fecha inválida y celda vacía, a 120 y 121 días. No agregar ficha ambulatoria ni ramas de antigüedad rechazadas.

### H07 — Texto nuevo convertido en fórmula en trazabilidad portable · P1

**Evidencia reproducida:** `exports.py:_trace_rows/_portable_preserved` incorpora cadenas con `trace.append(list(values))`. El texto sintético `=texto de prueba` queda como fórmula en E9 de la hoja oculta, aunque la celda visible está protegida.

**Objetivo:** conservar literalmente los textos introducidos por NuRus sin modificar las fórmulas originales del usuario.

**Pasos:** centralizar escritura segura de texto nuevo; aplicarla a observación, motivo, nombres y metadatos de trazabilidad; conservar números como números y fórmulas originales como fórmulas; comprobar también los demás exportadores y adjuntos que se integren.

**Verificación:** entradas que empiezan por `=`, `+`, `-` o `@` se guardan como texto donde corresponde, incluso en hojas ocultas. Las fórmulas originales SUM siguen intactas. La prueba demuestra creación de fórmula, no explotación ni ejecución de un ataque.

### H08 — Archivo final incompleto después de un fallo de escritura · P1

**Evidencia reproducida:** en `exports.py:export_preserved_workbook`, `target.open('xb')` seguido de `copyfileobj` deja el destino parcial cuando falla la copia. La sonda aisla ese último paso y deja 10 bytes; la limpieza solo elimina el temporal.

**Objetivo:** que una exportación fallida no se confunda con un producto válido ni impida un reintento controlado.

**Pasos:** completar y comprobar primero el temporal; publicar con una operación adecuada al sistema que no sobrescriba archivos ajenos; si se conserva la copia exclusiva actual, registrar que el destino fue creado por este intento y limpiarlo en el error; persistir recibo/hash solo después del éxito; mostrar la ruta únicamente como producto terminado cuando corresponda.

**Verificación:** falta de espacio simulada, destino ocupado, archivo bloqueado, interrupción durante copia y reintento. No eliminar un archivo que existía antes del intento ni sobrescribirlo para “resolver” el fallo.

### H09 — Contrato del Excel final distinto del esperado · P0

**Evidencia:** exportación histórica/catálogo §8: `FECHA_OBS`, `OBSERVACION`, `TT`, `CC`, `RES`. Exportación nueva: `NURUS_OBSERVACION`, `NURUS_ESTADO_REVISION` y trazabilidad. Si ya existe `OBSERVACION`, queda con su contenido anterior; un consumidor puede leerla y no la revisión nueva. La conservación de originales es una mejora, pero no resuelve por sí sola qué columna representa el producto vigente.

**Objetivo:** entregar primero Espera, Cumplimiento e Informes para revisión, y después utilizar su constancia revisada como insumo, sin destruir evidencia anterior.

**Pasos:**

1. Definir perfiles de propuesta y constancia revisada. La fecha de análisis no debe convertirse automáticamente en FECHA_OBS; esta última corresponde a la revisión/registro humano.
2. Conservar las columnas originales y sus valores; no reemplazar una observación previa silenciosamente. Los consumidores de NuRus deben usar el campo de revisión declarado por el perfil.
3. Incorporar los campos operativos faltantes en un espacio de revisión claramente identificado. TT/CC/RES siguen siendo de llenado manual; no reutilizar sus nombres con un significado nuevo.
4. Preparar las tres modalidades desde una carga común y generar los productos requeridos por modalidad sobre copias, conservando las demás hojas. No presentar OB como cuarta modalidad de análisis.
5. Mantener excluidos con color y estado textual; conservar totales y estilos originales. Verificar referencias de fórmulas antes de insertar columnas dentro de una región usada por fórmulas.
6. Registrar manifestación de producto: modalidad, archivo, hash, snapshot, perfil y versión.

**Verificación:** comparar hojas, valores, fórmulas, anchos, filtros, combinaciones, validaciones, filas ocultas y estilos; comprobar qué campo consume el creador de productos. El backend portable mantiene su advertencia de fidelidad reducida; no sustituye la aceptación nativa.

### H10 — Correos y proyectos todavía no son productos completos · P0

**Evidencia:** `app.py:open_product_dialog` termina en `persist_approved_product`; no llama a `outlook.save_draft`. No hay exportador Word en la rama. El adaptador publicado no agrega adjuntos y usa cuerpo de texto plano. En el legado hay generador Word, plantillas HTML, agrupación y creador de seis familias de comunicaciones.

**Objetivo:** cerrar “revisión aprobada → producto materializado” sin afirmar acciones inexistentes.

**Pasos:**

1. Recuperar por comparación los módulos candidatos `communications`, `delivery`, `resolutions`, `product_ui` y sus dependencias; no copiar la carpeta completa encima del repositorio.
2. Definir acciones por regla, modalidad, tribunal y destinatario. DCE no habilita proyectos de espera; mayoría de edad sola tampoco debe habilitar un egreso judicial.
3. Migrar las matrices definitivas: asunto, cuerpo, firma, destinatarios, tabla y adjuntos requeridos; registrar origen/versión.
4. Generar vistas previas de grupos explícitos, aprobar el contenido final y materializar Word o borrador.
5. Exigir adjuntos cuando el cuerpo o la política los requiera y vincular su hash al producto aprobado.
6. Guardar ruta/hash del Word o EntryID/StoreID de Outlook. Ante Save incierto, conciliar antes de reintentar.

**Verificación:** un recorrido por cada tipo produce el archivo o borrador esperado, sin envío, sin destinatarios de otro grupo y sin duplicación en reintento. Comparar formato Word con matrices, no solo texto. Los ejemplos genéricos no se presentan como modelos institucionales aprobados.

### H11 — Destinatario y CC editados pueden no coincidir con lo aprobado · P1

**Evidencia de código:** `app.py:prepare_preview` captura destinatario/CC; `approve_and_save` actualiza asunto y cuerpo, pero no vuelve a leer esos dos campos. Por ello el objeto persistido puede mantener valores anteriores a la edición visible. No se ejecutó esta interacción en GUI institucional.

**Objetivo:** que se apruebe exactamente lo que el usuario está viendo.

**Pasos:** capturar todos los campos finales al aprobar; revalidar destinatarios, CC obligatoria, cuerpo y adjuntos; invalidar aprobación al editar cualquier componente; mostrar un resumen compacto del producto definitivo; persistir esa misma versión antes de materializar.

**Verificación:** editar destinatario y CC después de la vista previa, aprobar y comparar pantalla, SQLite y borrador de prueba. La CC institucional permanece una sola vez. La edición no debe reintroducir variables sin resolver ni conservar una aprobación antigua.

### H12 — Configuración sin código y gobierno de plantillas incompletos · P1

**Evidencia:** el editor publicado modifica versiones existentes; no cubre completamente alta de tipos, políticas, variables, activación ni modelos Word. `catalog.py` carga texto y luego `service.py` calcula su hash leyendo de nuevo: son dos lecturas que podrían representar versiones distintas si se edita entre ellas. El catálogo histórico tenía metadatos de confirmación que no se conservaron.

**Objetivo:** permitir cambios operativos controlados sin reprogramar reglas.

**Pasos:** separar textos editables de reglas/umbrales; incorporar importación y exportación de paquetes de plantillas con validación de campos; definir borrador/publicada/retirada; conservar versión anterior y autor/motivo; cargar bytes, validar y calcular hash sobre la misma lectura; vincular productos a esa versión; exigir migración explícita cuando cambia el conjunto de variables.

**Verificación:** plantilla con campo faltante no se publica; una versión nueva no cambia productos antiguos; edición concurrente no produce hash ajeno al texto. No habilitar código arbitrario ni alterar plazos aprobados desde un editor de cuerpo de correo.

### H13 — Contactos y agrupación todavía no resuelven el trabajo cotidiano · P1

**Evidencia:** pestaña Contactos publicada de consulta; servicio de importación no conectado a un recorrido completo. La preparación desde snapshot exige valores únicos y la selección de la tabla es individual, sin el agrupamiento operativo del legado. La política de CC ya está centralizada y debe conservarse.

**Objetivo:** preparar comunicaciones por grupo correcto con pocas acciones y control de destinatarios.

**Pasos:** importar con vista previa de altas/cambios/conflictos; resolver nombre canónico y alias aprobados; separar destinatario de tribunal y de programa; agrupar solo casos aprobados por las claves del tipo de correo; presentar cantidad de casos y dirección del grupo; permitir corrección explícita antes de aprobar.

**Verificación:** nombres homónimos, alias, contacto ausente, CC duplicada, dos tribunales en un archivo y programas parecidos. No usar coincidencia difusa para escoger silenciosamente una dirección. No exigir destinatario completo para un borrador si la política vigente permite completarlo manualmente, pero mostrarlo como pendiente.

### H14 — Contador de correos no integrado en la rama · P1

**Evidencia:** el script original recorre Enviados, omite objetos no MailItem y exporta destinatario/fecha/asunto a una ruta fija. La rama no incluye esa función; la copia local contiene `adapters/sent_mail.py` con período y cuenta.

**Objetivo:** completar la tercera herramienta sin mezclarse con el envío de comunicaciones.

**Pasos:** migrar el adaptador candidato y su exportación; seleccionar cuenta/período; usar consulta de solo lectura; informar omitidos, errores y límites; permitir destino nuevo; registrar cuenta y rango de consulta. Optimizar el filtrado de Outlook solo después de verificar fechas y ordenación con el buzón real.

**Verificación:** días inicial/final incluidos, carpeta vacía, mensajes no correo, errores parciales, consulta limitada y múltiples cuentas. Un reporte truncado debe decirlo; su cifra no debe presentarse como total completo del buzón.

### H15 — Relecturas y exportación síncrona reducen velocidad percibida · P2, con parte de usabilidad P1

**Evidencia medida:** una tabla de 60 casos con cabecera en fila 14 dispara **32 llamadas a `pandas.read_excel`**. `_resolve_header_row` ya tiene las primeras 30 filas, pero vuelve a leer prefijos para cada candidato. `app.py:export_current_workbook` llama la exportación nativa en el hilo de la GUI; el exportador realiza numerosas asignaciones COM por celda.

**Objetivo:** reducir espera y mantener la ventana utilizable sin cambiar resultados.

**Pasos:** analizar candidatos sobre la matriz ya leída; separar lector de encabezados de validación del arreglo; reutilizar una copia/origen para las tres modalidades; realizar escritura COM en bloques; ejecutar exportación en un trabajador serial con inicialización COM propia; no compartir objetos COM entre hilos; mostrar fase, progreso y resultado final.

**Verificación:** equivalencia de mapeo y filas antes/después, recuento de lecturas que no crezca por cada candidato, tiempos de 100/1.000/10.000 filas y respuesta de GUI durante exportación. No prometer multiplicadores de velocidad sin medición. No paralelizar guardados de Office para acelerar artificialmente.

### H16 — Interfaz con estados engañosos y recuperación incompleta · P1

**Evidencia:** la captura muestra “seleccionado, sin analizar” junto a un análisis ya realizado. `file_var` no se actualiza con ese resultado. La GUI expone conteos de advertencias pero no todas las decisiones del lector; acciones en una misma fila compiten por ancho. Historial lista productos y no ofrece un recorrido completo para retomar un lote persistido.

**Objetivo:** que el usuario sepa qué falta para obtener sus archivos, sin leer detalles técnicos innecesarios.

**Pasos:** usar un solo estado de trabajo vigente; traducir `blocked/reviewed/excluded` a etiquetas claras; agrupar incidencias repetidas por causa; mostrar “cabecera utilizada/hoja” solo cuando requiera confirmación; mantener un botón principal contextual; habilitar acciones según estado; recuperar lotes desde historial; separar panel de detalle avanzado.

**Verificación:** pantalla de 1366×768 y escala de texto institucional, redimensionado, navegación por teclado, carga fallida, cambio de archivo, vuelta al lote anterior y cierre/reapertura. El error estructural debe permitir corregir la entrada, no obligar a revisar 103 filas.

### H17 — Copias divergentes, instalación y documentación desalineadas · P1

**Evidencia:** la rama tiene 81 pruebas; la copia local candidata tiene otra suite y módulos que no están publicados. El instalador publicado ahora busca Python 3.12 por varios comandos/rutas, mientras el error aportado solo mencionaba `py -3.12`. README sigue declarando pendientes algunos componentes parcialmente incorporados; `IMPLEMENTACION.md` describe una aprobación sin filas pendientes aunque el código actual aprueba juntas las pendientes sin incidencia.

**Objetivo:** saber qué versión se instala y evitar perder trabajo o atribuir funciones a un paquete que no las contiene.

**Pasos:** identificar un único commit de integración; comparar candidatos archivo por archivo; integrar por bloques con migraciones y pruebas; conservar copias históricas identificadas antes de retirar duplicados; mostrar versión/commit en Ayuda y diagnóstico; generar el paquete desde esa revisión; incluir inventario de dependencias y guía concordante.

**Instalación:** el cambio de búsqueda resuelve ausencia del launcher cuando existe Python 3.12 en otra ruta admitida; no instala un Python inexistente. El script aún busca Python antes de aprovechar una `.venv` existente y el `pip install` requiere dependencias accesibles. Preparar ruta explícita de Python y paquete de dependencias autorizado para equipos sin acceso, si ese es el escenario institucional. No eludir políticas de red.

**Verificación:** PC sin launcher, Python por usuario, ruta con espacios, entorno previo válido/inválido, dependencia ausente y actualización conservando SQLite. La batería de contrato del `.bat` no prueba que el instalador se haya ejecutado en ese PC.

### H18 — Cobertura automatizada insuficiente para aceptación institucional · P0 como puerta de liberación

**Evidencia:** la suite pasa mientras los defectos H01/H02/H03/H05/H07 siguen presentes. El test `test_native_backend_requires_windows_excel_environment` comprueba el rechazo fuera de Windows y se omite en Windows; **no** ejecuta Excel. Los tests de Outlook utilizan dobles. CI instala `dev,excel-legacy`, no un Office institucional.

**Objetivo:** basar la liberación en productos reales comprobados, no solo en un indicador verde de CI.

**Pasos:** convertir los hallazgos en regresiones; construir fixtures anonimizados fieles a las estructuras reales; comparar reglas con fecha fijada; validar tanto el Excel previo como el revisado; ejecutar aceptación Office en el PC objetivo; guardar acta con versiones, archivos, hashes, capturas, resultado y responsable; corregir y repetir únicamente los casos afectados más el recorrido integral.

**Verificación:** cero bloqueadores abiertos, cada producto obligatorio generado y abierto correctamente, ningún correo enviado, sin pérdida de estructura y sin duplicados. Un supuesto “funciona en Windows” debe especificar qué Windows, qué Office y qué recorrido se ejecutó.

### H19 — Exportación inicial condicionada a una aprobación posterior · P0 funcional

**Evidencia de código:** `export_preserved_workbook` obtiene un snapshot aprobado mediante `db.get_snapshot`. La GUI indica analizar y aprobar antes de exportar. **Evidencia funcional nueva:** el usuario necesita ese Excel para ejecutar la revisión individual y registrar la verdadera observación en RUS.

**Objetivo:** restablecer el orden real del trabajo.

**Pasos:** crear exportación de propuesta desde la evaluación inmutable, incluyendo incidencias y excluidos; rotularla como pendiente de revisión; mantener por separado la exportación de constancia y productos derivados; exigir aprobación únicamente de la información que esos productos posteriores consumen; no suprimir controles de identidad o integridad para aparentar que se entregó un producto terminado.

**Verificación:** un análisis puede producir Excel para revisión sin fingir que sus casos fueron revisados. Ese archivo no incrementa estadísticas de gestión ni crea borradores automáticamente.

### H20 — Falta retorno del Excel revisado y constancia del registro en RUS · P0 funcional

**Evidencia:** el flujo publicado permite edición en la GUI, pero no implementa el circuito completo de devolver un Excel editado externamente, identificar cada revisión y preparar desde esa versión. Tener `original_observation` y `edited_observation` ayuda, pero no acredita lo registrado en RUS.

**Objetivo:** utilizar fielmente el trabajo humano realizado fuera de NuRus.

**Pasos:** exportar identidad técnica estable por fila; importar la copia revisada y detectar modificaciones; conservar la propuesta y la constancia humana por separado; registrar FECHA_OBS y TT/CC/RES con su semántica administrativa; pedir una declaración compacta del alcance registrado en RUS; no atribuir verificación remota a esa declaración; generar productos desde un snapshot de esa constancia.

**Verificación:** editar y reordenar filas en Excel, volver a importar y asociar correctamente los cambios; detectar identidades alteradas o duplicadas; no sobrescribir finales previos; no tratar texto sin modificar como prueba de revisión.

## 7. Qué conservar, retirar o posponer

| Decisión | Elementos | Motivo |
|---|---|---|
| Conservar | Copia estable del archivo; hashes; referencias físicas; snapshots; edición con motivo; versiones inmutables; CC central; adaptador Outlook separado | Mejoran control sin alterar la evidencia de origen |
| Corregir | Mapeo de Informes, identificación de totales, cruce incompleto, fechas, ficha FAE, escritura de trazabilidad y archivos | Defectos demostrados o contradicción precisa con especificación |
| Unificar | Contratos de columnas, estado de revisión, catálogo y perfil de salida; exportadores locales alternativos | Evitar que una corrección exista solo en una ruta o copia |
| Retirar del flujo principal | Plantillas genéricas como si fueran modelos finales; exportación de solo tabla presentada como conservación completa; incidencias repetidas de esquema | Reducen confusión y falsas expectativas |
| Archivar después de integrar | ZIP y carpetas candidatas duplicadas, conservando versión/hash | Evitar pérdida de avances; no borrarlas antes de comparar |
| No reintroducir | C-06, clasificación automática de carga, ficha ambulatoria, envío automático | Decisiones rechazadas o incompatibles con invariantes |
| Posponer | Cambio de base de datos, nube, reescritura de toda la interfaz, nuevas modalidades y automatizaciones ajenas | No resuelven los bloqueadores de los productos solicitados |

No se recomienda eliminar pruebas ni trazabilidad para ganar velocidad. El ahorro material está en leer menos veces y escribir Office en bloques, además de reducir pasos de usuario.

## 8. Plan ordenado de corrección e integración

### Fase 1 — Entrada confiable y reglas evaluables

**Entrada:** commit auditado, fixtures derivados de AGOSTO y del ejemplo RUS.  
**Acciones:** H01, H02, H03 y validaciones H04/H05/H06; separar desde el inicio la salida de propuesta H19.  
**Salida:** tres modalidades leídas, totales conservados y cruce con estado explícito.  
**Terminado:** no hay bloqueos estructurales falsos; regresiones cubren las sondas; diferencias respecto al catálogo están justificadas.  
**No incluye:** rediseño general ni creación de correos reales.

1. Corregir Informes.
2. Clasificar filas estructurales y mantener coordenadas.
3. Cerrar el control de cruce.
4. Corregir validaciones dependientes de fechas/fichas.
5. Comparar nuevamente observaciones con la referencia histórica y revisar solo divergencias.

### Fase 2 — Excel final íntegro

**Dependencia:** fase 1 y perfil de salida H09.  
**Acciones:** H07, H08, H09, H19, H20 y la parte de exportación de H15.  
**Salida:** productos de las tres modalidades sobre copias, con fórmulas/estructura conservadas y excluidos identificados.  
**Terminado:** inspección nativa y comparación de estructura; destinos parciales no se presentan como productos.

1. Definir el campo de observación vigente sin destruir el original.
2. Corregir texto seguro y publicación de archivo.
3. Generar propuesta desde la evaluación; importar la constancia revisada y congelarla para productos posteriores; registrar artefactos y perfiles.
4. Abrir y comparar en Excel 2010.

### Fase 3 — Productos históricos completos

**Dependencia:** snapshots y contratos de salida estables.  
**Acciones:** H10–H14, recuperando módulos candidatos.  
**Salida:** comunicaciones agrupadas, Word, borradores y reporte de enviados.  
**Terminado:** cada familia tiene matriz vigente, datos requeridos, adjuntos, aprobación y evidencia del producto creado.

1. Migrar catálogo y plantillas con su estado de aprobación.
2. Conectar contactos y agrupación.
3. Completar revisión de todos los campos del producto.
4. Integrar Word y Outlook con recibos e idempotencia.
5. Integrar contador con período/cuenta y reporte de límites.

### Fase 4 — Experiencia e instalación coherentes

**Acciones:** H15–H17.  
**Salida:** flujo compacto, recuperación de trabajo y paquete identificable.  
**Terminado:** instalación reproducible y recorrido sin mensajes contradictorios en el equipo objetivo.

1. Eliminar relecturas medidas y exportación bloqueante de GUI.
2. Unificar estados, agrupar errores y recuperar lotes.
3. Generar paquete desde el commit integrado.
4. Actualizar guía, requisitos y diagnóstico en la misma revisión.

### Fase 5 — Aceptación y segunda auditoría

**Acción:** H18.  
**Salida:** acta técnica/funcional y lista final de incidencias.  
**Terminado:** las capacidades anunciadas corresponden exactamente a las comprobadas.

Las fases no autorizan enviar mensajes ni aprobar datos judiciales ficticios. Las pruebas de producto usan datos de prueba autorizados y cuentas/carpetas seleccionadas para borradores.

## 9. Protocolo concreto de aceptación con Excel 2010 y Outlook clásico

Registrar para cada caso: ID, commit/paquete, Windows/Office/Python, archivo y SHA-256, fecha de corte, resultado esperado/obtenido, evidencia, responsable y estado.

| Caso | Ejecución | Aceptación |
|---|---|---|
| A01 Instalación | Instalar en cuenta institucional, con y sin launcher según equipo | Abre la versión correcta y conserva datos anteriores |
| A02 Tres modalidades | Analizar AGOSTO y fixtures equivalentes | Informes se reconoce; sumas no son casos; coordenadas conservadas |
| A03 XLS del incidente | Cargar el archivo exacto de la captura y verificar cabecera/mapa | Identidades y campos correctos; causa del incidente cerrada con evidencia |
| A04 Cruce | Probar válido, ausente, incompleto, ambiguo y vacío | Solo estados autorizados continúan; excepción queda en snapshot/salida |
| A05 Revisión | Editar, excluir, restaurar y aprobar | Cambios invalidan aprobación anterior; excluidos conservados y no usados en productos nominales |
| A06 Excel | Exportar cada modalidad y abrir con Excel 2010 | Sin reparación del archivo; hojas, fórmulas, estilos, anchos, filtros y totales conservados |
| A07 Correos | Preparar cada familia, revisar cuerpo/CC/adjuntos/cuenta | Borrador real con contenido aprobado y CC institucional; ningún envío |
| A08 Outlook incierto | Simular fallo de recibo tras Save y reintentar | Se concilia; no se genera duplicado automáticamente |
| A09 Word | Generar por tribunal y causal permitida | Coincide con matriz; identidad correcta; sin variables pendientes ni causal indebida |
| A10 Contador | Consultar intervalo conocido y exportar | Conteo comparable con carpeta; omitidos/errores/límites visibles |
| A11 Recuperación | Cerrar/reabrir, recuperar lote y cambiar fuente original | Revisión recuperada; exportación usa copia congelada; cambios de origen detectables |
| A12 Rendimiento | Ejecutar tamaños representativos | Ventana responde; tiempos registrados; sin pérdida de resultados |
| A13 Circuito humano | Exportar propuesta sin aprobar; revisar en RUS y Excel; importar constancia | Cada registro conserva propuesta y edición; no se declara revisión automática ni verificación remota de RUS |

Después de las correcciones: ejecutar regresiones de cada H, revisar los cambios exactos contra el commit auditado, comparar productos con matrices, repetir el recorrido A01–A13 pertinente y actualizar el dictamen. Si una prueba falla, corregir su causa y repetir sus dependencias, evitando volver a auditar todo sin motivo.

## 10. Precisiones sobre afirmaciones anteriores y limitaciones

1. **La captura no acreditaba por sí sola la causa exacta del `.xls`.** El parche de cabecera desplazada tiene evidencia sintética, pero el incidente real requiere ese archivo. No es correcto darlo por resuelto solo por esa prueba.
2. **CI verde no acredita Excel ni Outlook institucionales.** El test nativo omitido en Windows no es un test que haya abierto Excel 2010. La aceptación permanece pendiente.
3. **Una copia local no equivale a integración publicada.** Los módulos candidatos existen y sus pruebas pasan en su contexto, pero faltan comparación, migraciones, integración y aceptación en la rama.
4. **Equivalencia de observaciones no es equivalencia de productos.** La comparación realizada es fuerte evidencia sobre esas entradas/reglas, no sobre todas las posibles planillas, formatos Word, adjuntos o acciones de Office.
5. **La salida RUS aportada no constituye un juego de prueba completo de extremo a extremo.** Faltan su entrada exacta, versión ejecutable identificada y contexto de ejecución. El Word representativo mencionado anteriormente no está entre las fuentes disponibles para esta comprobación.
6. Esta auditoría no demuestra ausencia absoluta de errores. Identifica defectos reproducibles, diferencias de contrato y límites materiales de las pruebas disponibles.

## 11. [G-ESTADO] Checkpoint para continuar

**Objetivo vigente:** completar NuRus como herramienta integrada fiel a sus reglas y productos, con revisión humana y conservación del origen.  
**Estado actual:** auditoría realizada sobre `afd82b8`; no se han aplicado las correcciones de este informe.  
**Confirmado:** 81 pruebas de rama pasan; 25 textos coincidentes; paridad de observaciones en las tablas comparables; defectos reproducidos de Informes, totales, cruce, fechas, ficha FAE, trazabilidad y copia parcial.  
**Pendiente:** H01–H20 según sus prioridades y aceptación A01–A13.  
**Conflictos:** conservar respaldo de las diferencias manual/catálogo para aceptación institucional. La separación entre propuesta, observación oficial y constancia quedó aclarada por el usuario y se desarrolla en el ajuste funcional.  
**Limitaciones:** `.xls` exacto del incidente, entrada original de la salida RUS y aceptación Office no disponibles.  
**Decisiones:** conservar catálogo consolidado salvo contradicción acreditada; no reintroducir decisiones rechazadas; integrar trabajo candidato por cambios revisados.  
**Dependencias:** importación y clasificación → Excel de propuestas → revisión humana en RUS y constancia Excel → retorno de constancia → productos derivados → aceptación.

### [L-SIGUIENTE]

Corregir el mapeo de `AGOSTO.xlsx / Informes` para que la coexistencia de `FECHA INGRESO` y `FEC. INGRESO EFECTIVO` no bloquee I-01/I-02, conservando ambas columnas y añadiendo una prueba de regresión con esa estructura.
