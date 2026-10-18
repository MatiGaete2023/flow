"""Meta 4 a partir de los lotes del Descargador SITFA: registros y bitácoras de un mes móvil.

No se conecta a SITFA ni a RUS: solo lee carpetas ya descargadas."""
from __future__ import annotations
import argparse
import calendar
from collections import Counter, defaultdict
from collections.abc import Mapping
from datetime import date, datetime
import hashlib
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import sys
from dataclasses import dataclass, field, replace

BASE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(BASE / 'descargador'))
sys.path.insert(0, str(BASE / 'src'))
import generate_meta4 as meta
from nurus.bitacora_html import read_snapshot
import pandas as pd
from openpyxl import Workbook
from openpyxl.comments import Comment
from openpyxl.styles import Font, PatternFill
from openpyxl.utils import get_column_letter

VERSION = '2.2'
ALERTA = 'ALERTA: última observación en Espera; registro en Cumplimiento'
EXTRA = ['ETAPA ÚLTIMA OBS.', 'CONTROL BITÁCORA', 'ALERTA']
VERIFICADA = 'VERIFICADA EN EL PERÍODO'
ROJO = 'FFC7CE'
AMARILLO = 'FFF2CC'
SALIDAS = ('Resultado META4', 'Salida META4')
EN_CURSO = '.en-curso-'
ALIAS_COLUMNAS = {'RUT RCEI': 'RUT', 'NOMBRE MENOR': 'NOMBRE'}
IDENTIDAD = ('tribunal_codigo', 'causa_id', 'ingreso_id')
NO_COMPLETADA = 'LECTURA NO COMPLETADA'
NO_VERIFICABLE = 'BITÁCORA NO VERIFICABLE'
IDENTIDAD_VISIBLE = 'REVISAR IDENTIDAD VISIBLE'
LEIDA_ANTES = 'REVISAR: bitácora leída antes de la fecha de corte'
BLOQUEANTES = (NO_VERIFICABLE, IDENTIDAD_VISIBLE)
CONSULTA_CUBIERTA = ('ENUMERADA', 'SIN_REGISTROS')


def periodo(corte):
    year, month = (corte.year - 1, 12) if corte.month == 1 else (corte.year, corte.month - 1)
    return date(year, month, min(corte.day, calendar.monthrange(year, month)[1])), corte


def fecha(value):
    # Toda entrada nace de bitacora_html._date(...).isoformat(): otro formato es un JSON alterado.
    try:
        return datetime.fromisoformat(str(value or '').strip().replace('Z', '+00:00')).replace(tzinfo=None)
    except ValueError:
        raise ValueError('Fecha de bitácora no reconocida.') from None


def etapa(entry):
    # La etapa se toma de la columna de la bitácora, nunca del texto libre.
    value = meta.norm(entry.get('etapa'))
    if value in ('ESPERA', 'EN ESPERA', 'LISTA DE ESPERA', 'LISTA ESPERA'):
        return 'Espera'
    if value in ('CUMPLIMIENTO', 'EN CUMPLIMIENTO'):
        return 'Cumplimiento'
    return None


def entradas_periodo(entries, corte):
    inicio, fin = periodo(corte)
    return [e for e in entries if inicio <= fecha(e['fecha']).date() <= fin]


def ultima(entries, corte):
    valid = [(fecha(e['fecha']), etapa(e)) for e in entradas_periodo(entries, corte) if etapa(e)]
    if not valid:
        return None, '', 'SIN OBSERVACIÓN DE ESPERA/CUMPLIMIENTO EN EL MES'
    last = max(item[0] for item in valid)
    stages = {s for d, s in valid if d == last}
    if len(stages) > 1:
        return last.date(), '', 'REVISAR: etapas distintas en la misma fecha y hora'
    return last.date(), stages.pop(), VERIFICADA


def dentro(folder, name):
    target = (folder / name).resolve()
    if not target.is_relative_to(folder.resolve()):
        raise ValueError('Referencia de bitácora fuera de la carpeta de origen.')
    return target


def es_salida(path, origen):
    """Carpetas de resultados (terminados o en curso) nunca son fuentes."""
    return any(part.startswith(SALIDAS) or EN_CURSO in part for part in path.relative_to(origen).parts)


@dataclass
class Lectura:
    """Bitácoras leídas desde uno o más lotes del Descargador."""
    registros: list = field(default_factory=list)
    estados: list = field(default_factory=list)
    cubiertas: set = field(default_factory=set)
    declara_alcance: bool = False
    avisos: list = field(default_factory=list)


def dia_captura(value):
    """Día local de la lectura. La extensión registra la hora en UTC."""
    moment = datetime.fromisoformat(str(value).replace('Z', '+00:00'))
    return (moment.astimezone() if moment.tzinfo else moment).date()


