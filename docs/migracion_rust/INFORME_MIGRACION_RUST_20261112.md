# Informe: migración de las herramientas Flow a Rust

12 de noviembre de 2026 · rama `informe/migracion-rust` (creada desde `Rediseno`, commit `5087523`).
Este documento es análisis y propuesta. No cambia código, no decide reglas y no marca nada como aceptado.

Convención: **N1** hecho medido en el repo · **N2** inferencia con supuesto · **N3** especulación.

## 1. Veredicto

- **Migración total inmediata: no recomendada.** El costo es reescribir ~51.000 líneas de producto y ~26.000 de
  pruebas (N1) para ganar sobre todo en distribución, no en funciones. Ningún problema registrado en
  `PLAN_VIGENTE.md` ni en `ESTADO.json` es de velocidad o de memoria (N1: no aparece ninguno).
- **Recomendado: migración por etapas, empezando por lo independiente** (opción B de la sección 6): Anonimizador →
  núcleo compartido → SITFA Lite → Meta 4 → Descargador. El Asistente queda en Python hasta una decisión expresa
  posterior.
- **Advertencia directa:** hay tres paquetes `BLOQUEADO_USUARIO` (WP-02 acta de uso real, WP-50, WP-70 cierre) (N1).
  La versión Python todavía no tiene acta firmada. Migrar antes de cerrar eso es rediseñar un producto que aún no se
  aceptó: reabre toda la validación en Windows con datos reales, que solo puede hacer el usuario. Si el objetivo es
  «sistema en producción sin intervención manual», el camino corto es firmar el acta, no cambiar de lenguaje.

## 2. Inventario (N1)

Medido con `git ls-files` y `wc -l` sobre `Rediseno`. «Producto» excluye pruebas.

```json
[
  {"herramienta": "Descargador SITFA", "version": "2.9.0", "ruta": "descargador/", "py_archivos": 78,
   "lineas_producto": 12100, "lineas_pruebas": 7500, "gui": "tkinter + flowkit",
   "terceros": ["lxml", "openpyxl", "xlrd", "reportlab", "Pillow", "icalendar"],
   "extra": "extensión Chrome MV3 (825 líneas JS) + puente HTTP en 127.0.0.1"},
  {"herramienta": "SITFA Lite", "version": "1.1.0", "ruta": "descargador_lite/", "py_archivos": 18,
   "lineas_producto": 4000, "lineas_pruebas": 700, "gui": "tkinter + flowkit",
   "terceros": ["lxml", "openpyxl", "xlrd", "reportlab", "Pillow"],
   "extra": "extensión MV3 propia (603 líneas JS), PyInstaller + Inno Setup"},
  {"herramienta": "Asistente CSMP", "version": "0.5.0.dev11", "ruta": "src/nurus/", "py_archivos": 113,
   "lineas_producto": 21500, "lineas_pruebas": 17200, "gui": "customtkinter + tkinter",
   "terceros": ["pandas", "openpyxl", "xlrd", "python-docx", "docxcompose", "lxml", "pywin32"],
   "extra": "COM con Outlook, Excel y Word; motor de reglas (~1.440 líneas); 6 plantillas .docx"},
  {"herramienta": "Anonimizador CSMP", "version": "1.1.0", "ruta": "anonimizador/", "py_archivos": 25,
   "lineas_producto": 4300, "lineas_pruebas": 1400, "gui": "tkinter + flowkit, y CLI",
   "terceros": ["openpyxl", "xlrd"], "extra": "sin red, sin COM, no importa a las otras herramientas"},
  {"herramienta": "Meta 4 · Registros y bitácoras", "version": "2.6.0", "ruta": "META4_Bitacoras/",
   "py_archivos": 12, "lineas_producto": 4600, "lineas_pruebas": 1700, "gui": "tkinter + flowkit",
   "terceros": ["pandas", "openpyxl", "xlrd", "lxml"], "extra": "solo lee carpetas locales"},
  {"herramienta": "Herramientas de desarrollo", "ruta": "tools/", "py_archivos": 37, "lineas_producto": 6000,
   "extra": "verificar_wp, check_real_ids, manuales, capturas y videos"}
]
```

Otros hechos que condicionan la migración (N1):

