# CSMP Assistant · Windows

**Versión 0.5.0.dev4 · 8 de octubre de 2026 (integración en pruebas).**

Programa de escritorio para revisar el trabajo actual y preparar archivos y borradores. Tiene cinco páginas: Trabajo, Correos, Resoluciones, Resultados y Configuración.

La versión de esta rama conserva las funciones principales de la línea 0.5 y
añade lectura de bitácoras RUS, sin escritura institucional, mediante el
Descargador/extensión 2.7.2. Acceso: **Resultados → Más acciones → Bitácoras RUS**.
Consulta el [informe de integración](docs/INTEGRACION_FINAL_20261008.md)
antes de usar la distribución. La aceptación con RUS real sigue pendiente.

## Instalación

Requiere Windows, Python 3.12 y Microsoft Excel de escritorio para la exportación preservada. La creación de borradores requiere Outlook clásico; Outlook nuevo no ofrece la automatización COM utilizada.

Ejecuta `Instalar_CSMP.bat` y después `Abrir_CSMP.bat`. Para instalar el wheel desde PowerShell:

```powershell
py -3.12 -m pip install "./nurus-0.5.0.dev4-py3-none-any.whl[excel-legacy,excel-native,outlook]"
py -3.12 -m nurus.personal.app
```

## Flujo

1. Carga el Excel en Trabajo, revisa avisos y observaciones, y exporta una copia actual. El original se conserva.
2. Prepara correos. En **Revisar nómina**, edita las celdas y marca las filas que se incluirán; después acepta para generar los adjuntos.
3. Revisa texto, destinatarios y adjuntos. El resumen final muestra el contenido efectivo antes de crear borradores en Outlook.
4. Prepara proyectos en Resoluciones, revisa sus datos y genera Word.
5. Consulta pendientes, fechas y entrega en Resultados. Un doble clic en un pendiente abre su registro o producto.

**Correo libre** y **Word manual** funcionan sin cargar Excel. Word admite texto libre o una matriz DOCX con variables completadas. Sus ediciones se guardan como trabajo manual actual. Los bloques favoritos admiten `{NOMBRE}`, `{RIT}`, `{TRIBUNAL}`, `{PROGRAMA}` y `{FECHA}`; se completan antes de insertarlos.

## Funciones adicionales

- Informe de calidad por fila: campos vacíos, números/fechas ilegibles e identidades repetidas para revisión.
- Explicación de sugerencias con reglas activadas, datos y umbrales configurados.
- Selección conservada aunque una búsqueda o filtro oculte filas.
- Preferencias de columnas visibles, recuperación de ediciones y productos actuales.
- Búsqueda de contactos con elección explícita entre coincidencias.
- Vencimiento de informe y egreso proyectado presentados por separado.
- Diagnóstico de versión, dependencias, matrices y registro de Office.
- ZIP de los productos que se seleccionen para la entrega.

La sesión y los recibos críticos se conservan para recuperar el trabajo y evitar efectos duplicados. El guardado omite escrituras cuando el estado completo no cambió. Los productos basados en datos modificados deben prepararse y revisarse nuevamente.

## Estado de entrega

La fuente disponible era `b95f0db`, del prototipo dev17. Las versiones locales dev19 y Descargador 2.4.4 de la memoria no se encuentran publicadas; las mejoras se implementaron sobre la fuente disponible. Este paquete no se identifica como una recuperación exacta de esas versiones.

Las pruebas locales incluyen interfaz con pantalla virtual y datos ficticios. La construcción del ejecutable y la aceptación con Excel y Outlook reales deben completarse en Windows. El flujo de Actions está configurado para Windows y la rama `feat/**`.

El registro real en RUS sigue pendiente del formulario y la sesión operables. Preparar una observación no acredita que haya sido guardada en RUS.

La herramienta separada **RUS · Inspector del registro actual** y su extensión de Chrome permiten capturar la estructura del formulario. Consulta [el avance de integración y las instrucciones](docs/REGISTRO_RUS_AVANCE_20261006.md).

Consulta [el informe de ejecución](docs/EJECUCION_WINDOWS_20261006.md).

## Mejoras de diseño y entrega · dev3

Configuración → Apariencia permite elegir Claro, Oscuro o Sistema. Ctrl+K abre la búsqueda de acciones. Resultados muestra productos, incidencias y acceso al elemento correcto. Las tablas se actualizan por ID y las cargas grandes ceden tiempo a la pantalla.

En la revisión de nóminas: F2 o doble clic para editar, Tab/Enter para avanzar, Esc para cancelar, Ctrl+V para revisar un pegado rectangular y Ctrl+Espacio para incluir/excluir filas. El texto pegado se conserva como texto Excel.

Los Excel nuevos tienen anchos por tipo de columna y encabezados repetidos al imprimir. Los Word conservan sus matrices; pueden generarse como proyectos por completar, pero la entrega final impide campos pendientes. La vista PDF requiere Word instalado. ZIP incorpora INDICE.txt y MANIFIESTO.json. Las operaciones por lote permiten detenerse entre productos.

El instalador Windows de Inno Setup instala por usuario, crea accesos directos y permite reinstalar conservando configuración. Consulte [implementación y comprobación](docs/MEJORAS_DISENO_20261007.md).

Para incidencias de bitácoras, compartir `diagnostico_bitacoras.json`
(generado junto al lote), no el `bitacoras.json` ni las copias HTML con
datos de NNA.
