"""Plan de integración 18-nov-2026: otro programa vigente, informe de avance pendiente en Escritos y asociación de
cada informe con su proyección. Datos ficticios; mismos casos que el oráculo Rust (rust/oraculo/generar_integracion.py)."""

from datetime import date

import pytest
from historia_fixtures import CORTE, CUMPLIMIENTO, ESPERA, INFORMES, bita, cumplimiento, fila, hist
from openpyxl import Workbook

from nurus.personal.config import defaults
from nurus.personal.motor.evidencia import (
    Escrito,
    Escritos,
    EvidenciaInforme,
    Historico,
    ProgramaHistorico,
    constancia_en_bitacora,
    programas_vigentes_otros,
)
from nurus.personal.work import Work

FRASE = 'Se hace presente que Persona cuenta con ingreso efectivo al programa {}.'
REV = 'Se hace presente que el informe de PRM LAJA registra ingreso a folio 58 y figura pendiente en el calendario. Se sugiere revisar su asociación con la fecha proyectada 20 de septiembre de 2026.'
PENDIENTE = 'Pendiente de resolver, informe de avance.'
VIG = ('--', 'SIN INFORMACION')


def analizar(tmp_path, modo, encabezados, filas, **extra):
    """extra: nombre -> {indice_de_fila: valor}; se traduce a record_id como hace el importador de paquetes."""
    libro = Workbook()
    hoja = libro.active
    hoja.title = modo
    hoja.append(encabezados)
    for datos in filas:
        hoja.append(datos)
    ruta = tmp_path / f'i_{modo.lower()}.xlsx'
    libro.save(ruta)
    ids = [r.id for r in Work(defaults()).analyze(ruta, modo, as_of=CORTE).rows]
    claves = {
        'historias': 'historia_by_record', 'bitacoras': 'bitacora_by_record', 'escritos': 'escritos_by_record',
        'historicos': 'historico_by_record', 'evidencias': 'evidencia_by_record',
        'bitacora_parcial': 'bitacora_parcial_by_record',
    }  # fmt: skip
    kwargs = {claves[k]: {ids[i]: v for i, v in d.items()} for k, d in extra.items() if k in claves}
    trabajo = Work(defaults())
    for fila_, programas in (extra.get('consignados') or {}).items():
        for programa in programas:
            trabajo.consignar(ids[fila_], programa)
    return trabajo.analyze(ruta, modo, as_of=CORTE, **kwargs)


def historico(*filas, cobertura='COMPLETA', identidad='VERIFICADA'):
    return Historico(cobertura, CORTE, identidad, tuple(ProgramaHistorico(*f, fila=n) for n, f in enumerate(filas, 1)))


def escritos(*filas, cobertura='COMPLETA', leido=CORTE):
    return Escritos(cobertura, leido, tuple(Escrito(*f, fila=n) for n, f in enumerate(filas, 1)))


def esp(tmp_path, **extra):
    return analizar(tmp_path, 'ESPERA', ESPERA, [fila(1, 'PRM Laja') + [40]], **extra).rows[0]


def cum(tmp_path, programa='PRM Laja', **extra):
    return analizar(tmp_path, 'CUMPLIMIENTO', CUMPLIMIENTO, [cumplimiento(1, programa)], **extra).rows[0]


# ---------------------------------------------------------------- otro programa vigente
def test_otro_programa_vigente_frase_exacta_una_sola_vez_y_sin_acciones_nuevas(tmp_path):
    h = historico(
        ('PRM Laja', *VIG), ('PRM Laja', *VIG), ('FAE Biobío', *VIG), ('FAE Biobío', *VIG), ('PDE Mulchén', '01/05/2026', 'EGRESO')
    )
    for fila_ in (cum(tmp_path, historicos={0: h}), esp(tmp_path, historicos={0: h})):
        assert fila_.observation.count(FRASE.format('FAE Biobío').replace('Persona', 'Persona')) == 1
        assert 'PDE Mulchén' not in fila_.observation and 'cuenta con ingreso efectivo' in fila_.observation
        assert 'HISTORIA.OTRO_PROGRAMA' in fila_.rules
        assert [o['programa'] for o in fila_.otros_programas] == ['FAE Biobío']
    # Sin gestión nueva por la frase: las acciones son las que ya tenía la fila.
    base = cum(tmp_path)
    assert cum(tmp_path, historicos={0: h}).actions == base.actions


