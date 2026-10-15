"""WP-R5: reglas de historia en Espera (1, 3, 5, 6, 7, 8, E9, E10) y transversales (R9, R11, C3, T2, T3, T5)."""

from historia_fixtures import ORDEN, bita, correr, esp, fila, hist, ESPERA


def test_regla1_folio_de_la_resolucion_de_ingreso_solo_si_es_unico(tmp_path):
    servicio = 'Ing.Serv. protección especializada y otras redes'
    datos = fila(1) + [5]
    enc = ['FEC. RESOLUCION']
    unico = esp(tmp_path, datos, hist((servicio, '2026-10-10', 77)), extra=['2026-10-10'], encabezados=enc)
    assert (
        'HISTORIA.E04_CON_FOLIO' in unico.rules and 'ordenado el 10 de octubre de 2026, folio 77.' in unico.observation
    )
    # La resolución se dice una sola vez: no se repite con la frase de «Tribunal dispone ingreso».
    assert 'HISTORIA.SERVICIO_INGRESO' not in unico.rules and unico.observation.count('folio 77') == 1
    doble = hist((servicio, '2026-10-10', 77), (servicio, '2026-10-10', 78))
    assert 'ESPERA.E04_RESOLUCION_RECIENTE' in esp(tmp_path, datos, doble, extra=['2026-10-10'], encabezados=enc).rules
    # Otro trámite de esa fecha no es la resolución de ingreso: no aporta folio.
    otro = hist(('Resolución ingreso', '2026-10-10', 77))
    assert 'ESPERA.E04_RESOLUCION_RECIENTE' in esp(tmp_path, datos, otro, extra=['2026-10-10'], encabezados=enc).rules


def test_regla3_reemplaza_e04_e05_e06_y_marca_nivel2(tmp_path):
    f = esp(tmp_path, fila(1) + [45], hist(('Ingreso efectivo', '2026-10-02', 41)))
    assert f.observation.endswith(
        'Medida revisada. En la historia de la causa consta un trámite descrito como «Ingreso efectivo», '
        'de fecha 2 de octubre de 2026, folio 41. Al 14 de octubre de 2026, RUS registra la medida en espera de ingreso efectivo.'
    )
    assert f.actions == [] and not any(r.startswith('ESPERA.') for r in f.rules)
    assert [(a[0], a[1]) for a in f.avisos] == [(2, 'R3_INGRESO_EFECTIVO')]
    varios = esp(
        tmp_path, fila(1) + [45], hist(('Ingreso efectivo', '2026-09-02', 40), ('Ingreso efectivo', '2026-10-02', 41))
    )
    assert (
        'constan 2 trámites descritos como «Ingreso efectivo»; el último, de fecha 2 de octubre de 2026, folio 41.'
        in varios.observation
    )


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
    h = hist(('Ingreso efectivo', '2026-10-02', 41))
    con = correr(tmp_path, 'ESPERA', ESPERA, filas, {0: h, 1: h}).rows[0]
    assert 'La historia es de la causa y no permite atribuirlo a un ingreso determinado.' in con.observation
    assert [a[1] for a in con.avisos] == ['R3_INGRESO_EFECTIVO']


def test_regla3_ignora_ingreso_efectivo_anterior_al_inicio_de_la_espera(tmp_path):
    # Corte 14-oct, 40 días de espera: la espera empezó el 4-sep; un ingreso del 20-jun es de otra medida.
    viejo = esp(tmp_path, fila(1) + [40], hist(('Ingreso efectivo', '2026-06-20', 3)))
    assert 'ESPERA.E05_PROYECTO_Y_CORREO' in viejo.rules and viejo.actions == ['programa_espera', 'PC_IE']
    assert viejo.avisos == [] and 'Medida revisada. En la historia' not in viejo.observation
    borde = esp(tmp_path, fila(1) + [40], hist(('Ingreso efectivo', '2026-09-04', 3)))
    assert borde.actions == [] and [a[1] for a in borde.avisos] == ['R3_INGRESO_EFECTIVO']
    sin_dias = esp(tmp_path, fila(1) + [''], hist(('Ingreso efectivo', '2026-06-20', 3)))
    assert [a[1] for a in sin_dias.avisos] == ['R3_INGRESO_EFECTIVO']


