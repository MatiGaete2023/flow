# Flow (Rust)

Un solo programa para Windows, sin instalar Python: lee el Excel de RUS, propone la observación de cada registro y
prepara el Excel de observaciones, los borradores de correo y los proyectos de resolución en Word.

**No escribe en RUS ni en SATURNO. No envía correos: solo deja borradores. Todo correo lleva CC a
`ucc_concepcion@pjud.cl`.** Las observaciones son una propuesta: las revisa y registra el usuario.

Estado: en desarrollo, **sin aceptación del usuario**. Ver [EJECUCION.md](EJECUCION.md).

## Usar

1. Abre `flow.exe`. (Windows puede avisar que el editor es desconocido: el archivo no está firmado.)
2. **Trabajo:** «Abrir Excel de RUS…» (Ctrl+O). Sirve el `.xls` que entrega SITFA tal cual y también `.xlsx`.
   Revisa cada fila; puedes editar la observación y marcarla como revisada. Ctrl+S guarda el Excel de observaciones.
   La primera vez pide la carpeta donde dejar los productos.
3. **Correos:** marca las modalidades revisadas, «Preparar borradores», completa los destinatarios que falten y
   guarda: en Borradores de Outlook, o como archivos `.eml` que Outlook abre como correo sin enviar.
4. **Resoluciones:** «Preparar proyectos», completa lo que falte (por ejemplo la duración) y «Generar Word».
5. **Resultados:** lista de lo creado y acceso a la carpeta.

Al volver a abrir el mismo archivo y corte, Flow recupera las ediciones y marcas guardadas.
«Volver a analizar» descarta esas ediciones por elección del usuario. Si una sesión o los contactos no se
pueden leer, Flow avisa y conserva el archivo anterior; no lo sustituye por datos vacíos.

Los guardados en Outlook tienen un recibo local: repetir el botón o el comando omite los ya guardados.
Un resultado incierto exige revisar personalmente Borradores y resolver su estado desde el detalle del
correo. La consola conserva una copia `.eml` de respaldo y continúa con los demás productos si Outlook
falla. Una copia de respaldo de un guardado incierto no debe usarse antes de comprobar Outlook.

## Captura desde SITFA

1. Descomprime la carpeta `extension` incluida con Flow. En Chrome abre `chrome://extensions`, activa el modo
   desarrollador y pulsa «Cargar descomprimida» para elegir esa carpeta.
2. Abre tu sesión de SITFA en Chrome. En Flow, pestaña **Captura**, pulsa «Abrir conexión local» y «Copiar código
   de conexión». Pulsa el icono de la extensión Flow desde la pestaña SITFA, pega el código y conecta.
3. Vuelve a Flow, elige la pantalla, pulsa «Cargar opciones» y marca tribunales, modalidades y pestañas, o meses y
   año según la consulta. En Seguimiento puedes activar bitácoras, historia y calendario por ingreso; elige de uno
   a cuatro meses naturales. Revisa el filtro de fechas y pulsa «Descargar» para elegir la carpeta de destino.
4. Cada página se verifica antes de guardarse. «Detener» conserva lo comprobado; «Retomar lote…» permite elegir
   la carpeta incompleta. Las lecturas verificadas del día pueden reutilizarse entre lotes de la misma carpeta
   padre. Máximo por corrida: 2000 bitácoras/calendarios y 2000 historias; Retomar continúa desde el avance guardado.
   Si falla una ventana, conserva los Excel y las demás lecturas. No edites sus archivos mientras Flow descarga o retoma.
5. Al terminar, «Llevar Excel verificados a Trabajo» carga las tablas de intervención. Egresados y órdenes de
   búsqueda aparecen con sus avisos, sin observaciones ni acciones automáticas. Los calendarios quedan como
   consultas y aportan el cruce cuando sus columnas lo permiten. Si se activa la opción, una consulta vacía produce un PDF con su
   selección visible. Mantén abiertas las pestañas de SITFA y conexión durante el lote.

El código de conexión cambia al cerrar y abrir el enlace; no se guarda. Chrome conserva tu sesión y contraseña.
La extensión de Flow y el programa deben pertenecer a la misma entrega.

En **Contactos** se guardan los correos de programas y tribunales, el CC adicional y la firma. Quedan solo en el
equipo (`%APPDATA%\Flow`), junto con el trabajo en curso (hasta 10 sesiones, 60 días).

Plantillas Word propias (por ejemplo para Tomé): archivos `TOME_PC_IE.docx`, `TOME_PC_INFO.docx`, `TOME_NOMENCL.docx`
en `%APPDATA%\Flow\plantillas`, con variables `{{RIT}}`, `{{NOMBRE}}`, `{{RUT}}`, `{{PROGRAMA}}`, `{{FECHA}}`,
`{{FECHA_RESOLUCION}}`, `{{DURACION}}`, `{{NOMENCLATURA}}`.

## Por consola

```
flow observar ARCHIVO [--etapa espera|cumplimiento|informes] [--corte AAAA-MM-DD] [--json]
flow producir ARCHIVO --salida CARPETA [--modalidades TEXTO] [--contactos contactos.json] [--outlook]
```

En el detalle de Trabajo se muestran la cronología de bitácora, los avisos CT/N/OB y las proyecciones del
calendario por ingreso. La reiteración conserva las acciones de la propuesta original. Revisa los avisos antes
de preparar productos.

## Anonimizador

