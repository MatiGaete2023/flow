from __future__ import annotations

import json
from datetime import date

from nurus.domain.models import Product, ProductStatus, Template
from nurus.rus.rules import format_date
from nurus.services.rendering import prepare
from nurus.storage.database import Database


class ProductBuildError(ValueError):
    pass


def _published_template(db: Database, template_id: str) -> Template:
    matches = [item for item in db.list_templates() if item.id == template_id and item.status == "published"]
    if len(matches) != 1:
        raise ProductBuildError("No existe una plantilla publicada única con ese identificador.")
    return matches[0]


def _single_value(records: list[dict[str, object]], column: str, label: str) -> str:
    values = {
        str(record.get(column, "") or "").strip()
        for record in records
        if str(record.get(column, "") or "").strip()
    }
    if len(values) > 1:
        raise ProductBuildError(
            f"Los registros seleccionados contienen más de un {label}; divide el producto explícitamente."
        )
    return next(iter(values), "")


def prepare_from_snapshot(
    db: Database,
    batch_id: str,
    template_id: str,
    *,
    record_ids: tuple[str, ...] | list[str] | None = None,
    recipient: str = "",
    cc: str = "",
) -> Product:
    """Prepara un producto exclusivamente desde la revisión congelada en SQLite."""
    try:
        snapshot = db.get_snapshot(batch_id)
    except ValueError as exc:
        raise ProductBuildError(str(exc)) from exc
    batch = snapshot["batch"]
    if not batch.get("review_import_hash"):
        raise ProductBuildError(
            "El producto requiere una constancia humana importada desde el Excel revisado."
        )

    template = _published_template(db, template_id)
    rows = snapshot["records"]
    selected_ids = set(record_ids or ())
    if selected_ids:
        known = {row["record_id"] for row in rows}
        missing = selected_ids - known
        if missing:
            raise ProductBuildError("La selección contiene registros que no pertenecen al lote aprobado.")
        rows = [row for row in rows if row["record_id"] in selected_ids]
    rows = [row for row in rows if row["decision"] == "approved"]
    if not rows:
        raise ProductBuildError("No hay registros aprobados seleccionados para preparar el producto.")

    mapping = json.loads(batch["column_mapping"] or "{}")
    values = [json.loads(row["values_json"]) for row in rows]

    tribunal = ""
    programa = ""
    if "TRIBUNAL" in template.allowed_variables:
        column = mapping.get("tribunal", "")
        tribunal = _single_value(values, column, "tribunal") if column else ""
    if "PROGRAMA" in template.allowed_variables:
        column = mapping.get("programa", "")
        programa = _single_value(values, column, "programa") if column else ""

    rit_column = mapping.get("rit", "")
    name_column = mapping.get("nombre", "")
    table_lines: list[str] = []
    observations: list[str] = []
    for row, value in zip(rows, values):
        rit = str(value.get(rit_column, "") or "").strip() if rit_column else ""
        name = str(value.get(name_column, "") or "").strip() if name_column else ""
        observation = str(row["edited_observation"] or "").strip()
        observations.append(observation)
        identity = " · ".join(part for part in (rit, name) if part) or f"fila {row['source_row']}"
        table_lines.append(f"{identity}: {observation}" if observation else identity)

    as_of = str(batch["as_of"] or "")
    try:
        formatted_as_of = format_date(date.fromisoformat(as_of))
    except ValueError:
        formatted_as_of = as_of

    context = {
        "TRIBUNAL": tribunal,
        "PROGRAMA": programa,
        "FECHA_CORTE": formatted_as_of,
        "OBSERVACION": "\n".join(item for item in observations if item),
        "TABLA_REGISTROS": "\n".join(table_lines),
    }
    return prepare(
        Product(
            kind=template.kind,
            template=template,
            context=context,
            recipient=recipient.strip(),
            cc=cc.strip(),
            batch_id=batch_id,
            source_snapshot_hash=batch["snapshot_hash"],
            record_ids=tuple(row["record_id"] for row in rows),
        )
    )


def approve_product(
    product: Product,
    *,
    subject: str | None = None,
    body: str | None = None,
) -> Product:
    """Registra la aprobación humana del texto final, sin producir efectos externos."""
    if product.status is ProductStatus.BLOCKED or product.issues:
        raise ProductBuildError("El producto contiene errores y no puede aprobarse.")
    final_subject = product.rendered_subject if subject is None else subject.strip()
    final_body = product.rendered_body if body is None else body.strip()
    if not final_body:
        raise ProductBuildError("El contenido final no puede quedar vacío.")
    product.rendered_subject = final_subject
    product.rendered_body = final_body
    product.status = ProductStatus.APPROVED
    return product


def persist_approved_product(db: Database, product: Product) -> None:
    if product.status is not ProductStatus.APPROVED:
        raise ProductBuildError("El producto debe aprobarse antes de guardarlo como definitivo.")
    db.record_product(product)
