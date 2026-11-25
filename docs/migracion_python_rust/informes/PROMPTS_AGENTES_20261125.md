# Prompts de los agentes en paralelo · 25 de noviembre de 2026

Base común: commit `9c9ca7e` de `estudio/paridad-python-rust-20261119`. Reparto de archivos, para que nadie pise a nadie:

| Agente | Rama | Encargo | Archivos propios |
|---|---|---|---|
| A (ya en marcha) | `estudio/paridad-python-rust-20261119` | E-A10, E-P1, E-I1 | `trabajo.rs`, `ui.rs`, `sincronizar.rs`, `conciliar.rs`, `productos.rs`, `producto_excel.rs`, `copia_excel.rs`, `personalizar.rs`, instalador y workflow |
| B | `estudio/agente-b-meta4-20261125` | Meta 4: ampliar el contraste con Python | `descargador/meta4.rs`, `meta4_ui.rs`, `generar_meta4.py`, `meta4.json` |
| C | `estudio/agente-c-parciales-20261125` | Descargador: causas de paquetes parciales | `descargador/{lecturas,bitacoras,vinculos,historico,historia,sitfa_ficticio}.rs`, `descargador_ui.rs` (solo el aviso), generadores y JSON de lecturas, bitácoras y vínculos |

Los tres tocan `descargador/paquete.rs` o lo usan: B solo lo usa; C puede cambiarlo sin alterar las firmas de `leer_registro`, `cargar_lote`, `cargar_lote_con`, `rutas_bitacoras`, `tablas_lote`, `mapeo`, `leer_json` ni los tipos `Lote`, `Fuente` y `Clave`.

## Bloque común (va dentro de cada prompt)

```
REGLAS DEL PROYECTO: datos sintéticos; nunca HAR/HTML/PDF/Excel/capturas reales (si hace falta un caso, constrúyelo ficticio); sin escritura en RUS/SATURNO; Outlook solo borradores con CC fijo; sin --force, sin merge a main ni a la rama estudio/paridad-python-rust-20261119, sin releases. La carpeta Python descargador/, src/ y tests/ es la referencia congelada: no se modifica (python3 tools/verificar_referencia_python.py debe seguir en PASA).

LÍMITES: no edites ESTADO.md, PLAN.md, REGISTRO_IMPLEMENTACION.md, AGENTS.md, CONTINUACION.md, el workflow, Cargo.toml ni Cargo.lock (sin dependencias nuevas), ni archivos de otros agentes. Solo extiendes oráculos que ya están en la lista del workflow; si necesitas uno nuevo, díselo al coordinador en el informe. Si tocas escrituras a disco, actualiza solo tu línea de tools/inventario_escrituras.py.

PRUEBAS: tools/probar_app.sh rapido <filtro> mientras trabajas y tools/probar_app.sh todo una vez antes de publicar (no lo encadenes con grep); cargo clippy --manifest-path app/Cargo.toml --features eframe/x11 --lib -- -D warnings; los verificadores estáticos que correspondan. Un oráculo nuevo o ampliado lleva una prueba permanente que altera lo esperado y exige que el comparador falle. Los generadores de Python deben ser deterministas y no depender del sistema (escribir bytes, fechas fijas, ZipInfo.create_system = 3, propiedades de openpyxl fijas). Si un generador necesita tkinter: apt-get update && apt-get install -y python3-tk y un venv con /usr/bin/python3.12 --system-site-packages más openpyxl pandas docxcompose python-docx xlrd lxml pillow numpy reportlab pytest.

PUBLICACIÓN: commits en tu rama con las líneas de atribución del sistema; git push -u origin <tu rama>. La CI de Windows corre sola en las ramas estudio/agente-*; revísala con las herramientas MCP de GitHub (actions_list, get_job_logs) sin sondear en bucle. HECHO = commit + CI en verde (linux y validar) y, si cambiaste un generador, el paso «Regenerar los oráculos» corrido (no omitido). ACEPTADO lo da solo el usuario. Lo que no puedas cumplir se informa como BLOQUEADO con el motivo.

INFORME (al terminar): escribe docs/migracion_python_rust/informes/AGENTE_<letra>_20261125.md, en un commit aparte y el último, con: 1) resumen en ≤10 líneas; 2) commits y archivos tocados; 3) pruebas y conteos antes → después; 4) runs de CI y su estado; 5) diferencias o hallazgos, clasificados; 6) texto propuesto para ESTADO.md, PLAN.md y el registro (≤10 líneas); 7) riesgos y pendientes. Después publica en el chat el mismo resumen (≤25 líneas) con tu rama, el último commit y el run de CI. El coordinador revisará, integrará a la rama principal y validará de nuevo.

ESTILO: español chileno, primera línea = la respuesta, sin cortesías, hechos | inferencias | especulación separados, código completo.
```

## Prompt del Agente B · Meta 4

