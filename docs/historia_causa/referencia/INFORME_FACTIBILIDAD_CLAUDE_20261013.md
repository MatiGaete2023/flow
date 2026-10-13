# Informe de factibilidad — Lectura de la «historia de la causa» en el Descargador

Fecha: 13 de octubre de 2026 · Corte revisado: `feat/ejecucion-plan-20261008` @ `1a2dd0f` (código = `622ed4f`) · Documento de análisis: no modifica código, reglas ni contratos.

Convención epistemológica: **N1** hecho verificado en repo/documento (con ruta) · **N2** inferencia con supuesto explícito · **N3** especulación / sin datos.

---

## 0. Veredicto

| Dimensión | Veredicto | Razón corta |
|---|---|---|
| Técnica (arquitectura, código) | **FACTIBLE, esfuerzo medio** | Ya existe el patrón completo para bitácoras (`DiaryRunner` + `diary_open` + parser + Excel + tabla del contrato). Historia es el mismo patrón con otra fuente. |
| Local (sin nube, sin credenciales) | **FACTIBLE, sin cambiar el modelo de seguridad** | Misma ruta: Chrome del usuario → extensión → puente `127.0.0.1` → archivos locales. Sin hosts ni permisos nuevos (N2: si el endpoint está bajo `familia.pjud.cl`). |
| Datos (¿sabemos qué devuelve SITFA?) | **BLOQUEADO hoy** | N1: el repo **no contiene** URL, parámetros ni estructura HTML del popup de historia. Solo existe el enlace `ShowHistoria(...)`. Sin muestra anónima real no se puede programar un parser fiable (decisión ya tomada en el plan). |
| Gobernanza | **BLOQUEADO hasta decisión expresa** | N1: `PLAN_VIGENTE.md` → «Historia SITFA/PDF y reglas CT-03..10: fuera de alcance hasta nueva decisión». Este pedido es esa decisión, pero debe quedar registrada por el usuario. |
| PDFs por trámite | **EXCLUIDO por decisión del usuario (13-oct)** | No se descargan ni abren PDFs. Solo se lee lo que muestra la ventana de historia (trámite, descripción, fecha, folio, etc.). Elimina la fase PDF y el riesgo de custodia/volumen. |

**Recomendación:** hacerlo, en fases, empezando por la fase 0 (obtener estructura anónima real; costo para el usuario ≈ 1 ejecución local). Ninguna línea de parser debe escribirse antes.

---

## 1. Qué existe hoy (N1)

### 1.1 Lo que ya se discutió y decidió

| Fuente | Contenido |
|---|---|
| `docs/plan_20261008/01_ESPECIFICACION_DESCARGADOR_RUS_SITFA.md` §3 `D-HIS-01` | Captura «Historia de causa SITFA: trámites, descripción, fecha, tipo, referencia/folio, relación con ingreso». Estado: «Reconocer vínculo actual; extractor PENDIENTE». |
| ídem §8 | «`ShowHistoria` no equivale a extracción histórica implementada». Pide capturar HTML/HAR representativos (encabezados, orden, paginación, links PDF, variantes por tribunal/etapa). Propone `HistoriaRunner` análogo a `DiaryRunner` y comando de extensión `history_open`. PDF «únicamente por selección explícita». |
| ídem §3 `D-T12`, §12 Etapa 4 | Criterio de aceptación: «fechas, trámites y referencias reales comparados con pantalla; paginación y cobertura documentadas». «Una función no comprobada no debe aparecer como disponible en UI». |
| ídem §13 | `[CONFLICTO_ABIERTO]` Historia SITFA y PDF: «el esquema de trámites no está acreditado… Resolución: muestras representativas». |
| `docs/plan_20261008/04_FLUJO…md` `CA-03` | Sin extractor de historia no se pueden activar reglas que dependan de ella. |
| `docs/ACEPTACION_INSTITUCIONAL_PENDIENTE.md` `G9`, `G12` | G9: caracterizar historia real (identidad CAUSA/INGRESO, columnas, paginación, origen, referencias) con diccionario anonimizado. G12: ampliar historia/contexto solo tras aceptación. |
| `PLAN_VIGENTE.md` | «Historia SITFA/PDF y reglas CT-03..10: fuera de alcance hasta nueva decisión.» |
| `docs/PROCESO_COMPLETO_PROYECTO_20261013.md` §tabla de contrato | `historia_tramites` (30 columnas): «Trámites (fuera de alcance por ahora)». |