def test_otro_programa_condiciones_de_vigencia_en_la_misma_fila(tmp_path):
    caso = lambda *f: cum(tmp_path, historicos={0: historico(('PRM Laja', *VIG), *f)}).observation  # noqa: E731
    assert 'cuenta con ingreso efectivo' not in caso(('FAE X', '--', 'EGRESO VOLUNTARIO'))  # causal sola
    assert 'cuenta con ingreso efectivo' not in caso(('FAE X', '01/05/2026', 'SIN INFORMACION'))  # fecha sola
    assert 'cuenta con ingreso efectivo' not in caso(('FAE X', '', ''))  # dato ilegible no acredita vigencia
    # Una fila antigua egresada y otra vigente del mismo nombre: vale la vigente.
    assert FRASE.format('FAE X') in caso(('FAE X', '01/05/2025', 'EGRESO'), ('FAE X', *VIG))
    # Dos otros programas vigentes de nombres diferentes: dos constancias.
    dos = caso(('FAE X', *VIG), ('PIE Y', *VIG))
    assert FRASE.format('FAE X') in dos and 'Asimismo, se hace presente que Persona cuenta con ingreso efectivo al programa PIE Y.' in dos


def test_otro_programa_compara_el_nombre_completo_no_la_sigla(tmp_path):
    h = historico(('PRM Laja Norte', *VIG), ('prm  LAJA', *VIG))
    assert programas_vigentes_otros(h, 'PRM Laja') == ['PRM Laja Norte']  # el actual se reconoce con otra grafía
    assert programas_vigentes_otros(historico(('PRM Otro', *VIG)), 'PRM Laja') == ['PRM Otro']
    assert programas_vigentes_otros(historico(('PRM Laja', *VIG)), 'PRM Laja') == []


@pytest.mark.parametrize(
    'h',
    [
        historico(('FAE X', *VIG), cobertura='FALLIDA'),
        historico(('FAE X', *VIG), cobertura='NO_SOLICITADA'),
        historico(('FAE X', *VIG), cobertura='VACIA_COMPROBADA'),
        historico(('FAE X', *VIG), identidad='INCIERTA'),
    ],
)
def test_historico_no_verificable_no_afirma_y_se_distingue(tmp_path, h):
    f = cum(tmp_path, historicos={0: h})
    assert 'cuenta con ingreso efectivo' not in f.observation
    codigos = [a[1] for a in f.avisos]
    assert ('HISTORICO_NO_DISPONIBLE' in codigos) == (h.cobertura == 'FALLIDA')
    assert ('HISTORICO_IDENTIDAD' in codigos) == (h.identidad != 'VERIFICADA')


def test_otro_programa_se_consigna_una_vez_aunque_cambie_la_etapa(tmp_path):
    h = historico(('FAE X', *VIG))
    registrada = bita(('2026-10-01', 'Persona PRM: Medida revisada. Se hace presente que Persona cuenta con ingreso efectivo al programa FAE X.'))
    assert FRASE.format('FAE X') not in esp(tmp_path, historicos={0: h}, bitacoras={0: registrada}).observation
    assert FRASE.format('FAE X') not in cum(tmp_path, historicos={0: h}, bitacoras={0: registrada}).observation


def test_constancia_previa_exige_mismo_nino_programa_completo_y_sin_negacion():
    def previa(texto):
        return constancia_en_bitacora(bita(('2026-10-01', texto)), 'Persona', 'PRM Laja') is not None

    assert previa('Se hace presente que Persona cuenta con ingreso efectivo al programa PRM Laja.')
    assert previa('Persona ingresó efectivamente al programa PRM Laja.')
    assert not previa('Se hace presente que Persona cuenta con ingreso efectivo al programa PRM Laja Norte.')
    assert not previa('Se hace presente que Otra cuenta con ingreso efectivo al programa PRM Laja.')
    assert not previa('Persona no cuenta con ingreso efectivo al programa PRM Laja.')
    assert not previa('Se evalúa el programa PRM Laja para Persona.')


