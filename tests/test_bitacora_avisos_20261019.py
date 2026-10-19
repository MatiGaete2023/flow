"""Paquete 2 (19-oct-2026): avisos de bitácora solo mostrar CT-05 (B-03), CT-07 (B-01) y CT-11 (B-02)."""

from datetime import date

from nurus.capture.temporal import display_chronology
from nurus.personal.bitacora_avisos import avisos
from nurus.personal.capture_findings import last_center_label, novelties_text
from nurus.personal.config import defaults
from test_cronologia_solo_mostrar import importar

CORTE = date(2026, 10, 8)
TEXTOS = defaults()['textos']
CORREO = TEXTOS['ESPERA']['E05_SOLO_CORREO']['texto']


def _entrada(event_id, event_at, text='Texto ficticio.', event_type='Administrativa', response=''):
    return {'event_id': event_id, 'event_at': event_at, 'origin': 'CENTRO', 'origin_verified': 'true',
            'coverage': 'COMPLETA', 'text': text, 'event_type': event_type, 'response': response}


def test_ct07_avisa_cuando_la_propuesta_repite_la_ultima_observacion():
    previa = _entrada('a', '2026-09-20', 'Persona SIGLA: ' + CORREO)
    display = display_chronology([previa], CORTE)
    alertas = avisos(display, 'Persona SIGLA: ' + CORREO, [], TEXTOS, CORTE)
    assert [r for r, _ in alertas] == ['CT-07']
    assert '20/09/2026' in alertas[0][1]
    assert not avisos(display, 'Persona SIGLA: Medida revisada, sin observaciones.', [], TEXTOS, CORTE)


def test_ct11_sugiere_proyecto_si_el_correo_e05_tiene_30_dias_o_mas():
    correo = _entrada('a', '2026-09-08', 'Persona SIGLA: ' + CORREO)
    display = display_chronology([correo], CORTE)
    alertas = dict(avisos(display, 'Persona SIGLA: otra cosa', ['ESPERA.E05_SOLO_CORREO'], TEXTOS, CORTE))
    assert 'hace 30 días' in alertas['CT-11'] and '08/09/2026' in alertas['CT-11']


def test_ct11_no_avisa_antes_de_30_dias_ni_fuera_de_e05_solo_correo():
    reciente = display_chronology([_entrada('a', '2026-09-20', 'Persona SIGLA: ' + CORREO)], CORTE)
    assert 'CT-11' not in dict(avisos(reciente, 'x', ['ESPERA.E05_SOLO_CORREO'], TEXTOS, CORTE))
    antiguo = display_chronology([_entrada('a', '2026-09-01', 'Persona SIGLA: ' + CORREO)], CORTE)
    assert 'CT-11' not in dict(avisos(antiguo, 'x', ['ESPERA.E05_PROYECTO_Y_CORREO'], TEXTOS, CORTE))


def test_ct05_observacion_al_tribunal_sin_respuesta_desde_30_dias():
    entradas = [
        _entrada('vieja', '2026-09-01', event_type='Al Tribunal'),
        _entrada('respondida', '2026-08-20', event_type='Al Tribunal', response='Téngase presente.'),
        _entrada('reciente', '2026-09-30', event_type='Al Tribunal'),
        _entrada('admin', '2026-08-15', event_type='Administrativa'),
    ]
    display = display_chronology(entradas, CORTE)
    assert [e['event_id'] for e in display['pending_court']] == ['vieja']
    alertas = avisos(display, '', [], TEXTOS, CORTE)
    assert alertas == [('CT-05', 'Observación al tribunal del 01/09/2026 sin respuesta registrada (hace 37 días).')]


def test_sin_cronologia_acreditada_no_hay_avisos():
    assert avisos(None, 'x', [], TEXTOS, CORTE) == []
    assert avisos(display_chronology([], CORTE), 'x', [], TEXTOS, CORTE) == []


def test_avisos_aparecen_en_columna_y_panel_sin_cambiar_la_observacion(tmp_path):
    def cambiar(t):
        t['bitacoras'][0].update(event_at='2026-09-01', event_type='Al Tribunal', response='')

    work, rows = importar(tmp_path, cambiar)
    (tmp_path / 'base').mkdir()
    _, base = importar(tmp_path / 'base')
    fila = rows['r0']
    assert last_center_label(work, fila) == '01/09/2026 · 1 aviso'
    texto = novelties_text(work, fila)
    assert 'Avisos (solo mostrar' in texto and 'CT-05 · Observación al tribunal del 01/09/2026' in texto
    assert (fila.observation, fila.rules, fila.actions) == (base['r0'].observation, base['r0'].rules, base['r0'].actions)


def test_ct07_reitera_toda_la_observacion_repetida(tmp_path):
    (tmp_path / 'b0').mkdir()
    _, base = importar(tmp_path / 'b0')
    propuesta = base['r0'].observation

    def repetir(t):
        t['bitacoras'][0]['text'] = propuesta

    work, rows = importar(tmp_path, repetir)
    fila = rows['r0']
    prefijo = propuesta.partition(': ')[0] + ': '
    assert fila.observation == prefijo + 'Se reitera observación anterior del 10 de septiembre de 2026.'
    assert fila.rules[-1] == 'COMUN.REITERA_OBSERVACION'
    assert fila.actions == base['r0'].actions
    assert any(w.startswith('CT-07: la observación propuesta repetía la del 10/09/2026') and propuesta in w
               for w in fila.warnings)
    # La propuesta vigente es la reiteración: ya no corresponde el aviso CT-07.
    assert 'CT-07' not in dict(avisos(work.capture_context['r0']['display'], fila.observation, fila.rules, TEXTOS,
                                      CORTE))


def test_ct07_reiteracion_desactivable(tmp_path):
    from nurus.capture import write_capture
    from nurus.personal.capture_input import import_capture
    from test_capture_contract import fixture

    (tmp_path / 'b0').mkdir()
    _, base = importar(tmp_path / 'b0')
    m, t = fixture()
    t['bitacoras'][0]['text'] = base['r0'].observation
    config = defaults()
    config['desactivadas'] = ['COMUN.REITERA_OBSERVACION']
    work = import_capture(write_capture(tmp_path / 'c.zip', m, t), config, as_of=CORTE)
    fila = {r.id: r for r in work.rows}['r0']
    assert fila.observation == base['r0'].observation
    assert 'CT-07' in dict(avisos(work.capture_context['r0']['display'], fila.observation, fila.rules, TEXTOS, CORTE))
