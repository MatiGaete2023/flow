"""E-A10 · Oráculo de anulaciones de columnas de origen por fila (`record_edits.apply(overrides=...)` y `_prepass_peers`).

Arma libros sintéticos de Espera, Cumplimiento e Informes con Flow Python, aplica anulaciones de origen por fila,
comprueba los pares recalculados en la misma causa (`duplicidad` R6 y `acumulado` J2), deshace (`undo`), y verifica
errores y salidas de `outputs.prepare_drafts`. La prueba Rust repite exactamente los mismos escenarios sobre `Trabajo`
y compara observaciones, reglas, acciones, marcas y deshacer.

Uso (desde la raíz): python app/qa/generar_anulaciones.py [salida.json]
"""

import json
import sys
import tempfile
from datetime import date
from pathlib import Path

sys.stdout.reconfigure(encoding='utf-8')
sys.stderr.reconfigure(encoding='utf-8')
RAIZ = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(RAIZ / 'src')]

from openpyxl import Workbook, load_workbook  # noqa: E402
from nurus.personal.config import defaults  # noqa: E402
import nurus.personal.outputs as outputs  # noqa: E402
from nurus.personal.outputs import prepare_drafts  # noqa: E402
from nurus.personal.record_edits import apply, undo  # noqa: E402
from nurus.personal.work import Work  # noqa: E402

CORTE = date(2026, 9, 13)
HOY = date(2099, 1, 1)


class _FijaMeta(type(date)):
    def __instancecheck__(cls, obj):
        return isinstance(obj, date)


class _Fija(date, metaclass=_FijaMeta):
    @classmethod
    def today(cls):
        return HOY


outputs.date = _Fija


def libro(carpeta, nombre, encabezados, filas):
    wb = Workbook()
    ws = wb.active
    ws.title = 'informe_1'
    ws.append(encabezados)
    for fila in filas:
        ws.append(list(fila))
    ruta = carpeta / f'{nombre}.xlsx'
    wb.save(ruta)
    return ruta


def estado_filas(obra):
    return [
        {
            'observacion': r.observation,
            'reglas': list(r.rules),
            'acciones': list(r.actions),
            'marcas': [a[1] for a in r.avisos],
            'overrides': dict(r.overrides),
        }
        for r in obra.rows
    ]


def cfg_oracle():
    cfg = defaults()
    cfg['contactos']['AFT EJEMPLO UNO'] = 'aft1@ejemplo.invalid'
    cfg['correos']['tribunales']['LAJA']['para'] = ['laja@ejemplo.invalid']
    return cfg


def caso(nombre, modo, encabezados, filas, pasos, con_borradores=False):
    with tempfile.TemporaryDirectory() as t:
        ruta = libro(Path(t), nombre, encabezados, filas)
        cfg = cfg_oracle()
        obra = Work(cfg).analyze(ruta, modo, as_of=CORTE)
        antes = estado_filas(obra)
        ultimo_undo = None
        error = None
        for paso in pasos:
            op = paso['op']
            if op == 'apply':
                try:
                    ultimo_undo = apply(obra, [obra.rows[paso['fila']].id], overrides=dict(paso['overrides']))
                except ValueError as exc:
                    error = str(exc)
            elif op == 'undo' and ultimo_undo is not None:
                undo(obra, ultimo_undo)
                ultimo_undo = None
        salida = {
            'escenario': nombre,
            'modo': modo,
            'encabezados': list(encabezados),
            'filas': [list(f) for f in filas],
            'pasos': pasos,
            'antes': antes,
            'despues': estado_filas(obra),
        }
        if error is not None:
            salida['error'] = error
        if con_borradores:
            carpeta_b = Path(t) / 'borradores'
            carpeta_b.mkdir(parents=True, exist_ok=True)
            borradores = prepare_drafts(obra, 'programa_espera', directory=carpeta_b)
            salida['borradores'] = []
            for d in borradores:
                filas_adj = []
                if d.attachments:
                    wb = load_workbook(d.attachments[0], data_only=True)
                    ws = wb.active
                    filas_adj = [[str(c or '') for c in row] for row in ws.iter_rows(values_only=True)]
                salida['borradores'].append(
                    {
                        'kind': d.kind,
                        'programa': d.program,
                        'filas': filas_adj,
                    }
                )
        return salida


