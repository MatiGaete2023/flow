"""Texto plano, calendarios .ics, bloques de HTML y Word (.docx)."""

from __future__ import annotations

import re
from datetime import date, timedelta
from html import escape, unescape

from .formats import decode_text
from .grid import identity_value, label_kind, structural
from .textengine import curator_name, shift_dates

KV_RE = re.compile(r"^(\s*(?:[-*•]\s*)?)([^:=\n]{2,40}?)(\s*[:=]\s*)(.*)$")
ICS_KEEP = {
    "BEGIN",
    "END",
    "VERSION",
    "PRODID",
    "CALSCALE",
    "METHOD",
    "DTSTART",
    "DTEND",
    "DTSTAMP",
    "CREATED",
    "LAST-MODIFIED",
    "TRANSP",
    "STATUS",
    "SEQUENCE",
    "CLASS",
    "X-WR-TIMEZONE",
    "TZID",
    "DURATION",
    "RRULE",
}
ICS_FREE = {"SUMMARY", "DESCRIPTION", "LOCATION", "COMMENT", "X-WR-CALNAME", "X-WR-CALDESC", "CATEGORIES"}


def seed_line(text, session):
    session.reserve_rits_in(text)
    match = KV_RE.match(text)
    if match and (kind := label_kind(match.group(2) + ":")) in ("persona", "curador") and match.group(4).strip():
        value = match.group(4)
        session.replacer.add_person(curator_name(value) if kind == "curador" else value)


def line(text, session):
    """Una línea o bloque: identidades etiquetadas, estructura segura o texto libre según el modo."""
    if not text.strip():
        return text
    match = KV_RE.match(text)
    if match and match.group(4).strip():
        kind = label_kind(match.group(2) + ":")
        if kind:
            return match.group(1) + match.group(2) + match.group(3) + str(identity_value(kind, match.group(4), session))
    if structural(text):
        return shift_dates(text, session.options.shift_days)
    return session.free(text)


def seed_text(data, session):
    for raw in decode_text(data).splitlines():
        seed_line(raw, session)


def anonymize_text(data, session):
    out, texts = [], []
    for raw in decode_text(data).splitlines():
        new = line(raw, session)
        out.append(new)
        texts.append((new, False))
    return ("\n".join(out) + "\n").encode("utf-8"), texts


def unfold(text):
    return re.sub(r"\r?\n[ \t]", "", text).splitlines()


def seed_ics(data, session):
    for raw in unfold(decode_text(data)):
        session.reserve_rits_in(raw)
        name, _, value = raw.partition(":")
        if name.split(";")[0].upper() in ICS_FREE:
            seed_line(value.replace("\\,", ",").replace("\\n", "\n"), session)


ICS_BASIC = re.compile(r"(?<!\d)(\d{4})(\d{2})(\d{2})(?=T\d{6}Z?(?!\d)|(?!\d))")


def shift_ics(value, days):
    """Fechas RFC 5545 básicas (20261021, 20261021T150000Z) y las escritas con guiones o barras."""
    if not days:
        return value

    def basic(match):
        try:
            moved = date(int(match.group(1)), int(match.group(2)), int(match.group(3))) + timedelta(days=days)
        except (ValueError, OverflowError):
            return match.group(0)
        return f"{moved.year:04d}{moved.month:02d}{moved.day:02d}"

    return ICS_BASIC.sub(basic, shift_dates(value, days))


def anonymize_ics(data, session):
    out, texts, serial = [], [], 0
    for raw in unfold(decode_text(data)):
        name, sep, value = raw.partition(":")
        prop = name.split(";")[0].upper()
        if not sep:
            continue
        if prop in ICS_KEEP:
            new_value = shift_ics(value, session.options.shift_days) if prop.startswith("DT") else value
            new = f"{name}:{new_value}"
        elif prop == "UID":
            serial += 1
            new = f"{name}:{session.token('uid', value)}@anonimizado"
        elif prop in ICS_FREE:
            new_value = session.free(value.replace("\\,", ",").replace("\\n", "\n").replace("\\;", ";"))
            new = f"{name}:{new_value.replace(',', chr(92) + ',').replace(chr(10), chr(92) + 'n')}"
        else:
            new = f"{prop}:{'[omitido]'}"
        out.append(new)
        texts.append((new, False))
    return ("\r\n".join(out) + "\r\n").encode("utf-8"), texts


