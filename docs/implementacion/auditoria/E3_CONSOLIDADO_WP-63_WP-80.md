# E3 consolidado · WP-63 (PR #34) y WP-80 (PR #35)

Revisor y Auditor: sesión `session_016DC9qxvuqT6mr91tHBaw7b` (Claude). Es una raíz distinta de los dos Ejecutores:
WP-80 (Claude, `session_012yjVgL…`) y WP-63 (Codex `/root`) · 11 de octubre de 2026 · pedido expreso del usuario.

Este informe reemplaza, para decidir la integración, a [WP-80.md](WP-80.md). Ese informe sigue vigente como detalle
de WP-80.

Convención: **N1** hecho comprobado aquí; **N2** inferencia por lectura de código, con el supuesto explícito.

## 1. Qué hay en el repositorio (N1)

| Rama / PR | Ejecutor | Base | Commits | Archivos | CI del PR |
| --- | --- | --- | --- | --- | --- |
| `wp/auditoria-refactor-20261011` · #34 · WP-63 | Codex (commits firmados «el usuario titular») | `2cf8ca8` | 20 | 56 (+4.068/−542) | verde (2/2) |
| `wp/80-auditoria` · #35 · WP-80 | Claude | `2cf8ca8` | 18 | 61 (+625/−307) | verde (2/2) |

Son **dos auditorías correctivas paralelas sobre la misma base y con el mismo mandato del usuario** («auditar,
corregir, refactorizar»).

- Comparten 17 archivos.
- Una fusión de prueba (WP-63 + WP-80) da **8 conflictos de contenido**:
  - `export_csmp_capture.py`, `lectura_bitacoras.py`, `test_paquete_lote_actual.py`;
  - `capture/contract.py`, `test_capture_input.py`, `test_herramientas_plan.py`;
  - `check_real_ids.py`, `verificar_wp.py`.
- No existe otra revisión E3 de WP-80: PR #35 no tiene reseñas ni otros comentarios.
- La «otra revisión» es el `REPORTE_REVISION_20261011.md` de WP-63. Su autor declara que sus subagentes «no declaran
  APROBADO/CONFORME formal»: su E3 también está pendiente.

## 2. Verificación (N1)

Mismo entorno para ambas: Linux, CPython 3.12.3, lock `linux-py312`, `xvfb-run` y `python3.12-tk` (sin `tkinter`,
la recolección falla por el entorno). WP-63 se midió con `PYTHONPATH` de su propio `src`.

| Control | WP-80 (`verificar_wp` de WP-80) | WP-63 (`verificar_wp` de WP-63) |
| --- | --- | --- |
| Suite Asistente | PASA · 603 (base 592) | PASA · 780 (base 592) |
| Suite Descargador | PASA · 236 (base 231) | PASA · 235 (base 231) |
| Inventario de pruebas (nodeids) | no existe ese control | PASA · sin bajas |
| Identidades / golden / restricciones | PASA / PASA / PASA | PASA / PASA / PASA |
| Ruff | PASA (E9/F63/F7/F82) | PASA (incluye F401/F841) |
| Módulos no alcanzables / enlaces | 0 / 0 | 0 / 0 |
| **Veredicto** | **PASA** | **PASA** |

Notas:

- La punta de `feat` (`2cf8ca8`) da 592/231: las bases declaradas son correctas.
- WP-63 regenera `linea_base.json` con un SHA posterior a la purga e inventario exacto. Así corrige la observación
  de mi informe anterior (SHA `79610fd` inexistente).
- Las 17 pruebas nuevas de WP-80 fallan sobre la base (comprobado). Las de WP-63 no se recorrieron una por una; el
  Ejecutor declara haberlas comprobado y conserva la evidencia de sus fallos iniciales.

## 3. Comparación de hallazgos

### 3.1 Coinciden (ambos lo encontraron y corrigieron)

| Tema | WP-80 | WP-63 | Recomendación al fusionar |
| --- | --- | --- | --- |
| Consolidado generado dos veces | `consolidated_note` devuelve bytes | `export_capture_with_note` | Conservar **WP-63**: una sola API y menos superficie. |
| Bloqueo de SITFA al fallar el disco al cerrar | `finally` + `EN_CURSO→INCOMPLETA` | `finally` | Unir: el `finally` de cualquiera + `INCOMPLETA` de WP-80. |
| Apóstrofo en OBSERVACION revisada | `_write_text` en todas las columnas y en la traza | `_write_text` solo en campos revisados | Conservar **WP-80** (cubre ID, reglas, decisiones y traza). |
| `zlib`/compresión en `check_workbook` | captura `KeyError`/`RuntimeError`/`NotImplementedError`/`OSError`; **no** `zlib.error` | captura `zlib.error`, `testzip()`, cifrado, duplicados y enlaces | Conservar **WP-63**; resuelve mi ítem 4 de WP-80. |
| E2: bajas numéricas sin lista | FALLA si hay número sin lista | FALLA + inventario exacto de nodeids | Conservar **WP-63** (superconjunto). |
| `check_real_ids` | lee por contenido los sufijos desconocidos (`.bin`, `.ps1`) | decodificación estricta UTF-8/cp1252/UTF-16 + OOXML anidado + RUT partido entre runs | **Unir**. Ninguno revisa por contenido en `--historia` (pendiente común). |

