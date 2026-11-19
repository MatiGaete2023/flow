# Registro de ejecución — Plan de integración FLOW Rust/Python (18-nov-2026)

Documento vivo para que otra IA continúe. Orden: **estado → qué se hizo → decisiones → pendientes → comandos**.
Actualizar al cerrar cada paso. Hechos (N1) separados de inferencias (N2) y especulación (N3).

## Fuentes y base
- Encargo: `01_Plan_integracion_FLOW_Rust_Python.md`, `02_Guia_de_consulta_RUS_SITFA.md` (copias en esta carpeta).
- Rama de trabajo: `claude/wizardly-volta-vl2lqg`, creada desde `origin/feat/flow-estudio-d-20261117` @ `d2935dd8087b7467677e12d423fac512f1461190` (Estudio D).
  `main` (1 commit, solo README) NO es el producto. No se fusionó nada a `main`, no hay PR, release ni `--force`.
- Una sola definición funcional: Python (`src/nurus/personal/motor/`) es la referencia; Rust la porta y se verifica contra el oráculo.

## Paso 1 — Línea base (N1, medida 2026-11-18, antes de cambios)
- Rust `cargo test --workspace --locked`: 314 pruebas, 0 fallos.
- Python `python -m pytest -q tests --continue-on-collection-errors`: 55 fallos/errores **preexistentes**, todos por entorno
  (el contenedor no tiene `tkinter`). Lista exacta: `base_python_fallos.txt`. Criterio: el conjunto no puede crecer.
  **Ojo**: sin `--continue-on-collection-errors` pytest aborta en la recolección y la comparación sale vacía (error mío en
  corridas intermedias; la corrida final con el flag da el mismo conjunto de 55).
- Verificado además: `cargo build --release --locked -p flow` compila (3 min 45 s).
- `python tools/verificar_wp.py` → FALLA por «Restricciones absolutas» y «Ruff» (medición no disponible en el contenedor); preexistente.
- Suite del Descargador: sus pruebas de interfaz necesitan `tkinter` y fallan igual aquí; las de lectura/exportación/paquete pasan
  (se corrieron con un stub de `tkinter` en el scratchpad, solo para importar `descargador.py`).

## Estado actual (resumen)
| Paso | Estado | Dónde |
|---|---|---|
| 1 Base y línea base | HECHO | este registro |
| 2 Dato de captura al motor (contrato aditivo) | HECHO en Python y Rust (lector); productor Python solo Escritos | `src/nurus/capture/contract.py` (1.2.0), `capture/escritos.py`, `rust/crates/flow-trabajo/src/{fuentes_nuevas,paquete}.rs` |
| 3 Escritos en el detalle de causa | HECHO: lector HTML, Descargador Python y Descargador Rust (misma respuesta de Historia; ver `lecturas.rs::leer_escritos`, `exportar.rs::escritos_de_causa`). La extensión JS no necesita cambios para la muestra actual (la tabla ya viene en la respuesta); **otras versiones que cargan la pestaña al abrirla: PENDIENTE** | `src/nurus/escritos_html.py`, `src/nurus/capture/historias.py`, `descargador/export_csmp_capture.py::_escritos` |
| 4 Ingreso Histórico Protección | PARCIAL (lectores ajustados a la estructura real «Nombre Proyecto»/LRPA): lector HTML, tabla del contrato, motor y **captura manual sin interfaz** del Descargador Python (`descargador/historico_manual.py::capturar`, `export_capture(historico=carpeta)`); pertenencia solo por RUT o nombre de ≥2 palabras visible. y el equivalente Rust (`flow-captura::leer_historico`, `flow-trabajo::historico_manual`, `exportar::Opciones.historico`; mismo `historico.json`). **Botón/control en la interfaz (Python y Rust): PENDIENTE** | `escritos_html.read_historico`, `descargador/historico_manual.py` |
| 5 «Solo una vez» + primer nombre | HECHO (bitácora + marca local idempotente `Work.consignar`); **botón en la interfaz: PENDIENTE** | `motor/evidencia.py`, `Work.consignar` |
| 6 Conservar cada vencimiento / fuentes | PARCIAL: asociación explícita por fecha proyectada; el agrupador `acumulado` sigue contando todas las fechas (decisión abajo) | `capture/escritos.py::evidencia_informe` |
| 7 Motor y composición | HECHO Python y Rust (oráculo 929 casos + paquete) | `motor/reglas_*.py`, `flow-motor/src/{hist,reglas,ctx,lib}.rs` |
| 8 Correo/Word/Excel | HECHO por acciones explícitas + filtro de proyectos Word (Python y Rust); Excel/nómina heredan `actions` | `personal/resolutions.py`, `flow-productos/src/word.rs` |
| 9 Matriz y paridad | CASI COMPLETO: ver `MATRIZ_PARIDAD.md` (9 de 10 escenarios cubiertos) | `tests/test_integracion*.py`, `rust/casos/integracion.json` |
| 10 Verificar y entregar | PARCIAL: pruebas pasan; falta CI en Windows, Chromium ficticio de la extensión y verificación visual | ver pendientes |

