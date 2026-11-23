# Prompt · Encargo E-K (R5: reiteración con el oráculo de cronología en `app/`)

Copiar desde la línea siguiente y entregar tal cual a la IA ejecutora.

---

Antes de cualquier otra cosa, ejecuta y pega la salida literal de `git log --oneline -1 && git status -sb`. Si no puedes ejecutar comandos en este repositorio, responde solo «SIN ACCESO» y detente.

Eres una IA ejecutora del proyecto Flow Rust. Tomas solo el encargo **E-K**. No planificas, no redefines criterios y no tocas otros encargos.

## Contexto mínimo

- Repositorio `MatiGaete2023/flow`, rama `estudio/paridad-python-rust-20261119`, desde el commit de la auditoría de E-J o uno posterior. No crear ramas, no usar `--force`, no fusionar a `main` y no crear releases.
- Leer antes:
  - `AGENTS.md`;
  - `docs/migracion_python_rust/ENCARGOS_IA.md`: reglas de entrega, E-K y la plantilla;
  - las secciones «Auditoría · E-B» y «Auditoría · E-J» de `docs/migracion_python_rust/REGISTRO_IMPLEMENTACION.md`;
  - la fila R5 de las tablas de `docs/migracion_python_rust/INFORME_FALLAS_20261120.md`.
- Hechos auditados (N1):
  - la reiteración (`COMUN.REITERA_OBSERVACION`) no la aplica el motor por fila, sino la cronología:
    - en Python, `bitacora_avisos.aplicar_reiteracion`;
    - en Rust, `Cronologia::aplicar_reiteracion` (`app/src/captura_trabajo/cronologia.rs`);
  - el oráculo es `rust/crates/flow-trabajo/tests/datos/cronologia.json`, generado por `rust/oraculo/generar_cronologia.py`:
    - 181 `casos` y 256 `similitudes`;
    - 18 casos terminan con `COMUN.REITERA_OBSERVACION` en `reiteracion.reglas`;
    - 1 caso trae la regla en `desactivadas`;
  - hoy solo lo consume el workspace histórico (`rust/crates/flow-trabajo/tests/cronologia.rs`); `grep -rn cronologia.json app/` no da resultados;
  - los eventos del oráculo traen `event_id`, `event_at`, `time_precision`, `time_zone`, `origin`, `origin_verified`, `coverage`, `text`, `event_type` y `response`. **No traen `captured_at`**: `cronologia::leer` de `app/` descarta las filas sin esa columna;
  - `aplicar_reiteracion` tiene dos modos:
    - `config.redaccion_registro = false` es la referencia Python (`NO_REITERAR_PYTHON`);
    - `true` aplica la decisión R5 parcial: no reiterar «Medida revisada, sin observaciones.» ni «La derivación … no se encuentra sujeta a seguimiento por este Centro.».
  - El oráculo refleja Python, así que se compara en modo `false`.

## Qué hacer

1. **Prueba de oráculo**, solo en `#[cfg(test)]` de `app/src/captura_trabajo/cronologia.rs` (o en un submódulo de prueba de ese archivo). Nombre: `oraculo_cronologia_reiteracion`.
   - Cargar `cronologia.json` desde el repositorio con `include_str!` y una ruta relativa al archivo de prueba.
   - Por cada caso:
     - construir las `Entrada` desde `eventos` y la `Cronologia` con `ordenar(entradas, desde, corte, leida)`;
     - `desde` y `hasta` son los de `display.period_from` y `display.period_to`, y `leida` sale del caso;
     - armar un `Resultado` con `observacion = propuesta`, eventos/reglas = `reglas` del caso y `config.desactivadas` = `desactivadas`, con `redaccion_registro: false`;
     - ejecutar `aplicar_reiteracion` y comparar con `reiteracion.observacion`, `reiteracion.reglas` y `reiteracion.avisos`.
   - Mapeo de campos:
     - antes de escribirlo, leer cómo Python decide que una lectura es del centro, su orden y su fecha (`bitacora_avisos.py` y `cronologia` en `src/nurus/personal/`). Documentar el mapeo en la entrega, con la línea de Python que lo respalda: `centro` desde `origin`/`origin_verified`/`coverage`, `instante` desde `time_precision`/`time_zone`/`event_at`, `al_tribunal` desde `event_type` y `sin_respuesta` desde `response`;
     - si el mapeo exige cambiar una función de producción (no solo agregar código de prueba), detenerse e informar `BLOQUEADO`, con la diferencia y la línea concreta.
   - Si el `Resultado` de `app/` no tiene un campo equivalente a `acciones`, no compararlo. Indicarlo en la entrega.
