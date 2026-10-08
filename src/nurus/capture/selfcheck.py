"""Comprobación portable incluida en ambos ejecutables; solo fixture vacía."""
from .contract import TABLES,write_capture,read_capture


def check_contract(folder):
    metadata={'capture_id':'ficticia-verificacion-paquete','captured_at':'2026-10-08T00:00:00Z','downloader':{'version':'verificacion','commit':'SINTETICO'},'contains_sensitive_data':False,'originals_authorized':False,'queries':[],'table_states':{table:'VACIA_COMPROBADA' if table in ('consultas','registros','incidencias') else 'NO_SOLICITADA' for table in TABLES}}
    path=write_capture(folder/'Captura_verificacion.zip',metadata,{'consultas':[],'registros':[],'incidencias':[]})
    result=read_capture(path)
    return not result.rows('registros') and result.manifest['schema_version']=='1.0.0'
