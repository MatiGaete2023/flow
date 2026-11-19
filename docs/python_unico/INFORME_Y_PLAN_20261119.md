# Flow Rust → herramientas Python: informe comparativo y plan (19-nov-2026)

> El plan vigente (reformulado con la guía de RUS/SITFA) está en [`PLAN.md`](PLAN.md); la sección 3 de este archivo queda como antecedente.

**Decisión del usuario (19-nov-2026):** «Ya no continuaré el proyecto en RUST, mejoraremos solo las herramientas en python».
Rust queda **congelado** como referencia (código, pruebas y oráculos se conservan; no se desarrolla más). Todo lo útil que
Rust tenga y Python no, se porta a Python y se expone con botones u opciones en la herramienta que corresponda.

Convención: **N1** = verificado en el código o los documentos del repo; **N2** = inferencia con su supuesto; **N3** = sin
medición. Fuentes: `rust/LEEME.md`, `rust/ESTUDIO_D_ESTADO.md`, `rust/REVISION_BITACORAS_ESQUEMA.md`, `rust/EJECUCION.md`,
`docs/integracion_20261118/REGISTRO.md`, código de `rust/crates/*` y de `src/nurus`, `descargador/`, `META4_Bitacoras/`,
`anonimizador/`.

---

## 1. Resumen ejecutivo

- **N1.** Rust fue un **port** de Python: cada módulo Rust cita su origen Python (`rust/EJECUCION.md` §2–8). Las reglas del
  motor son **las mismas** (paridad comprobada con 943 casos de oráculo). Rust **no tiene reglas propias**.
- **N1.** Lo que Rust tiene y Python no se concentra en: (a) **Revisión de bitácoras** (separar registros sin observación),
  (b) **botones** para funciones que en Python solo existen por consola o en el motor (consignar «otro programa»,
  capturar Ingreso Histórico), (c) un **rediseño de interfaz** (Estudio D: lista/ficha, biblioteca de resultados con
  búsqueda, etapas con conteos), (d) **consola** `observar`/`producir`, (e) **borradores `.eml`** cuando Outlook no está,
  (f) Meta 4 con **plantilla Excel elegible en la ventana**, (g) diagnósticos del flujo de paquete (historial anónimo,
  bitácoras faltantes por tribunal).
- **N1.** Varias correcciones recientes de Rust (E3B-11/12, E3C-09/10/11) arreglaban **errores propios de Rust**: el flujo
  Python ya cuenta lecturas fallidas y deja el paquete PARCIAL (`descargador/flujo_paquete.py:161-167`).
- **N1.** Una función de Rust **no se porta**: la captura dentro del Asistente. `AGENTS.md` lo prohíbe («El Asistente no
  navega SITFA ni inicia el Descargador»). En Python la captura sigue en el Descargador.
- **N3.** Velocidad: no hay mediciones comparables Rust/Python en el repo. Rust se distribuye como un `.exe` sin instalar
  Python; Python tiene su empaquetado portable (`portable/`). No se afirma que uno sea más rápido.

## 2. Inventario por herramienta

Formato de cada fila: **qué hace · cómo · producto** en Rust → **estado en Python** → **acción**.

### 2.1 Asistente (Trabajo → Correos → Resoluciones → Resultados → Contactos)

