"""Contrato de rangos simulado: preservación de celdas y límite de llamadas."""
from nurus.services.exports import _native_resolution_validation, _native_write_column


class Sheet:
    def __init__(self):
        self.values = {}
        self.writes = []
        self.formats = []

    def Cells(self, row, column):
        return row, column

    def Range(self, first, last):
        sheet = self
        assert first[1] == last[1]
        column = first[1]
        rows = range(first[0], last[0] + 1)

        class Range:
            @property
            def Value2(self):
                values = tuple((sheet.values.get((row, column)),) for row in rows)
                return values[0][0] if len(values) == 1 else values

            @Value2.setter
            def Value2(self, values):
                assert len(values) == len(rows)
                sheet.writes.append((first, last, values))
                for row, (value,) in zip(rows, values, strict=True):
                    sheet.values[row, column] = value

            @property
            def NumberFormat(self):
                return "@"

            @NumberFormat.setter
            def NumberFormat(self, value):
                sheet.formats.extend((row, column) for row in rows)
                assert value == "@"

        return Range()


def test_ten_thousand_rows_use_twenty_writes_and_keep_long_observations():
    sheet = Sheet()
    observation = "texto " * 500  # más de 255 caracteres
    _native_write_column(sheet, 4, [(row, observation) for row in range(2, 10002)])
    assert len(sheet.writes) == 20
    assert sheet.values[10001, 4] == observation


def test_sparse_rows_do_not_replace_formulas_or_format_outside_selection():
    sheet = Sheet()
    sheet.values[3, 2] = "=1+2"
    _native_write_column(sheet, 2, [(4, "última"), (2, "primera")])
    assert sheet.values[3, 2] == "=1+2"
    assert (3, 2) not in sheet.formats
    assert len(sheet.writes) == 2


def test_fill_proposal_only_in_empty_cells_including_scalar_range():
    sheet = Sheet()
    sheet.values.update({(2, 2): "observación original", (3, 2): "=1+2", (4, 2): ""})
    _native_write_column(sheet, 2, [(row, "propuesta") for row in range(2, 6)], keep_existing=True)
    _native_write_column(sheet, 2, [(7, "otra propuesta")], keep_existing=True)
    assert sheet.values[2, 2] == "observación original"
    assert sheet.values[3, 2] == "=1+2"
    assert sheet.values[4, 2] == sheet.values[5, 2] == "propuesta"
    assert sheet.values[7, 2] == "otra propuesta"
    assert (2, 2) not in sheet.formats and (3, 2) not in sheet.formats


def test_res_validation_uses_hidden_named_range_without_locale_call():
    class Cell:
        def __init__(self):
            self.Value2 = None

    class Validation:
        def __init__(self):
            self.deleted = False
            self.added = None
            self.IgnoreBlank = self.InCellDropdown = self.ShowError = self.ShowInput = None
            self.ErrorTitle = self.ErrorMessage = self.InputTitle = self.InputMessage = None

        def Delete(self):
            self.deleted = True

        def Add(self, *args):
            self.added = args

    class Range:
        def __init__(self):
            self.Validation = Validation()

    class Columns:
        def __call__(self, column):
            class Column:
                ColumnWidth = 10
            return Column()

    class Sheet:
        def __init__(self, name='Espera'):
            self.Name = name
            self.Visible = -1
            self._cells = {}
            self.Columns = Columns()
            self.range = Range()

        def Cells(self, row, column):
            self._cells.setdefault((row, column), Cell())
            return self._cells[(row, column)]

        def Range(self, first, last):
            return self.range

    class Worksheets:
        def __init__(self):
            self.items = {'Espera': Sheet('Espera')}

        def __call__(self, name):
            if name not in self.items:
                raise KeyError(name)
            return self.items[name]

        def Add(self):
            sheet = Sheet()
            self.items['NURUS_LISTAS'] = sheet
            return sheet

    class Name:
        def Delete(self):
            raise KeyError('not created yet')

    class Names:
        def __init__(self):
            self.added = None

        def __call__(self, name):
            return Name()

        def Add(self, name, reference):
            self.added = (name, reference)

    class Book:
        def __init__(self):
            self.Worksheets = Worksheets()
            self.Names = Names()
            # Reproduce la forma que provocó el error real: una tupla no invocable.
            self.International = ('no debe usarse',)

    book = Book()
    sheet = book.Worksheets('Espera')
    _native_resolution_validation(book, sheet, 8, 2, 20)

    technical = book.Worksheets('NURUS_LISTAS')
    assert technical.Visible == 0
    assert [technical.Cells(row, 1).Value2 for row in range(2, 5)] == ['PC_IE', 'PC_INFO', 'NOMENCL']
    assert book.Names.added == ('NURUS_RES_TIPOS', "='NURUS_LISTAS'!$A$2:$A$4")
    assert sheet.range.Validation.added == (3, 1, 1, '=NURUS_RES_TIPOS')
    assert sheet.range.Validation.InCellDropdown is True
