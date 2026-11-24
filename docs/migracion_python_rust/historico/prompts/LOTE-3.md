# Prompt · Lote 3 (E-Q1 lectores de contexto y E-Q2 vínculos del listado)

Copiar desde la línea siguiente y entregar tal cual a la IA ejecutora.

---

Antes de cualquier otra cosa, ejecuta y pega la salida literal de `git log --oneline -1 && git status -sb`. Si no puedes ejecutar comandos en este repositorio, responde solo «SIN ACCESO» y detente.

Eres una IA ejecutora del proyecto Flow Rust. Ejecutas **E-Q1 y después E-Q2**, en ese orden. No planificas, no redefines criterios y no tocas otros encargos.

## Contexto mínimo

- Repositorio `MatiGaete2023/flow`, rama `estudio/paridad-python-rust-20261119`, desde el commit que registra D-4 o uno posterior. Sin `--force`, sin `main` y sin releases.
- Leer antes:
  - `AGENTS.md`;
  - `docs/migracion_python_rust/ENCARGOS_IA.md` (reglas de entrega, la sección «Integración completa (D-4)», E-Q1, E-Q2 y la plantilla);
  - `docs/migracion_python_rust/INVENTARIO_HERRAMIENTAS_20261123.md`.
- Decisión D-4 del usuario: integrar en `app/` todas las herramientas de Flow Python. Este lote trae a `app/` la **lectura local** de las ventanas de contexto y de los vínculos del listado. **Todavía sin red ni puente:** eso es E-Q3.
- Hechos auditados (N1, 23-nov-2026):
  - `rust/crates/flow-captura` tiene `lecturas.rs` (`leer_bitacora`, `leer_historia`, `leer_calendario`, `leer_historico`, `leer_escritos_html`, `separar_ventana`) y `vinculos.rs` (`extraer`, `descartar_ambiguos`, `identidad`), con 81 pruebas en verde **contra sus fixtures guardados**;
  - **esos fixtures están desactualizados.** `python rust/oraculo/generar_lecturas.py` con el Python vigente cambia `rust/crates/flow-captura/tests/datos/lecturas_ficticias.json`: la bitácora agrega `alcance: "COMPLETA"` y `alcance_metodo: "SITFA_TABLA_INFORME_INGRESO_V1"` (bitácora V2), y entonces `cargo test -p flow-captura --test lecturas` falla en `ventanas_dan_lo_mismo_que_python` y en `destino_de_bitacora_admite_consulta_pero_no_otro_origen`;
  - `generar_vinculos.py` reproduce su fixture sin cambios (9 listados);
  - los oráculos corren con el Python 3.12 del sistema en un venv con `descargador/requirements.txt`, `pandas<3` y `pytest`.

## E-Q1 · Lectores locales de contexto

1. **Oráculo vigente**: copiar `rust/oraculo/generar_lecturas.py` a `app/qa/generar_lecturas.py`, con la salida en `app/tests/datos/lecturas.json`. Generarlo con el Python vigente.
   - Sin datos reales: solo las ventanas ficticias de `tests/bitacora_fixtures.py` y las demás del oráculo.
   - Verificar que dos corridas producen el mismo archivo; si no, volverlo determinista.
   - Si el oráculo no cubre `leer_historico` o `leer_escritos_html`, agregar casos ficticios usando las funciones Python correspondientes (`descargador/historico_manual.py`, `src/nurus/escritos_html.py`). No inventar salidas.
2. **Módulo** `app/src/descargador/lecturas.rs` (biblioteca), portado desde `flow-captura::lecturas`, con sus dependencias mínimas.
   - Implementar lo que el Python vigente agregó (por ejemplo `alcance` y `alcance_metodo`) **siguiendo el código Python**, con la línea que lo respalda.
   - Reutilizar lo que `app/` ya tiene (`app/src/captura*.rs`, `descargador/sitfa*`) en vez de duplicarlo. Si algo choca, informarlo.
3. **Prueba** `lecturas_iguales_a_python`:
   - recorre todo el fixture y compara cada salida JSON completa, sin normalizar ni omitir campos;
   - incluye las pruebas negativas del crate histórico (origen ajeno, identidad distinta, ventana fuera de 1 a 4 meses);
   - imprime `LECTURAS: n/n, negativas m/m, fallos 0`.
4. **Mutación de control**: quitar `alcance` de la salida de bitácora → la prueba debe fallar. Pegar la salida y revertir.

## E-Q2 · Vínculos del listado

1. Copiar `rust/oraculo/generar_vinculos.py` a `app/qa/generar_vinculos.py`, con la salida en `app/tests/datos/vinculos.json`, y generarlo con el Python vigente.
2. Portar `flow-captura::vinculos` a `app/src/descargador/vinculos.rs`, salvo `abrir_bitacora`, `abrir_historia` y `abrir_calendario`, que usan el puente y son de E-Q3.
3. Prueba `vinculos_iguales_a_python`: imprime `VINCULOS: n/n, fallos 0` y cubre los 9 listados y los ambiguos descartados.
4. Mutación de control: hacer que `descartar_ambiguos` no descarte → la prueba falla. Revertir.

## Reglas comunes

- **Sin red y sin puente:** no habilitar `diary_open`, `history_open` ni `calendar_open`. El puente de `app/` debe seguir rechazándolos; la prueba de `descargador/puente.rs:440` no cambia.
- No tocar reglas, umbrales ni textos (`config.rs`, `textos_base.json`), ni el motor Python. No modificar `rust/crates/*`: es la fuente del port.
- `tools/inventario_escrituras.py` debe seguir en `PASA`. Si el port agrega escrituras a disco, clasificarlas ahí con su destino.
- Comandos en L (en W, omitir `--features eframe/x11` y declararlo):
  - `cargo test --locked --features eframe/x11 --lib`;
  - la suite oficial del binario con variables (ver «Auditoría · lote 1» del registro);
  - `cargo clippy --locked --features eframe/x11 --lib -- -D warnings`;
  - `python tools/check_real_ids.py app/src app/qa app/tests`, `python tools/check_contactos.py app/src app/qa` y `python tools/inventario_escrituras.py`.
- Un encargo, un commit de código y un commit de registro (`## Entrega · E-Q1` y `## Entrega · E-Q2`), publicados antes de pasar al siguiente. Estado → `ENTREGADO`.
- Si E-Q1 queda `BLOQUEADO`, E-Q2 se ejecuta igual: no dependen entre sí.
- Comprobar cada función o prueba citada con `grep -rn "fn <nombre>" app/` y cada commit con `git cat-file -t <hash>` después del push.
- Un criterio que no se cumple se informa como `BLOQUEADO` o `FALLA` con su motivo. No relajar comparadores ni editar salidas esperadas a mano.

## Respuesta final

- La salida inicial de `git log`.
- Por encargo:
  - commits;
  - líneas literales `LECTURAS` o `VINCULOS` y `test result`;
  - la mutación;
  - los campos nuevos de Python implementados, con su línea de respaldo;
  - el resultado por criterio (PASA, FALLA o BLOQUEADO).
- Los pendientes.
