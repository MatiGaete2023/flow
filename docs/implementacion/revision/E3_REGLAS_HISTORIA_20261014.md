# Revisión E3 — reglas de historia del Asistente (14 de octubre de 2026)

Revisor E3 independiente (sesión distinta de la implementadora). Diff revisado: `f4e3c82..feat/reglas-historia-asistente`
(@ `35234c0`, idéntica a `claude/hopeful-hamilton-k2tcqp`). Base de comparación: worktree de `f4e3c82`.
Este informe no acepta ni declara institucional nada; solo el usuario, en un acta.

VEREDICTO: **APRUEBA CON CAMBIOS**

Los controles mecánicos pasan y la capa de análisis es sólida. Hay cuatro hallazgos mayores: tres alteran la observación
o las acciones fuera de lo declarado como deliberado (H1, H2, H3) y uno depende de una definición que no está en el repo
(H4). Ninguno es bloqueante si el usuario confirma que el comportamiento es el deseado; en ese caso basta documentarlo.

## Controles mecánicos

| Control | Resultado | Número |
| --- | --- | --- |
| `tools/estado.py validar` | PASA | — |
| `tools/check_links.py` | PASA | 0 enlaces rotos |
| `tools/check_real_ids.py` | PASA | 0 RUT reales |
| `ruff check` (alcance CI, excluye `motor/`) | PASA | 0 errores |
| `ruff format --check` | PASA | 320 archivos |
| pytest `tests` (`--continue-on-collection-errors`, xvfb) | igual a la base | rama: 946 pasan / 42 fallan / 8 errores de recolección; base: 843 / 42 / 8. Los 50 nodeids FAILED+ERROR son **idénticos** (tkinter) |
| golden `--expected docs/evidencia_p0/golden/semantic.json` | PASA | «PARIDAD PASÓ» |
| golden `f4e3c82` vs actual | dentro de la lista | 30 rutas distintas: `revision_textos` 3→4; `PROX_AUDIENCIA`; scope `HISTORIA`; `_meta/version` v9.1.0→v9.3.2; `espera_dce` 30→0; `espera_tome` 30→60; 24 rutas de 3 filas de Espera (texto, reglas, JSON de fila) con `HISTORIA.E05_DIAS`. 0 fuera de la lista |
| `grep -rn "\.Send(" src` | vacío | 0 |

Entorno: Python 3.13 (el lock es py312; se instaló sin errores), sin `tkinter`. No se ejecutó `verificar_wp.py`
(sus suites fallan por tkinter, ya documentado).

## Hallazgos (más graves primero)

### [H1] mayor — invariante A3: reglas que no dependen de la historia cambian la observación sin historia
`src/nurus/personal/motor/reglas_historia.py:95` (E10), `:127` (T5); llamadas en `reglas_espera.py:46,117` y
`reglas_informes.py:44`.

- **Hecho (N1)**: `lista_espera_larga` (E10) se basa solo en `T ESPERA` > 100 y programa residencial; `curador_instituciones`
  (T5) solo en la celda CURADOR de RUS. Ninguna consulta `historia()`. Se emiten aunque `ctx['historia'] is None`,
  incluso por el camino antiguo de Excel (sin paquete). T5 además marca nivel 1 (PRIORITARIO).
- **Caso que falla** (mismo libro, `as_of=2026-10-14`, ejecutado en la rama y en `f4e3c82`):
  - Espera: `RVA RESIDENCIA UNO`, `T ESPERA=150`, CURADOR `(…)UNO - Institución: Corporación A#(…)DOS - Institución: Fundación B`.
    Base: `[ESPERA.E05_PROYECTO_Y_CORREO]`. Rama: además `HISTORIA.E10_LISTA` («Registra 150 días en lista de espera.»)
    y `HISTORIA.T5_INSTITUCIONES` (texto «se sugiere eliminar…»).
  - Informes: misma celda CURADOR, vencido 20-sep → la rama agrega `HISTORIA.T5_INSTITUCIONES`.
- Las pruebas lo fijan a propósito (`test_historia_espera.py:79` llama a E10 sin historia; `:103` T5 sin historia), así
  que la implementación es coherente consigo misma. Lo que no cuadra es la documentación: WP-75 e
  `INFORME_REGLAS_HISTORIA_20261014.md` §3 dicen «golden PARIDAD PASÓ (A3)», pero el golden no tiene filas residenciales
  con espera > 100 ni curadores con «Institución:». Además, E10 y T5 no están en la lista de cambios deliberados de WP-73 ni en
  `PLAN_VIGENTE` q). `AGENTS.md` exige que las reglas tradicionales queden intactas «salvo decisión expresa del usuario».
