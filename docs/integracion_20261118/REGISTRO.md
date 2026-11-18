# Registro de ejecución — Plan de integración FLOW Rust/Python (18-nov-2026)

Documento vivo para que otra IA continúe. Orden: **estado → decisiones → pendientes → comandos**. Actualizar al cerrar cada paso.

## Fuentes del encargo
- `docs/integracion_20261118/01_Plan_integracion_FLOW_Rust_Python.md` y `02_Guia_de_consulta_RUS_SITFA.md` (copias del encargo).
- Rama de trabajo: `claude/wizardly-volta-vl2lqg`, creada desde `origin/feat/flow-estudio-d-20261117` @ `d2935dd8087b7467677e12d423fac512f1461190`.
- `main` (1 commit, solo README) NO representa el producto. No se fusiona nada a `main` ni se crean PR/releases.

## Paso 1 — Línea base (N1, medido 2026-11-18)
- Rust: `cargo test --workspace --locked` → 314 pruebas, 0 fallos.
- Python: `python -m pytest -q` → 55 fallos/errores **preexistentes**, todos por entorno: el contenedor no tiene `tkinter`
  (`ModuleNotFoundError: tkinter` / `TclError`). Lista exacta en `docs/integracion_20261118/base_python_fallos.txt`.
  Comparación posterior: el conjunto de fallos no debe crecer.
- `python tools/verificar_wp.py` → FALLA por «Restricciones absolutas» y «Ruff» (medición/línea base no disponible en el contenedor); preexistente.
- Dependencias instaladas aquí para correr: `docxcompose customtkinter icalendar` (pip).
