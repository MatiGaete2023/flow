import json
from pathlib import Path

from extraer_estructura_historia import forma, fugas, main

FIXTURES = Path(__file__).parent / 'fixtures' / 'historia_ficticia'
CANARIOS = (
    'NOMBRECANARIO',
    'APELLIDOCANARIO',
    'C-99999-2099',
    '99- 9-9999999-9',
    '9999999',
    'TOKENCANARIO',
    'TOKENDOCCANARIO',
    'TOKENANEXOCANARIO',
    'REQCANARIO',
    'CANARIOREF',
    'Juzgado Canario',
    'Centro Canario',
    'Tengase presente canario',
    '13/10/2099',
    '01/01/2099',
    '9120',
    'TOKENDOCCANARIO4',
    '11.111.111-1',
)


def har(path):
    entrada = {
        'request': {
            'method': 'GET',
            'url': 'https://familia.pjud.cl/SITFAWEB/ConsultaDetalleAtPublicoAccion.do?TipoPopup=3&a1=TOKENCANARIO%2Bxyz%3D',
            'headers': [{'name': 'Cookie', 'value': 'JSESSIONID=COOKIECANARIO'}],
        },
        'response': {
            'status': 200,
            'headers': [{'name': 'Set-Cookie', 'value': 'COOKIECANARIO'}],
            'content': {
                'mimeType': 'text/html; charset=windows-1252',
                'size': 1234,
                'text': (FIXTURES / 'ventana_historia.html').read_text(encoding='utf-8'),
            },
        },
    }
    previa = {
        'request': {
            'method': 'POST',
            'url': 'https://familia.pjud.cl/SITFAWEB/HistoriaIntermediaAccion.do',
            'postData': {'mimeType': 'application/x-www-form-urlencoded', 'text': 'rit=C-99999-2099&causa=100'},
        },
        'response': {
            'status': 302,
            'headers': [
                {'name': 'Location', 'value': '/SITFAWEB/ConsultaDetalleAtPublicoAccion.do?TipoPopup=3&a1=TOKENCANARIO'}
            ],
            'content': {'mimeType': 'text/html', 'size': 0},
        },
    }
    path.write_text(json.dumps({'log': {'entries': [previa, entrada]}}), encoding='utf-8')
    return path


def ejecutar(tmp_path, *archivos):
    salida = tmp_path / 'estructura.json'
    assert main(['-o', str(salida), *map(str, archivos)]) == 0
    return salida.read_text(encoding='utf-8')


def test_ventana_historia_sin_valores(tmp_path):
    texto = ejecutar(tmp_path, FIXTURES / 'ventana_historia.html')
    for canario in CANARIOS:
        assert canario not in texto, canario
    copia = json.loads(texto)['copias'][0]
    tabla = next(t for t in copia['tablas'] if t['encabezado'][:1] == ['Folio'])
    assert tabla['encabezado'] == [
        'Folio',
        'Doc.',
        'Sel',
        'Anexo',
        'Etapa',
        'Trámite',
        'Desc. Trámite',
        'Referencia',
        'Fecha Tram',
    ]
    assert tabla['id'] == '' and tabla['clase'].startswith('table ')
    assert tabla['filas_de_datos'] == 4 and tabla['filas_con_celdas_distintas_al_encabezado'] == 1
    columnas = {c['etiqueta']: c for c in tabla['columnas']}
    assert columnas['Fecha Tram']['formas'] == {'FECHA_DD/MM/AAAA': 3}
    assert columnas['Folio']['formas'] == {'ENTERO_4-6': 4}
    assert columnas['Referencia']['formas']['VALOR_CONOCIDO:Sin referencia'] == 1
    assert columnas['Doc.']['acciones'] == {'Reservado/1:str': 1, 'ShowPDF/2:str,str': 3}
    assert columnas['Anexo']['acciones'] == {'ShowAnexos/2:num,num': 1}
    assert copia['pestanas_visibles'] == {'Diligencias': 1, 'Escritos': 1, 'Historia': 1, 'Notificación': 1}
    assert copia['charset_declarado'] == 'windows-1252'
    assert 'RIT' in copia['etiquetas_reconocidas'] and 'Tribunal' in copia['etiquetas_reconocidas']
    assert 'InformeHistoria' in copia['script_en_linea']['funciones_definidas']
    assert copia['lector_historia'] == {
        'filas': 3,
        'vacio_explicito': False,
        'advertencias': {'HISTORIA_FILA_INCOMPLETA': 1},
        'campos_encabezado': sorted(
            [
                'RIT',
                'RUC',
                'Caratulado',
                'F. Ingreso',
                'Proc.',
                'Forma Inicio',
                'Est. Adm.',
                'Etapa',
                'Estado Proc.',
                'Tribunal',
            ]
        ),
        'reservada': True,
        'anexos': 1,
    }


