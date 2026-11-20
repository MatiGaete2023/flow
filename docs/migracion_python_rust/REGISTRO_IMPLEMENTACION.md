# Registro de implementación Python → Rust

Rama `estudio/paridad-python-rust-20261119`. Referencia Python fijada en `93a36312adbd24e2dc96b02dfac45d947193b8b1`. Punto de partida de esta sesión: `740a64b905eca207e83e8bd27f76bd5513bfaaa9`. El [plan](PLAN.md) mantiene el orden y los criterios de aceptación.

## Estado al cierre de esta sesión

| Entrega | Estado |
|---|---|
| E0 · Línea base | Confirmadas la punta de la rama y las huellas de los fuentes editados antes de intervenir. Cargo no resolvió el árbol completo por falta de dependencias locales. La comparación completa de suites y delta exclusivo de `rust/producto` sigue pendiente. |
| E1 · F06 y lectura de paquetes Python | Implementado el lector de revisión V2, con regresiones contra V1 y legado verificadas en una compilación aislada. Falta la prueba con el exportador del Descargador desde un lote y la comprobación del ejecutable con los productos de exportación. |
| E2–E7 | Pendientes. |

## Cambios de E1

- `rust/crates/flow-trabajo/src/paquete.rs` conserva la lógica de V1 y añade lectura V2. V2 valida método, estado, intervalo ISO, coherencia de captura, tipos de los campos y advertencias. Usa la tabla normalizada de bitácoras para los eventos y la consulta para acreditar cobertura. La asociación exige coincidencia de consulta e identidad RUS/tribunal/causa/ingreso, más autoría acreditada para contar una observación del centro. Ausencia requiere tabla disponible, intervalo cubierto y evidencia coherente. Varias consultas que compiten por un mismo ingreso V2 se consideran contradictorias, porque el productor Python consolida una lectura por ingreso antes de exportar.
- `periodo_revision` reconoce el intervalo V2, que la pantalla permite ajustar. La ficha de evidencia muestra el rango y alcance informado para ese formato. Los eventos V2 quedan en el informe por separado y la exportación JSON existente limita esos eventos al período seleccionado.
- `rust/crates/flow-trabajo/tests/revision_bitacoras.rs` añade dos pruebas sobre paquetes con tablas y huellas. Recrean los cuatro casos de [CASOS_PRIMERA_ENTREGA.json](CASOS_PRIMERA_ENTREGA.json) y ejercitan cambio de período, serialización del informe, advertencias, intervalos, versión desconocida, autoría, ingreso ajeno, lectura vacía, contradicción y ausencia de tabla.

## Verificación ejecutada

1. Reproduje el defecto con el lector de la base y los dos tests nuevos. Ambos fallaron con `NoVerificable` donde la entrada acreditaba `ConObservacion`.
2. Después del cambio pasaron 21 pruebas en Windows con Rust 1.100.0. La compilación aislada usó el lector ZIP, el adaptador y el clasificador del repositorio, con la parte de informe que cambia período y se serializa. Se excluyó el test dependiente de aplicar el motor completo. Incluyó 13 regresiones existentes sobre paquetes, cinco del clasificador, dos V2 y un contraste con un ZIP escrito por el contrato Python.
3. El ZIP de contraste se construyó con `nurus.capture.write_capture` y valores V2 ficticios. El clasificador Python y el lector Rust aislado coincidieron en los cuatro resultados. El ensayo no recorrió el lector HTML ni el exportador integral; la publicación del temporal de prueba usó una copia local debido al bloqueo de enlaces duros del entorno Windows.
4. `cargo test --locked --offline -p flow-trabajo --test revision_bitacoras` desde el workspace se detuvo al resolver `chrono-tz`. La recuperación en línea se detuvo por DNS de `index.crates.io`. La UI y la producción de XLSX quedaron pendientes de verificar en el ejecutable integrado.

## Siguiente acción exacta

En un entorno con las dependencias del [lock](VERIFICACION.md) instaladas:

1. Desde la raíz del checkout, ejecutar `cargo test --manifest-path rust/Cargo.toml --locked -p flow-trabajo --test revision_bitacoras` y `cargo test --manifest-path rust/Cargo.toml --locked --workspace`. El workflow `rust.yml` de partida no se activa automáticamente con pushes a esta rama.
2. Generar el ZIP con el Descargador Python desde las funciones `diaries`, `_paginar` y `_trabajo` de `descargador/test_revision_bitacoras_p2.py`, usando el lote ficticio de dos ingresos. Ejecutar también esa prueba Python. Abrir ese ZIP en el ejecutable Rust desde **Revisión de bitácoras**, fijar los períodos de [CASOS_PRIMERA_ENTREGA.json](CASOS_PRIMERA_ENTREGA.json), comprobar los cuatro resultados y exportar Excel/JSON. Verificar conteos y verificar que reabrir el mismo ZIP reproduce el resultado.
3. Con el camino de importación comprobado, empezar E2 F17 comparando perfiles y consultas del Descargador integral y Lite con `flow-captura/src/sitfa`. Mantener el diálogo de avance en este registro, con el SHA probado y los resultados reales. La interoperabilidad de los ZIP **producidos por Rust** pertenece a E2.

Los comandos, fixtures y salidas de prueba usan solo datos ficticios. La aceptación en el puesto institucional queda a cargo del usuario.
