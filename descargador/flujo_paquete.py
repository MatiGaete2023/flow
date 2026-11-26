"""Orquesta runners existentes; conserva fuentes y entrega un ZIP sin pasos manuales."""

from dataclasses import asdict, is_dataclass, replace
from datetime import date
import json
from pathlib import Path
from shutil import copy2
from tempfile import TemporaryDirectory

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
    historico=False,
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
    config = state.setdefault(
        'opciones',
        {
            'consolidado': consolidated,
            'calendarios': calendars,
            'historia': historia,
            'historico': historico,
            'historia_en_paquete': historia_en_paquete,
            'meses': meses,
            'informes': informes,
            'fecha_informes': (today or m.now().date()).isoformat(),
            'pdf_vacias': pdf_vacias,
        },
    )
    _validar_opciones(config)
    backup.mkdir(parents=True, exist_ok=bool(resume))
    if state.get('historial_version') != 2:
        state['historial'] = []
        state['historial_version'] = 2
    anotar(state, f"inicio; reintento {state.get('reintentos', 0)}" if resume else 'inicio')
    write(control, state)
    emit('pipeline_folder', str(root))
    anterior = {fase: root / carpeta for fase, carpeta in state['carpetas'].items()} if resume else {}

    def checkpoint():
        if cancel.is_set():
            raise m.PocError('Proceso detenido. Los originales verificados se conservan en respaldo.')

    def phase(name):
        checkpoint()
        state['fase'] = name
        anotar(state, 'fase ' + name)
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
            anotar(state, 'consultas reutilizadas del intento anterior')
            emit('status', 'Consultas ya validadas en el intento anterior: se reutilizan.')
        else:
            state['carpetas'].pop('consultas', None)
            BatchRunner(bridge, catalog, relay, cancel, pause=pause, evidencia=config['pdf_vacias']).run(batch, backup)
        if 'consultas' not in state['carpetas']:
            raise m.PocError('La consulta no identificó sus originales; no se creó el paquete.')
        phase('bitacoras')
        previas = anterior.get('bitacoras')
        previas = previas if previas and (previas / MANIFEST).is_file() else None
        reuse_base = Path(destination) if reuse and not resume else None
        lector = None
        if config['historia']:
            from lectura_historia import LectorHistoria

            historia_previa = previas / 'historia' / 'historias.json' if previas else None
            lector = LectorHistoria(
                bridge,
                relay,
                cancel,
                pause=pause,
                meses=config['meses'],
                **({'anterior': historia_previa} if historia_previa and historia_previa.is_file() else {}),
            )
        extra = {'historia': lector} if lector else {}
        # Cada lectura opcional se pide solo si está activa, como la historia.
        if config['calendarios']:
            extra['calendars'] = True
        if config['historico']:
            extra['historico'] = True  # P3c (D-H1): Ingreso Histórico Protección por persona del listado
        runner = DiaryRunner(bridge, catalog, relay, cancel, pause=pause, reuse_base=reuse_base, **extra)
        # Al retomar, el mismo lote de bitácoras continúa: lo ya leído (hash comprobado) no se vuelve a abrir.
        diaries = runner.run(batch, backup, previas) if previas else runner.run(batch, backup)
        anotar(
            state,
            f"bitácoras {diaries.get('estado')}; leídas {diaries.get('leidas', '?')}; no leídas {diaries.get('fallidas', 0)}; "
            f"consultas fallidas {diaries.get('consultas_fallidas', 0)}"
            + (f"; históricos {diaries.get('historicos')}" if diaries.get('historicos') else ''),
        )
        from lectura_bitacoras import aviso_faltantes

        # P8: qué bitácoras faltan, por tribunal, en el historial y como aviso visible al terminar.
        faltantes = diaries.get('faltantes_por_tribunal') or {}
        for aviso in aviso_faltantes(faltantes):
            emit('warning', aviso)
        por_codigo = {codigo: {**conteos, 'nombre': ''} for codigo, conteos in faltantes.items()
                      if codigo.isascii() and codigo.isdigit()}
        for aviso in aviso_faltantes(por_codigo):
            anotar(state, aviso)
        write(control, state)
        carpetas_informes, informes_completos = _informes(
            bridge, relay, emit, cancel, batch, backup, state, phase, pause, config['informes'],
            date.fromisoformat(config['fecha_informes']), config['pdf_vacias'],
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
        if lector and config['historia_en_paquete'] and lector.carpeta:
            candidata = lector.carpeta / 'historias.json'
            if candidata.is_file() and candidata.resolve().is_relative_to(backup.resolve()):
                ruta_historia = candidata
        target = root / 'Captura_CSMP.zip'
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
        if config['meses'] != 4:
            opciones['meses'] = config['meses']
        if config['historico']:
            opciones['historico'] = manifest.parent
        with TemporaryDirectory(prefix='.captura-', dir=backup) as temporal:
            nuevo = Path(temporal) / target.name
            try:
                result = package_current_lot(
                    sheets,
                    manifest,
                    nuevo,
                    config['consolidado'],
                    **opciones,
                    **({'historia': ruta_historia} if ruta_historia else {}),
                )
            except (m.PocError, ValueError, OSError):
                if not ruta_historia:
                    raise
                emit('warning', 'La historia no pudo incluirse en el paquete; las bitácoras se conservan.')
                historia_ok = False
                nuevo.unlink(missing_ok=True)
                result = package_current_lot(sheets, manifest, nuevo, config['consolidado'], **opciones)
            checkpoint()
            if target.exists():
                copia = Path(temporal) / 'anterior.zip'
                copy2(target, copia)
                copia.replace(backup / ('Captura_CSMP anterior ' + m.now().strftime('%Y-%m-%d %H-%M-%S-%f') + '.zip'))
            nuevo.replace(target)
            result['path'] = str(target)
        historico_fallido = config['historico'] and result.get('historico_incompleto', True)
        if historico_fallido:
            emit('warning', 'Quedaron lecturas del Ingreso Histórico Protección sin verificar. El paquete queda parcial.')
        partial = diaries['estado'] != 'COMPLETA' or bool(diaries['fallidas'] or diaries['consultas_fallidas'])
        # Un calendario por ingreso (WD-1) fallido o pendiente también deja el paquete parcial.
        partial = partial or bool(diaries.get('calendarios_fallidos'))
        # Sin el calendario (o con un mes faltante) el paquete se crea igual, pero queda marcado como parcial.
        partial = partial or not informes_completos or historico_fallido
        if config['historia']:
            partial = partial or not historia_ok
            result['historia'] = resumen or {'estado': 'INCOMPLETA'}
        state.update(estado='PARCIAL' if partial else 'COMPLETO', paquete=Path(result['path']).name)
        anotar(state, f"fin {state['estado']}" + ('' if informes_completos else '; calendario de informes incompleto'))
        write(control, state)
        result.update(sheets_folder=str(sheets), diary_manifest=str(manifest), partial=partial)
        emit('package_done', result)
        return result
    except BaseException as exc:
        state['estado'] = 'INCOMPLETO'
        codigo = 'ERROR_ARCHIVO' if isinstance(exc, OSError) else 'ERROR_OPERACION'
        anotar(state, f"error en fase {state['fase']} ({codigo})")
        write(control, state)
        raise


def _validar_opciones(config):
    booleanas = ('consolidado', 'calendarios', 'historia', 'historico', 'historia_en_paquete', 'informes', 'pdf_vacias')
    try:
        if not isinstance(config, dict) or any(type(config.get(k)) is not bool for k in booleanas):
            raise ValueError
        if type(config.get('meses')) is not int or not 1 <= config['meses'] <= 4:
            raise ValueError
        date.fromisoformat(config.get('fecha_informes'))
    except (TypeError, ValueError):
        raise m.PocError('Las opciones guardadas del paquete no son válidas.') from None


def anotar(state, evento):
    """Conserva los últimos 200 eventos de vocabulario controlado."""
    texto = ' '.join(evento.split())[:500]
    historial = state.setdefault('historial', [])
    historial.append(m.now().astimezone().isoformat(timespec='seconds') + ' ' + texto)
    del historial[:-200]


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
    from resultados import load_lot

    if carpeta is None:
        return False
    try:
        load_lot(carpeta)
    except (m.PocError, OSError, ValueError, TypeError, AttributeError, KeyError):
        return False
    return True


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
    Devuelve (carpetas, completo): conserva los meses verificados y reintenta solo los que falten."""
    if not informes or 'Cumplimiento' not in getattr(batch, 'tabs', ()):
        return [], True
    from lotes import Batch

    phase('informes')
    meses = {f'informes-{year}-{month}': (month, year) for month, year in meses_informes(today or m.now().date())}
    carpetas = {}
    for clave in meses:
        carpeta = state['carpetas'].get(clave)
        if carpeta and _consultas_validadas(backup.parent / carpeta):
            carpetas[clave] = backup.parent / carpeta
    pendientes = [clave for clave in meses if clave not in carpetas]
    if not pendientes:
        return list(carpetas.values()), True

    try:
        catalog = bridge.call('catalog', {'screen': 'calendario_informes'})
        for clave in pendientes:
            if cancel.is_set():
                raise m.PocError('Proceso detenido. Los originales verificados se conservan en respaldo.')
            month, year = meses[clave]
            state['fase'] = clave
            state['carpetas'].pop(clave, None)
            plan = Batch(batch.tribunals, (), (), screen='calendario_informes', month=month, year=year)
            try:
                BatchRunner(bridge, catalog, relay, cancel, pause=pause, evidencia=pdf_vacias).run(plan, backup)
                carpeta = state['carpetas'].get(clave)
                if not carpeta:
                    raise m.PocError('La consulta no identificó sus originales.')
                carpetas[clave] = backup.parent / carpeta
            except m.PocError as exc:
                if cancel.is_set():
                    raise
                emit('warning', f'No se incluyeron los informes por vencer de {month}/{year}. {exc}')
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
    return [carpetas[clave] for clave in meses if clave in carpetas], len(carpetas) == len(meses)


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
