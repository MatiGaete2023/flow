# Continuación · relevo del orquestador (26 de noviembre de 2026)

Para abrir un chat nuevo como **orquestador**, con dos agentes: **A · Antigravity** y **C · ChatGPT**. La fuente del estado es [ESTADO.md](ESTADO.md); este archivo resume el proyecto, el reparto, el procedimiento, las lecciones y el prompt de arranque.

## Resumen

**Hechos (26-nov, 02:15 UTC)**

- Avance funcional ~88 %: 28 capacidades, 21 en «Sí» y 7 en «Parcial» (F02, F08, F10, F11, F13, F23, F24). Aceptado 0 %: el acta la da el usuario.
- `HECHO` el 25-nov: E-A10, E-P1, E-P2, E-I1, E-I3, E-A11 y E-T6 (A); E-R3, E-Q9 y E-Q10 (B); E-Q8, E-S2 y E-S3 (C).
- **E-Q11** (descarga conjunta CSMP y «Retomar pendientes», B · ChatGPT Terra) integrado en `55cba7c` (documentos en `6181828` y `279df9a`). Queda `HECHO` cuando el run de la rama principal 45740546126 (commit `6181828`) termine en verde y su paso «Regenerar los oráculos» muestre `flujo_csmp: 8 casos` y «Oráculos regenerados sin diferencias.». En Linux, el oráculo es idéntico (SHA `ca3fd849…`).
- Última corrida de entrega: 45708908715 (`60fe600`), verde. Su artefacto `Flow_Rust_Windows_exe` (vence el 9-dic) **no** trae E-A11, E-T6 ni E-Q11. Hay que lanzar una entrega nueva (`workflow_dispatch` con `entrega=true`).
- Fuera del proyecto por decisión del usuario: manuales, registrador de observaciones RUS, extractores de estructura, variables Word por fila, exclusión manual de una fila, paginación del Word manual, M6, M7, formatos antiguos de Meta 4 y firma Authenticode del instalador. Las actualizaciones llegan por OneDrive institucional (E-I3).

**Inferencias (N2)**

- Casi todo lo que sigue en «Parcial» depende de pruebas del usuario con el `.exe`: Outlook real (F13), Chrome y SITFA reales (F17, F19), plantilla real de Meta 4 (F23 y F24). En F02 no hay en Python una conducta multifila distinta de la ya cotejada. Los próximos encargos saldrán de lo que falle en la prueba corta, más que del plan.
- B queda sin ejecutor. Sus archivos (`flujo_paquete.rs`, `flujo_csmp.rs`, `meta4.rs` y sus generadores) pasan a C.

**Especulación (N3):** que el acta de un mes real (~25-dic) no obligue a reabrir encargos ya `HECHO`.

## Reparto (decisión del usuario del 26-nov; revertible)

| Agente · ejecutor | Rama | Encargo | Archivos propios |
|---|---|---|---|
| A · Antigravity | `estudio/agente-a-<tema>-<fecha>` (hasta ahora trabajaba en la principal; desde aquí, rama propia) | Sin encargo | Asistente: `sincronizar.rs`, `trabajo.rs`, `productos.rs`, `outlook.rs`, `actualizacion.rs`, `installer/`, `ui.rs` en sus secciones, y sus generadores y oráculos |
| C · ChatGPT (GPT-5.6 Terra) | `estudio/agente-c-<tema>-<fecha>` | Sin encargo | Descargador: `descargador/`, `descargador_ui.rs`, `descargador_resultados_ui.rs`, `meta4_ui.rs`, y sus generadores y oráculos (incluye lo que era de B) |

- Ni DeepSeek ni Gemini tienen papel. Los informes y commits anteriores conservan su firma (B · DeepSeek, B · ChatGPT Terra, C · ChatGPT Luna).
- **El orquestador va en una sesión aparte de los agentes.** Si es ChatGPT, nunca en la misma conversación ni en el mismo clon que C. No implementa encargos de A ni de C.
- Cada agente entrega `informes/AGENTE_<letra>_<fecha>.md` con firma y no edita `ESTADO`, `PLAN`, `REGISTRO`, `AGENTS`, `CONTINUACION` ni el workflow. La única excepción es la línea de su oráculo en el workflow, si el prompt la autoriza.

## Capacidades de cada ejecutor (observadas el 25 y 26-nov)

