"""WP-R4: funciones puras de las reglas de historia (sin motor ni textos)."""

from datetime import date

from historia_fixtures import bita, hist

from nurus.personal.motor.analisis_historia import (
    busqueda,
    duplicidades,
    instituciones,
    materias,
    posterior,
    primera_observacion,
)

ORDEN, TERMINO, SIN_EFECTO = 'Orden de búsqueda', 'Termino de búsqueda', 'Deja sin efecto búsqueda'


def test_posterior_compara_por_folio_y_si_no_por_fecha():
    h = hist(('A', '2026-10-01', 5), ('A', '2026-09-01', 9), ('B', '2026-10-02', 'S/N'), ('C', '2026-10-03', 'S/N'))
    por = {t.folio_txt or t.desc: t for t in h.tramites}
    a5, a9, b, c = por['5'], por['9'], h.buscar('B')[0], h.buscar('C')[0]
    assert posterior(a9, a5) is True and posterior(a5, a9) is False
    assert posterior(c, b) is True and posterior(b, c) is False
    assert posterior(b, b) is False


def test_busqueda_empareja_el_cierre_con_la_orden_anterior_mas_cercana():
    h = hist((ORDEN, '2026-08-01', 10), (ORDEN, '2026-09-01', 20), (TERMINO, '2026-09-15', 30))
    pares, vigentes = busqueda(h)
    assert [(c.folio, o.folio) for c, o in pares] == [(30, 20)]
    assert [o.folio for o in vigentes] == []  # el término posterior cierra ambas por folio


def test_busqueda_orden_posterior_al_cierre_queda_vigente_y_cierre_huerfano_no_tiene_orden():
    h = hist((TERMINO, '2026-09-15', 30), (ORDEN, '2026-10-01', 40))
    pares, vigentes = busqueda(h)
    assert [(c.folio, o) for c, o in pares] == [(30, None)] and [o.folio for o in vigentes] == [40]
    assert busqueda(hist(('Otro', '2026-10-01', 1))) == ([], [])


def test_busqueda_deja_sin_efecto_cierra_igual_que_termino():
    pares, vigentes = busqueda(hist((ORDEN, '2026-09-01', 10), (SIN_EFECTO, '2026-09-02', 11)))
    assert len(pares) == 1 and vigentes == []


def test_instituciones_cuenta_categorias_distintas_sin_tildes_ni_mayusculas():
    celda = '(1-9)A - Institución: Programa Mi Abogado#(2-7)B - INSTITUCION: programa mi abogado#(3-5)C - Institución: Otra institución pública'
    assert len(instituciones(celda)) == 2
    assert instituciones('') == set() and instituciones(None) == set() and instituciones('(1-9)A sin rótulo') == set()


def test_materias_de_reitera_y_no_se_reitera_lo_reiterado():
    assert materias('Persona: No registra curador ad litem en RUS. Se sugiere asociarlo informáticamente.') == {
        'CURADOR'
    }
    assert materias('Medida vencida en RUS desde el 1 de septiembre de 2026.') == {'VENCIDA'}
    assert materias('No registra ficha individual en RUS. Se sugiere confeccionarla.') == {'SIN_FICHA'}
    assert materias('Pedro no registra ficha FAE en RUS. Se sugiere confeccionarla.') == {'SIN_FAE'}
    assert (
        materias(
            'Se reitera observación de fecha 1 de septiembre de 2026, respecto de la ausencia de curador ad litem en RUS.'
        )
        == set()
    )
    assert materias('Oído el 3 de octubre de 2026.') == set()


def test_primera_observacion_es_la_mas_antigua_de_la_materia_antes_del_corte():
    b = bita(
        ('2026-08-01', 'Otra cosa.'),
        ('2026-09-01', 'No registra curador ad litem en RUS. Se sugiere asociarlo informáticamente.'),
        ('2026-09-20', 'No registra curador ad litem en RUS. Se sugiere asociarlo informáticamente.'),
    )
    assert primera_observacion(b, 'CURADOR', date(2026, 10, 14)).fecha == date(2026, 9, 1)
    assert primera_observacion(b, 'CURADOR', date(2026, 9, 1)) is None
    assert primera_observacion(b, 'VENCIDA', date(2026, 10, 14)) is None


def test_duplicidades_por_nna_hermanos_y_causas_sin_repeticion():
    registros = [
        ('a', 'LAJA', 'C-1-2026', '11.111.111-1', 'PRM UNO'),
        ('b', 'laja', 'c-1-2026', '11111111-1', 'prm uno'),
        ('c', 'LAJA', 'C-2-2026', '11111111-1', 'PRM UNO'),
        ('d', 'LAJA', 'C-2-2026', '11111111-1', 'PRM DOS'),
        ('e', 'LAJA', 'C-3-2026', '11111111-1', 'PRM UNO'),
        ('f', 'LAJA', '', '11111111-1', 'PRM UNO'),
        ('g', 'LAJA', '', '11111111-1', 'PRM UNO'),
        ('h', 'LAJA', 'C-4-2026', '11111111-1', 'PRM UNO'),  # hermanos: misma causa y programa, otro RUT
        ('i', 'LAJA', 'C-4-2026', '22222222-2', 'PRM UNO'),
        ('j', 'LAJA', 'C-5-2026', '', 'PRM UNO'),  # sin RUT no se afirma duplicidad
        ('k', 'LAJA', 'C-5-2026', '', 'PRM UNO'),
    ]
    assert duplicidades(registros) == {
        'a': 'MISMO_PROGRAMA',
        'b': 'MISMO_PROGRAMA',
        'c': 'OTRO_REGISTRO',
        'd': 'OTRO_REGISTRO',
        'h': 'OTRO_REGISTRO',
        'i': 'OTRO_REGISTRO',
        'j': 'OTRO_REGISTRO',
        'k': 'OTRO_REGISTRO',
    }


def test_busqueda_empareja_con_la_orden_mas_cercana_aunque_tenga_folio_no_numerico():
    h = hist(
        ('Orden de búsqueda', '2026-07-01', 10),
        ('Orden de búsqueda', '2026-08-01', 's/f'),
        ('Termino de búsqueda', '2026-08-10', 30),
    )
    pares, vigentes = busqueda(h)
    assert pares[0][1].folio_txt == 's/f' and vigentes == []
