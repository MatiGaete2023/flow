"""E-A5 · Oráculo de la preparación de correos de Flow Python (`outputs.prepare_drafts`): qué borradores salen, a quién,
con qué asunto, texto, CC, filas y adjunto obligatorio, para tres trabajos sintéticos (Espera, Cumplimiento e Informes) y
varios alcances (todas las modalidades, una modalidad, solo filas elegidas y un tribunal). Cubre la agrupación, la firma
y el alcance de los correos (F12).

Uso (desde la raíz): python app/qa/generar_correos.py [salida.json]   (por omisión app/tests/datos/correos.json)
"""

import json
import sys
import tempfile
from datetime import date
from pathlib import Path
from unittest.mock import MagicMock

sys.stdout.reconfigure(encoding='utf-8')
RAIZ = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RAIZ / 'src')]
for _m in ('tkinter', 'tkinter.ttk', 'tkinter.filedialog', 'tkinter.messagebox', 'tkinter.simpledialog'):
    sys.modules.setdefault(_m, MagicMock())

from openpyxl import Workbook  # noqa: E402
from nurus.personal.config import defaults  # noqa: E402
from nurus.personal.mail_controls import alcance_modalidades, selected_court_record_ids  # noqa: E402
from nurus.personal.outputs import prepare_drafts, value  # noqa: E402
from nurus.personal.work import Work  # noqa: E402

CORTE = date(2026, 11, 20)
PERIODO = 'el período revisado'
CONTACTOS = {'AFT EJEMPLO UNO': 'uno@ejemplo.invalid; uno2@ejemplo.invalid', 'FAE EJEMPLO': 'fae@ejemplo.invalid'}
TRIBUNALES = {'LAJA': 'laja@ejemplo.invalid', 'MULCHEN': 'mulchen@ejemplo.invalid; mulchen2@ejemplo.invalid'}
FIRMA = 'Firma ficticia\nUnidad de prueba'
CC_ADICIONAL = 'adicional@ejemplo.invalid; ucc_concepcion@pjud.cl'

TRABAJOS = {
    'ESPERA': (
        ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'T ESPERA'],
        [
            ['X-1-2026', 'Jgdo. L. y G. de Laja', 'Persona Uno', '11111111-1', 'AFT EJEMPLO UNO', 90],
            ['X-2-2026', 'Jgdo. L. y G. de Laja', 'Persona Dos', '22222222-2', 'AFT EJEMPLO UNO', 95],
            ['X-3-2026', 'Jgdo. L. y G. de Laja', 'Persona Tres', '33333333-3', 'AFT EJEMPLO DOS', 100],
            ['X-4-2026', 'Jgdo. L. y G. de Mulchen', 'Persona Cuatro', '44444444-4', 'AFT EJEMPLO UNO', 120],
            ['X-5-2026', 'Juzgado de Familia Tomé', 'Persona Cinco', '55555555-5', 'FAE EJEMPLO', 80],
            ['X-6-2026', 'Jgdo. L. y G. de Laja', 'Persona Seis', '66666666-6', 'AFT EJEMPLO UNO', 5],
        ],
        ['programa_espera', 'espera'],
    ),
    'CUMPLIMIENTO': (
        ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'DIAS DE CUMPLIMIENTO', 'DIAS PARA EGRESAR', 'FEC.EGRESO PROYECTADO'],
        [
            ['X-1-2026', 'Jgdo. L. y G. de Laja', 'Persona Uno', '11111111-1', 'AFT EJEMPLO UNO', 100, -5, '15/11/2026'],
            ['X-2-2026', 'Jgdo. L. y G. de Laja', 'Persona Dos', '22222222-2', 'AFT EJEMPLO UNO', 200, 10, '30/11/2026'],
            ['X-3-2026', 'Jgdo. L. y G. de Mulchen', 'Persona Tres', '33333333-3', 'AFT EJEMPLO DOS', 300, 25, '15/12/2026'],
            ['X-4-2026', 'Juzgado de Familia Tomé', 'Persona Cuatro', '44444444-4', 'FAE EJEMPLO', 50, 80, '15/02/2027'],
        ],
        ['medidas', 'cumplimiento', 'programa_vencido', 'programa_por_vencer'],
    ),
    'INFORMES': (
        ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'FECHA VENCIMIENTO'],
        [
            ['X-1-2026', 'Jgdo. L. y G. de Laja', 'Persona Uno', '11111111-1', 'AFT EJEMPLO UNO', '15/11/2026'],
            ['X-2-2026', 'Jgdo. L. y G. de Laja', 'Persona Dos', '22222222-2', 'AFT EJEMPLO UNO', '25/11/2026'],
            ['X-3-2026', 'Jgdo. L. y G. de Mulchen', 'Persona Tres', '33333333-3', 'AFT EJEMPLO DOS', '30/12/2026'],
            ['X-4-2026', 'Juzgado de Familia Tomé', 'Persona Cuatro', '44444444-4', 'FAE EJEMPLO', '18/11/2026'],
        ],
        ['informes', 'programa_vencido', 'programa_por_vencer'],
    ),
}