| Función | Rust (qué · cómo · producto) | Python | Acción |
|---|---|---|---|
| Análisis y observaciones | Lee Excel RUS o ZIP; mismo motor; Excel de observaciones | **Igual** (es la referencia) | Ninguna |
| Correos | Borradores en Outlook con recibo local antes/después; **o archivos `.eml`** si no hay Outlook; CC fijo | Outlook con recibos y estado incierto; **sin `.eml`** | P6 (opcional, E3: toca borradores) |
| Resoluciones | Proyectos Word por tribunal/RIT/tipo; un `.docx` con todos | Igual; respaldo de tipo corregido en E3C-07/E3D-04 | Ninguna |
| Resultados | **Biblioteca** con búsqueda y carpetas previas (Estudio D P5) | Lista de resultados **sin búsqueda** | P4 |
| Contactos | Lista/ficha, importación con confirmación | Botón «Importar contactos» con conflictos (`app_base.py:1943`) | Verificar paridad en P4 |
| Detalle del registro | Cronología de bitácora, avisos CT/N/OB, proyecciones del calendario | Cronología WP-50 y avisos **presentes** | Ninguna |
| **Consignar «otro programa»** | Botón «Marcar «X» como consignada» en el detalle | Motor y `Work.consignar` **sí**; **sin botón** (solo `tools/consignar.py`) | **P1** |
| **Revisión de bitácoras** | Pantalla propia: clasifica cada registro en Con observación / **Sin observación** / No verificable / Revisada; el motor corre sobre el paquete completo y solo después se eligen las filas «Sin observación»; exporta XLSX + JSON atómicos (`revision_bitacoras_ui.rs`, `flow-trabajo/revision_*`, `flow-productos/revision.rs`) | **No existe** | **P2** (pedido del usuario) |
| Diseño Estudio D | Tema común, lista/ficha, panel activo, etapas con conteos, confirmación antes de recalcular, ayuda, escalado 200 % | Tema compartido WP-87 y UX-02/03; sin lista/ficha ni conteos por etapa | P5 (selectivo) |
| Consola | `flow observar` / `flow producir` (Excel, nóminas, `.eml`, Word) | No hay equivalente del Asistente | P7 (opcional) |

### 2.2 Descargador

| Función | Rust | Python | Acción |
|---|---|---|---|
| Consultas, bitácoras, historia, calendario por ingreso | Igual (port de `lotes.py`, `lectura_bitacoras.py`) | **Igual** | Ninguna |
| «Consultar y crear paquete», informes por vencer, Retomar | Igual (port de `flujo_paquete.py`) | **Igual** | Ninguna |
| **Escritos de la causa** | Se leen de la misma respuesta de Historia | Igual, pero **solo si la tabla viene en esa respuesta**; si la pestaña carga aparte, queda FALLIDA | **P3** (pedido del usuario; necesita estructura anónima; E3: extensión) |
| **Ingreso Histórico Protección** | `flow historico` (consola) + lector | `historico_manual.py` (consola), **sin botón** ni captura automática | **P3** (pedido del usuario; necesita estructura anónima; E3: extensión) |
| Evidencia de bitácora V1 (tabla reconocida, vacía comprobada, recorte por paginador) | `lecturas.rs::cobertura_bitacora`, `revision_bitacoras` en el lote | **No la registra** | **P2** (la necesita la Revisión) |
| Diagnóstico | Historial anónimo en `proceso.json` (E3D-05); aviso de bitácoras faltantes por tribunal | `diagnostico_bitacoras.json` y `diagnostico_historia.json` anónimos; versión corregida hoy (`99a2c11`) | P8 |

### 2.3 Meta 4

| Función | Rust | Python | Acción |
|---|---|---|---|
| Planilla Meta 4, consolidados, alertas, regenerar desde Registros revisado | Igual (port de `META4_Bitacoras/scripts`) | **Igual**, con «Regenerar desde Registros revisado…» | Ninguna |
| Plantilla Excel propia | Botón «Elegir plantilla Excel…» y `--plantilla` | `load_template(path)` existe en el código; **sin opción en la ventana** (N2: verificar) | P4 |
| Lotes de origen | Solo lotes de Flow Rust | Lotes del Descargador Python | Ninguna |

### 2.4 Anonimizador

| Función | Rust | Python | Acción |
|---|---|---|---|
| Formatos, modos estricto/conservador, semilla en memoria, desplazar fechas, generalizar nacimiento, rechazo completo | Port de `anonimizador_csmp` | **Igual** (es la referencia) | Ninguna |

