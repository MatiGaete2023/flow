"""El proyecto NOMENCL toma la nomenclatura de RES + observación (o de la regla) y el Word no sale en cursiva.

Pedido del usuario (16-nov-2026): «que el asistente lea la columna RES y la observación a fin de que sepa qué
nomenclatura se tiene que usar, ya sea egreso o ingreso … y que al crear el Word el texto no quede en cursiva».
Datos ficticios.
"""

from docx import Document
from docx.oxml.ns import qn
from openpyxl import Workbook

from nurus.personal.config import BASE, defaults
from nurus.personal.resolutions import (
    EGRESO_SERVICIO,
    INGRESO_INFORME_X,
    automatic_project_selections,
    generate_projects,
    prepare_projects,
    project_nomenclature,
)
from nurus.personal.work import Work

TEMPLATES = BASE / 'plantillas_word'


def libro(tmp_path, filas, nombre='res.xlsx'):
    book = Workbook()
    sheet = book.active
    sheet.title = 'Registros'
    sheet.append(['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'OBSERVACION', 'RES'])
    for fila in filas:
        sheet.append(fila)
    path = tmp_path / nombre
    book.save(path)
    return Work.external(path, defaults())


def proyectos(work):
    selected = automatic_project_selections(work)
    projects, errors = prepare_projects(work, selected, TEMPLATES)
    assert not errors
    return projects


def test_res_nomencl_con_observacion_de_egreso_usa_egreso(tmp_path):
    work = libro(
        tmp_path,
        [
            [
                'X-1-2099',
                'LAJA',
                'Persona Uno',
                '11111111-1',
                'AFT EJEMPLO',
                'Causa finalizada, aplicar egreso.',
                'NOMENCL',
            ]
        ],
    )
    (project,) = proyectos(work)
    assert project.kind == 'NOMENCL'
    assert project.values['NOMENCLATURA'] == EGRESO_SERVICIO
    assert '"' + EGRESO_SERVICIO + '"' in project.text
    assert 'COMPLETAR NOMENCLATURA' not in project.text


def test_res_nomencl_con_observacion_de_informe_usa_ingreso(tmp_path):
    work = libro(
        tmp_path,
        [
            [
                'X-2-2099',
                'MULCHEN',
                'Persona Dos',
                '22222222-2',
                'AFT EJEMPLO',
                'Informe de avance a folio 12.',
                'NOMENCL',
            ]
        ],
    )
    (project,) = proyectos(work)
    assert project.values['NOMENCLATURA'] == INGRESO_INFORME_X


def test_observacion_con_nomenclatura_entre_comillas_manda(tmp_path):
    texto = 'Se remite proyecto que aplica nomenclatura «Ingreso informe de diagnóstico» para regularizar.'
    work = libro(tmp_path, [['X-3-2099', 'LAJA', 'Persona Tres', '33333333-3', 'AFT EJEMPLO', texto, 'NOMENCL']])
    (project,) = proyectos(work)
    assert project.values['NOMENCLATURA'] == 'Ingreso informe de diagnóstico'


def test_la_observacion_de_nomenclatura_elige_el_tipo_y_la_nomenclatura(tmp_path):
    # RES marcado sin tipo: la observación define NOMENCL y cuál nomenclatura.
    texto = 'Se remite proyecto de nomenclatura «Egreso de servicio protección especializada y otras redes».'
    work = libro(tmp_path, [['X-4-2099', 'LAJA', 'Persona Cuatro', '44444444-4', 'AFT EJEMPLO', texto, 'X']])
    (project,) = proyectos(work)
    assert project.kind == 'NOMENCL' and project.values['NOMENCLATURA'] == EGRESO_SERVICIO


def test_sin_observacion_util_la_regla_del_motor_decide(tmp_path):
    work = libro(tmp_path, [['X-5-2099', 'LAJA', 'Persona Cinco', '55555555-5', 'AFT EJEMPLO', 'Texto.', 'NOMENCL']])
    row = work.rows[0]
    row.rules = ['HISTORIA.C10_ESCRITO']
    assert project_nomenclature([row]) == INGRESO_INFORME_X
    row.rules = ['HISTORIA.P_FINALIZADA']
    assert project_nomenclature([row]) == EGRESO_SERVICIO
    # La observación revisada por el usuario prevalece sobre la regla.
    row.review['OBSERVACION'] = 'Se recepciona informe de cumplimiento.'
    assert project_nomenclature([row]) == INGRESO_INFORME_X


def test_observacion_ambigua_o_ajena_no_inventa_nomenclatura(tmp_path):
    work = libro(
        tmp_path,
        [
            [
                'X-6-2099',
                'LAJA',
                'Persona Seis',
                '66666666-6',
                'AFT EJEMPLO',
                'Informe de avance; evaluar egreso.',
                'NOMENCL',
            ],
            ['X-7-2099', 'LAJA', 'Persona Siete', '77777777-7', 'AFT EJEMPLO', 'Texto.', 'NOMENCL'],
        ],
    )
    for project in proyectos(work):
        assert project.values['NOMENCLATURA'] == '[COMPLETAR NOMENCLATURA]'


def test_observacion_ambigua_no_cae_a_la_regla(tmp_path):
    # Revisión Codex (PR #58): con regla C10 y observación que nombra egreso e informe, no se elige por la regla.
    work = libro(
        tmp_path,
        [
            [
                'X-13-2099',
                'LAJA',
                'Persona Trece',
                '14141414-1',
                'AFT EJEMPLO',
                'Informe de avance; evaluar egreso.',
                'NOMENCL',
            ]
        ],
    )
    row = work.rows[0]
    row.rules = ['HISTORIA.C10_ESCRITO']
    assert project_nomenclature([row]) == ''


def test_nomenclatura_editada_por_el_usuario_se_respeta(tmp_path):
    work = libro(tmp_path, [['X-8-2099', 'LAJA', 'Persona Ocho', '88888888-8', 'AFT EJEMPLO', 'Egreso.', 'NOMENCL']])
    work.rows[0].word_overrides['NOMENCLATURA'] = 'Nomenclatura escrita a mano'
    (project,) = proyectos(work)
    assert project.values['NOMENCLATURA'] == 'Nomenclatura escrita a mano'


def _cursivas(doc):
    found = []
    for run in doc.element.body.iter(qn('w:r')):
        text = ''.join(t.text or '' for t in run.iter(qn('w:t')))
        rpr = run.find(qn('w:rPr'))
        marks = [] if rpr is None else rpr.findall(qn('w:i')) + rpr.findall(qn('w:iCs'))
        if text.strip() and (not marks or any(m.get(qn('w:val')) not in ('0', 'false') for m in marks)):
            found.append(text)
    return found


def test_word_generado_sin_cursiva_en_ninguna_matriz(tmp_path):
    filas = [
        ['X-9-2099', 'LAJA', 'Persona Nueve', '99999999-9', 'AFT EJEMPLO', 'Aplicar egreso.', 'NOMENCL'],
        ['X-10-2099', 'MULCHEN', 'Persona Diez', '10101010-1', 'AFT EJEMPLO', 'Informe de avance.', 'NOMENCL'],
        ['X-11-2099', 'LAJA', 'Persona Once', '12121212-1', 'AFT EJEMPLO', 'Texto.', 'PC_IE'],
        ['X-12-2099', 'MULCHEN', 'Persona Doce', '13131313-1', 'AFT EJEMPLO', 'Texto.', 'PC_INFO'],
    ]
    work = libro(tmp_path, filas)
    projects = proyectos(work)
    assert {p.kind for p in projects} == {'NOMENCL', 'PC_IE', 'PC_INFO'}
    projects[0].text += '\nPárrafo incorporado por el usuario.'
    out = tmp_path / 'Resoluciones.docx'
    generate_projects(work, projects, out)
    doc = Document(out)
    texto = '\n'.join(p.text for p in doc.paragraphs)
    assert EGRESO_SERVICIO in texto and INGRESO_INFORME_X in texto
    assert 'Párrafo incorporado por el usuario.' in texto
    assert _cursivas(doc) == []
    assert all(not run.font.italic for p in doc.paragraphs for run in p.runs)
