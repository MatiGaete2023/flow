"""WP-R5: reglas de historia en Espera (1, 3, 5, 6, 7, 8, E9, E10) y transversales (R9, R11, C3, T2, T3, T5)."""

from historia_fixtures import ORDEN, bita, correr, esp, fila, hist, ESPERA


def test_regla1_folio_de_la_resolucion_de_ingreso_solo_si_es_unico(tmp_path):
    servicio = 'Ing.Serv. protección especializada y otras redes'
    datos = fila(1) + [5]
    enc = ['FEC. RESOLUCION']
    unico = esp(tmp_path, datos, hist((servicio, '2026-10-10', 77)), extra=['2026-10-10'], encabezados=enc)
    assert (
        'HISTORIA.E04_CON_FOLIO' in unico.rules
        and 'ordenado en resolución de fecha 10 de octubre de 2026 (folio 77).' in unico.observation
    )
    # La resolución se dice una sola vez: no se repite con la frase de «Tribunal dispone ingreso».
    assert 'HISTORIA.SERVICIO_INGRESO' not in unico.rules and unico.observation.count('folio 77') == 1
    doble = hist((servicio, '2026-10-10', 77), (servicio, '2026-10-10', 78))
    assert 'ESPERA.E04_RESOLUCION_RECIENTE' in esp(tmp_path, datos, doble, extra=['2026-10-10'], encabezados=enc).rules
    # Otro trámite de esa fecha no es la resolución de ingreso: no aporta folio.
    otro = hist(('Resolución ingreso', '2026-10-10', 77))
    assert 'ESPERA.E04_RESOLUCION_RECIENTE' in esp(tmp_path, datos, otro, extra=['2026-10-10'], encabezados=enc).rules


def test_ingreso_efectivo_con_rus_en_espera_aplica_reglas_normales_y_solo_marca(tmp_path):
    # «Ingreso efectivo» es de Cumplimiento (el programa informa que el NNA ya está en intervención).
    f = esp(tmp_path, fila(1) + [45], hist(('Ingreso efectivo', '2026-10-02', 41)))
    assert f.rules == ['ESPERA.E05_PROYECTO_Y_CORREO', 'HISTORIA.E05_DIAS'] and f.actions == [
        'programa_espera',
        'PC_IE',
    ]
    assert 'Ingreso efectivo' not in f.observation and 'folio' not in f.observation
    assert [(a[0], a[1]) for a in f.avisos] == [(2, 'R3_INGRESO_EFECTIVO')]
    varios = esp(
        tmp_path, fila(1) + [45], hist(('Ingreso efectivo', '2026-09-02', 40), ('Ingreso efectivo', '2026-10-02', 41))
    )
    assert varios.observation == f.observation and [a[1] for a in varios.avisos] == ['R3_INGRESO_EFECTIVO']


def test_sin_cobertura_rigen_las_reglas_originales(tmp_path):
    base = esp(tmp_path, fila(1) + [45])
    for cobertura in ('NO_SOLICITADA', 'VACIA_COMPROBADA', 'FALLIDA'):
        f = esp(tmp_path, fila(1) + [45], hist(('Ingreso efectivo', '2026-10-02', 41), cobertura=cobertura))
        assert f.observation == base.observation and f.rules == base.rules and f.avisos == []


def test_regla6_duplicidad_mismo_programa_nivel1_otro_programa_sin_marca(tmp_path):
    filas = [
        fila(1, rit='C-1-2026') + [45],
        fila(2, rit='C-1-2026') + [45],
        fila(3, 'PRM DOS', rit='C-9-2026') + [45],
        fila(4, 'PRM TRES', rit='C-9-2026') + [45],
    ]
    trabajo = correr(tmp_path, 'ESPERA', ESPERA, filas, {})
    assert [[a[1] for a in r.avisos] for r in trabajo.rows] == [['R6_DUPLICIDAD'], ['R6_DUPLICIDAD'], [], []]
    assert 'Persona Ficticia 3' not in ''.join(r.observation for r in trabajo.rows[:2])
    # Decisión u (14-oct): la regla 6 usa solo datos de RUS; con Excel (sin paquete) da lo mismo.
    excel = correr(tmp_path, 'ESPERA', ESPERA, filas, paquete=False)
    assert [r.avisos for r in excel.rows] == [r.avisos for r in trabajo.rows]


