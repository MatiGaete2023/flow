# Revisión del producto y del plan de continuidad — NuRus / CSMP

**Fecha:** 13 de septiembre de 2026. **Finalidad:** decidir la siguiente implementación. Esta revisión no modifica el programa ni fusiona ramas.

## 1. Dictamen

**Recomiendo aprobar el cambio de enfoque, con una corrección: adoptar el flujo del Asistente como referencia de producto, pero no decidir todavía que todo su código será el tronco definitivo.** Debe superarse primero una prueba acotada de integración que conserve las mejoras de NuRus sin trasladar su complejidad visible.

Tu crítica central está respaldada por el código: el trabajo técnico no se tradujo suficientemente en menos pasos. NuRus exige constancia importada, estados y aprobaciones para producir documentos; el Asistente resulta más directo, pero también pide archivos separados para Correos y Resoluciones, reconstruye el Excel y contiene defectos que NuRus ya corrigió. Cambiar de repositorio o de interfaz sin resolver esas dependencias repetiría el problema.

La alternativa recomendable es una **migración selectiva por flujos completos**, con una sola aplicación y un solo motor activo. Primero: cargar → procesar → abrir una copia Excel preservada, con observaciones y excepciones visibles. Después: utilizar ese mismo trabajo para preparar Word y borradores, incorporando los cambios humanos sin exigir volver a buscar el archivo.

**No recomiendo continuar ampliando dev7 como producto final ni descartarlo. Tampoco recomiendo un reemplazo completo del motor antes de disponer de la matriz de comportamiento.** La arquitectura concreta debe elegirse por el menor cambio necesario para ese flujo, no por la cantidad de pruebas ni por el tamaño de cada proyecto.

## 2. Alcance, evidencia y límites

Se revisaron los cuatro adjuntos actuales, el manual CSMP disponible, el catálogo incluido en el Asistente y los módulos relevantes de lectura, reglas, exportación, correos, resoluciones y Enviados de NuRus. Se ejecutaron las suites automatizadas y verificaciones adicionales sobre casos sintéticos, sin abrir Outlook ni enviar mensajes.

| Fuente | Identificación comprobada | Alcance útil |
|---|---|---|
| NuRus main | `b7e6930f625eb64a01f1e638577a368564699379` | Referencia remota; no se fusionó |
| NuRus implementación | `e50eee4859fcdced832b52e5177041694d6fd109`, rama `implementacion-plan-2026-09-08` | Código actual dev7; 83 archivos locales cotejados con los blobs de esa revisión |
| Asistente adjunto | `Asistente (1).zip`; versión interna `v9.1.0` | Motor, GUI, comunicaciones, resoluciones, catálogo y pruebas |
| Creador adjunto | `Creador de Correos.py`; versión interna `2.1` | UX, configuración, validación y creación de borradores |
| Configuración adjunta | `config_correos_csmp.json` | 3 tribunales y 6 plantillas generales |
| Contador adjunto | `Contador de Correos.PY` | Consulta de Enviados y exportación |
| Manual | `CSMP_2025_Fusionado(2).md` | Espera, Cumplimiento, Informes, instrucciones generales y proyectos |

**Límites:** no se localizó el código original identificable de RUS Engine entre las fuentes accesibles revisadas. Su simplicidad y comportamiento histórico se consideran referencias aportadas por tu informe, no una paridad de código comprobada aquí. Tampoco se acreditó la vigencia institucional de actas locales, destinatarios o plantillas Word finales mediante una fuente posterior al material disponible. No se inspeccionó como evidencia funcional el Word adicional que aparece en el árbol remoto de NuRus.

Tu prueba exitosa de Python 3.14 y exportación COM en tu computador es **evidencia de uso informada por ti**. Se conserva como tal: no corresponde volver a declarar ese caso totalmente pendiente, pero tampoco extenderlo a todas las combinaciones de Excel, formatos, cuentas Outlook y documentos.

## 3. Correcciones al informe recibido

