# E3 consolidado final · WP-63 (PR #34) y WP-80 (PR #35)

**Versión final para continuar**, 11 de octubre de 2026. Reemplaza la versión anterior de este archivo (commit
`e67a234`). [WP-80.md](WP-80.md) se conserva como detalle de la primera revisión de WP-80.

Revisor y Auditor: sesión `session_016DC9qxvuqT6mr91tHBaw7b` (Claude), de una raíz distinta a la de ambos Ejecutores:

- WP-80: Claude, `session_012yjVgL…`.
- WP-63: Codex `/root`.

Fuentes contrastadas:

1. Esta revisión, sobre el repositorio.
2. El reporte de WP-63 (`REPORTE_REVISION_20261011.md`).
3. El análisis de ChatGPT entregado por el usuario. Solo se recibió su resumen, no el archivo completo; lo que no
   aparece en el resumen no se evalúa.

Convención: **N1** hecho comprobado aquí con un comando o una ejecución; **N2** inferencia por lectura de código, con
el supuesto explícito.

> **Actualización (11 de octubre de 2026, tarde):** WP-63 está **INTEGRADO** en `feat` (`f64bc7c`). WP-80 fusionó `feat`
> y quedó APROBADO / CONFORME con condiciones (§5). Las tablas siguientes describen el estado previo a esa integración.

## 1. Estado del repositorio (N1, al 11 de octubre de 2026)

| Elemento | WP-63 | WP-80 |
| --- | --- | --- |
| PR / rama | #34 · `wp/auditoria-refactor-20261011` | #35 · `wp/80-auditoria` |
| Commit revisado | `84b92e4` | `ea81235` |
| Commits / archivos | 20 / 56 (+4.068/−542) | 18 / 61 (+625/−307) |
| Base | `2cf8ca8` (punta de `feat`, sin cambios) | `2cf8ca8` |
| Estado / CI | borrador · CI verde (2/2) | borrador · CI verde (2/2) |
| Ejecutor | Codex (commits firmados «el usuario titular») | Claude |

- Las dos ramas modifican 17 archivos en común.
- Una fusión de prueba de ambas da **8 conflictos de contenido**:
  - `export_csmp_capture.py`, `lectura_bitacoras.py`, `test_paquete_lote_actual.py`;
  - `capture/contract.py`, `test_capture_input.py`, `test_herramientas_plan.py`;
  - `check_real_ids.py`, `verificar_wp.py`.
- `ESTADO.json` se regenera; no se edita a mano.
- Ninguna rama cambió desde la revisión anterior. No hubo fusiones ni publicaciones.

## 2. Contraste de las tres fuentes

| Punto | ChatGPT (resumen) | Esta revisión | Resultado |
| --- | --- | --- | --- |
| Paquetes complementarios; no se fusionan sin resolver conflictos | Sí | Sí; además nombra los 8 archivos en conflicto | **Coinciden** |
| Archivos en común | 17 | 17 | **Coinciden** (N1) |
| WP-80: CAMBIOS / NO CONFORME | Sí | Sí | **Coinciden** |
| WP-63 corrige `zlib.error`, pero no revisa extensiones desconocidas en el historial | Sí | Sí | **Coinciden** (N1) |
| Estado E3 de WP-63 | «pendiente de E3 independiente» | antes: «APROBADO con observaciones», con revisión de pruebas y herramientas por muestreo | **Se ajusta** (ver §5): la revisión de código de WP-63 fue **parcial**; no corresponde un APROBADO pleno. |
| GUI: 6 hallazgos previos | Sí | Sí | **Coinciden** |
| GUI: control de calidad en paquetes mixtos | Hallazgo nuevo | No detectado antes | **Confirmado N1** → G-7 |
| GUI: indicadores de proyectos Word | Hallazgo nuevo | No detectado antes | **Confirmado N1** → G-8 |
| Orden de integración | «integración controlada» (sin orden explícito en el resumen) | WP-63 primero | Se mantiene WP-63 primero (§7). |
| CI E4, `linea_base` y decisiones D1–D9 | Sin datos en el resumen | Evaluados | Rige esta revisión. |

## 3. Verificación (N1)

