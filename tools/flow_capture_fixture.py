"""Genera capturas exclusivamente ficticias para aceptación y benchmark."""

from pathlib import Path
import argparse
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'src'))
from nurus.capture.contract import write_capture


from nurus.capture.fixtures import fixture

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('destination', type=Path)
    parser.add_argument('--rows', type=int, default=4)
    parser.add_argument('--partial', action='store_true')
    args = parser.parse_args()
    if not 0 <= args.rows <= 20000:
        parser.error('Fixture limitada a 20.000 registros.')
    m, t = fixture(args.rows, args.partial)
    print(write_capture(args.destination, m, t))