### 1.2 Lo que ya está construido y se reutiliza

| Pieza | Ruta | Estado / relevancia |
|---|---|---|
| Identidad remota de la historia | `descargador/vinculos.py:93-107` | Lee `ShowHistoria(tribunal, RIT, causa_id)` (3 args). Verifica `hist[0]==tribunal`, `hist[2]==obs[0]` (causa) y RIT concordante. **La historia es de alcance CAUSA, no ingreso.** |
| Filas sin enlace de historia | `vinculos.py:102-104`, `test_lectura_bitacoras.py:185` | En **Espera** la fila puede no traer `ShowHistoria`. Esas causas quedarían sin historia (cobertura `NO_DISPONIBLE`), nunca fabricadas por RIT. |
| Lector secuencial | `descargador/lectura_bitacoras.py` (`DiaryRunner`, `_Lectura`, `leer_bitacora`, `_abrir_bitacora`) | Pausa 1 s (`PAUSA_SITFA_S`), tope 600 lecturas (`MAX_LECTURAS_POR_LOTE`), checkpoints, reanudación, reutilización ≤12 h (EF-04), cancelar/pausar, manifiesto `bitacoras.json`, HTML con SHA-256. |
| Comando de extensión | `descargador/extension/pagina.js:393-431` | `diary_open`: valida la tupla contra el DOM del listado (exactamente 1 enlace coincidente), arma URL `IrPopUpInformesAccion.do` con 7 parámetros numéricos, `GET`, `credentials:include`, `redirect:'error'`, timeout ≤60 s, tope 80 MB, devuelve base64 + SHA-256. Errores tipados `DIARY_*`. |
| Parser | `src/nurus/bitacora_html.py` (`read_snapshot`, `decode_snapshot`) | Modelo para `historia_html.py`: decodificación cp1252/utf-8 sin reemplazos, identidad en campos ocultos. |
| Excel de copia íntegra | `src/nurus/capture/diaries.py:327` (`export_diaries`) | Modelo para `Historia_RUS.xlsx` (sin truncar, hojas con huellas, texto extenso particionado). |
| Contrato de intercambio | `contracts/csmp_capture_v1/`, `src/nurus/capture/contract.py:94,435-490`, `coverage.py:12`, `requests.py:10` | **La tabla `historia_tramites` ya existe** (30 columnas: `event_id`, `event_at`, `time_precision`, `scope` CAUSA/INGRESO, `event_type`, `description`, `reference`, `status`, `document_id`, `identity_status`, `raw_json`…), con validaciones: CAUSA ⇒ `ingreso_id` vacío; documento referido debe existir; estado de tabla coherente con filas. `export_csmp_capture.py:300` ya la declara `NO_SOLICITADA` por defecto. |
| Documentos | tabla `documentos`, `contract.py:441-452` | `content_read` debe ser `'false'`; `download_state` ∈ {NO_SOLICITADA, NO_DISPONIBLE, FALLIDA, DESCARGADO}; `DESCARGADO` exige archivo íntegro con SHA-256 en `files` y `originals_authorized`. |
| Estructura anónima | `descargador/extraer_estructura_bitacora.py`, `extraer_estructura_har.py` | Ya aceptan el nombre `ShowHistoria` como acción a registrar (solo nombres de campos). Patrón para el nuevo extractor anónimo de historia. |
| Protocolo extensión↔app | `descargador/contrato_integral.py` | `PROTOCOLS` se compara por **igualdad exacta** con lo que declara la extensión; agregar `historia_lectura` exige cambio simultáneo y versión (2.7.6 → 2.8.0). |
| Empaquetado | `descargador/SITFA_Descargador.spec` | `hiddenimports` explícito (`nurus.bitacora_html`): un parser nuevo debe agregarse ahí o falla el EXE. |

