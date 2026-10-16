"""Resume la estructura de un HAR de SITFA sin valores, cookies ni cuerpos.

Uso (en el PC autorizado):
    python extraer_estructura_har.py captura.har estructura.json

Solo `estructura.json` puede compartirse. El HAR original no sale del equipo.
Conserva solo host, rutas, campos y acciones de listas estructurales explícitas,
cantidad de argumentos, tipo de ventana numérico, contenido conocido y conteos.
Los nombres desconocidos se reemplazan por marcadores: también pueden ser datos.
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
HOSTS = frozenset({"familia.pjud.cl", "sitfa.ejemplo.invalid"})
PATHS = frozenset(
    {
        "/SITFAWEB/InformesDAction.do",
        "/SITFAWEB/MaoDAction.do",
        "/SITFAWEB/jsp/Login/LoginB4.jsp",
        "/Consulta/Buscar",
    }
)
FIELDS = frozenset(
    """COD_Centro GLS_TribunalOrigen FLG_Consulta COD_Lengueta
COD_Tribunal_sel TIP_Consulta COD_CentroResidencial COD_PlazoIntervencion TIP_Informe
COD_TiempoEspera TIP_Causa ROL_Causa ERA_Causa RUT_Consulta RUT_DvConsulta
COD_TipLitigante CHK_Consulta FEC_Inicio FEC_Fin irAccion NUM_PaginaEspera
NUM_TotalEspera NUM_PaginaCumplimiento NUM_TotalCumplimiento NUM_PaginaInforme
NUM_TotalInforme NUM_PaginaEgreso NUM_TotalEgreso GLS_Tribunal COD_Rus
COD_TribunalDist FEC_Desde FEC_Hasta COD_Medida COD_EstBusqueda NUM_Total
FLG_Busqueda COD_TribunalActual GLS_TribunalActual busquedaPorTribunal COD_Mes_sel
COD_Anio_Sel CHK_Contrae rit nombre txtRit txtRut cboTribunal""".split()
)
ACTIONS = frozenset(
    {
        "ShowObservaciones",
        "ShowHistoria",
        "ShowInformesProgramados",
        "ShowExcel",
    }
)
METHODS = frozenset({"GET", "POST", "HEAD", "OPTIONS", "PUT", "PATCH", "DELETE"})
MIMES = frozenset(
    {
        "text/html",
        "text/plain",
        "text/css",
        "text/javascript",
        "application/javascript",
        "application/json",
        "application/xml",
        "text/xml",
        "application/pdf",
        "application/vnd.ms-excel",
        "application/octet-stream",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "image/png",
        "image/jpeg",
        "image/gif",
        "image/svg+xml",
        "image/x-icon",
    }
)


def names_only(names):
    return sorted(
        {name if isinstance(name, str) and name in FIELDS else "CAMPO_NO_RECONOCIDO" for name in names if name}
    )


def safe_location(value):
    try:
        url = urlsplit(value if isinstance(value, str) else "")
        host = url.hostname
    except ValueError:
        return "HOST_NO_RECONOCIDO", "RUTA_NO_RECONOCIDA"
    path = url.path if url.path in PATHS else "RUTA_NO_RECONOCIDA"
    if url.path.startswith("/sitfa/reportes/") and url.path.endswith(".xls"):
        path = "/sitfa/reportes/ARCHIVO.xls"
    return host if host in HOSTS else "HOST_NO_RECONOCIDO", path


def count_only(value, minimum=0, maximum=None):
    if type(value) is int and value >= minimum and (maximum is None or value <= maximum):
        return value
    return None


def field_names(request):
    names = [item.get("name", "") for item in request.get("queryString", [])]
    post = request.get("postData") or {}
    if post.get("params"):
        names += [item.get("name", "") for item in post["params"]]
    elif post.get("text") and "x-www-form-urlencoded" in post.get("mimeType", ""):
        names += [name for name, _ in parse_qsl(post["text"], keep_blank_values=True)]
    return names_only(names)


def onclick_actions(html):
    actions = Counter()
    for name, args in ONCLICK.findall(html):
        parts = [part.strip().strip("'\"") for part in args.split(",")] if args.strip() else []
        known = name in ACTIONS
        window_type = parts[0] if known and parts and re.fullmatch(r"[0-9]{1,2}", parts[0]) else "?"
        name = name if known else "ACCION_NO_RECONOCIDA"
        actions[f"{name}/{len(parts)}/tipo={window_type}"] += 1
    return dict(sorted(actions.items()))


def summarize(har):
    entries = har.get("log", {}).get("entries", [])
    result = []
    for entry in entries:
        request, response = entry.get("request", {}), entry.get("response", {})
        host, path = safe_location(request.get("url", ""))
        content = response.get("content") or {}
        raw_mime = content.get("mimeType")
        mime = raw_mime.split(";")[0] if isinstance(raw_mime, str) else ""
        mime = mime if mime in MIMES else "CONTENIDO_NO_RECONOCIDO"
        method = request.get("method")
        item = {
            "metodo": method if isinstance(method, str) and method in METHODS else "METODO_NO_RECONOCIDO",
            "host": host,
            "ruta": path,
            "campos_enviados": field_names(request),
            "estado_http": count_only(response.get("status"), 100, 599),
            "tipo_contenido": mime,
            "bytes_respuesta": count_only(content.get("size")),
        }
        text = content.get("text") or ""
        if (
            mime == "text/html"
            and isinstance(text, str)
            and text
            and content.get("encoding") != "base64"
            and len(text) <= MAX_BODY
        ):
            item["acciones_onclick"] = onclick_actions(text)
            item["campos_formulario"] = names_only(FIELD.findall(text))
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
