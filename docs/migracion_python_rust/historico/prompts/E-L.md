# Prompt · Encargo E-L (pruebas condicionadas que pasan sin ejecutarse)

Copiar desde la línea siguiente y entregar tal cual a la IA ejecutora.

---

Antes de cualquier otra cosa, ejecuta y pega la salida literal de `git log --oneline -1 && git status -sb`. Si no puedes ejecutar comandos en este repositorio, responde solo «SIN ACCESO» y detente.

Eres una IA ejecutora del proyecto Flow Rust. Tomas solo el encargo **E-L**. No planificas, no redefines criterios y no tocas otros encargos.

## Contexto mínimo

- Repositorio `MatiGaete2023/flow`, rama `estudio/paridad-python-rust-20261119`, desde `163f310` o uno posterior. No crear ramas, no usar `--force`, no fusionar a `main` y no crear releases.
- Leer antes:
  - `AGENTS.md`;
  - `docs/migracion_python_rust/ENCARGOS_IA.md`: reglas de entrega (regla 4: sin saltos silenciosos), E-L y la plantilla;
  - `.github/workflows/flow-rust-local.yml`.
- Hecho auditado (N1): estas pruebas de `app/src` terminan con `return` cuando falta su variable de entorno, y `cargo test` las informa como `ok` sin haber probado nada:

| Archivo | Prueba | Variable |
|---|---|---|
| `motor.rs:884` | `oraculo_python` | `FLOW_CASOS` |
| `motor/historia.rs:1077` | `oraculo_historia` | `FLOW_CASOS` |
| `captura.rs:438` | `contrasta_el_contrato_y_rechaza_identidad_cambiada_con_huella_correcta` | `FLOW_REFERENCIA` |
| `captura_trabajo.rs:962` | `trabajo_abre_contexto_e3_del_exportador_python` | `FLOW_E3_PAQUETE` |
| `trabajo.rs:531` | `cumplimiento_usa_contexto_e3_del_exportador_python_para_el_proximo_informe` | `FLOW_E3_PAQUETE` |
| `trabajo.rs:556` | `a2_consignacion_p1_y_sesion` | `FLOW_E3_PAQUETE` |
| `trabajo.rs:637` | `informes_atribuyen_la_historia_y_los_escritos_segun_flow_python` | `FLOW_REFERENCIA` |
| `trabajo.rs:670` | `espera_coteja_folio_y_correo_del_contexto_e3_del_exportador_python` | `FLOW_E3_PAQUETE` |
| `trabajo.rs:690` | `calendario_91_conserva_precedencia_y_aviso_al_abrir_trabajo` | `FLOW_REFERENCIA` |
| `trabajo.rs:708` | `informes_coteja_contexto_e3_del_exportador_python_con_una_sola_fecha` | `FLOW_E3_PAQUETE` |
| `trabajo.rs:724` | `trabajo_reitera_contexto_e3_del_exportador_python_solo_para_el_ingreso` | `FLOW_E3_PAQUETE` |
| `trabajo.rs:821` | `formatos_sitfa` | `FLOW_FIXTURES` |
| `revision_ui.rs:373` | `exporta_mismo_lote_de_python` | `FLOW_P2_PAQUETE` |
| `revision_ui.rs:394` | `exporta_resultados_y_preserva_el_destino_existente` | `FLOW_REFERENCIA` |
| `ui.rs:2241` | `trabajo_abre_paquete_cambia_etapa_y_conserva_el_activo_ante_error` | `FLOW_REFERENCIA` |

  `ui.rs:301` (modo captura de pantalla `trabajo-contexto`) no es una prueba y ya usa `expect`: queda fuera del encargo.

  Confirma la lista con `grep -rnE 'std::env::var(_os)?\("FLOW_' app/src`. Si aparece otra prueba con el mismo patrón, inclúyela; si una ya no existe, indícalo.
- En la CI (`flow-rust-local.yml`, Windows):
  - `FLOW_CASOS`, `FLOW_FIXTURES` y `FLOW_REFERENCIA` están definidas a nivel del job;
  - `FLOW_E3_PAQUETE` se define recién en el paso «Abrir en Trabajo…», y ahí solo corren las pruebas cuyo nombre contiene `contexto_e3_del_exportador_python`. En consecuencia, `a2_consignacion_p1_y_sesion` nunca corre con su paquete;
  - `FLOW_P2_PAQUETE` no se define en ningún paso.

## Qué hacer

1. **Helper de prueba**, en un único módulo `#[cfg(test)]` (por ejemplo, `app/src/prueba_entorno.rs`, declarado con `#[cfg(test)] mod prueba_entorno;` en `main.rs`): `fn requerida(nombre: &str) -> std::ffi::OsString`. Si la variable falta, hace `panic!("{nombre} es obligatoria para esta prueba (ver docs/migracion_python_rust/REGISTRO_IMPLEMENTACION.md)")`.
2. **En cada prueba de la tabla**:
   - reemplazar `let Some(x) = var_os(...) else { return }` por `let x = requerida("FLOW_…")`;
   - agregar `#[ignore = "requiere FLOW_…"]`.

   El resultado: sin variables, la suite las informa como `ignored`. Si se piden con `--include-ignored` y falta la variable, fallan. No cambiar ninguna aserción.