## Qué se implementó (archivos)
**Python**
- `motor/evidencia.py` (nuevo): `Escrito(s)`, `ProgramaHistorico`, `Historico`, `EvidenciaInforme`, `programas_vigentes_otros`,
  `constancia_en_bitacora`. Vigencia = Fecha egreso `--` **y** Causal `SIN INFORMACION` en la misma fila (dos guiones, también Unicode).
- `motor/reglas_historia.py`: `otro_programa`, `escritos_pendientes`, `escrito_pendiente`, `informe_pendiente`; `informe_historia` y
  `escrito_sin_ingreso` aceptan `folio` explícito; `informe_historia` emite la frase de asociación cuando la resolución ya consta.
- `motor/reglas_{espera,cumplimiento,informes}.py`: integración (Espera y Cumplimiento: otro programa; Cumplimiento e Informes: pendiente).
- `textos_base.json` + `rus/textos_observaciones.json`: `HISTORIA.OTRO_PROGRAMA`, `INFORME_AVANCE_PENDIENTE`, `INFORME_REVISAR_ASOCIACION`.
- `work.py`: `analyze(... escritos_by_record, historico_by_record, evidencia_by_record, bitacora_parcial_by_record)`, `Row.otros_programas`,
  `Work.consignar` (idempotente; recalcula la fila), `consignaciones` persistido en `trabajo.json`.
- `capture/contract.py`: tablas `escritos` y `historico_proteccion`, versión **1.2.0 aditiva** (solo si el paquete las usa; 1.0/1.1 siguen
  idénticos y se leen con estado `NO_SOLICITADA`); `manifest.schema.json` actualizado; `capture/escritos.py` (paquete → motor).
- `personal/capture_input.py`: cablea las fuentes en `import_capture` y en el recálculo tras edición (`captured_extra_context`).
- `capture/temporal.py`: `RULESET_VERSION` termina en `+integracion-1` (fuerza recalcular trabajos antiguos).
- `escritos_html.py` (nuevo): `read_escritos`, `read_historico` con estados diferenciados (`ESCRITOS_AUSENTE` / `ESCRITOS_NO_VERIFICADA` /
  `ESCRITOS_RESPUESTA_INCOMPLETA`…).
- `descargador/export_csmp_capture.py::_escritos`: lee Escritos de la misma respuesta de Historia respaldada (`historias.cargar`).
- `personal/resolutions.py`: en casos con fechas resueltas por evidencia, cada proyecto incluye solo las filas que lo piden.
**Rust**
- `flow-dominio/src/evidencia.rs` (nuevo, espejo de `evidencia.py`), `flow-motor` (`ctx.rs`, `hist.rs`, `reglas.rs`, `lib.rs`: `Entrada` y
  `Resultado.otros_programas`), `flow-trabajo` (`Contexto` con las fuentes nuevas; `fuentes_nuevas.rs`; `paquete.rs`), `flow-productos/word.rs`.
- Oráculos: `rust/oraculo/generar_integracion.py` → `rust/casos/integracion.json` (929 casos Python=Rust);
  `generar_paquete_integracion.py` → `crates/flow-trabajo/tests/datos/paquete_integracion.{zip,json}`.
  `generar_casos.py` se regeneró (los 3 JSON base cambian de forma masiva: se añadieron 3 textos y eso desplaza el muestreo aleatorio de
  reglas desactivadas; el contenido funcional que cambió es solo `INFORME_INGRESADO` → frase de asociación).

## Decisiones tomadas (RATIFICADAS por el usuario el 18-nov-2026: «Ratifico, y continua con el plan»; la 6 y la 7 son técnicas)
1. **Texto del caso «resolución ya consta + calendario pendiente»**: se usa la frase del plan (`INFORME_REVISAR_ASOCIACION`) en lugar de
   `INFORME_INGRESADO` en Informes vencidos con ingreso atribuido; se **conserva** la frase principal «Informe de avance vencido desde…».
   Cambia 2 pruebas existentes (`test_historia_informes.py`) y los casos base del oráculo.
