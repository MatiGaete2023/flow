"""Lectura defensiva de ZIP: sin extraer a disco, con límites de expansión y rutas seguras."""

from __future__ import annotations

import stat
import zipfile
import zlib
from io import BytesIO

MAX_MEMBERS = 5000
MAX_TOTAL = 400 * 1024 * 1024
MAX_MEMBER = 200 * 1024 * 1024
MAX_RATIO = 300
MAX_DEPTH = 3


class UnsafeArchive(ValueError):
    """ZIP corrupto, cifrado, con rutas inseguras o que excede los límites."""


def safe_name(name):
    clean = name.replace("\\", "/")
    parts = clean.split("/")
    if not clean or clean.startswith("/") or any(p in ("..",) for p in parts) or ":" in parts[0] or "\x00" in clean:
        raise UnsafeArchive("Ruta interna del ZIP insegura.")
    return "/".join(p for p in parts if p not in ("", "."))


def _open(data):
    try:
        return zipfile.ZipFile(BytesIO(data))
    except zipfile.BadZipFile as exc:
        raise UnsafeArchive("ZIP corrupto o incompatible.") from exc


def _check(archive):
    infos, total, seen = archive.infolist(), 0, set()
    if len(infos) > MAX_MEMBERS:
        raise UnsafeArchive("El ZIP contiene demasiados archivos.")
    for info in infos:
        if info.is_dir():
            continue
        name = safe_name(info.filename)
        mode = info.external_attr >> 16
        if stat.S_ISLNK(mode) or stat.S_IFMT(mode) not in (0, stat.S_IFREG):
            raise UnsafeArchive("El ZIP contiene enlaces simbólicos u objetos especiales.")
        if info.flag_bits & 1:
            raise UnsafeArchive("El ZIP está cifrado.")
        if name.casefold() in seen:
            raise UnsafeArchive("El ZIP repite un nombre de archivo.")
        seen.add(name.casefold())
        total += info.file_size
        if info.file_size > MAX_MEMBER or total > MAX_TOTAL or info.file_size / max(1, info.compress_size) > MAX_RATIO:
            raise UnsafeArchive("La expansión del ZIP excede el límite permitido.")
    return infos


def validate(data):
    """Comprueba límites y rutas sin descomprimir (para libros xlsx/docx antes de abrirlos con otra biblioteca)."""
    with _open(data) as archive:
        _check(archive)


def members(data):
    """Devuelve [(ruta interna, bytes)] validando límites. Ignora carpetas."""
    result = []
    with _open(data) as archive:
        infos = _check(archive)
        for info in infos:
            if info.is_dir():
                continue
            try:
                raw = archive.read(info)
            except (zipfile.BadZipFile, RuntimeError, NotImplementedError, EOFError, zlib.error) as exc:
                raise UnsafeArchive("ZIP corrupto o con compresión no soportada.") from exc
            if len(raw) != info.file_size:
                raise UnsafeArchive("ZIP inconsistente.")
            result.append((safe_name(info.filename), raw))
    return result


def build(entries):
    """[(ruta, bytes)] -> ZIP en memoria con fecha técnica fija."""
    buffer = BytesIO()
    with zipfile.ZipFile(buffer, "w", zipfile.ZIP_DEFLATED) as archive:
        for name, raw in entries:
            info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = (stat.S_IFREG | 0o600) << 16
            archive.writestr(info, raw)
    return buffer.getvalue()
