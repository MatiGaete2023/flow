# Plan del proyecto nuevo en Rust

12 de noviembre de 2026 · rama `plan/proyecto-rust` (desde `informe/migracion-rust`).
Antecedente: [informe de migración](../migracion_rust/INFORME_MIGRACION_RUST_20261112.md), secciones 11 y 12.
Este plan no cambia las herramientas Python: siguen en `Rediseno`, intactas, como referencia y oráculo opcional.

Convención: **N1** hecho medido · **N2** inferencia · **N3** especulación.

## 1. Marco

### Prohibiciones (decisión del usuario, 12 de noviembre de 2026)

1. Sin ingreso de datos en RUS ni SATURNO.
2. Sin envío de correos: solo borradores de Outlook.
3. Todo correo con CC a `ucc_concepcion@pjud.cl`.
4. Sin datos reales en repositorio, CI ni conversaciones con agentes: solo datos ficticios o estructura anónima.
5. Sin credenciales, cookies ni tokens en código, registros o fixtures.

Fuente de 1–3: «No se levantan lo siguiente: No se ingresan datos a rus y saturno, no se envian correos
electronicos, todos los correos con cc al ucc». Fuente de 4–5: «Mantenlo».

### Lo que ya no obliga

Motor y textos intocables, paridad exacta con Python, separación Asistente/Descargador, paquetes WP, revisión E3 y
acta de la versión Python como requisito previo.

### Forma del producto

Proyecto nuevo, **una sola aplicación**, motor rediseñado, Python como oráculo opcional (se compara y se explica
cada diferencia; no se exige igualdad).

### Ramas

- Todo cambio va en rama nueva. Las herramientas Python no se tocan.
- Este plan: `plan/proyecto-rust`. Cada fase: `rust/fase-N-nombre`, creada desde la anterior.
- Sin fusión a `main` ni a `Rediseno` sin autorización escrita del usuario.

## 2. Decisiones abiertas

Cada una trae un valor por defecto. Se avanza con el defecto salvo que el usuario diga otra cosa.

```json
[
  {"id": "R1", "pregunta": "¿Dónde vive el código?", "defecto": "carpeta rust/ de este repositorio, en ramas rust/*",
   "alternativa": "repositorio nuevo (hay que crearlo y darle acceso al agente)"},
  {"id": "R2", "pregunta": "¿Meta 4 y Anonimizador dentro de la misma aplicación?",
   "defecto": "sí: módulos de la misma aplicación, también usables por línea de comandos",
   "alternativa": "ejecutables separados que comparten el núcleo"},
  {"id": "R3", "pregunta": "¿Qué pasa con SITFA Lite?",
   "defecto": "se entrega como la misma aplicación con solo el módulo Captura visible",
   "nota": "N2: Lite parece tener otros usuarios (instalador, manuales y videos propios); confirmar"},
  {"id": "R4", "pregunta": "¿Biblioteca de interfaz?", "defecto": "se elige en la Fase 1 con una prueba (slint o egui)",
   "criterios": "tabla editable de cientos de filas, teclado completo, tildes y ñ, tema claro/oscuro, .exe único"},
  {"id": "R5", "pregunta": "¿Textos de observaciones?", "defecto": "se copian sin cambiar una letra desde textos_base.json",
   "nota": "rediseñar el motor no autoriza a redactar de nuevo: los textos fueron aprobados uno a uno"},
  {"id": "R6", "pregunta": "¿Umbrales y reglas?", "defecto": "los vigentes hoy (18 umbrales, reglas del plan vigente a–ab)",
   "nota": "cualquier cambio de regla es decisión del usuario, igual que antes"},
  {"id": "R7", "pregunta": "¿Cómo se acepta cada fase?", "defecto": "prueba del usuario en Windows con su guía y nota firmada",
   "alternativa": "sin acta formal: basta el visto bueno en la conversación"},
  {"id": "R8", "pregunta": "¿Puente con Chrome?", "defecto": "igual que hoy: HTTP en 127.0.0.1 con código secreto en RAM",
   "alternativa": "native messaging (sin puerto), evaluable después de la Fase 6"}
]
```

## 3. Arquitectura

Un workspace de Cargo. Un solo `.exe` para Windows; el núcleo compila y se prueba también en Linux.

```
rust/
  Cargo.toml  Cargo.lock
  crates/
    flow-tabular    lee .xls/.xlsx por contenido: BIFF, OOXML, XML 2003, HTML, CSV
    flow-html       lectores de bitácora, historia de la causa y calendario
    flow-dominio    tipos: Registro, Tribunal, Programa, Etapa, Fecha, Hallazgo, Lote
    flow-motor      reglas de Espera, Cumplimiento, Informes e Historia; textos; umbrales
    flow-excel      escritura de libros: observaciones, nóminas, consolidado, Meta 4
    flow-word       proyectos de resolución desde plantillas .docx
    flow-outlook    borradores (solo Windows; sin función de envío)
    flow-captura    puente local, protocolo con la extensión, lotes, checkpoints, PNG→PDF
    flow-anonimo    anonimizador
    flow-meta4      planilla Meta 4 y consolidados
    flow-almacen    trabajo local, sesiones, retención
    flow-ui         ventanas
    flow            ejecutable: ventana y subcomandos
  extension/        extensión Chrome MV3 (JavaScript, adaptada de la actual)
  especificacion/   casos y documentos de la Fase 0
```

