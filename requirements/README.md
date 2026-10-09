# Construcción reproducible por plataforma

Los cuatro lockfiles fijan dependencias transitivas y hashes de distribuciones:
Windows CPython 3.12/3.13/3.14 (CI y EXE) y Linux CPython 3.12 (desarrollo).
No incluyen la distribución `nurus`: se instala desde el checkout identificado
después de instalar el entorno bloqueado. Los extras COM solo entran en Windows.

```sh
python -m pip install --require-hashes -r requirements/linux-py312.lock
python -m pip install --no-deps --no-build-isolation .
python tools/flow_dependency_inventory.py --lock requirements/linux-py312.lock --output artifacts/dependencies.json
```

En Windows elegir `windows-py312.lock`, `windows-py313.lock` o `windows-py314.lock`
según el intérprete. CI verifica las versiones realmente instaladas y distribuye
el inventario, SHA del lock y SHA del código con los artefactos. PyInstaller,
Playwright, Ruff, setuptools y wheel también están bloqueados. La compilación
wheel desactiva aislamiento para usar el setuptools fijado.

Actualizar los locks con uv (versión usada registrada en la evidencia de cierre):

```sh
uv pip compile pyproject.toml requirements/build.in --extra dev --extra excel-legacy --extra excel-native --extra outlook --python-version 3.12 --python-platform windows --generate-hashes --no-annotate --no-header --index-url https://pypi.org/simple -o requirements/windows-py312.lock
```

Repetir para 3.13 y 3.14. Linux usa `--python-platform linux` y omite los extras
COM. Revisar el diff y ejecutar regresión y CI antes de aprobar una actualización.
Los rangos de `pyproject.toml` siguen describiendo compatibilidad; los locks
identifican el conjunto probado de construcción. Esto controla dependencias,
sin prometer identidad binaria frente a timestamps u otros factores del build.
