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
   padre. Máximo por corrida: 600 bitácoras/calendarios y 600 historias; Retomar continúa desde el avance guardado.
   Si falla una ventana, conserva los Excel y las demás lecturas. No edites sus archivos mientras Flow descarga o retoma.
5. Al terminar, «Llevar Excel verificados a Trabajo» carga las tablas de intervención. Litigantes y calendarios
   quedan como consultas en la carpeta del lote. Si se activa la opción, una consulta vacía produce un PDF con su
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

## Qué todavía no hace

- Captura incorpora bitácora, historia y calendario verificados al análisis. Siguen pendientes los avisos de
  cronología CT-05/07/11 y los controles por paquete N-03/N-04, órdenes de búsqueda y cotejo de calendario;
  ver el estado detallado en EJECUCION.md.
- Anonimizador y Meta 4 siguen siendo las herramientas Python.

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