| Afirmación o propuesta | Resultado de esta revisión | Ajuste necesario |
|---|---|---|
| Las referencias de main y dev7 son las indicadas | Confirmado mediante consulta remota | Mantenerlas inmóviles como referencias |
| Asistente es una base operacional más próxima | Respaldado por sus módulos y GUI | Usar su flujo; condicionar el tronco técnico a una integración breve |
| Hay diferencias generales entre umbrales de Asistente y NuRus | Demasiado amplio | En los umbrales principales revisados coinciden; las diferencias materiales están en validaciones, fallback, cruce y relación con el manual |
| Hay que volver a cerrar todos los 30/45/60 días | El catálogo registra decisiones previas explícitas | No reabrir indiscriminadamente; distinguir decisiones registradas, contradicciones y respaldo pendiente |
| Hay redacciones distintas que deben unificarse | Los 25 textos de ambos catálogos coinciden exactamente | Mejorar su veracidad y calidad una sola vez, después de separar las acciones |
| El Contador debe portarse como nueva función | NuRus ya tiene consulta por fechas, cuenta, exportación y tratamiento de errores | Reutilizar ese adaptador y completar filtros; no reescribir desde el script rígido |
| El JSON de correos representa toda la configuración necesaria | Incluye 6 correos generales; no cubre por sí solo todas las comunicaciones a programas | Unificar un catálogo que distinga tribunal/programa y su adjunto específico |
| CC debe ser configurable | Hay un invariante expreso: copia a `ucc_concepcion@pjud.cl` en cada correo | Editar CC adicional; impedir que una opción de usuario quite el obligatorio |
| Instalación y distribución al final | Los problemas de arranque bloquearon el uso real | Comprobar instalación mínima desde la primera entrega; dejar empaquetado definitivo para el cierre |

## 4. Hallazgos y acciones propuestas

P0 impide confiar en la selección o el contenido de una salida. P1 afecta directamente el flujo o una funcionalidad comprometida. P2 es una mejora acotada posterior. La prioridad no implica que el programa envíe o ejecute actuaciones automáticamente.

| ID / prioridad | Hallazgo y evidencia | Acción y objetivo |
|---|---|---|
| H01 · P0 | Asistente deriva Word de frases mediante `detectar_tipo`; Correos Espera filtra `PATRON_ESPERA`. Cambiar la redacción de una misma intención dejó de seleccionar PC_IE en V03. [A02, A04] | Retornar reglas y acciones estructuradas; que editar lenguaje no cambie la elegibilidad inadvertidamente |
| H02 · P0 | Ambos catálogos dicen «Se remite correo…» durante el cálculo. Crear una propuesta no acredita una gestión. V08 confirma textos iguales. [A01, N01] | Separar hallazgo, sugerencia, archivo/borrador creado y gestión confirmada; evitar constancias falsas |
| H03 · P0 | Manual y perfil operativo documentado difieren en DCE, Espera y medidas próximas a vencer. [M01, A07] | Mantener la diferencia visible y su procedencia; no escoger el plazo por cuál código se porte primero |
| H04 · P0 | El manual limita ciertos proyectos al primer pide cuenta y exige revisar escritos/nomenclaturas. Las fechas del Excel no acreditan esas condiciones. [M01, A04] | Señalar «proyecto posible; verificar primer requerimiento y antecedentes» cuando falte el dato; selección humana sobre sugerencias, sin aprobación fila a fila de todo el Excel |
| H05 · P0 | Con días negativos y egreso futuro, Asistente redacta «vencida»; NuRus advierte contradicción. Reproducido en V06. [A01, N01] | Portar la validación y conservar el resto de la fila; no afirmar vencimiento sobre datos incompatibles |
| H06 · P0 | El Creador permite eludir CC obligatorio con `validar_manual=False` y bloquea Para vacío. Ambos resultados reproducidos en V07. [C01] | CC obligatorio en el adaptador final, independiente del editor; permitir borrador sin Para con aviso |
| H07 · P0 | El resolvedor del Asistente documenta fuzzy solo para futura selección humana, pero el generador lo llama con `permitir_fuzzy=True`. Sin contacto, omite el grupo. [A02, A03] | Exacto/alias aprobado para asignación automática; aproximaciones como sugerencias; desconocido → borrador sin destinatario |
| H08 · P1 | El exportador del Asistente crea `Workbook()` nuevo desde un DataFrame y reasigna estilos/anchos. No preserva el libro original. [A05] | Sustituir esa salida por el exportador preservado; mantener hojas, fórmulas, estilos, filtros y filas excluidas coloreadas |
| H09 · P1 | Asistente mantiene campos Excel separados para correo y resolución; NuRus exige constancia importada para esas operaciones. [A06, N02, N03] | Mantener una sesión de trabajo común y la ruta del archivo generado; actualizar cambios sobre ese archivo sin pedir seleccionarlo nuevamente |
| H10 · P1 | Hay contenido judicial en funciones Python del Asistente; NuRus convierte texto a un documento nuevo. No equivalen a llenar una matriz Word real. Faltan PC_INFO Laja y plantillas de Tomé en la ruta revisada. [A04, N03] | Implementar `.docx` por tribunal/tipo, con sustitución que preserve formato; «sin plantilla» debe ser pendiente, no proyecto terminado |
| H11 · P1 | El Creador guarda JSON directamente; ante error de lectura vuelve silenciosamente a valores predeterminados. Además fusiona defaults con lo cargado. [C01] | Guardado atómico, copia recuperable y diagnóstico claro; evitar reintroducir contactos/plantillas eliminados o continuar con destinatarios inesperados |
| H12 · P1 | Asistente escoge primera hoja en lectura simple y, en Cumplimiento, la primera restante como cruce. [A05] | Reutilizar reconocimiento de cabeceras y modalidad de NuRus; mostrar selector solo si hay ambigüedad real, con nombres y vista previa |
| H13 · P1 | Para cruce duplicado, Asistente conserva la última fila; NuRus detecta fechas futuras distintas. [A05, N04] | Conservar detección de conflicto; nunca resolver por orden accidental. Es una diferencia material con el catálogo G-08 que debe quedar documentada |
| H14 · P1 | El Creador incorpora los mismos adjuntos a varios tribunales después de una confirmación; el Asistente arma tablas HTML para programas sin adjunto en esa llamada. [C01, A02, M01] | Generar nóminas por destinatario; distinguir libro de revisión completo de adjuntos parciales. Verificar los adjuntos exigidos por cada comunicación |
| H15 · P1 | Algunos informes anuncian que «se revisaron todos» y «se registraron observaciones» aunque analizar no acredita esa revisión. [C02] | Usar textos finales solo para el alcance que el usuario confirme revisado; no reemplazar la revisión real por un botón de aprobación técnica |
| H16 · P2 | Contador recorre Enviados completo y usa ruta fija. NuRus ya corta por fecha y señala resultados limitados/errores. [C03, N05] | Portar NuRus; añadir filtros tribunal/tipo/búsqueda. Separar «no clasificado» de «no existe» |