3. **Paquete B**: `paquete_b_redaccion_mensual` y `paquete_b_cobertura_r3` ya hacen `panic` sin `FLOW_CASOS`. Marcarlas también con `#[ignore = "requiere FLOW_CASOS"]` para que la suite sin variables quede verde y honesta.
4. **CI** (`.github/workflows/flow-rust-local.yml`):
   - el paso «Probar la aplicación desde app» se mantiene como está (corre lo no ignorado);
   - agregar después del paso que genera el contexto E3 un paso que corra `cargo test --manifest-path app/Cargo.toml --locked --bin flow_rust -- --include-ignored --skip exporta_mismo_lote_de_python --skip velocidad_20000`. En él deben estar definidas `FLOW_CASOS`, `FLOW_FIXTURES`, `FLOW_REFERENCIA` y `FLOW_E3_PAQUETE`;
   - `exporta_mismo_lote_de_python` se salta porque la CI no tiene `FLOW_P2_PAQUETE`. Dejarlo anotado en el registro como pendiente; no inventar un paquete.
   - No tocar los pasos de build ni de artefactos.
5. No tocar funciones de producción, aserciones, `config.rs` ni `textos_base.json`.

## Cómo probar (entorno L; en W, omitir `--features eframe/x11` e informarlo)

Prerrequisito para `FLOW_E3_PAQUETE`, desde la raíz:

```
/usr/bin/python3.12 -m venv <venv> && <venv>/bin/pip install -r descargador/requirements.txt "pandas<3" pytest
<venv>/bin/python app/qa/generar_contexto_e3.py . <carpeta>
```

Desde `app/`:

```
cargo test --locked --features eframe/x11 --bin flow_rust
FLOW_CASOS=../rust/casos FLOW_FIXTURES=../rust/crates/flow-tabular/tests/datos FLOW_REFERENCIA=.. FLOW_E3_PAQUETE=<carpeta>/contexto-e3.zip cargo test --locked --features eframe/x11 --bin flow_rust -- --include-ignored --skip exporta_mismo_lote_de_python --skip velocidad_20000
cargo test --locked --features eframe/x11 --bin flow_rust oraculo_python -- --include-ignored
cargo test --locked --features eframe/x11 --lib
cargo clippy --locked --features eframe/x11 --bin flow_rust --tests
```

1. El primero va sin variables: debe pasar y listar las pruebas de la tabla como `ignored`.
2. El segundo lleva todas las variables: deben pasar todas, incluidas las ignoradas.
3. El tercero va sin `FLOW_CASOS`: debe fallar con el mensaje del helper.
4. El cuarto corre la biblioteca, para confirmar que no cambió.
5. Clippy: sin advertencias nuevas en los archivos tocados.

Desde la raíz: `python tools/check_real_ids.py app/src` y `python tools/check_contactos.py app/src`.

## Criterios de aceptación

- `grep -rnE 'var(_os)?\("FLOW_[A-Z0-9_]+"\)[^;]*else *\{ *return' app/src` sin resultados. Pegar la salida vacía y el código de salida.
- Sin variables: `test result: ok. X passed; 0 failed; Y ignored`, con Y = 1 (`velocidad_20000`) + las pruebas de la tabla + las 2 del paquete B. Pegar la lista de las ignoradas (`cargo test … -- --list --ignored`, o equivalente).
- Con variables y `--include-ignored --skip exporta_mismo_lote_de_python --skip velocidad_20000`: `passed` = X + Y − 2, `0 failed; 0 ignored; … 2 filtered out`: las dos omitidas son `velocidad_20000` (lenta) y `exporta_mismo_lote_de_python` (sin paquete P2). Si la cuenta difiere, explicarla.
- Sin `FLOW_CASOS` y con `--include-ignored`, `oraculo_python` falla con el mensaje del helper.
- La CI tiene el paso nuevo y no cambia nada más. Si se puede ejecutar (push a la rama), pegar el enlace y el resultado del run. Si no, indicar `NO VERIFICADO EN CI`.
- Sin cambios de lógica de producción: `git diff` de `app/src` solo en bloques `#[cfg(test)]` y en la declaración del módulo de prueba.

## Entrega

- En `docs/migracion_python_rust/REGISTRO_IMPLEMENTACION.md`, agregar antes de «Siguiente paso exacto» la sección `## Entrega · E-L`, con la plantilla de `ENCARGOS_IA.md`. Incluir:
  - las líneas literales `test result` de cada corrida;
  - la lista de ignoradas;
  - el comando oficial de la suite completa con variables, para que las próximas entregas lo usen.
- En `ENCARGOS_IA.md`, cambiar solo el estado de E-L de `ABIERTO` a `ENTREGADO`.
- Commit en español, con el formato de los anteriores, y publicar en la misma rama.
- Un criterio que no se cumple se informa como `BLOQUEADO` o `FALLA` con su motivo.
- Comprobar cada prueba citada con `grep -rn "fn <nombre>" app/` y cada commit con `git cat-file -t <hash>` después del push.

Tu respuesta final debe traer:
- la salida inicial de `git log`;
- el commit publicado;
- las líneas `test result` literales de las tres corridas;
- la lista de ignoradas;
- el resultado por criterio (PASA, FALLA o BLOQUEADO);
- los pendientes.
