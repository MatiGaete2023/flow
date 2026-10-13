# Diccionario CSMP_CAPTURE_V1

CSV UTF-8 (BOM aceptado), coma RFC 4180, LF; todas las celdas son cadenas.
Vacío representa desconocido/no aplicable; en text/response representa literal vacío.
IDs no se convierten en números ni se construyen desde RIT/nombre. Campos JSON son objetos.
Cada fila conserva procedencia. Se conservan filas repetidas legítimas con ID local por fila física.
Estados de tablas y consultas no prueban historia completa.

## consultas

| Columna | Tipo / semántica |
|---|---|
| `query_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `source_system` | Texto literal; vacío si no aplicable. |
| `screen` | Texto literal; vacío si no aplicable. |
| `tab` | Texto literal; vacío si no aplicable. |
| `court_code` | Texto literal; vacío si no aplicable. |
| `court_name` | Texto literal; vacío si no aplicable. |
| `modality_code` | Texto literal; vacío si no aplicable. |
| `coverage` | Enum NO_SOLICITADA, NO_CONSULTADA, COMPLETA, VACIA_COMPROBADA, PARCIAL, FALLIDA, CANCELADA, IDENTIDAD_INCIERTA, NO_DISPONIBLE. |
| `captured_at` | ISO 8601; eventos/períodos admiten fecha sin hora. No completar hora/zona. |
| `filters_json` | Objeto JSON válido, sin claves duplicadas. raw íntegro; normalized por mapeo versionado. |
| `expected_pages` | Entero positivo en texto (páginas admiten cero); saved <= expected; completa exige igualdad. |
| `saved_pages` | Entero positivo en texto (páginas admiten cero); saved <= expected; completa exige igualdad. |
| `period_from` | ISO 8601; eventos/períodos admiten fecha sin hora. No completar hora/zona. |
| `period_to` | ISO 8601; eventos/períodos admiten fecha sin hora. No completar hora/zona. |
| `time_zone` | Texto literal; vacío si no aplicable. |
| `history_complete` | true/false; content_read siempre false. Garantía institucional exige evidencia externa. |
| `institutional_verified` | true/false; content_read siempre false. Garantía institucional exige evidencia externa. |
| `error` | Texto literal; vacío si no aplicable. |

## registros

| Columna | Tipo / semántica |
|---|---|
| `capture_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `query_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `source_system` | Texto literal; vacío si no aplicable. |
| `court_code` | Texto literal; vacío si no aplicable. |
| `court_name` | Texto literal; vacío si no aplicable. |
| `rit_original` | Texto literal; vacío si no aplicable. |
| `cause_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `ingreso_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `person_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `program_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `modality_raw` | Texto literal; vacío si no aplicable. |
| `modality_code` | Texto literal; vacío si no aplicable. |
| `tab` | Texto literal; vacío si no aplicable. |
| `captured_at` | ISO 8601; eventos/períodos admiten fecha sin hora. No completar hora/zona. |
| `original_file` | Texto literal; vacío si no aplicable. |
| `original_sheet` | Texto literal; vacío si no aplicable. |
| `original_row` | Entero positivo en texto (páginas admiten cero); saved <= expected; completa exige igualdad. |
| `source_sha256` | 64 hexadecimales SHA-256; obligatorio en procedencia. |
| `coverage` | Enum NO_SOLICITADA, NO_CONSULTADA, COMPLETA, VACIA_COMPROBADA, PARCIAL, FALLIDA, CANCELADA, IDENTIDAD_INCIERTA, NO_DISPONIBLE. |
| `record_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `record_type` | Texto literal; vacío si no aplicable. |
| `identity_status` | VERIFICADA exige tribunal+causa+ingreso remotos; INCIERTA o CAUSA no autoriza cruce individual. |
| `raw_json` | Objeto JSON válido, sin claves duplicadas. raw íntegro; normalized por mapeo versionado. |
| `normalized_json` | Objeto JSON válido, sin claves duplicadas. raw íntegro; normalized por mapeo versionado. |

## bitacoras

