# Registro de implementación · Flow Rust

Entradas desde el 24 de noviembre de 2026. Lo anterior (turnos 1 a 17, auditorías y entregas E-B a E-Q6) está en [historico/REGISTRO_hasta_20261124.md](historico/REGISTRO_hasta_20261124.md). El estado vigente y el siguiente paso están en [ESTADO.md](ESTADO.md); este archivo no se lee al iniciar.

## Formato

Una entrada por encargo, de diez líneas como máximo, la más reciente arriba:

```
## <fecha> · <encargo> · <commit>
- Cambió: <archivos o módulos>
- Acredita: <prueba u oráculo y su conteo>
- Diferencias declaradas: <ninguna | detalle>
- Pendiente: <ninguno | detalle>
```

No se copian salidas completas ni listas de mutaciones: la prueba afirma sus conteos y la CI del commit es la evidencia.

## 2026-11-24 · Consolidación de la documentación y decisiones D-5 a D-7

- Cambió: `ESTADO.md` (nuevo), `PLAN.md` y `VERIFICACION.md` (reescritos), este registro (rotado), `AGENTS.md`, `MIGRACION_RUST.md`, `CONTINUAR.md` y `tools/probar_app.sh` (nuevo). Catorce documentos pasaron a `historico/`. En `app/`, solo el mensaje de `prueba_entorno.rs`, que ahora remite a `VERIFICACION.md`.
- Acredita: `tools/probar_app.sh todo` en Linux, `58 passed` y `43 passed; 10 filtered out`, 0 fallos.
- Decisiones del usuario: D-5 (el Descargador recolecta y Flow procesa), D-6 (F27 sigue en el alcance) y D-7 (orden Descargador, exportaciones, Asistente). Detalle en [PLAN.md](PLAN.md).
- Pendiente: D-8; E-V1 a E-V3. No se eliminó ningún archivo ni se cambió la CI.

## 2026-11-24 · Revisión del plan y de la documentación · `751634b`

- Cambió: [REVISION_20261124.md](REVISION_20261124.md). Sin cambios de código.
- Hallazgos: los oráculos de `app/qa` usan el Python de esta rama, que difiere de `flow-python` (`afc4d12`) en `descargador/flujo_paquete.py` y `src/nurus/personal/revision_bitacoras.py`; la matriz tenía capacidades pendientes sin encargo; la suite corre en Linux con `--features eframe/x11`, sin `.aislado`.

## 2026-11-24 · E-Q5 y E-Q6 · `61b307c` a `09448d4`

- Cambió: `descargador/paquete.rs`, `descargador_ui.rs` y `descargador/preferencias.rs`.
- Acredita: `PAQUETE: 5/5` contra `export_capture` y `package_current_lot`; `ui_descargador_bitacoras_y_paquete`.
- Diferencias declaradas: las de [ESTADO.md](ESTADO.md).
- Pendiente: E-Q5d y E-Q7.