```
Eres el Agente B. Trabajas en MatiGaete2023/flow, rama estudio/agente-b-meta4-20261125 (parte de 9c9ca7e). Otros dos agentes trabajan en paralelo: A (Asistente, Word, instalador) y C (Descargador). No toques lo suyo.

Lee solo AGENTS.md (incluida «Trabajo en paralelo»), docs/migracion_python_rust/ESTADO.md, PLAN.md y CONTINUACION.md.

ENCARGO · Meta 4 (F23/F24): ampliar el contraste con Python y cubrir lo que el oráculo actual no ejercita.
Hoy app/qa/generar_meta4.py (22 casos) compara meta4_bitacoras.generar de Python con descargador/meta4.rs::calcular, y en 5 casos también los libros y el reporte (9 hojas auxiliares declaradas en HOJAS_DECLARADAS; decisión D-13: se conservan, no se alinean). Pero el lote ficticio solo trae Residencia en Cumplimiento del tribunal de Tomé.

Tareas:
1. Ampliar generar_meta4.py y meta4.json con casos de: listas de Espera; FAE y Residencia en el mismo lote; otros tribunales (Laja, Mulchén) y más de un tribunal; un registro de Cumplimiento cuya última etapa es Espera (ALERTA); dos lotes que no se solapan; filtro con varios tribunales; observaciones del mismo día con etapas distintas («REVISAR: etapas distintas…»); bordes de la ventana (día inicial y final); programa que no es FAE ni Residencia (queda fuera de META 4 pero en Registros).
2. Oráculo de regeneración: regenerar(registros, destino, corte, emission, template) de Python contra la regeneración de Rust (meta4_ui / flow_meta4 publicar_regenerado), con y sin Bitacoras_del_mes.xlsx original y con un Registros.xlsx cuyas celdas de revisión se editaron. Compara valores, no diseño.
3. Plantilla propia: validar_plantilla de Python contra la validación de Rust con plantillas sintéticas (válida, sin hojas, con estilos inválidos).
4. Clasifica cada diferencia: (a) error de Rust: corrígelo en descargador/meta4.rs o meta4_ui.rs; (b) diferencia auxiliar de hoja: agrégala a HOJAS_DECLARADAS con su motivo; (c) ambigüedad o error de Python: no lo cambies y repórtalo.

Archivos tuyos: app/qa/generar_meta4.py, app/tests/datos/meta4.json, app/src/descargador/meta4.rs, app/src/meta4_ui.rs y, solo si es indispensable y lo informas, rust/crates/flow-meta4/** (sin cambiar el diseño de los libros).
Prohibido: ui.rs, trabajo.rs, sincronizar.rs, conciliar.rs, productos.rs, producto_excel.rs, copia_excel.rs, descargador/paquete.rs (úsalo sin cambiarlo) y lo del bloque común.

[Pega aquí el BLOQUE COMÚN con <letra> = B]
```

## Prompt del Agente C · Descargador: paquetes parciales

```
Eres el Agente C. Trabajas en MatiGaete2023/flow, rama estudio/agente-c-parciales-20261125 (parte de 9c9ca7e). Otros dos agentes trabajan en paralelo: A (Asistente, Word, instalador) y B (Meta 4). No toques lo suyo.

Lee solo AGENTS.md (incluida «Trabajo en paralelo»), docs/migracion_python_rust/ESTADO.md, PLAN.md y CONTINUACION.md.

ENCARGO · Descargador: causas de los «paquetes parciales».
Hechos: el usuario reportó que algunos paquetes salen como parciales. Dos lotes reales (114 y 14 registros) se leyeron todo LEIDA, incluido el caso «- - -» en Fecha Informe (32 PENDIENTE con fecha vacía leídos bien), y la causa no se reprodujo. No hay archivos reales ni los habrá: solo extractos anónimos. La extensión de Rust ya escribe diagnostico_bitacoras.json y diagnostico_calendarios.json (estados y códigos CAL_*). Causas candidatas: ENVIADO con «- - -» en Fecha Informe; «- - -» en Fecha Proyección; estado distinto de ENVIADO o PENDIENTE; calendario de informes por mes incompleto; histórico sin verificar; .exe anterior al 12 de noviembre.

Tareas:
1. Inventario en app/src/descargador/ y app/src/captura/ de cada condición que produce cobertura parcial, incidencia o aviso de paquete parcial (busca parcial, PARCIAL, coverage, incidencias, FALLIDA, NO_VERIFICADA, VACIA_COMPROBADA, CAL_*). Tabla: código → condición → archivo:línea.
2. Reproduce cada candidata con datos ficticios (descargador/sitfa_ficticio.rs y los generadores generar_lecturas.py, generar_bitacoras.py, generar_vinculos.py). Para cada una, una prueba Rust que afirme el resultado exacto (estado de la lectura, cobertura del paquete, incidencias). Donde Python tenga el mismo flujo, extiende su oráculo existente (lecturas, bitacoras, vinculos o paquete) y contrástalo.
3. Diagnóstico anónimo accionable: un resumen «motivo de cobertura parcial» por paquete (códigos y conteos por consulta; sin RUT, RIT, nombres ni texto libre), en los diagnósticos existentes o en uno hermano nuevo, y un aviso al crear el paquete que nombre el código de la causa. Prueba permanente que verifique que ningún dato personal ficticio aparece en el diagnóstico.
4. Documento corto «Qué enviar» para el usuario: qué archivo anónimo mandar cuando vea un paquete parcial, y cómo leer su aviso.
5. Informa cuáles candidatas sí producen un paquete parcial y cuáles no.

Archivos tuyos: app/src/descargador/{lecturas,bitacoras,vinculos,historico,historia,sitfa_ficticio}.rs, app/src/descargador_ui.rs (solo el texto del aviso), app/qa/generar_{lecturas,bitacoras,vinculos}.py y sus JSON en app/tests/datos/. descargador/paquete.rs puedes cambiarlo sin alterar las firmas de leer_registro, cargar_lote, cargar_lote_con, rutas_bitacoras, tablas_lote, mapeo, leer_json ni los tipos Lote, Fuente y Clave (los usa el Agente B).
Prohibido: ui.rs, trabajo.rs, sincronizar.rs, conciliar.rs, productos.rs, descargador/meta4.rs, meta4_ui.rs y lo del bloque común.

[Pega aquí el BLOQUE COMÚN con <letra> = C]
```

