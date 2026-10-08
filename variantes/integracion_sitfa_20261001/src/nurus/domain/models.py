from __future__ import annotations

from dataclasses import dataclass, field
from datetime import UTC, datetime
from enum import StrEnum
from uuid import uuid4


class ProductKind(StrEnum):
    EMAIL = "email"
    RESOLUTION = "resolution"
    EXPORT = "export"


class ProductStatus(StrEnum):
    PENDING = "pending"
    READY = "ready"
    BLOCKED = "blocked"
    APPROVED = "approved"
    CREATED = "created"
    UNCERTAIN = "uncertain"


@dataclass(frozen=True)
class Template:
    id: str
    name: str
    kind: ProductKind
    subject: str
    body: str
    allowed_variables: tuple[str, ...]
    status: str = "draft"
    version: int = 1


@dataclass
class Product:
    kind: ProductKind
    template: Template
    context: dict[str, str]
    recipient: str = ""
    cc: str = ""
    batch_id: str = ""
    record_ids: tuple[str, ...] = ()
    attachments: list[tuple[str, str]] = field(default_factory=list)
    required_attachment: bool = False
    id: str = field(default_factory=lambda: str(uuid4()))
    status: ProductStatus = ProductStatus.PENDING
    rendered_subject: str = ""
    rendered_body: str = ""
    issues: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)
    source_snapshot_hash: str = ""

    def mark_ready(self) -> None:
        """Solo los errores de contenido bloquean el producto.

        Un correo sin destinatario puede seguir listo para guardarse como borrador,
        porque NuRus nunca envía correos y el destinatario puede completarse durante
        la revisión humana en Outlook.
        """
        self.status = ProductStatus.READY if not self.issues else ProductStatus.BLOCKED


def utc_now() -> str:
    return datetime.now(UTC).isoformat(timespec="seconds")
