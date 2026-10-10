# Corrección de nóminas adjuntas — 30 de septiembre de 2026

Versión **0.4.0.dev14**, sobre dev13 (`382fa4190f1a68eb79da860f0f16dffa9c5ce84d`),
en la rama `codex/auditoria-ux-20260925-final`.

## Hallazgos reproducidos

- `F. VENCIMIENTO`, que usa el propio generador de nóminas, no estaba entre
  los encabezados reconocidos por el lector. Reimportarlo podía dejar la fecha
  sin mapeo e incluso inferir Espera si la hoja también tenía días de espera.
- El mapeo depende del modo del trabajo. Pedir otro correo desde una hoja con
  ambos datos dejaba vacío el campo que quedaba fuera de ese mapeo, aunque su
  columna estuviera presente.
- Las nóminas generales usaban una columna «VENCIMIENTO / ESPERA» y elegían el
  primer valor disponible. Una fecha ausente no debe sustituirse por días de espera.
- En el código dev13 revisado ya había bordes finos. No se reproduce su ausencia
  total con esa fuente. Dev14 fija el color negro de forma explícita y añade
  comprobaciones de las cuatro caras de cada celda al guardar/reabrir nóminas de
  informes, espera e informativos. No se atribuye la ausencia observada en un
  archivo concreto sin inspeccionar ese archivo o su instalación.

## Implementación

1. `rus/columns.py` comparte los alias de vencimiento entre Informes y el cruce:
   admite también F. VENCIMIENTO, F.VENCIMIENTO, FVENCIMIENTO y FECHA DE VENCIMIENTO.
2. `personal/outputs.py` elige el dato por categoría del correo: Informes usa
   vencimiento, Espera usa días de espera y Cumplimiento/Medidas usa egreso
   proyectado. La columna lleva el título específico; no mezcla esas magnitudes.
3. Si la columna no está mapeada por el modo activo, busca su alias en las celdas
   de la misma fila. No cambia el modo ni reejecuta reglas para obtener ese dato.
   Las correcciones tienen prioridad, incluido cero o vacío explícito. Dos
   columnas candidatas contradictorias se rechazan como ambiguas.
4. El adjunto muestra «Sin dato» si ese campo está ausente o solo contiene
   espacios. No inventa fechas ni convierte la ausencia en cero. Una corrección
   vacía sigue vacía en el trabajo; el Excel identifica expresamente esa ausencia.
5. Todas las celdas del área de registros, encabezados incluidos, tienen los
   cuatro bordes finos de color negro `FF000000`. No dependen de las líneas de
   cuadrícula de Excel. Se conserva la separación de nóminas por programa.
6. `product_state.py` incorpora a la huella el dato operativo efectivo del correo,
   también cuando proviene de una columna fuera del modo activo. Cambiarlo obliga
   a volver a preparar el producto para actualizar la nómina.

## Verificación obtenida

- Quince regresiones nuevas de importación, adjuntos, datos corregidos, ceros,
  datos ausentes, reimportación de nómina, bordes y dependencias.
- Ejecución enfocada: **92 aprobadas**. Suite completa en Windows/Python 3.13:
  **379 aprobadas, 1 omitida**, en 51,88 segundos. La omisión comprueba rechazo
  del backend nativo fuera de Windows.
- Wheel dev14 construido e instalado con pip en una carpeta aislada. Un proceso
  nuevo, sin `src` ni `tests` en su ruta de importación, prepara el adjunto de
  Informes desde una hoja Espera con F. VENCIMIENTO: fecha 15/10/2026, «Sin dato»
  para la fila sin fecha y bordes negros en las cuatro caras de todas las celdas.
- El paquete instalado conserva los cinco parches y seis plantillas Word.
  Compilación y `git diff --check` correctos.
- Las pruebas usan datos ficticios y no crean borradores reales en Outlook.
  La CI remota de Windows 3.12/3.13/3.14 se ejecuta después de publicar; los
  resultados locales no se presentan como resultados de esa ejecución remota.

## Actualizar y regenerar

Cerrar el asistente, descomprimir el ZIP dev14 y ejecutar `Instalar_CSMP.bat`;
después abrir `Abrir_CSMP.bat` y comprobar **0.4.0.dev14** en el título. Descargar
fuentes sin reinstalar no sustituye el paquete que usa el lanzador.

En Correos, volver a **Preparar tipo** o **Preparar todos** para regenerar las
nóminas automáticas. El arreglo no modifica retroactivamente archivos adjuntos
ya existentes. Revisar fecha/días y bordes antes de guardar en Outlook. Si figura
«Sin dato», revisar ese campo en la hoja o en Trabajo; volver a preparar después
de corregirlo. Se conservan las ediciones personales de texto y destinatarios.
