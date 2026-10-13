# Plan paso a paso — WP-52 a WP-56

Para un programador o IA que ejecuta sin rediseñar. Cada paso dice **qué archivo**, **qué hacer**, **cómo
comprobarlo** y **qué no hacer**. Si un paso no se puede cumplir tal cual: detenerse, anotar `BLOQUEADO` con el
motivo en la ficha del WP y preguntar. No improvisar endpoints, columnas ni reglas.

## 0. Antes de empezar (cada WP)

1. Leer `AGENTS.md`, `PLAN_VIGENTE.md`, este directorio (`docs/historia_causa/`) y la ficha del WP.
2. Rama: `git fetch origin feat/historia-causa-sitfa && git checkout -b wp/WP-5X-<slug> origin/feat/historia-causa-sitfa`.
3. Entorno (una vez): ver «Verificación estándar» en `docs/implementacion/PLAN_IMPLEMENTACION.md`
   (Python 3.12, `requirements/linux-py312.lock`, `pip install -e .`, `ruff`, Node para la prueba de extensión).
4. Línea de partida: `python -m pytest descargador -q` y `python -m pytest tests -q` en verde.
5. Al terminar: `python tools/verificar_wp.py --salida docs/implementacion/verificacion/WP-5X.json` → PASA.
   Commit `WP-5X: <qué cambia>` con cuerpo «Verificación: PASA …». Sin `--force`. Sin merge a `main`.
6. E3 (revisión independiente) obligatoria en WP-53 (extensión), WP-54 (retención) y WP-55 (contrato).

Dependencias: WP-52 → WP-53 → WP-54 → WP-55 → WP-56. WP-55 puede empezar tras WP-54 en paralelo con la UI de WP-56.

---

## WP-52 · H0: caracterización anónima y contrato de la ventana

**Tipo:** documentación + herramienta. **No riesgoso** salvo el registro de la decisión.
**Archivos:** `PLAN_VIGENTE.md`, `docs/implementacion/estado/global.json`, `docs/implementacion/ESTADO.json`,
`docs/implementacion/estado/WP-52.json`…`WP-56.json` (nuevos), `docs/implementacion/PLAN_IMPLEMENTACION.md`
(fichas en «Ola 5»), `contracts/csmp_capture_v1/HISTORIA_VENTANA.md` (nuevo), `descargador/extraer_estructura_historia.py`
(ya existe; solo ajustes si H0 lo exige), `descargador/fixtures/historia_ficticia/` (reemplazar por fixtures fieles).

Pasos:
1. **Registrar la decisión.** En `PLAN_VIGENTE.md` → «Decisiones del 13 de octubre de 2026», agregar:
   «o) Historia de la causa en el Descargador: solo tabla de la pestaña Historia (folio, etapa, trámite,
   descripción, referencia, fecha, presencia de anexo), ventana de 4 meses, sin PDF, uso personal en Windows; el
   Asistente la analizará en otro paso. CT-03..10 y PDF siguen fuera de alcance. Fuente: mensajes del usuario del
   13-oct.» y reemplazar la línea «Historia SITFA/PDF … fuera de alcance» por «PDF y CT-03..10: fuera de alcance;
   historia tabular: WP-52..WP-56». Replicar en `global.json` → `decisiones_adicionales`.
2. **Registrar WP-52..WP-56** en `ESTADO.json` y `estado/WP-5X.json` con el formato de `estado/WP-99.json`
   (`titulo`, `ola: 5`, `tipo`, `depende_de`, `decisiones: ["o"]`, `estado: "PENDIENTE"`…). Copiar las fichas de
   este documento a «Ola 5» de `PLAN_IMPLEMENTACION.md`. Correr `python tools/estado.py` (debe validar).
3. **Pedir H0 al usuario** con [05_GUIA_USUARIO_H0](05_GUIA_USUARIO_H0.md). Esperar `estructura_historia.json`.
   No avanzar sin él.
4. **Revisar el JSON recibido** (sin valores). Responder por escrito en `HISTORIA_VENTANA.md`:
   - Escenario de acceso A/B/C/D (ver 01 §5) con la evidencia: esqueleto de `ShowHistoria` (o la función que
     use la pestaña), peticiones del HAR (rutas, método, redirección, nombres y forma de parámetros).
   - ¿El enlace existe en Espera y en Cumplimiento? ¿Con qué función y aridad?
   - Tabla de historia: id/clase, etiquetas exactas del encabezado, formas por columna, filas irregulares.
   - Paginación: ¿hay controles? ¿una sola tabla con todas las filas?
   - Vacío explícito: ¿qué muestra una causa sin trámites? (si H0 incluye una).
   - Acceso denegado/sesión vencida: ¿cómo se ve? (si H0 lo incluye).
   - Charset declarado.
