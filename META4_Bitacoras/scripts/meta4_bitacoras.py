"""Meta 4 a partir de originales SITFA y bitácoras de un mes móvil."""
from __future__ import annotations
import argparse
import calendar
from collections import defaultdict
from datetime import date, datetime
import hashlib
import json
from pathlib import Path
import sys
import tempfile
from dataclasses import replace

BASE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(BASE / 'descargador'))
sys.path.insert(0, str(BASE / 'src'))
import generate_meta4 as meta
from nurus.bitacora_html import read_snapshot
from openpyxl import Workbook, load_workbook
from openpyxl.styles import PatternFill, Font
from openpyxl.comments import Comment

ALERTA = 'ALERTA: última observación en Espera; registro en Cumplimiento'
EXTRA = ['ETAPA ÚLTIMA OBS.', 'CONTROL BITÁCORA', 'ALERTA']


def periodo(corte):
    year, month = (corte.year - 1, 12) if corte.month == 1 else (corte.year, corte.month - 1)
    return date(year, month, min(corte.day, calendar.monthrange(year, month)[1])), corte


def fecha(value):
    if isinstance(value, datetime):
        return value.replace(tzinfo=None)
    if isinstance(value, date):
        return datetime.combine(value, datetime.min.time())
    value = str(value or '').strip()
    try:
        return datetime.fromisoformat(value.replace('Z', '+00:00')).replace(tzinfo=None)
    except ValueError:
        for fmt in ('%d/%m/%Y %H:%M:%S', '%d/%m/%Y %H:%M', '%d/%m/%Y', '%d-%m-%Y'):
            try:
                return datetime.strptime(value, fmt)
            except ValueError:
                pass
    raise ValueError('Fecha de bitácora no reconocida.')


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
    return last.date(), stages.pop(), 'VERIFICADA EN EL PERÍODO'


def dentro(folder, name):
    target = (folder / name).resolve()
    if not target.is_relative_to(folder.resolve()) or target.is_symlink():
        raise ValueError('Referencia de bitácora fuera de la carpeta de origen.')
    return target


def cargar_bitacoras(root, corte):
    result = []
    paths = sorted(root.rglob('bitacoras_meta4.json'))
    if not paths:
        paths = sorted(root.rglob('bitacoras.json'))
    if not paths:
        raise ValueError('Faltan bitácoras. Descarga con esta herramienta o selecciona una carpeta que contenga bitacoras.json y sus HTML.')
    for path in paths:
        data = json.loads(path.read_text(encoding='utf-8'))
        own = data.get('tipo') == 'META4_BITACORAS_MES'
        if own and (periodo(corte)[0] < date.fromisoformat(data['desde']) or corte > date.fromisoformat(data['hasta'])):
            raise ValueError('Las bitácoras guardadas no cubren este período. Usa su fecha de corte o vuelve a descargarlas.')
        if not own and (data.get('tipo') != 'BITACORAS_LECTURA' or data.get('version') != 1):
            raise ValueError('Formato de bitácoras no reconocido.')
        for original in data.get('registros', []):
            record = dict(original)
            record['entries'] = []
            record['control'] = 'LECTURA NO COMPLETADA'
            if record.get('estado') == 'LEIDA':
                try:
                    if own:
                        snapshot = record['snapshot']
                    else:
                        body = dentro(path.parent, record['archivo']).read_bytes()
                        if hashlib.sha256(body).hexdigest() != record['sha256']:
                            raise ValueError('La bitácora cambió después de su descarga.')
                        snapshot = read_snapshot(body, record['params'])
                    if tuple(str(snapshot[k]) for k in ('tribunal_codigo', 'causa_id', 'ingreso_id')) != tuple(str(record[k]) for k in ('tribunal_codigo', 'causa_id', 'ingreso_id')):
                        raise ValueError('La bitácora pertenece a otro ingreso.')
                    record['entries'] = entradas_periodo(snapshot['entries'], corte)
                    record['control'] = 'OK'
                    if record.get('advertencias'):
                        record['control'] = 'REVISAR IDENTIDAD VISIBLE'
                except (KeyError, ValueError, OSError, TypeError):
                    record['control'] = 'BITÁCORA NO VERIFICABLE'
            result.append(record)
    return result


def normal(value):
    return meta.norm(value)


