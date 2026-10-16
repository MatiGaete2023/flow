"""Descarga de Meta 4 reutilizando el transporte de solo lectura del Descargador."""
from datetime import datetime
import hashlib
import json
from pathlib import Path
import sys

from meta4_bitacoras import BASE, entradas_periodo, periodo, generar
from lotes import Batch, BatchRunner, ChromeTransport
from lectura_bitacoras import DiaryRunner, LimiteLecturas, MAX_LECTURAS_POR_LOTE, write
import motor as m


class DiarioMensual(DiaryRunner):
    def __init__(self, *args, corte, **kwargs):
        super().__init__(*args, reuse_base=None, **kwargs)
        self.corte = corte

    def _abrir_bitacora(self, session, record):
        if session.openings >= MAX_LECTURAS_POR_LOTE:
            raise LimiteLecturas('Límite de 600 lecturas: selecciona menos tribunales por ejecución.')
        session.openings += 1
        result = self.bridge.call('diary_open', {'params': record['params'], 'timeout': 60000})
        body = ChromeTransport.body(result)
        digest = hashlib.sha256(body).hexdigest()
        if len(body) > 8 * 1024 * 1024 or digest != result.get('digest') or result.get('params') != record['params']:
            raise m.PocError('La respuesta de bitácora no coincide con la solicitud.')
        datetime.fromisoformat(result['startedAt'].replace('Z', '+00:00'))
        snapshot = session.read(body, record['params'])
        session.anotar(record, snapshot)
        snapshot['entries'] = entradas_periodo(snapshot['entries'], self.corte)
        # No se guarda HTML, textos de meses anteriores ni metadatos de esas entradas.
        keep = ('tribunal_codigo', 'causa_id', 'ingreso_id', 'rit', 'persona', 'centro', 'entries')
        record.update(estado='LEIDA', capturada=result['startedAt'],
                      snapshot={k: snapshot[k] for k in keep}, entradas=len(snapshot['entries']))

    def _resultado(self, session):
        data = dict(session.state)
        data.update(tipo='META4_BITACORAS_MES', desde=str(periodo(self.corte)[0]), hasta=str(self.corte))
        target = session.folder / 'bitacoras_meta4.json'
        write(target, data)
        session.manifest.unlink(missing_ok=True)
        value = {'manifest': str(target), 'estado': data['estado'],
                 'leidas': sum(r['estado'] == 'LEIDA' for r in data['registros']),
                 'fallidas': sum(r['estado'] != 'LEIDA' for r in data['registros'])}
        self.emit('diary_done', value)
        return value


def descargar(bridge, catalog, tribunales, destination, corte, emit, cancel):
    batch = Batch(tuple(tribunales), ('1', '3'), ('Espera', 'Cumplimiento'))
    batch.plan(catalog)
    root = Path(destination) / ('Descarga META4 ' + datetime.now().strftime('%Y-%m-%d %H-%M-%S-%f'))
    root.mkdir(parents=True)
    emit('status', 'Descargando registros de Espera y Cumplimiento, FAE y Residencia…')
    BatchRunner(bridge, catalog, emit, cancel, pause=1).run(batch, root)
    if cancel.is_set():
        raise m.PocError('Proceso detenido; se conservan las fuentes descargadas.')
    emit('status', 'Leyendo bitácoras y conservando solo el último mes…')
    result = DiarioMensual(bridge, catalog, emit, cancel, corte=corte).run(batch, root)
    if cancel.is_set():
        raise m.PocError('Proceso detenido; se conservan las bitácoras leídas.')
    report = generar(root, root / 'Resultado META4', corte)
    report['lectura'] = result['estado']
    emit('status', f"Terminado: {report['registros']} registros, {report['alertas']} alertas. Lectura {result['estado']}.")
    return root / 'Resultado META4', report
