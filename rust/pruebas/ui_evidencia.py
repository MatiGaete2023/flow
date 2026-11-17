"""Ejecuta la interfaz real en un entorno ficticio aislado y convierte capturas PPM a PNG."""
import argparse
import base64
import json
import os
from pathlib import Path
import struct
import subprocess
import zlib


def png(ppm):
    cabecera, size, limite, pixels = ppm.read_bytes().split(b"\n", 3)
    if cabecera != b"P6" or limite != b"255":
        raise ValueError("Captura PPM no admitida")
    width, height = map(int, size.split())
    if len(pixels) != width * height * 3:
        raise ValueError("Captura incompleta")
    def chunk(tipo, datos):
        return struct.pack(">I", len(datos)) + tipo + datos + struct.pack(">I", zlib.crc32(tipo + datos))
    filas = b"".join(b"\x00" + pixels[y * width * 3:(y + 1) * width * 3] for y in range(height))
    data = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
    data += chunk(b"IDAT", zlib.compress(filas, 9)) + chunk(b"IEND", b"")
    destino = ppm.with_suffix(".png")
    destino.write_bytes(data)
    ppm.unlink()
    return destino


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--exe", required=True)
    parser.add_argument("--salida", required=True)
    parser.add_argument("--completa", action="store_true")
    args = parser.parse_args()
    destino = Path(args.salida).resolve()
    destino.mkdir(parents=True, exist_ok=False)
    exe = str(Path(args.exe).resolve())
    pantallas = ["trabajo", "trabajo-evidencia", "trabajo-editor", "correos", "resoluciones", "resultados", "contactos", "captura", "anonimizador", "meta4", "revision", "revision-evidencia", "revision-propuesta", "ayuda", "recalculo"]
    dimensiones = [(1366, 768), (1920, 1080)] if args.completa else [(1366, 768)]
    escalas = [1.0, 1.25, 1.5, 2.0] if args.completa else [1.0]
    resultados = []
    for pantalla in pantallas:
        for ancho, alto in dimensiones:
            for escala in escalas:
                nombre = f"{pantalla}-{ancho}x{alto}-{round(escala * 100)}"
                proceso = subprocess.run([exe, "--demo-ui", pantalla, str(destino), str(ancho), str(alto), str(escala)],
                                         capture_output=True, text=True, timeout=40)
                ppm = destino / f"{nombre}.ppm"
                if proceso.returncode or not ppm.is_file():
                    raise RuntimeError(f"Falló captura {nombre}: código {proceso.returncode}: {proceso.stderr[-3000:]}")
                imagen = png(ppm)
                datos = json.loads(imagen.with_suffix(".json").read_text())
                if datos["pixeles"] != [ancho, alto]:
                    raise RuntimeError(f"Tamaño observado distinto del solicitado: {datos}")
                resultados.append(datos)
    (destino / "MATRIZ_CAPTURAS.json").write_text(json.dumps(resultados, indent=2), encoding="utf-8")
    print(f"CAPTURAS_SINTETICAS={len(resultados)}")
    vista = destino / "trabajo-1366x768-100.png"
    print("FLOW_UI_PREVIEW_BEGIN")
    print(base64.b64encode(vista.read_bytes()).decode("ascii"))
    print("FLOW_UI_PREVIEW_END")
    if args.completa:
        for nombre in ["revision-1366x768-100.png","revision-propuesta-1366x768-200.png","revision-evidencia-1366x768-200.png","contactos-1366x768-200.png"]:
            print("FLOW_UI_IMAGE_BEGIN " + nombre)
            print(base64.b64encode((destino / nombre).read_bytes()).decode("ascii"))
            print("FLOW_UI_IMAGE_END")
    print("FLOW_UI_MATRIX_BEGIN")
    print(json.dumps(resultados))
    print("FLOW_UI_MATRIX_END")
    paquete = destino / "Paquete_FICTICIO.zip"
    subprocess.run([exe, "--paquete-demo", str(paquete)], check=True, timeout=20)
    print("FLOW_DEMO_ZIP_BEGIN")
    print(base64.b64encode(paquete.read_bytes()).decode("ascii"))
    print("FLOW_DEMO_ZIP_END")


if __name__ == "__main__":
    main()

