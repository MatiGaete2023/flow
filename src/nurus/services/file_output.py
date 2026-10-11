"""Escritura de productos nuevos sin sobrescribir ni dejar resultados truncados."""

import os
from pathlib import Path
import shutil
import tempfile


def _umask():
    """Umask actual sin cambiarlo (os.umask lo modifica para todo el proceso, también otros hilos)."""
    try:
        for line in Path("/proc/self/status").read_text().splitlines():
            if line.startswith("Umask:"):
                return int(line.split()[1], 8)
    except OSError:
        pass
    return 0o022


def write_new_file(destination, writer):
    target = Path(destination).expanduser().resolve()
    if target.exists():
        raise ValueError("El archivo ya existe; elige un destino nuevo.")
    target.parent.mkdir(parents=True, exist_ok=True)
    handle = tempfile.NamedTemporaryFile(
        prefix="nurus-producto-", suffix=target.suffix, dir=target.parent, delete=False
    )
    temporary = Path(handle.name)
    handle.close()
    created = False
    try:
        writer(temporary)
        if not temporary.is_file() or temporary.stat().st_size == 0:
            raise ValueError("No se produjo un archivo completo.")
        # Publicación atómica: os.link crea el nombre final completo de una vez y falla si ya existe,
        # de modo que un corte a mitad nunca deja un producto truncado con el nombre final.
        if os.name == "posix":
            # El temporal nace 0600 y el enlace conserva el inodo: mismos permisos que open("xb").
            os.chmod(temporary, 0o666 & ~_umask())
        try:
            os.link(temporary, target)
            created = True
        except FileExistsError:
            # Destino creado por otro durante la generación: se conserva intacto.
            raise
        except OSError:
            # Sistemas de archivos sin enlaces duros (SMB/FAT): copia exclusiva como respaldo, no atómica.
            with target.open("xb") as output:
                created = True
                with temporary.open("rb") as source:
                    shutil.copyfileobj(source, output)
    except BaseException:
        if created:
            target.unlink(missing_ok=True)
        raise
    finally:
        temporary.unlink(missing_ok=True)
    return target