### 2.5 Reglas

- **N1.** Mismas reglas en ambos (oráculo `rust/casos/*.json`). Las reglas nuevas del plan 18-nov (escrito pendiente,
  otro programa vigente, revisar asociación, discordancias E3B–E3D) están **en Python primero** y Rust las copia.
- Rust no agrega reglas. Nada que portar en el motor.

## 3. Plan para Python

Clase de cambio según la decisión j (`PLAN_VIGENTE.md`): **E3** = revisión independiente obligatoria (motor, textos, Outlook,
contrato, extensión SITFA, anonimizador); **V** = basta `tools/verificar_wp.py` y pruebas.

| Paquete | Contenido | Herramienta y botón | Bloqueo | Clase |
|---|---|---|---|---|
| **P1** | Botón «Marcar como consignada» para la frase «otro programa vigente» en el detalle de Trabajo; deshacer; la marca persiste y el recálculo la respeta | Asistente · Trabajo · detalle | Ninguno | V |
| **P2** | **Revisión de bitácoras en Python** (= «separar los registros sin observación»): (a) el Descargador registra la evidencia V1 de cada bitácora (tabla reconocida, vacía comprobada, recorte); (b) nueva pantalla del Asistente que clasifica Con / Sin observación / No verificable, filtra, permite marcar Revisada y exporta Excel + JSON; el motor corre sobre todo el paquete antes de separar | Descargador (sin botón nuevo) · Asistente: botón «Revisión de bitácoras…» | Ninguno | V (no toca extensión ni contrato: metadatos aditivos en `filters_json`) |
| **P3** | **Escritos** cuando la pestaña carga aparte y **Ingreso Histórico Protección** automático por persona; casillas «Leer escritos» y «Leer histórico» en Seguimiento | Descargador · opciones de lectura | Estructura anónima de ambas vistas (`extraer_estructura_escritos.py`, y una equivalente para el histórico) | **E3** (extensión) |
| **P4** | Resultados con búsqueda y carpetas previas; Meta 4 con «Elegir plantilla Excel…»; paridad de importación de contactos | Asistente · Resultados; Meta 4 | Ninguno | V |
| **P5** | Mejoras de diseño Estudio D que valgan en Tk: conteos por etapa, confirmación antes de recalcular, lista/ficha en Correos y Resoluciones, ayuda contextual | Asistente | Ninguno | V |
| **P6** | Guardar borradores como `.eml` cuando Outlook no está (CC fijo, sin envío) | Asistente · Correos | Decisión del usuario | **E3** (borradores) |
| **P7** | Consola `observar` / `producir` del Asistente | `python -m nurus.personal …` | Decisión del usuario | V |
| **P8** | Historial anónimo en `respaldo/proceso.json` y aviso final de bitácoras faltantes por tribunal en el Descargador | Descargador | Ninguno | V |

**Orden propuesto:** P1 → P2 → P8 → P4 → P3 (cuando lleguen las estructuras) → P5 → P6/P7 (si el usuario los quiere).

**Criterio de terminado de cada paquete:** pruebas nuevas que fallan sin el cambio; suite Python sin fallos nuevos
(`base_python_fallos.txt` en este entorno); `ruff` sin avisos nuevos; botón visible y documentado en el manual de la
herramienta; REGISTRO actualizado. Nada se marca «aceptado»: la puerta sigue siendo el acta del usuario.

## 4. Qué pasa con Rust

- Se conserva en `rust/` sin desarrollo nuevo. Sus pruebas, oráculos y documentos sirven de referencia para portar P2.
- Los oráculos Python→Rust (`rust/oraculo/*`) dejan de ser obligatorios en cada cambio; se mantienen solo si un cambio los
  rompe y se decide actualizarlos.
- El disparador de CI agregado hoy para esta rama (`f8784f3`) puede quedar: solo corre si cambia `rust/**`.
- Borrar `rust/` es decisión del usuario; no se hace.