def archivos_bitacoras(root):
    paths = [p for p in sorted(root.rglob('bitacoras.json')) if not es_salida(p, root)]
    # Formato propio de Meta 4 1.0/1.1: se sigue aceptando para lotes ya generados.
    return paths + [p for p in sorted(root.rglob('bitacoras_meta4.json')) if not es_salida(p, root)]


def _consulta(item):
    return str(item.get('tribunal', '')), str(item.get('tab', '')), str(item.get('modality', ''))


def _leer_bitacoras(root, corte):
    """Registros de bitácora del período, con el estado y el alcance de cada lote de lectura."""
    lectura = Lectura()
    paths = archivos_bitacoras(root)
    if not paths:
        raise ValueError('Faltan bitácoras. Selecciona la carpeta de bitácoras del Descargador (bitacoras.json y sus HTML).')
    inicio = periodo(corte)[0]
    for path in paths:
        if path.stat().st_size > 64 * 1024 * 1024:
            raise ValueError('Un lote de bitácoras excede 64 MB; revisa la carpeta elegida.')
        data = json.loads(path.read_text(encoding='utf-8'))
        own = data.get('tipo') == 'META4_BITACORAS_MES'
        if own and (inicio < date.fromisoformat(data['desde']) or corte > date.fromisoformat(data['hasta'])):
            raise ValueError('Las bitácoras guardadas no cubren este período. Usa su fecha de corte o vuelve a descargarlas.')
        if not own and (data.get('tipo') != 'BITACORAS_LECTURA' or data.get('version') != 1):
            raise ValueError('Formato de bitácoras no reconocido.')
        lectura.estados.append(str(data.get('estado') or 'SIN ESTADO'))
        if 'consultas' in data:
            lectura.declara_alcance = True
            lectura.cubiertas.update(_consulta(q) for q in data['consultas'] if q.get('estado') in CONSULTA_CUBIERTA)
        seleccion = data.get('seleccion') or {}
        if seleccion.get('dates') or seleccion.get('keep_filters'):
            lectura.avisos.append(f'El lote de bitácoras {path.parent.name} usó filtros de fecha o del formulario; '
                                  'puede no incluir todos los ingresos.')
        # Un lote retomado conserva lecturas previas aún no reenumeradas: también valen.
        for original in data.get('registros', []) + data.get('anteriores_no_reenumeradas', []):
            record = dict(original)
            record['entries'] = []
            record['control'] = NO_COMPLETADA
            if record.get('estado') == 'LEIDA':
                try:
                    if own:
                        snapshot = record['snapshot']
                    else:
                        body = dentro(path.parent, record['archivo']).read_bytes()
                        if hashlib.sha256(body).hexdigest() != record['sha256']:
                            raise ValueError('La bitácora cambió después de su descarga.')
                        snapshot = read_snapshot(body, record['params'])
                    if tuple(str(snapshot[k]) for k in IDENTIDAD) != tuple(str(record[k]) for k in IDENTIDAD):
                        raise ValueError('La bitácora pertenece a otro ingreso.')
                    leida = dia_captura(record['capturada'])
                    record['entries'] = entradas_periodo(snapshot['entries'], corte)
                    if record.get('advertencias'):
                        record['control'] = IDENTIDAD_VISIBLE
                    elif leida < corte:
                        # Lo escrito entre la lectura y el corte no está en la bitácora guardada.
                        record['control'] = LEIDA_ANTES
                    else:
                        record['control'] = 'OK'
                except (KeyError, ValueError, OSError, TypeError):
                    record['control'] = NO_VERIFICABLE
            lectura.registros.append(record)
    return lectura


def cargar_bitacoras(root, corte):
    return _leer_bitacoras(root, corte).registros


def fecha_lectura(root):
    """Último día con lecturas en los lotes de bitácoras; sirve para proponer la fecha de corte."""
    days = []
    for path in archivos_bitacoras(Path(root)):
        try:
            data = json.loads(path.read_text(encoding='utf-8'))
            for record in data.get('registros', []) + data.get('anteriores_no_reenumeradas', []):
                if record.get('estado') == 'LEIDA' and record.get('capturada'):
                    days.append(dia_captura(record['capturada']))
        except (OSError, ValueError, TypeError, AttributeError):
            continue
    return max(days) if days else None


def normal(value):
    return meta.norm(value)


def _tribunal(value):
    return meta.tribunal_name(value) or normal(value)


def _rut(value):
    return normal(value).replace(' ', '')


