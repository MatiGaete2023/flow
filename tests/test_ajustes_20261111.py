"""Ajustes del 11 de noviembre de 2026 (instrucción del usuario): composición, proyección de ingreso en Espera,
tipo de trámite, contexto anterior a la ventana y trámites sin regla. Casos ficticios."""

from datetime import date

from historia_fixtures import CUMPLIMIENTO, DESDE, HASTA, ORDEN, bita, correr, cum, cumplimiento, esp, fila, inf
from nurus.personal.motor.historia import construir


def _cuerpo(fila_):
    return fila_.observation.split(': ', 1)[1]


def _hist(*items, desde=DESDE):
    """items: (descripción, fecha, folio[, tipo de trámite])."""
    filas = [
        {
            'event_at': item[1],
            'description': item[0],
            'folio': str(item[2]),
            'raw_json': '{"Trámite": "%s"}' % item[3] if len(item) > 3 else '{}',
        }
        for item in items
    ]
    return construir(filas, 'COMPLETA', desde, HASTA)


# ---------- composición ----------
def test_cada_sugerencia_queda_junto_a_su_hecho(tmp_path):
    datos = cumplimiento(1, 'RTA UNO', ind='2026-02-01') + ['']
    f = correr(tmp_path, 'CUMPLIMIENTO', CUMPLIMIENTO + ['CURADOR'], [datos]).rows[0]
    assert _cuerpo(f) == (
        'No registra curador ad litem en RUS. Se sugiere asociarlo informáticamente. '
        'Ficha individual actualizada por última vez el 1 de febrero de 2026. Se sugiere actualizarla.'
    )


def test_e04_con_e05_no_repite_la_oracion_inicial_ni_declara_que_no_requiere_gestion(tmp_path):
    enc = ['FEC. RESOLUCION', 'CURADOR']
    f = esp(tmp_path, fila(1, 'DCE UNO') + [4], extra=['2026-10-10', ''], encabezados=enc)
    assert _cuerpo(f) == (
        'Medida revisada, a la espera de ingreso efectivo. Ingreso al programa DCE UNO ordenado el 10 de octubre de 2026. '
        'Se remite correo al programa consultando fecha estimada de ingreso. '
        'No registra curador ad litem en RUS. Se sugiere asociarlo informáticamente.'
    )
    assert f.actions == ['programa_espera']
    # Sin acción del Centro ni gestión del Tribunal, el paréntesis de E-04 se conserva.
    sola = esp(tmp_path, fila(1) + [4], extra=['2026-10-10', '11111111-1'], encabezados=enc)
    assert _cuerpo(sola).endswith('(Observación administrativa; no requiere gestión del Tribunal).')


def test_proximo_informe_con_medida_por_vencer_solo_si_vence_antes_del_egreso(tmp_path):
    from test_reglas_cumplimiento import CRUCE, _cumplimiento

    def ejecutar(fecha):
        cruce = [CRUCE, ['X-1-2026', '11111111-0', 'Persona Ficticia 1', 'LAJA', 'PRM UNO', fecha]]
        return _cumplimiento(tmp_path, [(100, 10, '2026-06-01', '2026-10-24')], cruce=cruce).rows[0]

    antes = ejecutar('2026-10-20')
    assert antes.rules == ['CUMPLIMIENTO.C05_POR_VENCER', 'CUMPLIMIENTO.C10_HOJA2']
    assert antes.actions == ['medidas', 'programa_por_vencer']
    despues = ejecutar('2026-10-28')
    assert despues.rules == ['CUMPLIMIENTO.C05_POR_VENCER'] and despues.actions == ['medidas']


# ---------- proyección de ingreso del programa (Espera) ----------
PROYECCION = 'Persona PRM: Medida revisada, a la espera de ingreso efectivo. Programa proyecta ingreso efectivo para el mes de {}.'


def test_proyeccion_de_un_mes_futuro_se_copia_literal_y_no_repite_gestiones(tmp_path):
    previa = bita(
        ('2026-09-20', PROYECCION.format('diciembre') + ' Audiencia decretada para el 25 de septiembre de 2026.')
    )
    f = esp(tmp_path, fila(1) + [65], bitacora=previa, extra=[''], encabezados=['CURADOR'])
    # Solo se copia la oración de la proyección; el resto se arma con los datos vigentes.
    assert _cuerpo(f) == (
        'Medida revisada, a la espera de ingreso efectivo. Programa proyecta ingreso efectivo para el mes de diciembre. '
        'No registra curador ad litem en RUS. Se sugiere asociarlo informáticamente.'
    )
    assert f.rules == ['ESPERA.E07_PROYECCION', 'COMUN.CURADOR'] and f.actions == []