### 3.2 Solo WP-63 (WP-80 no los corrige; siguen en WP-80)

Severidad según WP-63; las regresiones están en su rama.

- **Alta, privacidad:**
  - el anonimizador filtraba nombres bajo APELLIDO/DEMANDANTE/DEMANDADO/SOLICITANTE/INTERVINIENTE;
  - un RIT podía coincidir con su pseudónimo;
  - `=` se convertía en fórmula al anonimizar.
- **Alta, datos:**
  - un consolidado incoherente con el CSV del motor (p. ej. T ESPERA) podía usarse para la copia nativa: comparación
    celda a celda en `_map_consolidated`;
  - la retención podía borrar un Conjunto con una descarga anidada en uso;
  - reimportar el mismo ZIP desde otra carpeta conservaba la ruta anterior.
- **Alta, verificación:**
  - procedencia (SHA completo + árbol limpio) en el informe;
  - «integrable» solo si todos los controles pasan;
  - una medición fallida no reemplaza la línea base.
- **Media:**
  - el tope de lecturas se podía superar con una caché corrupta (el control se movió a `_abrir_bitacora`);
  - destinatario inválido rompía Resultados;
  - firma de «Siguiente paso» obsoleta;
  - «Importar último» bloqueaba Tk;
  - `#texto` se perdía como error de Excel;
  - el hash se calculaba después de publicar en `exports`;
  - el inventario de «Datos guardados» no toleraba datos parciales.
- **E4 en CI (medio):** Playwright, PyInstaller e instalador pasan a correr al integrar o a pedido (`!= pull_request`).
  La base los corría **solo en PR**, al revés de E4; WP-80 no lo corrigió. WP-63 **mantiene la CI mínima en el PR
  `feat → main`**, que WP-80 había quitado (mi ítem obligatorio 1).
- **Documental:**
  - sincroniza en `PLAN_VIGENTE.md` las decisiones D1–D9. N1: coinciden literalmente con `estado/global.json` de
    `feat`, mientras que `PLAN_VIGENTE.md` en `feat` las mostraba como «pendiente»;
  - mueve el `CONTINUAR.md` anterior a `HISTORIAL.md`.

### 3.3 Solo WP-80 (N1: siguen presentes en la rama de WP-63, comprobado con `grep`)

| Id | Defecto | Ubicación en WP-63 |
| --- | --- | --- |
| D1 (alto) | un Conjunto terminado (`PREPARADO_SIN_CONSULTA_FIRMAS`) no se puede cargar como fuente | `archivo_local.py:87` |
| A2 (alto) | reimportar sin ediciones exige conciliación (decisiones por defecto cuentan como edición) | `capture_reconciliation.py:51` |
| A3 (alto) | guardar un respaldo desvía `storage_directory` y los recibos de borradores | `app_base.py:664`, `download_link.py:48/74`, `download_transfer.py:112` |
| G1 (alto) | un evento defectuoso detiene para siempre la cola del Descargador | `descargador.py:1276` |
| A6 (medio) | una clave nueva en `textos_base.json` inutiliza la configuración anterior | sin `complete_texts` |
| A4 (medio) | aviso falso «Sin cruce utilizable» en paquetes CUMPLIMIENTO | `work.py:269` |
| C1–C4 (medio/bajo) | `'²'` en páginas/fila revienta `int()` fuera de `CaptureError`; período con zonas; `recoverable` y `sha256` sin validar | `contract.py:341,389` |
| C6 (bajo) | `import_lote` sin control de claves duplicadas ni de tipos | `capture/diaries.py` |
| D2b (bajo) | lote `EN_CURSO` tras un error de disco al cerrar | `lectura_bitacoras._cerrar` |
| D4/D5 (bajo) | pestaña Egresados tipada `OTRO` / `NO_VERIFICADA` | `export_csmp_capture.py:151`, `lectura_bitacoras.py:165` |
| D6 (bajo) | respuesta tardía de la extensión corta la conexión | `puente.py:165` |
| D7 (bajo) | checkpoint cada ~12 lecturas, no cada 25 | `lectura_bitacoras.py:509,525` |
| T3 (medio) | sesiones `.bin`/`.ps1` sin revisar por identidades | `check_real_ids.texts` |
| C7, A5, B1, retiro de código muerto | — | — |