def test_regla6_hermanos_en_la_misma_causa_y_programa_no_son_duplicidad(tmp_path):
    filas = [fila(1, rit='C-1-2026', rut='11111111-1') + [45], fila(2, rit='C-1-2026', rut='22222222-2') + [45]]
    trabajo = correr(tmp_path, 'ESPERA', ESPERA, filas, {})
    assert all(r.avisos == [] for r in trabajo.rows)
    h = hist(('Orden de búsqueda', '2026-10-02', 41))
    con = correr(tmp_path, 'ESPERA', ESPERA, filas, {0: h, 1: h}).rows[0]
    assert 'La historia es de la causa y no permite atribuirlo a un ingreso determinado.' in con.observation
    assert [a[1] for a in con.avisos] == ['R11_BUSQUEDA_VIGENTE']


def test_ingreso_efectivo_anterior_al_inicio_de_la_espera_no_marca(tmp_path):
    # Corte 14-oct, 40 días de espera: la espera empezó el 4-sep; un ingreso del 20-jun es de otra medida.
    viejo = esp(tmp_path, fila(1) + [40], hist(('Ingreso efectivo', '2026-06-20', 3)))
    assert viejo.actions == ['programa_espera', 'PC_IE'] and viejo.avisos == []
    borde = esp(tmp_path, fila(1) + [40], hist(('Ingreso efectivo', '2026-09-04', 3)))
    assert borde.actions == ['programa_espera', 'PC_IE'] and [a[1] for a in borde.avisos] == ['R3_INGRESO_EFECTIVO']
    sin_dias = esp(tmp_path, fila(1) + [''], hist(('Ingreso efectivo', '2026-06-20', 3)))
    assert [a[1] for a in sin_dias.avisos] == ['R3_INGRESO_EFECTIVO']


def test_aclaracion_de_causa_es_transversal_y_sale_una_vez_al_final(tmp_path):
    aclaracion = 'La historia es de la causa y no permite atribuirlo a un ingreso determinado.'
    filas = [fila(1, rit='C-1-2026', rut='11111111-1') + [45], fila(2, rit='C-1-2026', rut='22222222-2') + [45]]
    h = hist(
        ('Ing.Serv. protección especializada y otras redes', '2026-10-02', 41),
        ('Pide cuenta búsqueda', '2026-10-05', 42),
    )
    f = correr(tmp_path, 'ESPERA', ESPERA, filas, {0: h, 1: h}).rows[0]
    assert f.observation.endswith(aclaracion) and f.observation.count(aclaracion) == 1
    # Un solo registro en la causa, o ninguna cita de la historia: no se aclara.
    solo = correr(tmp_path, 'ESPERA', ESPERA, filas[:1], {0: h}).rows[0]
    assert aclaracion not in solo.observation
    sin_cita = correr(tmp_path, 'ESPERA', ESPERA, filas, {0: hist(), 1: hist()}).rows[0]
    assert aclaracion not in sin_cita.observation


def test_servicio_dice_lo_resuelto_y_las_constancias_dce_no_van_en_espera(tmp_path):
    solicitud = ('SPE. Resp. Solicitud DCE (escrito)', '2026-10-01', 5)
    # El informe clínico diagnóstico se hace constar solo en Cumplimiento.
    dce = esp(
        tmp_path,
        fila(1, 'DCE UNO') + [3],
        hist(solicitud, ('Resp. Diagnóstico clínico especializado.', '2026-10-02', 6)),
    )
    base = esp(tmp_path, fila(1, 'DCE UNO') + [3])
    assert (dce.observation, dce.rules, dce.avisos) == (base.observation, base.rules, base.avisos)
    sv = esp(tmp_path, fila(1) + [3], hist(('Ing.Serv. protección especializada y otras redes', '2026-10-02', 7)))
    # Servicio: se dice lo resuelto, con el programa de la fila de RUS y la fecha de la resolución.
    assert (
        'Tribunal dispone ingreso a PRM UNO, en resolución de fecha 2 de octubre de 2026 (folio 7).' in sv.observation
    )
    assert 'Ing.Serv.' not in sv.observation