Reglas de dependencia:

- `flow-motor` solo depende de `flow-dominio`. No lee archivos, no conoce Excel, no mira el reloj: la fecha de corte
  entra como parámetro. Así cada regla se prueba con un registro armado a mano.
- `flow-outlook` es el único crate que habla con Outlook. Expone `guardar_borrador` y nada más.
- `flow-captura` es el único que habla con la extensión. Solo tiene operaciones de lectura.
- Ningún crate escribe en la red salvo el puente en `127.0.0.1`.

### Motor rediseñado

- Entrada: `Registro` tipado (fechas como fechas, ausente distinto de ilegible), historia y bitácora opcionales,
  umbrales, fecha de corte.
- Salida: lista de `Hallazgo { regla, clase, nivel, texto, acciones }`.
  `clase` ∈ constancia · acción del Centro · gestión del Tribunal. `acciones` ∈ correo · proyecto (PC_IE, PC_INFO,
  NOMENCL).
- La composición (orden transversal, «un dato se dice una vez», «Asimismo», aclaración de causa al final) es una
  etapa aparte que recibe hallazgos y produce la observación final.
- Un dato ilegible produce un hallazgo de incidencia, nunca silencio.

### Prohibiciones convertidas en pruebas

```json
[
  {"prohibicion": "sin escritura RUS/SATURNO", "garantia": "no existe módulo de registro; la extensión declara escritura_rus:false y el programa rechaza otra cosa", "prueba": "CI falla si la extensión contiene un envío de formulario fuera de la lista de consultas permitidas"},
  {"prohibicion": "sin envío de correos", "garantia": "flow-outlook no tiene función de envío", "prueba": "CI falla si aparece el identificador Send en flow-outlook"},
  {"prohibicion": "CC fijo", "garantia": "constante única, no configurable", "prueba": "todo borrador de prueba lleva exactamente ese CC"},
  {"prohibicion": "sin datos reales", "garantia": "control de identidades sobre cada commit", "prueba": "hook y CI con check_real_ids y check_contactos (Fase 0: se reutilizan los de Python)"},
  {"prohibicion": "sin credenciales", "garantia": "el puente no recibe cookies; código de conexión solo en RAM", "prueba": "prueba del puente: ninguna respuesta ni registro contiene el código"}
]
```

## 4. Fases

Tamaños relativos S < M < L < XL. No hay semanas: no tengo datos para estimarlas.
Cada fase termina con: pruebas en verde, compilación Windows, y prueba del usuario (R7).

### Fase 0 — Especificación (M) · rama `rust/fase-0-especificacion`

No se escribe Rust. Se extrae del repo lo que el proyecto nuevo debe saber.

```json
[
  {"entregable": "REGLAS.md", "contenido": "cada regla: condición, umbral, texto, clase, acciones, etapa, decisión que la originó",
   "fuentes": ["src/nurus/personal/motor/", "PLAN_VIGENTE.md decisiones q–ab", "tests/test_reglas_*.py"]},
  {"entregable": "textos.json y umbrales.json", "contenido": "61 textos y 18 umbrales, copiados literalmente",
   "fuentes": ["src/nurus/personal/textos_base.json", "src/nurus/personal/config.py"]},
  {"entregable": "ENTRADAS.md", "contenido": "las 4 variantes de .xls, columnas y sus alias, HTML de bitácora, historia y calendario",
   "fuentes": ["src/nurus/rus/columns.py", "descargador/motor.py", "src/nurus/*_html.py", "descargador/extraer_estructura_*.py"]},
  {"entregable": "CAPTURA.md", "contenido": "5 pantallas, 4 pestañas, paginación, protocolo del puente, ritmo, límite de 600 lecturas",
   "fuentes": ["descargador/perfiles.py", "descargador/puente.py", "descargador/extension/", "descargador/lotes.py"]},
  {"entregable": "SALIDAS.md", "contenido": "cada libro Excel, las 6 plantillas Word y sus variables, borradores, ICS, PDF, Meta 4",
   "fuentes": ["src/nurus/personal/outputs.py", "src/nurus/personal/resolutions.py", "src/nurus/services/exports.py", "META4_Bitacoras/LEEME.md"]},
  {"entregable": "casos/*.json", "contenido": "casos de prueba neutros: registro ficticio + fecha de corte → observación esperada",
   "fuentes": ["tests/ (932 pruebas del Asistente)", "docs/evidencia_p0/golden/semantic.json", "descargador/fixtures/"]},
  {"entregable": "FUERA.md", "contenido": "lo que NO se lleva: capa de compatibilidad, textos legados, reconciliación de paquetes, retiros ya decididos",
   "fuentes": ["src/nurus/personal/config.py (LEGACY_*)", "src/nurus/capture/"]}
]
```

