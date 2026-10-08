from __future__ import annotations

import json
from dataclasses import dataclass
from datetime import date
from pathlib import Path

from nurus.rus import Mode, evaluate_batch, read_workbook
from nurus.rus.models import EvaluationBatch, EvaluationStatus
from nurus.storage.database import Database


@dataclass(frozen=True)
class ReviewRow:
    record_id: str
    status: str
    decision: str
    rit: str
    tribunal: str
    programa: str
    nombre: str
    observation: str
    original_observation: str
    source_sheet: str
    source_row: int
    rule_ids: tuple[str, ...]
    issues: tuple[str, ...]


class WorkController:
    """Orquesta lectura, evaluación, persistencia y revisión sin efectos externos."""

    def __init__(self, db: Database) -> None:
        self.db = db

    def analyze(
        self,
        path: str | Path,
        mode: Mode | str,
        *,
        as_of: date | None = None,
        sheet_name: str | None = None,
        cross_sheet_name: str | None = None,
        header_row: int = 1,
        max_file_size_bytes: int | None = None,
    ) -> EvaluationBatch:
        read_batch = read_workbook(
            path,
            mode,
            sheet_name=sheet_name,
            cross_sheet_name=cross_sheet_name,
            header_row=header_row,
            max_file_size_bytes=max_file_size_bytes,
        )
        result = evaluate_batch(read_batch, as_of=as_of)
        self.db.save_evaluation_batch(result, source_bytes=read_batch.source_bytes)
        return result

    @staticmethod
    def rows_from_batch(batch: EvaluationBatch) -> tuple[ReviewRow, ...]:
        mapping = dict(batch.column_mapping)
        rows: list[ReviewRow] = []
        for item in batch.evaluations:
            values = dict(item.values)
            issues = tuple(f"{issue.code}: {issue.message}" for issue in item.issues)
            decision = (
                "excluded" if item.status is EvaluationStatus.EXCLUDED else
                "blocked" if item.status is EvaluationStatus.BLOCKED else
                "pending"
            )
            rows.append(
                ReviewRow(
                    record_id=item.record_id,
                    status=item.status.value,
                    decision=decision,
                    rit=str(values.get(mapping.get("rit", ""), "") or ""),
                    tribunal=str(values.get(mapping.get("tribunal", ""), "") or ""),
                    programa=str(values.get(mapping.get("programa", ""), "") or ""),
                    nombre=str(values.get(mapping.get("nombre", ""), "") or ""),
                    observation=item.observation,
                    original_observation=item.observation,
                    source_sheet=item.source.sheet_name,
                    source_row=item.source.row_number,
                    rule_ids=tuple(item.rule_ids),
                    issues=issues,
                )
            )
        return tuple(rows)

    def load_rows(self, batch_id: str) -> tuple[ReviewRow, ...]:
        batch = self.db.get_batch(batch_id)
        if batch is None:
            raise KeyError("Lote no encontrado.")
        mapping = json.loads(batch["column_mapping"] or "{}")
        result: list[ReviewRow] = []
        for row in self.db.list_review_records(batch_id):
            values = json.loads(row["values_json"])
            issue_data = json.loads(row["issues_json"])
            result.append(
                ReviewRow(
                    record_id=row["record_id"],
                    status=row["evaluation_status"],
                    decision=row["decision"],
                    rit=str(values.get(mapping.get("rit", ""), "") or ""),
                    tribunal=str(values.get(mapping.get("tribunal", ""), "") or ""),
                    programa=str(values.get(mapping.get("programa", ""), "") or ""),
                    nombre=str(values.get(mapping.get("nombre", ""), "") or ""),
                    observation=row["edited_observation"],
                    original_observation=row["original_observation"],
                    source_sheet=row["source_sheet"],
                    source_row=int(row["source_row"]),
                    rule_ids=tuple(json.loads(row["rule_ids_json"])),
                    issues=tuple(
                        f"{item.get('code', '')}: {item.get('message', '')}".strip(": ")
                        for item in issue_data
                    ),
                )
            )
        return tuple(result)

    def approve_record(
        self,
        batch_id: str,
        record_id: str,
        *,
        observation: str | None = None,
        reason: str = "",
    ) -> None:
        self.db.set_record_decision(
            batch_id,
            record_id,
            "approved",
            observation=observation,
            reason=reason,
        )

    def exclude_record(
        self,
        batch_id: str,
        record_id: str,
        *,
        reason: str,
    ) -> None:
        self.db.set_record_decision(
            batch_id,
            record_id,
            "excluded",
            reason=reason,
        )

    def restore_record(self, batch_id: str, record_id: str) -> None:
        self.db.restore_record(batch_id, record_id)

    def document_missing_cross_sheet_exception(
        self, batch_id: str, *, responsible: str, reason: str
    ) -> None:
        self.db.document_missing_cross_sheet_exception(
            batch_id, responsible=responsible, reason=reason
        )

    def approve_batch(self, batch_id: str, *, require_review_import: bool = False) -> str:
        """Una acción humana, una transacción de aprobación y snapshot."""
        return self.db.approve_batch(batch_id, require_review_import=require_review_import)
