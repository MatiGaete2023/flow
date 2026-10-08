# Preparación de P2 — contrato aún no implementado

Esta entrega P0 no declara existente `CSMP_CAPTURE_V1`. La primera modificación
funcional tras aprobar G0 será su validador; no una reescritura de GUI.

| Decisión | Propuesta técnica para revisión | Consecuencia |
|---|---|---|
| Codificación | CSV UTF-8, delimitador coma, comillas RFC 4180, sin conversión numérica de IDs | Productor/consumidor idénticos; Excel se exporta por ruta propia |
| Nulos | Campo vacío para nulo; distinguir en diccionario el texto vacío legítimo | No introducir cero ficticio; cerrar ambigüedades por columna |
| Identidad | `capture_id/query_id/record_id` locales + sistema/tribunal/causa/ingreso verificados; alcance `CAUSA` separado | No unir personas por RIT/nombre |
| Fuentes opcionales | Separar `NO_SOLICITADA` (fuera del plan) de `NO_CONSULTADA` (solicitada pero no ejecutada), `VACIA_COMPROBADA`, `FALLIDA`, `PARCIAL` y cobertura completa por consulta/fuente | La omisión y cero filas no demuestran ausencia de acontecimientos |
| Tablas | `consultas`, `registros`, `bitacoras`, `historia_tramites`, `documentos`, `incidencias`; definir además representación de calendarios | La lista del plan menciona calendarios pero no una tabla dedicada: cerrar antes del schema |
| Tiempo | ISO 8601 con precisión y zona conocida/desconocida explícitas; `as_of` separado | No fabricar hora, zona ni orden en empates |
| Texto extenso | CSV conserva literal completo; Excel particiona/sidecar con referencia y hash cuando supera 32767 unidades UTF-16 | Sin truncamiento; no usar un Excel consolidado como única fuente |
| Fórmulas | Tablas técnicas nunca se ejecutan; derivados Excel escriben texto; definir escape reversible para CSV de usuario | No corromper literal `=texto` al normalizar |
| Originales | Excluidos por defecto; inclusión únicamente conforme a autorización institucional, conservando referencia/hash | La aprobación técnica no autoriza transferir datos de NNA |
| Límites | Medir tamaño/memoria de fixtures; mantener límites existentes mientras se define presupuesto ZIP/CSV | No inventar umbrales de rendimiento ni ampliar límites sin prueba |
| ZIP | Rechazo de traversal, symlinks, duplicados/case collisions Windows, cifrado inesperado, expansión y hashes incorrectos; prohibir XXE en cualquier lector XML de originales | Validar antes de tocar sesión; no ejecutar macros, fórmulas ni entidades externas |
| Publicación | Temporal validado, destino nuevo y operación atómica sin sobrescribir | Conservar paquete anterior al fallar |
| Reimportación | Captura+hash inmutables; decisiones humanas separadas y vinculadas a identidad comprobada | No duplicar ni pisar ediciones |
| Solicitud de antecedentes | Archivo declarativo sin código/URLs arbitrarias; consumo manual por Descargador | Asistente no inicia Chrome/RUS |
| Modelo del consumidor | Adaptar `Work`/`Row` y `SourceReference` vigentes; separar raw/normalizado, identidad remota, coberturas y varios hallazgos por ingreso | `record_id` local nunca reemplaza la identidad remota; no duplicar estados/UI |
| Cambio de sesión | Construir candidato local completamente validado y cambiar el trabajo activo solo al finalizar; ante rechazo conservar ediciones, selecciones y productos | Importación transaccional y recuperación de sesión independiente de historial de causas |
| Versiones y fecha de corte | `schema_version`, `ruleset_version`/`policy_version`, versión del mapeo, `as_of` explícito y `analyzed_at` técnico separado; negociar cambios opcionales 1.x | Igual entrada/corte/reglas da mismos hallazgos; versión mayor desconocida o requisitos nuevos incompatibles se rechazan |
| Calendarios sin ID de ingreso | Relación completa validada o `NO_ASOCIADO`, conservando filas legítimas repetidas y su procedencia | No cruzar por RIT+nombre ni suprimir multiplicidad |
| Fuentes Excel legado | Etiquetar `SOLO_PLANILLA` y fuentes faltantes sin enriquecer desde otra sesión o SQLite histórico | No convertir ausencia de bitácora/historia en «sin actuaciones» |
| Productos y UI | Snapshot de decisiones, huellas de dependencia y estados locales; propuestas, productos preparados y registro oficial son hechos separados | Conservar edición TT/CC/RES, contactos, adjuntos quitados y plantillas; no afirmar envío/registro |
| Referencias a documentos | Metadatos/hash/estado de descarga sin inferir contenido de PDF no leído; causa sin ingreso permanece con alcance CAUSA | No activar reglas dependientes de historia sin contrato y aceptación P9/P10 |

P2 entregará `README.md`, `manifest.schema.json`, diccionario por tabla,
fixture completa/parcial, validador y pruebas de seguridad. La especificación 02
ya está incorporada: el consumidor debe cumplir A-T01..A-T20 y el pipeline de
su §5, conservando Excel legado y sin iniciar Chrome/Descargador.

El diseño propuesto `WorkBatch` y el ejemplo `analyze_package` del documento 02
son ilustrativos; no se declaran implementados. P5 extenderá los modelos vigentes
mediante adaptadores. Las tablas pueden tener cero filas con estado explícito;
una tabla opcional no solicitada puede omitirse solo si el manifiesto lo declara.
El schema aún debe cerrar nombres, nulos y representación de calendarios.

Mediciones pendientes: fixtures de 100, 500 y 2000 registros para lectura,
validación, normalización, análisis y UI; distinguir tiempo/memoria por etapa y
no fijar umbrales sin medir. El benchmark de Treeview existente mide solo dibujo,
no toda la importación ni el rendimiento institucional.

No requieren sesión institucional: esquema, fixtures y validador de referencia.
Sí requieren decisión funcional o evidencia: política de originales, atribución
de bitácoras a centro, granularidad de historia, nuevas reglas y textos.
