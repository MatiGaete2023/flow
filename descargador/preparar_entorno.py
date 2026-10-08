"""Comprobación e instalación ligada al contenido de requirements.txt."""
import hashlib
from pathlib import Path
import subprocess
import sys


def main():
    if sys.version_info<(3,10):raise RuntimeError('Se requiere Python 3.10 o posterior.')
    try:import tkinter
    except ImportError:raise RuntimeError('Este Python no incluye Tkinter. Reinstala Python con soporte Tcl/Tk.') from None
    requirements=Path(__file__).resolve().parent/'requirements.txt'
    marker=Path(sys.prefix)/'dependencias.sha256'
    expected=hashlib.sha256(requirements.read_bytes()).hexdigest()
    if not marker.is_file() or marker.read_text().strip()!=expected:
        print('Instalando dependencias del descargador...')
        subprocess.run([sys.executable,'-m','pip','install','-r',str(requirements)],check=True)
        subprocess.run([sys.executable,'-m','pip','check'],check=True)
        marker.write_text(expected,encoding='ascii')


if __name__=='__main__':
    try:main()
    except (RuntimeError,OSError,subprocess.CalledProcessError) as exc:
        print('E_INSTALACION:',exc);raise SystemExit(1)
