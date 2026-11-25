# Migración Python → Flow Rust

- Desarrollar la migración sobre `app/`. La rama remota es `estudio/paridad-python-rust-20261119`.
- Leer al iniciar, y nada más: [el estado](docs/migracion_python_rust/ESTADO.md) y [el plan](docs/migracion_python_rust/PLAN.md). Consultar [la verificación](docs/migracion_python_rust/VERIFICACION.md) y [el diseño](docs/migracion_python_rust/DISENO.md) cuando el encargo lo pida. `docs/migracion_python_rust/historico/` y el resto de `docs/` no se leen salvo que el encargo los cite.
- Conservar `app/src/main.rs`, `app/src/ui.rs` y `app/src/theme.rs` como base de navegación y diseño. Integrar capacidades en las vistas existentes o extenderlas con sus componentes.
- Usar Flow Python para reglas, contratos, flujos y casos de prueba. El workspace histórico `rust/` aporta lógica y evidencia transferibles.
- Usar datos sintéticos. El Asistente funciona con entradas locales. El Descargador lee con la sesión del usuario. RUS y SATURNO quedan en solo lectura y Outlook se limita a borradores con el CC obligatorio.
- Al cerrar un encargo, en el mismo commit: actualizar `ESTADO.md` y agregar una entrada de diez líneas como máximo al [registro](docs/migracion_python_rust/REGISTRO_IMPLEMENTACION.md). Publicar en la rama indicada, sin fusionar a `main` ni crear releases. El acta de uso real corresponde al usuario.

## Encargos y evidencia

- La sesión que implementa un encargo también lo prueba. Estados: `ABIERTO` → `HECHO` (commit y CI en verde) → `ACEPTADO` (solo el usuario).
- Un `HECHO` exige una prueba en el repositorio que afirme sus conteos y que falle si faltan sus datos. No citar pruebas, funciones ni archivos que no estén en el commit publicado.
- Si un criterio no se puede cumplir, se informa como `BLOQUEADO` con el motivo.

## Trabajo en paralelo (agentes coordinados)

- La rama `estudio/paridad-python-rust-20261119` la integra el coordinador. Un agente en paralelo trabaja en su propia rama `estudio/agente-<letra>-<tema>-<fecha>` (la CI también corre ahí) y **no toca** `ESTADO.md`, `PLAN.md`, `REGISTRO_IMPLEMENTACION.md`, `AGENTS.md`, `CONTINUACION.md` ni el workflow.
- En lugar de eso escribe `docs/migracion_python_rust/informes/AGENTE_<letra>_<fecha>.md` con lo hecho, las pruebas, los runs de CI y el texto propuesto para `ESTADO.md`, `PLAN.md` y el registro; el coordinador lo revisa, lo integra y valida de nuevo en la rama principal.
- Cada agente tiene archivos propios (los fija su prompt). Una función compartida no cambia de firma sin avisar en el informe.
- **Ejecutores fijos (25-nov, usuario):** A = Antigravity, B = DeepSeek, C = ChatGPT (GPT-5.6 Luna). Un agente no toma encargos de otra letra.
- **Firma obligatoria.** Cada informe empieza con un bloque «Firma» (agente y letra, herramienta y modelo, identidad git `nombre <correo>` con la que firmó los commits, ruta del clon, rama, y la lista de commits que hizo esa sesión, uno por línea con su SHA). Cada commit lleva al final la línea `Agente: <letra> · <herramienta>`. Un commit que el informe no lista no se atribuye a ese agente; un informe sin firma no se integra.

## Entrega y actualización

- Toda corrección se entrega de modo que se aplique **sin reinstalar**: el ejecutable se reemplaza desde `%LOCALAPPDATA%\FlowRust\actualizaciones` (aviso y botón en Flow, con verificación de SHA-256) y nada que no haya cambiado se vuelve a instalar.
- La extensión de Chrome vive en una carpeta fija (`extension-integral`, `extension-lite`) y solo se toca si cambia su propio código; entonces se actualiza en el lugar y en Chrome basta «Recargar».
- La CI entrega el `.exe` con su `SHA256.txt` (huella y commit) listo para dejar en esa carpeta.

## Presupuesto de pruebas

- Mientras se trabaja: `tools/probar_app.sh rapido <filtro>` sobre el módulo tocado. Antes de publicar: `tools/probar_app.sh todo`, una vez. La CI hace el resto; no repetir en local lo que la CI ya corre.
- No requieren prueba nueva: documentación, registro, rótulos y código de prueba.
- Un oráculo nuevo lleva una sola prueba permanente que altera el dato esperado y exige que el comparador falle. Sin mutaciones manuales.
- Los verificadores estáticos (`check_real_ids`, `check_contactos`, `verificar_textos_umbrales`, `inventario_escrituras`) se corren solo si el cambio toca lo que revisan.
- Las pruebas de interfaz se limitan a los flujos; la interfaz la valida el usuario.
- Filtrar las salidas (`test result`, `FAILED`, `panicked`) y no copiarlas al registro.

## Reglas que no se negocian

- Sin escritura automática en RUS ni SATURNO. Sin módulo de registro RUS ni SATURNO; si reapareciera, `submit`
  debe lanzar error.
- Outlook solo borradores (`mail.Save()`); nunca `.Send(`. CC fijo `ucc_concepcion@pjud.cl`.
- Sin credenciales, cookies ni tokens en código, logs, fixtures o conversaciones.
- **Sin HAR, HTML, PDF, Excel ni capturas reales; solo estructura anónima** generada
  localmente (`descargador/extraer_estructura_har.py`, `diagnostico_bitacoras.json`,
  `tools/anonimizar_excel.py`). Si alguien ofrece un archivo real, pedir el extracto anónimo.
- Reglas, umbrales y textos tradicionales intactos (`src/nurus/personal/motor/`,
  `textos_base.json`) salvo decisión expresa del usuario y paridad golden.
- El Asistente no navega SITFA ni inicia el Descargador.
- No marcar nada como «aceptado» o «institucional»: solo el usuario, en un acta.
- Sin `--force`, sin fusionar a `main` y sin releases salvo autorización escrita del usuario.