def test_proyeccion_del_mes_del_corte_o_ya_pasada_vuelve_a_la_regla_por_defecto(tmp_path):
    for mes in ('octubre', 'septiembre'):
        previa = bita(('2026-09-20', PROYECCION.format(mes)))
        laja = esp(tmp_path, fila(1) + [65], bitacora=previa)
        assert laja.rules == ['ESPERA.E05_PROYECTO_Y_CORREO'] and laja.actions == ['programa_espera', 'PC_IE']
        # No se fuerza proyecto y correo: DCE sigue con solo correo y bajo el umbral no hay gestión.
        dce = esp(tmp_path, fila(1, 'DCE UNO') + [65], bitacora=previa)
        assert dce.rules == ['ESPERA.E05_SOLO_CORREO'] and dce.actions == ['programa_espera']
        assert esp(tmp_path, fila(1) + [10], bitacora=previa).rules == ['ESPERA.E06_SIN_RESOLUCION']


def test_proyeccion_solo_cuenta_en_la_ultima_observacion_y_admite_el_anio(tmp_path):
    antigua = bita(('2026-08-01', PROYECCION.format('diciembre')), ('2026-09-20', 'Persona PRM: Medida revisada.'))
    assert esp(tmp_path, fila(1) + [65], bitacora=antigua).rules == ['ESPERA.E05_PROYECTO_Y_CORREO']
    # Enero sin año, escrito en septiembre, es enero del año siguiente.
    enero = esp(tmp_path, fila(1) + [65], bitacora=bita(('2026-09-20', PROYECCION.format('enero'))))
    assert enero.rules == ['ESPERA.E07_PROYECCION']
    pasado = esp(tmp_path, fila(1) + [65], bitacora=bita(('2026-09-20', PROYECCION.format('diciembre de 2025'))))
    assert pasado.rules == ['ESPERA.E05_PROYECTO_Y_CORREO']
    # Sin bitácora (Excel) rigen las reglas actuales.
    assert esp(tmp_path, fila(1) + [65]).rules == ['ESPERA.E05_PROYECTO_Y_CORREO']


def test_mayoria_de_edad_prevalece_sobre_la_proyeccion(tmp_path):
    previa = bita(('2026-09-20', PROYECCION.format('diciembre')))
    f = esp(tmp_path, fila(1) + [65], bitacora=previa, extra=['2000-01-01'], encabezados=['FEC. NACIMIENTO'])
    assert f.rules == ['COMUN.MAYORIA_EDAD']


# ---------- historia: tipo de trámite, novedad, sin regla, contexto ----------
def test_el_tramite_se_nombra_por_lo_que_es(tmp_path):
    escrito = cum(tmp_path, cumplimiento(1), _hist(('Inf. Cump Programa Ambulatorio', '2026-10-01', 30, 'Escrito')))
    assert 'Informe de avance recepcionado en escrito de fecha 1 de octubre de 2026 (folio 30).' in escrito.observation
    assert escrito.actions == ['NOMENCL']
    actuacion = cum(tmp_path, cumplimiento(1), _hist(('Resp. continuidad de atención', '2026-10-01', 4, 'Actuación')))
    assert 'en actuación de fecha 1 de octubre de 2026 (folio 4)' in actuacion.observation
    # Resolución, o sin el dato (paquetes anteriores): se conserva «en resolución de fecha».
    for tipo in ('Resolución', None):
        item = ('Resp. continuidad de atención', '2026-10-01', 4) + ((tipo,) if tipo else ())
        assert (
            'en resolución de fecha 1 de octubre de 2026 (folio 4)'
            in cum(tmp_path, cumplimiento(1), _hist(item)).observation
        )


def test_una_constancia_ya_revisada_no_se_repite(tmp_path):
    h = _hist(('Resp. continuidad de atención', '2026-08-01', 5))
    assert 'continuidad de atención' in cum(tmp_path, cumplimiento(1), h).observation
    # Observación posterior al trámite: ya se revisó, cite o no su folio.
    posterior = bita(('2026-09-01', 'Persona PRM: Medida revisada, sin observaciones.'))
    assert _cuerpo(cum(tmp_path, cumplimiento(1), h, posterior)) == 'Medida revisada, sin observaciones.'
    # Observación del mismo día que cita el folio: tampoco se repite; sin citarlo, se dice.
    citada = bita(('2026-08-01', 'Persona PRM: consta respuesta de continuidad (folio 5).'))
    assert 'continuidad' not in cum(tmp_path, cumplimiento(1), h, citada).observation
    mismo_dia = bita(('2026-08-01', 'Persona PRM: Medida revisada, sin observaciones.'))
    assert 'continuidad de atención' in cum(tmp_path, cumplimiento(1), h, mismo_dia).observation


