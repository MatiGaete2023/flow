#!/usr/bin/env bash
# Pruebas de app/ con salida resumida. Uso:
#   tools/probar_app.sh rapido [filtro]   pruebas sin datos externos (o solo las que calzan con el filtro)
#   tools/probar_app.sh todo              agrega oráculos y pruebas con datos versionados (--include-ignored)
# Las pruebas que necesitan un ZIP generado por Flow Python solo corren si FLOW_E3_PAQUETE está definida.
set -uo pipefail
raiz="$(cd "$(dirname "$0")/.." && pwd)"
export FLOW_CASOS="${FLOW_CASOS:-$raiz/rust/casos}"
export FLOW_FIXTURES="${FLOW_FIXTURES:-$raiz/rust/crates/flow-tabular/tests/datos}"
export FLOW_REFERENCIA="${FLOW_REFERENCIA:-$raiz}"
modo="${1:-rapido}"
[ $# -gt 0 ] && shift
base=(cargo test --manifest-path "$raiz/app/Cargo.toml" --locked)
# eframe no trae backend de ventana en Linux por omisión.
[ "$(uname -s)" = Linux ] && base+=(--features eframe/x11)
case "$modo" in
  rapido) orden=("${base[@]}" "$@") ;;
  todo)
    omitir=(--skip velocidad_20000 --skip exporta_mismo_lote_de_python)
    if [ -z "${FLOW_E3_PAQUETE:-}" ]; then
      omitir+=(--skip contexto_e3 --skip a2_consignacion_p1_y_sesion --skip ui_trabajo_consigna_otro_programa)
      echo "AVISO: sin FLOW_E3_PAQUETE se omiten las pruebas del ZIP de contexto de Flow Python."
    fi
    orden=("${base[@]}" "$@" -- --include-ignored "${omitir[@]}") ;;
  *) echo "Uso: $0 {rapido|todo} [filtro]" >&2; exit 2 ;;
esac
"${orden[@]}" 2>&1 | grep -E '^test result|FAILED|panicked|^error|^[A-Z0-9_]+: |^oraculo'
exit "${PIPESTATUS[0]}"