## Cómo revisa y coordina el coordinador

1. Por agente: `git diff 9c9ca7e..<rama> --stat` para confirmar que solo tocó sus archivos; leer su informe; comprobar la CI de su rama (linux, validar y «Regenerar los oráculos» corrido).
2. Integrar de a una a la rama principal (`git merge --no-ff`), resolver conflictos si los hubiera, correr `tools/probar_app.sh todo`, clippy y verificadores, y pasar el texto propuesto a ESTADO, PLAN y registro.
3. Publicar y esperar la CI de Windows en la rama principal: recién ahí el encargo queda `HECHO`.
4. Decidir en la integración lo que cada agente marcó como ambiguo, y preguntar al usuario solo lo que cambie reglas judiciales.

## Prompt del Agente C · Segundo encargo · E-S2 conjuntos de Flow Python en Resultados

Pensado para GPT-5.6 Luna: pasos numerados, archivos cerrados y criterio de parada explícito.

```
Eres el Agente C. Trabajas en MatiGaete2023/flow. Crea la rama estudio/agente-c-conjuntos-20261125 desde el último commit de origin/estudio/paridad-python-rust-20261119 (ya trae integrado tu encargo anterior, E-Q8). Otros agentes: A (Asistente, Word, instalador) y B (Meta 4). No toques lo suyo.

PASO 0 · CHECKOUT (antes de leer o escribir nada).
1. Busca un clon válido: una carpeta donde `git remote get-url origin` termine en MatiGaete2023/flow(.git). Una carpeta con otro origen, en `master` o sin `.git` NO sirve, aunque se llame «flow».
2. Si no existe: git clone https://github.com/MatiGaete2023/flow.git flow-rust-c (en una carpeta nueva y vacía, fuera de carpetas de archivos pegados).
3. En el clon: git fetch origin estudio/paridad-python-rust-20261119 y luego git checkout -b estudio/agente-c-conjuntos-20261125 origin/estudio/paridad-python-rust-20261119 (si la rama ya existe en origin, git checkout -B estudio/agente-c-conjuntos-20261125 origin/estudio/agente-c-conjuntos-20261125). Comprueba git merge-base --is-ancestor 622a56b HEAD (debe terminar en 0).
4. Si el clon o el fetch fallan, responde BLOQUEADO con el error literal y esta línea para el usuario: «Abre el agente en la carpeta de tu clon local de MatiGaete2023/flow, o conecta el repositorio al entorno del agente, y vuelve a pegar este prompt».
5. Informa en tu primera respuesta la ruta del clon, la rama y el commit de HEAD.

RECORDATORIO: no edites ESTADO, PLAN, REGISTRO, AGENTS, CONTINUACION ni el workflow (salvo la línea del punto 6). En el informe pon ENTREGADO, no HECHO. «Regenerar los oráculos» debe haber CORRIDO (no skipped) y nombrar archivo_local en su log; el paso ahora termina con «Oráculos regenerados sin diferencias.» o con «Oráculos que no coinciden: …».

Lee solo AGENTS.md (incluida «Trabajo en paralelo»), docs/migracion_python_rust/ESTADO.md, PLAN.md (entradas E-S2 y E-Q9) y CONTINUACION.md.

AMPLIACIÓN (prueba del usuario, 25-nov): «Descargar lote» trajo solo las hojas de RUS, «Leer bitácoras del lote» el resto en otra carpeta, y no hubo forma de juntarlos en un paquete. Este encargo resuelve juntar lo ya descargado; descargar todo en una sola acción es E-Q9 (no lo hagas aquí). Agrega a las tareas:
2b. En Resultados, «Crear paquete» trabaja sobre una colección de carpetas elegidas (lotes y conjuntos, con «Quitar») más una lista de controles bitacoras.json de otras carpetas («Añadir bitácoras…», selector de archivos bitacoras.json, con «Quitar»), como ventana_resultados.py (choose, add_diaries, package_folders, package_diaries, export). Arma el paquete con cargar_fuentes y paquete::exportar_con con Opciones { bitacoras, emparejar: true, .. } (export_capture_with_note(lots, diaries=…, match_diaries=True)). Muestra la nota de BITACORAS_REUTILIZADAS y de bitácoras sin coincidencia que ya produce el exportador. Una referencia ausente se muestra como NO_DISPONIBLE y bloquea crear el paquete hasta quitarla, como review_missing_diaries.
4b. En el oráculo, al menos 3 casos más: un lote de listados con un bitacoras.json de otra carpeta (todas coinciden), con uno que coincide en parte y con uno que no coincide; compara las tablas del paquete y las incidencias con Python.

ENCARGO · E-S2 · Resultados abre conjuntos de Flow Python (F22).
Hechos: el usuario abrió en «Resultados del lote» una carpeta antigua de Flow Python y Rust respondió «falta resumen.json». Python acepta tres tipos de carpeta (descargador/archivo_local.py, kind): bitacoras.json → «Bitácoras», flujo.json → «Conjunto», resumen.json → «Descarga», en ese orden de prioridad; sin ninguno: «La carpeta debe conservar resumen.json, flujo.json o bitacoras.json y sus originales.». Python resuelve un conjunto a las carpetas de sus fases (archivo_local.load_sources + flujo_csmp.source_folder y FLOW_DONE). Rust (descargador/paquete.rs, cargar_lote) hoy solo reconoce el tipo y pide elegir una subcarpeta.

Referencia Python (léela antes de escribir código; no la modifiques): descargador/archivo_local.py (kind, load_sources), descargador/flujo_csmp.py (FLOW_DONE, source_folder), descargador/resultados.py (load_lot, read_json), descargador/ventana_resultados.py (choose, load: mensajes de Conjunto y Bitácoras; el export de crear paquete) y las pruebas descargador/test_reutilizacion_local.py y test_referencias_ausentes.py.

Tareas, en orden:
1. Nuevo app/src/descargador/archivo_local.rs con:
   - tipo(carpeta) → Bitacoras | Conjunto | Descarga, misma prioridad y mismo mensaje de error que kind.
   - fases_conjunto(carpeta, parcial) → carpetas de las fases: acepta estado PREPARADO_SIN_CONSULTA_FIRMAS, VALIDADA o COMPLETA; otro estado sin parcial da exactamente «El conjunto está incompleto. Permite resultados incompletos para conservar su alcance explícito.»; omite fases sin «carpeta»; rechaza con los textos de source_folder una carpeta vacía o no texto, igual a la raíz o fuera del conjunto (incluye «..» y rutas absolutas). Normaliza rutas antes de comparar (lección de Windows: nombres cortos RUNNER~1 y enlaces; usa la misma normalización que ya usa el Descargador).
   - cargar_fuentes(carpetas, parcial) → Vec<Lote>: como load_sources; ignora Bitácoras, expande conjuntos, deduplica por carpeta normalizada conservando el primer orden, carga cada una con paquete::cargar_lote_con(carpeta, parcial) y, si no queda ninguna, da «Añade al menos una descarga de hojas RUS para crear el paquete del Asistente.».
   - Declara el módulo con una línea en app/src/descargador.rs.
2. Resultados (app/src/descargador_resultados_ui.rs): al elegir una carpeta de conjunto, muestra sus fases (nombre de carpeta y estado de cada resumen.json) y el mensaje «Conjunto disponible. Crear paquete reúne sus fases…» adaptado a los botones que existen en Rust; los productos por lote (consolidado, CSV, ICS, ZIP, comparar) se ofrecen por fase. Crear paquete desde un conjunto usa cargar_fuentes y el exportador existente del paquete. Una carpeta de Bitácoras muestra qué hacer con ella en Rust (sin inventar una función nueva). Una carpeta incompleta respeta la casilla de resultados incompletos que ya existe.
3. En paquete.rs solo puedes cambiar el texto del error de cargar_lote para flujo.json, para que remita a Resultados. No cambies las firmas de leer_registro, cargar_lote, cargar_lote_con, rutas_bitacoras, tablas_lote, mapeo, leer_json ni los tipos Lote, Fuente y Clave (los usa B).
4. Oráculo nuevo: app/qa/generar_archivo_local.py y app/tests/datos/archivo_local.json. Construye carpetas ficticias con los ayudantes existentes (generar_lote.py, generar_excel.reconstruir) y graba lo que hace Python. Mínimo 12 casos: prioridad de los tres tipos; carpeta sin control; flujo.json con JSON inválido; los tres estados aceptados; estado incompleto con y sin parcial; fase sin carpeta; carpeta vacía, igual a la raíz, con «..» y absoluta; la misma fase en dos conjuntos y un conjunto más su propia fase (deduplicación y orden); solo carpetas de Bitácoras (lista vacía); y 2 casos que además crean el paquete desde un conjunto y comparan sus tablas con export_capture de Python sobre load_sources. Compara tipo, carpetas resueltas (rutas relativas a la raíz del caso, con «/»), mensajes de error exactos y tablas del paquete. El generador debe ser determinista y sin dependencias del sistema (bytes, fechas fijas, ZipInfo.create_system = 3, propiedades fijas de openpyxl).
5. Pruebas Rust: conjuntos_iguales_a_python (afirma el número de casos) y oraculo_de_conjuntos_detecta_alteraciones (altera una carpeta resuelta, un mensaje y una tabla y exige que el comparador falle). Una prueba de vista: abrir un conjunto ficticio en Resultados muestra sus fases y crea el paquete.
6. Única excepción al bloque común sobre el workflow: agrega 'archivo_local' al arreglo de oráculos del paso «Regenerar los oráculos con la referencia Python» de .github/workflows/flow-rust-local.yml y nada más en ese archivo. Infórmalo.
7. Si archivo_local.rs o la vista escriben a disco, agrega o ajusta su línea en tools/inventario_escrituras.py.

Fuera de alcance (infórmalo como pendiente, no lo implementes): historial de referencias en preferencias (remember, discover), «Ver descarga conjunta» (show_joint) y la recuperación de un flujo a medias (resume_joint).

Archivos tuyos: app/src/descargador/archivo_local.rs (nuevo), una línea en app/src/descargador.rs, app/src/descargador_resultados_ui.rs (incluida la colección de carpetas y bitácoras), el texto citado de paquete.rs, app/src/descargador/resultados.rs solo si hace falta para exportar por fase, app/qa/generar_archivo_local.py, app/tests/datos/archivo_local.json, la línea del workflow del punto 6 y tu línea de tools/inventario_escrituras.py.
Prohibido: ui.rs, trabajo.rs, sincronizar.rs, conciliar.rs, productos.rs, producto_excel.rs, copia_excel.rs, descargador/meta4.rs, meta4_ui.rs, cualquier archivo Python de descargador/, src/ o tests/, y lo del bloque común.

CRITERIO DE PARADA: si una conducta de Python es ambigua o parece un error, no la «corrijas» en Rust: reprodúcela, fíjala en el oráculo y repórtala como hallazgo. Si algo exige cambiar una firma protegida, otro archivo prohibido o una dependencia, detente y repórtalo como BLOQUEADO. Si la CI falla dos veces por la misma causa sin que la entiendas, detente y reporta el log.

[Pega aquí el BLOQUE COMÚN con <letra> = C; el informe va en docs/migracion_python_rust/informes/AGENTE_C_20261125b.md]
```