2. **Contadores**: imprimir `CRONOLOGIA: casos N, coinciden M, reiteradas R/18, desactivada D/1, fallos F` y afirmar:
   - `N == 181`;
   - `M == N`;
   - `R == 18` en las coincidencias con reiteración;
   - `F == 0`.

   Un caso que no coincide es FALLA: imprimir su índice, la observación esperada y la obtenida.
3. **Similitudes**: si `app/` tiene una función de similitud usada por la cronología, comprobar las 256 `similitudes` con tolerancia `1e-14`. Si no la tiene, anotarlo como «no aplica en `app/`» y no crearla.
4. **Decisión R5 parcial** (modo `redaccion_registro: true`): en los casos del oráculo cuya propuesta sea una de las dos excepciones, afirmar que no se reitera.
   - Contarlos: `R5_PARCIAL: excepciones X, no reiteradas X`.
   - Si `X == 0`, informar `SIN_COBERTURA`. No inventar casos en el JSON. La prueba unitaria existente `aplica_r5_al_texto_identico_y_respeta_sus_dos_excepciones` ya cubre esa decisión.
5. No tocar:
   - funciones de producción;
   - `config.rs`;
   - `textos_base.json`;
   - el motor Python;
   - `cronologia.json`.

   Si una diferencia no está aprobada, registrarla como decisión pendiente del usuario, sin ajustar el comparador.

## Cómo probar (entorno L)

Desde `app/`, con winit en Linux:

- `cargo test --locked --features eframe/x11 --bin flow_rust oraculo_cronologia -- --nocapture`
- `FLOW_CASOS=../rust/casos cargo test --locked --features eframe/x11 --bin flow_rust` (suite completa; hoy: 46 passed, 1 ignored)
- `cargo clippy --locked --features eframe/x11 --bin flow_rust --tests` (sin advertencias nuevas en `cronologia.rs`)

En Windows (W), omitir `--features eframe/x11` e informarlo.

Desde la raíz:

- `python tools/check_real_ids.py app/src`
- `python tools/check_contactos.py app/src`

## Criterios de aceptación

- `CRONOLOGIA: casos 181, coinciden 181, reiteradas 18/18, desactivada 1/1, fallos 0`, con la prueba en `app/`.
- Mapeo de campos documentado y respaldado por líneas de Python.
- Línea `R5_PARCIAL: …` literal (con cifras, o `SIN_COBERTURA`).
- La suite completa pasa: el total sube en las pruebas nuevas y no hay fallos.
- Sin cambios de lógica de producción: `git diff` fuera de `#[cfg(test)]` vacío en `app/src`.

## Entrega

- En `docs/migracion_python_rust/REGISTRO_IMPLEMENTACION.md`, agregar antes de «Siguiente paso exacto» la sección `## Entrega · E-K`, con la plantilla de `ENCARGOS_IA.md`. Incluir:
  - las líneas literales `CRONOLOGIA: …`, `R5_PARCIAL: …` y `test result: …`;
  - el mapeo de campos.
- En `ENCARGOS_IA.md`, cambiar solo el estado de E-K de `ABIERTO` a `ENTREGADO`. Nunca a `APROBADO` ni `CERRADO`.
- Commit en español, con el formato de los anteriores, y publicar en la misma rama.
- Un criterio que no se cumple se informa como `BLOQUEADO` o `FALLA` con su motivo. No relajar el comparador ni inventar valores esperados.
- No citar commits, funciones ni pruebas que no estén en tu commit:
  - comprobar cada función o prueba con `grep -rn "fn <nombre>" app/`;
  - comprobar cada commit con `git cat-file -t <hash>` después del push.

Tu respuesta final debe traer:
- la salida inicial de `git log`;
- el commit publicado;
- las líneas `CRONOLOGIA`, `R5_PARCIAL` y `test result` literales;
- el mapeo de campos;
- el resultado por criterio (PASA, FALLA o BLOQUEADO);
- los pendientes.