- **Antigravity:** clon Windows del usuario, con `cargo` y Python. Puede lanzar `workflow_dispatch`. Ha citado runs que no existían (45672133399): verificar siempre.
- **ChatGPT Terra (Codex):** el clon `/workspace/flow` **no tiene `cargo`**, y en Windows no tiene WSL, Docker ni VM. Se detuvo dos veces creyendo que necesitaba Linux local. Que publique en su rama y use la CI como verificación. Si la regeneración en Linux hace falta, la hace el orquestador.
- **Orquestador:** necesita Linux con `cargo` y el venv de los generadores, y acceso a GitHub Actions: conclusión de `linux` y `validar`, logs, `workflow_dispatch`. Sin eso, la integración queda `BLOQUEADO`.

## Procedimiento de revisión e integración

1. Leer el informe: firma completa y línea `Agente: <letra> · <herramienta>` en cada commit (`git log --format='%h %s | %(trailers:key=Agente,valueonly)'`). Confirmar con `git diff <principal>...<rama> --stat` que solo tocó sus archivos.
2. CI de la rama, leída por el orquestador y no citada por el agente: `linux` y `validar` en `success`, y «Regenerar los oráculos» **corrido** (no «skipped») con el oráculo del encargo. Si el commit solo tocó Rust, la regeneración se omite: lanzar `workflow_dispatch`, que regenera todos.
3. Regenerar en Linux el oráculo del encargo dos veces y exigir bytes idénticos al versionado.
4. `git merge --no-ff` a la rama principal. Correr `tools/probar_app.sh todo`, `cargo clippy --manifest-path app/Cargo.toml --locked --features eframe/x11 --lib -- -D warnings` y los verificadores: `inventario_escrituras`, `verificar_solo_lectura`, `verificar_referencia_python`, `check_real_ids app/src`, `check_contactos app/src`, `verificar_textos_umbrales`.
5. Actualizar `ESTADO`, `PLAN` y `REGISTRO` (entrada de 10 líneas como máximo) y publicar. Recién con la CI de Windows en verde en la rama principal, con la regeneración corrida, el encargo queda `HECHO`; el cierre se hace en un commit aparte que cita el run.
6. Lo ambiguo lo decide el orquestador y lo registra como revertible en PLAN. Solo se pregunta al usuario lo que cambie reglas o textos judiciales (tandas de opción múltiple, máximo 4).

## Lecciones operativas

