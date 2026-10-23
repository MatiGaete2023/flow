"""Contrato explícito de las piezas entregadas juntas; no consulta RUS."""

from pathlib import Path
import sys


def extension_dir():
    """Carpeta que Chrome debe cargar: junto al código o, en el EXE, en la raíz de la entrega (build_integral.ps1)."""
    if getattr(sys, 'frozen', False):
        return Path(sys.executable).resolve().parent.parent / 'extension'
    return Path(__file__).resolve().parent / 'extension'


PROTOCOLS = {
    'bitacoras_lectura': 1,
    'descarga_conjunta': 1,
    'pdf_seleccion': 1,
    'pdf_copia_fija': 1,
    'calendario_lectura': 1,
    'historia_lectura': 1,
}
VERSION = '2.8.1'


def capabilities():
    try:
        from _build_meta import BUILD_COMMIT, BUILD_VERSION
    except ImportError:
        BUILD_COMMIT = 'desarrollo'
        BUILD_VERSION = VERSION
    return {
        'tipo': 'CSMP_RUS_CAPACIDADES',
        'version': BUILD_VERSION,
        'commit': BUILD_COMMIT,
        'protocolos': dict(PROTOCOLS),
        'escritura_rus': False,
        'intercambio_local': {'CSMP_CAPTURE': '1.0.0', 'CSMP_REQUEST': '1.0.0'},
    }


def verify_extension(bridge):
    import motor as m

    try:
        result = bridge.call('capabilities')
    except m.PocError:
        raise m.PocError(
            'La extensión conectada no acredita el paquete integral. En chrome://extensions carga la carpeta extension de esta entrega, recárgala y abre una nueva conexión desde RUS.'
        ) from None
    if not isinstance(result, dict) or result.get('escritura_rus') is not False:
        raise m.PocError(
            'La extensión no incluye conjuntamente lectura de bitácoras e historia, descarga CSMP y PDF con selección restituida. Actualiza la extensión desde la carpeta de esta entrega y vuelve a conectar.'
        )
    if result.get('version') != VERSION or result.get('protocolos') != PROTOCOLS:
        # Chrome sigue usando la carpeta que se cargó la primera vez, aunque se descomprima una entrega nueva en otra.
        raise m.PocError(
            'Chrome usa una copia anterior de la extensión (declara '
            + str(result.get('version', 'sin versión'))
            + '; esta herramienta requiere '
            + VERSION
            + '). En chrome://extensions quita «SITFA · Descargador local integral» y pulsa Cargar descomprimida con la carpeta '
            + str(extension_dir())
            + '. Luego abre una nueva conexión desde SITFA.'
        )
    return {'version': result.get('version', 'sin versión'), 'protocolos': dict(PROTOCOLS)}
