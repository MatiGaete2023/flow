"""Orquesta runners existentes; conserva fuentes y entrega un ZIP sin pasos manuales."""

from dataclasses import asdict, is_dataclass, replace
import json
from pathlib import Path

import motor as m
from lotes import BatchRunner
from lectura_bitacoras import MANIFEST, DiaryRunner, saved_batch, write

PROCESO = 'proceso.json'


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
    calendars=False,
    historia=False,
    historia_en_paquete=True,
    meses=4,
    informes=False,
    today=None,
    resume=None,
    pdf_vacias=True,
):
    """Consulta, bitácoras, informes y ZIP. Con `resume` (carpeta «Paquete CSMP …») continúa un paquete anterior:
    reutiliza las consultas validadas y las bitácoras ya leídas, y vuelve a crear el ZIP."""
    if resume:
        root = Path(resume).resolve()
        backup = root / 'respaldo'
        control = backup / PROCESO
        state = _proceso(control)
        batch = _alcance(root, state)
        state.update(estado='EN_CURSO', reintentos=state.get('reintentos', 0) + 1)
    else:
        root = Path(destination) / ('Paquete CSMP ' + m.now().strftime('%Y-%m-%d %H-%M-%S-%f'))
        backup = root / 'respaldo'
        control = backup / PROCESO
        state = {'estado': 'EN_CURSO', 'fase': 'consultas', 'carpetas': {}}
        if is_dataclass(batch):
            state['seleccion'] = asdict(batch)
    if batch.screen != 'seguimiento':
        raise m.PocError('Consultar y crear paquete requiere Seguimiento; las otras consultas siguen en Avanzado.')
    batch.plan(catalog)
    backup.mkdir(parents=True, exist_ok=bool(resume))
    write(control, state)
    emit('pipeline_folder', str(root))
    anterior = {fase: root / carpeta for fase, carpeta in state['carpetas'].items()} if resume else {}

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
        if _consultas_validadas(anterior.get('consultas')):
            emit('status', 'Consultas ya validadas en el intento anterior: se reutilizan.')
        else:
            state['carpetas'].pop('consultas', None)
            BatchRunner(bridge, catalog, relay, cancel, pause=pause, evidencia=pdf_vacias).run(batch, backup)
        if 'consultas' not in state['carpetas']:
            raise m.PocError('La consulta no identificó sus originales; no se creó el paquete.')
        phase('bitacoras')
        previas = anterior.get('bitacoras')
        previas = previas if previas and (previas / MANIFEST).is_file() else None
        reuse_base = Path(destination) if reuse and not resume else None
        lector = None
        if historia:
            from lectura_historia import LectorHistoria

            historia_previa = previas / 'historia' / 'historias.json' if previas else None
            lector = LectorHistoria(
                bridge,
                relay,
                cancel,
                pause=pause,
                meses=meses,
                **({'anterior': historia_previa} if historia_previa and historia_previa.is_file() else {}),
            )
        extra = {'historia': lector} if lector else {}
        # Cada lectura opcional se pide solo si está activa, como la historia.
        if calendars:
            extra['calendars'] = True
        runner = DiaryRunner(bridge, catalog, relay, cancel, pause=pause, reuse_base=reuse_base, **extra)
        # Al retomar, el mismo lote de bitácoras continúa: lo ya leído (hash comprobado) no se vuelve a abrir.
        diaries = runner.run(batch, backup, previas) if previas else runner.run(batch, backup)
        carpetas_informes, informes_completos = _informes(
            bridge, relay, emit, cancel, batch, backup, state, phase, pause, informes, today, pdf_vacias
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
        if target.exists():
            # El ZIP parcial del intento anterior se guarda en respaldo; el nuevo ocupa su lugar.
            target.replace(backup / ('Captura_CSMP anterior ' + m.now().strftime('%Y-%m-%d %H-%M-%S-%f') + '.zip'))
        opciones = {}
        if not informes_completos:
            opciones['avisos'] = [
                (
                    'CALENDARIO_INCOMPLETO',
                    'Se pidió el calendario de informes por vencer y no quedó completo en el paquete.',
                )
            ]
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
        # Un calendario por ingreso (WD-1) fallido o pendiente también deja el paquete parcial.
        partial = partial or bool(diaries.get('calendarios_fallidos'))
        # Sin el calendario (o con un mes faltante) el paquete se crea igual, pero queda marcado como parcial.
        partial = partial or not informes_completos
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


def _proceso(control):
    try:
        state = json.loads(control.read_text(encoding='utf-8'))
    except (OSError, ValueError):
        raise m.PocError('Elige una carpeta «Paquete CSMP …» creada por «Consultar y crear paquete».') from None
    if not isinstance(state, dict) or not isinstance(state.get('carpetas'), dict):
        raise m.PocError('El control del paquete (respaldo/proceso.json) no es válido.')
    if state.get('estado') == 'COMPLETO':
        raise m.PocError('Ese paquete ya está completo; no hay nada que retomar.')
    for carpeta in state['carpetas'].values():
        if not isinstance(carpeta, str) or not (control.parent.parent / carpeta).resolve().is_relative_to(
            control.parent.resolve()
        ):
            raise m.PocError('El control del paquete apunta fuera de su respaldo.')
    return state


def pendientes(destino, dias=7):
    """Paquetes cortados en `destino` (los últimos `dias`), del más reciente al más antiguo: [(carpeta, fase)].

    Solo los que quedaron EN_CURSO o INCOMPLETO: los PARCIALES terminaron y se revisan en el Asistente."""
    import time

    destino, limite, encontrados = Path(destino), time.time() - dias * 86400, []
    try:
        candidatos = list(destino.glob('Paquete CSMP */respaldo/' + PROCESO))
    except OSError:
        return []
    for control in candidatos:
        try:
            if control.stat().st_mtime < limite:
                continue
            state = json.loads(control.read_text(encoding='utf-8'))
        except (OSError, ValueError):
            continue
        if isinstance(state, dict) and state.get('estado') in ('EN_CURSO', 'INCOMPLETO'):
            encontrados.append((control.stat().st_mtime, control.parent.parent, state.get('fase', '')))
    return [(carpeta, fase) for _, carpeta, fase in sorted(encontrados, reverse=True)]


def _alcance(root, state):
    """Alcance guardado del paquete; los paquetes anteriores a 2.8.2 solo lo tienen en su lote de bitácoras."""
    if isinstance(state.get('seleccion'), dict):
        return saved_batch({'seleccion': state['seleccion']})
    carpeta = state['carpetas'].get('bitacoras')
    if carpeta and (root / carpeta / MANIFEST).is_file():
        from lectura_bitacoras import load

        return saved_batch(load(root / carpeta / MANIFEST))
    raise m.PocError('Este paquete no guardó su alcance (se cortó en las consultas). Crea un paquete nuevo.')


def _consultas_validadas(carpeta):
    try:
        return json.loads((carpeta / 'resumen.json').read_text(encoding='utf-8')).get('estado') == 'VALIDADA'
    except (TypeError, OSError, ValueError, AttributeError):
        return False


def retomar(bridge, catalog, emit, cancel, carpeta, consolidated=True, **opciones):
    return run(bridge, catalog, emit, cancel, None, None, consolidated, resume=carpeta, **opciones)


def meses_informes(today, dias=60):
    """Meses (mes, año) del calendario que cubren desde hoy hasta hoy + `dias`: a veces dos, a veces tres."""
    from datetime import timedelta

    fin = today + timedelta(days=dias)
    meses, actual = [], today.replace(day=1)
    while actual <= fin:
        meses.append((str(actual.month), str(actual.year)))
        actual = (actual.replace(day=28) + timedelta(days=4)).replace(day=1)
    return meses


def _informes(bridge, relay, emit, cancel, batch, backup, state, phase, pause, informes, today, pdf_vacias=True):
    """Calendario de informes por vencer de los próximos 60 días, solo con la hoja de Cumplimiento (es la que lo usa).
    La extensión cambia SITFA al calendario por sí sola (sin pedirlo al usuario) y al terminar vuelve a Seguimiento.
    Devuelve (carpetas, completo): un mes que falla no descarta los ya descargados, pero el paquete queda parcial."""
    if not informes or 'Cumplimiento' not in getattr(batch, 'tabs', ()):
        return [], True
    from lotes import Batch

    phase('informes')
    carpetas = []
    meses = meses_informes(today or m.now().date())
    try:
        catalog = bridge.call('catalog', {'screen': 'calendario_informes'})
        for month, year in meses:
            state['fase'] = f'informes-{year}-{month}'
            plan = Batch(batch.tribunals, (), (), screen='calendario_informes', month=month, year=year)
            BatchRunner(bridge, catalog, relay, cancel, pause=pause, evidencia=pdf_vacias).run(plan, backup)
            carpetas.append(backup.parent / state['carpetas'][state['fase']])
    except m.PocError as exc:
        if cancel.is_set():
            raise
        emit('warning', 'No se incluyó completo el calendario de informes por vencer: ' + str(exc))
    finally:
        # La consulta siguiente parte en Seguimiento; si no se puede volver, el aviso de la próxima corrida lo dirá.
        try:
            bridge.call('catalog', {'screen': 'seguimiento'})
        except Exception:
            pass
    return carpetas, len(carpetas) == len(meses)


def run_por_hoja(bridge, catalog, emit, cancel, batch, destination, consolidated=True, **opciones):
    """Un paquete por hoja de RUS (Espera, Cumplimiento, Informes) cuando hay varias: cada hoja se consulta, se lee y se
    entrega por separado, así se puede empezar a trabajar con la primera mientras corre la siguiente y un corte solo
    pierde la hoja en curso. El calendario de informes por vencer se agrega a la hoja de Cumplimiento."""
    if len(batch.tabs) < 2:
        return [run(bridge, catalog, emit, cancel, batch, destination, consolidated, **opciones)]
    resultados, fallas = [], []
    for numero, hoja in enumerate(batch.tabs, 1):
        if cancel.is_set():
            break
        emit('status', f'Hoja {numero} de {len(batch.tabs)}: {hoja}.')
        parcial = dict(opciones)
        parcial['informes'] = bool(opciones.get('informes')) and hoja == 'Cumplimiento'
        try:
            resultados.append(
                run(bridge, catalog, emit, cancel, replace(batch, tabs=(hoja,)), destination, consolidated, **parcial)
            )
        except m.PocError as exc:
            if cancel.is_set():
                raise
            # Una hoja que falla no impide las siguientes; sus fuentes quedan en su carpeta de respaldo.
            emit('warning', f'La hoja {hoja} no se pudo completar: {exc}')
            fallas.append(exc)
    if fallas and not resultados:
        raise fallas[0]
    return resultados
