"""WD-1: calendario de informes por ingreso, del Descargador al Asistente; todos los datos son ficticios."""

import base64
from datetime import date
import hashlib
import json
from pathlib import Path
import threading

import lectura_bitacoras as lb
from export_csmp_capture import export_capture
from lotes import Batch
from nurus.capture import read_capture
from nurus.personal.capture_input import import_capture
from nurus.personal.config import defaults
from test_lectura_bitacoras import Bridge, CATALOG
from test_reutilizacion_local import bound_lot, diaries
from csmp_shared import parser


def calendar_page(causa, tribunal, ingreso, rows=(('10/11/2026', '', 'PENDIENTE'),)):
    body = ''.join(f'<tr><td>{a}</td><td>{b}</td><td>{c}</td></tr>' for a, b, c in rows)
    return (
        '<html><body><form name="InformesPpalForm" action="/SITFAWEB/InformesDAction.do">'
        f'<input type="hidden" name="COD_Tribunal" value="{tribunal}">'
        f'<input type="hidden" name="CRR_IdCausa" value="{causa}">'
        f'<input type="hidden" name="ID_Ingreso" value="{ingreso}">'
        '<table><tr><td>Plazo Remisión: 3 Meses</td><td>Plazo Intervención: 12 Meses</td></tr></table>'
        '<table><tr><th>Fecha Proyección</th><th>Fecha Informe</th><th>Estado</th></tr>'
        f'{body}</table></form></body></html>'
    )


class CalendarBridge(Bridge):
    def __init__(self, wrong=(), **kwargs):
        super().__init__(**kwargs)
        self.calendars = []
        self.wrong_calendar = set(wrong)

    def call(self, command, payload=None, timeout=75):
        if command != 'calendar_open':
            return super().call(command, payload, timeout)
        self.events.append((command, None))
        args = payload['args']
        self.calendars.append(args[10])
        ingreso = '999' if args[10] in self.wrong_calendar else args[10]
        data = calendar_page(args[0], args[2], ingreso).encode('utf-8')
        return {
            'status': 200,
            'type': 'text/html',
            'body': base64.b64encode(data).decode(),
            'digest': hashlib.sha256(data).hexdigest(),
            'args': args,
            'startedAt': '2026-10-19T12:00:00Z',
        }


def run(tmp_path, bridge, tabs=('Cumplimiento',), calendars=True, resume=None):
    parser()
    return lb.DiaryRunner(bridge, CATALOG, lambda *a: None, threading.Event(), pause=0, calendars=calendars).run(
        Batch(('49',), ('2',), tabs), tmp_path, resume
    )


def records(result):
    return json.loads(Path(result['manifest']).read_text(encoding='utf-8'))['registros']


def test_calendario_tras_cada_bitacora_y_solo_con_casilla(tmp_path):
    bridge = CalendarBridge()
    result = run(tmp_path / 'con', bridge)
    assert result['estado'] == 'COMPLETA' and result['calendarios_leidos'] == 2
    assert bridge.calendars == ['201', '202']
    for record in records(result):
        reading = record['calendario_lectura']
        assert reading['estado'] == 'LEIDA' and reading['archivo'].startswith('calendario-')
        assert reading['proyecciones'] == [
            {'fila': 1, 'fecha_proyeccion': '2026-11-10', 'fecha_informe': '', 'estado': 'PENDIENTE'}
        ]
    commands = [c for c, _ in bridge.events]
    assert commands.index('calendar_open') == commands.index('diary_open') + 1
    bridge = CalendarBridge()
    assert run(tmp_path / 'sin', bridge, calendars=False)['calendarios_leidos'] == 0 and not bridge.calendars
    bridge = CalendarBridge()
    run(tmp_path / 'espera', bridge, tabs=('Espera',))
    assert not bridge.calendars


def test_calendario_ajeno_falla_sin_invalidar_la_bitacora_y_se_retoma(tmp_path):
    bridge = CalendarBridge(wrong={'202'})
    result = run(tmp_path, bridge)
    assert result['estado'] == 'COMPLETA' and result['leidas'] == 2 and result['calendarios_fallidos'] == 1
    failed = records(result)[1]['calendario_lectura']
    assert failed['estado'] == 'FALLIDA' and 'no corresponde al ingreso' in failed['error']
    bridge = CalendarBridge()
    again = run(tmp_path, bridge, resume=result['folder'])
    assert bridge.calendars == ['202'] and again['calendarios_leidos'] == 2
    assert records(again)[0]['calendario_lectura']['reutilizada'] is True


