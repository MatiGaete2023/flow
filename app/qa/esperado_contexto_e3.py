"""Salida de referencia de Flow Python para el paquete C (contexto-e3-historia.zip).

Uso (desde la raíz): python app/qa/esperado_contexto_e3.py <zip> <salida.json>

Abre el ZIP con nurus.personal.capture_input.import_capture en las tres etapas de Trabajo. Escribe por fila
record_id, etapa, corte, observación, reglas y marcas. Si una etapa no tiene filas en el paquete, la deja como
NO_APLICA con el motivo. El corte movido es anterior al inicio de la ventana de historia (2026-06-13).
"""

import json
import sys
from datetime import date
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(RAIZ / 'src'))

from nurus.personal.capture_input import import_capture
from nurus.personal.config import defaults

CORTE = date(2026, 10, 19)
CORTE_MOVIDO = date(2026, 6, 12)
ETAPAS = ('ESPERA', 'CUMPLIMIENTO', 'INFORMES')


def filas_de(zip_path, corte):
    trabajo = import_capture(zip_path, defaults(), as_of=corte)
    filas = []
    for row in trabajo.rows:
        etapa = trabajo.capture_modes[row.id]
        filas.append({
            'record_id': row.id,
            'etapa': etapa,
            'corte': corte.isoformat(),
            'observacion': row.observation,
            'reglas': list(row.rules),
            'marcas': [{'nivel': nivel, 'codigo': codigo, 'texto': texto}
                       for nivel, codigo, texto in row.avisos],
        })
    return trabajo, filas


def main():
    zip_path = Path(sys.argv[1]).resolve()
    salida = Path(sys.argv[2]).resolve()
    trabajo, filas = filas_de(zip_path, CORTE)
    _, filas_movidas = filas_de(zip_path, CORTE_MOVIDO)
    etapas = [e for e in ETAPAS if e in trabajo.capture_modes.values()]
    no_aplica = [
        {'etapa': etapa, 'corte': CORTE.isoformat(),
         'motivo': f'capture_modes no registra filas de la etapa {etapa}.'}
        for etapa in ETAPAS if etapa not in etapas
    ]
    salida.write_text(
        json.dumps({
            'corte_fijo': CORTE.isoformat(),
            'corte_movido': CORTE_MOVIDO.isoformat(),
            'etapas': etapas,
            'filas': filas + filas_movidas,
            'no_aplica': no_aplica,
        }, ensure_ascii=False, indent=1),
        encoding='utf-8', newline='\n')
    print(f'{len(filas)} filas en {len(etapas)} etapas a {CORTE}; '
          f'{len(filas_movidas)} filas a {CORTE_MOVIDO}; no aplica: {len(no_aplica)}')


if __name__ == '__main__':
    main()
