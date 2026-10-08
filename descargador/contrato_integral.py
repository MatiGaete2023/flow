"""Contrato explícito de las piezas entregadas juntas; no consulta RUS."""
PROTOCOLS={'bitacoras_lectura':1,'descarga_conjunta':1,'pdf_seleccion':1}
VERSION='2.7.2'

def capabilities():
    try:from _build_meta import BUILD_COMMIT,BUILD_VERSION
    except ImportError:BUILD_COMMIT='desarrollo';BUILD_VERSION=VERSION
    return {'tipo':'CSMP_RUS_CAPACIDADES','version':BUILD_VERSION,'commit':BUILD_COMMIT,
            'protocolos':dict(PROTOCOLS),'escritura_rus':False}

def verify_extension(bridge):
    import motor as m
    try:result=bridge.call('capabilities')
    except m.PocError:
        raise m.PocError('La extensión conectada no acredita el paquete integral. En chrome://extensions carga la carpeta extension de esta entrega, recárgala y abre una nueva conexión desde RUS.') from None
    if not isinstance(result,dict) or result.get('protocolos')!=PROTOCOLS or result.get('escritura_rus') is not False:
        raise m.PocError('La extensión no incluye conjuntamente lectura de bitácoras, descarga CSMP y PDF con selección restituida. Actualiza la extensión desde la carpeta de esta entrega y vuelve a conectar.')
    if result.get('version')!=VERSION:
        raise m.PocError('La extensión declara una versión distinta de '+VERSION+'. Desactiva las copias anteriores, carga extension de este mismo paquete y vuelve a conectar.')
    return {'version':result.get('version','sin versión'),'protocolos':dict(PROTOCOLS)}