- **Propuesta**: que el usuario decida. (a) Si E10 y T5 son reglas RUS por diseño: agregarlas a la lista de cambios deliberados
  (WP-73 / PLAN_VIGENTE) y añadir una fila de golden que las cubra. (b) Si no: condicionar con `if _h() is None: return []`
  al inicio de ambas funciones. [CONFLICTO_ABIERTO-1]

### [H2] mayor — camino de captura: J2 y la marca R6 aparecen sin historia
`src/nurus/personal/work.py:374` (`self._prepass(batch, cols, historia_by_record is not None)`);
`capture_input.py` siempre pasa un dict.

- **Hecho (N1)**: `import_capture` arma `historia_by_record = {record_id: None, …}` aunque el paquete no traiga ninguna
  consulta `historia_tramites`. Como el dict no es `None`, `_prepass` corre. Eso emite `HISTORIA.J2_ACUMULADOS` (texto) en
  Informes y `R6_DUPLICIDAD` (nivel 1) en Espera y Cumplimiento.
- **Caso que falla**:
  - Informes, 2 filas `C-1-2026`/LAJA/mismo RUT y programa, vencidas el 20-ago y el 20-sep, `historia_by_record={}`:
    obs. termina en «Registra 2 informes vencidos en RUS; el más antiguo venció el 20 de agosto de 2026.» Con el mismo libro
    por el camino Excel, no aparece. El mismo dato produce dos resultados según cómo entre.
  - Espera, 2 filas mismo RIT/programa, sin historia: `avisos=[(1,'R6_DUPLICIDAD',…)]`; en la bandeja, ambas filas
    salen `PRIORITARIO` (`case_statuses` → `(1,'PRIORITARIO','POR_PREPARAR')`).
- `test_historia_informes.py::test_j2…` asserta exactamente esto (`correr(..., {})` sin historia → J2), así que es de
  diseño. Pero contradice A3 tal como lo formula el encargo («sin historia, resultado idéntico»).
- **Propuesta**: que el usuario defina si J2 y R6 dependen de la historia. Si dependen: `with_history = any(h is not None
  and h.con_datos for h in (historia_by_record or {}).values())`. Si no: declararlo y hacer que el camino Excel también
  ejecute el pre-pase, para que el resultado no dependa de la vía de entrada. [CONFLICTO_ABIERTO-1]

### [H3] mayor — I1/I2 dan por contradicho un informe vencido con trámites anteriores al vencimiento y quitan PC_INFO
`src/nurus/personal/motor/reglas_informes.py:29-34`; `reglas_historia.py:301` (`informe_historia` no filtra por fecha).

- **Hecho (N1)**: hay contradicción si existe cualquier «Ingreso informe de cumplimiento X» o «Prorroga» en la
  ventana, sin importar su fecha respecto del vencimiento. En ese caso el principal pasa a `HISTORIA.I01_SIN_ENVIO_*`. El
  evento `INFORMES.I01_*` desaparece, y con él las acciones `programa_vencido` y `PC_INFO`.
- **Caso que falla**: informe vencido el 20-sep-2026; historia `Ingreso informe de cumplimiento X` del 01-jul-2026, folio 30
  (el informe del período anterior). Resultado: `actions=[]`, marca nivel 2 «Informe vencido en RUS contradicho por la
  historia». Lo mismo con `Prorroga` del 20-jun-2026. Sin historia: `['programa_vencido','PC_INFO']`. La única prueba
  (`test_i1…`) usa un ingreso **posterior** al vencimiento (25-sep vs 20-sep), así que el caso anterior no está cubierto.
- **Inferencia (N2)**: un trámite anterior al vencimiento no puede acreditar el envío del informe que venció después.
  Supuesto: el doc de reglas no ordena ignorar la fecha. No tengo datos sobre el texto exacto de I1/I2 en el doc «Reglas del
  Asistente». A diferencia de la regla 3, esta falta de filtro no figura como riesgo aceptado.
- **Propuesta**: en `informe_historia`, `lista = [t for t in tramites(nombre) if t.fecha >= _d(fecha_vencimiento)]` cuando
  `vencido`, más una prueba con un trámite anterior. Para I2 (Prórroga de la medida), el usuario debe confirmar si una
  prórroga contradice un informe vencido. [CONFLICTO_ABIERTO-2]

