"""El extractor de HAR conserva estructura y descarta todo valor."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from extraer_estructura_har import main

COOKIE = 'ASP.NET_SessionId=cookieficticia9z8y7x'
RIT = 'X-987-2025'
NAME = 'PERSONA INVENTADA PRUEBA'
RUT = '11111111-0'  # Identidad ficticia con dígito verificador inválido.
HTML = (
    '<html><body><form><input name="txtRit" value="' + RIT + '"><select name="cboTribunal"></select></form>'
    '<table><tr><td>' + NAME + '</td><td>' + RUT + '</td>'
    "<td><a onclick=\"ShowObservaciones(11,'" + RIT + "','" + NAME + "','" + RUT + "',4,'x')\">Ver</a></td></tr></table>"
    '</body></html>'
)


def har():
    return {'log': {'entries': [{
        'request': {
            'method': 'POST',
            'url': 'https://sitfa.ejemplo.invalid/Consulta/Buscar?rit=' + RIT + '&nombre=' + NAME.replace(' ', '+'),
            'headers': [{'name': 'Cookie', 'value': COOKIE}],
            'cookies': [{'name': 'ASP.NET_SessionId', 'value': COOKIE}],
            'queryString': [{'name': 'rit', 'value': RIT}, {'name': 'nombre', 'value': NAME}],
            'postData': {'mimeType': 'application/x-www-form-urlencoded', 'text': 'txtRit=' + RIT + '&txtRut=' + RUT},
        },
        'response': {
            'status': 200,
            'headers': [{'name': 'Set-Cookie', 'value': COOKIE}],
            'content': {'mimeType': 'text/html; charset=utf-8', 'size': len(HTML), 'text': HTML},
        },
    }]}}


class ExtraerEstructuraHarTest(unittest.TestCase):
    def test_salida_sin_valores_y_con_acciones(self):
        with tempfile.TemporaryDirectory() as temp:
            source, target = Path(temp) / 'captura.har', Path(temp) / 'estructura.json'
            source.write_text(json.dumps(har()), encoding='utf-8')
            self.assertEqual(main([str(source), str(target)]), 0)
            text = target.read_text(encoding='utf-8')
        for secret in (COOKIE, 'cookieficticia', RIT, '987', NAME, 'INVENTADA', RUT, '11111111'):
            self.assertNotIn(secret, text)
        data = json.loads(text)
        request = data['peticiones'][0]
        self.assertEqual(data['tipo'], 'ESTRUCTURA_HAR_SIN_VALORES')
        self.assertIn('ShowObservaciones/6/tipo=11', request['acciones_onclick'])
        self.assertEqual(request['campos_enviados'], ['nombre', 'rit', 'txtRit', 'txtRut'])
        self.assertEqual(request['ruta'], '/Consulta/Buscar')

    def test_no_sobrescribe_salida(self):
        with tempfile.TemporaryDirectory() as temp:
            source, target = Path(temp) / 'captura.har', Path(temp) / 'estructura.json'
            source.write_text(json.dumps(har()), encoding='utf-8')
            target.write_text('previo', encoding='utf-8')
            self.assertEqual(main([str(source), str(target)]), 1)
            self.assertEqual(target.read_text(encoding='utf-8'), 'previo')


    def test_no_sobrescribe_salida_creada_durante_lectura(self):
        with tempfile.TemporaryDirectory() as temp:
            source, target = Path(temp) / 'captura.har', Path(temp) / 'estructura.json'
            source.write_text(json.dumps(har()), encoding='utf-8')
            read_text = Path.read_text

            def concurrent_write(path, *args, **kwargs):
                target.write_text('previo', encoding='utf-8')
                return read_text(path, *args, **kwargs)

            with patch.object(Path, 'read_text', autospec=True, side_effect=concurrent_write):
                self.assertEqual(main([str(source), str(target)]), 1)
            self.assertEqual(target.read_text(encoding='utf-8'), 'previo')


if __name__ == '__main__':
    unittest.main()
