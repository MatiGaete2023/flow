# CSMP 0.5.0.dev4 + SITFA Descargador 2.7.2 — integración de bitácoras
Actualizado el 8 de octubre de 2026.

## Producto y alcance
La rama `feat/consolidacion-bitacoras-20261008` parte de la versión 0.5.0.dev3
de `main`; conserva Trabajo, Correos, Resoluciones, Resultados, Configuración,
nóminas editables, Word y correo manual, preparación de borradores y recuperación.
Incorpora el lector, parser compartido, importación HAR, análisis y exportación
de bitácoras RUS de la rama experimental `f2809581`, junto al Descargador y
extensión 2.7.2.

Acceso: **Resultados → Más acciones → Bitácoras RUS**. Controles:
Consultar bitácoras (Chrome autenticado), recuperar consulta, importar HAR y
abrir `bitacoras.json`. Se elige el rango de análisis (hasta cuatro meses);
«Copia íntegra» contiene el texto de las observaciones de la tabla recuperada.
No existe envío de correo ni escritura de observaciones en este recorrido.

## Identidad, límites e integridad
Se coteja tribunal, causa e ingreso RUS, más contexto de etapa y modalidad.
Las diferencias visuales de nombre/RIT producen advertencias sin mezclar
ingresos; una identidad remota incompatible se rechaza. El control de lecturas
preserva errores, consultas fallidas y hash SHA-256; no presenta un fallo como
una bitácora vacía. Un HTML visible no acredita por sí solo todo el historial
que pudiera tener el servidor.

## Distribución y aceptación
Los workflows verifican CSMP 0.5.0.dev4, Descargador/extensión 2.7.2,
pruebas de identidad y recuperación, y construyen un paquete Windows conjunto.
La ejecución exacta de CI determina si está aprobado: este documento no
presume resultados de pruebas aún no realizadas.

Para usar lectura en lote, descargar **CSMP-Descargador-Integral-dev4-SITFA-272** desde
GitHub Actions del commit aprobado. Extraer completo en carpeta nueva.
Desactivar extensiones antiguas y cargar la carpeta `extension` que viaja
con los dos EXE. El instalador individual de CSMP es un producto separado.

## Verificación institucional pendiente
Reabrir las dos bitácoras que anteriormente fallaron (10 de 12 consultadas),
contrastar RIT visible, nombre, texto completo, fecha, tipo y CC con RUS.
Extender a Residencia, Ambulatorio, FAE y DCE, pestañas, paginación,
interrupción y reintento. Nunca publicar HAR ni datos de NNA en el repositorio.
No se habilita escritura RUS ni se modifica SATURNO.

## Diagnósticos de octubre: lectura pendiente de aceptación

La entrega SITFA 2.7.2 corrige el reporte de versión de la extensión (el
manifiesto 2.7.1 anunciaba equivocadamente 2.7.0), exige coincidencia
explícita entre Descargador y extensión, excluye texto de scripts
`<script>` en celdas RUT y admite `ShowObservaciones` único sin
`ShowHistoria` solo cuando sus identificadores remotos están presentes
y son válidos. Si `ShowHistoria` existe, se contrasta su identidad.

En el lote real Ambulatorio/Espera se observaron 98 de 98 filas
`SIN_VINCULO`. Su causa exacta sigue pendiente porque los JSON no
incluyen la respuesta HTML de la tabla. El manifiesto ahora ofrece
`vinculo_diagnostico` categórico sin contenido personal. No se
generan consultas GET sin un `ShowObservaciones` inequívoco, no se
adivinan IDs a partir de RIT, no se escribe en RUS.

El lote Residencia (10/12 leídas) y DCE (2/22 leídas) mostraba el
mensaje antiguo de validación estricta de RIT/persona, ausente del
lector consolidado. Los manifiestos recibidos indicaban build
`8c58b998` y extensión `2.7.0`, no prueba suficiente del paquete
integral aprobado. Reemplazar ambos EXE y la extensión desde
**un mismo artefacto aprobado** y comprobar el nuevo manifiesto.
No publicar JSON ni HTML con datos de NNA en GitHub.

## Separación de líneas
`main` y `experimento/plan-integral-csmp-20261002` quedan intactas.
La consolidación de código está en una rama de integración; su promoción
exige resultados de CI y aceptación con sesión RUS real.

### Diagnóstico anónimo de fallas de bitácoras (revisión octubre 2026)

Cada ejecución escribe `diagnostico_bitacoras.json` junto al
`bitacoras.json` original. **Compartir únicamente el diagnóstico anónimo**;
incluye estado, versiones de Descargador/extensión, conteos por tribunal
numérico, modalidad y pestaña, tipos de SIN_VINCULO y errores categóricos,
pero no nombres, RIT, RUT, identidades remotas, textos de observaciones,
HTML, archivos locales ni rutas del equipo.

El `bitacoras.json` y las capturas HTML conservan información protegida:
no subirlos a GitHub, tickets ni servicios públicos. Si una revisión
requiere evidencia de un enlace o de la respuesta real de RUS, debe
efectuarse localmente bajo las autorizaciones institucionales.

Este reporte **no acredita por sí solo** que la bitácora coincida con
la visualizada en RUS ni que exista lectura íntegra de otras pestañas.
