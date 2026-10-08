# Borrador de Mulchén ausente al filtrar por Residencial

## Causa reproducida

La clasificación de modalidades omitía los códigos residenciales `RTA`, `RTT`
y `RVA`, aunque existen en el catálogo de contactos del repositorio.
Al seleccionar solo «Residencial», el filtro descartaba sus registros antes
de agruparlos por tribunal. La selección manual también aplica este filtro.
Por eso el borrador no llegaba a aparecer en el asistente ni podía guardarse
posteriormente en Outlook.

La sesión local se examinó sin modificarla: los siete registros de Mulchén
pertenecían a programas RTA/RTT. Con todas las modalidades seleccionadas se
preparaba un borrador; con solo Residencial se preparaban cero. El mismo
resultado se obtuvo en automático y en manual. Tras la corrección, ambos
flujos preparan un borrador con los siete registros y sus destinatarios.
Las pruebas sintéticas conservan ese patrón sin incorporar datos personales.

Esto completa el diagnóstico pendiente en
[`correccion_mulchen_20260928.md`](correccion_mulchen_20260928.md): la corrección
anterior de alias resolvía una inconsistencia diferente, pero no este filtro.

## Cambios

- `modalities.py`: reconoce RTA, RTT y RVA como residenciales.
- Reconoce también las etiquetas completas de las cuatro modalidades, usadas
  en columnas «MODALIDAD» o «TIPO PROGRAMA».
- Al clasificar por programa, utiliza el valor corregido desde la interfaz.
- Mantiene los filtros: no incorpora programas ambulatorios, FAE, DCE o sin
  modalidad reconocida a una selección exclusivamente residencial.

## Comprobaciones

- Regresión inicial: cero borradores tanto en automático como en manual.
- Regresión corregida: un borrador con siete registros; sin incluir la fila
  ambulatoria de Mulchén ni las filas de otro tribunal.
- «Preparar todos» respeta la misma selección y genera el informativo.
- Prueba de interfaz Windows con preparación asíncrona: tarjeta visible,
  destinatarios y asunto cargados, siete registros en automático y uno al
  seleccionar una fila manualmente.
- Comprobación de la sesión local en lectura, antes y después del cambio.
- No se enviaron correos ni se guardaron borradores reales en Outlook.
- Pruebas enfocadas: **42 aprobadas**. Suite completa: **346 aprobadas,
  1 omitida y 2 fallidas**. Las dos fallidas comprueban que la altura del editor
  alcance 120 píxeles; se reprodujeron también en una copia aislada del commit
  anterior (`5d67d34` local, equivalente a `d4b3cc1` remoto), sin esta corrección.
  Esta incidencia previa de dimensiones no se modifica en este parche.

## Actualizar la aplicación instalada

Cerrar el asistente, descargar/descomprimir la rama
`codex/auditoria-ux-20260925-final`, ejecutar `Instalar_CSMP.bat` desde esa
carpeta y luego `Abrir_CSMP.bat`. Es necesario reinstalar: el instalador copia
el paquete al entorno virtual; descargar nuevos archivos fuente por sí solo
no actualiza el paquete que abre el lanzador. La configuración y la sesión
siguen en la carpeta de usuario `CSMP_Personal`.
