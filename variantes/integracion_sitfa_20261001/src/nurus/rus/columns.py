from __future__ import annotations

import re
import unicodedata
from collections.abc import Iterable

from .models import Mode


class ColumnMappingError(ValueError):
    def __init__(self, logical_key: str, candidates: tuple[str, ...]) -> None:
        self.logical_key = logical_key
        self.candidates = candidates
        super().__init__(
            f"Columna ambigua para {logical_key}: {', '.join(candidates)}. "
            "Seleccione una columna explícitamente."
        )


def normalize(value: object) -> str:
    text = unicodedata.normalize("NFKD", str(value or "")).encode("ascii", "ignore").decode()
    text = re.sub(r"[-().]+", " ", text.lower().strip())
    return re.sub(r"\s+", " ", text)


COMMON: dict[str, tuple[str, ...]] = {
    "programa": ("DERIVACION", "DERIVACIÓN", "PROGRAMA", "NOMBRE CENTRO"),
    "tribunal": ("TRIBUNAL",),
    "nombre": ("NOMBRE", "NOMBRE COMPLETO", "NOMBRE MENOR"),
    "nacimiento": ("FEC. NACIMIENTO", "FEC.NACIMIENTO", "FECHA NACIMIENTO", "FEC NACIMIENTO"),
    "curador": ("CURADOR", "CURADOR AD LITEM", "CURADOR AD-LITEM", "CURADOR AD LITEM.", "CUR. AD LITEM", "CURADOR/A AD LITEM", "CURADOR/A AD-LITEM"),
    "oido": ("FEC. OIDO", "FEC.OIDO", "FEC OIDO", "FECHAOIDO", "FEC. OÍDO", "FEC.OÍDO"),
    "resolucion": ("FEC. RESOLUCIÓN", "FEC.RESOLUCIÓN", "FEC. RESOLUCION", "FEC.RESOLUCION", "FECHA RESOLUCION", "FEC RESOLUCION"),
    "prox_aud": ("PROXS. AUDS.", "PROXS AUDS", "PROX AUD", "PROXIMA AUDIENCIA", "PRÓXIMA AUDIENCIA", "PROX. AUD.", "PROX. AUDS."),
    "rit": ("RIT",),
    "rut": ("RUT", "RUT MENOR", "RUT NNA", "RUT LITIGANTE"),
}

MODE_COLUMNS: dict[Mode, dict[str, tuple[str, ...]]] = {
    Mode.ESPERA: {"espera": ("T ESPERA", "T_ESPERA", "DIAS_ESPERA", "TESPERA", "DÍAS DE ESPERA", "DIAS DE ESPERA")},
    Mode.CUMPLIMIENTO: {
        "dias_cumpl": ("DIAS DE CUMPLIMIENTO", "DÍAS DE CUMPLIMIENTO", "DIAS CUMPLIMIENTO"),
        "dias_egresar": ("DIAS PARA EGRESAR", "DÍAS PARA EGRESAR", "DIAS EGRESAR"),
        "ingreso": ("FEC.INGRESO EFECTIVO", "FEC. INGRESO EFECTIVO", "FEC INGRESO EFECTIVO"),
        "egreso_proy": ("FEC.EGRESO PROYECTADO", "FEC. EGRESO PROYECTADO", "FEC EGRESO PROYECTADO"),
        "ficha_fae": ("FEC.ACT.F.FAE", "FEC. ACT. F. FAE", "FEC ACT F FAE", "FEC.ACT.F.FAE/FAS", "FEC. ACT. F. FAE/FAS"),
        "ficha_ind": ("FEC.ACT.F.INDIVIDUAL", "FEC. ACT. F. INDIVIDUAL", "FEC ACT F INDIVIDUAL"),
    },
    Mode.INFORMES: {
        "vencimiento": ("FECHA VENCIMIENTO", "FEC.VENCIMIENTO", "FEC. VENCIMIENTO"),
        # FECHA INGRESO y FEC. INGRESO EFECTIVO son datos distintos en los
        # libros RUS. I-01/I-02 no consumen la primera; no deben colisionar.
        "ingreso": ("FEC.INGRESO EFECTIVO", "FEC. INGRESO EFECTIVO", "FEC INGRESO EFECTIVO"),
    },
}

H2_COLUMNS: dict[str, tuple[str, ...]] = {
    "rit": ("RIT",),
    "rut": ("RUT MENOR", "RUT"),
    "nombre": ("NOMBRE MENOR", "NOMBRE"),
    "tribunal": ("TRIBUNAL",),
    "programa": ("NOMBRE CENTRO", "DERIVACION", "DERIVACIÓN"),
    "vencimiento": ("FECHA VENCIMIENTO", "FEC.VENCIMIENTO", "FEC. VENCIMIENTO"),
}


def _map(headers: Iterable[str], aliases: dict[str, tuple[str, ...]]) -> dict[str, str]:
    header_list = [str(header).strip() for header in headers]
    mapping: dict[str, str] = {}
    for key, names in aliases.items():
        normalized_aliases = {normalize(name) for name in names}
        candidates: list[str] = []
        for header in header_list:
            if normalize(header) in normalized_aliases and header not in candidates:
                candidates.append(header)
        if len(candidates) > 1:
            raise ColumnMappingError(key, tuple(candidates))
        if candidates:
            mapping[key] = candidates[0]
    return mapping


def map_columns(headers: Iterable[str], mode: Mode | str) -> dict[str, str]:
    selected = Mode(mode)
    return _map(headers, {**COMMON, **MODE_COLUMNS[selected]})


def map_cross_columns(headers: Iterable[str]) -> dict[str, str]:
    return _map(headers, H2_COLUMNS)


def has_full_cross_mapping(headers: Iterable[str]) -> bool:
    try:
        return set(map_cross_columns(headers)) == set(H2_COLUMNS)
    except ColumnMappingError:
        return False
