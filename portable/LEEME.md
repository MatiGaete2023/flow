# Anonimizador CSMP portable

15 de octubre de 2026. Windows 10/11 de 64 bits (x64).

## Uso

1. Extrae el ZIP en una carpeta local.
2. Abre `AnonimizadorCSMP.exe`. No necesita instalar Python, Excel ni el Asistente.
3. Selecciona un Excel de observaciones o una captura ZIP `CSMP_CAPTURE_V1`.
4. Elige Conservador o Estricto y pulsa **Anonimizar**.
5. La copia `Copia_CSMP_anon_<aleatorio>.xlsx/.zip` queda junto al original.
   Espera la confirmación antes de cerrar la ventana.

La carpeta de origen debe permitir escritura. Trabaja en un disco local NTFS:
el código vigente publica las copias mediante enlaces físicos, que pueden no
estar disponibles en memorias FAT/exFAT o algunas carpetas de red.

El modo conservador sustituye identidades detectables y conserva texto útil.
El estricto omite textos libres completos. Revisa manualmente la copia antes de
compartirla: ninguna modalidad garantiza anonimización absoluta y las fechas
o combinaciones de hechos pueden permitir identificar indirectamente a alguien.

Acepta `.xlsx`, `.xlsm`, `.xls` y ZIP de captura versiones 1.0/1.1 reconocidas por
el contrato vigente. Reconstruye el Excel con valores; elimina macros, fórmulas,
objetos y metadatos. No conserva formato visual. Un ZIP genérico del Descargador
no equivale a `CSMP_CAPTURE_V1`. Los ZIP con PDF, HTML, documentos u originales
binarios se rechazan completos; no se excluyen elementos silenciosamente.

El procesamiento es local y no utiliza red, servicios externos ni registros del
contenido. El nombre y la ruta originales no se muestran en la ventana principal;
el selector de archivos de Windows sí muestra los archivos de tu equipo.
No cierres la aplicación durante el proceso. Ante un error, el original permanece
intacto y se muestra un mensaje genérico para no revelar datos.

## Verificación de la entrega

`SHA256SUMS.txt` contiene la huella del ejecutable y de los otros archivos.
Puedes contrastarla en PowerShell con:

```powershell
Get-FileHash .\AnonimizadorCSMP.exe -Algorithm SHA256
```

Para repetir la autoprueba exclusivamente sintética del ejecutable:

```powershell
$p = Start-Process .\AnonimizadorCSMP.exe -ArgumentList '--self-test autoprueba-nueva.json' -Wait -PassThru
$p.ExitCode
Get-Content .\autoprueba-nueva.json
```

Resultado esperado: código 0 y `PASA`. El informe debe tener un nombre nuevo;
si ya existe, la autoprueba termina con código 2 sin sobrescribirlo.
No utiliza archivos personales ni necesita conexión.

El ejecutable no tiene firma Authenticode. No se ha realizado una validación
manual en cada versión de Windows; consulta `VERIFICACION.json` para el entorno
exacto donde se compiló. Una prueba sintética no es aceptación institucional.

## Código y recompilación

Base: `MatiGaete2023/flow`, rama `feat/ejecucion-plan-20261008`, commit
`9726448af37b86cf73a87e25d087cea839fb641b`. La lógica original y el contrato se
reutilizan sin cambios. La entrada portable amplía la ventana para evitar recortar
estados y advertencias e incorpora una autoprueba. No modifica el Asistente.

`AnonimizadorCSMP-fuentes.zip` contiene el subconjunto necesario, su procedencia,
las instrucciones y las dependencias de compilación fijadas. Desde su raíz,
con Python 3.13 x64 para Windows y Tkinter instalado:

```powershell
python -m venv .venv
.\.venv\Scripts\python.exe -m pip install -r portable/requirements-build.txt
.\portable\build.ps1 -Python (Resolve-Path .venv/Scripts/python.exe).Path
```

La compilación requiere acceso a PyPI para instalar dependencias. La aplicación
resultante funciona sin Python instalado. La receta usa un entorno aislado para
no incluir dependencias del Asistente. No se promete reproducción bit a bit.

El workflow `anonimizador-portable.yml` compila en Windows, ejecuta las pruebas
existentes y la autoprueba congelada, y publica un artefacto con retención de 30 días.
No publica una release ni fusiona ramas. La integración al repositorio queda
sujeta a sus controles de revisión; esta entrega no declara `verificar_wp.py`
completo ni revisión E3 independiente.