def test_aclaracion_de_causa_es_transversal_y_sale_una_vez_al_final(tmp_path):
    aclaracion = 'La historia es de la causa y no permite atribuirlo a un ingreso determinado.'
    filas = [fila(1, rit='C-1-2026', rut='11111111-1') + [45], fila(2, rit='C-1-2026', rut='22222222-2') + [45]]
    h = hist(('Ingreso efectivo', '2026-10-02', 41), ('Deja sin efecto medida interconexión M.N.', '2026-10-05', 42))
    f = correr(tmp_path, 'ESPERA', ESPERA, filas, {0: h, 1: h}).rows[0]
    assert f.observation.endswith(aclaracion) and f.observation.count(aclaracion) == 1
    # Un solo registro en la causa, o ninguna cita de la historia: no se aclara.
    solo = correr(tmp_path, 'ESPERA', ESPERA, filas[:1], {0: h}).rows[0]
    assert aclaracion not in solo.observation
    sin_cita = correr(tmp_path, 'ESPERA', ESPERA, filas, {0: hist(), 1: hist()}).rows[0]
    assert aclaracion not in sin_cita.observation


def test_dce_direccion_regional_servicio_y_lista_larga(tmp_path):
    dce = esp(tmp_path, fila(1, 'DCE UNO') + [3], hist(('SPE. Resp. Solicitud DCE (escrito)', '2026-10-01', 5)))
    assert '«SPE. Resp. Solicitud DCE (escrito)», de fecha 1 de octubre de 2026, folio 5.' in dce.observation
    otro = esp(tmp_path, fila(1) + [3], hist(('SPE. Resp. Solicitud DCE (escrito)', '2026-10-01', 5)))
    assert 'Solicitud DCE' not in otro.observation
    dr = esp(
        tmp_path,
        fila(1) + [3],
        hist(
            ('Pide cuenta Derivación D.R.', '2026-10-01', 6),
            ('Ing.Serv. protección especializada y otras redes', '2026-10-02', 7),
        ),
    )
    assert '«Pide cuenta Derivación D.R.»' in dr.observation
    # Servicio: se dice lo resuelto, con el programa de la fila de RUS y la fecha de la resolución.
    assert (
        'Tribunal dispone ingreso a PRM UNO, en resolución de folio 7, de fecha 2 de octubre de 2026.' in dr.observation
    )
    assert 'Ing.Serv.' not in dr.observation
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


def test_busqueda_termino_cita_la_orden_y_vigente_marca_nivel1(tmp_path):
    cerrada = hist((ORDEN, '2026-09-01', 10), ('Termino de búsqueda', '2026-09-20', 20))
    f = esp(tmp_path, fila(1) + [3], cerrada)
    assert (
        'Búsqueda decretada el 1 de septiembre de 2026 (folio 10), con término el 20 de septiembre de 2026 (folio 20).'
        in (f.observation)
    )
    assert f.avisos == [] and f.observation.count('Búsqueda decretada') == 1
    sin_efecto = esp(
        tmp_path, fila(1) + [3], hist((ORDEN, '2026-09-01', 10), ('Deja sin efecto búsqueda', '2026-09-20', 20))
    )
    assert (
        'Búsqueda decretada el 1 de septiembre de 2026 (folio 10), dejada sin efecto el 20 de septiembre de 2026 (folio 20).'
        in sin_efecto.observation
    )
    vigente = esp(tmp_path, fila(1) + [3], hist((ORDEN, '2026-09-01', 10)))
    assert [a[:2] for a in vigente.avisos] == [(1, 'R11_BUSQUEDA_VIGENTE')]
    huerfano = esp(tmp_path, fila(1) + [3], hist(('Termino de búsqueda', '2026-09-20', 20)))
    assert 'consta un trámite descrito como «Termino de búsqueda»' in huerfano.observation