def buscar(row, diaries):
    values = row.values
    trib = meta.tribunal_name(values.get('TRIBUNAL')) or normal(values.get('TRIBUNAL'))
    rit = normal(values.get('RIT'))
    rut = normal(values.get('RUT')).replace(' ', '')
    nombre = normal(values.get('NOMBRE'))
    programa = normal(values.get('DERIVACIÓN'))
    candidates = []
    for diary in diaries:
        dt = meta.tribunal_name(diary.get('tribunal_nombre')) or normal(diary.get('tribunal_nombre'))
        if dt != trib or normal(diary.get('rit')) != rit:
            continue
        if rut:
            if normal(diary.get('rut')).replace(' ', '') != rut:
                continue
        elif not nombre or normal(diary.get('nombre')) != nombre:
            continue
        if not programa or normal(diary.get('programa')) != programa:
            continue
        remote = values.get('_REMOTE')
        if remote and tuple(str(diary.get(k, '')) for k in ('tribunal_codigo', 'causa_id', 'ingreso_id')) != remote:
            continue
        candidates.append(diary)
    identities = {(r.get('tribunal_codigo'), r.get('causa_id'), r.get('ingreso_id')) for r in candidates}
    if len(identities) != 1 or not all(next(iter(identities), ())):
        return None, 'IDENTIDAD AMBIGUA' if candidates else 'SIN BITÁCORA ASOCIADA'
    if any(r.get('control') != 'OK' for r in candidates):
        return None, next(r['control'] for r in candidates if r.get('control') != 'OK')
    # Una misma identidad puede aparecer en ambas pestañas. Se elige la captura más reciente.
    newest = max(candidates, key=lambda r: str(r.get('capturada', '')))
    return newest, 'OK'


def enriquecer(rows, diaries, corte):
    for group in rows.values():
        for row in group:
            diary, control = buscar(row, diaries)
            day, stage = None, ''
            if diary:
                day, stage, control = ultima(diary['entries'], corte)
            row.values['FECHA OBS.'] = day
            row.values[EXTRA[0]] = stage
            row.values[EXTRA[1]] = control
            row.values['ALERTA'] = ALERTA if row.status == 'Cumplimiento' and stage == 'Espera' else ''


def tabla(ws, headers, records):
    ws.append(headers)
    for record in records:
        ws.append([meta.excel_value(record.get(h)) for h in headers])
    for cell in ws[1]:
        cell.fill = PatternFill('solid', fgColor='1F4E78')
        cell.font = Font(color='FFFFFF', bold=True)
    ws.freeze_panes = 'A2'
    ws.auto_filter.ref = ws.dimensions
    for col in ws.columns:
        ws.column_dimensions[col[0].column_letter].width = 24
    for row in ws.iter_rows(min_row=2):
        alert = bool(row[headers.index('ALERTA')].value) if 'ALERTA' in headers else False
        for cell in row:
            if isinstance(cell.value, (date, datetime)):
                cell.number_format = 'dd-mm-yyyy'
            if cell.data_type == 'f':
                cell.data_type = 's'
            if alert:
                cell.fill = PatternFill('solid', fgColor='FFC7CE')


def consolidado(path, rows):
    wb = Workbook()
    wb.remove(wb.active)
    for status in ('Espera', 'Cumplimiento'):
        tabla(wb.create_sheet(status), meta.SOURCE_HEADERS[status] + EXTRA, [r.values for r in rows[status]])
    groups = defaultdict(list)
    for status, group in rows.items():
        for r in group:
            key = (normal(r.values.get('RUT')).replace(' ', ''), normal(r.values.get('NOMBRE')))
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
    duplicates = unified(duplicates)
    tabla(wb.create_sheet('Duplicados'), headers, duplicates)
    alert_rows = [dict(r.values, UBICACIÓN=status) for status, group in rows.items() for r in group if r.values['ALERTA'] or r.values[EXTRA[1]] != 'VERIFICADA EN EL PERÍODO']
    tabla(wb.create_sheet('Alertas y revisión'), headers, unified(alert_rows))
    wb.save(path)


def marcar_planilla(wb, records, rows):
    # Mismo orden por modalidad y tribunal que read_registros/rebuild; no se coteja solo por RIT.
    flagged = defaultdict(list)
    for status, group in rows.items():
        for r in group:
            sheet = f'{r.category} {status}'
            if sheet in records and r.tribunal in records[sheet]:
                flagged[(sheet, r.tribunal)].append(r)
    for name, spec in meta.SHEET_DEFS.items():
        ws = wb[name]
        pos = 1
        for court in meta.TRIBUNALS:
            pos += 4
            source = flagged[(name, court)]
            if records[name][court]:
                pos += 1
                for r in source:
                    control = r.values[EXTRA[1]]
                    alert = r.values['ALERTA']
                    if alert or control != 'VERIFICADA EN EL PERÍODO':
                        ws.cell(pos, 8).comment = Comment(alert or control, 'META 4')
                        for cell in ws[pos][:8]:
                            cell.fill = PatternFill('solid', fgColor='FFC7CE' if alert else 'FFF2CC')
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
    ws = wb.create_sheet('Observaciones del mes')
    tabla(ws, headers + ['FECHA CENTRO', 'ETAPA', 'TIPO OBSERVACIÓN', 'OBSERVACIÓN CENTRO'], observations)
    for cell in list(ws.columns)[7][1:]:
        cell.number_format = 'dd-mm-yyyy hh:mm:ss'
    tabla(wb.create_sheet('Control de lecturas'), headers + ['LECTURA', 'CONTROL'], controls)
    wb.save(path)


