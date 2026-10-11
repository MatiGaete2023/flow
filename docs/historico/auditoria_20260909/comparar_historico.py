"""Comparación de observaciones, no certificación de equivalencia de productos.
Requiere la carpeta Asistente original; no publica datos de personas.
"""
import argparse
from collections import Counter
from datetime import datetime, date
import importlib
import json
from pathlib import Path
import sys
from unittest.mock import patch
from contextlib import ExitStack
import pandas as pd
from nurus.rus import Mode, read_workbook, evaluate_batch

class FrozenMeta(type):
    def __instancecheck__(cls, value):
        # Mantener el reconocimiento de datetime reales al fijar el reloj antiguo.
        return isinstance(value, datetime)

class FrozenDateTime(datetime, metaclass=FrozenMeta):
    @classmethod
    def now(cls, tz=None):
        return cls(2026, 9, 9, 12, tzinfo=tz)

def run(sources, legacy):
    sys.path.insert(0, str(legacy.resolve()))
    from motor.procesador import _calcular_simple, _calcular_cumplimiento, _construir_indice_hoja2
    from motor.mapeo_columnas import mapear_columnas, mapear_columnas_hoja2
    from motor.reglas_espera import generar_observacion_espera
    from motor.reglas_informes import generar_observacion_informes
    output = []
    with ExitStack() as stack:
        for name in ['utilidades', 'reglas_espera', 'reglas_cumplimiento', 'reglas_informes', 'procesador']:
            stack.enter_context(patch.object(importlib.import_module('motor.' + name), 'datetime', FrozenDateTime))
        for filename, modes in [('AGOSTO.xlsx', list(Mode)),
                                ('RUS_CUMPLIMIENTO_20260904_114132_856292.xlsx', [Mode.CUMPLIMIENTO])]:
            path = sources / filename
            for mode in modes:
                entry = {'file': filename, 'mode': mode.value, 'as_of': '2026-09-09'}
                frame = pd.read_excel(path, sheet_name='Informes' if mode is Mode.INFORMES else mode.value,
                                      dtype=object, keep_default_na=False)
                columns = mapear_columnas(frame, mode.value)
                # Para medir reglas con los mismos datos se usa Informes de AGOSTO
                # como cruce; no acredita el selector automático del motor antiguo.
                if mode is Mode.CUMPLIMIENTO:
                    index = {}
                    if filename == 'AGOSTO.xlsx':
                        cross = pd.read_excel(path, sheet_name='Informes', dtype=object, keep_default_na=False)
                        index = _construir_indice_hoja2(cross, mapear_columnas_hoja2(cross))
                    old, _ = _calcular_cumplimiento(frame, columns, index)
                else:
                    function = generar_observacion_espera if mode is Mode.ESPERA else generar_observacion_informes
                    old, _ = _calcular_simple(frame, columns, function)
                entry['legacy_rows_evaluated'] = len(old)
                entry['legacy_ingreso_column'] = columns.get('ingreso')
                try:
                    new = evaluate_batch(read_workbook(path, mode), as_of=date(2026, 9, 9))
                    equal, different = 0, Counter()
                    differences = []
                    for item in new.evaluations:
                        text = old.iloc[item.source.row_number - 2]['OBSERVACION']
                        if text == item.observation:
                            equal += 1
                        else:
                            different[item.status.value] += 1
                            differences.append({'source_row': item.source.row_number,
                                                'status': item.status.value,
                                                'rule_ids': item.rule_ids,
                                                'legacy_length': len(text),
                                                'nurus_length': len(item.observation)})
                    entry.update(nurus_rows=len(new.evaluations), identical_observations=equal,
                                 differences_by_status=dict(different), differences=differences)
                except Exception as exc:
                    entry.update(nurus_error=str(exc))
                output.append(entry)
    return output

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--sources', type=Path, required=True)
    parser.add_argument('--legacy', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    text = json.dumps(run(args.sources, args.legacy), ensure_ascii=False, indent=2)
    args.output.write_text(text + '\n', encoding='utf-8')
    print(text)
