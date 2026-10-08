from __future__ import annotations

import hashlib
from io import BytesIO
import os
import tempfile
from pathlib import Path

from .columns import H2_COLUMNS, ColumnMappingError, map_columns, map_cross_columns, normalize
from .models import Mode, ReadBatch, SourceRecord, SourceReference

_ALLOWED_SUFFIXES = {".xls", ".xlsx", ".xlsm"}
_AUTO_DETECT_HEADER_SCAN_ROWS = 30
_REQUIRED_MAPPING_KEYS: dict[Mode, tuple[str, ...]] = {
    Mode.ESPERA: ("programa", "tribunal", "espera"),
    Mode.CUMPLIMIENTO: ("programa", "tribunal", "dias_cumpl", "dias_egresar"),
    Mode.INFORMES: ("programa", "tribunal", "vencimiento"),
}
_HEADER_IDENTITY_KEYS = ("rit", "rut", "nombre")


class WorkbookReadError(ValueError):
    pass


def list_workbook_sheets(path: str | Path) -> tuple[str, ...]:
    """Inspección de nombres para el selector; no ejecuta reglas ni macros."""
    import pandas as pd

    source = Path(path).expanduser().resolve()
    if not source.is_file() or source.suffix.lower() not in _ALLOWED_SUFFIXES:
        raise WorkbookReadError("Selecciona un libro .xls, .xlsx o .xlsm existente.")
    try:
        with pd.ExcelFile(BytesIO(source.read_bytes()), engine=_engine(source)) as book:
            return tuple(book.sheet_names)
    except Exception as exc:
        raise WorkbookReadError(f"No se pudieron identificar las hojas: {exc}") from exc


def _snapshot_file(path: Path, *, max_file_size_bytes: int | None = None) -> tuple[Path, str]:
    if max_file_size_bytes is not None and path.stat().st_size > max_file_size_bytes:
        raise WorkbookReadError(
            f"El archivo supera el límite configurado de {max_file_size_bytes} bytes."
        )
    digest = hashlib.sha256()
    temp_path: Path | None = None
    try:
        with path.open("rb") as source, tempfile.NamedTemporaryFile(
            suffix=path.suffix, delete=False
        ) as target:
            temp_path = Path(target.name)
            total = 0
            for chunk in iter(lambda: source.read(1024 * 1024), b""):
                total += len(chunk)
                if max_file_size_bytes is not None and total > max_file_size_bytes:
                    raise WorkbookReadError("El archivo supera el límite configurado durante la lectura.")
                digest.update(chunk)
                target.write(chunk)
        return temp_path, digest.hexdigest()
    except (OSError, WorkbookReadError) as exc:
        if temp_path:
            temp_path.unlink(missing_ok=True)
        raise WorkbookReadError(f"No se pudo crear una copia estable del archivo: {exc}") from exc


def _engine(path: Path) -> str:
    return "xlrd" if path.suffix.lower() == ".xls" else "openpyxl"


def _is_empty(value: object) -> bool:
    if value is None:
        return True
    text = str(value).strip().casefold()
    return text in {"", "nan", "nat", "none"}


def _records(
    frame,
    source_name: str,
    digest: str,
    sheet_name: str,
    header_row: int,
    mapping: dict[str, str],
) -> tuple[SourceRecord, ...]:
    headers = [str(header).strip() for header in frame.columns]
    records: list[SourceRecord] = []
    for index, row in zip(frame.index, frame.itertuples(index=False, name=None), strict=True):
        values = dict(zip(headers, row, strict=True))
        if all(_is_empty(value) for value in values.values()):
            continue
        # Totales, pies y notas pertenecen a la estructura del libro. Solo se
        # convierten en registros las filas con algún dato mapeado de caso.
        # Esto conserva en Excel las fórmulas originales sin pedir su revisión.
        if not any(
            column in values and not _is_empty(values[column])
            for column in mapping.values()
        ):
            continue
        records.append(
            SourceRecord(
                values=values,
                source=SourceReference(
                    source_name,
                    digest,
                    sheet_name,
                    header_row + int(index) + 1,
                ),
            )
        )
    return tuple(records)


def _sheet_by_name(names: list[str], requested: str) -> str:
    matches = [name for name in names if name.strip().casefold() == requested.strip().casefold()]
    if len(matches) != 1:
        raise WorkbookReadError(f"No existe una hoja única llamada {requested!r}.")
    return matches[0]


