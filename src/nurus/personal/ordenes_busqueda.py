"""Órdenes de búsqueda (Excel «Búsqueda Lit», menú 94): avisos OB-01..OB-05 solo mostrar.

Decisión del usuario del 19 de octubre de 2026: se reabre la revisión de órdenes de búsqueda sin cruces con otras
fuentes (el Excel no trae causa ni ingreso remotos) y sin cambiar observaciones ni acciones.
"""

from nurus.rus.columns import normalize
from .motor.utilidades import fecha_es, fecha_mayoria, fechas_varias, get_date

DIAS_SIN_RESULTADO = 60
COLUMNAS = {
    'estado': 'ESTADO O B',
    'resultado': 'RESULTADO',
    'tramite': 'FECHA TRAMITE',
    'nacimiento': 'FEC NACIMIENTO',
    'envio': 'ESTADO ENVIO',
    'tipo': 'TIPO LITIGANTE',
    'curador': 'CURADOR',
    'audiencias': 'PROX AUDS',
}


def _clave(texto):
    return ' '.join(normalize(texto).upper().split())


def _columnas(values):
    disponibles = {_clave(k): k for k in values}
    return {nombre: disponibles[_clave(titulo)] for nombre, titulo in COLUMNAS.items() if _clave(titulo) in disponibles}


def _dia(valor):
    fecha = get_date(valor)
    return fecha.date() if hasattr(fecha, 'date') else fecha


def _vacio(valor):
    return str(valor or '').strip().lower() in ('', '-', '--', '---', 'nan', 'none')


def avisos_orden(values, as_of):
    """Avisos de una fila de Búsqueda Lit; solo órdenes en estado «Activa». Una columna ausente omite su aviso."""
    cols = _columnas(values)
    get = lambda nombre: values.get(cols[nombre], '') if nombre in cols else None  # noqa: E731
    if 'estado' not in cols or _clave(get('estado')) != 'ACTIVA':
        return []
    out = []
    tramite = _dia(get('tramite')) if 'tramite' in cols else None
    if 'resultado' in cols and _vacio(get('resultado')) and tramite and (as_of - tramite).days > DIAS_SIN_RESULTADO:
        out.append(f'OB-01: orden de búsqueda activa sin resultado desde el {fecha_es(tramite)} '
                   f'({(as_of - tramite).days} días).')
    nacimiento = _dia(get('nacimiento')) if 'nacimiento' in cols else None
    if nacimiento:
        mayoria = _dia(fecha_mayoria(nacimiento))
        if mayoria and mayoria <= as_of:
            out.append(f'OB-02: orden de búsqueda activa y la persona cumplió 18 años el {fecha_es(mayoria)}.')
    if 'envio' in cols and _clave(get('envio')) != 'REALIZADA':
        estado = str(get('envio') or '').strip() or 'sin estado'
        out.append(f'OB-03: el envío de la orden no figura como realizado ({estado}).')
    if 'tipo' in cols and 'curador' in cols and _clave(get('tipo')) in ('NINO', 'NINA', 'ADOLESCENTE') \
            and _vacio(get('curador')):
        out.append('OB-04: orden de búsqueda activa de un niño, niña o adolescente sin curador ad litem registrado.')
    if 'audiencias' in cols:
        futuras = sorted({_dia(f) for f in fechas_varias(get('audiencias'))[0] if _dia(f) >= as_of})
        if futuras:
            out.append(f'OB-05: audiencia fijada para el {fecha_es(futuras[0])} con orden de búsqueda activa.')
    return out
