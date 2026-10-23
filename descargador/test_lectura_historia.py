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
QUERY = {'tribunal_nombre': 'Juzgado Canario de Prueba'}


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
    assert causa['incompletas'] == 1 and causa['tribunal_nombre'] == QUERY['tribunal_nombre']
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
            # La ventana ficticia muestra el mismo RIT pedido (el lector coteja el RIT visible).
            rit = payload['params']['rit'].encode()
            return FakeBridge(
                respuesta=VENTANA.replace(b'C-99999-2099</div>', rit + b'</div>').replace(
                    b'Juzgado Canario de Prueba', CATALOG['tribunales'][payload['params']['tribunal']].encode()
                )
            ).call(command, payload)
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


def test_e3_rit_visible_distinto_falla_sin_guardar(tmp_path):
    bridge = FakeBridge(respuesta=VENTANA.replace(b'C-99999-2099</div>', b'C-11111-2099</div>'))
    item = lector(tmp_path, bridge)
    item.pagina(SELECCION, QUERY, 1, LISTADO, PERFIL)
    (causa,) = estado(item)['causas'].values()
    assert causa['estado'] == 'FALLIDA' and causa['error'] == 'HISTORIA_IDENTIDAD_VISIBLE'
    assert not list(item.carpeta.glob('historia-*.html'))


def test_e3_respuesta_cortada_falla(tmp_path):
    item = lector(tmp_path, FakeBridge(respuesta=VENTANA.split(b'</tbody>')[0]))
    item.pagina(SELECCION, QUERY, 1, LISTADO, PERFIL)
    (causa,) = estado(item)['causas'].values()
    assert causa['estado'] == 'FALLIDA' and causa['error'] == 'HISTORIA_RESPUESTA_INCOMPLETA'


def test_e3_rit_con_dos_causas_no_se_abre(tmp_path):
    parser()
    fila = LISTADO.decode().split('<tr><td><a href="#" onclick="ShowHistoria')[1].split('</tr>')[0]
    otra = fila.replace(',100)', ',101)')
    cuerpo = LISTADO.replace(
        b'</table></form>', ('<tr><td><a href="#" onclick="ShowHistoria' + otra + '</tr></table></form>').encode()
    )
    bridge = FakeBridge()
    item = lector(tmp_path, bridge)
    item.pagina(SELECCION, QUERY, 1, cuerpo, PERFIL)
    assert bridge.calls == []
    assert {c['estado'] for c in estado(item)['causas'].values()} == {'IDENTIDAD_INCIERTA'}


@pytest.mark.parametrize('cambio', [b'causa', b'rit'])
def test_e3_conflicto_de_identidad_entre_paginas_invalida_ambas(tmp_path, cambio):
    item = lector(tmp_path)
    item.pagina(SELECCION, QUERY, 1, LISTADO, PERFIL)
    siguiente = (
        LISTADO.replace(b',100)', b',101)') if cambio == b'causa' else LISTADO.replace(b'C-99999-2099', b'C-99998-2099')
    )
    item.pagina(SELECCION, QUERY, 2, siguiente, PERFIL)
    assert len(item.bridge.calls) == 1
    assert {c['estado'] for c in estado(item)['causas'].values()} == {'IDENTIDAD_INCIERTA'}
    assert all(v['identidad'] == 'INCIERTA' for v in estado(item)['vinculos'])
    assert all('en_ventana' not in c and 'anteriores' not in c for c in estado(item)['causas'].values())
    assert item.cerrar()['estado'] == 'INCOMPLETA'


def test_e3_tribunal_visible_distinto_no_guarda_html(tmp_path):
    item = lector(tmp_path)
    item.pagina(SELECCION, {'tribunal_nombre': 'Otro juzgado ficticio'}, 1, LISTADO, PERFIL)
    assert next(iter(estado(item)['causas'].values()))['error'] == 'HISTORIA_TRIBUNAL_VISIBLE'
    assert not list(item.carpeta.glob('*.html'))


def test_e3_permission_error_no_revela_ruta_en_diagnostico(tmp_path):
    item = lector(tmp_path, FakeBridge(error=PermissionError('/home/UsuarioCanario/archivo')))
    item.pagina(SELECCION, QUERY, 1, LISTADO, PERFIL)
    item.cerrar()
    diagnostico = (item.carpeta / 'diagnostico_historia.json').read_text()
    assert 'UsuarioCanario' not in diagnostico and '/home/' not in diagnostico
    assert json.loads(diagnostico)['codigos_error'] == {'HISTORIA_PERSISTENCIA': 1}


@pytest.mark.parametrize('operacion', ['abrir', 'pagina', 'cerrar'])
def test_e3_fallo_historia_no_afecta_bitacoras(tmp_path, monkeypatch, operacion):
    puente = BridgeConHistoria()
    item = LectorHistoria(puente, lambda *a: None, threading.Event(), pause=0)

    def falla(*args):
        raise PermissionError('/home/UsuarioCanario/no-escribible')

    monkeypatch.setattr(item, operacion, falla)
    parser()
    result = DiaryRunner(puente, CATALOG, lambda *a: None, threading.Event(), pause=0, historia=item).run(
        Batch(('49',), ('2',), ('Cumplimiento',)), tmp_path
    )
    assert result['estado'] == 'COMPLETA' and result['consultas_fallidas'] == 0
    assert item.resultado['estado'] == 'INCOMPLETA'
    assert 'UsuarioCanario' not in json.dumps(item.estado)