def test_marca_local_borrador_descartado_y_bitacora_parcial(tmp_path):
    h = historico(('FAE X', *VIG))
    assert FRASE.format('FAE X') in cum(tmp_path, historicos={0: h}).observation  # borrador descartado: no consta
    consignado = cum(tmp_path, historicos={0: h}, consignados={0: ['fae  x']})
    assert FRASE.format('FAE X') not in consignado.observation
    parcial = cum(tmp_path, historicos={0: h}, bitacora_parcial={0: True})
    assert FRASE.format('FAE X') in parcial.observation and 'OTRO_PROGRAMA_BITACORA_PARCIAL' in [a[1] for a in parcial.avisos]
    mencion = cum(tmp_path, historicos={0: h}, bitacoras={0: bita(('2026-10-01', 'Se coordinó con el programa FAE X.'))})
    assert FRASE.format('FAE X') in mencion.observation and 'OTRO_PROGRAMA_MENCION_PREVIA' in [a[1] for a in mencion.avisos]


def test_primer_nombre_indeterminado_pide_revision_y_no_inventa(tmp_path):
    h = historico(('FAE X', *VIG))
    libro = [['X-1-2026', 'LAJA', '', '11111111-0', 'PRM Laja', 40]]
    f = analizar(tmp_path, 'ESPERA', ESPERA, libro, historicos={0: h}).rows[0]
    assert 'cuenta con ingreso efectivo' not in f.observation
    assert 'OTRO_PROGRAMA_SIN_NOMBRE' in [a[1] for a in f.avisos]


def test_consignar_es_idempotente(tmp_path):
    trabajo = Work(defaults())
    assert trabajo.consignar('r1', 'FAE  X') == trabajo.consignar('r1', 'fae x')
    assert trabajo.consignaciones == {'r1': ['fae x']}


# ---------------------------------------------------------------- escritos pendientes
TIPOS = {
    'PRM Laja': 'Inf. Cump Programa Ambulatorio',
    'AFT Laja': 'Inf. Cump Programa Ambulatorio',
    'PIE Tomé': 'Inf. Cump Programa Ambulatorio',
    'PDE Mulchén': 'Inf. Cump Programa Ambulatorio',
    'PDC Laja': 'Inf. Cump Programa Ambulatorio',
    'FAE PRO Biobío': 'Inf. Cump FAE/FAS',
    'FAS Tomé': 'Inf. Cump FAE/FAS',
    'RES Residencia Familiar': 'Inf. Cump Centro Residencial',
}


def informes(tmp_path, programa, *fechas, **extra):
    filas = [fila(1, programa) + [f] for f in fechas]
    return analizar(tmp_path, 'INFORMES', INFORMES, filas, **extra).rows


@pytest.mark.parametrize('programa,tipo', TIPOS.items())
def test_escrito_pendiente_del_tipo_correcto_da_la_frase_y_excluye_el_pide_cuenta(tmp_path, programa, tipo):
    [f] = informes(tmp_path, programa, '2026-09-20', escritos={0: escritos((tipo, 'Pendiente'))})
    assert f.observation.endswith(PENDIENTE) and f.observation.count(PENDIENTE) == 1
    assert f.actions == [] and 'INFORMES.I01_VENCIDO_GENERAL' not in f.rules
    assert 'I01_ESCRITO_PENDIENTE' in [a[1] for a in f.avisos]


@pytest.mark.parametrize(
    'fila_escrito',
    [
        ('Inf. Cump FAE/FAS', 'Pendiente'),  # tipo de otra familia
        ('Inf. Cump Programa Ambulatorio', 'Resuelto'),  # estado distinto
        ('Otro escrito', 'Pendiente'),
    ],
)
def test_escrito_de_otro_tipo_o_estado_no_activa_la_regla(tmp_path, fila_escrito):
    [f] = informes(tmp_path, 'PRM Laja', '2026-09-20', escritos={0: escritos(fila_escrito)})
    assert PENDIENTE not in f.observation and f.actions == ['programa_vencido', 'PC_INFO']


