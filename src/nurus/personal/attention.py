"""Estado operativo por caso y bandeja de atención; solo lectura."""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

from .capture_changes import label as change_label
from .outputs import draft_fingerprint_for_display, draft_preview_scope, due_value
from .product_state import stale
from .ux_support import _resolution_codes


@dataclass(frozen=True)
class CaseStatus:
    row_id: str
    estado: str
    motivo: str
    proxima_accion: str
    fuente: str = ''
    capturado: str = ''
    vence: str = ''
    cambio: str = ''


def _get(product, key, default=None):
    return product.get(key, default) if isinstance(product, dict) else getattr(product, key, default)


def _ids(product):
    return list(_get(product, 'record_ids', ()) or ())


def _date(value):
    if not value:
        return ''
    text = str(value)
    try:
        return datetime.fromisoformat(text.replace('Z', '+00:00')).strftime('%d/%m/%Y')
    except ValueError:
        try:
            return datetime.strptime(text, '%Y-%m-%d').strftime('%d/%m/%Y')
        except ValueError:
            return text


def _source(work, row):
    source = getattr(work, 'capture_sources', {}).get(row.id, {})
    name = source.get('source_system') or Path(getattr(work, 'path', '') or '').name
    captured = _date(source.get('captured_at') or getattr(work, 'as_of', ''))
    return str(name or 'Trabajo local'), captured


def _receipt_created(receipts, key):
    return bool(key and receipts.get(key, {}).get('state') == 'created')


def _batch_products(work, drafts, projects):
    receipts = getattr(work, 'receipts', {}) or {}
    mail_prepared, mail_created, due = {}, {}, {}
    uncertain = set()
    with draft_preview_scope():
        for draft in drafts or ():
            ids = _ids(draft)
            try:
                key = draft_fingerprint_for_display(draft)
            except (OSError, ValueError):
                key = ''
            created = _receipt_created(receipts, key)
            for rid in ids:
                mail_prepared[rid] = True
                mail_created[rid] = mail_created.get(rid, False) or created
                if rid not in due:
                    due[rid] = (
                        due_value(work, next((r for r in work.rows if r.id == rid), None), _get(draft, 'kind', ''))
                        if next((r for r in work.rows if r.id == rid), None)
                        else ''
                    )
        words = {rid: [] for rid in (row.id for row in work.rows)}
        for project in projects or ():
            for rid in _ids(project):
                words.setdefault(rid, []).append(project)
    for receipt in receipts.values():
        if receipt.get('kind') != 'draft' or receipt.get('state') != 'uncertain':
            continue
        uncertain.update(receipt.get('record_ids') or ([receipt.get('record_id')] if receipt.get('record_id') else []))
    return mail_prepared, mail_created, words, uncertain, due


def case_statuses(work, drafts, projects):
    mail_prepared, mail_created, words, uncertain, due = _batch_products(work, drafts, projects)
    receipts = getattr(work, 'receipts', {}) or {}
    changes = getattr(work, 'capture_changes', None)
    result = {}
    for row in getattr(work, 'rows', ()):
        word_products = words.get(row.id, [])
        products = drafts or []
        mail_products = [p for p in products if row.id in _ids(p)]
        obsolete = any(stale(work, product) for product in mail_products + word_products)
        codes = _resolution_codes(row)
        need_mail = bool(getattr(row, 'actions', ()) or mail_products)
        need_word = bool(codes)
        copy_ready = bool(getattr(work, 'output', '') and Path(work.output).is_file())
        source, captured = _source(work, row)
        change = change_label(changes, row.id)
        reason = str((getattr(row, 'warnings', ()) or [])[0]) if getattr(row, 'warnings', None) else ''
        if not reason and getattr(row, 'rules', None):
            reason = str(row.rules[0]).split('.', 1)[-1]
        reason = reason or change
        if row.excluded:
            state, action = 'EXCLUIDO', 'Sin acción: excluido'
        elif row.id in uncertain:
            state, action = 'INCIERTO', 'Revisa Borradores en Outlook antes de repetir'
        elif obsolete:
            state, action = 'OBSOLETO', 'Prepara nuevamente: cambiaron los datos'
        elif getattr(row, 'warnings', None):
            state, action = 'REQUIERE_REVISION', 'Revisa el aviso: ' + str(row.warnings[0])
        elif (getattr(row, 'actions', None) or codes) and (
            not copy_ready or (need_mail and not mail_prepared.get(row.id)) or (need_word and not word_products)
        ):
            state = 'POR_PREPARAR'
            action = (
                'Exporta la copia'
                if not copy_ready
                else 'Prepara correo'
                if need_mail and not mail_prepared.get(row.id)
                else 'Prepara Word'
            )
        elif (getattr(row, 'actions', None) or codes) and (
            (need_mail and mail_prepared.get(row.id) and not mail_created.get(row.id))
            or (need_word and word_products and not any(_word_receipt(receipts, row.id) for _ in word_products))
        ):
            state = 'PREPARADO'
            action = 'Guarda el borrador' if need_mail and not mail_created.get(row.id) else 'Genera el Word'
        elif (getattr(row, 'actions', None) or codes) and (
            (not need_mail or mail_created.get(row.id)) and (not need_word or _word_receipt(receipts, row.id))
        ):
            state, action = 'BORRADOR_GUARDADO', 'Pendiente de actuación: envío o firma (decisión humana)'
        else:
            state, action = 'SIN_ACCION', 'Sin gestión propuesta'
        result[row.id] = CaseStatus(row.id, state, reason, action, source, captured, due.get(row.id, ''), change)
    return result


def _word_receipt(receipts, row_id):
    return any(
        receipt.get('kind') == 'word'
        and row_id in (receipt.get('record_ids') or [receipt.get('record_id')])
        and receipt.get('state', 'created') == 'created'
        for receipt in receipts.values()
    )


def attention_items(work, drafts, projects):
    statuses = case_statuses(work, drafts, projects)
    ranks = {'INCIERTO': 0, 'OBSOLETO': 1, 'REQUIERE_REVISION': 2, 'POR_PREPARAR': 3, 'PREPARADO': 4}

    def key(item):
        try:
            due = datetime.strptime(item.vence, '%d/%m/%Y') if item.vence else datetime.max
        except ValueError:
            due = datetime.max
        return (ranks[item.estado], due, 0 if item.cambio else 1, item.row_id)

    return sorted((item for item in statuses.values() if item.estado in ranks), key=key)