W_T = re.compile(r"(?P<popen><w:p\b(?:[^>/]|/(?!>))*>)|(?P<pclose></w:p>)|(?P<t><w:t\b[^>]*?>)(?P<text>[^<]*)</w:t>")
DOCX_PARTS = re.compile(r"^word/(?:glossary/)?(document|header\d*|footer\d*|footnotes|endnotes|comments)\.xml$")
# Autores en comentarios, revisiones y people.xml (w:, w15: u otro prefijo).
DOCX_AUTHORS = re.compile(r'(\b\w+:(?:author|initials|userId))="[^"]*"')
CUSTOM_VALUES = re.compile(r"(<vt:(?:lpwstr|lpstr|bstr)\b[^>]*>)[^<]*(</vt:(?:lpwstr|lpstr|bstr)>)")


def _docx_paragraphs(xml):
    stack, groups, counter = [], {}, 0
    for match in W_T.finditer(xml):
        if match.group("popen"):
            counter += 1
            stack.append(counter)
        elif match.group("pclose"):
            if stack:
                stack.pop()
        else:
            groups.setdefault(stack[-1] if stack else 0, []).append(match)
    return groups


def docx_texts(data):
    """Texto por párrafo de un .docx (para sembrar nombres y revisar)."""
    from .safezip import members

    result = []
    for name, raw in members(data):
        if DOCX_PARTS.match(name):
            xml = raw.decode("utf-8", "replace")
            for group in _docx_paragraphs(xml).values():
                result.append(unescape("".join(m.group("text") for m in group)))
    return result


def anonymize_docx(data, session):
    from . import safezip
    from .formats import Unsupported

    entries, texts = [], []
    for name, raw in safezip.members(data):
        low = name.lower()
        if low.startswith(("word/media/", "word/embeddings/", "word/activex/")) or "vbaproject" in low:
            raise Unsupported("El Word contiene imágenes u objetos incrustados; no se puede anonimizar con garantías.")
        if low.startswith(("customxml/", "docprops/thumbnail")):
            continue
        if DOCX_PARTS.match(name):
            xml = raw.decode("utf-8", "replace")
            edits = []
            for group in _docx_paragraphs(xml).values():
                original = unescape("".join(m.group("text") for m in group))
                new = line(original, session) if original.strip() else original
                texts.append((new, False))
                first = True
                for match in group:
                    edits.append(
                        (match.start(), match.end(), match.group("t"), escape(new, quote=False) if first else "")
                    )
                    first = False
            for start, end, tag, text in sorted(edits, reverse=True):
                if "xml:space" not in tag:
                    tag = tag[:-1] + ' xml:space="preserve">'
                xml = xml[:start] + tag + text + "</w:t>" + xml[end:]
            xml = DOCX_AUTHORS.sub(r'\1="anonimizado"', xml)
            xml = re.sub(r"<w:instrText\b[^>]*>[^<]*</w:instrText>", "", xml)
            raw = xml.encode("utf-8")
        elif low == "docprops/core.xml":
            xml = raw.decode("utf-8", "replace")
            for tag in ("dc:creator", "cp:lastModifiedBy", "dc:title", "dc:subject", "dc:description", "cp:keywords"):
                xml = re.sub(rf"(<{tag}\b[^>]*>)[^<]*(</{tag}>)", r"\1anonimizado\2", xml)
            raw = xml.encode("utf-8")
        elif low == "docprops/app.xml":
            xml = raw.decode("utf-8", "replace")
            for tag in ("Company", "Manager", "HyperlinkBase"):
                xml = re.sub(rf"(<{tag}\b[^>]*>)[^<]*(</{tag}>)", r"\1\2", xml)
            raw = xml.encode("utf-8")
        elif low.startswith("word/") and low.endswith(".xml"):
            xml = DOCX_AUTHORS.sub(r'\1="anonimizado"', raw.decode("utf-8", "replace"))
            raw = xml.encode("utf-8")
        elif low == "docprops/custom.xml":
            raw = CUSTOM_VALUES.sub(r"\1anonimizado\2", raw.decode("utf-8", "replace")).encode("utf-8")
        elif low.endswith(".rels"):
            xml = raw.decode("utf-8", "replace")
            xml = re.sub(r'Target="[^"]*"(\s+TargetMode="External")', r'Target="about:blank"\1', xml)
            xml = re.sub(r'(TargetMode="External"\s+)Target="[^"]*"', r'\1Target="about:blank"', xml)
            raw = xml.encode("utf-8")
        entries.append((name, raw))
    return safezip.build(entries), texts
