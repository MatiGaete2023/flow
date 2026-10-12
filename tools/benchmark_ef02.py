"""EF-02: benchmark sintético de importación, edición, exportación y guardado del Asistente.

Uso:
    python tools/benchmark_ef02.py [--filas 200 1000] [--repeticiones 3] [--perfil] [--salida archivo.json]

Solo datos ficticios (nurus.capture.fixtures). Los tiempos dependen del equipo: sirven para comparar
antes/después en la misma máquina, no como requisito. --perfil agrega las 12 funciones con más tiempo
acumulado por operación (nombres de función, nunca datos).
"""

import argparse
import cProfile
import json
import pstats
import statistics
import sys
import tempfile
import time
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

from nurus.capture import write_capture  # noqa: E402
from nurus.capture.fixtures import fixture  # noqa: E402
from nurus.personal import record_edits  # noqa: E402
from nurus.personal.capture_input import import_capture  # noqa: E402
from nurus.personal.config import defaults  # noqa: E402

CUTOFF = date(2026, 10, 8)


def _profile(function):
    profiler = cProfile.Profile()
    profiler.enable()
    function()
    profiler.disable()
    stats = pstats.Stats(profiler)
    rows = sorted(stats.stats.items(), key=lambda item: item[1][3], reverse=True)
    top = []
    for (filename, _line, name), (_cc, calls, _tt, cumulative, _callers) in rows:
        if "nurus" not in filename:
            continue
        module = filename.split("nurus", 1)[1].lstrip("/\\").replace("\\", "/")
        top.append({"funcion": f"{module}:{name}", "llamadas": calls, "acumulado_s": round(cumulative, 4)})
        if len(top) == 12:
            break
    return top


def measure(rows, repetitions, profile):
    with tempfile.TemporaryDirectory() as folder:
        folder = Path(folder)
        metadata, tables = fixture(rows)
        package = write_capture(folder / "captura.zip", metadata, tables)
        config = defaults()
        timings = {"importar": [], "editar_1_registro": [], "exportar": [], "guardar_sesion": []}
        profiles = {}
        work = None
        for repetition in range(repetitions):
            start = time.perf_counter()
            work = import_capture(package, config, as_of=CUTOFF)
            timings["importar"].append(time.perf_counter() - start)
            target = work.rows[0].id
            start = time.perf_counter()
            record_edits.apply(work, [target], review={"OBSERVACION": f"Observación ficticia {repetition}."})
            timings["editar_1_registro"].append(time.perf_counter() - start)
            start = time.perf_counter()
            work.export(folder / f"salida-{repetition}.xlsx")
            timings["exportar"].append(time.perf_counter() - start)
            start = time.perf_counter()
            work.save(folder / f"sesion-{repetition}")
            timings["guardar_sesion"].append(time.perf_counter() - start)
        if profile:
            target = work.rows[0].id
            profiles["editar_1_registro"] = _profile(
                lambda: record_edits.apply(work, [target], review={"OBSERVACION": "Perfil ficticio."})
            )
            profiles["exportar"] = _profile(lambda: work.export(folder / "perfil.xlsx"))
            profiles["guardar_sesion"] = _profile(lambda: work.save(folder / "sesion-perfil"))
        result = {
            "filas": len(work.rows),
            "mediana_s": {key: round(statistics.median(values), 4) for key, values in timings.items()},
        }
        if profiles:
            result["perfil"] = profiles
        return result


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--filas", type=int, nargs="+", default=[200, 1000])
    parser.add_argument("--repeticiones", type=int, default=3)
    parser.add_argument("--perfil", action="store_true")
    parser.add_argument("--salida", type=Path)
    args = parser.parse_args(argv)
    if any(n < 1 for n in args.filas) or args.repeticiones < 1:
        parser.error("Filas y repeticiones deben ser positivas.")
    report = {
        "python": sys.version.split()[0],
        "mediciones": [measure(n, args.repeticiones, args.perfil) for n in args.filas],
    }
    text = json.dumps(report, ensure_ascii=False, indent=2)
    if args.salida:
        args.salida.write_text(text + "\n", encoding="utf-8")
    print(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
