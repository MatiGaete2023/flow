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

- No descarga desde SITFA: se sigue usando el Descargador actual para obtener los Excel.
- No lee bitácoras ni historia de la causa: las reglas que dependen de ellas ya están en el motor, pero sin esa
  entrada no se activan.
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
