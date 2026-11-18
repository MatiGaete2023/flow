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
- `python tools/verificar_wp.py` → FALLA por «Restricciones absolutas» y «Ruff» (medición no disponible en el contenedor); preexistente.
- Suite del Descargador: sus pruebas de interfaz necesitan `tkinter` y fallan igual aquí; las de lectura/exportación/paquete pasan
  (se corrieron con un stub de `tkinter` en el scratchpad, solo para importar `descargador.py`).

## Estado actual (resumen)
| Paso | Estado | Dónde |
|---|---|---|
| 1 Base y línea base | HECHO | este registro |
| 2 Dato de captura al motor (contrato aditivo) | HECHO en Python y Rust (lector); productor Python solo Escritos | `src/nurus/capture/contract.py` (1.2.0), `capture/escritos.py`, `rust/crates/flow-trabajo/src/{fuentes_nuevas,paquete}.rs` |
| 3 Escritos en el detalle de causa | HECHO (lector HTML + Descargador Python); **extensión JS/Rust captura: PENDIENTE** | `src/nurus/escritos_html.py`, `src/nurus/capture/historias.py`, `descargador/export_csmp_capture.py::_escritos` |
| 4 Ingreso Histórico Protección | PARCIAL: lector HTML, tabla del contrato, motor. **Control de captura manual en el Descargador: PENDIENTE** | `escritos_html.read_historico`, tabla `historico_proteccion` |
| 5 «Solo una vez» + primer nombre | HECHO (bitácora + marca local idempotente `Work.consignar`); **botón en la interfaz: PENDIENTE** | `motor/evidencia.py`, `Work.consignar` |
| 6 Conservar cada vencimiento / fuentes | PARCIAL: asociación explícita por fecha proyectada; el agrupador `acumulado` sigue contando todas las fechas (decisión abajo) | `capture/escritos.py::evidencia_informe` |
| 7 Motor y composición | HECHO Python y Rust (oráculo 929 casos + paquete) | `motor/reglas_*.py`, `flow-motor/src/{hist,reglas,ctx,lib}.rs` |
| 8 Correo/Word/Excel | HECHO por acciones explícitas + filtro de proyectos Word (Python y Rust); Excel/nómina heredan `actions` | `personal/resolutions.py`, `flow-productos/src/word.rs` |
| 9 Matriz y paridad | PARCIAL: ver `MATRIZ_PARIDAD.md` (8 de 10 escenarios cubiertos) | `tests/test_integracion*.py`, `rust/casos/integracion.json` |
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

## Decisiones tomadas (N2: las tomé yo, el usuario no las ratificó; revisar)
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

## Pendientes (en orden de valor) — para quien continúe
1. **Productor real de la asociación explícita** (N3: no hay muestra poblada de Escritos con `Doc.` ni del calendario por proyección; los HAR
   traen Escritos con 0 filas). Definir con una muestra anónima `extraer_estructura_*` cómo enlazar `Doc.` con Historia/calendario antes de poblar `linked_*`.
2. **Extensión y captura Rust**: `descargador/extension/pagina.js` y `rust/extension/pagina.js`, `flow-captura` (`lecturas.rs`, `recoleccion.rs`,
   `vinculos.rs`): leer Escritos del detalle (hoy solo Python lo hace desde el HTML respaldado) y el control manual del Histórico de Protección
   (sin consultas autónomas por RUT). Añadir escenarios Chromium ficticios (`rust/pruebas/*.cjs`) y el workflow `rust.yml`.
3. **Interfaz**: botón «marcar constancia como consignada» (`Work.consignar`), visualización de las marcas nuevas, guía visual.
4. **Escenarios sin cubrir** (matriz): dos programas/NNA en una causa y dos ingresos sucesivos al mismo programa con evidencia; cambio de corte,
   selección oculta, reintento tras cancelación y producto ya editado; caso de estructura real (16 vencimientos/10 ingresos) con datos ficticios.
5. Paquete **Rust→lector Python** y lote antiguo en ambos sentidos con las tablas nuevas (Rust aún no *escribe* las tablas nuevas).
6. Anonimizador (`personal/anonymize_package.py`): decidir cómo tratar `requester`/`program_name`/`doc` de las tablas nuevas.
7. E3 (revisión por otro agente) exigida por `AGENTS.md`/decisión j: tocan `motor/`, `textos_base.json`, `contracts/` y el contrato. Nada está «aceptado».
8. CI Windows/SITFA y `cargo build --release --locked -p flow`; `tools/verificar_wp.py` (hoy falla por entorno).

## Comandos de verificación (desde la raíz)
```bash
python -m pytest -q -p no:cacheprovider tests --continue-on-collection-errors   # 55 fallos preexistentes por tkinter; no deben crecer
cd rust && cargo test --workspace --locked                                       # 316 pruebas
python rust/oraculo/generar_integracion.py && python rust/oraculo/generar_paquete_integracion.py   # regenerar oráculos (Python manda)
python rust/oraculo/generar_datos.py                                             # textos/umbrales Python → Rust
```
Descargador (necesita tkinter real; aquí se usó un stub): `cd descargador && python -m pytest -q test_paquete_escritos.py test_paquete_historia.py`.