### [H4] mayor (condicionado a la definición de la regla 6) — hermanos en la misma causa y programa salen como «duplicidad», nivel 1
`src/nurus/personal/motor/analisis_historia.py:66` (`duplicidades` agrupa por tribunal+RIT y compara programa, sin RUT).

- **Hecho (N1)**: dos NNA distintos (RUT 11111111-1 y 22222222-2, nombres distintos) en `C-9-2026`, ambos en `PRM UNO`:
  los dos reciben `(1,'R6_DUPLICIDAD','Más de un ingreso al mismo programa en la causa')`. Si además hay «Ingreso efectivo»
  en la historia, la regla 3 agrega `R3_MULTI`.
- **Inferencia (N2)**: en protección es habitual que hermanos compartan RIT y programa. Si la regla 6 apunta a ingresos
  duplicados **del mismo NNA**, la bandeja se llenará de PRIORITARIO falsos. La prueba `test_regla6…` usa nombres
  distintos, por lo que no distingue los dos casos.
- **Propuesta**: si la regla es por NNA, agregar el RUT normalizado a la clave de `duplicidades` (`(tribunal, rit, rut)`).
  Si es por causa, dejarlo y bajar a nivel 2. No tengo datos sobre la definición en el doc. [CONFLICTO_ABIERTO-3]

### [H5] menor — un mismo trámite se cita dos veces
`reglas_historia.py:53` (`constancia` solo omite si **todos** están emitidos); `analisis_historia.py:35`.

- **Caso 1**: historia `Ingreso efectivo` 01-jul f10, `Ingreso efectivo` 01-sep f20, `Deja sin efecto medida interconexión
  M.N.` 10-sep f25, en Espera. T2 cita f20 como trámite anterior. Después, la regla 3 emite `VARIOS` («constan 2 trámites…
  el último… folio 20»). El folio 20 aparece dos veces en la misma observación.
- **Caso 2**: `Orden de búsqueda` f10, `Termino de búsqueda` f20, `Deja sin efecto búsqueda` f30. Los dos cierres se
  emparejan con la misma orden, y la orden f10 se cita dos veces.
- **Caso 3 (mezcla de folios)**: `max(previas, key=(folio, fecha))` en `busqueda` prioriza el folio numérico, mientras que
  `posterior` cae a la fecha si un folio no es numérico. Con una orden f10 (01-jul) y otra «s/f» (01-ago) seguidas de un
  término el 10-ago, se empareja con la f10 en vez de la más cercana. `deja_sin_efecto_mn` usa `(fecha, folio)`: los dos
  criterios no coinciden.
- **Propuesta**: en `constancia`, filtrar `lista = [t for t in lista if _id(t) not in emitidos()]` antes de componer. En
  `busqueda`, usar `key=lambda o: (o.fecha, o.folio)` igual que T2. Decidir si una orden puede tener dos cierres.

### [H6] menor — la regla 1 toma el folio de cualquier trámite de esa fecha
`reglas_historia.py:74` (`folio_resolucion`).

- **Caso**: resolución RUS del 05-oct-2026; historia con un único trámite ese día, `Ficha individual` folio 7. Sale
  `E04_CON_FOLIO`: «Ingreso al programa PRM UNO ordenado el 5 de octubre de 2026, folio 7». Ese folio es el de la ficha, no
  el de la resolución.
- Corresponde a la decisión N2 nº 6 y al [CONFLICTO_ABIERTO] heredado (nombre exacto de la resolución de ingreso).
  Además, `folio_txt` acepta «s/f» o «-1» como folio válido (`t.folio_txt` no vacío), y lo mismo pasa en `folio_ingreso`.
- **Propuesta**: excluir de la búsqueda los nombres de trámite conocidos que no son resoluciones (fichas, informes,
  escritos) y exigir `t.folio >= 0`, hasta que el usuario dé el nombre exacto.

### [H7] menor — cambio no declarado en `RULE_SCOPES`
`src/nurus/personal/capture_findings.py:9`.

- **Hecho (N1)**: además de `HISTORIA.`, se agregó `COMUN.` al filtro de `updated_contexts`. Antes, los hallazgos `COMUN.*`
  no se filtraban y se duplicaban al reimportar. **Inferencia (N2)**: el cambio es correcto, pero no figura en el
  informe ni en WP-73/78.
- **Propuesta**: mencionarlo en WP-78 y agregar una prueba de reimportación que cuente los hallazgos `COMUN.*`.

