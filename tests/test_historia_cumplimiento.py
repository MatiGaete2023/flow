"""WP-R6: reglas de historia en Cumplimiento (C1, C2, C4, C5, C8, C9, C10) y sus cambios a reglas originales."""

from historia_fixtures import bita, cum, cumplimiento, hist


def test_ingreso_efectivo_verificado_por_la_historia_en_30_dias(tmp_path):
    informado = 'La fecha de ingreso efectivo informada al Tribunal corresponde al 1 de junio de 2026.'
    # La historia guarda la fecha de la resolución, días después de la de RUS: es el mismo ingreso.
    for fecha in ('2026-06-01', '2026-06-06', '2026-07-01'):
        f = cum(tmp_path, cumplimiento(1, dc=100, ingreso='2026-06-01'), hist(('Ingreso efectivo', fecha, 5)))
        assert f.observation.split(': ', 1)[1] == 'Medida revisada. ' + informado and f.actions == []
    # Anterior a RUS, más de 30 días después o ausente: se ignora, sin contraste ni «no consta».
    for fecha in ('2026-05-31', '2026-07-02'):
        f = cum(tmp_path, cumplimiento(1, dc=100, ingreso='2026-06-01'), hist(('Ingreso efectivo', fecha, 5)))
        assert f.observation.endswith('Medida revisada, sin observaciones.')
    assert cum(tmp_path, cumplimiento(1, dc=100, ingreso='2026-06-01'), hist()).rules == [
        'CUMPLIMIENTO.C09_SIN_OBSERVACIONES'
    ]


def test_ingreso_reciente_conserva_su_texto_sin_folio_ni_contraste(tmp_path):
    for fecha in ('2026-10-04', '2026-10-09', '2026-09-30'):
        f = cum(tmp_path, cumplimiento(1, dc=10, ingreso='2026-10-04'), hist(('Ingreso efectivo', fecha, 9)))
        assert f.observation.split(': ', 1)[1] == (
            'Ingreso efectivo al programa PRM UNO registrado el 4 de octubre de 2026. '
            'RUS registra como fecha de egreso proyectado el 1 de mayo de 2027.'
        )
        assert 'folio' not in f.observation and 'INGRESO_INFORMADO' not in ''.join(f.rules)


def _prorroga(tmp_path, tramite, *, egreso, dpe, bitacora=None):
    return cum(tmp_path, cumplimiento(1, egreso=egreso, dpe=dpe, dc=100 if dpe >= 0 else dpe), hist(tramite), bitacora)


def test_prorroga_vigente_dice_hasta_cuando_el_egreso_proyectado(tmp_path):
    f = _prorroga(tmp_path, ('Prorroga', '2026-10-07', 3), egreso='2027-03-01', dpe=100)
    assert f.observation.split(': ', 1)[1] == (
        'Medida revisada. Se hace presente que a folio 3, de fecha 7 de octubre de 2026, '
        'se prorroga la medida hasta el 1 de marzo de 2027.'
    )
    assert f.avisos == [] and f.rules == ['HISTORIA.C2_PRORROGA_HASTA', 'HISTORIA.MEDIDA_REVISADA']


def test_prorroga_reciente_con_medida_por_vencer_no_repite_la_fecha_y_marca_prioritaria(tmp_path):
    f = _prorroga(tmp_path, ('Prorroga', '2026-10-07', 3), egreso='2026-10-29', dpe=15)
    assert f.observation.split(': ', 1)[1] == (
        'Medida próxima a vencer en RUS el 29 de octubre de 2026. '
        'Se hace presente que a folio 3, de fecha 7 de octubre de 2026, se prorroga la medida.'
    )
    assert [a[:2] for a in f.avisos] == [(1, 'PRORROGA_REVISAR')] and f.actions == ['medidas']
    # Prórroga de más de 90 días: la frase sale, la marca no.
    antigua = _prorroga(tmp_path, ('Prorroga', '2026-07-01', 3), egreso='2026-10-29', dpe=15)
    assert 'se prorroga la medida.' in antigua.observation and antigua.avisos == []
    limite = _prorroga(tmp_path, ('Prorroga', '2026-07-16', 3), egreso='2026-10-29', dpe=15)
    assert [a[1] for a in limite.avisos] == ['PRORROGA_REVISAR']  # exactamente 90 días
    fuera = _prorroga(tmp_path, ('Prorroga', '2026-07-15', 3), egreso='2026-10-29', dpe=15)
    assert fuera.avisos == []  # 91 días


