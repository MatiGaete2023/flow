"""Vocabulario de columnas: qué encabezados identifican personas y cuáles lee el motor de reglas."""

from __future__ import annotations

import re

from .textengine import normalize

SENSITIVE = (
    "nombre",
    "apellido",
    "nna",
    "menor",
    "nino",
    "nina",
    "joven",
    "usuario",
    "beneficiario",
    "victima",
    "ruc",
    "madre",
    "padre",
    "litigante",
    "abogado",
    "apoderado",
    "tutor",
    "hermano",
    "familiar",
    "responsable",
    "profesional",
    "dupla",
    "contacto",
    "referente",
    "cuidador",
    "adulto",
    "educador",
    "psicolog",
    "trabajador",
    "asistente",
    "denunciante",
    "imputado",
    "demandante",
    "demandado",
    "solicitante",
    "interviniente",
    "defensor",
    "fiscal",
    "juez",
    "magistrado",
    "consejero",
    "funcionario",
    "digitador",
    "encargado",
    "supervisor",
    "coordinador",
    "autor",
    "paciente",
    "hijo",
    "hija",
    "adolescente",
    "conyuge",
    "pareja",
    "acusado",
    "querellante",
    "sujeto",
    "titular",
    "representante",
    "tia",
    "tio",
    "abuelo",
    "abuela",
    "padrastro",
    "madrastra",
    "pariente",
    "testigo",
    "perito",
    "terapeuta",
    "interventor",
    "delegado",
    "gestor",
    "tramitador",
    "firmante",
    "remitente",
    "destinatario",
    "creado por",
    "modificado por",
    "ingresado por",
)
KEEP = ("nombre centro", "nombre programa", "nombre tribunal", "nombre proyecto", "nombre institucion")
FREE_TEXT = (
    "observacion",
    "obs",
    "glosa",
    "detalle",
    "texto",
    "comentario",
    "nota",
    "motivo",
    "respuesta",
    "descripcion",
    "mensaje",
    "referencia",
    "error",
    "antecedente",
    "resumen",
    "fundamento",
)
# Columnas que lee el motor de reglas y no son identidad: se conservan tal cual (más los alias de RUS/SITFA).
ENGINE_HEADERS = (
    "DERIVACION",
    "PROGRAMA",
    "NOMBRE CENTRO",
    "TRIBUNAL",
    "FEC. NACIMIENTO",
    "FEC.NACIMIENTO",
    "FECHA NACIMIENTO",
    "FEC NACIMIENTO",
    "FEC. OIDO",
    "FEC.OIDO",
    "FEC OIDO",
    "FECHAOIDO",
    "FEC. RESOLUCION",
    "FEC.RESOLUCION",
    "FECHA RESOLUCION",
    "FEC RESOLUCION",
    "PROXS. AUDS.",
    "PROXS AUDS",
    "PROX AUD",
    "PROXIMA AUDIENCIA",
    "PROX. AUD.",
    "PROX. AUDS.",
    "FECHA VENCIMIENTO",
    "FEC.VENCIMIENTO",
    "FEC. VENCIMIENTO",
    "F. VENCIMIENTO",
    "F.VENCIMIENTO",
    "FVENCIMIENTO",
    "FECHA DE VENCIMIENTO",
    "T ESPERA",
    "T_ESPERA",
    "DIAS_ESPERA",
    "TESPERA",
    "DIAS DE ESPERA",
    "DIAS DE CUMPLIMIENTO",
    "DIAS CUMPLIMIENTO",
    "DIAS PARA EGRESAR",
    "DIAS EGRESAR",
    "FEC.INGRESO EFECTIVO",
    "FEC. INGRESO EFECTIVO",
    "FEC INGRESO EFECTIVO",
    "FEC.EGRESO PROYECTADO",
    "FEC. EGRESO PROYECTADO",
    "FEC EGRESO PROYECTADO",
    "FEC.ACT.F.FAE",
    "FEC. ACT. F. FAE",
    "FEC ACT F FAE",
    "FEC.ACT.F.FAE/FAS",
    "FEC. ACT. F. FAE/FAS",
    "FEC.ACT.F.INDIVIDUAL",
    "FEC. ACT. F. INDIVIDUAL",
    "FEC ACT F INDIVIDUAL",
    "MODALIDAD",
    "ESTADO",
    "ETAPA",
    "COBERTURA",
    "TIPO",
    "TIPO OBSERVACION",
    "MATERIA",
    "MEDIDA",
    "REGION",
    "COMUNA",
    "FECHA",
    "FECHA CENTRO",
    "FECHA TRIBUNAL",
)
PROTECTED = {normalize(name) for name in ENGINE_HEADERS}
SAFE_TITLE = re.compile(
    r"(espera|cumplimiento|informes?|egresados?|otros?|original|consolidado|trabajo|resumen|hoja ?\d*|sheet ?\d*"
    r"|informe[ _]?\d+|nurus[ _][\w ]+|cruce|hoja2|vencimientos?|bitacoras?|incidencias?|cobertura|registros?"
    r"|nuevas|ausentes|calendarios?|documentos|consultas|historia[ _]tramites"
    r"|observaciones?(?:[ ](?:espera|cumplimiento|informes?|residencial|ambulatorio|fae|dce|laja|mulchen|tome))*)"
)
SAFE_EXTRA_KEYS = {
    "estado",
    "coverage",
    "cobertura",
    "version",
    "schema_version",
    "schema_id",
    "mapping_version",
    "tipo",
    "type",
    "bytes",
    "rows",
    "table",
    "tables",
    "path",
    "count",
    "total",
    "leidas",
    "fallidas",
    "recoverable",
    "severity",
    "code",
    "phase",
    "scope",
    "origin",
    "status",
    "time_zone",
    "time_precision",
    "screen",
    "tab",
    "source_system",
    "court_code",
    "modality_code",
    "record_type",
    "event_type",
    "identity_status",
    "download_state",
    "content_read",
    "history_complete",
    "institutional_verified",
    "association",
    "expected_pages",
    "saved_pages",
    "period_from",
    "period_to",
    "captured_at",
    "event_at",
    "response_at",
}
STATE_WORDS = {
    "NO_SOLICITADA",
    "NO_CONSULTADA",
    "COMPLETA",
    "VACIA_COMPROBADA",
    "PARCIAL",
    "FALLIDA",
    "CANCELADA",
    "IDENTIDAD_INCIERTA",
    "NO_DISPONIBLE",
    "OK",
    "VALIDADA",
    "VALIDADO",
    "INCOMPLETA",
    "INCOMPLETO",
    "VERIFICADO",
    "VERIFICADA",
    "INCIERTA",
    "LEIDA",
    "EN_CURSO",
    "ERROR",
    "CENTRO",
    "OTRO",
    "DESCONOCIDO",
    "CAUSA",
    "INGRESO",
    "ESPERA",
    "CUMPLIMIENTO",
    "INFORMES",
    "EGRESADOS",
    "DESCARGADO",
    "NO_ASOCIADO",
    "SEGUNDO",
    "DIA",
    "DESCONOCIDA",
}
ID_RE = re.compile(
    r"(\b(?:causa|ingreso|persona|centro|programa|entrada|registro|consulta|evento|documento)\s+(?:rus|sitfa|id)\b"
    r"|(?:^|\s)id(?:\s|$)|_id$|^id_|\bidentificador\b|\bfolio\b|\bcorrelativo\b)"
)
TECHNICAL = {
    "sitfa causa rus": "cause_id",
    "sitfa ingreso rus": "ingreso_id",
    "sitfa persona rus": "person_id",
    "sitfa centro rus": "program_id",
    "sitfa sha256": "source_sha256",
    "sitfa archivo": "original_file",
    "sitfa hoja": "original_sheet",
}
HASH_RE = re.compile(r"(sha ?-?256|huella|\bhash\b|checksum|\bmd5\b)")


