"""Fixtures ficticios compartidos por las pruebas de historia de la causa."""

import json
from datetime import date
from types import SimpleNamespace

from openpyxl import Workbook

from nurus.capture.contract import TABLES
from nurus.capture.fixtures import fixture
from nurus.personal.config import defaults
from nurus.personal.motor.historia import construir, observaciones
from nurus.personal.work import Work

DESDE, HASTA = date(2026, 6, 14), date(2026, 10, 14)


def hist(*items, cobertura='COMPLETA', desde=DESDE, hasta=HASTA):
    """items: (descripción, 'aaaa-mm-dd', folio)."""
    filas = [{'event_at': fecha, 'description': desc, 'folio': str(folio)} for desc, fecha, folio in items]
    return construir(filas, cobertura, desde, hasta)


def bita(*items):
    """items: ('aaaa-mm-dd', texto)."""
    return observaciones([{'event_at': fecha, 'text': texto} for fecha, texto in items])


class Todas(dict):
    """dict cuyo get devuelve siempre el mismo valor: prueba sin conocer los record_id."""

    def __init__(self, valor):
        super().__init__()
        self.valor = valor

    def get(self, clave, default=None):
        return self.valor


def captura_con_historia(tramites, cobertura='COMPLETA', causa='causa-ficticia-0', registros=4):
    """(metadata, tablas) del paquete ficticio con historia de UNA causa. tramites: (desc, fecha, folio)."""
    metadata, tablas = fixture(registros)
    base = tablas['registros'][0]
    consulta = {k: '' for k in TABLES['consultas']}
    filtros = {
        'historia_causa': {'source_system': 'RUS', 'court_code': base['court_code'], 'cause_id': causa},
        'ventana': {'criterio': 'ventana_4_meses', 'desde': DESDE.isoformat(), 'hasta': HASTA.isoformat()},
    }
    consulta.update(
        query_id='hist-1',
        source_system='RUS',
        screen='historia_tramites',
        tab='HISTORIA',
        court_code=base['court_code'],
        modality_code=base['modality_code'],
        court_name=base['court_name'],
        coverage=cobertura,
        captured_at=base['captured_at'],
        filters_json=json.dumps(filtros),
        expected_pages='1',
        saved_pages='1',
        time_zone='America/Santiago',
        history_complete='false',
        institutional_verified='false',
    )
    tablas['consultas'].append(consulta)
    comunes = {k: base[k] for k in TABLES['registros'] if k in base and k in TABLES['historia_tramites']}
    filas = []
    for n, (desc, fecha, folio) in enumerate(tramites, 1):
        fila = {k: '' for k in TABLES['historia_tramites']}
        fila.update(comunes)
        fila.update(
            query_id='hist-1',
            cause_id=causa,
            ingreso_id='',
            person_id='',
            program_id='',
            coverage='PARCIAL' if cobertura == 'PARCIAL' else 'COMPLETA',
            event_id=f'hist-1-{n}',
            event_at=fecha,
            time_precision='DIA',
            time_zone='DESCONOCIDA',
            scope='CAUSA',
            event_type='tramite',
            description=desc,
            reference='',
            status='CAPTURADO',
            identity_status='CAUSA',
            folio=str(folio),
            stage='Cumplimiento',
            annex_visible='false',
            raw_json='{}',
        )
        filas.append(fila)
    tablas['historia_tramites'] = filas
    metadata['table_states']['historia_tramites'] = cobertura
    return metadata, tablas


def captura_falsa(consultas, filas):
    """Sustituto mínimo de la captura para probar _historias_por_causa."""
    return SimpleNamespace(
        manifest={'queries': consultas}, rows=lambda tabla: filas if tabla == 'historia_tramites' else []
    )


# ---- Work.analyze con historia inyectada (sin paquete ZIP) ----
CORTE = date(2026, 10, 14)
ESPERA = ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'NOMBRE CENTRO', 'T ESPERA']
CUMPLIMIENTO = [
    'RIT',
    'TRIBUNAL',
    'NOMBRE',
    'RUT',
    'NOMBRE CENTRO',
    'DIAS DE CUMPLIMIENTO',
    'DIAS PARA EGRESAR',
    'FEC. INGRESO EFECTIVO',
    'FEC. EGRESO PROYECTADO',
    'FEC.ACT.F.INDIVIDUAL',
    'FEC.ACT.F.FAE',
    'FEC. NACIMIENTO',
]
INFORMES = ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'NOMBRE CENTRO', 'FECHA VENCIMIENTO']
ORDEN = 'Orden de búsqueda'


def fila(n, programa='PRM UNO', tribunal='LAJA', rit=None, rut='11111111-0'):
    return [rit or f'X-{n}-2026', tribunal, f'Persona Ficticia {n}', rut, programa]


def correr(tmp_path, modo, encabezados, filas, historias=None, bitacoras=None, config=None, paquete=True):
    libro = Workbook()
    hoja = libro.active
    hoja.title = modo
    hoja.append(encabezados)
    for datos in filas:
        hoja.append(datos)
    ruta = tmp_path / f'h_{modo.lower()}.xlsx'
    libro.save(ruta)
    if not paquete:
        return Work(config or defaults()).analyze(ruta, modo, as_of=CORTE)
    ids = [r.id for r in Work(config or defaults()).analyze(ruta, modo, as_of=CORTE).rows]
    return Work(config or defaults()).analyze(
        ruta,
        modo,
        as_of=CORTE,
        historia_by_record={ids[i]: h for i, h in (historias or {}).items()},
        bitacora_by_record={ids[i]: b for i, b in (bitacoras or {}).items()},
    )


def esp(tmp_path, datos, historia=None, bitacora=None, extra=(), encabezados=()):
    return correr(
        tmp_path,
        'ESPERA',
        ESPERA + list(encabezados),
        [datos + list(extra)],
        {0: historia} if historia else None,
        {0: bitacora} if bitacora else None,
    ).rows[0]


def cum(tmp_path, datos, historia=None, bitacora=None):
    return correr(
        tmp_path,
        'CUMPLIMIENTO',
        CUMPLIMIENTO,
        [datos],
        {0: historia} if historia else None,
        {0: bitacora} if bitacora else None,
    ).rows[0]


def cumplimiento(
    n, programa='PRM UNO', dc=100, dpe=100, ingreso='2026-06-01', egreso='2027-05-01', ind='', fae='', nac=''
):
    return fila(n, programa) + [dc, dpe, ingreso, egreso, ind, fae, nac]


# ---------- Informes ----------
def inf(tmp_path, datos, historia=None):
    return correr(tmp_path, 'INFORMES', INFORMES, [datos], {0: historia} if historia else None).rows[0]
