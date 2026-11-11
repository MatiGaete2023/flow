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
    return {
        'event_id': event_id,
        'event_at': event_at,
        'origin': 'CENTRO',
        'origin_verified': 'true',
        'coverage': 'COMPLETA',
        'text': text,
        'event_type': event_type,
        'response': response,
    }


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
    assert (fila.observation, fila.rules, fila.actions) == (
        base['r0'].observation,
        base['r0'].rules,
        base['r0'].actions,
    )


def _reiterar(previas, propuesta, reglas=(), acciones=(), excluida=False):
    from nurus.personal.bitacora_avisos import aplicar_reiteracion
    from nurus.personal.work import Row

    fila = Row('r', 2, {}, propuesta, list(reglas), list(acciones), [], excluida)
    eventos = [_entrada(str(n), fecha, texto) for n, (fecha, texto) in enumerate(previas)]
    aplicar_reiteracion([fila], {'r': {'display': display_chronology(eventos, CORTE)}}, defaults())
    return fila


def test_ct07_reitera_solo_una_observacion_identica_y_sin_gestion():
    texto = 'Persona PRM: Medida vencida en RUS desde el 1 de septiembre de 2026.'
    fila = _reiterar([('2026-09-10', texto)], texto, ['CUMPLIMIENTO.C04_VENCIDA'], ['medidas'])
    assert fila.observation == 'Persona PRM: Se reitera observación anterior del 10 de septiembre de 2026.'
    assert fila.rules[-1] == 'COMUN.REITERA_OBSERVACION' and fila.actions == ['medidas']
    assert any(
        w.startswith('CT-07: la observación propuesta repetía la del 10/09/2026') and texto in w for w in fila.warnings
    )
    # Cambia un dato (la fecha): es una novedad y se escribe completa.
    otra = texto.replace('1 de septiembre', '2 de septiembre')
    assert _reiterar([('2026-09-10', texto)], otra, ['CUMPLIMIENTO.C04_VENCIDA']).observation == otra


def test_ct07_no_reitera_gestiones_del_centro_ni_derivaciones_sin_seguimiento():
    for regla, cuerpo in [
        ('ESPERA.E05_SOLO_CORREO', CORREO),
        ('ESPERA.E05_PROYECTO_Y_CORREO', TEXTOS['ESPERA']['E05_PROYECTO_Y_CORREO']['texto']),
        ('INFORMES.I01_VENCIDO_GENERAL', 'Informe de avance vencido desde el 1 de julio de 2026. Se deja constancia…'),
        ('INFORMES.I02_POR_VENCER_GENERAL', 'Se remite correo al programa PRM UNO informando…'),
        ('HISTORIA.C10_ESCRITO', 'Informe de avance recepcionado…'),
        ('HISTORIA.P_FINALIZADA', TEXTOS['HISTORIA']['P_FINALIZADA']['texto']),
        ('ESPERA.E07_PROYECCION', 'Medida revisada, a la espera de ingreso efectivo. Programa proyecta ingreso…'),
        ('COMUN.NO_SEGUIMIENTO', 'La derivación OPD no se encuentra sujeta a seguimiento por este Centro.'),
    ]:
        texto = 'Persona PRM: ' + cuerpo
        assert _reiterar([('2026-09-10', texto)], texto, [regla]).observation == texto, regla
    sin = 'Persona OPD: La derivación OPD no se encuentra sujeta a seguimiento por este Centro.'
    assert _reiterar([('2026-09-10', sin)], sin, [], excluida=True).observation == sin


def test_ct07_compara_con_la_ultima_que_no_es_reiteracion_y_no_encadena():
    texto = 'Persona PRM: Medida revisada, sin observaciones.'
    previas = [
        ('2026-08-10', texto),
        ('2026-09-10', 'Persona PRM: Se reitera observación anterior del 10 de agosto de 2026.'),
    ]
    # Sin alternancia: la última entrada es una reiteración y se compara con la anterior completa.
    fila = _reiterar(previas, texto, ['CUMPLIMIENTO.C09_SIN_OBSERVACIONES'])
    assert fila.observation == 'Persona PRM: Se reitera observación anterior del 10 de agosto de 2026.'
    # Una reiteración por materia no se reemplaza por otra reiteración.
    materia = 'Persona PRM: No registra curador ad litem en RUS; se reitera la sugerencia formulada en la observación del 1 de agosto de 2026.'
    assert _reiterar([('2026-09-10', materia)], materia, ['HISTORIA.REITERA', 'COMUN.CURADOR']).observation == materia


def test_ct07_en_el_paquete_no_reitera_la_espera_con_correo_y_proyecto(tmp_path):
    (tmp_path / 'b0').mkdir()
    _, base = importar(tmp_path / 'b0')
    propuesta = base['r0'].observation

    def repetir(t):
        t['bitacoras'][0]['text'] = propuesta

    _, rows = importar(tmp_path, repetir)
    assert rows['r0'].observation == propuesta and rows['r0'].actions == base['r0'].actions
    assert 'COMUN.REITERA_OBSERVACION' not in rows['r0'].rules


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