En la pestaña **Anonimizador**, agrega archivos o carpetas y elige un destino existente. Cada ejecución crea
una carpeta nueva con nombres neutros, copias verificadas e `INFORME.json`; no modifica los originales.
Por defecto usa modo Estricto. Conservador conserva más texto y exige una revisión más cuidadosa.
Puedes desplazar fechas, generalizar nacimientos y usar una semilla para repetir seudónimos. La semilla solo
permanece en memoria; no se guarda en ajustes ni informes. La cancelación elimina las copias parciales del lote.

Admite Excel, CSV/TSV, JSON/JSONL, texto, HTML, ICS, DOCX sin objetos y ZIP, incluidos CSMP_CAPTURE con huellas
recalculadas. Se excluyen PDF, imágenes, objetos incrustados y formatos no admitidos; puedes exigir rechazo
completo si aparece uno. Todos los archivos de la selección comparten la misma sesión de seudónimos.
Los documentos se reconstruyen: pueden perder formato, propiedades, enlaces y objetos. **Revisa el informe y
las copias antes de compartirlas. Ningún control automático garantiza anonimato absoluto.**

```text
flow anonimizar ARCHIVO_O_CARPETA... --salida CARPETA
                [--modo estricto|conservador] [--desplazar-dias N] [--generalizar-nacimiento]
                [--rechazar-no-admitidos] [--semilla-stdin]
```

`--semilla-stdin` lee la semilla de la entrada estándar, sin incluirla en argumentos o informes.

## Meta 4

En la pestaña **Meta 4**, agrega los lotes capturados por Flow (la carpeta que contiene `resumen.json`) con
Seguimiento → Espera y Cumplimiento de FAE y Residencia y sus bitácoras. Revisa la fecha de corte y pulsa
«Generar Meta 4». Sin marcar tribunales van todos; con alguno marcado, la planilla termina en `- parcial`.

- Usa las observaciones del **mes calendario que termina en el corte** (16/09 a 16/10; 28/02 a 31/03).
- FECHA OBS. es la observación del centro más reciente cuya columna Etapa sea Espera o Cumplimiento. Informes no
  participa. Sin observación en ese mes queda vacía; no se toma de meses anteriores.
- Rojo: registro en Cumplimiento cuya última observación es de Espera. Amarillo: revisar el control de bitácora
  (lectura no completada, leída antes del corte, identidad visible distinta, sin bitácora asociada, empate).
- Cada ejecución crea una carpeta nueva con `META 4 - actualizado AAAA-MM.xlsx`, `Registros.xlsx`,
  `Registros consolidados.xlsx`, `Bitacoras_del_mes.xlsx` y `Reporte META4.json`. Un error no deja nada a medias.
- La misma consulta en dos lotes se rechaza, para no duplicar registros.

```text
flow meta4 LOTE [LOTE…] --salida CARPETA_NUEVA [--corte AAAA-MM-DD] [--emision AAAA-MM-DD] [--tribunales "Laja,Tomé"] [--plantilla PLANTILLA.xlsx]
```

### Regenerar Meta 4 después de revisar Registros.xlsx

Abre Meta 4 y pulsa **«Regenerar desde Registros revisado…»**. Selecciona `Registros.xlsx` del resultado
anterior, ya corregido, y elige la carpeta de salida. También se puede usar la consola:

```text
flow meta4-regenerar "CARPETA/Registros.xlsx" --salida "CARPETA_NUEVA" [--plantilla PLANTILLA.xlsx]
```

El archivo `Reporte META4.json` y `Bitacoras_del_mes.xlsx` originales deben estar junto a
`Registros.xlsx`. Flow conserva las bitácoras, toma el período y los tribunales del reporte, relee las fechas
y etapas corregidas y recalcula las alertas. No altera la carpeta anterior y rechaza una salida existente.
El libro nuevo de Registros reconstruye las columnas conocidas: no copia hojas o columnas añadidas a mano.

## Qué todavía no hace

- Meta 4 lee los lotes capturados por Flow, no las carpetas del Descargador Python.
- La plantilla Excel personalizada es **opcional**: en la pestaña Meta 4 pulsa «Elegir plantilla Excel…».
  También se admite en consola añadiendo `--plantilla "C:\\Ruta\\plantilla.xlsx"` a los comandos
  `meta4` y `meta4-regenerar`. Sin plantilla se utiliza el diseño estándar.
- La plantilla debe ser un `.xlsx` con las cuatro hojas `Residencia Espera`,
  `Residencia Cumplimiento`, `FAE Espera` y `FAE Cumplimiento`. Cada hoja debe traer
  las filas modelo de tribunal, informe, fecha, cabecera RIT, un registro de ejemplo,
  «SIN REGISTROS» y «TOTAL:». Flow reutiliza estilos de celda, anchos, alturas y tema
  para reconstruir las cuatro hojas con **los datos calculados**, no con los datos
  de ejemplo de la plantilla. Las fechas se muestran como día-mes-año.
- No se importan fórmulas, dibujos, macros, relaciones externas ni objetos de la
  plantilla. Se rechazan hojas modelo combinadas y plantillas incompletas. El
  archivo aportado nunca se modifica, y un error impide publicar productos parciales.
- No hay instalador: es un solo `flow.exe` más la carpeta `extension`. Se copian y se usan.

## Desarrollo

```bash
cd rust && cargo test --workspace
```

```bash
cargo build --release -p flow
```

Ejecutable de Windows desde Linux (necesita `gcc-mingw-w64-x86-64` y el target `x86_64-pc-windows-gnu`):

```bash
cargo build --release -p flow --target x86_64-pc-windows-gnu
```

Las reglas se comparan contra el motor Python con `python3 rust/oraculo/generar_casos.py` (ver EJECUCION.md).
