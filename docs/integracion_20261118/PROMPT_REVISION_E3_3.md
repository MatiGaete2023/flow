# Prompt de tercera revisión E3 — integración 18-nov-2026 (correcciones E3B y bitácoras de Mulchén)

Pegar completo en una sesión o modelo **distinto** del que implementó (decisión j de `PLAN_VIGENTE.md`).

---

## ROL

Eres revisor E3 independiente del repo `MatiGaete2023/flow`. Revisas; **no implementas** en la rama de trabajo: reportas hallazgos con
evidencia (archivo:línea, entrada ficticia, obtenido vs esperado) y, a lo sumo, propones parche y prueba que falle hoy. No marques nada
como «aceptado» ni «institucional» (solo el usuario, en un acta). Español de Chile, directo; separa hechos (N1), inferencias con supuesto
(N2) y especulación (N3).

## QUÉ REVISAS (punta `aa0d531` de `claude/wizardly-volta-vl2lqg` o posterior; anota el SHA exacto)

1. **Correcciones de la segunda E3** (resumen en `docs/integracion_20261118/E3_INFORME_2_20261119.md`, NO PASA). La tabla «Correcciones
   E3B» de `REGISTRO.md` dice cómo se corrigió cada E3B-01…E3B-14. Para cada una: ¿corregida en **Python y Rust** (cuando aplica)?, ¿la
   prueba `test_e3b_*` / `e3b_*` falla al revertir la corrección?, ¿introdujo regresión?
2. **No regresión de E3-01…E3-16** (tabla «Correcciones E3» del mismo registro).
3. **Revisión E3 completa** del diff `origin/feat/flow-estudio-d-20261117...HEAD` (base `d2935dd`), áreas A–E.
4. **Área F (Mulchén)**: verificar si E3B-11/12/13 cubren los dos síntomas reportados y qué falta para confirmarlo con el usuario.

Lee primero: `AGENTS.md`, `PLAN_VIGENTE.md`, `docs/integracion_20261118/REGISTRO.md`, `E3_INFORME_20261119.md`,
`E3_INFORME_2_20261119.md`, `MATRIZ_PARIDAD.md`, `GUIA_VERIFICACION_INTERFAZ.md`.

## PROHIBICIONES

Sin `--force`, sin merge a `main`, sin releases, sin PR hacia `feat/**`. **Sin HAR, HTML, PDF, Excel, ZIP ni capturas reales**: solo
fixtures ficticios; evidencia real solo como extracto anónimo (`descargador/extraer_estructura_*.py`, `diagnostico_bitacoras.json`,
`tools/anonimizar_excel.py`). Sin credenciales, cookies ni tokens. Outlook solo `Save()`, nunca `.Send(`; CC fijo
`ucc_concepcion@pjud.cl`. Sin escritura en RUS/SATURNO.

## PREPARACIÓN

```
git fetch origin claude/wizardly-volta-vl2lqg feat/flow-estudio-d-20261117
git checkout -B e3/integracion-3-$(date +%Y%m%d) origin/claude/wizardly-volta-vl2lqg
pip install pandas openpyxl python-docx lxml docxcompose customtkinter icalendar pytest jsonschema reportlab Pillow ruff
export PYTHONPATH=$PWD/src
git worktree add /tmp/base d2935dd
git worktree add /tmp/e3dos 8e28335        # punta revisada en la segunda E3 (antes de las correcciones E3B)
```

## CONTROLES MECÁNICOS (reporta números)

1. `python -m pytest -q -p no:cacheprovider tests --continue-on-collection-errors` (flag obligatorio). Sin `tkinter`: conjunto idéntico a
   `docs/integracion_20261118/base_python_fallos.txt` (55). Con Tk: compara identificadores contra `/tmp/base`, no totales.
   Descargador desde `descargador/` (sin Tk real, `test_lotes.py::InterfaceTests` falla por entorno; dilo).
2. `cd rust && cargo test --workspace --locked` (esperado **340**, 0 fallos) y `cargo build --release --locked -p flow`. Clippy/rustfmt
   tienen fallas preexistentes de la base: reporta solo las nuevas.
3. `ruff check src descargador tools tests rust/oraculo` comparado con `/tmp/base`: ningún aviso nuevo en archivos tocados por el plan.
4. Oráculos: `python rust/oraculo/{generar_integracion,generar_paquete_integracion,generar_paquete_estructura,generar_datos,validar_paquete_lote}.py`
   y `git status`: **sin cambios**, también en Windows (LF, ZIP con sistema creador fijo). Distingue contenido de bytes.
5. Golden contra la base: único cambio funcional esperado en observaciones existentes `INFORME_INGRESADO` → `INFORME_REVISAR_ASOCIACION`
   (Informes vencidos con ingreso atribuido); diferencias de **acciones**: cero.
6. `grep -rn "\.Send(" src rust` vacío; `python tools/check_real_ids.py`; `python tools/check_links.py`.

## REVISIÓN DE FONDO

**A. Reglas y textos** (`motor/evidencia.py`, `contexto.py`, `reglas_historia.py`, `reglas_*.py`, `textos_base.json`, `resolutions.py`).
El error caro es **retirar un pide cuenta que sí corresponde**. Ataca E3B-07 (sujeto): nombres compuestos, NFD, mayúsculas, dos niños con
igual primer nombre, «X informa que Y…», comas, apellidos antes del verbo, nombre completo vacío.