def test_tipo_correcto_en_una_fila_y_pendiente_en_otra_no_activa_la_regla(tmp_path):
    e = escritos(('Inf. Cump Programa Ambulatorio', 'Resuelto'), ('Otro escrito', 'Pendiente'))
    [f] = informes(tmp_path, 'PRM Laja', '2026-09-20', escritos={0: e})
    assert PENDIENTE not in f.observation and f.actions == ['programa_vencido', 'PC_INFO']


@pytest.mark.parametrize('cobertura', ['VACIA_COMPROBADA', 'FALLIDA', 'NO_SOLICITADA', 'NO_CONSULTADA'])
def test_bandeja_sin_filas_utilizables_no_cambia_nada(tmp_path, cobertura):
    [f] = informes(tmp_path, 'PRM Laja', '2026-09-20', escritos={0: Escritos(cobertura, CORTE, ())})
    assert PENDIENTE not in f.observation and f.actions == ['programa_vencido', 'PC_INFO']


def test_dos_candidatos_pendientes_sin_vinculo_es_ambiguo_y_conserva_acciones(tmp_path):
    t = 'Inf. Cump Programa Ambulatorio'
    [f] = informes(tmp_path, 'PRM Laja', '2026-09-20', escritos={0: escritos((t, 'Pendiente'), (t, 'Pendiente'))})
    assert PENDIENTE not in f.observation and f.actions == ['programa_vencido', 'PC_INFO']
    assert 'ESCRITO_PENDIENTE_SIN_ATRIBUIR' in [a[1] for a in f.avisos]


def test_pendiente_tambien_se_detecta_desde_cumplimiento(tmp_path):
    f = cum(tmp_path, escritos={0: escritos(('Inf. Cump Programa Ambulatorio', 'Pendiente'))})
    assert PENDIENTE in f.observation and f.observation.count(PENDIENTE) == 1
    assert cum(tmp_path).actions == f.actions


def test_historia_posterior_prevalece_sobre_la_bandeja_y_el_conflicto_se_marca(tmp_path):
    t = 'Inf. Cump Programa Ambulatorio'
    historia = hist((t, '2026-09-25', 30), hasta=date(2026, 10, 14))
    [vieja] = informes(tmp_path, 'PRM Laja', '2026-09-20', historias={0: historia}, escritos={0: escritos((t, 'Pendiente'), leido=date(2026, 10, 1))})
    assert PENDIENTE not in vieja.observation and 'recepcionado en resolución de fecha 25 de septiembre' in vieja.observation
    [dudosa] = informes(tmp_path, 'PRM Laja', '2026-09-20', historias={0: historia}, escritos={0: escritos((t, 'Pendiente'), leido=date(2026, 10, 14))})
    assert PENDIENTE not in dudosa.observation and 'ESCRITO_ESTADOS_INCOMPATIBLES' in [a[1] for a in dudosa.avisos]


# ---------------------------------------------------------------- varias fechas del mismo ingreso
def test_ingreso_con_tres_fechas_cada_una_con_su_evidencia(tmp_path):
    t = 'Inf. Cump Programa Ambulatorio'
    bandeja = Escritos('COMPLETA', CORTE, (Escrito(t, 'Pendiente', vinculo='EXPLICITO', fecha_proyectada=date(2026, 8, 20)),))
    historia = hist((t, '2026-09-25', 30))
    evidencias = {1: EvidenciaInforme(escrito_historia_folio='30', fecha_proyectada=date(2026, 9, 20))}
    a, b, c = informes(
        tmp_path, 'PRM Laja', '2026-08-20', '2026-09-20', '2026-10-05',
        historias={0: historia, 1: historia, 2: historia}, escritos={0: bandeja, 1: bandeja, 2: bandeja}, evidencias=evidencias,
    )  # fmt: skip
    assert PENDIENTE in a.observation and a.actions == []
    assert 'recepcionado en resolución de fecha 25 de septiembre de 2026 (folio 30)' in b.observation and b.actions == ['NOMENCL']
    assert c.actions == ['programa_vencido', 'PC_INFO'] and 'Informe de avance vencido desde el 5 de octubre de 2026.' in c.observation
    assert 'Registra 3 informes vencidos en RUS; el más antiguo venció el 20 de agosto de 2026.' in c.observation