## Prompt del Agente A · E-P1 Word por tribunal (con preparación del checkout y recordatorio)

Motivo: el primer intento no arrancó porque el agente abrió una carpeta que no era el repositorio (`…\work\flow-feat-flow-estudio-d-20261117`, rama `master`). El paso 0 deja el checkout correcto o se declara BLOQUEADO.

```
Eres el Agente A. Trabajas en MatiGaete2023/flow, directamente en la rama estudio/paridad-python-rust-20261119. Otros agentes: B (Meta 4) y C (Descargador, rama estudio/agente-c-conjuntos-20261125). No toques lo suyo.

PASO 0 · CHECKOUT (antes de leer o escribir nada).
1. Busca un clon válido: una carpeta donde `git remote get-url origin` termine en MatiGaete2023/flow(.git). Una carpeta con otro origen, en `master` o sin `.git` NO sirve, aunque se llame «flow»; no la uses ni la modifiques.
2. Si no existe, clónalo en una carpeta nueva y vacía, fuera de cualquier carpeta de archivos pegados:
   git clone https://github.com/MatiGaete2023/flow.git flow-rust-a
3. En el clon:
   git fetch origin estudio/paridad-python-rust-20261119
   git checkout -B estudio/paridad-python-rust-20261119 origin/estudio/paridad-python-rust-20261119
   git status -sb
   git log --oneline -3
   La rama debe seguir a origin y contener el commit 1065344 (git merge-base --is-ancestor 1065344 HEAD termina en 0). Si hay cambios locales que no son tuyos, detente: no los descartes.
4. Si el clon o el fetch fallan (sin red, sin credenciales de GitHub, repositorio privado), no sigas: responde BLOQUEADO con el error literal y esta línea para el usuario: «Abre el agente en la carpeta de tu clon local de MatiGaete2023/flow, o conecta el repositorio MatiGaete2023/flow al entorno del agente, y vuelve a pegar este prompt».
5. Informa en tu primera respuesta la ruta del clon, la rama y el commit de HEAD.

RECORDATORIO (dos fallas del encargo anterior, E-A10):
- No edites ESTADO.md, PLAN.md, REGISTRO_IMPLEMENTACION.md, AGENTS.md, CONTINUACION.md ni .github/workflows/*. En a059930 y faee8ea los editaste: los integra solo el coordinador. Tu texto propuesto va en el informe. Única excepción de este encargo: la línea del punto 6.
- HECHO exige que el paso «Regenerar los oráculos con la referencia Python» del job validar haya CORRIDO (conclusión success, no skipped) y haya comparado tus oráculos por nombre. En E-A10 el run 45652241372 lo omitió porque el commit solo tocaba demo.rs, y lo declaraste HECHO igual. Revisa el log del paso: debe listar tus oráculos. Si quedó omitido, dilo en el informe; no declares HECHO.
- No escribas «HECHO» en ningún archivo: en el informe pon ENTREGADO con commit, run y salida literal del paso. El estado lo cambia el coordinador.

Lee solo AGENTS.md, docs/migracion_python_rust/ESTADO.md, PLAN.md (entrada E-P1) y CONTINUACION.md.

ENCARGO · E-P1 · Proyectos Word: elegir el formato al crearlos (F14).
Decisión del usuario (25-nov): al crear los proyectos Word, la persona elige entre «un .docx por proyecto» (lo que hace hoy productos::generar_word) y «un .docx por tribunal». En el segundo, cada archivo reúne los proyectos de un tribunal, con un índice al inicio y un salto de página entre proyectos.
Referencia Python (no la modifiques): src/nurus/personal/resolutions.py, generate_projects. Python une todos los proyectos en un solo .docx con docxcompose (Composer(docs[0], preserve_styles=True); add_page_break antes de cada proyecto siguiente; without_italics al final; require_complete) y no tiene índice. Rust separa por tribunal y agrega el índice por decisión del usuario.

Tareas, en orden:
1. En productos.rs, una función nueva que genere un .docx por tribunal a partir de los mismos Proyecto que usa generar_word, sin cambiar generar_word ni su firma. Cada matriz conserva sus estilos (como preserve_styles). Salto de página entre proyectos; ningún proyecto empieza a mitad de página. Sin cursivas al final, como without_italics. Carpeta nueva con nueva_ruta, nunca sobrescribe.
2. Índice al inicio de cada archivo: título «Índice» y una línea por proyecto en el orden del archivo, con número, RIT y tipo de proyecto (kind_label). Sin números de página ni campos de Word. No agregues otros textos judiciales: los proyectos van con el texto exacto de su matriz y de la revisión.
3. En ui.rs, en «Resoluciones», el selector «Un .docx por proyecto / Un .docx por tribunal» antes de crear. Por omisión, el formato actual. Mensaje final con la carpeta y cuántos archivos y proyectos quedaron.
4. Recibos: cada proyecto deja su recibo como hoy, con la ruta del archivo de su tribunal.
5. Oráculo nuevo app/qa/generar_word_tribunal.py y app/tests/datos/word_tribunal.json. Python arma, con datos ficticios y las matrices de src/nurus/personal/plantillas_word, los proyectos de al menos 2 tribunales (Laja y Mulchén) y 2 tipos, y genera con generate_projects el .docx de cada tribunal pasándole solo los proyectos de ese tribunal. Graba por archivo la secuencia de párrafos (texto y si va precedido de salto de página) y si quedan cursivas. Rust compara lo mismo, quitando su índice, que se afirma aparte (líneas exactas). Mínimo 6 casos: 1 proyecto, 3 proyectos de un tribunal, 2 tribunales, texto editado en la revisión, matriz propia del usuario y un proyecto con cursivas en la matriz. Determinista: escribir bytes, fechas fijas, ZipInfo.create_system = 3.
6. Única excepción de workflow de este encargo: agrega 'word_tribunal' al arreglo de oráculos del paso «Regenerar los oráculos con la referencia Python» de .github/workflows/flow-rust-local.yml. Nada más en ese archivo.
7. Pruebas Rust: word_por_tribunal_igual_a_python (afirma el número de casos y de archivos) y oraculo_de_word_tribunal_detecta_alteraciones (altera un párrafo, un salto y una línea del índice, y exige que el comparador falle). Si generar_word deja de pasar sus pruebas actuales, detente.
8. Si escribes a disco por una ruta nueva, ajusta la línea de productos.rs en tools/inventario_escrituras.py.

Archivos tuyos: productos.rs, ui.rs (solo «Resoluciones»), app/qa/generar_word_tribunal.py, app/tests/datos/word_tribunal.json, la línea del punto 6 y la línea de productos.rs del inventario.
Prohibido: descargador/**, descargador_ui.rs, descargador_resultados_ui.rs, meta4_ui.rs, Cargo.toml y Cargo.lock (sin dependencias nuevas), archivos Python de src/, descargador/ y tests/, y los documentos del recordatorio.

CRITERIO DE PARADA: si Python y la decisión del usuario chocan en algo que no sea el índice ni la separación por tribunal (por ejemplo estilos, saltos o textos), no lo resuelvas: fíjalo en el oráculo y repórtalo. Si algo exige una dependencia nueva o un archivo prohibido, BLOQUEADO. Si la CI falla dos veces por la misma causa sin que la entiendas, detente y reporta el log.

PRUEBAS: tools/probar_app.sh rapido word mientras trabajas; tools/probar_app.sh todo una vez antes de publicar; cargo clippy --manifest-path app/Cargo.toml --features eframe/x11 --lib -- -D warnings; verificar_textos_umbrales.py e inventario_escrituras.py. Si el generador necesita tkinter: apt-get install -y python3-tk y un venv con /usr/bin/python3.12 --system-site-packages más openpyxl pandas docxcompose python-docx xlrd lxml pillow numpy reportlab pytest.

PUBLICACIÓN: commits en estudio/paridad-python-rust-20261119 con las líneas de atribución del sistema; git push origin estudio/paridad-python-rust-20261119 (sin --force; si lo rechaza, git fetch y git merge --no-edit, nunca rebase de commits ajenos). Revisa la CI con las herramientas de GitHub sin sondear en bucle.

INFORME: docs/migracion_python_rust/informes/AGENTE_A_20261125.md, en un commit aparte y el último: 1) resumen ≤10 líneas; 2) ruta del clon, commits y archivos; 3) pruebas y conteos antes → después; 4) run de CI, con la conclusión y el log del paso «Regenerar los oráculos» (debe nombrar word_tribunal); 5) hallazgos (hechos | inferencias | especulación); 6) texto propuesto para ESTADO, PLAN y registro (≤10 líneas); 7) riesgos y pendientes. Estado del encargo en el informe: ENTREGADO o BLOQUEADO.

ESTILO: español chileno, primera línea = la respuesta, sin cortesías, código completo.
```

