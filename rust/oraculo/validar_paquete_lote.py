"""Oráculo de «Crear paquete de este lote» (Descargador Rust): valida con el contrato de Python el ZIP que Rust arma desde
un lote ficticio y lo importa con el Asistente Python, para que Rust compare lo que este propone.

Uso (desde la raíz del repo, con el Python del entorno de pruebas):
    python rust/oraculo/validar_paquete_lote.py

Lee rust/crates/flow-trabajo/tests/datos/paquete_desde_lote.zip (lo regenera
`FLOW_REGENERAR=1 cargo test -p flow-trabajo --test exportar`) y escribe paquete_desde_lote.json al lado.
Termina con error si `read_capture` (validate_manifest, huellas, filas y cobertura) rechaza el paquete.
"""

import json
import sys
from datetime import date
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(RAIZ / 'src'))

from nurus.capture import read_capture  # noqa: E402
from nurus.personal.capture_input import import_capture  # noqa: E402
from nurus.personal.config import defaults  # noqa: E402

CORTE = date(2026, 10, 20)


def main():
    datos = RAIZ / 'rust/crates/flow-trabajo/tests/datos'
    paquete = datos / 'paquete_desde_lote.zip'
    captura = read_capture(paquete)
    print('Contrato válido:', captura.manifest['schema_id'], captura.manifest['schema_version'])
    print('Estados:', json.dumps(captura.manifest['table_states'], ensure_ascii=False, sort_keys=True))
    print('Filas:', {t: len(f) for t, f in sorted(captura.tables.items())})
    work = import_capture(paquete, defaults(), as_of=CORTE)
    filas = [
        {
            'rit': json.loads(work.capture_sources[r.id]['normalized_json']).get('RIT', ''),
            'observacion': r.observation,
            'reglas': list(r.rules),
            'acciones': list(r.actions),
            'marcas': [{'nivel': n, 'codigo': c, 'texto': t} for n, c, t in r.avisos],
            'cronologia': work.capture_context[r.id]['display'].get('status', '')
            if isinstance(work.capture_context.get(r.id, {}).get('display'), dict)
            else '',
        }
        for r in work.rows
    ]
    (datos / 'paquete_desde_lote.json').write_text(
        json.dumps({'corte': CORTE.isoformat(), 'filas': filas}, ensure_ascii=False, indent=1) + '\n', encoding='utf-8'
    )
    for f in filas:
        print(f['rit'], '|', f['observacion'][:160], '|', f['reglas'], '|', f['acciones'], '|', f['cronologia'])


if __name__ == '__main__':
    main()
