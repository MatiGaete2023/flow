"""Orquesta runners existentes; conserva fuentes y entrega un ZIP sin pasos manuales."""

from pathlib import Path

import motor as m
from lotes import BatchRunner
from lectura_bitacoras import DiaryRunner, write


def run(bridge, catalog, emit, cancel, batch, destination, consolidated=True, *, pause=1.0):
    if batch.screen != 'seguimiento':
        raise m.PocError('Consultar y crear paquete requiere Seguimiento; las otras consultas siguen en Avanzado.')
    batch.plan(catalog)
    root = Path(destination) / ('Paquete CSMP ' + m.now().strftime('%Y-%m-%d %H-%M-%S-%f'))
    backup = root / 'respaldo'
    backup.mkdir(parents=True)
    control = backup / 'proceso.json'
    state = {'estado': 'EN_CURSO', 'fase': 'consultas', 'carpetas': {}}
    write(control, state)
    emit('pipeline_folder', str(root))

    def checkpoint():
        if cancel.is_set():
            raise m.PocError('Proceso detenido. Los originales verificados se conservan en respaldo.')

    def phase(name):
        checkpoint()
        state['fase'] = name
        write(control, state)
        emit('phase', name)

    def relay(name, value):
        if name == 'folder':
            folder = Path(value).resolve()
            if not folder.is_relative_to(backup.resolve()):
                raise m.PocError('La carpeta de fuentes quedó fuera de respaldo.')
            state['carpetas'][state['fase']] = str(folder.relative_to(root))
            write(control, state)
            emit('pipeline_source', {'fase': state['fase'], 'folder': str(folder)})
        elif name not in ('done', 'diary_done'):
            emit(name, value)

    try:
        phase('consultas')
        BatchRunner(bridge, catalog, relay, cancel, pause=pause).run(batch, backup)
        if 'consultas' not in state['carpetas']:
            raise m.PocError('La consulta no identificó sus originales; no se creó el paquete.')
        phase('bitacoras')
        diaries = DiaryRunner(bridge, catalog, relay, cancel, pause=pause).run(batch, backup)
        phase('paquete')
        from descargador import package_current_lot

        sheets = root / state['carpetas']['consultas']
        manifest = Path(diaries['manifest'])
        if not manifest.resolve().is_relative_to(backup.resolve()):
            raise m.PocError('El control de bitácoras quedó fuera de respaldo.')
        result = package_current_lot(sheets, manifest, root / 'Captura_CSMP.zip', consolidated)
        partial = diaries['estado'] != 'COMPLETA' or bool(diaries['fallidas'] or diaries['consultas_fallidas'])
        state.update(estado='PARCIAL' if partial else 'COMPLETO', paquete=Path(result['path']).name)
        write(control, state)
        result.update(sheets_folder=str(sheets), diary_manifest=str(manifest), partial=partial)
        emit('package_done', result)
        return result
    except BaseException:
        state['estado'] = 'INCOMPLETO'
        write(control, state)
        raise
