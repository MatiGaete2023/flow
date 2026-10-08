"""Escritura de productos nuevos sin sobrescribir ni dejar resultados truncados."""
from pathlib import Path
import shutil
import tempfile


def write_new_file(destination, writer):
    target = Path(destination).expanduser().resolve()
    if target.exists():
        raise ValueError("El archivo ya existe; elige un destino nuevo.")
    target.parent.mkdir(parents=True, exist_ok=True)
    handle = tempfile.NamedTemporaryFile(prefix="nurus-producto-", suffix=target.suffix,
                                         dir=target.parent, delete=False)
    temporary = Path(handle.name)
    handle.close()
    created = False
    try:
        writer(temporary)
        if not temporary.is_file() or temporary.stat().st_size == 0:
            raise ValueError("No se produjo un archivo completo.")
        # La apertura exclusiva protege también frente a un destino creado después
        # de la primera comprobación. Solo se publica después de generar el producto.
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


def excel_text(value):
    """Protege texto externo en salidas nuevas sin alterar fórmulas de origen."""
    if isinstance(value, str) and value.lstrip().startswith(("=", "+", "-", "@")):
        return "'" + value
    return value
