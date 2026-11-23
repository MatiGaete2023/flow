# Prompt · Encargo E-B (comparador estricto de la redacción mensual)

Copiar desde la línea siguiente y entregar tal cual a la IA ejecutora.

---

Eres una IA ejecutora del proyecto Flow Rust. Tomas solo el encargo **E-B**. No planificas, no redefines criterios y no tocas otros encargos.

## Contexto mínimo

- Repositorio `MatiGaete2023/flow`, rama `estudio/paridad-python-rust-20261119`, desde el commit `4fe07da` o uno posterior. No crear ramas, no usar `--force`, no fusionar a `main` y no crear releases.
- Leer antes: `AGENTS.md`, `docs/migracion_python_rust/ENCARGOS_IA.md` (reglas de entrega y encargo E-B) y la sección «Pruebas de la biblioteca en Linux» y siguientes de `docs/migracion_python_rust/VERIFICACION.md`.
- Las reglas de redacción están en `docs/migracion_python_rust/INFORME_FALLAS_20261120.md`: primero la tabla de diferencias R1–R6 y después la tabla de decisiones.
- R6 aprobada, ampliada por la decisión D-1 del 23 de noviembre: nombre del programa en tipo título, conservando el guion del nombre de origen («PRM CIUDAD DEL NIÑO - MULCHÉN» → «Ciudad del Niño - Mulchén»).

## Problema que corriges

La prueba `motor::historia::tests::paquete_b_redaccion_mensual` (`app/src/motor/historia.rs`) tiene cuatro defectos:

1. Clasifica como R6 cualquier diferencia de mayúsculas, tildes o guiones **en toda la observación**, porque normaliza el texto completo. Así ocultaría errores fuera del nombre del programa.
2. Solo afirma la ausencia de `R3_MULTI`. No verifica R2, R3 ni R4.
3. No verifica R5.
4. Sin `FLOW_CASOS` termina con `return` y aparece como aprobada.

Valores de referencia en `f6a8f28` (N1, Linux):

- salida actual: `PAQUETE_B: Evaluados: 2582, Idénticos: 1997, Solo R1: 7, Solo R6: 577, R1+R6: 1, Fallos inesperados: 0`;
- las 585 diferencias estaban dentro del nombre del programa;
- 51 de ellas son el guion conservado.

## Qué hacer

Solo cambias código de prueba en `app/src/motor/historia.rs`. Las funciones de producción, `config.rs` y `textos_base.json` no se tocan.

1. **R6 exacta.** Por caso:
   - tomar el nombre de origen de `c["campos"]["programa"]["v"]`, el campo que usa `motor.rs` en las llamadas a `titulo_programa` y `titulo_programa_registro`;
   - calcular `py = util::titulo_programa(nombre)` y `rs = util::titulo_programa_registro(nombre)`;
   - en la observación esperada, ya sin la oración C10 si corresponde R1, reemplazar `py` por `rs`;
   - el resultado debe ser **idéntico byte a byte** a la observación obtenida. Si no, el caso es FALLA;
   - eliminar toda normalización global: minúsculas, tildes, guiones y espacios.

   Si el caso tiene un campo de programa del cruce o de otra hoja que entre a la observación, tratarlo igual y documentarlo.
2. **R6_GUION.** Dentro de R6, contar aparte los casos en que `rs` contiene « - » y `py` no. Es una variante aceptada (D-1), no FALLA.
3. **R1 exacta.** Solo con los eventos `CUMPLIMIENTO.C05_POR_VENCER` y `CUMPLIMIENTO.C10_HOJA2`, quitar de la esperada **una** oración «Próximo informe de avance de … vence el ….». La oración debe existir en la esperada y no debe existir en la obtenida.
4. **R2, R3 y R4 en la observación obtenida**, con `redaccion_registro: true`, en los 2.582 casos:
   - R2: no contiene «La historia es de la causa y no permite atribuirlo a un ingreso determinado.» ni el texto `R3_MULTI`;
   - R3: con `HISTORIA.E04_CON_FOLIO` y `ESPERA.E05_SOLO_CORREO`, «Medida revisada, a la espera de ingreso efectivo.» aparece una sola vez y no aparece «RUS registra … días de espera»;
   - R4: si contiene «Tribunal dispone ingreso a», no contiene «RUS registra … días de espera».

   Contar los casos en que aplica cada comprobación.