def test_cinco_fechas_dos_acreditadas_dejan_tres_exigibles_y_una_fila_duplicada_no_cuenta_doble(tmp_path):
    fechas = ['2026-06-20', '2026-07-20', '2026-08-20', '2026-09-20', '2026-10-05', '2026-10-05']
    evidencias = {i: EvidenciaInforme(resolucion_folio=str(40 + i)) for i in (0, 1)}
    historia = hist(('Ingreso informe de cumplimiento X', '2026-07-25', 40), ('Ingreso informe de cumplimiento X', '2026-08-10', 41))
    filas = informes(tmp_path, 'PRM Laja', *fechas, historias={i: historia for i in range(6)}, evidencias=evidencias)
    acreditadas, exigibles = filas[:2], filas[2:]
    for f in acreditadas:
        assert f.actions == [] and 'figura pendiente en el calendario' in f.observation
    for f in exigibles:
        assert 'Registra 6 informes vencidos' not in f.observation
        assert f.actions == ['programa_vencido', 'PC_INFO']


def test_nomenclatura_presente_y_calendario_pendiente_pide_revisar_asociacion(tmp_path):
    historia = hist(('Ingreso informe de cumplimiento X', '2026-09-25', 58))
    [f] = informes(tmp_path, 'PRM Laja', '2026-09-20', historias={0: historia})
    assert REV in f.observation
    assert f.actions == [] and 'NOMENCL' not in f.actions


def test_escrito_incorporado_sin_ingreso_posterior_conserva_el_proyecto_de_nomenclatura(tmp_path):
    t = 'Inf. Cump Programa Ambulatorio'
    [f] = informes(tmp_path, 'PRM Laja', '2026-09-20', historias={0: hist((t, '2026-09-25', 30))})
    assert 'se remite proyecto de resolución que aplica la nomenclatura «Ingreso Informe de Cumplimiento X»' in f.observation
    assert f.actions == ['NOMENCL']


def test_productos_solo_incluyen_la_fecha_exigible_y_nomenclatura_va_por_su_cuenta(tmp_path):
    """Paso 8: la decisión llega a correo/Word por acciones explícitas, no por el prefijo del identificador de regla."""
    from nurus.personal.resolutions import automatic_project_selections

    t = 'Inf. Cump Programa Ambulatorio'
    bandeja = Escritos('COMPLETA', CORTE, (Escrito(t, 'Pendiente', vinculo='EXPLICITO', fecha_proyectada=date(2026, 8, 20)),))
    historia = hist((t, '2026-09-25', 30))
    filas = [fila(1, 'PRM Laja') + [f] for f in ('2026-08-20', '2026-09-20', '2026-10-05')]
    trabajo = analizar(
        tmp_path, 'INFORMES', INFORMES, filas,
        historias={i: historia for i in range(3)}, escritos={i: bandeja for i in range(3)},
        evidencias={1: EvidenciaInforme(escrito_historia_folio='30')},
    )  # fmt: skip
    pendiente, incorporada, exigible = trabajo.rows
    assert [r.actions for r in trabajo.rows] == [[], ['NOMENCL'], ['programa_vencido', 'PC_INFO']]
    assert automatic_project_selections(trabajo) == [(incorporada.id, 'NOMENCL'), (exigible.id, 'PC_INFO')]
    assert 'INFORMES.I01_VENCIDO_GENERAL' not in pendiente.rules + incorporada.rules


