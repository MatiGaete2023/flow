from __future__ import annotations

import re

from nurus.domain.models import Product, Template
from nurus.services.policy import with_mandatory_cc

TOKEN = re.compile(r"\{([A-Z_]+)\}")
_PLACEHOLDER_VALUES = {"por completar", "sin registros cargados"}


def _invalid_brace_syntax(text: str) -> bool:
    cleaned = TOKEN.sub("", text)
    return "{" in cleaned or "}" in cleaned


def render(template: Template, context: dict[str, str]) -> tuple[str, str, list[str]]:
    issues: list[str] = []
    permitted = set(template.allowed_variables)

    if _invalid_brace_syntax(template.subject) or _invalid_brace_syntax(template.body):
        issues.append("Sintaxis de variable inválida en la plantilla.")

    def replace(match: re.Match[str]) -> str:
        name = match.group(1)
        if name not in permitted:
            issues.append(f"Variable no permitida: {name}")
            return match.group(0)
        value = str(context.get(name, "")).strip()
        if not value or value.casefold() in _PLACEHOLDER_VALUES:
            issues.append(f"Falta dato para: {name}")
            return match.group(0)
        return value

    subject = TOKEN.sub(replace, template.subject)
    body = TOKEN.sub(replace, template.body)
    return subject, body, list(dict.fromkeys(issues))


def prepare(product: Product) -> Product:
    if product.kind.value == "email":
        product.cc = with_mandatory_cc(product.cc)
    subject, body, issues = render(product.template, product.context)
    product.rendered_subject, product.rendered_body = subject, body
    product.issues = issues
    product.warnings = []
    if product.kind.value == "email" and not product.recipient.strip():
        product.warnings.append(
            "Sin destinatario: el borrador quedará disponible para completarlo manualmente."
        )
    product.mark_ready()
    return product

