"""WP-R6: reglas de historia en Cumplimiento (C1, C2, C4, C5, C8, C9, C10) y sus cambios a reglas originales."""

from historia_fixtures import bita, cum, cumplimiento, hist


def test_c1_y_c03_con_folio(tmp_path):
    c1 = cum(tmp_path, cumplimiento(1), hist(('Ingreso efectivo', '2026-06-02', 5)))
    assert 'folio 5. RUS registra el ingreso efectivo el 1 de junio de 2026.' in c1.observation
    c03 = cum(tmp_path, cumplimiento(1, dc=10, ingreso='2026-10-04'), hist(('Ingreso efectivo', '2026-10-04', 9)))
    assert (
        'registrado el 4 de octubre de 2026, folio 9.' in c03.observation
        and 'RUS registra el ingreso efectivo' not in c03.observation
    )
    dif = cum(tmp_path, cumplimiento(1, dc=10, ingreso='2026-10-04'), hist(('Ingreso efectivo', '2026-10-03', 9)))
    assert 'RUS registra el ingreso efectivo el 4 de octubre de 2026.' in dif.observation


def test_c2_prorroga_con_egreso_proyectado(tmp_path):
    f = cum(tmp_path, cumplimiento(1, egreso='2026-11-01'), hist(('Prorroga', '2026-09-01', 3)))
    assert (
        'Prórroga», de fecha 1 de septiembre de 2026, folio 3. Al 14 de octubre de 2026, RUS registra como fecha de egreso proyectado el 1 de noviembre de 2026.'
        in f.observation
    )


def test_c4_fichas_agrega_folio_o_constancia(tmp_path):
    h = hist(('Ficha individual', '2026-10-01', 8))
    coincide = cum(tmp_path, cumplimiento(1, 'RTA UNO', ind='2026-10-01'), h)
    assert (
        'Ficha individual actualizada el 1 de octubre de 2026. Consta en la historia de la causa a folio 8.'
        in coincide.observation
    )
    sin = cum(tmp_path, cumplimiento(1, 'RTA UNO'), h)
    assert 'No registra ficha individual en RUS' in sin.observation and sin.observation.endswith(
        'RUS no registra la ficha.'
    )
    ambulatoria = cum(
        tmp_path, cumplimiento(1, 'RTA UNO', ind='2026-10-01'), hist(('Ficha Ambulatoria', '2026-10-01', 8))
    )
    assert 'Ficha Ambulatoria' not in ambulatoria.observation


def test_c5_reitera_vencida_conserva_la_accion(tmp_path):
    previa = bita(('2026-09-20', 'Persona X: Medida vencida en RUS desde el 1 de septiembre de 2026.'))
    f = cum(tmp_path, cumplimiento(1, dpe=-5, dc=-5, egreso='2026-09-01'), bitacora=previa)
    assert (
        'Se reitera observación de fecha 20 de septiembre de 2026, respecto de la medida vencida en RUS.'
        in f.observation
    )
    assert 'Medida vencida en RUS desde' not in f.observation and f.actions == ['medidas']


def test_c8_no_uso_exige_cobertura_que_permita_ausencia(tmp_path):
    datos = cumplimiento(1, dpe=-5, dc=-5, egreso='2026-09-01')
    f = cum(tmp_path, datos, hist(('Ficha FAE', '2026-10-01', 1)))
    assert (
        'No consta en la historia de la causa, período consultado 14 de junio de 2026 a 14 de octubre de 2026, un trámite descrito como «Prórroga». Medida vencida en RUS desde el 1 de septiembre de 2026.'
        in f.observation
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
    assert 'informe de avance se encuentra recepcionado a folio 30, de fecha 1 de octubre de 2026' in f.observation
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


def test_mayoria_de_edad_marca_solo_con_la_medida_vigente(tmp_path):
    vigente = cum(tmp_path, cumplimiento(1, nac='2008-01-01'))
    assert vigente.rules == ['COMUN.MAYORIA_EDAD'] and [a[:2] for a in vigente.avisos] == [(3, 'REVISAR_MAYORIA')]
    vencida = cum(tmp_path, cumplimiento(1, dpe=-5, dc=-5, egreso='2026-09-01', nac='2008-01-01'))
    assert vencida.rules == ['COMUN.MAYORIA_EDAD'] and vencida.avisos == []
