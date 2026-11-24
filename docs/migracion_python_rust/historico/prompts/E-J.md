# Prompt · Encargo E-J (casos de R3 en el oráculo)

Copiar desde la línea siguiente y entregar tal cual a la IA ejecutora.

---

Eres una IA ejecutora del proyecto Flow Rust. Tomas solo el encargo **E-J**. No planificas, no redefines criterios y no tocas otros encargos.

## Contexto mínimo

- Repositorio `MatiGaete2023/flow`, rama `estudio/paridad-python-rust-20261119`, desde el commit de la auditoría de E-B o uno posterior. No crear ramas, no usar `--force`, no fusionar a `main` y no crear releases.
- Leer antes:
  - `AGENTS.md`;
  - `docs/migracion_python_rust/ENCARGOS_IA.md`: reglas de entrega, E-J y la plantilla;
  - la sección «Auditoría · E-B» de `docs/migracion_python_rust/REGISTRO_IMPLEMENTACION.md`;
  - las tablas de diferencias y de decisiones de `docs/migracion_python_rust/INFORME_FALLAS_20261120.md`, fila R3.
- R3 aprobada: con `HISTORIA.E04_CON_FOLIO` y `ESPERA.E05_SOLO_CORREO`, la observación lleva un solo «Medida revisada, a la espera de ingreso efectivo.» y no lleva «Al <FECHA>, RUS registra <N> días de espera.».
- Hechos auditados (N1):
  - el corpus `rust/casos/espera.json` no tiene ningún caso con esas dos reglas juntas, por eso `paquete_b_cobertura_r3` (`app/src/motor/historia.rs`) está `#[ignore]` y falla al ejecutarla;
  - en Python (`src/nurus/personal/motor/reglas_espera.py`), `E04_CON_FOLIO` exige una resolución reciente (`0 <= días < parametro('resolucion_reciente')`) cuyo folio entregue `H.folio_resolucion`. `E05_SOLO_CORREO` exige un programa DCE (`es_dce`) con espera ≥ `parametro('espera_dce')` y sin proyección (`E07_PROYECCION`);
  - el Python vigente ya no escribe los días de espera (comentario del 11-nov-2026 en `reglas_espera.py`).

## Qué hacer

1. **Casos dirigidos.** En `rust/oraculo/generar_casos.py::dirigidos`, rama `etapa == 'ESPERA'`, agregar al menos 3 casos sintéticos al final de la lista, para no renumerar los existentes:
   - programa DCE;
   - columna de resolución con fecha reciente y un trámite de historia con folio para esa fecha;
   - espera ≥ umbral DCE;
   - una variante con dos fragmentos que repitan «Medida revisada, a la espera de ingreso efectivo.», si Python lo permite.

   Datos solo ficticios: RIT con año 2099, RUT con dígito verificador inválido y nombres de fantasía, como el resto de `dirigidos`.
2. **Regenerar** con la semilla y la cantidad por defecto: `python3 rust/oraculo/generar_casos.py`, con `pandas<3` (el entorno puede traer pandas 3: usar un venv). Comprobar con `git diff --stat rust/casos` y con un script que compare por `id` que:
   - los casos previos no cambian;
   - solo se agregan los nuevos.

   Si cambia un caso previo, detenerse e informarlo como `BLOQUEADO`.
3. **Comprobar en Python** que cada caso nuevo trae `HISTORIA.E04_CON_FOLIO` y `ESPERA.E05_SOLO_CORREO` en `esperado.reglas`. Si Python no puede producir ambas reglas juntas, informar `BLOQUEADO` con la línea de `reglas_espera.py` que lo impide. No forzar el caso ni editar `esperado` a mano.
4. **Prueba Rust** en `app/src/motor/historia.rs`, solo en código de prueba:
   - actualizar `CASOS_PAQUETE_B` al nuevo total de evaluados;
   - quitar el `#[ignore]` de `paquete_b_cobertura_r3`;
   - si Python y Rust difieren en un caso nuevo, la única diferencia admitida es R3. Quitar de la esperada exactamente la segunda aparición de «Medida revisada, a la espera de ingreso efectivo.» y, si existe, la oración «Al … RUS registra … días de espera.». Luego, igualdad byte a byte. Contar el caso como `R3` en la línea `PAQUETE_B`. Cualquier otra diferencia es FALLA;
   - ajustar a los nuevos totales los pisos de `oraculo_python` y `oraculo_historia` solo si fallan por la cantidad, no por el contenido.
5. No tocar funciones de producción, `config.rs`, `textos_base.json` ni el motor Python. Si una prueba muestra una diferencia no aprobada, registrarla como decisión pendiente del usuario.

## Cómo probar (entorno L)

Desde `app/`, con winit en Linux:

- `FLOW_CASOS=../rust/casos cargo test --locked --features eframe/x11 --bin flow_rust paquete_b -- --nocapture`
- `FLOW_CASOS=../rust/casos cargo test --locked --features eframe/x11 --bin flow_rust oraculo -- --nocapture`
- `FLOW_CASOS=../rust/casos cargo test --locked --features eframe/x11 --bin flow_rust` (suite completa; hoy: 45 passed, 2 ignored)
- `cargo clippy --locked --features eframe/x11 --bin flow_rust --tests` (sin advertencias nuevas en `historia.rs`)

Desde la raíz:

- `python tools/check_real_ids.py app/src` y `python tools/check_contactos.py app/src`

## Criterios de aceptación

- `PAQUETE_B_COBERTURA: R3 n/n, R4 …, fallos 0`, con n ≥ 3, y la prueba pasa sin `#[ignore]`.
- `PAQUETE_B: … R3 n/n …, fallos 0`. R1 7, R6 526, R6_GUION 51 y R1+R6 1 no cambian.
- Los casos previos de `rust/casos/*.json` quedan byte a byte iguales; solo se agregan casos.
- `oraculo_python` y `oraculo_historia` pasan con sus nuevos totales literales.
- Sin `FLOW_CASOS`, las pruebas del paquete B fallan.

## Entrega

- Agregar a `docs/migracion_python_rust/REGISTRO_IMPLEMENTACION.md`, antes de «Siguiente paso exacto», la sección `## Entrega · E-J` con la plantilla de `ENCARGOS_IA.md`. Incluir las líneas literales `PAQUETE_B: …`, `PAQUETE_B_COBERTURA: …`, `oraculo_…` y `test result: …`, más el resultado de la comparación por `id`.
- En `ENCARGOS_IA.md`, cambiar solo el estado de E-J de `ABIERTO` a `ENTREGADO`. Nunca a `APROBADO` ni `CERRADO`.
- Commit en español, con el formato de los anteriores, y publicar en la misma rama.
- Un criterio que no se cumple se informa como `BLOQUEADO` con su motivo. No relajar el comparador ni inventar valores esperados.
- No citar funciones ni pruebas que no estén en tu commit; comprobar cada una con `grep -rn "fn <nombre>" app/`.

Tu respuesta final: commit publicado, las líneas `PAQUETE_B` y `PAQUETE_B_COBERTURA` literales, el resultado por criterio (PASA, FALLA o BLOQUEADO) y los pendientes.