def test_tramites_sin_regla_solo_dejan_una_marca(tmp_path):
    h = _hist(
        ('Pide cuenta informe', '2026-10-05', 40),
        ('Audiencia de revisión', '2026-10-06', 41),
        ('Prórroga de medida', '2026-10-07', 42),
    )
    f = cum(tmp_path, cumplimiento(1), h)
    assert _cuerpo(f) == 'Medida revisada, sin observaciones.'
    assert [a[:2] for a in f.avisos] == [(3, 'TRAMITES_SIN_REGLA')]
    assert (
        f.avisos[0][2]
        == 'Trámites sin regla en la historia (3): Audiencia de revisión, Pide cuenta informe, Prórroga de medida'
    )
    # Las nomenclaturas ignoradas por decisión y las ya revisadas no cuentan.
    ignorada = cum(tmp_path, cumplimiento(1), _hist(('Derivación Dirección Reg.', '2026-10-05', 40)))
    assert ignorada.avisos == []
    vista = cum(tmp_path, cumplimiento(1), h, bita(('2026-10-10', 'Persona PRM: Medida revisada, sin observaciones.')))
    assert vista.avisos == []


def test_orden_de_busqueda_anterior_a_la_ventana_sigue_vigente_como_contexto(tmp_path):
    # El paquete conserva el último trámite anterior a la ventana de ciertas nomenclaturas (contexto).
    antigua = _hist((ORDEN, '2026-03-01', 10))
    f = cum(tmp_path, cumplimiento(1), antigua)
    assert [a[1] for a in f.avisos] == ['R11_BUSQUEDA_VIGENTE']
    assert _cuerpo(f) == 'Medida revisada, sin observaciones.'  # contexto: no se redacta como constancia nueva
    # Con su término en la ventana, la búsqueda se dice completa y deja de estar vigente.
    cerrada = cum(
        tmp_path, cumplimiento(1), _hist((ORDEN, '2026-03-01', 10), ('Termino de búsqueda', '2026-10-02', 60))
    )
    assert 'se decretó orden de búsqueda' in cerrada.observation and 'se dispuso el término' in cerrada.observation
    assert cerrada.avisos == []


def test_dos_ordenes_y_un_termino_dejan_una_orden_vigente(tmp_path):
    h = _hist((ORDEN, '2026-09-25', 58), (ORDEN, '2026-09-26', 59), ('Termino de búsqueda', '2026-10-02', 60))
    assert [a[1] for a in cum(tmp_path, cumplimiento(1), h).avisos] == ['R11_BUSQUEDA_VIGENTE']


def test_causa_p_finalizada_no_caduca_al_salir_la_sentencia_de_la_ventana(tmp_path):
    datos = fila(1, rit='P-5-2026') + [300, 100, '2026-01-01', '2027-05-01', '', '', '']
    f = cum(tmp_path, datos, _hist(('Dictación de sentencia', '2026-02-01', 30)))
    assert f.rules == ['HISTORIA.P_FINALIZADA'] and f.actions == ['NOMENCL']
    egresada = _hist(
        ('Dictación de sentencia', '2026-02-01', 30),
        ('Egreso de servicio protección especializada y otras redes', '2026-03-01', 31),
    )
    assert cum(tmp_path, datos, egresada).rules == ['CUMPLIMIENTO.C09_SIN_OBSERVACIONES']


def test_contexto_anterior_exporta_solo_el_ultimo_de_cada_nomenclatura():
    from nurus.capture.historias import contexto_anterior

    def f(n, desc, fecha, folio):
        return {'fila': n, 'descripcion': desc, 'fecha_iso': fecha, 'folio': str(folio)}

    anteriores = [
        f(1, 'Orden de búsqueda', '2026-03-01', 10),
        f(2, 'Orden de búsqueda', '2026-01-01', 5),
        f(3, 'Dictación de sentencia', '2025-12-01', 2),
        f(4, 'Otro trámite', '2026-02-01', 7),
        f(5, 'Prorroga', '', 8),
    ]
    assert [(x['descripcion'], x['folio']) for x in contexto_anterior(anteriores)] == [
        ('Orden de búsqueda', '10'),
        ('Dictación de sentencia', '2'),
    ]


def test_informe_vencido_y_un_solo_registro_mantiene_la_contradiccion(tmp_path):
    h = _hist(('Ingreso informe de cumplimiento X', '2026-10-05', 40))
    f = inf(tmp_path, fila(1) + ['2026-10-01'], h)
    assert f.actions == [] and [a[:2] for a in f.avisos] == [(2, 'I01_CONTRADICHA')]
    assert date(2026, 10, 14) == HASTA  # corte ficticio de los fixtures
