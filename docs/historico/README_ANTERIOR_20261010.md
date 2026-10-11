> Histórico: README de la raíz hasta el 10 de octubre de 2026 (WP-24). Vigente: [README](../../README.md) y [PLAN_VIGENTE](../../PLAN_VIGENTE.md).

# Flow — ejecución del plan NURUS / CSMP

Repositorio de trabajo: **MatiGaete2023/flow**. Entrega: CSMP `0.5.0.dev4`,
Descargador/extensión `2.7.5`. Baseline de procedencia: NuRus `251d5c4`.

**Resumen completo:** [fundamentos, planificación, problemas, implementación, pruebas y próximos pasos](../../RESUMEN_PROYECTO_NURUS_CSMP_20261009.md).

**Corrección del ZIP:** [recuperar o excluir referencias de bitácoras no disponibles](../CORRECCION_REFERENCIAS_ZIP_20261009.md).

**Para aprender a usar el Descargador:** [manual con imágenes](../manual_descargador_2_7_4/README.md),
[PDF](../manual_descargador_2_7_4/Manual_ilustrado_Descargador_SITFA_2.7.4.pdf)
y [práctica sin SITFA](../manual_descargador_2_7_4/Practica_ficticia_Descargador_2.7.4.zip).
Incluye recuperar bitácoras anteriores, generar su Excel y reunir hojas para el Asistente.

Para retomar: **[CONTINUAR.md](../../CONTINUAR.md)**. Documentos originales:
[plan y especificaciones](../plan_20261008/README.md). Primera entrega:
[ejecución P0](../EJECUCION_BASELINE_20261008.md),
[matriz funcional](../MATRIZ_FUNCIONAL_20261008.md) y herramientas de auditoría
en `tools/flow_baseline_*.py`.

El MVP implementa intercambio `CSMP_CAPTURE_V1`, Excel autónomo de bitácoras,
importación local, revisión y productos. Las nuevas reglas CT están desactivadas;
historia/PDF y aceptación institucional siguen pendientes. Consulta la
[guía vigente](../GUIA_ENTREGA_MVP_20261008.md) y [trazabilidad](../TRAZABILIDAD_EJECUCION_20261008.md).

La especificación 02 ya está incorporada: [análisis y cambios pertinentes](../ANALISIS_ESPECIFICACION_02_20261008.md).
La matriz comprende 115 requisitos de las cuatro especificaciones.

# CSMP Assistant · Windows

**Versión 0.5.0.dev4 · 8 de octubre de 2026 (integración en pruebas).**

Programa de escritorio para revisar el trabajo actual y preparar archivos y borradores. Tiene cinco páginas: Trabajo, Correos, Resoluciones, Resultados y Configuración.

La versión de esta rama conserva las funciones principales de la línea 0.5 y
añade importación local de paquetes preparados por el Descargador/extensión
2.7.5. Acceso: **Trabajo → Importar paquete**. Las consultas de bitácoras y su
Excel se realizan desde Descargador. La aceptación con RUS real sigue pendiente.

## Instalación

Requiere Windows y Python 3.12 si se usa la distribución de fuentes. La copia
Excel de un paquete funciona sin Office; la exportación preservada de Excel legado
requiere Excel de escritorio. La creación de borradores requiere Outlook clásico;
Outlook nuevo no ofrece la automatización COM utilizada.

Ejecuta `Instalar_CSMP.bat` y después `Abrir_CSMP.bat`. Para instalar el wheel desde PowerShell:

```powershell
py -3.12 -m pip install "./nurus-0.5.0.dev4-py3-none-any.whl[excel-legacy,excel-native,outlook]"
py -3.12 -m nurus.personal.app
```

## Flujo

1. Carga el Excel en Trabajo, revisa avisos y observaciones, y exporta una copia actual. El original se conserva.
2. Prepara correos: los adjuntos se generan automáticamente. **Editar adjunto (opcional)** permite cambiar solo la nómina del correo seleccionado; aceptar actualiza ese adjunto.
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

El inspector del registro RUS y su extensión de captura se retiraron por decisión del usuario (D3, 10 de octubre de 2026); el antecedente queda en [docs/historico/REGISTRO_RUS_AVANCE_20261006.md](REGISTRO_RUS_AVANCE_20261006.md).

Consulta [el informe de ejecución](../EJECUCION_WINDOWS_20261006.md).

## Mejoras de diseño y entrega · dev3

Configuración → Apariencia permite elegir Claro, Oscuro o Sistema. Ctrl+K abre la búsqueda de acciones. Resultados muestra productos, incidencias y acceso al elemento correcto. Las tablas se actualizan por ID y las cargas grandes ceden tiempo a la pantalla.

En la revisión de nóminas: F2 o doble clic para editar, Tab/Enter para avanzar, Esc para cancelar, Ctrl+V para revisar un pegado rectangular y Ctrl+Espacio para incluir/excluir filas. El texto pegado se conserva como texto Excel.

Los Excel nuevos tienen anchos por tipo de columna y encabezados repetidos al imprimir. Los Word conservan sus matrices; pueden generarse como proyectos por completar, pero la entrega final impide campos pendientes. La vista PDF requiere Word instalado. ZIP incorpora INDICE.txt y MANIFIESTO.json. Las operaciones por lote permiten detenerse entre productos.

El instalador Windows de Inno Setup instala por usuario, crea accesos directos y permite reinstalar conservando configuración. Consulte [implementación y comprobación](../MEJORAS_DISENO_20261007.md).

Para incidencias de bitácoras, compartir `diagnostico_bitacoras.json`
(generado junto al lote), no el `bitacoras.json` ni las copias HTML con
datos de NNA.

El avance funcional está en [implementación del MVP](../IMPLEMENTACION_MVP_20261008.md),
[contrato v1](../../contracts/csmp_capture_v1/README.md) y
[aceptaciones pendientes](../ACEPTACION_INSTITUCIONAL_PENDIENTE.md).
