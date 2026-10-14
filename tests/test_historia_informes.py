"""WP-R7: reglas de historia en Informes (I1, I2, I3, J2) y recorte de I-01."""

from historia_fixtures import correr, fila, hist, inf, INFORMES


def test_i1_contradice_i01_sin_frase_de_envio_y_marca_nivel2(tmp_path):
    f = inf(tmp_path, fila(1) + ['2026-09-20'], hist(('Ingreso informe de cumplimiento X', '2026-09-25', 58)))
    assert f.observation.split(': ', 1)[1] == (
        'Informe de avance vencido desde el 20 de septiembre de 2026. En la historia de la causa consta un trámite '
        'descrito como «Ingreso informe de cumplimiento X», de fecha 25 de septiembre de 2026, folio 58. '
        'RUS registra el informe vencido desde el 20 de septiembre de 2026.'
    )
    assert f.actions == [] and [a[:2] for a in f.avisos] == [(2, 'I01_CONTRADICHA')]
    dce = inf(
        tmp_path, fila(1, 'DCE UNO') + ['2026-09-20'], hist(('Ingreso informe de cumplimiento X', '2026-09-20', 58))
    )
    assert 'Informe diagnóstico vencido desde el 20 de septiembre de 2026.' in dce.observation


def test_i1_ingreso_anterior_al_vencimiento_no_desmiente(tmp_path):
    f = inf(tmp_path, fila(1) + ['2026-09-20'], hist(('Ingreso informe de cumplimiento X', '2026-07-01', 30)))
    assert f.rules == ['INFORMES.I01_VENCIDO_GENERAL'] and f.actions == ['programa_vencido', 'PC_INFO']
    assert f.avisos == [] and 'folio 30' not in f.observation


def test_i2_prorroga_vencido_es_solo_constancia_y_conserva_acciones(tmp_path):
    f = inf(tmp_path, fila(1) + ['2026-09-20'], hist(('Prorroga', '2026-09-25', 58)))
    assert 'INFORMES.I01_VENCIDO_GENERAL' in f.rules and f.actions == ['programa_vencido', 'PC_INFO']
    assert f.avisos == [] and f.observation.endswith('«Prorroga», de fecha 25 de septiembre de 2026, folio 58.')


def test_i2_por_vencer_conserva_i02_y_no_marca(tmp_path):
    f = inf(tmp_path, fila(1) + ['2026-10-20'], hist(('Prorroga', '2026-10-01', 9)))
    assert 'INFORMES.I02_POR_VENCER_GENERAL' in f.rules and f.actions == ['programa_por_vencer']
    assert f.observation.endswith('«Prorroga», de fecha 1 de octubre de 2026, folio 9.') and f.avisos == []


def test_i3_escrito_sin_ingreso_en_informes(tmp_path):
    f = inf(tmp_path, fila(1) + ['2026-09-20'], hist(('Inf. Cump Programa Ambulatorio', '2026-10-01', 30)))
    assert 'recepcionado a folio 30' in f.observation and f.actions == []


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
