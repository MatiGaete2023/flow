"""Comprobación portable incluida en ambos ejecutables; datos ficticios."""
from .contract import TABLES,write_capture,read_capture


def check_contract(folder):
    metadata={'capture_id':'ficticia-verificacion-paquete','captured_at':'2026-10-08T00:00:00Z','downloader':{'version':'verificacion','commit':'SINTETICO'},'contains_sensitive_data':False,'originals_authorized':False,'queries':[],'table_states':{table:'VACIA_COMPROBADA' if table in ('consultas','registros','incidencias') else 'NO_SOLICITADA' for table in TABLES}}
    path=write_capture(folder/'Captura_verificacion.zip',metadata,{'consultas':[],'registros':[],'incidencias':[]})
    result=read_capture(path)
    if result.rows('registros') or result.manifest['schema_version']!='1.0.0':
        return False
    from .fixtures import fixture
    from hashlib import sha256
    for partial in (False,True):
        metadata,tables=fixture(partial=partial)
        path=write_capture(folder/('Captura_parcial.zip' if partial else 'Captura_completa.zip'),metadata,tables)
        result=read_capture(path)
        if result.digest!=sha256(path.read_bytes()).hexdigest():return False
        if len(result.rows('registros'))!=4:return False
        if {r['modality_code'] for r in result.rows('registros')}!={'1','2','3','4'}:return False
        if len(result.rows('bitacoras'))!=(0 if partial else 4):return False
        if result.manifest['table_states']['bitacoras']!=('NO_CONSULTADA' if partial else 'COMPLETA'):return False
        if result.manifest['contains_sensitive_data'] is not False:return False
    return True
