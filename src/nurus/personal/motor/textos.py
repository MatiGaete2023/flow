import re

from .contexto import current

# Orden de las observaciones (decisión x, 15-oct): primero las constancias, luego las acciones del Centro
# (correos y proyectos de resolución), después la gestión del Tribunal y, al final, la aclaración de causa.
ORDEN_GRUPOS = {'C': 0, 'A': 1, 'T': 2, 'F': 3}

# Grupo de cada oración de los textos que mezclan grupos; los demás son constancias.
# C constancia, A acción del Centro, T gestión del Tribunal, F aclaración final.
ESTRUCTURA = {
    'ESPERA.E05_SOLO_CORREO': 'CA',
    'ESPERA.E05_PROYECTO_Y_CORREO': 'CA',
    'INFORMES.I01_VENCIDO_DCE': 'CA',
    'INFORMES.I01_VENCIDO_GENERAL': 'CA',
    'INFORMES.I02_POR_VENCER_DCE': 'A',
    'INFORMES.I02_POR_VENCER_GENERAL': 'A',
    'COMUN.CURADOR': 'CT',
    'CUMPLIMIENTO.C07_SIN_FICHA': 'CT',
    'CUMPLIMIENTO.C07_FICHA_ANTIGUA': 'CT',
    'CUMPLIMIENTO.C08_FICHA_FAE': 'CT',
    'HISTORIA.C10_ESCRITO': 'CA',
    'HISTORIA.T5_INSTITUCIONES': 'T',
    'HISTORIA.R3_MULTI': 'F',
}


class Frag(str):
    """Fragmento de observación con su grupo; `partes` lleva (texto, grupo) si el texto mezcla grupos."""

    grupo = 'C'
    partes = ()


def con_grupo(texto, grupo):
    if not texto:
        return texto
    fragmento = Frag(texto)
    fragmento.grupo = grupo
    return fragmento


def _armar(plantilla, letras, datos):
    if len(letras) == 1:
        return con_grupo(plantilla.format_map(datos), letras)
    oraciones = re.split(r'(?<=\.)\s+', plantilla.strip())
    if len(oraciones) != len(letras):
        # Texto editado por el usuario con otra cantidad de oraciones: se conserva entero, en el grupo del inicio.
        return con_grupo(plantilla.format_map(datos), letras[0])
    partes = tuple((oracion.format_map(datos), letra) for oracion, letra in zip(oraciones, letras))
    fragmento = Frag(plantilla.format_map(datos))
    fragmento.grupo = letras[0]
    fragmento.partes = partes
    return fragmento


def render(modo, id_regla, **datos):
    ctx = current.get()
    key = modo + '.' + id_regla
    if key in ctx['config']['desactivadas'] or key in ctx.get('invalid', ()):
        return ''
    ctx['events'].append(key)
    plantilla = ctx['config']['textos'][modo][id_regla]['texto']
    return _armar(plantilla, ESTRUCTURA.get(key, 'C'), datos)
