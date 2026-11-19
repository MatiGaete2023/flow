"""Revisión de bitácoras: separar los registros con observación, sin observación y no verificables (P2, plan flow-python).

Port de la herramienta de Flow Rust (`rust/REVISION_BITACORAS_ESQUEMA.md`). No cambia el motor: el análisis corre sobre el
paquete completo y esta revisión solo clasifica y filtra después. Una bitácora acredita «sin observación» únicamente si su
ventana se leyó con la estructura reconocida (método SITFA_TABLA_INFORME_INGRESO_V1), sin indicios de recorte ni
advertencias, y se leyó en o después del fin del período revisado.
"""

from __future__ import annotations

import json
from datetime import date

CON = 'CON_OBSERVACION'
SIN = 'SIN_OBSERVACION'
NO_VERIFICABLE = 'NO_VERIFICABLE'
NOMBRES = {CON: 'Con observación', SIN: 'Sin observación', NO_VERIFICABLE: 'No verificable'}


def _dia(texto):
    try:
        return date.fromisoformat(str(texto or '')[:10])
    except ValueError:
        return None


def evidencia_por_ingreso(capture):
    """{(court_code, cause_id, ingreso_id): revision_bitacoras} desde las consultas de bitácora del paquete."""
    salida = {}
    for consulta in capture.manifest.get('queries', ()):
        if consulta.get('screen') != 'bitacoras':
            continue
        try:
            filtros = json.loads(consulta.get('filters_json') or '{}')
        except ValueError:
            continue
        identidad = filtros.get('remote_identity') or {}
        clave = tuple(str(identidad.get(k, '')) for k in ('court_code', 'cause_id', 'ingreso_id'))
        if all(clave):
            salida[clave] = {**(filtros.get('revision_bitacoras') or {}), 'lectura': consulta.get('coverage', '')}
    return salida


def datos_de_registro(record, evidencia, eventos):
    """Lo que la revisión necesita de un registro VERIFICADO, para guardarlo en la fila (sobrevive a la sesión)."""
    clave = tuple(str(record.get(k, '')) for k in ('court_code', 'cause_id', 'ingreso_id'))
    ev = evidencia.get(clave)
    fechas = sorted({d.isoformat() for e in eventos if str(e.get('text', '')).strip() for d in [_dia(e.get('event_at'))] if d})
    return {
        'leida': bool(ev) and ev.get('lectura') != 'FALLIDA',
        'alcance': (ev or {}).get('coverage', ''),
        'completa': bool((ev or {}).get('completa')),
        'capturada': str((ev or {}).get('captured_at', ''))[:10],
        'fechas_centro': fechas,
    }


def periodo_por_defecto(corte):
    """Mes calendario que termina en el corte, ambos días incluidos, como Meta 4 (16/09 a 16/10; 28/02 a 31/03)."""
    mes, anio = (corte.month - 1, corte.year) if corte.month > 1 else (12, corte.year - 1)
    import calendar

    dia = min(corte.day, calendar.monthrange(anio, mes)[1])
    return date(anio, mes, dia), corte


def leer_fecha(texto):
    """dd/mm/aaaa o aaaa-mm-dd → date; ValueError con un motivo legible."""
    import re

    texto = str(texto or '').strip()
    iso = re.fullmatch(r'(\d{4})-(\d{1,2})-(\d{1,2})', texto)
    local = re.fullmatch(r'(\d{1,2})[/-](\d{1,2})[/-](\d{4})', texto)
    try:
        if iso:
            return date(int(iso[1]), int(iso[2]), int(iso[3]))
        if local:
            return date(int(local[3]), int(local[2]), int(local[1]))
    except ValueError:
        pass
    raise ValueError(f'Fecha no reconocida: «{texto}». Escríbela como dd/mm/aaaa.')


def periodo(desde_texto, hasta_texto):
    desde, hasta = leer_fecha(desde_texto), leer_fecha(hasta_texto)
    if desde > hasta:
        raise ValueError('La fecha «desde» es posterior a «hasta».')
    return desde, hasta