### [H8] menor — documentación con conteos inconsistentes
- `textos_base.json` tiene **34** claves `HISTORIA`. WP-72 dice «32 textos» y el informe §1 dice «33 textos con P_FINALIZADA».
- `PLAN_VIGENTE.md` q) todavía dice que las redacciones (P) «esperan aprobación» y que todo es `confirmado:false`; s) y t)
  lo corrigen. Conviene actualizar q) para que no se contradiga.

## Resultados sin hallazgo (verificados)

- **B. Cambios deliberados**: los umbrales, la `PROX_AUDIENCIA` y la migración `TEXT_REVISION` 3→4 se probaron con una
  configuración rev3 real de `f4e3c82`:
  - rev3 intacta → dce 0, tomé 60, «Audiencia decretada», scope HISTORIA agregado.
  - dce 15 / tomé 45 personalizados → se respetan.
  - proyecto_tomé 40 → tomé queda topado en 40.
  - texto de audiencia editado → se respeta.
  - rev2 → migra igual.
  - rev4 → no se toca.

  El bloque va después de rev<2 y antes del bloque general que fija `revision_textos=4`. El tope solo actúa en el paso
  30→60, porque `validate` ya rechaza tomé > proyecto. Mayoría de edad sola con marca 3, E-05 + días, sin seguimiento +
  «Medida sin vigencia», C-03 + egreso proyectado y `C10_HOJA2` con C-03: presentes. No falta ninguna regla original: todas
  las ramas de la base siguen llamándose; `_complementarias` solo dejó de usarse en la rama de mayoría, que es lo deliberado.
- **C. Orden**: en Espera, Cumplimiento e Informes se evalúa primero «sin seguimiento», luego la mayoría de edad (no existe en
  Informes, como en la base) y luego `P_FINALIZADA`. Ninguna regla nueva se evalúa antes. En Espera, T2 (`mn`) se evalúa
  antes que la regla 3 y que las constancias de D.R. y búsqueda.
- **D. Capa de análisis**: `clave()` (NFKD + minúsculas + `-().`→espacio) se aplica en ambos lados. Folios «-1» y «s/f» dan
  `folio=-1` y se comparan por fecha. REITERA llama a `evento(original)`, por lo que las acciones se conservan (C04 →
  `medidas`). Una observación «Se reitera…» no cuenta como primera, y la de hoy se excluye. T5 cuenta instituciones
  distintas tras `clave()`. C8 usa `historia()` + `permite_ausencia` + `desde/hasta`. `con_datos` = COMPLETA/PARCIAL.
- **E. P_FINALIZADA**: verificado con «p-1-2026» (dispara), «C-1-2026» (no), egreso el mismo día (no), egreso con punto final
  (lo reconoce), «Conciliación» con tilde (dispara), egreso anterior al segundo cierre (dispara), cobertura PARCIAL
  (dispara), FALLIDA/NO_CONSULTADA/CANCELADA (no). Sin acciones ni marca.
- **F. Clave de unión**: el contrato la respalda. `contracts/csmp_capture_v1/DICCIONARIO.md:176-177` declara
  `historia_causa (source_system, court_code, cause_id)`. En el exportador, el registro toma
  `cause_id = SITFA_CAUSA_RUS = ShowObservaciones[0]` (`descargador/vinculos.py:141`), y `extract` exige
  `obs[0] == ShowHistoria[2]` cuando hay enlace de historia (`vinculos.py:134`). La historia usa `causa_id = ShowHistoria[2]`
  (`vinculos.py:199`). `court_code` sale de `selection.tribunal` en los dos lados (`export_csmp_capture.py:101,385`). Gana la
  mejor cobertura (empate: la primera). La bitácora solo se usa para registros `VERIFICADA`.
  `RULESET_VERSION=…+historia-1`, y el caché también depende del hash de la configuración (incluye la versión de textos).
  `Row.avisos` tiene valor por defecto, así que `Row(**r)` carga sesiones antiguas.
- **G. Textos**: las variables de cada clave literal coinciden con el `render` del código. Las claves dinámicas (MATERIA_*,
  I1_*, I01_SIN_ENVIO_*, R11_*) reciben las variables correctas. No hay claves huérfanas ni faltantes. El espejo RUS tiene el
  mismo texto en todos los scopes (sin `confirmado`, igual que antes). `confirmado:true` está exactamente en las 15
  redacciones (P) más `P_FINALIZADA`; las otras 18 están en `false`.
