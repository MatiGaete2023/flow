# Prompt de segunda revisión E3 — integración 18-nov-2026 + diagnóstico de bitácoras de Mulchén

Pegar completo en una sesión o modelo **distinto** del que implementó (decisión j de `PLAN_VIGENTE.md`).

---

## ROL

Eres revisor E3 independiente del repo `MatiGaete2023/flow`. Revisas y diagnosticas; **no implementas** en la rama de trabajo:
reportas hallazgos con evidencia (archivo:línea, entrada ficticia, salida obtenida vs esperada) y, a lo sumo, propones un parche y
una prueba que falle en el informe. No marques nada como «aceptado» ni «institucional» (solo el usuario, en un acta). Español de Chile,
directo; separa hechos (N1), inferencias con supuesto (N2) y especulación (N3).

## QUÉ REVISAS

1. **Verificación de las correcciones de la primera E3** (`docs/integracion_20261118/E3_INFORME_20261119.md`, veredicto NO PASA
   sobre `c83414d`). La tabla «Correcciones E3» de `docs/integracion_20261118/REGISTRO.md` dice cómo se corrigió cada E3-01…E3-16.
   Para cada una: ¿quedó corregida en **Python y Rust**?, ¿la prueba `test_e3_*` / caso `INT-*-D30…D41` / prueba Rust la ejercita de
   verdad (falla si se revierte la corrección)?, ¿introdujo una regresión?
2. **Revisión E3 completa** del diff `origin/feat/flow-estudio-d-20261117...origin/claude/wizardly-volta-vl2lqg` (base `d2935dd`),
   con el mismo alcance de la primera: reglas y textos, contrato y captura, paridad Rust, productos y pruebas (áreas A–E abajo).
3. **Diagnóstico nuevo (área F)**: en los últimos prototipos, con el **Juzgado de Letras y Garantía de Mulchén**, la lectura de
   bitácoras no se produce o queda incompleta y no deja terminar la descarga del paquete; y al **retomar** o **crear** un paquete el
   programa dice que **no encuentra el archivo .json** y no lo hace.

Lee primero: `AGENTS.md`, `PLAN_VIGENTE.md`, `docs/integracion_20261118/REGISTRO.md`, `E3_INFORME_20261119.md`,
`01_Plan_integracion_FLOW_Rust_Python.md`, `02_Guia_de_consulta_RUS_SITFA.md`, `MATRIZ_PARIDAD.md`, `GUIA_VERIFICACION_INTERFAZ.md`.

## PROHIBICIONES

Sin `--force`, sin merge a `main`, sin releases, sin PR hacia `feat/**`. **Sin HAR, HTML, PDF, Excel, ZIP ni capturas reales**: solo
fixtures ficticios; si necesitas evidencia real, pide al usuario el extracto anónimo (herramientas listadas en F). Sin credenciales,
cookies ni tokens. Outlook solo `Save()`, nunca `.Send(`; CC fijo `ucc_concepcion@pjud.cl`. Sin escritura en RUS/SATURNO.

## PREPARACIÓN

```
git fetch origin claude/wizardly-volta-vl2lqg feat/flow-estudio-d-20261117
git checkout -B e3/integracion-2-$(date +%Y%m%d) origin/claude/wizardly-volta-vl2lqg
pip install pandas openpyxl python-docx lxml docxcompose customtkinter icalendar pytest jsonschema reportlab Pillow ruff
export PYTHONPATH=$PWD/src
git worktree add /tmp/base d2935dd
git worktree add /tmp/e3uno c83414d        # punta revisada en la primera E3
```
Anota el SHA exacto que revisas. Sin `tkinter` hay 55 fallos/errores preexistentes en `tests/` (lista en
`docs/integracion_20261118/base_python_fallos.txt`); compáralos contra `/tmp/base`. En Windows con Tk los números cambian: repórtalos
y compara conjuntos de identificadores, no totales.

## CONTROLES MECÁNICOS (repite y reporta números)

1. `python -m pytest -q -p no:cacheprovider tests --continue-on-collection-errors` (el flag es obligatorio): el conjunto de fallos debe
   ser idéntico al de la base. Suite del Descargador desde `descargador/` (con Tk si lo tienes).
2. `cd rust && cargo test --workspace --locked` (esperado 330, 0 fallos) y `cargo build --release --locked -p flow`. Si no tienes Rust,
   dilo y dictamina Rust solo por lectura de código.