def test_c3_derecho_oido_solo_sin_fecha_en_rus(tmp_path):
    h = hist(('Derecho a ser oído', '2026-09-10', 12))
    enc = ['FEC. OIDO']
    assert (
        'Oído el 10 de septiembre de 2026.' in esp(tmp_path, fila(1) + [3], h, extra=[''], encabezados=enc).observation
    )
    f = esp(tmp_path, fila(1) + [3], h, extra=['2026-10-01'], encabezados=enc)
    assert 'Oído el 1 de octubre de 2026.' in f.observation and '10 de septiembre' not in f.observation


def test_t2_cita_ambos_tramites_una_sola_vez_y_marca_nivel2(tmp_path):
    h = hist(('Ingreso efectivo', '2026-09-01', 10), ('Deja sin efecto medida interconexión M.N.', '2026-09-15', 20))
    f = esp(tmp_path, fila(1) + [3], h)
    assert (
        '«Deja sin efecto medida interconexión M.N.», de fecha 15 de septiembre de 2026, folio 20, posterior al trámite descrito como «Ingreso efectivo», de fecha 1 de septiembre de 2026, folio 10.'
        in f.observation
    )
    assert f.observation.count('folio 10') == 1 and f.observation.count('folio 20') == 1
    assert (2, 'T2_MN') == f.avisos[0][:2] or any(a[:2] == (2, 'T2_MN') for a in f.avisos)
    anterior = hist(
        ('Deja sin efecto medida interconexión M.N.', '2026-09-01', 5), ('Ingreso efectivo', '2026-09-15', 6)
    )
    assert not any(a[1] == 'T2_MN' for a in esp(tmp_path, fila(1) + [3], anterior).avisos)


def test_mayoria_de_edad_sale_sola_y_marca_nivel3(tmp_path):
    f = esp(tmp_path, fila(1) + [45], extra=['2008-01-01'], encabezados=['FEC. NACIMIENTO'])
    assert f.rules == ['COMUN.MAYORIA_EDAD'] and [a[:2] for a in f.avisos] == [(3, 'REVISAR_MAYORIA')]
    assert 'Se sugiere egresar la medida.' in f.observation and 'días de espera' not in f.observation


def test_continuidad_se_cita_una_sola_vez_en_dce(tmp_path):
    f = esp(tmp_path, fila(1, 'DCE UNO') + [3], hist(('Resp. continuidad de atención', '2026-10-01', 4)))
    assert f.observation.count('Resp. continuidad de atención') == 1
    otra = esp(tmp_path, fila(1) + [3], hist(('Resp. continuidad de atención', '2026-10-01', 4)))
    assert otra.observation.count('Resp. continuidad de atención') == 1


def test_curador_dado_de_baja_o_designado_solo_se_dice_si_rus_no_tiene_curador(tmp_path):
    sin_rus = dict(extra=[''], encabezados=['CURADOR'])
    baja = ('Deja sin efecto designación C. Ad-Litem', '2026-09-01', 4)
    designa = ('Designa abogado', '2026-10-01', 5)
    f = esp(tmp_path, fila(1) + [3], hist(baja), **sin_rus)
    assert f.observation.split(': ', 1)[1] == (
        'Medida revisada, a la espera de ingreso efectivo al programa PRM UNO. '
        'Se hace presente que a folio 4, de fecha 1 de septiembre de 2026, se dejó sin efecto la designación de curador ad litem. '
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


def test_un_tramite_ya_citado_por_t2_no_se_vuelve_a_citar_en_la_regla3(tmp_path):
    h = hist(
        ('Ingreso efectivo', '2026-09-10', 10),
        ('Ingreso efectivo', '2026-09-20', 20),
        ('Deja sin efecto medida interconexión M.N.', '2026-09-25', 25),
    )
    f = esp(tmp_path, fila(1) + [40], h)
    assert f.observation.count('folio 20') == 1
    assert 'consta un trámite descrito como «Ingreso efectivo», de fecha 10 de septiembre de 2026, folio 10.' in (
        f.observation
    )
