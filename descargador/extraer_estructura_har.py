"""Resume la estructura de un HAR de SITFA sin valores, cookies ni cuerpos.

Uso (en el PC autorizado):
    python extraer_estructura_har.py captura.har estructura.json

Solo `estructura.json` puede compartirse. El HAR original no sale del equipo.
Conserva: método, ruta sin parámetros, nombres de campos enviados, nombres de
acciones onclick con su cantidad de argumentos y tipo de ventana (si es literal
numérico de 1 o 2 dígitos), tipo de contenido y conteos. Descarta todo valor.
"""
import json
import re
import sys
from collections import Counter
from pathlib import Path
from urllib.parse import parse_qsl, urlsplit

ONCLICK = re.compile(r"onclick\s*=\s*[\"']\s*(?:javascript:)?\s*([A-Za-z_$][\w$]*)\s*\(([^)]*)\)", re.I)
FIELD = re.compile(r"<(?:input|select|textarea)[^>]*\bname\s*=\s*[\"']([^\"']+)[\"']", re.I)
MAX_BODY = 20 * 1024 * 1024


def field_names(request):
    names = [item.get("name", "") for item in request.get("queryString", [])]
    post = request.get("postData") or {}
    if post.get("params"):
        names += [item.get("name", "") for item in post["params"]]
    elif post.get("text") and "x-www-form-urlencoded" in post.get("mimeType", ""):
        names += [name for name, _ in parse_qsl(post["text"], keep_blank_values=True)]
    return sorted({name for name in names if name})


def onclick_actions(html):
    actions = Counter()
    for name, args in ONCLICK.findall(html):
        parts = [part.strip().strip("'\"") for part in args.split(",")] if args.strip() else []
        window_type = parts[0] if parts and re.fullmatch(r"\d{1,2}", parts[0]) else "?"
        actions[f"{name}/{len(parts)}/tipo={window_type}"] += 1
    return dict(sorted(actions.items()))


def summarize(har):
    entries = har.get("log", {}).get("entries", [])
    result = []
    for entry in entries:
        request, response = entry.get("request", {}), entry.get("response", {})
        url = urlsplit(request.get("url", ""))
        content = response.get("content") or {}
        mime = (content.get("mimeType") or "").split(";")[0]
        item = {
            "metodo": request.get("method", ""),
            "host": url.hostname or "",
            "ruta": url.path,
            "campos_enviados": field_names(request),
            "estado_http": response.get("status"),
            "tipo_contenido": mime,
            "bytes_respuesta": content.get("size"),
        }
        text = content.get("text") or ""
        if mime == "text/html" and text and content.get("encoding") != "base64" and len(text) <= MAX_BODY:
            item["acciones_onclick"] = onclick_actions(text)
            item["campos_formulario"] = sorted(set(FIELD.findall(text)))
            item["filas_tabla"] = len(re.findall(r"<tr\b", text, re.I))
        result.append(item)
    return {
        "tipo": "ESTRUCTURA_HAR_SIN_VALORES",
        "version": 1,
        "entradas": len(result),
        "peticiones": result,
        "advertencia": "Sin cookies, encabezados, parámetros, cuerpos ni datos personales.",
    }


def main(argv):
    if len(argv) != 2:
        print("Uso: python extraer_estructura_har.py captura.har estructura.json")
        return 2
    source, target = Path(argv[0]), Path(argv[1])
    if target.exists():
        print("El archivo de salida ya existe; elige otro nombre.")
        return 1
    data = json.loads(source.read_text(encoding="utf-8"))
    try:
        with target.open("x", encoding="utf-8") as output:
            output.write(json.dumps(summarize(data), ensure_ascii=False, indent=2))
    except FileExistsError:
        print("El archivo de salida ya existe; elige otro nombre.")
        return 1
    print(f"Estructura guardada en {target}. Comparte solo ese archivo; conserva el HAR en este equipo.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