## Prompt del Agente B · Segundo encargo · E-Q9 «Consultar y crear paquete» (DeepSeek V4.1 Flash)

Pensado para un modelo de implementación guiada: pasos cerrados, firmas fijas y criterio de parada. Sin datos reales: el hallazgo del usuario se describe solo con conteos.

```
Eres el Agente B. Trabajas en MatiGaete2023/flow, en la rama nueva estudio/agente-b-flujo-20261125. Otros agentes: A (Asistente, Word: productos.rs y ui.rs) y C (Resultados del Descargador: descargador_resultados_ui.rs, archivo_local.rs). No toques lo suyo.

PASO 0 · CHECKOUT (antes de leer o escribir nada).
1. Busca un clon donde `git remote get-url origin` termine en MatiGaete2023/flow(.git). Otra carpeta, una en `master` o sin `.git` no sirve aunque se llame «flow».
2. Si no existe: git clone https://github.com/MatiGaete2023/flow.git flow-rust-b (carpeta nueva y vacía).
3. git fetch origin estudio/paridad-python-rust-20261119
   git checkout -b estudio/agente-b-flujo-20261125 origin/estudio/paridad-python-rust-20261119
   git merge-base --is-ancestor f7b0481 HEAD   (debe terminar en 0; ya trae tu E-R3 integrado)
4. Si el clon o el fetch fallan: BLOQUEADO con el error literal y esta línea para el usuario: «Abre el agente en la carpeta de tu clon local de MatiGaete2023/flow, o conecta el repositorio al entorno del agente, y vuelve a pegar este prompt».
5. Primera respuesta: ruta del clon, rama y commit de HEAD.

Lee solo AGENTS.md (incluida «Trabajo en paralelo»), docs/migracion_python_rust/ESTADO.md, PLAN.md (entrada E-Q9) y CONTINUACION.md.

CONTEXTO (prueba del usuario, 25-nov, solo conteos): «Descargar» trajo las hojas de RUS (1 lote VALIDADA, 2 consultas) y, en otro proceso, «Leer bitácoras del lote» leyó 84 bitácoras, 84 calendarios y 66 causas en otra carpeta. El usuario quiere que una sola acción haga todo y deje todo en una sola carpeta, como Flow Python.

REFERENCIA PYTHON (léela antes de escribir código; no la modifiques): descargador/flujo_paquete.py (run, retomar, pendientes, _proceso, _alcance, _consultas_validadas, _informes, meses_informes, anotar, _validar_opciones), su uso en descargador/descargador.py (start_pipeline, resume_pipeline, botón «Consultar y crear paquete», menú «Retomar paquete…», eventos pipeline_folder y pipeline_source) y las pruebas descargador/test_flujo_paquete.py. run_por_hoja queda fuera: infórmalo como pendiente.

ENCARGO · E-Q9 · «Consultar y crear paquete» en una acción (F17, F19, F22).
1. Nuevo app/src/descargador/flujo_paquete.rs con el equivalente de run y retomar:
   - Solo con pantalla Seguimiento; otro caso da exactamente «Consultar y crear paquete requiere Seguimiento; las otras consultas siguen en Avanzado.».
   - Carpeta nueva «Paquete CSMP <fecha y hora>» dentro del destino, con respaldo/proceso.json (estado, fase, carpetas, opciones, historial, reintentos) escrito como Python en cada cambio de fase.
   - Fases en el orden de Python: consultas (lote::ejecutar), bitácoras con calendarios, historia e histórico según las opciones (bitacoras::ejecutar_con_avance), informes si se piden (_informes y meses_informes) y paquete (paquete::empaquetar_lote o paquete::exportar_con, según corresponda). Las subcarpetas de cada fase quedan dentro de «Paquete CSMP …»; el ZIP también.
   - Opciones con los mismos nombres y validación que _validar_opciones: consolidado, calendarios, historia, historico, historia_en_paquete, meses, informes, fecha_informes, pdf_vacias.
   - Retomar: reutiliza las consultas validadas (_consultas_validadas) y las bitácoras ya leídas, completa lo pendiente y vuelve a crear el ZIP; un proceso.json ajeno, dañado o de otra selección se rechaza con los textos de Python.
   - Cancelación y pausa con los mecanismos que ya usan lote::ejecutar y bitacoras::ejecutar_con_avance.
   - Declara el módulo con una línea en app/src/descargador.rs.
   No cambies las firmas de lote::ejecutar, bitacoras::ejecutar_con_avance, paquete::empaquetar_lote, paquete::exportar_con, paquete::cargar_lote, paquete::cargar_lote_con ni los tipos Lote, Fuente y Clave. Si te falta algo, agrégalo como función nueva en flujo_paquete.rs.
2. Interfaz (app/src/descargador_ui.rs, solo lo de esta acción): con Seguimiento, la acción principal es «Consultar y crear paquete», con el avance por fase (reutiliza el avance por fila de E-Q7), pausa y cancelación; «Retomar paquete…» elige una carpeta «Paquete CSMP …». Al terminar muestra el aviso de cobertura que ya produce el paquete, «Abrir carpeta del paquete» y «Abrir en Trabajo». «Descargar», «Leer bitácoras del lote» y «Crear paquete de este lote…» siguen disponibles como pasos sueltos (en Python quedan en Avanzado: puedes moverlos a una sección «Avanzado» sin quitarlos).
3. Oráculo nuevo app/qa/generar_flujo_paquete.py y app/tests/datos/flujo_paquete.json. Python ejecuta flujo_paquete.run y retomar con un puente ficticio (usa los ayudantes de app/qa/generar_lote.py, generar_paquete.py y generar_bitacoras.py; nada real) y graba por caso: secuencia de fases y estados de proceso.json, historial normalizado (sin horas), nombres relativos de las subcarpetas, mensajes de error exactos y tablas del ZIP final. Rust corre lo mismo con descargador/sitfa_ficticio.rs. Mínimo 8 casos: todo completo; sin calendarios; con historia e histórico; con informes; cancelación tras las consultas y retomar; falla de una bitácora y retomar; proceso.json de otra selección (error); pantalla que no es Seguimiento (error). Determinista: escribir bytes, horas fijas, ZipInfo.create_system = 3, JSON del lote en LF antes del base64 y la carpeta temporal con Path(tmp).resolve() (en Windows llega como RUNNER~1).
4. Pruebas Rust: flujo_paquete_igual_a_python (afirma el número de casos y de fases) y oraculo_de_flujo_paquete_detecta_alteraciones (altera una fase, un mensaje y una tabla y exige que el comparador falle). Una prueba de vista: con SITFA ficticio, «Consultar y crear paquete» deja una sola carpeta con las fuentes y el ZIP.
5. Única excepción de workflow: agrega 'flujo_paquete' al arreglo de oráculos del paso «Regenerar los oráculos con la referencia Python» de .github/workflows/flow-rust-local.yml. Nada más en ese archivo.
6. Agrega la línea de flujo_paquete.rs en tools/inventario_escrituras.py (carpeta nueva, proceso.json por temporal y renombrado, nunca sobrescribe).

Archivos tuyos: app/src/descargador/flujo_paquete.rs (nuevo), una línea en app/src/descargador.rs, app/src/descargador_ui.rs (solo esta acción, su avance y «Retomar paquete…»), app/qa/generar_flujo_paquete.py, app/tests/datos/flujo_paquete.json, la línea del punto 5 y tu línea del inventario.
Prohibido: descargador_resultados_ui.rs, archivo_local.rs, paquete.rs, lote.rs, bitacoras.rs, lecturas.rs, meta4.rs, ui.rs, trabajo.rs, productos.rs, Cargo.toml y Cargo.lock (sin dependencias nuevas), todo archivo Python de descargador/, src/ y tests/, y ESTADO.md, PLAN.md, REGISTRO_IMPLEMENTACION.md, AGENTS.md y CONTINUACION.md.

REGLAS: datos sintéticos; sin HAR/HTML/PDF/Excel/capturas reales; sin escritura en RUS/SATURNO (la extensión solo lee); Outlook no se toca; sin --force, sin merge a main ni a la rama principal, sin releases. python3 tools/verificar_referencia_python.py y tools/verificar_solo_lectura.py deben seguir en PASA.

CRITERIO DE PARADA: si una conducta de Python es ambigua o parece un error, reprodúcela, fíjala en el oráculo y repórtala; no la «corrijas». Si necesitas cambiar una firma protegida o un archivo prohibido, BLOQUEADO con el motivo. Si la CI falla dos veces por la misma causa sin que la entiendas, detente y reporta el log.

PRUEBAS: tools/probar_app.sh rapido flujo mientras trabajas; tools/probar_app.sh todo una vez antes de publicar; cargo clippy --manifest-path app/Cargo.toml --features eframe/x11 --lib -- -D warnings; inventario_escrituras.py y verificar_solo_lectura.py. Si el generador necesita tkinter: apt-get install -y python3-tk y un venv con /usr/bin/python3.12 --system-site-packages más openpyxl pandas docxcompose python-docx xlrd lxml pillow numpy reportlab pytest icalendar.

PUBLICACIÓN: commits en tu rama con las líneas de atribución del sistema; git push -u origin estudio/agente-b-flujo-20261125. Revisa la CI con las herramientas de GitHub sin sondear en bucle. El paso «Regenerar los oráculos» debe CORRER (no skipped) y su log debe terminar en «Oráculos regenerados sin diferencias.»; si dice «Oráculos que no coinciden: …», corrige antes de informar.

INFORME: docs/migracion_python_rust/informes/AGENTE_B_20261125b.md, en un commit aparte y el último: 1) resumen ≤10 líneas; 2) ruta del clon, commits y archivos; 3) pruebas y conteos antes → después; 4) run de CI con la conclusión y la última línea del paso de regeneración; 5) hallazgos (hechos | inferencias | especulación); 6) texto propuesto para ESTADO, PLAN y registro (≤10 líneas); 7) riesgos y pendientes (incluido run_por_hoja). Estado: ENTREGADO o BLOQUEADO; el HECHO lo pone el coordinador.

ESTILO: español chileno, primera línea = la respuesta, sin cortesías, código completo.
```
