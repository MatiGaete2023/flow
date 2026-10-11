"""Último paquete CSMP_CAPTURE del Descargador. Solo lectura de archivos locales: no inicia
procesos, no abre Chrome ni el Descargador (restricción 7)."""

import json
import os
import zipfile
from pathlib import Path

DOWNLOADER_DIR = 'SITFA_Descargador_Integral'
MAX_DEPTH = 2


def downloader_root(environ=None):
    environ = os.environ if environ is None else environ
    return Path(environ.get('LOCALAPPDATA') or Path.home() / 'AppData' / 'Local') / DOWNLOADER_DIR


def search_folders(environ=None):
    """Carpeta de descargas por defecto y la carpeta de destino guardada por el Descargador."""
    root = downloader_root(environ)
    folders = [root / 'descargas']
    try:
        preferences = json.loads((root / 'preferencias.json').read_text(encoding='utf-8'))
        destination = preferences.get('destino') if isinstance(preferences, dict) else None
        if isinstance(destination, str) and destination.strip():
            folders.append(Path(destination))
    except (OSError, ValueError):
        pass
    result = []
    for folder in folders:
        if folder.is_dir() and not folder.is_symlink() and folder.resolve() not in [f.resolve() for f in result]:
            result.append(folder)
    return result


def _zips(folder, depth=MAX_DEPTH):
    pending = [(folder, 0)]
    while pending:
        current, level = pending.pop()
        try:
            entries = list(current.iterdir())
        except OSError:
            continue
        for entry in entries:
            if entry.is_symlink():
                continue
            if entry.is_file() and entry.suffix.lower() == '.zip':
                yield entry
            elif entry.is_dir() and level < depth:
                pending.append((entry, level + 1))


def _declares_capture(path):
    try:
        with zipfile.ZipFile(path) as archive:
            info = archive.getinfo('manifest.json')
            if info.file_size > 4 * 1024 * 1024:
                return False
            return json.loads(archive.read(info).decode('utf-8-sig')).get('schema_id') == 'CSMP_CAPTURE'
    except (OSError, KeyError, ValueError, zipfile.BadZipFile, AttributeError):
        return False


def latest_capture(folders=None):
    """El ZIP más reciente que declara CSMP_CAPTURE y además pasa la validación completa."""
    from nurus.capture import read_capture

    candidates = []
    for folder in search_folders() if folders is None else folders:
        for path in _zips(Path(folder)):
            try:
                candidates.append((path.stat().st_mtime, path))
            except OSError:
                continue
    for _, path in sorted(candidates, key=lambda item: item[0], reverse=True):
        if not _declares_capture(path):
            continue
        try:
            read_capture(path)
        except Exception:  # paquete dañado o ajeno: se ignora y se prueba el siguiente
            continue
        return path
    return None