3. `ruff check --select E9,F63,F7,F82,F401,F841 src descargador tools tests rust/oraculo`.
4. Oráculos: `python rust/oraculo/generar_integracion.py`, `generar_paquete_integracion.py`, `generar_paquete_estructura.py`,
   `generar_datos.py`, `validar_paquete_lote.py`; luego `git status`. Esperado: **sin cambios** también en Windows (E3-15: LF y ZIP con
   sistema creador fijo). Cualquier diferencia es hallazgo; distingue contenido de bytes.
5. Golden contra la base: el único cambio funcional esperado en observaciones existentes es `INFORME_INGRESADO` →
   `INFORME_REVISAR_ASOCIACION` en Informes vencidos con ingreso atribuido. Ejecuta el motor nuevo sobre los casos del oráculo de la base
   (Espera, Cumplimiento, Informes) y reporta diferencias de **acciones** (esperado: cero).
6. `grep -rn "\.Send(" src rust` vacío; `python tools/check_real_ids.py`; `python tools/check_links.py`.

## REVISIÓN DE FONDO

**A. Reglas y textos (Python referencia)** — `motor/evidencia.py`, `motor/contexto.py`, `motor/reglas_historia.py`, `reglas_*.py`,
`textos_base.json`, `work.py`, `resolutions.py`. Prioridad: **el error caro es retirar un pide cuenta que sí corresponde**.
Intenta romperlo con: vínculos `AJENO`/`EXPLICITO` mezclados, bandejas `PARCIAL`/`IDENTIDAD_INCIERTA`/`VACIA_COMPROBADA`, lecturas
posteriores al corte, varias lecturas de la misma causa (empates y discordancias), `folios_ajenos`, evidencia explícita de Historia vs
pendiente del mismo informe, frase desactivada, sujeto de la constancia («X informa que Y cuenta con…», nombres compuestos, comas),
dos niños / ingresos sucesivos en una causa.

**B. Contrato, captura y anonimización** — `capture/contract.py` (validación de alcance de filas nuevas, 1.0/1.1/1.2),
`capture/escritos.py` (`_mejores_consultas` al corte, `evidencia_informe`), `capture_input.py`, `record_edits.py`, `escritos_html.py`
(titular sin tablas de programas, pestaña sin tabla), `anonymize_package.py` (catálogos, `court_code` en filtros),
`descargador/export_csmp_capture.py` (PARCIAL con filas cortas), `historico_manual.py`. ¿Una variante plausible de la estructura real
(ver «Hallazgos de las capturas de SITFA» en REGISTRO) rompe los lectores? ¿Algún dato real o token puede filtrarse?

**C. Paridad Rust** — `flow-dominio/evidencia.rs`, `flow-motor/{hist,reglas,lib}.rs`, `flow-captura/lecturas.rs`,
`flow-trabajo/{fuentes_nuevas,exportar,paquete,historico_manual,lib}.rs`, `flow-productos/word.rs`, `flow/{cli,ui}.rs`. Busca
divergencias que el oráculo no cubre (selección al corte por texto de `captured_at`, normalización, regex, orden de marcas).
Nota: Python **rechaza** un paquete con filas fuera de alcance al leerlo; Rust también (`validar_alcance`): confirma igualdad.

**D. Productos** — `_tipos_efectivos`/`_fuera_del_proyecto` (Python) y `preparar` (Rust): decisiones humanas (`none`, un tipo, RES
marcado sin tipo), casos con y sin evidencia, correo/nómina/Excel.

**E. Pruebas** — ¿las pruebas nuevas fallan al revertir su corrección? (revierte localmente y prueba). Pruebas que dependan de
archivos regenerados o del orden del sistema de archivos.

**F. Diagnóstico: bitácoras de Mulchén y «no encuentra el archivo .json»** (síntoma reportado por el usuario; N1 = su reporte, no
reproducido por el implementador).
- **Primero** pregunta al usuario (o anota como pendiente si no responde): qué aplicación y versión (Descargador Python «SITFA
  Descargador» o Flow Rust), el **texto exacto** del mensaje y en qué botón aparece («Consultar y crear paquete», «Retomar», «Crear
  paquete de este lote»), y pide **solo extractos anónimos**: `diagnostico_bitacoras.json` del lote fallido, la salida de
  `python descargador/extraer_estructura_bitacora.py BITACORA_MULCHEN.html > estructura.json` y la lista de nombres de **archivos
  `.json`** de la carpeta «Paquete CSMP …» (sin abrirlos ni copiar HTML/Excel).
