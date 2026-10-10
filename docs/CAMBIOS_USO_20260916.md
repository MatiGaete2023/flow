# Ajustes de uso — 16 de septiembre de 2026

Versión vigente: **0.4.0.dev6**. Este documento registra únicamente la consolidación realizada el 16-09-2026. Los cambios anteriores permanecen en sus documentos históricos por fecha.

## Consolidación dev6

- Se eliminan los cuatro módulos transitorios `runtime_fixes_20260916*`. Sus comportamientos pasan a módulos definitivos; importar `nurus.personal` deja de producir monkey-patching.
- Trabajo captura la observación visible al exportar. Las filas editadas quedan `REVISADO`; las restantes `PENDIENTE`. Los campos técnicos existentes que no fueron modificados se preservan.
- Se mantiene la neutralización de fórmulas de Excel para observaciones que comienzan con caracteres interpretables como fórmula.
- La identidad de borradores pasa a contenido efectivo: Para, CC, asunto, cuerpo, obligatoriedad y nombre/hash de adjuntos. La ruta UUID temporal deja de formar parte de la identidad.
- **Guardar TODOS** puede preparar y guardar el lote si todavía no existe una vista previa preparada.
- Resoluciones usa precedencia tipo explícito en `RES` > observación revisada > acción del motor. Una observación sin RES ni acción de proyecto no crea por sí sola una resolución.
- La deduplicación tribunal/RIT/tipo se ejecuta antes de poblar la interfaz, no como limpieza visual posterior.
- Cambiar el tipo requiere una selección explícita y solo afecta esas filas. Agregar manualmente evita duplicar un proyecto ya visible para la misma causa/tipo.
- Se eliminan imports privados sin uso en los módulos de resoluciones/trabajo intervenidos y la función privada `_excel_column_name` sin referencias del exportador.
- Se agrega contrato de higiene de repositorio para impedir reaparición de `runtime_fixes_*` y basura generada común.
- El workflow se actualiza para Windows y la distribución vigente pasa a `CSMP-Windows-dev6`.

## Verificación

La CI del HEAD de la release comprueba instalación aislada, wheel, dependencias, compilación, suite, recursos empaquetados, cinco parches de matrices y seis matrices base. Python 3.12 agrega smoke de la GUI y construcción de la distribución Windows.

## Límites vigentes

- No hay matrices de Tomé; no se genera una matriz artificial sin fuente.
- La CI no sustituye la prueba institucional real con Excel 2010 y Outlook clásico.
- No se escribe en RUS/SATURNO y no se envían correos; Outlook solo recibe borradores.

[CONFLICTO_ABIERTO] El Manual de Funciones CSMP 2025 indica, para DCE en Informes, pedir cuenta de informes pendientes de entrega de más de 40 días. El motor vigente clasifica como `I01_VENCIDO_DCE` cualquier informe DCE cuya fecha de vencimiento ya pasó. La auditoría no modifica esa regla sin resolver previamente cuál es la instrucción operativa vigente.

[G-ESTADO] Repositorio consolidado como **0.4.0.dev6**, sin la capa transitoria de parches de ejecución.
[L-SIGUIENTE] Ejecutar aceptación Office institucional con copias autorizadas y resolver el conflicto de umbral DCE contra la instrucción operativa vigente.
