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

## Decisiones del usuario vigentes (20-nov-2026)

Registradas en detalle en el [informe de fallas](INFORME_FALLAS_20261120.md#decisiones-del-usuario--20-nov-2026). Obligan a Flow Rust:

- **Producto Excel.** Copia del consolidado o del original con las columnas NURUS al final, en el orden `NURUS_ID_REGISTRO, NURUS_REGLAS, NURUS_PROPUESTA, FECHA_OBS, OBSERVACION, TT, CC, RES, NURUS_ESTADO_REVISION, NURUS_DECISIONES`. Debe calzar con el registro mensual del usuario. Prima sobre el orden del código de `flow-python`.
- **Redacción de observaciones.**
  - **R1:** con medida por vencer, sin la oración del próximo informe.
  - **R2:** la imposibilidad de atribuir un trámite de la causa se registra como aviso interno cuando hay varios ingresos.
  - **R3:** DCE con un solo encabezado y sin días.
  - **R4:** sin días de espera cuando el tribunal dispuso el ingreso.
  - **R5:** texto vigente en lugar de reiterar solo para «Medida revisada, sin observaciones.» y para la derivación sin seguimiento; en lo demás se reitera.
  - **R6:** nombre del programa en tipo título.
  - R7 y R8 sin cambio.
- **Alcance.** Los cambios de redacción se aplican en Flow Rust. Ajustar Flow Python requiere instrucción expresa. Cada regla cambiada lleva casos ficticios en la línea golden de Rust.
- **Orden de corrección.** Producto Excel y marcas, extensión y contactos, plantillas de resolución, redacción, E3 (Mejor Niñez, escritos e historia) e inventario de opciones.
