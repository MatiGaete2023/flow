# Flow · migración de Python a Rust

**Continúa sobre el workspace `rust/` que ya está en esta rama.** Python es la referencia de conducta. El prototipo de escritorio de la carpeta local queda disponible en [prototipos/flow-egui](prototipos/flow-egui) para recuperar su diseño y reproducir sus pruebas. El primer cambio de producto recomendado es hacer que el lector Rust entienda la evidencia de bitácora que hoy produce el Descargador Python.

| Para qué | Archivo |
|---|---|
| Entender las tres implementaciones, sus límites y las decisiones de arquitectura | [Informe técnico](docs/migracion_python_rust/INFORME.md) |
| Seguir los procesos desde el botón hasta la salida | [Flujos paso a paso](docs/migracion_python_rust/FLUJOS.md) |
| Comparar cada texto y umbral de fábrica | [Catálogos Python/Rust](docs/migracion_python_rust/CATALOGOS.md) |
| Localizar cada diferencia y su prueba de aceptación | [Matriz por función](docs/migracion_python_rust/MATRIZ.md) |
| Implementar por entregas, en orden de dependencia | [Plan de ejecución](docs/migracion_python_rust/PLAN.md) |
| Repetir la verificación y distinguir resultados de pendientes | [Pruebas y evidencia](docs/migracion_python_rust/VERIFICACION.md) |
| Buscar un archivo Python, su propósito y sus símbolos de entrada | [Inventario de las fuentes Python](docs/migracion_python_rust/INVENTARIO.tsv) |
| Iniciar otra sesión de IA | [Instrucción de continuidad](docs/migracion_python_rust/CONTINUAR.md) |

Corte del estudio: 19 de noviembre de 2026, Chile. Base completa: `flow-python@93a36312adbd24e2dc96b02dfac45d947193b8b1`. La rama contiene los fuentes Python, contratos, datos sintéticos, matrices Word, diez crates Rust, extensión, oráculos y pruebas heredadas. La entrega agrega la referencia local y el análisis. El avance del producto y su aceptación en Windows se registran aparte.
