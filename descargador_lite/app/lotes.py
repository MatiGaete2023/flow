"""Planificacion y descargas secuenciales (Lite), con una carpeta por lote y nombres descriptivos."""

import base64
import hashlib
from collections import Counter
from dataclasses import dataclass, asdict
from datetime import datetime
import json
from pathlib import Path
import re
import unicodedata
from urllib.parse import urlencode, unquote_plus

import motor as m
from perfiles import Selection, response_profile, selection_profile
from evidencias import screenshot_pdf

MODALITIES = {'1': 'Residencia', '2': 'Ambulatorio', '3': 'FAE', '4': 'DCE'}
TAB_ORDER = ('Espera', 'Cumplimiento', 'Informes', 'Egresados')
ALL_MEASURES = '0'
SCREEN_TITLES = {
    'seguimiento': 'Seguimiento',
    'litigantes': 'Busqueda Lit',
    'calendario_informes': 'Informes por vencer',
    'calendario_medidas': 'Medidas por vencer',
}
MAX_NAME = 150


def safe_name(value):
    value = ''.join(c for c in unicodedata.normalize('NFKD', str(value)) if not unicodedata.combining(c))
    value = re.sub(r'[<>:"/\\|?*\x00-\x1f]', ' ', value)
    return ' '.join(value.replace('.', ' ').split()).rstrip(' .')[:125]


def tribunal_name(value):
    value = safe_name(value)
    value = re.sub(r'^Juzgado de Familia\s+', 'Jdo Familia ', value, flags=re.I)
    value = re.sub(r'^Jgdo\s+L\s+y\s+G\s+de\s+', 'Jdo Letras y Garantia ', value, flags=re.I)
    value = re.sub(r'^Jgdo\s+(?:Letras|L)\s+de\s+', 'Jdo Letras ', value, flags=re.I)
    return value


@dataclass(frozen=True)
class Dates:
    start: str
    end: str

    def __post_init__(self):
        try:
            a = datetime.strptime(self.start, '%d/%m/%Y')
            b = datetime.strptime(self.end, '%d/%m/%Y')
            if a > b or not 1900 <= a.year <= 2100 or not 1900 <= b.year <= 2100:
                raise ValueError()
            if a.strftime('%d/%m/%Y') != self.start or b.strftime('%d/%m/%Y') != self.end:
                raise ValueError()
        except ValueError:
            raise m.PocError('Use fechas válidas DD/MM/AAAA, con Desde anterior o igual a Hasta.') from None

    @property
    def label(self):
        return f'{self.start.replace("/", "-")} al {self.end.replace("/", "-")}'


def report_label(selection, catalog):
    if selection.screen != 'seguimiento':
        return ''
    if selection.tab == 'Informes':
        return catalog['informes'].get(selection.report, '')
    if selection.tab == 'Cumplimiento':
        return catalog.get('medidas', {}).get(selection.report, '') if selection.report else 'Todas las medidas'
    return ''


def query_parts(selection, catalog, dates):
    """Etiquetas de la consulta, en el orden en que aparecen en el nombre del archivo."""
    parts = [SCREEN_TITLES[selection.screen]]
    if selection.screen == 'seguimiento':
        parts += [selection.tab, MODALITIES.get(selection.modality, catalog['modalidades'].get(selection.modality, ''))]
        report = report_label(selection, catalog)
        if report:
            parts.append(report)
    elif selection.screen == 'litigantes':
        parts.append(catalog['estados'][selection.state])
    else:
        parts.append(f'{catalog["meses"][selection.month]} {catalog["anios"][selection.year]}')
    if dates is not None and not selection.screen.startswith('calendario_'):
        parts.append(dates.label)
    return [safe_name(p) for p in parts if safe_name(p)]


def file_base(selection, catalog, dates=None):
    """Ej.: «Seguimiento - Cumplimiento - Ambulatorio - Vencidos - Jdo Familia Mulchen - 01-10-2026 al 31-10-2026»."""
    parts = query_parts(selection, catalog, dates)
    tribunal = tribunal_name(catalog['tribunales'][selection.tribunal])
    head = ' - '.join(parts[: 4 if selection.screen == 'seguimiento' else 2])
    tail = parts[4 if selection.screen == 'seguimiento' else 2 :]
    room = MAX_NAME - len(head) - sum(len(x) + 3 for x in tail) - 3
    tribunal = tribunal[: max(12, room)].rstrip(' .')
    return ' - '.join([head, tribunal, *tail])