def test_lista_de_espera_larga_reemplaza_los_dias_de_espera(tmp_path):
    larga = esp(tmp_path, fila(1, 'RTA UNO') + [120])
    assert 'Registra 120 días en lista de espera.' in larga.observation
    # Los días de espera se dicen una vez: «lista de espera larga» reemplaza a «RUS registra N días de espera».
    assert 'días de espera.' not in larga.observation.replace('lista de espera.', '')
    assert larga.rules == ['ESPERA.E05_PROYECTO_Y_CORREO', 'HISTORIA.E10_LISTA'] and larga.actions == [
        'programa_espera',
        'PC_IE',
    ]
    assert 'lista de espera' not in esp(tmp_path, fila(1, 'RTA UNO') + [100]).observation
    assert 'lista de espera' not in esp(tmp_path, fila(1, 'PRM UNO') + [120]).observation
    # El umbral es configurable (lista_espera_larga).
    from copy import deepcopy

    from historia_fixtures import ESPERA, correr
    from nurus.personal.config import defaults

    config = deepcopy(defaults())
    config['umbrales']['lista_espera_larga'] = 150
    otra = correr(tmp_path, 'ESPERA', ESPERA, [fila(1, 'RTA UNO') + [120]], None, None, config).rows[0]
    assert 'lista de espera' not in otra.observation


def test_reitera_curador_cita_la_primera_observacion(tmp_path):
    previa = bita(
        ('2026-09-01', 'Persona X: No registra curador ad litem en RUS. Se sugiere asociarlo informáticamente.'),
        (
            '2026-09-20',
            'Persona X: Se reitera observación de fecha 1 de septiembre de 2026, respecto de la ausencia de curador ad litem en RUS.',
        ),
    )
    f = esp(tmp_path, fila(1) + [3], bitacora=previa, extra=[''], encabezados=['CURADOR'])
    assert (
        'Se reitera la observación del 1 de septiembre de 2026 sobre la ausencia de curador ad litem en RUS.'
        in f.observation
    )
    assert 'Se sugiere asociarlo' not in f.observation
    sin = esp(tmp_path, fila(1) + [3], extra=[''], encabezados=['CURADOR'])
    assert 'Se sugiere asociarlo informáticamente.' in sin.observation


def test_t5_exige_dos_instituciones_distintas(tmp_path):
    dos = (
        '(11-1)ABOG A - Institución: Programa Mi Abogado#(22-2)ABOG B - Institución: Corporación de Asistencia Judicial'
    )
    una = '(11-1)ABOG A - Institución: Programa Mi Abogado#(22-2)ABOG B - Institución: Programa Mi Abogado'
    f = esp(tmp_path, fila(1) + [3], extra=[dos], encabezados=['CURADOR'])
    assert 'se visualizan 2 instituciones distintas como curadores ad litem de Persona' in f.observation
    assert (1, 'T5_INSTITUCIONES') == f.avisos[0][:2]
    assert (
        'instituciones distintas' not in esp(tmp_path, fila(1) + [3], extra=[una], encabezados=['CURADOR']).observation
    )


def test_busqueda_nombra_primero_la_orden_y_luego_sus_tramites(tmp_path):
    orden = (
        'Se hace presente que en resolución de fecha 1 de septiembre de 2026 (folio 10), se decretó orden de búsqueda.'
    )
    cerrada = esp(tmp_path, fila(1) + [3], hist((ORDEN, '2026-09-01', 10), ('Termino de búsqueda', '2026-09-20', 20)))
    termino = 'Posteriormente, en resolución de fecha 20 de septiembre de 2026 (folio 20), se dispuso el término de la búsqueda.'
    assert orden + ' ' + termino in cerrada.observation and cerrada.avisos == []
    sin_efecto = esp(
        tmp_path, fila(1) + [3], hist((ORDEN, '2026-09-01', 10), ('Deja sin efecto búsqueda', '2026-09-20', 20))
    )
    assert (
        orden
        + ' Posteriormente, en resolución de fecha 20 de septiembre de 2026 (folio 20), se dejó sin efecto la búsqueda.'
        in (sin_efecto.observation)
    )
    completa = esp(
        tmp_path,
        fila(1) + [3],
        hist(
            ('Pide cuenta búsqueda', '2026-09-10', 15),
            ('Deja sin efecto búsqueda', '2026-09-20', 20),
            (ORDEN, '2026-09-01', 10),
        ),
    )
    assert completa.observation.index('orden de búsqueda') < completa.observation.index('se pidió cuenta')
    assert completa.observation.index('se pidió cuenta') < completa.observation.index('se dejó sin efecto la búsqueda')
    vigente = esp(tmp_path, fila(1) + [3], hist((ORDEN, '2026-09-01', 10)))
    assert [a[:2] for a in vigente.avisos] == [(1, 'R11_BUSQUEDA_VIGENTE')]
    huerfano = esp(tmp_path, fila(1) + [3], hist(('Termino de búsqueda', '2026-09-20', 20)))
    assert (
        'Se hace presente que en resolución de fecha 20 de septiembre de 2026 (folio 20), consta el trámite «Termino de búsqueda».'
        in huerfano.observation
    )
    citada = esp(tmp_path, fila(1) + [3], hist(('Pide cuenta búsqueda', '2026-09-25', 21)))
    assert 'consta el trámite «Pide cuenta búsqueda».' in citada.observation


