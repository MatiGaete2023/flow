# Aceptación institucional — CSMP Assistant personal 0.4.0.dev11

Estado al 22 de septiembre de 2026: **VALIDACIÓN FUNCIONAL REAL DEL FLUJO PRINCIPAL SATISFACTORIA**. El usuario confirmó funcionamiento en procesamiento/modificación del Excel, RES categórico, generación de resoluciones, creación y edición de borradores de correo y modificación de parámetros. Las pruebas institucionales específicas que no fueron verificadas expresamente permanecen pendientes.

Responsable: usuario principal. Fecha: 22-09-2026. Commit base probado: `f67af0998640223344cc9af653f80b8334a82fe6`.

## Validación funcional real del 22 de septiembre de 2026

El usuario informó una prueba satisfactoria del flujo real: procesamiento del Excel y modificación de su contenido, selector RES categórico, creación de proyectos de resolución, creación y modificación de borradores de correo y modificación de parámetros/configuración. El fallo COM detectado previamente al configurar RES (`'tuple' object is not callable`) fue corregido y el mismo flujo volvió a funcionar en Excel real. Esta evidencia justifica congelar dev11 como **candidata funcional**, pero no convierte automáticamente en aprobadas las pruebas ACEP que requieren condiciones específicas no reportadas (duplicados Outlook, Enviados, Cumplimiento con/sin cruce, matrices faltantes, reinicio, medición de tiempos, etc.).

| ID | Acción | Criterio de aceptación | Resultado / evidencia |
|---|---|---|---|
| ACEP-01 | Instalar y abrir | Abre cinco áreas sin privilegios administrativos; registra Python 3.12–3.14 | Pendiente |
| ACEP-02 | Procesar Espera | Copia abre sin reparación; cantidad y observaciones esperadas; original intacto | **OK funcional 22-09** — procesamiento Excel real confirmado tras hotfix RES |
| ACEP-03 | Editar en Trabajo y exportar directamente | Sin pulsar antes “Aplicar edición”, la copia contiene la observación visible; fila editada `REVISADO`, filas intactas `PENDIENTE`; TT/CC/RES previos no se borran; `RES` nuevo ofrece desplegable PC_IE/PC_INFO/NOMENCL | **OK funcional 22-09** — modificación del Excel y RES confirmadas |
| ACEP-04 | Cumplimiento con y sin cruce | Con cruce: C-10 según fuente; sin cruce: advertencia, sin C-10 y sin diálogo de excepción/bloqueo | Pendiente |
| ACEP-05 | Editar planilla y actualizar | Cambios asociados por identidad; Correos/Word reutilizan la copia sin nueva carga | **OK funcional 22-09** — cambios Excel reutilizados en productos |
| ACEP-06 | Preparar correos | **Preparar TODOS** incluye informativo general + comunicaciones específicas; modalidades correctas; Para puede quedar vacío; CC institucional; nómina por programa y sin excluidos | **OK funcional 22-09** — creación y edición de correos confirmadas; casos de borde siguen cubiertos por CI |
| ACEP-07 | Guardar lote de borradores | Todos quedan en Borradores, ninguno en Enviados; repetir preparación idéntica no duplica aunque cambie la carpeta temporal del adjunto; editar cuerpo/destinatario sí produce versión nueva | Pendiente |
| ACEP-08 | Clasificar resoluciones | `PC_IE`, `PC_INFO` o `NOMENCL` en RES prevalece y aparece como `Definido en RES`; un valor RES desconocido se advierte sin generar tipo; marcas antiguas 1/X siguen legibles; una observación sin acción ni RES no crea proyecto | **OK funcional 22-09** — RES categórico y reconocimiento confirmados |
| ACEP-09 | Editar resoluciones una a una | Lista con una fila por tribunal/RIT/tipo; cambiar tipo afecta solo selección explícita; no seleccionar nada no modifica todos | Pendiente |
| ACEP-10 | Generar Word Laja/Mulchén | Un Word; una resolución por grupo tribunal/RIT/tipo; varios NNA junto a su cédula; saltos de página; fechas en palabras | **OK funcional 22-09** — creación de resoluciones confirmada |
| ACEP-11 | Matrices | Seis matrices Laja/Mulchén disponibles; Tomé se informa como faltante sin sustituto automático | Pendiente |
| ACEP-12 | Enviados/estadísticas y reinicio | Consulta solo lectura, exportación correcta, sesión recuperable y tiempos/clics registrados | Pendiente |
| ACEP-13 | UX final | Búsqueda por RIT/nombre/programa, filtros no destructivos, tipos RES legibles, barra de contexto, carpeta de salida y atajos funcionan a 1024×650 sin ocultar acciones críticas | Pendiente |

Ante un error, registrar ID, operación, mensaje completo, versión/commit y archivo de prueba; conservar original y salida. Ante guardado Outlook incierto, revisar Borradores antes de repetir.

Antes de declarar una release candidata debe además resolverse el conflicto DCE documentado en `ESTADO_CONSOLIDADO_20260920.md` y `CAMBIOS_USO_20260916.md`.

Aceptación funcional del flujo principal: **APROBADA PARA FASE DE USO COTIDIANO CONTROLADO**. La aceptación institucional completa continúa abierta para los ACEP todavía pendientes. No promover a 1.0 estable hasta completar el período breve de uso real y revisar incidencias.