2. **Pendiente solo en vencidos**: la regla de Escritos se evalúa en Informes **vencidos** (`dias<0`) y en Cumplimiento; no en Espera
   (no hay informe esperado) ni en «por vencer» (el correo preventivo no es «pedir cuenta»).
3. **Atribución**: (a) asociación explícita (`linked_ingreso_id` + `linked_projection` == fecha del informe; o referencia documental `Doc.` común
   con filas de Historia); (b) un único candidato pendiente en causa de un solo registro y sin otros vencidos. Varios candidatos → marca
   `ESCRITO_PENDIENTE_SIN_ATRIBUIR` y se conservan acciones. Bandeja vs Historia: prevalece la lectura posterior de Historia; sin secuencia → marca
   `ESCRITO_ESTADOS_INCOMPATIBLES`.
4. **J2 «Registra N informes vencidos»** sigue contando todas las fechas vencidas de RUS (es un hecho de RUS); la exclusión por evidencia
   actúa sobre acciones y proyectos, no sobre ese conteo.
5. **Marca local «consignado»** = `Work.consignaciones[row_id]` (claves de programa). Nunca se escribe al generar/exportar. Falta el botón.
6. Un `Escrito` solo es «EXPLICITO» si el **paquete** lo asoció (`linked_*`). El Descargador actual **no sabe asociar**: deja `linked_*` vacío,
   por lo que hoy rige solo la rama «único candidato». La asociación explícita está lista en motor/contrato/Rust pero sin productor real.
7. Histórico: `ownership` VERIFICADA solo si el texto visible contiene RUT/nombre esperado; si no, INCIERTA (se continúa sin afirmar).

## Hallazgos de las capturas de SITFA del usuario (19-nov-2026; solo estructura, valores ficticios en fixtures)
- **Escritos** (N1): columnas `Doc.`, `Tipo Escrito`, `Solicitante`, `Estado`; valores en **minúscula** («inf. cump programa ambulatorio»,
  «pendiente») y `Doc.` es solo un icono PDF (sin texto). La comparación ya es insensible a mayúsculas/tildes. **No hay folio, fecha ni
  período visibles**: el enlace con la proyección no puede salir de la fila; rige «único candidato».
- **Ingreso Histórico Protección** (N1): ventana «Historial Menores» (`PopUpHistorial.jsp?tipo_popUp=77&RUT_Litigante=…&RUT_DV=…`). La
  columna del programa se llama **«Nombre Proyecto»** (no «Programa»), el encabezado no es la primera fila (hay banda de título),
  existe otra tabla **«Ingreso Historico(LRPA)»** con casi las mismas columnas y hay una fila de relleno al pie. Vigente = `Fecha Egreso`
  `--` y `Causal Egreso` `SIN INFORMACION` (confirmado en la fila real). Lectores Python y Rust adaptados (`fixtures_integracion/`).
- **Pertenencia**: la dirección de la ventana pide el RUT de la persona; `historico_manual.rut_de_url` lo usa como prueba fuerte (y como
  desmentido si pide otro RUT). No se guarda la dirección.
- **N2 (riesgo)**: el nombre del programa del histórico («PRM - NOMBRE») debe ser igual, con la normalización de FLOW (guiones y tildes), al de
  la columna programa de Espera/Cumplimiento; si SITFA los escribe distinto, «otro programa» incluirá al actual. Comprobar con un caso real.