### 3.4 Discrepancias de criterio

1. **Mayúsculas en `[Content_Types].xml`.**
   - WP-80 lo acepta sin distinguir mayúsculas.
   - WP-63 exige el nombre canónico y rechaza duplicados por mayúsculas.
   - N2: Excel siempre escribe el canónico. Basta el criterio de WP-63, que además cierra la ambigüedad de
     duplicados. Retirar la prueba `test_lowercase_content_types_is_read_not_keyerror` de WP-80 al fusionar y listarla
     (regla 12).
2. **Publicación atómica.**
   - WP-80 cambió `file_output` a `os.link`, que deja `0600` en POSIX (medido).
   - WP-63 dejó `file_output` y corrigió `exports` con hash previo y destino exclusivo.
   - Recomendación: conservar `os.link` solo con `chmod` previo, o retirar C7. Sin el arreglo, C7 es una regresión
     en Linux; en Windows no afecta.
3. **CI.**
   - WP-80 filtra `pull_request` a `feat/**` y retira pasos duplicados.
   - WP-63 aplica E4 por evento.
   - Conservar **WP-63**. De WP-80 solo vale la caché de pip y la eliminación del `pip check` y de la ejecución de
     `pytest descargador` duplicados, siempre que el job integral siga corriendo al integrar.

## 4. Hallazgos que ninguna de las dos corrige (repositorio)

1. **Medio.** `record_view.py:140,146`: «Datos de origen» y «Datos Word» nunca muestran el valor original ni el
   automático. CustomTkinter no activa `placeholder_text` con `textvariable` (N1, reproducido con `xvfb-run`).
2. **Medio.** `check_real_ids --historia` filtra por sufijo en ambas ramas: una sesión `.bin` del historial no se
   revisa. Afecta a WP-17/E5.
3. **Bajo.** A2 residual de WP-80: `excluded`, que la regla `COMUN.NO_SEGUIMIENTO` asigna sola, cuenta como edición
   (`capture_reconciliation.py:61`, `work.py:289`; N2).
4. **Bajo.** Hallazgos GUI de [WP-80.md](WP-80.md):
   - G-2: `_download_joint` muerto;
   - G-3: `lock()` no cubre las ventanas `Toplevel`;
   - G-4: dos criterios para «borrador guardado»;
   - G-6: falta «Cancelar» en el resumen previo a Outlook.
   - G-5 queda mitigado en «Siguiente paso» por WP-63 (usa `stat`), pero `current_tools.pending()` sigue calculando
     el hash de los adjuntos.
5. **Proceso.** Dos WP de auditoría simultáneos sobre la misma base, sin coordinación del Orquestador, duplicaron
   trabajo y dejan 8 conflictos. El plan de WP-63 cuenta «35 WP» sin WP-80, y WP-80 no figura en
   `PLAN_IMPLEMENTACION.md`.

## 5. Revisión de código

- **WP-63: APROBADO CON OBSERVACIONES.** Revisé completos `src/`, CI y los cambios de `contract`; de
  `check_real_ids` y `verificar_wp`, las partes que se superponen con WP-80. Las 4.000 líneas de pruebas y
  herramientas nuevas se revisaron por muestreo (N2 para lo no leído). No encontré defectos bloqueantes.
  Observaciones:
  1. `check_real_ids` sigue sin revisar sufijos desconocidos ni el historial por contenido (incorporar T3 de WP-80).
  2. `lectura_bitacoras`: quedan dos `persistir(lazy=True)` por registro (D7) y el estado `EN_CURSO` tras un error
     de disco.
  3. `next_step._signature` serializa todos los recibos a JSON y hace `stat` de todos los archivos en cada pulso.
     N2: es aceptable con los volúmenes actuales, pero conviene medirlo con 10 000 filas (`benchmark_interface.py`).
- **WP-80: CAMBIOS, condicionado al orden de integración.**
  - El ítem obligatorio (CI hacia `main`) **desaparece si WP-63 se integra antes** y, al resolver el conflicto, se
    conserva su `tests.yml`.
  - Siguen abiertos:
    - permisos `0600` de C7;
    - A2 residual (`excluded`);
    - prueba de A3 por la ruta real.
  - Ya no se aplican, porque los cubre WP-63: `zlib` en `check_workbook` y la decodificación de `check_real_ids`.

