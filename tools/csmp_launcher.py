"""Arranque Windows y diagnóstico aislado de distribución."""

import sys
from pathlib import Path
import json

if __name__ == '__main__':
    try:
        from nurus.personal.app import main

        main()
    except Exception as exc:
        option = '--verificar-paquete' if '--verificar-paquete' in sys.argv else None
        if option:
            index = sys.argv.index(option) + 1
            if index < len(sys.argv):
                target = Path(sys.argv[index]).resolve()
                target.parent.mkdir(parents=True, exist_ok=True)
                target.with_suffix('.error.json').write_text(
                    json.dumps({'ok': False, 'error': str(exc)}, ensure_ascii=False), encoding='utf-8'
                )
            sys.exit(1)
        raise