5. **Si escenario D** (solo con clic real): detener todo el plan, marcar WP-53..56 `BLOQUEADO_USUARIO` e informar.
   **Si escenario C**: presentar el esqueleto al usuario y pedir decisión P7 antes de WP-53.
6. **Fixtures fieles:** reemplazar `descargador/fixtures/historia_ficticia/*.html` por HTML **inventados** que
   repliquen exactamente la estructura de H0 (mismos ids, clases, encabezados, anidamiento, charset, funciones y
   aridad) con valores canario. Añadir variantes: sin trámites, 1 trámite, ≥ 30 trámites, texto largo (> 1.000
   caracteres), fecha ilegible, con anexo, encabezado distinto, HTML de login, acceso denegado, y paginación si
   existe. Mantener las pruebas de `test_extraer_estructura_historia.py` verdes.
7. Verificación: `python tools/check_real_ids.py` 0 hallazgos; `verificar_wp.py` PASA.

**Criterio de salida:** `HISTORIA_VENTANA.md` responde todas las preguntas del paso 4 con evidencia anónima;
escenario A o B (o C autorizado); fixtures fieles versionados; decisión registrada.
**No hacer:** pedir o aceptar HTML/HAR/capturas reales; adivinar `a1`; escribir el parser.

---

## WP-53 · H1: lector local (extensión + parser + runner)

**Tipo:** función nueva. **Riesgoso (extensión) → E3.**
**Archivos:** `descargador/extension/pagina.js`, `descargador/extension/manifest.json` (solo versión),
`descargador/contrato_integral.py`, `descargador/vinculos.py`, `descargador/lectura_bitacoras.py` (gancho de una
línea), `descargador/lectura_historia.py` (nuevo), `src/nurus/historia_html.py` (nuevo),
`descargador/SITFA_Descargador.spec`, pruebas: `descargador/test_historia_extension.cjs` (nuevo),
`descargador/test_lectura_historia.py` (nuevo), `tests/test_historia_html.py` (nuevo), ajustes en
`descargador/test_bitacoras_extension.cjs` y pruebas que comparan `PROTOCOLS`/versión.

Pasos:
1. **Parser primero** (`src/nurus/historia_html.py`), solo con los fixtures fieles de WP-52:
   - Reutilizar `decode_snapshot` y `normalize` de `nurus.bitacora_html` (importar, no copiar).
   - `HEADERS = (...)` exactamente como `HISTORIA_VENTANA.md`.
   - `read_historia(data, *, source='', captured_at='') -> dict` con claves `encabezado` (dict de etiqueta → texto
     visible), `filas` (lista de dicts: `folio, etapa, tramite, descripcion, referencia, fecha_texto, fecha_iso,
     anexo, fila`), `vacio_explicito` (bool), `advertencias` (lista), `sha256`.
   - Localizar la tabla por el id/clase de H0; si hay 0 o > 1 candidatas ⇒ `ValueError('HISTORIA_TABLA_NO_RECONOCIDA')`.
   - Encabezado distinto de `HEADERS` ⇒ `ValueError('HISTORIA_ENCABEZADO_DISTINTO')`.
   - Formulario de login / texto de sesión (según H0) ⇒ `ValueError('HISTORIA_SESION')`.
   - Fecha no `dd/mm/aaaa` ⇒ fila con `fecha_iso=''` y advertencia `HISTORIA_FECHA_ILEGIBLE` (no se descarta).
   - Texto con `visible_cell_text` (de `descargador/vinculos.py`, o equivalente en el parser) sin scripts.
   - Prohibido: `eval`, ejecutar scripts, seguir enlaces, leer atributos `onclick` de Doc./Sel/Anexo salvo para
     saber si existe el icono de anexo.
   - Pruebas `tests/test_historia_html.py`: cada fixture de WP-52 → resultado esperado; ninguna variante inválida
     devuelve `filas == []` sin error, salvo el vacío explícito.
2. **Vínculo de historia** en `descargador/vinculos.py`: nueva función `extract_historia(table, tribunal_code)` que,
   sin exigir `ShowObservaciones`, devuelva por fila `{rit, nombre, rut, programa, tribunal_codigo, causa_id,
   enlace}` cuando haya **exactamente un** enlace de historia válido (función y aridad de H0), con `tribunal`
   y RIT concordantes. `extract` existente **no cambia**. Pruebas en `descargador/test_vinculos.py`
   (añadir casos: fila solo con historia; dos enlaces ⇒ descartada; tribunal distinto ⇒ descartada).
