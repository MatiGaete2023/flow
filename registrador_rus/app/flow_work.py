"""Puerta local de revisión de Work. No modifica Work ni habilita escritura RUS."""
import json
from .contracts import ContractError
from .flow_export import candidate, approve_candidate, ReviewCandidate

TABS = {'ESPERA': 'Espera', 'CUMPLIMIENTO': 'Cumplimiento',
        'INFORMES_POR_VENCER': 'Informes', 'INFORMES_VENCIDOS': 'Informes'}
REPORT_LABELS = {'INFORMES_POR_VENCER': 'Por vencer (a 25 días)', 'INFORMES_VENCIDOS': 'Vencidos'}
# Evidencia del recurso descargador/fixtures/selector_vencimiento_sitfa.js,
# rama tdInforme. Solo para comprobar procedencia histórica, no catálogo vivo.
REPOSITORY_REPORT_CATALOG = {'1': 'Por vencer (a 25 días)', '2': 'Vencidos', '3': 'Recibidos'}


def _one(rows, key, value):
    hits = [row for row in rows if row.get(key) == value]
    if len(hits) != 1:
        raise ContractError('PROCEDENCIA_AUSENTE_O_AMBIGUA')
    return hits[0]


def verify_origin(source, queries, origin, *, report_catalog=None):
    """Prueba el menú y la pestaña con la consulta de origen del paquete.
    
    Para Informes exige catálogo de códigos observado localmente por el consumidor:
    no deduce el filtro de una fecha ni utiliza códigos supuestos.
    """
    if origin not in TABS or not isinstance(source, dict):
        raise ContractError('ORIGEN_INVALIDO')
    q = _one(queries, 'query_id', source.get('query_id'))
    if (not source.get('query_id') or source.get('source_system') != 'RUS' or
            q.get('source_system') != 'RUS' or q.get('screen') != 'seguimiento' or
            q.get('tab') != TABS[origin] or source.get('tab') != q.get('tab') or
            not source.get('court_code') or source.get('court_code') != q.get('court_code')):
        raise ContractError('CONSULTA_FUERA_DE_ALCANCE')
    if origin in REPORT_LABELS:
        try:
            filters = json.loads(q['filters_json'])
            report = filters['report']
            if (type(report_catalog) is not dict or type(report) is not str or
                    report_catalog.get(report) != REPORT_LABELS[origin]):
                raise ValueError
        except (TypeError, KeyError, ValueError):
            raise ContractError('FILTRO_INFORMES_NO_ACREDITADO') from None


def load_verified_capture(work):
    """Una lectura validada por fase, nunca una autorización de envío."""
    # Dependencias reales de Flow: importación tardía para conservar demo autónoma.
    from nurus.personal.work import Work
    from nurus.personal.sync import require_current_copy
    from nurus.capture import read_capture

    if not isinstance(work, Work) or getattr(work, 'source_scope', '') != 'PAQUETE':
        raise ContractError('TRABAJO_FLOW_NO_VERIFICADO')
    try:
        require_current_copy(work)
        capture = read_capture(work.path)
    except (OSError, ValueError, RuntimeError):
        raise ContractError('FUENTE_O_COPIA_NO_VIGENTE') from None
    if capture.digest != getattr(work, 'capture_digest', ''):
        raise ContractError('PAQUETE_CAMBIADO')
    return capture


def verify_record(work, capture, row_ref, origin, report_catalog):
    source = _one(capture.rows('registros'), 'record_id', row_ref)
    if source != getattr(work, 'capture_sources', {}).get(row_ref):
        raise ContractError('PROCEDENCIA_CAMBIADA')
    verify_origin(source, capture.rows('consultas'), origin, report_catalog=report_catalog)


def _guard(work, row_ref, origin, report_catalog):
    verify_record(work, load_verified_capture(work), row_ref, origin, report_catalog)


def candidate_from_work(work, row_ref, *, work_id, origin, destination, observation_state=None, report_catalog=None):
    _guard(work, row_ref, origin, report_catalog)
    return candidate(work, row_ref, work_id=work_id, origin=origin, destination=destination,
                     observation_state=observation_state)


def approve_from_work(work, frozen, *, report_catalog=None, **confirmation):
    if type(frozen) is not ReviewCandidate:
        raise ContractError('REVISION_INVALIDA')
    _guard(work, frozen.row_ref, frozen.origin, report_catalog)
    return approve_candidate(work, frozen, **confirmation)