- **Mapea cada mensaje de «.json» a su origen** y a qué archivo espera: `respaldo/proceso.json` (`descargador/flujo_paquete.py` y
  `descargador.py:1327`; Rust `flow-trabajo/src/flujo.rs::leer`), `resumen.json` (`descargador/lotes.py`, Rust `flow-captura/lotes.rs`),
  `bitacoras.json` (`descargador/lectura_bitacoras.py`, `descargador.py:1438`, `:1675`), `historias.json` (`flujo_paquete.py:96`, `:126`),
  `flujo.json` (`descargador.py:1502`). Para cada uno: ¿quién lo crea, cuándo se escribe por primera vez, qué pasa si el proceso se corta
  antes, y qué carpeta elige «Retomar» (p. ej. `flujo_paquete.py:205` usa `glob('Paquete CSMP */respaldo/proceso.json')`)?
- **Reproduce con datos ficticios** (SITFA ficticio de `rust/pruebas/*.cjs`, `descargador/formulario_ficticio.py`, fixtures de
  `descargador/fixtures/`, `test_lectura_bitacoras.py`, `test_flujo_paquete.py`, `rust/crates/flow-trabajo/tests/flujo.rs`) al menos:
  1. Tribunal Mulchén con sus variantes de nombre (`Jgdo. L. y G. de Mulchén`, `Juzgado de Letras y Gar. de Mulchén`, `MULCHEN`, con y sin
     tilde, NFC/NFD) en catálogo, listado, ventana de bitácora y comparación de identidad/tribunal visible. ¿Alguna lectura se rechaza por
     identidad o se clasifica en otra causa?
  2. Bitácoras con paginador, marcos, tabla vacía, respuesta cortada o lenta (timeouts) y el detector de recorte (`PARCIAL`): ¿una sola
     bitácora parcial o fallida impide **terminar** el paquete en vez de crearlo con la fuente PARCIAL y un aviso?
  3. Corte del proceso (cancelación, cierre, error) en cada fase de «Consultar y crear paquete»; luego «Retomar» y «Crear paquete de este
     lote». ¿Queda alguna carpeta sin su `.json` de control? ¿«Retomar» elige otra carpeta (la más reciente) que no lo tiene?
  4. Rutas Windows: longitud total (`MAX_PATH` 260) con nombres de tribunal/modalidad (`lotes.py::safe_name`, `tribunal_name`), carpetas en
     OneDrive o con caracteres no ASCII, y destino movido entre ejecución y reanudación.
  5. Tope de lecturas (`MAX_LECTURAS_POR_LOTE`), reanudación con `bitacoras.json` de otro alcance («El alcance no coincide…») y con
     lotes de Mulchén mezclados con Laja/Tomé.
- **Entrega**: causa raíz con evidencia (N1) o, si no se reproduce, las hipótesis ordenadas por probabilidad (N2) con la prueba ficticia
  que la confirmaría y qué extracto anónimo falta; severidad (bloquea la entrega = crítica); parche propuesto mínimo (Python y Rust si
  aplica) y prueba que falle hoy. Mensajes de error: deben decir qué archivo falta, en qué carpeta y qué hacer.

## FORMATO DEL INFORME

`docs/integracion_20261118/E3_INFORME_2_<fecha>.md` en tu rama `e3/…` (no en la de trabajo). Contenido:
1. SHA revisado, entorno (SO, Python, Rust sí/no, Tk sí/no).
2. **Tabla E3-01…E3-16**: `ID | corregido Python | corregido Rust | prueba que lo cubre (¿falla al revertir?) | observación`.
3. Veredicto por área A–F: **PASA / PASA CON OBSERVACIONES / NO PASA** (nunca «aceptado»).
4. Hallazgos nuevos: `ID (E3B-nn) | severidad (crítica: retira un pide cuenta que corresponde, afirma un hecho falso, filtra datos o
   bloquea la entrega; alta; media; baja) | archivo:línea | entrada → obtenido vs esperado | parche propuesto`.
5. Área F: síntomas, mapa de mensajes `.json`, reproducciones (pasos y resultado), causa raíz o hipótesis, extractos que faltan del
   usuario, parche y prueba propuestos.
6. Controles mecánicos 1–6 con números; casos de regresión ficticios propuestos; qué no pudiste verificar (Tk/egui, Windows, SITFA real).
