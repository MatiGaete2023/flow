"""Control de correos y RIT (tools/check_contactos.py) con datos ficticios.

Los valores se arman por partes para que este archivo no contenga, él mismo, un correo o un RIT detectable.
"""

import subprocess
import sys
from pathlib import Path

TOOLS = Path(__file__).resolve().parents[1] / 'tools'
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

import check_contactos as contactos

AT = '@'
INSTITUCIONAL = 'ucc_concepcion' + AT + 'pjud.cl'


def revisar(tmp_path, monkeypatch, texto, permitidos=()):
    monkeypatch.setattr(contactos, 'ALLOWLIST', tmp_path / 'permitidos.txt')
    (tmp_path / 'permitidos.txt').write_text('\n'.join(permitidos) + '\n', encoding='utf-8')
    muestra = tmp_path / 'muestra.txt'
    muestra.write_text(texto, encoding='utf-8')
    return contactos.main([str(muestra)])


def test_correo_de_dominio_real_falla_sin_mostrar_el_valor(tmp_path, monkeypatch, capsys):
    assert revisar(tmp_path, monkeypatch, 'Contacto: persona.prueba' + AT + 'correo.cl\n') == 1
    salida = capsys.readouterr().out
    assert 'p***' + AT + 'correo.cl' in salida
    assert 'persona.prueba' not in salida


def test_dominios_reservados_e_institucional_pasan(tmp_path, monkeypatch):
    texto = ' '.join(
        [
            'uno' + AT + 'example.test',
            'dos' + AT + 'programa.example',
            'tres' + AT + 'example.com',
            'cuatro' + AT + 'ejemplo.cl',
            'cinco' + AT + 'buzon.invalid',
            INSTITUCIONAL.upper(),
        ]
    )
    assert revisar(tmp_path, monkeypatch, texto, [INSTITUCIONAL]) == 0


def test_institucional_sin_lista_falla(tmp_path, monkeypatch):
    assert revisar(tmp_path, monkeypatch, INSTITUCIONAL) == 1


def test_identificadores_pegados_no_crean_una_direccion_nueva(tmp_path, monkeypatch):
    # ID de prueba parametrizada: «dirección-DIRECCIÓN» sin espacio entre ambas.
    texto = 'test_cc[' + INSTITUCIONAL + '-' + INSTITUCIONAL.upper() + ']'
    assert revisar(tmp_path, monkeypatch, texto, [INSTITUCIONAL]) == 0


def test_rit_falla_enmascarado_y_el_anio_2099_pasa(tmp_path, monkeypatch, capsys):
    assert revisar(tmp_path, monkeypatch, 'Causa ' + '-'.join(['C', '123', '2025'])) == 1
    salida = capsys.readouterr().out
    assert 'C-***-2025' in salida and '123' not in salida
    assert revisar(tmp_path, monkeypatch, 'Causa ' + '-'.join(['C', '123', '2099'])) == 0


def test_huella_registrada_se_acepta_con_ceros_a_la_izquierda(tmp_path, monkeypatch):
    huella = contactos.fingerprint('rit:C-123-2025')
    assert revisar(tmp_path, monkeypatch, '-'.join(['C', '0123', '2025']), [huella]) == 0


def test_stage_revisa_el_indice_y_no_la_copia_de_trabajo(tmp_path, monkeypatch):
    subprocess.run(['git', 'init', '-q', str(tmp_path)], check=True)
    monkeypatch.setattr(contactos, 'ROOT', tmp_path)
    monkeypatch.setattr(contactos.ids, 'ROOT', tmp_path)
    monkeypatch.setattr(contactos, 'ALLOWLIST', tmp_path / 'tools' / 'contactos_permitidos.txt')
    nota = tmp_path / 'nota.txt'
    nota.write_text('persona' + AT + 'correo.cl\n', encoding='utf-8')
    subprocess.run(['git', 'add', 'nota.txt'], cwd=tmp_path, check=True)
    nota.write_text('sin datos\n', encoding='utf-8')
    assert contactos.main(['--stage']) == 1
