"""Metadatos de productos del trabajo actual, sin inferir envíos ni fechas pasadas."""
from datetime import datetime


def receipt(kind, state, *, record_ids=(), **details):
    return dict(kind=kind, state=state, created_at=datetime.now().astimezone().isoformat(timespec='seconds'),
                record_ids=list(record_ids), **details)
