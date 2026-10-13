#!/usr/bin/env bash
# Los mismos controles de código de «CI rápida Linux» (sin instalación ni inventario de dependencias),
# antes de hacer push: un push rojo cuesta una corrida. Mantener sincronizado con .github/workflows/ci-rapida.yml.
# Uso:
#   tools/ci_local.sh                         # controles + ambas suites (como la CI)
#   tools/ci_local.sh descargador/test_x.py   # controles + solo esas pruebas (mientras se desarrolla)
set -euo pipefail
cd "$(dirname "$0")/.."
export PYTHONPATH="$PWD/src${PYTHONPATH:+:$PYTHONPATH}"
run() { echo "== $*"; "$@"; }
run python tools/check_real_ids.py
run python tools/estado.py validar
run python tools/check_links.py
run python -m ruff check --select E9,F63,F7,F82,F401,F841 src descargador tools --extend-exclude src/nurus/personal/motor
run python -m ruff format --check src descargador tools tests
xvfb=""
if [ -z "${DISPLAY:-}" ] && command -v xvfb-run >/dev/null; then xvfb="xvfb-run -a"; fi
if [ "$#" -gt 0 ]; then
  run $xvfb python -m pytest -q -p no:cacheprovider "$@"
else
  run $xvfb python -m pytest -q -p no:cacheprovider tests
  run $xvfb python -m pytest -q -p no:cacheprovider descargador
  run node descargador/test_bitacoras_extension.cjs
fi
echo "CI local: PASA"