## 6. Alineación

| Regla | WP-63 | WP-80 |
| --- | --- | --- |
| 1–7 restricciones absolutas | Cumple. Control 5 PASA; `motor/` y `textos_base.json` intactos; golden PASA; el anonimizador escribe texto explícito. | Cumple (ver WP-80.md) |
| Decisión «Quitar apóstrofo» | Cumple, parcial: solo campos revisados | Cumple, completa |
| D1–D9 | Las sincroniza en `PLAN_VIGENTE.md` con el texto literal (N1) | Sin efecto |
| 8–9 rama propia, sin `--force` | Cumple | Cumple |
| 10 `pre-<ID>` | No aplica: no elimina archivos de código | **No cumple**: falta `pre-WP-80` |
| 11 mensaje `<ID>:` + «Verificación:» | **Parcial**: 9/20 con prefijo, 10/20 con «Verificación» | **No cumple**: 2/18 con prefijo, 0/18 con «Verificación» |
| 13 alcance | Amplio, autorizado («auditar… actualizar el repositorio y el plan») | Amplio, autorizado |
| E1 estado por WP | **Observación**: además de `estado/WP-63.json`, edita `global.json`, `estado/WP-70.json`, `PLAN_IMPLEMENTACION.md` y `PLAN_VIGENTE.md`. Cambia dependencias de WP-70 a «funcional»: decisión de plan que **debe confirmar el usuario**. | Cumple |
| E2 sin holgura | Cumple: regenera la línea base e inventario | Cumple; línea base de `feat` desfasada |
| E3 revisión cruzada | Pendiente → este informe | Este informe |
| E4 CI mínima por PR | **Cumple** (corrige la inversión de la base) | **No cumple** (quita CI al PR hacia `main`) |
| Trazabilidad | **Observación**: los commits de Codex llevan el nombre del usuario; la raíz solo consta en la ficha. Pedir `Co-authored-by`/«Ejecutor:» en el cuerpo. | Cumple |

Veredictos:

- **WP-63: CONFORME CON OBSERVACIONES.** Falta la confirmación del usuario a los cambios de plan (dependencias de
  WP-70) y a la regla 11 parcial.
- **WP-80: NO CONFORME (formal).** E4, reglas 10 y 11. E4 se subsana con el orden de integración del punto 7.

## 7. Recomendación de integración (Orquestador / usuario)

1. **Integrar WP-63 primero.** Trae la base más amplia: privacidad, línea base, verificador, E4 y plan.
2. **Actualizar WP-80 sobre esa `feat`** con una fusión (sin rebase ni `--force`) y resolver los 8 conflictos así:
   - `export_csmp_capture`, `descargador.py`, `ventana_resultados.py`: API de WP-63 (`export_capture_with_note`);
     descartar `consolidated_note` de WP-80 y adaptar su prueba G1.
   - `lectura_bitacoras`: límite en `_abrir_bitacora` (WP-63), más `INCOMPLETA` y un solo `persistir(lazy)` (WP-80).
   - `contract`: `check_workbook` de WP-63, más `DIGITS`, `_instant_after`, `recoverable` y `sha256` de WP-80.
     Retirar y listar la prueba de minúsculas.
   - `check_real_ids`: decodificación y OOXML de WP-63, más lectura por contenido de sufijos desconocidos de WP-80.
   - `verificar_wp`: versión de WP-63; descartar T2 de WP-80 (superado) y su prueba, listada como eliminada.
   - CI: `tests.yml` y `descargador-integral.yml` de WP-63; de WP-80 solo la caché de pip.
   - Conservar `estado/WP-*.json` de ambas ramas y regenerar con `python tools/estado.py agregar`.
3. **Antes de integrar WP-80:**
   - `chmod` antes de `os.link` (o retirar C7);
   - A2 residual;
   - etiqueta `pre-WP-80`;
   - mensaje de integración con el formato de la regla 11.
4. Volver a correr `verificar_wp.py` (versión de WP-63) sobre WP-80 actualizado, contra la línea base posterior a
   WP-63 (E2).
5. Abrir un WP para G-1 y para `--historia` por contenido.

## 8. Decisiones que solo puede tomar el usuario

1. Aceptar el orden **WP-63 → WP-80** (o el inverso, que obliga a rehacer la resolución de CI y la línea base).
2. Confirmar el cambio de dependencias de WP-70 a «funcional» (WP-02, WP-17, WP-50, WP-63) que introduce WP-63.
3. Regla 11: aceptar el historial tal cual o exigir mensajes de integración con el formato.
4. Firma de commits de Codex con el nombre del usuario: aceptarla o exigir marca de ejecutor.