def _validate_headers(values: list[str], sheet_name: str) -> list[str]:
    values = [str(value).strip() for value in values]
    seen: dict[str, str] = {}
    duplicates: list[str] = []
    for header in values:
        normalized = normalize(header)
        if not normalized:
            continue
        if normalized in seen:
            duplicates.extend([seen[normalized], header])
        else:
            seen[normalized] = header
    if duplicates:
        unique = list(dict.fromkeys(duplicates))
        raise WorkbookReadError(
            f"Encabezados duplicados o equivalentes en {sheet_name!r}: {', '.join(unique)}."
        )
    return values


def _headers(pd, workbook, sheet_name: str, header_row: int) -> list[str]:
    try:
        raw = pd.read_excel(
            workbook,
            sheet_name=sheet_name,
            header=None,
            nrows=header_row,
            dtype=object,
            keep_default_na=False,
        )
    except Exception as exc:
        raise WorkbookReadError(f"No se pudieron leer los encabezados de {sheet_name!r}: {exc}") from exc
    if raw.empty or len(raw.index) < header_row:
        raise WorkbookReadError(
            f"La hoja {sheet_name!r} no contiene la fila de encabezado {header_row}."
        )
    return _validate_headers(raw.iloc[header_row - 1].tolist(), sheet_name)


def _has_required_mapping(mapping: dict[str, str], mode: Mode) -> bool:
    required = set(_REQUIRED_MAPPING_KEYS[mode])
    return required.issubset(mapping) and any(key in mapping for key in _HEADER_IDENTITY_KEYS)


def _resolve_header_row(pd, workbook, sheet_name: str, mode: Mode, header_row: int) -> tuple[int, list[str], dict[str, str]]:
    """Usa la fila indicada o detecta una cabecera desplazada solo si es inequívoca."""
    initial_error: Exception | None = None
    try:
        headers = _headers(pd, workbook, sheet_name, header_row)
        mapping = map_columns(headers, mode)
        if _has_required_mapping(mapping, mode) or header_row != 1:
            return header_row, headers, mapping
    except (WorkbookReadError, ColumnMappingError) as exc:
        initial_error = exc

    # Algunas exportaciones RUS incluyen una portada o filas vacías antes de la tabla.
    # La detección se limita a las primeras filas y exige todos los campos obligatorios
    # más un identificador de persona o causa para no aceptar una tabla incidental.
    try:
        raw = pd.read_excel(
            workbook,
            sheet_name=sheet_name,
            header=None,
            nrows=_AUTO_DETECT_HEADER_SCAN_ROWS,
            dtype=object,
            keep_default_na=False,
        )
    except Exception as exc:
        raise WorkbookReadError(f"No se pudieron revisar las filas iniciales de {sheet_name!r}: {exc}") from exc

    candidates: list[tuple[int, list[str], dict[str, str]]] = []
    for index in range(len(raw.index)):
        candidate_row = index + 1
        if candidate_row == header_row:
            continue
        try:
            candidate_headers = _validate_headers(raw.iloc[index].tolist(), sheet_name)
            candidate_mapping = map_columns(candidate_headers, mode)
        except (WorkbookReadError, ColumnMappingError):
            continue
        if _has_required_mapping(candidate_mapping, mode):
            candidates.append((candidate_row, candidate_headers, candidate_mapping))

    if len(candidates) == 1:
        return candidates[0]
    if len(candidates) > 1:
        rows = ", ".join(str(item[0]) for item in candidates)
        raise WorkbookReadError(
            f"Se encontraron varias filas que parecen encabezados en {sheet_name!r}: {rows}. "
            "Corrige o simplifica la planilla antes de analizarla."
        )
    if initial_error is not None:
        raise initial_error

    # Conserva el comportamiento previo: la evaluación explicará las columnas faltantes.
    headers = _headers(pd, workbook, sheet_name, header_row)
    return header_row, headers, map_columns(headers, mode)


def _read_sheet(pd, workbook, sheet_name: str, header_row: int):
    try:
        return pd.read_excel(
            workbook,
            sheet_name=sheet_name,
            header=header_row - 1,
            dtype=object,
            keep_default_na=False,
        )
    except Exception as exc:
        raise WorkbookReadError(f"No se pudo leer la hoja {sheet_name!r}: {exc}") from exc


