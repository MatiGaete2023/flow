"""Reglas transversales de correo aprobadas por el usuario."""
import re

MANDATORY_CC = "ucc_concepcion@pjud.cl"


def with_mandatory_cc(value: str) -> str:
    addresses = []
    seen = set()
    for part in [*re.split(r"[;,]", value or ""), MANDATORY_CC]:
        address = part.strip()
        if address and address.casefold() not in seen:
            addresses.append(address)
            seen.add(address.casefold())
    return "; ".join(addresses)

