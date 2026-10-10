# Reporte de revisión del plan y de lo avanzado

10 de octubre de 2026 · claude-sesion-012yjVgL · rama `plan/revision-20261010`, desde `feat` `e80444e`.

**Convención:** N1 = hecho verificado (archivo, commit o salida); N2 = inferencia con su supuesto; N3 = especulación.

## 1. Resumen

**N1. Integración.** Desde `e80444e` no se integró nada en `feat`.
- Hay 15 PR en borrador de esta sesión: la pila #12–#25 y #26, que es independiente.
- Codex tiene 3 PR: #4 (WP-14, aprobado y conforme), #8 (WP-15) y #11 (WP-60).

**N1. Revisión independiente.** Cinco revisores aislados (solo lectura) revisaron todo lo avanzado y la formulación del
plan.
- Encontraron 1 error grave de privacidad, 1 de interfaz grave y alrededor de 20 medios o bajos.
- Todos los errores de código confirmados quedaron **corregidos y empujados**, con prueba que falla sin la corrección
  (sección 3).
- La punta de la pila (`wp/WP-42`, `3e2d571`) da PASA en `verificar_wp.py` con 148 pruebas retiradas declaradas:
  512/11 en el Asistente y 198/59 en el Descargador.

**N1. CI de GitHub.** Desde las 03:24 UTC Actions no asigna runners: los jobs terminan en 1–2 s con `runner_id: 0`.
- N2: es el límite de minutos o de gasto de Actions (Windows cuenta ×2). No se puede verificar desde aquí.
- **Acción del usuario:** revisar *Settings › Billing › Actions* y volver a lanzar los jobs.

**N1. El problema de fondo es el proceso, no el código.**
- La pila de 14 PR existe porque todos editan `docs/implementacion/ESTADO.json`.
- Esa pila multiplica los minutos de CI (cada corrección se propaga a 10–13 PR) y contradice la regla 8 («rama desde
  la punta de `feat`»).
- Esta rama implementa la salida: estado por WP y un verificador que no deja holgura (sección 5).

## 2. Estado actual

| Bloque | WP | PR | Verificación local | CI |
| --- | --- | --- | --- | --- |
| Retiros | 20, 21, 22, 23 | #12–#15 | PASA | verde antes del corte |
| Correcciones | 30, 31, 32, 35, 24, 34 | #16–#21 | PASA | verde antes del corte |
| Datos sensibles | 16 | #22 | PASA (anonimizador reescrito) | sin runners |
| Interfaz | 40, 41, 42 | #23–#25 | PASA | sin runners |
| Descargador | 33 | #26 (contra `feat`) | PASA (625/11, 203/59) | sin runners |
| Descargador | 43 | #28 (contra `feat`, E6) | PASA (625/11, 200/59) | sin runners |
| Descargador | 62 | #29 (sobre #26, dependencia funcional) | PASA; Radon 77 → 12 | sin runners |
| Interfaz | 44 | #30 (sobre #25, dependencia de archivo) | PASA (514/11) | sin runners |
| Interfaz | 45 | #31 (sobre #30, dependencia de archivo; D9: retirar) | PASA (515/11) | sin runners |
| Descargador | 51 | #32 (contra `feat`, sobre #28; D6: sí) | PASA (637/11, 204/59); salida nativa con Excel sin probar | sin runners |
| Plan | — | #27 (este reporte, contra `feat`) | PASA (629/11) | sin runners |
| Codex | 14, 15, 60 | #4, #8, #11 | sus fichas | — |

**Bloqueados por decisión del usuario:**

| WP | Decisión |
| --- | --- |
| WP-02 | D2 |
| WP-17 | D1 |
| WP-50 | D5 |
| WP-70 | D7 |

**Bloqueados por dependencias:**
- WP-61 va al final, con todo lo demás integrado.

Fuera de WP-61 y de los bloqueados por decisión, ya no quedan WP ejecutables para esta sesión: WP-43, WP-44 y WP-62
se ejecutaron después de esta revisión.

## 3. Hallazgos en lo avanzado y su corrección

