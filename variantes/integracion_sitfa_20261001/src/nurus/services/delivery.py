"""Materialización de borradores con recibo e idempotencia conservadora."""
from dataclasses import asdict
import json

from nurus.adapters.outlook import DraftSaveUncertain, save_draft
from nurus.domain.models import ProductStatus, utc_now


def save_approved_draft(db, product, *, confirmed=False, account_key=None, folder_path=()):
    if not confirmed or product.status is not ProductStatus.APPROVED:
        raise ValueError("Aprueba el contenido y confirma la creación del borrador.")
    with db.connect() as conn:
        conn.execute("BEGIN IMMEDIATE")
        stored = conn.execute(
            """SELECT p.*,b.status AS batch_status,b.snapshot_hash AS current_snapshot_hash
               FROM products p JOIN batches b ON b.id=p.batch_id WHERE p.id=?""",
            (product.id,),
        ).fetchone()
        expected_attachments = [list(item) for item in product.attachments]
        if (
            stored is None or stored["status"] != "approved"
            or stored["subject"] != product.rendered_subject
            or stored["body"] != product.rendered_body
            or stored["recipient"] != product.recipient
            or stored["cc"] != product.cc
            or json.loads(stored["attachments"]) != expected_attachments
        ):
            raise ValueError("El contenido difiere del producto aprobado. Crea una nueva versión.")
        if (
            stored["batch_status"] != "approved"
            or stored["source_snapshot_hash"] != stored["current_snapshot_hash"]
        ):
            raise ValueError(
                "La constancia cambió desde la aprobación del producto. Prepara una nueva versión."
            )
        prior = conn.execute(
            "SELECT status FROM deliveries WHERE product_id=?", (product.id,)
        ).fetchone()
        if prior and prior[0] != "failed":
            raise ValueError("Existe un intento previo; revisa Outlook antes de volver a crear el borrador.")
        conn.execute(
            """INSERT INTO deliveries VALUES(?,?,?,?)
               ON CONFLICT(product_id) DO UPDATE SET
               status=excluded.status,receipt_json=excluded.receipt_json,updated_at=excluded.updated_at""",
            (product.id, "attempting", "{}", utc_now()),
        )
    try:
        receipt = save_draft(
            product, confirmed=True, account_key=account_key, folder_path=folder_path
        )
    except Exception as exc:
        state = "uncertain" if isinstance(exc, DraftSaveUncertain) else "failed"
        with db.connect() as conn:
            conn.execute(
                "UPDATE deliveries SET status=?,receipt_json=?,updated_at=? WHERE product_id=?",
                (state, json.dumps({"error": str(exc)}, ensure_ascii=False), utc_now(), product.id),
            )
        raise
    with db.connect() as conn:
        conn.execute(
            "UPDATE deliveries SET status='created',receipt_json=?,updated_at=? WHERE product_id=?",
            (json.dumps(asdict(receipt), ensure_ascii=False), utc_now(), product.id),
        )
        conn.execute("UPDATE products SET status='created' WHERE id=?", (product.id,))
    product.status = ProductStatus.CREATED
    return receipt