def test_listado_y_funcion_show_historia(tmp_path):
    texto = ejecutar(tmp_path, FIXTURES / 'listado_seguimiento.html')
    for canario in CANARIOS:
        assert canario not in texto, canario
    copia = json.loads(texto)['copias'][0]
    assert copia['acciones_onclick']['ShowHistoria/3:num,str,num'] == 1
    assert copia['acciones_onclick']['ShowObservaciones/6:num,num,str,num,num,str'] == 1
    esqueleto = copia['script_en_linea']['esqueletos']['ShowHistoria']
    assert '/SITFAWEB/ConsultaDetalleAtPublicoAccion.do?TipoPopup=VALOR&a1=VALOR' in esqueleto
    assert '/SITFAWEB/Encrypt_AjaxAccion.do?opcion=VALOR&TIP_Causa=VALOR' in esqueleto
    # Un argumento de código igual a un id de tabla emitido (Lengueta('tablaCumplimiento')) no bloquea la salida.
    assert copia['acciones_onclick']['Lengueta/1:str'] == 1
    assert 'window.open' in esqueleto and '$.ajax' in esqueleto and '"&B4=VALOR&FLG_Encripta=VALOR"' in esqueleto
    assert copia['formularios'][0]['accion']['ruta'] == '/SITFAWEB/InformesDAction.do'


def test_har_rutas_parametros_y_redireccion_sin_valores(tmp_path):
    texto = ejecutar(tmp_path, har(tmp_path / 'c.har'))
    for canario in (*CANARIOS, 'COOKIECANARIO', 'JSESSIONID'):
        assert canario not in texto, canario
    previa, ventana = json.loads(texto)['copias'][0]['peticiones']
    assert previa['url']['ruta'] == '/SITFAWEB/HistoriaIntermediaAccion.do'
    assert [c['nombre'] for c in previa['campos_post']] == ['rit', 'causa']
    assert previa['redireccion']['ruta'] == '/SITFAWEB/ConsultaDetalleAtPublicoAccion.do'
    assert ventana['url']['parametros'] == [
        {'nombre': 'TipoPopup', 'forma': 'DIGITOS/1-3'},
        {'nombre': 'a1', 'forma': 'BASE64_O_CODIFICADO/13-64'},
    ]
    assert ventana['html']['tablas']


def test_fuga_detectada_impide_escribir(tmp_path, monkeypatch):
    import extraer_estructura_historia as modulo

    monkeypatch.setattr(modulo, 'forma', lambda texto: str(texto))
    salida = tmp_path / 'estructura.json'
    assert modulo.main(['-o', str(salida), str(FIXTURES / 'ventana_historia.html')]) == 3
    assert not salida.exists()


def test_no_sobrescribe_y_formas_basicas(tmp_path):
    salida = tmp_path / 'estructura.json'
    salida.write_text('previo', encoding='utf-8')
    assert main(['-o', str(salida), str(FIXTURES / 'ventana_historia.html')]) == 1
    assert salida.read_text(encoding='utf-8') == 'previo'
    assert forma('') == 'VACIO' and forma('C-1-2026') == 'FORMA_RIT' and forma('Sin referencia.').startswith('VALOR')
    assert fugas('{"x": "NOMBRECANARIO"}', {'NOMBRECANARIO'}) == [13]


