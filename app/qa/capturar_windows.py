"""Captura las vistas de Flow Rust con el ejemplo de la aplicación y un perfil aislado."""

import argparse
import os
import shutil
import subprocess
from pathlib import Path
from uuid import uuid4

VISTAS = (
    ("trabajo", None),
    ("correos", "correos"),
    ("resoluciones", "resoluciones"),
    ("anonimizador", "anonimizador"),
    ("descargador", "descargador"),
    ("ajustes-oscuro", "ajustes,oscuro"),
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--exe", required=True, type=Path)
    parser.add_argument("--salida", required=True, type=Path)
    args = parser.parse_args()
    exe = args.exe.resolve(strict=True)
    salida = args.salida.resolve()
    salida.mkdir(parents=True, exist_ok=True)
    perfil = salida / f"flow-rust-qa-{uuid4().hex}"
    perfil.mkdir()
    env = dict(os.environ, APPDATA=str(perfil), TEMP=str(perfil), TMP=str(perfil))
    try:
        trabajos = [(nombre, vista, None) for nombre, vista in VISTAS]
        if paquete := os.environ.get("FLOW_E3_PAQUETE"):
            trabajos.append(("trabajo-contexto", None, Path(paquete).resolve(strict=True)))
        for nombre, vista, paquete in trabajos:
            # La ruta nueva hace observable el fallo aunque exista una captura anterior.
            captura = perfil / f"{nombre}.png"
            comando = [str(exe), "--screenshot", str(captura)]
            if vista:
                comando += ["--vista", vista]
            if paquete:
                comando.append(str(paquete))
            subprocess.run(comando, env=env, check=True, timeout=35)
            datos = captura.read_bytes()
            if not datos.startswith(b"\x89PNG\r\n\x1a\n") or len(datos) < 24:
                raise ValueError(f"Captura PNG inválida para {nombre}")
            (salida / captura.name).write_bytes(datos)
            print(f"{nombre}: {len(datos)} bytes")
    finally:
        if perfil.resolve().parent == salida:
            shutil.rmtree(perfil)


if __name__ == "__main__":
    main()