def test_proyectos_word_no_arrastran_fechas_resueltas_de_otro_periodo(tmp_path):
    from nurus.personal.config import BASE
    from nurus.personal.resolutions import automatic_project_selections, prepare_projects

    t = 'Inf. Cump Programa Ambulatorio'
    bandeja = Escritos('COMPLETA', CORTE, (Escrito(t, 'Pendiente', vinculo='EXPLICITO', fecha_proyectada=date(2026, 8, 20)),))
    historia = hist((t, '2026-09-25', 30))
    filas = [fila(1, 'PRM Laja') + [f] for f in ('2026-08-20', '2026-09-20', '2026-10-05')]
    trabajo = analizar(
        tmp_path, 'INFORMES', INFORMES, filas,
        historias={i: historia for i in range(3)}, escritos={i: bandeja for i in range(3)},
        evidencias={1: EvidenciaInforme(escrito_historia_folio='30')},
    )  # fmt: skip
    trabajo.export(tmp_path / 'copia.xlsx', backend='portable', reduced_fidelity=True)
    proyectos, errores = prepare_projects(trabajo, automatic_project_selections(trabajo), BASE / 'plantillas_word')
    pendiente, incorporada, exigible = (r.id for r in trabajo.rows)
    por_tipo = {p.kind: p.record_ids for p in proyectos}
    assert not errores and por_tipo == {'NOMENCL': [incorporada], 'PC_INFO': [exigible]}


def test_cumplimiento_atribuye_un_unico_escrito_pendiente_vinculado_a_su_ingreso(tmp_path):
    t = 'Inf. Cump Programa Ambulatorio'
    vinculado = Escritos('COMPLETA', CORTE, (Escrito(t, 'Pendiente', vinculo='EXPLICITO', fecha_proyectada=date(2026, 8, 20)),))
    f = cum(tmp_path, escritos={0: vinculado})
    assert PENDIENTE in f.observation and 'ESCRITO_PENDIENTE_SIN_ATRIBUIR' not in [a[1] for a in f.avisos]


def test_e3_09_decision_humana_sin_proyecto_no_reaparece_en_el_word_del_caso(tmp_path):
    from nurus.personal.config import BASE
    from nurus.personal.resolutions import automatic_project_selections, prepare_projects

    t = 'Inf. Cump Programa Ambulatorio'
    bandeja = Escritos('COMPLETA', CORTE, (Escrito(t, 'Pendiente', vinculo='EXPLICITO', fecha_proyectada=date(2026, 8, 20)),))
    filas = [fila(1, 'PRM Laja') + [f] for f in ('2026-08-20', '2026-10-05')]
    trabajo = analizar(tmp_path, 'INFORMES', INFORMES, filas, escritos={0: bandeja, 1: bandeja})
    pendiente, exigible = trabajo.rows
    pendiente.decisions['resolution'] = 'none'
    trabajo.export(tmp_path / 'copia.xlsx', backend='portable', reduced_fidelity=True)
    seleccion = automatic_project_selections(trabajo)
    assert seleccion == [(exigible.id, 'PC_INFO')]
    proyectos, errores = prepare_projects(trabajo, seleccion, BASE / 'plantillas_word')
    assert not errores and [p.record_ids for p in proyectos] == [[exigible.id]]


# ---------------------------------------------------------------- correcciones E3 (19-nov-2026)
def test_e3_01_escrito_vinculado_a_otro_ingreso_no_retira_el_pide_cuenta_de_este(tmp_path):
    t = 'Inf. Cump Programa Ambulatorio'
    ajeno = Escritos('COMPLETA', CORTE, (Escrito(t, 'Pendiente', vinculo='AJENO'),))
    [f] = informes(tmp_path, 'PRM Laja', '2026-08-25', escritos={0: ajeno})
    assert f.actions == ['programa_vencido', 'PC_INFO'] and PENDIENTE not in f.observation
    assert 'Pendiente' not in cum(tmp_path, escritos={0: ajeno}).observation


def test_e3_05_bandeja_parcial_o_de_identidad_incierta_no_acredita_candidato_unico(tmp_path):
    t = 'Inf. Cump Programa Ambulatorio'
    for cobertura in ('PARCIAL', 'IDENTIDAD_INCIERTA'):
        [f] = informes(tmp_path, 'PRM Laja', '2026-09-20', escritos={0: escritos((t, 'Pendiente'), cobertura=cobertura)})
        assert f.actions == ['programa_vencido', 'PC_INFO'] and PENDIENTE not in f.observation, cobertura
    # Un vínculo propio explícito sí vale en una lectura parcial de identidad verificada.
    propio = Escritos('PARCIAL', CORTE, (Escrito(t, 'Pendiente', vinculo='EXPLICITO', fecha_proyectada=date(2026, 9, 20)),))
    [g] = informes(tmp_path, 'PRM Laja', '2026-09-20', escritos={0: propio})
    assert g.actions == [] and PENDIENTE in g.observation


