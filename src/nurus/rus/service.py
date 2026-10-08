from __future__ import annotations

from datetime import date

from .catalog import load_catalog_snapshot
from .columns import H2_COLUMNS, ColumnMappingError, map_columns, map_cross_columns
from .models import (
    Evaluation,
    EvaluationBatch,
    EvaluationStatus,
    Issue,
    Mode,
    ReadBatch,
    SourceRecord,
)
from .rules import (
    as_date,
    evaluate_compliance,
    evaluate_reports,
    evaluate_waiting,
    match_key,
    no_follow_up,
    tribunal,
)

ENGINE_VERSION = "0.2.1"

_REQUIRED_COLUMNS: dict[Mode, tuple[str, ...]] = {
    Mode.ESPERA: ("programa", "tribunal", "espera"),
    Mode.CUMPLIMIENTO: ("programa", "tribunal", "dias_cumpl", "dias_egresar"),
    Mode.INFORMES: ("programa", "tribunal", "vencimiento"),
}


def _cross_index(
    records: tuple[SourceRecord, ...],
    columns: dict[str, str],
    as_of: date,
    excel_epoch: str,
) -> tuple[dict[tuple[str, ...], SourceRecord], dict[tuple[str, ...], tuple[SourceRecord, ...]], list[str]]:
    if not records:
        return {}, {}, []
    missing = set(H2_COLUMNS) - set(columns)
    if missing:
        return {}, {}, [
            f"CROSS_MAPPING_MISSING: faltan {', '.join(sorted(missing))}; C-10 no se evaluará."
        ]
    index: dict[tuple[str, ...], SourceRecord] = {}
    conflicts: dict[tuple[str, ...], tuple[SourceRecord, ...]] = {}
    for record in records:
        due = as_date(record.values.get(columns["vencimiento"]), excel_epoch)
        if not due or due <= as_of:
            continue
        key = match_key(record.values, columns)
        if key and key[0]:
            if key in conflicts:
                conflicts[key] += (record,)
            elif key in index:
                previous = index[key]
                previous_due = as_date(previous.values.get(columns["vencimiento"]), excel_epoch)
                if previous_due != due:
                    conflicts[key] = (index.pop(key), record)
            else:
                index[key] = record
    warnings = ([f"CROSS_RECORD_CONFLICT: {len(conflicts)} identidades con vencimientos futuros distintos; revisar las filas relacionadas."]
                if conflicts else [])
    return index, conflicts, warnings


def _missing_value(value: object) -> bool:
    return str(value if value is not None else "").strip().casefold() in {
        "",
        "nan",
        "nat",
        "none",
        "---",
        "-",
    }


def _precheck(
    record: SourceRecord, columns: dict[str, str], mode: Mode
) -> tuple[Issue, ...]:
    issues: list[Issue] = []
    missing_columns = [key for key in _REQUIRED_COLUMNS[mode] if key not in columns]
    if missing_columns:
        issues.append(
            Issue(
                "REQUIRED_COLUMNS_MISSING",
                "Faltan columnas obligatorias para "
                f"{mode.value}: {', '.join(missing_columns)}.",
                record.source,
            )
        )
        return tuple(issues)

    missing_values: list[str] = []
    # Programa y tribunal identifican el contexto mínimo de cualquier modalidad.
    # Los campos propios de cada regla (días/fechas) se validan dentro de la regla
    # para conservar su código de incidencia específico.
    for key in ("programa", "tribunal"):
        column = columns.get(key)
        if column and _missing_value(record.values.get(column)):
            missing_values.append(key)
    if missing_values:
        issues.append(
            Issue(
                "REQUIRED_DATA_MISSING",
                "Faltan datos obligatorios para revisar la fila: "
                f"{', '.join(missing_values)}.",
                record.source,
            )
        )

    if "tribunal" in columns and not _missing_value(record.values.get(columns["tribunal"])):
        if tribunal(record.values.get(columns["tribunal"])) is None:
            issues.append(
                Issue(
                    "G-06",
                    "Tribunal no reconocido; la fila requiere revisión manual.",
                    record.source,
                )
            )
    return tuple(issues)


def _dedupe_issues(issues: tuple[Issue, ...] | list[Issue]) -> tuple[Issue, ...]:
    seen: set[tuple[str, str, int]] = set()
    result: list[Issue] = []
    for issue in issues:
        key = (issue.code, issue.message, issue.source.row_number)
        if key not in seen:
            seen.add(key)
            result.append(issue)
    return tuple(result)