@dataclass(frozen=True)
class Batch:
    screen: str
    tribunals: tuple[str, ...]
    modalities: tuple[str, ...] = ()
    tabs: tuple[str, ...] = ()
    informes: tuple[str, ...] = ()
    medidas: tuple[str, ...] = ()
    states: tuple[str, ...] = ()
    months: tuple[str, ...] = ()
    year: str | None = None
    dates: Dates | None = None
    keep_filters: bool = False

    def plan(self, catalog):
        if not self.tribunals:
            raise m.PocError('Marca al menos un tribunal.')
        for values in (self.tribunals, self.modalities, self.tabs, self.informes, self.medidas, self.states, self.months):
            if len(values) != len(set(values)):
                raise m.PocError('El lote tiene opciones repetidas.')
        if self.screen == 'seguimiento':
            if not self.modalities or not self.tabs:
                raise m.PocError('Marca al menos una modalidad y una pestaña.')
            if set(self.modalities) - set(MODALITIES) or set(self.tabs) - set(TAB_ORDER):
                raise m.PocError('Modalidad o pestaña no auditada.')
            if 'Informes' in self.tabs and not self.informes:
                raise m.PocError('Marca al menos un tipo de informe (pestaña Informes).')
            if 'Cumplimiento' in self.tabs and not self.medidas:
                raise m.PocError('Marca al menos un filtro de medidas (pestaña Cumplimiento).')

            def reports(tab):
                if tab == 'Informes':
                    return list(self.informes)
                if tab == 'Cumplimiento':
                    return [None if code == ALL_MEASURES else code for code in self.medidas]
                return [None]

            result = [
                Selection(t, tab, mod, report)
                for t in self.tribunals
                for tab in TAB_ORDER
                if tab in self.tabs
                for mod in self.modalities
                for report in reports(tab)
            ]
        elif self.screen == 'litigantes':
            if not self.states:
                raise m.PocError('Marca al menos un estado.')
            result = [Selection(t, '', '', screen='litigantes', state=s) for t in self.tribunals for s in self.states]
        elif self.screen.startswith('calendario_'):
            if self.dates:
                raise m.PocError('Informes y medidas por vencer usan mes y año, no un rango de fechas.')
            if not self.months or not self.year:
                raise m.PocError('Marca al menos un mes y un año.')
            result = [
                Selection(t, '', '', screen=self.screen, month=mo, year=self.year)
                for t in self.tribunals
                for mo in self.months
            ]
        else:
            raise m.PocError('Consulta no disponible en la versión Lite.')
        for selection in result:
            selection_profile(selection, catalog)
        return result


