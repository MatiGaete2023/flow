"""Lectura de solicitudes focalizadas: no navega ni ejecuta órdenes del archivo."""
from nurus.capture.requests import read_request


def summarize_request(path):
    data=read_request(path)
    grouped={}
    for request in data['requests']:
        key=(request['court_code'],request['source'])
        grouped[key]=grouped.get(key,0)+1
    lines=['Solicitud declarativa validada. Capture las fuentes seleccionadas desde el Descargador con su sesión autorizada.','No se ejecutan consultas desde este archivo.']
    lines.extend(f'{court} · {source}: {count} ingresos' for (court,source),count in sorted(grouped.items()))
    if any(item['source'] in ('historia_tramites','documentos') for item in data['requests']):lines.append('Historia/PDF: extractor y aceptación institucional pendientes; no sustituir por bitácoras.')
    return '\n'.join(lines)