**Defectos actuales y fallos históricos no son lo mismo.** El instalador dev7 ya busca Python 3.12–3.14 y valida el ejecutable; el exportador nativo usa `SaveCopyAs`. Los errores antiguos de `-m` y `SaveAs` explican tus quejas, pero no se declaran presentes de nuevo sin reproducirlos en esa revisión. [N06, N07]

## 5. Matriz funcional de migración

«Eliminar» significa retirar del producto activo o del flujo visible, conservando la referencia original. No autoriza borrar archivos históricos.

| Función / necesidad real | Fuente preferente | Destino | Acción | Criterio de salida |
|---|---|---|---|---|
| Espera / Cumplimiento / Informes | Reglas comparadas Asistente/NuRus | Trabajo | CONSERVAR + MEJORAR contrato | Misma condición aprobada, salida estructurada |
| Lectura `.xls/.xlsx/.xlsm` y cabeceras | NuRus | Entrada común | PORTAR | Reconoce archivos representativos y explica ambigüedad |
| Copia fiel Excel | NuRus COM | Trabajo | PORTAR | Original intacto, estructura conservada |
| Excluidos e incidencias | NuRus + requisitos del usuario | Excel/Trabajo | CONSERVAR + simplificar | Filas visibles, color y motivo; fallo local no paraliza todo |
| Exportación DataFrame como salida principal | Asistente | — | ELIMINAR | Nunca sustituye al libro preservado |
| Observación propuesta | Catálogo compartido | Trabajo/Excel | MEJORAR | Hallazgo y sugerencia sin gestión ficticia |
| Fecha/observación humana/carga/proyecto | Excel y revisión RUS | Trabajo común | CONSERVAR | Dato humano permanece diferenciado de propuesta |
| Recarga manual de salida propia | Ambos flujos | — | ELIMINAR | Ninguna selección repetida del mismo archivo |
| Aprobación de cada fila para obtener Excel | NuRus | — | ELIMINAR del flujo normal | Excel accesible tras procesar |
| Control de revisión efectuada | Requisito CSMP | Alcance revisado | MEJORAR | No presume RUS actualizado; confirmación útil y acotada |
| Hash y referencia de origen | NuRus | Interior de la sesión | CONSERVAR | Diagnóstico recuperable sin hash visible obligatorio |
| Estados de productos/snapshots visibles | NuRus | — | ELIMINAR de UX | Trabajo no exige conocerlos |
| Correos generales al tribunal | Creador + JSON | Correos | PORTAR | Plantilla editable, alcance y destinatarios visibles |
| Correos a programas | Asistente + política NuRus | Correos | MEJORAR | Regla/acción estructurada, agrupación y adjunto correctos |
| CC obligatorio / solo borradores | Adaptador NuRus | Outlook | CONSERVAR | Save; nunca Send; CC no desactivable |
| Firma y vista previa | Creador / adaptadores | Correos | PORTAR selectivamente | Firma de cuenta elegida sin perder edición |
| Contactos y alias | Resolver exacto NuRus + editor/importación | Configuración | UNIFICAR | Una fuente vigente; no fuzzy automático |
| Plantillas judiciales | Matrices Word reales | Resoluciones | MEJORAR | Formato preservado y variables resueltas |
| Textos judiciales codificados | Ambos | — | RETIRAR después de paridad | No perder tipo/tribunal existente al sustituirlos |
| Enviados | Adaptador NuRus | Enviados | PORTAR + completar | Solo lectura, filtros, cuenta y total verificable |
| Estadísticas de gestión | Constancia humana / Enviados | Resumen de trabajo | CONSERVAR acotado | No contar propuestas como revisiones o envíos |
| Parámetros y textos operativos | Catálogos/configuraciones | Configuración | MEJORAR | Edición sin Python, validación y recuperación |
| Instalación aislada | NuRus dev7 | Distribución | CONSERVAR | Arranque desde acceso evidente sin preparación manual repetida |
| Benchmarks masivos / escenarios multiusuario | Prototipo | Fuera de alcance | NO AMPLIAR | Priorizar tiempo y clics del ciclo real |

