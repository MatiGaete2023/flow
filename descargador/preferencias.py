"""Selecciones locales sin filtros privados ni autenticación."""

import calendar
from dataclasses import asdict
from datetime import date, timedelta
import json
import os
from pathlib import Path

from lotes import Batch, Dates
from motor import PocError


def data_directory():
    root = Path(os.environ.get('LOCALAPPDATA', Path.home() / 'AppData' / 'Local')) / 'SITFA_Descargador_Integral'
    root.mkdir(parents=True, exist_ok=True)
    return root


def date_preset(name, today=None):
    today = today or date.today()
    if name == 'Últimos 30 días':
        return today - timedelta(days=29), today
    first = today.replace(day=1)
    if name == 'Mes anterior':
        last = first - timedelta(days=1)
        return last.replace(day=1), last
    if name == 'Este mes':
        return first, today.replace(day=calendar.monthrange(today.year, today.month)[1])
    raise PocError('Período rápido desconocido.')


PAUSA_MINIMA_S = 0.5
PAUSA_DEFECTO_S = 1.0
PAUSA_MAXIMA_S = 10.0


def clamp_pause(value):
    """Pausa entre consultas a SITFA: nunca menos de 0,5 s."""
    return min(PAUSA_MAXIMA_S, max(PAUSA_MINIMA_S, float(value)))


class Preferences:
    def __init__(self, path=None, memory=False):
        self.path = Path(path) if path else None if memory else data_directory() / 'preferencias.json'
        self.data = {
            'favoritos': {},
            'destino': '',
            'tribunales': None,
            'aviso_sonoro': False,
            'csmp': '',
            'paquete_consolidado': True,
            'tema': 'Sistema',
            'historial_lotes': [],
            'historial_bitacoras': [],
            'pausa_consultas': PAUSA_DEFECTO_S,
        }
        if self.path and self.path.is_file():
            try:
                value = json.loads(self.path.read_text(encoding='utf-8'))
                if isinstance(value, dict):
                    for key in self.data:
                        if key in value and isinstance(value[key], type(self.data[key])):
                            self.data[key] = value[key]
                    if isinstance(value.get('tribunales'), list):
                        self.data['tribunales'] = value['tribunales']
                    if isinstance(value.get('pausa_consultas'), (int, float)) and not isinstance(
                        value['pausa_consultas'], bool
                    ):
                        self.data['pausa_consultas'] = clamp_pause(value['pausa_consultas'])
            except (OSError, ValueError):
                pass

    def save(self):
        if not self.path:
            return
        self.path.parent.mkdir(parents=True, exist_ok=True)
        temp = self.path.with_suffix('.tmp')
        temp.write_text(json.dumps(self.data, indent=2, ensure_ascii=False), encoding='utf-8')
        temp.replace(self.path)

    def favorite(self, name, batch, *, calendar_offset=0):
        if not name.strip():
            raise PocError('Escribe un nombre para el favorito.')
        if batch.keep_filters:
            raise PocError('Los favoritos no guardan filtros privados preparados en SITFA.')
        value = asdict(batch)
        value['dates'] = None
        value['keep_filters'] = False
        if calendar_offset not in (0, 1):
            raise PocError('Período relativo inválido.')
        value['calendar_offset'] = calendar_offset
        value['date_policy'] = 'Últimos 30 días' if batch.dates else ''
        self.data['favoritos'][name.strip()[:80]] = value
        self.save()

    def restore(self, name, catalog, today=None):
        value = dict(self.data['favoritos'][name])
        missing = [k for k in value['tribunals'] if k not in catalog['tribunales']]
        value['tribunals'] = tuple(k for k in value['tribunals'] if k in catalog['tribunales'])
        for field in ('modalities', 'tabs'):
            value[field] = tuple(value[field])
        offset = value.pop('calendar_offset', 0)
        policy = value.pop('date_policy', '')
        if value.get('screen', '').startswith('calendario_'):
            now = today or date.today()
            year, month = divmod(now.year * 12 + now.month - 1 + offset, 12)
            value['month'] = str(month + 1)
            value['year'] = str(year)
            if value['month'] not in catalog.get('meses', {}) or value['year'] not in catalog.get('anios', {}):
                raise PocError('El período relativo del perfil no está disponible en SITFA.')
        if policy or value.get('screen') == 'carga':
            start, end = date_preset('Últimos 30 días', today)
            value['dates'] = Dates(start.strftime('%d/%m/%Y'), end.strftime('%d/%m/%Y'))
        batch = Batch(**value)
        batch.plan(catalog)
        return batch, missing


def pending_plan(folder):
    from resultados import read_json

    summary = read_json(Path(folder) / 'resumen.json')
    if not summary.get('plan'):
        raise PocError('El lote antiguo no tiene plan estructurado.')
    if summary.get('otros_filtros') == 'formulario':
        raise PocError('Prepara otra vez los filtros privados; no se guardan para recuperación.')
    done = [
        {k: v for k, v in q.get('seleccion', {}).items() if k not in ('fecha_descarga',)} for q in summary['consultas']
    ]
    return [s for s in summary['plan'] if {k: v for k, v in s.items() if k not in ('fecha_descarga',)} not in done]