Severidades: **A** = alta, **M** = media, **B** = baja. «Commit» es el commit de corrección en la rama del WP;
el anonimizador (WP-16) se corrigió en `7d53245`.

### 3.1 Datos sensibles: anonimizador (WP-16)

**A · N1.** Con datos ficticios se reprodujeron **10 filtraciones** y **2 cambios de reglas**. El control final no
detectaba ninguno.
- Encabezado bajo la fila 15.
- RUT sin guion, con espacios o de 6 dígitos; teléfonos; domicilios; RIT sin letra.
- Nombres sin tilde o en NFD.
- Curador sin RUT.
- Apellidos en columnas no reconocidas.
- Gráficos, tablas, autofiltros, formato condicional, nombres de hoja, encabezados de página y **vínculos externos**
  con la ruta `C:/Users/<persona>`.
- Teléfono dentro de una fórmula; fórmulas sin el valor en caché.

**Corrección (`7d53245`):** el módulo se reescribió para fallar cerrado.
1. La copia se construye **nueva y solo con valores** (`data_only`). Eso elimina de una vez todos los objetos del libro.
2. Todo pasa por NFC, y los nombres se buscan sin tildes ni mayúsculas.
3. Se ampliaron los patrones de RUT, RIT, teléfono, domicilio y números de 7–9 dígitos. Los RUT se reemplazan
   conservando el formato, para que `COMUN.CURADOR` dé lo mismo.
4. Control genérico: un nombre registrado, un RUT con dígito válido, un teléfono o un correo real **impiden crear la
   copia**.
5. El guardado es atómico: archivo temporal y luego renombre.
6. Pruebas: +13 casos (`tests/test_anonimizar_casos.py`). Los 15 escenarios del revisor ya no filtran.

**Límite conocido (N1):** un nombre suelto en una columna sin palabra clave, que no aparezca en otra parte, no se puede
reconocer. La advertencia final pide revisar la copia.

### 3.2 Interfaz (WP-40, WP-41, WP-42)

| Sev. | Hallazgo (N1, reproducido) | Corrección | Commit |
| --- | --- | --- | --- |
| A | Con más de 500 filas, `unlock` restauraba el estado viejo: «Guardar», «Vista previa» y «Preparar» quedaban deshabilitados con su acción disponible | `interaction.unlock` recalcula con `mail_block`; prueba con 700 filas | WP-42 `c83ea3f` |
| M | La barra «Siguiente paso» no veía ediciones que dejan un producto desactualizado | `refresh(force=True)` en `_update_context` | WP-41 `1d49dde` |
| M | Una excepción en `_tick` se repetía cada 1,5 s | Se informa una vez y el temporizador sigue | WP-41 `1d49dde` |
| B | Las preferencias de columnas antiguas imponían el orden viejo | El orden lo fija siempre `WORK_COLUMNS` | WP-40 `6bb22f4` |
| B | «Preferencias de vista» quedó inalcanzable con el editor plegado | Comando en la paleta Ctrl+K | WP-40 `6bb22f4` |
| B | Desborde a 1024×650 sin Excel (1072 px de 829) | Motivos cortos y `mail_note` en la fila del resumen (696/829 px); el editor sigue ≥120 px | WP-42 `3e2d571` |
| B | Menú «Más» sin `grab_release` ni tema | Corregido | WP-42 `c83ea3f` |

### 3.3 Retiros (WP-20 a WP-23)

- **M · N1.** `tools/flow_plan_progress.py` y `tools/flow_baseline_matrix.py` abortaban: la evidencia P0 nombra 7 rutas
  retiradas.
  - Corrección: `docs/evidencia_p0/rutas_retiradas.json`, que ambas omiten sin editar la evidencia. Commit `f44be75`.
- **B · N1.** `download_link.start` todavía podía lanzar el Descargador (`subprocess.Popen`) y contradecía la
  restricción 7.
  - Corrección: se retira; se conserva la recuperación. Esto contradice la frase «sin tocar» de la ficha WP-23
    ([CONFLICTO_ABIERTO] para el Revisor). Se retiran 5 pruebas y el acumulado sube a 148, con la lista en
    `verificacion/WP-23-eliminadas-revision.txt`.