## Correcciones E3 (19-nov-2026) — informe en `E3_INFORME_20261119.md`, veredicto NO PASA sobre `c83414d`
Las 16 observaciones se corrigieron en Python y Rust, cada una con prueba propia (`test_e3_*` en `tests/` y `descargador/`, casos
dirigidos `INT-*-D30…D41` del oráculo y pruebas Rust). Requiere **nueva revisión E3** sobre la punta actual.
| ID | Corrección |
|---|---|
| E3-01 | Escrito vinculado a otro ingreso = `vinculo='AJENO'`; nunca es candidato (`capture/escritos.py`, `reglas_historia.escritos_pendientes`, `fuentes_nuevas.rs`, `hist.rs`). |
| E3-02 | Contrato (Python) y lector Rust (`validar_alcance`) rechazan filas de `escritos`/`historico_proteccion` fuera del alcance y pantalla de su consulta. El anonimizador trata `court_code` de los filtros igual que la columna. |
| E3-03 | Pertenencia del histórico con el texto del titular, excluyendo las tablas de programas (Protección y LRPA). |
| E3-04 | Anonimizador: `egreso_cause`, `escrito_type`, `escrito_status` se conservan solo si son de catálogo; `egreso_date` solo si es fecha o `--`; lo demás se depura como texto libre. |
| E3-05 | Escritos con `IDENTIDAD_INCIERTA` conservan ese estado; «único candidato» exige bandeja `COMPLETA`. El vínculo propio explícito vale aunque la lectura sea parcial. |
| E3-06 | Escritos e histórico con `leido` posterior al corte no deciden (`contexto.escritos/historico`, `flow_motor::analizar`). |
| E3-07 | Se elige la última lectura fiable (completa o vacía comprobada) al corte; lecturas futuras descartadas; dos completas discordantes del mismo instante → PARCIAL. |
| E3-08 | Descargador Python y Rust: fila corta → consulta y filas PARCIAL; sin conversión forzada a `VACIA_COMPROBADA`. |
| E3-09 | Proyectos: tipos efectivos por fila tras la decisión humana (`_tipos_efectivos`; Rust: RES con tipo concreto). |
| E3-10 | Folios enlazados a otro informe (`EvidenciaInforme.folios_ajenos`) no entran por la vía genérica; la evidencia explícita de Historia del mismo informe prevalece sobre el pendiente (marca `ESCRITO_ESTADOS_INCOMPATIBLES`). Oráculo de integración corregido (D-PEND / D-ESC / D-OTRO). |
| E3-11 | `informe_pendiente` devuelve la decisión aparte de la frase: con la frase desactivada no vuelven correo ni pide cuenta. |
| E3-12 | La constancia previa exige que el primer nombre sea el sujeto inmediato del verbo. |
| E3-13 | Control visible de la pestaña «Escritos» sin tabla → `ESCRITOS_NO_VERIFICADA`. **Efecto**: la ventana de muestra (H0) trae ese control sin tabla, así que hoy cada causa exporta Escritos `FALLIDA` con incidencia (no vacía). Referencias Rust regeneradas con `FLOW_REGENERAR=1`. |
| E3-14 | Rust acepta U+2212 como guion. |
| E3-15 | Generadores escriben LF (`newline='\n'`); `write_capture` fija `create_system=3`; la prueba de estructura genera en carpeta temporal y compara sin tocar los versionados. |
| E3-16 | Consignar es transaccional en Python (revierte la marca si falla el recálculo) y Rust (copia; solo reemplaza si el recálculo resulta). |

## Correcciones E3B (19-nov-2026) — segunda revisión, resumen en `E3_INFORME_2_20261119.md`, veredicto NO PASA
Las 14 observaciones se corrigieron con prueba propia (`test_e3b_*` en `tests/` y `descargador/test_lotes.py`; Rust `e3b_*` en
`flow-trabajo/src/fuentes_nuevas.rs`, `flow-dominio/src/evidencia.rs`, `flow-productos/tests/nomenclatura.rs`,
`flow-trabajo/tests/flujo.rs`). Requiere **tercera revisión E3** sobre la punta actual.
| ID | Corrección |
|---|---|
| E3B-01 | Orden de lecturas por instante UTC; el corte compara el día escrito en la lectura (`escritos.py::_instante`, `fuentes_nuevas.rs::instante`). |
| E3B-02 | Empate de completas en el mismo instante: discrepancia por cobertura **o contenido** (firma de filas) → PARCIAL `_discordante`, elegida la de menor `query_id` en cualquier orden. Histórico discordante → identidad INCIERTA. |
| E3B-03 | Escritos con `IDENTIDAD_INCIERTA` no producen `EvidenciaInforme`. |
| E3B-04 | Referencia `Doc.` con varios dueños: no se atribuye a ninguno y sus folios quedan bloqueados para la vía genérica. |
| E3B-05 | Titular del histórico solo desde campos rotulados (RUT/RUN/nombre del niño), fuera de las tablas de programas (Python y Rust). |
| E3B-06 | Anonimizador emite el literal canónico del catálogo; cualquier otro valor se depura. |
| E3B-07 | Sujeto compuesto: primer nombre + prefijo del nombre completo; texto en NFC antes de tokenizar (Python y Rust). |
| E3B-08/09 | Proyectos: tipos efectivos tras la decisión humana con el mismo cálculo al elegir y al preparar (`_tipos_efectivos`, `word.rs::tipos_efectivos`). |
| E3B-10 | Generadores de oráculo escriben LF (incluidas llamadas multilínea). |
| E3B-11 | Flujo Rust: bitácoras pedidas sin leer (sin vínculo, pendientes, descartadas o ingreso verificado sin bitácora en el paquete) → **PARCIAL** con aviso por tribunal (`recoleccion::avisos_bitacoras_faltantes`, `Producto.sin_bitacora`, `flujo::avisos_de_bitacoras`). Mismo aviso en «Crear paquete de este lote» y al terminar un lote. Fuentes FALLIDA/INCIERTA se listan (las bitácoras son PARCIALES por diseño y no se listan). |
| E3B-12 | Retomar: una lectura referenciada ausente o alterada ya no bloquea. `lotes::ejecutar(retomar)` la descarta y deja su ingreso pendiente (`recoleccion::reparar_lecturas`); `flujo::fase_lote` no reutiliza un lote VALIDADA con lecturas no íntegras. Sin JSON inventado. Mensajes nombran el archivo. **Causa probable N2 del «no encuentra el .json» de Mulchén**: `Recoleccion::con_fecha` abortaba con «Falta una lectura verificada del lote» en cada intento. |
| E3B-13 | `flujo::ejecutar_en` devuelve la raíz del intento; `flujo::mensaje_fallo` la nombra o dice que no alcanzó a guardar el control. La UI ya no recomienda otro paquete pendiente. |
| E3B-14 | Descargador Python emite `folder`/`plan` después del primer `resumen.json`. |

