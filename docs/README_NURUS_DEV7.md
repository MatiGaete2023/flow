# NuRus

Aplicación de escritorio local para analizar y revisar planillas del seguimiento de medidas de protección con trazabilidad y control humano.

**Versión de esta rama:** `0.3.0.dev7`. Requiere aceptación institucional antes de uso productivo.

Rendimiento y seguimiento actual: [informe dev6](docs/RENDIMIENTO_Y_PENDIENTES_20260913.md).

Revisión automática y mejoras de integridad: [informe anterior dev5](docs/REVISION_AUTOMATIZADA_20260912.md).

Correcciones de reconocimiento, guardado Excel y reutilización de la copia: [informe y prueba de actualización](docs/CORRECCIONES_EXCEL_20260912.md).

## Invariante de correo

**NuRus nunca envía correos electrónicos.** La integración Outlook solo puede guardar borradores mediante `Save()`, después de una confirmación explícita. El destinatario puede quedar vacío para completarlo durante la revisión humana. No existe una ruta automática de envío.

## Disponible actualmente

- Motor RUS único para **Espera, Cumplimiento e Informes**.
- Lectura de una copia estable del Excel; el SHA-256 corresponde a los mismos bytes procesados.
- Soporte `.xlsx`/`.xlsm` mediante `openpyxl` y `.xls` mediante el extra `xlrd`.
- Procedencia por archivo, SHA-256, hoja y fila física.
- Detección visible de columnas ambiguas y de filas bloqueadas.
- GUI: seleccionar archivo no equivale a analizar; botón **Analizar archivo** ejecuta lector y reglas fuera del hilo de Tkinter.
- Exportación inmediata del Excel de propuestas, conservando todas las hojas, fórmulas, estilos, anchos y filtros; las filas excluidas permanecen visibles y coloreadas.
- Retorno del Excel revisado mediante ID estable y comprobación de identidad; valida `OBSERVACION`, `FECHA_OBS`, `TT`, `CC` y `RES`, y exige confirmación humana de registro en RUS.
- Snapshot SHA-256 de la constancia; los productos posteriores usan ese snapshot y no recalculan la planilla.
- Cumplimiento sin hoja de cruce solo se aprueba tras documentar responsable y motivo; la excepción queda dentro del snapshot.
- SQLite local con claves foráneas activas, lote/origen persistente y versiones inmutables de plantillas.
- Correos agrupados por tribunal/programa, comunicaciones particulares, adjuntos limitados al grupo y edición de plantillas/políticas sin modificar Python.
- Proyectos Word desde matrices históricas identificadas, con revisión judicial y aprobación separadas.
- Adaptador Outlook conectado para **guardar borradores únicamente**, con recibo e impedimento de reintento automático cuando el resultado es incierto.
- Importación de contactos con vista previa y resolución por identidad/alias exactos.
- Contador de Enviados de solo lectura y estadísticas por `FECHA_OBS`, conservando los valores administrativos originales.

## Pendiente antes de uso productivo

- Aprobación funcional y jurídica de las matrices históricas de correos y proyectos.
- Definir, desde las matrices vigentes, el diccionario institucional de valores TT/CC/RES; hasta entonces las estadísticas los cuentan sin reinterpretarlos.
- Matriz completa de pruebas en Windows 10, Excel 2010 y Outlook clásico institucional.
- Aceptación del exportador nativo `.xls/.xlsx/.xlsm`, borradores reales y documentos Word en el equipo institucional.

## Instalación recomendada en Windows

La instalación debe quedar aislada del Python general del equipo. La rama `dev7` admite **Python 3.12, 3.13 y 3.14**; versiones posteriores no se consideran soportadas hasta ser verificadas.

1. Descarga o clona el repositorio en una carpeta estable.
2. Ejecuta `Instalar_NuRus.bat`. Si `.venv` ya es válido, no necesita `py`. Para una instalación nueva detecta Python 3.12–3.14 mediante `NURUS_PYTHON_EXE`, el launcher `py`, rutas estándar y los comandos `python`/`python3.x`.
3. Después abre con doble clic `Abrir_NuRus.bat`.

Al actualizar el código, vuelve a ejecutar `Instalar_NuRus.bat`: instala una copia del paquete, no una referencia editable al código fuente. La base de trabajo permanece en su ubicación local.

El instalador crea `.venv` y usa ese Python para NuRus; no modifica políticas de Windows ni desactiva controles institucionales.
Si existe una carpeta `paquetes`, instala desde ella sin Internet (`--no-index`). Esa carpeta debe contener todas las ruedas autorizadas y sus dependencias.

## Flujo operativo

1. Analiza una copia del Excel en Espera, Cumplimiento o Informes. Si hay varias hojas candidatas, usa **Hojas y encabezado…** para indicar la principal y el cruce; no se elige una hoja ambigua por su nombre.
2. Exporta propuestas; esto no acredita revisión.
3. Revisa cada registro en RUS y deja en Excel la constancia, fecha y campos administrativos.
4. Guarda y cierra el Excel exportado, pulsa **Usar revisión guardada** y confirma responsable/registro en RUS. NuRus recuerda la ruta por lote: no necesitas buscar otra vez el archivo. Para una copia movida o una devolución diferente, usa **Elegir otra copia…**.
5. Cada devolución parcial conserva su hash; las filas ausentes permanecen pendientes.
6. Si Cumplimiento no tiene cruce, documenta la excepción.
7. **Usar revisión guardada** intenta congelar la constancia al completar todas las filas revisables. Si falta una excepción de cruce, documéntala y pulsa **Congelar constancia**. Después prepara todos los productos desde esa constancia sin recargar el Excel para cada uno.

Para continuar otro día, usa **Retomar lote…**: recupera registros, copia guardada y constancia desde la base local, sin repetir el análisis.

Instalación manual equivalente desde `cmd`:

```cmd
python -m venv .venv
.venv\Scripts\python.exe -m pip install ".[excel-legacy,excel-native,outlook]"
.venv\Scripts\python.exe -m pip check
.venv\Scripts\python.exe -m nurus.app
```

Para desarrollo y pruebas:

```cmd
.venv\Scripts\python.exe -m pip install -e ".[dev,excel-legacy,excel-native,outlook]"
.venv\Scripts\python.exe -m pytest -q
```

## Datos locales

La base de trabajo se crea en `%LOCALAPPDATA%\NuRus` en Windows o `~/.local/share/NuRus` en otros sistemas. No debe instalarse en una carpeta compartida de red ni subirse a GitHub con información de NNA.

## Seguridad y aceptación

NuRus no escribe en SATURNO y no envía correos. Cualquier función no cubierta por la matriz de `docs/PRUEBAS.md` debe permanecer deshabilitada o identificada como no validada. El estado técnico y los pendientes se mantienen en `docs/IMPLEMENTACION.md`.