| Columna | Tipo / semántica |
|---|---|
| `capture_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `query_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `source_system` | Texto literal; vacío si no aplicable. |
| `court_code` | Texto literal; vacío si no aplicable. |
| `court_name` | Texto literal; vacío si no aplicable. |
| `rit_original` | Texto literal; vacío si no aplicable. |
| `cause_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `ingreso_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `person_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `program_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `modality_raw` | Texto literal; vacío si no aplicable. |
| `modality_code` | Texto literal; vacío si no aplicable. |
| `tab` | Texto literal; vacío si no aplicable. |
| `captured_at` | ISO 8601; eventos/períodos admiten fecha sin hora. No completar hora/zona. |
| `original_file` | Texto literal; vacío si no aplicable. |
| `original_sheet` | Texto literal; vacío si no aplicable. |
| `original_row` | Entero positivo en texto (páginas admiten cero); saved <= expected; completa exige igualdad. |
| `source_sha256` | 64 hexadecimales SHA-256; obligatorio en procedencia. |
| `coverage` | Enum NO_SOLICITADA, NO_CONSULTADA, COMPLETA, VACIA_COMPROBADA, PARCIAL, FALLIDA, CANCELADA, IDENTIDAD_INCIERTA, NO_DISPONIBLE. |
| `event_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `event_at` | ISO 8601; eventos/períodos admiten fecha sin hora. No completar hora/zona. |
| `time_precision` | Texto literal; vacío si no aplicable. |
| `time_zone` | Texto literal; vacío si no aplicable. |
| `origin` | Texto literal; vacío si no aplicable. |
| `origin_verified` | true/false; content_read siempre false. Garantía institucional exige evidencia externa. |
| `author` | Texto literal; vacío si no aplicable. |
| `event_type` | Texto literal; vacío si no aplicable. |
| `text` | Texto literal; vacío si no aplicable. |
| `response` | Texto literal; vacío si no aplicable. |
| `response_at` | ISO 8601; eventos/períodos admiten fecha sin hora. No completar hora/zona. |
| `identity_status` | VERIFICADA exige tribunal+causa+ingreso remotos; INCIERTA o CAUSA no autoriza cruce individual. |

## calendarios

| Columna | Tipo / semántica |
|---|---|
| `capture_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `query_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `source_system` | Texto literal; vacío si no aplicable. |
| `court_code` | Texto literal; vacío si no aplicable. |
| `court_name` | Texto literal; vacío si no aplicable. |
| `rit_original` | Texto literal; vacío si no aplicable. |
| `cause_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `ingreso_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `person_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `program_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `modality_raw` | Texto literal; vacío si no aplicable. |
| `modality_code` | Texto literal; vacío si no aplicable. |
| `tab` | Texto literal; vacío si no aplicable. |
| `captured_at` | ISO 8601; eventos/períodos admiten fecha sin hora. No completar hora/zona. |
| `original_file` | Texto literal; vacío si no aplicable. |
| `original_sheet` | Texto literal; vacío si no aplicable. |
| `original_row` | Entero positivo en texto (páginas admiten cero); saved <= expected; completa exige igualdad. |
| `source_sha256` | 64 hexadecimales SHA-256; obligatorio en procedencia. |
| `coverage` | Enum NO_SOLICITADA, NO_CONSULTADA, COMPLETA, VACIA_COMPROBADA, PARCIAL, FALLIDA, CANCELADA, IDENTIDAD_INCIERTA, NO_DISPONIBLE. |
| `calendar_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `event_at` | ISO 8601; eventos/períodos admiten fecha sin hora. No completar hora/zona. |
| `event_type` | Texto literal; vacío si no aplicable. |
| `status` | Texto literal; vacío si no aplicable. |
| `association` | VERIFICADA con identidad completa; NO_ASOCIADO conserva fila. |
| `raw_json` | Objeto JSON válido, sin claves duplicadas. raw íntegro; normalized por mapeo versionado. |
| `normalized_json` | Objeto JSON válido, sin claves duplicadas. raw íntegro; normalized por mapeo versionado. |

## historia_tramites

