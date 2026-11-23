# Prompt · Encargo E-C (paquete C: integración con ZIP sintéticos de Flow Python)

Copiar desde la línea siguiente y entregar tal cual a la IA ejecutora.

---

Antes de cualquier otra cosa, ejecuta y pega la salida literal de `git log --oneline -1 && git status -sb`. Si no puedes ejecutar comandos en este repositorio, responde solo «SIN ACCESO» y detente.

Eres una IA ejecutora del proyecto Flow Rust. Tomas solo el encargo **E-C**. No planificas, no redefines criterios y no tocas otros encargos; en particular, no corrijas E-L.

## Contexto mínimo

- Repositorio `MatiGaete2023/flow`, rama `estudio/paridad-python-rust-20261119`, desde el commit de la auditoría de E-K o uno posterior. No crear ramas, no usar `--force`, no fusionar a `main` y no crear releases.
- Leer antes:
  - `AGENTS.md`;
  - `docs/migracion_python_rust/ENCARGOS_IA.md`: reglas de entrega, E-C, E-L y la plantilla;
  - `docs/migracion_python_rust/PLAN_PRUEBAS.md`, «Paquete C»;
  - la sección «Auditoría · E-K» de `docs/migracion_python_rust/REGISTRO_IMPLEMENTACION.md`, que trae la línea base de E-C.
- Hechos auditados (N1, entorno L):
  - `app/qa/generar_contexto_e3.py . <carpeta>` genera seis ZIP: `contexto-e3.zip`, `-espera`, `-informes`, `-reiteracion`, `-discordante` y `base.zip`. Para correrlo hace falta:
    - Python 3.12 del sistema, por `tkinter`, que no viene en `/usr/local/bin/python3`;
    - un venv con `descargador/requirements.txt`, `pandas<3` y `pytest`.
  - Con `FLOW_REFERENCIA=..` y `FLOW_E3_PAQUETE=<carpeta>/contexto-e3.zip`, la suite del binario da `47 passed; 0 failed; 1 ignored`.
  - Las pruebas E3 actuales (`trabajo.rs`, `captura_trabajo.rs`) comparan contra **literales escritos a mano**, no contra una salida de Python generada en la misma corrida. Además, sin variable terminan con `return` (hallazgo E-L).
  - En Python, el ZIP se abre con `nurus.personal.capture_input.import_capture(path, defaults(), as_of=…)`. El generador ya lo usa en las líneas 130, 151 y 169.

## Qué hacer

1. **Paquete nuevo.** En `app/qa/generar_contexto_e3.py`, agregar `contexto-e3-historia.zip` sin alterar los ZIP existentes. Debe traer los seis elementos de PLAN_PRUEBAS C.1:
   - orden de búsqueda de contexto con su término;
   - continuidad de atención;
   - prórroga reciente con la medida por vencer;
   - designación de curador con RUS sin curador;
   - causa P con sentencia;
   - bitácora con proyección de ingreso para un mes futuro.

   Datos solo ficticios (RIT año 2099 o `X-…-2026` como el resto, RUT sintético, nombres de fantasía). Reutilizar los fixtures de `descargador/test_*.py`, como hace el generador. Si un elemento no se puede producir con esos fixtures sin datos reales, informar `BLOQUEADO` para ese elemento.

   Comprobar que los ZIP previos no cambian: comparar el `sha256` de cada uno antes y después. Si el generador no es determinista, mostrar dos corridas sin tu cambio y comparar por contenido de tablas.
2. **Salida de referencia de Python.** Nuevo script `app/qa/esperado_contexto_e3.py <zip> <salida.json>`. Abre el ZIP con `import_capture` en las tres etapas (Espera, Cumplimiento, Informes) que admita el paquete, con `as_of` fijo y con un corte movido antes del inicio de la ventana. Escribe por fila: `record_id`, etapa, corte, `observacion`, reglas/eventos y marcas.
   - Usar las funciones públicas de `nurus.personal`.
   - Si una etapa no aplica al paquete, escribirla como `NO_APLICA` con el motivo que da Python.
3. **Prueba Rust** en `app/src/trabajo.rs` o `app/src/captura_trabajo.rs`, solo en `#[cfg(test)]`. Nombre: `paquete_c_contexto_e3_historia_igual_a_python`.
   - Lee `FLOW_E3_HISTORIA` (ZIP) y `FLOW_E3_ESPERADO` (JSON del paso 2).
   - Si falta alguna variable, **falla** con `panic!("FLOW_E3_HISTORIA y FLOW_E3_ESPERADO son obligatorias para el paquete C")`. Nada de `return`.
   - Abre el ZIP en Trabajo en cada etapa y corte del JSON y compara fila a fila observación, reglas y marcas.
   - Diferencias admitidas: solo R1 y R6, clasificadas con las mismas funciones del paquete B (`motor::historia::tests`). Si no son reutilizables desde ese módulo, informarlo; no copiar una versión relajada.
   - Imprimir `PAQUETE_C: etapas E, filas N, identicas I, R1 a, R6 b, fallos F` y, por cada corte movido, `PAQUETE_C_CORTE: filas N, constancias_historia 0`.
