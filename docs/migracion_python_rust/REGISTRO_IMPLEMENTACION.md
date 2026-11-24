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

## 2026-11-24 · E-V1 y E-V2

- Cambió: `descargador/`, `src/` y `tests/` igualados a `flow-python` `afc4d12` (9 archivos); `app/qa/REFERENCIA_PYTHON.json`, `tools/verificar_referencia_python.py` y la CI (checkout del SHA y verificación); `bitacoras.json` y `paquete.json` regenerados; `prueba_entorno.rs` con rutas por omisión; `x11` para Linux en `app/Cargo.toml`; `contrastar_bitacoras.py` pide `--include-ignored`.
- Acredita: `tools/probar_app.sh todo`; contraste V2 local contra `afc4d12` (10 casos V1 y 5 P2); `verificar_referencia_python.py` en PASA.
- Diferencias declaradas: ninguna nueva. Rust ya seguía la revisión de bitácoras de `afc4d12`.
- Pendiente: CI de Windows; E-V3.

## 2026-11-24 · E-S1 · `d56c1ed` y el commit de cierre

- Cambió: `descargador/resultados.rs` (CSV, ICS, ZIP de entrega y comparador), `excel.rs` (hoja con formato de cuerpo), oráculo `app/qa/generar_resultados.py` → `app/tests/datos/resultados.json`, agregado a la CI.
- Acredita: `PRODUCTOS: 13/13`; pruebas permanentes `oraculo_de_productos_detecta_alteraciones` y `comparador_de_excel_detecta_alteraciones`; `tools/probar_app.sh todo`.
- Diferencias declaradas: sin `include_originals` (ninguna ruta de Python lo activa).
- Pendiente: CI de Windows; ofrecer los productos en la vista de Resultados.

## 2026-11-24 · D-8: retiro de la documentación Python de esta rama

- Cambió: se retiraron 656 archivos de `docs/` y, de la raíz, `CONTINUAR.md`, `PLAN_VIGENTE.md` y `RESUMEN_PROYECTO_NURUS_CSMP_20261009.md`. `README.md` reescrito. `docs/` pasó de 36 MB a 1,2 MB.
- Se conservan: `docs/migracion_python_rust/`, `docs/aceptacion/`, `docs/evidencia_p0/golden/`, `docs/implementacion/linea_base.json` y `docs/implementacion/verificacion/FLOW_D.json` (los usa `rust/`).
- Acredita: `tools/probar_app.sh todo` sin fallos; `check_real_ids`, `check_contactos`, `verificar_textos_umbrales` e `inventario_escrituras` en PASA.
- Diferencias declaradas: 18 archivos de pruebas y herramientas Python citan documentos retirados; esas pruebas se ejecutan en `flow-python`. No se ejecutó la suite Python en esta rama.
- Pendiente: flujos de CI de otras ramas, `rust/*.md`, `prototipos/` y el árbol Python (tras E-V1) siguen sin tocar.

## 2026-11-24 · E-Q5d y parte de E-S1 · `c734f84` (otra sesión)

- Cambió: `descargador/excel.rs` y `descargador/resultados.rs` (nuevos), `paquete.rs`, `xls.rs`, `descargador_ui.rs` y el oráculo `app/qa/generar_excel.py`.
- Acredita: `EXCEL_BITACORAS: 3/3` y `LIBROS: 14/14`. Repetido tras la fusión con la consolidación: `tools/probar_app.sh todo`, `61 passed` y `43 passed`, 0 fallos.
- Pendiente: CI de Windows de ese commit; de E-S1 faltan CSV, ICS, ZIP y el comparador. Esa sesión no dejó entrada de registro; esta la escribió la sesión de consolidación a partir del mensaje del commit.

## 2026-11-24 · Consolidación de la documentación y decisiones D-5 a D-7

- Cambió: `ESTADO.md` (nuevo), `PLAN.md` y `VERIFICACION.md` (reescritos), este registro (rotado), `AGENTS.md`, `MIGRACION_RUST.md`, `CONTINUAR.md` y `tools/probar_app.sh` (nuevo). Catorce documentos pasaron a `historico/`. En `app/`, solo el mensaje de `prueba_entorno.rs`, que ahora remite a `VERIFICACION.md`.
- Acredita: `tools/probar_app.sh todo` en Linux, `58 passed` y `43 passed; 10 filtered out`, 0 fallos (antes de fusionar `c734f84`).
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