def test_e3_persistencia_historia_no_afecta_bitacoras(tmp_path, monkeypatch):
    original = lectura_historia.write

    def falla(path, datos):
        if Path(path).name == 'historias.json':
            raise PermissionError('/home/UsuarioCanario/no-escribible')
        return original(path, datos)

    monkeypatch.setattr(lectura_historia, 'write', falla)
    puente = BridgeConHistoria()
    item = LectorHistoria(puente, lambda *a: None, threading.Event(), pause=0)
    parser()
    result = DiaryRunner(puente, CATALOG, lambda *a: None, threading.Event(), pause=0, historia=item).run(
        Batch(('49',), ('2',), ('Cumplimiento',)), tmp_path
    )
    assert result['estado'] == 'COMPLETA' and result['consultas_fallidas'] == 0
    assert item.resultado['estado'] == 'INCOMPLETA'
    assert 'UsuarioCanario' not in json.dumps(item.estado)


def test_e3_reintento_explicito_solo_conserva_causas_del_listado_actual(tmp_path):
    primero = lector(tmp_path / 'uno')
    primero.pagina(SELECCION, QUERY, 1, LISTADO, PERFIL)
    primero.cerrar()
    segundo = lector(tmp_path / 'dos', anterior=primero.carpeta / 'historias.json')
    listado = LISTADO.replace(b',100)', b',101)').replace(b'C-99999-2099', b'C-99998-2099')
    segundo.pagina(SELECCION, QUERY, 1, listado, PERFIL)
    assert set(estado(segundo)['causas']) == {'RUS|49|101'}


def test_e3_fecha_posterior_tiene_advertencia(tmp_path):
    item = lector(tmp_path, FakeBridge(VENTANA.replace(b'13/10/2099</td>', b'14/10/2099</td>')))
    item.pagina(SELECCION, QUERY, 1, LISTADO, PERFIL)
    assert 'HISTORIA_FECHA_POSTERIOR' in next(iter(estado(item)['causas'].values()))['advertencias']


def test_e3_rit_del_enlace_distinto_en_mayusculas_se_envia_tal_cual(tmp_path):
    cuerpo = LISTADO.replace(b"ShowHistoria(49,'C-99999-2099',100)", b"ShowHistoria(49,'c-99999-2099',100)")
    bridge = FakeBridge()
    item = lector(tmp_path, bridge)
    item.pagina(SELECCION, QUERY, 1, cuerpo, PERFIL)
    assert bridge.calls == [{'tribunal': '49', 'rit': 'c-99999-2099', 'causa': '100'}]
    assert next(iter(estado(item)['causas'].values()))['estado'] == 'LEIDA'


def test_e3_posible_paginacion_queda_como_advertencia_visible(tmp_path):
    paginada = VENTANA.replace(b'</body>', b'<ul class="pagination"><li>2</li></ul></body>')
    item = lector(tmp_path, FakeBridge(paginada))
    item.pagina(SELECCION, QUERY, 1, LISTADO, PERFIL)
    (causa,) = estado(item)['causas'].values()
    assert causa['estado'] == 'LEIDA' and 'HISTORIA_POSIBLE_PAGINACION' in causa['advertencias']


def test_e3_error_inesperado_en_el_lote_igual_genera_excel_y_diagnostico(tmp_path):
    puente = BridgeConHistoria()
    lector_ = LectorHistoria(puente, lambda *a: None, threading.Event(), pause=0)

    class Falla(DiaryRunner):
        def leer_bitacora(self, *args, **kwargs):
            raise RuntimeError('inesperado ficticio')

    with pytest.raises(RuntimeError):
        Falla(puente, CATALOG, lambda *a: None, threading.Event(), pause=0, historia=lector_).run(
            Batch(('49',), ('2',), ('Cumplimiento',)), tmp_path
        )
    (lote,) = tmp_path.glob('Bitacoras *')
    assert (lote / 'Historia_SITFA.xlsx').is_file() and (lote / 'historia' / 'diagnostico_historia.json').is_file()
    assert json.loads((lote / 'historia' / 'historias.json').read_text(encoding='utf-8'))['estado'] == 'INCOMPLETA'
    assert json.loads((lote / 'bitacoras.json').read_text(encoding='utf-8'))['estado'] != 'EN_CURSO'


def test_reintento_con_otro_periodo_vuelve_a_leer_la_historia(tmp_path):
    # Revisión Codex en el PR #50 (21-oct-2026): una causa leída con 4 meses se reutilizaba al reintentar con 1 mes y
    # el paquete declaraba la cobertura anterior.
    primero = lector(tmp_path / 'uno')
    primero.pagina(SELECCION, QUERY, 1, LISTADO, PERFIL)
    primero.cerrar()
    bridge = FakeBridge()
    parser()
    segundo = LectorHistoria(
        bridge, lambda *a: None, threading.Event(), pause=0, anterior=primero.carpeta / 'historias.json', meses=1
    )
    segundo.abrir(tmp_path / 'dos')
    segundo.pagina(SELECCION, QUERY, 1, LISTADO, PERFIL)
    assert len(bridge.calls) == 1
    (causa,) = estado(segundo)['causas'].values()
    assert not causa.get('reutilizada') and estado(segundo)['meses'] == 1