3. **Extensión** (`pagina.js`):
   - Copiar la estructura de `diary_open` en un bloque `if(command==="history_open")` antes de `search`.
   - Validar payload (claves exactas, regex), pantalla `seguimiento`, `run.diaryDocument` presente, y que el
     listado vigente contenga exactamente un enlace de historia igual (para escenario A, leer la URL de ese
     enlace; para B, ejecutar la petición intermedia en lista blanca y extraer `a1` de `Location` o del HTML).
   - Validar URL final: `new URL(...)`, origen, `pathname === "/SITFAWEB/ConsultaDetalleAtPublicoAccion.do"`,
     parámetros exactos (`TipoPopup` = valor de H0, `a1` con regex y largo ≤ 512), sin hash.
   - `fetch(url,{method:"GET",credentials:"include",cache:"no-store",redirect:"error",signal:AbortSignal.timeout(60000)})`
     (escenario B: solo la intermedia con `redirect:"manual"`).
   - Tope 8 MB (no 80) para esta respuesta; `content-type` debe incluir `html`.
   - Devolver `{status,type,body,params,startedAt,digest}`; errores `HISTORY_*` como `DIARY_*`.
   - `capabilities`: `version:'2.8.0'`, `protocolos:{bitacoras_lectura:1,descarga_conjunta:1,pdf_seleccion:1,historia_lectura:1}`.
   - `manifest.json`: `version` y `version_name` 2.8.0. **No tocar** `permissions` ni `host_permissions`.
   - `contrato_integral.py`: `PROTOCOLS` con `historia_lectura: 1`, `VERSION = '2.8.0'`, y actualizar el mensaje
     de `verify_extension`.
   - Prueba `descargador/test_historia_extension.cjs` (modelo: `test_bitacoras_extension.cjs`): una llamada válida
     hace 1 `fetch` GET con `credentials:'include'` a la ruta exacta; cambiar cualquier valor, añadir clave,
     otra ruta, otro host, `a1` inválido, redirección inesperada o respuesta > 8 MB ⇒ error y **cero** `fetch`
     adicionales; `document.cookie` nunca se lee.
4. **Gancho en `DiaryRunner`** (`lectura_bitacoras.py`): en `enumerar_listado`, cambiar
   `self.leer_bitacora(session, selection, query, number, item, seen)` por
   `self.procesar_fila(session, selection, query, number, item, seen)` y añadir
   `def procesar_fila(self, *a): return self.leer_bitacora(*a)`. Nada más. Toda la suite de bitácoras debe
   seguir verde sin cambios.
5. **Runner** (`descargador/lectura_historia.py`):
   - `class HistoryRunner(DiaryRunner)` que sobrescribe `run`, `procesar_fila`, `_cerrar`, `_resultado` y usa su
     propia sesión `_LecturaHistoria` (manifiesto `historias.json`, carpeta `respaldo/historia/` o
     `<destino>/historia-<fecha>` fuera del flujo de paquete).
   - `procesar_fila`: obtiene el vínculo de historia de la fila con `extract_historia` sobre la página vigente
     (cachear por página), agrupa por `('RUS', tribunal, causa_id)`; si la causa ya se procesó en este lote,
     solo añade el vínculo (registro → causa) y sigue; si no, la lee.
   - Leer: tope `MAX_HISTORIAS_POR_LOTE = 600` (excepción propia como `LimiteLecturas`), `bridge.call(
     'history_open', {...,'timeout':60000})`, validar `digest`/`params`/tamaño/`startedAt`, `read_historia`,
     guardar HTML con `m.save_exclusive`, calcular ventana (`nurus.capture.temporal.earliest`), contar
     `en_ventana` y `fuera_de_ventana`, estado `LEIDA` o el que corresponda.
   - Pausa `self.pause` entre lecturas reales; cancelación con `session.detener_si_cancelada()`.
   - `historias.json` nunca contiene `a1`. `diagnostico_historia.json` solo conteos y códigos.
   - Pruebas `descargador/test_lectura_historia.py` con un `Bridge` falso (modelo: `test_lectura_bitacoras.py`):
     dos ingresos misma causa ⇒ 1 lectura; causa sin enlace ⇒ `NO_DISPONIBLE`; respuesta de login ⇒ `FALLIDA`;
     digest distinto ⇒ `FALLIDA`; tope alcanzado ⇒ resto `PENDIENTE` y lote `INCOMPLETA`; cancelar ⇒ `CANCELADA`
     sin perder lo leído; reanudar no repite causas `LEIDA`; trámites fuera de ventana contados y no listados;
     ninguna clave `a1` en archivos de salida.
