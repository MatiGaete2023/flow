# Continuación · sesión del 25 de noviembre de 2026

Para abrir un chat nuevo como **orquestador**. La fuente del estado es [ESTADO.md](ESTADO.md); este archivo resume la sesión, el reparto de agentes, el procedimiento de revisión, las lecciones y el prompt de arranque.

## Rol y reparto

Claude es el **orquestador y coordinador**: decide, integra y valida. Tres agentes de otras familias (ChatGPT, DeepSeek y Gemini) trabajan en ramas propias; el usuario asigna quién es quién y cambiarlo a mitad de un encargo no conviene.

| Agente | Rama | Encargo | Archivos propios |
|---|---|---|---|
| A | `estudio/paridad-python-rust-20261119` | E-A10 (anulaciones de columnas de origen por fila), luego E-P1 (Word por tribunal con índice) y E-I1 (instalador) | `trabajo.rs`, `ui.rs`, `sincronizar.rs`, `conciliar.rs`, `productos.rs`, `producto_excel.rs`, `copia_excel.rs`, `personalizar.rs`, instalador |
| B | `estudio/agente-b-meta4-20261125` | Meta 4: ampliar el contraste con Python (Espera, FAE, otros tribunales, regeneración, plantilla) | `descargador/meta4.rs`, `meta4_ui.rs`, `generar_meta4.py`, `meta4.json` |
| C | `estudio/agente-c-parciales-20261125` → `estudio/agente-c-conjuntos-20261125` | E-Q8 (entregado e integrado); E-S2: conjuntos `flujo.json` de Flow Python en Resultados | E-S2: `descargador/archivo_local.rs` (nuevo), `descargador_resultados_ui.rs`, `generar_archivo_local.py` y `archivo_local.json`, la línea `archivo_local` del workflow y su línea del inventario |

Los prompts completos están en [informes/PROMPTS_AGENTES_20261125.md](informes/PROMPTS_AGENTES_20261125.md). Cada agente entrega un informe en `informes/AGENTE_<letra>_<fecha>.md` y no edita `ESTADO`, `PLAN`, `REGISTRO`, `AGENTS`, `CONTINUACION` ni el workflow.

**Estado a las 16:45 UTC del 25-nov:** E-Q8 `HECHO` (run 45652560712). E-A10 espera la corrida manual que regenera todos los oráculos. **Antes:** C entregó E-Q8 (informe `AGENTE_C_20261125.md`) y quedó integrado (`3689c92`); su siguiente encargo es E-S2 (abrir conjuntos `flujo.json` de Flow Python en Resultados, prompt en `informes/PROMPTS_AGENTES_20261125.md`), en una rama nueva desde la principal. A publicó E-A10 (`faee8ea`) con la CI de Windows en rojo por una carrera entre pruebas; A y el orquestador la corrigieron en paralelo; quedó la de A (`886d222`). B aún no publica.

## Procedimiento de revisión e integración

1. Por rama con informe: leerlo, comprobar la CI de su rama (`linux`, `validar` y «Regenerar los oráculos» corrido si cambió un generador) y `git diff 9c9ca7e..<rama> --stat` para confirmar que solo tocó sus archivos.
2. Integrar de a una a la rama principal con `git merge --no-ff`; resolver conflictos; correr `tools/probar_app.sh todo`, clippy de la biblioteca y los verificadores.
3. Pasar el texto propuesto a `ESTADO`, `PLAN` y `REGISTRO`; publicar y esperar la CI de Windows en la rama principal: recién ahí el encargo queda `HECHO`.
4. Lo que un agente marque como ambiguo lo decide el orquestador, salvo lo que cambie reglas o textos judiciales, que se pregunta al usuario.

## Qué se hizo

Cerrados con CI de Windows en verde (`HECHO`; «aceptado» solo lo da el usuario):

