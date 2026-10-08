# Preparación de P2 — contrato aún no implementado

Esta entrega P0 no declara existente `CSMP_CAPTURE_V1`. La primera modificación
funcional tras aprobar G0 será su validador; no una reescritura de GUI.

| Decisión | Propuesta técnica para revisión | Consecuencia |
|---|---|---|
| Codificación | CSV UTF-8, delimitador coma, comillas RFC 4180, sin conversión numérica de IDs | Productor/consumidor idénticos; Excel se exporta por ruta propia |
| Nulos | Campo vacío para nulo; distinguir en diccionario el texto vacío legítimo | No introducir cero ficticio; cerrar ambigüedades por columna |
| Identidad | `capture_id/query_id/record_id` locales + sistema/tribunal/causa/ingreso verificados; alcance `CAUSA` separado | No unir personas por RIT/nombre |
| Fuentes opcionales | Manifiesto declara disponible, vacía comprobada, parcial, fallida o no consultada por consulta/fuente | La omisión no demuestra ausencia de acontecimientos |
| Tablas | `consultas`, `registros`, `bitacoras`, `historia_tramites`, `documentos`, `incidencias`; definir además representación de calendarios | La lista del plan menciona calendarios pero no una tabla dedicada: cerrar antes del schema |
| Tiempo | ISO 8601 con precisión y zona conocida/desconocida explícitas; `as_of` separado | No fabricar hora, zona ni orden en empates |
| Texto extenso | CSV conserva literal completo; Excel particiona/sidecar con referencia y hash cuando supera 32767 unidades UTF-16 | Sin truncamiento; no usar un Excel consolidado como única fuente |
| Fórmulas | Tablas técnicas nunca se ejecutan; derivados Excel escriben texto; definir escape reversible para CSV de usuario | No corromper literal `=texto` al normalizar |
| Originales | Excluidos por defecto; inclusión únicamente conforme a autorización institucional, conservando referencia/hash | La aprobación técnica no autoriza transferir datos de NNA |
| Límites | Medir tamaño/memoria de fixtures; mantener límites existentes mientras se define presupuesto ZIP/CSV | No inventar umbrales de rendimiento ni ampliar límites sin prueba |
| ZIP | Rechazo de traversal, symlinks, duplicados/case collisions Windows, cifrado inesperado, expansión y hashes incorrectos | Validar antes de tocar sesión |
| Publicación | Temporal validado, destino nuevo y operación atómica sin sobrescribir | Conservar paquete anterior al fallar |
| Reimportación | Captura+hash inmutables; decisiones humanas separadas y vinculadas a identidad comprobada | No duplicar ni pisar ediciones |
| Solicitud de antecedentes | Archivo declarativo sin código/URLs arbitrarias; consumo manual por Descargador | Asistente no inicia Chrome/RUS |

P2 entregará `README.md`, `manifest.schema.json`, diccionario por tabla,
fixture completa/parcial, validador y pruebas de seguridad. Se comprobará
la especificación 02 antes de fijar modelo del consumidor.

No requieren sesión institucional: esquema, fixtures y validador de referencia.
Sí requieren decisión funcional o evidencia: política de originales, atribución
de bitácoras a centro, granularidad de historia, nuevas reglas y textos.
