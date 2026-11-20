# Plan vigente · Python → Flow Rust

El destino es el escritorio que ya existe en `app/`. Incorporar las herramientas del usuario sobre su arquitectura eframe/egui y su [diseño vigente](DISENO.md). Reutilizar lógica del estudio anterior cuando encaje en las estructuras de la aplicación y pase el mismo caso de Python. La [matriz](MATRIZ.md) conserva los identificadores F01–F28 para aprovechar la trazabilidad de ayer.

| Entrega | Trabajo en `app/` | Comprobación necesaria |
|---|---|---|
| E0 · Base | Publicar el proyecto local como aplicación vigente. Fijar referencia visual, compilación y casos por fuente. | Probar `flow_rust.exe` y comparar sus vistas con la referencia de Flow Rust. Separar el estado de cada plataforma y de cada corpus. |
| E1 · ZIP y bitácoras | Completar `captura.rs`, adaptar tablas y procedencia a `trabajo.rs`; incorporar Revisión de bitácoras al flujo Trabajo. | ZIP 1.0, 1.1, 1.2, evidencia V1 y V2, legado incierto. Comparar estados, Excel y JSON con el exportador Python. Un ZIP inválido conserva el trabajo activo. |
| E2 · Descargador y Lite | Portar perfiles, lecturas, lotes, preferencias, pausa, reanudación y exportación. Adaptar el puente de escritorio en Rust y conservar el código que corre dentro de Chromium en JS. | Pruebas con sitio ficticio. Lotes Python→Rust y Rust→Python; resultados útiles sobreviven a la cancelación y al reintento parcial. |
| E3 · Motor y contexto | Ampliar `motor.rs`, `trabajo.rs` y configuración con historia, bitácora, calendario, Escritos, otro programa y consignación. | Cotejar texto, eventos ordenados, marcas, avisos, exclusiones y acciones con el corpus actualizado y casos multifila. |
| E4 · Trabajo y Excel | Integrar copia preservada, conciliación por celda y recuperación de sesiones. | Conservar hojas, fórmulas, estilos y decisiones en libro reordenado. Comprobar cancelación, archivo ajeno y conflictos. |
| E5 · Productos | Completar correos, nóminas, Word, ajustes, contactos, formularios manuales, resultados y vigencia. | Comparación semántica de productos y prueba de Outlook con doble de prueba. Verificar reintento y estado incierto. |
| E6 · Meta 4 y anonimización | Agregar el flujo Meta 4 y ampliar el Anonimizador local con formatos, lotes y comprobación de salidas. | Carpetas históricas Python, plantilla y regeneración. Coherencia de seudónimos y rechazo de salidas con identificadores originales. |
| E7 · Entrega y utilidades | Completar distribución Windows, ayuda, diagnósticos y utilidades. Revisar cada entrada operativa del inventario. | Instalación y ejecución sintética; comprobar salidas y pantallas desde el mismo binario. Registrar aceptación del usuario por separado. |

## Secuencia por dependencia

1. Completar el lector semántico detrás de la comprobación estructural. Portar encabezados CSV y límites, filas y estados, concordancia consulta-tabla, identidad de fuente, validación del consolidado y manejo de archivos opcionales. Solo entonces habilitar la apertura de ZIP en Trabajo.
2. Usar el fixture P2 de `descargador/test_revision_bitacoras_p2.py` para contrastar V2, y los casos del estudio anterior para V1 y legado. Las filas se asocian por sistema, tribunal, causa, ingreso y consulta. Paginación, falta de respaldo y lecturas contradictorias dejan el resultado no verificable. Emitir V2 solo con la evidencia que pide el contrato de Python.
3. Portar Descargador y Lite a flujos que usen los componentes visuales de [Flow Rust](DISENO.md). El Descargador maneja su conexión y lotes. Para el histórico opcional, usar la opción desactivada de origen, validar el titular visible y el marco de ruta prevista y guardar procedencia verificable. Suspender el histórico tras cinco fallas consecutivas y conservar bitácoras válidas. Tomar de Python los contadores, pausas y límites por tipo de apertura. Cubrir variantes de Escritos, paquetes parciales, favoritos y reanudación.
4. Mantener el avance y los criterios por fila en la matriz. Una prueba que pasa en el workspace histórico sirve como caso de contraste. La paridad se acredita ejecutando el destino `app/` desde su entrada normal.

El alcance abarca los flujos disponibles de Asistente, Descargador integral, Lite, Meta 4 y Anonimizador, más las utilidades operativas. Las herramientas de CI, pruebas y generación de material siguen disponibles como utilidades de desarrollo. Las funciones que el registro de Python marca postergadas siguen identificadas para decisión del usuario. Mantener las capacidades ya presentes en Flow Rust, incluido el guardado EML.