def column_kind(header):
    """Clasifica un encabezado: rut, rit, curador, mail, telefono, direccion, persona, texto, ident, hash o None."""
    name = normalize(header)
    if not name:
        return None
    if name in TECHNICAL:
        return "tech"
    if HASH_RE.search(name):
        return "hash"
    if any(name.startswith(keep) for keep in KEEP):
        return None
    compact = name.replace(" ", "")
    if re.match(r"rit(\b|\d)", name) or re.search(r"\brit\b", name):
        return "rit"
    if "curador" in name or re.search(r"\bcur\b", name):
        return "curador"
    if compact in ("rut", "run") or re.match(r"(rut|run)(\b|\d)", name) or re.search(r"\b(rut|run)\b", name):
        return "rut"
    if any(word in name for word in ("correo", "email", "mail")):
        return "mail"
    if any(word in name for word in ("telefono", "fono", "celular")) or re.search(r"\btel\b", name):
        return "telefono"
    if any(word in name for word in ("direccion", "domicilio")):
        return "direccion"
    if is_protected(header):
        return None
    if "json" in name:
        return "json"
    if ID_RE.search(name):
        return "ident"
    if any(re.search(r"\b" + word, name) for word in SENSITIVE):
        return "persona"
    if any(name.startswith(word) for word in FREE_TEXT):
        return "texto"
    return None


def is_protected(header):
    """Columna que lee el motor de reglas (o fecha/tribunal): conserva su texto salvo identificadores."""
    name = normalize(header)
    return name in PROTECTED or name.startswith(("tribunal", "fecha", "fec ", "dias ", "t espera"))


def is_birth(header):
    return "nacimiento" in normalize(header)