- **Código duplicado por diseño.** `flowkit.py` existe en 5 copias idénticas (mismo MD5). El lector de Excel de
  `descargador/motor.py` está copiado en `descargador_lite/app/motor.py` y derivado en
  `META4_Bitacoras/scripts/lector_excel.py` (`ORIGEN_COMPONENTES.txt`). También hay copia de `nurus/bitacora_html.py`.
- **Formatos de entrada.** SITFA entrega `.xls` que por contenido son BIFF, OOXML, XML 2003 o tablas HTML. Hay tres
  lectores de HTML (bitácora, historia, calendario) hechos con `lxml.html`.
- **Formatos de salida.** `.xlsx` con estilos, comentarios, formato condicional y validación de datos (`openpyxl`);
  `.docx` desde plantillas (`python-docx` + `docxcompose` + parches XML en `matrix_patches/`); PDF desde captura PNG
  (`reportlab` + `Pillow`); `.ics`; ZIP con manifiesto y SHA-256 (contrato `CSMP_CAPTURE_V1`).
- **Windows.** Outlook por COM con `mail.Save()` (`adapters/outlook.py`), Excel de escritorio por COM para la salida
  de «fidelidad nativa» (`services/exports.py`), Word por COM para vista previa (`word_preview.py`), registro de
  Windows para tema oscuro.
- **Pruebas.** Línea base: Asistente 932 aprobadas y 6 xfail; Descargador 283 aprobadas y 62 subpruebas; paridad
  golden PASA (`docs/implementacion/linea_base.json`). Además hay pruebas `.cjs` de la extensión.
- **Entrega.** PyInstaller (3 `.spec`), Inno Setup, 4 archivos lock con hashes (Linux 3.12 y Windows 3.12/3.13/3.14),
  9 workflows de CI.
- **Material de usuario atado a la interfaz actual.** 90 PNG, 11 MP4 y manuales PDF generados desde capturas de las
  ventanas Tk.
- **Toolchain.** En este contenedor hay `cargo 1.97.0`. No se compiló nada para este informe.

## 3. Equivalencias en Rust

Los crates se consultaron hoy en crates.io (N1: existen, con esa versión). Que cubran el caso exacto de este repo es
**N2** y se confirma con las pruebas de concepto de la Fase 0.

```json
[
  {"hoy": "xlrd / openpyxl lectura / pandas.read_excel", "rust": "calamine 0.38",
   "nota": "lee BIFF y OOXML; NO lee HTML ni XML 2003 con extensión .xls: parser propio"},
  {"hoy": "lxml.html (bitácora, historia, calendario)", "rust": "scraper 0.29 (html5ever)",
   "nota": "RIESGO: html5ever arma el árbol según HTML5 y libxml2 no; el HTML mal formado puede dar otra tabla"},
  {"hoy": "lxml.etree, XML 2003, parches de Word", "rust": "quick-xml 0.42", "nota": "sin DTD ni entidades: mejor"},
  {"hoy": "openpyxl escritura nueva", "rust": "rust_xlsxwriter 0.102",
   "nota": "estilos, notas, formato condicional y validación; solo crea, no abre"},
  {"hoy": "openpyxl abrir-modificar-guardar", "rust": "umya-spreadsheet 4.0.0-beta.1",
   "nota": "RIESGO: versión beta; alternativa: reconstruir el libro con valores"},
  {"hoy": "python-docx + docxcompose", "rust": "zip 9.5 + quick-xml (a mano); docx-rs 0.4 solo escribe",
   "nota": "RIESGO ALTO: no hay equivalente de docxcompose; hay que programar la unión de documentos"},
  {"hoy": "pywin32 (Outlook, Excel, Word)", "rust": "windows 0.62 (IDispatch)",
   "nota": "funciona, pero cada llamada COM son varias líneas; solo se prueba en Windows"},
  {"hoy": "http.server (puente)", "rust": "tiny_http 0.12", "nota": "equivalente directo"},
  {"hoy": "reportlab + Pillow (PNG a PDF)", "rust": "printpdf 0.12 + image", "nota": "caso simple: una imagen"},
  {"hoy": "icalendar", "rust": "icalendar 0.17", "nota": "equivalente"},
  {"hoy": "zipfile, hashlib, hmac, secrets, zoneinfo", "rust": "zip, sha2, hmac, rand, chrono-tz 0.10",
   "nota": "equivalentes"},
  {"hoy": "tkinter / customtkinter / flowkit", "rust": "slint 1.18 o eframe (egui) 0.37; tauri 3 está en alpha",
   "nota": "no existe Tk en Rust: las cinco interfaces se rehacen completas"},
  {"hoy": "PyInstaller + locks + Inno Setup", "rust": "cargo build --release + Cargo.lock",
   "nota": "un .exe por herramienta, sin Python instalado"}
]
```