**Mulchén (área F)**: N1 sin causa individual confirmada. N2: los dos síntomas calzan con E3B-11 (faltan sin aviso) y E3B-12 (error y no se
puede retomar). Para confirmar hace falta del usuario: versión del ejecutable, botón usado, mensaje literal y nombres relativos de los JSON.

## Correcciones E3C (19-nov-2026) — tercera revisión, resumen en `E3_INFORME_3_20261119.md`, veredicto NO PASA sobre `c31d5f2`
Cada hallazgo con prueba propia (`test_e3c_*` en `tests/`; Rust `e3c_*` en `fuentes_nuevas.rs`, `flow-dominio/src/evidencia.rs`,
`flow-captura/tests/lecturas.rs`, `flow-trabajo/tests/flujo.rs`). Las correcciones de captura se probaron por mutación: cada
reversión hace caer al menos una prueba Python. Requiere **cuarta revisión E3**.
| ID | Corrección |
|---|---|
| E3C-01 | Lectura de Escritos discordante: sin vínculo explícito (`escritos_para`), folios de TODAS las lecturas empatadas bloqueados (`evidencia_informe`, `_filas_empate`) y `Escritos.discordante` → `escrito_pendiente` devuelve None con marca `ESCRITOS_DISCORDANTES` (se conservan las acciones). Oráculo: INT-INF-D42, INT-CUM-D43. |
| E3C-02 | Firma de contenido = multiconjunto ordenado con solicitante; el orden de filas no cuenta. |
| E3C-03/04 | Titular: rótulo completo (`RUN del representante` no vale), valor tras «:», texto siguiente o celda hermana de la misma fila saltando comentarios; nunca una tabla. Python y Rust con los mismos fixtures. |
| E3C-05 | Constancia: NFC de toda la observación antes del verbo; partículas entre nombres («del Río», «de la Fuente»); coma pegada al verbo; sujeto de varias palabras sin nombre completo → no es constancia (se mantiene la frase). |
| E3C-06 | Anonimizador: literal de catálogo comparado en NFC, sin tildes ni mayúsculas; cualquier sufijo (también puntuación) se depura. |
| E3C-07 | Python: el respaldo elegido por el operador (`manual_word`) llega igual a elegir, filtrar y preparar (`prepare_projects(..., fallback_kind)`); grupo vacío → error legible, sin `IndexError`. Rust usa un único respaldo fijo (PC_IE) en ambos pasos. |
| E3C-08 | Dominio de fechas 1900–2199: el contrato Python rechaza con mensaje propio; Rust `validar_alcance` igual; `instante`/`_instante` comparan como texto fuera del dominio (sin OverflowError). |
| E3C-09 | «sin vínculo comprobado» cuenta como pendiente reintentable (`recoleccion::PENDIENTE`): Retomar vuelve a consultar SITFA. Un ingreso verificado sin lectura ni aviso dice que requiere paquete nuevo. |
| E3C-10 | Bitácoras leídas con indicio de recorte (`alcance_revision_fuente.coverage = PARCIAL`) se cuentan por tribunal (`Producto.recortadas`) y dejan PARCIAL; `aviso_incompletas` solo omite `bitacoras`/`consultas` PARCIAL. |
| E3C-11 | `mensaje_fallo` lee el control: solo sugiere Retomar si es legible y retomable; dañado o COMPLETO → lo dice y pide paquete nuevo. |
| E3C-12 | Pruebas desde el ZIP hasta acciones para orden UTC, contenido, identidad incierta, dos dueños y discordancia. |
| Ruff | Los 24 avisos nuevos contra `d2935dd` corregidos (noqa innecesarios, imports, variable sin uso); sin avisos nuevos en el plan. |

