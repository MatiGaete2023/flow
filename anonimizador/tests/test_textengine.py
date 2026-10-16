from datetime import date

from anonimizador_csmp.textengine import (
    IDENTITY_OMITTED,
    OMITTED,
    Options,
    Pseudonyms,
    Session,
    check_digit,
    fold,
    scrub_unknown,
    shift_dates,
)
from conftest import valid_rut


def test_fechas_en_texto_no_se_confunden_con_causas(session):
    text = session.replacer.text("Audiencia el 05-10-2026 y 10/2026; vence 2026-11-03", free=True, scrub=True)
    assert text == "Audiencia el 05-10-2026 y 10/2026; vence 2026-11-03"


def test_causa_con_tipo_se_reemplaza_sin_colision(session):
    session.reserve_rits_in("P-1-2026")
    text = session.replacer.text("Ver causa P-1-2026 y también P 432 2026", free=True)
    assert "P-1-2026" not in text and "432" not in text
    assert text.count("-2026") == 2


def test_pseudonimo_no_se_destruye_en_modo_conservador(session):
    session.replacer.add_person("Juan Pérez Soto")
    text = session.free("Se contacta a juan perez soto por la audiencia.")
    assert "Persona Ficticia 1" in text
    assert IDENTITY_OMITTED not in text


def test_apellido_tras_rol_se_oculta_completo(session):
    text = session.free("Madre Marta Rojas llamó; contactar a Pedro Soto Díaz.")
    assert "Rojas" not in text and "Soto" not in text and "Marta" not in text
    assert text.count(IDENTITY_OMITTED) == 2


def test_nombres_en_mayusculas_y_particulas(session):
    text = session.free("Declaró MARTA ROJAS DE LA FUENTE ante el tribunal")
    assert "ROJAS" not in text and "MARTA" not in text


def test_instituciones_se_conservan(session):
    text = session.free("Oficiar al Tribunal de Familia y al Servicio Mejor Niñez")
    assert "Tribunal de Familia" in text and "Mejor Niñez" in text


def test_rut_valido_y_telefono_se_sustituyen(session, rut):
    text = session.free(f"RUT {rut}, fono 987654321, correo ana@example.com")
    assert rut not in text and "987654321" not in text and "ana@example.com" not in text
    assert "@example.invalid" in text
    found, _ = session.leaks([(text, False)])
    assert not found


def test_rut_conserva_formato_y_es_invalido(session):
    text = session.replacer.text(valid_rut(dots=False), free=False)
    number, dv = text.split("-")
    assert number.startswith("9") and check_digit(number) != dv


def test_rutas_con_usuario_y_enlaces(session):
    text = session.free(r"Ver C:\Users\Pedro González\Descargas\x.xls y https://sitio.cl/a?id=1 y \\SERVIDOR\carpeta")
    assert "Pedro" not in text and "González" not in text and "sitio.cl" not in text and "SERVIDOR" not in text
    found, _ = session.leaks([(text, False)])
    assert not found


def test_modo_estricto_omite_texto_libre(strict_session):
    assert strict_session.free("Cualquier texto con Juan Pérez") == OMITTED
    assert strict_session.free("   ") == "   "


def test_desplazar_fechas_conserva_formato():
    assert shift_dates("2026-10-14T10:00:00-03:00", 5) == "2026-10-19T10:00:00-03:00"
    assert shift_dates("31-12-2026 y 1/1/2027", 1) == "01-01-2027 y 2/1/2027"
    assert shift_dates("fecha 31-02-2026", 3) == "fecha 31-02-2026"
    assert shift_dates("sin fechas", 3) == "sin fechas"


def test_semilla_hace_estables_los_seudonimos_entre_sesiones():
    a, b = Pseudonyms("clave"), Pseudonyms("clave")
    a.get("persona", "Ana Díaz"), a.get("persona", "Luis Soto")
    b.get("persona", "Luis Soto")
    assert a.get("persona", "Luis Soto") == b.get("persona", "luis soto")
    assert a.get("rut", "17654321-K") == b.get("rut", "17654321-k")
    assert Pseudonyms("otra").get("persona", "Luis Soto") != a.get("persona", "Luis Soto")


def test_sin_semilla_el_mismo_dato_recibe_el_mismo_reemplazo():
    p = Pseudonyms()
    assert p.get("persona", "Ana Díaz") == p.get("persona", "ANA  DIAZ") == "Persona Ficticia 1"
    assert p.get("persona", "Luis") == "Persona Ficticia 2"


def test_scrub_unknown_no_toca_marcadores():
    assert scrub_unknown("Persona Ficticia 3 asistió") == "Persona Ficticia 3 asistió"


def test_fold_y_opciones():
    assert fold("  Ñandú-Pérez ") == "nandu perez"
    try:
        Options(mode="otro")
    except ValueError:
        pass
    else:
        raise AssertionError("modo inválido admitido")
    assert Session().strict


def test_shift_value_fecha():
    from anonimizador_csmp.textengine import shift_value

    assert shift_value(date(2026, 1, 1), 2) == date(2026, 1, 3)


def test_rit_ficticio_de_cinco_digitos_no_se_confunde_con_telefono(session):
    session.reserve_rits_in("F-12345-2026")
    text = session.replacer.text("Causa F-12345-2026", free=True)
    found, _ = session.leaks([(text, False)])
    assert not found and "12345" not in text


def test_huellas_hexadecimales_no_disparan_falsos_positivos(session):
    # Una huella o un id aleatorio que empieza con nueve dígitos parecía un teléfono (≈1,5 % de las ejecuciones).
    texts = [
        ("123456789" + "a" * 55, True),
        ("anon-" + "123456789012345678901234", True),
        ("anon-123456789abcdef012345678", False),
    ]
    found, _ = session.leaks(texts)
    assert not found
