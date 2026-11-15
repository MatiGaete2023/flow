"""WP-R6: reglas de historia en Cumplimiento (C1, C2, C4, C5, C8, C9, C10) y sus cambios a reglas originales."""

from historia_fixtures import CUMPLIMIENTO, bita, correr, cum, cumplimiento, hist


def test_ingreso_efectivo_de_la_historia_ya_no_se_dice_en_cumplimiento(tmp_path):
    # «La fecha de ingreso efectivo informada al Tribunal…» se retiró (11-nov-2026): la historia acredita que existe
    # el trámite, no la fecha informada, y la frase se repetía meses después del ingreso.
    for fecha in ('2026-06-01', '2026-06-06', '2026-07-01', '2026-05-31', '2026-07-02'):
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


def test_prorroga_vigente_se_cita_como_tramite_y_el_egreso_de_rus_va_aparte(tmp_path):
    # La historia acredita el trámite «Prorroga», no su término: sin «hasta» (11-nov-2026).
    f = _prorroga(tmp_path, ('Prorroga', '2026-10-07', 3), egreso='2027-03-01', dpe=100)
    assert f.observation.split(': ', 1)[1] == (
        'Medida revisada. Se hace presente que en resolución de fecha 7 de octubre de 2026 (folio 3), '
        'consta el trámite «Prorroga». RUS registra como fecha de egreso proyectado el 1 de marzo de 2027.'
    )
    assert f.avisos == [] and f.rules == [
        'HISTORIA.BASE_TRAMITE',
        'HISTORIA.EGRESO_PROYECTADO',
        'HISTORIA.MEDIDA_REVISADA',
    ]


def test_prorroga_con_medida_por_vencer_se_cita_junto_a_la_medida_y_marca_prioritaria(tmp_path):
    f = _prorroga(tmp_path, ('Prorroga', '2026-10-07', 3), egreso='2026-10-29', dpe=15)
    assert f.observation.split(': ', 1)[1] == (
        'Medida próxima a vencer en RUS el 29 de octubre de 2026. Se hace presente que en resolución de fecha '
        '7 de octubre de 2026 (folio 3), consta el trámite «Prorroga».'
    )
    assert [a[:2] for a in f.avisos] == [(1, 'PRORROGA_REVISAR')] and f.actions == ['medidas']
    # Prórroga de más de 90 días: ni frase ni marca.
    antigua = _prorroga(tmp_path, ('Prorroga', '2026-07-01', 3), egreso='2026-10-29', dpe=15)
    assert 'prorroga' not in antigua.observation.lower() and antigua.avisos == []
    limite = _prorroga(tmp_path, ('Prorroga', '2026-07-16', 3), egreso='2026-10-29', dpe=15)
    assert [a[1] for a in limite.avisos] == ['PRORROGA_REVISAR']  # exactamente 90 días
    fuera = _prorroga(tmp_path, ('Prorroga', '2026-07-15', 3), egreso='2026-10-29', dpe=15)
    assert fuera.avisos == []  # 91 días


def test_prorroga_con_medida_vencida_se_cita_junto_a_la_medida_y_se_marca(tmp_path):
    f = _prorroga(tmp_path, ('Prorroga', '2026-08-01', 3), egreso='2026-10-01', dpe=-13)
    assert f.observation.split(': ', 1)[1] == (
        'Medida vencida en RUS desde el 1 de octubre de 2026. Se hace presente que en resolución de fecha '
        '1 de agosto de 2026 (folio 3), consta el trámite «Prorroga».'
    )
    assert [a[:2] for a in f.avisos] == [(1, 'PRORROGA_REVISAR')] and f.actions == ['medidas']
    antigua = _prorroga(tmp_path, ('Prorroga', '2026-06-20', 3), egreso='2026-10-01', dpe=-13)
    assert antigua.observation.endswith('Medida vencida en RUS desde el 1 de octubre de 2026.')
    assert antigua.avisos == [] and 'prorroga' not in antigua.observation.lower()


def test_prorroga_posterior_al_egreso_de_rus_indica_que_rus_no_la_refleja(tmp_path):
    f = _prorroga(tmp_path, ('Prorroga', '2026-10-07', 3), egreso='2026-09-01', dpe=100)
    assert [a[:2] for a in f.avisos] == [(1, 'PRORROGA_REVISAR')]
    assert 'consta el trámite «Prorroga»' in f.observation and 'hasta' not in f.observation


def test_prorroga_ya_mencionada_en_la_bitacora_se_ignora_con_su_marca(tmp_path):
    previa = bita(('2026-10-08', 'Persona X: Se hace presente que a folio 3 se prorroga la medida.'))
    f = _prorroga(tmp_path, ('Prorroga', '2026-10-07', 3), egreso='2026-10-29', dpe=15, bitacora=previa)
    assert f.observation.split(': ', 1)[1] == 'Medida próxima a vencer en RUS el 29 de octubre de 2026.'
    assert f.avisos == []
    otra = bita(('2026-10-08', 'Persona X: Se hace presente que a folio 99 se prorroga la medida.'))
    assert _prorroga(tmp_path, ('Prorroga', '2026-10-07', 3), egreso='2026-10-29', dpe=15, bitacora=otra).avisos