## 6. Matriz de reglas observadas y discrepancias

Esta es una **matriz de implementación comparada**, no una declaración de reglas institucionalmente vigentes. A = Asistente v9.1; N = NuRus dev7. Los textos exactos son las 25 entradas de sus catálogos, idénticas según V08. «Correo/proyecto» describe lo sugerido por el texto o el módulo: hoy A no devuelve un contrato de acciones.

| ID / modo | Condición y umbral observados en A y N | Texto / acción observada | Decisión para migración |
|---|---|---|---|
| E-01 / común | Derivación fuera de seguimiento; corte previo | NO_SEGUIMIENTO; sin gestión sugerida | Conservar fila; no eliminarla del Excel |
| E-02 / C-01 | Edad ≥18 | MAYORIA_EDAD; sugiere egreso | Conservar; no convertir sugerencia en egreso realizado |
| E-03 / C-02 | Entre 1 y 60 días para mayoría | PROXIMA_MAYORIA | Coinciden; no reabrir sin causa |
| E-04 / Espera | Resolución entre hoy y 29 días atrás | E04_RESOLUCION_RECIENTE | Conservar; revisar acumulación con E-05 para evitar frase administrativa contradictoria |
| E-05 / DCE | Espera ≥30, cualquier tribunal | E05_SOLO_CORREO | Perfil documentado; contraste manual F01 |
| E-05 / Laja, Mulchén no DCE | Espera ≥30 | E05_PROYECTO_Y_CORREO | Perfil documentado; actas y primer requerimiento F02/F05 |
| E-05 / Tomé no DCE | 30–59; ≥60 | Correo; luego correo + proyecto | Perfil documentado; Word Tomé aún incompleto |
| E-06 / Espera | Sin principal; A permite base incluso con incidencia; N no si hay incidencia | E06_SIN_RESOLUCION | Diferencia reproducida V05; proponer base neutral + aviso, no «sin observaciones» engañoso |
| T-01 / Espera-Cumplimiento | Curador no acreditado en columna disponible | CURADOR | Mantener decisión manual con/sin carga; no inventar respuesta previa |
| T-02 / Espera-Cumplimiento | Oído hace 0–45 días | OIDO | Coinciden |
| T-03 / tres modos | Audiencia hoy o futura, cuando se compone observación | PROX_AUDIENCIA | Coinciden en intención; no asumir revisión de carpeta |
| C-03 / Cumplimiento | Días cumplimiento 0–30 y fecha ingreso disponible | C03_INGRESO_RECIENTE | Conservar validación de fecha; no calcular sustituto silencioso |
| C-04 / Cumplimiento | Días egreso o cumplimiento negativos, con fecha egreso | C04_VENCIDA | N detecta fecha futura contradictoria; portar esa corrección V06 |
| C-05 / Cumplimiento | Días egreso 0–45, con fecha, sin vencida | C05_VENCE_HOY / C05_POR_VENCER | Ambos 45; manual un mes F03 |
| C-06 / Cumplimiento | Cálculo de hitos eliminado por catálogo | Sin implementación vigente | No reintroducir como «mejora» |
| C-07 / residencial | Prefijos RTA/RTT/RES/RFA/RVA; ficha ausente, >180 días o 0–30 días | C07_SIN_FICHA / ANTIGUA / RECIENTE | Coinciden en umbrales; ausencia de columna no equivale a ausencia de ficha |
| C-08 / FAE/FAS | Ingreso hace >120 días, sin ficha | C08_FICHA_FAE | Preservar diagnóstico N cuando no existe la columna |
| C-09 / Cumplimiento | Sin principal | SIN_OBSERVACIONES o BASE_BREVE | Condicionar conclusión a datos evaluables; no ocultar incidencias |
| C-10 / Cumplimiento | Vencimiento futuro en cruce; no C-04/C-05 | C10_HOJA2 | Sin cálculo sustituto; excepción documentada si falta cruce; duplicados F04 |
| I-01 / Informes | Fecha vencimiento anterior a hoy | VENCIDO_DCE / GENERAL; texto de correo | Fechas no prueban informe realmente pendiente; primer pide cuenta F05, DCE F06 |
| I-02 / Informes | Fecha entre hoy y +30 días | POR_VENCER_DCE / GENERAL; correo | Coinciden; excluir de nómina lo ya presentado según revisión humana |
| Informes fuera de ventana | Fecha >30 días | Ambos retornan vacío | Identificar «sin alerta automática»; no confundir con registro revisado en RUS |
| G-01–G-04 | Prefijo, fechas, nombre programa y puntuación | Composición del texto | Mantener decisiones documentadas; medir mejoras lingüísticas con ejemplos |
| G-05 / G-06 | Datos incompletos y tribunal desconocido | Omitir regla afectada y avisar | Revisar severidad local; no convertir 1 defecto de estructura en 103 revisiones idénticas |
| G-07 / G-08 | Orden de fragmentos y cruce | A usa última coincidencia; N distingue conflictos | Conservar orden; resolver discrepancia de duplicados expresamente |

