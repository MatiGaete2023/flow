"""Orquesta runners existentes; conserva fuentes y entrega un ZIP sin pasos manuales."""

from datetime import datetime
from pathlib import Path

import motor as m
from lotes import BatchRunner
from lectura_bitacoras import DiaryRunner, write


def _historia_anterior(destination):
    """Control de historia más reciente del mismo destino, leído hoy y hace menos de 12 h; None si no hay."""
    import time

    limite = time.time() - 12 * 3600
    hoy = m.now().date()
    candidatos = []
    for ruta in Path(destination).glob('Paquete CSMP */respaldo/*/historia/historias.json'):
        if ruta.is_symlink() or not ruta.is_file():
            continue
        modificado = ruta.stat().st_mtime
        if modificado >= limite and datetime.fromtimestamp(modificado, m.now().tzinfo).date() == hoy:
            candidatos.append((modificado, ruta))
    return max(candidatos)[1] if candidatos else None


def run(
    bridge,
    catalog,
    emit,
    cancel,
    batch,
    destination,
    consolidated=True,
    *,
    pause=1.0,
    reuse=True,
    historia=False,
    historia_en_paquete=True,
):
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
        reuse_base = Path(destination) if reuse else None
        lector = None
        if historia:
            from lectura_historia import LectorHistoria

            lector = LectorHistoria(
                bridge, relay, cancel, pause=pause, anterior=_historia_anterior(destination) if reuse else None
            )
        extra = {'historia': lector} if lector else {}
        diaries = DiaryRunner(bridge, catalog, relay, cancel, pause=pause, reuse_base=reuse_base, **extra).run(
            batch, backup
        )
        phase('paquete')
        from descargador import package_current_lot

        sheets = root / state['carpetas']['consultas']
        manifest = Path(diaries['manifest'])
        if not manifest.resolve().is_relative_to(backup.resolve()):
            raise m.PocError('El control de bitácoras quedó fuera de respaldo.')
        resumen = lector.resultado if lector else None
        historia_ok = bool(lector and resumen and resumen['estado'] == 'COMPLETA')
        ruta_historia = None
        if lector and historia_en_paquete and lector.carpeta:
            candidata = lector.carpeta / 'historias.json'
            if candidata.is_file() and candidata.resolve().is_relative_to(backup.resolve()):
                ruta_historia = candidata
        target = root / 'Captura_CSMP.zip'
        try:
            result = package_current_lot(
                sheets, manifest, target, consolidated, **({'historia': ruta_historia} if ruta_historia else {})
            )
        except (m.PocError, ValueError, OSError) as exc:
            if not ruta_historia:
                raise
            # Un fallo de la historia nunca impide el paquete de siempre.
            emit('warning', 'La historia no pudo incluirse en el paquete: ' + str(exc))
            historia_ok = False
            result = package_current_lot(sheets, manifest, target, consolidated)
        partial = diaries['estado'] != 'COMPLETA' or bool(diaries['fallidas'] or diaries['consultas_fallidas'])
        if historia:
            partial = partial or not historia_ok
            result['historia'] = resumen or {'estado': 'INCOMPLETA'}
        state.update(estado='PARCIAL' if partial else 'COMPLETO', paquete=Path(result['path']).name)
        write(control, state)
        result.update(sheets_folder=str(sheets), diary_manifest=str(manifest), partial=partial)
        emit('package_done', result)
        return result
    except BaseException:
        state['estado'] = 'INCOMPLETO'
        write(control, state)
        raise
