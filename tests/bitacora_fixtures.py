"""Bitácora RUS ficticia (popup y control de lectura) compartida por pruebas y herramientas."""

from hashlib import sha256
import html
import json

from nurus.bitacora_html import HEADERS

PARAMS = {
    'tipo_popUp': '12',
    'COD_Tribunal': '11',
    'CRR_IdCausa': '22',
    'ID_Ingreso': '33',
    'COD_Etapa': '3',
    'TIP_Consulta': '2',
}


def popup(text='Texto completo que la tabla oculta.', response='---', kind='Al tribunal'):
    fields = {
        **PARAMS,
        'tipo_popUp': '',
        'RIT_Causa': 'P-12-2026',
        'GLS_Nombre': 'Persona de prueba',
        'GLS_Centro': 'Centro de prueba',
    }
    inputs = ''.join(
        f'<input type="hidden" name="{k}" value="{html.escape(v, quote=True)}">' for k, v in fields.items()
    )
    hidden = {
        'HOBS_Centro_': '',
        'HOBS_Tribunal_': response,
        'HCOD_Estado_': '6',
        'HFUN_Tribunal_': 'USR',
        'HFLG_Envio_': '1',
    }
    hidden['HOBS_Centro_'] = text
    hidden = ''.join(
        f'<input type="hidden" name="{k}44" value="{html.escape(v, quote=True)}">' for k, v in hidden.items()
    )
    headers = ''.join(f'<td>{h}</td>' for h in HEADERS)
    values = [kind, 'Cumplimiento', '12/08/2026 10:13', 'Centro', 'Texto recortado', '---', '---', 'Recortado']
    row = (
        ''.join(f'<td>{v}</td>' for v in values) + '<td><img id="44" onclick="MuestraDiv(this.id,3)"><img id="44"></td>'
    )
    return f'<html><body><form name="InformesPpalForm" action="/SITFAWEB/InformesDAction.do">{inputs}<table id="TablaInforme"><tr>{headers}</tr><tr>{row}</tr>{hidden}</table></form></body></html><script>/* RUS añade un script al final */</script>'


def manifest(tmp_path):
    body = popup().encode()
    (tmp_path / 'captura.html').write_bytes(body)
    record = {
        'tribunal_codigo': '11',
        'causa_id': '22',
        'ingreso_id': '33',
        'rit': 'P-12-2026',
        'nombre': 'Persona de prueba',
        'programa': 'Centro de prueba',
        'estado': 'LEIDA',
        'params': PARAMS,
        'archivo': 'captura.html',
        'sha256': sha256(body).hexdigest(),
        'capturada': '2026-10-08T12:00:00Z',
        'modalidad_nombre': 'FAE',
        'pestana': 'Cumplimiento',
        'pagina': 1,
        'fila': 2,
    }
    data = {
        'version': 1,
        'tipo': 'BITACORAS_LECTURA',
        'estado': 'INCOMPLETA',
        'fecha': '2026-10-08',
        'seleccion': {'tribunals': ['11'], 'modalities': ['1', '2', '3', '4'], 'tabs': ['Cumplimiento']},
        'consultas': [
            {
                'tribunal': '11',
                'modality': '3',
                'tab': 'Cumplimiento',
                'estado': 'ENUMERADA',
                'registros': 3,
                'paginas': 1,
            }
        ],
        'registros': [
            record,
            {**record, 'ingreso_id': '34', 'estado': 'FALLIDA', 'error': 'Sesión vencida'},
            {**record, 'ingreso_id': '', 'causa_id': '', 'estado': 'SIN_VINCULO', 'error': 'Sin enlace comprobado'},
        ],
    }
    path = tmp_path / 'bitacoras.json'
    path.write_text(json.dumps(data), encoding='utf-8')
    return path