### 1.3 Lo que NO existe (N1)

- URL, método, parámetros y HTML del popup/endpoint de historia.
- Encabezados de tabla, orden, paginación, variantes por tribunal/etapa, forma de los enlaces PDF.
- Qué ocurre con el rol/permiso del usuario en esa pantalla (¿todos los tribunales la muestran?).
- Fixture ficticio de historia (solo hay `ShowHistoria('49','X-1-2026','100')` como enlace en pruebas).

---

### 1.4 Hallazgos de la captura visual aportada por el usuario (13-oct, sin reproducir datos)

La captura es de la ventana «Información de la Causa» (la causa lleva una marca de reserva visible en el encabezado). Contiene datos reales; **no se copió al repo**. Solo se registran sus rasgos estructurales:

- **N1 – Endpoint:** ruta `/SITFAWEB/ConsultaDetalleAtPublicoAccion.do?TipoPopup=3&a1=<token>`. Mismo origen `familia.pjud.cl` ⇒ no exige permisos nuevos en el manifiesto. Es **distinta** de la ruta de bitácoras (`IrPopUpInformesAccion.do`), por lo que `history_open` necesita su propia ruta en lista blanca.
- **N1 – Parámetro `a1` opaco** (aspecto de texto cifrado/base64 codificado en URL). A diferencia de la bitácora, **la URL no se puede construir con `causa_id` + `tribunal`**. Sin conocer cómo se genera `a1`, no hay que inventarlo ni derivarlo.
- **N2 – Consecuencia:** `a1` probablemente lo produce la función JS `ShowHistoria` de la página (en el cliente) o el servidor lo entrega en el propio listado. La extensión solo puede reproducir esa llamada si `a1` está en el DOM/JS del listado actual o se obtiene por una petición de lectura del mismo origen. **Si depende de clic/ventana con estado, el esfuerzo sube** (ver R1). Verificable en fase 0 con la estructura anónima del listado (cuerpo de `ShowHistoria` y atributos del enlace), sin valores.
- **N2 – Posible vigencia limitada de `a1`** (token de sesión/tiempo): obligaría a leer cada historia **inmediatamente tras** obtener el listado, no reutilizando tokens guardados.
- **N1 – Contenido de la pestaña «Historia»:** columnas `Folio · Doc. · Sel · Anexo · Etapa · Trámite · Desc. Trámite · Referencia · Fecha Tram`. Orden visible: **por fecha descendente**, no por folio (folios consecutivos pueden aparecer invertidos dentro de una misma fecha). Referencia puede ser «Sin referencia.» (valor literal a conservar, no vacío).
- **N1 – Folios con saltos** en lo visible (hay folios ausentes entre filas mostradas). **Inferencia N2:** la continuidad de folios no acredita completitud ni su ausencia; puede haber filtro, ocultamiento o folios de otras pestañas. No se debe deducir «historia completa» por folio; se necesita comprobar paginación/scroll.
- **N1 – Otras pestañas:** `Notificación`, `Diligencias`, `Escritos`; y botones `Litigantes`, `Materias`, `Plazos`, `Remisión`. **Fuera de alcance** (solo «Historia»).
- **N1 – Encabezado de causa:** RIT, RUC, Caratulado, F. Ingreso, Proc., Forma Inicio, Est. Adm., Etapa, Estado Proc., Tribunal, Texto Requerimiento (icono de documento). Sirve para el cotejo de identidad visible (como `visible_identity_warnings` en bitácoras).
- **N1 – Iconos de la columna «Doc.», «Sel» y «Anexo»** son enlaces/controles de documento: **se ignoran** (decisión del usuario: sin PDFs). Opcionalmente se registra solo la *presencia* de anexo como dato visible, sin abrirlo.
- **N3 – Causa reservada:** puede existir acceso restringido o contenido distinto según el perfil; el lector debe tratar una respuesta de acceso denegado como `NO_DISPONIBLE`, nunca como «sin trámites».
- **Higiene de datos:** la captura incluye RIT, RUC y nombre. Según `AGENTS.md`, las muestras deben llegar solo como estructura anónima. Para próximas aclaraciones, tapar RIT/RUC/caratulado o usar el extractor local.

