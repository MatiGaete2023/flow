"""Transferencia exclusivamente ficticia. No acepta registros reales ni concede permisos."""
import argparse
from hashlib import sha256
import os
from pathlib import Path
import stat
from tempfile import NamedTemporaryFile

from .contracts import ContractError
from .simulation import demo_operations
from .transfer import PreviewBatch, MAX_BYTES

SUFFIX = '.flow-demo.json'
DEMO_SOURCE = sha256(b'Flow-demo-transfer-v1').hexdigest()


def make_demo_batch(count=8):
    return PreviewBatch(demo_operations(count), DEMO_SOURCE, 0)


def require_demo_batch(batch):
    if type(batch) is not PreviewBatch or batch != make_demo_batch(len(batch.operations)):
        raise ContractError('SOLO_LOTE_FICTICIO_GENERADO')
    return batch


def export_demo(path, count=8):
    """Crea archivo nuevo sin sobrescribir; no recibe texto ni identificadores externos."""
    target = Path(path)
    if not target.name.endswith(SUFFIX):
        raise ContractError('EXTENSION_FICTICIA_REQUERIDA')
    data = make_demo_batch(count).encode()
    temp = None
    try:
        with NamedTemporaryFile(dir=target.parent, prefix='.flow-demo-', delete=False) as stream:
            temp = Path(stream.name)
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        # Publicación exclusiva, sin sobrescritura ni fallback inseguro.
        os.link(temp, target)
    finally:
        if temp is not None:
            temp.unlink(missing_ok=True)
    return target


def import_demo(path):
    """Solo un archivo regular local; los bytes se validan antes de inicializar el diario."""
    source = Path(path)
    if not source.name.endswith(SUFFIX) or source.is_symlink():
        raise ContractError('ARCHIVO_FICTICIO_INVALIDO')
    if not stat.S_ISREG(source.stat().st_mode):
        raise ContractError('ARCHIVO_FICTICIO_INVALIDO')
    with source.open('rb') as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise ContractError('ARCHIVO_FICTICIO_INVALIDO')
        data = stream.read(MAX_BYTES+1)
    return require_demo_batch(PreviewBatch.decode(data))


def export_demo_dialog(parent):
    from tkinter import filedialog, simpledialog, messagebox
    count = simpledialog.askinteger('Lote ficticio', 'Cantidad de operaciones inventadas (1–400):',
                                   initialvalue=8, minvalue=1, maxvalue=400, parent=parent)
    if count is None:
        return
    path = filedialog.asksaveasfilename(parent=parent, title='Guardar lote exclusivamente ficticio',
                                        defaultextension=SUFFIX, filetypes=[('Lote ficticio', '*.flow-demo.json')],
                                        initialfile='lote.flow-demo.json')
    if not path:
        return
    export_demo(path, count)
    messagebox.showinfo('Lote ficticio creado', 'Abre el registrador con --demo-batch y este archivo. No contiene datos del Trabajo.', parent=parent)


def main():
    parser = argparse.ArgumentParser(description='Generar un lote de demostración. No lee datos de Flow.')
    parser.add_argument('output')
    parser.add_argument('--count', type=int, default=8)
    args = parser.parse_args()
    try:
        export_demo(args.output, args.count)
    except (OSError, ValueError) as exc:
        parser.exit(2, 'No se pudo crear un lote ficticio nuevo: ' + type(exc).__name__ + '\n')


if __name__ == '__main__':
    main()