- **Generadores deterministas e independientes del sistema:**
  - escribir bytes y normalizar a LF el JSON o texto antes de pasarlo a base64;
  - `Path(tmp).resolve()`, porque Windows entrega `RUNNER~1`;
  - `as_posix()` y `sorted(rglob(...))`;
  - en ZIP y docx, `ZipInfo.create_system = 3` y fecha fija; docx con `ZIP_STORED`;
  - importar `generar_lote`, que trae reloj fijo y `write_text` con LF;
  - sin horas locales;
  - si Python se comporta distinto por sistema (separadores `\`), emular Windows en el generador.
- Venv local: `apt-get install -y python3-tk`; `python3.12 -m venv --system-site-packages`, más `openpyxl pandas docxcompose python-docx xlrd lxml pillow numpy reportlab pytest icalendar`. El `python3` 3.13 no ve `tkinter`.
- Un agente sin checkout válido no arranca: cada prompt empieza con el paso 0 (clon con `origin` MatiGaete2023/flow, rama y commit verificados, o `BLOQUEADO`).
- Un ejecutor por letra y un solo orquestador activo; si hay dos, chocan `ESTADO`, `PLAN` y `REGISTRO`. Cuando una herramienta retoma un encargo, que lo haga desde el commit publicado, no desde cambios locales sin commit.
- Antes de reabrir un encargo, revisar ESTADO: Terra rehízo E-S3 ya cerrado (`AGENTE_C_20261125c.md`, continuación descartada).
- `check_contactos` rechaza direcciones con aspecto real en pruebas: usar `example.test`.
- No hacer trabajo pesado en cada cuadro de la interfaz.
- `tools/probar_app.sh todo` tarda ~20 s. Clippy va con `--lib`; el binario tiene código muerto anterior que no forma parte del gate (por ejemplo, un `mut` sobrante en `descargador_ui.rs`).

## Pendiente del usuario

- Prueba corta con el `.exe` de la entrega nueva: «Consultar y crear paquete», descarga conjunta CSMP y «Retomar pendientes», juntar hojas y bitácoras en Resultados, actualizar desde OneDrive y «Guardar un borrador de prueba en Outlook».
- Lista `dir /b` (solo nombres) de la carpeta del lote antiguo de Python que Resultados rechazó.
- Acta de un mes real en paralelo con Python (~25-dic).

## Prompt para el chat nuevo (orquestador)

```
Eres el orquestador y coordinador de la migración Python → Flow Rust en MatiGaete2023/flow, rama estudio/paridad-python-rust-20261119. Trabajas con dos agentes: A = Antigravity (Asistente) y C = ChatGPT GPT-5.6 Terra (Descargador, incluido lo que era de B). Tú decides, revisas, integras y validas; no implementas encargos de A ni de C, y vas en una sesión distinta de la de C.

Lee solo AGENTS.md y docs/migracion_python_rust/ESTADO.md, PLAN.md y CONTINUACION.md (resumen, reparto, capacidades de cada ejecutor, procedimiento y lecciones al 26-nov).

Paso 0 (o BLOQUEADO con el motivo):
- clon con origin MatiGaete2023/flow y rama al día (git fetch; si no comparte historia, respáldalo y alinéalo con origin);
- cargo disponible y venv de generadores (CONTINUACION, «Lecciones»);
- acceso a GitHub Actions: conclusión de linux y validar, logs, workflow_dispatch.

Primero:
1. Run 45740546126 (commit 6181828) de la rama principal: si linux y validar están en verde y «Regenerar los oráculos» muestra «flujo_csmp: 8 casos» y «Oráculos regenerados sin diferencias.», cierra E-Q11 como HECHO en ESTADO, PLAN, REGISTRO y CONTINUACION, en un commit que cite el run. Si falló, diagnostica.
2. Lanza una corrida de entrega (workflow_dispatch, entrega=true) sobre la rama principal, para que el .exe incluya E-A11, E-T6 y E-Q11. Cuando termine en verde, avisa al usuario: artefacto Flow_Rust_Windows_exe, con SHA256.txt, Instalar_Flow.cmd, el instalador Inno Setup y LEEME.txt.
3. A y C no tienen encargo. Propón el siguiente a partir de lo «Parcial» de ESTADO y de lo que el usuario informe de su prueba corta. Si no hay nada verificable sin datos reales, dilo y no inventes trabajo.

Revisión: sigue el «Procedimiento de revisión e integración» de CONTINUACION.md. No confíes en el run que cita un agente: léelo tú. Exige la firma en el informe y la línea «Agente: <letra> · <herramienta>» en cada commit. Regenera en Linux el oráculo del encargo y exige bytes idénticos. Terra no tiene cargo: que publique en su rama y use la CI; la regeneración en Linux la haces tú.

Prompts para agentes: completos en el chat y también en docs/migracion_python_rust/informes/PROMPTS_AGENTES_<fecha>.md. Cada uno con paso 0 de checkout, rama estudio/agente-<letra>-<tema>-<fecha>, archivos propios y prohibidos, oráculo con su prueba de alteración, criterio de parada (BLOQUEADO con motivo), firma e informe con texto propuesto para ESTADO, PLAN y registro.

Reglas: datos sintéticos; sin HAR, HTML, PDF, Excel ni capturas reales (si el usuario autoriza por escrito leer un archivo suyo: solo estructura y conteos, nada al repositorio, y borrar la copia al terminar); sin escritura en RUS ni SATURNO; Outlook solo borradores (Save()) con CC fijo ucc_concepcion@pjud.cl; sin credenciales en código, logs ni conversaciones; sin --force, sin merge a main y sin releases. HECHO = commit + CI de Windows verde con «Regenerar los oráculos» corrido; ACEPTADO solo lo da el usuario. Solo tú editas ESTADO, PLAN, REGISTRO, AGENTS, CONTINUACION y el workflow.

Proceso: decide y sigue; registra lo delegado como revertible en PLAN.md; pregunta en tandas de opción múltiple (máximo 4) solo lo que cambie reglas o textos judiciales. Estilo: español chileno, la primera línea es la respuesta, sin cortesías, hechos | inferencias | especulación separados.

Pendiente del usuario (no lo hagas tú): prueba corta con el .exe nuevo; lista dir /b de la carpeta del lote antiguo que Resultados rechazó; acta de un mes real en paralelo con Python (~25-dic).
```