6. **Empaquetado:** añadir `'nurus.historia_html'` y `'nurus.capture.temporal'` a `hiddenimports` en
   `SITFA_Descargador.spec`.
7. Verificación: suites verdes; `node descargador/test_bitacoras_extension.cjs` y
   `node descargador/test_historia_extension.cjs`; `verificar_wp.py` PASA; E3 aprobada.

**Criterio de salida:** con fixtures, `HistoryRunner` produce `historias.json` y HTML correctos; ninguna variante
inválida aparece como vacía; extensión acotada a la ruta exacta; bitácoras sin cambios de comportamiento.
**No hacer:** UI, Excel ni ZIP; reutilización entre sesiones; abrir PDF; cambiar `extract` ni `leer_bitacora`.

---

## WP-54 · H2: Excel de revisión, respaldo y retención

**Tipo:** función nueva. **Riesgoso (retención) → E3.**
**Archivos:** `src/nurus/capture/historias.py` (nuevo), `descargador/lectura_historia.py` (llamar al export en
`_resultado`), `descargador/preferencias.py` (retención), `descargador/archivo_local.py`,
`descargador/ventana_resultados.py` (mostrar historia del lote), pruebas `tests/test_historia_export.py`,
`descargador/test_retencion_lotes.py` (casos nuevos).

Pasos:
1. `export_historias(manifest, destination) -> Path` en `src/nurus/capture/historias.py` (modelo:
   `nurus.capture.diaries.export_diaries`): carga `historias.json`, relee cada HTML verificando SHA-256, aplica
   `read_historia`, filtra por ventana y escribe `Historia_SITFA.xlsx` con las hojas y columnas **exactas** de
   02 §8.1. Escritura atómica (temporal + `replace`); destino existente ⇒ error.
2. Toda celda de texto con `literal()` (copiar la función de `export_csmp_capture.py` a un sitio común o
   importarla; no reimplementar distinto). Textos > 32.000 con `text_parts`.
3. Presentación con `excel_presentacion.configure_sheet` y anchos de `column_width`.
4. En `HistoryRunner._resultado`, generar el Excel como hace `DiaryRunner._resultado` con `Bitacoras_RUS.xlsx`
   (nombre alternativo con sufijo si existe; error de Excel no borra la lectura: queda `excel_error`).
5. Retención: en `preferencias.py`, añadir `'historias.json'` a las dos tuplas de controles
   (`_package_controls` y `old_batches`). Prueba: lote con solo `historias.json` antiguo es elegible; reciente no.
6. `archivo_local.py`/`ventana_resultados.py`: reconocer `historias.json` como fuente del lote y ofrecer
   «Abrir Historia_SITFA.xlsx». No crear visor.
7. Pruebas `tests/test_historia_export.py`: columnas y orden exactos; filtro de ventana; «Sin referencia.»
   intacto; texto que empieza con `=` queda como texto; texto largo partido sin pérdida; HTML alterado (hash
   distinto) ⇒ error; causa `FALLIDA` aparece en «Causas» y no en «Trámites».

**Criterio de salida:** Excel abre en Office (prueba Windows en CI si existe; si no, `openpyxl.load_workbook`),
cumple 02 §8.1; retención incluye historia.

---

## WP-55 · H3: historia en el paquete del Asistente

**Tipo:** contrato (aditivo). **Riesgoso → E3.**
**Archivos:** `src/nurus/capture/contract.py`, `contracts/csmp_capture_v1/DICCIONARIO.md`,
`contracts/csmp_capture_v1/README.md`, `descargador/export_csmp_capture.py`, `descargador/descargador.py`
(`package_current_lot`: parámetro opcional `historia=None`), pruebas `tests/test_capture_contract.py`,
`tests/test_capture_input.py`, `descargador/test_export_csmp_capture.py`.

Pasos:
1. **Contrato aditivo:** en `contract.py`, `EXTENSIONS = {'historia_tramites': ('folio', 'stage', 'annex_visible', 'raw_json')}`.
   El escritor usa `TABLES[t] + EXTENSIONS.get(t, ())` solo si alguna fila trae esas claves; el lector ya acepta
   columnas extra al final. Validar si están presentes: `annex_visible ∈ {'true','false'}`, `raw_json` objeto JSON
   (ya cubierto por la validación existente de `raw_json`). **No** cambiar `VERSION`/`VERSION_CONSOLIDATED`
   ni `required_features`.