def main():
    destino = Path(sys.argv[1]) if len(sys.argv) > 1 else RAIZ / 'app' / 'tests' / 'datos' / 'anulaciones.json'
    enc_espera = ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'T ESPERA']
    enc_informes = ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'FECHA VENCIMIENTO']
    enc_cumpl = ['RIT', 'TRIBUNAL', 'NOMBRE', 'RUT', 'DERIVACION', 'F. EGRESO PROYECTADO', 'DIAS DE CUMPLIMIENTO', 'DIAS PARA EGRESAR']

    casos = [
        caso(
            'espera_baja_a_alta_y_limpia',
            'ESPERA',
            enc_espera,
            [('X-1-2026', 'Jgdo. L. y G. de Laja', 'Persona Uno', '11111111-1', 'AFT EJEMPLO UNO', 10)],
            [
                {'op': 'apply', 'fila': 0, 'overrides': {'espera': '45'}},
            ],
        ),
        caso(
            'espera_limpia_vuelve_a_origen',
            'ESPERA',
            enc_espera,
            [('X-1-2026', 'Jgdo. L. y G. de Laja', 'Persona Uno', '11111111-1', 'AFT EJEMPLO UNO', 10)],
            [
                {'op': 'apply', 'fila': 0, 'overrides': {'espera': '45'}},
                {'op': 'apply', 'fila': 0, 'overrides': {}},
            ],
        ),
        caso(
            'espera_cero_explicito',
            'ESPERA',
            enc_espera,
            [('X-1-2026', 'Jgdo. L. y G. de Laja', 'Persona Uno', '11111111-1', 'AFT EJEMPLO UNO', 45)],
            [
                {'op': 'apply', 'fila': 0, 'overrides': {'espera': '0'}},
            ],
        ),
        caso(
            'espera_crea_duplicidad_en_par',
            'ESPERA',
            enc_espera,
            [
                ('C-1-2026', 'Jgdo. L. y G. de Laja', 'Persona Uno', '11111111-1', 'AFT EJEMPLO UNO', 45),
                ('C-1-2026', 'Jgdo. L. y G. de Laja', 'Persona Dos', '22222222-2', 'AFT EJEMPLO UNO', 45),
            ],
            [
                {'op': 'apply', 'fila': 1, 'overrides': {'rut': '11111111-1'}},
            ],
        ),
        caso(
            'espera_crea_y_deshace_duplicidad_en_par',
            'ESPERA',
            enc_espera,
            [
                ('C-1-2026', 'Jgdo. L. y G. de Laja', 'Persona Uno', '11111111-1', 'AFT EJEMPLO UNO', 45),
                ('C-1-2026', 'Jgdo. L. y G. de Laja', 'Persona Dos', '22222222-2', 'AFT EJEMPLO UNO', 45),
            ],
            [
                {'op': 'apply', 'fila': 1, 'overrides': {'rut': '11111111-1'}},
                {'op': 'undo'},
            ],
        ),
        caso(
            'espera_rompe_duplicidad_en_par',
            'ESPERA',
            enc_espera,
            [
                ('C-1-2026', 'Jgdo. L. y G. de Laja', 'Persona Uno', '11111111-1', 'AFT EJEMPLO UNO', 45),
                ('C-1-2026', 'Jgdo. L. y G. de Laja', 'Persona Uno', '11111111-1', 'AFT EJEMPLO UNO', 45),
            ],
            [
                {'op': 'apply', 'fila': 1, 'overrides': {'rut': '99999999-9'}},
            ],
        ),
        caso(
            'informes_crea_acumulado_en_par',
            'INFORMES',
            enc_informes,
            [
                ('C-2-2026', 'Jgdo. L. y G. de Laja', 'Persona Uno', '11111111-1', 'AFT EJEMPLO UNO', '2026-08-20'),
                ('C-2-2026', 'Jgdo. L. y G. de Laja', 'Persona Uno', '11111111-1', 'AFT EJEMPLO UNO', '2026-11-20'),
            ],
            [
                {'op': 'apply', 'fila': 1, 'overrides': {'vencimiento': '2026-09-01'}},
            ],
        ),
        caso(
            'informes_crea_y_deshace_acumulado_en_par',
            'INFORMES',
            enc_informes,
            [
                ('C-2-2026', 'Jgdo. L. y G. de Laja', 'Persona Uno', '11111111-1', 'AFT EJEMPLO UNO', '2026-08-20'),
                ('C-2-2026', 'Jgdo. L. y G. de Laja', 'Persona Uno', '11111111-1', 'AFT EJEMPLO UNO', '2026-11-20'),
            ],
            [
                {'op': 'apply', 'fila': 1, 'overrides': {'vencimiento': '2026-09-01'}},
                {'op': 'undo'},
            ],
        ),
        caso(
            'espera_cambia_programa_a_dce',
            'ESPERA',
            enc_espera,
            [('X-3-2026', 'Jgdo. L. y G. de Laja', 'Persona Uno', '11111111-1', 'AFT EJEMPLO UNO', 45)],
            [
                {'op': 'apply', 'fila': 0, 'overrides': {'programa': 'DCE EJEMPLO'}},
            ],
        ),
        caso(
            'espera_cambia_programa_a_sin_seguimiento',
            'ESPERA',
            enc_espera,
            [('X-4-2026', 'Jgdo. L. y G. de Laja', 'Persona Uno', '11111111-1', 'AFT EJEMPLO UNO', 45)],
            [
                {'op': 'apply', 'fila': 0, 'overrides': {'programa': 'SIN SEGUIMIENTO'}},
            ],
        ),
        caso(
            'cumplimiento_egreso_por_vencer',
            'CUMPLIMIENTO',
            enc_cumpl,
            [('X-5-2026', 'Jgdo. L. y G. de Laja', 'Persona Uno', '11111111-1', 'AFT EJEMPLO UNO', '2026-12-30', 50, 108)],
            [
                {'op': 'apply', 'fila': 0, 'overrides': {'egreso_proy': '2026-09-25', 'dias_egresar': '12'}},
            ],
        ),
        caso(
            'productos_usan_datos_anulados',
            'ESPERA',
            enc_espera,
            [('X-6-2026', 'Jgdo. L. y G. de Laja', 'Persona Original', '11111111-1', 'AFT EJEMPLO UNO', 10)],
            [
                {'op': 'apply', 'fila': 0, 'overrides': {'espera': '45', 'nombre': 'Persona Editada', 'rit': 'X-99-2026'}},
            ],
            con_borradores=True,
        ),
        caso(
            'columna_invalida_rechaza',
            'ESPERA',
            enc_espera,
            [('X-7-2026', 'Jgdo. L. y G. de Laja', 'Persona Uno', '11111111-1', 'AFT EJEMPLO UNO', 10)],
            [
                {'op': 'apply', 'fila': 0, 'overrides': {'no_existe': '1'}},
            ],
        ),
    ]
    destino.write_bytes(
        json.dumps({'corte': CORTE.isoformat(), 'casos': casos}, ensure_ascii=False, indent=1).encode('utf-8')
    )
    print(f'{len(casos)} casos en {destino}')


if __name__ == '__main__':
    main()