def _detect_primary_sheet(pd, workbook, names: list[str], selected_mode: Mode, header_row: int):
    """Selecciona una tabla única de la modalidad solicitada, sin cambiar reglas.

    No usa cantidad de columnas como desempate: dos tablas compatibles requieren
    elección explícita, aunque una tenga más datos opcionales que la otra.
    """
    usable = [name for name in names if normalize(name) not in {"ob", "medidas vencidas"}]
    candidates: list[tuple[int, Mode, str, int, list[str], dict[str, str]]] = []
    for name in usable:
        raw = pd.read_excel(workbook, sheet_name=name, header=None,
                            nrows=max(header_row, _AUTO_DETECT_HEADER_SCAN_ROWS),
                            dtype=object, keep_default_na=False)
        for index in range(len(raw.index)):
            if header_row != 1 and index != header_row - 1:
                continue
            try:
                headers = _validate_headers(raw.iloc[index].tolist(), name)
            except WorkbookReadError:
                continue
            for candidate_mode in Mode:
                try:
                    mapping = map_columns(headers, candidate_mode)
                except ColumnMappingError:
                    continue
                if _has_required_mapping(mapping, candidate_mode):
                    candidates.append((len(mapping), candidate_mode, name, index + 1, headers, mapping))

    preferred = [item for item in candidates if item[1] is selected_mode]
    if not candidates:
        raise WorkbookReadError(
            "No se identifica una tabla de Espera, Cumplimiento o Informes en las hojas del archivo. "
            "Revisa sus encabezados o selecciona la hoja explícitamente."
        )
    if not preferred:
        descriptions = ", ".join(dict.fromkeys(f"{item[1].value} ({item[2]})" for item in candidates))
        raise WorkbookReadError(
            f"No hay una tabla compatible con {selected_mode.value}. "
            f"Se detectaron: {descriptions}. Cambia la modalidad y vuelve a analizar; "
            "no necesitas seleccionar el archivo otra vez."
        )
    if len(preferred) != 1:
        descriptions = ", ".join(f"{item[2]} (fila {item[3]})" for item in preferred)
        raise WorkbookReadError(
            "La estructura coincide con más de una tabla posible: " + descriptions + ". "
            "Selecciona la hoja explícitamente."
        )
    _, actual_mode, name, row, headers, mapping = preferred[0]
    return actual_mode, name, row, headers, mapping


def _excel_epoch(workbook) -> str:
    book = getattr(workbook, "book", None)
    datemode = getattr(book, "datemode", None)
    if datemode == 1:
        return "1904"
    epoch = getattr(book, "epoch", None)
    if getattr(epoch, "year", None) == 1904:
        return "1904"
    return "1900"


def _select_cross(pd, workbook, names: list[str], requested: str | None, warnings: list[str]):
    names = [name for name in names if normalize(name) not in {"ob", "medidas vencidas"}]
    checked = [_sheet_by_name(names, requested)] if requested else names
    candidates = []
    ambiguous = False
    for name in checked:
        raw = pd.read_excel(workbook, sheet_name=name, header=None,
                            nrows=_AUTO_DETECT_HEADER_SCAN_ROWS, dtype=object, keep_default_na=False)
        for index in range(len(raw.index)):
            try:
                headers = _validate_headers(raw.iloc[index].tolist(), name)
                mapping = map_cross_columns(headers)
            except ColumnMappingError as exc:
                ambiguous = True
                warnings.append(f"CROSS_MAPPING_AMBIGUOUS: {name}: {exc}")
                continue
            except WorkbookReadError:
                # Solo una fila con suficientes alias de cruce puede ser una
                # cabecera duplicada; títulos repetidos en una portada no bastan.
                mapping = {}
                try:
                    unique = list(dict.fromkeys(str(v).strip() for v in raw.iloc[index].tolist()))
                    mapping = map_cross_columns(unique)
                except ColumnMappingError:
                    pass
                if set(mapping) == set(H2_COLUMNS):
                    ambiguous = True
                    warnings.append(f"CROSS_MAPPING_AMBIGUOUS: encabezados duplicados en {name}.")
                continue
            if set(mapping) == set(H2_COLUMNS):
                candidates.append((name, index + 1, mapping))
    if len(candidates) > 1 or ambiguous:
        warnings.append("CROSS_SHEET_AMBIGUOUS: selecciona o corrige la hoja de cruce explícitamente.")
        return "", 1, {}
    if candidates:
        return candidates[0]
    if requested or any(name.strip().casefold() == "hoja2" for name in checked):
        warnings.append("CROSS_MAPPING_MISSING: no se encontró una cabecera de cruce completa; C-10 no se evaluará.")
    return "", 1, {}


