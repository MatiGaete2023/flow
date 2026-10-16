"""Copia anónima de un Excel RUS/SITFA. La lógica vive en nurus.personal.anonymize.

Uso (en el PC autorizado):
    python tools/anonimizar_excel.py original.xls copia_anonima.xlsx [--desplazar-fechas DIAS]
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from nurus.personal.anonymize import main

if __name__ == "__main__":
    sys.exit(main())