def indexar_bitacoras(diaries):
    """Índice por (tribunal, RIT): cada fila compara solo con sus bitácoras posibles."""
    index = defaultdict(list)
    for diary in diaries:
        index[(_tribunal(diary.get('tribunal_nombre')), normal(diary.get('rit')))].append(diary)
    return index


def buscar(row, diaries):
    index = diaries if isinstance(diaries, Mapping) else indexar_bitacoras(diaries)
    values = row.values
    rut = _rut(values.get('RUT'))
    nombre = normal(values.get('NOMBRE'))
    programa = normal(values.get('DERIVACIÓN'))
    remote = values.get('_REMOTE')
    candidates = []
    for diary in index.get((_tribunal(values.get('TRIBUNAL')), normal(values.get('RIT'))), ()):
        if rut:
            if _rut(diary.get('rut')) != rut:
                continue
        elif not nombre or normal(diary.get('nombre')) != nombre:
            continue
        if not programa or normal(diary.get('programa')) != programa:
            continue
        if remote and tuple(str(diary.get(k, '')) for k in IDENTIDAD) != remote:
            continue
        candidates.append(diary)
    identities = {tuple(r.get(k) for k in IDENTIDAD) for r in candidates}
    if len(identities) != 1 or not all(next(iter(identities), ())):
        return None, 'IDENTIDAD AMBIGUA' if candidates else 'SIN BITÁCORA ASOCIADA'
    # Un archivo alterado o una identidad visible distinta siempre detienen la asociación.
    blocking = next((r['control'] for r in candidates if r.get('control') in BLOQUEANTES), None)
    if blocking:
        return None, blocking
    # La misma identidad puede venir de ambas pestañas o de varios lotes (p. ej. por el límite
    # de lecturas): una lectura fallida en un lote no anula la lectura verificada de otro.
    read = [r for r in candidates if r.get('control') in ('OK', LEIDA_ANTES)]
    if not read:
        return None, candidates[0].get('control', NO_COMPLETADA)
    newest = max(read, key=lambda r: str(r.get('capturada', '')))
    if newest['control'] != 'OK':
        return None, newest['control']
    return newest, 'OK'


def enriquecer(rows, diaries, corte):
    index = indexar_bitacoras(diaries)
    for group in rows.values():
        for row in group:
            diary, control = buscar(row, index)
            day, stage = None, ''
            if diary:
                day, stage, control = ultima(diary['entries'], corte)
            row.values['FECHA OBS.'] = day
            row.values[EXTRA[0]] = stage
            row.values[EXTRA[1]] = control
            row.values['ALERTA'] = ALERTA if row.status == 'Cumplimiento' and stage == 'Espera' else ''


def requiere_revision(values):
    return bool(values['ALERTA']) or values[EXTRA[1]] != VERIFICADA


def tabla(ws, headers, records, formatos=None):
    """Hoja simple. `formatos` fija el formato numérico de columnas por encabezado."""
    formatos = formatos or {}
    ws.append(headers)
    header_fill = PatternFill('solid', fgColor='1F4E78')
    header_font = Font(color='FFFFFF', bold=True)
    for cell in ws[1]:
        cell.fill = header_fill
        cell.font = header_font
    ws.freeze_panes = 'A2'
    for index in range(1, len(headers) + 1):
        ws.column_dimensions[get_column_letter(index)].width = 24
    alert_col = headers.index('ALERTA') if 'ALERTA' in headers else None
    column_formats = [formatos.get(h) for h in headers]
    memo = meta.StyleMemo()
    alert_fill = PatternFill('solid', fgColor=ROJO)
    for row_idx, record in enumerate(records, 2):
        row = [ws.cell(row_idx, col, meta.excel_value(record.get(h))) for col, h in enumerate(headers, 1)]
        alert = alert_col is not None and bool(row[alert_col].value)
        for cell, forced in zip(row, column_formats):
            meta.guard_text(cell)
            number_format = forced or ('dd-mm-yyyy' if isinstance(cell.value, (date, datetime)) else None)
            if not number_format and not alert:
                continue

            def setter(target, number_format=number_format, alert=alert):
                if number_format:
                    target.number_format = number_format
                if alert:
                    target.fill = alert_fill

            memo.apply(cell, (number_format, alert), setter)
    ws.auto_filter.ref = f'A1:{get_column_letter(len(headers))}{len(records) + 1}'


RESUMEN = ['TRIBUNAL', 'FAE ESPERA', 'FAE CUMPLIMIENTO', 'RESIDENCIA ESPERA', 'RESIDENCIA CUMPLIMIENTO',
           'TOTAL REGISTROS', 'ALERTAS', 'REVISAR BITÁCORA', 'SIN FECHA OBS.', 'CONSULTAS SIN BITÁCORAS']
SIN_TRIBUNAL = 'SIN TRIBUNAL RECONOCIDO'