### Conflictos y decisiones que requieren tratamiento explícito

| ID | Evidencia A | Evidencia B | Resolución propuesta / dato necesario |
|---|---|---|---|
| F01 | Manual Espera: DCE por correo sin importar días | Catálogo E-05/E-06 registra decisión previa de 30 días; ambos motores la aplican | Mantener perfil documentado como referencia; identificar instrucción operativa vigente. No cambiar automáticamente a 0 ni presentar 30 como mandato del manual |
| F02 | Manual: gestión LE en X desde 60 días, salvo acuerdo en acta | Catálogo: Laja/Mulchén 30; Tomé 30/60 | Asociar tribunal, ámbito P/X y respaldo del acuerdo al perfil. No basta un umbral global |
| F03 | Manual Cumplimiento: próximas a vencer a un mes | Catálogo C-05 y ambos códigos: 45 días | Diferencia documental real. Determinar si 45 es alerta interna ampliada o criterio de gestión autorizado; pueden ser parámetros distintos |
| F04 | Catálogo G-08 y A: última fila duplicada | N: fechas futuras distintas producen conflicto | Recomiendo N para no seleccionar fecha arbitraria; registrar el cambio de decisión, mostrar ambos antecedentes |
| F05 | Manual proyectos: primer pide cuenta; escritos pendientes y nomenclaturas requieren examen | Exportación RUS y reglas de fechas no acreditan esas condiciones | Conservar verificación humana de procedencia en proyectos sugeridos, no un bloqueo de todo el análisis |
| F06 | Manual Informes menciona DCE pendiente >40 días | I-01 usa cualquier vencimiento pasado, sin condición adicional de 40 días | Aclarar qué hito inicia esos 40 días; no sumar 40 al vencimiento sin fuente. Separar alerta de gestión |
| F07 | Configuración adjunta y defaults Python difieren, por ejemplo en Tomé | Creador fusiona fuentes; Asistente mantiene catastro y alias propios | Conservar la configuración adjunta como candidato explícito, presentar diferencias antes de migrar. No reincorporar un contacto por antigüedad del código |

No hace falta detener toda implementación por F01–F07: puede construirse el flujo y las validaciones con perfiles de referencia identificados. Sí debe quedar pendiente declarar paridad institucional de las decisiones afectadas. Las decisiones previas documentadas no se borran por esta auditoría.

## 7. Contrato mínimo del motor y trabajo humano

El cambio fundamental es correcto, pero los booleanos únicos `correo_sugerido` y `proyecto_sugerido` quedan cortos cuando una fila dispara varias reglas. Recomiendo un resultado pequeño:

```text
ResultadoRegistro
  origen: identificador + hoja + fila (interno)
  reglas_aplicadas: [identificadores]
  hallazgos: [hechos obtenidos del Excel]
  observacion_propuesta
  acciones_sugeridas: [{tipo, plantilla, motivo, requiere_verificar}]
  advertencias: [{campo, motivo, regla_afectada}]

ConstanciaHumana (opcional hasta que exista revisión)
  observacion_final, fecha_revision, con_sin_carga,
  proyecto_procede, gestion_confirmada
```

La fecha de análisis debe ser un parámetro fijo de esa ejecución. El catálogo/umbrales usados quedan identificados internamente. **Cambiar un texto no cambia una regla; cambiar una observación humana tampoco confirma ni cancela automáticamente una acción:** la selección correspondiente debe poder ajustarse.

RUS es la fuente de verdad de la observación oficial. La aplicación no puede comprobar que se registró allí solo leyendo Excel. «Borrador creado» no significa «correo enviado»; «Word generado» no significa «proyecto remitido, firmado o resuelto». Enviados tampoco demuestra por sí solo toda la gestión judicial.

**El archivo generado debe continuar disponible sin recarga.** La sesión conoce su ruta; se ofrece Abrir Excel y, si fue editado, Actualizar cambios sobre esa misma copia. Al preparar salidas se detectan cambios y se valida únicamente identidad/columnas afectadas. Si el archivo se movió, solo entonces se pide localizarlo. Debe existir recuperación de sesión y una asociación estable de filas, sin confiar exclusivamente en posición después de ordenar el Excel.

