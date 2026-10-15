"""Orquesta runners existentes; conserva fuentes y entrega un ZIP sin pasos manuales."""

from dataclasses import replace
from pathlib import Path

import motor as m
from lotes import BatchRunner
from lectura_bitacoras import DiaryRunner, write


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
    meses=4,
    informes=False,
    esperar_pantalla=None,
    today=None,
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

            lector = LectorHistoria(bridge, relay, cancel, pause=pause, meses=meses)
        extra = {'historia': lector} if lector else {}
        diaries = DiaryRunner(bridge, catalog, relay, cancel, pause=pause, reuse_base=reuse_base, **extra).run(
            batch, backup
        )
        carpetas_informes = _informes(
            bridge, relay, emit, cancel, batch, backup, state, phase, pause, informes, esperar_pantalla, today
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
        opciones = {}
        if carpetas_informes:
            opciones['extra_folders'] = carpetas_informes
        if meses != 4:
            opciones['meses'] = meses
        try:
            result = package_current_lot(
                sheets,
                manifest,
                target,
                consolidated,
                **opciones,
                **({'historia': ruta_historia} if ruta_historia else {}),
            )
        except (m.PocError, ValueError, OSError):
            if not ruta_historia:
                raise
            # Un fallo de la historia nunca impide el paquete de siempre.
            emit('warning', 'La historia no pudo incluirse en el paquete; las bitácoras se conservan.')
            historia_ok = False
            result = package_current_lot(sheets, manifest, target, consolidated, **opciones)
        partial = diaries['estado'] != 'COMPLETA' or bool(diaries['fallidas'] or diaries['consultas_fallidas'])
        if informes:
            # Sin el calendario el paquete se crea igual, pero queda marcado como parcial.
            partial = partial or not carpetas_informes
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


def _informes(bridge, relay, emit, cancel, batch, backup, state, phase, pause, informes, esperar_pantalla, today):
    """Calendario de informes por vencer (mes actual y siguiente, unos 60 días): pide al usuario cambiar SITFA a ese
    calendario y actualizar las opciones; si no se logra, el paquete se crea igual y queda parcial."""
    if not informes:
        return []
    from flujo_csmp import months
    from lotes import Batch

    phase('informes')
    try:
        if esperar_pantalla:
            esperar_pantalla('calendario_informes')
        catalog = bridge.call('catalog', {'screen': 'calendario_informes'})
        carpetas = []
        for month, year in months(today or m.now().date()):
            state['fase'] = f'informes-{year}-{month}'
            plan = Batch(batch.tribunals, (), (), screen='calendario_informes', month=month, year=year)
            BatchRunner(bridge, catalog, relay, cancel, pause=pause).run(plan, backup)
            carpetas.append(backup.parent / state['carpetas'][state['fase']])
        return carpetas
    except m.PocError as exc:
        if cancel.is_set():
            raise
        emit('warning', 'No se incluyó el calendario de informes por vencer: ' + str(exc))
        return []


def run_por_hoja(bridge, catalog, emit, cancel, batch, destination, consolidated=True, **opciones):
    """Un paquete por hoja de RUS (Espera, Cumplimiento, Informes) cuando hay varias: cada hoja se consulta, se lee y se
    entrega por separado, así se puede empezar a trabajar con la primera mientras corre la siguiente y un corte solo
    pierde la hoja en curso. El calendario de informes por vencer se agrega a la hoja de Cumplimiento."""
    if len(batch.tabs) < 2:
        return [run(bridge, catalog, emit, cancel, batch, destination, consolidated, **opciones)]
    resultados = []
    for numero, hoja in enumerate(batch.tabs, 1):
        if cancel.is_set():
            break
        emit('status', f'Hoja {numero} de {len(batch.tabs)}: {hoja}.')
        parcial = dict(opciones)
        parcial['informes'] = bool(opciones.get('informes')) and hoja == 'Cumplimiento'
        resultados.append(
            run(bridge, catalog, emit, cancel, replace(batch, tabs=(hoja,)), destination, consolidated, **parcial)
        )
    return resultados
