"""Avisos de bitácora «solo mostrar» (decisión del usuario, 19 de octubre de 2026): CT-05, CT-07 y CT-11.

Las entradas «Centro» de la bitácora RUS son las observaciones propias del CSMP; el tribunal responde en
«Observación Tribunal». Los avisos no cambian la observación propuesta, reglas, acciones ni umbrales.
"""

from datetime import date
from difflib import SequenceMatcher

from nurus.rus.columns import normalize

DIAS_CORREO_E05 = 30
SIMILITUD = 0.92


def _dia(evento):
    return date.fromisoformat(evento['event_at'][:10])


def _fecha(evento):
    return '/'.join(reversed(evento['event_at'][:10].split('-')))


def _cuerpo(texto):
    """Texto comparable sin el prefijo «Nombre SIGLA: » que antepone el motor."""
    t = normalize(texto or '')
    head, sep, rest = t.partition(': ')
    return rest if sep and len(head) <= 40 else t


def repite_ultima(display, propuesta):
    """Entrada «Centro» más reciente cuyo texto repite (≥0,92) la propuesta, o None."""
    if not display or display.get('status') != 'ACREDITADA':
        return None
    cuerpo = _cuerpo(propuesta)
    if not cuerpo:
        return None
    for evento in display['last_center']:
        if SequenceMatcher(None, cuerpo, _cuerpo(evento.get('text', '')), autojunk=False).ratio() >= SIMILITUD:
            return evento
    return None


def avisos(display, propuesta, reglas, textos, as_of):
    """Lista de (regla, texto). Solo con cronología acreditada del centro dentro del horizonte vigente."""
    if not display or display.get('status') != 'ACREDITADA':
        return []
    result = []
    evento = repite_ultima(display, propuesta)
    if evento:
        result.append(
            (
                'CT-07',
                f'La observación propuesta repite la última del centro ({_fecha(evento)}); '
                'revisar antes de registrarla.',
            )
        )
    if 'ESPERA.E05_SOLO_CORREO' in reglas:
        correo = _cuerpo(textos['ESPERA']['E05_SOLO_CORREO']['texto']).rstrip('. ')
        previos = [
            e
            for e in display.get('center_period', ())
            if correo and correo in _cuerpo(e.get('text', '')) and (as_of - _dia(e)).days >= DIAS_CORREO_E05
        ]
        if previos:
            evento = max(previos, key=_dia)
            dias = (as_of - _dia(evento)).days
            result.append(
                (
                    'CT-11',
                    f'El correo de E-05 ya se envió el {_fecha(evento)} (hace {dias} días) y sigue sin '
                    'ingreso efectivo; evaluar proyecto de resolución.',
                )
            )
    for evento in display.get('pending_court', ()):
        dias = (as_of - _dia(evento)).days
        result.append(
            ('CT-05', f'Observación al tribunal del {_fecha(evento)} sin respuesta registrada (hace {dias} días).')
        )
    return result


def aplicar_reiteracion(rows, contexts, config):
    """CT-07 con texto (decisión del usuario 19-oct-2026): si la observación propuesta repite la última del centro,
    se consigna «Se reitera observación anterior del {fecha}» para toda la observación. Acciones sin cambios.
    """
    from .motor.utilidades import fecha_es

    regla = 'COMUN.REITERA_OBSERVACION'
    if regla in config['desactivadas']:
        return
    plantilla = config['textos']['COMUN']['REITERA_OBSERVACION']['texto']
    for row in rows:
        evento = repite_ultima(contexts.get(row.id, {}).get('display'), row.observation)
        if not evento:
            continue
        head, sep, _ = row.observation.partition(': ')
        prefijo = head + sep if sep and len(head) <= 40 else ''
        texto = plantilla.format_map({'FECHA_OBSERVACION_ANTERIOR': fecha_es(_dia(evento))})
        row.warnings.append(
            f'CT-07: la observación propuesta repetía la del {_fecha(evento)}; se consigna la '
            f'reiteración. Texto propuesto originalmente: {row.observation}'
        )
        row.observation = prefijo + texto
        row.rules = [*row.rules, regla]