def test_e3_06_lecturas_posteriores_al_corte_no_deciden(tmp_path):
    t = 'Inf. Cump Programa Ambulatorio'
    futura = escritos((t, 'Pendiente'), leido=date(2026, 10, 24))  # el corte es 14-oct
    [f] = informes(tmp_path, 'PRM Laja', '2026-09-20', escritos={0: futura})
    assert f.actions == ['programa_vencido', 'PC_INFO']
    h = Historico('COMPLETA', date(2026, 10, 24), 'VERIFICADA', (ProgramaHistorico('FAE X', *VIG),))
    assert 'cuenta con ingreso efectivo' not in cum(tmp_path, historicos={0: h}).observation


def test_e3_10_folio_asociado_a_otro_informe_no_se_atribuye_por_la_via_generica(tmp_path):
    historia = hist(('Ingreso informe de cumplimiento X', '2026-09-25', 31))
    ajeno = EvidenciaInforme(fecha_proyectada=date(2026, 9, 20), folios_ajenos=('31',))
    [f] = informes(tmp_path, 'PRM Laja', '2026-09-20', historias={0: historia}, evidencias={0: ajeno})
    assert 'folio 31' not in f.observation and f.actions == ['programa_vencido', 'PC_INFO']


def test_e3_10_evidencia_explicita_de_historia_prevalece_sobre_el_pendiente_del_mismo_informe(tmp_path):
    t = 'Inf. Cump Programa Ambulatorio'
    bandeja = Escritos('COMPLETA', CORTE, (Escrito(t, 'Pendiente', vinculo='EXPLICITO', fecha_proyectada=date(2026, 9, 20)),))
    historia = hist((t, '2026-09-22', 30), ('Ingreso informe de cumplimiento X', '2026-09-25', 31))
    ev = EvidenciaInforme('30', '31', date(2026, 9, 25), '', date(2026, 9, 20))
    [f] = informes(tmp_path, 'PRM Laja', '2026-09-20', historias={0: historia}, escritos={0: bandeja}, evidencias={0: ev})
    assert 'registra ingreso a folio 31' in f.observation and PENDIENTE not in f.observation and f.actions == []
    assert 'ESCRITO_ESTADOS_INCOMPATIBLES' in [a[1] for a in f.avisos]


def test_e3_11_frase_desactivada_no_reactiva_el_pide_cuenta(tmp_path):
    from nurus.personal.config import defaults as base

    t = 'Inf. Cump Programa Ambulatorio'
    config = base()
    config['desactivadas'] = ['HISTORIA.INFORME_AVANCE_PENDIENTE']
    libro = Workbook()
    hoja = libro.active
    hoja.title = 'INFORMES'
    hoja.append(INFORMES)
    hoja.append(fila(1, 'PRM Laja') + ['2026-09-20'])
    ruta = tmp_path / 'des.xlsx'
    libro.save(ruta)
    rid = Work(config).analyze(ruta, 'INFORMES', as_of=CORTE).rows[0].id
    f = Work(config).analyze(ruta, 'INFORMES', as_of=CORTE, escritos_by_record={rid: escritos((t, 'Pendiente'))}).rows[0]
    assert f.actions == [] and PENDIENTE not in f.observation


def test_e3_12_constancia_exige_que_el_sujeto_sea_el_nino():
    def previa(texto):
        return constancia_en_bitacora(bita(('2026-10-01', texto)), 'Ana', 'PRM Luna') is not None

    assert not previa('Ana informa que Beto cuenta con ingreso efectivo al programa PRM Luna.')
    assert previa('Beto informa que Ana cuenta con ingreso efectivo al programa PRM Luna.')
    assert previa('Se hace presente que Ana cuenta con ingreso efectivo al programa PRM Luna.')


def test_e3_14_guion_unicode_menos_vale_como_sin_fecha():
    h = historico(('FAE Sol', '−−', 'SIN INFORMACION'))
    assert programas_vigentes_otros(h, 'PRM Laja') == ['FAE Sol']