def tribunal_de(row):
    return row.tribunal or meta.tribunal_name(row.values.get('TRIBUNAL')) or SIN_TRIBUNAL


def resumen_tribunales(rows, faltan=(), tribunales=None):
    """Conteos por tribunal: registros por lista, alertas, filas por revisar y consultas sin bitácoras.

    `faltan` son pares (etiqueta, tribunal). Se listan los tribunales del filtro o los que tienen datos."""
    counts = defaultdict(Counter)
    for status, group in rows.items():
        for r in group:
            c = counts[tribunal_de(r)]
            c[f'{(r.category or "").upper()} {status.upper()}'] += 1
            c['TOTAL REGISTROS'] += 1
            c['ALERTAS'] += bool(r.values['ALERTA'])
            c['REVISAR BITÁCORA'] += r.values[EXTRA[1]] != VERIFICADA
            c['SIN FECHA OBS.'] += r.values.get('FECHA OBS.') is None
    for _, tribunal in faltan:
        counts[tribunal or SIN_TRIBUNAL]['CONSULTAS SIN BITÁCORAS'] += 1
    order = [t for t in meta.TRIBUNALS if (tribunales is None and t in counts) or (tribunales and t in tribunales)]
    order += [t for t in counts if t not in meta.TRIBUNALS]
    result = [dict({h: counts[t][h] for h in RESUMEN[1:]}, TRIBUNAL=t) for t in order]
    total = Counter()
    for item in result:
        total.update({h: item[h] for h in RESUMEN[1:]})
    result.append(dict({h: total[h] for h in RESUMEN[1:]}, TRIBUNAL='TOTAL'))
    return result


def consolidado(path, rows, resumen=None):
    wb = Workbook()
    wb.remove(wb.active)
    if resumen is not None:
        ws = wb.create_sheet('Resumen por tribunal')
        tabla(ws, RESUMEN, resumen)
        ws.column_dimensions['A'].width = 30
        for cell in ws[ws.max_row]:
            cell.font = Font(bold=True)
    for status in ('Espera', 'Cumplimiento'):
        tabla(wb.create_sheet(status), meta.SOURCE_HEADERS[status] + EXTRA, [r.values for r in rows[status]])
    groups = defaultdict(list)
    for status, group in rows.items():
        for r in group:
            key = (_rut(r.values.get('RUT')), normal(r.values.get('NOMBRE')))
            if all(key):
                groups[key].append(dict(r.values, UBICACIÓN=status))
    duplicates = [r for group in groups.values() if len(group) > 1 for r in group]
    titles = ['UBICACIÓN'] + meta.ESPERA_HEADERS + meta.CUMPLIMIENTO_HEADERS + EXTRA
    canonical = {}
    for h in titles:
        canonical.setdefault(normal(h), h)
    headers = list(canonical.values())

    def unified(records):
        return [{canonical.get(normal(k), k): v for k, v in record.items()} for record in records]

    tabla(wb.create_sheet('Duplicados'), headers, unified(duplicates))
    alert_rows = [dict(r.values, UBICACIÓN=status) for status, group in rows.items() for r in group if requiere_revision(r.values)]
    tabla(wb.create_sheet('Alertas y revisión'), headers, unified(alert_rows))
    meta.save_workbook_atomic(wb, path)


def marcar_planilla(wb, records, rows, tribunales=None):
    # Mismo orden por modalidad y tribunal que read_registros/rebuild; no se coteja solo por RIT.
    flagged = defaultdict(list)
    for status, group in rows.items():
        for r in group:
            sheet = f'{r.category} {status}'
            if sheet in records and r.tribunal in records[sheet]:
                flagged[(sheet, r.tribunal)].append(r)
    fills = {color: PatternFill('solid', fgColor=color) for color in (ROJO, AMARILLO)}
    for name in meta.SHEET_DEFS:
        ws = wb[name]
        pos = 1
        for court in [t for t in meta.TRIBUNALS if tribunales is None or t in tribunales]:
            pos += 4
            source = flagged[(name, court)]
            written = records[name][court]
            # Control de alineación: la marca solo se aplica si cada fila coincide con su RIT escrito.
            if len(source) != len(written) or any(
                normal(r.values.get('RIT')) != normal(rec[0]) for r, rec in zip(source, written)
            ):
                raise ValueError(f'No se pudo alinear la marca de bitácoras con la planilla ({name}, {court}).')
            if written:
                pos += 1
                for r in source:
                    if requiere_revision(r.values):
                        alert = r.values['ALERTA']
                        ws.cell(pos, 8).comment = Comment(alert or r.values[EXTRA[1]], 'META 4')
                        fill = fills[ROJO if alert else AMARILLO]
                        for cell in ws[pos][:8]:
                            cell.fill = fill
                    pos += 1
            else:
                pos += 1
            pos += 1