def test_fichas_de_la_historia_no_agregan_texto_a_la_observacion(tmp_path):
    # Solo rigen las reglas originales: existencia (FAE, tras 120 días) y antigüedad (individual).
    individual = cum(
        tmp_path, cumplimiento(1, 'RTA UNO', ind='2026-10-01'), hist(('Ficha individual', '2026-10-01', 8))
    )
    assert individual.observation.split(': ', 1)[1] == 'Ficha individual actualizada el 1 de octubre de 2026.'
    sin = cum(tmp_path, cumplimiento(1, 'RTA UNO'), hist(('Ficha individual', '2026-10-01', 8)))
    assert sin.observation.split(': ', 1)[1] == 'No registra ficha individual en RUS. Se sugiere confeccionarla.'
    fae = cum(tmp_path, cumplimiento(1, 'FAE UNO', ingreso='2026-01-01', fae=''), hist(('Ficha FAE', '2026-10-01', 8)))
    assert fae.observation.split(': ', 1)[1] == 'Persona no registra ficha FAE en RUS. Se sugiere confeccionarla.'
    reciente = cum(
        tmp_path, cumplimiento(1, 'FAE UNO', ingreso='2026-09-20', dc=24), hist(('Ficha FAE', '2026-10-01', 8))
    )
    assert 'ficha' not in reciente.observation.lower()


def test_la_medida_vencida_no_se_reitera_y_conserva_su_fecha(tmp_path):
    # Una observación previa de otro vencimiento no es el mismo episodio: se dice la fecha vigente (11-nov-2026).
    previa = bita(('2026-05-01', 'Persona X: Medida vencida en RUS desde el 1 de abril de 2026.'))
    f = cum(tmp_path, cumplimiento(1, dpe=-5, dc=-5, egreso='2026-09-01'), bitacora=previa)
    assert f.observation.split(': ', 1)[1] == 'Medida vencida en RUS desde el 1 de septiembre de 2026.'
    assert f.rules == ['CUMPLIMIENTO.C04_VENCIDA'] and f.actions == ['medidas']


def test_no_consta_prorroga_ya_no_se_aplica(tmp_path):
    datos = cumplimiento(1, dpe=-5, dc=-5, egreso='2026-09-01')
    for historia in (hist(('Ficha FAE', '2026-10-01', 1)), hist(cobertura='VACIA_COMPROBADA'), None):
        f = cum(tmp_path, datos, historia)
        assert f.observation.split(': ', 1)[1] == 'Medida vencida en RUS desde el 1 de septiembre de 2026.'
        # Un trámite sin regla solo deja su marca de nivel 3; nunca texto ni «no consta prórroga».
        assert [a[1] for a in f.avisos if a[1] != 'TRAMITES_SIN_REGLA'] == [] and f.actions == ['medidas']


def test_solicitud_dce_en_cumplimiento_con_medida_vencida(tmp_path):
    datos = cumplimiento(1, 'DCE UNO', dpe=-5, dc=-5, egreso='2026-09-01')
    f = cum(tmp_path, datos, hist(('SPE. Resp. Solicitud DCE (escrito)', '2026-09-05', 12)))
    assert f.observation.split(': ', 1)[1] == (
        'Medida vencida en RUS desde el 1 de septiembre de 2026. Informe clínico diagnóstico ordenado en autos, '
        'se encuentra en la causa en resolución de fecha 5 de septiembre de 2026 (folio 12), a la espera de lo que se resuelva en la audiencia respectiva.'
    )
    # Sin medida vencida, en otro programa, o con las otras nomenclaturas DCE: no se dice nada.
    solicitud = ('SPE. Resp. Solicitud DCE (escrito)', '2026-09-05', 12)
    assert 'Informe clínico' not in cum(tmp_path, cumplimiento(1, 'DCE UNO'), hist(solicitud)).observation
    assert (
        'Informe clínico'
        not in cum(
            tmp_path, cumplimiento(1, 'PRM UNO', dpe=-5, dc=-5, egreso='2026-09-01'), hist(solicitud)
        ).observation
    )
    for ignorada in ('Resp. Diagnóstico clínico especializado.', 'SPE. Resp. Rectificación DCE (escrito)'):
        g = cum(tmp_path, datos, hist((ignorada, '2026-09-05', 12)))
        assert (
            g.observation.split(': ', 1)[1] == 'Medida vencida en RUS desde el 1 de septiembre de 2026.'
            and g.avisos == []
        )


def test_c10_escrito_sin_ingreso_posterior(tmp_path):
    escrito = ('Inf. Cump Programa Ambulatorio', '2026-10-01', 30)
    f = cum(tmp_path, cumplimiento(1), hist(escrito))
    assert f.observation.split(': ', 1)[1] == (
        'Medida revisada. Informe de avance recepcionado en resolución de fecha 1 de octubre de 2026 (folio 30). '
        'Se deja constancia de que se remite proyecto de resolución que aplica la nomenclatura '
        '«Ingreso Informe de Cumplimiento X» a fin de regularizar informáticamente la causa.'
    )
    cerrado = cum(tmp_path, cumplimiento(1), hist(escrito, ('Ingreso informe de cumplimiento X', '2026-10-02', 31)))
    assert 'recepcionado' not in cerrado.observation
    assert f.actions == ['NOMENCL']