def read_workbook(
    path: str | Path,
    mode: Mode | str,
    *,
    sheet_name: str | None = None,
    cross_sheet_name: str | None = None,
    header_row: int = 1,
    max_file_size_bytes: int | None = None,
) -> ReadBatch:
    """Lee una copia estable del libro y conserva procedencia física de cada fila.

    `header_row` es 1-based. No se modifica el archivo original ni se ejecutan macros.
    La copia temporal garantiza que el SHA-256 corresponde a los mismos bytes
    procesados por pandas.
    """
    if header_row < 1:
        raise WorkbookReadError("header_row debe ser mayor o igual a 1.")
    try:
        import pandas as pd
    except ImportError as exc:  # pragma: no cover
        raise WorkbookReadError("Falta pandas; instala las dependencias de NuRus.") from exc

    selected_mode = Mode(mode)
    source = Path(path).expanduser().resolve()
    if not source.is_file():
        raise WorkbookReadError(f"Archivo no encontrado: {source}")
    if source.suffix.lower() not in _ALLOWED_SUFFIXES:
        raise WorkbookReadError("Solo se admiten archivos .xls, .xlsx y .xlsm.")

    snapshot, digest = _snapshot_file(source, max_file_size_bytes=max_file_size_bytes)
    try:
        try:
            source_bytes = snapshot.read_bytes()
            if hashlib.sha256(source_bytes).hexdigest() != digest:
                raise WorkbookReadError("La copia de origen no coincide con su hash.")
            workbook = pd.ExcelFile(BytesIO(source_bytes), engine=_engine(source))
        except ImportError as exc:
            dependency = "xlrd" if source.suffix.lower() == ".xls" else "openpyxl"
            raise WorkbookReadError(
                f"No se puede leer {source.suffix}: instala {dependency}."
            ) from exc
        except Exception as exc:
            raise WorkbookReadError(f"No se pudo abrir el libro: {exc}") from exc

        with workbook:
            names = list(workbook.sheet_names)
            if not names:
                raise WorkbookReadError("El libro no contiene hojas.")
            detected_structure = None
            if sheet_name:
                primary_name = _sheet_by_name(names, sheet_name)
            elif any(name.strip().casefold() == selected_mode.value.casefold() for name in names):
                primary_name = _sheet_by_name(names, selected_mode.value)
            elif len(names) == 1:
                primary_name = names[0]
            else:
                detected_structure = _detect_primary_sheet(
                    pd, workbook, names, selected_mode, header_row
                )
                selected_mode, primary_name, actual_header_row, primary_headers, primary_mapping = detected_structure
            if normalize(primary_name) in {"ob", "medidas vencidas"}:
                raise WorkbookReadError("OB y Medidas vencidas no se procesan como Espera, Cumplimiento o Informes.")

            if detected_structure is None:
                try:
                    actual_header_row, primary_headers, primary_mapping = _resolve_header_row(
                        pd, workbook, primary_name, selected_mode, header_row
                    )
                except ColumnMappingError as exc:
                    raise WorkbookReadError(str(exc)) from exc
            primary = _records(
                _read_sheet(pd, workbook, primary_name, actual_header_row),
                source.name,
                digest,
                primary_name,
                actual_header_row,
                primary_mapping,
            )

            warnings: list[str] = []
            if detected_structure is not None:
                warnings.append(
                    f"MODE_AND_SHEET_AUTODETECTED: se detectó {selected_mode.value} en {primary_name!r} por sus columnas."
                )
            if actual_header_row != header_row:
                warnings.append(
                    f"HEADER_ROW_AUTODETECTED: se usó la fila {actual_header_row} como encabezado de {primary_name!r}."
                )
            cross: tuple[SourceRecord, ...] = ()
            cross_mapping: dict[str, str] = {}
            chosen = ""
            if selected_mode is Mode.CUMPLIMIENTO:
                other_names = [name for name in names if name != primary_name]
                chosen, cross_header, cross_mapping = _select_cross(
                    pd, workbook, other_names, cross_sheet_name, warnings
                )
                if chosen:
                    cross = _records(
                        _read_sheet(pd, workbook, chosen, cross_header),
                        source.name, digest, chosen, cross_header, cross_mapping,
                    )
                    if cross_header != 1:
                        warnings.append(f"CROSS_HEADER_AUTODETECTED: {chosen}, fila {cross_header}.")
                if not chosen:
                    warnings.append(
                        "CROSS_SHEET_NOT_SELECTED: C-10 no se evaluará sin una hoja de cruce identificable."
                    )

            return ReadBatch(
                mode=selected_mode,
                records=primary,
                cross_records=cross,
                warnings=tuple(dict.fromkeys(warnings)),
                source_path=str(source),
                workbook_name=source.name,
                workbook_sha256=digest,
                primary_sheet=primary_name,
                cross_sheet=chosen,
                header_row=actual_header_row,
                excel_epoch=_excel_epoch(workbook),
                column_mapping=primary_mapping,
                cross_mapping=cross_mapping,
                source_bytes=source_bytes,
            )
    finally:
        try:
            os.unlink(snapshot)
        except OSError:
            pass
