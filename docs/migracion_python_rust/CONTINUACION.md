# Continuación · sesión del 25 de noviembre de 2026

Para abrir un chat nuevo. La fuente del estado sigue siendo [ESTADO.md](ESTADO.md); este archivo solo resume lo que hizo esta sesión, lo aprendido y la explicación de firma y alojamiento.

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

**En curso:** E-I2 (actualización en el lugar y extensión de carpeta fija), commit `5f21612`, run 45644045890. Pasar a `HECHO` solo si `linux` y `validar` quedan en verde.

**Decisiones del usuario (25-nov):** «módulo» con tilde; D-10 y D-11 revertidas; D-12 diferida; D-13 conservar las hojas del crate de Meta 4; D-5 confirmada; Word por tribunal con índice y salto de página (E-P1); Meta 4 solo con lotes del Descargador; de F08 solo las anulaciones de origen (E-A10); instalador (E-I1); Anonimizador sin conteo por tipo; Word manual sin paginación; M6 y M7 descartados; acta con un mes real en paralelo con Python; el asistente decide y sigue, registrando lo delegado como revertible. Actualización sin reinstalar: carpeta local y botón (E-I2).

**Orden de trabajo:** E-A10 → E-P1 → E-I1.

## Lecciones operativas

- Los oráculos se regeneran en Windows y se comparan con lo versionado: deben ser **deterministas y sin depender del sistema**. Escribir bytes (no `write_text`), fijar fechas (`date.today()` fijo en `generar_correos.py`), `ZipInfo.create_system = 3` y fecha de modificación fija en los `.xlsx` de openpyxl.
- Tras publicar, confirmar que el paso «Regenerar los oráculos» **corrió** (no «skipped») además de ver el run en verde.
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

## Prompt para el chat nuevo

```
Continúa la migración Python → Flow Rust en MatiGaete2023/flow, rama estudio/paridad-python-rust-20261119. Lee solo AGENTS.md, docs/migracion_python_rust/ESTADO.md, PLAN.md y CONTINUACION.md.

Reglas: datos sintéticos; sin HAR/HTML/PDF/Excel/capturas reales; sin escritura en RUS/SATURNO; Outlook solo borradores con CC fijo; sin --force ni merge a main ni releases. Al cerrar un encargo: actualizar ESTADO.md y agregar ≤10 líneas a REGISTRO_IMPLEMENTACION.md en el mismo commit, publicar en la rama y esperar la CI de Windows en verde (HECHO = commit + CI verde, con «Regenerar los oráculos» corrido; ACEPTADO solo lo da el usuario). Commit con las líneas de atribución del sistema.

Primero: revisa el run 45644045890 (commit 5f21612, E-I2) con las herramientas MCP de GitHub. Si linux y validar están en verde, pasa E-I2 a HECHO en ESTADO.md, PLAN.md y REGISTRO y publica. Si falla, lee el log y corrige.

Luego sigue el orden decidido: E-A10 (anulaciones de columnas de origen por fila, F08; antes de escribir código lee _prepass_peers y apply(overrides=…) en src/nurus/personal/record_edits.py, con oráculo contra Python y su prueba permanente que altera lo esperado), después E-P1 (Word por tribunal con índice y salto de página) y E-I1 (instalador, que respeta la carpeta de actualizaciones de E-I2).

Proceso: decide y sigue; registra las decisiones delegadas como revertibles en PLAN.md; pregunta al usuario en tandas de opción múltiple (máximo 4 preguntas por tanda). Estilo: español chileno, primera línea = la respuesta, sin cortesías, hechos | inferencias | especulación separados, código completo.

Pendiente del usuario (no los hagas tú): prueba corta del tramo 1 con el .exe (espera que termine de preparar GitHub Actions), diagnósticos de lecturas parciales (dependen de la prueba corta), acta de un mes real en paralelo con Python (en ~1 mes) y la decisión de firma y alojamiento del instalador (se decide tras la prueba corta).
```