def test_busqueda_anterior_a_dos_meses_o_ya_observada_no_genera_texto(tmp_path):
    # La ventana se cuenta desde el último trámite: orden antigua con término reciente se dice, empezando por la orden.
    reciente = esp(tmp_path, fila(1) + [3], hist((ORDEN, '2026-07-01', 10), ('Termino de búsqueda', '2026-10-01', 20)))
    assert reciente.observation.index('se decretó orden de búsqueda') < reciente.observation.index(
        'término de la búsqueda'
    )
    antigua = esp(tmp_path, fila(1) + [3], hist((ORDEN, '2026-07-01', 10), ('Termino de búsqueda', '2026-08-01', 20)))
    assert 'búsqueda' not in antigua.observation
    previa = bita(
        (
            '2026-10-02',
            'Persona X: Se hace presente que en resolución de fecha 1 de septiembre de 2026 (folio 10), se decretó orden de búsqueda.',
        )
    )
    observada = esp(tmp_path, fila(1) + [3], hist((ORDEN, '2026-09-01', 10)), previa)
    assert 'orden de búsqueda' not in observada.observation
    nueva = esp(
        tmp_path, fila(1) + [3], hist((ORDEN, '2026-09-01', 10), ('Termino de búsqueda', '2026-10-05', 20)), previa
    )
    assert 'se dispuso el término de la búsqueda' in nueva.observation


def test_c3_derecho_oido_solo_sin_fecha_en_rus(tmp_path):
    h = hist(('Derecho a ser oído', '2026-09-10', 12))
    enc = ['FEC. OIDO']
    assert (
        'Oído el 10 de septiembre de 2026.' in esp(tmp_path, fila(1) + [3], h, extra=[''], encabezados=enc).observation
    )
    f = esp(tmp_path, fila(1) + [3], h, extra=['2026-10-01'], encabezados=enc)
    assert 'Oído el 1 de octubre de 2026.' in f.observation and '10 de septiembre' not in f.observation


def test_deja_sin_efecto_dr_y_mn_solo_marcan_nivel_2_sin_texto(tmp_path):
    base = esp(tmp_path, fila(1) + [3])
    for nombre, marca in (
        ('Deja sin efecto derivación D.R.', 'DEJA_SIN_EFECTO_DR'),
        ('Deja sin efecto medida interconexión M.N.', 'DEJA_SIN_EFECTO_MN'),
    ):
        f = esp(tmp_path, fila(1) + [3], hist((nombre, '2026-09-15', 20)))
        assert f.observation == base.observation and f.rules == base.rules and f.actions == base.actions
        assert [a[:2] for a in f.avisos] == [(2, marca)]
    ambas = esp(
        tmp_path,
        fila(1) + [3],
        hist(
            ('Deja sin efecto derivación D.R.', '2026-09-15', 20),
            ('Deja sin efecto medida interconexión M.N.', '2026-09-16', 21),
        ),
    )
    assert [a[1] for a in ambas.avisos] == ['DEJA_SIN_EFECTO_DR', 'DEJA_SIN_EFECTO_MN']


def test_derivacion_a_direccion_regional_se_ignora_en_todo_sentido(tmp_path):
    base = esp(tmp_path, fila(1) + [3])
    for nombre in ('Derivación Dirección Reg.', 'Pide cuenta Derivación D.R.', 'Resp. Derivación dir. regional.'):
        f = esp(tmp_path, fila(1) + [3], hist((nombre, '2026-10-01', 6)))
        assert (f.observation, f.rules, f.actions, f.avisos) == (
            base.observation,
            base.rules,
            base.actions,
            base.avisos,
        )


def test_mayoria_de_edad_sale_sola_y_marca_nivel3(tmp_path):
    f = esp(tmp_path, fila(1) + [45], extra=['2008-01-01'], encabezados=['FEC. NACIMIENTO'])
    assert f.rules == ['COMUN.MAYORIA_EDAD'] and [a[:2] for a in f.avisos] == [(3, 'REVISAR_MAYORIA')]
    assert 'Se sugiere egresar la medida.' in f.observation and 'días de espera' not in f.observation