4. **Mutación de control**: cambiar un carácter de la observación esperada en una copia del JSON. La prueba debe fallar. Pegar esa salida y no publicar la copia.
5. No tocar:
   - funciones de producción;
   - `config.rs`;
   - `textos_base.json`;
   - el motor Python;
   - las pruebas E3 existentes.

   Si Python y Rust difieren fuera de R1/R6, registrarlo como hallazgo con la fila, el texto esperado y el obtenido, sin ajustar el comparador.

## Cómo probar (entorno L)

Desde la raíz:

```
/usr/bin/python3.12 -m venv <venv> && <venv>/bin/pip install -r descargador/requirements.txt "pandas<3" pytest
<venv>/bin/python app/qa/generar_contexto_e3.py . <carpeta>
<venv>/bin/python app/qa/esperado_contexto_e3.py <carpeta>/contexto-e3-historia.zip <carpeta>/esperado-historia.json
```

Desde `app/`:

```
FLOW_E3_HISTORIA=<carpeta>/contexto-e3-historia.zip FLOW_E3_ESPERADO=<carpeta>/esperado-historia.json cargo test --locked --features eframe/x11 --bin flow_rust paquete_c -- --nocapture
cargo test --locked --features eframe/x11 --bin flow_rust paquete_c
FLOW_REFERENCIA=.. FLOW_E3_PAQUETE=<carpeta>/contexto-e3.zip FLOW_CASOS=../rust/casos FLOW_E3_HISTORIA=… FLOW_E3_ESPERADO=… cargo test --locked --features eframe/x11 --bin flow_rust
cargo clippy --locked --features eframe/x11 --bin flow_rust --tests
```

- El segundo comando corre sin variables y debe fallar.
- El tercero es la suite completa; hoy da 47 passed y 1 ignored.

Además: `python tools/check_real_ids.py app/src app/qa` y `python tools/check_contactos.py app/src app/qa`.

En Windows (W) omitir `--features eframe/x11` e informarlo.

## Criterios de aceptación

- `contexto-e3-historia.zip` trae los seis elementos. Por cada uno, una línea de evidencia: tabla y fila del ZIP. Los ZIP previos no cambian.
- `PAQUETE_C: … fallos 0`, con Espera, Cumplimiento e Informes. Las etapas que no apliquen van como `NO_APLICA` con el motivo de Python.
- `PAQUETE_C_CORTE: … constancias_historia 0` por cada corte movido.
- La mutación de control hace fallar la prueba.
- Sin variables, la prueba falla.
- La suite completa pasa y clippy no muestra advertencias nuevas en los archivos tocados.
- Sin cambios de lógica de producción.

## Entrega

- En `docs/migracion_python_rust/REGISTRO_IMPLEMENTACION.md`, agregar antes de «Siguiente paso exacto» la sección `## Entrega · E-C`, con la plantilla de `ENCARGOS_IA.md`. Incluir:
  - las líneas literales `PAQUETE_C`, `PAQUETE_C_CORTE` y `test result`;
  - la salida de la mutación;
  - los `sha256` antes y después.
- En `ENCARGOS_IA.md`, cambiar solo el estado de E-C de `ABIERTO` a `ENTREGADO`. Nunca a `APROBADO` ni `CERRADO`.
- Commit en español, con el formato de los anteriores, y publicar en la misma rama. No publicar los ZIP ni el JSON generados: viven en una carpeta temporal.
- Un criterio que no se cumple se informa como `BLOQUEADO` o `FALLA` con su motivo. No relajar el comparador ni inventar valores esperados.
- No citar commits, funciones ni pruebas que no estén en tu commit:
  - comprobar cada función o prueba con `grep -rn "fn <nombre>" app/`;
  - comprobar cada commit con `git cat-file -t <hash>` después del push.

Tu respuesta final debe traer:
- la salida inicial de `git log`;
- el commit publicado;
- las líneas `PAQUETE_C`, `PAQUETE_C_CORTE` y `test result` literales;
- la evidencia de los seis elementos;
- la mutación;
- el resultado por criterio (PASA, FALLA o BLOQUEADO);
- los pendientes.