def exportar_bitacoras(path, diaries):
    wb = Workbook()
    wb.remove(wb.active)
    observations, controls = [], []
    for d in diaries:
        common = {'TRIBUNAL': d.get('tribunal_nombre'), 'RIT': d.get('rit'), 'RUT': d.get('rut'),
                  'NOMBRE': d.get('nombre'), 'PROGRAMA': d.get('programa'),
                  'CAUSA ID': d.get('causa_id'), 'INGRESO ID': d.get('ingreso_id')}
        controls.append(dict(common, LECTURA=d.get('estado'), CONTROL=d.get('control')))
        for e in d['entries']:
            observations.append(dict(common, **{'FECHA CENTRO': fecha(e['fecha']), 'ETAPA': e.get('etapa'),
                                               'TIPO OBSERVACIÓN': e.get('tipo'), 'OBSERVACIÓN CENTRO': e.get('texto')}))
    headers = ['TRIBUNAL', 'RIT', 'RUT', 'NOMBRE', 'PROGRAMA', 'CAUSA ID', 'INGRESO ID']
    tabla(wb.create_sheet('Observaciones del mes'),
          headers + ['FECHA CENTRO', 'ETAPA', 'TIPO OBSERVACIÓN', 'OBSERVACIÓN CENTRO'], observations,
          {'FECHA CENTRO': 'dd-mm-yyyy hh:mm:ss'})
    tabla(wb.create_sheet('Control de lecturas'), headers + ['LECTURA', 'CONTROL'], controls)
    meta.save_workbook_atomic(wb, path)


def _celda(value):
    """Mismo valor que entregaba pandas al releer el XLSX intermedio, salvo textos con '='."""
    if value is None or (isinstance(value, str) and value == ''):
        return None
    if isinstance(value, float) and value.is_integer():
        return int(value)
    if isinstance(value, date) and not isinstance(value, datetime):
        return datetime.combine(value, datetime.min.time())
    return value


def hoja_desde_grilla(grid):
    """DataFrame de la grilla validada por el descargador, sin escribir un XLSX temporal."""
    rows = []
    for row in grid:
        values = [_celda(v) for v in row]
        if 'RIT' in [normal(v) for v in values]:
            values = [ALIAS_COLUMNAS.get(normal(v), v) for v in values]
        while values and values[-1] is None:
            values.pop()
        rows.append(values)
    while rows and not rows[-1]:
        rows.pop()
    return pd.DataFrame(rows, dtype=object) if rows else pd.DataFrame()


def _agrupar_paginas(files):
    """Las páginas de una misma consulta forman un solo candidato; no son duplicados."""
    groups = defaultdict(list)
    for f in files:
        for c in f.candidates:
            groups[c.key].append(c)
    selected, merged_groups = {}, {}
    for key, group in groups.items():
        for c in group:
            c.selected = True
        base = next((c for c in group if c.kind == 'EXCEL'), group[0])
        pages = {id(c.file_result): c.file_result for c in group}.values()
        view = replace(base.file_result, orphan_rows=[r for page in pages for r in page.orphan_rows])
        merged = replace(base, file_result=view, rows=[r for c in group for r in c.rows])
        selected[key] = merged
        merged_groups[key] = [merged]
    return selected, merged_groups


def fuentes(origen):
    return _fuentes(origen)[:4]