class ChromeTransport:
    def __init__(self, bridge, profile):
        self.bridge = bridge
        self.profile = profile

    @staticmethod
    def body(result, excel=False):
        try:
            if not isinstance(result, dict) or not isinstance(result.get('body'), str):
                raise ValueError()
            if len(result['body']) > 4 * ((80 * 1024 * 1024 + 2) // 3):
                raise ValueError()
            if not excel and not isinstance(result.get('type'), str):
                raise ValueError()
            data = base64.b64decode(result['body'], validate=True)
            if result['status'] != 200 or not data or len(data) > 80 * 1024 * 1024:
                raise ValueError()
            if not excel and 'html' not in result['type'].lower():
                raise ValueError()
            return data
        except (KeyError, TypeError, ValueError):
            raise m.PocError('Respuesta vacía, vencida o fuera del formato permitido.') from None

    def search(self, pairs, timeout_ms):
        m.query_pairs(urlencode(pairs), self.profile)
        return self.body(self.bridge.call('search', {'pairs': pairs, 'timeout': timeout_ms}))

    def download(self, url, timeout_ms):
        m.validate_export_url(url, self.profile)
        return self.body(self.bridge.call('download', {'url': url, 'timeout': timeout_ms}), True)


def evidence_context(selection, catalog, dates):
    context = [['Tribunal', catalog['tribunales'][selection.tribunal] + ' (código ' + selection.tribunal + ')']]
    context.append(['Consulta', SCREEN_TITLES[selection.screen]])
    if selection.screen == 'seguimiento':
        context.append(['Pestaña', selection.tab])
        context.append(['Modalidad', catalog['modalidades'][selection.modality]])
        report = report_label(selection, catalog)
        if report:
            context.append(['Informe' if selection.tab == 'Informes' else 'Medidas', report])
    elif selection.screen == 'litigantes':
        context.append(['Estado', catalog['estados'][selection.state]])
    else:
        context.append(['Mes', catalog['meses'][selection.month] + ' ' + catalog['anios'][selection.year]])
    if not selection.screen.startswith('calendario_'):
        context.append(['Fechas', dates.start + ' – ' + dates.end if dates else 'Sin filtro de fechas'])
    return [[label, value[:500]] for label, value in context]


class BatchRunner:
    def __init__(self, bridge, catalog, emit, cancel, pause=0.0):
        self.bridge = bridge
        self.catalog = catalog
        self.emit = emit
        self.cancel = cancel
        self.pause = max(0.0, float(pause))

    def cancelled(self):
        if self.cancel.is_set():
            raise m.PocError('Lote cancelado. Se conservan los archivos verificados.')

    def run(self, batch, destination):
        plan = batch.plan(self.catalog)
        if not plan:
            raise m.PocError('El lote no tiene consultas.')
        folder = Path(destination) / m.now().strftime('%Y-%m-%d %H-%M-%S')
        if folder.exists():
            folder = Path(destination) / m.now().strftime('%Y-%m-%d %H-%M-%S-%f')
        folder.mkdir(parents=True, exist_ok=False)
        self.emit('folder', str(folder))
        self.emit('plan', len(plan))
        bases = Counter(file_base(s, self.catalog, batch.dates) for s in plan)
        used = Counter()
        completed = []
        state = 'EN_CURSO'

        def unique(base, extension, page=None, total=1):
            name = base
            if bases[base] > 1:
                used[base] += 1
                name += f' ({used[base]})'
            if total > 1:
                name += f' - p{page} de {total}'
            return name + extension

        planned = [
            {
                **asdict(s),
                'tribunal_nombre': self.catalog['tribunales'][s.tribunal],
                'etiquetas': query_parts(s, self.catalog, batch.dates),
            }
            for s in plan
        ]

        def summary():
            payload = {
                'version': 'lite-1',
                'fecha_hora': m.now().isoformat(),
                'estado': state,
                'consultas_esperadas': len(plan),
                'consultas_completadas': len(completed),
                'filtro_fecha': None if batch.dates is None else asdict(batch.dates),
                'otros_filtros': 'formulario SITFA' if batch.keep_filters else 'todos los registros',
                'consultas': completed,
                'plan': planned,
            }
            temp = folder / 'resumen.tmp'
            temp.write_text(json.dumps(payload, indent=2, ensure_ascii=False), encoding='utf-8')
            temp.replace(folder / 'resumen.json')

        summary()
        locked = False
        try:
            self.bridge.call('lock')
            locked = True
            for index, selection in enumerate(plan, 1):
                self.cancelled()
                if index > 1 and self.pause and self.cancel.wait(self.pause):
                    self.cancelled()
                desired = selection_profile(selection, self.catalog)
                base = file_base(selection, self.catalog, batch.dates)
                self.emit('query', {'index': index, 'total': len(plan), 'label': base})
                dates = None if selection.screen.startswith('calendario_') else batch.dates
                prepared = self.bridge.call(
                    'prepare',
                    {
                        'selection': selection.__dict__,
                        'dates': None if dates is None else dates.__dict__,
                        'keep_filters': batch.keep_filters and not selection.screen.startswith('calendario_'),
                    },
                )
                try:
                    wire_pairs = prepared['pairs']
                    if (
                        not isinstance(wire_pairs, list)
                        or len(wire_pairs) > 100
                        or any(
                            not isinstance(pair, (list, tuple))
                            or len(pair) != 2
                            or any(not isinstance(value, str) for value in pair)
                            for pair in wire_pairs
                        )
                    ):
                        raise ValueError()
                    pairs = m.query_pairs(urlencode([tuple(pair) for pair in wire_pairs]), desired)
                except (KeyError, TypeError, ValueError):
                    raise m.PocError('El formulario no entregó parámetros válidos.') from None
                fields = dict(pairs)
                if not selection.screen.startswith('calendario_'):
                    if fields.get('FLG_Consulta') != ('1' if dates else '0'):
                        raise m.PocError('No se pudo activar correctamente el filtro de fecha.')
                    if dates and (
                        unquote_plus(fields.get('FEC_Inicio', '')) != dates.start
                        or unquote_plus(fields.get('FEC_Fin', '')) != dates.end
                        or fields.get('CHK_Consulta') != 'on'
                    ):
                        raise m.PocError('Las fechas enviadas no coinciden con la selección.')
                transport = ChromeTransport(self.bridge, desired)
                profile, first = response_profile(transport.search(pairs, 60000), pairs, selection, self.catalog)
                manifest = f'verificacion {index:03}.json'
                evidence = None
                if first is None:
                    self.cancelled()
                    evidence = self.capture_empty(
                        selection, profile, pairs, dates, folder, unique(base, ' - SIN REGISTROS.pdf')
                    )
                    m.write_manifest(folder, [], 0, 'SIN_RESULTADOS', profile, manifest)
                    self.emit('evidence', evidence)
                    result = []
                else:
                    self.emit('total', first.total)
                    transport.profile = profile
                    names = {}

                    def filename(number, total):
                        if not names:
                            names['base'] = unique(base, '')
                        return names['base'] + (f' - p{number} de {total}' if total > 1 else '') + '.xls'

                    result = m.run_sequence(
                        transport,
                        pairs,
                        first,
                        folder,
                        profile=profile,
                        cancel=self.cancel,
                        pause=self.pause,
                        folder_ready=True,
                        manifest_name=manifest,
                        filename_factory=filename,
                        progress=lambda item, path: self.emit('progress', {**item, 'consulta': base}),
                    )
                completed.append(
                    {
                        'consulta': base,
                        'estado': 'VALIDADA' if result else 'SIN_RESULTADOS',
                        'paginas': len(result),
                        'registros': sum(x['registros'] for x in result),
                        'archivos': [x['archivo'] for x in result] or [evidence['archivo']],
                        'verificacion': manifest,
                        'seleccion': planned[index - 1],
                    }
                )
                info = json.loads((folder / manifest).read_text(encoding='utf-8'))
                info['seleccion'] = planned[index - 1]
                info['filtro_fecha'] = None if dates is None else asdict(dates)
                if evidence:
                    info['evidencia'] = evidence
                temp = folder / (manifest + '.tmp')
                temp.write_text(json.dumps(info, indent=2, ensure_ascii=False), encoding='utf-8')
                temp.replace(folder / manifest)
                summary()
                self.emit('query_done', completed[-1])
            state = 'VALIDADA'
            summary()
            self.emit(
                'done',
                {
                    'folder': str(folder),
                    'queries': len(completed),
                    'files': len(list(folder.glob('*.xls'))),
                    'pdfs': len(list(folder.glob('*.pdf'))),
                    'records': sum(x['registros'] for x in completed),
                },
            )
            return completed
        except BaseException:
            state = 'INCOMPLETA'
            summary()
            raise
        finally:
            if locked:
                try:
                    restored = self.bridge.call('unlock', timeout=8)
                    if isinstance(restored, dict) and restored.get('restored') is False:
                        self.emit('warning', 'No se restauraron todos los filtros de SITFA. Revisa el formulario.')
                except Exception:
                    self.emit('warning', 'No se pudo confirmar la restauración de los filtros de SITFA.')

    def capture_empty(self, selection, profile, pairs, dates, folder, name):
        """Consulta sin registros: pantallazo de SITFA con la selección visible, guardado como PDF."""
        self.emit('status', 'Consulta sin registros: capturando pantallazo…')
        context = evidence_context(selection, self.catalog, dates)
        try:
            native = self.bridge.call('evidence_open', {'pairs': pairs, 'label': profile.label, 'context': context})
            native_html = ChromeTransport.body(native)
            digest = hashlib.sha256(native_html).hexdigest()
            if native.get('digest') != digest:
                raise m.PocError('La pantalla de evidencia no coincide con la respuesta recibida.')
            _, native_first = response_profile(native_html.decode('utf-8'), pairs, selection, self.catalog)
            if native_first is not None:
                raise m.PocError('La consulta cambió y ahora tiene registros. Repite el lote; no se guardó un PDF vacío.')
            native_root = m.root_html(native_html.decode('utf-8'))
            banners = native_root.xpath('//*[@id="sitfaContextoConsulta"]')
            context_text = 'Consulta solicitada · ' + ' · '.join(label + ': ' + value for label, value in context)
            if (
                len(banners) != 1
                or banners[0].get('data-context') != json.dumps(context, ensure_ascii=False, separators=(',', ':'))
                or banners[0].text_content() != context_text
            ):
                raise m.PocError('La pantalla no identifica la consulta solicitada. Recarga la extensión Lite.')
            native_fields = m.fields_from_form(native_root.xpath('//form[@name=$name]', name=profile.form_name)[0])
            if any(native_fields.get(k) != v for k, v in profile.target.items()):
                raise m.PocError('Los controles de la pantalla a capturar no corresponden a la selección.')
            if dates and any(
                unquote_plus(native_fields.get(k, '')) != v
                for k, v in zip(('FEC_Inicio', 'FEC_Fin'), (dates.start, dates.end))
            ):
                raise m.PocError('Las fechas de la pantalla a capturar no corresponden al lote.')
            self.cancelled()
            capture = self.bridge.call('evidence_capture', {'digest': digest})
            pdf, checksum = screenshot_pdf(capture, digest, profile.label)
            path = folder / name
            m.save_exclusive(path, pdf)
            if hashlib.sha256(path.read_bytes()).hexdigest() != checksum:
                path.unlink(missing_ok=True)
                raise m.PocError('No se pudo verificar la escritura del PDF.')
            return {
                'archivo': name,
                'registros': 0,
                'sha256': checksum,
                'tipo': 'PDF_PANTALLAZO_SIN_REGISTROS',
                'contexto_consulta': context,
            }
        finally:
            try:
                self.bridge.call('evidence_close', timeout=8)
            except Exception:
                pass
