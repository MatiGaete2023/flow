# Anonimizador CSMP portable

Windows 10/11 de 64 bits (x64). Herramienta **independiente**: no necesita Python, Excel, el Asistente ni el
Descargador, y no contiene código de ninguno de los dos.

## Uso

1. Extrae el ZIP en una carpeta local.
2. Abre `AnonimizadorCSMP.exe`.
3. **Agregar archivos…** o **Agregar carpeta…**: Excel (`.xlsx`, `.xls`, también los `.xls` que en realidad son
   HTML o XML), HTML, JSON, CSV, ICS, TXT, Word o ZIP, incluidos los paquetes del Descargador y las entregas del
   Asistente.
4. Elige el modo (Estricto recomendado) y pulsa **Anonimizar**.
5. Las copias quedan junto al original (o en la carpeta de destino que elijas): una sola entrada produce
   `Copia_CSMP_anon_<aleatorio>.<ext>`; varias entradas o una carpeta producen `anonimizado_<aleatorio>/` con un
   `informe_anonimizacion.txt` sin datos del contenido.

La ventana no muestra nombres ni rutas de los archivos (solo «Entrada n: tipo») y los mensajes de error son
genéricos, para no filtrar datos al compartir una captura de pantalla. El procesamiento es local: no hay red,
servicios externos ni registros. El selector de archivos de Windows sí muestra tus carpetas.

Todos los archivos de una misma ejecución comparten seudónimos: «Persona Ficticia 3» es la misma persona en el Excel,
el JSON y el ZIP. Con una frase de **semilla** también lo es entre ejecuciones.

Revisa manualmente toda copia antes de compartirla: ninguna modalidad garantiza anonimización absoluta y las fechas o
combinaciones de hechos pueden identificar indirectamente a alguien. Detalle de alcance y límites en
`anonimizador/README.md` (incluido en `AnonimizadorCSMP-fuentes.zip`).

## Verificación de la entrega

`SHA256SUMS.txt` contiene la huella del ejecutable y de los otros archivos:

```powershell
Get-FileHash .\AnonimizadorCSMP.exe -Algorithm SHA256
```

Autoprueba exclusivamente sintética del ejecutable (el informe debe tener un nombre nuevo; si existe, termina con
código 2 sin sobrescribirlo):

```powershell
$p = Start-Process .\AnonimizadorCSMP.exe -ArgumentList '--self-test autoprueba-nueva.json' -Wait -PassThru
$p.ExitCode
Get-Content .\autoprueba-nueva.json
```

Resultado esperado: código 0 y `PASA`. No usa archivos personales ni conexión.

El ejecutable no tiene firma Authenticode. `VERIFICACION.json` indica el entorno exacto de compilación. Una prueba
sintética no es aceptación institucional.

## Código y recompilación

`AnonimizadorCSMP-fuentes.zip` contiene el paquete `anonimizador_csmp`, sus pruebas, la procedencia (commit y huellas),
las dependencias de compilación fijadas y el workflow. Desde su raíz, con Python 3.13 x64 para Windows y Tkinter:

```powershell
python -m venv .venv
.\.venv\Scripts\python.exe -m pip install -r portable/requirements-build.txt
.\portable\build.ps1 -Python (Resolve-Path .venv/Scripts/python.exe).Path
```

La compilación requiere PyPI. No se promete reproducción bit a bit.

El workflow `anonimizador-portable.yml` prueba en Linux (Python 3.12) y Windows, compila, ejecuta la autoprueba del
`.exe` y publica un artefacto con retención de 30 días. No publica releases ni fusiona ramas.
