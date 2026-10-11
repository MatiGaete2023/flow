from __future__ import annotations

from copy import deepcopy
from dataclasses import dataclass, field
from enum import StrEnum
from hashlib import sha256
from types import MappingProxyType
from typing import Mapping


class Mode(StrEnum):
    ESPERA = "ESPERA"
    CUMPLIMIENTO = "CUMPLIMIENTO"
    INFORMES = "INFORMES"


def _freeze(value: object) -> object:
    if isinstance(value, Mapping):
        return MappingProxyType({str(key): _freeze(item) for key, item in value.items()})
    if isinstance(value, list):
        return tuple(_freeze(item) for item in value)
    if isinstance(value, tuple):
        return tuple(_freeze(item) for item in value)
    if isinstance(value, set):
        return frozenset(_freeze(item) for item in value)
    return deepcopy(value)


@dataclass(frozen=True)
class SourceReference:
    """Ubicación verificable de una fila en el libro que la originó."""

    workbook_name: str
    workbook_sha256: str
    sheet_name: str
    row_number: int


@dataclass(frozen=True)
class SourceRecord:
    values: Mapping[str, object]
    source: SourceReference
    record_id: str = ""

    def __post_init__(self) -> None:
        object.__setattr__(self, "values", _freeze(dict(self.values)))
        if not self.record_id:
            raw = f"{self.source.workbook_sha256}|{self.source.sheet_name}|{self.source.row_number}"
            object.__setattr__(self, "record_id", sha256(raw.encode("utf-8")).hexdigest()[:24])


@dataclass(frozen=True)
class ReadBatch:
    mode: Mode
    records: tuple[SourceRecord, ...]
    cross_records: tuple[SourceRecord, ...] = ()
    warnings: tuple[str, ...] = ()
    source_path: str = ""
    workbook_name: str = ""
    workbook_sha256: str = ""
    primary_sheet: str = ""
    cross_sheet: str = ""
    header_row: int = 1
    excel_epoch: str = "1900"
    column_mapping: Mapping[str, str] = field(default_factory=dict)
    cross_mapping: Mapping[str, str] = field(default_factory=dict)
    source_bytes: bytes = field(default=b"", repr=False, compare=False)

    def __post_init__(self) -> None:
        object.__setattr__(self, "column_mapping", _freeze(dict(self.column_mapping)))
        object.__setattr__(self, "cross_mapping", _freeze(dict(self.cross_mapping)))