Puede prepararse material preliminar desde las propuestas. Los textos que afirman revisión realizada y los reportes de actuaciones deben usar la constancia humana del alcance real. No se impone volver a cargar todo ni aprobar cada fila para obtener el Excel.

**Conservar lo original** significa preservar contenido y formato del libro, con adiciones deliberadas para observaciones y revisión. Se mantienen excluidos con color y motivo. La copia íntegra para tu revisión y la nómina adjunta a un destinatario son productos distintos: no se adjunta automáticamente el libro completo a cada programa.

## 8. Configuración y Word: límites prácticos

- Un único repositorio local de configuración operativa, separado de la instalación y recuperable mediante copia. Las actualizaciones no deben borrar ediciones. No crear un constructor universal de reglas.
- Umbrales con nombre claro, unidad, ámbito, límites válidos y procedencia. Distinguir ventana de alerta de criterio para pedir cuenta; mostrar efecto en ejemplos antes de guardar.
- Textos y plantillas con validación de variables. Si falta una variable obligatoria, explicar qué completar. Una plantilla corrupta no debe disparar silenciosamente un texto predeterminado diferente.
- Contactos exactos y alias aprobados. Las coincidencias aproximadas se ofrecen para elección, no se asignan solas. Sin coincidencia: conservar el borrador con Para vacío.
- CC adicional editable; el CC institucional y «nunca enviar» no son opciones desactivables. Validar en el último adaptador, incluso si el JSON fue editado externamente.
- Plantillas `.docx` por tribunal/tipo: preservar párrafos, tablas, encabezados y formato. Las variables pueden estar divididas entre fragmentos de Word; una sustitución ingenua de `paragraph.text` perdería formato. Probar ese caso expresamente.
- Versionar solo las matrices utilizadas, con ejemplos anonimizados. No inventar un modelo judicial definitivo para llenar huecos de Laja o Tomé. El resultado parcial debe informar qué proyectos no pudo generar.

## 9. Plan corregido, con entregas verificables

Las fases propuestas son razonables, pero diez frentes separados pueden volver a consumir semanas sin un flujo útil. Las agruparía así:

| Fase | Entrada / acción | Salida y criterio de terminado | No incluye |
|---|---|---|---|
| 0. Referencias | Versiones y hashes identificados; localizar RUS Engine pendiente | Referencias preservadas y mapa de procedencia; sin seguir ampliando dev7 | Borrar herramientas anteriores |
| 1. Contrato y decisiones | Matrices de este informe, catálogo y manual | Perfiles identificados; acciones independientes del texto; conflictos F01–F07 con tratamiento explícito | Reabrir todas las decisiones antiguas |
| 2. Un flujo vertical | Espera real + lector/exportador NuRus + interfaz simple | Cargar una vez, procesar y abrir Excel; excluidos visibles; archivo disponible en correo/Word; instalación mínima comprobable | Nuevo editor completo, Enviados, optimizaciones masivas |
| 3. Tres modos y continuidad | Extender contrato probado a Cumplimiento/Informes | Excepción de cruce con motivo y alcance; cambios humanos sobre la copia; sin recarga repetida; fallos locales explicados | Automatizar actos en RUS |
| 4. Salidas utilizables | UX Creador, contactos únicos y matrices Word | Borradores editables, sin Para permitido, CC fijo, adjuntos por grupo, proyectos reales con pendientes claros | Envío, firma o resolución automática |
| 5. Autonomía y Enviados | Editor acotado + adaptador ya existente | Configuración recuperable, filtros Enviados y exportación; estadísticas con categorías correctas | Deducir actuaciones por frases del asunto |
| 6. Aceptación y distribución | Flujo mensual representativo | Menos pasos/tiempo/correcciones que herramientas actuales; paquete reproducible y guía breve | Declarar éxito solo por tests |

**Decisión técnica al terminar fase 2:** conservar el armazón del Asistente si integrar los servicios preservados resulta limpio y acotado; si exige arrastrar dos motores o reescribir gran parte de los módulos, utilizar una capa de interfaz pequeña sobre servicios seleccionados de NuRus. En ambos casos, una sola aplicación y un solo contrato. No mantener dos implementaciones productivas de la misma regla.

**Criterios de no avance:** no migrar al usuario si la salida pierde hojas/formato, sigue exigiendo recargar el Excel propio, cambia un umbral sin procedencia o afirma una gestión no realizada. Si una salida falla, conservar el Excel ya generado y permitir reintentar únicamente la salida afectada.

