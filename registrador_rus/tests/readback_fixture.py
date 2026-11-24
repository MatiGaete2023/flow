from datetime import datetime, timezone
from registrador_rus.app.readback import METHOD


def snapshot(op, when, *, rows=(), partial=False):
    i = op.remote_identity
    ident = dict(tribunal_codigo=i.tribunal_id, causa_id=i.causa_id, ingreso_id=i.ingreso_id)
    return dict(ident, captured_at=when.isoformat(), stage_code='1',
                entries=[dict(ident, **r) for r in rows],
                row_metadata=[dict(entry_id=r['entry_id'], state=op.observation_state) for r in rows],
                alcance_metodo=METHOD, alcance='PARCIAL' if partial else ('COMPLETA' if rows else 'VACIA_COMPROBADA'))


def bind(j, op, when=None, **kwargs):
    when = when or datetime.fromisoformat(j._now())
    return j.bind_baseline(op, snapshot(op, when), author='demo', stage='Etapa ficticia',
                           stage_code='1', server_day=when.astimezone(timezone.utc).date(), **kwargs)
