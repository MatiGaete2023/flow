# Cómo probar CSMP y el descargador integrales

Versiones: CSMP **0.4.0.dev18** y descargador/extensión **2.4.4**. Estos paquetes usan configuración propia de prototipo. Conservar las carpetas anteriores permite volver a la versión de uso.

## Preparación

1. Extraer los dos ZIP en carpetas permanentes. Cada ejecutable necesita su carpeta `_internal` completa.
2. Actualizar la extensión desde la carpeta del descargador de esta versión: `_internal/extension`. En Chrome, cargarla o recargar la copia que apunta a esa carpeta. Confirmar versión **2.4.4**.
3. Entrar personalmente en RUS y abrir Seguimiento.
4. Abrir `CSMP_Integral.exe`. En Trabajo → Archivo/opciones → Descargar y analizar, seleccionar una vez el `SITFA_Descargador.exe` de esta entrega. CSMP abre una ventana del descargador para recibir su resultado.

## Flujo conjunto

1. En CSMP elegir Espera o Cumplimiento y pulsar **Descargar y analizar**.
2. Conectar la extensión al código de esa ventana del descargador. Conservar las pestañas de RUS y conexión. Cargar las opciones actuales, seleccionar tribunales/modalidades y pulsar **Descarga conjunta CSMP**.
3. Se consulta la principal completa e informes por vencer del mes actual y siguiente. La consulta de Carga/resoluciones firmadas se retiró del flujo porque bloqueaba su retorno a CSMP.
4. Al terminar, el libro vuelve a CSMP para el análisis y la copia inicial. Revisar observaciones, alertas y campos, y pulsar **Exportar copia actual** para obtener el producto final mediante Excel de escritorio.
5. La nueva descarga no contiene firmas: su cobertura figura **NO_CONSULTADA**. El detalle de firmas solo puede utilizarse con fuentes anteriores que sí contengan firmas; no prueba ausencia de movimientos en este flujo.
6. **Resultados** permite informes de firmas/revisiones o de gestión del período, del trabajo actual y de otras sesiones guardadas.

## Qué contrastar

- Todos los ingresos de la consulta permanecen, incluso antiguos. Varias firmas no multiplican filas principales.
- La falta de consulta de firmas se informa expresamente y no crea alertas de firmas inexistentes. Las exclusiones y los bordes anteriores se conservan.
- Los informes de los dos meses siguen el orden original de las fuentes y no se mezclan personas o centros que compartan RIT.
- Laja, Mulchén y Tomé conservan sus reglas, borradores y proyectos. Un tribunal diferente permite fuentes y alertas, sin esos productos históricos.
- Si se interrumpe una fase, **Más opciones → Retomar descarga conjunta** recupera las consultas verificadas y pide únicamente las pendientes.
- En Mulchén comprobar borradores automáticos/manuales, incluido Residencial RTA/RTT/RVA. Revisar en sus adjuntos la fecha o tiempo de espera y los bordes.

## Límites pendientes de completar

La descarga conjunta no consulta Carga. Para fuentes anteriores, la cobertura parcial y «Sin coincidencias en los informes consultados» tampoco acreditan ausencia de cambios en la causa.

El analizador y exportador común de bitácoras tienen pruebas locales, pero el lector real, la revisión de calendario individual, el registro automático de observaciones y sus recibos siguen pendientes. Estos ejecutables no registran observaciones nuevas en RUS. El informe de gestión solo cuenta como nuevas las que tengan un recibo de registro comprobado; un Excel importado no basta.

La aceptación completa sigue el documento de estado de los 32 elementos, incluido en ambos paquetes. Una falla debe conservar el lote y sus manifiestos para localizar la consulta afectada; evitar copiar credenciales o códigos de conexión en informes públicos.
