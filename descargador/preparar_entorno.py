"""Comprobación e instalación ligada a las dependencias fijadas con hash."""

import hashlib
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent


class FaltaLock(RuntimeError):
    """La versión de Python en uso no tiene lock con hashes (código de salida 3)."""


def lock_file():
    """Lock con hashes de la versión de Python en uso. Sin lock no se instala nada sin verificar."""
    platform = 'windows' if sys.platform == 'win32' else 'linux'
    folder = HERE.parent / 'requirements'
    lock = folder / f'{platform}-py{sys.version_info[0]}{sys.version_info[1]}.lock'
    if not folder.is_dir():
        # Copia sin la raíz del repositorio: cambiar de Python no lo arregla (código 1, no 3).
        raise RuntimeError(
            'Falta la carpeta requirements junto al descargador: usa el ejecutable entregado o '
            'una copia completa del repositorio.'
        )
    if not lock.is_file():
        raise FaltaLock(f'Falta {lock.name}: esta versión de Python no tiene lock; usa Python 3.12, 3.13 o 3.14.')
    return lock


def main():
    if sys.version_info < (3, 10):
        raise RuntimeError('Se requiere Python 3.10 o posterior.')
    if sys.version_info[:2] != (3, 12):
        print('Aviso: la versión probada es Python 3.12; se usa', sys.version.split()[0])
    try:
        import tkinter  # noqa: F401 (comprobación de disponibilidad)
    except ImportError:
        raise RuntimeError('Este Python no incluye Tkinter. Reinstala Python con soporte Tcl/Tk.') from None
    lock = lock_file()
    marker = Path(sys.prefix) / 'dependencias.sha256'
    expected = hashlib.sha256(lock.read_bytes()).hexdigest()
    if not marker.is_file() or marker.read_text().strip() != expected:
        print('Instalando dependencias del descargador...')
        subprocess.run([sys.executable, '-m', 'pip', 'install', '--require-hashes', '-r', str(lock)], check=True)
        subprocess.run([sys.executable, '-m', 'pip', 'check'], check=True)
        marker.write_text(expected, encoding='ascii')


if __name__ == '__main__':
    try:
        main()
    except FaltaLock as exc:
        print('E_INSTALACION:', exc)
        raise SystemExit(3)
    except (RuntimeError, OSError, subprocess.CalledProcessError) as exc:
        print('E_INSTALACION:', exc)
        raise SystemExit(1)
