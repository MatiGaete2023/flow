# Instalación recuperable y excepción de Cumplimiento

Fecha: 9 de septiembre de 2026.  
Código: `81769e3052d67f053efd55ee6892fdb581ad0ee9`.  
Rama: `implementacion-plan-2026-09-08`.

## Error de instalación: no se encontró `py -3.12`

El mensaje recibido no acredita que Python 3.12 no esté instalado. Solo acreditaba que el instalador anterior no encontró el lanzador `py`.

El instalador primero reutiliza `.venv\Scripts\python.exe` cuando ya existe y es Python 3.12. En ese caso no necesita el comando `py`. Para crear un entorno nuevo prueba:

1. la ruta indicada en `NURUS_PYTHON_EXE`;
2. `py -3.12`;
3. `python`, si corresponde a Python 3.12;
4. `python3.12`;
5. `%LOCALAPPDATA%\Programs\Python\Python312\python.exe`;
6. `%ProgramFiles%\Python312\python.exe`.

También verifica que una carpeta `.venv` existente use Python 3.12 antes de reutilizarla. Si no cumple, se detiene sin borrarla.

La instalación incorpora los extras de Excel/Outlook y `python-docx`. Si existe `paquetes\`, usa exclusivamente ese repositorio local; de lo contrario `pip` necesita acceso autorizado a dependencias.

### Qué hacer en el otro PC

1. Actualiza la copia de NuRus para incorporar el nuevo `Instalar_NuRus.bat`.
2. Abre `cmd` en esa carpeta y ejecuta:

   ```cmd
   Instalar_NuRus.bat
   ```

3. Si vuelve a indicar que no encuentra Python 3.12, ejecuta:

   ```cmd
   python --version
   python3.12 --version
   ```

   Si conoces la ruta exacta, ejecuta `set "NURUS_PYTHON_EXE=C:\ruta\a\Python312\python.exe"` y vuelve a llamar a `Instalar_NuRus.bat`. Si ninguna alternativa muestra `Python 3.12.x`, corresponde instalar Python 3.12 por usuario o solicitar soporte institucional.
4. Si informa que `.venv` no usa Python 3.12, cierra NuRus y renombra esa carpeta, por ejemplo a `.venv-anterior`; vuelve a ejecutar el instalador. No borrar una base de NuRus ni respaldos de datos.
5. Abre `Abrir_NuRus.bat` solo después de que la instalación termine correctamente.

## Excepción de hoja cruzada de Cumplimiento

Cuando Cumplimiento no contiene una hoja de cruce identificable, la pantalla informa que se requiere excepción documentada. El lote no se puede aprobar mientras falte esa decisión.

La persona revisora debe presionar **Documentar excepción de cruce**, indicar responsable y motivo, cargar el Excel revisado y después **Congelar constancia**. NuRus guarda la excepción dentro del snapshot con:

- código de excepción;
- responsable;
- motivo;
- fecha de registro.

Modificar posteriormente ese registro devuelve el lote a revisión y genera un nuevo snapshot al aprobarlo. Los snapshots anteriores conservan la excepción que tuvieron.

Si existen varias hojas de cruce posibles o columnas ambiguas, NuRus no permite la excepción. La excepción no reemplaza la revisión individual en RUS; la constancia debe identificar todas las filas y contener fecha/observación válidas.

La hoja oculta de trazabilidad de la exportación incorpora la información de excepciones del snapshot.

## Verificación

[Actions 34369824759](https://github.com/MatiGaete2023/NuRus/actions/runs/34369824759) ejecutó:

- Linux: **80 pruebas aprobadas**.
- Windows: **79 pruebas aprobadas y 1 omitida**, porque requiere Excel de escritorio real.

Las pruebas cubren detección del instalador sin lanzador `py`, respaldo de migración v4→v5, rechazo de Cumplimiento sin excepción, validación de responsable y motivo, rechazo de hoja ambigua, persistencia de la excepción dentro del snapshot y preservación de los snapshots anteriores.

## Aceptación institucional pendiente

La CI no puede acreditar Excel 2010, Outlook clásico ni la configuración institucional. Ejecutar una vez en el PC objetivo:

1. Instalar con el script actualizado y abrir NuRus.
2. Importar copias de una planilla Espera, una Cumplimiento sin cruce, una Cumplimiento con cruce, una Informes y, si se usan, archivos `.xls` y `.xlsm`.
3. En Cumplimiento sin cruce, comprobar que no congela sin excepción; documentar responsable y motivo, cargar el Excel revisado, congelar y revisar la traza.
4. Exportar y comprobar fórmulas, filtros, anchos, estilos, hojas ocultas, OB y Medidas vencidas, las columnas de NuRus y las filas excluidas.
5. Preparar un producto de correo con destinatario vacío y verificar la CC obligatoria. Guardar únicamente un borrador en Outlook, confirmar que no se envía y revisar la cuenta/carpeta correcta.
6. Conservar el resultado de cada paso, el hash del snapshot y cualquier mensaje de error. No usar planillas reales en GitHub.

[L-SIGUIENTE]
Ejecutar esta aceptación en Windows 10 con las copias institucionales y registrar los resultados antes de integrar la rama en main.