La velocidad debe medirse primero en el tiempo humano: selecciones, recargas, correcciones y clics. A nivel técnico, reutilizar una lectura por trabajo, escribir a Excel por bloques y procesar tareas COM fuera del hilo de interfaz con inicialización/liberación apropiadas. No abrir una investigación de rendimiento de 10.000 registros sin un problema real.

## 10. Verificaciones ejecutadas

Entorno de revisión local, sin Office institucional. Las pruebas se ejecutaron sobre copias; no se editó código de aplicación.

| ID | Verificación | Resultado |
|---|---|---|
| V01 | Suite Asistente: `python -m pytest -q` con dependencias disponibles | **175 aprobadas, 3 omitidas**, 2,73 s |
| V02 | Suite NuRus dev7, con `src` en PYTHONPATH | **148 aprobadas, 1 omitida**, 3,54 s; la prueba de cmd.exe requiere Windows |
| V03 | Misma intención de PC_IE, redacción alternativa | Detecta original; deja de detectar alternativa. Confirma acoplamiento |
| V04 | DCE con 0, 29, 30, 59 y 60 días, fecha fija | Ambos inician mención de correo/E-05 en 30 |
| V05 | Espera inválida con identidad válida | A devuelve base; N devuelve incidencia sin base en ese caso |
| V06 | Días negativos con fecha de egreso futura | A afirma vencida; N advierte contradicción |
| V07 | Validación del Creador, sin COM | CC puede quedar vacío sin error al desactivar validación; Para vacío se bloquea |
| V08 | Comparación exacta de textos | **25 comparados, 0 diferencias** |
| V09 | Cotejo con árbol Git remoto | **83 archivos coincidentes** con blobs dev7; documento Word adicional no incluido en la copia funcional auditada |

V03–V08 son reproducibles mediante `verificar_hallazgos.py`; sus resultados están en `evidencia_auditoria.json`. Se usaron nombres y correos sintéticos. Las pruebas aprobadas no contradicen H01–H16: una suite puede proteger el comportamiento existente y no detectar que ese comportamiento incumple el flujo deseado.

### Verificación posterior necesaria

Comparar herramientas actuales y nueva versión con los mismos archivos y fecha de análisis: Espera, Cumplimiento e Informes; `.xls`, `.xlsx` y, si se usa, `.xlsm`; 100–500 registros; Laja, Mulchén y Tomé. Incluir ausencia de contacto, adjunto obligatorio, fecha contradictoria, cruce ausente/duplicado, archivo abierto en Excel y fallo parcial Outlook.

Registrar: tiempo desde descarga RUS hasta Excel/Word/borradores listos; número de selecciones repetidas; correcciones de observaciones y Word; salidas omitidas. Revisar fórmulas/hojas/filtros de una muestra y comprobar borradores en la cuenta institucional. **Éxito: cero recargas innecesarias, cero envíos automáticos, ninguna pérdida de estructura requerida y mejora demostrada del ciclo real.**

## 11. Decisiones recomendadas ahora

1. Aprobar el enfoque de herramienta personal y la migración selectiva, manteniendo revisión humana y fuente oficial RUS.
2. Aprobar la separación regla/observación/acción y la sesión compartida como requisitos de la próxima entrega.
3. Adoptar las matrices de este informe como punto de partida; conservar los perfiles documentados sin atribuirles una vigencia institucional aún no acreditada.
4. Ejecutar primero fases 0–2; elegir el tronco técnico al comprobar el flujo vertical. No prometer todavía que el Asistente completo es la mejor base de código.
5. Retirar complejidad visible y preservar internamente protección de originales, identidad de filas y prevención de duplicados donde tengan una utilidad concreta.

No se requiere volver a decidir si deben conservarse excluidos, si va CC institucional o si se permite Para vacío: esas decisiones ya están cerradas.

## 12. Referencias de hallazgos

Referencias locales a adjuntos: el manifiesto anexo identifica el archivo original por SHA-256. Las funciones permiten localizar el fragmento exacto. Referencias NuRus fijadas al commit auditado, para no depender del movimiento posterior de la rama.