**B. Contrato, captura y anonimización** (`capture/escritos.py`, `escritos_html.py`, `anonymize_package.py`, `export_csmp_capture.py`).
E3B-01 (UTC y día local del corte: `Z`, offsets distintos, sin zona, solo fecha, microsegundos), E3B-02 (empates por contenido en
cualquier orden del manifiesto; histórico discordante → INCIERTA), E3B-03/04 (identidad incierta y referencia con varios dueños), E3B-05
(titular solo rotulado: rótulo y valor en celdas/elementos hermanos, `RUN`, «Nombre del NNA», valores en tablas de programas), E3B-06
(literal canónico; variantes con espacios, tildes, sufijos libres).

**C. Paridad Rust** (`fuentes_nuevas.rs`, `flow-dominio/evidencia.rs`, `flow-motor/{hist,reglas}.rs`, `flow-captura/lecturas.rs`,
`flow-productos/word.rs`). Busca entradas donde Python y Rust difieran y el oráculo no cubra: parseo de `captured_at` (Python
`fromisoformat` vs chrono), orden por `query_id`, firmas de filas con `clave_texto`, `\p{L}`/`\p{Nd}` vs `\w`, NFC.

**D. Productos** (E3B-08/09): `_tipos_efectivos` / `word.rs::tipos_efectivos` con `none`, un tipo, RES marcado sin tipo, con y sin
evidencia; ¿elegir y preparar usan exactamente el mismo cálculo? Correo/nómina/Excel.

**E. Pruebas**: revierte localmente cada corrección E3B y confirma que su prueba falla. Pruebas dependientes del orden del sistema de
archivos, de la hora local o de archivos regenerados.

**F. Flujo de paquete Rust y Mulchén (E3B-11/12/13)** — `flow-captura/src/{recoleccion,lotes}.rs`, `flow-trabajo/src/{flujo,exportar}.rs`,
`flow/src/captura_ui.rs`.
- **E3B-11**: `recoleccion::bitacoras_faltantes`/`avisos_bitacoras_faltantes`, `Producto.sin_bitacora`, `flujo::avisos_de_bitacoras`.
  ¿Falsos positivos que dejen PARCIAL un paquete correcto (registros verificados de pestañas sin bitácora aplicable, varias etapas del
  mismo ingreso, lotes sin bitácoras pedidas, solo calendario)? ¿Falsos negativos (bitácora leída de otro ingreso que se excluye por
  identidad)? ¿El aviso por tribunal es legible con el nombre del catálogo? ¿Se muestra igual en los tres puntos (flujo, «Crear paquete de
  este lote», fin de lote)?
- **E3B-12**: `recoleccion::reparar_lecturas` (llamada en `lotes::ejecutar` al retomar) y la comprobación de integridad en
  `flujo::fase_lote`. Prueba: lectura borrada, alterada, renombrada; de tipo historia (clave por causa, varias filas); sin fila ubicable en
  `paginas_lectura` (aviso `Lectura descartada:` permanente: ¿deja el paquete PARCIAL para siempre y es razonable?); falla de la relectura
  en SITFA; lecturas reutilizadas de lotes hermanos o de otros «Paquete CSMP». ¿Algún camino aún aborta con «Falta la lectura verificada…»
  sin opción de seguir? ¿Se inventa algún JSON?
- **E3B-13**: `flujo::ejecutar_en` y `mensaje_fallo`. Dos carpetas bajo el mismo destino (antigua con control, nueva sin control): el
  mensaje debe nombrar la nueva y nunca recomendar la antigua. Falla al crear `respaldo/`.
- **Mulchén** (N1 = reporte del usuario, no reproducido): pide al usuario, o déjalo como pendiente, versión del ejecutable, botón usado,
  **mensaje literal** y lista de nombres relativos de los `.json` de la carpeta «Paquete CSMP …» (sin abrirlos). Con eso, dictamina si los
  síntomas («faltan bitácoras sin aviso» y «error y no se puede retomar / no se generó el JSON») quedan cubiertos por E3B-11/12/13 o hay
  otra causa: variantes del nombre del tribunal (NFC/NFD, abreviaturas), rutas Windows largas (`MAX_PATH`), OneDrive, tope `TOPE`/
  `MAX_FALLAS_SEGUIDAS` cortando el lote. Reproduce con `rust/crates/flow-trabajo/tests/flujo.rs` y los fixtures ficticios.

## FORMATO DEL INFORME

`docs/integracion_20261118/E3_INFORME_3_<fecha>.md` en tu rama `e3/…` (no en la de trabajo):
1. SHA revisado y entorno (SO, Python, Rust sí/no, Tk sí/no).
2. **Tabla E3B-01…E3B-14**: `ID | corregido Python | corregido Rust | prueba (¿falla al revertir?) | observación`; y una línea por E3-01…16
   solo si hay regresión.
3. Veredicto por área A–F: **PASA / PASA CON OBSERVACIONES / NO PASA** (nunca «aceptado»).
4. Hallazgos nuevos: `ID (E3C-nn) | severidad (crítica: retira un pide cuenta que corresponde, afirma un hecho falso, filtra datos o
   bloquea la entrega; alta; media; baja) | archivo:línea | entrada → obtenido vs esperado | parche propuesto`.
5. Área F: reproducciones (pasos y resultado), causa raíz o hipótesis ordenadas (N2) y qué dato del usuario falta.
6. Controles 1–6 con números; qué no pudiste verificar (Tk/egui, Windows, SITFA real).