def clasificar(datos, desde, hasta):
    """(clasificación, motivo) de un registro para el período [desde, hasta]."""
    if not datos:
        return NO_VERIFICABLE, 'Sin bitácora verificada en el paquete (identidad no verificada o paquete sin bitácoras).'
    en_periodo = [f for f in datos.get('fechas_centro', ()) if desde.isoformat() <= f <= hasta.isoformat()]
    if en_periodo:
        return CON, f'{len(en_periodo)} observación(es) del centro en el período; última {max(en_periodo)}.'
    if not datos.get('leida'):
        return NO_VERIFICABLE, 'La bitácora no se pudo leer.'
    if datos.get('alcance') not in ('COMPLETA', 'VACIA_COMPROBADA'):
        motivo = 'La ventana mostró indicios de recorte (paginador u otro control).' if datos.get('alcance') == 'PARCIAL' else (
            'Lectura sin evidencia de alcance (paquete anterior a esta revisión): vuelve a capturar.'
        )
        return NO_VERIFICABLE, motivo
    if not datos.get('completa'):
        return NO_VERIFICABLE, 'La lectura tiene advertencias de identidad visible.'
    capturada = _dia(datos.get('capturada'))
    if capturada is None or capturada < hasta:
        return NO_VERIFICABLE, 'La bitácora se leyó antes del fin del período: no acredita ausencia hasta esa fecha.'
    return SIN, 'Bitácora leída completa, sin observaciones del centro en el período.'


def revisar(work, desde, hasta):
    """[{'row': fila, 'clasificacion', 'motivo'}] de los registros no excluidos, en el orden de Trabajo."""
    salida = []
    for fila in work.rows:
        if fila.excluded:
            continue
        clase, motivo = clasificar(getattr(fila, 'revision_bitacora', None) or {}, desde, hasta)
        salida.append({'row': fila, 'clasificacion': clase, 'motivo': motivo})
    return salida


def resumen(items):
    conteo = {CON: 0, SIN: 0, NO_VERIFICABLE: 0}
    for item in items:
        conteo[item['clasificacion']] += 1
    return conteo


def exportar(work, items, desde, hasta, destino):
    """Excel (una fila por registro, con su clasificación, motivo y propuesta) y JSON con el resumen, en una carpeta NUEVA.
    Devuelve (ruta_excel, ruta_json). No escribe nada si la carpeta ya existe."""
    from pathlib import Path

    from openpyxl import Workbook

    from .outputs import value

    destino = Path(destino)
    if destino.exists():
        raise ValueError('La carpeta de destino ya existe; elige otro nombre.')
    libro = Workbook()
    hoja = libro.active
    hoja.title = 'Revisión de bitácoras'
    hoja.append(['Clasificación', 'Motivo', 'Tribunal', 'RIT', 'Nombre', 'Programa', 'Observación propuesta'])
    for item in items:
        fila = item['row']
        celdas = [
            NOMBRES[item['clasificacion']],
            item['motivo'],
            value(work, fila, 'tribunal'),
            value(work, fila, 'rit'),
            value(work, fila, 'nombre'),
            value(work, fila, 'programa'),
            fila.observation,
        ]
        # Un texto que empieza con «=» se escribe como texto, nunca como fórmula.
        hoja.append([("'" + c) if isinstance(c, str) and c[:1] in '=+-@' else c for c in celdas])
    destino.mkdir(parents=True)
    excel = destino / 'Revision_bitacoras.xlsx'
    libro.save(excel)
    informe = destino / 'Revision_bitacoras.json'
    informe.write_text(
        json.dumps(
            {
                'tipo': 'REVISION_BITACORAS',
                'version': 1,
                'periodo': {'desde': desde.isoformat(), 'hasta': hasta.isoformat()},
                'corte': getattr(work, 'as_of', ''),
                'resumen': {NOMBRES[k]: v for k, v in resumen(items).items()},
                'registros': [{'id': i['row'].id, 'clasificacion': i['clasificacion'], 'motivo': i['motivo']} for i in items],
            },
            ensure_ascii=False,
            indent=2,
        ),
        encoding='utf-8',
    )
    return excel, informe