## 4. Ventajas

- **Entrega (la ganancia real).** Un `.exe` por herramienta, sin instalar Python, sin `.venv`, sin 4 locks ni matriz
  3.12–3.14. Desaparecen `Instalar_CSMP.bat`, `Iniciar.cmd` y los `.spec` (N1: existen hoy). N2: menos falsos
  positivos de antivirus que con PyInstaller; no medido.
- **Fin de las copias.** Un workspace de Cargo permite un solo crate de interfaz y un solo lector de Excel/HTML
  usados por todos. Hoy la unicidad se sostiene con pruebas que comparan copias (N1: `tests/test_flowkit.py`).
- **Errores detectados al compilar.** Con agentes de IA programando, el compilador rechaza `None` no tratados,
  tipos mezclados (fecha/texto en celdas) y ramas olvidadas. N2: baja la clase de defectos «fecha ilegible tratada
  en silencio», que el repo combate hoy con pruebas.
- **Reglas inviolables expresables en tipos.** Un adaptador de Outlook sin método de envío no puede enviar; un
  módulo de registro RUS inexistente no puede escribir. Hoy se controla leyendo el código como texto (N1).
- **Entradas no confiables.** HTML, ZIP y XLS externos se procesan sin riesgo de corrupción de memoria y se pueden
  someter a fuzzing (sección 8).
- **Arranque y consumo.** N2: arranque casi instantáneo y menos memoria (no se carga pandas). No es un problema
  registrado hoy.

## 5. Desventajas y riesgos

- **Costo.** ~51.000 líneas de producto y ~26.000 de pruebas a reescribir. Las pruebas no se traducen solas: muchas
  leen el código Python como texto o parchean módulos.
- **La validación no se puede automatizar.** No hay datos reales en el repo (regla). Toda paridad contra SITFA real,
  Outlook y Office la hace el usuario en Windows. Ese es el cuello de botella, no la programación.
- **Motor de reglas.** `src/nurus/personal/motor/` es intocable salvo decisión expresa y paridad golden. Reescribirlo
  en Rust es tocarlo. Además usa `pd.to_datetime(..., dayfirst=True)` y `pd.to_numeric` (N1), cuyo comportamiento
  permisivo hay que reproducir caso a caso. Riesgo: una observación distinta sin que nadie lo note.
- **Word.** Sin equivalente de `docxcompose`. Es la pieza con más riesgo técnico.
- **COM.** Outlook, Excel y Word desde Rust son posibles pero verbosos, y no se pueden probar en Linux.
- **Interfaz.** Las cinco ventanas se rehacen. Se pierden los 90 PNG, 11 MP4 y manuales PDF actuales, y el pipeline
  de capturas con `pywinauto`. Los atajos (Ctrl+O, Ctrl+K, Enter, Esc, F1) hay que reimplementarlos.
- **Dos lenguajes durante la transición.** Mientras convivan, hay dos CI, dos formas de empaquetar y el contrato
  `CSMP_CAPTURE_V1` implementado dos veces.
- **Librerías inmaduras.** `umya-spreadsheet` está en beta y `tauri` 3 en alpha (N1).
- **Compilación.** N2: una compilación limpia en CI Windows con GUI y `windows` tarda varios minutos; el ciclo de
  prueba es más lento que con Python.
- **`verificar_wp.py` y la línea base** miden pytest. Hay que extenderlos para contar `cargo test`, o la puerta de
  verificación deja de cubrir lo migrado.

## 6. Opciones

