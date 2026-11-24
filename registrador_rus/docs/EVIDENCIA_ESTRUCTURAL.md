# Evidencia de estructura recuperada — 24 de noviembre de 2026

Revisión dirigida de los tres ZIP originales conservados localmente y del código de Explorador V1.6. SHA-256 completos de los tres archivos concuerdan con el Plan Maestro. No publicar ZIP, eventos, datos judiciales ni capturas.

| Sesión | Eventos | OBSERVATION_MANUAL_STEP | DOM_STRUCTURE con observation_form |
|---|---:|---:|---:|
| V1.5 e6043fc8… (Espera) | 162 | 20 | 8 |
| V1.5 e90089c9… (otras vistas) | 93 | 18 | 6 |
| V1.6 5f82c777… (Espera) | 87 | 15 | 4 |
| Total | 342 | 53 | 18 |

En los 53 manual_snapshot.controls, los controles visibles presentan esta firma, sin excepciones en la muestra, índices 0–5: select COD_Estado_Detalle_Nuevo, checkbox NO_CATALOGADO, textarea NO_CATALOGADO, text NO_CATALOGADO, button irAccion, button NO_CATALOGADO. Los seis constan habilitados. El editor visible no tiene maxlength exportado; los editores ocultos sí incluyen GLS_Observacion con 2000. V1.6 documenta tres opciones del selector, no sus códigos o rótulos completos.

Código original extension/content_observer.js: safeTechnicalName (80–85) reemplaza nombres no admitidos; isHistoryControl / entryControls / orderedEntryControls (114–126) excluyen HOBS_ y controles históricos ocultos y ordenan hidden al final; manualFormSnapshot (149–188) conserva atributos estructurales, no valores del editor ni IDs DOM. No interpretar NO_CATALOGADO como atributo HTML ausente. El índice solo es parte de una firma, no una autorización de operación.

Nuevo detector pasivo: exige un formulario único, POST, TablaInforme visible, cinco campos de contexto ocultos presentes, firma exacta de controles visibles, Estado con tres opciones, irAccion único visible y ausencia de contradicciones. Devuelve únicamente resultado categórico; no devuelve nodos, no lee valores de campos de identidad, no modifica controles, no genera eventos ni conecta a RUS. Si no existe asociación label/input, devuelve false; si el texto del botón no confirma Nueva obs, también false. No sustituye el detector estricto de preparación ni se carga en la extensión aprobada. Fixture reconstruido con datos inventados, no HTML real.

Verificación: node --check de structure.js y REGISTRADOR_CHROMIUM=/usr/bin/chromium python registrador_rus/tests/run_structure.py -> PASS 12 structural checks. Chromium local Linux; solicitudes abortadas. Se comprobó cero eventos submit/input/change/click durante inspección. No acredita compatibilidad DOM real, acción del formulario, identidad efectiva, ni códigos de Estado. No se ha integrado el adaptador de preparación institucional. Mantener write_enabled:false. No se requiere repetir las cuatro grabaciones; si se necesita evidencia adicional, una inspección pasiva focalizada del formulario deberá resolver solo las dudas que persistan.

## Evidencia operativa estructural — 24/11/2026, Inspector 0.2.1, Espera

El operador entregó un IntegralEvidenceV1, no truncado, de un solo documento raíz, accesible, sin marcos internos. Un formulario InformesPpalForm POST al destino esperado, 37 controles propios y descendientes; 6 visibles y 8 ocultos relevantes. Seis campos de contexto únicos, ocultos, propietario 0: COD_Tribunal, CRR_IdCausa, ID_Ingreso, COD_Etapa, TIP_Consulta y tipo_popUp. No se recogieron valores ni se verificó igualdad de identidad con una fila de Flow. TablaInforme TABLE única visible, owner=0 (closest form), encabezados exactos de nueve columnas y orden 0–8; los cinco prefijos históricos presentes. contains_candidate=false es coherente: el campo mide si la tabla contiene el formulario, no al revés. No se acredita completitud del historial por conteos de paginación.
 
| Control | Atributos comprobados |
|---|---|
| Estado nuevo | id=name COD_Estado_Detalle_Nuevo; 3 opciones: ... (código omitido), 3 Ingreso Efectivo, 4 Gestión Previa; ambas habilitadas |
| Envío | input checkbox name chkBitacora; sin id, único por nombre, etiqueta próxima sí, asociación label formal no |
| Editor | textarea id GLS_ObsCentro_Nueva, name id_Obs_centro; sin maxlength HTML; onblur y onchange presentes |
| Contador | id=name Txt_Cont2, text readonly |
| Crear | name irAccion, submit, rótulo Nueva obs, onclick presente, sin formaction/formmethod |
| Volver | name volv, button |

irAccion aparece dos veces en total: la otra instancia es un submit oculto dentro de modal de edición. Casilla chkBitacoraEdit, selector COD_Estado_Detalle y editor GLS_ObsCentro / GLS_Observacion pertenecen a edición oculta; maxlength 2000 solo acreditado allí. El editor nuevo no tiene maxlength; no deducir límite servidor, comportamiento del contador ni efectos de eventos. Casilla desmarcada=administrativa CC0, marcada=tribunal CC1 según instrucción del operador y exploraciones previas; esta inspección no registra checked ni prueba escritura. No se publican datos originales ni valores.