2. Documentar en `DICCIONARIO.md` (sección `historia_tramites` → «Extensión aditiva historia v1») y en `README.md`.
3. **Exportador:** `export_capture(..., historia=None)` acepta rutas de `historias.json`; por causa crea la consulta
   y filas exactamente como 02 §8.2 (sin `remote_identity`), incidencias `HISTORIA_FALLIDA`/`HISTORIA_NO_DISPONIBLE`,
   `table_states['historia_tramites']` por `aggregate`. Sin historia ⇒ salida **idéntica** a la actual.
4. `package_current_lot(sheets, diary_manifest, target, consolidated, historia=None)` la pasa al exportador.
5. Pruebas:
   - Ida y vuelta: exportar con historia ficticia → `read_capture` valida → filas `scope=CAUSA`, `ingreso_id=''`.
   - **Compatibilidad del Asistente:** `import_capture` del paquete con historia produce el mismo trabajo
     (registros, observaciones, productos) que sin historia; paridad golden PASA; `event_index` no recibe filas CAUSA.
   - Paquete sin historia: bytes de tablas y manifiesto iguales a los actuales (salvo ids/fechas ya variables).
   - `python tools/flow_capture_cli.py validate contracts/csmp_capture_v1/fixtures/completa.zip` y `parcial.zip` PASA.

**Criterio de salida:** paquete con historia válido e importable por el Asistente actual sin cambios en su trabajo.
**No hacer:** tocar `capture_input.py`, `temporal.py`, motor o productos del Asistente.

---

## WP-56 · H4: interfaz, flujo, entrega Windows y prueba real

**Tipo:** interfaz + entrega. No riesgoso salvo que toque extensión (no debería).
**Archivos:** `descargador/descargador.py`, `descargador/flujo_paquete.py`, `descargador/preferencias.py`,
`descargador/MANUAL…`/`docs/MANUAL_DESCARGADOR_MVP_20261008.md` (sección nueva), `descargador/CHANGELOG.md`,
`docs/aceptacion/GUIA_PRUEBAS_USUARIO_<fecha>.md` (casos H-xx de 04), pruebas de UI existentes ampliadas.

Pasos:
1. Preferencias: `'leer_historia': False`, `'historia_en_paquete': True` (defaults), persistentes.
2. UI de 02 §9: casillas, confirmación previa con conteo de causas (contar con `extract_historia` sobre el
   listado ya leído; si no hay listado, mostrar «se contarán al consultar»), progreso, botón Avanzado
   «Leer historia del lote» y «Reintentar historias fallidas…», resumen final, deshabilitado si la extensión no
   declara `historia_lectura`.
3. `flujo_paquete.run(..., historia=False)`: tras `phase('bitacoras')`, si `historia`, `phase('historia')` y
   `HistoryRunner(...).run(batch, backup)`. **Un fallo de historia no detiene el paquete:** se captura la
   excepción, se registra en `proceso.json` y el paquete se crea con historia `PARCIAL/FALLIDA` declarada.
   `partial` del resultado incluye el estado de historia.
4. Manual y CHANGELOG (versión 2.8.0): qué lee, qué no (sin PDF), ventana de 4 meses, cómo actualizar la extensión.
5. Construcción Windows con la CI existente (`tools/build_integral.ps1`); el artefacto y la extensión deben
   mostrar 2.8.0 y el mismo commit.
6. Guía de prueba real con los casos H-01…H-12 de [04](04_PRUEBAS_Y_ACEPTACION.md) y acta
   `docs/aceptacion/ACTA_HISTORIA_<fecha>.json` (plantilla en 04). Solo el usuario la firma.

**Criterio de salida:** función desactivada ⇒ comportamiento y productos actuales idénticos; activada ⇒ casos
H-xx PASA en el PC del usuario; acta firmada. Hasta el acta, la opción dice «(prueba)».

---

## Estimación (N2, recalcular tras WP-52)

| WP | Jornadas | Supuesto |
|---|---|---|
| WP-52 | 1–2 + tiempo del usuario | Extractor ya entregado; falta corrida del usuario y fixtures fieles |
| WP-53 | 3–5 | Escenario A o B; tabla única sin paginación compleja |
| WP-54 | 1–2 | Modelo `export_diaries` |
| WP-55 | 1–2 | Extensión aditiva sin cambio de versión |
| WP-56 | 2–3 | UI simple + build + guía |
| **Total** | **8–14** | Escenario C o paginación con estado: +3–5 |
