"""Calendario de informes por ingreso (WD-1) y cotejo CAL-03 (WA-6, decisiones del usuario 19-oct-2026).

«Una nomenclatura, varias vistas»: el calendario del ingreso, la pestaña Informes y Hoja2 (calendario 91) describen
el mismo informe proyectado. Si difieren se avisa una vez (CAL-03, solo mostrar); nunca se generan dos observaciones.
Vencimiento = próxima proyección PENDIENTE; solo PENDIENTE puede estar vencido.
"""

from collections import defaultdict
from datetime import date

from .motor.utilidades import fecha_es

TIPO = 'INFORME_PROYECTADO'


def proyecciones(capture, identity):
    """Proyecciones verificadas por ingreso, ordenadas: {identidad: [(fecha, estado), ...]}."""
    found = defaultdict(set)
    for row in capture.rows('calendarios'):
        if row['association'] == 'VERIFICADA' and row['event_type'] == TIPO and row['event_at']:
            found[identity(row)].add((date.fromisoformat(row['event_at'][:10]), row['status']))
    return {key: sorted(values) for key, values in found.items()}


def primera_pendiente(items):
    return next((fecha for fecha, estado in items if estado == 'PENDIENTE'), None)


def proxima_pendiente(items, as_of):
    return next((fecha for fecha, estado in items if estado == 'PENDIENTE' and fecha > as_of), None)


def avisos_cotejo(items, as_of, *, vencimiento_informes=None, fecha_hoja2=None):
    if not items:
        return []
    out = []
    pendiente = primera_pendiente(items)
    if vencimiento_informes and vencimiento_informes != pendiente:
        out.append(
            'CAL-03: la pestaña Informes indica vencimiento el '
            + fecha_es(vencimiento_informes)
            + (
                ' y el calendario del ingreso proyecta el informe pendiente el ' + fecha_es(pendiente)
                if pendiente
                else ' y el calendario del ingreso no tiene informes pendientes'
            )
            + '; revisar en RUS.'
        )
    proxima = proxima_pendiente(items, as_of)
    if fecha_hoja2 and fecha_hoja2 != proxima:
        out.append(
            'CAL-03: Hoja2 (calendario 91) indica el '
            + fecha_es(fecha_hoja2)
            + (
                ' y el calendario del ingreso proyecta el próximo informe el ' + fecha_es(proxima)
                if proxima
                else ' y el calendario del ingreso no tiene un próximo informe pendiente'
            )
            + '; revisar en RUS.'
        )
    return out