def test_carpeta_y_comodines(tmp_path):
    carpeta = tmp_path / 'listado_files'
    carpeta.mkdir()
    (carpeta / 'funciones.js').write_text(
        "function ShowHistoria(a,b,c){window.open('/SITFAWEB/ConsultaDetalleAtPublicoAccion.do?TipoPopup=3&a1='+a,'x');}",
        encoding='utf-8',
    )
    (carpeta / 'imagen.gif').write_bytes(b'GIF89a')
    texto = ejecutar(tmp_path, carpeta, str(FIXTURES / '*.html'))
    copias = json.loads(texto)['copias']
    assert [c['tipo'] for c in copias] == ['JS', 'HTML', 'HTML']
    assert 'ShowHistoria' in copias[0]['esqueletos']


def test_har_con_html_windows1252_en_base64(tmp_path):
    import base64

    html = (FIXTURES / 'ventana_historia.html').read_text(encoding='utf-8').encode('cp1252')
    entrada = {
        'request': {'method': 'GET', 'url': 'https://familia.pjud.cl/SITFAWEB/ConsultaDetalleAtPublicoAccion.do'},
        'response': {
            'status': 200,
            'content': {'mimeType': 'text/html', 'encoding': 'base64', 'text': base64.b64encode(html).decode()},
        },
    }
    path = tmp_path / 'b.har'
    path.write_text(json.dumps({'log': {'entries': [entrada]}}), encoding='utf-8')
    texto = ejecutar(tmp_path, path)
    for canario in CANARIOS:
        assert canario not in texto, canario
    tablas = json.loads(texto)['copias'][0]['peticiones'][0]['html']['tablas']
    assert any(t['encabezado'][:1] == ['Folio'] for t in tablas)


def test_fuga_de_dato_con_forma_de_identificador_sigue_bloqueando(tmp_path):
    import extraer_estructura_historia as modulo

    sensibles = modulo.Sensibles()
    sensibles.add('ROJASCANARIO')
    sensibles.codigo.update({'COD_Lengueta', 'texto con espacios canario'})
    modulo.EMITIDOS.clear()
    assert modulo.fugas('{"id": "ROJASCANARIO"}', sensibles) == [12]
    assert modulo.fugas('{"campo": "COD_Lengueta"}', sensibles) == []
    assert modulo.fugas('{"x": "texto con espacios canario"}', sensibles) == [26]
    assert modulo.fugas('{"bytes": null, "x": "ShowHistorialFichas"}', {'null', 'Historial'}) == []


def test_casos_adversariales_de_la_revision_e3(tmp_path):
    pagina = tmp_path / 'adversarial.html'
    pagina.write_text(
        """<html><head><script>
// revisar "x" caso de Zacarias Fulanito
// nota: Ana Beltrana dijo 'ok'
// O'Higgins Canario
var patron = /Rigoberta Ficticia/g;
/* bloque con Hermenegildo Canario */
function ShowHistoria(a, b, c){ var z = 'Petronila Canaria'; window.open('/SITFAWEB/X.do?a1=' + a, 'v'); }
</script></head><body>
<form name="f" method="Clotildo" action="buscar.do?Clotilde=1&TIP_Causa=2"><input name="x" value="1"></form>
<a onclick="alerta('Causa Menganito Ficticio(Zutanez)')">x</a>
<table><tr><th>Folio</th></tr><tr><td>1</td></tr></table>
</body></html>""",
        encoding='utf-8',
    )
    texto = ejecutar(tmp_path, pagina)
    for dato in (
        'Zacarias',
        'Fulanito',
        'Beltrana',
        'Higgins',
        'Rigoberta',
        'Hermenegildo',
        'Petronila',
        'Menganito',
        'Ficticio',
        'Zutanez',
        'Clotilde',
        'Clotildo',
    ):
        assert dato not in texto, dato
    copia = json.loads(texto)['copias'][0]
    assert copia['acciones_onclick'] == {'alerta/1:str': 1}
    esqueleto = copia['script_en_linea']['esqueletos']['ShowHistoria']
    assert 'window.open("/SITFAWEB/X.do?a1=VALOR"' in esqueleto
    formulario = copia['formularios'][0]
    assert formulario['metodo'] == 'OTRO'
    assert [p['nombre'] for p in formulario['accion']['parametros']] == ['CAMPO_NO_RECONOCIDO', 'TIP_Causa']