Entorno común: Linux, CPython 3.12.3, lock `linux-py312`, `xvfb-run` y `python3.12-tk`. Sin `tkinter` la recolección
falla por el entorno, no por el código. WP-63 se midió con `PYTHONPATH` apuntando a su propio `src`.

| Control | WP-80 | WP-63 |
| --- | --- | --- |
| Suite Asistente | PASA · 603 (base 592) | PASA · 780 (base 592) |
| Suite Descargador | PASA · 236 (base 231) | PASA · 235 (base 231) |
| Inventario de nodeids | no existe en su verificador | PASA · sin bajas |
| Identidades / golden / restricciones | PASA | PASA |
| Ruff | PASA (E9/F63/F7/F82) | PASA (+F401/F841) |
| Módulos no alcanzables / enlaces | 0 / 0 | 0 / 0 |
| **Veredicto** | **PASA** | **PASA** |

Datos adicionales:

- La punta de `feat` da 592/231 (N1).
- Las 17 pruebas nuevas de WP-80 fallan sin sus correcciones (N1).
- La comprobación equivalente para las pruebas de WP-63 la declara su Ejecutor, que conserva la evidencia de los
  fallos iniciales. Aquí no se repitió.
- Falta la verificación del código combinado: **no se ha hecho y es requisito de integración**.

## 4. Hallazgos consolidados

### 4.1 Corregidos en ambas ramas (elegir una versión al fusionar)

| Tema | Versión a conservar | Motivo |
| --- | --- | --- |
| Consolidado generado dos veces | WP-63 (`export_capture_with_note`) | Una sola API y menos superficie. |
| SITFA bloqueada si falla el disco al cerrar | `finally` de WP-63 + `EN_CURSO→INCOMPLETA` de WP-80 | Se complementan. |
| Apóstrofo en las columnas revisadas | WP-80 | Cubre ID, reglas, decisiones y traza; WP-63 solo los campos revisados. |
| Excepciones crudas en `check_workbook` | WP-63 | Agrega `zlib.error`, `testzip()`, cifrado, duplicados y enlaces. |
| E2: bajas de pruebas sin lista | WP-63 | Superconjunto: inventario exacto de nodeids. |
| `check_real_ids` | Unir: decodificación estricta y OOXML anidado de WP-63 + lectura por contenido de sufijos desconocidos de WP-80 | Se complementan. |

### 4.2 Solo en WP-63

- **Alta:**
  - anonimizador: filtraba nombres en APELLIDO/DEMANDANTE/DEMANDADO/SOLICITANTE/INTERVINIENTE; un RIT podía coincidir
    con su pseudónimo; un `=` se convertía en fórmula;
  - consolidado incoherente con el CSV del motor;
  - retención que borra un Conjunto en uso;
  - reimportar un ZIP movido conservaba la ruta anterior;
  - procedencia del verificador.
- **Media:**
  - tope de lecturas superable con una caché corrupta;
  - destinatario inválido rompía Resultados;
  - firma de «Siguiente paso» desactualizada;
  - «Importar último» bloqueaba Tk;
  - `#texto` se perdía como error de Excel;
  - hash calculado después de publicar;
  - «Datos guardados» no toleraba datos parciales.
- **E4 en CI:** la base corría Playwright, PyInstaller y el instalador **solo en PR**, al revés de E4. WP-63 lo
  corrige y mantiene CI en el PR hacia `main`.
- **Documental:** D1–D9 sincronizadas en `PLAN_VIGENTE.md`. Coinciden literalmente con `estado/global.json` (N1).
  `linea_base.json` queda posterior a la purga.

### 4.3 Solo en WP-80 (N1: siguen presentes en la rama de WP-63)

| Id | Severidad | Defecto | Ubicación en WP-63 |
| --- | --- | --- | --- |
| D1 | alta | Un Conjunto terminado no se puede cargar como fuente | `archivo_local.py:87` |
| A2 | alta | Reimportar sin ediciones exige conciliación | `capture_reconciliation.py:51` |
| A3 | alta | Los respaldos desvían `storage_directory` y los recibos | `app_base.py:664`, `download_link.py:48/74`, `download_transfer.py:112` |
| G1 | alta | Un evento defectuoso detiene la cola del Descargador | `descargador.py:1276` |
| A6 | media | Una clave nueva en `textos_base.json` inutiliza la configuración anterior | `config.py` (falta `complete_texts`) |
| A4 | media | Aviso falso «Sin cruce utilizable» en CUMPLIMIENTO | `work.py:269` |
| C1–C4 | media/baja | `'²'` revienta `int()`; período con zonas; `recoverable`/`sha256` sin validar | `contract.py:341,389` |
| T3 | media | Sesiones `.bin`/`.ps1` no se revisan por identidades | `check_real_ids.texts` |
| D4–D7, C6, A5, B1 | baja | Egresados, `TASK_EXPIRED`, checkpoint 25, `import_lote`, avisos | varios |