def fuentes(origen):
    """Normaliza XLS BIFF, XML, HTML tabular y OOXML sin modificar originales.

    El manifiesto del descargador identifica TODAS las páginas de una consulta.
    No se selecciona solo un archivo por combinación cuando hay varias páginas.
    """
    import motor as m
    summaries = list(origen.rglob('resumen.json'))
    summaries = [p for p in summaries if not any(x.startswith(('Resultado META4', 'Salida META4')) for x in p.relative_to(origen).parts)]
    if len(summaries) > 1:
        raise ValueError('Hay varios lotes descargados: selecciona la carpeta de un solo lote y sus bitácoras.')
    source_paths = []
    bindings = {}
    if summaries:
        path = summaries[0]
        summary = json.loads(path.read_text(encoding='utf-8'))
        if summary.get('estado') != 'VALIDADA':
            raise ValueError('La descarga de registros está incompleta. Completa el lote antes de generar Meta 4.')
        for query in summary['consultas']:
            if query['seleccion'].get('tab') not in ('Espera', 'Cumplimiento') or query['seleccion'].get('modality') not in ('1', '3'):
                continue
            manifest = dentro(path.parent, query['verificacion'])
            check = json.loads(manifest.read_text(encoding='utf-8'))
            if check.get('estado') not in ('VALIDADA', 'SIN_RESULTADOS', 'COMPLETA', 'COMPLETO'):
                raise ValueError('Una consulta del descargador no está validada.')
            if check.get('paginas_verificadas') != check.get('paginas_esperadas'):
                raise ValueError('Faltan páginas del descargador.')
            if not check['archivos'] and check.get('evidencia'):
                source_paths.append(dentro(path.parent, check['evidencia']['archivo']))
            for page in check['archivos']:
                source = dentro(path.parent, page['archivo'])
                if hashlib.sha256(source.read_bytes()).hexdigest() != page['sha256']:
                    raise ValueError('Un Excel del descargador cambió después de su verificación.')
                source_paths.append(source)
                bindings[source] = page.get('vinculos_ingreso', [])
    else:
        source_paths = [p for p in origen.rglob('*') if p.suffix.lower() in meta.EXCEL_SUFFIXES | meta.PDF_SUFFIXES and p.is_file() and not p.is_symlink() and not meta.output_file_reason(p) and not p.name.startswith(('Bitacoras_', 'Registros consolidados')) and not any(x.startswith(('Resultado META4', 'Salida META4')) for x in p.relative_to(origen).parts)]
    files, issues = [], []
    with tempfile.TemporaryDirectory(prefix='meta4-lectura-') as tmp:
        for i, path in enumerate(source_paths):
            if path.suffix.lower() in meta.PDF_SUFFIXES:
                files.append(meta.inspect_zero_record_file(path, origen, 'PDF', {}, issues))
                continue
            readable = Path(tmp) / f'{i}.xlsx'
            try:
                grid = m.read_excel(path.read_bytes()).grid
                wb = Workbook()
                for row in grid:
                    aliases = {'RUT RCEI': 'RUT', 'NOMBRE MENOR': 'NOMBRE'}
                    values = [aliases.get(normal(v), v) for v in row] if 'RIT' in [normal(v) for v in row] else list(row)
                    wb.active.append(values)
                wb.save(readable)
                wb.close()
                files.append(meta.inspect_excel(path, origen, readable, issues))
            except m.PocError:
                files.append(meta.inspect_excel(path, origen, path, issues))
    broken = [f for f in files if f.result == 'ERROR']
    if broken:
        raise ValueError('No se generó una planilla parcial: hay reportes que no se pudieron leer. Revisa: ' + ', '.join(f.relative_path for f in broken))
    for f in files:
        for candidate in f.candidates:
            for row in candidate.rows:
                v = row.values
                identity = (normal(v.get('RIT')), normal(v.get('RUT')).replace(' ', ''), normal(v.get('NOMBRE')), normal(v.get('DERIVACIÓN')))
                matching = [b for b in bindings.get(f.path, []) if (normal(b.get('rit')), normal(b.get('rut')).replace(' ', ''), normal(b.get('nombre')), normal(b.get('programa'))) == identity]
                remotes = {tuple(str(b.get(k, '')) for k in ('tribunal_codigo', 'causa_id', 'ingreso_id')) for b in matching}
                if len(remotes) == 1 and all(next(iter(remotes))):
                    v['_REMOTE'] = next(iter(remotes))
    if summaries:
        groups = defaultdict(list)
        for f in files:
            for c in f.candidates:
                groups[c.key].append(c)
        selected = {}
        for key, group in groups.items():
            for c in group:
                c.selected = True
            selected[key] = replace(group[0], rows=[r for c in group for r in c.rows])
    else:
        selected, groups = meta.select_candidates(files, issues)
    return files, issues, selected, groups