## Correcciones E3D (19-nov-2026) — cuarta revisión NO independiente, `E3_INFORME_4_20261119.md`
El revisor externo quedó a medias; a pedido del usuario la terminó la sesión implementadora (no cumple la decisión j).
| ID | Corrección |
|---|---|
| E3D-01/02 | Discordancia entre TODA lectura útil del mismo instante (`_compatibles`): parciales iguales entre sí; parcial contenida en la completa; completas idénticas. Python y Rust. |
| E3D-03 | Prueba de que solo cambiar el solicitante es discordancia. |
| E3D-04 | `preparacion_desde_interfaz` (sin Tk) lleva el respaldo a selección y Word; `app_base` la usa. |
| E3D-05 | `proceso.json` con `historial` anónimo (fases, decisión por lote, error literal, resumen final). |

## Pendientes (en orden de valor) — para quien continúe
1. **Asociación explícita informe–escrito**: la fila de Escritos no trae folio/fecha/`Doc.` textual. Para decidir si el icono `Doc.` lleva un
   identificador estable, correr `python descargador/extraer_estructura_escritos.py ESCRITOS.html [HISTORIAL.html]` sobre el HTML real y entregar
   **solo la salida** (anónima: nombres de atributos, función del onclick y tipo de argumentos, sin valores). Con eso se podrá poblar `linked_*`.
2. **Histórico — uso hoy**: sin botón de captura en las interfaces. Camino sin interfaz: guardar la ventana como HTML y ejecutar
   `python descargador/historico_manual.py --carpeta LOTE --html vista.html --tribunal COD --person-id ID --rut RUT [--url DIRECCION]`
   o `flow historico VISTA.html --carpeta LOTE --tribunal COD --persona ID --rut RUT [--url DIRECCION]`; luego crear el paquete con la carpeta
   (`export_capture(historico=LOTE)` / `exportar::Opciones.historico`). Falta el control de captura dentro de las ventanas.
3. **Consignación — uso hoy**: Rust tiene botón en el detalle de Trabajo («Marcar «X» como consignada»), `flow consignar PAQUETE --fila ID
   --programa P --archivo consignaciones.json` y `--consignaciones` en `observar/producir`. Python: `python tools/consignar.py SESION --listar |
   --fila ID --programa P`. **Falta el botón en la interfaz Tk** (la interfaz Python no muestra aún las marcas `avisos`).
4. **Escenario 9 incompleto**: selección oculta y reintento tras cancelación (interfaz/cola).
5. **Revisión E3 independiente** sobre la punta actual (tres NO PASA y una cuarta terminada por la sesión implementadora; ver
   «Correcciones E3», «E3B», «E3C» y «E3D»). Nada está «aceptado». Idealmente con Rust instalado (la tercera no pudo ejecutar `cargo`). Mulchén: el usuario no conserva versión, botón,
   mensaje ni nombres de JSON; la próxima vez que ocurra, anotar esos cuatro datos. Prompt: `PROMPT_REVISION_E3_4.md`.
6. CI Windows/SITFA; `tools/verificar_wp.py` (falla por entorno); escenarios Chromium ficticios y `rust.yml` para las fuentes nuevas.

## Comandos de verificación (desde la raíz)
```bash
python -m pytest -q -p no:cacheprovider tests --continue-on-collection-errors   # 55 fallos preexistentes por tkinter; no deben crecer
cd rust && cargo test --workspace --locked                                       # 349 pruebas
python rust/oraculo/generar_integracion.py && python rust/oraculo/generar_paquete_integracion.py   # regenerar oráculos (Python manda)
python rust/oraculo/generar_datos.py                                             # textos/umbrales Python → Rust
```
Descargador (necesita tkinter real; aquí se usó un stub): `cd descargador && python -m pytest -q test_paquete_escritos.py test_paquete_historia.py`.
