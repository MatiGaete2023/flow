import pandas as pd
from nurus.rus.reader import _records


def test_tuple_reader_keeps_physical_rows_types_and_leaves_frame_untouched():
    frame = pd.DataFrame([["X-1", 42, None], [None, None, None], ["X-2", 1.5, "=A1"]],
                         columns=[" RIT ", "NUMERO", "FORMULA"], index=[0, 3, 5], dtype=object)
    expected = frame.copy(deep=True)
    records = _records(frame, "original.xlsx", "sha", "Hoja", 4, {"rit": "RIT"})
    assert [record.source.row_number for record in records] == [5, 10]
    assert type(records[0].values["NUMERO"]) is int
    assert records[1].values["NUMERO"] == 1.5
    assert records[1].values["FORMULA"] == "=A1"
    pd.testing.assert_frame_equal(frame, expected)
