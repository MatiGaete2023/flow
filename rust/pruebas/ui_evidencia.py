"""Ejecuta la interfaz real en un entorno ficticio aislado y convierte capturas PPM a PNG."""
import argparse
import base64
import json
import os
from pathlib import Path
import struct
import subprocess
import time
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


def miniatura(ppm):
    _, size, _, pixels = ppm.read_bytes().split(b"\n", 3)
    width, height = map(int, size.split())
    tw, th = 320, 180
    copia = bytearray(tw * th * 3)
    for y in range(th):
        for x in range(tw):
            origen = (min(height - 1, y * height // th) * width + min(width - 1, x * width // tw)) * 3
            destino = (y * tw + x) * 3
            copia[destino:destino + 3] = pixels[origen:origen + 3]
    return bytes(copia)


def mosaico(destino, nombre, vistas):
    ancho, alto = 320 * 5, 180 * ((len(vistas) + 4) // 5)
    pixels = bytearray(ancho * alto * 3)
    for n, vista in enumerate(vistas):
        for y in range(180):
            inicio = (((n // 5) * 180 + y) * ancho + (n % 5) * 320) * 3
            pixels[inicio:inicio + 960] = vista[y * 960:(y + 1) * 960]
    ppm = destino / (nombre + ".ppm")
    ppm.write_bytes(f"P6\n{ancho} {alto}\n255\n".encode() + pixels)
    return png(ppm)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--exe", required=True)
    parser.add_argument("--salida", required=True)
    parser.add_argument("--completa", action="store_true")
    args = parser.parse_args()
    destino = Path(args.salida).resolve()
    destino.mkdir(parents=True, exist_ok=False)
    exe = str(Path(args.exe).resolve())
    pantallas = ["trabajo", "trabajo-evidencia", "trabajo-editor", "correos", "resoluciones", "resultados", "contactos", "captura", "anonimizador", "meta4", "revision", "revision-evidencia", "revision-propuesta", "revision-inicial", "ayuda", "recalculo", "sin-sesion", "error", "correo-incierto", "proyecto-obsoleto"]
    if os.name != "nt":
        pantallas.append("teclado")
    dimensiones = [(1366, 768), (1920, 1080)] if args.completa else [(1366, 768)]
    escalas = [1.0, 1.25, 1.5, 2.0] if args.completa else [1.0]
    resultados = []
    vistas = {}
    for pantalla in pantallas:
        for ancho, alto in dimensiones:
            for escala in escalas:
                nombre = f"{pantalla}-{ancho}x{alto}-{round(escala * 100)}"
                comando = [exe, "--demo-ui", pantalla, str(destino), str(ancho), str(alto), str(escala)]
                if pantalla == "teclado":
                    with subprocess.Popen(comando, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True) as proceso:
                        ventanas = subprocess.run(["xdotool", "search", "--sync", "--onlyvisible", "--all", "--pid", str(proceso.pid), "--name", "Flow"],
                                                  capture_output=True, text=True, timeout=15, check=True).stdout.split()
                        if len(ventanas) != 1:
                            raise RuntimeError(f"No se encontró una única ventana sintética: {ventanas}")
                        subprocess.run(["xdotool", "windowfocus", "--sync", ventanas[0]], check=True, timeout=5)
                        foco = subprocess.run(["xdotool", "getwindowfocus"], capture_output=True, text=True, check=True).stdout.strip()
                        if foco != ventanas[0]:
                            raise RuntimeError("El foco no pertenece a la demostración")
                        time.sleep(0.2)
                        subprocess.run(["xdotool", "key", "--clearmodifiers", "Tab", "Tab", "Tab"], check=True, timeout=5)
                        stdout, stderr = proceso.communicate(timeout=40)
                        codigo = proceso.returncode
                else:
                    proceso = subprocess.run(comando, capture_output=True, text=True, timeout=40)
                    codigo, stderr = proceso.returncode, proceso.stderr
                ppm = destino / f"{nombre}.ppm"
                if codigo or not ppm.is_file():
                    raise RuntimeError(f"Falló captura {nombre}: código {codigo}: {stderr[-3000:]}")
                _, _, _, pixeles = ppm.read_bytes().split(b"\n", 3)
                if pixeles.count(bytes((32, 36, 58))) < ancho * alto // 20:
                    raise RuntimeError(f"{nombre}: no se ve el fondo índigo de Estudio D")
                vistas.setdefault((ancho, alto, escala), []).append(miniatura(ppm))
                imagen = png(ppm)
                datos = json.loads(imagen.with_suffix(".json").read_text())
                if datos["pixeles"] != [ancho, alto]:
                    raise RuntimeError(f"Tamaño observado distinto del solicitado: {datos}")
                if pantalla == "teclado" and not datos["foco_activo"]:
                    raise RuntimeError("Tab no produjo foco observable en la interfaz real")
                resultados.append(datos)
    (destino / "MATRIZ_CAPTURAS.json").write_text(json.dumps(resultados, indent=2), encoding="utf-8")
    (destino / "INDICE_MOSAICOS.json").write_text(json.dumps(pantallas), encoding="utf-8")
    for (ancho, alto, escala), entradas in vistas.items():
        nombre = f"mosaico-{ancho}x{alto}-{round(escala * 100)}"
        imagen = mosaico(destino, nombre, entradas)
        if escala in (1.0, 2.0):
            print("FLOW_UI_IMAGE_BEGIN " + imagen.name)
            print(base64.b64encode(imagen.read_bytes()).decode("ascii"))
            print("FLOW_UI_IMAGE_END")
    print(f"CAPTURAS_SINTETICAS={len(resultados)}")
    vista = destino / "trabajo-1366x768-100.png"
    print("FLOW_UI_PREVIEW_BEGIN")
    print(base64.b64encode(vista.read_bytes()).decode("ascii"))
    print("FLOW_UI_PREVIEW_END")
    if args.completa:
        for nombre in ["revision-1366x768-100.png","revision-propuesta-1366x768-200.png","revision-evidencia-1366x768-200.png","revision-inicial-1366x768-200.png","correo-incierto-1366x768-200.png","proyecto-obsoleto-1366x768-200.png","contactos-1366x768-200.png"]:
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