5. **R5.** Buscar en el corpus (`rust/casos/*.json`) los casos con `COMUN.REITERA_OBSERVACION` en las reglas esperadas u obtenidas.
   - Si existen, verificar la regla parcial de R5: no reiterar cuando el texto vigente es «Medida revisada, sin observaciones.» o «La derivación … no se encuentra sujeta a seguimiento por este Centro.», y reiterar en los demás.
   - Si no existen, informar `R5: BLOQUEADO — 0 casos en el corpus`. Citar la prueba existente que cubre la reiteración (`grep -rn REITERA_OBSERVACION app/src`). No inventar casos.
6. **Sin salto silencioso.** Sin `FLOW_CASOS`, la prueba falla con `panic!("FLOW_CASOS es obligatorio para paquete_b_redaccion_mensual")`. Comprobar que la CI la define (`.github/workflows/flow-rust-local.yml`, variable `FLOW_CASOS`). Si no la define, informarlo sin tocar el workflow.
7. **Salida única** de la prueba:
   `PAQUETE_B: evaluados N, identicos N, R1 N, R6 N (R6_GUION N), R1+R6 N, R2 ok/N, R3 ok/N, R4 ok/N, R5 <ok/N | BLOQUEADO>, fallos 0`.
   Si hay fallos, imprimir hasta 5 con id de caso y la primera palabra distinta. La prueba falla si `fallos > 0`.

## Cómo probar (entorno Linux)

1. Preparar `.aislado/` según `VERIFICACION.md`:
   - biblioteca: `[lib] path = "../app/src/lib.rs"`;
   - binario `motor_aislado`, con enlaces en `src/` a `anon`, `captura_trabajo`, `config`, `copia_excel`, `demo`, `lectura`, `motor`, `productos`, `trabajo` y `util` (`.rs` y carpeta);
   - `src/main.rs` declara esos módulos y un `main` vacío;
   - `.aislado/` queda excluido en `.git/info/exclude`.
2. Ejecutar:
   - `FLOW_CASOS=$PWD/../rust/casos cargo test --bin motor_aislado -- --nocapture paquete_b oraculo`
   - `cargo test --bin motor_aislado` (suite completa del binario)
   - `cargo clippy --bin motor_aislado --tests -- -D warnings`, que no debe agregar advertencias en `historia.rs`
3. Criterios de aceptación:
   - `oraculo_python` 174/174 y `oraculo_historia` 2582/2582 sin cambios;
   - en la línea `PAQUETE_B`: `fallos 0` y R1 + R6 + R1+R6 = 585 (R6_GUION debería ser 51; si difiere, explicar por qué);
   - R2, R3 y R4 en `ok` con su conteo;
   - R5 en `ok` o `BLOQUEADO` con su fundamento;
   - sin `FLOW_CASOS`, la prueba falla.

## Entrega

- Antes de publicar: `python tools/check_real_ids.py app/src` y `python tools/check_contactos.py app/src`.
- Agregar a `docs/migracion_python_rust/REGISTRO_IMPLEMENTACION.md`, antes de «Siguiente paso exacto», la sección `## Entrega · E-B` con la plantilla de `ENCARGOS_IA.md`. Incluir las **líneas literales** de salida (`PAQUETE_B: …`, `ORACULO: …`, `oraculo_historia: …`, `test result: …`).
- En `ENCARGOS_IA.md`, cambiar solo el estado de E-B de `ABIERTO` a `ENTREGADO`. Nunca a `APROBADO`.
- Commit en español, con el formato de los anteriores, y publicar en la misma rama.
- Si algo no se puede cumplir, informarlo como `BLOQUEADO` con el motivo. No relajar el comparador para que pase.
- No citar funciones ni pruebas que no estén en tu commit.

Tu respuesta final: commit publicado, la línea `PAQUETE_B` literal, el resultado por criterio (PASA, FALLA o BLOQUEADO) y los pendientes.