### 4.4 Abiertos en ambas ramas (nadie los corrige)

| # | Sev. | Ubicación | Defecto | Evidencia | Arreglo propuesto |
| --- | --- | --- | --- | --- | --- |
| G-1 | media | `record_view.py:140,146` | «Datos de origen» y «Datos Word» no muestran el valor original ni el automático | N1: CustomTkinter 5.2.2 no activa `placeholder_text` con `textvariable` (reproducido con `xvfb`) | Etiqueta «Origen: …» junto al campo |
| G-7 | media | `work_tools.py:14-24` | En un paquete con varias modalidades, `quality()` valida todas las filas con `REQUIRED[work.mode]` (modo de la primera hoja) | N1: paquete ficticio de 3 ESPERA + 1 CUMPLIMIENTO → la fila CUMPLIMIENTO marca «`espera` · Campo obligatorio vacío» (falso positivo), y sus `dias_cumpl`/`dias_egresar` nunca se validan | Usar `work.capture_modes.get(row.id, work.mode)`, como ya hace `record_edits.py:189` |
| G-8 | media | `ux_support.py:59-66` | «Word ✓» se muestra con cualquier recibo Word que contenga el registro: no compara tipo, existencia del archivo ni vigencia | N1: fila con `PC_IE` y un recibo `PC_INFO` cuyo archivo no existe → `['RES PC_IE', 'Word ✓']`. `current_tools.pending()` sí exige tipo y archivo: las dos vistas se contradicen | Un ayudante común con el criterio de `pending()` (tipo + `is_file` + `product_hash`), una marca por código |
| H-1 | media | `check_real_ids.history_blobs` | `--historia` filtra por sufijo: una sesión `.bin` del historial no se revisa (E5/WP-17) | N1 (código en ambas ramas) | Aplicar en el historial el criterio por contenido |
| A2r | baja | `capture_reconciliation.py:61`, `work.py:289` | `excluded`, que asigna la regla `COMUN.NO_SEGUIMIENTO`, cuenta como edición | N2 | Comparar con el valor que asigna la regla |
| G-2 | baja | `app_base.py:538` | `_download_joint` muerto (solo lo usa una prueba) | N1 (`git grep`) | Retirarlo y listar la prueba retirada |
| G-3 | baja | `interaction.py:22` | `lock()` no cubre las ventanas `Toplevel` | N1 (código); sin camino con pérdida demostrado | `app._guard` en esos comandos |
| G-4 | baja | `current_tools.py:27` / `ux_support.py:77` | Dos criterios para «borrador guardado» | N2 | Un ayudante común |
| G-5 | baja | `current_tools.py:27` | `pending()` calcula el SHA-256 de los adjuntos en cada llamada. «Siguiente paso» ya lo evita con `stat` en WP-63 | N1 (código) | Memoria por `(ruta, mtime, tamaño)` |
| G-6 | baja | `current_tools.py:110` | Falta «Cancelar» en el resumen previo a Outlook | N1 (código) | Agregar el botón |

### 4.5 Defectos que introduce el propio WP-80

1. **C7 · `file_output` con `os.link`: productos en `0600` en POSIX** (N1; antes `0644`).
   - En Windows no afecta.
   - Arreglo: `chmod` antes del enlace, o retirar C7.
2. **E4 · quita la CI del PR `feat → main`** (`tests.yml:10-12`).
   - Queda resuelto si se conserva el `tests.yml` de WP-63 al fusionar.
3. **La prueba de A3 no recorre la ruta real** (`_save_session → archive_current`).

### 4.6 Discrepancias de criterio entre las ramas