- **H. Bandeja**: nivel 1 va primero incluso sobre `INCIERTO`; se muestra solo el aviso de nivel menor; `CaseStatus.nivel=0`
  y `marca=''` por defecto; `RULE_SCOPES` incluye `HISTORIA.`.
- **I. Privacidad**: los fixtures son ficticios (`check_real_ids` pasa). No hay `.Send(`, escritura a RUS/SATURNO ni red. Los
  xlsx regenerados del golden son productos ficticios.

## Decisiones N2 desafiadas

1. **Medida vigente descartada**: ya no es N2, la decisión r) de `PLAN_VIGENTE` la toma el usuario. Sin objeción.
2. **Regla 3 sin filtro de fecha**: el riesgo es mayor que «avisos de nivel 2». La regla 3 **reemplaza E-04/E-05/E-06 y
   borra las acciones**. Caso: `T ESPERA=40`, historia `Ingreso efectivo` 20-jun-2026 (anterior al inicio de la espera,
   ~04-sep). Resultado: `actions=[]` (sin `programa_espera` ni `PC_IE`). Sin historia: `['programa_espera','PC_IE']`. Se
   podría filtrar con `t.fecha >= corte - T ESPERA`. Si el usuario mantiene el riesgo, debe quedar claro en el acta que
   incluye la pérdida de la acción. [CONFLICTO_ABIERTO-4]
3. **AMBULATORIO por exclusión**: solo afecta si existe el escrito exacto «Inf. Cump Programa Ambulatorio». Riesgo bajo.
4. **C6 solo programa duplicado**: ver H4.
5. **T4 primera fecha futura**: no encontré una implementación de T4 en el diff; `_audiencia` no cambió. No tengo datos
   sobre dónde se aplica.
6. **Regla 1, folio único por fecha**: ver H6.
7. **J2, una fila por informe**: agrupa por tribunal+RIT+RUT+programa. Si la descarga trae una fila por NNA y no por
   informe, J2 nunca dispara (falso negativo silencioso). No verificable sin una nómina real anonimizada.
8. **P_FINALIZADA en Informes solo en filas con observación**: decidido por el usuario (c). Sin objeción.
9. **WP-71..79**: correcto. `tools/estado.py` valida con `WP-\d{2}`.

## Cobertura de pruebas (J)

- Hay pruebas con aserciones concretas para las 34 claves y las marcas.
- `test_historia_finalizada.py::test_solo_rit_que_parte_con_p[""]` **no prueba el RIT vacío**: `fila(rit='')` lo reemplaza
  por `X-1-2026` (`rit or f'X-{n}-2026'`). El código maneja bien el vacío, pero la prueba no puede fallar por esa causa.
- Faltan casos de borde:
  - I1/I2 con un trámite anterior al vencimiento (H3).
  - Hermanos en duplicidad (H4).
  - Re-cita entre T2 y la regla 3 (H5).
  - `P_FINALIZADA` con derivación sin seguimiento (debe ganar NO_SEGUIMIENTO).
  - `P_FINALIZADA` en una fila de Informes que no genera observación.
  - Reimportación con `COMUN.*` (H7).
- Sin historia: E10, T5, J2 y R6 tienen pruebas que **afirman** que disparan sin historia, coherentes con H1/H2.
  `test_historia_paquete::test_sin_trámites…` compara dos importaciones de la rama entre sí, no contra la base, así que no
  protege A3.

## No verificado

- La clave de unión contra un **paquete real**: verifiqué solo el código y el contrato. Falta un extracto anónimo.
- El texto del doc «Reglas del Asistente» para E10, T5, J2, R6, I1/I2 y T4: No tengo datos (no está en el repo).
- Si «Ingreso informe de cumplimiento X» es un nombre literal de SITFA o una plantilla con número: No tengo datos.
- La suite con tkinter (8 módulos de `tests/` y 4 de `descargador/`) y la CI Windows.

## [CONFLICTO_ABIERTO] nuevos

1. E10, T5, J2 y R6 se aplican sin historia (H1, H2): ¿son reglas RUS por diseño (y entonces se declaran como cambio
   deliberado y entran al golden) o deben exigir historia?
2. I1/I2: ¿un trámite anterior al vencimiento contradice el informe vencido? ¿Una prórroga de la medida contradice un
   informe vencido? (H3)
3. Regla 6: ¿la duplicidad es por NNA (RUT) o por causa? (H4)
4. Regla 3: el riesgo aceptado incluye la eliminación de PC_IE/correo en Espera, no solo el aviso. ¿Se mantiene? (N2 nº 2)