**Campos que se leerán (alcance confirmado por el usuario):** `Folio`, `Etapa`, `Trámite`, `Desc. Trámite`, `Referencia`, `Fecha Tram` (+ indicador de anexo si es visible), más el encabezado de causa para validar identidad. Nada más; el análisis posterior lo hará el Asistente en otro paso (fuera de este alcance).

---

## 2. Cómo funcionaría (diseño propuesto)

### 2.1 Flujo (idéntico al de bitácoras, una fuente más)

```
Listado Seguimiento (ya leído)
  └─ vinculos.extract → (tribunal, RIT, causa_id) por fila con ShowHistoria
       └─ dedupe por (tribunal, causa_id)          ← una lectura por CAUSA, no por ingreso
            └─ extensión: history_open (GET, solo lectura, misma sesión Chrome)
                 └─ puente 127.0.0.1 → app: valida digest/params/tamaño
                      └─ guarda historia-<clave>-<sha16>.html (exclusivo, inmutable)
                           └─ parser historia_html.py → trámites + referencias a documentos
                                └─ historias.json (manifiesto) + Historia_RUS.xlsx
                                     └─ tabla historia_tramites (+ documentos) en Captura_CSMP.zip
```

### 2.2 Cambios por componente

| # | Componente | Cambio | Tamaño (N2) |
|---|---|---|---|
| 1 | `descargador/extension/pagina.js` | Comando `history_open`: validar que el enlace `ShowHistoria` con los 3 valores exista exactamente una vez en el listado actual; construir la URL **fija** (ruta en lista blanca, parámetros numéricos/alfanuméricos validados por regex); mismas defensas que `diary_open` (sin redirecciones, tope de tamaño, timeout). Códigos `HISTORY_*`. Agregar `historia_lectura:1` a `capabilities`; versión 2.8.0. Sin cambios en `manifest.json` si el host es `familia.pjud.cl`. | ~80-120 líneas JS |
| 2 | `descargador/contrato_integral.py` | `PROTOCOLS['historia_lectura']=1`, `VERSION`. Mantiene el bloqueo si extensión y app no coinciden. | trivial |
| 3 | `src/nurus/historia_html.py` (nuevo) | Parser determinista (lxml) con el mismo rigor de `bitacora_html.py`: identidad, filas, fechas ISO sin inventar hora/zona, texto íntegro, enlaces a documento como **referencia** (no se abren). Error explícito ante estructura desconocida (nunca «0 trámites» silencioso — lección de la falla C-01). | 250-400 líneas + fixtures |
| 4 | `descargador/lectura_historia.py` (nuevo) o extensión de `lectura_bitacoras.py` | `HistoriaRunner`: reutilizar `_Lectura`, pausa, tope, checkpoint, reutilización, cancelación. Estados `LEIDA/FALLIDA/SIN_VINCULO/NO_DISPONIBLE/PENDIENTE`. Manifiesto propio (`historias.json`) para no alterar `bitacoras.json` v1. | 300-450 líneas |
| 5 | `src/nurus/capture/historias.py` (nuevo) | `export_historias` → `Historia_RUS.xlsx` (copia íntegra, hoja de control con cobertura/huellas) y filas de `historia_tramites`. | 200-300 líneas |
| 6 | `descargador/export_csmp_capture.py` | Poblar `historia_tramites` (folio en `reference`, resto de columnas visibles en `raw_json`); **tabla `documentos` no se toca** (sin PDFs), `document_id` queda vacío; `table_states['historia_tramites']` por consulta. Contrato v1 **sin cambio de esquema** (N2: `raw_json` absorbe columnas imprevistas). | ~60 líneas |
| 7 | `descargador/descargador.py` | Botón **«Leer historia de causas del lote»** (+ reintento de fallidas), casilla «Incluir historia en el paquete» (preferencia persistente como «Incluir XLSX consolidado»), progreso y resumen. **Oculto/deshabilitado hasta que `capabilities` y la aceptación G9 lo habiliten** (regla del plan). | ~100-150 líneas |
| 8 | `ventana_resultados.py`, `resultados.py` | Mostrar lotes con historia, estado, apertura de carpeta; resumen de cobertura. | ~80 líneas |
| 9 | `extraer_estructura_historia.py` (nuevo) | Igual a `extraer_estructura_bitacora.py`: conteos, códigos, encabezados normalizados, sin valores de celdas. **Es lo que desbloquea la fase 0.** | ~120 líneas |
| 10 | Retención/anonimización | Incluir `historia-*.html` y `historias.json` en la retención de 60 días / 10 sesiones y en `anonimizar`/escáner de identidades reales (tools + `check_real_ids`). | ~40 líneas + pruebas |
| 11 | Pruebas | Fixtures **ficticios** de historia (1 página, varias, vacía, sin enlace, identidad distinta, charset cp1252, estructura desconocida); pruebas JS análogas a `test_bitacoras_extension.cjs`; pruebas de contrato; golden/línea base actualizados. | ~40-60 pruebas |
| 12 | Asistente (opcional, fase posterior) | Hoy solo usa `historia_tramites` en reglas CT desactivadas y para eventos de alcance INGRESO (`capture_input.py:119`). **No es necesario para la petición** (leer desde el Descargador). Mostrar historia en el Asistente = decisión aparte. | fuera de alcance |

