"""El extractor de HAR conserva estructura y descarta todo valor."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from extraer_estructura_har import main, summarize

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

    def test_nombres_arbitrarios_no_se_comparten(self):
        data = har()
        entry = data['log']['entries'][0]
        request = entry['request']
        request['url'] = 'https://tokenficticio.ejemplo.invalid/persona/' + RIT
        request['queryString'].append({'name': 'nombre_PERSONA_FICTICIA', 'value': 'ficticio'})
        request['postData']['text'] += '&post_PERSONA_FICTICIA=ficticio'
        entry['response']['content']['text'] += (
            '<input name="form_PERSONA_FICTICIA">'
            '<a onclick="mostrar_PERSONA_FICTICIA(11, 2)">Ver</a>'
        )
        result = summarize(data)
        text = json.dumps(result)
        for value in (RIT, 'tokenficticio', 'PERSONA_FICTICIA'):
            self.assertNotIn(value, text)
        request = result['peticiones'][0]
        self.assertEqual(request['host'], 'HOST_NO_RECONOCIDO')
        self.assertEqual(request['ruta'], 'RUTA_NO_RECONOCIDA')
        self.assertIn('CAMPO_NO_RECONOCIDO', request['campos_enviados'])
        self.assertIn('CAMPO_NO_RECONOCIDO', request['campos_formulario'])
        self.assertIn('ACCION_NO_RECONOCIDA/2/tipo=?', request['acciones_onclick'])
        self.assertIn('ShowObservaciones/6/tipo=11', request['acciones_onclick'])

    def test_parametros_post_y_rutas_parcialmente_conocidas(self):
        data = har()
        entry = data['log']['entries'][0]
        entry['request']['postData']['params'] = [
            {'name': 'txtRit', 'value': RIT},
            {'name': 'param_PERSONA_FICTICIA', 'value': NAME},
        ]
        cases = (
            ('/SITFAWEB/InformesDAction.do', '/SITFAWEB/InformesDAction.do'),
            ('/SITFAWEB/InformesDAction.do/' + RIT, 'RUTA_NO_RECONOCIDA'),
            ('/sitfa/reportes/' + RIT + '.xls', '/sitfa/reportes/ARCHIVO.xls'),
        )
        for path, expected in cases:
            with self.subTest(path=path):
                entry['request']['url'] = 'https://familia.pjud.cl' + path
                result = summarize(data)['peticiones'][0]
                self.assertEqual(result['host'], 'familia.pjud.cl')
                self.assertEqual(result['ruta'], expected)
                self.assertEqual(result['campos_enviados'], ['CAMPO_NO_RECONOCIDO', 'nombre', 'rit', 'txtRit'])
                self.assertNotIn('PERSONA_FICTICIA', json.dumps(result))
                self.assertNotIn(RIT, json.dumps(result))

    def test_metadatos_con_texto_no_se_comparten(self):
        data = har()
        entry = data['log']['entries'][0]
        entry['request']['method'] = 'METODO_PERSONA_FICTICIA'
        entry['request']['url'] = 'https://[url_invalida_PERSONA_FICTICIA'
        entry['response']['status'] = 'ESTADO_PERSONA_FICTICIA'
        entry['response']['content']['mimeType'] = 'application/PERSONA_FICTICIA'
        entry['response']['content']['size'] = 'BYTES_PERSONA_FICTICIA'
        result = summarize(data)['peticiones'][0]
        self.assertNotIn('PERSONA_FICTICIA', json.dumps(result))
        self.assertEqual(result['metodo'], 'METODO_NO_RECONOCIDO')
        self.assertEqual(result['tipo_contenido'], 'CONTENIDO_NO_RECONOCIDO')
        self.assertIsNone(result['estado_http'])
        self.assertIsNone(result['bytes_respuesta'])

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
