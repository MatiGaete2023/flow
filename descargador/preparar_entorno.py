"""Comprobación e instalación ligada a las dependencias fijadas con hash."""
import hashlib
from pathlib import Path
import subprocess
import sys

HERE=Path(__file__).resolve().parent


def dependency_file():
    """Lock con hashes de la versión de Python en uso; requirements.txt solo si el lock no viaja con la copia."""
    platform='windows' if sys.platform=='win32' else 'linux'
    lock=HERE.parent/'requirements'/f'{platform}-py{sys.version_info[0]}{sys.version_info[1]}.lock'
    if lock.is_file():return lock,['--require-hashes','-r',str(lock)]
    print('Aviso: no se encontró',lock.name,'; se usan los rangos de requirements.txt (sin hashes).')
    requirements=HERE/'requirements.txt'
    return requirements,['-r',str(requirements)]


def main():
    if sys.version_info<(3,10):raise RuntimeError('Se requiere Python 3.10 o posterior.')
    if sys.version_info[:2]!=(3,12):print('Aviso: la versión probada es Python 3.12; se usa',sys.version.split()[0])
    try:import tkinter
    except ImportError:raise RuntimeError('Este Python no incluye Tkinter. Reinstala Python con soporte Tcl/Tk.') from None
    source,arguments=dependency_file()
    marker=Path(sys.prefix)/'dependencias.sha256'
    expected=hashlib.sha256(source.read_bytes()).hexdigest()
    if not marker.is_file() or marker.read_text().strip()!=expected:
        print('Instalando dependencias del descargador...')
        subprocess.run([sys.executable,'-m','pip','install',*arguments],check=True)
        subprocess.run([sys.executable,'-m','pip','check'],check=True)
        marker.write_text(expected,encoding='ascii')


if __name__=='__main__':
    try:main()
    except (RuntimeError,OSError,subprocess.CalledProcessError) as exc:
        print('E_INSTALACION:',exc);raise SystemExit(1)