def _fuentes(origen):
    """Normaliza XLS BIFF, XML, HTML tabular y OOXML sin modificar originales.

    El manifiesto del descargador identifica TODAS las páginas de una consulta.
    No se selecciona solo un archivo por combinación cuando hay varias páginas.
    Devuelve además las consultas de Meta 4 incluidas en el lote.
    """
    import motor as m
    summaries = [p for p in origen.rglob('resumen.json') if not es_salida(p, origen)]
    if len(summaries) > 1:
        raise ValueError('Hay varios lotes descargados: selecciona la carpeta de un solo lote de registros.')
    sources = []
    bindings = {}
    consultas = []
    filtros = []
    if summaries:
        path = summaries[0]
        summary = json.loads(path.read_text(encoding='utf-8'))
        if summary.get('estado') != 'VALIDADA':
            raise ValueError('La descarga de registros está incompleta. Completa el lote antes de generar Meta 4.')
        # Un lote filtrado se acepta, pero puede dejar registros fuera de Meta 4: se informa siempre.
        rango = summary.get('filtro_fecha')
        if rango:
            desde = rango.get('desde', '?') if isinstance(rango, dict) else '?'
            hasta = rango.get('hasta', '?') if isinstance(rango, dict) else '?'
            filtros.append(f'Filtro de fechas {desde} a {hasta}: el lote incluye solo los ingresos de ese rango.')
        if summary.get('otros_filtros', 'todos los registros') != 'todos los registros':
            filtros.append('Filtros del formulario de SITFA conservados: el lote puede no incluir todos los registros.')
        for query in summary['consultas']:
            if query['seleccion'].get('tab') not in ('Espera', 'Cumplimiento') or query['seleccion'].get('modality') not in ('1', '3'):
                continue
            if query['seleccion'].get('tab') == 'Cumplimiento' and query['seleccion'].get('report') not in (None, '', '0'):
                filtros.append(f"Filtro de vencimiento en {query.get('consulta') or 'Cumplimiento'}: "
                               'incluye solo esas medidas.')
            consultas.append(query)
            manifest = dentro(path.parent, query['verificacion'])
            check = json.loads(manifest.read_text(encoding='utf-8'))
            if check.get('estado') not in ('VALIDADA', 'SIN_RESULTADOS', 'COMPLETA', 'COMPLETO'):
                raise ValueError('Una consulta del descargador no está validada.')
            if check.get('paginas_verificadas') != check.get('paginas_esperadas'):
                raise ValueError('Faltan páginas del descargador.')
            if not check['archivos'] and check.get('evidencia'):
                sources.append((dentro(path.parent, check['evidencia']['archivo']), None))
            for page in check['archivos']:
                source = dentro(path.parent, page['archivo'])
                body = source.read_bytes()
                if hashlib.sha256(body).hexdigest() != page['sha256']:
                    raise ValueError('Un Excel del descargador cambió después de su verificación.')
                sources.append((source, body))
                bindings[source] = page.get('vinculos_ingreso', [])
    else:
        sources = [(p, None) for p in sorted(origen.rglob('*'))
                   if p.suffix.lower() in meta.EXCEL_SUFFIXES | meta.PDF_SUFFIXES and p.is_file()
                   and not p.is_symlink() and not meta.output_file_reason(p)
                   and not p.name.startswith(('Bitacoras_', 'Registros consolidados')) and not es_salida(p, origen)]
    files, issues = [], []
    for path, body in sources:
        if path.suffix.lower() in meta.PDF_SUFFIXES:
            files.append(meta.inspect_zero_record_file(path, origen, 'PDF', {}, issues))
            continue
        try:
            grid = m.read_excel(body if body is not None else path.read_bytes()).grid
        except m.PocError:
            files.append(meta.inspect_excel(path, origen, path, issues))
            continue
        files.append(meta.inspect_excel(path, origen, None, issues, frames=[('Sheet', hoja_desde_grilla(grid))]))
    broken = [f for f in files if f.result == 'ERROR']
    if broken:
        raise ValueError('No se generó una planilla parcial: hay reportes que no se pudieron leer. Revisa: ' + ', '.join(f.relative_path for f in broken))
    for f in files:
        index = defaultdict(set)
        for b in bindings.get(f.path, []):
            identity = (normal(b.get('rit')), _rut(b.get('rut')), normal(b.get('nombre')), normal(b.get('programa')))
            index[identity].add(tuple(str(b.get(k, '')) for k in IDENTIDAD))
        if not index:
            continue
        for candidate in f.candidates:
            for row in candidate.rows:
                v = row.values
                remotes = index.get((normal(v.get('RIT')), _rut(v.get('RUT')), normal(v.get('NOMBRE')), normal(v.get('DERIVACIÓN'))), ())
                if len(remotes) == 1 and all(next(iter(remotes))):
                    v['_REMOTE'] = next(iter(remotes))
    if summaries:
        selected, groups = _agrupar_paginas(files)
    else:
        selected, groups = meta.select_candidates(files, issues)
    return files, issues, selected, groups, consultas, filtros


def sin_bitacoras(consultas, lectura):
    """Consultas con registros que ningún lote de bitácoras enumeró: pares (etiqueta, tribunal)."""
    if not lectura.declara_alcance:
        return []
    faltan = []
    for query in consultas:
        seleccion = query['seleccion']
        if query.get('estado') == 'VALIDADA' and _consulta(seleccion) not in lectura.cubiertas:
            label = ' / '.join(str(seleccion.get(k) or '') for k in ('tribunal_nombre', 'tab', 'modalidad_nombre'))
            faltan.append((label, meta.tribunal_name(seleccion.get('tribunal_nombre'))))
    return faltan