| Columna | Tipo / semántica |
|---|---|
| `capture_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `query_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `source_system` | Texto literal; vacío si no aplicable. |
| `court_code` | Texto literal; vacío si no aplicable. |
| `court_name` | Texto literal; vacío si no aplicable. |
| `rit_original` | Texto literal; vacío si no aplicable. |
| `cause_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `ingreso_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `person_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `program_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `modality_raw` | Texto literal; vacío si no aplicable. |
| `modality_code` | Texto literal; vacío si no aplicable. |
| `tab` | Texto literal; vacío si no aplicable. |
| `captured_at` | ISO 8601; eventos/períodos admiten fecha sin hora. No completar hora/zona. |
| `original_file` | Texto literal; vacío si no aplicable. |
| `original_sheet` | Texto literal; vacío si no aplicable. |
| `original_row` | Entero positivo en texto (páginas admiten cero); saved <= expected; completa exige igualdad. |
| `source_sha256` | 64 hexadecimales SHA-256; obligatorio en procedencia. |
| `coverage` | Enum NO_SOLICITADA, NO_CONSULTADA, COMPLETA, VACIA_COMPROBADA, PARCIAL, FALLIDA, CANCELADA, IDENTIDAD_INCIERTA, NO_DISPONIBLE. |
| `event_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `event_at` | ISO 8601; eventos/períodos admiten fecha sin hora. No completar hora/zona. |
| `time_precision` | Texto literal; vacío si no aplicable. |
| `time_zone` | Texto literal; vacío si no aplicable. |
| `scope` | CAUSA o INGRESO. CAUSA sin ingreso; no atribuir a NNA. |
| `event_type` | Texto literal; vacío si no aplicable. |
| `description` | Texto literal; vacío si no aplicable. |
| `reference` | Texto literal; vacío si no aplicable. |
| `status` | Texto literal; vacío si no aplicable. |
| `document_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `identity_status` | VERIFICADA exige tribunal+causa+ingreso remotos; INCIERTA o CAUSA no autoriza cruce individual. |

### Extensión aditiva (Descargador 2.8.0)

Columnas opcionales **al final** de `historia_tramites`; solo se escriben si alguna fila las trae. Un lector que
conozca solo las columnas base las ignora (se leen `fields[:len(TABLES)]`). No cambia `schema_version`.

| Columna | Tipo / semántica |
|---|---|
| `folio` | Folio visible del trámite, texto. |
| `stage` | Etapa visible del trámite, texto. |
| `annex_visible` | `true` o `false`: hay icono de anexo; no se abrió ni descargó el documento. |
| `raw_json` | Objeto JSON con Folio, Etapa, Trámite, Desc. Trámite, Referencia y Fecha Tram tal como se leyeron. |

Filas de historia: `scope=CAUSA`, `ingreso_id`, `person_id` y `program_id` vacíos, `identity_status=CAUSA`,
`time_precision=DIA`. La consulta no lleva `remote_identity`; declara `historia_causa`
(`source_system`, `court_code`, `cause_id`) y `ventana` (`criterio`, `desde`, `hasta`).

## documentos

| Columna | Tipo / semántica |
|---|---|
| `capture_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `query_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `source_system` | Texto literal; vacío si no aplicable. |
| `court_code` | Texto literal; vacío si no aplicable. |
| `court_name` | Texto literal; vacío si no aplicable. |
| `rit_original` | Texto literal; vacío si no aplicable. |
| `cause_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `ingreso_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `person_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `program_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `modality_raw` | Texto literal; vacío si no aplicable. |
| `modality_code` | Texto literal; vacío si no aplicable. |
| `tab` | Texto literal; vacío si no aplicable. |
| `captured_at` | ISO 8601; eventos/períodos admiten fecha sin hora. No completar hora/zona. |
| `original_file` | Texto literal; vacío si no aplicable. |
| `original_sheet` | Texto literal; vacío si no aplicable. |
| `original_row` | Entero positivo en texto (páginas admiten cero); saved <= expected; completa exige igualdad. |
| `source_sha256` | 64 hexadecimales SHA-256; obligatorio en procedencia. |
| `coverage` | Enum NO_SOLICITADA, NO_CONSULTADA, COMPLETA, VACIA_COMPROBADA, PARCIAL, FALLIDA, CANCELADA, IDENTIDAD_INCIERTA, NO_DISPONIBLE. |
| `document_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `parent_event_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `scope` | CAUSA o INGRESO. CAUSA sin ingreso; no atribuir a NNA. |
| `reference` | Texto literal; vacío si no aplicable. |
| `local_file` | Texto literal; vacío si no aplicable. |
| `sha256` | 64 hexadecimales SHA-256; obligatorio en procedencia. |
| `download_state` | Texto literal; vacío si no aplicable. |
| `content_read` | true/false; content_read siempre false. Garantía institucional exige evidencia externa. |

## incidencias

| Columna | Tipo / semántica |
|---|---|
| `capture_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `query_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `record_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `incident_id` | ID de texto; query/record/event/document/calendar/incident únicos. IDs remotos solo de fuente. |
| `phase` | Texto literal; vacío si no aplicable. |
| `code` | Texto literal; vacío si no aplicable. |
| `severity` | Texto literal; vacío si no aplicable. |
| `message` | Texto literal; vacío si no aplicable. |
| `recoverable` | true/false; content_read siempre false. Garantía institucional exige evidencia externa. |