def test_prorroga_con_medida_vencida_no_se_afirma_solo_se_marca(tmp_path):
    f = _prorroga(tmp_path, ('Prorroga', '2026-08-01', 3), egreso='2026-10-01', dpe=-13)
    assert f.observation.split(': ', 1)[1] == 'Medida vencida en RUS desde el 1 de octubre de 2026.'
    assert [a[:2] for a in f.avisos] == [(1, 'PRORROGA_REVISAR')] and f.actions == ['medidas']
    antigua = _prorroga(tmp_path, ('Prorroga', '2026-06-20', 3), egreso='2026-10-01', dpe=-13)
    assert antigua.observation.endswith('Medida vencida en RUS desde el 1 de octubre de 2026.')
    assert antigua.avisos == [] and 'prorroga' not in antigua.observation.lower()


def test_prorroga_posterior_al_egreso_de_rus_indica_que_rus_no_la_refleja(tmp_path):
    f = _prorroga(tmp_path, ('Prorroga', '2026-10-07', 3), egreso='2026-09-01', dpe=100)
    assert [a[:2] for a in f.avisos] == [(1, 'PRORROGA_REVISAR')]
    assert 'prorroga' not in f.observation.lower()


def test_prorroga_ya_mencionada_en_la_bitacora_se_ignora_con_su_marca(tmp_path):
    previa = bita(('2026-10-08', 'Persona X: Se hace presente que a folio 3 se prorroga la medida.'))
    f = _prorroga(tmp_path, ('Prorroga', '2026-10-07', 3), egreso='2026-10-29', dpe=15, bitacora=previa)
    assert f.observation.split(': ', 1)[1] == 'Medida próxima a vencer en RUS el 29 de octubre de 2026.'
    assert f.avisos == []
    otra = bita(('2026-10-08', 'Persona X: Se hace presente que a folio 99 se prorroga la medida.'))
    assert _prorroga(tmp_path, ('Prorroga', '2026-10-07', 3), egreso='2026-10-29', dpe=15, bitacora=otra).avisos


def test_c4_fichas_agrega_folio_o_constancia(tmp_path):
    h = hist(('Ficha individual', '2026-10-01', 8))
    coincide = cum(tmp_path, cumplimiento(1, 'RTA UNO', ind='2026-10-01'), h)
    assert (
        'Ficha individual actualizada el 1 de octubre de 2026. Consta en la historia de la causa a folio 8.'
        in coincide.observation
    )
    sin = cum(tmp_path, cumplimiento(1, 'RTA UNO'), h)
    # La regla original ya dice que RUS no registra la ficha: la historia no lo repite.
    assert sin.observation.split(': ', 1)[1] == (
        'No registra ficha individual en RUS. En la historia de la causa consta un trámite descrito como '
        '«Ficha individual», de fecha 1 de octubre de 2026, folio 8. Se sugiere confeccionarla.'
    )
    reciente = cum(
        tmp_path, cumplimiento(1, 'FAE UNO', ingreso='2026-09-20', dc=24), hist(('Ficha FAE', '2026-10-01', 8))
    )
    assert reciente.observation.endswith('RUS no registra la ficha.')
    ambulatoria = cum(
        tmp_path, cumplimiento(1, 'RTA UNO', ind='2026-10-01'), hist(('Ficha Ambulatoria', '2026-10-01', 8))
    )
    assert 'Ficha Ambulatoria' not in ambulatoria.observation


def test_c5_reitera_vencida_conserva_la_accion(tmp_path):
    previa = bita(('2026-09-20', 'Persona X: Medida vencida en RUS desde el 1 de septiembre de 2026.'))
    f = cum(tmp_path, cumplimiento(1, dpe=-5, dc=-5, egreso='2026-09-01'), bitacora=previa)
    assert 'Se reitera la observación del 20 de septiembre de 2026 sobre la medida vencida en RUS.' in f.observation
    assert 'Medida vencida en RUS desde' not in f.observation and f.actions == ['medidas']


def test_c8_no_uso_exige_cobertura_que_permita_ausencia(tmp_path):
    datos = cumplimiento(1, dpe=-5, dc=-5, egreso='2026-09-01')
    f = cum(tmp_path, datos, hist(('Ficha FAE', '2026-10-01', 1)))
    # La frase de la medida ya dice el vencimiento: C8 no lo repite.
    assert f.observation.split(': ', 1)[1] == (
        'Medida vencida en RUS desde el 1 de septiembre de 2026. No consta en la historia de la causa, '
        'período consultado 14 de junio de 2026 a 14 de octubre de 2026, un trámite descrito como «Prorroga».'
    )
    previa = bita(('2026-09-20', 'Persona X: Medida vencida en RUS desde el 1 de septiembre de 2026.'))
    reiterada = cum(tmp_path, datos, hist(('Ficha FAE', '2026-10-01', 1)), previa)
    assert reiterada.observation.split(': ', 1)[1] == (
        'Se reitera la observación del 20 de septiembre de 2026 sobre la medida vencida en RUS. '
        'No consta en la historia de la causa, período consultado 14 de junio de 2026 a 14 de octubre de 2026, '
        'un trámite descrito como «Prorroga». Medida vencida en RUS desde el 1 de septiembre de 2026.'
    )
    assert any(a[:2] == (3, 'C8_NO_USO') for a in f.avisos)
    assert (
        'No consta' not in cum(tmp_path, datos, hist(('Ficha FAE', '2026-10-01', 1), cobertura='PARCIAL')).observation
    )
    assert 'No consta' not in cum(tmp_path, datos, hist(('Prorroga', '2026-10-01', 1))).observation
    assert 'No consta' in cum(tmp_path, datos, hist(cobertura='VACIA_COMPROBADA')).observation
    assert 'No consta' not in cum(tmp_path, datos, None).observation