| Encargo | Run |
|---|---|
| E-R1, E-R2 (Meta 4 desde lotes del Descargador, vista) y E-X1, E-X2 (guardia de solo lectura, diagnóstico de pestañas) | 45487699648 |
| E-A6 importación de contactos (catastro Excel y JSON) | 45503587810 |
| E-A7 recálculo al editar columnas de origen en la copia | 45625544370 |
| E-A8 gestión de correo por fila y `NURUS_DECISIONES` | 45634055296 |
| E-A9 recálculo en trabajos de paquete | 45639115067 |
| Reversión de D-10 y D-11 por decisión del usuario | 45637998137 |
| E-I2 actualización en el lugar y extensión de carpeta fija (`5f21612`) | 45644045890 |
| E-U2 correcciones de la primera prueba con el `.exe` (desplazamiento en Descargador y Meta 4, aclaración de «Validar perfil», mensajes de carpeta de lote) (`6d7fc27`) | 45648680625 |

E-U2 falta confirmarlo con el `.exe` nuevo. Primera prueba del usuario con el `.exe` de E-I2: el Descargador no se podía usar con la ventana pequeña (corregido), «Validar perfil» es una consulta única (ahora se explica) y «Resultados del lote» rechazó un lote antiguo de Flow Python con «falta resumen.json»: Python abre carpetas con `resumen.json`, `flujo.json` (conjunto «Flujo CSMP …») o `bitacoras.json`, y Rust solo la primera. Falta la lista de nombres de archivos (`dir /b`) de esa carpeta para saber si es un conjunto y abrir sus fases.

**Decisiones del usuario (25-nov):** «módulo» con tilde; D-10 y D-11 revertidas; D-12 diferida; D-13 conservar las hojas del crate de Meta 4; D-5 confirmada; Word por tribunal con índice y salto de página (E-P1); Meta 4 solo con lotes del Descargador; de F08 solo las anulaciones de origen (E-A10); instalador (E-I1); Anonimizador sin conteo por tipo; Word manual sin paginación; M6 y M7 descartados; acta con un mes real en paralelo con Python; el orquestador decide y sigue, registrando lo delegado como revertible. Actualización sin reinstalar: carpeta local y botón (E-I2).

## Modelos recomendados para los agentes (datos del 25-nov, sin verificar en las páginas oficiales)

El perfil de las tareas: Rust y Python en un repositorio con muchas reglas, oráculos deterministas, lectura de logs de CI y disciplina con límites de archivos. No exigen razonamiento de frontera; sí seguir instrucciones largas sin salirse de ellas.

| Familia | Opción económica recomendada | Para qué | Escalar a |
|---|---|---|---|
| OpenAI | **GPT-5.6 Luna** (entrada ~US$0,20 y salida ~US$1,20 por millón tras la rebaja de julio) | Agente con más archivos y pasos (tipo A) y cualquier tarea de código del día a día | **GPT-5.6 Terra** (~US$2 / US$12) si Luna se atasca; **Sol** (~US$4–5 / US$20–30; la promoción tenía fecha, ver abajo) solo para lo más difícil. Reemplaza a GPT-5.4 mini, que era mi recomendación inicial por no haber visto la familia 5.6 |
| DeepSeek | **V4 Flash** (entrada ~US$0,14 y salida ~US$0,28 por millón; hay tarifa por hora punta y alzas reportadas) | Implementación guiada por especificación (tipo B); modo con razonamiento para lo difícil | V4 Pro solo si Flash falla (precio con cifras contradictorias) |
| Google | **Gemini 3.5 Flash** (entrada US$0,75 y salida US$4,50 por millón, incluye razonamiento) | Lectura y análisis de muchos archivos con contexto largo (tipo C) | Gemini Pro; Flash-Lite (~US$0,25–0,30 de entrada) solo para inventarios de lectura |

**Familia GPT-5.6 (Sol, Terra, Luna).** Los tres comparten contexto de ~1,05 millones de tokens. Un índice de terceros de agentes de código los ubica cerca (Sol ~80, Terra ~77, Luna ~75; las cifras varían entre fuentes) mientras Luna cuesta alrededor de una veinteava parte de Sol en entrada: para este repositorio, Luna es la mejor relación. El precio de Sol tenía una promoción hasta al menos el 21-nov y una publicación habla de volver al precio normal el 30-nov. La página de precios de OpenAI también lista variantes `gpt-6` que no verifiqué: comprobar cuál está disponible en el plan del usuario.