def test_sin_seguimiento_vencida_y_mayoria_de_edad(tmp_path):
    f = cum(tmp_path, cumplimiento(1, 'OPD COMUNAL', dpe=-5, dc=-5, egreso='2026-09-01'))
    assert f.observation.endswith('Medida sin vigencia en RUS desde el 1 de septiembre de 2026.')
    assert 'Medida sin vigencia' not in cum(tmp_path, cumplimiento(1, 'OPD COMUNAL')).observation


def test_medida_revisada_cuando_solo_hay_reglas_nuevas(tmp_path):
    f = cum(tmp_path, cumplimiento(1), hist(('Resp. continuidad de atención', '2026-10-01', 4)))
    assert f.observation.split(': ', 1)[1] == (
        'Medida revisada. Se hace presente que en resolución de fecha 1 de octubre de 2026 (folio 4), '
        'consta respuesta sobre la continuidad de atención.'
    )
    con_original = cum(
        tmp_path, cumplimiento(1, egreso='2026-11-01', dpe=5), hist(('Resp. continuidad de atención', '2026-10-01', 4))
    )
    assert 'Medida revisada.' not in con_original.observation and 'continuidad de atención' in con_original.observation


def test_mayoria_de_edad_marca_siempre_por_la_edad(tmp_path):
    vigente = cum(tmp_path, cumplimiento(1, nac='2008-01-01'))
    assert vigente.rules == ['COMUN.MAYORIA_EDAD'] and [a[:2] for a in vigente.avisos] == [(3, 'REVISAR_MAYORIA')]
    vencida = cum(tmp_path, cumplimiento(1, dpe=-5, dc=-5, egreso='2026-09-01', nac='2008-01-01'))
    assert vencida.rules == ['COMUN.MAYORIA_EDAD', 'CUMPLIMIENTO.C04_VENCIDA']
    assert [a[:2] for a in vencida.avisos] == [(3, 'REVISAR_MAYORIA')] and vencida.actions == ['medidas']


def test_ingreso_reciente_con_medida_por_vencer_dice_el_egreso_una_sola_vez(tmp_path):
    f = cum(tmp_path, cumplimiento(1, dc=10, ingreso='2026-10-04', egreso='2026-10-29', dpe=15))
    assert f.observation.split(': ', 1)[1] == (
        'Ingreso efectivo al programa PRM UNO registrado el 4 de octubre de 2026. '
        'Medida próxima a vencer en RUS el 29 de octubre de 2026.'
    )


def test_la_columna_res_se_llena_con_el_tipo_de_proyecto_que_el_motor_identifica(tmp_path):
    from nurus.personal.work import _automatic_resolution

    escrito = cum(tmp_path, cumplimiento(1, 'FAE UNO', dc=100, dpe=100), hist(('Inf. Cump FAE/FAS', '2026-10-01', 30)))
    assert _automatic_resolution(escrito) == 'NOMENCL'
    escrito.decisions['resolution'] = 'none'
    assert _automatic_resolution(escrito) is None
    sin = cum(tmp_path, cumplimiento(2, 'FAE UNO', dc=100, dpe=100))
    assert _automatic_resolution(sin) is None


def test_prorroga_reciente_no_se_calla_por_un_numero_suelto_en_la_bitacora(tmp_path):
    fila_ = cumplimiento(1, dc=200, dpe=-5, ingreso='2026-01-01', egreso='2026-10-09')
    previa = bita(('2026-09-20', 'Persona X: Revisada prórroga de fecha 5 de marzo de 2026.'))
    con = cum(tmp_path, fila_, hist(('Prorroga', '2026-09-15', 5)), previa)
    assert 'PRORROGA_REVISAR' in [a[1] for a in con.avisos]


def test_res_sugerido_no_cuenta_como_revision_al_reimportar_la_copia(tmp_path):
    from nurus.personal.config import defaults
    from nurus.personal.statistics import summarize
    from nurus.personal.work import Work, _human_reviewed

    trabajo = correr(
        tmp_path,
        'CUMPLIMIENTO',
        CUMPLIMIENTO,
        [cumplimiento(1, 'FAE UNO'), cumplimiento(2, 'FAE UNO')],
        {0: hist(('Inf. Cump FAE/FAS', '2026-10-01', 30))},
    )
    copia = trabajo.export(tmp_path / 'copia.xlsx', backend='portable', reduced_fidelity=True)
    externo = Work.external(copia, defaults(), 'CUMPLIMIENTO')
    assert externo.rows[0].review.get('RES') == 'NOMENCL'
    assert not any(_human_reviewed(r) for r in externo.rows) and summarize(externo)['datos_incompletos'] == 0