def test_c9_diagnostico_dce_con_medida_vencida(tmp_path):
    datos = cumplimiento(1, 'DCE UNO', dpe=-5, dc=-5, egreso='2026-09-01')
    f = cum(tmp_path, datos, hist(('Resp. Diagnóstico clínico especializado.', '2026-09-05', 12)))
    assert (
        'Informe diagnóstico ordenado en autos, se encuentra a folio 12 de fecha 5 de septiembre de 2026, a la espera de lo que se resuelva en la audiencia respectiva.'
        in f.observation
    )
    assert (
        'Informe diagnóstico ordenado'
        not in cum(
            tmp_path, cumplimiento(1, 'DCE UNO'), hist(('Resp. Diagnóstico clínico especializado.', '2026-09-05', 12))
        ).observation
    )


def test_c10_escrito_sin_ingreso_posterior(tmp_path):
    escrito = ('Inf. Cump Programa Ambulatorio', '2026-10-01', 30)
    f = cum(tmp_path, cumplimiento(1), hist(escrito))
    assert f.observation.split(': ', 1)[1] == (
        'Medida revisada. Informe de avance recepcionado a folio 30, de fecha 1 de octubre de 2026. '
        'Se deja constancia de que se remite proyecto de resolución que aplica la nomenclatura '
        '«Ingreso Informe de Cumplimiento X» a fin de regularizar informáticamente la causa.'
    )
    cerrado = cum(tmp_path, cumplimiento(1), hist(escrito, ('Ingreso informe de cumplimiento X', '2026-10-02', 31)))
    assert 'recepcionado' not in cerrado.observation
    assert f.actions == []


def test_sin_seguimiento_vencida_y_mayoria_de_edad(tmp_path):
    f = cum(tmp_path, cumplimiento(1, 'OPD COMUNAL', dpe=-5, dc=-5, egreso='2026-09-01'))
    assert f.observation.endswith('Medida sin vigencia en RUS desde el 1 de septiembre de 2026.')
    assert 'Medida sin vigencia' not in cum(tmp_path, cumplimiento(1, 'OPD COMUNAL')).observation


def test_medida_revisada_cuando_solo_hay_reglas_nuevas(tmp_path):
    f = cum(tmp_path, cumplimiento(1), hist(('Resp. continuidad de atención', '2026-10-01', 4)))
    assert f.observation.split(': ', 1)[1].startswith('Medida revisada. En la historia de la causa consta')
    con_original = cum(
        tmp_path, cumplimiento(1, egreso='2026-11-01', dpe=5), hist(('Resp. continuidad de atención', '2026-10-01', 4))
    )
    assert 'Medida revisada. En la historia' not in con_original.observation


def test_mayoria_de_edad_marca_siempre_por_la_edad(tmp_path):
    vigente = cum(tmp_path, cumplimiento(1, nac='2008-01-01'))
    assert vigente.rules == ['COMUN.MAYORIA_EDAD'] and [a[:2] for a in vigente.avisos] == [(3, 'REVISAR_MAYORIA')]
    vencida = cum(tmp_path, cumplimiento(1, dpe=-5, dc=-5, egreso='2026-09-01', nac='2008-01-01'))
    assert vencida.rules == ['COMUN.MAYORIA_EDAD'] and [a[:2] for a in vencida.avisos] == [(3, 'REVISAR_MAYORIA')]


def test_ingreso_reciente_con_medida_por_vencer_dice_el_egreso_una_sola_vez(tmp_path):
    f = cum(tmp_path, cumplimiento(1, dc=10, ingreso='2026-10-04', egreso='2026-10-29', dpe=15))
    assert f.observation.split(': ', 1)[1] == (
        'Ingreso efectivo al programa PRM UNO registrado el 4 de octubre de 2026. '
        'Medida próxima a vencer en RUS el 29 de octubre de 2026.'
    )
