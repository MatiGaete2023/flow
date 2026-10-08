# Migración de lectores y reglas RUS

El paquete `nurus.rus` integra Espera, Cumplimiento e Informes en un único flujo local:

```python
from nurus.rus import Mode, evaluate_batch, read_workbook

lectura = read_workbook("RUS.xlsx", Mode.ESPERA)
resultado = evaluate_batch(lectura)
```

No modifica el Excel, no escribe en SATURNO y no envía correos. Cada evaluación incluye:

- `source.workbook_name` y `source.workbook_sha256`;
- `source.sheet_name` y `source.row_number`;
- `related_sources` cuando Cumplimiento aplica C-10 desde la hoja secundaria;
- `rule_ids`, observación e incidencias revisables.

## Selección de hojas

Espera e Informes usan la primera hoja, salvo que se indique `sheet_name`. Cumplimiento prefiere una hoja llamada `Cumplimiento`; el cruce C-10 usa una hoja indicada por `cross_sheet_name`, una llamada `Hoja2` o la única hoja que tenga las seis columnas del cruce. Si existen varias candidatas, el cruce queda desactivado con una advertencia: nunca se toma una pestaña solo por ser la segunda.

## Textos editables

Los textos de observación están en `src/nurus/rus/textos_observaciones.json`. Se pueden editar sin cambiar Python. Cada entrada debe conservar los placeholders que utiliza su regla, por ejemplo `{PROGRAMA}` o `{FECHA_VENCIMIENTO}`. Para usar una copia de trabajo del catálogo:

```python
resultado = evaluate_batch(lectura, catalog_path="C:/NuRus/config/textos_observaciones.json")
```

Revise el JSON y ejecute las pruebas antes de publicarlo. Una clave eliminada o un placeholder incorrecto detiene la evaluación con un error claro, sin crear productos externos.

## Dependencias

Instalación habitual:

```powershell
py -3.12 -m pip install -e .[dev]
py -3.12 -m pytest -q
```

Para libros `.xls` de Excel 2010 instale también `.[excel-legacy]`. Los libros `.xlsx` y `.xlsm` usan `openpyxl`.