1. **Mayúsculas en `[Content_Types].xml`.**
   - WP-80 acepta el nombre sin distinguir mayúsculas.
   - WP-63 exige el canónico y rechaza duplicados.
   - Se adopta **WP-63** (N2: Excel siempre escribe el canónico). Al fusionar se retira la prueba de minúsculas de
     WP-80 y se lista (regla 12).
2. **CI.**
   - Se adopta el esquema de **WP-63**.
   - De WP-80 solo se conserva la caché de pip y el retiro de los pasos duplicados, siempre que el job integral siga
     corriendo al integrar.

## 5. Veredictos

| WP | Revisión de código | Alineación | Comentario |
| --- | --- | --- | --- |
| **WP-63** | **APROBADO CON OBSERVACIONES** (revisión completa; ver §5.1) | **CONFORME** (con las decisiones del §8) | Sin bloqueos. Observaciones no bloqueantes en §5.1. |
| **WP-80** | **APROBADO** (revisión final sobre `95991b6`, tras fusionar `feat` con WP-63; ver [WP-80.md](WP-80.md)) | **CONFORME**, sujeto a: (1) regenerar la línea base después de WP-63 y volver a verificar (E2); (2) etiqueta `pre-WP-80`; (3) squash con el formato de la regla 11; CI verde | Conflictos resueltos conforme al §7.4. Verificación PASA 796/240, sin bajas frente a la punta de `feat`. |

Observaciones de alineación de WP-63:

- Edita el plan fuera de su propio archivo de estado: `PLAN_IMPLEMENTACION.md`, `PLAN_VIGENTE.md`, `global.json` y
  `WP-70.json`.
- Convierte las dependencias de WP-70 en funcionales: confirmado por el usuario (§8.2).
- Regla 11 parcial: 9/20 commits con prefijo, 10/20 con «Verificación».
- Commits de Codex firmados con el nombre del usuario.

Ambas cumplen las restricciones absolutas 1–7:

- Sin `.Send(`; CC fijo.
- Sin escritura en RUS ni SATURNO.
- Sin datos reales.
- `motor/` y `textos_base.json` intactos; golden PASA.

### 5.1 Cierre de la E3 de WP-63 (lectura íntegra de las herramientas)

Leídos completos `tools/verificar_wp.py` (583 líneas), `tools/check_real_ids.py` (286) y
`tests/test_audit_tools.py` (322).

- **Lo comprobado está bien.** El diseño falla cerrado:
  - un control no medible da FALLA;
  - las bajas de pruebas se aceptan solo con nodeids del inventario exacto;
  - la procedencia exige SHA, árbol limpio e igualdad antes y después de medir;
  - la línea base no se sobrescribe si la medición falla;
  - los diagnósticos no copian contenido.
- **Restricciones.** El control de restricciones de WP-63 es más estricto: prohíbe en `src/` los imports de red,
  procesos y automatización, además de `.Send`. N1: **WP-80 también lo pasa** (`restrictions() == []`), y pasa el
  ruff ampliado (F401/F841 en `src`, `descargador` y `tools`).
- **Pruebas.** Las de `test_audit_tools.py` ejercitan comportamiento real, con RUT calculado en tiempo de ejecución.
- **Observaciones no bloqueantes:**

| # | Sev. | Ubicación (rama WP-63) | Observación | Evidencia | Arreglo |
| --- | --- | --- | --- | --- | --- |
| O1 | baja | `tools/check_real_ids.py:280` | Un DOCX/XLSX con deflate corrupto termina con traceback `zlib.error` y **código 1** («hallazgos»), no con el 2 que promete su docstring («revisión incompleta») | N1: reproducido con un DOCX sintético. Sigue fallando cerrado: `verificar_wp.count_real_ids` lo trata como no medible → FALLA | Agregar `zlib.error` (y `lzma.LZMAError`) al `except` de `main` y una prueba |
| O2 | media | `check_real_ids.texts` / `history_blobs` | Omite sufijos desconocidos (`.bin`, `.ps1`, `.iss`) en el árbol y en `--historia` | N1 (código) | Unir con T3 de WP-80 al fusionar (§7.4) y extenderlo a `--historia` (H-1) |
| O3 | nit | `tools/verificar_wp.py:24-25` | Control «9.» duplicado en el docstring | N1 | Dejar una línea |
| O4 | nit | `tests/test_audit_tools.py:185` | `eval` de condiciones del workflow (sin builtins) | N1. Es contenido versionado, por lo que el riesgo es bajo, pero la prueba es frágil | Comparar el texto de la condición |
| O5 | baja | `lectura_bitacoras.py:509,525` | Dos `persistir(lazy)` por registro (D7) y `EN_CURSO` tras un error de disco | N1 | Lo trae WP-80 al fusionar |
| O6 | baja | `next_step._signature` | Serializa todos los recibos y hace `stat` de los archivos en cada pulso | N2 | Medir con `benchmark_interface.py --rows 10000` |

