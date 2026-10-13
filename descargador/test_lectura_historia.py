"""LectorHistoria: una lectura por causa, errores nunca como vacío, sin valores cifrados."""

import base64
import hashlib
import json
import threading
from pathlib import Path

import pytest
import lectura_historia
from lectura_historia import LectorHistoria
from lectura_bitacoras import DiaryRunner
from lotes import Batch
from perfiles import Selection, selection_profile
from test_lectura_bitacoras import Bridge as BridgeBitacoras, run as run_bitacoras
from test_lotes import CATALOG
from csmp_shared import parser

FIXTURES = Path(__file__).parent / 'fixtures' / 'historia_ficticia'
VENTANA = (FIXTURES / 'ventana_historia.html').read_bytes()
LISTADO = (FIXTURES / 'listado_seguimiento.html').read_bytes()
SELECCION = Selection('49', 'Cumplimiento', '2')
PERFIL = selection_profile(SELECCION, CATALOG)
QUERY = {'tribunal_nombre': 'Juzgado ficticio'}


class FakeBridge:
    connected = True

    def __init__(self, respuesta=VENTANA, error=None):
        self.calls, self.respuesta, self.error = [], respuesta, error

    def call(self, command, payload=None, timeout=75):
        assert command == 'history_open'
        self.calls.append(payload['params'])
        if self.error:
            raise self.error
        data = self.respuesta
        return {
            'status': 200,
            'type': 'text/html',
            'body': base64.b64encode(data).decode(),
            'digest': hashlib.sha256(data).hexdigest(),
            'params': payload['params'],
            'startedAt': '2099-10-13T12:00:00Z',
        }


def lector(tmp_path, bridge=None, cancel=None, anterior=None):
    parser()
    item = LectorHistoria(
        bridge or FakeBridge(), lambda *a: None, cancel or threading.Event(), pause=0, anterior=anterior
    )
    item.abrir(tmp_path)
    return item


def estado(item):
    return json.loads((item.carpeta / 'historias.json').read_text(encoding='utf-8'))


def test_dos_filas_misma_causa_una_llamada_y_ventana(tmp_path):
    parser()
    fila = LISTADO.decode().split('<tr><td><a href="#" onclick="ShowHistoria')[1].split('</tr>')[0]
    cuerpo = LISTADO.replace(
        b'</table></form>', ('<tr><td><a href="#" onclick="ShowHistoria' + fila + '</tr></table></form>').encode()
    )
    cuerpo = cuerpo.replace(
        b'</table></form>',
        b'<tr><td>C-77777-2099</td><td>x</td><td>Sin enlace</td><td>p</td><td></td></tr></table></form>',
        1,
    )
    bridge = FakeBridge()
    item = lector(tmp_path, bridge)
    item.pagina(SELECCION, QUERY, 1, cuerpo, PERFIL)
    assert len(bridge.calls) == 1 and bridge.calls[0] == {'tribunal': '49', 'rit': 'C-99999-2099', 'causa': '100'}
    datos = estado(item)
    (causa,) = datos['causas'].values()
    assert causa['estado'] == 'LEIDA' and causa['en_ventana'] == 3 and causa['anteriores'] == 0
    assert causa['incompletas'] == 1 and causa['tribunal_nombre'] == 'Juzgado ficticio'
    assert len(datos['vinculos']) == 2
    assert (item.carpeta / causa['archivo']).read_bytes() == VENTANA


def test_ningun_archivo_contiene_valores_cifrados(tmp_path):
    item = lector(tmp_path)
    item.pagina(SELECCION, QUERY, 1, LISTADO, PERFIL)
    item.cerrar()
    for ruta in tmp_path.rglob('*.json'):
        texto = ruta.read_text(encoding='utf-8')
        assert 'a1=' not in texto and 'crr_causa' not in texto


@pytest.mark.parametrize(
    'respuesta, codigo',
    [
        (b'<html><form><input type="password" name="clave"></form></html>', 'HISTORIA_SESION'),
        (b'<html><table></table></html>', 'HISTORIA_TABLA_NO_RECONOCIDA'),
    ],
)
def test_error_de_lectura_es_fallida_nunca_vacia(tmp_path, respuesta, codigo):
    item = lector(tmp_path, FakeBridge(respuesta))
    item.pagina(SELECCION, QUERY, 1, LISTADO, PERFIL)
    (causa,) = estado(item)['causas'].values()
    assert causa['estado'] == 'FALLIDA' and causa['error'] == codigo
    assert item.cerrar()['estado'] == 'INCOMPLETA'


def test_digest_distinto_y_error_de_extension_son_fallidas(tmp_path):
    class Digest(FakeBridge):
        def call(self, command, payload=None, timeout=75):
            result = super().call(command, payload, timeout)
            result['digest'] = '0' * 64
            return result

    item = lector(tmp_path, Digest())
    item.pagina(SELECCION, QUERY, 1, LISTADO, PERFIL)
    assert next(iter(estado(item)['causas'].values()))['estado'] == 'FALLIDA'
    import motor as m

    error = m.PocError('x')
    error.code = 'HISTORY_LINK'
    otro = lector(tmp_path / 'b' if (tmp_path / 'b').mkdir() is None else tmp_path, FakeBridge(error=error))
    otro.pagina(SELECCION, QUERY, 1, LISTADO, PERFIL)
    assert next(iter(estado(otro)['causas'].values()))['error'] == 'HISTORY_LINK'


