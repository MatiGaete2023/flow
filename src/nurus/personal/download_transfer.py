"""Solicitudes locales de descarga, conservadas aunque se cierre CSMP."""
from datetime import datetime
from pathlib import Path
import json
import re
import sys
from uuid import uuid4

from .config import atomic_json

STATES = {'PENDIENTE', 'ERROR_INICIO', 'CERRADA_SIN_RESULTADO', 'INCORPORADA'}


class Transfer:
    def __init__(self, folder, identifier, data):
        self.folder = Path(folder).resolve()
        self.identifier = identifier
        self.data = data

    @property
    def path(self):
        return self.folder / (self.identifier + '.solicitud.json')

    @property
    def reply(self):
        return self.folder / (self.identifier + '.json')

    @property
    def recovery_folder(self):
        path = (self.folder / 'sesiones' / self.identifier).resolve()
        text = str(path)
        # Los archivos de sesión llevan la huella completa como nombre. Usar
        # rutas extendidas en este archivo interno evita el límite Windows de
        # 260 caracteres sin cambiar las rutas que recibe el descargador.
        if sys.platform == 'win32' and not text.startswith('\\\\?\\'):
            text = '\\\\?\\UNC\\' + text[2:] if text.startswith('\\\\') else '\\\\?\\' + text
            return Path(text)
        return path

    def update(self, state, **facts):
        if state not in STATES:
            raise ValueError('Estado de descarga desconocido.')
        updated = {**self.data, **facts, 'state': state}
        atomic_json(self.path, updated)
        self.data = updated

    @classmethod
    def create(cls, folder, mode, destination):
        if mode not in ('ESPERA', 'CUMPLIMIENTO'):
            raise ValueError('La descarga conjunta requiere Espera o Cumplimiento.')
        item = cls(folder, uuid4().hex, {
            'version': 1, 'mode': mode, 'destination': str(Path(destination).resolve()),
            'created_at': datetime.now().astimezone().isoformat(), 'state': 'PENDIENTE',
        })
        item.update('PENDIENTE')
        return item

    @classmethod
    def load(cls, folder, path):
        folder = Path(folder).resolve()
        path = Path(path).resolve()
        suffix = '.solicitud.json'
        identifier = path.name[:-len(suffix)] if path.name.endswith(suffix) else ''
        if path.parent != folder or not re.fullmatch(r'[0-9a-f]{32}', identifier):
            raise ValueError('La solicitud no pertenece al intercambio de este prototipo.')
        if path.stat().st_size > 16_000:
            raise ValueError('La solicitud excede el tamaño permitido.')
        data = json.loads(path.read_text(encoding='utf-8'))
        if not isinstance(data, dict) or data.get('version') != 1:
            raise ValueError('Formato de solicitud no reconocido.')
        if data.get('mode') not in ('ESPERA', 'CUMPLIMIENTO') or data.get('state') not in STATES:
            raise ValueError('La solicitud contiene un modo o estado desconocido.')
        if not isinstance(data.get('destination'), str) or not data['destination']:
            raise ValueError('La solicitud no conserva su carpeta de salida.')
        try:
            datetime.fromisoformat(data['created_at'])
        except (ValueError, TypeError, KeyError):
            raise ValueError('La solicitud no conserva su fecha de creación.') from None
        if data['state'] == 'INCORPORADA' and not re.fullmatch(r'[0-9a-f]{64}', str(data.get('source_hash', ''))):
            raise ValueError('La solicitud incorporada no conserva la huella del libro.')
        return cls(folder, identifier, data)


def transfers(folder):
    """Cada solicitud se elige explícitamente; nunca se toma la última por posición."""
    folder = Path(folder)
    paths = sorted(folder.glob('*.solicitud.json'))
    if len(paths) > 5_000:
        raise ValueError('Demasiadas solicitudes: conserva las anteriores fuera del intercambio activo.')
    return [Transfer.load(folder, path) for path in paths]


def archive_current(work, configuration_folder):
    """Conservar también las ediciones posteriores a la devolución inicial."""
    identifier = getattr(work, 'download_transfer_id', '')
    if not identifier:
        return
    if not isinstance(identifier, str) or not re.fullmatch(r'[0-9a-f]{32}', identifier):
        raise ValueError('El trabajo contiene una solicitud de descarga inválida.')
    folder = Path(configuration_folder) / 'intercambio'
    transfer = Transfer.load(folder, folder / (identifier + '.solicitud.json'))
    if transfer.data.get('source_hash') != work.source_hash or transfer.data['mode'] != work.mode:
        raise ValueError('El trabajo ya no corresponde a su devolución de descarga.')
    work.save(transfer.recovery_folder / 'incorporada')


def recovery_work(transfer, current=None):
    """Prefiere las ediciones actuales o guardadas; no vuelve a analizar el origen."""
    from .work import Work
    snapshot = transfer.recovery_folder / 'incorporada'
    interrupted = (snapshot / 'trabajo.json').is_file()
    if transfer.data['state'] != 'INCORPORADA' and not interrupted:
        return None
    expected = transfer.data.get('source_hash', '')
    if not re.fullmatch(r'[0-9a-f]{64}', expected):
        raise ValueError('La recuperación no conserva la huella de la descarga.')
    mode = transfer.data['mode']
    def matches(work):
        return work is not None and work.source_hash == expected and work.mode == mode
    # El archivo propio demuestra que el análisis terminó incluso si falló la
    # actualización del estado después de guardar esa sesión.
    archived = Work.load(snapshot)
    if not matches(archived):
        raise ValueError('La sesión de devolución no corresponde a esta descarga.')
    if transfer.data['state'] != 'INCORPORADA':
        transfer.update('INCORPORADA')
    if matches(current):
        return current
    latest = transfer.folder.parent / 'sesion'
    if (latest / 'trabajo.json').is_file():
        work = Work.load(latest)
        if matches(work):
            return work
    return archived