def test_continuidad_de_atencion_se_hace_constar_una_sola_vez(tmp_path):
    esperado = 'Se hace presente que en resolución de fecha 1 de octubre de 2026 (folio 4), consta respuesta sobre la continuidad de atención.'
    for programa in ('DCE UNO', 'PRM UNO'):
        f = esp(tmp_path, fila(1, programa) + [3], hist(('Resp. continuidad de atención', '2026-10-01', 4)))
        assert f.observation.count(esperado) == 1


def test_curador_dado_de_baja_o_designado_solo_se_dice_si_rus_no_tiene_curador(tmp_path):
    sin_rus = dict(extra=[''], encabezados=['CURADOR'])
    baja = ('Deja sin efecto designación C. Ad-Litem', '2026-09-01', 4)
    designa = ('Designa abogado', '2026-10-01', 5)
    f = esp(tmp_path, fila(1) + [3], hist(baja), **sin_rus)
    assert f.observation.split(': ', 1)[1] == (
        'Medida revisada, a la espera de ingreso efectivo al programa PRM UNO. '
        'Se hace presente que en resolución de fecha 1 de septiembre de 2026 (folio 4), se dejó sin efecto la designación de curador ad litem. '
        'No registra curador ad litem en RUS. Se sugiere asociarlo informáticamente.'
    )
    assert [a[:2] for a in f.avisos] == [(2, 'CURADOR_SIN_EFECTO')]
    # Designación posterior a la baja: se dice la designación (el curador falta en RUS) y se omite la baja.
    d = esp(tmp_path, fila(1) + [3], hist(baja, designa), **sin_rus)
    assert 'se designó curador ad litem' in d.observation and 'se dejó sin efecto' not in d.observation
    assert [a[:2] for a in d.avisos] == [(2, 'CURADOR_DESIGNADO')]
    # Baja posterior a la designación: manda la baja.
    b = esp(
        tmp_path, fila(1) + [3], hist(designa, ('Deja sin efecto designación C. Ad-Litem', '2026-10-05', 6)), **sin_rus
    )
    assert 'se dejó sin efecto' in b.observation and 'se designó' not in b.observation
    # Con curador en RUS no se puede verificar la vigencia desde la historia: se ignora, sin marca.
    con_rus = '(11-1)ABOGADO UNO - Institución: Programa Mi Abogado'
    r = esp(tmp_path, fila(1) + [3], hist(baja, designa), extra=[con_rus], encabezados=['CURADOR'])
    assert 'curador' not in r.observation.lower() and r.avisos == []
    # Sin columna de curador en la nómina no se evalúa.
    assert 'curador' not in esp(tmp_path, fila(1) + [3], hist(baja, designa)).observation.lower()


def test_busqueda_conector_por_posicion_y_bitacora_sin_falsos_positivos(tmp_path):
    dos = esp(
        tmp_path,
        fila(1) + [3],
        hist(
            (ORDEN, '2026-09-01', 10),
            ('Pide cuenta búsqueda', '2026-09-10', 15),
            ('Pide cuenta búsqueda', '2026-09-25', 25),
        ),
    )
    assert dos.observation.count('Posteriormente,') == 1 and dos.observation.count('Luego,') == 1
    # Un número suelto (el día de otra fecha) no prueba que la bitácora ya observó esta orden de búsqueda.
    otra = bita(('2026-09-05', 'Persona X: orden de búsqueda de fecha 3 de marzo de 2026 (folio 40).'))
    assert 'orden de búsqueda' in esp(tmp_path, fila(1) + [3], hist((ORDEN, '2026-09-01', 3)), otra).observation
    misma = bita(('2026-09-05', 'Persona X: consta orden de búsqueda (folio 3).'))
    assert 'orden de búsqueda' not in esp(tmp_path, fila(1) + [3], hist((ORDEN, '2026-09-01', 3)), misma).observation


def test_e04_sin_folio_no_se_repite_con_la_resolucion_de_ingreso(tmp_path):
    servicio = 'Ing.Serv. protección especializada y otras redes'
    f = esp(
        tmp_path,
        fila(1) + [3],
        hist((servicio, '2026-10-02', 30)),
        extra=['2026-10-01'],
        encabezados=['FEC. RESOLUCION'],
    )
    assert 'ordenado el 1 de octubre de 2026' in f.observation and 'Tribunal dispone ingreso' not in f.observation
