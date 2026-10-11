"""Validación local del contrato sin datos personales en el diagnóstico."""

import argparse
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'src'))
from nurus.capture import read_capture

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('command', choices=['validate'])
    parser.add_argument('path', type=Path)
    args = parser.parse_args()
    capture = read_capture(args.path)
    print(
        json.dumps(
            {
                'valid': True,
                'schema_version': capture.manifest['schema_version'],
                'sha256': capture.digest,
                'tables': {k: len(v) for k, v in capture.tables.items()},
                'states': capture.manifest['table_states'],
            },
            ensure_ascii=False,
            indent=2,
        )
    )