Hechos (N1, de las páginas consultadas): los modelos y precios de arriba aparecen en resultados de las páginas de precios de cada proveedor y de terceros; los de DeepSeek y Gemini tienen cifras que se contradicen y promociones con fecha. Inferencias (N2): el reparto por tipo de tarea sale de lo que describe cada proveedor y de la matriz de roles del usuario (ChatGPT arquitecto, DeepSeek implementador de especificación, Gemini procesador de volumen); no probé a ninguno con este repositorio. Especulación (N3): que un modelo económico cumpla el protocolo de ramas, informes y CI sin supervisión. **Cómo acotar el riesgo:** el orquestador revisa cada rama con el procedimiento de arriba y no integra nada sin CI verde. Privacidad: a ningún agente se le dan archivos reales; DeepSeek se opera desde servidores del proveedor, así que conviene mantener lo que reciba solo en especificación técnica.

**Modelos gratis de OpenCode (Zen).** La lista cambia seguido (en las fuentes aparecen DeepSeek V4 Flash, Nemotron 3 Ultra, North Mini Code, MiMo, MiniMax M3, Big Pickle y otros) y hay límites diarios y de tasa que, según reportes, se agotan en minutos. Úsense solo para tareas pequeñas, verificables por pruebas o CI y sin datos reales; no para el agente A ni para sesiones largas. **Por verificar antes de enviarles código del repositorio:** si los prompts se usan para mejorar modelos (la documentación dice que algunos son gratis «mientras se recoge opinión»); la página oficial no se pudo leer desde la sesión del orquestador. La lista vigente sale de `/models` en OpenCode.