def test_calendario_cuenta_en_el_maximo_de_lecturas(tmp_path, monkeypatch):
    monkeypatch.setattr(lb, 'MAX_LECTURAS_POR_LOTE', 3)
    bridge = CalendarBridge()
    result = run(tmp_path, bridge)
    first, second = records(result)
    assert result['estado'] == 'INCOMPLETA' and len(bridge.openings) == 2 and bridge.calendars == ['201']
    assert second['estado'] == 'LEIDA' and second['calendario_lectura']['estado'] == 'PENDIENTE'
    assert first['calendario_lectura']['estado'] == 'LEIDA'


def _with_calendars(manifest, rows_by_ingreso):
    data = json.loads(manifest.read_text(encoding='utf-8'))
    for record in data['registros']:
        rows = rows_by_ingreso.get(record['ingreso_id'])
        if rows is None:
            continue
        args = [
            record['causa_id'],
            '300',
            '113',
            '400',
            'X-1-2026',
            '01/01/2026',
            'PRM Centro',
            'Persona',
            '12 Meses',
            '3 Meses',
            record['ingreso_id'],
            '0',
        ]
        raw = calendar_page(record['causa_id'], '113', record['ingreso_id'], rows).encode('utf-8')
        name = f"calendario-{record['ingreso_id'] * 8}-{'0' * 16}.html"
        (manifest.parent / name).write_bytes(raw)
        record['calendario_lectura'] = {
            'estado': 'LEIDA',
            'args': args,
            'archivo': name,
            'sha256': hashlib.sha256(raw).hexdigest(),
            'capturada': '2026-10-19T12:00:00Z',
        }
    manifest.write_text(json.dumps(data, ensure_ascii=False), encoding='utf-8')
    return manifest


def test_paquete_exporta_proyecciones_verificadas_y_el_asistente_coteja(tmp_path):
    manifest = _with_calendars(
        diaries(tmp_path / 'bitacoras', 2),
        {
            '201': (('01/08/2026', '30/07/2026', 'ENVIADO'), ('01/11/2026', '', 'PENDIENTE')),
            '202': (('20/12/2026', '', 'PENDIENTE'),),
        },
    )
    lots = [bound_lot(tmp_path / 'principal'), bound_lot(tmp_path / 'informes', 'Informes')]
    path = export_capture(lots, tmp_path / 'paquete.zip', diaries=[manifest], match_diaries=True)
    capture = read_capture(path)
    rows = capture.rows('calendarios')
    assert {(r['ingreso_id'], r['event_at'], r['status']) for r in rows} == {
        ('201', '2026-08-01', 'ENVIADO'),
        ('201', '2026-11-01', 'PENDIENTE'),
        ('202', '2026-12-20', 'PENDIENTE'),
    }
    assert {r['event_type'] for r in rows} == {'INFORME_PROYECTADO'} and {r['association'] for r in rows} == {
        'VERIFICADA'
    }
    assert json.loads(rows[0]['normalized_json'])['plazo_remision_meses'] == 3
    screens = [q['screen'] for q in capture.rows('consultas')]
    assert screens.count('calendario_ingreso') == 2
    assert capture.manifest['table_states']['calendarios'] == 'COMPLETA'
    work = import_capture(path, defaults(), as_of=date(2026, 10, 19))
    # Las proyecciones complementan el registro; no se agregan como registros propios.
    assert len(work.rows) == 4
    by_type = {
        (work.capture_sources[r.id]['record_type'], work.capture_sources[r.id]['ingreso_id']): r for r in work.rows
    }
    informes = by_type['INFORMES', '201']
    assert any(
        w.startswith('CAL-03: la pestaña Informes indica vencimiento el 10 de octubre de 2026')
        for w in informes.warnings
    )
    # Sin Hoja2, el calendario aporta la fecha del próximo informe para C-10.
    assert work.cross_dates.get(by_type['CUMPLIMIENTO', '201'].id) in ('2026-11-01', date(2026, 11, 1))
