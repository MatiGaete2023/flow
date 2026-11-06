# zip2exe

Convierte el ZIP de un repositorio Python (ej. GitHub → *Code → Download ZIP*) en un `.exe` usando PyInstaller dentro de un venv aislado.

## Requisitos
- Windows + Python ≥ 3.11 en PATH (`py` o `python`).
- Internet para `pip` (instala PyInstaller + dependencias del repo).
- PyInstaller **no** compila cruzado: el `.exe` solo se genera corriendo esto en Windows.

## Uso
- Arrastrar el ZIP sobre `zip2exe.bat`, o:
```
zip2exe.bat C:\descargas\mi-repo-main.zip
python zip2exe.py mi-repo-main.zip --entry src\app.py --windowed --name MiApp
```
Salida: carpeta `dist\` junto al ZIP (o `--out`).

## Autodetección
- Entry point (en orden): `main.py`, `app.py`, `run.py`, `__main__.py`, `cli.py`, `gui.py`, `*.pyw` → `[project.scripts]` de `pyproject.toml` → `paquete/__main__.py` → único `.py` raíz con `if __name__ == "__main__"`. Ambigüedad → error pidiendo `--entry`.
- Dependencias: `requirements.txt` → si no, `pip install .` (pyproject/setup.py).
- Modo ventana: entry `.pyw` o que importe tkinter/PyQt/PySide/wx/customtkinter/kivy/flet/pygame.
- Datos: carpetas `assets data resources res static templates images img icons config`. En el código, leerlas con `getattr(sys, "_MEIPASS", os.path.dirname(__file__))` como base.
- Ícono: primer `.ico` en raíz o `assets/ resources/ icons/ img/`.
- Si el repo trae `*.spec`, se usa tal cual (`--ignore-spec` para evitarlo).

## Opciones
| Flag | Efecto |
|---|---|
| `--entry RUTA` | script principal |
| `--name N` | nombre del exe |
| `--onedir` | carpeta exe+DLLs (arranca más rápido, menos falsos positivos AV) |
| `--windowed` / `--console` | forzar modo |
| `--icon X.ico` | ícono |
| `--add-data RUTA` | incluir datos (repetible; reemplaza autodetección) |
| `--hidden-import MOD` | módulos no detectados (repetible) |
| `--keep-work` | conservar temporal para depurar |

## Seguridad
El build ejecuta `setup.py`/código de build del repo vía `pip`. Usar solo con repos confiables. Extracción protegida contra zip-slip.