### 2.3 Salidas para el usuario

1. `Historia_RUS.xlsx`: una fila por trámite (fecha, tipo, descripción, referencia/folio, estado, origen, identidad tribunal+causa, huella y ubicación de fuente) + hoja de control (qué causas se leyeron, fallaron, no tenían enlace).
2. HTML originales íntegros con SHA-256 en la carpeta del lote (respaldo, igual que bitácoras).
3. Tabla `historia_tramites` dentro de `Captura_CSMP.zip` (si se marca la casilla).
4. Resumen en pantalla: leídas / fallidas / sin vínculo / ya reutilizadas.

### 2.4 Reglas de diseño heredadas (no negociables)

- Identidad solo por `(tribunal, causa_id)` remotos; **prohibido** construir consultas por RIT/nombre/RUT.
- Historia = alcance **CAUSA**: no atribuir trámites a un NNA/ingreso (el contrato lo valida).
- Fallo/parcial/no leído **≠** «sin actuaciones» (estados de cobertura del contrato).
- Solo lectura; sin escritura RUS; sin credenciales/cookies en código, logs o paquete.
- Fecha de captura ≠ fecha del trámite; hora y zona desconocidas se conservan como tales.

---

### 2.5 Ventana de 4 meses (decisión del usuario)

- Se conservan como filas (Excel y `historia_tramites`) solo los trámites con `Fecha Tram` dentro de la ventana, con la misma regla de `src/nurus/capture/temporal.py:earliest(as_of)` (4 meses naturales hacia atrás desde la fecha de corte). Es coherente con D5/CT-01.
- La fecha de corte se fija al leer; se guarda en el manifiesto junto al criterio (`criterio_temporal: ventana_4_meses`) para que se vea en el Excel de control, igual que `carga.py` documenta su criterio.
- **Cobertura:** «leído completo respecto de la ventana». Los trámites anteriores **no se leyeron**, no «no existen»: nunca presentar como historia total ni deducir ausencia de actuaciones antiguas.
- **Optimización condicionada (N2):** si la muestra muestra orden descendente por fecha (visible), se podría dejar de paginar al cruzar el límite. Solo se activa si la fase 0 comprueba el orden; si no, se leen todas las páginas y se filtra después.
- El HTML original queda íntegro en disco como respaldo local (con SHA-256) aunque solo se exporte la ventana; el filtrado ocurre en el parser, no al guardar.
- Fechas `dd/mm/aaaa` sin hora: `time_precision='DIA'`, zona desconocida (contrato v1). Fila sin fecha legible: no se descarta; se conserva con incidencia.