def configuracion(con_firma=True, cc_adicional=True):
    cfg = defaults()
    cfg['contactos'] = dict(CONTACTOS)
    for clave, para in TRIBUNALES.items():
        cfg['correos']['tribunales'][clave]['para'] = [p.strip() for p in para.split(';')]
    cfg['firma'] = FIRMA if con_firma else ''
    cfg['correos']['cc_adicional'] = CC_ADICIONAL if cc_adicional else ''
    return cfg


def trabajo(modo, cfg, carpeta):
    encabezados, filas, _ = TRABAJOS[modo]
    libro = Workbook()
    hoja = libro.active
    hoja.title = 'informe_1'
    hoja.append(['Reporte'])
    hoja.append([])
    hoja.append(encabezados)
    for f in filas:
        hoja.append(f)
    ruta = carpeta / 'o.xlsx'
    libro.save(ruta)
    return Work(cfg).analyze(ruta, modo, as_of=CORTE)


def escenario(nombre, modo, claves=None, rits=None, tribunales=None, firma=True, cc_adicional=True):
    """`claves`: modalidades (None = todas); `rits`: filas elegidas; `tribunales`: claves de tribunal elegidas."""
    cfg = configuracion(firma, cc_adicional)
    caso = {'nombre': nombre, 'modo': modo, 'modalidades': claves, 'rits': rits, 'tribunales': tribunales, 'firma': firma,
            'cc_adicional': cc_adicional, 'borradores': []}
    with tempfile.TemporaryDirectory() as t:
        obra = trabajo(modo, cfg, Path(t))
        elegidas = None
        if rits is not None:
            elegidas = {r.id for r in obra.rows if value(obra, r, 'rit') in rits}
        if tribunales is not None:
            por_tribunal = set(selected_court_record_ids(obra, tribunales))
            elegidas = por_tribunal if elegidas is None else elegidas & por_tribunal
        por_id = {r.id: value(obra, r, 'rit') for r in obra.rows}
        caso['acciones'] = {value(obra, r, 'rit'): list(r.actions) for r in obra.rows}
        for tipo in TRABAJOS[modo][2]:
            borradores = prepare_drafts(
                obra, tipo, modalities=alcance_modalidades(claves or ['RES', 'AMB', 'FAE', 'DCE']), period=PERIODO, selected=elegidas,
                directory=Path(t), modality_keys=claves, defer_attachments=True,
            )
            for d in borradores:
                caso['borradores'].append({
                    'tipo': tipo, 'asunto': d.subject, 'para': d.to, 'cc': d.cc, 'cuerpo': d.body,
                    'rits': sorted(por_id[i] for i in d.record_ids), 'obligatorio': bool(d.required),
                    'destino': d.recipient_type,
                })
    caso['borradores'].sort(key=lambda b: (b['tipo'], b['asunto']))
    return caso


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'correos.json'
    casos = []
    for modo in TRABAJOS:
        casos.append(escenario(f'{modo.lower()}_todo', modo))
        casos.append(escenario(f'{modo.lower()}_una_modalidad', modo, claves=['AMB']))
        casos.append(escenario(f'{modo.lower()}_dos_modalidades', modo, claves=['AMB', 'FAE']))
        casos.append(escenario(f'{modo.lower()}_solo_elegidas', modo, rits=['X-1-2026', 'X-3-2026']))
        casos.append(escenario(f'{modo.lower()}_un_tribunal', modo, tribunales=['LAJA']))
        casos.append(escenario(f'{modo.lower()}_sin_firma_ni_cc_adicional', modo, firma=False, cc_adicional=False))
    datos = {'corte': CORTE.isoformat(), 'periodo': PERIODO, 'contactos': CONTACTOS, 'tribunales': TRIBUNALES, 'firma': FIRMA,
             'cc_adicional': CC_ADICIONAL,
             'trabajos': {m: {'encabezados': e, 'filas': f} for m, (e, f, _) in TRABAJOS.items()}, 'casos': casos}
    destino.write_bytes((json.dumps(datos, ensure_ascii=False, indent=1) + '\n').encode('utf-8'))
    for c in casos:
        print(c['nombre'], len(c['borradores']), sorted({b['tipo'] for b in c['borradores']}))
    print(f'{len(casos)} casos en {destino}')


if __name__ == '__main__':
    main()