```
A. No migrar; simplificar la entrega en Python
   Costo: bajo.  Beneficio: cierra el plan actual sin riesgo nuevo.
   Sirve si: la prioridad es el acta y el uso real.

B. Por etapas, lo independiente primero; el Asistente queda en Python   ← RECOMENDADA
   Costo: medio, repartido; cada etapa se puede detener sin dejar nada a medias.
   Beneficio: .exe únicos para 4 de 5 herramientas; un solo núcleo de lectura.
   Sirve si: se quiere Rust sin arriesgar reglas, Word ni Outlook.

C. Total por etapas (B + Asistente)
   Costo: alto. Incluye motor, Word y COM.
   Beneficio: sin Python en ninguna parte.
   Sirve si: B terminó bien y el usuario decide expresamente migrar el motor.

D. Reescritura completa de una vez
   RECHAZADA: meses sin entregar nada usable y sin paridad verificable por partes.
```

Híbrido con PyO3 (núcleo Rust llamado desde Python): descartado como destino. Mantiene Python y PyInstaller y agrega
la compilación de Rust; suma los costos de ambos.

## 7. Plan de migración (opción B, ampliable a C)

Principio: **el Python actual es el oráculo.** Cada etapa termina cuando el binario Rust da el mismo resultado que
Python sobre los mismos fixtures ficticios, y el usuario lo confirma en Windows. Python no se borra hasta ese acta.

Tamaños relativos: S < M < L < XL. No doy semanas: no tengo datos para estimarlas (N3 si las diera).

### Fase 0 — Decisión y base (S)

1. Decisión escrita del usuario: opción y momento (antes o después de WP-02/WP-70).
2. Workspace `rust/` con `Cargo.lock`; job de CI Linux (`cargo test`, `clippy`, `fmt`) y uno Windows de compilación.
3. Tres pruebas de concepto que deciden si se sigue:
   - leer con `calamine` y con el parser propio los fixtures `.xls` de las cuatro variantes y comparar con Python;
   - leer los fixtures de bitácora e historia con `scraper` y comparar el resultado con `lxml`;
   - una ventana con lista, fichas, calendario y barra de progreso en `slint` y en `egui`, para elegir uno.
4. Extender `tools/verificar_wp.py` y `linea_base.json` con el conteo de `cargo test`.
5. Reglas en el código: prueba que falla si aparece `Send` en el adaptador de Outlook o un módulo de registro RUS.

Salida: informe de las tres pruebas. Si la 1 o la 2 no dan paridad, se detiene aquí.

### Fase 1 — Anonimizador (M) · piloto

Por qué primero: no depende de nada, no usa COM, tiene CLI (se compara sin interfaz) y es crítico en seguridad.

- Crates: `flow-tabular` (lector BIFF/OOXML/XML 2003/HTML/CSV), `flow-zip` (ZIP seguro), `anonimizador`.
- Paridad: mismos argumentos, mismos códigos de salida 0/1/2, mismos seudónimos con `--semilla` (HMAC), misma
  verificación final de «falla cerrado».
- Prueba diferencial: correr ambos sobre `anonimizador/tests` y comparar las salidas celda a celda.
- `.docx`: aquí solo se reemplaza texto, sin unir documentos; sirve de ensayo para Word.

### Fase 2 — Núcleo compartido (M)

- `flow-html`: bitácora, historia y calendario.
- `flow-contract`: `CSMP_CAPTURE_V1` (ZIP, manifiesto, SHA-256), validado contra `contracts/…/manifest.schema.json`
  y sus fixtures.
- `flow-ui`: el equivalente de `flowkit` (barra superior, banner, pie con acción principal, teclado común).
- `flow-bridge`: el puente local (código secreto en RAM, origen `chrome-extension://`, sin registro).

### Fase 3 — SITFA Lite (M)

- Reutiliza `flow-bridge`, `flow-tabular` y `flow-ui`. Agrega PNG→PDF.
- **La extensión sigue en JavaScript** (Chrome lo exige). El protocolo y la comprobación de versión no cambian.
- Las pruebas con `sitfa_ficticio.py` se reutilizan: el SITFA ficticio queda en Python como arnés de prueba.

### Fase 4 — Meta 4 (M)