---

## 3. Funcionamiento local (verificación punto por punto)

| Requisito | Cómo se mantiene | Nivel |
|---|---|---|
| Sin nube / sin servicios externos | Único tráfico: Chrome del usuario → `familia.pjud.cl` (sesión ya abierta) y `127.0.0.1` (puente). Nada sale a otro host. | N1 (arquitectura vigente) |
| Sin credenciales | `credentials:'include'` usa la sesión del navegador; la extensión no lee cookies (`manifest`: solo `activeTab`, `scripting`). | N1 |
| Permisos mínimos | `host_permissions` ya cubre `familia.pjud.cl` y `127.0.0.1`. Una nueva ruta del mismo origen no exige permiso nuevo. Si la historia viviera en **otro** dominio, habría que ampliar el manifiesto (cambio E3). | N2 |
| Sin IA ni procesamiento remoto | Parser determinista (lxml) y reglas Python; sin llamadas a modelos. | N1 (patrón vigente) |
| Datos en disco del usuario | HTML, JSON y Excel en la carpeta del lote; retención 60 días/10 sesiones; escáner de identidades reales vigente. | N1 |
| Sin datos reales en repo/CI/chats | Fixtures ficticios; la muestra real solo sale como estructura anónima (`extraer_estructura_historia.py`). | N1 (AGENTS.md) |
| Puente | Mismo código efímero, origen de extensión validado, sin persistencia. | N1 |
| Empaquetado Windows | Agregar módulos a `hiddenimports`; sin dependencias nuevas (`lxml`, `openpyxl` ya fijadas con hash). | N1 |

Riesgo local residual: **volumen y sensibilidad**. La historia contiene trámites de causas de familia con datos de NNA. Se trata igual que las bitácoras (local, sin compartir). El HTML íntegro de historia puede pesar más que una bitácora (N3: desconocido) → revisar el tope de 8 MB por respuesta de `_abrir_bitacora`.

---

## 4. Qué se necesita

### 4.1 Del usuario (bloqueantes)

