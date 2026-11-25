"""E-S2 · Oráculo de referencias locales, conjuntos y paquetes con bitácoras reutilizadas."""

import base64
import hashlib
import io
import json
import re
import shutil
import sys
import tempfile
import zipfile
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(ROOT / 'app' / 'qa'), str(ROOT / 'descargador'), str(ROOT / 'src'), str(ROOT / 'tests')]

import generar_lote  # fija el reloj y normaliza Path.write_text a LF
from generar_excel import reconstruir
from test_resultados import fixture
from resultados import load_lot, read_json
from flujo_csmp import FLOW_DONE
from archivo_local import kind, load_sources
import flujo_csmp  # noqa: E402

# Python trata «\\» como separador solo en Windows: en Linux «sub\\fase» es un nombre literal y la fase no existe
# (y el error trae la ruta temporal). Para que el oráculo sea el mismo en cualquier sistema se reproduce la conducta de
# Windows durante la generación; la referencia no se modifica y la entrada del caso conserva «sub\\fase».
_source_folder = flujo_csmp.source_folder


def _source_folder_portable(root, name):
    return _source_folder(root, name.replace('\\', '/') if isinstance(name, str) else name)


flujo_csmp.source_folder = _source_folder_portable
from test_reutilizacion_local import bound_lot, diaries
from export_csmp_capture import export_capture
from nurus.capture import read_capture

from openpyxl import Workbook

_workbook_init = Workbook.__init__
_workbook_save = Workbook.save


def _fixed_workbook_init(self, *args, **kwargs):
    _workbook_init(self, *args, **kwargs)
    self.properties.creator = 'Flow Python'
    self.properties.lastModifiedBy = 'Flow Python'
    self.properties.created = datetime(2026, 10, 8, 12, 0, 0)
    self.properties.modified = datetime(2026, 10, 8, 12, 0, 0)


Workbook.__init__ = _fixed_workbook_init


def _fixed_workbook_save(self, *args, **kwargs):
    self.properties.created = datetime(2026, 10, 8, 12, 0, 0)
    self.properties.modified = datetime(2026, 10, 8, 12, 0, 0)
    return _workbook_save(self, *args, **kwargs)


Workbook.save = _fixed_workbook_save


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, sort_keys=True), encoding='utf-8')


def files(root):
    return {p.relative_to(root).as_posix(): base64.b64encode(p.read_bytes().replace(b'\r\n', b'\n') if p.suffix == '.json' else p.read_bytes()).decode('ascii')
            for p in sorted(root.rglob('*')) if p.is_file()}


def make_lot(root, bound=False):
    source = root.parent / (root.name + '-generated')
    if bound:
        lot = bound_lot(source).folder
    else:
        lot = fixture(source)
    # openpyxl fija propiedades, pero usa la hora local en el ZIP interno del XLSX.
    for book in lot.glob('*.xls*'):
        original = book.read_bytes()
        output = io.BytesIO()
        with zipfile.ZipFile(io.BytesIO(original)) as source_zip, zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as target_zip:
            for info in source_zip.infolist():
                fixed = zipfile.ZipInfo(info.filename, (2020, 1, 1, 0, 0, 0))
                # XLSX payloads must be byte-identical across Python/zlib versions.
                # Stored ZIP entries keep the generated oracle independent of the
                # runner's deflate implementation.
                fixed.compress_type = zipfile.ZIP_STORED
                fixed.create_system = 3
                fixed.external_attr = 0o600 << 16
                content = source_zip.read(info.filename)
                if info.filename == 'docProps/core.xml':
                    content = re.sub(rb'(<dcterms:modified\b[^>]*>).*?(</dcterms:modified>)', rb'\g<1>2026-10-08T12:00:00Z\g<2>', content)
                target_zip.writestr(fixed, content)
        book.write_bytes(output.getvalue())
        verification = lot / 'verificacion 001.json'
        manifest = json.loads(verification.read_text(encoding='utf-8'))
        manifest['archivos'][0]['sha256'] = hashlib.sha256(book.read_bytes()).hexdigest()
        manifest['archivos'][0]['bytes'] = book.stat().st_size
        write_json(verification, manifest)
    encoded = files(lot)
    reconstruir(root, encoded)
    shutil.rmtree(source)


