# Flow Rust · migración funcional

La aplicación Rust vigente de esta rama es [`app/`](app/), el proyecto de escritorio eframe/egui. Su entrada es `app/src/main.rs` y el ejecutable es `flow_rust.exe`. La referencia funcional es [`flow-python`](https://github.com/MatiGaete2023/flow/tree/flow-python).

| Documento | Contenido |
|---|---|
| [Estado](docs/migracion_python_rust/ESTADO.md) | Qué hay en `app/`, qué falta, siguiente paso y pendientes del usuario |
| [Plan](docs/migracion_python_rust/PLAN.md) | Encargos abiertos por tramo y decisiones del usuario |
| [Verificación](docs/migracion_python_rust/VERIFICACION.md) | Comandos de prueba por nivel |
| [Diseño](docs/migracion_python_rust/DISENO.md) | Pantallas y referencia visual |
| [Registro](docs/migracion_python_rust/REGISTRO_IMPLEMENTACION.md) | Entradas cortas por encargo desde el 24-nov-2026 |
| [Revisión del 24-nov](docs/migracion_python_rust/REVISION_20261124.md) | Fundamento de la consolidación y propuestas pendientes |
| [Histórico](docs/migracion_python_rust/historico/) | Registro, encargos, matriz e informes anteriores |

El workspace `rust/` es la implementación anterior. Aporta crates y oráculos reutilizables, y los datos de prueba `rust/casos`.