1. **Decisión escrita**: autorizar sacar «Historia SITFA» del estado «fuera de alcance» (se registra como decisión nueva en `PLAN_VIGENTE.md`/`ESTADO.json`) y confirmar alcance inicial: solo lectura tabular (sin PDF).
2. **Muestra estructural anónima (fase 0).** En el PC autorizado, con sesión SITFA abierta y **sin copiar contenido**:
   - Abrir la historia de 2-3 causas variadas (una de Espera, una de Cumplimiento, una con muchos trámites, idealmente de distinto tribunal y de Laja/Mulchén/Tomé).
   - Generar la estructura anónima con el extractor local (hay que crearlo, ver §2.2 #9) o, mientras tanto, `extraer_estructura_har.py` sobre un HAR de esa navegación → solo `estructura.json`.
   - Anotar a mano (sin datos): ¿se abre en popup o página?, ¿hay paginación o «ver más»?, ¿orden cronológico asc/desc?, ¿cada trámite trae enlace a PDF?, ¿hay trámites visibles solo según perfil?
3. **Criterio de «historia»**: resuelto — solo ventana de 4 meses y solo los campos visibles de la tabla (folio, etapa, trámite, descripción, referencia, fecha); sin texto de resoluciones ni PDFs.
4. **Ejecución de la prueba real** posterior (acta tipo G9): comparar N causas contra pantalla; registrar PASA/FALLA y conteos, sin datos.

### 4.2 Del programador / agente

- Seguir `AGENTS.md`: leer `PLAN_VIGENTE.md`, `ESTADO.json`, `PLAN_IMPLEMENTACION.md`; nuevo WP (propuesto **WP-100 «Lectura de historia de causa»**) con ficha, dependencias y criterio de salida.
- Revisión **E3 independiente** obligatoria (decisión j del 12-oct): toca extensión SITFA y posiblemente `contracts/`.
- `python tools/verificar_wp.py` PASA contra `linea_base.json` (hoy 932 Asistente / 283 Descargador, paridad golden 17); regenerar línea base solo con el candidato congelado.
- Extensión y app de la **misma versión y SHA** (la extensión se instala a mano: el usuario debe recargarla; el Descargador debe bloquear con mensaje claro si la versión no coincide — mecanismo ya existente).

### 4.3 Dependencias previas / recomendadas

- **WP-99 / C-01 abierto (N1):** bitácoras de Espera se leen vacías (36 de 40 lecturas sin entradas) y falta `estructura.json` anónima de Espera/Cumplimiento. El plan dice «no desarrollar historia sobre captura inestable» (spec §12 Etapa 1). **Inferencia N2:** la fase 0 y el diseño pueden avanzar en paralelo (no tocan el parser de bitácoras), pero **no publicar** la lectura de historia a uso real hasta cerrar C-01, porque comparte listado, vínculos y runner.
- Las filas de Espera sin `ShowHistoria` limitan la cobertura exactamente donde hay más fallos de bitácora (N2: puede que Espera dependa más de bitácora que de historia).

---

## 5. Plan por fases

| Fase | Contenido | Entrada | Salida / criterio | Esfuerzo (N2) |
|---|---|---|---|---|
| **0. Descubrimiento** | Crear `extraer_estructura_historia.py`; usuario genera estructura anónima de 2-3 causas; documentar contrato real (URL ruta, parámetros, encabezados, paginación, enlaces PDF) en `contracts/csmp_capture_v1/` como diccionario anonimizado. | Decisión §4.1 #1-3 | Diccionario G9 aprobado por el usuario | 0,5 día programador + ~30 min usuario |
| **1. Núcleo lector** | `history_open` + `historia_html.py` + `HistoriaRunner` + manifiesto + fixtures ficticios + pruebas JS/Python. Sin UI visible. | Fase 0 | Pruebas verdes; fallos tipificados; sin «0 trámites» silenciosos | 2-3 días |
| **2. Salidas** | `Historia_RUS.xlsx`, filas `historia_tramites` en paquete, resultados/retención/anonimizador, `hiddenimports`. | Fase 1 | Round-trip contrato (export→validate→import) PASA; golden sin cambios | 1-2 días |
| **3. UI y flujo** | Botón/casilla/progreso/reintento, integración con «Consultar y crear paquete» (historia opcional, apagada por defecto), mensajes de cobertura. | Fase 2 | Prueba de UI (Tk) y flujo con puente simulado | 1 día |
| **4. Revisión y entrega** | E3, `verificar_wp.py`, construcción Windows, versión 2.8.0 + SHA. | Fase 3 | Candidato congelado + línea base | 1 día |
| **5. Prueba real (usuario)** | Guía tipo `GUIA_PRUEBAS_USUARIO` con casos H-01…H-0n (leer, reutilizar, cancelar/retomar, causa sin enlace, comparación contra pantalla, volumen) y acta. | Fase 4 | Acta: PASA/FALLA con conteos | 1 sesión usuario |
| ~~6. PDFs~~ | **Descartada por decisión del usuario.** Si un trámite muestra un enlace a documento, se ignora (no se sigue ni se descarga). | — | — | — |

Total fases 0-4: **≈ 5,5-8 días de programación** (N2; supone que la estructura real es HTML tabular sin JS dinámico ni CAPTCHA). Subir a 10-15 días si la historia carga por XHR/JSON, paginación por formulario con estado de sesión, o múltiples variantes por tribunal.

---

## 6. Riesgos

| # | Riesgo | Prob. (N2) | Mitigación |
|---|---|---|---|
| R1 | La historia no se obtiene por un GET simple: la URL lleva un `a1` opaco (confirmado en captura, §1.4) | **media-alta** | Fase 0 lo determina antes de invertir. La extensión puede replicar la llamada solo si es lectura y mismo origen; si requiere clic/DOM, se rediseña (más costo). |
| R2 | Estructura distinta por tribunal/etapa | media | Parser con mapeo versionado + fallo explícito ante encabezados desconocidos; muestras de ≥3 tribunales. |
| R3 | Paginación o historia muy larga | media | Iterar páginas con tope y cobertura `PARCIAL`; no declarar `COMPLETA` sin comprobar total. |
| R4 | Falsa sensación de completitud (repetir el defecto C-01) | alta si se omite | Estados de cobertura del contrato, `NO_LEIDA`/`FALLIDA` visibles, prueba negativa «estructura desconocida ⇒ error». |
| R5 | Más tráfico a SITFA | baja | Una lectura por causa (menos que por ingreso), misma pausa de 1 s, tope por lote, reutilización ≤12 h. Estimación: 344 registros habituales ⇒ ≤344 lecturas ≈ 6+ min solo de pausa (N2; el nº de causas únicas es N3). |
| R6 | Filtración de datos en pruebas/CI | baja con proceso | Solo fixtures ficticios; extractor anónimo; escáner de identidades en CI. |
| R7 | Desalineación extensión/app | media (instalación manual) | `PROTOCOLS` por igualdad + versión + SHA: la app ya rechaza extensión antigua con mensaje. |
| R8 | Colisión con ramas/estado del plan | baja | Trabajar desde `feat/ejecucion-plan-20261008` (decisión n), un WP, sin tocar motor/Outlook/textos. |
| R9 | PDF: tamaño, tipo, custodia | — | Eliminado: no hay descarga de PDFs. |

---

## 7. Decisiones que debe tomar el usuario

1. ¿Autoriza abrir WP-100 y retirar «Historia SITFA» de «fuera de alcance»? (necesario por `PLAN_VIGENTE.md`).
2. ~~Alcance temporal~~ Resuelto: **solo ventana de 4 meses** (decisión del usuario, 13-oct). Ver §2.5.
3. ¿Historia solo desde el listado del lote (causas visibles en la captura) o también consulta por causa suelta? **Recomendación:** solo desde el lote (cumple «sin fabricar consultas por RIT»).
4. ¿Casilla «Incluir historia» apagada por defecto? **Recomendación:** sí (menos tráfico, menos datos sensibles).
5. ~~PDFs~~ Resuelto: excluidos (decisión del usuario).
6. ¿Se espera cerrar C-01 antes de publicar al uso real? **Recomendación:** sí.

---

## 8. Especulación (N3, no verificable hoy)

- Que `ShowHistoria` abra un popup análogo a `IrPopUpInformesAccion.do` con otro `tipo_popUp` (reutilizaría casi todo el código de `diary_open`).
- Que la historia sirva luego para activar CT-03/CT-04/CT-08 (hoy desactivadas, sin aceptación), lo que requiere decisión y acta aparte; **este informe no lo propone**.

---

## 9. Próximo paso mínimo (una sola acción)

Responder las decisiones 1-2 de §7. Con eso se crea el extractor anónimo `extraer_estructura_historia.py` y la ficha WP-100; el usuario ejecuta una vez el extractor sobre 2-3 causas y entrega solo `estructura.json`.

---

### Fuentes (corte `1a2dd0f`)

`PLAN_VIGENTE.md` · `AGENTS.md` · `docs/plan_20261008/01_ESPECIFICACION_DESCARGADOR_RUS_SITFA.md` (§1, 2, 3, 8, 9, 12, 13) · `docs/plan_20261008/04_FLUJO_TRABAJO_INTEGRADO_CSMP.md` (CA-03) · `docs/ACEPTACION_INSTITUCIONAL_PENDIENTE.md` (G9, G12) · `docs/PROCESO_COMPLETO_PROYECTO_20261013.md` · `contracts/csmp_capture_v1/{README,DICCIONARIO,DECISIONES_PENDIENTES}.md` · `descargador/{vinculos,lectura_bitacoras,contrato_integral,export_csmp_capture,extraer_estructura_har,extraer_estructura_bitacora}.py` · `descargador/extension/{manifest.json,pagina.js}` · `descargador/SITFA_Descargador.spec` · `src/nurus/{bitacora_html.py,capture/{contract,coverage,requests,temporal,diaries}.py,personal/capture_input.py}` · `Informe_integral_Flow_CSMP.pdf` (13 pp., 13-oct-2026).
