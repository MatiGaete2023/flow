"""Comprueba las copias sintéticas de Rust con el contrato Python sin modificar el oráculo."""
import subprocess
import sys
import tempfile
from pathlib import Path
RAIZ = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(RAIZ / 'src'))
from nurus.capture.contract import read_capture
with tempfile.TemporaryDirectory(prefix='flow-contrato-ficticio-') as carpeta:
    subprocess.run(['cargo', 'run', '--locked', '-p', 'flow-anonimo', '--example', 'probar_paquetes', '--', carpeta], cwd=RAIZ/'rust', check=True)
    archivos = list(Path(carpeta).glob('*.zip'))
    assert len(archivos) == 3
    for archivo in archivos:
        read_capture(archivo)
print('Tres copias ficticias de Rust: contrato Python PASA.')