RETENCION_DIAS = 60
PRESERVABLES = frozenset({'Bitacoras_RUS.xlsx', 'diagnostico_bitacoras.json'})


def onedrive_warning(path):
    return (
        'Advertencia: esta carpeta está en OneDrive. Los lotes pueden sincronizarse fuera de este equipo.'
        if 'onedrive' in str(path).casefold()
        else ''
    )


PACKAGE_PREFIX = 'Paquete CSMP '


def _package_controls(folder):
    """Paquete de «Consultar y crear paquete» (revisión externa H3): controles bajo respaldo/ y el ZIP.

    Solo cuenta si respaldo/proceso.json existe; ignora enlaces en cualquier tramo de la ruta.
    """
    backup = folder / 'respaldo'
    process = backup / 'proceso.json'
    if (
        not folder.name.startswith(PACKAGE_PREFIX)  # E3 WP-94 C4: una carpeta ajena no se toma por paquete
        or backup.is_symlink()
        or not backup.is_dir()
        or process.is_symlink()
        or not process.is_file()
    ):
        return []
    controls = [process]
    for name in ('resumen.json', 'bitacoras.json', 'flujo.json'):
        for path in backup.glob('*/' + name):
            if not path.is_symlink() and not path.parent.is_symlink() and path.is_file():
                controls.append(path)
    controls += [path for path in folder.glob('*.zip') if path.is_file() and not path.is_symlink()]
    return controls


def old_batches(root, *, max_days=RETENCION_DIAS, protected=()):
    """Lista controles antiguos directos: ignora enlaces, carpetas ajenas y lotes vigentes."""
    import time

    root = Path(root)
    if not root.is_dir() or root.is_symlink():
        return []
    excluded = {Path(path).resolve() for path in protected if path}
    limit = time.time() - max_days * 86400
    result = []
    for folder in sorted(root.iterdir()):
        if (
            folder.is_symlink()
            or not folder.is_dir()
            or any(path.is_relative_to(folder.resolve()) for path in excluded)
        ):
            continue
        controls = [
            folder / name
            for name in ('resumen.json', 'bitacoras.json', 'flujo.json')
            if (folder / name).is_file() and not (folder / name).is_symlink()
        ] + _package_controls(folder)
        if not controls:
            continue
        modified = max(path.stat().st_mtime for path in controls)
        if folder.name.startswith(PACKAGE_PREFIX) and _package_controls(folder):
            # E3 WP-94 C4: en un paquete, cualquier archivo reciente (p. ej. del usuario) lo mantiene vigente.
            files = [p for p in folder.rglob('*') if p.is_file() and not p.is_symlink()]
            modified = max([modified, *(p.stat().st_mtime for p in files)])
        if modified >= limit:
            continue
        size = sum(path.stat().st_size for path in folder.rglob('*') if path.is_file() and not path.is_symlink())
        result.append({'folder': folder, 'bytes': size, 'modified': modified})
    return result


def manage_old_batches(
    root, selected, *, confirmed=False, preserve=False, archive=None, max_days=RETENCION_DIAS, protected=()
):
    """Borra o mueve únicamente lotes elegibles después de confirmar y volver a comprobarlos."""
    import shutil

    if not confirmed:
        return []
    root = Path(root).resolve()
    available = {
        item['folder'].resolve(): item['folder'] for item in old_batches(root, max_days=max_days, protected=protected)
    }
    selected = list(dict.fromkeys(Path(path).resolve() for path in selected))
    if any(path not in available for path in selected):
        raise PocError(
            'La selección cambió o contiene lotes vigentes, recientes o fuera de la carpeta. Actualiza la lista.'
        )
    target = Path(archive).resolve() if archive else None
    if target:
        if target == root or any(target == path or target.is_relative_to(path) for path in selected):
            raise PocError('La carpeta de archivo debe estar fuera de los lotes seleccionados.')
        if any((target / path.name).exists() for path in selected):
            raise PocError('La carpeta de archivo ya contiene un lote con el mismo nombre.')
    changed = []
    for path in selected:
        if target:
            target.mkdir(parents=True, exist_ok=True)
            shutil.move(str(path), str(target / path.name))
        elif preserve:
            package = bool(_package_controls(path))
            for child in path.iterdir():
                if child.name in PRESERVABLES and child.is_file() and not child.is_symlink():
                    continue
                if package and child.suffix.lower() == '.zip' and child.is_file() and not child.is_symlink():
                    continue  # E3 WP-94 C2: en un paquete, las bitácoras viven en el ZIP; «conservar» lo conserva
                if child.is_dir() and not child.is_symlink():
                    shutil.rmtree(child)
                else:
                    child.unlink()
            if not any(path.iterdir()):
                path.rmdir()
        else:
            shutil.rmtree(path)
        changed.append(str(path))
    return changed
