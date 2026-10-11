"""Crear/validar actas locales; no ejecuta pruebas ni aprueba gates."""

import argparse
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'src'))
from nurus.capture.acceptance import CASES, KINDS, template, read_acta, write_acta, summary
from nurus.capture import CaptureError


def main():
    # JSON y diagnósticos usan la misma codificación al ejecutarse por tuberías.
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, 'reconfigure'):
            stream.reconfigure(encoding='utf-8')
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    create = commands.add_parser('init', help='Plantilla con todos los resultados NO_EJECUTADO.')
    create.add_argument('output', type=Path)
    create.add_argument('--commit', required=True)
    create.add_argument('--assistant-version', required=True)
    create.add_argument('--downloader-version', required=True)
    create.add_argument('--extension-version', required=True)
    create.add_argument('--case', choices=CASES, action='append')
    create.add_argument('--gate', choices=[f'G{i}' for i in range(14)], default='')
    create.add_argument('--kind', choices=KINDS, default='INSTITUCIONAL')
    check = commands.add_parser('validate', help='Valida estructura, no autenticidad de evidencias ni aceptación.')
    check.add_argument('input', type=Path)
    args = parser.parse_args()
    try:
        if args.command == 'init':
            artifact = {
                'source_commit': args.commit,
                'assistant_version': args.assistant_version,
                'downloader_version': args.downloader_version,
                'extension_version': args.extension_version,
            }
            data = template(artifact, cases=args.case or CASES, gate=args.gate, evidence_kind=args.kind)
            write_acta(args.output, data)
        else:
            data = read_acta(args.input)
        print(json.dumps(summary(data), ensure_ascii=False, indent=2))
    except (CaptureError, ValueError, OSError):
        # No imprimir contenido o ruta del archivo de entrada: puede ser sensible.
        parser.exit(2, 'Acta no válida o destino no disponible. Consulte los campos y códigos del manual.\n')


if __name__ == '__main__':
    main()