def case(base, name, folder='fuente'):
    root = base / name
    (root / folder).mkdir(parents=True, exist_ok=True)
    return root, root / folder


def add_flow(root, folder, state=FLOW_DONE, phases=None, also_summary=False, also_diaries=False):
    if phases is not None:
        write_json(folder / 'flujo.json', {'estado': state, 'fases': phases})
    if also_summary:
        write_json(folder / 'resumen.json', {'estado': 'VALIDADA', 'consultas': []})
    if also_diaries:
        write_json(folder / 'bitacoras.json', {'estado': 'COMPLETA', 'registros': []})


def run(out):
    with tempfile.TemporaryDirectory(prefix='archivo-local-') as temp:
        base = Path(temp).resolve()
        cases = []
        def record(name, root, folder, load=True, packages=None, diaries_paths=(), partial=False):
            item = {'caso': name, 'archivos': files(root), 'parcial': partial}
            try:
                _, label = kind(folder)
                item['tipo'] = label
            except Exception as exc:
                item['tipo_error'] = str(exc)
            try:
                if (folder / 'flujo.json').is_file():
                    from flujo_csmp import source_folder
                    data = read_json(folder / 'flujo.json')
                    state = data.get('estado')
                    if state not in (FLOW_DONE, 'VALIDADA', 'COMPLETA') and not partial:
                        raise ValueError('El conjunto está incompleto. Permite resultados incompletos para conservar su alcance explícito.')
                    phases = [source_folder(folder, fase['carpeta']) for fase in data.get('fases', {}).values() if fase.get('carpeta')]
                    item['fases'] = [p.relative_to(root).as_posix() for p in phases]
                elif 'fases_input' in item:
                    pass
            except Exception as exc:
                item['fases_error'] = str(exc)
            if name == 'carpeta_vacia':
                try:
                    from flujo_csmp import source_folder
                    source_folder(folder, '')
                except Exception as exc:
                    item['direct_error'] = str(exc)
            if load:
                try:
                    lots = load_sources([folder], partial=partial)
                    item['fuentes'] = [lot.folder.relative_to(root).as_posix() for lot in lots]
                except Exception as exc:
                    item['fuentes_error'] = str(exc)
            if packages is not None:
                lots = load_sources([folder])
                target = root / 'captura.zip'
                capture = read_capture(export_capture(lots, target, diaries=list(diaries_paths), match_diaries=True))
                item['paquete'] = {name: list(capture.rows(name)) for name in capture.tables}
                item['incidencias'] = list(capture.rows('incidencias'))
            cases.append(item)

        # Prioridad de tipo y ausencia de control.
        root, folder = case(base, 'prioridad_bitacoras')
        add_flow(root, folder, also_summary=True, also_diaries=True)
        record('prioridad_bitacoras', root, folder)
        root, folder = case(base, 'prioridad_conjunto')
        add_flow(root, folder, phases={}, also_summary=True)
        record('prioridad_conjunto', root, folder, load=False)
        root, folder = case(base, 'tipo_descarga')
        make_lot(folder)
        record('tipo_descarga', root, folder)
        root, folder = case(base, 'sin_control')
        record('sin_control', root, folder, load=False)
        root, folder = case(base, 'flujo_json_invalido')
        (folder / 'flujo.json').write_bytes(b'{\n')
        record('flujo_json_invalido', root, folder, load=False)

        for state in (FLOW_DONE, 'VALIDADA', 'COMPLETA'):
            name = 'estado_' + state.lower()
            root, folder = case(base, name)
            make_lot(folder / 'fase')
            add_flow(root, folder, state, {'principal': {'carpeta': 'fase'}})
            record(name, root, folder)
        root, folder = case(base, 'incompleto_sin_parcial')
        make_lot(folder / 'fase')
        add_flow(root, folder, 'EN_CURSO', {'principal': {'carpeta': 'fase'}})
        record('incompleto_sin_parcial', root, folder)
        root, folder = case(base, 'incompleto_con_parcial')
        make_lot(folder / 'fase')
        add_flow(root, folder, 'EN_CURSO', {'principal': {'carpeta': 'fase'}})
        record('incompleto_con_parcial', root, folder, partial=True)
        root, folder = case(base, 'fase_sin_carpeta')
        add_flow(root, folder, FLOW_DONE, {'omitida': {'estado': 'PENDIENTE'}})
        record('fase_sin_carpeta', root, folder)
        for name, phase_name in [('carpeta_vacia', ''), ('carpeta_raiz', '.'), ('carpeta_parent', '../fuera'), ('carpeta_absoluta', '/fuera')]:
            root, folder = case(base, name)
            add_flow(root, folder, FLOW_DONE, {'fase': {'carpeta': phase_name}})
            record(name, root, folder, load=False)
        root, folder = case(base, 'separadores_ambos')
        make_lot(folder / 'sub' / 'fase')
        add_flow(root, folder, FLOW_DONE, {'fase': {'carpeta': 'sub\\fase'}})
        record('separadores_ambos', root, folder)
        root, folder = case(base, 'deduplicacion_orden')
        outer = folder / 'outer'
        inner = outer / 'inner'
        make_lot(inner / 'fase')
        make_lot(outer / 'extra')
        add_flow(root, outer, FLOW_DONE, {'primero': {'carpeta': 'inner/fase'}, 'segundo': {'carpeta': 'extra'}})
        add_flow(root, inner, FLOW_DONE, {'misma': {'carpeta': 'fase'}})
        item = {'caso': 'deduplicacion_orden', 'archivos': files(root), 'tipo': 'Conjunto', 'fases': ['fuente/outer/inner/fase', 'fuente/outer/extra'],
                'seleccion': ['fuente/outer', 'fuente/outer/inner', 'fuente/outer/inner/fase'], 'parcial': False}
        try:
            item['fuentes'] = [lot.folder.relative_to(root).as_posix() for lot in load_sources([outer, inner, inner / 'fase'])]
        except Exception as exc:
            item['fuentes_error'] = str(exc)
        cases.append(item)
        root, folder = case(base, 'solo_bitacoras')
        write_json(folder / 'bitacoras.json', {'estado': 'COMPLETA', 'registros': []})
        record('solo_bitacoras', root, folder)

        # Tres exports de conjunto con correspondencia completa, parcial y nula.
        for name, diary_mode in [('paquete_coincidencia_total', 'all'), ('paquete_coincidencia_parcial', 'partial'), ('paquete_sin_coincidencia', 'none')]:
            root, folder = case(base, name)
            make_lot(folder / 'fase', bound=True)
            add_flow(root, folder, FLOW_DONE, {'principal': {'carpeta': 'fase'}})
            diary_dir = root / 'controles'
            control = diaries(diary_dir, 2)
            data = json.loads(control.read_text(encoding='utf-8'))
            if diary_mode == 'partial':
                data['registros'] = data['registros'][:1]
            elif diary_mode == 'none':
                for i, row in enumerate(data['registros']):
                    row['causa_id'] = str(900 + i)
                    row['ingreso_id'] = str(800 + i)
            write_json(control, data)
            record(name, root, folder, packages=True, diaries_paths=[control])

        # Include additional precise path errors, including a string with only whitespace and wrong JSON types.
        root, folder = case(base, 'carpeta_no_texto')
        add_flow(root, folder, FLOW_DONE, {'fase': {'carpeta': 7}})
        record('carpeta_no_texto', root, folder, load=False)
        data = {'version': 1, 'casos': cases}
        out.write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')


if __name__ == '__main__':
    run(Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / 'app' / 'tests' / 'datos' / 'archivo_local.json')
