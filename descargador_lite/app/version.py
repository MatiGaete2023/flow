"""Version unica de SITFA Lite; la extension debe declarar la misma."""

from pathlib import Path
import sys

VERSION = '1.0.0'
EXTENSION_TYPE = 'SITFA_LITE_CAPACIDADES'
PROTOCOLS = {'descarga_xls': 1, 'pdf_sin_registros': 1}


def extension_dir():
    """Carpeta que Chrome debe cargar: junto al EXE instalado o, en desarrollo, junto a app/."""
    if getattr(sys, 'frozen', False):
        return Path(sys.executable).resolve().parent / 'extension'
    return Path(__file__).resolve().parent.parent / 'extension'


def verify_extension(bridge):
    import motor as m

    try:
        result = bridge.call('capabilities')
    except m.PocError:
        raise m.PocError(
            'La extensión conectada no es SITFA Lite. En chrome://extensions carga la carpeta extension de SITFA Lite y vuelve a conectar.'
        ) from None
    if (
        not isinstance(result, dict)
        or result.get('tipo') != EXTENSION_TYPE
        or result.get('escritura_rus') is not False
        or result.get('version') != VERSION
        or result.get('protocolos') != PROTOCOLS
    ):
        raise m.PocError(
            'Chrome usa otra versión de la extensión (requiere SITFA Lite '
            + VERSION
            + '). En chrome://extensions quita la anterior y carga la carpeta '
            + str(extension_dir())
            + '.'
        )
    return result