Se define detector independiente observed_form.js de solo lectura, no cargado en extensión, prueba reconstruida con datos ficticios. Exige atributos exactos, propiedad, visibilidad, encabezados, contexto estructural, asociación o proximidad de etiqueta y códigos disponibles 3/4 con rótulos correctos; excluye placeholder, no adivina código. No compara identidad efectiva, no toca DOM y devuelve exclusivamente diagnóstico. Se mantienen las guardas anteriores y no se habilita preparación ni escritura real. Alcance: una muestra de Espera; otros orígenes requieren verificación del catálogo y controles al uso. No repetir exploración general. 

## Evidencia complementaria de las cuatro vistas — 24/11/2026

El operador proporcionó tres informes IntegralEvidenceV1 del inspector 0.2.1 y atribuyó expresamente el orden a Cumplimiento, Informes por vencer, Informes vencidos. Se compararon los tres JSON completos con los hallazgos de Espera. Los tres son DIAGNOSTIC, no truncados, write_enabled=false; documento raíz único accesible, formulario InformesPpalForm POST con acción esperada, seis campos de contexto únicos ocultos y propiedad del formulario, TablaInforme única visible con nueve encabezados exactos, seis controles de creación visibles habilitados: COD_Estado_Detalle_Nuevo, chkBitacora, GLS_ObsCentro_Nueva/name id_Obs_centro, Txt_Cont2 readonly, irAccion submit Nueva obs, volv button. Dos irAccion por documento, solo uno visible; los controles de edición ocultos tienen in_modal=false en Cumplimiento y true en ambas vistas Informes. No usar in_modal como única exclusión de edición. Posiciones DOM variables (no selectores posicionales); editor nuevo sin maxlength HTML, editor de edición con maxlength 2000. No se comprobó límite servidor ni ejecución de eventos.

Catálogo técnico observado por origen (cadenas, códigos del selector Estado de nueva observación; distinto del filtro de consulta de Informes):

| Origen | Código : rótulo |
|---|---|
| ESPERA | 3 : Ingreso Efectivo; 4 : Gestión Previa |
| CUMPLIMIENTO | 5 : Solicita Plazo; 6 : Realizada; 7 : Pide Cuenta |
| INFORMES_POR_VENCER | 8 : Solicita Plazo; 9 : Realizada; 10 : Pide Cuenta |
| INFORMES_VENCIDOS | 8 : Solicita Plazo; 9 : Realizada; 10 : Pide Cuenta |

Todas las opciones operativas indicadas estaban habilitadas en las muestras; placeholder ... con código OMITIDO, nunca inferir ni admitir. La atribución de vistas viene del operador, no de los valores de contexto (no se capturaron). No confundir códigos de estado con los de selección de consulta. Al desarrollar: catálogo contextual por origen, validación de código/rótulo disponibles y unicidad al uso, selección explícita sin valor predeterminado, detector común sin dependencias de índices ni modal único. El detector observed_form.js del commit c49dcd4 aún contiene catálogo limitado a Espera: actualizar en siguiente etapa, no afirmar compatibilidad implementada. Mantener prueba del detector pendiente y agregar fixtures por los cuatro orígenes incluyendo edición oculta sin modal en Cumplimiento. No se necesitan más exploraciones estructurales de las cuatro vistas para estos atributos. No publicar informes originales, identidades, valores de campos ni textos; este registro contiene solo estructura técnica. Identidad efectiva, eventos, persistencia, custodia, autorización institucional y piloto siguen pendientes. Este commit exclusivamente documental responde a la entrega de evidencia adicional; no habilita escritura ni modifica código.

## Decisión obligatoria del operador — 24/11/2026, 15:16 Chile

El operador dispone que siempre se seleccione Gestión Previa en Espera y Realizado en todas las demás vistas. La denominación exacta observada en RUS es Realizada. Asignación fija, automática y no editable por operación: ESPERA -> código "4" (Gestión Previa); CUMPLIMIENTO -> "6" (Realizada); INFORMES_POR_VENCER -> "9" (Realizada); INFORMES_VENCIDOS -> "9" (Realizada). Esta decisión sustituye las instrucciones anteriores de elegir Estado explícitamente o no asignar un valor por defecto. Los demás estados observados son evidencia del catálogo, no opciones admitidas para registrar con esta herramienta. Mostrar Estado calculado en previsualización y aprobación; comprobar código y rótulo disponibles y únicos en el selector de la bitácora antes de cualquier preparación, bloquear ante discrepancia, jamás elegir una alternativa. No confundir con filtros de consulta ni con destino: chkBitacora desmarcada CC0 / marcada CC1 sigue independiente. La autorización humana de cada registro permanece obligatoria. Próxima implementación: centralizar regla contextual, actualizar diálogo y validación de contratos de Flow, detector común y pruebas de los cuatro orígenes; preservar contratos ficticios de demo separados. Esta actualización documental no modifica aún el código ni habilita escritura.