def evaluate_batch(
    batch: ReadBatch,
    *,
    as_of: date | None = None,
    catalog_path: str | None = None,
) -> EvaluationBatch:
    """Evalúa un lote de forma determinista y sin efectos externos.

    Todas las filas de origen se conservan como reviewed, blocked o excluded.
    """
    today = as_of or date.today()
    catalog, catalog_hash = load_catalog_snapshot(catalog_path)
    warnings = list(batch.warnings)

    try:
        columns = dict(batch.column_mapping) or (
            map_columns(batch.records[0].values.keys(), batch.mode) if batch.records else {}
        )
    except ColumnMappingError as exc:
        columns = {}
        warnings.append(f"COLUMN_MAPPING_AMBIGUOUS: {exc}")

    missing_columns = [key for key in _REQUIRED_COLUMNS[batch.mode] if key not in columns]
    if missing_columns:
        warnings.append(
            "REQUIRED_COLUMNS_MISSING: faltan "
            + ", ".join(missing_columns)
            + "; las filas quedan bloqueadas."
        )

    cross: dict[tuple[str, ...], SourceRecord] = {}
    cross_conflicts: dict[tuple[str, ...], tuple[SourceRecord, ...]] = {}
    if batch.mode is Mode.CUMPLIMIENTO:
        try:
            cross_columns = dict(batch.cross_mapping)
            if batch.cross_records and not cross_columns:
                cross_columns = map_cross_columns(batch.cross_records[0].values.keys())
        except ColumnMappingError as exc:
            cross_columns = {}
            warnings.append(f"CROSS_MAPPING_AMBIGUOUS: {exc}")
        cross, cross_conflicts, cross_warnings = _cross_index(
            batch.cross_records, cross_columns, today, batch.excel_epoch
        )
        warnings.extend(cross_warnings)

    evaluations: list[Evaluation] = []
    for record in batch.records:
        precheck = _precheck(record, columns, batch.mode)
        excluded = False
        program_column = columns.get("programa")
        if program_column and no_follow_up(record.values.get(program_column)):
            excluded = True

        if any(issue.code.startswith("REQUIRED_") for issue in precheck):
            observation, rule_ids, rule_issues = "", (), ()
            related = ()
        elif batch.mode is Mode.ESPERA:
            observation, rule_ids, rule_issues = evaluate_waiting(
                record, columns, catalog, today, batch.excel_epoch
            )
            related = ()
        elif batch.mode is Mode.INFORMES:
            observation, rule_ids, rule_issues = evaluate_reports(
                record, columns, catalog, today, batch.excel_epoch
            )
            related = ()
        else:
            key = match_key(record.values, columns)
            cross_record = cross.get(key) if key else None
            cross_due = None
            if cross_record:
                cross_due_column = dict(batch.cross_mapping).get("vencimiento")
                if not cross_due_column:
                    try:
                        cross_due_column = map_cross_columns(
                            cross_record.values.keys()
                        )["vencimiento"]
                    except (ColumnMappingError, KeyError):
                        cross_due_column = ""
                if cross_due_column:
                    cross_due = as_date(
                        cross_record.values.get(cross_due_column),
                        batch.excel_epoch,
                    )
            observation, rule_ids, rule_issues = evaluate_compliance(
                record,
                columns,
                catalog,
                today,
                cross_due,
                batch.excel_epoch,
            )
            related = (
                (cross_record.source,)
                if cross_record is not None and "C-10" in rule_ids
                else ()
            )
            if key in cross_conflicts:
                related = tuple(item.source for item in cross_conflicts[key])
                rule_issues = (*rule_issues, Issue(
                    "CROSS_RECORD_CONFLICT",
                    "La hoja cruzada contiene vencimientos futuros distintos para este registro; C-10 no se infiere. Revisar las filas de origen relacionadas.",
                    record.source,
                ))

        issues = _dedupe_issues([*precheck, *rule_issues])
        if excluded or "E-01" in rule_ids:
            status = EvaluationStatus.EXCLUDED
        elif issues:
            status = EvaluationStatus.BLOCKED
        else:
            status = EvaluationStatus.REVIEWED

        evaluations.append(
            Evaluation(
                record_id=record.record_id,
                source=record.source,
                values=record.values,
                observation=observation,
                rule_ids=tuple(rule_ids),
                issues=issues,
                related_sources=related,
                status=status,
            )
        )

    return EvaluationBatch(
        mode=batch.mode,
        as_of=today,
        evaluations=tuple(evaluations),
        warnings=tuple(dict.fromkeys(warnings)),
        source_path=batch.source_path,
        workbook_name=batch.workbook_name,
        workbook_sha256=batch.workbook_sha256,
        primary_sheet=batch.primary_sheet,
        cross_sheet=batch.cross_sheet,
        header_row=batch.header_row,
        excel_epoch=batch.excel_epoch,
        engine_version=ENGINE_VERSION,
        catalog_sha256=catalog_hash,
        column_mapping=columns,
    )