## 6. Proceso

Se ejecutaron dos WP de auditoría simultáneos sobre la misma base, sin coordinación del Orquestador. Eso duplicó
trabajo y dejó 8 conflictos. Además, el plan de WP-63 cuenta «35 WP» sin WP-80.

Regla adoptada por el usuario (§8.5): **un solo WP correctivo transversal activo a la vez**.

## 7. Plan para continuar (en este orden)

1. ~~Completar la E3 de WP-63~~ **Hecho** (§5.1): APROBADO CON OBSERVACIONES / CONFORME. O1 y O3 son opcionales antes
   de integrar.
2. **Decisiones del usuario** (§8).
3. **Integrar WP-63** en `feat`. Regenerar `ESTADO.json` (`estado.py agregar`) y la línea base (E2).
4. **Actualizar WP-80** fusionando `feat` (sin rebase ni `--force`) y resolver así los conflictos:
   - `export_csmp_capture`, `descargador.py`, `ventana_resultados.py`: API de WP-63; adaptar la prueba G1 de WP-80.
   - `lectura_bitacoras`: límite en `_abrir_bitacora` (WP-63) + `INCOMPLETA` y un solo `persistir(lazy)` (WP-80).
   - `contract`: `check_workbook` de WP-63 + `DIGITS`, `_instant_after`, `recoverable` y `sha256` de WP-80; retirar y
     listar la prueba de minúsculas.
   - `check_real_ids`: unir los dos enfoques.
   - `verificar_wp`: versión de WP-63; retirar T2 de WP-80 y listar su prueba.
   - CI: versión de WP-63 + caché de pip.
5. **Corregir en WP-80, antes de integrarlo:**
   - C7 (`chmod`);
   - prueba de A3 por la ruta real;
   - A2r;
   - etiqueta `pre-WP-80`;
   - mensaje de integración con el formato de la regla 11.
6. **Verificar el código combinado** con `verificar_wp.py` de WP-63, en Linux y en la CI de Windows. Nueva revisión E3
   del diff de fusión. Integrar WP-80.
7. **Nuevo WP de GUI** (después de 6), cada hallazgo con una prueba que falle antes del arreglo:
   - prioridad media: G-1, G-7, G-8 y H-1;
   - prioridad baja: G-2 a G-6 y A2r.

## 8. Decisiones del usuario (respondidas el 11 de octubre de 2026)

Respuestas literales del usuario en esta sesión. El Orquestador debe registrarlas en `estado/global.json` →
`decisiones`.

| # | Pregunta | Respuesta |
| --- | --- | --- |
| 1 | Orden de integración | «WP-63 → WP-80 (Recomendado)» |
| 2 | Dependencias de WP-70 como funcionales (WP-02, WP-17, WP-50, WP-63) | «Confirmar (Recomendado)» |
| 3 | Regla 11 | «Exigir en integración (Recomendado)»: se acepta el historial; el commit de integración lleva «WP-XX: …» y «Verificación: PASA», sin `--force` |
| 4 | Commits de Codex firmados con el nombre del usuario | «Aceptar»: la ficha WP y el estado bastan para identificar al Ejecutor |
| 5 | Un solo WP correctivo transversal a la vez | «Sí, uno a la vez (Recomendado)» |

Efecto en los veredictos:

- **WP-63** queda **CONFORME**: los cambios al plan quedan confirmados, la firma es aceptada y la regla 11 se exige en
  la integración.
- **WP-80** sigue **NO CONFORME (formal)** hasta cumplir el §7.5. E4 queda subsanado si se adopta la CI de WP-63 al
  fusionar; la regla 11 se cumple en el commit de integración.