- **A01:** Asistente, `motor/reglas_espera.py::generar_observacion_espera`, `motor/reglas_cumplimiento.py::generar_observacion_cumplimiento`, `motor/reglas_informes.py::generar_observacion_informes`, `motor/textos_observaciones.json`.
- **A02:** Asistente, `comunicaciones/generador_correos.py`, filtro `PATRON_ESPERA`, llamadas `resolver(...permitir_fuzzy=True)`, construcción de borradores y `_crear_borrador_outlook`.
- **A03:** Asistente, `comunicaciones/contactos_programas.py::resolver`; comparación exacta/alias, tokens y ambigüedad.
- **A04:** Asistente, `resoluciones/generador_resoluciones.py::detectar_tipo`, `generar_resoluciones`, `_gen_pc_ie_*`, `_gen_pc_informe_mulchen`; casos sin plantilla Laja/Tomé.
- **A05:** Asistente, `motor/procesador.py::_guardar_excel`, `_leer_excel`, `_leer_cumplimiento`, `_construir_indice_hoja2`.
- **A06:** Asistente, `gui/app.py`, variables `correo_espera_var`, `res_excel_var`, `_worker_motor`, `_finish_motor`.
- **A07:** Asistente, `docs/especificaciones/Catalogo_Reglas_CSMP_v2_20260714.md`, E-05/E-06, C-05, G-05/G-08 y decisiones descartadas. Es evidencia de decisiones registradas, no sustituto de un acta institucional.
- **C01:** `Creador de Correos.py`, `cargar_config`, `guardar_config`, `CSMPMailApp._validate_before_create`, `_create_drafts`, editor de plantillas/configuración.
- **C02:** `config_correos_csmp.json`, `tribunales`, `plantillas`, `cc_obligatorio`, `validar_manual`.
- **C03:** `Contador de Correos.PY::main`, ruta de salida y recorrido `sent_items.Items`.
- **M01:** `CSMP_2025_Fusionado(2).md`: Espera, pasaje anterior al marcador Página 8; Cumplimiento Página 9; Informes Páginas 12–13; instrucciones generales Página 14; proyectos Páginas 16–17. La conversión Markdown de tablas puede alterar el orden de lectura: no se resuelve por ella sola una ambigüedad de plazo o acuerdo.
- **N01:** [NuRus reglas](https://github.com/MatiGaete2023/NuRus/blob/e50eee4859fcdced832b52e5177041694d6fd109/src/nurus/rus/rules.py), `evaluate_waiting`, `evaluate_compliance`, `evaluate_reports`; [catálogo](https://github.com/MatiGaete2023/NuRus/blob/e50eee4859fcdced832b52e5177041694d6fd109/src/nurus/rus/textos_observaciones.json).
- **N02:** [NuRus comunicaciones](https://github.com/MatiGaete2023/NuRus/blob/e50eee4859fcdced832b52e5177041694d6fd109/src/nurus/services/communications.py), `prepare_communications`, `attach_snapshot_table`.
- **N03:** [NuRus resoluciones](https://github.com/MatiGaete2023/NuRus/blob/e50eee4859fcdced832b52e5177041694d6fd109/src/nurus/services/resolutions.py), `prepare_resolution`, `export_resolution`.
- **N04:** [NuRus análisis/cruce](https://github.com/MatiGaete2023/NuRus/blob/e50eee4859fcdced832b52e5177041694d6fd109/src/nurus/rus/service.py), `_cross_index`, `_precheck`.
- **N05:** [NuRus Enviados](https://github.com/MatiGaete2023/NuRus/blob/e50eee4859fcdced832b52e5177041694d6fd109/src/nurus/adapters/sent_mail.py), `scan_sent_items`, `count_sent_mail`, `export_sent_report`; [Outlook](https://github.com/MatiGaete2023/NuRus/blob/e50eee4859fcdced832b52e5177041694d6fd109/src/nurus/adapters/outlook.py).
- **N06:** [NuRus exportación](https://github.com/MatiGaete2023/NuRus/blob/e50eee4859fcdced832b52e5177041694d6fd109/src/nurus/services/exports.py), `_native_preserved`, `_export_preserved_payload`, `export_proposal_workbook`.
- **N07:** [Instalador dev7](https://github.com/MatiGaete2023/NuRus/blob/e50eee4859fcdced832b52e5177041694d6fd109/Instalar_NuRus.bat), `find_python`, `try_python`; [dependencias](https://github.com/MatiGaete2023/NuRus/blob/e50eee4859fcdced832b52e5177041694d6fd109/pyproject.toml).

## [G-ESTADO] — checkpoint de esta revisión

**Objetivo:** decidir una herramienta personal más rápida y útil que el conjunto actual.

**Cambio material:** referencias remotas comprobadas; matrices funcional y de reglas elaboradas; 16 hallazgos priorizados; recomendación de migración selectiva condicionada a un flujo vertical. Código de aplicación sin cambios.

**Confirmado:** V01–V09; 25 textos iguales; dependencia textual y fallos de validación reproducidos; NuRus aporta correcciones que no deben perderse.

**Conflictos:** F01–F07, con decisiones previas y evidencia diferenciadas.

**Limitaciones:** RUS Engine original no localizado; falta acreditar actas, contactos vigentes y matrices Word finales; Office institucional no ejecutado en esta auditoría.

**Pendiente:** decisión sobre el plan ajustado y ejecución de fases 0–2; aceptación completa del flujo posterior.

## [L-SIGUIENTE]

Una vez adoptado el plan ajustado, preparar la prueba vertical de Espera con un archivo representativo: una carga, exportación preservada y disponibilidad automática del resultado para Correos y Resoluciones.