- **B · N1.** El control de `registration.submit` pasaba siempre, porque el archivo ya no existe.
  - Corrección: control negativo. Una importación en `src/` de `subprocess`, `webbrowser`, `playwright`, `selenium`,
    `pywinauto`, `requests`, `httpx`, `urllib.request`, `http.client` o `socket` da FALLA.

### 3.4 Correcciones (WP-31, WP-32, WP-35, WP-24, WP-34)

| Sev. | Hallazgo (N1) | Corrección | Commit |
| --- | --- | --- | --- |
| M | Concurrencia: el push y el PR compartían grupo y se cancelaban entre sí | El grupo incluye el evento | WP-34 `207ae99` |
| M | `Iniciar.cmd` dejaba para siempre un `.venv` de una versión sin lock | Prueba 3.12→3.14 y borra el `.venv` (código 3) | WP-35 `0127a1e` |
| M | Instalación sin conexión con `paquetes\`: el lock trae herramientas de desarrollo | **Pendiente**: lock de ejecución separado (sección 4, paso 6) | — |
| B | `write_build_meta --versiones` ignoraba `--descargador DIR` | Corregido, y lo usa `build_integral.ps1` | WP-31 `89c1198` |
| B | `Work.export`: el hash de la copia quedaba fuera del `try` | Corregido; prueba que falla sin el cambio | WP-32 `6e57c9d` |
| B | El README de procedencia estaba modificado y el inventario no veía `historico/` | Corregido: 491 → 459 cambiados | WP-24 `1c0f4d8` |
| B | `MIGRACION_Y_MODULOS` decía «conservar» módulos ya retirados | Corregido | WP-24 `1c0f4d8` |

**N2. Para confirmar con el usuario:** en la etapa «propuesta», una fórmula que ya existía en OBSERVACION se copia
como texto. Supuesto: es lo deseado, para que la copia no ejecute fórmulas.

### 3.5 Formulación del plan (N1, citado de los documentos)

- **La pila contradice el propio plan.** La regla 8 pide rama desde `feat`. El ciclo de vida exige dependencias
  INTEGRADO, y se tomaron WP con dependencias EN_VERIFICACION. Las olas 2–4 partieron sin integrar las anteriores.
- **El verificador acumulaba holgura.** `--pruebas-eliminadas-asistente 143` se aplicaba a todos los WP posteriores
  (de WP-24 a WP-42), que podían perder entre 10 y 18 pruebas sin dar FALLA. Además, un control sin dato se omitía en
  vez de fallar.
- **`ESTADO.json` desactualizado:**
  - `pr: null` en WP-20 a WP-35 aunque sus PR existen.
  - WP-15: EN_VERIFICACION sin ejecutor. WP-60: TOMADO sin ejecutor.
  - WP-61 y WP-70 usan `TODAS_LAS_DEMAS`, que forma un ciclo e incluye WP bloqueados.
  - Conflictos sin dueño.
  - Los 6 conflictos R-T de WP-60 no están registrados.
  - Lo detecta `tools/estado.py validar`.
- **Etiquetas `pre-<ID>`.** Solo existe `pre-WP-20` y es local. N3: subir una etiqueta cuyo árbol contiene
  `variantes/` (con identidades reales) reexpondría esos datos.
- **Criterios no verificables hoy:** «CI Windows verde» en WP-23, 31, 34 y 35 depende de que haya runners.
- **Revisión independiente:** nadie cumple el rol de Revisor o Auditor para los PR de esta sesión. N2: si Orquestador
  y revisores de Codex comparten raíz, su independencia es dudosa.

## 4. Cómo abordar lo pendiente, paso a paso

1. **Usuario (5 min).** Revisar *Settings › Billing › Actions* (minutos o límite de gasto) y volver a lanzar los jobs
   de #22–#26.
   - Con la configuración nueva de WP-34, cada PR corre solo 3.12 y la matriz completa corre al integrar.
2. **Orquestador.** Decidir sobre las enmiendas E1–E6 (sección 5) e integrar esta rama. Después, correr una sola vez
   `python tools/estado.py dividir` y `python tools/estado.py agregar`.
3. **Revisión cruzada (E3).**
   - Codex revisa y audita los PR de esta sesión; esta sesión revisa #4, #8 y #11 de Codex.
   - Orden sugerido: #12 → #13 → #14 → #15 (retiros), luego #22 (privacidad), luego #4 (retención; los datos de NNA
     siguen sin plazo hasta integrarlo).
4. **Integrar la pila en orden, con squash.**
   - #12 primero: su rama arrastra commits legado; el diff neto es solo WP-20.
   - Tras cada integración, el Orquestador regenera `linea_base.json`: eso elimina la holgura acumulada.
   - Al rebasar cada PR siguiente, `ESTADO.json` entra en conflicto: se resuelve tomando la versión de `feat` y
     reaplicando solo la entrada de ese WP. Con E1 ese paso desaparece.
5. **Usuario.** Confirmar o resolver los conflictos abiertos:

   | Conflicto | Decisión pendiente |
   | --- | --- |
   | `cc_obligatorio` frente a la paridad golden | Regenerar golden en WP-12 o conservar el campo |
   | 4 F401 en `motor/` | Autorizar la limpieza o mantener la exclusión |
   | `download_link.start` | Retiro hecho; confirmar |
   | Fórmulas de OBSERVACION como texto | Confirmar |
   | Los 6 R-T de WP-60 | Pendiente de decisión |

6. **Ejecutor (WP nuevo o ampliación de WP-35).** Lock de ejecución con hashes,
   `requirements/windows-py3XX-runtime.lock`, para `Instalar_CSMP.bat` y `preparar_entorno.py`.
   - Generarlo con la misma herramienta que produjo los locks actuales.
   - Prueba: instalación sin conexión con `paquetes\` en un runner Windows.
7. **Usuario, en el PC autorizado.** Probar el anonimizador con un `.xls` real.
   - Protocolo (E5): solo vuelve al agente «se creó o no se creó» y la lista de **tipos** de dato que bloquearon.
     Nunca el archivo ni su contenido.
8. **Integrar #26, luego #28, #29 y #30.** WP-61 queda para el final, excluyendo `motor/`.

## 5. Cambios al plan e implementación

| ID | Prioridad | Cambio | Implementación |
| --- | --- | --- | --- |
| E1 | P1 | **Estado por WP.** `ESTADO.json` pasa a ser una vista generada. Cada Ejecutor edita solo `docs/implementacion/estado/WP-XX.json`; el Orquestador edita `estado/global.json`. Así los PR van directo contra `feat`, sin apilarse | **Hecho:** `tools/estado.py` (`dividir`, `agregar`, `validar`). Ida y vuelta idéntica comprobada sobre el `ESTADO.json` real. `validar` detecta estados desconocidos, dependencias inexistentes o `TODAS_LAS_DEMAS`, estados sin ejecutor o rama, INTEGRADO con dependencias sin integrar y conflictos sin dueño |
| E2 | P1 | **Verificador que falla ante la duda.** Un control sin dato da FALLA. Las pruebas retiradas se declaran con `--lista-eliminadas` (nodeids que deben estar ausentes) y no con conteos. La línea base se regenera tras cada integración (ya lo decía el plan; ahora es requisito para integrar el siguiente) | **Hecho** en `tools/verificar_wp.py`. Probado con una corrida real y 4 pruebas unitarias. Los conteos `--pruebas-eliminadas-*` siguen aceptados, marcados «sin lista verificable» |
| E3 | P1 | **Revisión cruzada.** Los PR de un agente los revisa y audita otro agente de distinta raíz. Los subagentes de una misma raíz cuentan como el mismo agente | Texto en «Roles» (sección 6). Campos `revisor` y `auditor` en `estado/WP-XX.json` |
| E4 | P2 | **Presupuesto de CI.** Push solo a `feat/**` y `main`; en PR, solo 3.12; la matriz completa, Playwright e instalador corren al integrar o a pedido. La concurrencia incluye el evento | **Hecho** en WP-34 (`207ae99`). La aceptación de la ficha queda: «un PR corre 3.12; la integración corre la matriz» |
| E5 | P2 | **Privacidad.** No subir `pre-<ID>` con árbol que contenga datos reales. WP-17 cubre etiquetas, ramas legado y `ccr-*`. Pruebas con datos reales solo en el PC autorizado, con salida binaria o agregada | Texto en «Reglas» (sección 6) |
| E6 | P2 | **Dependencias explícitas.** `TODAS_LAS_DEMAS` se reemplaza por una lista. WP-61 excluye `motor/`. Las dependencias se marcan como `funcional` o `archivo`. El gate es por dependencias, no por ola. WP-33 se ejecutó contra `feat` porque su dependencia con WP-14 era solo de archivo | Texto en «Olas y dependencias» (sección 6). El Orquestador ajusta las entradas de WP-33, 44, 61 y 70 |

**Sugerencias de mejora, sin implementar (N2):**
- Una prueba de layout que corra en Linux con Xvfb en cada PR, como `_layout_probe` sin pywinauto. Habría detectado el
  editor de 110 px antes de la CI Windows.
- `verificar_wp.py` debería incluir el lint de WP-34 (F401/F841) para que local y CI coincidan.
- Un fixture con más de 500 filas en la suite base: los errores de render por trozos solo aparecen ahí.

## 6. Texto de las enmiendas al plan (aprobado)

El usuario aprobó E1–E6 el 10 de octubre de 2026 («Apruebo las enmiendas E1–E6 del plan»). El texto quedó en
`docs/implementacion/PLAN_IMPLEMENTACION.md`, después de «Reglas de alcance», e incorporado en la regla 13, en «Roles»
(E3) y en «Olas y dependencias» (E6).

**Pasos del Orquestador al integrar esta rama (rol del Orquestador, no del Ejecutor):**

1. Integrar esta rama en `feat` antes de los PR que todavía editan `ESTADO.json`, o resolver su conflicto primero.
2. `python tools/estado.py dividir` sobre el `ESTADO.json` vigente en `feat`, y commit de `docs/implementacion/estado/`.
3. E6: reemplazar `TODAS_LAS_DEMAS` en `estado/WP-61.json` y `estado/WP-70.json` por listas explícitas; marcar como
   `archivo` las dependencias WP-33 → WP-14, WP-43 → WP-33 y WP-44 → WP-43 (se ejecutaron sin ellas integradas),
   con el formato `{"wp": "WP-14", "tipo": "archivo"}`; un texto simple cuenta como `funcional`. WP-61 excluye
   `src/nurus/personal/motor/`. Hoy `validar` da FALLA solo por esas dos entradas `TODAS_LAS_DEMAS`.
4. `python tools/estado.py agregar` y luego `validar`, que debe dar PASA. Desde ahí la CI corre `validar` en cada PR.
5. E3: anotar `revisor` y `auditor` en cada `estado/WP-XX.json`, siempre de una raíz distinta a la del Ejecutor.
6. E2: regenerar `linea_base.json` tras cada integración; el siguiente WP se verifica contra esa línea base.

> **Enmiendas del 10 de octubre de 2026 (aprobadas por el usuario el 10 de octubre de 2026)**
>
> - **E1.** Cada Ejecutor escribe solo `docs/implementacion/estado/WP-XX.json`. `ESTADO.json` se regenera con
>   `python tools/estado.py agregar` y la CI corre `validar`. Hasta que el Orquestador ejecute `dividir`, rige la regla
>   actual.
> - **E2.** El Verificador declara las pruebas retiradas con `--lista-eliminadas` (nodeids). Un control sin dato es
>   FALLA. Ningún WP se integra sobre una línea base anterior a la última integración.
> - **E3.** Revisor y Auditor son siempre de una raíz distinta a la del Ejecutor.
> - **E4.** Cada PR corre la CI mínima (3.12). La matriz completa y los empaquetados corren al integrar o a pedido.
> - **E5.** No se sube una etiqueta o rama cuyo árbol contenga datos reales. Las pruebas con datos reales devuelven al
>   agente solo un resultado binario o agregado.
> - **E6.** Las dependencias son listas explícitas, de tipo `funcional` o `archivo`. Una dependencia de archivo no
>   impide ejecutar el WP contra `feat`; solo obliga a resolver el conflicto textual al integrar.
