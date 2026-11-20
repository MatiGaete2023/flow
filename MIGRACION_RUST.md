# Flow Rust · migración funcional

La aplicación Rust vigente de esta rama es [`app/`](app/), el proyecto de escritorio eframe/egui de la carpeta local Flow Rust. Su entrada es `app/src/main.rs` y el ejecutable es `flow_rust.exe`. La referencia funcional es [`flow-python`](https://github.com/MatiGaete2023/flow/tree/flow-python).

| Documento | Contenido |
|---|---|
| [Plan vigente](docs/migracion_python_rust/PLAN.md) | Orden de implementación y contratos que se deben conservar |
| [Diseño vigente](docs/migracion_python_rust/DISENO.md) | Pantallas, componentes visuales y comprobación del ejecutable correcto |
| [Matriz de paridad](docs/migracion_python_rust/MATRIZ.md) | Las 28 capacidades y su situación en `app/` |
| [Registro y continuidad](docs/migracion_python_rust/REGISTRO_IMPLEMENTACION.md) | Evidencia transferible y siguiente cambio |
| [Verificación](docs/migracion_python_rust/VERIFICACION.md) | Pruebas y comandos reproducibles |

El [cierre del estudio anterior](https://github.com/MatiGaete2023/flow/tree/ed6a0e43f8efa0856e0294af92ac8e0317c69df7) conserva las pruebas y los cambios de `rust/crates/`. Se pueden trasladar comportamientos y casos sintéticos que coincidan con Python. Sus resultados y su artefacto visual corresponden a esa implementación.