Término: el usuario lee `REGLAS.md` y confirma que describe lo que quiere. Es el documento más importante del
proyecto; todo lo demás se deriva de él.

### Fase 1 — Base y pruebas de concepto (S) · `rust/fase-1-base`

- Workspace, `Cargo.lock`, CI Linux (`test`, `clippy`, `fmt`) y compilación Windows.
- Control de identidades en hook y CI.
- Tres pruebas: lectura de las 4 variantes de `.xls`; lectura de los HTML ficticios; ventana con tabla editable en
  `slint` y en `egui` (decide R4).
- Término: informe corto de las tres pruebas y biblioteca de interfaz elegida. Si la lectura no funciona, se
  detiene y se informa.

### Fase 2 — Lectura y dominio (M) · `rust/fase-2-lectura`

- `flow-dominio`, `flow-tabular`, `flow-html`.
- Subcomando `flow leer ARCHIVO` que imprime lo leído como JSON: sirve para comparar con Python sin interfaz.
- Término: todos los fixtures ficticios se leen; diferencias con Python listadas y explicadas.

### Fase 3 — Motor (L) · `rust/fase-3-motor`

- `flow-motor` con las reglas de `REGLAS.md`, en este orden: comunes → Espera → Cumplimiento → Informes → Historia.
- Subcomando `flow observar` que toma registros y fecha de corte y entrega las observaciones.
- Cada caso de `casos/*.json` es una prueba. Comparación opcional con el motor Python sobre los mismos casos.
- Término: todos los casos pasan y el usuario revisa una muestra de observaciones sobre datos ficticios.

### Fase 4 — Anonimizador (M) · `rust/fase-4-anonimo`

- `flow-anonimo` y subcomando `flow anonimizar` con los mismos modos y la verificación de «falla cerrado».
- Va antes de la interfaz porque no la necesita y deja al usuario una herramienta útil temprano.

### Fase 5 — Productos (L) · `rust/fase-5-productos`

- `flow-excel`: observaciones, nóminas, consolidado.
- `flow-word`: proyectos desde las plantillas; un documento por lote.
- `flow-outlook`: borradores con CC fijo, adjuntos, estado incierto y registro de lo ya creado para no duplicar.
- `flow-almacen`: trabajo, sesiones y retención (60 días, 10 sesiones).
- Término: desde la línea de comandos, un lote ficticio produce Excel, Word y borradores en Outlook del usuario.

### Fase 6 — Interfaz (L) · `rust/fase-6-interfaz`

- `flow-ui`: Trabajo → Correos → Resoluciones → Resultados, con teclado completo.
- Importar Excel o carpeta de lote. Editar observaciones. Bandeja «Qué requiere mi atención».
- Módulo Anonimizador con ventana.

### Fase 7 — Captura desde SITFA (L) · `rust/fase-7-captura`

- `flow-captura` y extensión adaptada: Seguimiento, litigantes, calendarios, bitácoras, historia.
- Lotes con checkpoints y «Retomar»; PDF de consulta vacía; reutilización de bitácoras leídas hoy.
- Al ser una sola aplicación, lo capturado pasa directo a Trabajo. El ZIP queda como exportación.
- Va al final porque es lo único que solo se puede probar con SITFA real, y hasta entonces el Descargador Python
  sigue entregando los Excel que el resto de la aplicación ya sabe leer.

### Fase 8 — Meta 4 y cierre (M) · `rust/fase-8-meta4`

- `flow-meta4` con ventana y subcomando.
- Instalador, manual y guía de pruebas nuevos.
- Decisión del usuario sobre el retiro de las herramientas Python.

## 5. Qué se usa mientras tanto

Las herramientas Python siguen siendo las de trabajo diario hasta que el usuario acepte la fase que las reemplaza:
Asistente hasta la Fase 6, Descargador y Lite hasta la 7, Meta 4 hasta la 8, Anonimizador hasta la 4.

## 6. Riesgos

```json
[
  {"riesgo": "REGLAS.md queda incompleta y el motor nuevo olvida un caso límite", "mitigacion": "los casos salen de las 932 pruebas existentes, no de la memoria; comparación opcional con Python"},
  {"riesgo": "el HTML real de SITFA difiere del ficticio", "mitigacion": "extractores de estructura anónima ya existentes; el usuario entrega solo estructura"},
  {"riesgo": "Outlook, Excel y Word por COM no se prueban en CI", "mitigacion": "capa mínima; prueba manual del usuario con guía en cada fase que los toca"},
  {"riesgo": "unir documentos Word", "mitigacion": "un documento por lote construido de una vez, sin unir archivos ya generados"},
  {"riesgo": "el proyecto se alarga y conviven dos sistemas", "mitigacion": "cada fase entrega algo usable por línea de comandos; se puede detener al final de cualquiera"},
  {"riesgo": "rediseñar más de lo necesario", "mitigacion": "FUERA.md fija el alcance en la Fase 0; función nueva solo por decisión del usuario"}
]
```

## 7. Siguiente paso

Fase 0, primer entregable: `REGLAS.md` de las reglas comunes y de Espera (lo más chico), para que el usuario valide
el formato antes de extraer el resto.
