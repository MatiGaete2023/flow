"""Oráculo del agregado Word por tribunal (E-P1). Datos ficticios, sin red."""

import json
import sys
import tempfile
import zipfile
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

sys.stdout.reconfigure(encoding='utf-8')
RAIZ = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(RAIZ / 'src'))

from docx import Document  # noqa: E402
from nurus.personal import resolutions  # noqa: E402

MATRICES = RAIZ / 'src/nurus/personal/plantillas_word'
FECHA = (2026, 11, 25, 0, 0, 0)


def casos():
    return [
        {'nombre': 'un_proyecto', 'proyectos': [p('LAJA', 'PC_IE', 'P-1-2026')]},
        {'nombre': 'tres_proyectos_mismo_tribunal', 'proyectos': [p('LAJA', k, f'P-{i}-2026') for i, k in enumerate(('PC_IE', 'PC_INFO', 'NOMENCL'), 1)]},
        {'nombre': 'dos_tribunales', 'proyectos': [p('LAJA', 'PC_IE', 'P-4-2026'), p('MULCHEN', 'PC_INFO', 'P-5-2026')]},
        {'nombre': 'texto_editado_en_revision', 'proyectos': [p('MULCHEN', 'PC_IE', 'P-6-2026', editado=True)]},
        {'nombre': 'matriz_propia_usuario', 'proyectos': [p('TOME', 'PC_INFO', 'P-7-2026', propia=True)]},
        {'nombre': 'matriz_con_cursivas', 'proyectos': [p('MULCHEN', 'NOMENCL', 'P-8-2026', cursiva=True)]},
    ]


def p(tribunal, tipo, rit, editado=False, propia=False, cursiva=False):
    return {'tribunal': tribunal, 'tipo': tipo, 'rit': rit, 'editado': editado, 'propia': propia, 'cursiva': cursiva}


def plantillas(proyecto, temporal):
    familia = 'MULCHEN' if proyecto['tribunal'] == 'TOME' else proyecto['tribunal']
    ruta = MATRICES / familia / f"{proyecto['tipo']}.docx"
    doc = Document(ruta)
    if proyecto['cursiva']:
        doc.paragraphs[0].runs[0].italic = True
    salida = Path(temporal) / f"{proyecto['tribunal']}_{proyecto['tipo']}_{proyecto['rit']}.docx"
    doc.save(salida)
    return salida


def render_falso(project, target):
    doc = Document(project.template)
    lines = project.text.split('\n')
    for i, paragraph in enumerate(doc.paragraphs):
        if i < len(lines):
            # Conserva las matrices, estilos de párrafo y formato del primer run.
            runs = paragraph.runs
            if runs:
                runs[0].text = lines[i]
                for run in runs[1:]:
                    run.text = ''
            else:
                paragraph.add_run(lines[i])
    for line in lines[len(doc.paragraphs):]:
        doc.add_paragraph(line)
    doc.save(target)
    return doc


def normalizar_zip(ruta):
    temporal = ruta.with_suffix('.normalizado.docx')
    with zipfile.ZipFile(ruta, 'r') as entrada, zipfile.ZipFile(temporal, 'w', zipfile.ZIP_DEFLATED) as salida:
        for nombre in entrada.namelist():
            info = zipfile.ZipInfo(nombre, FECHA)
            info.create_system = 3
            info.compress_type = zipfile.ZIP_DEFLATED
            salida.writestr(info, entrada.read(nombre))
    temporal.replace(ruta)


def extraer(ruta):
    doc = Document(ruta)
    parrafos = []
    salto_pendiente = False
    for p in doc.element.body:
        tag = p.tag.rsplit('}', 1)[-1]
        texto = ''.join(t.text or '' for t in p.iter() if t.tag.rsplit('}', 1)[-1] == 't')
        salto = any(n.tag.rsplit('}', 1)[-1] == 'br' and n.get('{http://schemas.openxmlformats.org/wordprocessingml/2006/main}type') == 'page' for n in p.iter())
        if texto.strip():
            parrafos.append({'texto': texto, 'salto_previo': salto_pendiente})
            salto_pendiente = False
        elif salto:
            salto_pendiente = True
    cursivas = any(n.tag.rsplit('}', 1)[-1] in ('i', 'iCs') and n.get('{http://schemas.openxmlformats.org/wordprocessingml/2006/main}val') not in ('0', 'false', 'off') for n in doc.element.body.iter())
    return parrafos, cursivas


def generar(caso):
    with tempfile.TemporaryDirectory() as temporal:
        projects = []
        for i, spec in enumerate(caso['proyectos']):
            template = plantillas(spec, temporal)
            doc = Document(template)
            text = '\n'.join(p.text for p in doc.paragraphs)
            variables = {
                'RIT': spec['rit'], 'NOMBRE': 'Persona Ficticia Uno', 'RUT': 'ID-FICTICIO-1',
                'PROGRAMA': 'PRM revisado ficticio' if spec['editado'] else 'PRM Ficticio',
                'FECHA': 'veinticinco de noviembre de dos mil veintiséis',
                'FECHA_RESOLUCION': 'catorce de septiembre de dos mil veintiséis',
                'DURACION': '12 meses', 'NOMENCLATURA': 'Nomenclatura ficticia',
            }
            for key, value in variables.items():
                text = text.replace('{{' + key + '}}', value)
            projects.append(resolutions.Project(key=f"caso{i}", court=spec['tribunal'], rit=spec['rit'], kind=spec['tipo'], record_ids=[i], template=str(template), values={}, text=text, original_text=text))
        work = SimpleNamespace(receipts={})
        por_tribunal = {}
        for tribunal in dict.fromkeys(x['tribunal'] for x in caso['proyectos']):
            # Un archivo por tribunal: el generador de referencia recibe solo sus proyectos.
            subset = [x for x in projects if x.court == tribunal]
            tmp = Path(temporal) / f'{tribunal}.docx'
            with patch.object(resolutions, '_validate_project_template', lambda project: None), \
                 patch.object(resolutions, 'render_project', render_falso), \
                 patch('nurus.personal.product_state.require_fresh', lambda work, project: None), \
                 patch('nurus.personal.word_quality.require_complete', lambda *a, **k: None), \
                 patch('nurus.personal.word_quality.improve_pagination', lambda doc: None):
                resolutions.generate_projects(work, subset, tmp)
            normalizar_zip(tmp)
            parrafos, cursivas = extraer(tmp)
            por_tribunal[tribunal] = {'parrafos': parrafos, 'quedan_cursivas': cursivas, 'salto_tras_indice': True,
                                      'indice': ['Índice'] + [f"{i}. {x.rit} — {resolutions.kind_label(x.kind)}" for i, x in enumerate(subset, 1)]}
        return {'nombre': caso['nombre'], 'proyectos': caso['proyectos'], 'archivos': por_tribunal}


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app/tests/datos/word_tribunal.json'
    datos = {'oraculo': 'word_tribunal', 'fecha': '2026-11-25', 'casos': [generar(c) for c in casos()]}
    destino.parent.mkdir(parents=True, exist_ok=True)
    destino.write_bytes((json.dumps(datos, ensure_ascii=False, indent=2) + '\n').encode('utf-8'))
    print(f"WORD_TRIBUNAL: {len(datos['casos'])} casos, {sum(len(c['archivos']) for c in datos['casos'])} archivos")


if __name__ == '__main__':
    main()