Fuentes: [precios de OpenAI](https://developers.openai.com/api/docs/pricing), [rebaja de Terra y Luna](https://community.openai.com/t/announcing-a-major-price-drop-for-5-6-terra-and-luna-and-fast-mode-for-5-6-sol/1388484), [GPT-5.6](https://openai.com/index/gpt-5-6/), [precios de DeepSeek](https://api-docs.deepseek.com/quick_start/pricing), [precios de Gemini](https://ai.google.dev/gemini-api/docs/pricing). Revisarlas antes de contratar.

## Lecciones operativas

- Los oráculos se regeneran en Windows y se comparan con lo versionado: deben ser **deterministas y sin depender del sistema**. Escribir bytes (no `write_text`), fijar fechas (`date.today()` fijo en `generar_correos.py`), `ZipInfo.create_system = 3` y fecha de modificación fija en los `.xlsx` de openpyxl.
- Tras publicar, confirmar que el paso «Regenerar los oráculos» **corrió** (no «skipped») además de ver el run en verde. Un commit que solo toca código Rust lo omite: si el encargo trae oráculos nuevos y su run de regeneración no corrió, lanzar la CI a mano (`workflow_dispatch` regenera todos). Pasó con E-A10. La primera corrida «todos» destapó que `generar_lote.py` volcaba JSON con CRLF en Windows: todo archivo de texto escrito por Python que se pase a base64 se normaliza a LF.
- Un agente sin checkout válido no arranca: cada prompt empieza con el «paso 0» (clon con `origin` MatiGaete2023/flow, rama y commit verificados, o BLOQUEADO).
- Un `canonicalize()` distinto del nombre elegido (nombre corto `RUNNER~1`, enlaces) rompía la verificación de bitácoras solo en Windows: normalizar las rutas de entrada.
- No hacer trabajo pesado por cuadro en la interfaz (la huella del ejecutable en Ajustes congeló las pruebas).
- Entorno local para los generadores de Python que arman paquetes (necesitan `tkinter`): `apt-get update && apt-get install -y python3-tk`, y un `venv` con `/usr/bin/python3.12 --system-site-packages` más `openpyxl pandas docxcompose python-docx xlrd lxml pillow numpy reportlab pytest`. El `python3` por omisión (3.13) no ve `tkinter`.
- `tools/probar_app.sh todo` tarda unos 20 s; clippy: `--lib -- -D warnings` (el binario tiene código muerto anterior que no es parte del gate).
- Revisar la CI con las herramientas MCP de GitHub (`actions_list`, `get_job_logs`); no hay `gh` salvo para búsquedas.

## Firma del instalador y dónde se aloja

**Firma.** Un certificado digital que firma el `.exe` o el instalador. Windows (SmartScreen) y los antivirus institucionales comprueban quién lo publicó y que no cambió. Sin firma, lo habitual es el aviso «Windows protegió su PC / editor desconocido» (se salta con «Más información → Ejecutar de todas formas»); en equipos con política estricta (AppLocker o antivirus de la institución) puede bloquearse del todo.

Opciones, de menos a más trabajo:

1. **Sin firma + huella SHA-256 (lo de hoy).** La CI entrega `SHA256.txt` y Flow verifica la huella antes de actualizarse. Sirve si el equipo permite ejecutar archivos sin firma.
2. **Certificado propio (autofirmado).** Gratis; solo es de confianza si el área de informática del equipo lo instala como «editor de confianza». Es lo común en instituciones.
3. **Certificado comercial de firma de código.** Se compra (pago anual, precio por verificar) y evita el aviso de SmartScreen. Desde 2023 la llave debe estar en un dispositivo seguro o en un servicio en la nube, lo que complica firmar desde la CI.

**Alojamiento.** Dónde queda el instalador o el `.exe` nuevo:

- **Hoy:** artefacto de GitHub Actions (se conserva 14 días, pide iniciar sesión, se baja a mano).
- **GitHub Releases:** URL estable y permanente. `AGENTS.md` prohíbe crear releases sin autorización escrita; con el repositorio privado, bajar sin credenciales no es posible, y hacerlo público no es adecuado (el código describe procesos del tribunal).
- **Carpeta local de actualizaciones (decidido para E-I2):** el usuario deja el archivo; sin red ni credenciales.
- **Carpeta compartida o de red institucional:** sirve si hay varios equipos; depende de las políticas de TI.

**Recomendación (inferencia, no probada):** no decidir todavía. La prueba corta con el `.exe` dirá si el equipo lo bloquea. Si no lo bloquea, seguir con la opción 1. Si lo bloquea, pedir a informática que confíe en un certificado propio (opción 2), que resuelve también el instalador. Para el alojamiento, mantener la carpeta local y evaluar la carpeta de red solo si aparecen más equipos.

## Prompt para el chat nuevo (orquestador)

```
Eres el orquestador y coordinador del proyecto de migración Python → Flow Rust en MatiGaete2023/flow. Tres agentes (de las familias ChatGPT, DeepSeek y Gemini) trabajan en ramas propias; tú decides, revisas, integras y validas. Lee solo AGENTS.md y docs/migracion_python_rust/ESTADO.md, PLAN.md y CONTINUACION.md (esta última tiene el reparto, el procedimiento de revisión, las lecciones y los pendientes).

Primero: con git fetch revisa las ramas estudio/paridad-python-rust-20261119 (agente A), estudio/agente-b-meta4-20261125 (B) y estudio/agente-c-parciales-20261125 (C): commits nuevos e informes en docs/migracion_python_rust/informes/. Para cada rama con informe sigue el «Procedimiento de revisión e integración» de CONTINUACION.md. Revisa también la CI de Windows del último commit de la rama principal con las herramientas MCP de GitHub y cierra los estados que falten (E-U2 está HECHO con el run 45648680625).

Reglas: datos sintéticos; sin HAR/HTML/PDF/Excel/capturas reales; sin escritura en RUS/SATURNO; Outlook solo borradores con CC fijo; sin --force, sin merge a main y sin releases. HECHO = commit + CI de Windows verde con «Regenerar los oráculos» corrido; ACEPTADO solo lo da el usuario. Commits con las líneas de atribución del sistema. Solo tú editas ESTADO, PLAN, REGISTRO, AGENTS, CONTINUACION y el workflow.

Proceso: decide y sigue; registra lo delegado como revertible en PLAN.md; pregunta al usuario en tandas de opción múltiple (máx. 4) solo lo que cambie reglas o textos judiciales. Estilo: español chileno, primera línea = la respuesta, sin cortesías, hechos | inferencias | especulación separados, código completo.

Pendiente del usuario (no lo hagas tú): la lista de nombres de archivos (dir /b) de la carpeta del lote antiguo de Flow Python que Resultados rechazó; confirmar que el .exe nuevo (artefacto del run 45648680625, en %LOCALAPPDATA%\FlowRust\actualizaciones) se desplaza en Descargador y Meta 4; diagnósticos de paquetes parciales cuando existan; acta de un mes real en paralelo con Python (~25-dic); firma y alojamiento del instalador (se decide tras la prueba del .exe); asignar qué modelo corre cada agente.
```