def tribunales_disponibles(origen):
    """Tribunales de Meta 4 consultados en el lote de registros (vacío si no hay resumen.json)."""
    origen = Path(origen)
    found = set()
    for path in origen.rglob('resumen.json'):
        if es_salida(path, origen):
            continue
        try:
            summary = json.loads(path.read_text(encoding='utf-8'))
            for query in summary.get('consultas', []):
                name = meta.tribunal_name((query.get('seleccion') or {}).get('tribunal_nombre'))
                if name:
                    found.add(name)
        except (OSError, ValueError, AttributeError):
            continue
    return [t for t in meta.TRIBUNALS if t in found]


def normalizar_tribunales(values):
    """Nombres canónicos de Meta 4; None significa todos."""
    if not values:
        return None
    result = []
    for value in values:
        name = meta.tribunal_name(value)
        if not name:
            raise ValueError(f'Tribunal no reconocido en el alcance de Meta 4: {value}.')
        if name not in result:
            result.append(name)
    return [t for t in meta.TRIBUNALS if t in result]


def aplicar_filtro(tribunales, files, issues, selected, groups, faltan):
    """Deja solo las combinaciones de los tribunales elegidos; el resto queda NO INCLUIDO."""
    if tribunales is None:
        return selected, groups, faltan
    for f in files:
        for c in f.candidates:
            if c.tribunal not in tribunales:
                c.selected = False
                c.selection_note = 'Fuera del filtro de tribunales'
    issues[:] = [i for i in issues if not (i.level == 'FALTANTE' and i.combination.split(' | ')[0] not in tribunales)]
    selected = {k: v for k, v in selected.items() if k[0] in tribunales}
    groups = {k: v for k, v in groups.items() if k[0] in tribunales}
    return selected, groups, [item for item in faltan if item[1] in tribunales]


def _escribir(destino, corte, emission, template, lectura, faltan, files, issues, selected, groups, rows, tribunales,
              filtros=()):
    estados = lectura.estados
    diaries = lectura.registros
    if tribunales is not None:
        diaries = [d for d in diaries if meta.tribunal_name(d.get('tribunal_nombre')) in tribunales]
    registros = destino / 'Registros.xlsx'
    meta.write_registros_workbook(registros, rows, files, selected, groups, issues, extra_headers=EXTRA,
                                  row_fill=lambda r: ROJO if r.values['ALERTA'] else None, scope=tribunales)
    records, counts, unmatched = meta.read_registros(registros)
    wb = meta.load_template(template)
    summary = meta.rebuild(wb, records, emission or corte, tribunales)
    marcar_planilla(wb, records, rows, tribunales)
    inicio, fin = periodo(corte)
    control = wb.create_sheet('Control bitácoras')
    control.append(['Período de bitácoras', inicio, fin])
    control.append(['Lectura de bitácoras', ', '.join(sorted(set(estados)))])
    control.append(['Consultas sin bitácoras', len(faltan)])
    control.append(['Tribunales incluidos', 'Todos' if tribunales is None else ', '.join(tribunales)])
    control.append(['Filtros del lote de registros', ' | '.join(filtros) if filtros else 'Sin filtros'])
    control.append(['Rojo: última observación en Espera y registro en Cumplimiento. Amarillo: revisar control de bitácora.'])
    control.append(['Sin observación en el mes: FECHA OBS. vacía. Informes no participa.'])
    for cell in control[1][1:]:
        cell.number_format = 'dd-mm-yyyy'
    parcial = '' if tribunales is None else ' - parcial'
    out = destino / f'META 4 - actualizado {(emission or corte):%Y-%m}{parcial}.xlsx'
    meta.save_workbook_atomic(wb, out)
    resumen = resumen_tribunales(rows, faltan, tribunales)
    consolidado(destino / 'Registros consolidados.xlsx', rows, resumen)
    exportar_bitacoras(destino / 'Bitacoras_del_mes.xlsx', diaries)
    totals, counted, problems = meta.verify_meta_output(out)
    problems = list(problems)
    if totals != summary or counted != summary:
        problems.append('Los totales escritos no coinciden con los registros generados.')
    if sum(summary.values()) != sum(len(v) for v in rows.values()) - len(unmatched):
        problems.append('Faltan registros en la planilla final.')
    if problems:
        raise ValueError('La planilla requiere revisión: ' + '; '.join(problems))
    every = [r for group in rows.values() for r in group]
    report = {
        'version': VERSION,
        'desde': str(inicio),
        'hasta': str(fin),
        'registros': len(every),
        'alertas': sum(bool(r.values['ALERTA']) for r in every),
        'revision_bitacora': sum(r.values[EXTRA[1]] != VERIFICADA for r in every),
        'controles_bitacora': dict(Counter(r.values[EXTRA[1]] for r in every)),
        'lectura_bitacoras': sorted(set(estados)),
        'consultas_sin_bitacoras': [label for label, _ in faltan],
        'tribunales': 'TODOS' if tribunales is None else tribunales,
        'resumen_tribunales': resumen,
        'avisos_bitacoras': lectura.avisos,
        'filtros_registros': list(filtros),
        'totales_meta4': summary,
        'excluidos_meta4': len(unmatched),
        'incidencias_fuentes': [i.message for i in issues],
    }
    (destino / 'Reporte META4.json').write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf-8')
    return report