- Lectura de lotes, asociación de bitácoras, escritura de 4 libros con colores y comentarios (`rust_xlsxwriter`).
- «Regenerar desde Registros revisado» necesita leer un `.xlsx` editado por el usuario: basta `calamine`.
- Punto delicado: reproducir los valores que hoy da pandas al releer (N1: comentario en `meta4_bitacoras.py:504`).

### Fase 5 — Descargador integral (L)

- Lotes, checkpoints, ritmo de consultas, reutilización de bitácoras, retención de 60 días, resultados e historial,
  ICS y creación del paquete. 283 pruebas a portar.
- Se mantiene: no lee cookies ni contraseñas, no escribe en RUS, tareas y código solo en RAM.

### Puerta entre B y C

Revisión con el usuario antes de tocar el Asistente. Requiere decisión expresa sobre el motor.

### Fase 6 — Asistente (XL), solo con decisión expresa

Orden interno, de menor a mayor riesgo:

1. Lectura de RUS y del paquete (reutiliza el núcleo).
2. **Motor**: traducción literal regla por regla; `textos_base.json` se usa tal cual, sin cambiar una coma. Puerta:
   golden `semantic.json` idéntico y las pruebas de reglas en verde, con los mismos 6 xfail.
3. Salidas Excel, incluida la de fidelidad nativa por COM.
4. Word: plantillas, parches y unión de documentos. Comparar el XML resultante contra el de Python.
5. Outlook: solo `Save()`, CC fijo `ucc_concepcion@pjud.cl`, estado incierto y diario de recibos como hoy.
6. Interfaz (Trabajo → Correos → Resoluciones → Resultados).

### Cierre de cada fase

- `cargo test` y prueba diferencial contra Python: PASA.
- Compilación Windows y prueba del usuario con su guía; acta del usuario.
- Recién entonces se retira el Python de esa herramienta y se regeneran manuales y capturas.
- Sin fusión a `main` ni release sin autorización escrita.

## 8. Qué más se puede hacer en Rust

Ordenado de más a menos útil para este repo (N2 salvo indicación):

1. **Hook de identidades como binario.** `check_real_ids` y `check_contactos` hoy necesitan Python en la máquina
   para el pre-commit. Un `.exe` chico corre igual en cualquier clon. Bajo riesgo, buen primer ejercicio.
2. **Verificador de paquetes independiente.** `verificar_paquete` como comando único: valida ZIP, manifiesto y
   huellas sin abrir ninguna de las dos aplicaciones.
3. **Fuzzing de los lectores.** `cargo-fuzz` sobre los parsers de HTML, XLS y ZIP: encuentra entradas que cuelgan o
   rompen el lector antes de que aparezcan en uso real. No tiene equivalente cómodo en el Python actual.
4. **Un solo ejecutable con subcomandos** (`flow anonimizar`, `flow meta4`, `flow verificar`), además de las
   ventanas.
5. **Puente por «native messaging» de Chrome** en vez de HTTP en 127.0.0.1: sin puerto abierto y sin el permiso
   `http://127.0.0.1/*` en la extensión. Es un cambio de diseño que requiere instalar un manifiesto en el equipo:
   decisión del usuario, no parte de la migración.
6. **Anonimizador compilado a WebAssembly**, para usarlo desde una página local sin instalar nada. N3: no evaluado.
7. **Índice local de lotes y sesiones** (SQLite) para búsqueda e historial más rápidos. N3: hoy no hay queja de
   lentitud registrada.
8. **RUS Engine:** sin datos. No está en este repositorio, no lo revisé.

## 9. Lo que no cambia con la migración

- Sin escritura en RUS ni SATURNO. Outlook solo borradores. CC fijo.
- Sin credenciales, cookies ni tokens. Sin datos reales en repo, CI ni conversaciones.
- Reglas, umbrales y textos intactos salvo decisión expresa y paridad golden.
- El Asistente no navega SITFA ni inicia el Descargador.
- Nada «aceptado» sin acta del usuario.

## 10. Siguiente paso

Una sola decisión: **A, B o C**, y si se empieza **antes o después del acta (WP-02)**. Con B, lo siguiente es la
Fase 0, punto 3 (las tres pruebas de concepto).
