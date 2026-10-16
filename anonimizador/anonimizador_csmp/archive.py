"""ZIP genéricos (entregas del Descargador y del Asistente, carpetas comprimidas) y paquetes de captura."""

from __future__ import annotations

import json
import posixpath
from hashlib import sha256

from . import capture, jsonfmt, safezip
from .textengine import fold, scrub_unknown

MANIFEST_NAMES = {"manifiesto.json"}
INDEX_NAMES = {"indice.txt"}


def _clean(segment, session):
    probe = segment.replace("_", " ")
    return session.replacer.text(probe, free=True) == probe and scrub_unknown(probe) == probe


def neutral_path(path, index, ext, session, dirs):
    parts = path.split("/")
    out = []
    for part in parts[:-1]:
        if _clean(part, session):
            out.append(part)
        else:
            out.append(dirs.setdefault(part, f"carpeta_{len(dirs) + 1}"))
    stem = posixpath.splitext(parts[-1])[0]
    out.append((stem if _clean(stem, session) and stem else f"archivo_{index}") + ext)
    return "/".join(out)


def collect(data, session, collect_bytes, depth):
    entries = safezip.members(data)
    if capture.is_capture(entries):
        capture.collect(entries, session, lambda raw, name: collect_bytes(raw, name, session, depth + 1))
        return
    for name, raw in entries:
        try:
            collect_bytes(raw, name, session, depth + 1)
        except Exception:
            continue


def process(data, session, process_bytes, depth):
    """ZIP -> (ZIP nuevo, textos, tipo). `process_bytes(raw, nombre, depth)` devuelve Output o lanza Rejected."""
    if depth > safezip.MAX_DEPTH:
        raise safezip.UnsafeArchive("ZIP anidado demasiado profundo.")
    entries = safezip.members(data)
    if capture.is_capture(entries):
        result, texts = capture.transform(
            entries, session, lambda raw, name: _inner(process_bytes, raw, name, depth + 1)
        )
        return result, texts, "captura"
    specials, regular = {}, []
    for name, raw in entries:
        base = posixpath.basename(name).lower()
        if base in MANIFEST_NAMES | INDEX_NAMES:
            specials[base] = (name, raw)
        else:
            regular.append((name, raw))
    # Nombres nuevos antes de transformar: así resumen.json y manifiestos citan los mismos nombres.
    dirs, plan = {}, []
    for index, (name, raw) in enumerate(regular, 1):
        ext = posixpath.splitext(name)[1].lower() or ""
        plan.append((name, raw, index, ext))
    for name, raw, index, ext in plan:
        new = neutral_path(name, index, ext, session, dirs)
        session.name_map.setdefault(posixpath.basename(name), posixpath.basename(new))
    out, texts, skipped, used = [], [], [], set()
    for name, raw, index, ext in plan:
        try:
            result = process_bytes(raw, name, depth + 1)
        except Exception as exc:
            skipped.append(getattr(exc, "label", "no admitido"))
            continue
        new = neutral_path(name, index, result.ext if result.ext else ext, session, dirs)
        while new.casefold() in used:
            new = posixpath.splitext(new)[0] + f"_{index}" + posixpath.splitext(new)[1]
        used.add(new.casefold())
        session.name_map[posixpath.basename(name)] = posixpath.basename(new)
        out.append((name, new, result))
        texts.extend(result.texts)
    if not out:
        raise safezip.UnsafeArchive("El ZIP no contiene archivos que se puedan anonimizar.")
    if skipped and not session.options.skip_unsupported:
        raise safezip.UnsafeArchive("El ZIP contiene archivos no admitidos y se pidió rechazarlo completo.")
    if skipped:
        session.report.warn(
            f"{len(skipped)} archivo(s) dentro del ZIP se excluyeron (no admitidos o con datos personales no "
            "depurables); no se copiaron sin tratar."
        )
    members = [(new, result.data) for _, new, result in out]
    members += _specials(specials, out, session)
    return safezip.build(members), texts, "zip"


def _inner(process_bytes, raw, name, depth):
    result = process_bytes(raw, name, depth)
    return result.data, result.ext, result.kind


def _specials(specials, out, session):
    by_full = {name: (new, result) for name, new, result in out}
    by_base = {}
    for name, new, result in out:
        by_base.setdefault(posixpath.basename(name), []).append((new, result))
    members = []
    state = "VERIFICADO"
    if "manifiesto.json" in specials:
        name, raw = specials["manifiesto.json"]
        try:
            manifest = json.loads(raw.decode("utf-8-sig"))
            items = manifest["archivos"]
            assert isinstance(items, list)
        except (ValueError, KeyError, UnicodeError, AssertionError, TypeError):
            manifest = None
        if manifest is not None:
            state = str(manifest.get("estado", state))
            result = {k: v for k, v in manifest.items() if k not in ("archivos",)}
            result = {
                k: jsonfmt.value(k, v, session) if not isinstance(v, (dict, list)) else v for k, v in result.items()
            }
            files = []
            for item in items:
                if not isinstance(item, dict):
                    continue
                key = str(item.get("archivo", ""))
                hit = by_full.get(key) or (by_base.get(posixpath.basename(key)) or [None])[0]
                if hit is None:
                    continue
                new, converted = hit
                entry = {
                    "archivo": new,
                    "nombre": posixpath.basename(new),
                    "bytes": len(converted.data),
                    "sha256": sha256(converted.data).hexdigest(),
                }
                for extra in ("tipo", "estado", "registros"):
                    if extra in item:
                        entry[extra] = item[extra] if isinstance(item[extra], (int, float)) else str(item[extra])[:40]
                files.append(entry)
            result["archivos"] = files
            members.append((name, (json.dumps(result, ensure_ascii=False, indent=2) + "\n").encode("utf-8")))
        else:
            members.append((name, b"{}\n"))
    if "indice.txt" in specials:
        name, raw = specials["indice.txt"]
        first = raw.decode("utf-8", "replace").splitlines()[:1]
        if first and fold(first[0]).replace(" ", "") in ("incompleto", "verificado"):
            state = first[0].strip()
        lines = [state, "Copia anonimizada. Consultar MANIFIESTO.json para comprobar integridad.", ""]
        lines += sorted(new for _, new, _ in out)
        members.append((name, ("\n".join(lines) + "\n").encode("utf-8")))
    return members