def generar(origen, destino, corte, emission=None, template=None, bitacoras=None, tribunales=None):
    """`origen`: lote de registros del Descargador. `bitacoras`: carpeta de sus lotes de
    bitácoras; si se omite, se buscan dentro de `origen`. `tribunales`: nombres a incluir
    (por defecto, todos); con filtro, la planilla se marca como parcial."""
    tribunales = normalizar_tribunales(tribunales)
    origen, destino = Path(origen).resolve(), Path(destino).resolve()
    if not origen.is_dir():
        raise ValueError('El origen debe ser una carpeta del descargador.')
    raiz_bitacoras = Path(bitacoras).resolve() if bitacoras else origen
    if not raiz_bitacoras.is_dir():
        raise ValueError('La carpeta de bitácoras no existe.')
    if destino.exists():
        raise ValueError('La carpeta de salida ya existe; indica una carpeta nueva.')
    template = Path(template).expanduser().resolve() if template else None
    if template and not template.is_file():
        raise ValueError('No existe la plantilla indicada.')
    lectura = _leer_bitacoras(raiz_bitacoras, corte)
    files, issues, selected, groups, consultas, filtros = _fuentes(origen)
    for filtro in filtros:
        issues.append(meta.Issue('ADVERTENCIA', '', '', filtro))
    if not any(f.candidates for f in files):
        raise ValueError('No se encontraron reportes de Espera/Cumplimiento reconocidos para FAE o Residencia.')
    selected, groups, faltan = aplicar_filtro(tribunales, files, issues, selected, groups,
                                              sin_bitacoras(consultas, lectura))
    if tribunales is not None and not selected:
        raise ValueError('El lote no contiene registros de los tribunales elegidos.')
    for label, _ in faltan:
        issues.append(meta.Issue('ADVERTENCIA', '', label, 'Ningún lote de bitácoras enumeró esta consulta; '
                                 'sus registros quedan sin FECHA OBS.'))
    for aviso in lectura.avisos:
        issues.append(meta.Issue('ADVERTENCIA', '', '', aviso))
    rows = meta.consolidated_rows(selected)
    enriquecer(rows, lectura.registros, corte)
    # Se escribe en una carpeta temporal hermana: si algo falla, no queda una planilla parcial.
    trabajo = destino.with_name(destino.name + EN_CURSO + secrets.token_hex(4))
    trabajo.mkdir(parents=True)
    try:
        report = _escribir(trabajo, corte, emission, template, lectura, faltan,
                           files, issues, selected, groups, rows, tribunales, filtros)
        os.replace(trabajo, destino)
    except BaseException:
        shutil.rmtree(trabajo, ignore_errors=True)
        raise
    return report


def _fecha_arg(text):
    try:
        return date.fromisoformat(text)
    except ValueError:
        raise argparse.ArgumentTypeError('usa el formato AAAA-MM-DD') from None


def _mes_arg(text):
    if not re.fullmatch(r'\d{4}-(0[1-9]|1[0-2])', text):
        raise argparse.ArgumentTypeError('usa el formato AAAA-MM')
    return date.fromisoformat(text + '-01')


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--folder', required=True, help='Lote de registros del Descargador (resumen.json y Excel)')
    p.add_argument('--diaries', help='Carpeta con los lotes de bitácoras; por defecto, dentro de --folder')
    p.add_argument('--output-dir', required=True)
    p.add_argument('--cutoff', type=_fecha_arg, help='Fecha de corte AAAA-MM-DD; por defecto, hoy')
    p.add_argument('--month', type=_mes_arg, help='Mes de emisión AAAA-MM; no cambia el período de bitácoras')
    p.add_argument('--template')
    p.add_argument('--tribunal', action='append', help='Incluir solo este tribunal; repetir para varios')
    args = p.parse_args()
    cutoff = args.cutoff or date.today()
    if cutoff > date.today():
        p.exit(1, 'La fecha de corte no puede estar en el futuro.\n')
    try:
        print(json.dumps(generar(args.folder, args.output_dir, cutoff, args.month, args.template, args.diaries, args.tribunal), ensure_ascii=False, indent=2))
    except (ValueError, OSError) as exc:
        p.exit(1, str(exc) + '\n')


if __name__ == '__main__':
    main()
