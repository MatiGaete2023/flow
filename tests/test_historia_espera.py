"""WP-R5: reglas de historia en Espera (1, 3, 5, 6, 7, 8, E9, E10) y transversales (R9, R11, C3, T2, T3, T5)."""

from historia_fixtures import ORDEN, bita, correr, esp, fila, hist, ESPERA


def test_regla1_folio_solo_si_es_unico(tmp_path):
    datos = fila(1) + [5]
    enc = ['FEC. RESOLUCION']
    unico = esp(tmp_path, datos, hist(('Resolución ingreso', '2026-10-10', 77)), extra=['2026-10-10'], encabezados=enc)
    assert (
        'HISTORIA.E04_CON_FOLIO' in unico.rules and 'ordenado el 10 de octubre de 2026, folio 77.' in unico.observation
    )
    doble = hist(('Resolución ingreso', '2026-10-10', 77), ('Otro', '2026-10-10', 78))
    assert 'ESPERA.E04_RESOLUCION_RECIENTE' in esp(tmp_path, datos, doble, extra=['2026-10-10'], encabezados=enc).rules


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


def test_regla3_multi_ingreso_agrega_aclaracion(tmp_path):
    filas = [fila(1, rit='C-1-2026') + [45], fila(2, 'PRM DOS', rit='C-1-2026') + [45]]
    h = hist(('Ingreso efectivo', '2026-10-02', 41))
    f = correr(tmp_path, 'ESPERA', ESPERA, filas, {0: h, 1: h}).rows[0]
    assert 'La historia es de la causa y no permite atribuirlo a un ingreso determinado.' in f.observation


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
    assert (
        '«Pide cuenta Derivación D.R.»' in dr.observation
        and '«Ing.Serv. protección especializada y otras redes»' in dr.observation
    )
    larga = esp(tmp_path, fila(1, 'RTA UNO') + [120])
    assert larga.observation.endswith('Registra 120 días en lista de espera.')
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
        'Se reitera observación de fecha 1 de septiembre de 2026, respecto de la ausencia de curador ad litem en RUS.'
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
        'de fecha 20 de septiembre de 2026, folio 20, que pone término a la búsqueda decretada mediante «Orden de búsqueda» de fecha 1 de septiembre de 2026, folio 10.'
        in f.observation
    )
    assert f.avisos == [] and f.observation.count('Orden de búsqueda') == 1
    sin_efecto = esp(
        tmp_path, fila(1) + [3], hist((ORDEN, '2026-09-01', 10), ('Deja sin efecto búsqueda', '2026-09-20', 20))
    )
    assert 'que deja sin efecto la búsqueda decretada' in sin_efecto.observation
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


def test_designacion_de_curador_se_cita(tmp_path):
    f = esp(tmp_path, fila(1) + [3], hist(('Designa abogado', '2026-10-01', 4)), extra=[''], encabezados=['CURADOR'])
    assert '«Designa abogado»' in f.observation and 'Se sugiere asociarlo informáticamente.' in f.observation


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
