# Migración Python → Flow Rust

- Desarrollar la migración sobre `app/`, tomada de la carpeta local Flow Rust. La rama remota es `estudio/paridad-python-rust-20261119`.
- Leer [la entrada de migración](MIGRACION_RUST.md), [el diseño](docs/migracion_python_rust/DISENO.md) y [el registro](docs/migracion_python_rust/REGISTRO_IMPLEMENTACION.md) antes de modificar la aplicación.
- Conservar `app/src/main.rs`, `app/src/ui.rs` y `app/src/theme.rs` como base de navegación y diseño. Integrar capacidades en las vistas existentes o extenderlas con sus componentes.
- Usar Flow Python para reglas, contratos, flujos y casos de prueba. El workspace histórico `rust/` aporta lógica y evidencia transferibles.
- Usar datos sintéticos. El Asistente funciona con entradas locales. El Descargador lee con la sesión del usuario. RUS y SATURNO quedan en solo lectura y Outlook se limita a borradores con el CC obligatorio.
- Actualizar el registro con código, pruebas, pendientes y siguiente paso antes de cerrar cada sesión. Publicar en la rama indicada, sin fusionar a `main` ni crear releases. El acta de uso real corresponde al usuario.

## Roles

- Las IA ejecutoras toman un encargo de [ENCARGOS_IA.md](docs/migracion_python_rust/ENCARGOS_IA.md), entregan evidencia con su plantilla y lo dejan `ENTREGADO`. La planificación y la auditoría repiten las comprobaciones y son las únicas que cambian el estado. Un `PASA` sin commit, comando y salida literal no cuenta.

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

## Verificación del proyecto Python de referencia

`python tools/verificar_wp.py` decide PASA/FALLA contra `docs/implementacion/linea_base.json`.
Desde WP-11, el hook y la CI controlan identidades tras la limpieza de WP-10.