def test_tope_deja_resto_pendiente(tmp_path, monkeypatch):
    monkeypatch.setattr(lectura_historia, 'MAX_HISTORIAS_POR_LOTE', 1)
    segunda = LISTADO.replace(b'100)', b'101)').replace(b'C-99999-2099', b'C-99998-2099')
    primera = LISTADO
    fila = segunda.decode().split('<tr><td><a href="#" onclick="ShowHistoria')[1].split('</tr>')[0]
    cuerpo = primera.replace(
        b'</table></form>', ('<tr><td><a href="#" onclick="ShowHistoria' + fila + '</tr></table></form>').encode()
    )
    bridge = FakeBridge()
    item = lector(tmp_path, bridge)
    item.pagina(SELECCION, QUERY, 1, cuerpo, PERFIL)
    datos = estado(item)
    assert len(bridge.calls) == 1 and datos['tope_alcanzado'] is True
    assert sorted(c['estado'] for c in datos['causas'].values()) == ['LEIDA', 'PENDIENTE']
    assert item.cerrar()['estado'] == 'INCOMPLETA'


def test_cancelar_no_hace_mas_llamadas(tmp_path):
    cancel = threading.Event()
    cancel.set()
    bridge = FakeBridge()
    item = lector(tmp_path, bridge, cancel)
    item.pagina(SELECCION, QUERY, 1, LISTADO, PERFIL)
    assert not bridge.calls
    assert item.cerrar()['estado'] == 'CANCELADA'


def test_anterior_solo_relee_fallidas(tmp_path):
    primero = lector(tmp_path / 'uno' if (tmp_path / 'uno').mkdir() is None else tmp_path, FakeBridge())
    primero.pagina(SELECCION, QUERY, 1, LISTADO, PERFIL)
    primero.cerrar()
    bridge = FakeBridge()
    segundo = lector(
        tmp_path / 'dos' if (tmp_path / 'dos').mkdir() is None else tmp_path,
        bridge,
        anterior=primero.carpeta / 'historias.json',
    )
    segundo.pagina(SELECCION, QUERY, 1, LISTADO, PERFIL)
    assert not bridge.calls
    (causa,) = estado(segundo)['causas'].values()
    assert causa['estado'] == 'LEIDA' and causa['reutilizada'] is True
    assert (segundo.carpeta / causa['archivo']).is_file()
    # una fallida anterior sí se vuelve a pedir
    datos = estado(primero)
    next(iter(datos['causas'].values()))['estado'] = 'FALLIDA'
    (primero.carpeta / 'historias.json').write_text(json.dumps(datos), encoding='utf-8')
    otro = FakeBridge()
    tercero = lector(
        tmp_path / 'tres' if (tmp_path / 'tres').mkdir() is None else tmp_path,
        otro,
        anterior=primero.carpeta / 'historias.json',
    )
    tercero.pagina(SELECCION, QUERY, 1, LISTADO, PERFIL)
    assert len(otro.calls) == 1


def test_error_del_listado_no_se_propaga(tmp_path):
    item = lector(tmp_path)
    item.pagina(SELECCION, QUERY, 1, b'<html>sin tabla</html>', PERFIL)
    assert len(estado(item)['errores_listado']) == 1
    assert item.cerrar()['estado'] == 'INCOMPLETA'


class BridgeConHistoria(BridgeBitacoras):
    def call(self, command, payload=None, timeout=75):
        if command == 'history_open':
            self.events.append((command, None))
            return FakeBridge().call(command, payload)
        return super().call(command, payload, timeout)


def estado_historia(carpeta):
    return json.loads((carpeta / 'historia' / 'historias.json').read_text(encoding='utf-8'))


def bitacoras(folder):
    datos = json.loads(Path(folder, 'bitacoras.json').read_text(encoding='utf-8'))
    for registro in datos['registros']:
        registro.pop('capturada', None)
    return datos['registros']


def test_bitacoras_identicas_con_y_sin_historia_y_excel(tmp_path):
    sin = run_bitacoras(tmp_path / 'sin' if (tmp_path / 'sin').mkdir() is None else tmp_path, BridgeConHistoria())
    puente = BridgeConHistoria()
    parser()
    lector_ = LectorHistoria(puente, lambda *a: None, threading.Event(), pause=0)
    (tmp_path / 'con').mkdir()
    con = DiaryRunner(puente, CATALOG, lambda *a: None, threading.Event(), pause=0, historia=lector_).run(
        Batch(('49',), ('2',), ('Cumplimiento',)), tmp_path / 'con'
    )
    assert bitacoras(sin['folder']) == bitacoras(con['folder'])
    assert sin['estado'] == con['estado'] == 'COMPLETA'
    assert len([e for e in puente.events if e[0] == 'history_open']) == 1
    carpeta = Path(con['folder'])
    assert (carpeta / 'historia' / 'historias.json').is_file() and (
        carpeta / 'historia' / 'diagnostico_historia.json'
    ).is_file()
    assert (carpeta / 'Historia_SITFA.xlsx').is_file() and 'excel_error' not in estado_historia(carpeta)
    assert not (Path(sin['folder']) / 'historia').exists()
