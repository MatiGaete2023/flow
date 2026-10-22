"""WP-R7: reglas de historia en Informes (I1, I2, I3, J2) y recorte de I-01."""

from historia_fixtures import correr, fila, hist, inf, INFORMES


def test_i1_contradice_i01_sin_frase_de_envio_y_marca_nivel2(tmp_path):
    f = inf(tmp_path, fila(1) + ['2026-09-20'], hist(('Ingreso informe de cumplimiento X', '2026-09-25', 58)))
    assert f.observation.split(': ', 1)[1] == (
        'Informe de avance vencido desde el 20 de septiembre de 2026. '
        'Se hace presente que en resolución de fecha 25 de septiembre de 2026 (folio 58), se ingresa el informe de cumplimiento.'
    )
    assert f.actions == [] and [a[:2] for a in f.avisos] == [(2, 'I01_CONTRADICHA')]
    dce = inf(
        tmp_path, fila(1, 'DCE UNO') + ['2026-09-20'], hist(('Ingreso informe de cumplimiento X', '2026-09-20', 58))
    )
    assert 'Informe diagnóstico vencido desde el 20 de septiembre de 2026.' in dce.observation


def test_i1_ingreso_anterior_al_vencimiento_no_desmiente(tmp_path):
    f = inf(tmp_path, fila(1) + ['2026-09-20'], hist(('Ingreso informe de cumplimiento X', '2026-07-01', 30)))
    assert f.rules == ['INFORMES.I01_VENCIDO_GENERAL'] and f.actions == ['programa_vencido', 'PC_INFO']
    assert [a[:2] for a in f.avisos] == [(2, 'ENTREGA_ANTERIOR')] and 'folio 30' not in f.observation


def test_prorroga_no_agrega_nada_en_informes(tmp_path):
    vencido = inf(tmp_path, fila(1) + ['2026-09-20'], hist(('Prorroga', '2026-09-25', 58)))
    base = inf(tmp_path, fila(1) + ['2026-09-20'])
    assert (vencido.observation, vencido.rules, vencido.actions, vencido.avisos) == (
        base.observation,
        base.rules,
        base.actions,
        base.avisos,
    )
    por_vencer = inf(tmp_path, fila(1) + ['2026-10-20'], hist(('Prorroga', '2026-10-01', 9)))
    assert por_vencer.observation.split(': ', 1)[1] == (
        'Se remite correo al programa PRM UNO informando que el próximo informe de avance vence el 20 de octubre de 2026.'
    )
    assert por_vencer.rules == ['INFORMES.I02_POR_VENCER_GENERAL'] and por_vencer.actions == ['programa_por_vencer']


def test_informes_solo_hace_constar_lo_de_informes(tmp_path):
    # Continuidad, interconexión, curador, oído y audiencia se hacen constar en Espera o Cumplimiento.
    historia = hist(
        ('Resp. continuidad de atención', '2026-10-01', 4),
        ('Deja sin efecto medida interconexión M.N.', '2026-10-02', 5),
        ('Designa abogado', '2026-10-03', 6),
        ('Derecho a ser oído', '2026-10-04', 7),
    )
    columnas = [*INFORMES, 'CURADOR', 'PROXIMA AUDIENCIA']
    dos = (
        '(11-1)ABOG A - Institución: Programa Mi Abogado#(22-2)ABOG B - Institución: Corporación de Asistencia Judicial'
    )
    f = correr(tmp_path, 'INFORMES', columnas, [[*fila(1), '2026-09-20', dos, '2026-11-01']], {0: historia}).rows[0]
    base = inf(tmp_path, fila(1) + ['2026-09-20'])
    assert (f.observation, f.rules, f.actions, f.avisos) == (base.observation, base.rules, base.actions, base.avisos)


def test_i3_escrito_sin_ingreso_en_informes(tmp_path):
    f = inf(tmp_path, fila(1) + ['2026-09-20'], hist(('Inf. Cump Programa Ambulatorio', '2026-10-01', 30)))
    assert 'recepcionado en resolución de fecha 1 de octubre de 2026 (folio 30)' in f.observation and f.actions == [
        'NOMENCL'
    ]


def test_j2_informes_vencidos_acumulados(tmp_path):
    filas = [
        fila(1, rit='C-1-2026') + ['2026-08-20'],
        fila(2, rit='C-1-2026', rut='11111111-0') + ['2026-09-20'],
        fila(3, rit='C-2-2026') + ['2026-09-20'],
    ]
    filas[1][2] = filas[0][2] = 'Persona Ficticia 1'
    trabajo = correr(tmp_path, 'INFORMES', INFORMES, filas, {})
    textos = [r.observation for r in trabajo.rows]
    assert all(
        'Registra 2 informes vencidos en RUS; el más antiguo venció el 20 de agosto de 2026.' in t for t in textos[:2]
    )
    assert 'informes vencidos' not in textos[2]
    # Decisión u (14-oct): J2 usa solo datos de RUS; con Excel (sin paquete) da lo mismo.
    assert [r.observation for r in correr(tmp_path, 'INFORMES', INFORMES, filas, paquete=False).rows] == textos


def test_dce_en_el_margen_con_informe_ingresado_despues_del_vencimiento(tmp_path):
    # Revisión E3 (21-oct-2026): el margen DCE de WA-7 no impide contrastar la historia con el vencimiento de RUS.
    f = inf(
        tmp_path, fila(1, 'DCE UNO') + ['2026-10-09'], hist(('Ingreso informe de cumplimiento X', '2026-10-11', 58))
    )
    assert f.observation.split(': ', 1)[1] == (
        'Informe diagnóstico vencido desde el 9 de octubre de 2026. '
        'Se hace presente que en resolución de fecha 11 de octubre de 2026 (folio 58), se ingresa el informe de cumplimiento.'
    )
    assert f.actions == [] and [a[:2] for a in f.avisos] == [(2, 'I01_CONTRADICHA')]


def test_dce_en_el_margen_con_informe_anterior_al_vencimiento_conserva_el_aviso(tmp_path):
    f = inf(
        tmp_path, fila(1, 'DCE UNO') + ['2026-10-09'], hist(('Ingreso informe de cumplimiento X', '2026-07-01', 30))
    )
    assert f.rules == ['INFORMES.I02_POR_VENCER_DCE'] and f.actions == ['programa_por_vencer']
    assert 'a más tardar el 19 de octubre de 2026' in f.observation and 'folio 30' not in f.observation
    assert [a[:2] for a in f.avisos] == [(2, 'ENTREGA_ANTERIOR')]
