# CSMP Windows · 0.5.0.dev3 · Diseño, velocidad y entregas

Implementación de las 16 propuestas del informe GitHub del 7 de octubre de 2026.

| Propuesta | Cambio aplicado | Comprobación |
|---|---|---|
| 01 Diseño | Paleta compartida, Claro/Oscuro/Sistema, adaptación del tamaño al escalado y foco de botones. | Cambio de tema y capturas de interfaz. |
| 02 Resultados | Lista de productos, estado, siguiente acción y navegación por identidad de producto. | Dos asuntos iguales abren borradores distintos. |
| 03 Nóminas | Editor de celda, Tab/Enter/Esc, pegado rectangular revisable e inclusión por teclado. | Pegado atómico y preservación de filas ocultas. |
| 04 Teclado | Ctrl+K y registro de comandos con condiciones; atajos existentes. | Paleta real y navegación. |
| 05 Medir | Herramienta reproducible y tiempos de operaciones en diagnóstico actual. | Mediana y p95 del dibujo de 1.000/10.000/50.000 filas. |
| 06 Tablas | Actualización por ID y cargas en bloques en el hilo Tk. | Latido de interfaz durante carga grande; selección oculta conservada. |
| 07 Reutilizar | Índices y caché limitado a una revisión sin mutaciones. | Igualdad con cálculo sin caché y caducidad tras editar. |
| 08 Progreso | Fases, avance conocido y detención en puntos seguros. | Unidades confirmadas no se repiten ni pierden. |
| 09 Excel | Formatos compartidos, anchos semánticos, títulos repetidos y ancho de impresión. | Reabrir libros, valores literales y opciones de impresión. |
| 10 Word | Comprobación en todas las partes XML, control de viudas y encabezados; PDF mediante Word. | Campos en encabezados y entrega bloqueada si incompleta. |
| 11 Correos | HTML sobrio opcional, firma conservada y resumen con destinatarios/adjuntos. | Literalidad HTML y prueba COM simulada; aceptación Outlook pendiente en puesto real. |
| 12 ZIP | Carpetas, índice y manifiesto de entrega actual; integridad de bytes archivados. | Archivos homónimos, SHA-256 y destino protegido. |
| 13 Calendario | Implementado en Descargador 2.6.0: título identificable, UID condicionado a identidad única y validación RFC. | Relectura con icalendar, casos ambiguos y cambio de fecha. |
| 14 Control final | Modelo de productos con problemas concretos y estado real de cada efecto. | Destinatario/adjunto ausente y documento pendiente. |
| 15 Instalación | Inno Setup por usuario con instalación, reinstalación y desinstalación verificadas en CI. | Ejecutable instalado abre sus cinco pantallas y conserva la carpeta de configuración. |
| 16 Flujo Windows | Pruebas de interfaz, comprobación nativa con pywinauto y herramienta de aceptación Office. | Batería automatizada y paquete Windows. |

## Uso y medición

Instalar la rueda o paquete antes de ejecutar herramientas. Para medir: `py -3.12 tools/benchmark_interface.py --rows 1000 10000 50000 --repeats 10`. Para perfilar lectura/análisis/nómina instala `.[performance]` y ejecuta `py -3.12 tools/profile_work.py --rows 1000`. El perfil usa sólo datos ficticios. Las medidas de benchmark son de dibujo; no equivalen al tiempo total de análisis ni a Office. Las tablas grandes ceden control a Tk; siguen representando todas las filas del modelo.

El formato institucional mantiene los bordes negros originales. Los formatos nuevos se aplican sólo a salidas generadas; la copia preservada del Excel conserva fórmulas y estilos de origen. El editor de nómina mantiene las correcciones dentro del adjunto.

Un proyecto Word puede contener `[COMPLETAR ...]` para continuar su edición en Word. Resultados lo señala y la entrega final lo bloquea. Completar el archivo y volver a actualizar Resultados permite revisar su estado. Una variable `{{...}}` pendiente siempre es un error de plantilla. La vista PDF usa una instancia separada de Word y no sustituye ni modifica el DOCX.

La detención es cooperativa. Una escritura Excel/Outlook en curso termina antes de comprobar la solicitud. Una tanda Word agrupada se publica sólo después de completar todos sus documentos. Las operaciones con resultado externo incierto conservan sus controles previos de recuperación.

## Instalación y actualización

Construir con PyInstaller y después `ISCC.exe installer\CSMP.iss`. Instala en `%LOCALAPPDATA%\Programs\CSMP_Assistant`; configuración y trabajo actual permanecen en `%LOCALAPPDATA%\CSMP_Personal_Prototipo_Integral`. Reinstalar una versión nueva permite actualizar el programa. Desinstalar no elimina los datos de trabajo.

Se eligió Inno Setup. La adopción de Velopack, Polars, un visor PDF web o una migración a otro toolkit estaba condicionada en el informe y no se justifica para estas mejoras. La actualización automática y la firma requieren un canal y certificado de publicación; el instalador construido no está firmado. No hay un servicio externo de actualización ni de correo.

## Aceptación en el puesto Windows

`py -3.12 tools/verify_office_windows.py` comprueba Excel y PDF mediante Word cuando Office está instalado. Con `--outlook-draft` guarda un borrador ficticio para revisar firma y adjunto; permanece en Outlook y debe eliminarse después. No envía correo. El informe JSON distingue resultados comprobados de no disponibles. La impresión y firma requieren revisión visual con la versión institucional de Office.

El adaptador de registro real RUS sigue necesitando evidencias del formulario autenticado. Las comprobaciones ficticias y las mejoras de interfaz no permiten afirmar que se registró en RUS.