def generar(origen, destino, corte, emission=None, template=None):
    origen, destino = Path(origen).resolve(), Path(destino).resolve()
    if not origen.is_dir():
        raise ValueError('El origen debe ser una carpeta del descargador.')
    diaries = cargar_bitacoras(origen, corte)
    files, issues, selected, groups = fuentes(origen)
    rows = meta.consolidated_rows(selected)
    if not any(f.candidates for f in files):
        raise ValueError('No se encontraron reportes de Espera/Cumplimiento reconocidos para FAE o Residencia.')
    enriquecer(rows, diaries, corte)
    destino.mkdir(parents=True, exist_ok=False)
    registros = destino / 'Registros.xlsx'
    meta.write_registros_workbook(registros, rows, files, selected, groups, issues)
    # Agregar control y alertas también al consolidado usado para construir la planilla.
    book = load_workbook(registros)
    for status in rows:
        ws = book[meta.find_sheet(book.sheetnames, status)]
        start = ws.max_column + 1
        for col, h in enumerate(EXTRA, start):
            ws.cell(1, col, h)
            ws.column_dimensions[ws.cell(1, col).column_letter].width = 42
        for index, row in enumerate(rows[status], 2):
            for col, h in enumerate(EXTRA, start):
                ws.cell(index, col, row.values[h])
            if row.values['ALERTA']:
                for cell in ws[index]:
                    cell.fill = PatternFill('solid', fgColor='FFC7CE')
        ws.auto_filter.ref = ws.dimensions
    book.save(registros)
    records, counts, unmatched = meta.read_registros(registros)
    wb = meta.load_template(Path(template) if template else None)
    summary = meta.rebuild(wb, records, emission or corte)
    marcar_planilla(wb, records, rows)
    inicio, fin = periodo(corte)
    control = wb.create_sheet('Control bitácoras')
    control.append(['Período de bitácoras', inicio, fin])
    control.append(['Rojo: última observación en Espera y registro en Cumplimiento. Amarillo: revisar control de bitácora.'])
    control.append(['Sin observación en el mes: FECHA OBS. vacía. Informes no participa.'])
    out = destino / f'META 4 - actualizado {(emission or corte):%Y-%m}.xlsx'
    meta.save_workbook_atomic(wb, out)
    consolidado(destino / 'Registros consolidados.xlsx', rows)
    exportar_bitacoras(destino / 'Bitacoras_del_mes.xlsx', diaries)
    verification = meta.verify_meta_output(out)
    problems = list(verification[2])
    if verification[0] != summary or verification[1] != summary:
        problems.append('Los totales escritos no coinciden con los registros generados.')
    if sum(summary.values()) != sum(len(v) for v in rows.values()) - len(unmatched):
        problems.append('Faltan registros en la planilla final.')
    if problems:
        raise ValueError('La planilla requiere revisión: ' + '; '.join(problems))
    total = sum(len(v) for v in rows.values())
    alerts = sum(bool(r.values['ALERTA']) for group in rows.values() for r in group)
    reviews = sum(r.values[EXTRA[1]] != 'VERIFICADA EN EL PERÍODO' for group in rows.values() for r in group)
    report = {'version': '1.0', 'desde': str(inicio), 'hasta': str(fin), 'registros': total, 'alertas': alerts, 'revision_bitacora': reviews, 'totales_meta4': summary, 'excluidos_meta4': len(unmatched), 'incidencias_fuentes': [i.message for i in issues]}
    (destino / 'Reporte META4.json').write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf-8')
    return report


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--folder', required=True)
    p.add_argument('--output-dir', required=True)
    p.add_argument('--cutoff', type=date.fromisoformat, default=date.today())
    p.add_argument('--month', help='Mes de emisión AAAA-MM; no cambia el período de bitácoras')
    p.add_argument('--template')
    args = p.parse_args()
    try:
        emission = date.fromisoformat(args.month + '-01') if args.month else None
        print(json.dumps(generar(args.folder, args.output_dir, args.cutoff, emission, args.template), ensure_ascii=False, indent=2))
    except (ValueError, OSError) as exc:
        p.exit(1, str(exc) + '\n')

if __name__ == '__main__':
    main()
